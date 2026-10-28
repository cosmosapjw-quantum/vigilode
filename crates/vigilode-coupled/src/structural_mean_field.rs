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
//! candidate still undergoes a full original-space residual screen; its work
//! is included in the report. `MeanFieldPlan` retains the operational f64
//! screen. `MeanFieldEnclosurePlan` bounds the same equations using outward
//! arithmetic and covers the full-state lift, RHS and mean rounding errors.
//! Charged APIs retain attempted work on every returned failure.

use rodas5p_core::{
    CoreError, CoreResult,
    directed::{self, Interval},
    rodas5p_coefficients,
};
use rodas5p_integrators::ParallelExecution;
use serde::Serialize;

const S: usize = 8;

#[derive(Clone, Debug)]
pub struct MeanFieldAffine {
    decay: f64,
    coupling: f64,
    forcing: Vec<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct StructuralWork {
    /// Attempted calls, including calls which returned a numerical failure.
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
    /// Components included in a compensated-mean rounding enclosure.
    pub enclosed_mean_components: u64,
    /// Actually constructed lift components covered by a rounding enclosure.
    pub enclosed_lift_components: u64,
    /// Full original-stage infinity norms bounded by the factorized screen.
    pub factorized_residual_stages: u64,
}

/// A declined attempt with all work retained. Parallel jobs are joined and
/// charged before the first error in stage order is returned.
#[derive(Debug)]
pub struct StructuralFailure {
    pub error: CoreError,
    pub work: StructuralWork,
}

impl std::fmt::Display for StructuralFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}

impl std::error::Error for StructuralFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

pub type StructuralResult<T> = Result<T, StructuralFailure>;

#[derive(Clone, Debug, Serialize)]
pub struct StructuralCandidate {
    pub y_new: Vec<f64>,
    pub stages_u: Vec<Vec<f64>>,
    pub error_vector: Vec<f64>,
    pub full_residual_linf: f64,
    pub work: StructuralWork,
}

