//! Exact two-channel raw-U reduction for an immutable affine physical model.
//!
//! `f(y) = b - d*y + k*mean(y)*1`, with `d >= k >= 0`. Model coefficients
//! are supplied directly. This path never requests a derivative callback,
//! assembles or factors a state matrix, or performs a Newton iteration.
//! All initial and forcing components survive in the transverse channel.
//! Shared tableau initialization does perform one fixed 8-by-8 coefficient
//! inversion in the process-lifetime cache. That setup is not a state solve
//! and is outside the per-step work counters below.
//!
//! The original eight-stage RODAS5P target is unchanged. Two scalar triangular
//! recurrences replace repeated full-state stage construction. Every returned
//! candidate still undergoes an explicit original-space residual screen; that
//! screen's work is included in the report. The screen is an operational f64
//! measurement, not an interval enclosure of rounding error.

use rodas5p_core::{CoreError, CoreResult, rodas5p_coefficients};
use rodas5p_integrators::ParallelExecution;
use serde::Serialize;

const S: usize = 8;

#[derive(Clone, Debug)]
pub struct MeanFieldAffine {
    decay: f64,
    coupling: f64,
    forcing: Vec<f64>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct StructuralWork {
    pub rhs_evaluations: u64,
    pub physical_linear_actions: u64,
    pub shifted_resolvent_actions: u64,
    pub scalar_stage_solves: u64,
    pub full_state_causal_accumulations: u64,
    pub lifted_components: u64,
    pub original_space_residual_components: u64,
    pub jacobian_evaluations: u64,
    pub jvp_evaluations: u64,
    /// State-matrix factorizations during this step. The shared fixed-tableau
    /// initialization is not counted as step work.
    pub matrix_factorizations: u64,
    pub newton_updates: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct StructuralCandidate {
    pub y_new: Vec<f64>,
    pub stages_u: Vec<Vec<f64>>,
    pub error_vector: Vec<f64>,
    pub full_residual_linf: f64,
    pub work: StructuralWork,
}

/// A reusable h/rate-only plan. It is independent of state and forcing.
#[derive(Clone, Debug)]
pub struct MeanFieldPlan {
    decay: f64,
    coupling: f64,
    h: f64,
    transverse: [f64; S],
    collective: [f64; S],
}

/// An algebraically derived residual budget for a fixed-step trajectory.
///
/// These are ordinary f64 evaluations of exact real-arithmetic formulas, not
/// outward-rounded certificates. The bound concerns algebraic stage residuals;
/// time discretization, residual evaluation, and output rounding have separate
/// error contributions. No empirical safety multiplier is inserted.
#[derive(Clone, Debug, Serialize)]
pub struct AlgebraicResidualBudget {
    pub dimension: usize,
    pub steps: usize,
    pub requested_algebraic_error: f64,
    pub output_amplification: f64,
    pub step_amplification: f64,
    pub propagation_sum: f64,
    /// Sharp norm from all step/stage residuals to the last discrete output.
    pub global_residual_amplification: f64,
    pub residual_limit: f64,
    /// Two scalar adjoint recurrences, independent of the state dimension.
    pub scalar_bound_solves: u64,
}

fn finite(x: &[f64]) -> CoreResult<()> {
    if x.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(CoreError::NonFinite(
            "structural mean-field input/output".into(),
        ))
    }
}

// Dividing first prevents an otherwise finite mean from overflowing its sum.
// Neumaier compensation improves cancellation; it is not an interval bound.
fn mean(x: &[f64]) -> f64 {
    let scale = 1.0 / x.len() as f64;
    let (mut sum, mut correction) = (0.0_f64, 0.0_f64);
    for &v in x {
        let value = v * scale;
        let next = sum + value;
        correction += if sum.abs() >= value.abs() {
            (sum - next) + value
        } else {
            (value - next) + sum
        };
        sum = next;
    }
    sum + correction
}

impl MeanFieldAffine {
    pub fn new(decay: f64, coupling: f64, forcing: Vec<f64>) -> CoreResult<Self> {
        if !decay.is_finite()
            || !coupling.is_finite()
            || coupling < 0.0
            || decay < coupling
            || forcing.is_empty()
        {
            return Err(CoreError::InvalidInput(
                "require finite d >= k >= 0 and nonempty forcing".into(),
            ));
        }
        finite(&forcing)?;
        Ok(Self {
            decay,
            coupling,
            forcing,
        })
    }

