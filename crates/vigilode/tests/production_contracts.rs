use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use vigilode::{
    Atol, CallbackError, DerivativeFallback, Jacobian, Options, Problem, ProductionConfig,
    ProductionSolver, SwitchingConfig,
};

fn default_options() -> Options {
    Options {
        rtol: 1e-6,
        atol: Atol::Scalar(1e-9),
        h0: 1e-4,
        max_steps: 10_000,
        ..Options::default()
    }
}

#[test]
fn test_callback_failure_returns_partial_prefix_with_success_false() {
    let call_count = Arc::new(AtomicU64::new(0));
    let entered = call_count.clone();
    let problem = Problem::new(1, move |t, y, dy| {
        entered.fetch_add(1, Ordering::SeqCst);
        if t > 0.2 {
            return Err(CallbackError::Fatal(
                "user callback triggered failure at t > 0.2".into(),
            ));
        }
        dy[0] = -y[0];
        Ok(())
    })
    .autonomous();

    let config = ProductionConfig::rhs_only(default_options(), 10);

    // 1. Unchecked `solve`: returns Ok(solution) with success == false
    let sol = ProductionSolver::solve(&problem, (0.0, 1.0), &[1.0], &config).unwrap();
    assert!(!sol.success, "callback failure must yield success == false");
    assert!(
        sol.final_time <= 0.2,
        "final_time must not exceed failure point"
    );
    assert!(
        sol.final_time > 0.0,
        "final_time must have progressed before failure"
    );
    assert!(
        sol.stats.accounting_complete,
        "accounting must remain complete"
    );
    assert_eq!(sol.stats.rhs_calls, call_count.load(Ordering::SeqCst));
    assert!(
        sol.message.contains("user callback")
            || sol.message.contains("CallbackFailed")
            || sol.message.contains("callback")
            || !sol.message.is_empty()
    );

    // Checked outcome helpers on SwitchingSolution
    assert!(!sol.is_successful_endpoint(1.0));
    let outcome_err = sol.ensure_completed().unwrap_err();
    assert!(!outcome_err.success());
    assert_eq!(outcome_err.final_time(), sol.final_time);
    assert!(outcome_err.accounting_complete());

    // 2. Checked `solve_checked`: returns Err(Unsuccessful(boxed))
    let checked_res = ProductionSolver::solve_checked(&problem, (0.0, 1.0), &[1.0], &config);
    assert!(checked_res.is_err());
    let err = checked_res.unwrap_err();
    assert!(err.is_unsuccessful());
    assert!(!err.is_invalid_request());

    let partial = err.partial_solution().unwrap();
    assert!(!partial.success);
    assert_eq!(partial.final_time, sol.final_time);
    assert!(partial.stats.accounting_complete);
    assert!(partial.stats.rhs_calls > 0);
    assert_eq!(partial.stats.attempts, sol.stats.attempts);
    assert_eq!(partial.t.len(), sol.t.len());
    assert_eq!(partial.y.len(), sol.y.len());
}

#[test]
fn test_max_step_termination_returns_partial_prefix_with_success_false() {
    let problem = Problem::new(1, |_t, y, dy| {
        dy[0] = -y[0];
        Ok(())
    })
    .autonomous();

    // Very low max_steps budget: 5 steps
    let mut opts = default_options();
    opts.max_steps = 5;
    opts.h0 = 1e-4;
    let config = ProductionConfig::rhs_only(opts, 10);

    // 1. Unchecked `solve`: returns Ok(solution) with success == false
    let sol = ProductionSolver::solve(&problem, (0.0, 10.0), &[1.0], &config).unwrap();
    assert!(!sol.success, "exhausted budget must yield success == false");
    assert!(
        sol.final_time < 10.0,
        "solver cannot reach endpoint with 5 steps"
    );
    assert!(sol.stats.attempts <= 5);
    assert!(
        sol.stats.accounting_complete,
        "accounting must remain complete"
    );
    assert!(
        sol.message.contains("budget")
            || sol.message.contains("max_steps")
            || !sol.message.is_empty()
    );

    assert!(!sol.is_successful_endpoint(10.0));
    let outcome_err = sol.ensure_completed().unwrap_err();
    assert!(!outcome_err.success());
    assert_eq!(outcome_err.final_time(), sol.final_time);
    assert!(outcome_err.accounting_complete());

    // 2. Checked `solve_checked`: returns Err(Unsuccessful)
    let checked_res = ProductionSolver::solve_checked(&problem, (0.0, 10.0), &[1.0], &config);
    let err = checked_res.unwrap_err();
    assert!(err.is_unsuccessful());
    let partial = err.partial_solution().unwrap();
    assert!(!partial.success);
    assert_eq!(partial.final_time, sol.final_time);
    assert!(partial.stats.accounting_complete);
    assert!(partial.stats.rhs_calls > 0);
}

