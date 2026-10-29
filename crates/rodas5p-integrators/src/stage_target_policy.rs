//! N05: theorem-bound residual planning for an owned frozen linear model.
//!
//! Scope is the exact real interpretation of the represented matrix, fixed
//! positive Hilbert weights and the runtime's represented RODAS5P tableau.
//! This does not certify nonlinear Jacobian drift, local truncation error or
//! global ODE error. Generic lagged callbacks retain their diagnostic policy.

use std::sync::Arc;

use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, Rodas5pCoefficients,
    directed::{
        Interval, add_down, add_up, div_down, div_up, mul_down, mul_up, sqrt_down, sqrt_up,
    },
    rodas5p_coefficients,
};

const STAGES: usize = 8;
const ZERO: Interval = Interval { lo: 0.0, hi: 0.0 };
type Polynomial = Vec<Interval>;

/// The proved operator calculus used by a frozen-linear residual plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageBoundCalculus {
    /// Cayley contraction in the declared fixed Hilbert norm, including
    /// nonnormal dissipative matrices and rotations.
    DissipativeCayley,
    /// An exactly symmetric matrix with equal fixed weights and spectrum
    /// contained in (-infinity, 0]; Bernstein convex-hull bounds on R in [0,1].
    SymmetricNegativeReal,
}

/// An immutable, owned matrix and fixed Hilbert norm. Construction verifies
/// dissipativity by outward Gershgorin bounds on the symmetric part of
/// S^-1 J S. No externally supplied claim or mutable callback grants authority.
#[derive(Clone, Debug)]
pub struct BoundLinearStageModel {
    matrix: Arc<DenseMatrix>,
    scales: Arc<Vec<f64>>,
    calculus: StageBoundCalculus,
    output_bounds: [f64; STAGES],
    estimator_bounds: [f64; STAGES],
}

/// A plan bound to one immutable model, initial state, step and tolerances.
/// Targets are for normalized residuals r_i=(I-h gamma J)U_i-RHS_i in
/// ||v|| = sqrt(sum((v_i/s_i)^2)/n), not for P^-1 residuals.
#[derive(Clone, Debug)]
pub struct LinearStageTargetPlan {
    model: BoundLinearStageModel,
    initial: Vec<f64>,
    step: f64,
    atol: f64,
    rtol: f64,
    budget: f64,
    control_scale_lower: f64,
    targets: [f64; STAGES],
}

/// Directed, candidate-specific perturbation receipt. Private fields prevent
/// an arbitrary scalar from being reinterpreted as an assessed stage witness.
#[derive(Clone, Debug)]
pub struct LinearStageAssessment {
    residual_norm_upper: [f64; STAGES],
    output_error_upper: f64,
    estimator_error_upper: f64,
    stage_budget_used_upper: f64,
    combined_controller_metric_upper: f64,
    targets_satisfied: bool,
    within_stage_budget: bool,
    within_controller_budget: bool,
    work: LinearStageAssessmentWork,
}

/// All dense matrix applications and scalar output work performed by assess.
/// Planning checks and fixed-degree coefficient algebra are separate setup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinearStageAssessmentWork {
    /// Each stage computes J times its coupled state and J times U_i.
    pub interval_matrix_applications: u64,
    /// Scalar multiply-add terms within those interval matrix applications.
    pub matrix_terms: u64,
    /// Interval A and C coupling terms, including represented zero entries.
    pub stage_coupling_terms: u64,
    /// Terms in the final represented output sum.
    pub output_terms: u64,
}

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(format!("N05 linear stage policy: {message}"))
}
fn point(value: f64) -> CoreResult<Interval> {
    Interval::point(value)
}

