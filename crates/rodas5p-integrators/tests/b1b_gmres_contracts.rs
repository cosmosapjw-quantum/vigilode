//! Contracts of the B1b-gmres arm of the lagged-factorization RODAS5P
//! driver (production wave-2 node `b1b-gmres`; plan §5 T1, T2, T9, §6 B1;
//! preregistration `docs/plan/nodes/b1b-gmres.md`).
//!
//! * `StageSolver::Richardson` (the default) is the wave-1 driver bit for
//!   bit: pinned endpoint bits and counts recorded at `0879a7e` before any
//!   code change.
//! * With a refresh forced at every attempt the GMRES arm is the direct
//!   solve at every stage and reproduces the fast driver (1e-12 relative).
//! * The default GMRES arm iterates, charges every refresh, and routes
//!   `W x` through the JVP when the problem has one.
//! * The monitored right-preconditioned GMRES entry point of
//!   `rodas5p-krylov` reproduces `solve_gmres_into` bit for bit when the
//!   monitor accepts only at the last column of a full cycle.
//! * The preregistered measurement (`#[ignore]`d) prints the kill gate.

mod b1_problems;

use std::time::Instant;

use b1_problems::*;
use rodas5p_core::{
    CoreResult, DenseMatrix, DenseOperator, Preconditioner, PreconditionerSide, WorkCounters,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, RODAS5P_LAGGED_DRIVER_ID, RODAS5P_LAGGED_GMRES_DRIVER_ID,
    RODAS5P_LAGGED_GMRES_RESTART, Rodas5pLaggedOptions, Rodas5pLaggedResult, StageSolver,
    integrate_rodas5p_fast_observed, integrate_rodas5p_lagged_observed,
};
use rodas5p_krylov::{
    GmresCapacity, GmresConfig, GmresMonitorDecision, GmresWorkspace, solve_gmres_into,
    solve_gmres_into_right_monitored,
};

fn gmres() -> Rodas5pLaggedOptions {
    Rodas5pLaggedOptions {
        stage_solver: StageSolver::Gmres,
        ..Rodas5pLaggedOptions::default()
    }
}

fn run(
    case: &Case,
    rtol: f64,
    points: usize,
    options: Rodas5pLaggedOptions,
) -> Rodas5pLaggedResult {
    integrate_rodas5p_lagged_observed(
        &case.problem,
        case.span,
        &case.y0,
        &config(case, rtol),
        &schedule(case.span, points),
        options,
    )
    .unwrap()
}

