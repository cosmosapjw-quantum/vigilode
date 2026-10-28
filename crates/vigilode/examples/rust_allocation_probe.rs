//! Lightweight, single-solve diagnostic used for same-source before/after
//! comparisons of the finite-difference and nonautonomous derivative paths.
//! Output serialization and exact-solution evaluation happen after timing.
use serde_json::{Value, json};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
use vigilode::{Atol, Jacobian, Options, Problem, Rodas5p};

struct ProbeAlloc;
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}
fn note_alloc() {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCS.try_with(|n| n.set(n.get().saturating_add(1)));
    }
}
unsafe impl GlobalAlloc for ProbeAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note_alloc();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note_alloc();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        note_alloc();
        unsafe { System.realloc(p, layout, size) }
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: ProbeAlloc = ProbeAlloc;

#[derive(Clone, Default)]
struct Calls {
    rhs: Arc<AtomicU64>,
    jvp: Arc<AtomicU64>,
    ft: Arc<AtomicU64>,
}
fn rhs(c: &Calls) {
    c.rhs.fetch_add(1, Ordering::Relaxed);
}
fn jvp(c: &Calls) {
    c.jvp.fetch_add(1, Ordering::Relaxed);
}
fn ft(c: &Calls) {
    c.ft.fetch_add(1, Ordering::Relaxed);
}