impl BoundLinearStageModel {
    pub fn new(matrix: DenseMatrix, scales: Vec<f64>) -> CoreResult<Self> {
        let n = scales.len();
        if n == 0
            || matrix.nrows() != n
            || matrix.ncols() != n
            || scales.iter().any(|v| !v.is_finite() || *v <= 0.0)
            || matrix.as_slice().iter().any(|v| !v.is_finite())
        {
            return Err(invalid(
                "require a finite square matrix and positive finite fixed weights",
            ));
        }
        let equal_weights = scales.iter().all(|v| *v == scales[0]);
        let mut transformed = vec![ZERO; n * n];
        for i in 0..n {
            for j in 0..n {
                transformed[i * n + j] = if equal_weights {
                    point(matrix[(i, j)])?
                } else {
                    point(matrix[(i, j)])?
                        .mul(point(scales[j])?)?
                        .div(point(scales[i])?)?
                };
            }
        }
        for i in 0..n {
            let mut row_upper = transformed[i * n + i].hi;
            for j in 0..n {
                if i != j {
                    let symmetric = transformed[i * n + j]
                        .add(transformed[j * n + i])?
                        .scale(0.5)?;
                    row_upper = add_up(row_upper, symmetric.mag())?;
                }
            }
            if row_upper > 0.0 {
                return Err(invalid(
                    "the declared Hilbert dissipativity could not be proved",
                ));
            }
        }
        let symmetric =
            equal_weights && (0..n).all(|i| (0..i).all(|j| matrix[(i, j)] == matrix[(j, i)]));
        let calculus = if symmetric {
            StageBoundCalculus::SymmetricNegativeReal
        } else {
            StageBoundCalculus::DissipativeCayley
        };
        let coefficients = checked_coefficients()?;
        let mut estimator = [0.0; STAGES];
        estimator[STAGES - 1] = 1.0;
        Ok(Self {
            matrix: Arc::new(matrix),
            scales: Arc::new(scales),
            calculus,
            output_bounds: transfer_bounds(coefficients, &coefficients.b_code, calculus)?,
            estimator_bounds: transfer_bounds(coefficients, &estimator, calculus)?,
        })
    }

    pub fn calculus(&self) -> StageBoundCalculus {
        self.calculus
    }
    pub fn dimension(&self) -> usize {
        self.scales.len()
    }
    pub fn output_amplification_bounds(&self) -> &[f64; STAGES] {
        &self.output_bounds
    }
    pub fn estimator_amplification_bounds(&self) -> &[f64; STAGES] {
        &self.estimator_bounds
    }

    /// Allocate at most `budget` of the unchanged unit controller budget to
    /// output perturbation PLUS contamination of the embedded estimator.
    /// `0 < budget <= 0.1`; invalid/zero tolerance scales refuse this policy.
    /// A refused plan must use the ordinary solver or refresh/direct fallback.
    pub fn plan(
        &self,
        initial: &[f64],
        step: f64,
        atol: f64,
        rtol: f64,
        budget: f64,
    ) -> CoreResult<LinearStageTargetPlan> {
        if initial.len() != self.dimension()
            || initial.iter().any(|v| !v.is_finite())
            || !step.is_finite()
            || step <= 0.0
            || !atol.is_finite()
            || atol < 0.0
            || !rtol.is_finite()
            || rtol < 0.0
            || !budget.is_finite()
            || budget <= 0.0
            || budget > 0.1
        {
            return Err(invalid("invalid state, step, tolerance or budget"));
        }
        // Lower bound in the fixed norm: end-step controller weights are
        // componentwise no smaller than atol+rtol*abs(initial).
        let mut control_scale_lower = f64::INFINITY;
        for (&y, &scale) in initial.iter().zip(self.scales.iter()) {
            let weight = add_down(atol, mul_down(rtol, y.abs())?)?;
            let lower = div_down(weight, scale)?;
            if lower <= 0.0 {
                return Err(invalid("zero or underflowed tolerance scale"));
            }
            control_scale_lower = control_scale_lower.min(lower);
        }
        let mut targets = [0.0; STAGES];
        for (i, target) in targets.iter_mut().enumerate() {
            let amplification = add_up(self.output_bounds[i], self.estimator_bounds[i])?;
            let denominator = mul_up(STAGES as f64, amplification.max(1.0))?;
            *target = div_down(mul_down(budget, control_scale_lower)?, denominator)?;
            if *target <= 0.0 {
                return Err(invalid("target underflow; use direct fallback"));
            }
        }
        Ok(LinearStageTargetPlan {
            model: self.clone(),
            initial: initial.to_vec(),
            step,
            atol,
            rtol,
            budget,
            control_scale_lower,
            targets,
        })
    }
}

impl LinearStageTargetPlan {
    pub fn normalized_residual_targets(&self) -> &[f64; STAGES] {
        &self.targets
    }
    pub fn calculus(&self) -> StageBoundCalculus {
        self.model.calculus
    }

