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

    fn validate_span(&self, t0: f64, tf: f64) -> CoreResult<()> {
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

    pub(crate) fn finish(self) -> CoreResult<(Vec<f64>, Vec<Vec<f64>>, usize)> {
        if self.next_index != self.schedule.times.len() {
            return Err(CoreError::InvalidInput(
                "integration ended before all requested outputs were recorded".into(),
            ));
        }
        Ok((self.times, self.states, self.clipped_steps))
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
