use vigilode::{
    Atol, DerivativeFallback, Dopri5, Jacobian, Options, Problem, SwitchingConfig,
    SwitchingDriverStats, SwitchingMethod, SwitchingSolver,
};

fn options() -> Options {
    Options {
        atol: Atol::Scalar(1e-10),
        rtol: 1e-7,
        h0: 0.002,
        max_steps: 50_000,
        ..Default::default()
    }
}

fn smooth() -> Problem {
    Problem::new(1, |_, y, out| {
        out[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, out| {
        out[0] = -1.0;
        Ok(())
    }))
    .autonomous()
}

#[test]
fn pure_explicit_tenure_matches_uninterrupted_observed_run() {
    let problem = smooth();
    let config = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        window_count: 12,
        explicit_attempt_budget: 100,
        pressure_steps: 1000,
        carry_explicit_step: true,
        ..Default::default()
    };
    let times: Vec<_> = (0..=12).map(|i| i as f64 / 12.0).collect();
    let expected =
        Dopri5::solve_observed(&problem, (0.0, 1.0), &[1.0], &options(), &times).unwrap();
    let actual = SwitchingSolver::solve(&problem, (0.0, 1.0), &[1.0], &options(), &config).unwrap();
    assert!(actual.success, "{}", actual.message);
    assert_eq!(
        actual.final_state[0].to_bits(),
        expected.final_state[0].to_bits()
    );
    assert_eq!(actual.stats.attempts, expected.stats.attempts);
    assert_eq!(actual.stats.rhs_calls, expected.stats.counters.rhs_calls);
    assert_eq!(actual.stats.switches, 0);
    assert_eq!(
        actual.stats.attempts,
        actual
            .stats
            .segments
            .iter()
            .map(|s| s.attempts)
            .sum::<usize>()
    );
    assert_eq!(
        actual.stats.rhs_calls,
        actual
            .stats
            .segments
            .iter()
            .map(|s| s.rhs_calls)
            .sum::<u64>()
    );
    let fsal: u64 = actual
        .stats
        .segments
        .iter()
        .map(|s| match &s.driver {
            SwitchingDriverStats::Dopri5(stats) => {
                assert_eq!(stats.attempts, s.attempts);
                assert_eq!(stats.counters.rhs_calls, s.rhs_calls);
                stats.fsal_reuses
            }
            _ => panic!("unexpected implicit segment"),
        })
        .sum();
    assert_eq!(fsal, expected.stats.fsal_reuses);
    assert_eq!(actual.stats.segments.len(), config.window_count);
}

#[test]
fn discarded_implicit_pilot_keeps_native_explicit_session_and_charges_work() {
    let problem = smooth();
    let config = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        window_count: 12,
        explicit_attempt_budget: 3,
        pressure_steps: 1,
        implicit_pilot_attempt_budget: 4,
        implicit_pilot_limit: 2,
        entry_cost_ratio: 0.01,
        carry_explicit_step: true,
        ..Default::default()
    };
    let actual = SwitchingSolver::solve(&problem, (0.0, 1.0), &[1.0], &options(), &config).unwrap();
    assert!(actual.success, "{}", actual.message);
    assert_eq!(actual.stats.switches, 0);
    assert!(
        actual
            .stats
            .segments
            .iter()
            .any(|s| s.is_probe && !s.committed && s.method == SwitchingMethod::Rodas5p)
    );
    assert_eq!(
        actual.stats.attempts,
        actual
            .stats
            .segments
            .iter()
            .map(|s| s.attempts)
            .sum::<usize>()
    );
    assert_eq!(
        actual.stats.rhs_calls,
        actual
            .stats
            .segments
            .iter()
            .map(|s| s.rhs_calls)
            .sum::<u64>()
    );
    let explicit_fsal: u64 = actual
        .stats
        .segments
        .iter()
        .filter_map(|s| match &s.driver {
            SwitchingDriverStats::Dopri5(stats) => Some(stats.fsal_reuses),
            _ => None,
        })
        .sum();
    assert!(explicit_fsal > 0);
}

#[test]
fn committed_implicit_route_reconstructs_explicit_fsal_state() {
    let problem = smooth();
    let config = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        window_count: 12,
        explicit_attempt_budget: 1,
        pressure_steps: 1,
        implicit_residence: 2,
        probe_interval: 2,
        probe_attempt_budget: 16,
        return_cost_ratio: 0.99,
        carry_explicit_step: true,
        ..Default::default()
    };
    let actual = SwitchingSolver::solve(&problem, (0.0, 1.2), &[1.0], &options(), &config).unwrap();
    assert!(actual.success, "{}", actual.message);
    let first_implicit = actual
        .stats
        .segments
        .iter()
        .position(|s| s.committed && s.method == SwitchingMethod::Rodas5p)
        .unwrap();
    let first_explicit_return = actual.stats.segments[first_implicit + 1..]
        .iter()
        .position(|s| s.committed && !s.is_probe && s.method == SwitchingMethod::Dopri5)
        .unwrap()
        + first_implicit
        + 1;
    let segment = &actual.stats.segments[first_explicit_return];
    assert_eq!(segment.attempts, 1);
    match &segment.driver {
        SwitchingDriverStats::Dopri5(stats) => assert_eq!(stats.fsal_reuses, 0),
        _ => panic!("wrong driver"),
    }
    assert_eq!(
        actual.stats.attempts,
        actual
            .stats
            .segments
            .iter()
            .map(|s| s.attempts)
            .sum::<usize>()
    );
}