    /// Safe absolute Euclidean tolerances for the unscaled raw residual
    /// `c-WU` of a GMRES client using this bound model. This is not a
    /// tolerance for `P^-1(c-WU)` or a relative GMRES stopping threshold.
    /// Final admission still requires assess() because it charges estimator
    /// contamination and returned-center rounding as well as stage residuals.
    pub fn raw_euclidean_residual_targets(&self) -> CoreResult<[f64; STAGES]> {
        let gamma = checked_coefficients()?.gamma;
        let a_upper = mul_up(self.step, gamma)?;
        let min_scale = self
            .model
            .scales
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let norm_conversion = mul_down(min_scale, sqrt_down(self.model.dimension() as f64)?)?;
        let mut result = [0.0; STAGES];
        for (out, &target) in result.iter_mut().zip(&self.targets) {
            *out = div_down(mul_down(target, norm_conversion)?, a_upper)?;
            if *out <= 0.0 {
                return Err(invalid("raw target underflow; use direct fallback"));
            }
        }
        Ok(result)
    }

    /// Recompute true normalized stage residuals from candidate U coordinates,
    /// the bound matrix and pinned tableau, outward. A caller cannot supply
    /// residual claims, replacement operators, scales or coefficient bounds.
    /// `returned` is the actual rounded output center, also charged here.
    /// The controller check is about the embedded estimate and stage error;
    /// it is not an enclosure of the exact ODE solution or its global error.
    pub fn assess(&self, stages: &[f64], returned: &[f64]) -> CoreResult<LinearStageAssessment> {
        let n = self.model.dimension();
        if stages.len() != STAGES * n
            || returned.len() != n
            || stages.iter().chain(returned).any(|v| !v.is_finite())
        {
            return Err(invalid("candidate shape or finite-value mismatch"));
        }
        let coefficients = checked_coefficients()?;
        let a = point(self.step)?.mul(point(coefficients.gamma)?)?;
        let mut residual_norm_upper = [0.0; STAGES];
        let mut state = vec![ZERO; n];
        let mut coupling = vec![ZERO; n];
        let mut residual = vec![ZERO; n];
        for stage in 0..STAGES {
            for k in 0..n {
                state[k] = point(self.initial[k])?;
                coupling[k] = ZERO;
                for previous in 0..stage {
                    let u = point(stages[previous * n + k])?;
                    state[k] = state[k].add(u.scale(coefficients.a[(stage, previous)])?)?;
                    coupling[k] =
                        coupling[k].add(u.scale(coefficients.c_matrix[(stage, previous)])?)?;
                }
            }
            for k in 0..n {
                let mut js = ZERO;
                let mut ju = ZERO;
                for l in 0..n {
                    js = js.add(state[l].scale(self.model.matrix[(k, l)])?)?;
                    ju = ju.add(point(stages[stage * n + l])?.scale(self.model.matrix[(k, l)])?)?;
                }
                residual[k] = a
                    .mul(js)?
                    .add(coupling[k].scale(coefficients.gamma)?)?
                    .sub(point(stages[stage * n + k])?)?
                    .add(a.mul(ju)?)?;
            }
            residual_norm_upper[stage] = norm_upper(&residual, &self.model.scales)?;
        }
        let mut output_error_upper = 0.0;
        let mut estimator_error_upper = 0.0;
        for (i, &r) in residual_norm_upper.iter().enumerate() {
            output_error_upper =
                add_up(output_error_upper, mul_up(self.model.output_bounds[i], r)?)?;
            estimator_error_upper = add_up(
                estimator_error_upper,
                mul_up(self.model.estimator_bounds[i], r)?,
            )?;
        }
        // Returned-center rounding is distinct from stage-equation residuals.
        let mut center_error = vec![ZERO; n];
        let mut estimator = vec![ZERO; n];
        let mut end_weights = vec![0.0; n];
        for k in 0..n {
            let mut ideal_returned_sum = point(self.initial[k])?;
            for i in 0..STAGES {
                ideal_returned_sum = ideal_returned_sum
                    .add(point(stages[i * n + k])?.scale(coefficients.b_code[i])?)?;
            }
            center_error[k] = point(returned[k])?.sub(ideal_returned_sum)?;
            estimator[k] = point(stages[(STAGES - 1) * n + k])?;
            end_weights[k] = add_down(
                self.atol,
                mul_down(self.rtol, self.initial[k].abs().max(returned[k].abs()))?,
            )?;
            if end_weights[k] <= 0.0 {
                return Err(invalid("zero end-step controller scale"));
            }
        }
        output_error_upper = add_up(
            output_error_upper,
            norm_upper(&center_error, &self.model.scales)?,
        )?;
        let stage_budget_used_upper = div_up(
            add_up(output_error_upper, estimator_error_upper)?,
            self.control_scale_lower,
        )?;
        let combined_controller_metric_upper = add_up(
            norm_upper(&estimator, &end_weights)?,
            stage_budget_used_upper,
        )?;
        Ok(LinearStageAssessment {
            residual_norm_upper,
            output_error_upper,
            estimator_error_upper,
            stage_budget_used_upper,
            combined_controller_metric_upper,
            targets_satisfied: residual_norm_upper
                .iter()
                .zip(self.targets)
                .all(|(r, t)| *r <= t),
            within_stage_budget: stage_budget_used_upper <= self.budget,
            within_controller_budget: combined_controller_metric_upper <= 1.0,
            work: LinearStageAssessmentWork {
                interval_matrix_applications: 2 * STAGES as u64,
                matrix_terms: 2 * STAGES as u64 * n as u64 * n as u64,
                stage_coupling_terms: (STAGES * (STAGES - 1) * n) as u64,
                output_terms: (STAGES * n) as u64,
            },
        })
    }
}

