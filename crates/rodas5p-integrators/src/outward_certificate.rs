//! Outward stage-target certificates for small problems (re-audit R3 of
//! 2026-10-01: HOM-02, HOM-03, HOM-04). Research only.
//!
//! For a declared [`StageTarget`] and the quadratic test family
//! `f(y + d) = f(y) + J d + q * d^2` (componentwise `q`), the exact stage
//! vectors `K*` solve, for `i = 0..s`,
//!
//! ```text
//! W K_i = h (J y - q y^2) + h J sum_{j<i} L*_ij K_j + h q (sum_{j<i} alpha_ij K_j)^2,
//! W = I - h gamma J.
//! ```
//!
//! [`certify_stage_target`] bounds `|K_hat - K*|` componentwise for any
//! candidate `K_hat` without knowing `K*`: an outward enclosure of the
//! residual, an [`InverseWitness`] `U >= |W^-1|` entrywise, and a positive
//! recurrence over the stages whose every operation rounds upward. The
//! output and embedded projections add the rounding of the candidate's own
//! `y_hat = y + sum b_i K_i` and `e_hat = sum btilde_i K_i`.
//!
//! What it certifies is a [`CertificateKind::StageTargetBound`]: the
//! distance of the candidate from the exact root of the declared stage
//! equations. The embedded quantity is a bound on the target's embedded
//! estimate, an error *proxy*; neither is a bound on the ODE's local or
//! global error.
//!
//! [`doubling_certificate`] (HOM-04) replaces the serial recurrence by a
//! finite path sum `E = (I + H^4)(I + H^2)(I + H) a` over the strictly lower
//! block matrix `H` (so `H^8 = 0` holds on the target's own structure) with an
//! a priori state radius `D`, which must close (`sum_{j<i} |alpha_ij| E_j <=
//! D_i`) or the attempt is rejected. The radius comes from
//! [`PastStepData`], which by construction holds no reference solution.

use rayon::prelude::*;
use rodas5p_core::{
    CoreError, CoreResult,
    directed::{Interval, add_up, div_up, mul_up, sqrt_up, sub_down, sum_up},
    sha256_hex,
};
use serde::{Deserialize, Serialize};

use crate::StageTarget;

/// What a certificate bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CertificateKind {
    /// `|K_hat - K*|` for the exact root `K*` of the declared stage
    /// equations, and the induced output/embedded projections.
    StageTargetBound,
}

/// The test family: `J`, `y`, `h` and the quadratic coefficient `q`.
#[derive(Clone, Debug, PartialEq)]
pub struct QuadraticStageProblem {
    pub jacobian: Vec<Vec<f64>>,
    pub y: Vec<f64>,
    pub h: f64,
    pub q: Vec<f64>,
}

impl QuadraticStageProblem {
    pub fn dimension(&self) -> usize {
        self.y.len()
    }

    fn validate(&self) -> CoreResult<()> {
        let n = self.dimension();
        if n == 0
            || self.q.len() != n
            || self.jacobian.len() != n
            || self.jacobian.iter().any(|row| row.len() != n)
            || !(self.h.is_finite() && self.h > 0.0)
            || !self
                .jacobian
                .iter()
                .flatten()
                .chain(&self.y)
                .chain(&self.q)
                .all(|value| value.is_finite())
        {
            return Err(CoreError::InvalidInput(
                "CERTIFICATE_NOT_VALIDATED: invalid quadratic stage problem".into(),
            ));
        }
        Ok(())
    }

    fn jacobian_digest(&self) -> String {
        let bits = self
            .jacobian
            .iter()
            .flatten()
            .map(|value| format!("{:016x}", value.to_bits()))
            .collect::<Vec<_>>()
            .join(",");
        sha256_hex(bits.as_bytes())
    }
}

/// What an inverse witness is a witness for: the operator
/// `W = M - h gamma J` with `M = I`, its structure and its tolerance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessIdentity {
    pub h_bits: u64,
    pub gamma_bits: u64,
    pub jacobian_sha256: String,
    pub structure: String,
    pub tolerance_bits: u64,
}