/// Endpoint bits and counts of the default (Richardson) arm at rtol 1e-6,
/// one output interval, printed at `0879a7e` (unmodified wave-1 code):
/// (id, accepted, rejected, factorizations, Richardson iterations,
/// abandoned iterations, fold of the endpoint bits, first and last
/// endpoint component bits).
type Pin = (&'static str, usize, usize, u64, u64, u64, u64, u64, u64);
const RICHARDSON_BASELINE: [Pin; 3] = [
    (
        "robertson",
        43,
        2,
        43,
        308,
        216,
        0xf470f0b1f6d5b609,
        0x3fe6e80d9df56862,
        0x3fd22fbe3d3364ef,
    ),
    (
        "hires",
        205,
        8,
        134,
        4434,
        770,
        0xb42427b86aeb21ca,
        0x3f482781614ba3c0,
        0x3f6758e2aa95f8f9,
    ),
    (
        "brusselator-1d-50 (n = 100)",
        82,
        10,
        70,
        1673,
        386,
        0x49a7c068cc1f9b46,
        0x3fee602eaf5419de,
        0x400885aee1f362df,
    ),
];

#[test]
fn richardson_default_is_the_wave1_driver_bit_for_bit() {
    assert_eq!(StageSolver::default(), StageSolver::Richardson);
    assert_eq!(
        Rodas5pLaggedOptions::default().stage_solver,
        StageSolver::Richardson
    );
    for (case, pin) in [robertson_case(), hires_case(), brusselator_case()]
        .iter()
        .zip(RICHARDSON_BASELINE)
    {
        assert_eq!(case.id, pin.0);
        let r = run(case, 1.0e-6, 1, Rodas5pLaggedOptions::default());
        assert_eq!(r.fast.driver, RODAS5P_LAGGED_DRIVER_ID);
        let end = r.fast.observed.y.last().unwrap();
        let fold = end.iter().fold(0u64, |h, v| h.rotate_left(7) ^ v.to_bits());
        assert_eq!(
            (
                r.fast.accepted_steps,
                r.fast.rejected_steps,
                r.fast.observed.counters.direct_factorizations,
                r.work.richardson_iterations,
                r.work.abandoned_iterations,
                fold,
                end[0].to_bits(),
                end[end.len() - 1].to_bits(),
            ),
            (pin.1, pin.2, pin.3, pin.4, pin.5, pin.6, pin.7, pin.8),
            "{}",
            case.id
        );
        assert_eq!(r.work.gmres_iterations, 0);
        assert_eq!(r.work.gmres_cycles, 0);
        assert_eq!(r.work.residual_applications, 0);
    }
}

#[test]
fn gmres_forced_refresh_reproduces_the_fast_driver_on_robertson_and_hires() {
    for case in [robertson_case(), hires_case()] {
        for rtol in [1.0e-4, 1.0e-6, 1.0e-8] {
            let adaptive = config(&case, rtol);
            let output = schedule(case.span, 8);
            let fast = integrate_rodas5p_fast_observed(
                &case.problem,
                case.span,
                &case.y0,
                &adaptive,
                &output,
            )
            .unwrap();
            let lagged = run(
                &case,
                rtol,
                8,
                Rodas5pLaggedOptions {
                    refresh_every_attempt: true,
                    stage_solver: StageSolver::Gmres,
                },
            );
            let (f, l) = (&fast.observed, &lagged.fast.observed);
            assert!(f.success && l.success, "{} rtol {rtol}", case.id);
            assert_eq!(lagged.fast.driver, RODAS5P_LAGGED_GMRES_DRIVER_ID);
            assert_eq!(f.t, l.t, "{} rtol {rtol}", case.id);
            assert_eq!(fast.accepted_steps, lagged.fast.accepted_steps);
            assert_eq!(fast.rejected_steps, lagged.fast.rejected_steps);
            assert_eq!(
                f.counters.direct_factorizations,
                l.counters.direct_factorizations
            );
            assert_eq!(f.counters.jacobian_builds, l.counters.jacobian_builds);
            assert_eq!(lagged.work.gmres_iterations, 0);
            assert_eq!(lagged.work.operator_applications, 0);
            for (a, b) in f.y.iter().flatten().zip(l.y.iter().flatten()) {
                let scale = a.abs().max(f64::MIN_POSITIVE);
                assert!(
                    (a - b).abs() <= 1.0e-12 * scale,
                    "{} rtol {rtol}: fast {a:e} vs gmres {b:e}",
                    case.id
                );
            }
        }
    }
}

fn assert_gmres_ledger(case_id: &str, lagged: &Rodas5pLaggedResult, has_jvp: bool) {
    let c = &lagged.fast.observed.counters;
    let w = &lagged.work;
    assert!(lagged.fast.observed.success, "{case_id}");
    assert_eq!(lagged.fast.driver, RODAS5P_LAGGED_GMRES_DRIVER_ID);
    assert_eq!(w.preconditioner_refreshes, c.direct_factorizations);
    assert_eq!(
        w.preconditioner_refreshes,
        w.initial_refreshes + w.forced_refreshes + w.contraction_refreshes + w.budget_refreshes
    );
    assert_eq!(w.forced_refreshes, 0);
    assert!(w.initial_refreshes >= 1);
    assert_eq!(
        c.jacobian_builds as usize,
        lagged.fast.attempts - lagged.fast.jacobian_reuses
    );
    assert_eq!(
        (w.exact_stage_solves + w.lagged_stage_solves + w.refreshed_stage_solves) as usize,
        8 * lagged.fast.attempts
    );
    // No Richardson work in the GMRES arm; every W x is an Arnoldi column
    // or a residual (explicit stopping check or cycle-start residual).
    assert_eq!(w.richardson_iterations, 0);
    assert!(w.gmres_iterations > 0, "{case_id}: no GMRES column");
    assert!(w.gmres_cycles > 0);
    assert!(w.lagged_stage_solves > 0, "{case_id}: no lagged stage");
    assert!(w.abandoned_iterations <= w.gmres_iterations);
    assert_eq!(
        w.operator_applications,
        w.gmres_iterations + w.residual_applications
    );
    // Every stored-LU application is a direct solve of the run.
    assert_eq!(w.preconditioner_applications, c.direct_solve_calls);
    // A stage takes at most 6 columns, so the restart (10) never fires.
    assert_eq!(RODAS5P_LAGGED_GMRES_RESTART, 10);
    assert!(w.gmres_cycles <= w.lagged_stage_solves + w.refreshed_stage_solves);
    if has_jvp {
        assert_eq!(w.jvp_applications, w.operator_applications);
        assert!(c.jvp_calls >= w.jvp_applications);
    } else {
        assert_eq!(w.jvp_applications, 0);
    }
}

#[test]
fn gmres_run_charges_every_refresh_and_counts_every_product() {
    for case in [robertson_case(), hires_case()] {
        let lagged = run(&case, 1.0e-6, 1, gmres());
        assert_gmres_ledger(case.id, &lagged, case.problem.has_jvp());
    }
}

/// The JVP branch of the stage operator inside GMRES: HIRES with a JVP,
/// default GMRES options; stages iterate and every `W x` is a JVP.
#[test]
fn gmres_jvp_path_runs_inside_iterating_stages() {
    let (problem, y0) = hires_problem_with_jvp(true).unwrap();
    assert!(problem.has_jvp() && problem.has_explicit_jacobian());
    let case = Case {
        id: "hires+jvp",
        problem,
        y0,
        span: (0.0, 321.8122),
        atol_scale: 1.0e-4,
    };
    let lagged = run(&case, 1.0e-6, 1, gmres());
    assert_gmres_ledger(case.id, &lagged, true);
    assert!(lagged.work.gmres_iterations > 0);
    let matvec = run(&hires_case(), 1.0e-6, 1, gmres());
    assert_gmres_ledger("hires", &matvec, false);
}

/// A small nonsymmetric system with a right preconditioner.
struct RightDiag(Vec<f64>);
impl Preconditioner for RightDiag {
    fn dimension(&self) -> usize {
        self.0.len()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        for ((o, v), d) in y.iter_mut().zip(x).zip(&self.0) {
            *o = v / d;
        }
        Ok(())
    }
    fn side(&self) -> PreconditionerSide {
        PreconditionerSide::Right
    }
}

fn small_system() -> (DenseOperator, RightDiag, Vec<f64>, Vec<f64>) {
    let n = 7;
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = 4.0 + i as f64;
        if i + 1 < n {
            a[(i, i + 1)] = -1.3;
            a[(i + 1, i)] = 0.7 + 0.1 * i as f64;
        }
        a[(i, (i + 3) % n)] += 0.25;
    }
    let d = (0..n).map(|i| 3.5 + 0.9 * i as f64).collect();
    let b = (0..n).map(|i| 1.0 + (i as f64).sin()).collect();
    let x0 = (0..n).map(|i| 0.1 * i as f64).collect();
    (DenseOperator::new(a).unwrap(), RightDiag(d), b, x0)
}