#[test]
fn test_no_progress_termination_returns_partial_prefix() {
    // Problem with extreme stiffness under explicit DOPRI5 causing repeated step rejections
    let problem = Problem::new(1, |_t, y, dy| {
        dy[0] = -1e10 * y[0];
        Ok(())
    })
    .autonomous();

    let mut opts = default_options();
    opts.h0 = 1e-3;
    opts.rtol = 1e-14;
    opts.atol = Atol::Scalar(1e-14);
    opts.max_steps = 25;
    let config = ProductionConfig::rhs_only(opts, 5);

    let sol = ProductionSolver::solve(&problem, (0.0, 1.0), &[1.0], &config).unwrap();
    assert!(
        !sol.success,
        "rejected stiff steps must yield success == false under explicit solver"
    );
    assert!(sol.final_time < 1.0);
    assert!(sol.stats.accounting_complete);
    assert!(!sol.is_successful_endpoint(1.0));
    assert!(sol.ensure_completed().is_err());

    let checked_res = ProductionSolver::solve_checked(&problem, (0.0, 1.0), &[1.0], &config);
    let err = checked_res.unwrap_err();
    assert!(err.is_unsuccessful());
    let partial = err.partial_solution().unwrap();
    assert!(!partial.success);
    assert!(partial.stats.accounting_complete);
}

#[test]
fn test_partial_accepted_prefix_inspection() {
    let call_count = Arc::new(AtomicU64::new(0));
    let entered = call_count.clone();
    let problem = Problem::new(2, move |t, y, dy| {
        entered.fetch_add(1, Ordering::SeqCst);
        if t > 0.35 {
            return Err(CallbackError::Fatal("fatal cut at t > 0.35".into()));
        }
        dy[0] = -y[0];
        dy[1] = y[0] - 2.0 * y[1];
        Ok(())
    })
    .autonomous();

    let config = ProductionConfig::rhs_only(default_options(), 8);
    let sol = ProductionSolver::solve(&problem, (0.0, 1.0), &[1.0, 0.5], &config).unwrap();

    // Verify prefix structural invariants:
    assert!(!sol.success);
    assert_eq!(
        sol.t.len(),
        sol.y.len(),
        "t and y dimensions must match in prefix"
    );
    assert!(!sol.t.is_empty(), "must have at least initial point");
    assert_eq!(sol.t[0], 0.0, "first time point must be t0");
    assert_eq!(&sol.y[0], &[1.0, 0.5], "first state must be y0");

    let last_t = *sol.t.last().unwrap();
    assert_eq!(
        last_t, sol.final_time,
        "final_time must match last time in prefix"
    );
    assert_eq!(
        sol.y.last().unwrap(),
        &sol.final_state,
        "final_state must match last state in prefix"
    );

    // Monotonicity of accepted prefix
    for w in sol.t.windows(2) {
        assert!(
            w[0] < w[1],
            "times in prefix must be strictly increasing: {} < {}",
            w[0],
            w[1]
        );
    }

    // WorkCounters inspection
    assert!(sol.stats.accounting_complete);
    assert_eq!(sol.stats.rhs_calls, call_count.load(Ordering::SeqCst));
    assert!(sol.stats.attempts > 0);
    assert!(sol.stats.accepted_steps > 0);

    // Checked outcome helpers
    assert!(!sol.is_successful_endpoint(1.0));
    assert!(!sol.is_successful_endpoint(sol.final_time)); // success is false!
    let outcome_err = sol.ensure_completed().unwrap_err();
    assert!(!outcome_err.success());
    assert_eq!(outcome_err.final_time(), sol.final_time);
    assert!(outcome_err.accounting_complete());
    assert!(!outcome_err.message().is_empty());
}