fn model(
    name: &str,
    span: f64,
    model_param: f64,
) -> Result<(Problem, Vec<f64>, Vec<f64>, Calls), String> {
    let calls = Calls::default();
    match name {
        "dense-fd" => {
            let n = 8;
            let c = calls.clone();
            let p = Problem::new(n, move |_t, y, o| {
                rhs(&c);
                for k in 0..n / 2 {
                    let (a, w) = (0.3 + 0.1 * k as f64, 0.5 + 0.1 * k as f64);
                    o[2 * k] = -a * y[2 * k] - w * y[2 * k + 1];
                    o[2 * k + 1] = w * y[2 * k] - a * y[2 * k + 1];
                }
                Ok(())
            })
            .autonomous();
            let y0: Vec<f64> = (0..n)
                .map(|i| {
                    if i % 2 == 0 {
                        1.0 + i as f64 / 10.0
                    } else {
                        0.15 + (i / 2) as f64 * 0.02
                    }
                })
                .collect();
            let exact: Vec<f64> = (0..n)
                .map(|i| {
                    let k = i / 2;
                    let (s, co) = ((0.5 + 0.1 * k as f64) * span).sin_cos();
                    let decay = (-(0.3 + 0.1 * k as f64) * span).exp();
                    let x = 1.0 + (2 * k) as f64 / 10.0;
                    let z = 0.15 + k as f64 * 0.02;
                    if i % 2 == 0 {
                        decay * (co * x - s * z)
                    } else {
                        decay * (s * x + co * z)
                    }
                })
                .collect();
            Ok((p, y0, exact, calls))
        }
        "banded-fd" => {
            let n = 64;
            let c = calls.clone();
            let p = Problem::new(n, move |_t, y, o| {
                rhs(&c);
                for i in 0..n {
                    o[i] = -1.5 * y[i]
                        + (if i > 0 { 0.2 * y[i - 1] } else { 0.0 })
                        + (if i + 1 < n { 0.2 * y[i + 1] } else { 0.0 });
                }
                Ok(())
            })
            .banded(1, 1)
            .autonomous();
            let y0: Vec<f64> = (0..n)
                .map(|i| (std::f64::consts::PI * (i + 1) as f64 / (n + 1) as f64).sin())
                .collect();
            let lambda = -1.5 + 0.4 * (std::f64::consts::PI / (n + 1) as f64).cos();
            let exact = y0.iter().map(|x| (lambda * span).exp() * x).collect();
            Ok((p, y0, exact, calls))
        }
        "nonautonomous-dense-fd" => {
            let n = 8;
            let c = calls.clone();
            let p = Problem::new(n, move |t, y, o| {
                rhs(&c);
                for i in 0..n {
                    let a = 1.0 + i as f64 / n as f64;
                    let lambda = 2.0 + i as f64 / 4.0;
                    let target = (-t).exp() * a;
                    o[i] = -lambda * (y[i] - target) - target;
                }
                Ok(())
            });
            Ok((
                p,
                (0..n).map(|i| 1.0 + i as f64 / n as f64).collect(),
                (0..n)
                    .map(|i| (-span).exp() * (1.0 + i as f64 / n as f64))
                    .collect(),
                calls,
            ))
        }
        "span-parameter-control" => {
            let n = 8;
            let c = calls.clone();
            let p = Problem::new(n, move |_t, y, o| {
                rhs(&c);
                for i in 0..n {
                    o[i] = -(0.5 + i as f64 * 0.1 + model_param) * y[i];
                }
                Ok(())
            })
            .autonomous();
            Ok((
                p,
                (0..n).map(|i| 1.0 + i as f64 * 0.05).collect(),
                (0..n)
                    .map(|i| {
                        (-(0.5 + i as f64 * 0.1 + model_param) * span).exp()
                            * (1.0 + i as f64 * 0.05)
                    })
                    .collect(),
                calls,
            ))
        }
        "time-jvp" => {
            let n = 16;
            let c = calls.clone();
            let cr = calls.clone();
            let p = Problem::new(n, move |t, y, o| {
                rhs(&c);
                for i in 0..n {
                    let a = 1.0 + i as f64 / n as f64;
                    let l = 2.0 + i as f64 / 8.0;
                    let target = (-t).exp() * a;
                    o[i] = -l * (y[i] - target) - target;
                }
                Ok(())
            })
            .jacobian(Jacobian::jvp(move |_t, _y, v, o| {
                jvp(&cr);
                for i in 0..n {
                    o[i] = -(2.0 + i as f64 / 8.0) * v[i];
                }
                Ok(())
            }));
            Ok((
                p,
                (0..n).map(|i| 1.0 + i as f64 / n as f64).collect(),
                (0..n)
                    .map(|i| (-span).exp() * (1.0 + i as f64 / n as f64))
                    .collect(),
                calls,
            ))
        }
        "autonomous-jvp" => {
            let n = 16;
            let c = calls.clone();
            let cr = calls.clone();
            let p = Problem::new(n, move |_t, y, o| {
                rhs(&c);
                for i in 0..n {
                    let j = (i / 2) * 2;
                    if i % 2 == 0 {
                        o[i] = -3.0 * y[i + 1];
                    } else {
                        o[i] = 3.0 * y[j];
                    }
                }
                Ok(())
            })
            .jacobian(Jacobian::jvp(move |_t, _y, v, o| {
                jvp(&cr);
                for i in 0..n {
                    let j = (i / 2) * 2;
                    if i % 2 == 0 {
                        o[i] = -3.0 * v[i + 1];
                    } else {
                        o[i] = 3.0 * v[j];
                    }
                }
                Ok(())
            }))
            .autonomous();
            let y0: Vec<f64> = (0..n)
                .map(|i| {
                    if i % 2 == 0 {
                        1.0 + i as f64 / 32.0
                    } else {
                        0.0
                    }
                })
                .collect();
            let exact: Vec<f64> = (0..n)
                .map(|i| {
                    let (s, co) = (3.0 * span).sin_cos();
                    let amp = 1.0 + ((i / 2) * 2) as f64 / 32.0;
                    if i % 2 == 0 { amp * co } else { amp * s }
                })
                .collect();
            Ok((p, y0, exact, calls))
        }
        _ => Err(format!("unknown case {name}")),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("rust_allocation_probe: {e}");
        std::process::exit(2);
    }
}
fn run() -> Result<(), String> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() == 2 && a[1] == "--help" {
        println!(
            "usage: rust_allocation_probe --case dense-fd|banded-fd|nonautonomous-dense-fd|span-parameter-control|time-jvp|autonomous-jvp [--span T] [--model-param P] [--mode probe|timing|alloc] [--repeats N]"
        );
        return Ok(());
    }
    let get = |k: &str, d: &str| -> String {
        a.windows(2)
            .find(|w| w[0] == k)
            .map(|w| w[1].clone())
            .unwrap_or_else(|| d.into())
    };
    let case = get("--case", "");
    let mode = get("--mode", "probe");
    if !["probe", "timing", "alloc"].contains(&mode.as_str()) {
        return Err("mode must be probe, timing or alloc".into());
    }
    let repeats: usize = get("--repeats", "1").parse().map_err(|_| "bad --repeats")?;
    if repeats == 0 {
        return Err("repeats must be positive".into());
    }
    let setup_start = Instant::now();
    let span: f64 = get("--span", "0.25").parse().map_err(|_| "bad --span")?;
    let model_param: f64 = get("--model-param", "0.2")
        .parse()
        .map_err(|_| "bad --model-param")?;
    if !span.is_finite() || span <= 0.0 || !model_param.is_finite() {
        return Err("span must be positive and both numeric inputs finite".into());
    }
    let (problem, y0, exact, calls) = model(&case, span, model_param)?;
    let opts = Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0: 1e-3,
        ..Options::default()
    };
    let setup_ns = setup_start.elapsed().as_nanos();
    ALLOCS.with(|x| x.set(0));
    let mut samples = Vec::with_capacity(repeats);
    for _ in 0..repeats {
        if mode == "timing" {
            let t0 = Instant::now();
            let sol =
                Rodas5p::solve(&problem, (0.0, span), &y0, &opts).map_err(|e| e.to_string())?;
            let solver_ns = t0.elapsed().as_nanos();
            drop(sol);
            samples
                .push(json!({"solver_ns":solver_ns,"solve_plus_drop_ns":t0.elapsed().as_nanos()}));
            continue;
        }
        if mode == "alloc" {
            ALLOCS.with(|x| x.set(0));
            TRACK.with(|x| x.set(true));
        }
        let sol = Rodas5p::solve(&problem, (0.0, span), &y0, &opts).map_err(|e| e.to_string())?;
        if mode == "alloc" {
            TRACK.with(|x| x.set(false));
        }
        let states = sol.y.clone();
        let times = sol.t.clone();
        let stats = sol.stats.clone();
        let status = sol.status.clone();
        drop(sol);
        let final_state = states.last().ok_or("missing final state")?;
        let err = if exact.is_empty() {
            None
        } else {
            Some(
                final_state
                    .iter()
                    .zip(&exact)
                    .map(|(x, r)| (x - r).abs())
                    .fold(0.0_f64, f64::max),
            )
        };
        let state_bits: Vec<String> = final_state
            .iter()
            .map(|x| format!("{:016x}", x.to_bits()))
            .collect();
        let all_bits: Vec<Vec<String>> = states
            .iter()
            .map(|row| {
                row.iter()
                    .map(|x| format!("{:016x}", x.to_bits()))
                    .collect()
            })
            .collect();
        let stats = serde_json::to_value(&stats).map_err(|e| e.to_string())?;
        let status = serde_json::to_value(&status).map_err(|e| e.to_string())?;
        let obs: Vec<String> = times
            .iter()
            .map(|x| format!("{:016x}", x.to_bits()))
            .collect();
        let alloc_count = if mode == "alloc" {
            ALLOCS.with(Cell::get)
        } else {
            0
        };
        samples.push(json!({"solve_allocations":if mode=="alloc" {Value::from(alloc_count)} else {Value::Null},"state_bits":state_bits,"observation_time_bits":obs,"observation_state_bits":all_bits,"endpoint_max_abs_error":err,"oracle":if !exact.is_empty() {"exact"} else {"not supplied; states exported for paired trajectory comparison"},"status":status,"stats":stats}));
    }
    let out = json!({"schema":"rust-allocation-probe-v1","case":case,"mode":mode,"span":span,"model_param":model_param,"repeats":repeats,"setup_ns":setup_ns,"allocation_tracking":"thread-local flag enabled only around solve; disabled for timing mode","callback_counts":{"rhs":calls.rhs.load(Ordering::Relaxed),"jvp":calls.jvp.load(Ordering::Relaxed),"time_derivative":calls.ft.load(Ordering::Relaxed)},"samples":samples,"setup":"setup_ns includes model/options/initial state construction; timing mode measures facade solve and solve-plus-drop without snapshots; probe/alloc snapshots contain exact outputs, status and complete Stats; JSON serialization and analytic oracle error evaluation are after timing"});
    println!(
        "{}",
        serde_json::to_string(&out).map_err(|e| e.to_string())?
    );
    Ok(())
}