impl WitnessIdentity {
    pub fn for_problem(
        problem: &QuadraticStageProblem,
        gamma: f64,
        structure: &str,
        tolerance: f64,
    ) -> Self {
        Self {
            h_bits: problem.h.to_bits(),
            gamma_bits: gamma.to_bits(),
            jacobian_sha256: problem.jacobian_digest(),
            structure: structure.into(),
            tolerance_bits: tolerance.to_bits(),
        }
    }
}

/// Cost of building a witness (HOM-03).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessWork {
    pub directed_operations: u64,
    pub stored_values: u64,
}

/// An entrywise upper bound `U >= |W^-1|` tied to the operator it bounds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InverseWitness {
    pub identity: WitnessIdentity,
    pub upper: Vec<Vec<f64>>,
    /// `||I - V W||_inf` (upper) for an approximate-inverse witness, 0 for an
    /// exact structural one.
    pub residual_norm_upper: f64,
    pub work: WitnessWork,
}

fn shifted_entry(
    problem: &QuadraticStageProblem,
    gamma: f64,
    a: usize,
    b: usize,
) -> CoreResult<Interval> {
    let identity = Interval::point(if a == b { 1.0 } else { 0.0 })?;
    let hgj = Interval::point(problem.h)?
        .mul(Interval::point(gamma)?)?
        .mul(Interval::point(problem.jacobian[a][b])?)?;
    identity.sub(hgj)
}

impl InverseWitness {
    /// Diagonal `J`: `U_aa = up(1 / |1 - h gamma J_aa|)`.
    #[allow(clippy::needless_range_loop)] // index form mirrors the matrix formula
    pub fn diagonal(problem: &QuadraticStageProblem, gamma: f64) -> CoreResult<Self> {
        problem.validate()?;
        let n = problem.dimension();
        let mut work = WitnessWork::default();
        let mut upper = vec![vec![0.0; n]; n];
        for a in 0..n {
            for b in 0..n {
                if a != b && problem.jacobian[a][b] != 0.0 {
                    return Err(CoreError::InvalidInput(
                        "INVERSE_WITNESS_UNAVAILABLE: Jacobian is not diagonal".into(),
                    ));
                }
            }
            let entry = shifted_entry(problem, gamma, a, a)?;
            if entry.contains_zero() {
                return Err(CoreError::InvalidInput(
                    "INVERSE_WITNESS_UNAVAILABLE: singular diagonal".into(),
                ));
            }
            upper[a][a] = div_up(1.0, entry.mig())?;
            work.directed_operations += 5;
        }
        work.stored_values = n as u64;
        Ok(Self {
            identity: WitnessIdentity::for_problem(problem, gamma, "diagonal", 0.0),
            upper,
            residual_norm_upper: 0.0,
            work,
        })
    }

    /// `n <= 2`: the adjugate over an interval determinant.
    pub fn small(problem: &QuadraticStageProblem, gamma: f64) -> CoreResult<Self> {
        problem.validate()?;
        let n = problem.dimension();
        let w = |a, b| shifted_entry(problem, gamma, a, b);
        let upper = match n {
            1 => {
                let entry = w(0, 0)?;
                if entry.contains_zero() {
                    return Err(CoreError::InvalidInput(
                        "INVERSE_WITNESS_UNAVAILABLE: singular W".into(),
                    ));
                }
                vec![vec![div_up(1.0, entry.mig())?]]
            }
            2 => {
                let det = w(0, 0)?.mul(w(1, 1)?)?.sub(w(0, 1)?.mul(w(1, 0)?)?)?;
                if det.contains_zero() {
                    return Err(CoreError::InvalidInput(
                        "INVERSE_WITNESS_UNAVAILABLE: determinant interval contains 0".into(),
                    ));
                }
                let adj = [[w(1, 1)?, -w(0, 1)?], [-w(1, 0)?, w(0, 0)?]];
                let mut upper = vec![vec![0.0; 2]; 2];
                for a in 0..2 {
                    for b in 0..2 {
                        upper[a][b] = adj[a][b].div(det)?.mag();
                    }
                }
                upper
            }
            _ => {
                return Err(CoreError::InvalidInput(
                    "INVERSE_WITNESS_UNAVAILABLE: small witness needs n <= 2".into(),
                ));
            }
        };
        Ok(Self {
            identity: WitnessIdentity::for_problem(problem, gamma, "exact-small", 0.0),
            work: WitnessWork {
                directed_operations: (12 * n * n) as u64,
                stored_values: (n * n) as u64,
            },
            upper,
            residual_norm_upper: 0.0,
        })
    }

