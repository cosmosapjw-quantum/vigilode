//! `VigilODE::solve_banded` and `VigilODE::solve_stages` (stage-lattice referee run, node
//! SL-C6). The explicit banded stage policies keep the protected driver's stages, controller
//! and output; only the stage linear algebra changes.
//!
//! - `Shipped` is `Rodas5p::solve` bit for bit;
//! - `Tight` returns the protected banded driver's states and callback work bit for bit;
//! - `TwoWay` returns the same bits with one and two workers and agrees with the protected
//!   driver to a small fraction of one tolerance unit;
//! - every request outside the banded fast route is refused before any callback.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use vigilode::{
    Atol, BandedStagePolicy, CallbackError, DriverWork, Jacobian, MfStageSolvePolicy, Options,
    Problem, Rodas5p, Solution, SolveError, StagePolicy, Status, Tier, Unsupported, VigilODE,
};

/// The one-dimensional Brusselator with interleaved species, Dirichlet `u = 1`, `v = 3`.
/// Band: lower = upper = 2. Every callback increments `calls`.
fn brusselator(cells: usize, calls: Arc<AtomicU64>, jacobian: bool) -> Problem {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let n = 2 * cells;
    let rhs_calls = calls.clone();
    let problem = Problem::new(n, move |_t, y, out| {
        rhs_calls.fetch_add(1, Ordering::Relaxed);
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (ul, vl) = if i == 0 {
                (1.0, 3.0)
            } else {
                (y[2 * i - 2], y[2 * i - 1])
            };
            let (ur, vr) = if i + 1 == cells {
                (1.0, 3.0)
            } else {
                (y[2 * i + 2], y[2 * i + 3])
            };
            out[2 * i] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur);
            out[2 * i + 1] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr);
        }
        Ok::<(), CallbackError>(())
    })
    .autonomous();
    if !jacobian {
        // Finite-difference band.
        return problem.banded(2, 2);
    }
    problem.jacobian(Jacobian::banded(2, 2, move |_t, y, band| {
        calls.fetch_add(1, Ordering::Relaxed);
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (da, db) = (2 * i * 5 + 2, (2 * i + 1) * 5 + 2);
            band[da] = 2.0 * u * v - 4.0 - 2.0 * c;
            band[da + 1] = u * u;
            band[db - 1] = 3.0 - 2.0 * u * v;
            band[db] = -u * u - 2.0 * c;
            if i > 0 {
                band[da - 2] = c;
                band[db - 2] = c;
            }
            if i + 1 < cells {
                band[da + 2] = c;
                band[db + 2] = c;
            }
        }
        Ok(())
    }))
}

fn initial(cells: usize) -> Vec<f64> {
    (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
            [1.0 + (2.0 * std::f64::consts::PI * x).sin(), 3.0]
        })
        .collect()
}

fn options() -> Options {
    Options {
        rtol: 1.0e-6,
        atol: Atol::Scalar(1.0e-6),
        h0: 1.0e-6,
        ..Options::default()
    }
}

const SPAN: (f64, f64) = (0.0, 10.0);

fn bits(s: &Solution) -> Vec<u64> {
    s.final_state()
        .unwrap()
        .iter()
        .map(|v| v.to_bits())
        .collect()
}

fn assert_same_run(a: &Solution, b: &Solution, label: &str) {
    assert_eq!(a.status, b.status, "{label}");
    assert_eq!(a.stats.accepted_steps, b.stats.accepted_steps, "{label}");
    assert_eq!(a.stats.rejected_steps, b.stats.rejected_steps, "{label}");
    assert_eq!(a.stats.attempts, b.stats.attempts, "{label}");
    assert_eq!(a.stats.counters, b.stats.counters, "{label}");
    assert_eq!(a.stats.callbacks, b.stats.callbacks, "{label}");
    assert_eq!(bits(a), bits(b), "{label}");
}

#[test]
fn shipped_is_rodas5p_solve_and_tight_returns_the_protected_bits() {
    for jacobian in [true, false] {
        let cells = 120;
        let problem = brusselator(cells, Arc::new(AtomicU64::new(0)), jacobian);
        let y0 = initial(cells);
        let opts = options();
        let reference = Rodas5p::solve(&problem, SPAN, &y0, &opts).unwrap();
        assert_eq!(reference.status, Status::Success);

        let shipped =
            VigilODE::solve_banded(&problem, SPAN, &y0, &opts, BandedStagePolicy::Shipped).unwrap();
        assert_same_run(&shipped, &reference, "shipped");
        assert_eq!(
            shipped.stats.route.driver_id,
            reference.stats.route.driver_id
        );
        assert_eq!(shipped.stats.tier_executed, Tier::Protected);

        let tight =
            VigilODE::solve_banded(&problem, SPAN, &y0, &opts, BandedStagePolicy::Tight).unwrap();
        assert_same_run(
            &tight,
            &reference,
            &format!("tight, analytic band {jacobian}"),
        );
        assert_eq!(tight.stats.tier_executed, Tier::Performance);
        assert!(matches!(
            tight.stats.driver_work,
            DriverWork::BandedClient {
                client: "rodas5p-fast-client-banded-tight-v21",
                workers: 1,
                ..
            }
        ));
        assert_eq!(
            tight.stats.route.entry_point,
            "integrate_rodas5p_fast_client_observed_with_atol"
        );

        // The unified stage entry is the same call.
        let unified = VigilODE::solve_stages(
            &problem,
            SPAN,
            &y0,
            &opts,
            StagePolicy::Banded(BandedStagePolicy::Tight),
        )
        .unwrap();
        assert_same_run(&unified, &tight, "solve_stages(Banded(Tight))");
        let unified_shipped =
            VigilODE::solve_stages(&problem, SPAN, &y0, &opts, StagePolicy::Shipped).unwrap();
        assert_same_run(&unified_shipped, &reference, "solve_stages(Shipped)");
    }
}

