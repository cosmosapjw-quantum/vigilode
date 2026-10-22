//! Shared clock and endpoint-sampling adapter for external opt-in steppers.
//!
//! This adapter contains no stage solver and makes no accuracy decision. It
//! gives companion crates the same represented-time and output-landing rules
//! as the native adaptive drivers without copying private clock arithmetic.
use crate::output::{OutputCollector, adaptive_end_step, below_min_step};
use crate::{
    AdaptiveControllerState, AdaptiveStepConfig, ObservedIntegrationResult, OutputSchedule,
};
use rodas5p_core::{CoreError, CoreResult, WorkCounters};

/// A represented physical interval, possibly shortened for an observation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptiveOutputTrial {
    /// The represented proposal before observation clipping. Pass this to the
    /// shared controller as `requested_h`, not the previous nominal proposal.
    pub requested_h: f64,
    /// The represented interval which the stage equations must integrate.
    pub h: f64,
    pub clipped: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MaxStepPolicy, rodas_next_step_after_attempt};

    #[test]
    fn adaptive_output_session_refusals_preserve_the_issued_transaction() {
        let adaptive = AdaptiveStepConfig {
            initial_step: 0.1,
            max_step: 0.1,
            ..AdaptiveStepConfig::default()
        };
        let schedule = OutputSchedule::new(vec![0.0, 0.1, 0.5]).unwrap();
        let mut s = AdaptiveOutputSession::new(&adaptive, &schedule, (0.0, 0.5), &[0.0]).unwrap();
        assert!(s.accept(0.1, &[0.1], false).is_err());
        let issued = s
            .trial(0.0, 0.1, &AdaptiveControllerState::default())
            .unwrap()
            .unwrap();
        assert!(s.accept(0.2, &[0.2], issued.clipped).is_err());
        assert!(s.accept(0.1, &[0.1, 0.1], issued.clipped).is_err());
        assert!(s.accept(0.1, &[f64::NAN], issued.clipped).is_err());
        s.accept(0.1, &[0.1], issued.clipped).unwrap();
        assert!(s.accept(0.2, &[0.2], false).is_err());
        let out = s
            .finish(false, "partial".into(), WorkCounters::default(), 1)
            .unwrap();
        assert_eq!(out.t, vec![0.0, 0.1]);
        assert_eq!(out.y, vec![vec![0.0], vec![0.1]]);
    }

    #[test]
    fn adaptive_output_session_large_clock_uses_represented_steps_and_strict_cap() {
        let start = 1e12_f64;
        let ulp = start.next_up() - start;
        let end = start + 4.0 * ulp;
        let schedule =
            OutputSchedule::new((0..=4).map(|i| start + i as f64 * ulp).collect()).unwrap();
        let adaptive = AdaptiveStepConfig {
            initial_step: 1e-4,
            max_step: 1e-4,
            min_step: 1e-10,
            ..AdaptiveStepConfig::default()
        };
        let controller = AdaptiveControllerState::default();
        let mut s = AdaptiveOutputSession::new(&adaptive, &schedule, (start, end), &[0.0]).unwrap();
        let mut t = start;
        let mut y = 0.0;
        for _ in 0..4 {
            let trial = s.trial(t, 1e-4, &controller).unwrap().unwrap();
            assert_eq!(trial.h, ulp);
            t += trial.h;
            y += trial.h;
            s.accept(t, &[y], trial.clipped).unwrap();
        }
        assert!(s.trial(t, 1e-4, &controller).unwrap().is_none());
        let out = s
            .finish(true, "success".into(), WorkCounters::default(), 4)
            .unwrap();
        assert_eq!(out.t, schedule.times());
        assert_eq!(out.y.last().unwrap()[0], end - start);
        let strict = AdaptiveStepConfig {
            max_step_policy: MaxStepPolicy::StrictRepresentedCap,
            ..adaptive
        };
        let mut s = AdaptiveOutputSession::new(&strict, &schedule, (start, end), &[0.0]).unwrap();
        assert!(s.trial(start, 1e-4, &controller).is_err());
        let out = s
            .finish(false, "strict refusal".into(), WorkCounters::default(), 0)
            .unwrap();
        assert_eq!(out.t, vec![start]);
    }

    #[test]
    fn adaptive_output_session_rejected_represented_trial_must_shrink() {
        let start = 1e12_f64;
        let ulp = start.next_up() - start;
        let end = start + 10.0 * ulp;
        let adaptive = AdaptiveStepConfig {
            initial_step: 3.0 * ulp,
            max_step: 3.0 * ulp,
            min_step: ulp * 0.1,
            ..AdaptiveStepConfig::default()
        };
        let mut s = AdaptiveOutputSession::new(
            &adaptive,
            &OutputSchedule::new(vec![start, end]).unwrap(),
            (start, end),
            &[0.0],
        )
        .unwrap();
        let mut controller = AdaptiveControllerState::default();
        let first = s.trial(start, 3.0 * ulp, &controller).unwrap().unwrap();
        rodas_next_step_after_attempt(
            &mut controller,
            &adaptive,
            first.requested_h,
            first.h,
            1.1,
            false,
            first.clipped,
        )
        .unwrap();
        // A nominal 2.55 ULP proposal rounds back to the rejected 3 ULP. The
        // native post-rejection rule instead chooses the preceding clock tick.
        let retry = s.trial(start, 2.55 * ulp, &controller).unwrap().unwrap();
        assert_eq!(first.h, 3.0 * ulp);
        assert_eq!(retry.h, 2.0 * ulp);
        assert!(retry.h < first.h);
    }
}