    /// A general approximate inverse `V`: `R >= |I - V W|` outward and
    /// `theta = ||R||_inf < 1`; then `|W^-1| <= (I - R)^-1 |V| <= |V| + R |V|
    /// + theta^2 / (1 - theta) 1 colmax(|V|)`. A banded or block-triangular
    /// `W` gives a structured `V` with the same bound; nothing here assumes
    /// that a Jacobian-vector product gives entrywise bounds on `J`.
    pub fn approximate(
        problem: &QuadraticStageProblem,
        gamma: f64,
        v: &[Vec<f64>],
        structure: &str,
    ) -> CoreResult<Self> {
        problem.validate()?;
        let n = problem.dimension();
        if v.len() != n || v.iter().any(|row| row.len() != n) {
            return Err(CoreError::Dimension(
                "INVERSE_WITNESS_UNAVAILABLE: approximate inverse shape".into(),
            ));
        }
        let mut work = WitnessWork::default();
        let mut residual = vec![vec![0.0; n]; n];
        for a in 0..n {
            for b in 0..n {
                let mut entry = Interval::point(if a == b { 1.0 } else { 0.0 })?;
                for (c, v_ac) in v[a].iter().enumerate() {
                    entry = entry
                        .sub(Interval::point(*v_ac)?.mul(shifted_entry(problem, gamma, c, b)?)?)?;
                    work.directed_operations += 8;
                }
                residual[a][b] = entry.mag();
            }
        }
        let theta = residual
            .iter()
            .map(|row| sum_up(row.iter().copied()))
            .collect::<CoreResult<Vec<_>>>()?
            .into_iter()
            .fold(0.0_f64, f64::max);
        if theta.is_nan() || theta >= 1.0 {
            return Err(CoreError::InvalidInput(format!(
                "INVERSE_WITNESS_UNAVAILABLE: ||I - V W||_inf <= {theta:e} is not below 1"
            )));
        }
        let abs_v = v
            .iter()
            .map(|row| row.iter().map(|value| value.abs()).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let column_max = (0..n)
            .map(|b| abs_v.iter().map(|row| row[b]).fold(0.0_f64, f64::max))
            .collect::<Vec<_>>();
        let tail = div_up(mul_up(theta, theta)?, sub_down(1.0, theta)?)?;
        let mut upper = vec![vec![0.0; n]; n];
        for a in 0..n {
            for b in 0..n {
                let first = sum_up((0..n).map(|c| {
                    mul_up(residual[a][c], abs_v[c][b]).expect("finite by construction")
                }))?;
                upper[a][b] = add_up(add_up(abs_v[a][b], first)?, mul_up(tail, column_max[b])?)?;
                work.directed_operations += (2 * n + 3) as u64;
            }
        }
        work.stored_values = (2 * n * n) as u64;
        Ok(Self {
            identity: WitnessIdentity::for_problem(problem, gamma, structure, theta),
            upper,
            residual_norm_upper: theta,
            work,
        })
    }

    fn apply_upper(&self, vector: &[f64]) -> CoreResult<Vec<f64>> {
        self.upper
            .iter()
            .map(|row| upper_dot(row, vector))
            .collect()
    }
}

fn upper_dot(a: &[f64], b: &[f64]) -> CoreResult<f64> {
    let mut total = 0.0;
    for (x, y) in a.iter().zip(b) {
        total = add_up(total, mul_up(x.abs(), y.abs())?)?;
    }
    Ok(total)
}

fn interval_dot(weights: &[Interval], values: &[f64]) -> CoreResult<Interval> {
    let mut total = Interval::point(0.0)?;
    for (w, v) in weights.iter().zip(values) {
        total = total.add(w.mul(Interval::point(*v)?)?)?;
    }
    Ok(total)
}

fn column(stages: &[Vec<f64>], a: usize, upto: usize) -> Vec<f64> {
    stages[..upto].iter().map(|stage| stage[a]).collect()
}

/// Residual intervals per stage and component, and increment bounds.
type ResidualEnclosure = (Vec<Vec<Interval>>, Vec<Vec<f64>>);

/// Residual intervals `r_i` of the declared stage equations at the
/// candidate and the upper bounds `|delta_i|` of the candidate's stage
/// increments.
fn residual_enclosure(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    candidate: &[Vec<f64>],
) -> CoreResult<ResidualEnclosure> {
    let n = problem.dimension();
    let h = Interval::point(problem.h)?;
    let gamma = Interval::point(target.gamma)?;
    let mut rhs = Vec::with_capacity(n);
    for a in 0..n {
        let jy = interval_dot(
            &problem.jacobian[a]
                .iter()
                .map(|value| Interval::point(*value))
                .collect::<CoreResult<Vec<_>>>()?,
            &problem.y,
        )?;
        let qyy = Interval::point(problem.q[a])?
            .mul(Interval::point(problem.y[a])?)?
            .mul(Interval::point(problem.y[a])?)?;
        rhs.push(h.mul(jy.sub(qyy)?)?);
    }
    let mut residuals = Vec::with_capacity(target.stages());
    let mut increments = Vec::with_capacity(target.stages());
    for i in 0..target.stages() {
        let alpha = target.alpha_rows[i]
            .iter()
            .map(|value| Interval::point(*value))
            .collect::<CoreResult<Vec<_>>>()?;
        let delta = (0..n)
            .map(|a| interval_dot(&alpha, &column(candidate, a, i)))
            .collect::<CoreResult<Vec<_>>>()?;
        let coupled = (0..n)
            .map(|a| interval_dot(&target.coupling_rows[i], &column(candidate, a, i)))
            .collect::<CoreResult<Vec<_>>>()?;
        let mut row = Vec::with_capacity(n);
        for a in 0..n {
            let mut wk = Interval::point(0.0)?;
            let mut jc = Interval::point(0.0)?;
            for b in 0..n {
                let identity = Interval::point(if a == b { 1.0 } else { 0.0 })?;
                let j_ab = Interval::point(problem.jacobian[a][b])?;
                let w_ab = identity.sub(h.mul(gamma)?.mul(j_ab)?)?;
                wk = wk.add(w_ab.mul(Interval::point(candidate[i][b])?)?)?;
                jc = jc.add(j_ab.mul(coupled[b])?)?;
            }
            let quadratic = h
                .mul(Interval::point(problem.q[a])?)?
                .mul(delta[a].mul(delta[a])?)?;
            row.push(wk.sub(h.mul(jc)?)?.sub(rhs[a])?.sub(quadratic)?);
        }
        increments.push(delta.iter().map(Interval::mag).collect());
        residuals.push(row);
    }
    Ok((residuals, increments))
}

/// A verified stage-target certificate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StageCertificate {
    pub kind: CertificateKind,
    pub target_id: String,
    pub witness: WitnessIdentity,
    /// SHA-256 of the candidate stage bits the bound is about.
    pub candidate_sha256: String,
    /// `E_i >= |K_hat_i - K*_i|` componentwise.
    pub stage_bound: Vec<Vec<f64>>,
    pub output_bound: Vec<f64>,
    pub embedded_difference_bound: Vec<f64>,
    pub output_wrms_upper: f64,
    pub embedded_difference_wrms_upper: f64,
    /// WRMS of `|e_hat| + B_e`: an upper bound on the target's embedded
    /// estimate (an error proxy, not an ODE error bound).
    pub embedded_target_wrms_upper: f64,
    /// WRMS of `(|e_hat| - B_e)_+`: a lower bound on the same.
    pub embedded_target_wrms_lower: f64,
    pub combined_proxy_upper: f64,
    pub directed_operations: u64,
}

