//! Quartic continuous extension for the Dormand–Prince 5(4) stages.
//!
//! The coefficients are Shampine's continuous extension, as tabulated in
//! SciPy 1.17.0 `scipy.integrate._ivp.rk.RK45.P`. Reference: L. F. Shampine,
//! *Some Practical Runge–Kutta Formulas*, Math. Comp. 46 (1986), 135–150,
//! DOI 10.1090/S0025-5718-1986-0815836-3. The extension has local interpolation error
//! O(h^5) for smooth ODEs; it is not an independently error-controlled dense
//! solution. Sampling does not add RHS evaluations or alter accepted steps.

use rodas5p_core::{CoreError, CoreResult, WorkCounters};

use crate::{
    AdaptiveControllerState, AdaptiveOutputSession, AdaptiveOutputTrial, AdaptiveStepConfig,
    ObservedIntegrationResult, OutputSchedule,
};

/// `y(theta) = y_old + h sum_j k_j sum_{q=1}^4 P[j][q-1] theta^q`.
///
/// This mathematical coefficient table is also exposed for independent
/// order-condition checks. Accepted endpoints are returned by exact copy.
pub const DOPRI5_DENSE_P: [[f64; 4]; 7] = [
    [
        1.0,
        -8048581381.0 / 2820520608.0,
        8663915743.0 / 2820520608.0,
        -12715105075.0 / 11282082432.0,
    ],
    [0.0; 4],
    [
        0.0,
        131558114200.0 / 32700410799.0,
        -68118460800.0 / 10900136933.0,
        87487479700.0 / 32700410799.0,
    ],
    [
        0.0,
        -1754552775.0 / 470086768.0,
        14199869525.0 / 1410260304.0,
        -10690763975.0 / 1880347072.0,
    ],
    [
        0.0,
        127303824393.0 / 49829197408.0,
        -318862633887.0 / 49829197408.0,
        701980252875.0 / 199316789632.0,
    ],
    [
        0.0,
        -282668133.0 / 205662961.0,
        2019193451.0 / 616988883.0,
        -1453857185.0 / 822651844.0,
    ],
    [
        0.0,
        40617522.0 / 29380423.0,
        -110615467.0 / 29380423.0,
        69997945.0 / 29380423.0,
    ],
];

struct DenseSamples {
    schedule: OutputSchedule,
    times: Vec<f64>,
    states: Vec<Vec<f64>>,
    /// Four dimension-sized blocks, including h, reused at accepted steps.
    coefficients: Vec<f64>,
    /// A failed interpolant never publishes a prefix of the current interval.
    pending: Vec<(f64, Vec<f64>)>,
}

impl DenseSamples {
    fn new(schedule: &OutputSchedule, t_span: (f64, f64), y0: &[f64]) -> CoreResult<Self> {
        schedule.validate_span(t_span.0, t_span.1)?;
        let coefficient_count = y0.len().checked_mul(4).ok_or_else(|| {
            CoreError::Dimension("DOPRI5 dense coefficient dimension overflow".into())
        })?;
        Ok(Self {
            schedule: schedule.clone(),
            times: vec![t_span.0],
            states: vec![y0.to_vec()],
            coefficients: vec![0.0; coefficient_count],
            pending: Vec::new(),
        })
    }

    fn prepare(
        &mut self,
        t_old: f64,
        y_old: &[f64],
        t_new: f64,
        y_new: &[f64],
        stages: &[f64],
    ) -> CoreResult<()> {
        self.pending.clear();
        let n = y_old.len();
        if !(t_old.is_finite() && t_new.is_finite() && t_new > t_old)
            || y_new.len() != n
            || stages.len() != 7 * n
            || self.coefficients.len() != 4 * n
        {
            return Err(CoreError::InvalidInput(
                "invalid DOPRI5 dense interval".into(),
            ));
        }
        let next_index = self.times.len();
        let Some(&next) = self.schedule.times().get(next_index) else {
            return Ok(());
        };
        if next <= t_old {
            return Err(CoreError::InvalidInput(
                "DOPRI5 dense interval skipped an output".into(),
            ));
        }
        if next > t_new {
            return Ok(());
        }
        let h = t_new - t_old;
        if next < t_new {
            self.coefficients.fill(0.0);
            // Scale derivatives by h before the O(1) polynomial coefficients;
            // h*P can underflow on represented subnormal intervals.
            for (stage, row) in DOPRI5_DENSE_P.iter().enumerate() {
                for (power, &coefficient) in row.iter().enumerate() {
                    if coefficient != 0.0 {
                        for (q, &k) in self.coefficients[power * n..(power + 1) * n]
                            .iter_mut()
                            .zip(&stages[stage * n..(stage + 1) * n])
                        {
                            *q += coefficient * (h * k);
                        }
                    }
                }
            }
            if self.coefficients.iter().any(|q| !q.is_finite()) {
                return Err(CoreError::NonFinite(
                    "DOPRI5 dense coefficient overflow".into(),
                ));
            }
        }
        for &time in &self.schedule.times()[next_index..] {
            if time > t_new {
                break;
            }
            let state = if time == t_new {
                y_new.to_vec()
            } else {
                // Membership is a physical-time comparison. At a large
                // crossing-zero interval a valid interior theta can round to
                // 1, and on a very wide interval it can underflow to 0.
                let elapsed = time - t_old;
                let remaining = t_new - time;
                let theta = elapsed / h;
                let delta = remaining / h;
                if !(time > t_old
                    && time < t_new
                    && theta.is_finite()
                    && (0.0..=1.0).contains(&theta)
                    && delta.is_finite()
                    && (0.0..=1.0).contains(&delta))
                {
                    return Err(CoreError::InvalidInput(
                        "DOPRI5 dense query outside interval".into(),
                    ));
                }
                (0..n)
                    .map(|i| {
                        let q = &self.coefficients;
                        if theta <= 0.5 {
                            // Use elapsed*k1 directly: theta may underflow even
                            // though the physical first-order increment does not.
                            y_old[i]
                                + elapsed * stages[i]
                                + theta
                                    * theta
                                    * (q[n + i] + theta * (q[2 * n + i] + theta * q[3 * n + i]))
                        } else {
                            // Evaluate the polynomial difference from its accepted
                            // endpoint. delta is computed independently of 1-theta.
                            // The endpoint derivative is k7 by the extension's
                            // exact order conditions; direct remaining*k7 avoids
                            // cancellation in the derivative coefficient sum.
                            let second = q[n + i] + 3.0 * q[2 * n + i] + 6.0 * q[3 * n + i];
                            let third = -q[2 * n + i] - 4.0 * q[3 * n + i];
                            y_new[i] - remaining * stages[6 * n + i]
                                + delta * delta * (second + delta * (third + delta * q[3 * n + i]))
                        }
                    })
                    .collect()
            };
            if state.iter().any(|v| !v.is_finite()) {
                return Err(CoreError::NonFinite("DOPRI5 dense state overflow".into()));
            }
            self.pending.push((time, state));
        }
        Ok(())
    }

