use rodas5p_core::{CoreError, CoreResult};
use rodas5p_integrators::{
    AdaptiveStepConfig, Dopri5AttemptErrorNorm as ErrorNorm, Dopri5Config, Dopri5Session,
    NativeSessionAdvance as Advance, NativeSessionIdentity as Identity,
    NativeSessionMethod as Method, OutputSchedule,
};

fn rhs(_: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
    out[0] = -4.0 * y[0];
    Ok(())
}

#[test]
fn attempts_report_h_norm_acceptance_without_changing_state_or_work() {
    let config = Dopri5Config {
        adaptive: AdaptiveStepConfig {
            initial_step: 0.4,
            rtol: 1e-10,
            atol: 1e-12,
            ..Default::default()
        },
        ..Default::default()
    };
    let schedule = OutputSchedule::new(vec![0.0, 0.3, 1.0]).unwrap();
    let id = Identity::new(Method::Dopri5, 1);
    let mut observed =
        Dopri5Session::new(&rhs, (0.0, 1.0), &[1.0], &config, &schedule, id).unwrap();
    let mut plain = Dopri5Session::new(&rhs, (0.0, 1.0), &[1.0], &config, &schedule, id).unwrap();
    assert!(observed.last_attempt().is_none());
    observed.set_attempt_telemetry(true);
    let mut accepted = 0;
    let mut rejected = 0;
    let mut clipped = 0;
    for target in [0.3, 1.0] {
        loop {
            let before = observed.checkpoint();
            let (t_before, y_before, rhs_before) =
                (before.t, before.y[0], before.counters.rhs_calls);
            let result = observed.advance_to(target, 1, id).unwrap();
            assert_eq!(result, plain.advance_to(target, 1, id).unwrap());
            let step = observed.last_attempt().unwrap();
            let after = observed.checkpoint();
            let reference = plain.checkpoint();
            assert!(plain.last_attempt().is_none());
            assert_eq!(step.t_before.to_bits(), t_before.to_bits());
            assert_eq!(step.t_after.to_bits(), after.t.to_bits());
            assert_eq!(step.rhs_delta, after.counters.rhs_calls - rhs_before);
            assert!(step.rhs_delta == 6 || step.rhs_delta == 7);
            assert_eq!(after.t.to_bits(), reference.t.to_bits());
            assert_eq!(after.y[0].to_bits(), reference.y[0].to_bits());
            assert_eq!(after.proposed_h.to_bits(), reference.proposed_h.to_bits());
            assert_eq!(after.counters, reference.counters);
            let ErrorNorm::Finite(norm) = step.error_norm else {
                panic!("finite decay trial");
            };
            assert_eq!(step.accepted, norm <= 1.0);
            assert!(step.trial_h > 0.0 && step.trial_h <= step.requested_h);
            if step.clipped {
                clipped += 1;
                assert!(step.trial_h < step.requested_h);
            }
            if step.accepted {
                accepted += 1;
                assert_eq!(after.t.to_bits(), (t_before + step.trial_h).to_bits());
            } else {
                rejected += 1;
                assert_eq!(after.t.to_bits(), t_before.to_bits());
                assert_eq!(after.y[0].to_bits(), y_before.to_bits());
            }
            if result != Advance::Yield {
                break;
            }
        }
    }
    assert!(accepted > 0 && rejected > 0 && clipped > 0);
    observed.set_attempt_telemetry(false);
    assert!(observed.last_attempt().is_none());
}

#[test]
fn retryable_and_fatal_callbacks_report_actual_attempts() {
    for retryable in [true, false] {
        let callback = |_: f64, _: &[f64], _: &mut [f64]| -> CoreResult<()> {
            if retryable {
                Err(CoreError::NonFinite("retry fixture".into()))
            } else {
                Err(CoreError::FatalCallback("fatal fixture".into()))
            }
        };
        let config = Dopri5Config {
            recover_nonfinite_rhs: true,
            ..Default::default()
        };
        let schedule = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
        let id = Identity::new(Method::Dopri5, 1);
        let mut session =
            Dopri5Session::new(&callback, (0.0, 1.0), &[1.0], &config, &schedule, id).unwrap();
        session.set_attempt_telemetry(true);
        assert_eq!(
            session.advance_to(1.0, 1, id).unwrap(),
            if retryable {
                Advance::Yield
            } else {
                Advance::Stopped
            }
        );
        let step = session.last_attempt().unwrap();
        assert_eq!(
            step.error_norm,
            if retryable {
                ErrorNorm::NonFinite
            } else {
                ErrorNorm::Unavailable
            }
        );
        assert!(!step.accepted);
        assert_eq!(step.t_before, 0.0);
        assert_eq!(step.t_after, 0.0);
        assert_eq!(step.rhs_delta, 1);
        assert_eq!(session.checkpoint().y, &[1.0]);
        assert_eq!(session.checkpoint().attempts, 1);
    }
}

#[test]
fn represented_step_failure_and_zero_quota_do_not_fabricate_attempts() {
    let span = (1e16, 1e16 + 4.0);
    let schedule = OutputSchedule::new(vec![span.0, span.1]).unwrap();
    let config = Dopri5Config::default();
    let id = Identity::new(Method::Dopri5, 1);
    let mut session = Dopri5Session::new(&rhs, span, &[1.0], &config, &schedule, id).unwrap();
    session.set_attempt_telemetry(true);
    assert_eq!(session.advance_to(span.1, 0, id).unwrap(), Advance::Yield);
    assert!(session.last_attempt().is_none());
    assert_eq!(session.advance_to(span.1, 1, id).unwrap(), Advance::Stopped);
    assert!(session.last_attempt().is_none());
    assert_eq!(session.checkpoint().attempts, 0);
    assert_eq!(session.checkpoint().counters.rhs_calls, 0);
}
