use rodas5p_core::{CoreError, CoreResult, WorkCounters};

/// Clock policy of the production drivers (re-audit R3, R3-TIME-01/03):
/// every step integrates the represented interval `t_end - t`
/// ([`represent`]), fixed-step drivers step on the indexed grid
/// `t0 + k h` ([`FixedGrid`]), and the end is reached by exact comparison.
pub const PRODUCTION_CLOCK_POLICY: &str = "represented-indexed-v1";

/// Clock policy of the sealed research replay drivers: the nominal step is
/// integrated, the clock accumulates `t += h`, and the span ends within
/// `10 eps max(|tf|, 1)`. The G4-S5B0 regime atlas, the unified gates and
/// the homotopy experiment and order-policy drivers keep this policy so
/// their sealed evidence stays reproducible; results produced under it are
/// not results of [`PRODUCTION_CLOCK_POLICY`].
pub const RESEARCH_REPLAY_CLOCK_POLICY: &str = "nominal-accumulated-v0";

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
/// the rounding residue of the time sum, `64 eps max(|t|, |t + h|,
/// |target|)` capped at `2^-10 h`. A step that would stop short of the
/// target by no more than that residue is extended to land on it,
/// so fixed steps of 0.1 reach 1.0 exactly instead of 0.9999999999999999
/// and then taking a 1e-16 step (re-audit R2 review). This is a step-size
/// rule only; which interval owns a requested time is still decided by
/// exact comparison, so adjacent representable requests stay distinct.
pub(crate) fn land(t: f64, proposed: f64, target: f64) -> CoreResult<Landing> {
    land_capped(t, proposed, target, StepCap::NONE)
}

/// How an adaptive driver's `max_step` binds the represented step
/// (re-audit R4, R4-TIME-DEV-04).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MaxStepPolicy {
    /// No represented step exceeds `max_step`; when no representable time
    /// after `t` lies within it, the run fails with a typed time-resolution
    /// error.
    StrictRepresentedCap,
    /// The represented step may exceed `max_step` by at most one clock
    /// resolution at its end (`ulp(t + h)`): the rounding of `t + h`
    /// (0.07 + 0.01 represents 0.010000000000000009), and a sub-ULP
    /// `max_step` becomes a full-ULP step. Not a hard maximum. The default,
    /// as before R4.
    #[default]
    AllowClockResolutionSlack,
}

/// A `max_step` and its policy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StepCap {
    pub(crate) max: f64,
    pub(crate) policy: MaxStepPolicy,
}

impl StepCap {
    pub(crate) const NONE: Self = Self {
        max: f64::INFINITY,
        policy: MaxStepPolicy::AllowClockResolutionSlack,
    };

    /// Whether a represented step `h` ending at `end` exceeds the cap
    /// beyond what the policy allows.
    fn exceeded(&self, h: f64, end: f64) -> bool {
        match self.policy {
            MaxStepPolicy::StrictRepresentedCap => h > self.max,
            MaxStepPolicy::AllowClockResolutionSlack => {
                h > self.max && h - self.max > end.next_up() - end
            }
        }
    }
}

/// [`land`] with a hard maximum `cap` on the represented step (an adaptive
/// driver's `max_step`): the rounding residue is never used to stretch a
/// step past `cap`, and the represented step is checked against it
/// ([`represent`]).
pub(crate) fn land_capped(t: f64, proposed: f64, target: f64, cap: StepCap) -> CoreResult<Landing> {
    let mut landing = land_nominal(t, proposed, target, cap)?;
    if landing.step > 0.0 {
        landing.step = represent(t, landing.step, cap)?;
        landing.lands = t + landing.step == target;
    }
    Ok(landing)
}

