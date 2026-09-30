use rodas5p_core::{CoreError, CoreResult, WorkCounters};

/// How a step is limited by a target time (span end, hard stop or clipped
/// output time).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Landing {
    /// The step to take.
    pub(crate) step: f64,
    /// The step was chosen to land on the target.
    pub(crate) lands: bool,
    /// The proposed step would have passed the target and was shortened.
    pub(crate) shortened: bool,
}

/// Limit a proposed step by `target`.
///
/// A step that would pass the target is shortened to land on it exactly
/// ([`step_to`]); it counts as shortened only when it passes by more than
/// the rounding residue of the time sum, `64 eps max(|t + h|, |target|)`
/// (or an eighth of the step). A step that would stop short of the target
/// by no more than that residue is extended to land on it,
/// so fixed steps of 0.1 reach 1.0 exactly instead of 0.9999999999999999
/// and then taking a 1e-16 step (re-audit R2 review). This is a step-size
/// rule only; which interval owns a requested time is still decided by
/// exact comparison, so adjacent representable requests stay distinct.
pub(crate) fn land(t: f64, proposed: f64, target: f64) -> CoreResult<Landing> {
    let to_target = step_to(t, target)?;
    let natural = t + proposed;
    let residue = 64.0 * f64::EPSILON * natural.abs().max(target.abs());
    if natural > target || proposed >= to_target {
        // Passing the target by no more than the rounding residue is a
        // landing, not a shortened step (0.07 + 0.01 = 0.08000000000000002).
        let overshoot = natural - target;
        return Ok(Landing {
            step: to_target,
            lands: true,
            shortened: overshoot > residue || overshoot > 0.125 * proposed,
        });
    }
    if natural == target {
        return Ok(Landing {
            step: proposed,
            lands: true,
            shortened: false,
        });
    }
    let gap = target - natural;
    let lands = gap <= residue && gap <= 0.125 * proposed;
    Ok(Landing {
        step: if lands { to_target } else { proposed },
        lands,
        shortened: false,
    })
}

/// The step toward the span end `tf` for a proposed step, by [`land`].
pub(crate) fn end_step(t: f64, proposed: f64, tf: f64) -> CoreResult<f64> {
    Ok(land(t, proposed, tf)?.step)
}

/// Slack for checking that a schedule's first and last requested times name
/// the integration span, and that a uniform spacing divides it: a few units
/// in the last place of the larger magnitude, with no absolute floor.
///
/// It is used only for these validations. Which accepted interval owns a
/// requested time, and when an integration is finished, are decided by
/// exact comparisons of represented times (audit R2-OUT-01): a slack there,
/// however small, merges adjacent representable requests and lets the end
/// of a short span at a large epoch count as reached before any step.
fn time_tolerance(left: f64, right: f64) -> f64 {
    (4.0 * f64::EPSILON * left.abs().max(right.abs())).max(f64::MIN_POSITIVE)
}

/// A typed failure for a step that would not move the represented time:
/// `t + h == t` (re-audit R2, R2-OUT-01: a fixed step below half an ULP of
/// `t` used to loop, and an end slack used to report success instead).
pub(crate) fn require_progress(t: f64, h: f64) -> CoreResult<()> {
    if t + h > t {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(format!(
            "step h = {h:e} does not advance the represented time t = {t:e}"
        )))
    }
}