    pub fn dimension(&self) -> usize {
        self.forcing.len()
    }

    fn validate_state(&self, y: &[f64]) -> CoreResult<()> {
        if y.len() != self.dimension() {
            return Err(CoreError::Dimension("structural mean-field state".into()));
        }
        finite(y)
    }

    pub fn rhs_into(&self, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        self.validate_state(y)?;
        if out.len() != y.len() {
            return Err(CoreError::Dimension(
                "structural mean-field RHS output".into(),
            ));
        }
        let shared = self.coupling * mean(y);
        for ((result, state), source) in out.iter_mut().zip(y).zip(&self.forcing) {
            *result = source - self.decay * state + shared;
        }
        finite(out)
    }

    fn apply_physical_linear(&self, x: &[f64]) -> CoreResult<Vec<f64>> {
        let shared = self.coupling * mean(x);
        let out: Vec<_> = x.iter().map(|v| -self.decay * v + shared).collect();
        finite(&out)?;
        Ok(out)
    }

    fn resolvent(&self, rhs: &[f64], hg: f64) -> CoreResult<Vec<f64>> {
        let mu = mean(rhs);
        let dt = 1.0 + hg * self.decay;
        let dc = 1.0 + hg * (self.decay - self.coupling);
        if !dt.is_finite() || !dc.is_finite() || dt <= 0.0 || dc <= 0.0 {
            return Err(CoreError::NonFinite(
                "structural shifted denominator".into(),
            ));
        }
        let out: Vec<_> = rhs.iter().map(|v| (v - mu) / dt + mu / dc).collect();
        finite(&out)?;
        Ok(out)
    }

    pub fn plan(&self, h: f64) -> CoreResult<MeanFieldPlan> {
        if !h.is_finite() || h <= 0.0 {
            return Err(CoreError::InvalidInput(
                "positive finite structural step required".into(),
            ));
        }
        let c = rodas5p_coefficients()?;
        let hg = h * c.gamma;
        if !hg.is_finite() || hg <= 0.0 {
            return Err(CoreError::NonFinite("structural h*gamma".into()));
        }
        let coefficients = |rate: f64| -> CoreResult<[f64; S]> {
            let shifted_rate = hg * rate;
            let denominator = 1.0 + shifted_rate;
            if !denominator.is_finite() || denominator <= 0.0 {
                return Err(CoreError::NonFinite("structural plan denominator".into()));
            }
            let mut transfer = [0.0; S];
            for i in 0..S {
                let mut rhs = hg;
                for (j, previous_transfer) in transfer.iter().take(i).enumerate() {
                    rhs += (c.gamma * c.c_matrix[(i, j)] - shifted_rate * c.a[(i, j)])
                        * *previous_transfer;
                }
                transfer[i] = rhs / denominator;
            }
            finite(&transfer)?;
            Ok(transfer)
        };
        Ok(MeanFieldPlan {
            decay: self.decay,
            coupling: self.coupling,
            h,
            transverse: coefficients(self.decay)?,
            collective: coefficients(self.decay - self.coupling)?,
        })
    }

    /// Strong derivative-free baseline: direct sequential original-stage
    /// elimination with the same analytic resolvent and the same final screen.
    pub fn sequential_step(
        &self,
        y: &[f64],
        h: f64,
        residual_limit: f64,
        execution: &ParallelExecution,
    ) -> CoreResult<StructuralCandidate> {
        self.validate_state(y)?;
        // The plan validates h/rates; this baseline does not use its transfers.
        validate_screen(residual_limit)?;
        if !h.is_finite() || h <= 0.0 {
            return Err(CoreError::InvalidInput("invalid structural h".into()));
        }
        let c = rodas5p_coefficients()?;
        let hg = h * c.gamma;
        if !hg.is_finite() || hg <= 0.0 {
            return Err(CoreError::NonFinite("structural h*gamma".into()));
        }
        let n = y.len();
        let mut stages = Vec::<Vec<f64>>::with_capacity(S);
        let mut work = StructuralWork::default();
        for i in 0..S {
            let mut state = y.to_vec();
            let mut mix = vec![0.0; n];
            for (j, previous_stage) in stages.iter().take(i).enumerate() {
                for k in 0..n {
                    state[k] += c.a[(i, j)] * previous_stage[k];
                    mix[k] += c.c_matrix[(i, j)] * previous_stage[k];
                }
                work.full_state_causal_accumulations += (2 * n) as u64;
            }
            let mut rhs = vec![0.0; n];
            self.rhs_into(&state, &mut rhs)?;
            work.rhs_evaluations += 1;
            for k in 0..n {
                rhs[k] = hg * rhs[k] + c.gamma * mix[k];
            }
            stages.push(self.resolvent(&rhs, hg)?);
            work.shifted_resolvent_actions += 1;
        }
        self.finish(y, h, stages, work, residual_limit, execution)
    }

