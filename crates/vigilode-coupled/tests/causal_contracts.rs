use rodas5p_core::{CoreError, DenseMatrix, rodas5p_coefficients};
use rodas5p_integrators::{OdeProblem, ParallelExecution};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use vigilode_coupled::{
    CausalSchedule, Config, FailureKind, coupled_step_causal, coupled_step_residual_first,
};

fn model(n: usize, eps: f64, forbid_current: bool, calls: Arc<AtomicUsize>) -> OdeProblem {
    OdeProblem::new(
        "causal-three-mode",
        n,
        Arc::new(move |_, y, f| {
            for i in 0..n {
                f[i] = [-1.0, -20.0, -100.0][i % 3] * y[i] + eps * y[i] * y[i];
            }
            Ok(())
        }),
        None,
        Some(Arc::new(|_, _| -> Result<DenseMatrix, CoreError> {
            panic!("dense Jacobian forbidden")
        })),
        Some(Arc::new(move |_, y, v, f| {
            calls.fetch_add(1, Ordering::SeqCst);
            if forbid_current {
                assert!(
                    y.iter().all(|v| *v == 0.1),
                    "current-stage derivative dispatched"
                );
            }
            for i in 0..n {
                f[i] = ([-1.0, -20.0, -100.0][i % 3] + 2.0 * eps * y[i]) * v[i];
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}
fn reference(n: usize, eps: f64, h: f64) -> Vec<f64> {
    let c = rodas5p_coefficients().unwrap();
    let mut u = vec![vec![0.0; n]; 8];
    for i in 0..8 {
        for k in 0..n {
            let mut state = 0.1;
            let mut mix = 0.0;
            for j in 0..i {
                state += c.a[(i, j)] * u[j][k];
                mix += c.c_matrix[(i, j)] * u[j][k];
            }
            let a = [-1.0, -20.0, -100.0][k % 3];
            u[i][k] = (h * c.gamma * (a * state + eps * state * state) + c.gamma * mix)
                / (1.0 - h * c.gamma * (a + 0.2 * eps));
        }
    }
    (0..n)
        .map(|k| 0.1 + (0..8).map(|i| c.b_code[i] * u[i][k]).sum::<f64>())
        .collect()
}
#[test]
fn nonlinear_root_uses_no_current_jvp_and_matches_direct_scalar_oracle() {
    for eps in [0.0, 0.5, 10.0] {
        for schedule in [CausalSchedule::Forward, CausalSchedule::BatchedPicard] {
            let calls = Arc::new(AtomicUsize::new(0));
            let p = model(96, eps, true, calls.clone());
            let out = coupled_step_causal(
                &p,
                0.0,
                &[0.1; 96],
                0.01,
                &Config::default(),
                &ParallelExecution::sequential(),
                schedule,
            );
            let c = out
                .candidate
                .as_ref()
                .unwrap_or_else(|| panic!("{schedule:?} {eps}: {:?}", out.failure));
            for (a, b) in c.y_new.iter().zip(reference(96, eps, 0.01)) {
                assert!((a - b).abs() < 3e-12, "{a} vs {b}");
            }
            assert_eq!(out.work.stage_jvp, 0);
            assert_eq!(out.work.newton_updates, 0);
            assert_eq!(out.work.derivative_batches, 0);
            assert_eq!(out.work.validation_jvp, 8);
            assert_eq!(out.work.small_factorizations, 1);
            assert_eq!(calls.load(Ordering::SeqCst) as u64, out.work.jvp_attempted);
            assert!(out.work.causal_picard_updates <= 8);
            if schedule == CausalSchedule::Forward {
                assert_eq!(out.work.rhs_attempted, 16);
                assert_eq!(out.work.causal_forward_stages, 8);
                assert_eq!(out.work.small_block_solves, 8);
            }
        }
    }
}
#[test]
fn changing_worker_count_keeps_batched_bits_and_counts() {
    let p = model(96, 5.0, true, Arc::new(AtomicUsize::new(0)));
    let a = coupled_step_causal(
        &p,
        0.0,
        &[0.1; 96],
        0.01,
        &Config::default(),
        &ParallelExecution::sequential(),
        CausalSchedule::BatchedPicard,
    );
    let b = coupled_step_causal(
        &p,
        0.0,
        &[0.1; 96],
        0.01,
        &Config::default(),
        &ParallelExecution::rayon(2).unwrap(),
        CausalSchedule::BatchedPicard,
    );
    let astate = &a.candidate.as_ref().unwrap().y_new;
    let bstate = &b.candidate.as_ref().unwrap().y_new;
    assert!(
        astate
            .iter()
            .zip(bstate)
            .all(|(x, y)| x.to_bits() == y.to_bits())
    );
    assert_eq!(a.work.rhs_attempted, b.work.rhs_attempted);
    assert_eq!(a.work.jvp_attempted, b.work.jvp_attempted);
}
#[test]
fn forward_saves_work_against_nonlinear_residual_first() {
    let p = model(96, 1.0, false, Arc::new(AtomicUsize::new(0)));
    let e = ParallelExecution::sequential();
    let old = coupled_step_residual_first(&p, 0.0, &[0.1; 96], 0.01, &Config::default(), &e);
    let new = coupled_step_causal(
        &p,
        0.0,
        &[0.1; 96],
        0.01,
        &Config::default(),
        &e,
        CausalSchedule::Forward,
    );
    assert!(old.candidate.is_some() && new.candidate.is_some());
    assert!(new.work.jvp_attempted < old.work.jvp_attempted);
    assert!(new.work.rhs_attempted < old.work.rhs_attempted);
    println!(
        "CAUSAL_WORK old={} new={}",
        serde_json::to_string(&old.work).unwrap(),
        serde_json::to_string(&new.work).unwrap()
    );
}
#[test]
fn budget_failure_preserves_state_and_attempted_work() {
    let p = model(96, 1.0, true, Arc::new(AtomicUsize::new(0)));
    let y = [0.1; 96];
    let cfg = Config {
        max_rhs: 7,
        ..Config::default()
    };
    let out = coupled_step_causal(
        &p,
        0.0,
        &y,
        0.01,
        &cfg,
        &ParallelExecution::sequential(),
        CausalSchedule::Forward,
    );
    assert_eq!(out.failure.unwrap().kind, FailureKind::Budget);
    assert_eq!(out.work.rhs_attempted, 1);
    assert_eq!(y, [0.1; 96]);
}
#[test]
fn full_residual_refuses_incomplete_basis() {
    let p = model(96, 1.0, true, Arc::new(AtomicUsize::new(0)));
    let cfg = Config {
        initial_basis: 1,
        max_basis: 1,
        max_expansions: 0,
        ..Config::default()
    };
    let out = coupled_step_causal(
        &p,
        0.0,
        &[0.1; 96],
        0.01,
        &cfg,
        &ParallelExecution::sequential(),
        CausalSchedule::Forward,
    );
    assert_eq!(out.failure.unwrap().kind, FailureKind::ProjectionLeakage);
    assert_eq!(out.work.validation_jvp, 8);
}

#[test]
fn adaptive_causal_flow_reaches_observations_with_independent_exact_solution() {
    use rodas5p_core::{LinearMethod, LinearSolverConfig, PreconditionerKind};
    use rodas5p_integrators::{AdaptiveStepConfig, OutputSchedule};
    use vigilode_coupled::{RootErrorPolicy, integrate_causal_observed};
    let p = model(6, 0.5, false, Arc::new(AtomicUsize::new(0)));
    let adaptive = AdaptiveStepConfig {
        atol: 1e-11,
        rtol: 1e-7,
        initial_step: 0.05,
        max_step: 0.05,
        ..Default::default()
    };
    let schedule = OutputSchedule::new(vec![0.0, 0.1, 0.2]).unwrap();
    let linear = LinearSolverConfig {
        method: LinearMethod::Gmres,
        preconditioner: PreconditionerKind::None,
        atol: 1e-14,
        rtol: 1e-12,
        ..Default::default()
    };
    for mode in [CausalSchedule::Forward, CausalSchedule::BatchedPicard] {
        let r = integrate_causal_observed(
            &p,
            (0.0, 0.2),
            &[0.1; 6],
            &adaptive,
            &schedule,
            &Config::default(),
            &ParallelExecution::sequential(),
            &linear,
            &RootErrorPolicy::Operational,
            mode,
        )
        .unwrap();
        assert!(
            r.observed.success,
            "{} {:?}",
            r.observed.message, r.terminal_failure
        );
        assert_eq!(r.observed.t, schedule.times());
        for (t, y) in r.observed.t.iter().zip(&r.observed.y) {
            for i in 0..6 {
                let a = [-1.0, -20.0, -100.0][i % 3];
                let e = (a * t).exp();
                let exact = 0.1 * e / (1.0 + (0.05 / a) * (1.0 - e));
                assert!(
                    (y[i] - exact).abs() < 3e-9,
                    "{mode:?} t={t} i={i} got={} exact={exact}",
                    y[i]
                );
            }
        }
        assert!(
            r.attempts
                .iter()
                .all(|a| a.candidate_work.newton_updates == 0 && a.candidate_work.stage_jvp == 0)
        );
        assert!(r.attempts.iter().all(|a| !a.fallback_attempted));
    }
}