/// Digest of a candidate's stage bits.
pub fn candidate_digest(candidate: &[Vec<f64>]) -> String {
    let bits = candidate
        .iter()
        .flatten()
        .map(|value| format!("{:016x}", value.to_bits()))
        .collect::<Vec<_>>()
        .join(",");
    sha256_hex(bits.as_bytes())
}

fn validate_inputs(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    candidate: &[Vec<f64>],
    witness: &InverseWitness,
) -> CoreResult<()> {
    problem.validate()?;
    let n = problem.dimension();
    if !target.strictly_lower_nilpotent() {
        return Err(CoreError::InvalidInput(
            "TARGET_SEMANTICS_UNRESOLVED: target is not strictly lower".into(),
        ));
    }
    if candidate.len() != target.stages()
        || candidate
            .iter()
            .any(|stage| stage.len() != n || !stage.iter().all(|value| value.is_finite()))
    {
        return Err(CoreError::InvalidInput(
            "CERTIFICATE_NOT_VALIDATED: candidate shape or values".into(),
        ));
    }
    let expected = WitnessIdentity {
        structure: witness.identity.structure.clone(),
        tolerance_bits: witness.identity.tolerance_bits,
        ..WitnessIdentity::for_problem(problem, target.gamma, "", 0.0)
    };
    if witness.identity != expected || witness.upper.len() != n {
        return Err(CoreError::InvalidInput(
            "CERTIFICATE_NOT_VALIDATED: the inverse witness is for another operator".into(),
        ));
    }
    Ok(())
}