    fn finish(
        &self,
        y: &[f64],
        h: f64,
        stages: Vec<Vec<f64>>,
        mut work: StructuralWork,
        residual_limit: f64,
        execution: &ParallelExecution,
    ) -> CoreResult<StructuralCandidate> {
        let c = rodas5p_coefficients()?;
        let hg = h * c.gamma;
        let n = y.len();
        let mut y_new = y.to_vec();
        for (i, stage) in stages.iter().enumerate().take(S) {
            for k in 0..n {
                y_new[k] += c.b_code[i] * stage[k];
            }
        }
        // The raw-U eighth stage is the embedded correction in this tableau.
        let error_vector = stages[S - 1].clone();
        finite(&y_new)?;
        let residuals = execution.map_ordered(&(0..S).collect::<Vec<_>>(), |&i| {
            let mut state = y.to_vec();
            let mut mix = vec![0.0; n];
            for (j, previous_stage) in stages.iter().take(i).enumerate() {
                for k in 0..n {
                    state[k] += c.a[(i, j)] * previous_stage[k];
                    mix[k] += c.c_matrix[(i, j)] * previous_stage[k];
                }
            }
            let mut fy = vec![0.0; n];
            self.rhs_into(&state, &mut fy)?;
            let ju = self.apply_physical_linear(&stages[i])?;
            let mut maximum = 0.0_f64;
            for k in 0..n {
                let residual = stages[i][k] - hg * ju[k] - c.gamma * mix[k] - hg * fy[k];
                if !residual.is_finite() {
                    return Err(CoreError::NonFinite("structural target residual".into()));
                }
                maximum = maximum.max(residual.abs());
            }
            Ok(maximum)
        })?;
        let full_residual_linf = residuals.into_iter().fold(0.0_f64, f64::max);
        work.rhs_evaluations += S as u64;
        work.physical_linear_actions += S as u64;
        work.full_state_causal_accumulations += (S * (S - 1) * n) as u64;
        work.original_space_residual_components += (S * n) as u64;
        if full_residual_linf > residual_limit {
            return Err(CoreError::InvalidInput(format!(
                "structural original-space residual {full_residual_linf:e} exceeds {residual_limit:e}"
            )));
        }
        Ok(StructuralCandidate {
            y_new,
            stages_u: stages,
            error_vector,
            full_residual_linf,
            work,
        })
    }
}

fn validate_screen(limit: f64) -> CoreResult<()> {
    if limit.is_finite() && limit > 0.0 {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(
            "positive finite residual limit required".into(),
        ))
    }
}