#[test]
fn monitored_gmres_matches_gmres_into_at_the_last_column_of_a_full_cycle() {
    let (op, pc, b, x0) = small_system();
    let n = b.len();
    let config = GmresConfig {
        restart: n,
        max_arnoldi: n,
        rtol: 1.0e-10,
        atol: 0.0,
    };
    let mut reference = vec![0.0_f64; n];
    let mut counters = WorkCounters::default();
    solve_gmres_into(
        &op,
        &pc,
        &b,
        Some(&x0),
        &config,
        None,
        &mut reference,
        &mut GmresWorkspace::default(),
        GmresCapacity::unbounded(),
        &mut counters,
    )
    .unwrap();
    let mut seen = Vec::new();
    let mut monitored = vec![0.0; n];
    let mut counters = WorkCounters::default();
    let report = solve_gmres_into_right_monitored(
        &op,
        &pc,
        &b,
        &x0,
        &config,
        &mut monitored,
        &mut GmresWorkspace::default(),
        &mut counters,
        &mut |k, _x: &[f64], _c: &mut WorkCounters| {
            seen.push(k);
            Ok(if k == n as u64 {
                GmresMonitorDecision::Accept
            } else {
                GmresMonitorDecision::Continue
            })
        },
    )
    .unwrap();
    assert!(report.accepted);
    assert_eq!(report.iterations, n as u64);
    assert_eq!(report.cycles, 1);
    assert_eq!(seen, (1..=n as u64).collect::<Vec<_>>());
    for (a, m) in reference.iter().zip(&monitored) {
        assert_eq!(a.to_bits(), m.to_bits());
    }
}