/// WRMS upper bound with the scale bounded from below.
fn wrms_upper(values: &[f64], scale_lower: &[f64]) -> CoreResult<f64> {
    let mut total = 0.0;
    for (value, scale) in values.iter().zip(scale_lower) {
        let ratio = div_up(value.abs(), *scale)?;
        total = add_up(total, mul_up(ratio, ratio)?)?;
    }
    sqrt_up(div_up(total, values.len() as f64)?)
}

fn wrms_lower(values: &[f64], scale_upper: &[f64]) -> CoreResult<f64> {
    use rodas5p_core::directed::{add_down, div_down, mul_down, sqrt_down};
    let mut total = 0.0;
    for (value, scale) in values.iter().zip(scale_upper) {
        let ratio = div_down(value.abs(), *scale)?;
        total = add_down(total, mul_down(ratio, ratio)?)?;
    }
    sqrt_down(div_down(total, values.len() as f64)?)
}

/// The projection part shared by both certificates.
#[allow(clippy::too_many_arguments)]
fn finish_certificate(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    candidate: &[Vec<f64>],
    y_hat: &[f64],
    e_hat: &[f64],
    stage_bound: Vec<Vec<f64>>,
    witness: &InverseWitness,
    atol: f64,
    rtol: f64,
    mut operations: u64,
) -> CoreResult<StageCertificate> {
    use rodas5p_core::directed::{add_down, mul_down};
    let n = problem.dimension();
    if y_hat.len() != n || e_hat.len() != n {
        return Err(CoreError::Dimension(
            "CERTIFICATE_NOT_VALIDATED: projection shape".into(),
        ));
    }
    let b = target
        .b
        .iter()
        .map(|value| Interval::point(*value))
        .collect::<CoreResult<Vec<_>>>()?;
    let bt = target
        .btilde
        .iter()
        .map(|value| Interval::point(*value))
        .collect::<CoreResult<Vec<_>>>()?;
    let mut output_bound = Vec::with_capacity(n);
    let mut embedded_bound = Vec::with_capacity(n);
    for a in 0..n {
        let stages = column(candidate, a, target.stages());
        let bounds = column(&stage_bound, a, target.stages());
        let y_exact = Interval::point(problem.y[a])?.add(interval_dot(&b, &stages)?)?;
        let e_exact = interval_dot(&bt, &stages)?;
        let rounding_y = y_exact.sub(Interval::point(y_hat[a])?)?.mag();
        let rounding_e = e_exact.sub(Interval::point(e_hat[a])?)?.mag();
        output_bound.push(add_up(upper_dot(&target.b, &bounds)?, rounding_y)?);
        embedded_bound.push(add_up(upper_dot(&target.btilde, &bounds)?, rounding_e)?);
        operations += (8 * target.stages() + 4) as u64;
    }
    let scale_lower = (0..n)
        .map(|a| {
            add_down(
                atol,
                mul_down(rtol, problem.y[a].abs().max(y_hat[a].abs()))?,
            )
        })
        .collect::<CoreResult<Vec<_>>>()?;
    let scale_upper = (0..n)
        .map(|a| add_up(atol, mul_up(rtol, problem.y[a].abs().max(y_hat[a].abs()))?))
        .collect::<CoreResult<Vec<_>>>()?;
    if scale_lower
        .iter()
        .any(|scale| scale.is_nan() || *scale <= 0.0)
    {
        return Err(CoreError::InvalidInput(
            "CERTIFICATE_NOT_VALIDATED: nonpositive error scale".into(),
        ));
    }
    let output_wrms_upper = wrms_upper(&output_bound, &scale_lower)?;
    let embedded_difference_wrms_upper = wrms_upper(&embedded_bound, &scale_lower)?;
    let embedded_upper = (0..n)
        .map(|a| add_up(e_hat[a].abs(), embedded_bound[a]))
        .collect::<CoreResult<Vec<_>>>()?;
    let embedded_lower = (0..n)
        .map(|a| Ok(sub_down(e_hat[a].abs(), embedded_bound[a])?.max(0.0)))
        .collect::<CoreResult<Vec<_>>>()?;
    let embedded_target_wrms_upper = wrms_upper(&embedded_upper, &scale_lower)?;
    let embedded_target_wrms_lower = wrms_lower(&embedded_lower, &scale_upper)?;
    Ok(StageCertificate {
        kind: CertificateKind::StageTargetBound,
        target_id: target.id.into(),
        witness: witness.identity.clone(),
        candidate_sha256: candidate_digest(candidate),
        stage_bound,
        output_bound,
        embedded_difference_bound: embedded_bound,
        output_wrms_upper,
        embedded_difference_wrms_upper,
        embedded_target_wrms_upper,
        embedded_target_wrms_lower,
        combined_proxy_upper: add_up(output_wrms_upper, embedded_target_wrms_upper)?,
        directed_operations: operations + witness.work.directed_operations,
    })
}

