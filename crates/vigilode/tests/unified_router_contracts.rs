//! Contract tests of the unified router (research loop L1, node V03 part B).
//!
//! The router must report what it executed, keep the work and the stop reason of every leg,
//! and must not take contractivity for non-stiffness. The problems, tolerances and bounds
//! are those of `research/production_loop_20261116/unified_router/PREDECLARED.json`.
//!
//! Part 1 uses only the public API of the base commit `587a1395`: each of its tests was run
//! against the base sources first (`logs/failing_first_base.log`). Part 2 tests the API that
//! this node added.
//!
//! Every test prints one `V03B_RECORD {json}` line with the numbers it measured before it
//! asserts; the receipt is built from those lines.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};
use vigilode::prelude::*;
use vigilode::{Callback, DerivativeFallback, SwitchingDriverStats};

// ------------------------------------------------------------------ counting allocator

struct CountingAlloc;

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static LARGEST: Cell<usize> = const { Cell::new(0) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
}

fn count_alloc(size: usize) {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = LARGEST.try_with(|c| c.set(c.get().max(size)));
        let _ = BYTES.try_with(|c| c.set(c.get() + size));
    }
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_alloc(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_alloc(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_alloc(new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

/// Run `f` and return its value, the largest single allocation and the allocated bytes.
fn allocation_scope<T>(f: impl FnOnce() -> T) -> (T, usize, usize) {
    LARGEST.with(|c| c.set(0));
    BYTES.with(|c| c.set(0));
    TRACK.with(|t| t.set(true));
    let result = f();
    TRACK.with(|t| t.set(false));
    (result, LARGEST.with(Cell::get), BYTES.with(Cell::get))
}

// ---------------------------------------------------------------------------- records

fn record(test: &str, fields: Value) {
    println!("V03B_RECORD {}", json!({ "test": test, "fields": fields }));
}

/// Callback entries counted inside the user closures: the callback boundary.
#[derive(Clone, Default)]
struct Calls {
    rhs: Arc<AtomicU64>,
    jacobian: Arc<AtomicU64>,
    jvp: Arc<AtomicU64>,
    fatal: Arc<AtomicU64>,
}

impl Calls {
    fn rhs(&self) -> u64 {
        self.rhs.load(Ordering::Relaxed)
    }
    fn jacobian(&self) -> u64 {
        self.jacobian.load(Ordering::Relaxed)
    }
    fn jvp(&self) -> u64 {
        self.jvp.load(Ordering::Relaxed)
    }
    fn fatal(&self) -> u64 {
        self.fatal.load(Ordering::Relaxed)
    }
}

fn bits(state: &[f64]) -> Vec<u64> {
    state.iter().map(|v| v.to_bits()).collect()
}

fn labels(solution: &VigilodeSolution) -> Vec<String> {
    solution
        .route_history
        .iter()
        .map(|r| r.to_tier.clone())
        .collect()
}

fn has_explicit_leg(solution: &VigilodeSolution) -> bool {
    labels(solution)
        .iter()
        .any(|label| label.to_ascii_lowercase().contains("dopri5"))
}

// --------------------------------------------------------------------------- problems

fn heat_rhs(n: usize, y: &[f64], dy: &mut [f64]) {
    let inv_dx2 = ((n + 1) as f64).powi(2);
    for i in 0..n {
        let left = if i == 0 { 0.0 } else { y[i - 1] };
        let right = if i + 1 == n { 0.0 } else { y[i + 1] };
        dy[i] = (left - 2.0 * y[i] + right) * inv_dx2;
    }
}

fn heat_sine(n: usize) -> Vec<f64> {
    (1..=n)
        .map(|i| (std::f64::consts::PI * i as f64 / (n + 1) as f64).sin())
        .collect()
}

/// Heat equation without a Jacobian (finite differences).
fn heat_fd(n: usize, calls: &Calls) -> Problem {
    let rhs = calls.rhs.clone();
    Problem::new(n, move |_t, y: &[f64], dy: &mut [f64]| {
        rhs.fetch_add(1, Ordering::Relaxed);
        heat_rhs(n, y, dy);
        Ok::<(), CallbackError>(())
    })
    .autonomous()
}

/// Heat equation with a dense Jacobian callback.
fn heat_dense(n: usize, calls: &Calls) -> Problem {
    let jacobian = calls.jacobian.clone();
    heat_fd(n, calls).jacobian(Jacobian::dense(move |_t, _y, jac: &mut [f64]| {
        jacobian.fetch_add(1, Ordering::Relaxed);
        let inv_dx2 = ((n + 1) as f64).powi(2);
        for i in 0..n {
            jac[i * n + i] = -2.0 * inv_dx2;
            if i > 0 {
                jac[i * n + i - 1] = inv_dx2;
            }
            if i + 1 < n {
                jac[i * n + i + 1] = inv_dx2;
            }
        }
        Ok(())
    }))
}

/// Heat equation with a band Jacobian callback (one sub- and one super-diagonal).
fn heat_band_callback(n: usize, calls: &Calls) -> Problem {
    let jacobian = calls.jacobian.clone();
    heat_fd(n, calls).jacobian(Jacobian::banded(1, 1, move |_t, _y, band: &mut [f64]| {
        jacobian.fetch_add(1, Ordering::Relaxed);
        let inv_dx2 = ((n + 1) as f64).powi(2);
        for i in 0..n {
            band[i * 3] = inv_dx2;
            band[i * 3 + 1] = -2.0 * inv_dx2;
            band[i * 3 + 2] = inv_dx2;
        }
        Ok(())
    }))
}

/// Heat equation with a Jacobian-vector product only.
fn heat_jvp(n: usize, calls: &Calls) -> Problem {
    let jvp = calls.jvp.clone();
    heat_fd(n, calls).jvp(move |_t, _y, v: &[f64], out: &mut [f64]| {
        jvp.fetch_add(1, Ordering::Relaxed);
        heat_rhs(n, v, out);
        Ok(())
    })
}

/// `y' = -1e6 (y - cos t)` without derivatives.
fn scalar_fd(calls: &Calls) -> Problem {
    let rhs = calls.rhs.clone();
    Problem::new(1, move |t: f64, y: &[f64], dy: &mut [f64]| {
        rhs.fetch_add(1, Ordering::Relaxed);
        dy[0] = -1.0e6 * (y[0] - t.cos());
        Ok::<(), CallbackError>(())
    })
}

/// `y' = -1e6 (y - cos t)` with its Jacobian and its time derivative.
fn scalar_dense(calls: &Calls) -> Problem {
    let jacobian = calls.jacobian.clone();
    scalar_fd(calls)
        .jacobian(Jacobian::dense(move |_t, _y, jac: &mut [f64]| {
            jacobian.fetch_add(1, Ordering::Relaxed);
            jac[0] = -1.0e6;
            Ok(())
        }))
        .time_derivative(|t: f64, _y: &[f64], out: &mut [f64]| {
            out[0] = -1.0e6 * t.sin();
            Ok(())
        })
}

fn oscillator(calls: &Calls) -> Problem {
    let rhs = calls.rhs.clone();
    Problem::new(2, move |_t, y: &[f64], dy: &mut [f64]| {
        rhs.fetch_add(1, Ordering::Relaxed);
        dy[0] = y[1];
        dy[1] = -y[0];
        Ok::<(), CallbackError>(())
    })
    .autonomous()
}

fn robertson_rhs(y: &[f64], dy: &mut [f64]) {
    dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
    dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
    dy[2] = 3.0e7 * y[1] * y[1];
}

fn robertson_fd(calls: &Calls) -> Problem {
    let rhs = calls.rhs.clone();
    Problem::new(3, move |_t, y: &[f64], dy: &mut [f64]| {
        rhs.fetch_add(1, Ordering::Relaxed);
        robertson_rhs(y, dy);
        Ok::<(), CallbackError>(())
    })
    .autonomous()
}

fn robertson_dense(calls: &Calls) -> Problem {
    let jacobian = calls.jacobian.clone();
    robertson_fd(calls).jacobian(Jacobian::dense(move |_t, y: &[f64], jac: &mut [f64]| {
        jacobian.fetch_add(1, Ordering::Relaxed);
        jac.copy_from_slice(&[
            -0.04,
            1.0e4 * y[2],
            1.0e4 * y[1],
            0.04,
            -1.0e4 * y[2] - 6.0e7 * y[1],
            -1.0e4 * y[1],
            0.0,
            6.0e7 * y[1],
            0.0,
        ]);
        Ok(())
    }))
}

/// Robertson with the conservation law as an algebraic equation (singular mass matrix).
fn robertson_dae(calls: &Calls) -> Problem {
    let rhs = calls.rhs.clone();
    let jacobian = calls.jacobian.clone();
    Problem::new(3, move |_t, y: &[f64], dy: &mut [f64]| {
        rhs.fetch_add(1, Ordering::Relaxed);
        dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        dy[2] = y[0] + y[1] + y[2] - 1.0;
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::dense(move |_t, y: &[f64], jac: &mut [f64]| {
        jacobian.fetch_add(1, Ordering::Relaxed);
        jac.copy_from_slice(&[
            -0.04,
            1.0e4 * y[2],
            1.0e4 * y[1],
            0.04,
            -1.0e4 * y[2] - 6.0e7 * y[1],
            -1.0e4 * y[1],
            1.0,
            1.0,
            1.0,
        ]);
        Ok(())
    }))
    .mass(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0])
    .autonomous()
}

/// `y' = -y` whose right-hand side fails fatally for `t > 0.3`.
fn fatal_rhs_after(calls: &Calls) -> Problem {
    let rhs = calls.rhs.clone();
    let fatal = calls.fatal.clone();
    Problem::new(1, move |t: f64, y: &[f64], dy: &mut [f64]| {
        rhs.fetch_add(1, Ordering::Relaxed);
        if t > 0.3 {
            fatal.fetch_add(1, Ordering::Relaxed);
            return Err(CallbackError::Fatal("sensor offline".into()));
        }
        dy[0] = -y[0];
        Ok(())
    })
    .autonomous()
}

/// The configuration the router gives the switching adapter.
fn router_switching_config() -> SwitchingConfig {
    SwitchingConfig {
        fallback: DerivativeFallback::AllowFiniteDifference,
        ..SwitchingConfig::default()
    }
}

// ======================================================================================
// Part 1: public API of the base commit
// ======================================================================================

/// One case of "the label is the driver that ran": a problem, how to solve it through the
/// router, and the driver identifier a direct call of the driver reports.
struct LabelCase {
    name: &'static str,
    labels: Vec<String>,
    direct_driver: String,
    success: bool,
}

fn rodas5p_label_case(
    name: &'static str,
    problem: &Problem,
    span: (f64, f64),
    y0: &[f64],
    options: VigilodeOptions,
) -> LabelCase {
    let direct = Rodas5p::solve(problem, span, y0, &options.options).expect("direct RODAS5P call");
    let unified = VigilODE::solve(problem, span, y0, &options).expect("unified solve");
    LabelCase {
        name,
        labels: labels(&unified),
        direct_driver: direct.stats.route.driver_id.to_string(),
        success: unified.success(),
    }
}

fn assert_label_cases(test: &str, cases: &[LabelCase]) {
    let rows: Vec<Value> = cases
        .iter()
        .map(|case| {
            json!({
                "case": case.name,
                "labels": case.labels,
                "direct_driver": case.direct_driver,
                "equal": case.labels == [case.direct_driver.clone()],
                "success": case.success,
            })
        })
        .collect();
    let mismatches: Vec<&str> = cases
        .iter()
        .filter(|case| case.labels != [case.direct_driver.clone()] || !case.success)
        .map(|case| case.name)
        .collect();
    record(
        test,
        json!({ "cases": rows, "mismatches": mismatches.len(), "total": cases.len() }),
    );
    assert!(
        mismatches.is_empty(),
        "label differs from the executed driver in {mismatches:?}"
    );
}

#[test]
fn p1_label_equals_executed_driver_on_the_explicit_route() {
    let calls = Calls::default();
    let problem = oscillator(&calls);
    let span = (0.0, 1.0);
    let y0 = [1.0, 0.0];
    let direct = Dopri5::solve(&problem, span, &y0, &Options::default()).expect("direct DOPRI5");
    let unified = VigilODE::solve(&problem, span, &y0, SolverStrategy::Auto).expect("unified");
    assert_label_cases(
        "p1_label_equals_executed_driver_on_the_explicit_route",
        &[LabelCase {
            name: "oscillator, finite differences, Auto",
            labels: labels(&unified),
            direct_driver: direct.stats.method.to_string(),
            success: unified.success(),
        }],
    );
    assert_eq!(bits(&unified.final_state), bits(&direct.final_state));
}

#[test]
fn p1_label_equals_executed_driver_on_every_rodas5p_driver_family() {
    let calls = Calls::default();
    let auto = VigilodeOptions::default;
    let heat0 = |n: usize| vec![1.0; n];
    let cases = vec![
        rodas5p_label_case(
            "robertson, dense Jacobian, Auto",
            &robertson_dense(&calls),
            (0.0, 1.0),
            &[1.0, 0.0, 0.0],
            auto(),
        ),
        rodas5p_label_case(
            "heat n=16, band Jacobian callback, Auto",
            &heat_band_callback(16, &calls),
            (0.0, 0.01),
            &heat0(16),
            auto(),
        ),
        rodas5p_label_case(
            "heat n=16, finite differences with a declared band, Auto",
            &heat_fd(16, &calls).banded(1, 1),
            (0.0, 0.01),
            &heat0(16),
            auto(),
        ),
        rodas5p_label_case(
            "heat n=12, JVP, Auto",
            &heat_jvp(12, &calls),
            (0.0, 0.01),
            &heat0(12),
            auto(),
        ),
        rodas5p_label_case(
            "heat n=25, JVP, HighPerformance",
            &heat_jvp(25, &calls),
            (0.0, 0.01),
            &heat0(25),
            auto().strategy(SolverStrategy::HighPerformance),
        ),
        rodas5p_label_case(
            "robertson DAE, singular mass matrix, Auto",
            &robertson_dae(&calls),
            (0.0, 1.0),
            &[1.0, 0.0, 0.0],
            auto(),
        ),
        rodas5p_label_case(
            "robertson, dense Jacobian, dense output, Auto",
            &robertson_dense(&calls),
            (0.0, 1.0),
            &[1.0, 0.0, 0.0],
            auto().dense_output(true),
        ),
        rodas5p_label_case(
            "oscillator, finite differences, RobustStiff",
            &oscillator(&calls),
            (0.0, 1.0),
            &[1.0, 0.0],
            auto().strategy(SolverStrategy::RobustStiff),
        ),
        rodas5p_label_case(
            "heat n=16, finite differences without structure, Auto",
            &heat_fd(16, &calls),
            (0.0, 0.01),
            &heat0(16),
            auto(),
        ),
    ];
    assert_label_cases(
        "p1_label_equals_executed_driver_on_every_rodas5p_driver_family",
        &cases,
    );
    let families: std::collections::BTreeSet<&str> =
        cases.iter().map(|c| c.direct_driver.as_str()).collect();
    assert!(
        families.len() >= 5,
        "the cases must cover the driver families of the facade, got {families:?}"
    );
}

#[test]
fn p1_label_equals_executed_driver_on_the_switching_adapter() {
    let calls = Calls::default();
    let problem = oscillator(&calls);
    let span = (0.0, 1.0);
    let y0 = [1.0, 0.0];
    let direct = SwitchingSolver::solve(
        &problem,
        span,
        &y0,
        &Options::default(),
        &router_switching_config(),
    )
    .expect("direct switching call");
    let unified = VigilODE::solve(&problem, span, &y0, SolverStrategy::AdaptiveReentrant)
        .expect("unified solve");
    assert_label_cases(
        "p1_label_equals_executed_driver_on_the_switching_adapter",
        &[LabelCase {
            name: "oscillator, AdaptiveReentrant",
            labels: labels(&unified),
            direct_driver: direct.schema.to_string(),
            success: unified.success(),
        }],
    );
    assert_eq!(bits(&unified.final_state), bits(&direct.final_state));
}

/// Shared body of the stiff contractive cases: the route and the bounded work.
fn assert_not_explicit_and_bounded(
    test: &str,
    problem: &Problem,
    calls: &Calls,
    span: (f64, f64),
    y0: &[f64],
    rho_bound: f64,
) {
    // one tenth of the least work DOPRI5 can do at its stability limit (PREDECLARED.json)
    let bound = (0.1 * 6.0 * rho_bound * (span.1 - span.0) / 3.3).floor() as u64;
    let solved = VigilODE::solve(problem, span, y0, SolverStrategy::Auto);
    let (labels, success, charged) = match &solved {
        Ok(solution) => (
            labels(solution),
            solution.success(),
            solution.stats.counters.rhs_calls,
        ),
        Err(_) => (Vec::new(), false, 0),
    };
    let explicit = solved.as_ref().is_ok_and(has_explicit_leg);
    record(
        test,
        json!({
            "labels": labels,
            "explicit_leg": explicit,
            "success": success,
            "rhs_calls_charged": charged,
            "rhs_entries_at_the_boundary": calls.rhs(),
            "rhs_bound": bound,
            "error": solved.as_ref().err().map(|e| e.to_string()),
        }),
    );
    assert!(success, "the stiff contractive problem must be solved");
    assert!(!explicit, "sent to the explicit route: {labels:?}");
    assert!(
        calls.rhs() <= bound && charged <= bound,
        "right-hand-side work {} (charged {charged}) exceeds the bound {bound}",
        calls.rhs()
    );
}

#[test]
fn p1_stiff_contractive_heat_equation_with_dense_jacobian_is_not_explicit() {
    let calls = Calls::default();
    assert_not_explicit_and_bounded(
        "p1_stiff_contractive_heat_equation_with_dense_jacobian_is_not_explicit",
        &heat_dense(64, &calls),
        &calls,
        (0.0, 10.0),
        &heat_sine(64),
        16900.0,
    );
}

#[test]
fn p1_stiff_contractive_scalar_with_dense_jacobian_is_not_explicit() {
    let calls = Calls::default();
    assert_not_explicit_and_bounded(
        "p1_stiff_contractive_scalar_with_dense_jacobian_is_not_explicit",
        &scalar_dense(&calls),
        &calls,
        (0.0, 1.0),
        &[1.0],
        1.0e6,
    );
}

#[test]
fn p1_stiff_contractive_heat_equation_without_jacobian_is_not_explicit() {
    let calls = Calls::default();
    assert_not_explicit_and_bounded(
        "p1_stiff_contractive_heat_equation_without_jacobian_is_not_explicit",
        &heat_fd(8, &calls),
        &calls,
        (0.0, 100.0),
        &heat_sine(8),
        324.0,
    );
}

#[test]
fn p1_stiff_contractive_scalar_without_jacobian_is_not_explicit() {
    let calls = Calls::default();
    assert_not_explicit_and_bounded(
        "p1_stiff_contractive_scalar_without_jacobian_is_not_explicit",
        &scalar_fd(&calls),
        &calls,
        (0.0, 1.0),
        &[1.0],
        1.0e6,
    );
}

fn status_text(status: &Status) -> String {
    serde_json::to_string(status).expect("a status serializes")
}

#[test]
fn p1_failing_switching_solve_reports_the_fatal_rhs_callback() {
    let test = "p1_failing_switching_solve_reports_the_fatal_rhs_callback";
    let span = (0.0, 1.0);
    let direct_calls = Calls::default();
    let direct = SwitchingSolver::solve(
        &fatal_rhs_after(&direct_calls),
        span,
        &[1.0],
        &Options::default(),
        &router_switching_config(),
    )
    .expect("direct switching call");
    let calls = Calls::default();
    let unified = VigilODE::solve(
        &fatal_rhs_after(&calls),
        span,
        &[1.0],
        SolverStrategy::AdaptiveReentrant,
    )
    .expect("a run that stops is Ok");
    record(
        test,
        json!({
            "direct_success": direct.success,
            "direct_message": direct.message,
            "direct_attempts": direct.stats.attempts,
            "max_steps": Options::default().max_steps,
            "unified_status": status_text(&unified.status),
            "unified_final_time": unified.final_time,
            "direct_final_time": direct.final_time,
            "fatal_entries": calls.fatal(),
        }),
    );
    assert!(!direct.success && direct.stats.attempts < Options::default().max_steps);
    assert!(!unified.success());
    match &unified.status {
        Status::Failed(FailureReason::CallbackFailed {
            callback,
            t,
            message,
        }) => {
            assert_eq!(*callback, Callback::Rhs);
            assert!(*t > 0.3, "the failing call was made at t = {t}");
            assert!(message.contains("sensor offline"), "message: {message}");
        }
        other => panic!("the stop reason is not the callback failure: {other:?}"),
    }
    assert_eq!(unified.final_time.to_bits(), direct.final_time.to_bits());
    assert_eq!(bits(&unified.final_state), bits(&direct.final_state));
}

#[test]
fn p1_failing_switching_solve_reports_the_fatal_jacobian_callback() {
    let test = "p1_failing_switching_solve_reports_the_fatal_jacobian_callback";
    // y' = -1e4 y with a Jacobian callback that always fails: the adapter stops when its
    // first implicit trial asks for the Jacobian.
    let build = || {
        Problem::new(1, |_t, y: &[f64], dy: &mut [f64]| {
            dy[0] = -1.0e4 * y[0];
            Ok::<(), CallbackError>(())
        })
        .jacobian(Jacobian::dense(|_t, _y, _jac: &mut [f64]| {
            Err(CallbackError::Fatal("jacobian table missing".into()))
        }))
        .autonomous()
    };
    let span = (0.0, 1.0);
    let direct = SwitchingSolver::solve(
        &build(),
        span,
        &[1.0],
        &Options::default(),
        &router_switching_config(),
    )
    .expect("direct switching call");
    let unified = VigilODE::solve(&build(), span, &[1.0], SolverStrategy::AdaptiveReentrant)
        .expect("a run that stops is Ok");
    let applicable = !direct.success && direct.stats.attempts < Options::default().max_steps;
    record(
        test,
        json!({
            "applicable": applicable,
            "direct_success": direct.success,
            "direct_message": direct.message,
            "direct_attempts": direct.stats.attempts,
            "unified_status": status_text(&unified.status),
        }),
    );
    assert!(
        applicable,
        "the direct call must stop before the end with attempts left"
    );
    match &unified.status {
        Status::Failed(FailureReason::CallbackFailed {
            callback, message, ..
        }) => {
            assert_eq!(*callback, Callback::Jacobian);
            assert!(
                message.contains("jacobian table missing"),
                "message: {message}"
            );
        }
        other => panic!("the stop reason is not the callback failure: {other:?}"),
    }
}

#[test]
fn p1_failing_switching_solve_with_attempts_left_is_not_reported_as_max_steps() {
    let test = "p1_failing_switching_solve_with_attempts_left_is_not_reported_as_max_steps";
    // y' = y^2, y(0) = 1 blows up at t = 1
    let build = || {
        Problem::new(1, |_t, y: &[f64], dy: &mut [f64]| {
            dy[0] = y[0] * y[0];
            Ok::<(), CallbackError>(())
        })
        .jacobian(Jacobian::dense(|_t, y: &[f64], jac: &mut [f64]| {
            jac[0] = 2.0 * y[0];
            Ok(())
        }))
        .autonomous()
    };
    let span = (0.0, 2.0);
    let options = Options::default();
    let direct =
        SwitchingSolver::solve(&build(), span, &[1.0], &options, &router_switching_config())
            .expect("direct switching call");
    let unified = VigilODE::solve(&build(), span, &[1.0], SolverStrategy::AdaptiveReentrant)
        .expect("a run that stops is Ok");
    let applicable = !direct.success && direct.stats.attempts < options.max_steps;
    record(
        test,
        json!({
            "applicable": applicable,
            "direct_success": direct.success,
            "direct_message": direct.message,
            "direct_attempts": direct.stats.attempts,
            "max_steps": options.max_steps,
            "direct_final_time": direct.final_time,
            "unified_status": status_text(&unified.status),
        }),
    );
    if !applicable {
        // PREDECLARED.json: the case is used only if the direct call stops with attempts left
        return;
    }
    assert!(!unified.success());
    assert!(
        !matches!(
            unified.status,
            Status::Failed(FailureReason::MaxStepsReached { .. })
        ),
        "attempts were left ({} of {}), yet the status is {:?}",
        direct.stats.attempts,
        options.max_steps,
        unified.status
    );
}

/// Uncharged callback entries of one run: entries at the boundary minus charged counters.
fn uncharged(entries: u64, charged: u64) -> i64 {
    entries as i64 - charged as i64
}

#[test]
fn p1_preflight_callbacks_are_charged_with_a_dense_jacobian() {
    let test = "p1_preflight_callbacks_are_charged_with_a_dense_jacobian";
    let span = (0.0, 1.0);
    let y0 = [1.0, 0.0, 0.0];
    let direct_calls = Calls::default();
    let direct = Rodas5p::solve(
        &robertson_dense(&direct_calls),
        span,
        &y0,
        &Options::default(),
    )
    .expect("direct RODAS5P call");
    let calls = Calls::default();
    let unified = VigilODE::solve(&robertson_dense(&calls), span, &y0, SolverStrategy::Auto)
        .expect("unified solve");
    let direct_uncharged = (
        uncharged(direct_calls.rhs(), direct.stats.counters.rhs_calls),
        uncharged(
            direct_calls.jacobian(),
            direct.stats.counters.jacobian_builds,
        ),
    );
    let unified_uncharged = (
        uncharged(calls.rhs(), unified.stats.counters.rhs_calls),
        uncharged(calls.jacobian(), unified.stats.counters.jacobian_builds),
    );
    record(
        test,
        json!({
            "direct": { "rhs_entries": direct_calls.rhs(), "rhs_charged": direct.stats.counters.rhs_calls,
                        "jacobian_entries": direct_calls.jacobian(), "jacobian_charged": direct.stats.counters.jacobian_builds },
            "unified": { "rhs_entries": calls.rhs(), "rhs_charged": unified.stats.counters.rhs_calls,
                         "jacobian_entries": calls.jacobian(), "jacobian_charged": unified.stats.counters.jacobian_builds },
            "direct_uncharged_rhs_jacobian": [direct_uncharged.0, direct_uncharged.1],
            "unified_uncharged_rhs_jacobian": [unified_uncharged.0, unified_uncharged.1],
        }),
    );
    assert_eq!(
        unified_uncharged, direct_uncharged,
        "the router made callbacks that are not in the counters"
    );
}

#[test]
fn p1_preflight_callbacks_are_charged_without_a_jacobian() {
    let test = "p1_preflight_callbacks_are_charged_without_a_jacobian";
    let span = (0.0, 1.0);
    let y0 = [1.0, 0.0];
    let direct_calls = Calls::default();
    let direct = Dopri5::solve(&oscillator(&direct_calls), span, &y0, &Options::default())
        .expect("direct DOPRI5 call");
    let calls = Calls::default();
    let unified = VigilODE::solve(&oscillator(&calls), span, &y0, SolverStrategy::Auto)
        .expect("unified solve");
    let direct_uncharged = uncharged(direct_calls.rhs(), direct.stats.counters.rhs_calls);
    let unified_uncharged = uncharged(calls.rhs(), unified.stats.counters.rhs_calls);
    record(
        test,
        json!({
            "direct": { "rhs_entries": direct_calls.rhs(), "rhs_charged": direct.stats.counters.rhs_calls },
            "unified": { "rhs_entries": calls.rhs(), "rhs_charged": unified.stats.counters.rhs_calls },
            "direct_uncharged_rhs": direct_uncharged,
            "unified_uncharged_rhs": unified_uncharged,
            "preflight_rhs_entries": calls.rhs() - direct_calls.rhs(),
        }),
    );
    assert!(unified.success() && direct.success);
    assert_eq!(
        unified_uncharged, direct_uncharged,
        "the router made right-hand-side calls that are not in the counters"
    );
}

#[test]
fn p1_preflight_callbacks_are_charged_with_a_jvp() {
    let test = "p1_preflight_callbacks_are_charged_with_a_jvp";
    let n = 25;
    let span = (0.0, 0.01);
    let y0 = vec![1.0; n];
    let direct_calls = Calls::default();
    let direct = Rodas5p::solve(&heat_jvp(n, &direct_calls), span, &y0, &Options::default())
        .expect("direct RODAS5P call");
    let calls = Calls::default();
    let unified = VigilODE::solve(&heat_jvp(n, &calls), span, &y0, SolverStrategy::Auto)
        .expect("unified solve");
    let direct_uncharged = uncharged(direct_calls.rhs(), direct.stats.counters.rhs_calls);
    let unified_uncharged = uncharged(calls.rhs(), unified.stats.counters.rhs_calls);
    record(
        test,
        json!({
            "direct": { "rhs_entries": direct_calls.rhs(), "rhs_charged": direct.stats.counters.rhs_calls,
                        "jvp_entries": direct_calls.jvp(), "jvp_charged": direct.stats.counters.jvp_vectors },
            "unified": { "rhs_entries": calls.rhs(), "rhs_charged": unified.stats.counters.rhs_calls,
                         "jvp_entries": calls.jvp(), "jvp_charged": unified.stats.counters.jvp_vectors },
            "direct_uncharged_rhs": direct_uncharged,
            "unified_uncharged_rhs": unified_uncharged,
        }),
    );
    assert_eq!(
        unified_uncharged, direct_uncharged,
        "the router made right-hand-side calls that are not in the counters"
    );
    assert_eq!(calls.jvp(), direct_calls.jvp());
}

#[test]
fn p1_preflight_callback_failure_is_a_typed_stop_not_invalid_input() {
    let test = "p1_preflight_callback_failure_is_a_typed_stop_not_invalid_input";
    let entries = Arc::new(AtomicU64::new(0));
    let counter = entries.clone();
    let problem = Problem::new(1, move |_t, _y: &[f64], _dy: &mut [f64]| {
        counter.fetch_add(1, Ordering::Relaxed);
        Err::<(), CallbackError>(CallbackError::Fatal("model not loaded".into()))
    })
    .autonomous();
    let solved = VigilODE::solve(&problem, (0.0, 1.0), &[1.0], SolverStrategy::Auto);
    let description = match &solved {
        Ok(solution) => format!("Ok: {}", status_text(&solution.status)),
        Err(error) => format!("Err: {error:?}"),
    };
    record(
        test,
        json!({
            "outcome": description,
            "rhs_entries": entries.load(Ordering::Relaxed),
            "rhs_charged": solved.as_ref().ok().map(|s| s.stats.counters.rhs_calls),
        }),
    );
    let solution = solved.expect("a callback failure is a stopped run, not a refused input");
    match &solution.status {
        Status::Failed(FailureReason::CallbackFailed {
            callback, message, ..
        }) => {
            assert_eq!(*callback, Callback::Rhs);
            assert!(message.contains("model not loaded"), "message: {message}");
        }
        other => panic!("the stop reason is not the callback failure: {other:?}"),
    }
    assert_eq!(solution.final_time, 0.0);
    assert_eq!(solution.final_state, vec![1.0]);
    assert_eq!(
        solution.stats.counters.rhs_calls,
        entries.load(Ordering::Relaxed),
        "the failing call is charged"
    );
    assert_eq!(
        entries.load(Ordering::Relaxed),
        1,
        "a fatal callback is not called again"
    );
}

#[test]
fn p1_fatal_callback_on_the_explicit_leg_is_not_retried_by_another_method() {
    let test = "p1_fatal_callback_on_the_explicit_leg_is_not_retried_by_another_method";
    let calls = Calls::default();
    let unified = VigilODE::solve(
        &fatal_rhs_after(&calls),
        (0.0, 1.0),
        &[1.0],
        SolverStrategy::Auto,
    )
    .expect("a run that stops is Ok");
    record(
        test,
        json!({
            "labels": labels(&unified),
            "status": status_text(&unified.status),
            "fatal_entries": calls.fatal(),
            "rhs_entries": calls.rhs(),
            "rhs_charged": unified.stats.counters.rhs_calls,
            "final_time": unified.final_time,
        }),
    );
    assert!(!unified.success());
    assert_eq!(
        calls.fatal(),
        1,
        "the callback that reported a fatal error was entered again"
    );
    match &unified.status {
        Status::Failed(FailureReason::CallbackFailed {
            callback, message, ..
        }) => {
            assert_eq!(*callback, Callback::Rhs);
            assert!(message.contains("sensor offline"), "message: {message}");
        }
        other => panic!("the stop reason is not the callback failure: {other:?}"),
    }
    assert!(
        unified.final_time > 0.0 && unified.final_time <= 0.3,
        "the accepted prefix ends at {}",
        unified.final_time
    );
    assert_eq!(unified.stats.counters.rhs_calls, calls.rhs());
}

#[test]
fn p1_fallback_shares_one_attempt_budget_and_keeps_the_work_of_the_failed_leg() {
    let test = "p1_fallback_shares_one_attempt_budget_and_keeps_the_work_of_the_failed_leg";
    let max_steps = 500usize;
    let calls = Calls::default();
    let options = VigilodeOptions::default().max_steps(max_steps);
    let unified = VigilODE::solve(
        &robertson_fd(&calls),
        (0.0, 10.0),
        &[1.0, 0.0, 0.0],
        &options,
    )
    .expect("unified solve");
    let labels = labels(&unified);
    let lower: Vec<String> = labels.iter().map(|l| l.to_ascii_lowercase()).collect();
    let explicit_at = lower.iter().position(|l| l.contains("dopri5"));
    let implicit_at = lower.iter().position(|l| l.contains("rodas5p"));
    let counters = &unified.stats.counters;
    let attempts_in_counters = counters.accepted_steps + counters.rejected_steps;
    record(
        test,
        json!({
            "labels": labels,
            "success": unified.success(),
            "max_steps": max_steps,
            "accepted_plus_rejected_in_counters": attempts_in_counters,
            "stats_attempts": unified.stats.attempts,
            "stats_accepted_steps": unified.stats.accepted_steps,
            "stats_rejected_steps": unified.stats.rejected_steps,
            "rhs_entries": calls.rhs(),
            "rhs_charged": counters.rhs_calls,
            "fallback_steps": counters.fallback_steps,
        }),
    );
    assert!(unified.success(), "the fallback must complete the run");
    assert!(
        matches!((explicit_at, implicit_at), (Some(e), Some(i)) if e < i),
        "a RODAS5P leg must follow a DOPRI5 leg: {labels:?}"
    );
    assert!(
        attempts_in_counters <= max_steps as u64,
        "{attempts_in_counters} attempts were made under a budget of {max_steps}"
    );
    assert_eq!(
        unified.stats.attempts as u64, attempts_in_counters,
        "Stats::attempts must cover every leg"
    );
    assert_eq!(
        (unified.stats.accepted_steps + unified.stats.rejected_steps) as u64,
        attempts_in_counters
    );
    assert_eq!(
        counters.rhs_calls,
        calls.rhs(),
        "right-hand-side calls of every leg and of the preflight are charged"
    );
    assert!(counters.fallback_steps > 0);
}

#[test]
fn p1_switching_leg_keeps_the_full_work_of_its_segments() {
    let test = "p1_switching_leg_keeps_the_full_work_of_its_segments";
    let span = (0.0, 40.0);
    let y0 = [1.0, 0.0, 0.0];
    let direct_calls = Calls::default();
    let direct = SwitchingSolver::solve(
        &robertson_dense(&direct_calls),
        span,
        &y0,
        &Options::default(),
        &router_switching_config(),
    )
    .expect("direct switching call");
    let calls = Calls::default();
    let unified = VigilODE::solve(
        &robertson_dense(&calls),
        span,
        &y0,
        SolverStrategy::AdaptiveReentrant,
    )
    .expect("unified solve");
    let mut factorizations = 0u64;
    let mut direct_solves = 0u64;
    for segment in &direct.stats.segments {
        if let SwitchingDriverStats::Rodas5p(stats) = &segment.driver {
            factorizations += stats.counters.direct_factorizations;
            direct_solves += stats.counters.direct_solve_calls;
        }
    }
    let counters = &unified.stats.counters;
    record(
        test,
        json!({
            "direct": { "success": direct.success, "attempts": direct.stats.attempts,
                        "accepted_steps": direct.stats.accepted_steps, "rejected_steps": direct.stats.rejected_steps,
                        "rhs_calls": direct.stats.rhs_calls, "jacobian_builds": direct.stats.jacobian_builds,
                        "linear_solves": direct.stats.linear_solves, "segments": direct.stats.segments.len(),
                        "segment_factorizations": factorizations, "segment_direct_solves": direct_solves,
                        "rhs_entries": direct_calls.rhs(), "jacobian_entries": direct_calls.jacobian() },
            "unified": { "success": unified.success(), "stats_attempts": unified.stats.attempts,
                         "stats_accepted_steps": unified.stats.accepted_steps, "stats_rejected_steps": unified.stats.rejected_steps,
                         "rhs_calls": counters.rhs_calls, "jacobian_builds": counters.jacobian_builds,
                         "linear_solves": counters.linear_solves,
                         "direct_factorizations": counters.direct_factorizations,
                         "direct_solve_calls": counters.direct_solve_calls,
                         "rhs_entries": calls.rhs(), "jacobian_entries": calls.jacobian() },
        }),
    );
    assert!(direct.success && unified.success());
    assert!(
        factorizations > 0,
        "the case needs an implicit segment to say anything"
    );
    assert_eq!(bits(&unified.final_state), bits(&direct.final_state));
    assert_eq!(
        (calls.rhs(), calls.jacobian()),
        (direct_calls.rhs(), direct_calls.jacobian())
    );
    assert_eq!(counters.rhs_calls, direct.stats.rhs_calls);
    assert_eq!(counters.jacobian_builds, direct.stats.jacobian_builds);
    assert_eq!(counters.linear_solves, direct.stats.linear_solves);
    assert_eq!(
        counters.direct_factorizations, factorizations,
        "factorizations of the segments were dropped"
    );
    assert_eq!(counters.direct_solve_calls, direct_solves);
    assert_eq!(unified.stats.attempts, direct.stats.attempts);
    assert_eq!(unified.stats.accepted_steps, direct.stats.accepted_steps);
    assert_eq!(unified.stats.rejected_steps, direct.stats.rejected_steps);
}

#[test]
fn p1_rodas5p_leg_is_the_direct_call_bit_for_bit() {
    let test = "p1_rodas5p_leg_is_the_direct_call_bit_for_bit";
    let span = (0.0, 40.0);
    let y0 = [1.0, 0.0, 0.0];
    let direct_calls = Calls::default();
    let direct = Rodas5p::solve(
        &robertson_dense(&direct_calls),
        span,
        &y0,
        &Options::default(),
    )
    .expect("direct RODAS5P call");
    let calls = Calls::default();
    let unified = VigilODE::solve(&robertson_dense(&calls), span, &y0, SolverStrategy::Auto)
        .expect("unified solve");
    let equal_state = bits(&unified.final_state) == bits(direct.final_state().unwrap());
    let equal_counters = serde_json::to_value(unified.stats.counters).unwrap()
        == serde_json::to_value(direct.stats.counters).unwrap();
    record(
        test,
        json!({
            "final_state_bits_equal": equal_state,
            "counters_equal": equal_counters,
            "attempts": [unified.stats.attempts, direct.stats.attempts],
            "rhs_entries": [calls.rhs(), direct_calls.rhs()],
        }),
    );
    assert!(equal_state && equal_counters);
    assert_eq!(unified.stats.attempts, direct.stats.attempts);
    assert_eq!(unified.stats.accepted_steps, direct.stats.accepted_steps);
    assert_eq!(calls.rhs(), direct_calls.rhs());
}

#[test]
fn p1_workspace_storage_is_linear_in_the_dimension() {
    let test = "p1_workspace_storage_is_linear_in_the_dimension";
    let n = 2000usize;
    let workspace = VigilodeWorkspace::new(n, 30);
    let retained = workspace.retained_bytes();
    // 64 doubles per unknown is far above any O(n) need and far below n doubles per unknown
    let bound = 64 * n * 8;
    record(
        test,
        json!({ "dimension": n, "retained_bytes": retained, "bound_bytes": bound, "dense_matrix_bytes": n * n * 8 }),
    );
    assert!(
        retained <= bound,
        "the workspace retains {retained} bytes for n = {n}: an n-by-n buffer is {} bytes",
        n * n * 8
    );
}

#[test]
fn p1_matrix_free_solve_allocates_no_dense_matrix() {
    let test = "p1_matrix_free_solve_allocates_no_dense_matrix";
    let n = 3000usize;
    let calls = Calls::default();
    let problem = heat_jvp(n, &calls);
    let y0 = vec![1.0; n];
    let (solved, largest, bytes) =
        allocation_scope(|| VigilODE::solve(&problem, (0.0, 1.0e-7), &y0, SolverStrategy::Auto));
    let unified = solved.expect("unified solve");
    let dense = n * n * 8;
    record(
        test,
        json!({
            "dimension": n,
            "largest_single_allocation_bytes": largest,
            "allocated_bytes": bytes,
            "dense_matrix_bytes": dense,
            "labels": labels(&unified),
            "success": unified.success(),
        }),
    );
    assert!(unified.success());
    assert!(
        largest < dense,
        "a single allocation of {largest} bytes was made on a matrix-free route (n*n*8 = {dense})"
    );
}

// ======================================================================================
// Part 2: the API that node V03 part B added
// ======================================================================================

mod part2 {
    use super::*;
    use rodas5p_integrators::Dopri5StopReason;
    use vigilode::coloring::SparsityPattern;
    use vigilode::{
        EXPLICIT_TRIAL_ATTEMPT_LIMIT, EXPLICIT_TRIAL_MAX_DIMENSION, FallbackEngine, GeneralOde,
        GeneralOdePolicy, MethodChoice, MethodRegistry, PreflightAnalyzer, RouteBasis, StopReason,
        UnifiedStop, Unsupported,
    };

    type Build = Box<dyn Fn(&Calls) -> Problem>;

    /// Heat equation on a ring (periodic), so that the Jacobian pattern is not banded.
    fn ring_rhs(n: usize, y: &[f64], dy: &mut [f64]) {
        let inv_dx2 = ((n + 1) as f64).powi(2);
        for i in 0..n {
            dy[i] = (y[(i + n - 1) % n] - 2.0 * y[i] + y[(i + 1) % n]) * inv_dx2;
        }
    }

    /// Sparse JVP provider: the tridiagonal heat operator (a banded pattern), or the ring
    /// (a general pattern).
    fn heat_sparse_jvp(n: usize, ring: bool, calls: &Calls) -> Problem {
        let rows: Vec<Vec<usize>> = (0..n)
            .map(|i| {
                let mut row = vec![i];
                if ring {
                    row.push((i + n - 1) % n);
                    row.push((i + 1) % n);
                } else {
                    if i > 0 {
                        row.push(i - 1);
                    }
                    if i + 1 < n {
                        row.push(i + 1);
                    }
                }
                row.sort_unstable();
                row.dedup();
                row
            })
            .collect();
        let pattern = SparsityPattern::from_rows(n, &rows).expect("a valid pattern");
        assert_eq!(pattern.is_banded(), !ring);
        let (rhs, jvp) = (calls.rhs.clone(), calls.jvp.clone());
        Problem::new(n, move |_t, y: &[f64], dy: &mut [f64]| {
            rhs.fetch_add(1, Ordering::Relaxed);
            if ring {
                ring_rhs(n, y, dy);
            } else {
                heat_rhs(n, y, dy);
            }
            Ok::<(), CallbackError>(())
        })
        .jacobian(Jacobian::sparse_jvp(
            pattern,
            move |_t, _y, v: &[f64], out: &mut [f64]| {
                jvp.fetch_add(1, Ordering::Relaxed);
                if ring {
                    ring_rhs(n, v, out);
                } else {
                    heat_rhs(n, v, out);
                }
                Ok(())
            },
        ))
        .autonomous()
    }

    /// Replay the legs that a solution reports with direct calls of the entry points it
    /// names: the final state, the callback entries and the charged right-hand sides.
    fn replay(
        build: &dyn Fn(&Calls) -> Problem,
        y0: &[f64],
        options: &VigilodeOptions,
        solution: &VigilodeSolution,
    ) -> (Vec<u64>, Calls, u64) {
        let calls = Calls::default();
        let problem = build(&calls);
        let mut state = y0.to_vec();
        let mut charged = 0u64;
        for leg in &solution.legs {
            let mut leg_options = options.options.clone();
            leg_options.max_steps = leg.attempt_budget;
            let span = (leg.t_start, leg.target);
            state = match leg.tier {
                SelectedTier::Tier1Explicit => {
                    let direct = Dopri5::solve(&problem, span, &state, &leg_options).unwrap();
                    assert_eq!(direct.stats.method, leg.driver_id);
                    charged += direct.stats.counters.rhs_calls;
                    direct.final_state
                }
                SelectedTier::TierProtectedReference => {
                    let direct = Rodas5p::solve(&problem, span, &state, &leg_options).unwrap();
                    assert_eq!(direct.stats.route.driver_id, leg.driver_id);
                    charged += direct.stats.counters.rhs_calls;
                    direct.final_state().unwrap().to_vec()
                }
                SelectedTier::Tier5AdaptiveSwitching => {
                    let config = router_switching_config();
                    let direct =
                        SwitchingSolver::solve(&problem, span, &state, &leg_options, &config)
                            .unwrap();
                    assert_eq!(direct.schema, leg.driver_id);
                    charged += direct.stats.rhs_calls;
                    direct.final_state
                }
            };
        }
        (bits(&state), calls, charged)
    }

    /// Prefix of the driver identifier that each value of `SelectedTier` runs. The match
    /// has no wildcard arm: a value added to the enum does not compile here until it is
    /// given a driver.
    fn driver_prefix(tier: SelectedTier) -> &'static str {
        match tier {
            SelectedTier::Tier1Explicit => "dopri5",
            SelectedTier::TierProtectedReference => "rodas5p",
            SelectedTier::Tier5AdaptiveSwitching => "vigilode.switching",
        }
    }

    #[test]
    fn p2_every_tier_has_a_driver_and_every_report_replays_as_direct_calls() {
        let test = "p2_every_tier_has_a_driver_and_every_report_replays_as_direct_calls";
        assert_eq!(SelectedTier::ALL.len(), 3);
        let providers: Vec<(&str, fn(usize) -> Build)> = vec![
            ("finite-difference", |n| Box::new(move |c| heat_fd(n, c))),
            ("fd-declared-band", |n| {
                Box::new(move |c| heat_fd(n, c).banded(1, 1))
            }),
            ("dense", |n| Box::new(move |c| heat_dense(n, c))),
            ("band-callback", |n| {
                Box::new(move |c| heat_band_callback(n, c))
            }),
            ("jvp", |n| Box::new(move |c| heat_jvp(n, c))),
            ("sparse-jvp-banded", |n| {
                Box::new(move |c| heat_sparse_jvp(n, false, c))
            }),
            ("sparse-jvp-general", |n| {
                Box::new(move |c| heat_sparse_jvp(n, true, c))
            }),
        ];
        let strategies = [
            SolverStrategy::Auto,
            SolverStrategy::HighPerformance,
            SolverStrategy::RobustStiff,
            SolverStrategy::AdaptiveReentrant,
        ];
        let regimes = [
            ProblemRegime::Unknown,
            ProblemRegime::Nonstiff,
            ProblemRegime::Stiff,
        ];
        let span = (0.0, 0.01);
        let mut tiers = std::collections::BTreeMap::<String, usize>::new();
        let mut drivers = std::collections::BTreeMap::<String, usize>::new();
        let mut routes = std::collections::BTreeMap::<String, usize>::new();
        let (mut cases, mut solved, mut refused, mut screened) = (0, 0, 0, 0);
        let mut failures: Vec<String> = Vec::new();
        for n in [6usize, 12] {
            let y0 = heat_sine(n);
            for (provider, build) in &providers {
                let build = build(n);
                for strategy in strategies {
                    for regime in regimes {
                        for dense_output in [false, true] {
                            cases += 1;
                            let name = format!(
                                "n={n} {provider} {strategy:?} {regime:?} dense_output={dense_output}"
                            );
                            let options = VigilodeOptions::default()
                                .strategy(strategy)
                                .regime(regime)
                                .dense_output(dense_output);
                            let calls = Calls::default();
                            let problem = build(&calls);
                            let solution = match VigilODE::solve(&problem, span, &y0, &options) {
                                Ok(solution) => solution,
                                Err(error) => {
                                    // a refusal precedes every callback, and the RODAS5P
                                    // facade refuses the same request
                                    refused += 1;
                                    let entries = calls.rhs() + calls.jacobian() + calls.jvp();
                                    let facade =
                                        Rodas5p::solve(&problem, span, &y0, &options.options);
                                    if entries != 0 || facade.is_ok() {
                                        failures.push(format!("{name}: refused ({error}) after {entries} callbacks, facade ok = {}", facade.is_ok()));
                                    }
                                    continue;
                                }
                            };
                            solved += 1;
                            let analysis = &solution.analysis;
                            screened += usize::from(analysis.screen.is_some());
                            *tiers
                                .entry(format!("{:?}", analysis.suggested_tier))
                                .or_default() += 1;
                            let mut problems = Vec::new();
                            if !solution.success() {
                                problems.push("not solved".to_owned());
                            }
                            if solution.legs.len() != solution.route_history.len() {
                                problems.push("history and legs differ in length".to_owned());
                            }
                            for (leg, entry) in solution.legs.iter().zip(&solution.route_history) {
                                *drivers.entry(leg.driver_id.clone()).or_default() += 1;
                                *routes.entry(leg.route_id.clone()).or_default() += 1;
                                if entry.to_tier != leg.driver_id {
                                    problems.push(format!(
                                        "label {} is not the driver {}",
                                        entry.to_tier, leg.driver_id
                                    ));
                                }
                                if !leg.driver_id.starts_with(driver_prefix(leg.tier)) {
                                    problems.push(format!(
                                        "driver {} under tier {:?}",
                                        leg.driver_id, leg.tier
                                    ));
                                }
                            }
                            // the plan of the analysis is the route of the first leg, unless
                            // the switching adapter refused and RODAS5P ran in its place
                            let first = &solution.legs[0];
                            if first.tier == analysis.suggested_tier {
                                if let Some(plan) = &analysis.planned_route
                                    && plan.id() != first.route_id
                                {
                                    problems.push(format!(
                                        "planned {} ran {}",
                                        plan.id(),
                                        first.route_id
                                    ));
                                }
                            } else if !(analysis.suggested_tier
                                == SelectedTier::Tier5AdaptiveSwitching
                                && dense_output
                                && solution.route_history[0].reason.contains("refused"))
                            {
                                problems.push(format!(
                                    "first leg {:?} under tier {:?}",
                                    first.tier, analysis.suggested_tier
                                ));
                            }
                            let (replayed, direct_calls, direct_charged) =
                                replay(&*build, &y0, &options, &solution);
                            if replayed != bits(&solution.final_state) {
                                problems.push(
                                    "the replay of the reported legs gives another state"
                                        .to_owned(),
                                );
                            }
                            if (calls.rhs(), calls.jacobian(), calls.jvp())
                                != (
                                    direct_calls.rhs() + analysis.rhs_calls,
                                    direct_calls.jacobian(),
                                    direct_calls.jvp(),
                                )
                            {
                                problems.push(format!(
                                    "callback entries {:?} differ from the replay {:?} plus {} preflight calls",
                                    (calls.rhs(), calls.jacobian(), calls.jvp()),
                                    (direct_calls.rhs(), direct_calls.jacobian(), direct_calls.jvp()),
                                    analysis.rhs_calls
                                ));
                            }
                            if solution.stats.counters.rhs_calls
                                != direct_charged + analysis.rhs_calls
                            {
                                problems.push(format!(
                                    "charged rhs {} is not the replay's {} plus {} preflight calls",
                                    solution.stats.counters.rhs_calls,
                                    direct_charged,
                                    analysis.rhs_calls
                                ));
                            }
                            if !problems.is_empty() {
                                failures.push(format!("{name}: {problems:?}"));
                            }
                        }
                    }
                }
            }
        }
        record(
            test,
            json!({
                "cases": cases, "solved": solved, "refused_before_any_callback": refused,
                "with_stiffness_screen": screened, "failures": failures.len(),
                "tiers": tiers, "drivers": drivers, "routes": routes,
                "first_failures": failures.iter().take(5).collect::<Vec<_>>(),
            }),
        );
        assert!(
            failures.is_empty(),
            "{} of {cases} cases: {failures:#?}",
            failures.len()
        );
        assert_eq!(tiers.len(), 3, "every tier must be produced: {tiers:?}");
        assert!(drivers.len() >= 6, "driver families covered: {drivers:?}");
    }

    #[test]
    fn p2_typed_stop_of_a_rodas5p_leg_and_its_prefix() {
        let test = "p2_typed_stop_of_a_rodas5p_leg_and_its_prefix";
        let calls = Calls::default();
        let problem = robertson_dense(&calls);
        let options = VigilodeOptions::default().max_steps(5);
        let span = (0.0, 40.0);
        let y0 = [1.0, 0.0, 0.0];
        let direct = Rodas5p::solve(&problem, span, &y0, &options.options).unwrap();
        let unified = VigilODE::solve(&problem, span, &y0, &options).unwrap();
        let expected = FailureReason::MaxStepsReached { max_steps: 5 };
        record(
            test,
            json!({
                "stop": serde_json::to_value(&unified.stop).unwrap(),
                "status": status_text(&unified.status),
                "final_time": unified.final_time,
                "direct_final_time": direct.t.last(),
                "attempts": unified.stats.attempts,
            }),
        );
        assert!(!unified.success());
        assert_eq!(
            unified.stop,
            Some(UnifiedStop::Rodas5p {
                reason: expected.clone()
            })
        );
        assert!(unified.stop.as_ref().unwrap().is_exact());
        assert_eq!(unified.status, Status::Failed(expected.clone()));
        assert_eq!(unified.status, direct.status);
        assert!(unified.final_time > 0.0 && unified.final_time < 40.0);
        assert_eq!(
            unified.final_time.to_bits(),
            direct.t.last().unwrap().to_bits()
        );
        assert_eq!(
            bits(&unified.final_state),
            bits(direct.final_state().unwrap())
        );
        assert_eq!(unified.stats.attempts, 5);
        // the typed error keeps the reason and the accepted prefix
        let final_time = unified.final_time;
        match unified.ensure_success() {
            Err(SolveError::Failed { reason, partial }) => {
                assert_eq!(reason, expected);
                assert_eq!(*partial.t.last().unwrap(), final_time);
                assert_eq!(partial.stats.attempts, 5);
            }
            other => panic!("expected the typed failure, got {other:?}"),
        }
    }

    #[test]
    fn p2_declared_nonstiff_runs_dopri5_with_the_whole_budget_and_no_second_method() {
        let test = "p2_declared_nonstiff_runs_dopri5_with_the_whole_budget_and_no_second_method";
        let calls = Calls::default();
        let problem = scalar_dense(&calls);
        let options = VigilodeOptions::default()
            .regime(ProblemRegime::Nonstiff)
            .max_steps(50);
        let unified = VigilODE::solve(&problem, (0.0, 1.0), &[1.0], &options).unwrap();
        record(
            test,
            json!({
                "drivers": unified.executed_drivers(),
                "stop": serde_json::to_value(&unified.stop).unwrap(),
                "status": status_text(&unified.status),
                "basis": serde_json::to_value(&unified.analysis.basis).unwrap(),
                "final_time": unified.final_time,
                "attempts": unified.stats.attempts,
                "jacobian_entries": calls.jacobian(),
            }),
        );
        assert_eq!(unified.executed_drivers(), ["dopri5-5(4)-fsal"]);
        assert_eq!(
            unified.analysis.basis,
            RouteBasis::DeclaredRegime {
                regime: ProblemRegime::Nonstiff
            }
        );
        assert!(!unified.analysis.fallback_to_rodas5p);
        assert!(unified.analysis.screen.is_none() && unified.analysis.rhs_calls == 0);
        match &unified.stop {
            Some(UnifiedStop::Dopri5 {
                reason, callback, ..
            }) => {
                assert_eq!(*reason, Some(Dopri5StopReason::AttemptBudget));
                assert!(callback.is_none());
            }
            other => panic!("expected the DOPRI5 budget stop, got {other:?}"),
        }
        assert_eq!(
            unified.status,
            Status::Failed(FailureReason::MaxStepsReached { max_steps: 50 })
        );
        assert_eq!(unified.stats.attempts, 50);
        assert!(unified.final_time > 0.0 && unified.final_time < 1.0);
        assert_eq!(calls.jacobian(), 0, "DOPRI5 never asks for the Jacobian");
        assert_eq!(unified.stats.counters.rhs_calls, calls.rhs());
        let explicit = unified
            .explicit_solution
            .as_ref()
            .expect("the DOPRI5 record");
        assert_eq!(explicit.stop_reason, Some(Dopri5StopReason::AttemptBudget));
        assert_eq!(bits(&explicit.final_state), bits(&unified.final_state));
    }

    #[test]
    fn p2_typed_stop_of_the_switching_adapter() {
        let test = "p2_typed_stop_of_the_switching_adapter";
        // a fatal right-hand side
        let calls = Calls::default();
        let fatal = VigilODE::solve(
            &fatal_rhs_after(&calls),
            (0.0, 1.0),
            &[1.0],
            SolverStrategy::AdaptiveReentrant,
        )
        .unwrap();
        let direct = SwitchingSolver::solve(
            &fatal_rhs_after(&Calls::default()),
            (0.0, 1.0),
            &[1.0],
            &Options::default(),
            &router_switching_config(),
        )
        .unwrap();
        // a blow-up: y' = y^2
        let blowup_problem = || {
            Problem::new(1, |_t, y: &[f64], dy: &mut [f64]| {
                dy[0] = y[0] * y[0];
                Ok::<(), CallbackError>(())
            })
            .jacobian(Jacobian::dense(|_t, y: &[f64], jac: &mut [f64]| {
                jac[0] = 2.0 * y[0];
                Ok(())
            }))
            .autonomous()
        };
        let blowup = VigilODE::solve(
            &blowup_problem(),
            (0.0, 2.0),
            &[1.0],
            SolverStrategy::AdaptiveReentrant,
        )
        .unwrap();
        record(
            test,
            json!({
                "fatal_stop": serde_json::to_value(&fatal.stop).unwrap(),
                "direct_message": direct.message,
                "blowup_stop": serde_json::to_value(&blowup.stop).unwrap(),
                "blowup_status": status_text(&blowup.status),
            }),
        );
        match &fatal.stop {
            Some(UnifiedStop::Switching {
                message,
                callback,
                attempts,
                max_steps,
                accounting_complete,
                ..
            }) => {
                assert_eq!(message, &direct.message, "the adapter's own message");
                let callback = callback.as_ref().expect("the fatal callback");
                assert_eq!(callback.callback, Callback::Rhs);
                assert!(callback.t > 0.3 && callback.message == "sensor offline");
                assert_eq!((*attempts, *max_steps), (direct.stats.attempts, 100_000));
                assert!(*accounting_complete);
            }
            other => panic!("expected the switching stop, got {other:?}"),
        }
        assert!(fatal.stop.as_ref().unwrap().is_exact());
        assert!(fatal.switching_solution.is_some());
        match &blowup.stop {
            Some(UnifiedStop::Switching {
                driver_stop,
                callback,
                method,
                ..
            }) => {
                assert!(callback.is_none());
                assert_eq!(*method, Some(vigilode::SwitchingMethod::Rodas5p));
                let stop = driver_stop.as_ref().expect("the RODAS5P stop record");
                assert_eq!(stop.reason, StopReason::MinimumStep);
            }
            other => panic!("expected the switching stop, got {other:?}"),
        }
        assert!(blowup.stop.as_ref().unwrap().is_exact());
        assert!(!blowup.stop.as_ref().unwrap().is_attempt_budget());
    }

    #[test]
    fn p2_legs_are_direct_calls_and_their_work_adds_up() {
        let test = "p2_legs_are_direct_calls_and_their_work_adds_up";
        let calls = Calls::default();
        let problem = robertson_fd(&calls).invariant("mass", |y| y[0] + y[1] + y[2]);
        let options = VigilodeOptions::default().max_steps(500);
        let y0 = [1.0, 0.0, 0.0];
        let unified = VigilODE::solve(&problem, (0.0, 10.0), &y0, &options).unwrap();
        assert_eq!(unified.legs.len(), 2);
        let (trial, fallback) = (&unified.legs[0], &unified.legs[1]);
        // the trial is Dopri5::solve with 250 attempts
        let mut trial_options = options.options.clone();
        trial_options.max_steps = 250;
        let direct_calls = Calls::default();
        let direct_trial = Dopri5::solve(
            &robertson_fd(&direct_calls),
            (0.0, 10.0),
            &y0,
            &trial_options,
        )
        .unwrap();
        // the fallback is Rodas5p::solve from the trial's last accepted state with the rest
        let mut rest_options = options.options.clone();
        rest_options.max_steps = 250;
        let direct_rest = Rodas5p::solve(
            &robertson_fd(&direct_calls),
            (direct_trial.final_time, 10.0),
            &direct_trial.final_state,
            &rest_options,
        )
        .unwrap();
        let screen = unified.analysis.screen.expect("the screen ran");
        let leg_rhs: u64 = unified.legs.iter().map(|leg| leg.counters.rhs_calls).sum();
        let mass0: f64 = y0.iter().sum();
        let mass1: f64 = unified.final_state.iter().sum();
        record(
            test,
            json!({
                "legs": serde_json::to_value(&unified.legs).unwrap(),
                "screen": serde_json::to_value(screen).unwrap(),
                "preflight_rhs_calls": unified.analysis.rhs_calls,
                "leg_rhs_calls": leg_rhs,
                "total_rhs_calls": unified.stats.counters.rhs_calls,
                "rhs_entries": calls.rhs(),
                "direct_rhs_entries": direct_calls.rhs(),
                "invariant_drift": unified.invariant_drift,
            }),
        );
        assert_eq!(
            (
                trial.tier,
                trial.attempt_budget,
                trial.attempts,
                trial.success
            ),
            (SelectedTier::Tier1Explicit, 250, 250, false)
        );
        assert!(matches!(
            trial.stop,
            Some(UnifiedStop::Dopri5 {
                reason: Some(Dopri5StopReason::AttemptBudget),
                callback: None,
                ..
            })
        ));
        assert_eq!(trial.t_end.to_bits(), direct_trial.final_time.to_bits());
        assert_eq!(
            (fallback.tier, fallback.attempt_budget, fallback.success),
            (SelectedTier::TierProtectedReference, 250, true)
        );
        assert_eq!(fallback.t_start.to_bits(), trial.t_end.to_bits());
        assert_eq!(fallback.attempts, direct_rest.stats.attempts);
        assert_eq!(
            serde_json::to_value(fallback.counters).unwrap(),
            serde_json::to_value(direct_rest.stats.counters).unwrap()
        );
        assert_eq!(
            bits(&unified.final_state),
            bits(direct_rest.final_state().unwrap())
        );
        // preflight + legs = total = what the callback boundary saw
        assert_eq!(screen.rhs_calls, 4, "n + 1 calls for n = 3");
        assert!(!screen.stiffness_evidence && screen.completed);
        assert_eq!(unified.analysis.rhs_calls, 4);
        assert_eq!(leg_rhs + 4, unified.stats.counters.rhs_calls);
        assert_eq!(unified.stats.counters.rhs_calls, calls.rhs());
        assert_eq!(calls.rhs(), direct_calls.rhs() + 4);
        assert_eq!(unified.stats.attempts, 250 + direct_rest.stats.attempts);
        assert_eq!(unified.stats.counters.fallback_steps, 1);
        // the solution of the RODAS5P leg is the direct one, and it starts at the checkpoint
        let inner = unified.inner_solution.as_ref().unwrap();
        assert_eq!(inner.t[0].to_bits(), trial.t_end.to_bits());
        // the drift is measured from the initial state of the solve, not from the checkpoint
        assert_eq!(unified.invariant_drift.len(), 1);
        assert_eq!(
            unified.invariant_drift[0].1.to_bits(),
            (mass1 - mass0).abs().to_bits()
        );
    }

    #[test]
    fn p2_exhausted_shared_budget_reports_the_total_and_keeps_the_leg_stop() {
        let test = "p2_exhausted_shared_budget_reports_the_total_and_keeps_the_leg_stop";
        let calls = Calls::default();
        // 20 attempts: a trial of 10 and, without evidence of stiffness on this span (the
        // screen gives L = 0.632, and 0.632 * 40 < 3.3 * 10), 10 for RODAS5P, which needs
        // more to reach t = 40
        let options = VigilodeOptions::default().max_steps(20);
        let unified = VigilODE::solve(
            &robertson_fd(&calls),
            (0.0, 40.0),
            &[1.0, 0.0, 0.0],
            &options,
        )
        .unwrap();
        record(
            test,
            json!({
                "drivers": unified.executed_drivers(),
                "leg_attempts": unified.legs.iter().map(|l| l.attempts).collect::<Vec<_>>(),
                "leg_budgets": unified.legs.iter().map(|l| l.attempt_budget).collect::<Vec<_>>(),
                "stop": serde_json::to_value(&unified.stop).unwrap(),
                "status": status_text(&unified.status),
                "final_time": unified.final_time,
                "rhs_entries": calls.rhs(),
                "rhs_charged": unified.stats.counters.rhs_calls,
            }),
        );
        assert!(!unified.success());
        assert_eq!(unified.legs.len(), 2);
        assert_eq!(unified.stats.attempts, 20);
        assert_eq!(
            unified.stop,
            Some(UnifiedStop::Rodas5p {
                reason: FailureReason::MaxStepsReached { max_steps: 10 }
            }),
            "the stop is the one the driver gave: its own budget"
        );
        assert_eq!(
            unified.status,
            Status::Failed(FailureReason::MaxStepsReached { max_steps: 20 }),
            "the status names the budget of the solve"
        );
        assert!(unified.final_time > unified.legs[0].t_end);
        assert_eq!(unified.stats.counters.rhs_calls, calls.rhs());
    }

    #[test]
    fn p2_declared_regime_routes_as_the_general_policy_does() {
        let test = "p2_declared_regime_routes_as_the_general_policy_does";
        let mut rows = Vec::new();
        for regime in [ProblemRegime::Stiff, ProblemRegime::Nonstiff] {
            let policy = GeneralOdePolicy {
                regime,
                derivatives: DerivativeFallback::AllowFiniteDifference,
                max_dense_dimension: usize::MAX,
                allow_matrix_free: true,
            };
            let cases: [(&str, Build, Vec<f64>); 3] = [
                ("oscillator", Box::new(oscillator), vec![1.0, 0.0]),
                (
                    "robertson-dense",
                    Box::new(robertson_dense),
                    vec![1.0, 0.0, 0.0],
                ),
                ("heat-jvp", Box::new(|c| heat_jvp(12, c)), heat_sine(12)),
            ];
            for (name, build, y0) in cases {
                let span = (0.0, 0.5);
                let problem = build(&Calls::default());
                let options = Options::default();
                let plan =
                    MethodRegistry::route(&problem, &options, &MethodChoice::General(policy))
                        .expect("the general policy admits the request");
                let general = GeneralOde::solve(&problem, span, &y0, &options, &policy).unwrap();
                let unified = VigilODE::solve(
                    &problem,
                    span,
                    &y0,
                    VigilodeOptions::default().regime(regime),
                )
                .unwrap();
                let equal_route = unified.executed_route() == Some(plan.route.id().as_str());
                let equal_state =
                    bits(&unified.final_state) == bits(general.final_state().unwrap());
                rows.push(json!({
                    "regime": format!("{regime:?}"), "problem": name,
                    "general_route": plan.route.id(), "unified_route": unified.executed_route(),
                    "equal_route": equal_route, "equal_state_bits": equal_state,
                    "legs": unified.legs.len(),
                }));
                assert!(
                    equal_route && equal_state && unified.legs.len() == 1,
                    "{name} {regime:?}"
                );
                assert_eq!(
                    unified.analysis.basis,
                    RouteBasis::DeclaredRegime { regime }
                );
            }
        }
        record(test, json!({ "cases": rows }));
    }

    #[test]
    fn p2_stiffness_screen_is_a_charged_lower_bound() {
        let test = "p2_stiffness_screen_is_a_charged_lower_bound";
        let screen_of = |build: &dyn Fn(&Calls) -> Problem, span: (f64, f64), y0: &[f64]| {
            let calls = Calls::default();
            let analysis =
                PreflightAnalyzer::analyze(&build(&calls), span, y0, &VigilodeOptions::default())
                    .unwrap();
            (analysis, calls.rhs())
        };
        let (scalar, scalar_entries) = screen_of(&scalar_fd, (0.0, 1.0), &[1.0]);
        let heat8 = |c: &Calls| heat_fd(8, c);
        let (heat, heat_entries) = screen_of(&heat8, (0.0, 100.0), &heat_sine(8));
        let (short, _) = screen_of(&heat8, (0.0, 1.0), &heat_sine(8));
        let (robertson, _) = screen_of(&robertson_fd, (0.0, 10.0), &[1.0, 0.0, 0.0]);
        let (osc, osc_entries) = screen_of(&oscillator, (0.0, 1.0), &[1.0, 0.0]);
        let bound =
            |a: &vigilode::PreflightAnalysis| a.screen.unwrap().lipschitz_lower_bound.unwrap();
        record(
            test,
            json!({
                "scalar": serde_json::to_value(scalar.screen).unwrap(),
                "heat_span_100": serde_json::to_value(heat.screen).unwrap(),
                "heat_span_1": serde_json::to_value(short.screen).unwrap(),
                "robertson": serde_json::to_value(robertson.screen).unwrap(),
                "oscillator": serde_json::to_value(osc.screen).unwrap(),
                "trial_limit": EXPLICIT_TRIAL_ATTEMPT_LIMIT,
                "dimension_limit": EXPLICIT_TRIAL_MAX_DIMENSION,
            }),
        );
        // every call is reported, and it is n + 1
        assert_eq!((scalar.rhs_calls, scalar_entries), (2, 2));
        assert_eq!((heat.rhs_calls, heat_entries), (9, 9));
        assert_eq!((osc.rhs_calls, osc_entries), (3, 3));
        // the threshold is 3.3 times the trial budget min(1000, max_steps / 2)
        assert_eq!(scalar.screen.unwrap().trial_attempts, 1000);
        assert_eq!(scalar.screen.unwrap().threshold, 3300.0);
        // a lower bound: never above the spectral radius, at least the norm over sqrt(n)
        assert!((bound(&scalar) / 1.0e6 - 1.0).abs() < 1e-6);
        assert!(bound(&heat) <= 324.0 * (1.0 + 1e-6) && bound(&heat) >= 324.0 / 8f64.sqrt());
        assert!((bound(&osc) - 1.0).abs() < 1e-6);
        // evidence moves the request to RODAS5P; no evidence leaves a budgeted trial
        for stiff in [&scalar, &heat] {
            assert!(stiff.screen.unwrap().stiffness_evidence);
            assert_eq!(stiff.suggested_tier, SelectedTier::TierProtectedReference);
            assert_eq!(stiff.basis, RouteBasis::StiffnessScreen);
        }
        for open in [&short, &robertson, &osc] {
            assert!(!open.screen.unwrap().stiffness_evidence);
            assert_eq!(open.suggested_tier, SelectedTier::Tier1Explicit);
            assert_eq!(open.basis, RouteBasis::UndeclaredRegimeTrial);
            assert!(open.fallback_to_rodas5p, "no evidence is not non-stiffness");
            assert_eq!(open.first_leg_attempts, 1000);
        }
        // a declaration needs no callback
        let calls = Calls::default();
        let declared = PreflightAnalyzer::analyze(
            &heat_dense(64, &calls),
            (0.0, 10.0),
            &heat_sine(64),
            &VigilodeOptions::default(),
        )
        .unwrap();
        assert_eq!(declared.basis, RouteBasis::SuppliedDerivative);
        assert_eq!(calls.rhs() + calls.jacobian(), 0);
        assert!(declared.screen.is_none());
    }

    #[test]
    fn p2_fallback_rule_and_trial_budget() {
        let test = "p2_fallback_rule_and_trial_budget";
        assert_eq!(FallbackEngine::explicit_trial_attempts(100_000), 1000);
        assert_eq!(FallbackEngine::explicit_trial_attempts(500), 250);
        assert_eq!(FallbackEngine::explicit_trial_attempts(1), 0);
        assert_eq!(FallbackEngine::remaining_attempts(500, 250), 250);
        assert_eq!(FallbackEngine::remaining_attempts(500, 600), 0);
        let follows: Vec<bool> = [
            Dopri5StopReason::AttemptBudget,
            Dopri5StopReason::StepUnderflow,
            Dopri5StopReason::NonFinite,
            Dopri5StopReason::Controller,
            Dopri5StopReason::Callback,
            Dopri5StopReason::Output,
        ]
        .into_iter()
        .map(|reason| FallbackEngine::follows_explicit_stop(Some(reason), false))
        .collect();
        assert_eq!(follows, [true, true, true, true, false, false]);
        assert!(!FallbackEngine::follows_explicit_stop(
            Some(Dopri5StopReason::AttemptBudget),
            true
        ));
        assert!(!FallbackEngine::follows_explicit_stop(None, false));
        // a budget of one attempt leaves no trial: RODAS5P runs, and one attempt is made
        let calls = Calls::default();
        let unified = VigilODE::solve(
            &oscillator(&calls),
            (0.0, 1.0),
            &[1.0, 0.0],
            VigilodeOptions::default().max_steps(1),
        )
        .unwrap();
        record(
            test,
            json!({
                "basis": serde_json::to_value(&unified.analysis.basis).unwrap(),
                "drivers": unified.executed_drivers(),
                "attempts": unified.stats.attempts,
                "status": status_text(&unified.status),
            }),
        );
        assert_eq!(
            unified.analysis.basis,
            RouteBasis::NoBudgetForTrial { max_steps: 1 }
        );
        assert_eq!(unified.legs.len(), 1);
        assert!(unified.legs[0].driver_id.starts_with("rodas5p"));
        assert!(unified.stats.attempts <= 1);
    }

    #[test]
    fn p2_unsupported_request_is_refused_before_any_callback() {
        let test = "p2_unsupported_request_is_refused_before_any_callback";
        let n = 12;
        let calls = Calls::default();
        let problem = heat_jvp(n, &calls);
        let options = VigilodeOptions::default().atol_vector(vec![1e-9; n]);
        let refused = VigilODE::solve(&problem, (0.0, 0.01), &heat_sine(n), &options);
        let entries = calls.rhs() + calls.jvp();
        // the caller can still declare the regime: DOPRI5 takes a vector tolerance
        let declared = VigilODE::solve(
            &problem,
            (0.0, 0.01),
            &heat_sine(n),
            options.clone().regime(ProblemRegime::Nonstiff),
        )
        .unwrap();
        record(
            test,
            json!({
                "refusal": refused.as_ref().err().map(|e| e.to_string()),
                "callback_entries_before_the_refusal": entries,
                "declared_nonstiff_drivers": declared.executed_drivers(),
            }),
        );
        assert!(matches!(
            refused,
            Err(SolveError::Unsupported(
                Unsupported::VectorAtolOnMatrixFreeRoute
            ))
        ));
        assert_eq!(entries, 0);
        assert_eq!(declared.executed_drivers(), ["dopri5-5(4)-fsal"]);
    }

    #[test]
    fn p2_a_refusal_of_the_switching_adapter_is_reported_and_rodas5p_runs() {
        let test = "p2_a_refusal_of_the_switching_adapter_is_reported_and_rodas5p_runs";
        let span = (0.0, 1.0);
        let y0 = [1.0, 0.0, 0.0];
        let direct_calls = Calls::default();
        let direct = Rodas5p::solve(
            &robertson_dae(&direct_calls),
            span,
            &y0,
            &Options::default(),
        )
        .unwrap();
        let calls = Calls::default();
        let unified = VigilODE::solve(
            &robertson_dae(&calls),
            span,
            &y0,
            SolverStrategy::AdaptiveReentrant,
        )
        .unwrap();
        record(
            test,
            json!({
                "suggested_tier": serde_json::to_value(unified.analysis.suggested_tier).unwrap(),
                "history": serde_json::to_value(&unified.route_history).unwrap(),
                "fallback_steps": unified.stats.counters.fallback_steps,
                "entries": [calls.rhs(), calls.jacobian()],
                "direct_entries": [direct_calls.rhs(), direct_calls.jacobian()],
            }),
        );
        assert_eq!(
            unified.analysis.suggested_tier,
            SelectedTier::Tier5AdaptiveSwitching
        );
        assert_eq!(unified.executed_drivers(), [direct.stats.route.driver_id]);
        assert!(unified.route_history[0].reason.contains("refused"));
        assert_eq!(unified.stats.counters.fallback_steps, 1);
        assert_eq!(
            bits(&unified.final_state),
            bits(direct.final_state().unwrap())
        );
        assert_eq!(
            (calls.rhs(), calls.jacobian()),
            (direct_calls.rhs(), direct_calls.jacobian())
        );
    }

    #[test]
    fn p2_records_are_strict_json_and_options_keep_their_file_format() {
        let test = "p2_records_are_strict_json_and_options_keep_their_file_format";
        let solutions = [
            VigilODE::solve(
                &robertson_fd(&Calls::default()),
                (0.0, 10.0),
                &[1.0, 0.0, 0.0],
                VigilodeOptions::default().max_steps(500),
            )
            .unwrap(),
            VigilODE::solve(
                &fatal_rhs_after(&Calls::default()),
                (0.0, 1.0),
                &[1.0],
                SolverStrategy::AdaptiveReentrant,
            )
            .unwrap(),
            VigilODE::solve(
                &fatal_rhs_after(&Calls::default()),
                (0.5, 1.0),
                &[1.0],
                SolverStrategy::Auto,
            )
            .unwrap(),
        ];
        let mut sizes = Vec::new();
        for solution in &solutions {
            let text = serde_json::to_string(solution).expect("the solution serializes");
            let value: Value = serde_json::from_str(&text).expect("strict JSON");
            for key in [
                "stop",
                "legs",
                "analysis",
                "route_history",
                "stats",
                "status",
            ] {
                assert!(value.get(key).is_some(), "missing {key}");
            }
            assert!(!text.contains("NaN") && !text.contains("Infinity"));
            sizes.push(text.len());
        }
        // the third solve stops in the preflight screen: no leg, a typed stop
        assert!(solutions[2].legs.is_empty());
        assert!(matches!(
            solutions[2].stop,
            Some(UnifiedStop::Preflight { .. })
        ));
        // options: every regime round-trips, and a file written before the field existed parses
        for regime in [
            ProblemRegime::Unknown,
            ProblemRegime::Nonstiff,
            ProblemRegime::Stiff,
        ] {
            let options = VigilodeOptions::default().regime(regime).max_steps(77);
            let text = serde_json::to_string(&options).unwrap();
            let back: VigilodeOptions = serde_json::from_str(&text).unwrap();
            assert_eq!(back, options);
        }
        let mut old: Value = serde_json::to_value(VigilodeOptions::default()).unwrap();
        old.as_object_mut().unwrap().remove("regime");
        let parsed: VigilodeOptions = serde_json::from_value(old).unwrap();
        assert_eq!(parsed.regime, ProblemRegime::Unknown);
        assert!(
            serde_json::from_str::<VigilodeOptions>(
                &serde_json::to_string(&VigilodeOptions::default())
                    .unwrap()
                    .replace("\"unknown\"", "\"sometimes\"")
            )
            .is_err()
        );
        record(test, json!({ "serialized_bytes": sizes }));
    }
}
