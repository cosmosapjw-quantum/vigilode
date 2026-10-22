use rodas5p_core::{CoreError, LinearMethod, LinearSolverConfig, rodas5p_coefficients};
use rodas5p_integrators::{AdaptiveFailureKind, OdeProblem, ParallelExecution};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use vigilode_coupled::{
    Config, FailureKind, L2Assumptions, RawStepSession, RootErrorPolicy,
    conditional_l2_error_estimate,
};

fn linear_config() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1e-13,
        atol: 1e-15,
        ..LinearSolverConfig::default()
    }
}
fn diagonal(rates: Vec<f64>) -> OdeProblem {
    counted_diagonal(rates).0
}
fn counted_diagonal(rates: Vec<f64>) -> (OdeProblem, Arc<AtomicU64>, Arc<AtomicU64>) {
    let j = rates.clone();
    let n = rates.len();
    let rhs_calls = Arc::new(AtomicU64::new(0));
    let jvp_calls = Arc::new(AtomicU64::new(0));
    let rc = rhs_calls.clone();
    let jc = jvp_calls.clone();
    let problem = OdeProblem::new(
        "diagonal",
        n,
        Arc::new(move |_, y, f| {
            rc.fetch_add(1, Ordering::SeqCst);
            for i in 0..n {
                f[i] = rates[i] * y[i];
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, _, v, out| {
            jc.fetch_add(1, Ordering::SeqCst);
            for i in 0..n {
                out[i] = j[i] * v[i];
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    (problem, rhs_calls, jvp_calls)
}
fn raw_reference(rates: &[f64], y: &[f64], h: f64) -> (Vec<Vec<f64>>, Vec<f64>) {
    let c = rodas5p_coefficients().unwrap();
    let mut u = vec![vec![0.0; y.len()]; 8];
    for i in 0..8 {
        for k in 0..y.len() {
            let mut state = y[k];
            let mut mix = 0.0;
            for j in 0..i {
                state += c.a[(i, j)] * u[j][k];
                mix += c.c_matrix[(i, j)] * u[j][k];
            }
            u[i][k] =
                (h * c.gamma * rates[k] * state + c.gamma * mix) / (1.0 - h * c.gamma * rates[k]);
        }
    }
    let mut yn = y.to_vec();
    for i in 0..8 {
        for k in 0..y.len() {
            yn[k] += c.b_code[i] * u[i][k];
        }
    }
    (u, yn)
}

#[test]
fn forced_fallback_preserves_raw_target_and_charges_verification() {
    let rates = vec![-1.0, -20.0, -100.0];
    let (p, rhs_calls, jvp_calls) = counted_diagonal(rates.clone());
    let y = vec![0.1; 3];
    let mut s = RawStepSession::new(&p, &linear_config()).unwrap();
    let c = Config {
        max_jvp: 0,
        ..Config::default()
    };
    let out = s.attempt(
        &p,
        0.0,
        &y,
        0.01,
        &c,
        &ParallelExecution::sequential(),
        &RootErrorPolicy::Operational,
    );
    assert!(out.fallback_attempted);
    assert!(out.terminal_failure.is_none());
    assert_eq!(out.candidate_attempt.work.rhs_attempted, 1);
    assert_eq!(out.fallback_counters.linear_solves, 8);
    assert_eq!(out.fallback_validation_work.rhs_attempted, 8);
    assert_eq!(out.fallback_validation_work.jvp_attempted, 8);
    assert_eq!(
        rhs_calls.load(Ordering::SeqCst),
        out.candidate_attempt.work.rhs_attempted
            + out.fallback_counters.rhs_calls
            + out.fallback_validation_work.rhs_attempted
    );
    assert_eq!(
        jvp_calls.load(Ordering::SeqCst),
        out.candidate_attempt.work.jvp_attempted
            + out.fallback_counters.jvp_calls
            + out.fallback_validation_work.jvp_attempted
    );
    assert_eq!(
        out.fallback_callback_work.rhs_attempted,
        out.fallback_counters.rhs_calls
    );
    assert_eq!(
        out.fallback_callback_work.rhs_completed,
        out.fallback_callback_work.rhs_attempted
    );
    assert_eq!(
        out.fallback_callback_work.jvp_attempted,
        out.fallback_counters.jvp_calls
    );
    assert_eq!(
        out.fallback_callback_work.jvp_completed,
        out.fallback_callback_work.jvp_attempted
    );
    assert_eq!(
        out.fallback_counters.jvp_vectors,
        out.fallback_counters.jvp_calls
    );
    let selected = out.selected.unwrap();
    assert_eq!(selected.selection, "sequential-raw-u-fallback");
    assert_eq!(selected.target_id, vigilode_coupled::TARGET_ID);
    let (_, reference) = raw_reference(&rates, &y, 0.01);
    for (a, b) in selected.y_new.iter().zip(reference) {
        assert!((a - b).abs() < 1e-12);
    }
    assert!(selected.full_residual_wrms <= c.full_residual_wrms_limit);
    assert_eq!(y, vec![0.1; 3]);
}

#[test]
fn underflowed_stage_step_cannot_bypass_validation_through_budget_fallback() {
    let p = diagonal(vec![-1.0]);
    let mut session = RawStepSession::new(&p, &linear_config()).unwrap();
    let h = f64::from_bits(1);
    assert!(h > 0.0);
    assert_eq!(h * rodas5p_coefficients().unwrap().gamma, 0.0);
    let cfg = Config {
        max_storage_bytes: 0,
        ..Config::default()
    };
    let out = session.attempt(
        &p,
        0.0,
        &[1.0],
        h,
        &cfg,
        &ParallelExecution::sequential(),
        &RootErrorPolicy::Operational,
    );
    assert_eq!(out.terminal_failure.unwrap().kind, FailureKind::Input);
    assert!(out.selected.is_none());
    assert!(!out.fallback_attempted);
    assert_eq!(out.candidate_attempt.work.rhs_attempted, 0);
}

#[test]
fn embedded_rejection_does_not_trigger_a_second_algorithm() {
    let p = diagonal(vec![-10.0]);
    let mut s = RawStepSession::new(&p, &linear_config()).unwrap();
    let c = Config {
        atol: 1e-12,
        rtol: 1e-9,
        full_residual_wrms_limit: 0.1,
        ..Config::default()
    };
    let out = s.attempt(
        &p,
        0.0,
        &[1.0],
        0.2,
        &c,
        &ParallelExecution::sequential(),
        &RootErrorPolicy::Operational,
    );
    let selected = out
        .selected
        .as_ref()
        .expect("scalar invariant subspace should produce a candidate");
    assert_eq!(selected.selection, "coupled-raw-u");
    assert!(selected.embedded_wrms > 1.0);
    assert!(selected.effective_error > 1.0);
    assert!(!out.fallback_attempted);
}

#[test]
fn causal_bound_dominates_deliberately_perturbed_raw_solution() {
    let h = 0.01;
    let rate = -3.0;
    let c = rodas5p_coefficients().unwrap();
    let (u, _) = raw_reference(&[rate], &[0.8], h);
    let mut perturbed = u.clone();
    for i in 0..8 {
        perturbed[i][0] += (i + 1) as f64 * 1e-8;
    }
    let mut r = [0.0; 8];
    for i in 0..8 {
        let mut state = 0.8;
        let mut mix = 0.0;
        for j in 0..i {
            state += c.a[(i, j)] * perturbed[j][0];
            mix += c.c_matrix[(i, j)] * perturbed[j][0];
        }
        r[i] = ((1.0 - h * c.gamma * rate) * perturbed[i][0]
            - h * c.gamma * rate * state
            - c.gamma * mix)
            .abs();
    }
    let a = L2Assumptions {
        inverse_norm_bound: 1.0 / (1.0 - h * c.gamma * rate),
        rhs_lipschitz_bound: rate.abs(),
        residual_roundoff_l2: 1e-15,
    };
    let b = conditional_l2_error_estimate(h, r, &a, &[1.0]).unwrap();
    for i in 0..8 {
        assert!((perturbed[i][0] - u[i][0]).abs() <= b.stage_l2[i]);
    }
    let actual: f64 = c
        .b_code
        .iter()
        .enumerate()
        .map(|(i, w)| w * (perturbed[i][0] - u[i][0]))
        .sum();
    assert!(actual.abs() <= b.endpoint_l2);
    assert_eq!(
        b.authority,
        "conditional-f64-estimate-user-assumptions-not-certified"
    );
}

#[test]
fn invalid_conditional_assumptions_run_no_callbacks() {
    let calls = Arc::new(AtomicU64::new(0));
    let cc = calls.clone();
    let p = OdeProblem::new(
        "counted",
        1,
        Arc::new(move |_, y, f| {
            cc.fetch_add(1, Ordering::SeqCst);
            f[0] = -y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, _, v, f| {
            f[0] = -v[0];
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let mut s = RawStepSession::new(&p, &linear_config()).unwrap();
    let policy = RootErrorPolicy::ConditionalL2 {
        assumptions: L2Assumptions {
            inverse_norm_bound: -1.0,
            rhs_lipschitz_bound: 1.0,
            residual_roundoff_l2: 0.0,
        },
        max_endpoint_wrms: 0.1,
    };
    let out = s.attempt(
        &p,
        0.0,
        &[1.0],
        0.01,
        &Config::default(),
        &ParallelExecution::sequential(),
        &policy,
    );
    assert_eq!(out.terminal_failure.unwrap().kind, FailureKind::Input);
    assert!(!out.fallback_attempted);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn callback_linear_solve_error_is_not_a_retryable_solver_failure() {
    let calls = Arc::new(AtomicU64::new(0));
    let cc = calls.clone();
    let rhs_calls = Arc::new(AtomicU64::new(0));
    let rc = rhs_calls.clone();
    let p = OdeProblem::new(
        "injected",
        1,
        Arc::new(move |_, y, f| {
            rc.fetch_add(1, Ordering::SeqCst);
            f[0] = -y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, _, _, _| {
            cc.fetch_add(1, Ordering::SeqCst);
            Err(CoreError::LinearSolve("injected callback error".into()))
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let mut s = RawStepSession::new(&p, &linear_config()).unwrap();
    let cfg = Config {
        max_jvp: 0,
        ..Config::default()
    };
    let out = s.attempt(
        &p,
        0.0,
        &[1.0],
        0.1,
        &cfg,
        &ParallelExecution::sequential(),
        &RootErrorPolicy::Operational,
    );
    assert!(out.fallback_attempted);
    assert!(out.selected.is_none());
    assert!(!out.retryable_failure);
    assert_eq!(out.terminal_failure.unwrap().kind, FailureKind::Callback);
    assert!(calls.load(Ordering::SeqCst) > 0);
    assert!(out.fallback_counters.jvp_calls > 0);
    assert_eq!(
        out.fallback_counters.jvp_calls,
        calls.load(Ordering::SeqCst)
    );
    assert_eq!(
        out.fallback_counters.jvp_vectors,
        calls.load(Ordering::SeqCst)
    );
    assert_eq!(
        out.fallback_callback_work.jvp_attempted,
        calls.load(Ordering::SeqCst)
    );
    assert_eq!(out.fallback_callback_work.jvp_completed, 0);
    assert_eq!(
        out.fallback_counters.rhs_calls,
        rhs_calls.load(Ordering::SeqCst) - out.candidate_attempt.work.rhs_attempted
    );
    assert_eq!(
        out.fallback_callback_work.rhs_completed,
        out.fallback_callback_work.rhs_attempted
    );
}

#[test]
fn genuine_iteration_exhaustion_is_retryable_and_counted() {
    let p = diagonal(vec![-1.0, -10.0, -100.0]);
    let linear = LinearSolverConfig {
        restart: 1,
        maxiter: 1,
        rtol: 1e-14,
        atol: 1e-15,
        ..linear_config()
    };
    let mut s = RawStepSession::new(&p, &linear).unwrap();
    let cfg = Config {
        max_jvp: 0,
        ..Config::default()
    };
    let out = s.attempt(
        &p,
        0.0,
        &[1.0; 3],
        0.2,
        &cfg,
        &ParallelExecution::sequential(),
        &RootErrorPolicy::Operational,
    );
    assert!(out.fallback_attempted);
    assert!(out.selected.is_none());
    assert!(out.retryable_failure);
    assert_eq!(
        out.retry_failure_kind,
        Some(AdaptiveFailureKind::LinearSolve)
    );
    assert_eq!(
        out.terminal_failure.unwrap().kind,
        FailureKind::RootIteration
    );
    assert!(out.fallback_counters.jvp_calls > 0);
}

#[test]
fn per_stage_residual_norms_reuse_the_original_validation() {
    let rates = vec![-1.0, -20.0, -100.0];
    let p = diagonal(rates.clone());
    let y = [0.1; 3];
    let h = 0.01;
    let out = vigilode_coupled::coupled_step_residual_first(
        &p,
        0.0,
        &y,
        h,
        &Config::default(),
        &ParallelExecution::sequential(),
    );
    let candidate = out.candidate.unwrap();
    let c = rodas5p_coefficients().unwrap();
    assert_eq!(out.work.validation_jvp, 8);
    for i in 0..8 {
        let mut residual = [0.0; 3];
        for k in 0..3 {
            let mut state = y[k];
            let mut mix = 0.0;
            for j in 0..i {
                state += c.a[(i, j)] * candidate.stages_u[j][k];
                mix += c.c_matrix[(i, j)] * candidate.stages_u[j][k];
            }
            residual[k] = candidate.stages_u[i][k]
                - h * c.gamma * rates[k] * candidate.stages_u[i][k]
                - c.gamma * mix
                - h * c.gamma * rates[k] * state;
        }
        let direct = rodas5p_core::safe_l2(&residual);
        assert!(candidate.full_residual_l2[i].is_finite());
        // Parenthesization of the independent scalar target can differ by a
        // few ulps; the norm must still describe residual-scale data.
        assert!((candidate.full_residual_l2[i] - direct).abs() < 1e-14);
    }
}

#[test]
fn a_changed_model_epoch_prevents_fallback_commit() {
    let epoch = Arc::new(AtomicU64::new(0));
    let jepoch = epoch.clone();
    let ee = epoch.clone();
    let p = OdeProblem::new(
        "changing",
        1,
        Arc::new(|_, y, f| {
            f[0] = -y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, _, v, f| {
            jepoch.fetch_add(1, Ordering::SeqCst);
            f[0] = -v[0];
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
    .with_model_epoch(Arc::new(move || ee.load(Ordering::SeqCst)));
    let mut s = RawStepSession::new(&p, &linear_config()).unwrap();
    let cfg = Config {
        max_jvp: 0,
        ..Config::default()
    };
    let y = [1.0];
    let out = s.attempt(
        &p,
        0.0,
        &y,
        0.1,
        &cfg,
        &ParallelExecution::sequential(),
        &RootErrorPolicy::Operational,
    );
    assert!(out.fallback_attempted);
    assert!(out.selected.is_none());
    assert!(!out.retryable_failure);
    assert_eq!(
        out.terminal_failure.unwrap().kind,
        FailureKind::ModelChanged
    );
    assert_eq!(y, [1.0]);
}