/// Endpoint-only observation and clock state for an external adaptive driver.
///
/// The caller owns the physical state, trial acceptance and controller. Calling
/// `trial` does not consume any output request. Call `accept` only after every
/// numerical acceptance test passes, and before committing the physical state.
pub struct AdaptiveOutputSession {
    collector: OutputCollector,
    adaptive: AdaptiveStepConfig,
    t_span: (f64, f64),
    committed_t: f64,
    dimension: usize,
    pending: Option<AdaptiveOutputTrial>,
}

impl AdaptiveOutputSession {
    pub fn new(
        adaptive: &AdaptiveStepConfig,
        output: &OutputSchedule,
        t_span: (f64, f64),
        y0: &[f64],
    ) -> CoreResult<Self> {
        adaptive.validate()?;
        if !t_span.0.is_finite() || !t_span.1.is_finite() || t_span.1 < t_span.0 {
            return Err(CoreError::InvalidInput(
                "adaptive output span must be finite and forward".into(),
            ));
        }
        let collector =
            OutputCollector::new(output, t_span, y0)?.with_max_step(adaptive.step_cap());
        Ok(Self {
            collector,
            adaptive: adaptive.clone(),
            t_span,
            committed_t: t_span.0,
            dimension: y0.len(),
            pending: None,
        })
    }

    /// Construct a trial using native maximum/minimum step, rejected-step and
    /// observation-landing rules. `None` means the driver cannot make progress
    /// within its minimum-step policy, or the final time is already reached.
    pub fn trial(
        &mut self,
        t: f64,
        proposed_h: f64,
        controller: &AdaptiveControllerState,
    ) -> CoreResult<Option<AdaptiveOutputTrial>> {
        self.pending = None;
        if t != self.committed_t {
            return Err(CoreError::InvalidInput(
                "adaptive output trial must start at the last committed time".into(),
            ));
        }
        if t == self.t_span.1 {
            return Ok(None);
        }
        let Some(requested_h) = adaptive_end_step(
            t,
            proposed_h,
            self.t_span.1,
            self.adaptive.step_cap(),
            controller,
        )?
        else {
            return Ok(None);
        };
        if below_min_step(t, requested_h, self.adaptive.min_step) && t + requested_h < self.t_span.1
        {
            return Ok(None);
        }
        let (h, clipped) = self.collector.limit_step(t, requested_h, self.t_span.1)?;
        let trial = AdaptiveOutputTrial {
            requested_h,
            h,
            clipped,
        };
        self.pending = Some(trial);
        Ok(Some(trial))
    }

    /// Commit an accepted endpoint to the observation stream. Validation occurs
    /// before the caller should replace its last accepted physical state.
    pub fn accept(&mut self, t: f64, y: &[f64], clipped: bool) -> CoreResult<()> {
        let Some(trial) = self.pending else {
            return Err(CoreError::InvalidInput(
                "accepted adaptive endpoint requires a pending issued trial".into(),
            ));
        };
        if !(t > self.committed_t && t <= self.t_span.1)
            || t != self.committed_t + trial.h
            || clipped != trial.clipped
        {
            return Err(CoreError::InvalidInput(
                "accepted adaptive output must match the issued represented trial".into(),
            ));
        }
        if y.len() != self.dimension {
            return Err(CoreError::Dimension(
                "accepted adaptive endpoint has the wrong dimension".into(),
            ));
        }
        self.collector.accept(t, y, clipped)?;
        self.committed_t = t;
        self.pending = None;
        Ok(())
    }

    /// Close a complete or partial stream without relabelling the last state as
    /// the final requested time. A success claim requires exact final arrival.
    pub fn finish(
        self,
        success: bool,
        message: String,
        counters: WorkCounters,
        internal_steps: usize,
    ) -> CoreResult<ObservedIntegrationResult> {
        if success && self.committed_t != self.t_span.1 {
            return Err(CoreError::InvalidInput(
                "cannot finish adaptive output successfully before the final time".into(),
            ));
        }
        let (t, y, output_clipped_steps) = if success {
            self.collector.finish()?
        } else {
            self.collector.finish_partial()
        };
        Ok(ObservedIntegrationResult {
            t,
            y,
            success,
            message,
            counters,
            internal_steps,
            output_clipped_steps,
        })
    }
}
