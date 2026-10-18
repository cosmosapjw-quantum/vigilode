//! Isolated Diffsol 0.16.1 JSON adapter; the root Cargo graph is untouched.
#[path = "../../../../../../crates/rodas5p-integrators/examples/component_benchmark_support/mod.rs"]
mod support;
use diffsol::{
    DenseMatrix, FaerLU, FaerMat, MatrixCommon, OdeBuilder, OdeSolverMethod, VectorHost,
};
use serde_json::{Value, json};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    env, fs,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Instant,
};
use support::{Case, Request};

struct Counting;
static ON: AtomicBool = AtomicBool::new(false);
static COUNT: AtomicU64 = AtomicU64::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if ON.load(Ordering::Relaxed) {
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        if ON.load(Ordering::Relaxed) {
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if ON.load(Ordering::Relaxed) {
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOC: Counting = Counting;

fn empty(status: &str, reason: &str) -> Value {
    json!({"status":status,"reason":reason,"times":[],"states":[],"counters":null,"timing":{"elapsed_ns":null,"timing_authority":"diagnostic_only"},"allocations":{"solve":null,"positive_control":null,"scope":"unmeasured"},"certificate":null})
}

fn solve_raw(case: &Case, id: &str) -> Value {
    if case.mass_matrix.is_some() {
        return empty(
            "UNSUPPORTED_CAPABILITY",
            "mass/DAE equation cannot be represented by this ordinary ODE adapter",
        );
    }
    if !case.supported() {
        return empty(
            "UNSUPPORTED_CAPABILITY",
            "model has no established real-vector Diffsol adapter",
        );
    }
    if !matches!(
        id,
        "diffsol-bdf"
            | "diffsol-trbdf2"
            | "diffsol-tr_bdf2"
            | "diffsol-esdirk34"
            | "diffsol-tsit45"
    ) {
        return empty("UNSUPPORTED_CAPABILITY", "unknown Diffsol method");
    }
    let n = case.initial_state.len();
    let c = case.clone();
    let d = case.clone();
    let init = case.initial_state.clone();
    let span = case.t_span;
    let problem = OdeBuilder::<FaerMat<f64>>::new()
        .p([0.0])
        .rtol(case.rtol)
        .atol(case.atol.clone())
        .h0(((span[1] - span[0]) / 1000.0).max(1e-10))
        .t0(span[0])
        .rhs_implicit(
            move |y, _p, t, out| {
                if c.rhs(t, y.as_slice(), out.as_mut_slice()).is_err() {
                    out.as_mut_slice().fill(f64::NAN);
                }
            },
            move |y, _p, t, v, out| {
                if d.jvp_into(t, y.as_slice(), v.as_slice(), out.as_mut_slice())
                    .is_err()
                {
                    out.as_mut_slice().fill(f64::NAN);
                }
            },
        )
        .init(
            move |_p, _t, y| {
                for i in 0..n {
                    y[i] = init[i];
                }
            },
            n,
        )
        .build();
    let Ok(problem) = problem else {
        return empty("NUMERICAL_FAILURE", "Diffsol problem construction failed");
    };
    macro_rules! run {($solver:expr)=>{{let mut s=match $solver {Ok(v)=>v,Err(e)=>return empty("NUMERICAL_FAILURE",&e.to_string())};let outcome=s.solve_dense(&case.output_times);let stats=serde_json::to_value(s.get_statistics()).unwrap_or(Value::Null);match outcome {Ok((matrix,stop))=>{let cols=matrix.ncols();let mut states=Vec::with_capacity(cols);for j in 0..cols{let mut row=Vec::with_capacity(n);for i in 0..n{row.push(matrix.get_index(i,j));}states.push(row);}let status=if cols==case.output_times.len()&&states.iter().flatten().all(|v|v.is_finite()){ "OK" }else{"NUMERICAL_FAILURE"};json!({"status":status,"reason":format!("{stop:?}"),"times":case.output_times,"states":states,"counters":stats,"timing":{"elapsed_ns":null,"timing_authority":"diagnostic_only"},"allocations":{"solve":null,"positive_control":null,"scope":"unmeasured"},"certificate":null})},Err(e)=>{let mut v=empty("NUMERICAL_FAILURE",&e.to_string());v["counters"]=stats;v}}}};}
    match id {
        "diffsol-bdf" => run!(problem.bdf::<FaerLU<f64>>()),
        "diffsol-trbdf2" | "diffsol-tr_bdf2" => run!(problem.tr_bdf2::<FaerLU<f64>>()),
        "diffsol-esdirk34" => run!(problem.esdirk34::<FaerLU<f64>>()),
        "diffsol-tsit45" => run!(problem.tsit45()),
        _ => unreachable!(),
    }
}
fn solve(case: &Case, id: &str) -> Value {
    let before = case.derivative_rhs_calls();
    let mut value = solve_raw(case, id);
    value["derivative_mode"] = json!(case.derivative_policy());
    value["derivative_work"] = json!({"extra_rhs_calls":case.derivative_rhs_calls()-before,"included_in_solver_counters":false});
    value
}
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 4 && args[1] == "--probe" && args[2] == "--request" {
        let value = match fs::read_to_string(&args[3])
            .ok()
            .and_then(|s| serde_json::from_str::<Request>(&s).ok())
        {
            Some(req) => req.case.probe(&req.probes),
            None => empty("ENVIRONMENT_FAILURE", "probe request read/parse failed"),
        };
        println!("{value}");
        return;
    }
    let mut v = if (args.len() == 3 || (args.len() == 5 && args[3] == "--measurement-mode"))
        && args[1] == "--request"
    {
        let mode = if args.len() == 5 {
            args[4].as_str()
        } else {
            "timing"
        };
        match fs::read_to_string(&args[2])
            .ok()
            .and_then(|s| serde_json::from_str::<Request>(&s).ok())
        {
            Some(req) => {
                if mode == "allocation" {
                    COUNT.store(0, Ordering::SeqCst);
                    ON.store(true, Ordering::SeqCst);
                    let mut v = solve(&req.case, &req.solver_id);
                    ON.store(false, Ordering::SeqCst);
                    let count = COUNT.load(Ordering::SeqCst);
                    let before = COUNT.load(Ordering::SeqCst);
                    ON.store(true, Ordering::SeqCst);
                    let positive = vec![1_u8; 1024];
                    std::hint::black_box(&positive);
                    ON.store(false, Ordering::SeqCst);
                    let control = COUNT.load(Ordering::SeqCst) - before;
                    v["measurement_mode"] = json!("allocation");
                    v["allocations"] = json!({"solve":count,"positive_control":control,"calibrated":control>0,"scope":"entire solve including setup/output; separate process"});
                    v
                } else if mode == "timing" {
                    let warm_start = Instant::now();
                    let warm = solve(&req.case, &req.solver_id);
                    let warm_ns = warm_start.elapsed().as_nanos();
                    if warm["status"] != "OK" {
                        let mut v = warm;
                        v["measurement_mode"] = json!("timing");
                        v["timing_scope"] =
                            json!("warmed_solver_setup_integration_output_certificate");
                        v["timing"] = json!({"elapsed_ns":null,"timed_seconds":null,"warmup_seconds":warm_ns as f64*1e-9,"timing_authority":"diagnostic_only","timing_scope":"warmed_solver_setup_integration_output_certificate"});
                        v
                    } else {
                        let start = Instant::now();
                        let mut v = solve(&req.case, &req.solver_id);
                        let elapsed = start.elapsed().as_nanos();
                        v["measurement_mode"] = json!("timing");
                        v["timing_scope"] =
                            json!("warmed_solver_setup_integration_output_certificate");
                        v["timing"] = json!({"elapsed_ns":elapsed,"total_seconds":elapsed as f64*1e-9,"timed_seconds":elapsed as f64*1e-9,"warmup_seconds":warm_ns as f64*1e-9,"timing_authority":"diagnostic_only","timing_scope":"warmed_solver_setup_integration_output_certificate"});
                        v
                    }
                } else {
                    empty("ENVIRONMENT_FAILURE", "invalid measurement mode")
                }
            }
            None => empty("ENVIRONMENT_FAILURE", "request read/parse failed"),
        }
    } else {
        empty(
            "ENVIRONMENT_FAILURE",
            "usage: component_benchmark --request PATH [--measurement-mode timing|allocation]",
        )
    };
    if !v.is_object() {
        v = empty("ENVIRONMENT_FAILURE", "internal response error");
    }
    println!("{v}");
}