#[test]
fn accepted_return_probe_at_global_budget_keeps_fully_accounted_checkpoint() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let rhs_calls = Arc::new(AtomicUsize::new(0));
    let rhs_after_checkpoint = Arc::new(AtomicUsize::new(0));
    let calls = Arc::clone(&rhs_calls);
    let late = Arc::clone(&rhs_after_checkpoint);
    let problem = Problem::new(1, move |t, _, out| {
        calls.fetch_add(1, Ordering::Relaxed);
        if t > 0.75 {
            late.fetch_add(1, Ordering::Relaxed);
        }
        out[0] = 0.0;
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, out| {
        out[0] = 0.0;
        Ok(())
    }))
    .autonomous();
    let config = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        window_count: 4,
        explicit_attempt_budget: 1,
        pressure_steps: 1,
        implicit_pilot_attempt_budget: 0,
        implicit_residence: 1,
        probe_interval: 1,
        return_cost_ratio: 0.99,
        carry_explicit_step: true,
        ..Default::default()
    };
    let opts = Options {
        h0: 0.25,
        max_steps: 3,
        ..options()
    };
    let actual = SwitchingSolver::solve(&problem, (0.0, 1.0), &[0.0], &opts, &config).unwrap();
    assert!(!actual.success);
    assert_eq!(actual.final_time, 0.75);
    assert_eq!(actual.final_state, vec![0.0]);
    assert_eq!(actual.stats.attempts, 3);
    assert_eq!(actual.stats.switches, 2);
    assert!(actual.stats.accounting_complete);
    assert!(actual.message.contains("total attempt budget exhausted"));
    assert!(actual.stats.segments.iter().any(|s| s.is_probe
        && s.committed
        && s.method == SwitchingMethod::Dopri5
        && s.reached_time == 0.75));
    assert_eq!(
        actual.stats.attempts,
        actual
            .stats
            .segments
            .iter()
            .map(|s| s.attempts)
            .sum::<usize>()
    );
    assert_eq!(
        actual.stats.rhs_calls,
        actual
            .stats
            .segments
            .iter()
            .map(|s| s.rhs_calls)
            .sum::<u64>()
    );
    assert_eq!(
        rhs_calls.load(Ordering::Relaxed) as u64,
        actual.stats.rhs_calls
    );
    assert_eq!(rhs_after_checkpoint.load(Ordering::Relaxed), 0);
}

#[test]
fn exact_final_time_budget_exhaustion_is_success() {
    let problem = Problem::new(1, |_, _, out| {
        out[0] = 0.0;
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, out| {
        out[0] = 0.0;
        Ok(())
    }))
    .autonomous();
    let config = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        window_count: 4,
        explicit_attempt_budget: 1,
        pressure_steps: usize::MAX,
        carry_explicit_step: true,
        ..Default::default()
    };
    let opts = Options {
        h0: 0.25,
        max_steps: 4,
        ..options()
    };
    let actual = SwitchingSolver::solve(&problem, (0.0, 1.0), &[0.0], &opts, &config).unwrap();
    assert!(actual.success, "{}", actual.message);
    assert_eq!(actual.final_time, 1.0);
    assert_eq!(actual.stats.attempts, 4);
    assert!(actual.stats.accounting_complete);
}

#[test]
fn zero_span_forbidden_retains_committed_zero_work_driver_record() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let problem = Problem::new(1, move |_, _, out| {
        observed.fetch_add(1, Ordering::Relaxed);
        out[0] = 0.0;
        Ok(())
    });
    let actual = SwitchingSolver::solve(
        &problem,
        (2.0, 2.0),
        &[3.0],
        &options(),
        &SwitchingConfig::default(),
    )
    .unwrap();
    assert!(actual.success);
    assert_eq!(actual.final_state, vec![3.0]);
    assert_eq!(actual.stats.attempts, 0);
    assert_eq!(actual.stats.rhs_calls, 0);
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert_eq!(actual.stats.segments.len(), 1);
    let segment = &actual.stats.segments[0];
    assert!(segment.committed && segment.success);
    assert_eq!(segment.method, SwitchingMethod::Dopri5);
    assert_eq!(segment.attempts, 0);
}