    fn commit(&mut self) {
        for (time, state) in self.pending.drain(..) {
            self.times.push(time);
            self.states.push(state);
        }
    }
}

/// Both policies share the same represented-time and controller machinery.
/// In dense mode only the physical span endpoints enter the clock session.
pub(crate) struct Dopri5OutputSession {
    clock: AdaptiveOutputSession,
    dense: Option<DenseSamples>,
}

impl Dopri5OutputSession {
    pub(crate) fn output_clipped_steps(&self) -> usize {
        self.clock.output_clipped_steps()
    }

    pub(crate) fn new(
        adaptive: &AdaptiveStepConfig,
        output: &OutputSchedule,
        t_span: (f64, f64),
        y0: &[f64],
        dense: bool,
    ) -> CoreResult<Self> {
        if !dense {
            return Ok(Self {
                clock: AdaptiveOutputSession::new(adaptive, output, t_span, y0)?,
                dense: None,
            });
        }
        let samples = DenseSamples::new(output, t_span, y0)?;
        let endpoints = OutputSchedule::new(if t_span.0 == t_span.1 {
            vec![t_span.0]
        } else {
            vec![t_span.0, t_span.1]
        })?;
        Ok(Self {
            clock: AdaptiveOutputSession::new(adaptive, &endpoints, t_span, y0)?,
            dense: Some(samples),
        })
    }

    pub(crate) fn trial(
        &mut self,
        t: f64,
        proposed_h: f64,
        controller: &AdaptiveControllerState,
    ) -> CoreResult<Option<AdaptiveOutputTrial>> {
        self.clock.trial(t, proposed_h, controller)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn accept(
        &mut self,
        t_old: f64,
        y_old: &[f64],
        t_new: f64,
        y_new: &[f64],
        stages: &[f64],
        clipped: bool,
    ) -> CoreResult<()> {
        if let Some(dense) = &mut self.dense {
            dense.prepare(t_old, y_old, t_new, y_new, stages)?;
        }
        self.clock.accept(t_new, y_new, clipped)?;
        if let Some(dense) = &mut self.dense {
            dense.commit();
        }
        Ok(())
    }

    pub(crate) fn finish(
        self,
        success: bool,
        message: String,
        counters: WorkCounters,
        internal_steps: usize,
    ) -> CoreResult<ObservedIntegrationResult> {
        let mut observed = self
            .clock
            .finish(success, message, counters, internal_steps)?;
        if let Some(dense) = self.dense {
            if success && dense.times.len() != dense.schedule.times().len() {
                return Err(CoreError::InvalidInput(
                    "DOPRI5 dense output incomplete at success".into(),
                ));
            }
            observed.t = dense.times;
            observed.y = dense.states;
            observed.output_clipped_steps = 0;
        }
        Ok(observed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_dense_preparation_never_publishes_partial_interval() {
        let schedule = OutputSchedule::new(vec![0.0, 0.25, 0.75, 1.0]).unwrap();
        let mut samples = DenseSamples::new(&schedule, (0.0, 1.0), &[0.0]).unwrap();
        // Coefficient formation can fail before any sample is prepared.
        let stages = [f64::MAX; 7];
        assert!(samples.prepare(0.0, &[0.0], 1.0, &[0.0], &stages).is_err());
        assert_eq!(samples.times, vec![0.0]);
        assert_eq!(samples.states, vec![vec![0.0]]);

        // Finite coefficients and a finite first interior sample can still
        // overflow at a later query. Even that prefix remains uncommitted.
        let mut samples = DenseSamples::new(&schedule, (0.0, 1.0), &[0.0]).unwrap();
        let stages = [-1.0e307; 7];
        assert!(
            samples
                .prepare(0.0, &[0.0], 1.0, &[f64::MAX], &stages)
                .is_err()
        );
        assert_eq!(samples.pending.len(), 1);
        assert_eq!(samples.times, vec![0.0]);
        assert_eq!(samples.states, vec![vec![0.0]]);
    }
}