/// A 60 x 60 nonsymmetric, non-normal system (convection-diffusion-like
/// band plus a long-range coupling) with a diagonal right preconditioner.
fn large_system() -> (DenseOperator, RightDiag, Vec<f64>, Vec<f64>) {
    let n = 60;
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = 3.0 + 0.05 * i as f64;
        if i + 1 < n {
            a[(i, i + 1)] = -1.6;
        }
        if i > 0 {
            a[(i, i - 1)] = -0.4 - 0.01 * i as f64;
        }
        if i + 2 < n {
            a[(i, i + 2)] = 0.15;
        }
        a[(i, (7 * i + 11) % n)] += 0.05 * ((i % 5) as f64 - 2.0);
    }
    let d = (0..n).map(|i| 2.5 + 0.04 * i as f64).collect();
    let b = (0..n).map(|i| (0.3 * i as f64).cos() + 0.1).collect();
    let x0 = (0..n).map(|i| 0.01 * (i % 7) as f64).collect();
    (DenseOperator::new(a).unwrap(), RightDiag(d), b, x0)
}

/// Fix round 1 (review Important 1): after the column/least-squares
/// refactor, the monitored iterate at the end of cycle `C` is bitwise
/// `solve_gmres_into`'s solution when that solve converges after `C`
/// restarted cycles, on a 60 x 60 system, with the allocating and the
/// reused least-squares paths.
#[test]
fn monitored_gmres_matches_gmres_into_over_restarted_cycles_on_a_large_system() {
    let (op, pc, b, x0) = large_system();
    let n = b.len();
    for reuse_ls in [false, true] {
        let workspace = || {
            if reuse_ls {
                GmresWorkspace::with_ls_workspace()
            } else {
                GmresWorkspace::default()
            }
        };
        let config = GmresConfig {
            restart: 8,
            max_arnoldi: 400,
            rtol: 1.0e-9,
            atol: 0.0,
        };
        let mut reference = vec![0.0_f64; n];
        let report = solve_gmres_into(
            &op,
            &pc,
            &b,
            Some(&x0),
            &config,
            None,
            &mut reference,
            &mut workspace(),
            GmresCapacity::unbounded(),
            &mut WorkCounters::default(),
        )
        .unwrap();
        assert!(report.cycles >= 2, "cycles {}", report.cycles);
        let last = report.iterations;
        let mut monitored = vec![0.0; n];
        let mut seen = 0_u64;
        let mreport = solve_gmres_into_right_monitored(
            &op,
            &pc,
            &b,
            &x0,
            &config,
            &mut monitored,
            &mut workspace(),
            &mut WorkCounters::default(),
            &mut |k, _x: &[f64], _c: &mut WorkCounters| {
                seen = k;
                Ok(if k == last {
                    GmresMonitorDecision::Accept
                } else {
                    GmresMonitorDecision::Continue
                })
            },
        )
        .unwrap();
        assert!(mreport.accepted);
        assert_eq!(seen, last);
        assert_eq!(mreport.iterations, last);
        assert_eq!(mreport.cycles, report.cycles);
        for (r, m) in reference.iter().zip(&monitored) {
            assert_eq!(r.to_bits(), m.to_bits(), "reuse_ls {reuse_ls}");
        }
    }
}