#[test]
fn test_full_endpoint_success() {
    let problem = Problem::new(2, |t, y, dy| {
        dy[0] = -y[0];
        dy[1] = 2.0 * t;
        Ok(())
    })
    .autonomous();

    let config = ProductionConfig::rhs_only(default_options(), 8);

    // 1. Unchecked `solve`: success == true, accounting_complete == true
    let sol = ProductionSolver::solve(&problem, (0.0, 1.0), &[1.0, 0.0], &config).unwrap();
    assert!(sol.success, "smooth IVP must reach success == true");
    assert_eq!(sol.final_time, 1.0);
    assert!(sol.stats.accounting_complete);
    assert!(sol.stats.rhs_calls > 0);
    assert!(
        sol.message.contains("reached final time")
            || sol.message.contains("reached the final time")
    );

    // Endpoint checks
    assert!(sol.is_successful_endpoint(1.0));
    assert!(
        !sol.is_successful_endpoint(0.5),
        "target mismatch must return false"
    );
    assert!(
        !sol.is_successful_endpoint(1.5),
        "target mismatch must return false"
    );
    assert!(sol.ensure_completed().is_ok());

    // 2. Checked `solve_checked`: returns Ok(solution) directly
    let checked =
        ProductionSolver::solve_checked(&problem, (0.0, 1.0), &[1.0, 0.0], &config).unwrap();
    assert!(checked.success);
    assert_eq!(checked.final_time, 1.0);
    assert!(checked.stats.accounting_complete);
    assert!(checked.is_successful_endpoint(1.0));
    assert!(checked.ensure_completed().is_ok());

    // State accuracy: y[0](1) = e^-1, y[1](1) = 1^2 = 1.0
    let exp_neg_1 = (-1.0f64).exp();
    assert!((checked.final_state[0] - exp_neg_1).abs() < 1e-5);
    assert!((checked.final_state[1] - 1.0).abs() < 1e-5);
}

#[test]
fn test_work_accounting_and_counters_preserved_across_outcomes() {
    // Problem with JVP
    let problem = Problem::new(1, |_t, y, dy| {
        dy[0] = -100.0 * y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_t, _y, v, out| {
        out[0] = -100.0 * v[0];
        Ok(())
    }))
    .autonomous();

    let mut switching = SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly);
    switching.implicit_pilot_attempt_budget = 20;
    switching.strict_entry_work_improvement = true;
    switching.strict_return_work_improvement = true;
    switching.carry_explicit_step = true;
    let config = ProductionConfig::selective_jvp(default_options(), switching, true);

    let sol = ProductionSolver::solve(&problem, (0.0, 1.0), &[1.0], &config).unwrap();
    assert!(sol.success && sol.stats.accounting_complete);
    assert!(sol.stats.rhs_calls > 0);
    assert!(sol.stats.attempts > 0);
    assert_eq!(sol.stats.jacobian_builds, 0); // JVP only, no full jacobian builds

    let checked = ProductionSolver::solve_checked(&problem, (0.0, 1.0), &[1.0], &config).unwrap();
    assert_eq!(sol.stats.rhs_calls, checked.stats.rhs_calls);
    assert_eq!(sol.stats.attempts, checked.stats.attempts);
    assert_eq!(sol.stats.jvp_vectors, checked.stats.jvp_vectors);
    assert_eq!(
        sol.stats.accounting_complete,
        checked.stats.accounting_complete
    );
}

#[test]
fn test_invalid_configuration_rejected_before_callbacks() {
    let call_count = Arc::new(AtomicU64::new(0));
    let entered = call_count.clone();
    let problem = Problem::new(1, move |_t, _y, dy| {
        entered.fetch_add(1, Ordering::SeqCst);
        dy[0] = 1.0;
        Ok(())
    });

    let mut config = ProductionConfig::rhs_only(default_options(), 5);
    config.schema_version = 999; // invalid schema version

    let err = ProductionSolver::solve_checked(&problem, (0.0, 1.0), &[0.0], &config).unwrap_err();
    assert!(err.is_invalid_request());
    assert!(!err.is_unsuccessful());
    assert!(err.partial_solution().is_none());
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        0,
        "callbacks must not be entered on invalid request"
    );
}