/// A step size `h > 0` with `t + h == target` in binary64, so a step that is
/// meant to land on `target` (the span end, a hard stop or a clipped output
/// time) lands on it exactly, and the drivers can use exact comparisons.
///
/// `target - t` is exact when the two are within a factor of two
/// (Sterbenz); otherwise a few neighbouring step sizes are tried. When no
/// step size lands exactly, the largest tried step that stays below
/// `target` is returned, so a step never overshoots its target. When even
/// that does not advance `t`, the result is a typed error rather than a
/// zero-length step. `target == t` gives 0.
pub(crate) fn step_to(t: f64, target: f64) -> CoreResult<f64> {
    if !(t.is_finite() && target.is_finite()) || target < t {
        return Err(CoreError::InvalidInput(
            "step target must be finite and not before the current time".into(),
        ));
    }
    if target == t {
        return Ok(0.0);
    }
    let mut h = target - t;
    if !h.is_finite() {
        // The distance itself overflows (e.g. -1e308 to 1e308): no single
        // step can land, and every finite proposal stays below the target.
        return Ok(f64::MAX);
    }
    let mut below: Option<f64> = None;
    for _ in 0..8 {
        if !(h.is_finite() && h > 0.0) {
            break;
        }
        let landed = t + h;
        if landed == target {
            return Ok(h);
        }
        if landed < target {
            if landed > t {
                below = Some(below.map_or(h, |best: f64| best.max(h)));
            }
            h = h.next_up();
        } else {
            h = h.next_down();
        }
    }
    below.ok_or_else(|| {
        CoreError::InvalidInput(format!(
            "no binary64 step advances t = {t:e} toward {target:e}"
        ))
    })
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
        // Exact bounds: the cursor compares stops to represented times
        // exactly, so a stop just before t0 could never be consumed.
        if self.hard_stops.iter().any(|time| *time < t0 || *time > tf) {
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
        let base = end_step(t, proposed_h, tf)?;
        if base <= 0.0 {
            return Err(CoreError::InvalidInput(
                "hard-stop step limit became nonpositive".into(),
            ));
        }
        while let Some(&stop) = self.stops.get(self.next_index) {
            if stop < t {
                return Err(CoreError::InvalidInput(
                    "hard-stop cursor advanced past a breakpoint".into(),
                ));
            }
            if stop == t {
                self.next_index += 1;
                continue;
            }
            let landing = land(t, base, stop)?;
            require_progress(t, landing.step)?;
            return Ok((landing.step, landing.shortened));
        }
        require_progress(t, base)?;
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
        if stop < t {
            return Err(CoreError::InvalidInput(
                "hard-stop cursor advanced past a breakpoint".into(),
            ));
        }
        if stop == t {
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
    end: f64,
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
            end: t_span.1,
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
        let base = end_step(t, proposed_h, tf)?;
        if base <= 0.0 {
            return Err(CoreError::InvalidInput(
                "output-aware step limit became nonpositive".into(),
            ));
        }
        let Some(next) = self.due(self.next_index) else {
            require_progress(t, base)?;
            return Ok((base, false));
        };
        if next <= t {
            return Err(CoreError::InvalidInput(
                "output collector advanced past a requested time".into(),
            ));
        }
        let landing = land(t, base, next)?;
        require_progress(t, landing.step)?;
        Ok((landing.step, landing.shortened))
    }

    /// The time at which request `index` is due. The last request is due at
    /// the span end, which it names up to [`time_tolerance`]; every other
    /// request is due at its own represented time.
    fn due(&self, index: usize) -> Option<f64> {
        let time = *self.schedule.times.get(index)?;
        Some(if index + 1 == self.schedule.times.len() {
            self.end
        } else {
            time
        })
    }

    pub(crate) fn accept(&mut self, t: f64, y: &[f64], clipped: bool) -> CoreResult<()> {
        if !t.is_finite() || !y.iter().all(|value| value.is_finite()) {
            return Err(CoreError::NonFinite(
                "accepted output state contains NaN/Inf".into(),
            ));
        }
        let Some(next) = self.due(self.next_index) else {
            return Ok(());
        };
        if t > next {
            return Err(CoreError::InvalidInput(
                "accepted step overshot a requested output time".into(),
            ));
        }
        if clipped {
            self.clipped_steps += 1;
        }
        if t == next {
            self.times.push(self.schedule.times[self.next_index]);
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
        // The interval (t_old, t_new] owns exactly the requests due in it,
        // by exact comparison; t_old's own requests were consumed by the
        // interval that ended there.
        while let Some(next) = self.due(self.next_index) {
            if next <= t_old {
                return Err(CoreError::InvalidInput(
                    "dense output interval begins after a requested time".into(),
                ));
            }
            if next > t_new {
                break;
            }
            let state = if next == t_new {
                y_new.to_vec()
            } else {
                let theta = (next - t_old) / (t_new - t_old);
                if !(theta.is_finite() && theta > 0.0 && theta <= 1.0) {
                    return Err(CoreError::InvalidInput(
                        "dense output requested time lies outside the accepted interval".into(),
                    ));
                }
                interpolate(theta)?
            };
            if !state.iter().all(|value| value.is_finite()) {
                return Err(CoreError::NonFinite(
                    "dense output state contains NaN/Inf".into(),
                ));
            }
            self.times.push(self.schedule.times[self.next_index]);
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
