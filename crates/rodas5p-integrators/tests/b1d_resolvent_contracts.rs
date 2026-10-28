//! B1d resolvent diagnostic: exact-arithmetic identities, finite guards and
//! real-driver parity/accounting. This is not a floating-point enclosure.
mod b1_problems;
use b1_problems::*;
use rodas5p_core::{DenseMatrix, LuFactorization, WorkCounters};
use rodas5p_integrators::*;
use std::sync::Arc;
const GAMMA: f64 = 0.21193756319429014;

fn abs_resolvent(jp: &DenseMatrix, hp: f64) -> DenseMatrix {
    let n = jp.nrows();
    let mut m = DenseMatrix::identity(n);
    for i in 0..n {
        for k in 0..n {
            m[(i, k)] -= hp * GAMMA * jp[(i, k)];
        }
    }
    let lu = LuFactorization::new(&m).unwrap();
    let mut r = DenseMatrix::zeros(n, n);
    for k in 0..n {
        let mut b = vec![0.0; n];
        b[k] = 1.0;
        let col = lu.solve(&b).unwrap();
        for i in 0..n {
            r[(i, k)] = col[i].abs();
        }
    }
    r
}
fn t_norm(h: f64, hp: f64, j: &DenseMatrix, jp: &DenseMatrix, s: &[f64]) -> f64 {
    let n = s.len();
    let mut p = DenseMatrix::zeros(n, n);
    let mut w = p.clone();
    for i in 0..n {
        for k in 0..n {
            p[(i, k)] = -jp[(i, k)];
            w[(i, k)] = -j[(i, k)];
        }
        p[(i, i)] += 1.0 / (hp * GAMMA);
        w[(i, i)] += 1.0 / (h * GAMMA);
    }
    let lu = LuFactorization::new(&p).unwrap();
    let mut t = DenseMatrix::identity(n);
    for k in 0..n {
        let b: Vec<_> = (0..n).map(|i| w[(i, k)]).collect();
        let x = lu.solve(&b).unwrap();
        for i in 0..n {
            t[(i, k)] -= x[i];
        }
    }
    (0..n)
        .map(|i| (0..n).map(|k| t[(i, k)].abs() * s[k] / s[i]).sum::<f64>())
        .fold(0.0, f64::max)
}
fn opt(solver: StageSolver) -> Rodas5pLaggedOptions {
    Rodas5pLaggedOptions {
        stage_solver: solver,
        staleness: StalenessRule::Resolvent,
        ..Default::default()
    }
}
fn run(case: &Case, rtol: f64, options: Rodas5pLaggedOptions) -> Rodas5pLaggedResult {
    integrate_rodas5p_lagged_observed(
        &case.problem,
        case.span,
        &case.y0,
        &config(case, rtol),
        &schedule(case.span, 1),
        options,
    )
    .unwrap()
}