impl LinearStageAssessment {
    pub fn residual_norm_upper(&self) -> &[f64; STAGES] {
        &self.residual_norm_upper
    }
    pub fn output_error_upper(&self) -> f64 {
        self.output_error_upper
    }
    pub fn estimator_error_upper(&self) -> f64 {
        self.estimator_error_upper
    }
    pub fn stage_budget_used_upper(&self) -> f64 {
        self.stage_budget_used_upper
    }
    pub fn combined_controller_metric_upper(&self) -> f64 {
        self.combined_controller_metric_upper
    }
    pub fn targets_satisfied(&self) -> bool {
        self.targets_satisfied
    }
    pub fn within_stage_budget(&self) -> bool {
        self.within_stage_budget
    }
    pub fn within_controller_budget(&self) -> bool {
        self.within_controller_budget
    }
    pub fn work(&self) -> LinearStageAssessmentWork {
        self.work
    }
}

fn checked_coefficients() -> CoreResult<&'static Rodas5pCoefficients> {
    let c = rodas5p_coefficients()?;
    if c.a.nrows() != STAGES
        || c.a.ncols() != STAGES
        || c.c_matrix.nrows() != STAGES
        || c.c_matrix.ncols() != STAGES
        || c.b_code.len() != STAGES
        || c.gamma <= 0.0
        || !c.gamma.is_finite()
    {
        return Err(invalid("unsupported tableau shape or gamma"));
    }
    for i in 0..STAGES {
        for j in i..STAGES {
            if c.a[(i, j)] != 0.0 || c.c_matrix[(i, j)] != 0.0 {
                return Err(invalid("tableau must be strictly lower triangular"));
            }
        }
    }
    Ok(c)
}

fn norm_upper(values: &[Interval], scales: &[f64]) -> CoreResult<f64> {
    let mut sum = 0.0;
    for (value, &scale) in values.iter().zip(scales) {
        let v = div_up(value.mag(), scale)?;
        sum = add_up(sum, mul_up(v, v)?)?;
    }
    sqrt_up(div_up(sum, values.len() as f64)?)
}