#[test]
fn monitored_gmres_abandons_and_exhausts_without_writing_the_output() {
    let (op, pc, b, x0) = small_system();
    let n = b.len();
    for (decision, expected) in [
        (GmresMonitorDecision::Abandon, 1_u64),
        (GmresMonitorDecision::Continue, 3_u64),
    ] {
        let config = GmresConfig {
            restart: 2,
            max_arnoldi: 3,
            rtol: 0.0,
            atol: 0.0,
        };
        let mut out = vec![-7.0; n];
        let mut counters = WorkCounters::default();
        let report = solve_gmres_into_right_monitored(
            &op,
            &pc,
            &b,
            &x0,
            &config,
            &mut out,
            &mut GmresWorkspace::default(),
            &mut counters,
            &mut |_k, _x: &[f64], _c: &mut WorkCounters| Ok(decision),
        )
        .unwrap();
        assert!(!report.accepted);
        assert_eq!(report.iterations, expected);
        assert!(out.iter().all(|v| *v == -7.0));
    }
    // Restart 2 with a 3-column budget: two cycles (2 + 1 columns).
    let report = solve_gmres_into_right_monitored(
        &op,
        &pc,
        &b,
        &x0,
        &GmresConfig {
            restart: 2,
            max_arnoldi: 3,
            rtol: 0.0,
            atol: 0.0,
        },
        &mut vec![0.0; n],
        &mut GmresWorkspace::default(),
        &mut WorkCounters::default(),
        &mut |_k, _x: &[f64], _c: &mut WorkCounters| Ok(GmresMonitorDecision::Continue),
    )
    .unwrap();
    assert_eq!(report.cycles, 2);
}

#[test]
fn monitored_gmres_refuses_a_left_preconditioner() {
    let (op, _, b, x0) = small_system();
    let n = b.len();
    let left = rodas5p_core::IdentityPreconditioner::new(n);
    let error = solve_gmres_into_right_monitored(
        &op,
        &left,
        &b,
        &x0,
        &GmresConfig::default(),
        &mut vec![0.0; n],
        &mut GmresWorkspace::default(),
        &mut WorkCounters::default(),
        &mut |_k, _x: &[f64], _c: &mut WorkCounters| Ok(GmresMonitorDecision::Accept),
    )
    .unwrap_err();
    assert!(matches!(error, rodas5p_core::CoreError::InvalidInput(_)));
}

struct Row {
    arm: &'static str,
    accepted: usize,
    rejected: usize,
    jacobians: u64,
    factorizations: u64,
    iterations: u64,
    abandoned: u64,
    operator_applications: u64,
    lu_applications: u64,
    error_max_abs: f64,
    error_tol_units: f64,
    wall_ms: f64,
}