fn land_nominal(t: f64, proposed: f64, target: f64, cap: StepCap) -> CoreResult<Landing> {
    let to_target = step_to(t, target)?;
    let natural = t + proposed;
    // Rounding residue of the time sum: relative to the largest magnitude
    // involved, including t, so it does not collapse for a target near zero
    // reached from t < 0 (-0.3 + 3 * 0.1 is 5.6e-17, not 0), and at most
    // 2^-10 of the step, so a step is never stretched or shortened silently
    // by more than 0.1% at a large epoch.
    let residue = (64.0 * f64::EPSILON * t.abs().max(natural.abs()).max(target.abs()))
        .min(proposed * 2.0_f64.powi(-10));
    if natural > target || proposed >= to_target {
        // Passing the target by no more than the residue is a landing, not
        // a shortened step (0.07 + 0.01 = 0.08000000000000002).
        return Ok(Landing {
            step: to_target,
            lands: t + to_target == target,
            shortened: natural - target > residue,
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
    // The extension respects a hard maximum up to one clock resolution at
    // the target, like `represent`.
    let extend = gap <= residue && !cap.exceeded(to_target, target);
    Ok(Landing {
        step: if extend { to_target } else { proposed },
        lands: extend && t + to_target == target,
        shortened: false,
    })
}

/// The step toward the span end `tf` for a proposed step, by [`land`].
pub(crate) fn end_step(t: f64, proposed: f64, tf: f64) -> CoreResult<f64> {
    Ok(land(t, proposed, tf)?.step)
}

/// [`end_step`] with a hard maximum on the represented step.
pub(crate) fn end_step_capped(t: f64, proposed: f64, tf: f64, cap: StepCap) -> CoreResult<f64> {
    Ok(land_capped(t, proposed, tf, cap)?.step)
}

/// The step the stage equations must use for a step of nominal size `step`
/// from `t` (re-audit R3, R3-TIME-01).
///
/// A stepper moves the clock to `t_end = t + step` rounded, but used to
/// integrate the nominal `step`; at `t = 1e12` a step of `1e-4` moved the
/// clock by one ULP (`1.22e-4`) and integrated `1e-4`, so eight steps over
/// an 8-ULP span integrated 0.8192 of it. The returned `h` has
/// `t + h == t_end` and is the represented interval `t_end - t` (exact
/// when `t` and `t_end` are within a factor of two, otherwise within a few
/// roundings of `h`, checked by [`check_clock`]), so the stage equations,
/// dense output, error estimate and controller history all see the
/// interval the clock records.
///
/// A represented step above `cap` is replaced by the one ending at the
/// representable time below `t_end`; when that does not advance `t`, the
/// time resolution at `t` is coarser than `cap` and the result is a typed
/// error instead of a silently enlarged step.
pub(crate) fn represent(t: f64, step: f64, cap: StepCap) -> CoreResult<f64> {
    if !(t.is_finite() && step.is_finite() && step > 0.0) {
        return Err(CoreError::InvalidInput(format!(
            "time resolution: step {step:e} at t = {t:e} must be finite and positive"
        )));
    }
    let t_end = t + step;
    if !(t_end > t && t_end.is_finite()) {
        return Err(CoreError::InvalidInput(format!(
            "time resolution: step h = {step:e} does not advance the represented time t = {t:e}"
        )));
    }
    let mut h = step_to(t, t_end)?;
    // The represented step may exceed the nominal one by the rounding of
    // t_end, up to one clock resolution at t_end; that is representation,
    // not enlargement (an exact 0.07 + 0.01 == 0.08 landing has
    // h = 0.010000000000000009). Only a larger excess is capped.
    if cap.exceeded(h, t_end) {
        let below = t_end.next_down();
        if below <= t {
            return Err(CoreError::InvalidInput(format!(
                "time resolution: the next representable time after t = {t:e} is {:e} away, \
                 above the maximum step {:e} ({:?})",
                t_end - t,
                cap.max,
                cap.policy
            )));
        }
        h = step_to(t, below)?;
        // Strict: one step below may still be above a sub-ULP cap.
        if cap.exceeded(h, below) {
            return Err(CoreError::InvalidInput(format!(
                "time resolution: no representable time after t = {t:e} within the maximum step {:e} ({:?})",
                cap.max, cap.policy
            )));
        }
    }
    check_clock(t, h)?;
    Ok(h)
}

/// True when a represented step `h` from `t` is below `min_step`: it ends
/// before the represented time `t + min_step` (re-audit R3 review). Comparing
/// the step sizes themselves would reject `0.02 -> 0.03`, whose represented
/// step 0.009999999999999998 rounds below a `min_step` of 0.01.
pub(crate) fn below_min_step(t: f64, h: f64, min_step: f64) -> bool {
    t + h < t + min_step
}

/// The adaptive drivers' step toward `tf`: `None` when the proposal does not
/// advance the represented time at all (the driver stops with
/// `success = false`, as before the represented clock), otherwise the
/// represented step, or a typed time-resolution error when the time
/// resolution at `t` exceeds `max_step`.
///
/// After a rejected attempt from the same `t` (re-audit R4, TIME-DEV-02),
/// the represented step is strictly shorter than the rejected one: at a
/// coarse clock a proposal of 2.55 ULPs represents as the rejected 3 ULPs
/// again, and the drivers used to retry the identical step until
/// `max_attempts`. The step ends at the representable time below the
/// rejected end instead, or the run fails with a typed time-resolution
/// error when there is none after `t`.
pub(crate) fn adaptive_end_step(
    t: f64,
    proposed: f64,
    tf: f64,
    cap: StepCap,
    controller: &crate::AdaptiveControllerState,
) -> CoreResult<Option<f64>> {
    if cap.policy == MaxStepPolicy::StrictRepresentedCap && t.next_up() - t > cap.max {
        return Err(CoreError::InvalidInput(format!(
            "time resolution: no representable time after t = {t:e} within the strict maximum step {:e}",
            cap.max
        )));
    }
    let nominal = proposed.min(cap.max);
    if nominal.is_nan() || nominal <= 0.0 || t + nominal <= t {
        return Ok(None);
    }
    let step = end_step_capped(t, nominal, tf, cap)?;
    match controller.last_rejected_trial() {
        Some(rejected) if step >= rejected => {
            let below = (t + rejected).next_down();
            if below <= t {
                return Err(CoreError::InvalidInput(format!(
                    "time resolution: the rejected step {rejected:e} at t = {t:e} is one clock \
                     resolution and cannot be shortened"
                )));
            }
            let shorter = step_to(t, below)?;
            check_clock(t, shorter)?;
            Ok(Some(shorter))
        }
        _ => Ok(Some(step)),
    }
}

/// A typed error unless the represented interval `fl(t + h) - t` equals `h`
/// to within `4 eps h` (exact residual of the sum by TwoSum).
pub(crate) fn check_clock(t: f64, h: f64) -> CoreResult<()> {
    let sum = t + h;
    let virtual_h = sum - t;
    let residual = (t - (sum - virtual_h)) + (h - virtual_h);
    if sum > t && residual.abs() <= 4.0 * f64::EPSILON * h {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(format!(
            "inconsistent clock: step h = {h:e} at t = {t:e} moves the clock by {:e}",
            h - residual
        )))
    }
}

/// Split a represented step `h` from `t` into two represented halves for
/// step doubling (re-audit R3, R3-TIME-01: the second half used to start at
/// `t + h/2` rounded and integrate `h/2`, which need not end at `t + h`).
/// Returns `(h1, t_mid, h2)` with `t + h1 == t_mid`, `t_mid + h2 == t + h`
/// and `t < t_mid < t + h`, or a typed error when the time resolution at `t`
/// cannot hold a midpoint.
pub(crate) fn split_clock(t: f64, h: f64) -> CoreResult<(f64, f64, f64)> {
    let t_end = t + h;
    let t_mid = t + 0.5 * h;
    if !(t < t_mid && t_mid < t_end) {
        return Err(CoreError::InvalidInput(format!(
            "time resolution: no representable midpoint in ({t:e}, {t_end:e}) for step doubling"
        )));
    }
    let h1 = step_to(t, t_mid)?;
    let h2 = step_to(t_mid, t_end)?;
    if t + h1 != t_mid || t_mid + h2 != t_end {
        return Err(CoreError::InvalidInput(format!(
            "inconsistent clock: step-doubling halves of h = {h:e} at t = {t:e} do not tile the step"
        )));
    }
    check_clock(t, h1)?;
    check_clock(t_mid, h2)?;
    Ok((h1, t_mid, h2))
}

/// The time grid of a fixed-step integration (re-audit R3, R3-TIME-03).
///
/// Step `k` ends at the indexed time `t0 + k h`, formed with one rounding,
/// and the last step ends at `tf` exactly; the clock never accumulates
/// `t += h`. The count is `round(span / h)` when that many steps reach `tf`
/// to within the rounding residue of the grid (at most `64 eps` of the
/// largest magnitude and `h / 1024`), otherwise `ceil(span / h)` with a
/// shorter final step. 1000 steps of 0.01 on (0, 10) are 1000 steps; the
/// accumulated clock used to end 1.7e-13 short and add a micro-step.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FixedGrid {
    t0: f64,
    tf: f64,
    h: f64,
    steps: u64,
}

impl FixedGrid {
    pub(crate) fn new(t0: f64, tf: f64, h: f64) -> CoreResult<Self> {
        if !(t0.is_finite() && tf.is_finite() && h.is_finite() && h > 0.0 && tf >= t0) {
            return Err(CoreError::InvalidInput(
                "fixed-step grid needs finite t0 <= tf and a finite positive step".into(),
            ));
        }
        let span = tf - t0;
        if !span.is_finite() {
            return Err(CoreError::InvalidInput(
                "fixed-step grid span overflows binary64".into(),
            ));
        }
        let ratio = span / h;
        if ratio > 2.0_f64.powi(52) {
            return Err(CoreError::InvalidInput(format!(
                "fixed-step grid of {ratio:e} steps is beyond the indexed range"
            )));
        }
        let rounded = ratio.round();
        let residue = (64.0 * f64::EPSILON * t0.abs().max(tf.abs()).max(span)).min(h / 1024.0);
        let steps = if span == 0.0 {
            0
        } else if rounded >= 1.0 && (rounded * h - span).abs() <= residue {
            rounded as u64
        } else {
            ratio.ceil().max(1.0) as u64
        };
        Ok(Self { t0, tf, h, steps })
    }

    fn reached_within_residue(&self, t: f64, point: f64, k: u64) -> bool {
        k < self.steps
            && point - t <= (64.0 * f64::EPSILON * t.abs().max(point.abs())).min(self.h / 1024.0)
    }

    /// The indexed time of grid point `k`: `t0 + k h`, and `tf` at the end.
    pub(crate) fn time(&self, k: u64) -> f64 {
        if k >= self.steps {
            self.tf
        } else {
            (self.t0 + k as f64 * self.h).min(self.tf)
        }
    }

    /// The nominal step from `t` to the first grid point after it. Grid
    /// points that round to or below `t` are skipped (several indexed times
    /// can round to one represented time when `h` is below one ULP but above
    /// half of one); a clipped step inside an interval returns to its grid
    /// point. A nominal step that does not advance `t` at all is a typed
    /// time-resolution error.
    pub(crate) fn step_from(&self, t: f64) -> CoreResult<f64> {
        if t >= self.tf {
            return Err(CoreError::InvalidInput(
                "fixed-step grid is already at its end".into(),
            ));
        }
        // A nominal step below half an ULP of t is below the time
        // resolution; skipping grid points would silently enlarge it.
        require_progress(t, self.h)?;
        let estimate = ((t - self.t0) / self.h).floor();
        let mut k = if estimate.is_finite() && estimate > 0.0 {
            (estimate as u64).min(self.steps)
        } else {
            0
        };
        while k > 0 && self.time(k) > t {
            k -= 1;
        }
        // A grid point within the rounding residue after t counts as
        // reached: after a clipped landing on a literal output 0.3, the
        // indexed point 0.1 * 3 = 0.30000000000000004 is not a step of its
        // own (re-audit R3 review).
        while self.time(k) <= t || self.reached_within_residue(t, self.time(k), k) {
            k += 1;
        }
        step_to(t, self.time(k))
    }
}

/// A typed failure for a step that would not move the represented time:
/// `t + h == t` (re-audit R2, R2-OUT-01: a fixed step below half an ULP of
/// `t` used to loop, and an end slack used to report success instead).
pub(crate) fn require_progress(t: f64, h: f64) -> CoreResult<()> {
    if t + h > t {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(format!(
            "time resolution: step h = {h:e} does not advance the represented time t = {t:e}"
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
        // At t0 = 1e12 a 4-ULP span with spacing 1 has zero intervals and
        // was accepted by an epoch-scaled slack, giving the schedule [tf] and
        // y0 reported as y(tf) (re-audit R3, R3-TIME-02).
        // The tolerance is the rounding of the inputs themselves (a few eps
        // of the largest magnitude, so decimal grids such as
        // uniform(1.1, 1.2, 0.1) pass) but never more than 2^-10 of the
        // spacing, and a nonzero span needs at least one interval.
        let tolerance =
            (8.0 * f64::EPSILON * start.abs().max(end.abs()).max(span)).min(spacing / 1024.0);
        if (span > 0.0 && intervals == 0) || (intervals as f64 * spacing - span).abs() > tolerance {
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

    /// The schedule must name the span's represented endpoints exactly, and
    /// a nonzero span needs both of them as distinct requests (re-audit R3,
    /// R3-TIME-02: `[tf]` for `t0 < tf` passed as both endpoints and
    /// returned the initial state labelled as the final time; endpoints a
    /// few ULP off were accepted and relabelled).
    pub(crate) fn validate_span(&self, t0: f64, tf: f64) -> CoreResult<()> {
        let first = *self.times.first().expect("nonempty schedule");
        let last = *self.times.last().expect("nonempty schedule");
        if first != t0 || last != tf {
            return Err(CoreError::InvalidInput(format!(
                "output schedule must start at t0 = {t0:e} and end at tf = {tf:e} exactly \
                 (got {first:e} .. {last:e})"
            )));
        }
        if tf > t0 && self.times.len() < 2 {
            return Err(CoreError::InvalidInput(
                "a nonzero integration span needs distinct start and end requests".into(),
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
    max_step: StepCap,
}

impl HardStopCursor {
    pub(crate) fn new(plan: &OutputSamplingPlan, t_span: (f64, f64)) -> CoreResult<Self> {
        plan.validate_span(t_span.0, t_span.1)?;
        Ok(Self {
            stops: plan.hard_stops.clone(),
            next_index: 0,
            max_step: StepCap::NONE,
        })
    }

    /// Hard maximum on represented steps (an adaptive driver's `max_step`).
    pub(crate) fn with_max_step(mut self, max_step: StepCap) -> Self {
        self.max_step = max_step;
        self
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
        let base = end_step_capped(t, proposed_h, tf, self.max_step)?;
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
            let landing = land_capped(t, base, stop, self.max_step)?;
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
    max_step: StepCap,
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
            max_step: StepCap::NONE,
        })
    }

    /// Hard maximum on represented steps (an adaptive driver's `max_step`).
    pub(crate) fn with_max_step(mut self, max_step: StepCap) -> Self {
        self.max_step = max_step;
        self
    }

    pub(crate) fn limit_step(&self, t: f64, proposed_h: f64, tf: f64) -> CoreResult<(f64, bool)> {
        if !(t.is_finite() && proposed_h.is_finite() && proposed_h > 0.0 && tf.is_finite()) {
            return Err(CoreError::InvalidInput(
                "output-aware step limit requires finite time and positive step".into(),
            ));
        }
        let base = end_step_capped(t, proposed_h, tf, self.max_step)?;
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
        let landing = land_capped(t, base, next, self.max_step)?;
        require_progress(t, landing.step)?;
        Ok((landing.step, landing.shortened))
    }

    /// The time at which request `index` is due: its own represented time
    /// (the last request is the span end exactly, [`OutputSchedule::validate_span`]).
    fn due(&self, index: usize) -> Option<f64> {
        let time = *self.schedule.times.get(index)?;
        debug_assert!(index + 1 < self.schedule.times.len() || time == self.end);
        Some(time)
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