// m_i(R) is the transfer from the normalized raw stage residual to the
// selected output. Backward elimination constructs p_i, then m_i=R p_i.
fn transfer_bounds(
    c: &Rodas5pCoefficients,
    output: &[f64],
    calculus: StageBoundCalculus,
) -> CoreResult<[f64; STAGES]> {
    let mut p: Vec<Polynomial> = (0..STAGES)
        .map(|i| {
            vec![Interval {
                lo: output[i],
                hi: output[i],
            }]
        })
        .collect();
    for i in (0..STAGES).rev() {
        for j in i + 1..STAGES {
            let a = point(c.a[(j, i)])?;
            let gc = point(c.gamma)?.mul(point(c.c_matrix[(j, i)])?)?;
            let (constant, linear) = match calculus {
                StageBoundCalculus::DissipativeCayley => {
                    (gc.sub(a)?.scale(0.5)?, gc.add(a)?.scale(0.5)?)
                }
                StageBoundCalculus::SymmetricNegativeReal => (-a, gc.add(a)?),
            };
            let term = multiply_linear(&p[j], constant, linear)?;
            if p[i].len() < term.len() {
                p[i].resize(term.len(), ZERO);
            }
            for (target, value) in p[i].iter_mut().zip(term) {
                *target = target.add(value)?;
            }
        }
    }
    let mut bounds = [0.0; STAGES];
    for (i, poly) in p.iter().enumerate() {
        let m = match calculus {
            StageBoundCalculus::DissipativeCayley => {
                multiply_linear(poly, point(0.5)?, point(0.5)?)?
            }
            StageBoundCalculus::SymmetricNegativeReal => multiply_linear(poly, ZERO, point(1.0)?)?,
        };
        bounds[i] = match calculus {
            StageBoundCalculus::DissipativeCayley => {
                m.iter().try_fold(0.0, |sum, c| add_up(sum, c.mag()))?
            }
            StageBoundCalculus::SymmetricNegativeReal => bernstein_bound(&m, 4)?,
        };
    }
    Ok(bounds)
}
fn multiply_linear(p: &[Interval], constant: Interval, linear: Interval) -> CoreResult<Polynomial> {
    let mut out = vec![ZERO; p.len() + 1];
    for (i, &coefficient) in p.iter().enumerate() {
        out[i] = out[i].add(coefficient.mul(constant)?)?;
        out[i + 1] = out[i + 1].add(coefficient.mul(linear)?)?;
    }
    Ok(out)
}
fn binomial(n: usize, k: usize) -> f64 {
    let mut value = 1_u64;
    for i in 0..k {
        value = value * (n - i) as u64 / (i + 1) as u64;
    }
    value as f64 // n <= 8: every integer is exactly represented.
}
fn bernstein_bound(power: &[Interval], subdivisions: usize) -> CoreResult<f64> {
    let degree = power.len() - 1;
    let mut bernstein = vec![ZERO; power.len()];
    for (k, b) in bernstein.iter_mut().enumerate() {
        for (j, coefficient) in power.iter().enumerate().take(k + 1) {
            let ratio = point(binomial(k, j))?.div(point(binomial(degree, j))?)?;
            *b = b.add(coefficient.mul(ratio)?)?;
        }
    }
    subdivided_bernstein_bound(&bernstein, subdivisions)
}
fn subdivided_bernstein_bound(coefficients: &[Interval], depth: usize) -> CoreResult<f64> {
    if depth == 0 {
        return Ok(coefficients.iter().map(Interval::mag).fold(0.0, f64::max));
    }
    let degree = coefficients.len() - 1;
    let mut row = coefficients.to_vec();
    let mut left = vec![ZERO; coefficients.len()];
    let mut right = left.clone();
    left[0] = row[0];
    right[degree] = row[degree];
    for level in 1..=degree {
        for i in 0..=degree - level {
            row[i] = row[i].add(row[i + 1])?.scale(0.5)?;
        }
        left[level] = row[0];
        right[degree - level] = row[degree - level];
    }
    Ok(subdivided_bernstein_bound(&left, depth - 1)?
        .max(subdivided_bernstein_bound(&right, depth - 1)?))
}

#[cfg(test)]
mod n05_tests {
    use super::*;
    use rodas5p_core::LuFactorization;

