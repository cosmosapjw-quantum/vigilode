//! Exact returned-value and callback observation probe for ownership projection.
//! Run the same source against baseline and candidate. This is a diagnostic;
//! expected failures are retained as complete serialized observations.
use serde_json::json;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
use vigilode::{Atol, CallbackError, Direction, Jacobian, Options, Problem, Rodas5p};

struct ProbeAllocator;
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}
fn note() {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCS.try_with(|n| n.set(n.get().saturating_add(1)));
    }
}
unsafe impl GlobalAlloc for ProbeAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        note();
        unsafe { System.realloc(p, layout, size) }
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: ProbeAllocator = ProbeAllocator;

const CASES: [&str; 11] = [
    "dense",
    "jvp",
    "declared-band",
    "vector-atol",
    "max-steps",
    "fatal",
    "zero-span",
    "dense-event",
    "event-nonfinite",
    "fd",
    "rhs-reentrant",
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let value = |key: &str, default: &str| {
        args.windows(2)
            .find(|a| a[0] == key)
            .map_or(default.to_string(), |a| a[1].clone())
    };
    if args.iter().any(|a| a == "--help") {
        println!(
            "ownership_projection_probe --case all|{} --mode probe|alloc|timing --repeats N",
            CASES.join("|")
        );
        return;
    }
    let selected = value("--case", "all");
    let mode = value("--mode", "probe");
    assert!(["probe", "alloc", "timing"].contains(&mode.as_str()));
    assert!(selected == "all" || CASES.contains(&selected.as_str()));
    let repeats: usize = value("--repeats", "1").parse().expect("integer repeats");
    assert!(repeats > 0);
    let mut rows = Vec::new();
    for case in CASES
        .into_iter()
        .filter(|c| selected == "all" || selected == *c)
    {
        for repeat in 0..repeats {
            let rhs_calls = Arc::new(AtomicU64::new(0));
            let derivative_calls = Arc::new(AtomicU64::new(0));
            let event_calls = Arc::new(AtomicU64::new(0));
            let nested_calls = Arc::new(AtomicU64::new(0));
            let rc = rhs_calls.clone();
            let nc = nested_calls.clone();
            let mut problem = Problem::new(2, move |_, y, out| {
                let call = rc.fetch_add(1, Ordering::Relaxed) + 1;
                if case == "fatal" && call == 15 {
                    return Err(CallbackError::Fatal(
                        "ownership probe injected fatal".into(),
                    ));
                }
                if case == "rhs-reentrant" && call == 1 {
                    nc.fetch_add(1, Ordering::Relaxed);
                    let nested = Problem::new(1, |_, z, dz| {
                        dz[0] = -z[0];
                        Ok(())
                    })
                    .autonomous();
                    let result = Rodas5p::solve(&nested, (0.0, 0.0), &[1.0], &Options::default())
                        .map_err(|e| CallbackError::Fatal(e.to_string()))?;
                    assert!(result.is_success());
                }
                out[0] = -y[0];
                out[1] = -2.0 * y[1];
                Ok(())
            })
            .autonomous();
            if case == "jvp" {
                let dc = derivative_calls.clone();
                problem = problem.jacobian(Jacobian::jvp(move |_, _, v, out| {
                    dc.fetch_add(1, Ordering::Relaxed);
                    out[0] = -v[0];
                    out[1] = -2.0 * v[1];
                    Ok(())
                }));
            } else if case != "fd" {
                let dc = derivative_calls.clone();
                problem = problem.jacobian(Jacobian::dense(move |_, _, out| {
                    dc.fetch_add(1, Ordering::Relaxed);
                    out.copy_from_slice(&[-1.0, 0.0, 0.0, -2.0]);
                    Ok(())
                }));
            }
            if case == "declared-band" {
                problem = problem.banded(0, 0);
            }
            let dense = case == "dense-event" || case == "event-nonfinite";
            if dense {
                let ec = event_calls.clone();
                problem = problem.event(
                    move |t, _| {
                        let call = ec.fetch_add(1, Ordering::Relaxed) + 1;
                        if case == "event-nonfinite" && call == 7 {
                            f64::NAN
                        } else {
                            t - 0.125
                        }
                    },
                    Direction::Up,
                );
            }
            let options = Options {
                rtol: 1e-7,
                atol: if case == "vector-atol" {
                    Atol::Vector(vec![1e-10, 2e-10])
                } else {
                    Atol::Scalar(1e-10)
                },
                h0: 0.01,
                max_steps: if case == "max-steps" { 2 } else { 1000 },
                dense_output: dense,
                ..Options::default()
            };
            let end = if case == "zero-span" { 0.0 } else { 0.25 };
            ALLOCS.with(|n| n.set(0));
            TRACK.with(|flag| flag.set(mode == "alloc"));
            let start = Instant::now();
            let result = Rodas5p::solve(&problem, (0.0, end), &[1.0, 2.0], &options);
            let solve_ns = start.elapsed().as_nanos();
            TRACK.with(|flag| flag.set(false));
            let allocations = ALLOCS.with(Cell::get);
            let solution = result.expect("a typed Solution including expected failure");
            let t_bits: Vec<u64> = solution.t.iter().map(|t| t.to_bits()).collect();
            let y_bits: Vec<Vec<u64>> = solution
                .y
                .iter()
                .map(|row| row.iter().map(|x| x.to_bits()).collect())
                .collect();
            let dense_samples: Vec<_> = [0.0_f64, 0.0375, 0.125, 0.2, 0.25].into_iter().map(|t| {
                json!({"t_bits":t.to_bits(), "state_bits":solution.at(t).map(|y| y.into_iter().map(f64::to_bits).collect::<Vec<_>>())})
            }).collect();
            let observation = serde_json::to_value(&solution).expect("solution serializes");
            let start_drop = Instant::now();
            drop(black_box(solution));
            let drop_ns = start_drop.elapsed().as_nanos();
            rows.push(json!({"case":case,"repeat":repeat,"observation":observation,"time_bits":t_bits,"state_bits":y_bits,"dense_samples":dense_samples,"callback_counts":{"rhs":rhs_calls.load(Ordering::Relaxed),"derivative":derivative_calls.load(Ordering::Relaxed),"event":event_calls.load(Ordering::Relaxed),"nested":nested_calls.load(Ordering::Relaxed)},"solve_allocations":if mode=="alloc" {Some(allocations)} else {None},"solve_ns":solve_ns,"drop_ns":drop_ns}));
        }
    }
    println!("{}", serde_json::to_string(&json!({"schema":"ownership-projection-probe-v1","mode":mode,"repeats":repeats,"rows":rows,"measurement_scope":"model construction outside; solve allocation count; solve and drop separately timed with full observation audit between; all repeats individually exported; no timing claim"})).unwrap());
}