/// The serial certificate (HOM-02): residual enclosure and the positive
/// recurrence
/// `E_i = U [ |r_i| + h (|J| sum_{j<i} |L*_ij| E_j + |q| (2 |delta_i| d_i + d_i^2)) ]`,
/// `d_i = sum_{j<i} |alpha_ij| E_j`, every operation rounded upward. Any non-finite intermediate, a witness
/// for another operator or a target that is not strictly lower is a typed
/// rejection.
#[allow(clippy::too_many_arguments)]
pub fn certify_stage_target(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    candidate: &[Vec<f64>],
    y_hat: &[f64],
    e_hat: &[f64],
    witness: &InverseWitness,
    atol: f64,
    rtol: f64,
) -> CoreResult<StageCertificate> {
    validate_inputs(target, problem, candidate, witness)?;
    let n = problem.dimension();
    let (residuals, increments) = residual_enclosure(target, problem, candidate)?;
    let mut operations = (target.stages() * n * (6 * n + 12)) as u64;
    let mut bound: Vec<Vec<f64>> = Vec::with_capacity(target.stages());
    for i in 0..target.stages() {
        let coupling = target.coupling_rows[i]
            .iter()
            .map(Interval::mag)
            .collect::<Vec<_>>();
        let mut right = Vec::with_capacity(n);
        let d = (0..n)
            .map(|a| upper_dot(&target.alpha_rows[i], &column(&bound, a, i)))
            .collect::<CoreResult<Vec<_>>>()?;
        let lc = (0..n)
            .map(|a| upper_dot(&coupling, &column(&bound, a, i)))
            .collect::<CoreResult<Vec<_>>>()?;
        for a in 0..n {
            let linear = upper_dot(&problem.jacobian[a], &lc)?;
            let remainder = mul_up(
                problem.q[a].abs(),
                add_up(
                    mul_up(mul_up(2.0, increments[i][a])?, d[a])?,
                    mul_up(d[a], d[a])?,
                )?,
            )?;
            right.push(add_up(
                residuals[i][a].mag(),
                mul_up(problem.h, add_up(linear, remainder)?)?,
            )?);
        }
        bound.push(witness.apply_upper(&right)?);
        operations += (n * (4 * i + 4 * n + 8)) as u64;
    }
    finish_certificate(
        target, problem, candidate, y_hat, e_hat, bound, witness, atol, rtol, operations,
    )
}