#[test]
fn two_way_is_deterministic_across_workers_and_tracks_the_protected_driver() {
    let cells = 300;
    let problem = brusselator(cells, Arc::new(AtomicU64::new(0)), true);
    let y0 = initial(cells);
    for opts in [
        options(),
        Options {
            atol: Atol::Vector(vec![1.0e-6; 2 * cells]),
            ..options()
        },
    ] {
        let reference = Rodas5p::solve(&problem, SPAN, &y0, &opts).unwrap();
        let one = VigilODE::solve_banded(
            &problem,
            SPAN,
            &y0,
            &opts,
            BandedStagePolicy::TwoWay { workers: 1 },
        )
        .unwrap();
        let two = VigilODE::solve_banded(
            &problem,
            SPAN,
            &y0,
            &opts,
            BandedStagePolicy::TwoWay { workers: 2 },
        )
        .unwrap();
        assert_same_run(&one, &two, "two-way, one vs two workers");
        assert!(matches!(
            two.stats.driver_work,
            DriverWork::BandedClient {
                client: "rodas5p-fast-client-banded-two-way-v1",
                workers: 2,
                ..
            }
        ));
        assert_eq!(one.status, Status::Success);
        let drift = (one.stats.attempts as f64 - reference.stats.attempts as f64).abs();
        assert!(drift <= 0.02 * reference.stats.attempts as f64);
        let scaled = one
            .final_state()
            .unwrap()
            .iter()
            .zip(reference.final_state().unwrap())
            .map(|(a, b)| (a - b).abs() / (1.0e-6 + 1.0e-6 * b.abs()))
            .fold(0.0f64, f64::max);
        assert!(scaled <= 1.0e-2, "{scaled:e} tolerance units");
    }
}

#[test]
fn candidate_policies_outside_the_banded_route_are_refused_before_any_callback() {
    let calls = Arc::new(AtomicU64::new(0));
    let cells = 20;
    let y0 = initial(cells);
    let n = 2 * cells;
    let banded = brusselator(cells, calls.clone(), true);
    let dense = Problem::new(n, {
        let calls = calls.clone();
        move |_t, _y, out| {
            calls.fetch_add(1, Ordering::Relaxed);
            out.fill(0.0);
            Ok::<(), CallbackError>(())
        }
    });
    let jvp_only = Problem::new(n, |_t, _y, out| {
        out.fill(0.0);
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::jvp({
        let calls = calls.clone();
        move |_t, _y, v, out| {
            calls.fetch_add(1, Ordering::Relaxed);
            out.copy_from_slice(v);
            Ok(())
        }
    }));
    let dense_output = Options {
        dense_output: true,
        ..options()
    };
    let refused = |problem: &Problem, opts: &Options, policy| {
        let error = VigilODE::solve_banded(problem, SPAN, &y0, opts, policy).unwrap_err();
        assert!(
            matches!(
                error,
                SolveError::Unsupported(Unsupported::BandedPolicyRequiresBandedRoute)
            ),
            "{error}"
        );
    };
    for policy in [
        BandedStagePolicy::Tight,
        BandedStagePolicy::TwoWay { workers: 1 },
        BandedStagePolicy::TwoWay { workers: 2 },
    ] {
        refused(&dense, &options(), policy);
        refused(&jvp_only, &options(), policy);
        refused(&banded, &dense_output, policy);
        refused(&banded.clone().mass(identity(n)), &options(), policy);
    }
    refused(
        &banded,
        &options(),
        BandedStagePolicy::TwoWay { workers: 0 },
    );
    refused(
        &banded,
        &options(),
        BandedStagePolicy::TwoWay { workers: 3 },
    );
    assert_eq!(calls.load(Ordering::Relaxed), 0);

    // A matrix-free policy through the unified entry keeps its own refusal.
    let error = VigilODE::solve_stages(
        &banded,
        SPAN,
        &y0,
        &options(),
        StagePolicy::MatrixFree(MfStageSolvePolicy::ColumnTested),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        SolveError::Unsupported(Unsupported::MatrixFreePolicyRequiresJvpRoute)
    ));
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}

fn identity(n: usize) -> Vec<f64> {
    let mut m = vec![0.0; n * n];
    for i in 0..n {
        m[i * n + i] = 1.0;
    }
    m
}
