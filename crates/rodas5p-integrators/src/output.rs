use rodas5p_core::{CoreError, CoreResult, WorkCounters};

fn time_tolerance(left: f64, right: f64) -> f64 {
    128.0 * f64::EPSILON * left.abs().max(right.abs()).max(1.0)
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutputSchedule {
    times: Vec<f64>,
}

impl OutputSchedule {
    pub fn new(times: Vec<f64>) -> CoreResult<Self> {
        if times.is_empty() || !times.iter().all(|value| value.is_finite()) {
            return Err(CoreError::InvalidInput(
                "output schedule must be finite and nonempty".into(),
            ));
        }
        if times.windows(2).any(|pair| pair[1] <= pair[0]) {
            return Err(CoreError::InvalidInput(
                "output schedule must be strictly increasing".into(),
            ));
        }
        Ok(Self { times })
    }

    pub fn uniform(start: f64, end: f64, spacing: f64) -> CoreResult<Self> {
        if !start.is_finite()
            || !end.is_finite()
            || !spacing.is_finite()
            || end < start
            || spacing <= 0.0
        {
            return Err(CoreError::InvalidInput(
                "invalid uniform output schedule".into(),
            ));
        }
        let span = end - start;
        let intervals = (span / spacing).round() as usize;
        if (start + intervals as f64 * spacing - end).abs() > time_tolerance(start, end) {
            return Err(CoreError::InvalidInput(
                "output spacing must divide the integration interval".into(),
            ));
        }
        let mut times = (0..=intervals)
            .map(|index| start + index as f64 * spacing)
            .collect::<Vec<_>>();
        if let Some(last) = times.last_mut() {
            *last = end;
        }
        Self::new(times)
    }

    pub fn times(&self) -> &[f64] {
        &self.times
    }

    pub(crate) fn validate_span(&self, t0: f64, tf: f64) -> CoreResult<()> {
        let first = *self.times.first().expect("nonempty schedule");
        let last = *self.times.last().expect("nonempty schedule");
        if (first - t0).abs() > time_tolerance(first, t0)
            || (last - tf).abs() > time_tolerance(last, tf)
        {
            return Err(CoreError::InvalidInput(
                "output schedule must include the integration start and end".into(),
            ));
        }
        Ok(())
    }
}

/// Sampling policy for dense-output integrations.
///
/// Requested output times are sampled from accepted intervals and never change
/// their size.  `hard_stops` are the separate, explicit discontinuity or
/// breakpoint landings for which a step may be shortened.  Keeping these two
/// concerns separate prevents an ordinary observation grid from contaminating
/// adaptive-controller history.
#[derive(Clone, Debug, PartialEq)]
pub struct OutputSamplingPlan {
    output: OutputSchedule,
    hard_stops: Vec<f64>,
}

impl OutputSamplingPlan {
    pub fn new(output: OutputSchedule, hard_stops: Vec<f64>) -> CoreResult<Self> {
        if !hard_stops.iter().all(|time| time.is_finite())
            || hard_stops.windows(2).any(|pair| pair[1] <= pair[0])
        {
            return Err(CoreError::InvalidInput(
                "hard stops must be finite and strictly increasing".into(),
            ));
        }
        Ok(Self { output, hard_stops })
    }

    pub fn dense(output: OutputSchedule) -> Self {
        Self {
            output,
            hard_stops: Vec::new(),
        }
    }

    pub fn output(&self) -> &OutputSchedule {
        &self.output
    }

    pub fn hard_stops(&self) -> &[f64] {
        &self.hard_stops
    }

    pub(crate) fn validate_span(&self, t0: f64, tf: f64) -> CoreResult<()> {
        self.output.validate_span(t0, tf)?;
        let tolerance = time_tolerance(t0, tf);
        if self
            .hard_stops
            .iter()
            .any(|time| *time < t0 - tolerance || *time > tf + tolerance)
        {
            return Err(CoreError::InvalidInput(
                "hard stop lies outside the integration span".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct HardStopCursor {
    stops: Vec<f64>,
    next_index: usize,
}

impl HardStopCursor {
    pub(crate) fn new(plan: &OutputSamplingPlan, t_span: (f64, f64)) -> CoreResult<Self> {
        plan.validate_span(t_span.0, t_span.1)?;
        Ok(Self {
            stops: plan.hard_stops.clone(),
            next_index: 0,
        })
    }

    /// Return the actual trial size and whether a hard stop shortened an
    /// otherwise requested step.  Output times are deliberately absent here.
    pub(crate) fn limit_step(
        &mut self,
        t: f64,
        proposed_h: f64,
        tf: f64,
    ) -> CoreResult<(f64, bool)> {
        if !(t.is_finite() && proposed_h.is_finite() && proposed_h > 0.0 && tf.is_finite()) {
            return Err(CoreError::InvalidInput(
                "hard-stop step limit requires finite time and positive step".into(),
            ));
        }
        let base = proposed_h.min(tf - t);
        if base <= 0.0 {
            return Err(CoreError::InvalidInput(
                "hard-stop step limit became nonpositive".into(),
            ));
        }
        while let Some(&stop) = self.stops.get(self.next_index) {
            let tolerance = time_tolerance(t, stop);
            if stop < t - tolerance {
                return Err(CoreError::InvalidInput(
                    "hard-stop cursor advanced past a breakpoint".into(),
                ));
            }
            if stop <= t + tolerance {
                self.next_index += 1;
                continue;
            }
            let to_stop = stop - t;
            if base > to_stop + tolerance {
                return Ok((to_stop, true));
            }
            if (base - to_stop).abs() <= tolerance {
                return Ok((to_stop, false));
            }
            break;
        }
        Ok((base, false))
    }

    /// Consume and report a declared hard stop reached by an accepted step.
    ///
    /// `limit_step`'s boolean records whether scheduling shortened the step,
    /// because controllers need that distinction.  Multistep history instead
    /// cares about the breakpoint identity itself, including the case where a
    /// natural step happens to land there without shortening.
    pub(crate) fn consume_landing(&mut self, t: f64) -> CoreResult<bool> {
        if !t.is_finite() {
            return Err(CoreError::InvalidInput(
                "hard-stop landing time must be finite".into(),
            ));
        }
        let Some(&stop) = self.stops.get(self.next_index) else {
            return Ok(false);
        };
        let tolerance = time_tolerance(t, stop);
        if stop < t - tolerance {
            return Err(CoreError::InvalidInput(
                "hard-stop cursor advanced past a breakpoint".into(),
            ));
        }
        if (stop - t).abs() <= tolerance {
            self.next_index += 1;
            return Ok(true);
        }
        Ok(false)
    }
}

#[derive(Clone, Debug)]
pub struct ObservedIntegrationResult {
    pub t: Vec<f64>,
    pub y: Vec<Vec<f64>>,
    pub success: bool,
    pub message: String,
    pub counters: WorkCounters,
    pub internal_steps: usize,
    pub output_clipped_steps: usize,
}

pub(crate) struct OutputCollector {
    schedule: OutputSchedule,
    next_index: usize,
    times: Vec<f64>,
    states: Vec<Vec<f64>>,
    clipped_steps: usize,
}

impl OutputCollector {
    pub(crate) fn new(
        schedule: &OutputSchedule,
        t_span: (f64, f64),
        y0: &[f64],
    ) -> CoreResult<Self> {
        schedule.validate_span(t_span.0, t_span.1)?;
        if y0.is_empty() || !y0.iter().all(|value| value.is_finite()) {
            return Err(CoreError::InvalidInput(
                "initial state for output collection must be finite and nonempty".into(),
            ));
        }
        Ok(Self {
            schedule: schedule.clone(),
            next_index: 1,
            times: vec![schedule.times[0]],
            states: vec![y0.to_vec()],
            clipped_steps: 0,
        })
    }

    pub(crate) fn limit_step(&self, t: f64, proposed_h: f64, tf: f64) -> CoreResult<(f64, bool)> {
        if !(t.is_finite() && proposed_h.is_finite() && proposed_h > 0.0 && tf.is_finite()) {
            return Err(CoreError::InvalidInput(
                "output-aware step limit requires finite time and positive step".into(),
            ));
        }
        let base = proposed_h.min(tf - t);
        if base <= 0.0 {
            return Err(CoreError::InvalidInput(
                "output-aware step limit became nonpositive".into(),
            ));
        }
        let Some(&next) = self.schedule.times.get(self.next_index) else {
            return Ok((base, false));
        };
        let tolerance = time_tolerance(t, next);
        if next < t - tolerance {
            return Err(CoreError::InvalidInput(
                "output collector advanced past a requested time".into(),
            ));
        }
        let to_next = next - t;
        if to_next <= tolerance {
            return Err(CoreError::InvalidInput(
                "output collector encountered a duplicate requested time".into(),
            ));
        }
        if base > to_next + tolerance {
            Ok((to_next, true))
        } else if (base - to_next).abs() <= tolerance {
            Ok((to_next, false))
        } else {
            Ok((base, false))
        }
    }

    pub(crate) fn accept(&mut self, t: f64, y: &[f64], clipped: bool) -> CoreResult<()> {
        if !t.is_finite() || !y.iter().all(|value| value.is_finite()) {
            return Err(CoreError::NonFinite(
                "accepted output state contains NaN/Inf".into(),
            ));
        }
        let Some(&next) = self.schedule.times.get(self.next_index) else {
            return Ok(());
        };
        let tolerance = time_tolerance(t, next);
        if t > next + tolerance {
            return Err(CoreError::InvalidInput(
                "accepted step overshot a requested output time".into(),
            ));
        }
        if clipped {
            self.clipped_steps += 1;
        }
        if (t - next).abs() <= tolerance {
            self.times.push(next);
            self.states.push(y.to_vec());
            self.next_index += 1;
        }
        Ok(())
    }

    /// Consume every requested time in one accepted dense interval exactly
    /// once.  This intentionally does not update `clipped_steps`: sampling an
    /// already accepted interval is not a step-size policy decision.
    pub(crate) fn accept_dense_interval<F>(
        &mut self,
        t_old: f64,
        t_new: f64,
        y_new: &[f64],
        mut interpolate: F,
    ) -> CoreResult<()>
    where
        F: FnMut(f64) -> CoreResult<Vec<f64>>,
    {
        if !(t_old.is_finite()
            && t_new.is_finite()
            && t_new > t_old
            && y_new.iter().all(|value| value.is_finite()))
        {
            return Err(CoreError::InvalidInput(
                "dense output interval must be finite, increasing, and finite-valued".into(),
            ));
        }
        while let Some(&next) = self.schedule.times.get(self.next_index) {
            let tolerance = time_tolerance(t_new, next);
            if next < t_old - tolerance {
                return Err(CoreError::InvalidInput(
                    "dense output interval begins after a requested time".into(),
                ));
            }
            if next > t_new + tolerance {
                break;
            }
            let theta = if (next - t_new).abs() <= tolerance {
                1.0
            } else if (next - t_old).abs() <= tolerance {
                0.0
            } else {
                (next - t_old) / (t_new - t_old)
            };
            if !(0.0..=1.0).contains(&theta) {
                return Err(CoreError::InvalidInput(
                    "dense output requested time lies outside the accepted interval".into(),
                ));
            }
            let state = if theta == 1.0 {
                y_new.to_vec()
            } else {
                interpolate(theta)?
            };
            if !state.iter().all(|value| value.is_finite()) {
                return Err(CoreError::NonFinite(
                    "dense output state contains NaN/Inf".into(),
                ));
            }
            self.times.push(next);
            self.states.push(state);
            self.next_index += 1;
        }
        Ok(())
    }

    pub(crate) fn finish(self) -> CoreResult<(Vec<f64>, Vec<Vec<f64>>, usize)> {
        if self.next_index != self.schedule.times.len() {
            return Err(CoreError::InvalidInput(
                "integration ended before all requested outputs were recorded".into(),
            ));
        }
        Ok((self.times, self.states, self.clipped_steps))
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.next_index == self.schedule.times.len()
    }

    /// Return the outputs accumulated so far without pretending that the
    /// requested schedule was completed.
    ///
    /// Adaptive research lanes use this on an explicit failure path so the
    /// attempted RHS/JVP/Krylov work and the last committed output remain
    /// auditable.  Successful integrations must continue to use [`finish`],
    /// which enforces complete coverage of the requested schedule.
    pub(crate) fn finish_partial(self) -> (Vec<f64>, Vec<Vec<f64>>, usize) {
        (self.times, self.states, self.clipped_steps)
    }
}
