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
    let direct = SwitchingSolver::solve(
        &build(),
        span,
        &[1.0],
        &options,
        &router_switching_config(),
    )
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
        uncharged(direct_calls.jacobian(), direct.stats.counters.jacobian_builds),
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
    let (solved, largest, bytes) = allocation_scope(|| {
        VigilODE::solve(&problem, (0.0, 1.0e-7), &y0, SolverStrategy::Auto)
    });
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