/// Data from earlier accepted steps that may set the a priori radius. It
/// has no field for a reference solution or for the current step's serial
/// bound: a predictor cannot leak them (HOM-04).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PastStepData {
    /// Largest residual magnitude of the last accepted step, if any.
    pub previous_max_residual: Option<f64>,
    pub previous_h: Option<f64>,
    pub order: i32,
}

/// `D = max(D0, kappa max|r| (h / h_prev)^p)` from past accepted data only.
pub fn predict_state_radius(
    past: &PastStepData,
    h: f64,
    floor: f64,
    kappa: f64,
) -> CoreResult<f64> {
    let predicted = match (past.previous_max_residual, past.previous_h) {
        (Some(residual), Some(previous_h)) if previous_h > 0.0 => {
            let ratio = div_up(h, previous_h)?;
            let mut power = 1.0_f64;
            for _ in 0..past.order.max(0) {
                power = mul_up(power, ratio)?;
            }
            mul_up(mul_up(kappa, residual.abs())?, power)?
        }
        _ => 0.0,
    };
    Ok(floor.max(predicted))
}

/// One radius attempt, kept whether or not it closed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RadiusAttempt {
    pub attempt: usize,
    pub radius: f64,
    pub closes: bool,
    pub max_state_radius: f64,
    pub reason: Option<String>,
}

/// The path-sum certificate and the ledger of every radius tried.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DoublingCertificate {
    pub certificate: Option<StageCertificate>,
    pub attempts: Vec<RadiusAttempt>,
    pub workers: usize,
}

type Matrix = Vec<Vec<f64>>;