/// Candidate whose full original eight-stage residual is enclosed in real
/// arithmetic for the represented model/tableau inputs and returned stages.
/// The bound does not include temporal truncation or endpoint rounding.
#[derive(Clone, Debug, Serialize)]
pub struct EnclosedStructuralCandidate {
    pub y_new: Vec<f64>,
    pub stages_u: Vec<Vec<f64>>,
    pub error_vector: Vec<f64>,
    pub full_residual_upper_bound: f64,
    pub stage_residual_upper_bounds: [f64; S],
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

/// Reusable dimension/rate/step-only outward residual plan. The screen uses
/// O(s*n+s*s) work and covers every component of every original stage.
#[derive(Clone, Debug)]
pub struct MeanFieldEnclosurePlan {
    base: MeanFieldPlan,
    dimension: usize,
    transverse_residual: [f64; S],
    collective_residual: [f64; S],
    mean_defect_coefficient: [f64; S],
    lift_defect_weights: [[f64; S]; S],
    hg_upper: f64,
    scalar_residual_factor: [f64; S],
    scalar_underflow_offset: [f64; S],
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
                for j in 0..i {
                    rhs +=
                        (c.gamma * c.c_matrix[(i, j)] - shifted_rate * c.a[(i, j)]) * transfer[j];
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
    /// Use `sequential_step_charged` when failed attempts may be retried.
    pub fn sequential_step(
        &self,
        y: &[f64],
        h: f64,
        residual_limit: f64,
        execution: &ParallelExecution,
    ) -> CoreResult<StructuralCandidate> {
        self.sequential_step_charged(y, h, residual_limit, execution)
            .map_err(|failure| failure.error)
    }

    /// Sequential reference with partial work on every returned failure.
    pub fn sequential_step_charged(
        &self,
        y: &[f64],
        h: f64,
        residual_limit: f64,
        execution: &ParallelExecution,
    ) -> StructuralResult<StructuralCandidate> {
        let mut work = StructuralWork::default();
        let result = (|| {
            self.validate_state(y)?;
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
            for i in 0..S {
                let mut state = y.to_vec();
                let mut mix = vec![0.0; n];
                for j in 0..i {
                    for k in 0..n {
                        state[k] += c.a[(i, j)] * stages[j][k];
                        mix[k] += c.c_matrix[(i, j)] * stages[j][k];
                    }
                    work.full_state_causal_accumulations += (2 * n) as u64;
                }
                let mut rhs = vec![0.0; n];
                work.rhs_evaluations += 1;
                self.rhs_into(&state, &mut rhs)?;
                for k in 0..n {
                    rhs[k] = hg * rhs[k] + c.gamma * mix[k];
                }
                work.shifted_resolvent_actions += 1;
                stages.push(self.resolvent(&rhs, hg)?);
            }
            self.finish(y, h, stages, &mut work, residual_limit, execution)
        })();
        result.map_err(|error| StructuralFailure { error, work })
    }

    fn finish(
        &self,
        y: &[f64],
        h: f64,
        stages: Vec<Vec<f64>>,
        work: &mut StructuralWork,
        residual_limit: f64,
        execution: &ParallelExecution,
    ) -> CoreResult<StructuralCandidate> {
        let c = rodas5p_coefficients()?;
        let hg = h * c.gamma;
        let n = y.len();
        let (y_new, error_vector) = endpoint(y, &stages)?;
        // Always return Ok at the scheduler boundary. Each row's actual result
        // is data, so no worker failure cancels or hides another scheduled row.
        let rows = execution.map_ordered(&(0..S).collect::<Vec<_>>(), |&i| {
            let mut linear_attempted = false;
            let mut mixed_components = 0_u64;
            let mut residual_components = 0_u64;
            let result = (|| {
                let mut state = y.to_vec();
                for j in 0..i {
                    for k in 0..n {
                        state[k] += c.a[(i, j)] * stages[j][k];
                    }
                }
                let mut fy = vec![0.0; n];
                self.rhs_into(&state, &mut fy)?;
                linear_attempted = true;
                let shared_u = self.coupling * mean(&stages[i]);
                let mut maximum = 0.0_f64;
                for k in 0..n {
                    mixed_components += 1;
                    let mut mix = 0.0;
                    for j in 0..i {
                        mix += c.c_matrix[(i, j)] * stages[j][k];
                    }
                    let ju = -self.decay * stages[i][k] + shared_u;
                    if !ju.is_finite() {
                        return Err(CoreError::NonFinite(
                            "structural mean-field input/output".into(),
                        ));
                    }
                    residual_components += 1;
                    let residual = stages[i][k] - hg * ju - c.gamma * mix - hg * fy[k];
                    if !residual.is_finite() {
                        return Err(CoreError::NonFinite("structural target residual".into()));
                    }
                    maximum = maximum.max(residual.abs());
                }
                Ok(maximum)
            })();
            Ok((
                result,
                linear_attempted,
                mixed_components,
                residual_components,
            ))
        })?;
        // Every scheduled row builds its state and attempts one RHS before
        // it can return an error. Aggregate these exact counts once.
        work.rhs_evaluations += S as u64;
        work.full_state_causal_accumulations += (S * (S - 1) / 2 * n) as u64;
        let mut first_error = None;
        let mut full_residual_linf = 0.0_f64;
        for (i, (result, linear_attempted, mixed_components, residual_components)) in
            rows.into_iter().enumerate()
        {
            work.physical_linear_actions += u64::from(linear_attempted);
            work.full_state_causal_accumulations += i as u64 * mixed_components;
            work.original_space_residual_components += residual_components;
            match result {
                Ok(value) => full_residual_linf = full_residual_linf.max(value),
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => (),
            }
        }
        if let Some(error) = first_error {
            return Err(error);
        }
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
            work: work.clone(),
        })
    }
}