impl MeanFieldPlan {
    /// Derive a common original-stage residual limit from a requested global
    /// algebraic error budget. With exact stage residual infinity norm at most
    /// delta on each step, the exact affine discrete trajectory differs by at
    /// most C * delta, where C is the sharp finite accumulated residual norm.
    /// Shared collective/transverse projectors let us compute C from scalar
    /// powers. Also report the simpler upper bound A * (1+G+...+G^(steps-1)),
    /// with A and G the sharp one-step residual and propagation norms. Neither
    /// formula uses a divided difference or assumes contraction.
    pub fn algebraic_residual_budget(
        &self,
        dimension: usize,
        steps: usize,
        requested_algebraic_error: f64,
    ) -> CoreResult<AlgebraicResidualBudget> {
        if dimension == 0
            || steps == 0
            || !requested_algebraic_error.is_finite()
            || requested_algebraic_error <= 0.0
        {
            return Err(CoreError::InvalidInput(
                "positive dimension, step count and algebraic error budget required".into(),
            ));
        }
        let c = rodas5p_coefficients()?;
        let hg = self.h * c.gamma;
        let adjoint = |rate: f64| -> CoreResult<[f64; S]> {
            let hr = hg * rate;
            let denominator = 1.0 + hr;
            let mut w = [0.0; S];
            for j in (0..S).rev() {
                let mut rhs = c.b_code[j];
                for (i, weight) in w.iter().enumerate().take(S).skip(j + 1) {
                    rhs -= (hr * c.a[(i, j)] - c.gamma * c.c_matrix[(i, j)]) * *weight;
                }
                w[j] = rhs / denominator;
            }
            finite(&w)?;
            Ok(w)
        };
        let wt = adjoint(self.decay)?;
        let wc = adjoint(self.decay - self.coupling)?;
        // For T=t*Q+c*P the diagonal entry is t+(c-t)/n and every
        // off-diagonal entry is (c-t)/n. Every row has the same absolute sum.
        let row_norm = |t: f64, c: f64| {
            let off_diagonal = (c - t) / dimension as f64;
            (t + off_diagonal).abs() + (dimension - 1) as f64 * off_diagonal.abs()
        };
        let output_amplification: f64 = (0..S).map(|i| row_norm(wt[i], wc[i])).sum();
        let stability = |rate: f64, transfer: &[f64; S]| {
            1.0 - rate * (0..S).map(|i| c.b_code[i] * transfer[i]).sum::<f64>()
        };
        let rt = stability(self.decay, &self.transverse);
        let rc = stability(self.decay - self.coupling, &self.collective);
        let step_amplification = row_norm(rt, rc);
        let mut propagation_sum = 0.0;
        let mut global_residual_amplification = 0.0;
        let (mut pt, mut pc) = (1.0, 1.0);
        for _ in 0..steps {
            propagation_sum = 1.0 + step_amplification * propagation_sum;
            global_residual_amplification += (0..S)
                .map(|i| row_norm(pt * wt[i], pc * wc[i]))
                .sum::<f64>();
            pt *= rt;
            pc *= rc;
        }
        let residual_limit = requested_algebraic_error / global_residual_amplification;
        finite(&[
            output_amplification,
            step_amplification,
            propagation_sum,
            global_residual_amplification,
        ])?;
        validate_screen(residual_limit)?;
        Ok(AlgebraicResidualBudget {
            dimension,
            steps,
            requested_algebraic_error,
            output_amplification,
            step_amplification,
            propagation_sum,
            global_residual_amplification,
            residual_limit,
            scalar_bound_solves: 2 * S as u64,
        })
    }

    /// Includes one full RHS and an eight-stage original-space residual check.
    /// Plan construction costs sixteen scalar solves and is reported separately.
    pub fn step(
        &self,
        model: &MeanFieldAffine,
        y: &[f64],
        residual_limit: f64,
        execution: &ParallelExecution,
    ) -> CoreResult<StructuralCandidate> {
        model.validate_state(y)?;
        validate_screen(residual_limit)?;
        if self.decay.to_bits() != model.decay.to_bits()
            || self.coupling.to_bits() != model.coupling.to_bits()
        {
            return Err(CoreError::InvalidInput(
                "structural plan rate identity mismatch".into(),
            ));
        }
        let mut f0 = vec![0.0; y.len()];
        model.rhs_into(y, &mut f0)?;
        let mu = mean(&f0);
        let ids: Vec<_> = (0..S).collect();
        let stages = execution.map_ordered(&ids, |&i| {
            let u: Vec<_> = f0
                .iter()
                .map(|v| self.transverse[i] * (v - mu) + self.collective[i] * mu)
                .collect();
            finite(&u)?;
            Ok(u)
        })?;
        let work = StructuralWork {
            rhs_evaluations: 1,
            lifted_components: (S * y.len()) as u64,
            ..StructuralWork::default()
        };
        model.finish(y, self.h, stages, work, residual_limit, execution)
    }

    pub fn step_size(&self) -> f64 {
        self.h
    }
    pub const fn scalar_setup_solves(&self) -> u64 {
        2 * S as u64
    }
}