    fn diagonal_model() -> BoundLinearStageModel {
        BoundLinearStageModel::new(
            DenseMatrix::from_rows(&[&[-1., 0.], &[0., -4.]]).unwrap(),
            vec![1., 1.],
        )
        .unwrap()
    }
    // Independent forward stage solves, rather than the backward polynomial
    // bound construction. Candidate arithmetic deliberately uses ordinary f64.
    fn direct_candidate(j: &DenseMatrix, y: &[f64], h: f64) -> (Vec<f64>, Vec<f64>) {
        let c = rodas5p_coefficients().unwrap();
        let n = y.len();
        let mut matrix = DenseMatrix::identity(n);
        for k in 0..n {
            for l in 0..n {
                matrix[(k, l)] -= h * c.gamma * j[(k, l)];
            }
        }
        let lu = LuFactorization::new(&matrix).unwrap();
        let mut stages = vec![0.; STAGES * n];
        for i in 0..STAGES {
            let mut state = y.to_vec();
            for p in 0..i {
                for k in 0..n {
                    state[k] += c.a[(i, p)] * stages[p * n + k];
                }
            }
            let mut rhs = j.matvec(&state).unwrap();
            for k in 0..n {
                rhs[k] *= h * c.gamma;
                for p in 0..i {
                    rhs[k] += c.gamma * c.c_matrix[(i, p)] * stages[p * n + k];
                }
            }
            stages[i * n..(i + 1) * n].copy_from_slice(&lu.solve(&rhs).unwrap());
        }
        let mut returned = y.to_vec();
        for i in 0..STAGES {
            for k in 0..n {
                returned[k] += c.b_code[i] * stages[i * n + k];
            }
        }
        (stages, returned)
    }
    fn scalar_transfers(z: f64) -> [f64; STAGES] {
        let c = rodas5p_coefficients().unwrap();
        let w = 1.0 / (1.0 - c.gamma * z);
        let mut p = [0.; STAGES];
        for i in (0..STAGES).rev() {
            p[i] = c.b_code[i];
            for j in i + 1..STAGES {
                p[i] += p[j] * (c.gamma * c.c_matrix[(j, i)] * w + c.a[(j, i)] * (w - 1.));
            }
        }
        p.map(|v| w * v)
    }

    #[test]
    fn scope_checks_and_rotation_counterexample_do_not_grant_real_spectrum() {
        let rotation = BoundLinearStageModel::new(
            DenseMatrix::from_rows(&[&[0., -1.], &[1., 0.]]).unwrap(),
            vec![1., 1.],
        )
        .unwrap();
        assert_eq!(rotation.calculus(), StageBoundCalculus::DissipativeCayley);
        assert!(rotation.output_amplification_bounds()[0] > 0.851904674493);
        assert!(
            BoundLinearStageModel::new(
                DenseMatrix::from_rows(&[&[0., 10.], &[0., 0.]]).unwrap(),
                vec![1., 1.]
            )
            .is_err()
        );
        assert!(BoundLinearStageModel::new(DenseMatrix::identity(1), vec![1.]).is_err());
        assert!(BoundLinearStageModel::new(DenseMatrix::zeros(1, 1), vec![0.]).is_err());
        // Gershgorin on the symmetric part admits a genuinely nonnormal J.
        let nonnormal = BoundLinearStageModel::new(
            DenseMatrix::from_rows(&[&[-1., 1.], &[0., -1.]]).unwrap(),
            vec![1., 1.],
        )
        .unwrap();
        assert_eq!(nonnormal.calculus(), StageBoundCalculus::DissipativeCayley);
        // Fixed weights are part of the model: arbitrary changes can destroy dissipativity.
        assert!(
            BoundLinearStageModel::new(
                DenseMatrix::from_rows(&[&[-1., 1.], &[0., -1.]]).unwrap(),
                vec![1., 100.]
            )
            .is_err()
        );
    }

    #[test]
    fn spectral_bound_is_tighter_and_dominates_independent_forward_values() {
        let model = diagonal_model();
        assert_eq!(model.calculus(), StageBoundCalculus::SymmetricNegativeReal);
        let c = checked_coefficients().unwrap();
        let cayley = transfer_bounds(c, &c.b_code, StageBoundCalculus::DissipativeCayley).unwrap();
        assert!(model.output_bounds.iter().sum::<f64>() < cayley.iter().sum::<f64>());
        for z in [0., -1e-12, -0.01, -0.5, -1., -3., -10., -1e3, -1e8] {
            for (actual, bound) in scalar_transfers(z).iter().zip(model.output_bounds) {
                assert!(actual.abs() <= bound, "z={z} actual={actual} bound={bound}");
            }
        }
        // Independent exact-rational oracle's actual binary64-tableau Cayley results.
        let real_exact_upper = [
            0.5773402226052611,
            0.2370456710315718,
            6.24636426367119,
            5.965853135260995,
            3.7294152753025385,
            4.567822998568431,
            1.0109942639901015,
            1.0,
        ];
        for (actual, reference) in model.output_bounds.iter().zip(real_exact_upper) {
            assert!(*actual >= reference && *actual <= reference + 1e-9);
        }
        let exact_upper = [
            2.3329670408268792,
            0.6336375439230381,
            8.336841185254002,
            7.505701499817989,
            4.694888068628397,
            5.819658901511291,
            1.0109942639901015,
            1.,
        ];
        for (actual, reference) in cayley.iter().zip(exact_upper) {
            assert!(*actual >= reference && *actual <= reference + 1e-10);
        }
    }