/// The preregistered B1b-gmres measurement (`docs/plan/nodes/b1b-gmres.md`):
/// rtol 1e-6, reference = the fast driver at rtol 1e-11. Kill gate per
/// problem: factorizations per accepted step > 0.4, or accepted steps >
/// 1.05x fast, or endpoint error (tolerance units) > 1.5x fast. Wall time
/// is diagnostic only. The Richardson arm is printed for comparison only.
#[test]
#[ignore = "measurement: run with --ignored --nocapture and record in docs/plan/nodes/b1b-gmres.md"]
fn b1b_gmres_measurement_hires_and_brusselator() {
    for case in [hires_case(), brusselator_case()] {
        let output = schedule(case.span, 1);
        let reference = integrate_rodas5p_fast_observed(
            &case.problem,
            case.span,
            &case.y0,
            &config(&case, 1.0e-11),
            &output,
        )
        .unwrap();
        assert!(reference.observed.success);
        let reference_end = reference.observed.y.last().unwrap().clone();
        let adaptive: AdaptiveStepConfig = config(&case, 1.0e-6);
        let start = Instant::now();
        let fast =
            integrate_rodas5p_fast_observed(&case.problem, case.span, &case.y0, &adaptive, &output)
                .unwrap();
        let fast_ms = start.elapsed().as_secs_f64() * 1e3;
        assert!(fast.observed.success);
        let (fe_abs, fe_tol) =
            endpoint_errors(fast.observed.y.last().unwrap(), &reference_end, &adaptive);
        let fc = &fast.observed.counters;
        let mut rows = vec![Row {
            arm: "fast",
            accepted: fast.accepted_steps,
            rejected: fast.rejected_steps,
            jacobians: fc.jacobian_builds,
            factorizations: fc.direct_factorizations,
            iterations: 0,
            abandoned: 0,
            operator_applications: 0,
            lu_applications: fc.direct_solve_calls,
            error_max_abs: fe_abs,
            error_tol_units: fe_tol,
            wall_ms: fast_ms,
        }];
        let mut works = Vec::new();
        for (arm, options) in [
            (
                "lagged-richardson (wave-1, comparison)",
                Rodas5pLaggedOptions::default(),
            ),
            ("lagged-gmres", gmres()),
        ] {
            let start = Instant::now();
            let lagged = run(&case, 1.0e-6, 1, options);
            let ms = start.elapsed().as_secs_f64() * 1e3;
            assert!(lagged.fast.observed.success);
            let (abs, tol) = endpoint_errors(
                lagged.fast.observed.y.last().unwrap(),
                &reference_end,
                &adaptive,
            );
            let c = &lagged.fast.observed.counters;
            let w = &lagged.work;
            rows.push(Row {
                arm,
                accepted: lagged.fast.accepted_steps,
                rejected: lagged.fast.rejected_steps,
                jacobians: c.jacobian_builds,
                factorizations: c.direct_factorizations,
                iterations: w.richardson_iterations + w.gmres_iterations,
                abandoned: w.abandoned_iterations,
                operator_applications: w.operator_applications,
                lu_applications: c.direct_solve_calls,
                error_max_abs: abs,
                error_tol_units: tol,
                wall_ms: ms,
            });
            works.push((arm, *w));
        }
        println!("\n## {} (rtol 1e-6, atol {:e})", case.id, adaptive.atol);
        println!(
            "| arm | accepted | rejected | Jacobians | factorizations | fact/accepted | iterations | abandoned it. | W·x | LU applications | endpoint max abs err | endpoint err (tol units) | wall ms (diagnostic) |"
        );
        println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|");
        for r in &rows {
            println!(
                "| {} | {} | {} | {} | {} | {:.3} | {} | {} | {} | {} | {:.3e} | {:.3e} | {:.2} |",
                r.arm,
                r.accepted,
                r.rejected,
                r.jacobians,
                r.factorizations,
                r.factorizations as f64 / r.accepted as f64,
                r.iterations,
                r.abandoned,
                r.operator_applications,
                r.lu_applications,
                r.error_max_abs,
                r.error_tol_units,
                r.wall_ms
            );
        }
        for (arm, w) in &works {
            println!(
                "{arm}: refreshes initial {}, contraction {}, budget {}; stages exact {}, lagged {}, refreshed {}; GMRES cycles {}, residual W·x {}",
                w.initial_refreshes,
                w.contraction_refreshes,
                w.budget_refreshes,
                w.exact_stage_solves,
                w.lagged_stage_solves,
                w.refreshed_stage_solves,
                w.gmres_cycles,
                w.residual_applications
            );
        }
        let g = &rows[2];
        let fact_ratio = g.factorizations as f64 / g.accepted as f64;
        let step_ratio = g.accepted as f64 / rows[0].accepted as f64;
        let error_ratio = g.error_tol_units / rows[0].error_tol_units;
        let fail_fact = fact_ratio > 0.4;
        let fail_steps = step_ratio > 1.05;
        let fail_error = error_ratio > 1.5;
        println!(
            "kill gate (lagged-gmres): fact/accepted {fact_ratio:.3} (> 0.4 fails: {fail_fact}), accepted ratio {step_ratio:.4} (> 1.05 fails: {fail_steps}), error ratio {error_ratio:.3} (> 1.5 fails: {fail_error}) => {}",
            if fail_fact || fail_steps || fail_error {
                "FAIL"
            } else {
                "PASS"
            }
        );
    }
}
