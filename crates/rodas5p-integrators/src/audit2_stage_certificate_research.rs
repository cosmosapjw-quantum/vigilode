//! Candidate-free, feature-gated synthetic stage-certificate checks.
//!
//! This module accepts only the deliberately narrow
//! `dimensionless-synthetic-l2/v1` schema.  It neither dispatches an
//! integrator nor makes a real-client or production claim.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const STAGE_CERTIFICATE_NORM_SCHEMA: &str = "dimensionless-synthetic-l2/v1";
pub const UNIT_SCALE_BITS: u64 = 1.0_f64.to_bits();

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrozenJsonDocument {
    pub document: Value,
    pub canonical_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageCertificateNorm {
    pub schema: String,
    pub scale_bits: Vec<u64>,
}

impl StageCertificateNorm {
    #[must_use]
    pub fn canonical(dimension: usize) -> Self {
        Self {
            schema: STAGE_CERTIFICATE_NORM_SCHEMA.into(),
            scale_bits: vec![UNIT_SCALE_BITS; dimension],
        }
    }

    fn validate(&self, dimension: usize) -> Result<(), Audit2StageCertificateError> {
        if self.schema != STAGE_CERTIFICATE_NORM_SCHEMA
            || dimension == 0
            || self.scale_bits.len() != dimension
            || self.scale_bits.iter().any(|bits| *bits != UNIT_SCALE_BITS)
        {
            return Err(Audit2StageCertificateError::InvalidNormSchema);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageCertificateProvenance {
    pub coefficient_digest: String,
    pub operator_identity: String,
    pub preconditioner_identity: String,
    /// Digest of every stage right-hand side, in stage order.
    pub rhs_digest: String,
    /// Digest of the inputs that drive the decision: `strict_lower`, both
    /// weight vectors, `kappa_upper`, `ehat`, every stage approximate solution
    /// and the inverse witness (audit F-064).
    pub decision_input_digest: String,
    pub restart: u32,
    pub max_arnoldi: u32,
    pub iteration_limit: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageCertificateWork {
    pub operator_applies: u64,
    pub preconditioner_applies: u64,
    pub arnoldi_iterations: u64,
}

impl StageCertificateWork {
    fn checked_add(self, other: Self) -> Result<Self, Audit2StageCertificateError> {
        Ok(Self {
            operator_applies: self
                .operator_applies
                .checked_add(other.operator_applies)
                .ok_or(Audit2StageCertificateError::InvalidWork)?,
            preconditioner_applies: self
                .preconditioner_applies
                .checked_add(other.preconditioner_applies)
                .ok_or(Audit2StageCertificateError::InvalidWork)?,
            arnoldi_iterations: self
                .arnoldi_iterations
                .checked_add(other.arnoldi_iterations)
                .ok_or(Audit2StageCertificateError::InvalidWork)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StageCertificateStageTrace {
    pub stage_index: usize,
    pub residual_history: Vec<f64>,
    pub complete: bool,
    pub work: StageCertificateWork,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageCertificatePartialFailure {
    pub stage_index: usize,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StageCertificateTrace {
    pub frozen_trace: FrozenJsonDocument,
    pub stage_traces: Vec<StageCertificateStageTrace>,
    pub completed_work: StageCertificateWork,
    pub partial_failure: Option<StageCertificatePartialFailure>,
}

/// One stage linear solve `W K_i = b_i` with its approximate solution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StageCertificateStageSolve {
    pub rhs: Vec<f64>,
    pub approximate_solution: Vec<f64>,
    pub caller_product_upper: f64,
    pub caller_q_upper: f64,
}

/// Input of the synthetic stage certificate.
///
/// The state dimension `n` is the size of the shared operator `W`; the stage
/// count `s` is the number of stage solves, trace rows, weights and the size
/// of `strict_lower`. The two are independent (audit F-060).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StageCertificateInput {
    pub frozen_plan: FrozenJsonDocument,
    pub trace: StageCertificateTrace,
    pub provenance: StageCertificateProvenance,
    pub norm: StageCertificateNorm,
    pub coefficients: Vec<f64>,
    pub operator: Vec<Vec<f64>>,
    pub preconditioner: Vec<Vec<f64>>,
    /// Approximate inverse `V` of `W`; it lets the evaluator verify that
    /// `kappa_upper` bounds `||W^{-1}||_2` (audit F-059).
    pub inverse_witness: Vec<Vec<f64>>,
    pub stages: Vec<StageCertificateStageSolve>,
    pub kappa_upper: f64,
    pub strict_lower: Vec<Vec<f64>>,
    pub endpoint_weights: Vec<f64>,
    pub estimator_weights: Vec<f64>,
    pub ehat: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageCertificateDecision {
    SyntheticConsistentAccept,
    SyntheticConsistentReject,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageCertificateReceiptAuthority {
    SyntheticSchemaConsistencyOnly,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StageCertificateReceipt {
    pub decision: StageCertificateDecision,
    pub authority: StageCertificateReceiptAuthority,
    pub frozen_plan: FrozenJsonDocument,
    pub trace: StageCertificateTrace,
    pub provenance: StageCertificateProvenance,
    pub norm: StageCertificateNorm,
    /// Verified upper bound on `||W^{-1}||_2` from the inverse witness.
    pub inverse_norm_upper: f64,
    pub solution_l2_upper: Vec<f64>,
    pub residual_l2_upper: Vec<f64>,
    pub directed_product_upper: Vec<f64>,
    pub q_upper: Vec<f64>,
    pub stage_majorant: Vec<f64>,
    pub endpoint_contamination: f64,
    pub estimator_contamination: f64,
    pub theta: f64,
    pub ehat_plus_theta: f64,
    pub ehat_minus_theta_lower: f64,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum Audit2StageCertificateError {
    #[error("stage certificate uses a noncanonical norm schema or scale")]
    InvalidNormSchema,
    #[error("stage certificate field is missing, nonfinite, negative, or inconsistent")]
    InvalidField,
    #[error("stage certificate frozen JSON document does not match its canonical SHA-256")]
    FrozenJsonDigestMismatch,
    #[error("stage certificate provenance identity does not match exact input bits")]
    ProvenanceMismatch,
    #[error("stage certificate residual history is incomplete")]
    IncompleteResidualHistory,
    #[error("stage certificate partial trace cannot produce a decision")]
    PartialTraceCannotDecide,
    #[error("stage certificate completed work is inconsistent or overflows")]
    InvalidWork,
    #[error("stage certificate caller bound is downward-rounded")]
    DownwardRoundedBound,
    #[error("stage certificate interval is neither a safe accept nor a safe reject")]
    InconclusiveInterval,
    #[error("stage certificate trace row exceeds the frozen max_arnoldi cap")]
    MaxArnoldiExceeded,
    #[error("stage certificate kappa does not bound the verified inverse norm")]
    KappaPremiseNotVerified,
    #[error("stage certificate upper bound overflowed; no decision")]
    NonFiniteBound,
}

/// Returns the SHA-256 of a JSON value serialized with recursively sorted map keys.
pub fn canonical_json_sha256(value: &Value) -> Result<String, Audit2StageCertificateError> {
    let normalized = canonicalize_json(value);
    let bytes =
        serde_json::to_vec(&normalized).map_err(|_| Audit2StageCertificateError::InvalidField)?;
    Ok(hex_lower(&Sha256::digest(bytes)))
}

/// Binds numerical input identities to the big-endian concatenation of IEEE-754 bits.
#[must_use]
pub fn audit2_stage_certificate_digest_f64_bits(bits: &[u64]) -> String {
    let mut hash = Sha256::new();
    for value in bits {
        hash.update(value.to_be_bytes());
    }
    hex_lower(&hash.finalize())
}

/// Preserves exactly the completed prefix and its accumulated work after an injected failure.
pub fn retain_completed_stage_traces(
    frozen_trace: FrozenJsonDocument,
    stage_traces: Vec<StageCertificateStageTrace>,
    injected_failure: Option<(usize, String)>,
) -> Result<StageCertificateTrace, Audit2StageCertificateError> {
    verify_frozen_json(&frozen_trace)?;
    let cutoff = injected_failure
        .as_ref()
        .map_or(stage_traces.len(), |(index, _)| *index);
    if cutoff > stage_traces.len()
        || injected_failure
            .as_ref()
            .is_some_and(|(_, reason)| reason.trim().is_empty())
    {
        return Err(Audit2StageCertificateError::InvalidField);
    }
    let mut completed_work = StageCertificateWork::default();
    let mut retained = Vec::with_capacity(cutoff);
    for (position, stage) in stage_traces.into_iter().enumerate() {
        if position >= cutoff {
            break;
        }
        if stage.stage_index != position
            || !stage.complete
            || stage.work.arnoldi_iterations == 0
            || usize::try_from(stage.work.arnoldi_iterations)
                .ok()
                .and_then(|iterations| iterations.checked_add(1))
                != Some(stage.residual_history.len())
            || stage
                .residual_history
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(Audit2StageCertificateError::IncompleteResidualHistory);
        }
        completed_work = completed_work.checked_add(stage.work)?;
        retained.push(stage);
    }
    Ok(StageCertificateTrace {
        frozen_trace,
        stage_traces: retained,
        completed_work,
        partial_failure: injected_failure.map(|(stage_index, reason)| {
            StageCertificatePartialFailure {
                stage_index,
                reason,
            }
        }),
    })
}

/// Recomputes the narrow synthetic L2 contract and admits only safe intervals.
///
/// Every quantity that enters the decision is an upper bound computed with
/// correctly rounded upward binary64 arithmetic, including the residuals and
/// norms (audit F-061, F-101). Each stage has its own residual bound
/// (audit F-060), and `kappa_upper` must dominate a verified bound on
/// `||W^{-1}||_2` (audit F-059).
pub fn evaluate_audit2_stage_certificate(
    input: StageCertificateInput,
) -> Result<StageCertificateReceipt, Audit2StageCertificateError> {
    verify_frozen_json(&input.frozen_plan)?;
    validate_provenance_policy(&input.provenance)?;
    validate_finite(&input.coefficients)?;
    let dimension = input.operator.len();
    let stage_count = input.stages.len();
    input.norm.validate(dimension)?;
    verify_trace(&input.trace, &input.provenance)?;
    if dimension == 0
        || stage_count == 0
        || !is_square(&input.operator, dimension)
        || !is_square(&input.preconditioner, dimension)
        || !is_square(&input.inverse_witness, dimension)
        || input.stages.iter().any(|stage| {
            stage.rhs.len() != dimension || stage.approximate_solution.len() != dimension
        })
        || !is_square(&input.strict_lower, stage_count)
        || input.endpoint_weights.len() != stage_count
        || input.estimator_weights.len() != stage_count
        || input.trace.stage_traces.len() != stage_count
        || !input.kappa_upper.is_finite()
        || input.kappa_upper < 0.0
        || !input.ehat.is_finite()
        || input.ehat < 0.0
    {
        return Err(Audit2StageCertificateError::InvalidField);
    }
    for stage in &input.stages {
        validate_finite(&stage.rhs)?;
        validate_finite(&stage.approximate_solution)?;
        if !stage.caller_product_upper.is_finite()
            || stage.caller_product_upper < 0.0
            || !stage.caller_q_upper.is_finite()
            || stage.caller_q_upper < 0.0
        {
            return Err(Audit2StageCertificateError::InvalidField);
        }
    }
    validate_matrix_finite(&input.operator)?;
    validate_matrix_finite(&input.preconditioner)?;
    validate_matrix_finite(&input.inverse_witness)?;
    validate_nonnegative(&input.endpoint_weights)?;
    validate_nonnegative(&input.estimator_weights)?;
    validate_strict_lower(&input.strict_lower)?;
    verify_provenance(&input)?;

    let inverse_norm_upper = verified_inverse_norm_upper(&input.operator, &input.inverse_witness)?;
    if input.kappa_upper < inverse_norm_upper {
        return Err(Audit2StageCertificateError::KappaPremiseNotVerified);
    }

    let mut solution_l2_upper = Vec::with_capacity(stage_count);
    let mut residual_l2_upper = Vec::with_capacity(stage_count);
    let mut directed_product_upper = Vec::with_capacity(stage_count);
    let mut q_upper = Vec::with_capacity(stage_count);
    for stage in &input.stages {
        let residual =
            residual_magnitude_upper(&input.operator, &stage.approximate_solution, &stage.rhs)?;
        let residual_norm = l2_upper(&residual)?;
        let solution_magnitude: Vec<f64> = stage
            .approximate_solution
            .iter()
            .map(|value| value.abs())
            .collect();
        let solution_norm = l2_upper(&solution_magnitude)?;
        let product = audit2_upper_mul(input.kappa_upper, residual_norm)?;
        if stage.caller_product_upper < product {
            return Err(Audit2StageCertificateError::DownwardRoundedBound);
        }
        let q = audit2_upper_add(solution_norm, product)?;
        if stage.caller_q_upper < q {
            return Err(Audit2StageCertificateError::DownwardRoundedBound);
        }
        solution_l2_upper.push(solution_norm);
        residual_l2_upper.push(residual_norm);
        directed_product_upper.push(product);
        q_upper.push(q);
    }
    let stage_majorant = forward_stage_majorant(&input.strict_lower, &q_upper)?;
    let endpoint_contamination = nonnegative_dot(&input.endpoint_weights, &stage_majorant)?;
    let estimator_contamination = nonnegative_dot(&input.estimator_weights, &stage_majorant)?;
    let theta = audit2_upper_add(endpoint_contamination, estimator_contamination)?;
    let ehat_plus_theta = audit2_upper_add(input.ehat, theta)?;
    let ehat_minus_theta_lower = lower_nonnegative_difference(input.ehat, theta);
    let decision = if ehat_plus_theta <= 1.0 {
        StageCertificateDecision::SyntheticConsistentAccept
    } else if ehat_minus_theta_lower > 1.0 {
        StageCertificateDecision::SyntheticConsistentReject
    } else {
        return Err(Audit2StageCertificateError::InconclusiveInterval);
    };
    Ok(StageCertificateReceipt {
        decision,
        authority: StageCertificateReceiptAuthority::SyntheticSchemaConsistencyOnly,
        frozen_plan: input.frozen_plan,
        trace: input.trace,
        provenance: input.provenance,
        norm: input.norm,
        inverse_norm_upper,
        solution_l2_upper,
        residual_l2_upper,
        directed_product_upper,
        q_upper,
        stage_majorant,
        endpoint_contamination,
        estimator_contamination,
        theta,
        ehat_plus_theta,
        ehat_minus_theta_lower,
    })
}

/// Digest of all stage right-hand sides, in stage order.
#[must_use]
pub fn audit2_stage_certificate_rhs_digest(stages: &[StageCertificateStageSolve]) -> String {
    let bits: Vec<u64> = stages
        .iter()
        .flat_map(|stage| stage.rhs.iter().map(|value| value.to_bits()))
        .collect();
    audit2_stage_certificate_digest_f64_bits(&bits)
}

/// Digest of the decision-driving inputs, in the order documented on
/// [`StageCertificateProvenance::decision_input_digest`].
#[must_use]
pub fn audit2_stage_certificate_decision_input_digest(input: &StageCertificateInput) -> String {
    let mut bits = Vec::new();
    bits.extend(
        input
            .strict_lower
            .iter()
            .flatten()
            .map(|value| value.to_bits()),
    );
    bits.extend(input.endpoint_weights.iter().map(|value| value.to_bits()));
    bits.extend(input.estimator_weights.iter().map(|value| value.to_bits()));
    bits.push(input.kappa_upper.to_bits());
    bits.push(input.ehat.to_bits());
    for stage in &input.stages {
        bits.extend(
            stage
                .approximate_solution
                .iter()
                .map(|value| value.to_bits()),
        );
    }
    bits.extend(
        input
            .inverse_witness
            .iter()
            .flatten()
            .map(|value| value.to_bits()),
    );
    audit2_stage_certificate_digest_f64_bits(&bits)
}

/// Correctly rounded upward sum of two finite nonnegative binary64 values.
///
/// The rounded-to-nearest sum is returned unchanged when it is exact, which
/// includes every sum with a zero operand; otherwise the next binary64 value
/// above it. The rounding error comes from the error-free TwoSum
/// transformation, so no ambient rounding mode is changed. Overflow is a
/// typed rejection with no decision.
pub fn audit2_upper_add(a: f64, b: f64) -> Result<f64, Audit2StageCertificateError> {
    if !a.is_finite() || !b.is_finite() || a < 0.0 || b < 0.0 {
        return Err(Audit2StageCertificateError::InvalidField);
    }
    let sum = a + b;
    if !sum.is_finite() {
        return Err(Audit2StageCertificateError::NonFiniteBound);
    }
    let b_virtual = sum - a;
    let error = (a - (sum - b_virtual)) + (b - b_virtual);
    finite_bound(if error > 0.0 { sum.next_up() } else { sum })
}

/// Correctly rounded upward product of two finite nonnegative binary64 values.
///
/// The rounding error of a normal product comes from one fused multiply-add
/// (`a * b - p` is exact there). Below `2^-969` that error may itself round,
/// so a nonzero product in that range is moved up unconditionally, which is
/// conservative. A positive product that underflows to zero becomes the
/// smallest subnormal. Overflow is a typed rejection with no decision.
pub fn audit2_upper_mul(a: f64, b: f64) -> Result<f64, Audit2StageCertificateError> {
    if !a.is_finite() || !b.is_finite() || a < 0.0 || b < 0.0 {
        return Err(Audit2StageCertificateError::InvalidField);
    }
    if a == 0.0 || b == 0.0 {
        return Ok(0.0);
    }
    let product = a * b;
    if !product.is_finite() {
        return Err(Audit2StageCertificateError::NonFiniteBound);
    }
    const EXACT_ERROR_THRESHOLD: f64 = f64::MIN_POSITIVE * 9_007_199_254_740_992.0; // 2^-969
    if product < EXACT_ERROR_THRESHOLD {
        return finite_bound(product.next_up());
    }
    let error = a.mul_add(b, -product);
    finite_bound(if error > 0.0 {
        product.next_up()
    } else {
        product
    })
}

fn finite_bound(value: f64) -> Result<f64, Audit2StageCertificateError> {
    value
        .is_finite()
        .then_some(value)
        .ok_or(Audit2StageCertificateError::NonFiniteBound)
}

/// Correctly rounded upward quotient `a / b` for finite `a >= 0`, `b > 0`.
fn upper_div(a: f64, b: f64) -> Result<f64, Audit2StageCertificateError> {
    if !a.is_finite() || !b.is_finite() || a < 0.0 || b <= 0.0 {
        return Err(Audit2StageCertificateError::InvalidField);
    }
    let quotient = a / b;
    if !quotient.is_finite() {
        return Err(Audit2StageCertificateError::NonFiniteBound);
    }
    if quotient < f64::MIN_POSITIVE * 9_007_199_254_740_992.0 {
        return finite_bound(quotient.next_up());
    }
    // q * b - a < 0 means the rounded quotient lies below a / b.
    let error = quotient.mul_add(b, -a);
    finite_bound(if error < 0.0 {
        quotient.next_up()
    } else {
        quotient
    })
}

/// Correctly rounded upward square root of a finite nonnegative value.
fn upper_sqrt(value: f64) -> Result<f64, Audit2StageCertificateError> {
    if !value.is_finite() || value < 0.0 {
        return Err(Audit2StageCertificateError::InvalidField);
    }
    let root = value.sqrt();
    if root == 0.0 {
        return Ok(0.0);
    }
    let error = root.mul_add(root, -value);
    finite_bound(if error < 0.0 { root.next_up() } else { root })
}

/// Lower bound on `minuend - subtrahend` for nonnegative operands, zero when
/// the difference is not positive.
fn lower_nonnegative_difference(minuend: f64, subtrahend: f64) -> f64 {
    if minuend <= subtrahend {
        return 0.0;
    }
    let difference = minuend - subtrahend;
    let virtual_subtrahend = minuend - difference;
    let error = (minuend - (difference + virtual_subtrahend)) + (virtual_subtrahend - subtrahend);
    let lower = if error < 0.0 {
        difference.next_down()
    } else {
        difference
    };
    lower.max(0.0)
}

/// Upward sum of nonnegative values.
fn upper_sum(values: &[f64]) -> Result<f64, Audit2StageCertificateError> {
    values
        .iter()
        .try_fold(0.0, |sum, value| audit2_upper_add(sum, *value))
}

/// Upper bound on the Euclidean norm of a vector of nonnegative magnitudes.
fn l2_upper(magnitudes: &[f64]) -> Result<f64, Audit2StageCertificateError> {
    let mut sum = 0.0;
    for value in magnitudes {
        sum = audit2_upper_add(sum, audit2_upper_mul(*value, *value)?)?;
    }
    upper_sqrt(sum)
}

/// `gamma_{k} = k u / (1 - k u)` rounded upward, `u = 2^-53`.
fn gamma_upper(k: usize) -> Result<f64, Audit2StageCertificateError> {
    let unit_roundoff = f64::EPSILON / 2.0;
    let k = k as f64;
    let numerator = audit2_upper_mul(k, unit_roundoff)?;
    if numerator >= 1.0 {
        return Err(Audit2StageCertificateError::NonFiniteBound);
    }
    upper_div(numerator, lower_nonnegative_difference(1.0, numerator))
}

/// Componentwise upper bounds on `|b - A x|` for the rounded recursive
/// evaluation, `|r_i| <= |fl(r_i)| + gamma_{n+1} (|b_i| + sum_j |a_ij x_j|)
/// plus (n + 1) eta`, with `eta` the smallest subnormal (Higham, Accuracy and
/// Stability of Numerical Algorithms, sections 3.1 and 3.5).
fn residual_magnitude_upper(
    operator: &[Vec<f64>],
    solution: &[f64],
    rhs: &[f64],
) -> Result<Vec<f64>, Audit2StageCertificateError> {
    let dimension = solution.len();
    let gamma = gamma_upper(dimension + 1)?;
    let underflow = audit2_upper_mul((dimension + 1) as f64, f64::from_bits(1))?;
    operator
        .iter()
        .zip(rhs)
        .map(|(row, rhs_value)| {
            let mut product = 0.0;
            let mut magnitudes = Vec::with_capacity(row.len() + 1);
            magnitudes.push(rhs_value.abs());
            for (entry, value) in row.iter().zip(solution) {
                product += entry * value;
                magnitudes.push(audit2_upper_mul(entry.abs(), value.abs())?);
            }
            let rounded = rhs_value - product;
            if !rounded.is_finite() {
                return Err(Audit2StageCertificateError::NonFiniteBound);
            }
            let magnitude = upper_sum(&magnitudes)?;
            if magnitude == 0.0 {
                // Every term is an exact zero, so the evaluation is exact.
                return Ok(0.0);
            }
            let rounding = audit2_upper_mul(gamma, magnitude)?;
            audit2_upper_add(audit2_upper_add(rounded.abs(), rounding)?, underflow)
        })
        .collect()
}

/// Upper bound on the spectral norm, `sqrt(||M||_1 ||M||_inf)`, of a matrix
/// given by componentwise upper bounds on its magnitudes.
fn spectral_norm_upper(magnitudes: &[Vec<f64>]) -> Result<f64, Audit2StageCertificateError> {
    let dimension = magnitudes.len();
    let mut row_max: f64 = 0.0;
    let mut column_sums = vec![0.0; dimension];
    for row in magnitudes {
        row_max = row_max.max(upper_sum(row)?);
        for (sum, value) in column_sums.iter_mut().zip(row) {
            *sum = audit2_upper_add(*sum, *value)?;
        }
    }
    let column_max = column_sums.iter().copied().fold(0.0_f64, f64::max);
    upper_sqrt(audit2_upper_mul(row_max, column_max)?)
}

/// Verified upper bound on `||W^{-1}||_2` from an approximate inverse `V`:
/// with `delta >= ||I - V W||_2 < 1`, `||W^{-1}||_2 <= ||V||_2 / (1 - delta)`.
fn verified_inverse_norm_upper(
    operator: &[Vec<f64>],
    witness: &[Vec<f64>],
) -> Result<f64, Audit2StageCertificateError> {
    let dimension = operator.len();
    let mut defect = Vec::with_capacity(dimension);
    for (row_index, witness_row) in witness.iter().enumerate() {
        let mut defect_row = Vec::with_capacity(dimension);
        for column_index in 0..dimension {
            let column: Vec<f64> = operator.iter().map(|row| row[column_index]).collect();
            let identity = if row_index == column_index { 1.0 } else { 0.0 };
            let bound =
                residual_magnitude_upper(std::slice::from_ref(witness_row), &column, &[identity])?;
            defect_row.push(bound[0]);
        }
        defect.push(defect_row);
    }
    let delta = spectral_norm_upper(&defect)?;
    if delta >= 1.0 {
        return Err(Audit2StageCertificateError::KappaPremiseNotVerified);
    }
    let witness_magnitudes: Vec<Vec<f64>> = witness
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).collect())
        .collect();
    let witness_norm = spectral_norm_upper(&witness_magnitudes)?;
    upper_div(witness_norm, lower_nonnegative_difference(1.0, delta))
}

fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonicalize_json).collect()),
        Value::Object(values) => {
            let mut keys: Vec<_> = values.keys().collect();
            keys.sort_unstable();
            let mut ordered = Map::new();
            for key in keys {
                ordered.insert(key.clone(), canonicalize_json(&values[key]));
            }
            Value::Object(ordered)
        }
        _ => value.clone(),
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    result
}

fn verify_frozen_json(document: &FrozenJsonDocument) -> Result<(), Audit2StageCertificateError> {
    let actual = canonical_json_sha256(&document.document)?;
    if !is_lower_hex_digest(&document.canonical_sha256) || actual != document.canonical_sha256 {
        return Err(Audit2StageCertificateError::FrozenJsonDigestMismatch);
    }
    Ok(())
}

fn is_lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_provenance_policy(
    provenance: &StageCertificateProvenance,
) -> Result<(), Audit2StageCertificateError> {
    if provenance.restart == 0
        || provenance.max_arnoldi == 0
        || provenance.iteration_limit == 0
        || provenance.restart > provenance.max_arnoldi
        || provenance.max_arnoldi > provenance.iteration_limit
        || ![
            &provenance.coefficient_digest,
            &provenance.operator_identity,
            &provenance.preconditioner_identity,
            &provenance.rhs_digest,
            &provenance.decision_input_digest,
        ]
        .into_iter()
        .all(|digest| is_lower_hex_digest(digest))
    {
        return Err(Audit2StageCertificateError::InvalidField);
    }
    Ok(())
}

fn verify_trace(
    trace: &StageCertificateTrace,
    provenance: &StageCertificateProvenance,
) -> Result<(), Audit2StageCertificateError> {
    verify_frozen_json(&trace.frozen_trace)?;
    if trace.partial_failure.is_some() {
        return Err(Audit2StageCertificateError::PartialTraceCannotDecide);
    }
    if trace.stage_traces.is_empty() {
        return Err(Audit2StageCertificateError::IncompleteResidualHistory);
    }
    let mut work = StageCertificateWork::default();
    for (position, stage) in trace.stage_traces.iter().enumerate() {
        if stage.stage_index != position
            || !stage.complete
            || stage.work.arnoldi_iterations == 0
            || stage.work.arnoldi_iterations > u64::from(provenance.iteration_limit)
            || usize::try_from(stage.work.arnoldi_iterations)
                .ok()
                .and_then(|iterations| iterations.checked_add(1))
                != Some(stage.residual_history.len())
            || stage
                .residual_history
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(Audit2StageCertificateError::IncompleteResidualHistory);
        }
        // `max_arnoldi` caps the Arnoldi vectors of every completed row on
        // its own; the iteration limit and the per-cycle restart length do
        // not substitute for it (audit F-057). Equality is legal.
        if stage.work.arnoldi_iterations > u64::from(provenance.max_arnoldi) {
            return Err(Audit2StageCertificateError::MaxArnoldiExceeded);
        }
        work = work.checked_add(stage.work)?;
    }
    if work != trace.completed_work {
        return Err(Audit2StageCertificateError::InvalidWork);
    }
    Ok(())
}

fn verify_provenance(input: &StageCertificateInput) -> Result<(), Audit2StageCertificateError> {
    let coefficient_bits: Vec<u64> = input
        .coefficients
        .iter()
        .map(|value| value.to_bits())
        .collect();
    let operator_bits: Vec<u64> = input
        .operator
        .iter()
        .flatten()
        .map(|value| value.to_bits())
        .collect();
    let preconditioner_bits: Vec<u64> = input
        .preconditioner
        .iter()
        .flatten()
        .map(|value| value.to_bits())
        .collect();
    if audit2_stage_certificate_digest_f64_bits(&coefficient_bits)
        != input.provenance.coefficient_digest
        || audit2_stage_certificate_digest_f64_bits(&operator_bits)
            != input.provenance.operator_identity
        || audit2_stage_certificate_digest_f64_bits(&preconditioner_bits)
            != input.provenance.preconditioner_identity
        || audit2_stage_certificate_rhs_digest(&input.stages) != input.provenance.rhs_digest
        || audit2_stage_certificate_decision_input_digest(input)
            != input.provenance.decision_input_digest
    {
        return Err(Audit2StageCertificateError::ProvenanceMismatch);
    }
    Ok(())
}

fn validate_finite(values: &[f64]) -> Result<(), Audit2StageCertificateError> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(Audit2StageCertificateError::InvalidField);
    }
    Ok(())
}

fn validate_nonnegative(values: &[f64]) -> Result<(), Audit2StageCertificateError> {
    if values
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(Audit2StageCertificateError::InvalidField);
    }
    Ok(())
}

fn validate_matrix_finite(matrix: &[Vec<f64>]) -> Result<(), Audit2StageCertificateError> {
    for row in matrix {
        validate_finite(row)?;
    }
    Ok(())
}

fn is_square(matrix: &[Vec<f64>], dimension: usize) -> bool {
    matrix.len() == dimension && matrix.iter().all(|row| row.len() == dimension)
}

fn validate_strict_lower(matrix: &[Vec<f64>]) -> Result<(), Audit2StageCertificateError> {
    for (row_index, row) in matrix.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            if !value.is_finite() || *value < 0.0 || (column_index >= row_index && *value != 0.0) {
                return Err(Audit2StageCertificateError::InvalidField);
            }
        }
    }
    Ok(())
}

fn forward_stage_majorant(
    strict_lower: &[Vec<f64>],
    q: &[f64],
) -> Result<Vec<f64>, Audit2StageCertificateError> {
    let mut propagated = Vec::with_capacity(q.len());
    for (row_index, row) in strict_lower.iter().enumerate() {
        let mut value = q[row_index];
        for (column_index, entry) in row.iter().take(row_index).enumerate() {
            let product = audit2_upper_mul(*entry, propagated[column_index])?;
            value = audit2_upper_add(value, product)?;
        }
        propagated.push(value);
    }
    Ok(propagated)
}

fn nonnegative_dot(weights: &[f64], values: &[f64]) -> Result<f64, Audit2StageCertificateError> {
    weights
        .iter()
        .zip(values)
        .try_fold(0.0, |sum, (weight, value)| {
            let product = audit2_upper_mul(*weight, *value)?;
            audit2_upper_add(sum, product)
        })
}