/// `A B` rounded upward for nonnegative matrices, rows in parallel with a
/// fixed per-entry summation order (bitwise identical for any worker count).
fn upper_matmul(a: &Matrix, b: &Matrix, workers: usize) -> CoreResult<Matrix> {
    let m = b.first().map_or(0, Vec::len);
    let row = |left: &Vec<f64>| -> CoreResult<Vec<f64>> {
        (0..m)
            .map(|c| {
                let mut total = 0.0;
                for (k, value) in left.iter().enumerate() {
                    if *value != 0.0 && b[k][c] != 0.0 {
                        total = add_up(total, mul_up(*value, b[k][c])?)?;
                    }
                }
                Ok(total)
            })
            .collect()
    };
    if workers <= 1 {
        return a.iter().map(row).collect();
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .map_err(|error| CoreError::InvalidInput(format!("certificate workers: {error}")))?;
    pool.install(|| a.par_iter().map(row).collect())
}

/// HOM-04: `E = S a`, `S = (I + H^4)(I + H^2)(I + H)` evaluated as three
/// doubling levels (`S += Q S; Q = Q^2`), with
/// `H_(i,u),(j,v) = h sum_w U_uw (|J_wv| |L*_ij| + [w = v] ell_iw |alpha_ij|)`
/// for `j < i`, `ell_iw = |q_w| (2 |delta_iw| + D)`, and `a_i = U |r_i|`.
/// The radius must close; each failed radius is recorded and retried at
/// four times the radius, up to `max_attempts`.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::needless_range_loop)] // index form mirrors the matrix formula
pub fn doubling_certificate(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    candidate: &[Vec<f64>],
    y_hat: &[f64],
    e_hat: &[f64],
    witness: &InverseWitness,
    atol: f64,
    rtol: f64,
    initial_radius: f64,
    max_attempts: usize,
    workers: usize,
) -> CoreResult<DoublingCertificate> {
    validate_inputs(target, problem, candidate, witness)?;
    let n = problem.dimension();
    let s = target.stages();
    let m = s * n;
    let (residuals, increments) = residual_enclosure(target, problem, candidate)?;
    let mut a = Vec::with_capacity(m);
    for residual in &residuals {
        let magnitudes = residual.iter().map(Interval::mag).collect::<Vec<_>>();
        a.extend(witness.apply_upper(&magnitudes)?);
    }
    let mut attempts = Vec::new();
    let mut radius = initial_radius;
    for attempt in 0..max_attempts.max(1) {
        let mut h_matrix = vec![vec![0.0; m]; m];
        for i in 0..s {
            for j in 0..i {
                let coupling = target.coupling_rows[i][j].mag();
                let alpha = target.alpha_rows[i][j].abs();
                for u in 0..n {
                    for v in 0..n {
                        let mut inner = 0.0;
                        for w in 0..n {
                            let ell = mul_up(
                                problem.q[w].abs(),
                                add_up(mul_up(2.0, increments[i][w])?, radius)?,
                            )?;
                            let mut term = mul_up(problem.jacobian[w][v].abs(), coupling)?;
                            if w == v {
                                term = add_up(term, mul_up(ell, alpha)?)?;
                            }
                            inner = add_up(inner, mul_up(witness.upper[u][w], term)?)?;
                        }
                        h_matrix[i * n + u][j * n + v] = mul_up(problem.h, inner)?;
                    }
                }
            }
        }
        let mut sum = (0..m)
            .map(|r| (0..m).map(|c| if r == c { 1.0 } else { 0.0 }).collect())
            .collect::<Matrix>();
        let mut power = h_matrix;
        for level in 0..3 {
            let product = upper_matmul(&power, &sum, workers)?;
            for (row, add) in sum.iter_mut().zip(&product) {
                for (value, extra) in row.iter_mut().zip(add) {
                    *value = add_up(*value, *extra)?;
                }
            }
            if level < 2 {
                power = upper_matmul(&power, &power, workers)?;
            }
        }
        let flat = sum
            .iter()
            .map(|row| upper_dot(row, &a))
            .collect::<CoreResult<Vec<_>>>()?;
        let bound = (0..s)
            .map(|i| flat[i * n..(i + 1) * n].to_vec())
            .collect::<Vec<_>>();
        let mut max_state_radius = 0.0_f64;
        for i in 0..s {
            for v in 0..n {
                let state = upper_dot(&target.alpha_rows[i], &column(&bound, v, i))?;
                max_state_radius = max_state_radius.max(state);
            }
        }
        let closes = max_state_radius <= radius;
        attempts.push(RadiusAttempt {
            attempt,
            radius,
            closes,
            max_state_radius,
            reason: (!closes).then(|| "RADIUS_CLOSURE_FAIL".into()),
        });
        if closes {
            let certificate = finish_certificate(
                target,
                problem,
                candidate,
                y_hat,
                e_hat,
                bound,
                witness,
                atol,
                rtol,
                (5 * m * m * m) as u64,
            )?;
            return Ok(DoublingCertificate {
                certificate: Some(certificate),
                attempts,
                workers,
            });
        }
        radius = mul_up(radius, 4.0)?;
    }
    Ok(DoublingCertificate {
        certificate: None,
        attempts,
        workers,
    })
}