#[test]
fn resolvent_bound_dominates_explicit_t_and_not_b1c() {
    let mut count = 0;
    let mut max_ratio = 0.0_f64;
    for case in [robertson_case(), hires_case()] {
        let adaptive = config(&case, 1e-6);
        let fast = integrate_rodas5p_fast_observed(
            &case.problem,
            case.span,
            &case.y0,
            &adaptive,
            &schedule(case.span, 8),
        )
        .unwrap();
        for (now, old) in [(1, 0), (4, 2), (8, 6)] {
            let mut c = WorkCounters::default();
            let j = case
                .problem
                .dense_jacobian(fast.observed.t[now], &fast.observed.y[now], &mut c)
                .unwrap();
            let jp = case
                .problem
                .dense_jacobian(fast.observed.t[old], &fast.observed.y[old], &mut c)
                .unwrap();
            let s: Vec<_> = fast.observed.y[now]
                .iter()
                .map(|v| adaptive.atol + adaptive.rtol * v.abs())
                .collect();
            for h in [1e-5, 1e-3, 1e-1] {
                for ratio in [0.5, 0.8, 1.0, 1.25, 2.0] {
                    let hp = h * ratio;
                    let r = abs_resolvent(&jp, hp);
                    let q = rodas5p_resolvent_staleness_bound(h, hp, GAMMA, &j, &jp, &r, &s);
                    let t = t_norm(h, hp, &j, &jp, &s);
                    assert!(t <= q * (1. + 1e-9), "t={t} q={q}");
                    let old = rodas5p_staleness_bound(h, hp, GAMMA, &j, &jp, &s);
                    if old.is_finite() {
                        assert!(q <= old * (1. + 1e-12));
                    }
                    if q > 0. {
                        max_ratio = max_ratio.max(t / q);
                    }
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 90);
    println!("90 independent explicit-system comparisons; max norm/bound={max_ratio:.17e}");
}

#[test]
fn nonnormal_closed_form_and_numeric_guards() {
    let jp = DenseMatrix::from_rows(&[&[-1., 1e8], &[0., -1.]]).unwrap();
    let a = 1e-6;
    let hp = a / GAMMA;
    let h = hp / 0.8;
    let r = abs_resolvent(&jp, hp);
    let q = rodas5p_resolvent_staleness_bound(h, hp, GAMMA, &jp, &jp, &r, &[1., 1.]);
    // |1-hp/h|=.2, not .25 as the prior plan's fixture stated.
    let expected = 0.2 * (1. / (1. + a) + a * 1e8 / (1. + a).powi(2));
    assert!((q / expected - 1.).abs() < 1e-12);
    assert_eq!(
        rodas5p_staleness_bound(h, hp, GAMMA, &jp, &jp, &[1., 1.]),
        f64::INFINITY
    );
    for weights in [[1., 0.], [f64::MIN_POSITIVE, f64::MAX]] {
        assert_eq!(
            rodas5p_resolvent_staleness_bound(h, hp, GAMMA, &jp, &jp, &r, &weights),
            f64::INFINITY
        );
    }
    let mut invalid = r.clone();
    invalid[(0, 0)] = -1.;
    assert_eq!(
        rodas5p_resolvent_staleness_bound(h, hp, GAMMA, &jp, &jp, &invalid, &[1., 1.]),
        f64::INFINITY
    );
}

#[test]
fn forced_refresh_matches_fast_and_all_solves_are_charged() {
    for case in [robertson_case(), hires_case()] {
        for rtol in [1e-4, 1e-6, 1e-8] {
            let fast = integrate_rodas5p_fast_observed(
                &case.problem,
                case.span,
                &case.y0,
                &config(&case, rtol),
                &schedule(case.span, 1),
            )
            .unwrap();
            for solver in [StageSolver::Richardson, StageSolver::Gmres] {
                let mut options = opt(solver);
                options.refresh_every_attempt = true;
                let r = run(&case, rtol, options);
                assert!(r.fast.observed.success);
                assert_eq!(r.fast.observed.y, fast.observed.y);
                assert_eq!(r.fast.accepted_steps, fast.accepted_steps);
                assert_eq!(r.staleness.checks, 0);
                assert_eq!(
                    r.staleness.resolvent_solves,
                    case.problem.dimension as u64 * r.work.preconditioner_refreshes
                );
                assert_eq!(
                    r.fast.observed.counters.direct_solve_calls,
                    r.work.preconditioner_applications + r.staleness.resolvent_solves
                );
            }
        }
    }
}

#[test]
fn zero_weight_refreshes_and_completes_on_both_rules() {
    let problem = OdeProblem::new(
        "zero-weight",
        2,
        Arc::new(|_, y, out| {
            out[0] = -y[0];
            out[1] = 0.;
            Ok(())
        }),
        None,
        Some(Arc::new(|_, _| {
            DenseMatrix::from_rows(&[&[-1., 0.], &[0., 0.]])
        })),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let case = Case {
        id: "zero-weight",
        problem,
        y0: vec![1., 0.],
        span: (0., 1.),
        atol_scale: 0.,
    };
    let fast = integrate_rodas5p_fast_observed(
        &case.problem,
        case.span,
        &case.y0,
        &config(&case, 1e-6),
        &schedule(case.span, 1),
    )
    .unwrap();
    for rule in [StalenessRule::Bounded, StalenessRule::Resolvent] {
        for solver in [StageSolver::Richardson, StageSolver::Gmres] {
            let mut options = opt(solver);
            options.staleness = rule;
            let r = run(&case, 1e-6, options);
            assert!(r.fast.observed.success);
            assert_eq!(r.fast.observed.y, fast.observed.y);
            assert!(r.staleness.checks > 0);
            assert_eq!(r.staleness.checks, r.staleness.unbounded_checks);
            assert!(r.staleness.q_pred_max.is_finite());
        }
    }
}

#[test]
#[ignore = "diagnostic, gate fixed in docs/plan/IMPLEMENTATION_WAVE4.md; B1D_RESULTS_OUT writes evidence"]
fn b1d_kill_gate_diagnostic() {
    let mut rows = Vec::new();
    let mut gate_pass = true;
    for case in [hires_case(), brusselator_case()] {
        let cfg = config(&case, 1e-6);
        let out = schedule(case.span, 1);
        let reference = integrate_rodas5p_fast_observed(
            &case.problem,
            case.span,
            &case.y0,
            &config(&case, 1e-11),
            &out,
        )
        .unwrap();
        let fast = integrate_rodas5p_fast_observed(&case.problem, case.span, &case.y0, &cfg, &out)
            .unwrap();
        assert!(reference.observed.success && fast.observed.success);
        let truth = reference.observed.y.last().unwrap();
        let baseline = endpoint_errors(fast.observed.y.last().unwrap(), truth, &cfg);
        for solver in [StageSolver::Gmres, StageSolver::Richardson] {
            let before = std::time::Instant::now();
            let r = run(&case, 1e-6, opt(solver));
            let elapsed = before.elapsed().as_secs_f64();
            assert!(r.fast.observed.success);
            let e = endpoint_errors(r.fast.observed.y.last().unwrap(), truth, &cfg);
            let facts = r.fast.observed.counters.direct_factorizations as f64
                / r.fast.accepted_steps as f64;
            let steps = r.fast.accepted_steps as f64 / fast.accepted_steps as f64;
            let err = e.1 / baseline.1;
            let pass = facts <= 0.4 && steps <= 1.05 && err <= 1.5;
            if solver == StageSolver::Gmres {
                gate_pass &= pass;
            }
            assert_eq!(
                r.work.preconditioner_refreshes,
                r.work.initial_refreshes
                    + r.work.forced_refreshes
                    + r.work.contraction_refreshes
                    + r.work.budget_refreshes
                    + r.staleness.staleness_refreshes
            );
            assert_eq!(
                r.staleness.resolvent_solves,
                case.problem.dimension as u64 * r.work.preconditioner_refreshes
            );
            assert_eq!(
                r.fast.observed.counters.direct_solve_calls,
                r.work.preconditioner_applications + r.staleness.resolvent_solves
            );
            rows.push(serde_json::json!({"problem":case.id,"dimension":case.problem.dimension,"rtol":cfg.rtol,"atol":cfg.atol,"solver":format!("{solver:?}"),"driver":r.fast.driver,"accepted":r.fast.accepted_steps,"rejected":r.fast.rejected_steps,"attempts":r.fast.attempts,"counters":r.fast.observed.counters,"work":r.work,"staleness":r.staleness,"fast_accepted":fast.accepted_steps,"fast_counters":fast.observed.counters,"fast_endpoint_error_tol_units":baseline.1,"endpoint_error_tol_units":e.1,"factorizations_per_accepted":facts,"steps_ratio":steps,"error_ratio":err,"would_pass_gate":pass,"gated_arm":solver==StageSolver::Gmres,"wall_seconds_diagnostic":elapsed}));
        }
    }
    let data = serde_json::json!({"node":"b1d-resolvent","measurement_status":"DIAGNOSTIC_NOT_PUBLISHED_PREREGISTRATION","would_pass_gate":gate_pass,"timing_authority":"diagnostic_only","arithmetic_authority":"floating_estimate_not_enclosure","routing":"unchanged_opt_in_only","rows":rows});
    println!("{}", serde_json::to_string_pretty(&data).unwrap());
    if let Ok(path) = std::env::var("B1D_RESULTS_OUT") {
        std::fs::write(path, serde_json::to_string_pretty(&data).unwrap() + "\n").unwrap();
    }
}
