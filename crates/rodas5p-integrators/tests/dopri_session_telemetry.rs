use rodas5p_core::{CoreError, CoreResult};
use rodas5p_integrators::{
    AdaptiveStepConfig, Dopri5Config, Dopri5Session, Dopri5StopReason,
    NativeSessionAdvance as Advance, NativeSessionIdentity as Identity,
    NativeSessionMethod as Method, OutputSchedule,
};

fn decay(_: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
    out[0] = -4.0 * y[0];
    Ok(())
}

#[test]
fn cumulative_telemetry_matches_finish_across_landing_segments() {
    let config = Dopri5Config {
        adaptive: AdaptiveStepConfig {
            initial_step: 0.01,
            rtol: 1e-8,
            atol: 1e-10,
            ..Default::default()
        },
        ..Default::default()
    };
    let schedule = OutputSchedule::new(vec![0.0, 0.07, 0.17, 0.3]).unwrap();
    let id = Identity::new(Method::Dopri5, 1);
    let mut session =
        Dopri5Session::new(&decay, (0.0, 0.3), &[1.0], &config, &schedule, id).unwrap();
    let initial = session.telemetry();
    assert_eq!(initial.rhs_completed, 0);
    assert_eq!(initial.fsal_reuses, 0);
    assert_eq!(initial.output_clipped_steps, 0);
    assert!(initial.stop_reason.is_none());
    assert_eq!(
        session.advance_to(0.07, usize::MAX, id).unwrap(),
        Advance::Checkpoint
    );
    let first = session.telemetry();
    assert!(first.rhs_completed > 0 && first.fsal_reuses > 0);
    assert!(first.output_clipped_steps > 0);
    let checkpoint = session.checkpoint();
    let counter_before = *checkpoint.counters;
    let t_before = checkpoint.t.to_bits();
    let state_before = checkpoint.y[0].to_bits();
    let repeated = session.telemetry();
    assert_eq!(repeated.rhs_completed, first.rhs_completed);
    assert_eq!(repeated.fsal_reuses, first.fsal_reuses);
    assert_eq!(*session.checkpoint().counters, counter_before);
    assert_eq!(session.checkpoint().t.to_bits(), t_before);
    assert_eq!(session.checkpoint().y[0].to_bits(), state_before);
    assert_eq!(
        session.advance_to(0.17, usize::MAX, id).unwrap(),
        Advance::Checkpoint
    );
    let second = session.telemetry();
    assert!(second.output_clipped_steps > first.output_clipped_steps);
    assert_eq!(
        session.advance_to(0.3, usize::MAX, id).unwrap(),
        Advance::Complete
    );
    let final_stats = session.telemetry();
    assert!(final_stats.rhs_completed > first.rhs_completed);
    assert!(final_stats.fsal_reuses > first.fsal_reuses);
    let result = session.finish().unwrap();
    assert_eq!(final_stats.rhs_completed, result.rhs_completed);
    assert_eq!(final_stats.fsal_reuses, result.fsal_reuses);
    assert_eq!(
        final_stats.output_clipped_steps,
        result.observed.output_clipped_steps
    );
    assert_eq!(final_stats.stop_reason, result.stop_reason);
}

#[test]
fn fatal_stop_telemetry_matches_partial_finish() {
    let fatal = |_: f64, _: &[f64], _: &mut [f64]| -> CoreResult<()> {
        Err(CoreError::FatalCallback("fatal telemetry fixture".into()))
    };
    let config = Dopri5Config::default();
    let schedule = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
    let id = Identity::new(Method::Dopri5, 1);
    let mut session =
        Dopri5Session::new(&fatal, (0.0, 1.0), &[1.0], &config, &schedule, id).unwrap();
    assert_eq!(session.advance_to(1.0, 1, id).unwrap(), Advance::Stopped);
    let stats = session.telemetry();
    assert_eq!(stats.stop_reason, Some(Dopri5StopReason::Callback));
    assert_eq!(stats.rhs_completed, 0);
    let result = session.finish().unwrap();
    assert_eq!(stats.stop_reason, result.stop_reason);
    assert_eq!(stats.rhs_completed, result.rhs_completed);
    assert_eq!(stats.fsal_reuses, result.fsal_reuses);
    assert_eq!(
        stats.output_clipped_steps,
        result.observed.output_clipped_steps
    );
}