fn endpoint(y: &[f64], stages: &[Vec<f64>]) -> CoreResult<(Vec<f64>, Vec<f64>)> {
    let c = rodas5p_coefficients()?;
    let mut y_new = y.to_vec();
    for i in 0..S {
        for k in 0..y.len() {
            y_new[k] += c.b_code[i] * stages[i][k];
        }
    }
    finite(&y_new)?;
    Ok((y_new, stages[S - 1].clone()))
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
                for i in (j + 1)..S {
                    rhs -= (hr * c.a[(i, j)] - c.gamma * c.c_matrix[(i, j)]) * w[i];
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
    /// Use `step_charged` when failed attempts may be retried.
    pub fn step(
        &self,
        model: &MeanFieldAffine,
        y: &[f64],
        residual_limit: f64,
        execution: &ParallelExecution,
    ) -> CoreResult<StructuralCandidate> {
        self.step_charged(model, y, residual_limit, execution)
            .map_err(|failure| failure.error)
    }

    /// Quotient step with all scheduled work retained on both success/failure.
    pub fn step_charged(
        &self,
        model: &MeanFieldAffine,
        y: &[f64],
        residual_limit: f64,
        execution: &ParallelExecution,
    ) -> StructuralResult<StructuralCandidate> {
        let mut work = StructuralWork::default();
        let result = (|| {
            self.validate(model, y, residual_limit)?;
            let mut f0 = vec![0.0; y.len()];
            work.rhs_evaluations += 1;
            model.rhs_into(y, &mut f0)?;
            let mu = mean(&f0);
            let rows = execution.map_ordered(&(0..S).collect::<Vec<_>>(), |&i| {
                let u: Vec<_> = f0
                    .iter()
                    .map(|v| self.transverse[i] * (v - mu) + self.collective[i] * mu)
                    .collect();
                Ok(finite(&u).map(|()| u))
            })?;
            // The complete vectors were constructed before finite checking,
            // including failed rows; the full batch is joined above.
            work.lifted_components += (S * y.len()) as u64;
            let mut first_error = None;
            let mut stages = Vec::with_capacity(S);
            for result in rows {
                match result {
                    Ok(u) => stages.push(u),
                    Err(error) if first_error.is_none() => first_error = Some(error),
                    Err(_) => (),
                }
            }
            if let Some(error) = first_error {
                return Err(error);
            }
            model.finish(y, self.h, stages, &mut work, residual_limit, execution)
        })();
        result.map_err(|error| StructuralFailure { error, work })
    }

    fn validate(&self, model: &MeanFieldAffine, y: &[f64], residual_limit: f64) -> CoreResult<()> {
        model.validate_state(y)?;
        validate_screen(residual_limit)?;
        if self.decay.to_bits() != model.decay.to_bits()
            || self.coupling.to_bits() != model.coupling.to_bits()
        {
            return Err(CoreError::InvalidInput(
                "structural plan rate identity mismatch".into(),
            ));
        }
        Ok(())
    }

    pub fn step_size(&self) -> f64 {
        self.h
    }
    pub const fn scalar_setup_solves(&self) -> u64 {
        2 * S as u64
    }
}

// An upward bound on the round-to-nearest error of an operation whose finite
// rounded result is z. Normal results use half an ulp. At the bottom binade,
// half an ulp is not representable, so the least positive subnormal is used.
// This bound also covers gradual underflow and ties; no fitted multiplier.
fn rounding_radius(z: f64) -> f64 {
    let exponent = (z.abs().to_bits() >> 52) & 0x7ff;
    if exponent <= 1 {
        f64::from_bits(1)
    } else if exponent <= 53 {
        f64::from_bits(1_u64 << (exponent - 2))
    } else {
        f64::from_bits((exponent - 53) << 52)
    }
}

/// Evaluate the same scaled Neumaier mean as `mean`, with an absolute error
/// bound relative to the exact mean of the represented inputs. FastTwoSum's
/// local correction is exact. Only scaling, correction accumulation, and the
/// final addition contribute unabsorbed rounding errors.
fn enclosed_mean(x: &[f64], work: &mut StructuralWork) -> CoreResult<(f64, f64)> {
    if x.len() == 1 {
        work.enclosed_mean_components += 1;
        finite(x)?;
        // P=I. Addition to +0 preserves the signed-zero convention of mean().
        return Ok((0.0 + x[0], 0.0));
    }
    let scale = 1.0 / x.len() as f64;
    let (mut sum, mut correction) = (0.0_f64, 0.0_f64);
    let (mut absolute_sum, mut product_error, mut correction_error) = (0.0, 0.0, 0.0);
    for &v in x {
        work.enclosed_mean_components += 1;
        let value = v * scale;
        let next = sum + value;
        let residual = if sum.abs() >= value.abs() {
            (sum - next) + value
        } else {
            (value - next) + sum
        };
        correction += residual;
        finite(&[value, next, correction])?;
        absolute_sum = directed::add_up(absolute_sum, value.abs())?;
        product_error = directed::add_up(product_error, rounding_radius(value))?;
        correction_error = directed::add_up(correction_error, rounding_radius(correction))?;
        sum = next;
    }
    let value = sum + correction;
    finite(&[value])?;
    // |scale-1/n|/scale times sum |x_i*scale|, with the latter bounded
    // by sum |fl(x_i*scale)| plus the product rounding error.
    let scale_relative_error = directed::div_up(rounding_radius(scale), scale)?;
    let scaling_error = directed::mul_up(
        scale_relative_error,
        directed::add_up(absolute_sum, product_error)?,
    )?;
    let error = directed::sum_up([
        product_error,
        scaling_error,
        correction_error,
        rounding_radius(value),
    ])?;
    Ok((value, error))
}

impl MeanFieldAffine {
    /// Construct a reusable plan with an outward original-space residual
    /// screen. Its step returns a charged failure when the bound exceeds the
    /// supplied limit; it never silently substitutes an operational screen.
    pub fn enclosure_plan(&self, h: f64) -> CoreResult<MeanFieldEnclosurePlan> {
        self.plan(h)?.enclosed_screen(self.dimension())
    }

    fn enclosed_rhs(&self, y: &[f64], work: &mut StructuralWork) -> CoreResult<(Vec<f64>, f64)> {
        work.rhs_evaluations += 1;
        let (mean_y, mean_error) = enclosed_mean(y, work)?;
        let shared = self.coupling * mean_y;
        finite(&[shared])?;
        let (mut largest_product, mut largest_difference, mut largest_output) =
            (0.0_f64, 0.0_f64, 0.0_f64);
        let mut out = Vec::with_capacity(y.len());
        for (&state, &source) in y.iter().zip(&self.forcing) {
            let product = self.decay * state;
            let difference = source - product;
            let value = difference + shared;
            finite(&[product, difference, value])?;
            largest_product = largest_product.max(product.abs());
            largest_difference = largest_difference.max(difference.abs());
            largest_output = largest_output.max(value.abs());
            out.push(value);
        }
        let error = directed::sum_up([
            rounding_radius(largest_product),
            rounding_radius(largest_difference),
            rounding_radius(largest_output),
            rounding_radius(shared),
            directed::mul_up(self.coupling, mean_error)?,
        ])?;
        Ok((out, error))
    }
}

impl MeanFieldPlan {
    /// Add a rigorously rounded full-space screen to this h/rate-only plan.
    /// The dimension is part of the screen identity because the exact induced
    /// norm of t*Q+c*P depends on it. The shared tableau values are interpreted
    /// as represented real coefficients; h*gamma is enclosed as a real product.
    pub fn enclosed_screen(&self, dimension: usize) -> CoreResult<MeanFieldEnclosurePlan> {
        if dimension == 0 || (dimension as u128) > (1_u128 << 53) {
            return Err(CoreError::InvalidInput(
                "enclosed mean requires an exactly represented positive dimension <= 2^53".into(),
            ));
        }
        let c = rodas5p_coefficients()?;
        let point = Interval::point;
        let gamma = point(c.gamma)?;
        let hg = point(self.h)?.mul(gamma)?;
        let decay = point(self.decay)?;
        let coupling = point(self.coupling)?;
        let collective_rate = decay.sub(coupling)?;
        let hdt = hg.mul(decay)?;
        let hdc = hg.mul(collective_rate)?;
        let scalar_residual = |hr: Interval, t: &[f64; S]| -> CoreResult<[f64; S]> {
            let mut residual = [0.0; S];
            for i in 0..S {
                let mut value = point(1.0)?.add(hr)?.mul(point(t[i])?)?.sub(hg)?;
                for j in 0..i {
                    let lower = hr
                        .mul(point(c.a[(i, j)])?)?
                        .sub(gamma.mul(point(c.c_matrix[(i, j)])?)?)?;
                    value = value.add(lower.mul(point(t[j])?)?)?;
                }
                residual[i] = value.mag();
            }
            Ok(residual)
        };
        let transverse_residual = scalar_residual(hdt, &self.transverse)?;
        let collective_residual = scalar_residual(hdc, &self.collective)?;
        let mut mean_defect_coefficient = [0.0; S];
        let mut lift_defect_weights = [[0.0; S]; S];
        let row_norm = |t: Interval, collective: Interval| -> CoreResult<f64> {
            let off = collective.sub(t)?.div(point(dimension as f64)?)?;
            directed::add_up(
                t.add(off)?.mag(),
                directed::mul_up((dimension - 1) as f64, off.mag())?,
            )
        };
        for i in 0..S {
            let mut lifting = point(self.transverse[i])?;
            for j in 0..i {
                lifting = lifting.add(point(c.a[(i, j)])?.mul(point(self.transverse[j])?)?)?;
                let common = gamma.mul(point(c.c_matrix[(i, j)])?)?;
                lift_defect_weights[i][j] = row_norm(
                    hdt.mul(point(c.a[(i, j)])?)?.sub(common)?,
                    hdc.mul(point(c.a[(i, j)])?)?.sub(common)?,
                )?;
            }
            lift_defect_weights[i][i] = row_norm(point(1.0)?.add(hdt)?, point(1.0)?.add(hdc)?)?;
            mean_defect_coefficient[i] = hg.mul(coupling)?.mul(lifting)?.mag();
        }
        // For n=1 the transverse channel vanishes exactly and each stage is
        // one rounded collective product (plus exact signed zeros). Barring
        // overflow, |fl(z)-z| <= 2^-53*|z| + 2^-1075. The least positive
        // subnormal encloses the unrepresentable final constant upward.
        let mut scalar_residual_factor = [0.0; S];
        let mut scalar_underflow_offset = [0.0; S];
        if dimension == 1 {
            const UNIT_ROUNDOFF: f64 = 1.0 / (1_u64 << 53) as f64;
            let underflow_radius = f64::from_bits(1);
            for i in 0..S {
                scalar_residual_factor[i] = collective_residual[i];
                for j in 0..=i {
                    scalar_residual_factor[i] = directed::add_up(
                        scalar_residual_factor[i],
                        directed::mul_up(
                            lift_defect_weights[i][j],
                            directed::mul_up(UNIT_ROUNDOFF, self.collective[j].abs())?,
                        )?,
                    )?;
                    scalar_underflow_offset[i] = directed::add_up(
                        scalar_underflow_offset[i],
                        directed::mul_up(lift_defect_weights[i][j], underflow_radius)?,
                    )?;
                }
            }
        }
        Ok(MeanFieldEnclosurePlan {
            base: self.clone(),
            dimension,
            transverse_residual,
            collective_residual,
            mean_defect_coefficient,
            lift_defect_weights,
            hg_upper: hg.hi,
            scalar_residual_factor,
            scalar_underflow_offset,
        })
    }
}

impl MeanFieldEnclosurePlan {
    pub fn step_size(&self) -> f64 {
        self.base.step_size()
    }

    pub fn base_plan(&self) -> &MeanFieldPlan {
        &self.base
    }

    fn scalar_step(
        &self,
        y: &[f64],
        f0: &[f64],
        rhs_error: f64,
        residual_limit: f64,
        execution: &ParallelExecution,
        work: &mut StructuralWork,
    ) -> CoreResult<EnclosedStructuralCandidate> {
        let mu = 0.0 + f0[0];
        work.enclosed_mean_components += 1;
        let rows = execution.map_ordered(&(0..S).collect::<Vec<_>>(), |&i| {
            let u = self.base.transverse[i] * (f0[0] - mu) + self.base.collective[i] * mu;
            Ok(finite(&[u]).map(|()| vec![u]))
        })?;
        work.lifted_components += S as u64;
        let mut first_error = None;
        let mut stages = Vec::with_capacity(S);
        for result in rows {
            match result {
                Ok(u) => {
                    work.enclosed_lift_components += 1;
                    stages.push(u);
                }
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => (),
            }
        }
        if let Some(error) = first_error {
            return Err(error);
        }
        let rhs_defect = directed::mul_up(self.hg_upper, rhs_error)?;
        let mut stage_residual_upper_bounds = [0.0; S];
        for (i, bound) in stage_residual_upper_bounds.iter_mut().enumerate() {
            *bound = directed::sum_up([
                directed::mul_up(self.scalar_residual_factor[i], mu.abs())?,
                self.scalar_underflow_offset[i],
                rhs_defect,
            ])?;
            work.factorized_residual_stages += 1;
            work.original_space_residual_components += 1;
        }
        let full_residual_upper_bound = stage_residual_upper_bounds.into_iter().fold(0.0, f64::max);
        if full_residual_upper_bound > residual_limit {
            return Err(CoreError::InvalidInput(format!(
                "enclosed structural original-space residual {full_residual_upper_bound:e} exceeds {residual_limit:e}"
            )));
        }
        let (y_new, error_vector) = endpoint(y, &stages)?;
        Ok(EnclosedStructuralCandidate {
            y_new,
            stages_u: stages,
            error_vector,
            full_residual_upper_bound,
            stage_residual_upper_bounds,
            work: work.clone(),
        })
    }

    /// Build all eight full stage vectors and enclose their original residual.
    /// Assumptions: IEEE binary64 round-to-nearest, gradual underflow, and no
    /// unsafe floating-point reassociation. The library uses no fast-math flags.
    /// This is a stage certificate; physical time truncation and endpoint
    /// addition errors remain separate contributions.
    pub fn step(
        &self,
        model: &MeanFieldAffine,
        y: &[f64],
        residual_limit: f64,
        execution: &ParallelExecution,
    ) -> StructuralResult<EnclosedStructuralCandidate> {
        let mut work = StructuralWork::default();
        let result = (|| {
            self.base.validate(model, y, residual_limit)?;
            if y.len() != self.dimension {
                return Err(CoreError::Dimension(
                    "enclosed structural plan dimension".into(),
                ));
            }
            let (f0, rhs_error) = model.enclosed_rhs(y, &mut work)?;
            if self.dimension == 1 {
                return self.scalar_step(y, &f0, rhs_error, residual_limit, execution, &mut work);
            }
            let (mu, mean_error) = enclosed_mean(&f0, &mut work)?;
            let rows = execution.map_ordered(&(0..S).collect::<Vec<_>>(), |&i| {
                let mut lifted = 0_u64;
                let mut enclosed = 0_u64;
                let result = (|| {
                    let mut u = Vec::with_capacity(y.len());
                    let collective = self.base.collective[i] * mu;
                    finite(&[collective])?;
                    let (mut largest_difference, mut largest_product, mut largest_output) =
                        (0.0_f64, 0.0_f64, 0.0_f64);
                    for &v in &f0 {
                        lifted += 1;
                        let difference = v - mu;
                        let product = self.base.transverse[i] * difference;
                        let value = product + collective;
                        finite(&[difference, product, value])?;
                        largest_difference = largest_difference.max(difference.abs());
                        largest_product = largest_product.max(product.abs());
                        largest_output = largest_output.max(value.abs());
                        enclosed += 1;
                        u.push(value);
                    }
                    let lift_error = directed::sum_up([
                        rounding_radius(largest_output),
                        rounding_radius(largest_product),
                        rounding_radius(collective),
                        directed::mul_up(
                            self.base.transverse[i].abs(),
                            rounding_radius(largest_difference),
                        )?,
                    ])?;
                    let difference_bound =
                        directed::add_up(largest_difference, rounding_radius(largest_difference))?;
                    Ok((u, lift_error, difference_bound))
                })();
                Ok((result, lifted, enclosed))
            })?;
            let mut first_error = None;
            let mut stages = Vec::with_capacity(S);
            let mut lift_errors = [0.0; S];
            let mut difference_bound = 0.0_f64;
            for (i, (result, lifted, enclosed)) in rows.into_iter().enumerate() {
                work.lifted_components += lifted;
                work.enclosed_lift_components += enclosed;
                match result {
                    Ok((u, error, bound)) => {
                        stages.push(u);
                        lift_errors[i] = error;
                        difference_bound = difference_bound.max(bound);
                    }
                    Err(error) if first_error.is_none() => first_error = Some(error),
                    Err(_) => (),
                }
            }
            if let Some(error) = first_error {
                return Err(error);
            }
            let rhs_defect = directed::mul_up(self.hg_upper, rhs_error)?;
            let mut stage_residual_upper_bounds = [0.0; S];
            for i in 0..S {
                let mut bound = directed::sum_up([
                    directed::mul_up(self.transverse_residual[i], difference_bound)?,
                    directed::mul_up(self.collective_residual[i], mu.abs())?,
                    directed::mul_up(self.mean_defect_coefficient[i], mean_error)?,
                    rhs_defect,
                ])?;
                for j in 0..=i {
                    bound = directed::add_up(
                        bound,
                        directed::mul_up(self.lift_defect_weights[i][j], lift_errors[j])?,
                    )?;
                }
                stage_residual_upper_bounds[i] = bound;
                work.factorized_residual_stages += 1;
                work.original_space_residual_components += self.dimension as u64;
            }
            let full_residual_upper_bound =
                stage_residual_upper_bounds.into_iter().fold(0.0, f64::max);
            if full_residual_upper_bound > residual_limit {
                return Err(CoreError::InvalidInput(format!(
                    "enclosed structural original-space residual {full_residual_upper_bound:e} exceeds {residual_limit:e}"
                )));
            }
            let (y_new, error_vector) = endpoint(y, &stages)?;
            Ok(EnclosedStructuralCandidate {
                y_new,
                stages_u: stages,
                error_vector,
                full_residual_upper_bound,
                stage_residual_upper_bounds,
                work: work.clone(),
            })
        })();
        result.map_err(|error| StructuralFailure { error, work })
    }
}