    #[test]
    fn normalized_stage_and_returned_center_errors_are_charged_without_budget_inflation() {
        let model = diagonal_model();
        let initial = [1., 2.];
        let plan = model.plan(&initial, 0.01, 1e-8, 1e-6, 0.1).unwrap();
        let raw_targets = plan.raw_euclidean_residual_targets().unwrap();
        let half_raw_targets = model
            .plan(&initial, 0.02, 1e-8, 1e-6, 0.1)
            .unwrap()
            .raw_euclidean_residual_targets()
            .unwrap();
        for (whole, half) in raw_targets.iter().zip(half_raw_targets) {
            assert!((whole / (2.0 * half) - 1.0).abs() < 1e-14);
        }
        let (u, y) = direct_candidate(&model.matrix, &initial, 0.01);
        let good = plan.assess(&u, &y).unwrap();
        assert!(
            good.targets_satisfied()
                && good.within_stage_budget()
                && good.within_controller_budget()
        );
        assert_eq!(good.work.interval_matrix_applications, 16);
        assert_eq!(good.work.matrix_terms, 64);
        assert_eq!(good.work.stage_coupling_terms, 112);
        let mut wrong_u = u.clone();
        wrong_u[0] += 1e-4;
        let bad_stage = plan.assess(&wrong_u, &y).unwrap();
        assert!(!bad_stage.targets_satisfied() && !bad_stage.within_stage_budget());
        assert!(bad_stage.estimator_error_upper() > 0.0);
        let mut wrong_center = y.clone();
        wrong_center[0] += 1e-3;
        let bad_center = plan.assess(&u, &wrong_center).unwrap();
        assert!(bad_center.targets_satisfied());
        assert!(!bad_center.within_stage_budget() && !bad_center.within_controller_budget());
        assert!(bad_center.output_error_upper() > 1e-3 / 2_f64.sqrt());
        // No method can replace the matrix/weights or supply a claimed tiny residual.
        assert!(model.plan(&initial, 0.01, 1e-8, 1e-6, 0.10001).is_err());
        assert!(model.plan(&[0., 0.], 0.01, 0., 1e-6, 0.1).is_err());
        assert!(
            model
                .plan(&initial, f64::INFINITY, 1e-8, 1e-6, 0.1)
                .is_err()
        );
        assert!(plan.assess(&u[..u.len() - 1], &y).is_err());
        let mut nonfinite = u.clone();
        nonfinite[0] = f64::NAN;
        assert!(plan.assess(&nonfinite, &y).is_err());
    }

    #[test]
    fn outward_assessment_covers_independent_actual_stage_perturbation() {
        for j in [
            DenseMatrix::from_rows(&[&[-1., 1.], &[0., -1.]]).unwrap(),
            DenseMatrix::from_rows(&[&[0., -1.], &[1., 0.]]).unwrap(),
        ] {
            let model = BoundLinearStageModel::new(j.clone(), vec![1., 1.]).unwrap();
            let plan = model.plan(&[1., 2.], 0.3, 1e-4, 1e-4, 0.1).unwrap();
            let (mut u, exact_output) = direct_candidate(&j, &[1., 2.], 0.3);
            let exact_estimator = u[14..].to_vec();
            u[0] += 1e-5;
            u[5] -= 1e-5;
            u[14] += 1e-6;
            let c = checked_coefficients().unwrap();
            let mut returned = vec![1., 2.];
            for i in 0..STAGES {
                for k in 0..2 {
                    returned[k] += c.b_code[i] * u[2 * i + k];
                }
            }
            let assessment = plan.assess(&u, &returned).unwrap();
            let output_difference = ((returned[0] - exact_output[0]).powi(2)
                + (returned[1] - exact_output[1]).powi(2))
            .sqrt()
                / 2_f64.sqrt();
            let estimator_difference = ((u[14] - exact_estimator[0]).powi(2)
                + (u[15] - exact_estimator[1]).powi(2))
            .sqrt()
                / 2_f64.sqrt();
            assert!(output_difference <= assessment.output_error_upper());
            assert!(estimator_difference <= assessment.estimator_error_upper());
        }
    }
}
