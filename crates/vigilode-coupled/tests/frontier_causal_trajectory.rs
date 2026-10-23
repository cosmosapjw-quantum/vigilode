use rodas5p_core::{CoreError, LinearMethod, LinearSolverConfig, PreconditionerKind};
use rodas5p_integrators::{AdaptiveStepConfig, OdeProblem, OutputSchedule, ParallelExecution};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use vigilode_coupled::{CausalSchedule, Config, RootErrorPolicy, integrate_causal_observed};

fn linear() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        preconditioner: PreconditionerKind::None,
        atol: 1e-14,
        rtol: 1e-12,
        maxiter: 256,
        restart: 64,
        ..Default::default()
    }
}

#[test]
fn rapid_trajectory_charges_rejections_and_matches_callback_boundary() {
    for mode in [CausalSchedule::Forward, CausalSchedule::BatchedPicard] {
        let rhs = Arc::new(AtomicU64::new(0));
        let jvp = Arc::new(AtomicU64::new(0));
        let rc = rhs.clone();
        let jc = jvp.clone();
        let p = OdeProblem::new(
            "rapid-callback-ledger",
            2,
            Arc::new(move |_, y, f| {
                rc.fetch_add(1, Ordering::SeqCst);
                f[0] = -0.1 * y[0] - 1000. * y[1];
                f[1] = 1000. * y[0] - 0.1 * y[1];
                Ok(())
            }),
            None,
            Some(Arc::new(|_, _| panic!("state Jacobian forbidden"))),
            Some(Arc::new(move |_, _, v, f| {
                jc.fetch_add(1, Ordering::SeqCst);
                f[0] = -0.1 * v[0] - 1000. * v[1];
                f[1] = 1000. * v[0] - 0.1 * v[1];
                Ok(())
            })),
            None,
            true,
            None,
            None,
        )
        .unwrap();
        let a = AdaptiveStepConfig {
            atol: 1e-10,
            rtol: 1e-7,
            initial_step: 0.02,
            max_step: 0.02,
            max_attempts: 2000,
            ..Default::default()
        };
        let output = OutputSchedule::new(vec![0., 0.005, 0.01, 0.02]).unwrap();
        let r = integrate_causal_observed(
            &p,
            (0., 0.02),
            &[1., 0.],
            &a,
            &output,
            &Config::default(),
            &ParallelExecution::sequential(),
            &linear(),
            &RootErrorPolicy::Operational,
            mode,
        )
        .unwrap();
        assert!(r.observed.success, "{}", r.observed.message);
        assert!(r.attempts.iter().any(|a| !a.accepted));
        assert_eq!(r.observed.counters.rhs_calls, rhs.load(Ordering::SeqCst));
        assert_eq!(r.observed.counters.jvp_calls, jvp.load(Ordering::SeqCst));
        let mut accepted_t = 0.;
        for attempt in &r.attempts {
            assert_eq!(attempt.t, accepted_t);
            if attempt.accepted {
                accepted_t += attempt.h;
            }
        }
        assert_eq!(accepted_t, 0.02);
        for (&t, y) in r.observed.t.iter().zip(&r.observed.y) {
            let exact = [
                (-0.1 * t).exp() * (1000. * t).cos(),
                (-0.1 * t).exp() * (1000. * t).sin(),
            ];
            for (got, want) in y.iter().zip(exact) {
                assert!((got - want).abs() < 4e-7, "{mode:?}: {got} {want}");
            }
        }
        assert!(
            r.attempts
                .iter()
                .all(|a| a.candidate_work.stage_jvp == 0 && a.candidate_work.newton_updates == 0)
        );
    }
}

#[test]
fn causal_callback_failure_preserves_accepted_prefix_and_work() {
    for mode in [CausalSchedule::Forward, CausalSchedule::BatchedPicard] {
        let rhs = Arc::new(AtomicU64::new(0));
        let rc = rhs.clone();
        let p = OdeProblem::new(
            "late-domain-failure",
            1,
            Arc::new(move |t, y, f| {
                rc.fetch_add(1, Ordering::SeqCst);
                if t >= 0.02 {
                    return Err(CoreError::InvalidInput("deliberate domain failure".into()));
                }
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
        let a = AdaptiveStepConfig {
            atol: 1e-10,
            rtol: 1e-7,
            initial_step: 0.01,
            max_step: 0.01,
            max_attempts: 100,
            ..Default::default()
        };
        let output = OutputSchedule::new(vec![0., 0.01, 0.02, 0.04]).unwrap();
        let r = integrate_causal_observed(
            &p,
            (0., 0.04),
            &[1.],
            &a,
            &output,
            &Config::default(),
            &ParallelExecution::sequential(),
            &linear(),
            &RootErrorPolicy::Operational,
            mode,
        )
        .unwrap();
        assert!(!r.observed.success);
        assert!(r.terminal_failure.is_some());
        assert_eq!(r.last_t, 0.02);
        assert_eq!(r.observed.t, vec![0., 0.01, 0.02]);
        assert!((r.last_y[0] - (-0.02f64).exp()).abs() < 1e-10);
        assert_eq!(r.observed.counters.rhs_calls, rhs.load(Ordering::SeqCst));
        assert!(!r.attempts.last().unwrap().accepted);
        assert!(!r.attempts.last().unwrap().retryable_failure);
    }
}
