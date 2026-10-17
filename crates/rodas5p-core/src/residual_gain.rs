//! Current-operator residual gain witness for rodas5p-core (DAG node AS05).
//!
//! Encloses the forward error `u^* - x` of a linear solve `W u = b` in a frozen
//! diagonal metric, for the specific operator target it is bound to.
//!
//! Authority types ([`VerifiedCorrection`], [`LinearTargetId`],
//! [`ResidualGainWitness`], [`FrozenMetric`]) have private fields, no public
//! constructors other than the witness API, and deliberately do not implement
//! `Deserialize` to prevent spoofing.

#![allow(
    clippy::large_enum_variant,
    clippy::collapsible_if,
    clippy::type_complexity,
    clippy::needless_range_loop,
    clippy::neg_cmp_op_on_partial_ord
)]

use std::sync::Arc;

use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::{
    CoreError, CoreResult, DenseMatrix, LuFactorization,
    directed::{Interval, add_up, div_up, mul_down, mul_up, sqrt_down, sqrt_up, sub_down},
    hash::sha256_f64_slice_bytes,
    nonnormal_certificate::symmetric_part_upper,
};

/// Reasons why a witness refuses to certify an enclosure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WitnessRejection {
    TargetMismatch,
    OperatorMismatch,
    MetricMismatch,
    StaleEpoch,
    OffBlockCoupling,
    UndeclaredStructure,
    RouteNotImplemented,
    NegativeShift,
    NonpositiveDenominator,
    NonfiniteMatrix,
    InvalidScale,
    NonfiniteRhs,
    NonfiniteCandidate,
    ArithmeticOverflow,
    DeterminantContainsZero,
}

impl WitnessRejection {
    /// Canonical rejection name matching research re-audit conventions.
    pub fn name(self) -> &'static str {
        match self {
            Self::TargetMismatch => "target_mismatch",
            Self::OperatorMismatch => "operator_mismatch",
            Self::MetricMismatch => "metric_mismatch",
            Self::StaleEpoch => "stale_epoch",
            Self::OffBlockCoupling => "off_block_coupling",
            Self::UndeclaredStructure => "undeclared_structure",
            Self::RouteNotImplemented => "route_not_implemented",
            Self::NegativeShift => "negative_shift",
            Self::NonpositiveDenominator => "nonpositive_denominator",
            Self::NonfiniteMatrix => "nonfinite_matrix",
            Self::InvalidScale => "nonpositive_or_nonfinite_scale",
            Self::NonfiniteRhs => "nonfinite_rhs",
            Self::NonfiniteCandidate => "nonfinite_candidate",
            Self::ArithmeticOverflow => "nonfinite_intermediate_or_outward_endpoint",
            Self::DeterminantContainsZero => "determinant_interval_contains_zero",
        }
    }
}

/// Source of an unverified estimate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EstimateSource {
    ProjectedNu,
    DenseLuComparator,
    UndeclaredAssembly,
}

/// Unverified error estimate (plain data).
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct EstimatedCorrection {
    pub estimate: f64,
    pub source: EstimateSource,
}

/// Declared uncertainty in operator assembly or JVP.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssemblyUncertainty {
    Exact,
    EntrywiseAbs(f64),
    Undeclared,
}

/// Declared structural sparsity of the operator.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclaredStructure {
    Dense,
    IsolatedBlocks(Vec<(usize, usize)>),
}

/// Operator mathematical definition.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorKind {
    Represented {
        matrix: Vec<Vec<f64>>,
    },
    Shifted {
        jacobian: Vec<Vec<f64>>,
        h: f64,
        gamma: f64,
    },
}

/// Frozen diagonal metric `s` with all entries finite and strictly positive.
///
/// Fields are private to prevent untracked metric modification.
/// Fixed capacity (up to 8 dimensions) stack buffer ensures zero heap allocations
/// during hot-path transport.
///
/// ```compile_fail
/// fn require_deserialize<'de, T: serde::Deserialize<'de>>() {}
/// require_deserialize::<rodas5p_core::residual_gain::FrozenMetric>();
/// ```
/// ```compile_fail
/// let _ = rodas5p_core::residual_gain::FrozenMetric {
///     scales: [0.0; 8],
///     len: 0,
///     digest: [0; 32],
/// };
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrozenMetric {
    scales: [f64; 8],
    len: usize,
    digest: [u8; 32],
}

impl Serialize for FrozenMetric {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("FrozenMetric", 2)?;
        state.serialize_field("scales", &self.scales[..self.len])?;
        state.serialize_field("digest", &self.digest)?;
        state.end()
    }
}

impl FrozenMetric {
    pub fn new(scales: &[f64]) -> Result<Self, WitnessRejection> {
        if scales.is_empty()
            || scales.len() > 8
            || scales.iter().any(|&s| !s.is_finite() || s <= 0.0)
        {
            return Err(WitnessRejection::InvalidScale);
        }
        let mut buf = [0.0; 8];
        buf[..scales.len()].copy_from_slice(scales);
        let digest = sha256_f64_slice_bytes(scales);
        Ok(Self {
            scales: buf,
            len: scales.len(),
            digest,
        })
    }

    pub fn scales(&self) -> &[f64] {
        &self.scales[..self.len]
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
struct OperatorDescriptor {
    operator_kind: OperatorKind,
    operator_digest: [u8; 32],
    model_epoch: Option<u64>,
    assembly_uncertainty: AssemblyUncertainty,
    declared_structure: DeclaredStructure,
}

/// Authority identifier binding operator entries, parameter bits, frozen metric,
/// model epoch, declared assembly uncertainty, and declared structure.
///
/// ```compile_fail
/// fn require_deserialize<'de, T: serde::Deserialize<'de>>() {}
/// require_deserialize::<rodas5p_core::residual_gain::LinearTargetId>();
/// ```
/// ```compile_fail
/// let _ = rodas5p_core::residual_gain::LinearTargetId {
///     operator: todo!(),
///     metric: todo!(),
/// };
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct LinearTargetId {
    operator: Arc<OperatorDescriptor>,
    metric: FrozenMetric,
}

impl Serialize for LinearTargetId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("LinearTargetId", 6)?;
        state.serialize_field("operator_kind", &self.operator.operator_kind)?;
        state.serialize_field("operator_digest", &self.operator.operator_digest)?;
        state.serialize_field("metric", &self.metric)?;
        state.serialize_field("model_epoch", &self.operator.model_epoch)?;
        state.serialize_field("assembly_uncertainty", &self.operator.assembly_uncertainty)?;
        state.serialize_field("declared_structure", &self.operator.declared_structure)?;
        state.end()
    }
}

impl LinearTargetId {
    pub fn represented(
        matrix: &[Vec<f64>],
        metric: FrozenMetric,
        model_epoch: Option<u64>,
        assembly_uncertainty: AssemblyUncertainty,
        declared_structure: DeclaredStructure,
    ) -> Result<Self, WitnessRejection> {
        let n = metric.len();
        if matrix.len() != n || matrix.iter().any(|r| r.len() != n) {
            return Err(WitnessRejection::TargetMismatch);
        }
        if matrix.iter().flatten().any(|x| !x.is_finite()) {
            return Err(WitnessRejection::NonfiniteMatrix);
        }
        if let AssemblyUncertainty::EntrywiseAbs(delta) = assembly_uncertainty {
            if !delta.is_finite() {
                return Err(WitnessRejection::NonfiniteMatrix);
            }
            if delta < 0.0 {
                return Err(WitnessRejection::InvalidScale);
            }
        }
        let mut flat = Vec::with_capacity(n * n);
        for row in matrix {
            flat.extend_from_slice(row);
        }
        let operator_digest = sha256_f64_slice_bytes(&flat);
        Ok(Self {
            operator: Arc::new(OperatorDescriptor {
                operator_kind: OperatorKind::Represented {
                    matrix: matrix.to_vec(),
                },
                operator_digest,
                model_epoch,
                assembly_uncertainty,
                declared_structure,
            }),
            metric,
        })
    }

    pub fn shifted(
        jacobian: &[Vec<f64>],
        h: f64,
        gamma: f64,
        metric: FrozenMetric,
        model_epoch: Option<u64>,
        assembly_uncertainty: AssemblyUncertainty,
        declared_structure: DeclaredStructure,
    ) -> Result<Self, WitnessRejection> {
        let n = metric.len();
        if jacobian.len() != n || jacobian.iter().any(|r| r.len() != n) {
            return Err(WitnessRejection::TargetMismatch);
        }
        if jacobian.iter().flatten().any(|x| !x.is_finite()) || !h.is_finite() || !gamma.is_finite()
        {
            return Err(WitnessRejection::NonfiniteMatrix);
        }
        if let AssemblyUncertainty::EntrywiseAbs(delta) = assembly_uncertainty {
            if !delta.is_finite() {
                return Err(WitnessRejection::NonfiniteMatrix);
            }
            if delta < 0.0 {
                return Err(WitnessRejection::InvalidScale);
            }
        }
        let mut flat = Vec::with_capacity(n * n + 2);
        for row in jacobian {
            flat.extend_from_slice(row);
        }
        flat.push(h);
        flat.push(gamma);
        let operator_digest = sha256_f64_slice_bytes(&flat);
        Ok(Self {
            operator: Arc::new(OperatorDescriptor {
                operator_kind: OperatorKind::Shifted {
                    jacobian: jacobian.to_vec(),
                    h,
                    gamma,
                },
                operator_digest,
                model_epoch,
                assembly_uncertainty,
                declared_structure,
            }),
            metric,
        })
    }

    pub fn operator_kind(&self) -> &OperatorKind {
        &self.operator.operator_kind
    }

    pub fn operator_digest(&self) -> &[u8; 32] {
        &self.operator.operator_digest
    }

    pub fn metric(&self) -> &FrozenMetric {
        &self.metric
    }

    pub fn model_epoch(&self) -> Option<u64> {
        self.operator.model_epoch
    }

    pub fn assembly_uncertainty(&self) -> AssemblyUncertainty {
        self.operator.assembly_uncertainty
    }

    pub fn declared_structure(&self) -> &DeclaredStructure {
        &self.operator.declared_structure
    }

    pub fn dim(&self) -> usize {
        self.metric.len()
    }
}

/// Verified forward error enclosure of a linear solve in a frozen metric.
///
/// Authority struct with private fields and zero deserialization.
///
/// ```compile_fail
/// fn require_deserialize<'de, T: serde::Deserialize<'de>>() {}
/// require_deserialize::<rodas5p_core::residual_gain::VerifiedCorrection>();
/// ```
/// ```compile_fail
/// let _ = rodas5p_core::residual_gain::VerifiedCorrection {
///     target_id: todo!(),
///     residual_intervals: todo!(),
///     correction_intervals: todo!(),
///     weighted_correction_intervals: todo!(),
///     len: 0,
///     weighted_inf_upper: 0.0,
///     wrms_upper: 0.0,
///     scaled_residual_gain_inf_upper: None,
///     verification_flops: 0,
/// };
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct VerifiedCorrection {
    target_id: LinearTargetId,
    residual_intervals: [Interval; 8],
    correction_intervals: [Interval; 8],
    weighted_correction_intervals: [Interval; 8],
    len: usize,
    weighted_inf_upper: f64,
    wrms_upper: f64,
    scaled_residual_gain_inf_upper: Option<f64>,
    verification_flops: u64,
}

impl Serialize for VerifiedCorrection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("VerifiedCorrection", 8)?;
        state.serialize_field("target_id", &self.target_id)?;
        let residuals: Vec<[f64; 2]> = self.residual_intervals[..self.len]
            .iter()
            .map(|z| z.as_pair())
            .collect();
        state.serialize_field("residual_intervals", &residuals)?;
        let corrections: Vec<[f64; 2]> = self.correction_intervals[..self.len]
            .iter()
            .map(|z| z.as_pair())
            .collect();
        state.serialize_field("correction_intervals", &corrections)?;
        let weighted: Vec<[f64; 2]> = self.weighted_correction_intervals[..self.len]
            .iter()
            .map(|z| z.as_pair())
            .collect();
        state.serialize_field("weighted_correction_intervals", &weighted)?;
        state.serialize_field("weighted_inf_upper", &self.weighted_inf_upper)?;
        state.serialize_field("wrms_upper", &self.wrms_upper)?;
        state.serialize_field(
            "scaled_residual_gain_inf_upper",
            &self.scaled_residual_gain_inf_upper,
        )?;
        state.serialize_field("verification_flops", &self.verification_flops)?;
        state.end()
    }
}

impl VerifiedCorrection {
    pub fn target_id(&self) -> &LinearTargetId {
        &self.target_id
    }

    pub fn residual_intervals(&self) -> &[Interval] {
        &self.residual_intervals[..self.len]
    }

    pub fn correction_intervals(&self) -> &[Interval] {
        &self.correction_intervals[..self.len]
    }

    pub fn weighted_correction_intervals(&self) -> &[Interval] {
        &self.weighted_correction_intervals[..self.len]
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn weighted_inf_upper(&self) -> f64 {
        self.weighted_inf_upper
    }

    pub fn wrms_upper(&self) -> f64 {
        self.wrms_upper
    }

    pub fn scaled_residual_gain_inf_upper(&self) -> Option<f64> {
        self.scaled_residual_gain_inf_upper
    }

    pub fn verification_flops(&self) -> u64 {
        self.verification_flops
    }

    /// Transports this certificate to a new diagonal metric `new_metric`.
    ///
    /// Multiplies bounds by `kappa = max_k (s_old,k / s_new,k)` and
    /// `kappa_rev = max_k (s_new,k / s_old,k)` computed outward.
    pub fn transport_to(&self, new_metric: &FrozenMetric) -> Result<Self, WitnessRejection> {
        if new_metric.len() != self.len {
            return Err(WitnessRejection::MetricMismatch);
        }
        let old_scales = self.target_id.metric().scales();
        let new_scales = new_metric.scales();

        let mut flops = 0u64;
        let mut kappa = 0.0_f64;
        let mut kappa_rev = 0.0_f64;
        let mut new_weighted = [Interval { lo: 0.0, hi: 0.0 }; 8];
        let mut direct_inf = 0.0_f64;
        let mut sum_sq_up = 0.0_f64;

        for k in 0..self.len {
            let ratio = div_up(old_scales[k], new_scales[k])
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let ratio_rev = div_up(new_scales[k], old_scales[k])
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            flops += 2;
            kappa = kappa.max(ratio);
            kappa_rev = kappa_rev.max(ratio_rev);

            let s_new_int =
                Interval::point(new_scales[k]).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let comp = self.correction_intervals[k]
                .div(s_new_int)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            flops += 4;
            let comp_mag = comp.mag();
            direct_inf = direct_inf.max(comp_mag);
            new_weighted[k] = comp;

            let sq =
                mul_up(comp_mag, comp_mag).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            sum_sq_up = add_up(sum_sq_up, sq).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            flops += 2;
        }

        let sqrt_sum = sqrt_up(sum_sq_up).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
        let sqrt_n_down =
            sqrt_down(self.len as f64).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
        let direct_wrms =
            div_up(sqrt_sum, sqrt_n_down).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
        flops += 3;

        let transported_inf = mul_up(self.weighted_inf_upper, kappa)
            .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
        flops += 1;
        let transported_wrms =
            mul_up(self.wrms_upper, kappa).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
        flops += 1;

        let final_inf = transported_inf.min(direct_inf);
        let final_wrms = transported_wrms.min(direct_wrms);

        let transported_gain = match self.scaled_residual_gain_inf_upper {
            Some(g) => {
                let scaled_once =
                    mul_up(g, kappa).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
                let scaled_twice = mul_up(scaled_once, kappa_rev)
                    .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
                flops += 2;
                Some(scaled_twice)
            }
            None => None,
        };

        let new_target = LinearTargetId {
            operator: Arc::clone(&self.target_id.operator),
            metric: *new_metric,
        };

        Ok(Self {
            target_id: new_target,
            residual_intervals: self.residual_intervals,
            correction_intervals: self.correction_intervals,
            weighted_correction_intervals: new_weighted,
            len: self.len,
            weighted_inf_upper: final_inf,
            wrms_upper: final_wrms,
            scaled_residual_gain_inf_upper: transported_gain,
            verification_flops: self.verification_flops + flops,
        })
    }
}

/// Result of evaluating a candidate linear solve against a witness.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(tag = "evidence_kind", content = "details")]
pub enum GainEvidence {
    Verified(VerifiedCorrection),
    Estimated(EstimatedCorrection),
    Unavailable(WitnessRejection),
}

impl GainEvidence {
    pub fn is_verified(&self) -> bool {
        matches!(self, Self::Verified(_))
    }

    pub fn is_estimated(&self) -> bool {
        matches!(self, Self::Estimated(_))
    }

    pub fn is_unavailable(&self) -> bool {
        matches!(self, Self::Unavailable(_))
    }

    pub fn as_verified(&self) -> Option<&VerifiedCorrection> {
        match self {
            Self::Verified(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_estimated(&self) -> Option<&EstimatedCorrection> {
        match self {
            Self::Estimated(e) => Some(e),
            _ => None,
        }
    }

    pub fn as_unavailable(&self) -> Option<WitnessRejection> {
        match self {
            Self::Unavailable(r) => Some(*r),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
struct LognormPrecomputed {
    mu_up: f64,
    mu_source: &'static str,
    denom_down: f64,
    sqrt_n_down: f64,
    interval_j: Vec<Vec<Interval>>,
}

/// Residual gain witness bound to an owned target identity.
///
/// Authority struct with private fields and zero deserialization.
///
/// ```compile_fail
/// fn require_deserialize<'de, T: serde::Deserialize<'de>>() {}
/// require_deserialize::<rodas5p_core::residual_gain::ResidualGainWitness>();
/// ```
/// ```compile_fail
/// let _ = rodas5p_core::residual_gain::ResidualGainWitness {
///     target: todo!(),
///     setup_flops: 0,
///     inverse_2x2: None,
///     determinant_2x2: None,
///     scaled_residual_gain_inf_upper: None,
///     blocks_inverse_2x2: Vec::new(),
///     lognorm_precomputed: None,
/// };
/// ```
#[derive(Clone, Debug)]
pub struct ResidualGainWitness {
    target: LinearTargetId,
    setup_flops: u64,
    inverse_2x2: Option<[[Interval; 2]; 2]>,
    determinant_2x2: Option<Interval>,
    scaled_residual_gain_inf_upper: Option<f64>,
    blocks_inverse_2x2: Vec<Option<[[Interval; 2]; 2]>>,
    lognorm_precomputed: Option<LognormPrecomputed>,
}

impl Serialize for ResidualGainWitness {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ResidualGainWitness", 3)?;
        state.serialize_field("target", &self.target)?;
        state.serialize_field("setup_flops", &self.setup_flops)?;
        state.serialize_field(
            "scaled_residual_gain_inf_upper",
            &self.scaled_residual_gain_inf_upper,
        )?;
        state.end()
    }
}

impl ResidualGainWitness {
    /// Constructs a witness for the bound target.
    pub fn new(target: LinearTargetId) -> Result<Self, WitnessRejection> {
        let n = target.dim();
        if n > 8 {
            return Err(WitnessRejection::RouteNotImplemented);
        }

        if let AssemblyUncertainty::Undeclared = target.assembly_uncertainty() {
            return Ok(Self {
                target,
                setup_flops: 0,
                inverse_2x2: None,
                determinant_2x2: None,
                scaled_residual_gain_inf_upper: None,
                blocks_inverse_2x2: Vec::new(),
                lognorm_precomputed: None,
            });
        }

        let mut flops = 0u64;
        let mut inverse_2x2 = None;
        let mut determinant_2x2 = None;
        let mut scaled_residual_gain_inf_upper = None;
        let mut blocks_inverse_2x2 = Vec::new();
        let mut lognorm_precomputed = None;

        match target.declared_structure() {
            DeclaredStructure::Dense => {
                if n == 2 {
                    match target.operator_kind() {
                        OperatorKind::Represented { matrix } => {
                            let (inv, det, gain, f) = setup_represented_2x2(
                                matrix,
                                target.metric().scales(),
                                target.assembly_uncertainty(),
                            )?;
                            flops += f;
                            inverse_2x2 = Some(inv);
                            determinant_2x2 = Some(det);
                            scaled_residual_gain_inf_upper = Some(gain);
                        }
                        OperatorKind::Shifted { jacobian, h, gamma } => {
                            let (inv, det, f) = setup_shifted_2x2(
                                jacobian,
                                *h,
                                *gamma,
                                target.assembly_uncertainty(),
                            )?;
                            flops += f;
                            inverse_2x2 = Some(inv);
                            determinant_2x2 = Some(det);
                        }
                    }
                } else if let OperatorKind::Shifted { jacobian, h, gamma } = target.operator_kind()
                {
                    // Route (d): lognorm dense n <= 8
                    let (lognorm, f) = setup_lognorm(
                        jacobian,
                        *h,
                        *gamma,
                        target.metric().scales(),
                        target.assembly_uncertainty(),
                    )?;
                    flops += f;
                    lognorm_precomputed = Some(lognorm);
                } else {
                    return Err(WitnessRejection::UndeclaredStructure);
                }
            }
            DeclaredStructure::IsolatedBlocks(ranges) => {
                validate_block_partition(n, ranges)?;
                check_off_block_coupling(target.operator_kind(), ranges)?;

                for &(start, end) in ranges {
                    let len = end - start;
                    if len == 2 {
                        let inv = setup_block_2x2(
                            target.operator_kind(),
                            start,
                            target.assembly_uncertainty(),
                            &mut flops,
                        )?;
                        blocks_inverse_2x2.push(Some(inv));
                    } else {
                        blocks_inverse_2x2.push(None);
                    }
                }
            }
        }

        Ok(Self {
            target,
            setup_flops: flops,
            inverse_2x2,
            determinant_2x2,
            scaled_residual_gain_inf_upper,
            blocks_inverse_2x2,
            lognorm_precomputed,
        })
    }

    pub fn target(&self) -> &LinearTargetId {
        &self.target
    }

    pub fn setup_flops(&self) -> u64 {
        self.setup_flops
    }

    pub fn determinant_2x2(&self) -> Option<Interval> {
        self.determinant_2x2
    }

    pub fn inverse_2x2(&self) -> Option<[[Interval; 2]; 2]> {
        self.inverse_2x2
    }

    pub fn scaled_residual_gain_inf_upper(&self) -> Option<f64> {
        self.scaled_residual_gain_inf_upper
    }

    pub fn lognorm_mu_up(&self) -> Option<f64> {
        self.lognorm_precomputed.as_ref().map(|p| p.mu_up)
    }

    pub fn lognorm_mu_source(&self) -> Option<&'static str> {
        self.lognorm_precomputed.as_ref().map(|p| p.mu_source)
    }

    pub fn lognorm_denom_down(&self) -> Option<f64> {
        self.lognorm_precomputed.as_ref().map(|p| p.denom_down)
    }

    /// Certifies the forward error of `candidate` for `W u = rhs` against the bound target.
    pub fn certify(&self, target: &LinearTargetId, rhs: &[f64], candidate: &[f64]) -> GainEvidence {
        // 1. Target authority checks
        if std::mem::discriminant(target.operator_kind())
            != std::mem::discriminant(self.target.operator_kind())
        {
            return GainEvidence::Unavailable(WitnessRejection::TargetMismatch);
        }
        if target.operator_digest() != self.target.operator_digest() {
            return GainEvidence::Unavailable(WitnessRejection::OperatorMismatch);
        }
        if target.metric().digest() != self.target.metric().digest() {
            return GainEvidence::Unavailable(WitnessRejection::MetricMismatch);
        }
        if target.model_epoch() != self.target.model_epoch() {
            return GainEvidence::Unavailable(WitnessRejection::StaleEpoch);
        }

        let n = self.target.dim();
        if rhs.len() != n || candidate.len() != n {
            return GainEvidence::Unavailable(WitnessRejection::TargetMismatch);
        }
        if rhs.iter().any(|x| !x.is_finite()) {
            return GainEvidence::Unavailable(WitnessRejection::NonfiniteRhs);
        }
        if candidate.iter().any(|x| !x.is_finite()) {
            return GainEvidence::Unavailable(WitnessRejection::NonfiniteCandidate);
        }

        // 2. Assembly check
        if let AssemblyUncertainty::Undeclared = target.assembly_uncertainty() {
            let est = self.estimate_lu_or_inf(rhs, candidate);
            return GainEvidence::Estimated(EstimatedCorrection {
                estimate: est,
                source: EstimateSource::UndeclaredAssembly,
            });
        }

        // 3. Hot-path certification
        let mut flops = 0u64;
        let result = match self.target.declared_structure() {
            DeclaredStructure::Dense => {
                if n == 2 {
                    if let Some(inv) = self.inverse_2x2 {
                        certify_dense_2x2(
                            &self.target,
                            inv,
                            rhs,
                            candidate,
                            self.scaled_residual_gain_inf_upper,
                            &mut flops,
                        )
                    } else {
                        Err(WitnessRejection::RouteNotImplemented)
                    }
                } else if let Some(ref lognorm) = self.lognorm_precomputed {
                    certify_lognorm(&self.target, lognorm, rhs, candidate, &mut flops)
                } else {
                    Err(WitnessRejection::UndeclaredStructure)
                }
            }
            DeclaredStructure::IsolatedBlocks(ranges) => certify_isolated_blocks(
                &self.target,
                ranges,
                &self.blocks_inverse_2x2,
                rhs,
                candidate,
                &mut flops,
            ),
        };

        match result {
            Ok(cert) => GainEvidence::Verified(cert),
            Err(rej) => GainEvidence::Unavailable(rej),
        }
    }

    fn estimate_lu_or_inf(&self, rhs: &[f64], candidate: &[f64]) -> f64 {
        let n = self.target.dim();
        let scales = self.target.metric().scales();
        match self.target.operator_kind() {
            OperatorKind::Represented { matrix } => {
                if let Ok(m) = DenseMatrix::new(n, n, matrix.iter().flatten().copied().collect()) {
                    if let Ok(lu) = LuFactorization::new(&m) {
                        let res: Vec<f64> = (0..n)
                            .map(|i| {
                                let dot: f64 = (0..n).map(|j| matrix[i][j] * candidate[j]).sum();
                                rhs[i] - dot
                            })
                            .collect();
                        if let Ok(sol) = lu.solve(&res) {
                            return sol
                                .iter()
                                .zip(scales.iter())
                                .map(|(&c, &s): (&f64, &f64)| (c / s).abs())
                                .fold(0.0_f64, f64::max);
                        }
                    }
                }
            }
            OperatorKind::Shifted { jacobian, h, gamma } => {
                let a = h * gamma;
                let mut data = vec![0.0; n * n];
                for i in 0..n {
                    for j in 0..n {
                        let val = if i == j {
                            1.0 - a * jacobian[i][j]
                        } else {
                            -a * jacobian[i][j]
                        };
                        data[i * n + j] = val;
                    }
                }
                if let Ok(m) = DenseMatrix::new(n, n, data) {
                    if let Ok(lu) = LuFactorization::new(&m) {
                        let res: Vec<f64> = (0..n)
                            .map(|i| {
                                let mut dot = 0.0;
                                for j in 0..n {
                                    let w_ij = if i == j {
                                        1.0 - a * jacobian[i][j]
                                    } else {
                                        -a * jacobian[i][j]
                                    };
                                    dot += w_ij * candidate[j];
                                }
                                rhs[i] - dot
                            })
                            .collect();
                        if let Ok(sol) = lu.solve(&res) {
                            return sol
                                .iter()
                                .zip(scales.iter())
                                .map(|(&c, &s): (&f64, &f64)| (c / s).abs())
                                .fold(0.0_f64, f64::max);
                        }
                    }
                }
            }
        }
        0.0
    }
}

// -----------------------------------------------------------------------------
// Helper Routines
// -----------------------------------------------------------------------------

fn setup_represented_2x2(
    matrix: &[Vec<f64>],
    scales: &[f64],
    uncertainty: AssemblyUncertainty,
) -> Result<([[Interval; 2]; 2], Interval, f64, u64), WitnessRejection> {
    let mut flops = 0u64;
    let delta = match uncertainty {
        AssemblyUncertainty::Exact => 0.0,
        AssemblyUncertainty::EntrywiseAbs(d) => d,
        AssemblyUncertainty::Undeclared => return Err(WitnessRejection::TargetMismatch),
    };

    let make_int = |v: f64| -> Result<Interval, WitnessRejection> {
        if delta == 0.0 {
            Interval::point(v).map_err(|_| WitnessRejection::ArithmeticOverflow)
        } else {
            Interval::new(
                sub_down(v, delta).map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                add_up(v, delta).map_err(|_| WitnessRejection::ArithmeticOverflow)?,
            )
            .map_err(|_| WitnessRejection::ArithmeticOverflow)
        }
    };

    let a = make_int(matrix[0][0])?;
    let b = make_int(matrix[0][1])?;
    let c = make_int(matrix[1][0])?;
    let d = make_int(matrix[1][1])?;

    let ad = a.mul(d).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    let bc = b.mul(c).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    let det = ad
        .sub(bc)
        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    flops += 3;

    if det.contains_zero() {
        return Err(WitnessRejection::DeterminantContainsZero);
    }

    let inv = [
        [
            d.div(det)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
            (-b).div(det)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
        ],
        [
            (-c).div(det)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
            a.div(det)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
        ],
    ];
    flops += 4;

    let mut gain = 0.0_f64;
    for i in 0..2 {
        let mut row_sum = 0.0_f64;
        for j in 0..2 {
            let term = mul_up(
                inv[i][j].mag(),
                div_up(scales[j], scales[i]).map_err(|_| WitnessRejection::ArithmeticOverflow)?,
            )
            .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            row_sum = add_up(row_sum, term).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            flops += 3;
        }
        gain = gain.max(row_sum);
    }

    Ok((inv, det, gain, flops))
}

fn setup_shifted_2x2(
    jacobian: &[Vec<f64>],
    h: f64,
    gamma: f64,
    uncertainty: AssemblyUncertainty,
) -> Result<([[Interval; 2]; 2], Interval, u64), WitnessRejection> {
    let mut flops = 0u64;
    let delta = match uncertainty {
        AssemblyUncertainty::Exact => 0.0,
        AssemblyUncertainty::EntrywiseAbs(d) => d,
        AssemblyUncertainty::Undeclared => return Err(WitnessRejection::TargetMismatch),
    };

    let h_int = Interval::point(h).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    let g_int = Interval::point(gamma).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    let a_int = h_int
        .mul(g_int)
        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    flops += 1;

    let mut w = [[Interval { lo: 0.0, hi: 0.0 }; 2]; 2];
    for i in 0..2 {
        for j in 0..2 {
            let j_val = jacobian[i][j];
            let j_int = if delta == 0.0 {
                Interval::point(j_val).map_err(|_| WitnessRejection::ArithmeticOverflow)?
            } else {
                Interval::new(
                    sub_down(j_val, delta).map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                    add_up(j_val, delta).map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                )
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?
            };
            let aj = a_int
                .mul(j_int)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            flops += 1;
            if i == j {
                w[i][j] = Interval::point(1.0)
                    .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                    .sub(aj)
                    .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
                flops += 1;
            } else {
                w[i][j] = -aj;
            }
        }
    }

    let ad = w[0][0]
        .mul(w[1][1])
        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    let bc = w[0][1]
        .mul(w[1][0])
        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    let det = ad
        .sub(bc)
        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    flops += 3;

    if det.contains_zero() {
        return Err(WitnessRejection::DeterminantContainsZero);
    }

    let inv = [
        [
            w[1][1]
                .div(det)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
            (-w[0][1])
                .div(det)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
        ],
        [
            (-w[1][0])
                .div(det)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
            w[0][0]
                .div(det)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
        ],
    ];
    flops += 4;

    Ok((inv, det, flops))
}

fn setup_block_2x2(
    op: &OperatorKind,
    start: usize,
    uncertainty: AssemblyUncertainty,
    flops: &mut u64,
) -> Result<[[Interval; 2]; 2], WitnessRejection> {
    match op {
        OperatorKind::Represented { matrix } => {
            let delta = match uncertainty {
                AssemblyUncertainty::Exact => 0.0,
                AssemblyUncertainty::EntrywiseAbs(d) => d,
                AssemblyUncertainty::Undeclared => return Err(WitnessRejection::TargetMismatch),
            };
            let make_int = |v: f64| -> Result<Interval, WitnessRejection> {
                if delta == 0.0 {
                    Interval::point(v).map_err(|_| WitnessRejection::ArithmeticOverflow)
                } else {
                    Interval::new(
                        sub_down(v, delta).map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                        add_up(v, delta).map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                    )
                    .map_err(|_| WitnessRejection::ArithmeticOverflow)
                }
            };
            let a = make_int(matrix[start][start])?;
            let b = make_int(matrix[start][start + 1])?;
            let c = make_int(matrix[start + 1][start])?;
            let d = make_int(matrix[start + 1][start + 1])?;

            let ad = a.mul(d).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let bc = b.mul(c).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let det = ad
                .sub(bc)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            *flops += 3;

            if det.contains_zero() {
                return Err(WitnessRejection::DeterminantContainsZero);
            }

            let inv = [
                [
                    d.div(det)
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                    (-b).div(det)
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                ],
                [
                    (-c).div(det)
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                    a.div(det)
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                ],
            ];
            *flops += 4;
            Ok(inv)
        }
        OperatorKind::Shifted { jacobian, h, gamma } => {
            let sub_j = vec![
                vec![jacobian[start][start], jacobian[start][start + 1]],
                vec![jacobian[start + 1][start], jacobian[start + 1][start + 1]],
            ];
            let (inv, _det, f) = setup_shifted_2x2(&sub_j, *h, *gamma, uncertainty)?;
            *flops += f;
            Ok(inv)
        }
    }
}

fn setup_lognorm(
    jacobian: &[Vec<f64>],
    h: f64,
    gamma: f64,
    scales: &[f64],
    uncertainty: AssemblyUncertainty,
) -> Result<(LognormPrecomputed, u64), WitnessRejection> {
    let mut flops = 0u64;
    let n = jacobian.len();
    if !(h.is_finite() && gamma.is_finite()) {
        return Err(WitnessRejection::NonfiniteMatrix);
    }
    let h_int = Interval::point(h).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    let g_int = Interval::point(gamma).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    let a_int = h_int
        .mul(g_int)
        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    flops += 1;

    if a_int.lo < 0.0 || h < 0.0 || gamma < 0.0 {
        return Err(WitnessRejection::NegativeShift);
    }

    let delta = match uncertainty {
        AssemblyUncertainty::Exact => 0.0,
        AssemblyUncertainty::EntrywiseAbs(d) => d,
        AssemblyUncertainty::Undeclared => return Err(WitnessRejection::TargetMismatch),
    };

    let mut interval_j = vec![vec![Interval { lo: 0.0, hi: 0.0 }; n]; n];
    for i in 0..n {
        for j in 0..n {
            let val = jacobian[i][j];
            let int_val = if delta == 0.0 {
                Interval::point(val).map_err(|_| WitnessRejection::ArithmeticOverflow)?
            } else {
                Interval::new(
                    sub_down(val, delta).map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                    add_up(val, delta).map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                )
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?
            };
            interval_j[i][j] = int_val;
        }
    }

    // Form B = D J D^-1 with D = diag(1/s)
    let mut b = vec![vec![Interval { lo: 0.0, hi: 0.0 }; n]; n];
    for i in 0..n {
        let si_inv = Interval::point(1.0)
            .map_err(|_| WitnessRejection::ArithmeticOverflow)?
            .div(Interval::point(scales[i]).map_err(|_| WitnessRejection::ArithmeticOverflow)?)
            .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
        for j in 0..n {
            let sj =
                Interval::point(scales[j]).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let term = si_inv
                .mul(interval_j[i][j])
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            b[i][j] = term
                .mul(sj)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            flops += 3;
        }
    }

    let (mu_up, mu_source) =
        symmetric_part_upper(&b).map_err(|_| WitnessRejection::ArithmeticOverflow)?;

    // Denominator = 1 - a * mu_up outward
    let term = if mu_up >= 0.0 {
        mul_up(a_int.hi, mu_up).map_err(|_| WitnessRejection::ArithmeticOverflow)?
    } else {
        mul_down(a_int.lo, mu_up).map_err(|_| WitnessRejection::ArithmeticOverflow)?
    };
    flops += 1;
    let denom_down = sub_down(1.0, term).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    flops += 1;

    if !(denom_down > 0.0) {
        return Err(WitnessRejection::NonpositiveDenominator);
    }

    let sqrt_n_down = sqrt_down(n as f64).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    flops += 1;

    Ok((
        LognormPrecomputed {
            mu_up,
            mu_source,
            denom_down,
            sqrt_n_down,
            interval_j,
        },
        flops,
    ))
}

fn validate_block_partition(n: usize, ranges: &[(usize, usize)]) -> Result<(), WitnessRejection> {
    if ranges.is_empty() {
        return Err(WitnessRejection::UndeclaredStructure);
    }
    let mut current = 0;
    for &(start, end) in ranges {
        if start != current || end <= start || end - start > 2 {
            return Err(WitnessRejection::UndeclaredStructure);
        }
        current = end;
    }
    if current != n {
        return Err(WitnessRejection::UndeclaredStructure);
    }
    Ok(())
}

fn check_off_block_coupling(
    op: &OperatorKind,
    ranges: &[(usize, usize)],
) -> Result<(), WitnessRejection> {
    let n = match op {
        OperatorKind::Represented { matrix } => matrix.len(),
        OperatorKind::Shifted { jacobian, .. } => jacobian.len(),
    };
    for i in 0..n {
        let block_i = ranges.iter().position(|&(s, e)| i >= s && i < e).unwrap();
        for j in 0..n {
            let block_j = ranges.iter().position(|&(s, e)| j >= s && j < e).unwrap();
            if block_i != block_j {
                let entry = match op {
                    OperatorKind::Represented { matrix } => matrix[i][j],
                    OperatorKind::Shifted { jacobian, .. } => jacobian[i][j],
                };
                if entry != 0.0 {
                    return Err(WitnessRejection::OffBlockCoupling);
                }
            }
        }
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Certification Kernels (Zero-Allocation Hot Path)
// -----------------------------------------------------------------------------

fn certify_dense_2x2(
    target: &LinearTargetId,
    inv: [[Interval; 2]; 2],
    rhs: &[f64],
    candidate: &[f64],
    gain_bound: Option<f64>,
    flops: &mut u64,
) -> Result<VerifiedCorrection, WitnessRejection> {
    let mut res = [Interval { lo: 0.0, hi: 0.0 }; 8];
    let mut corr = [Interval { lo: 0.0, hi: 0.0 }; 8];
    let mut weighted = [Interval { lo: 0.0, hi: 0.0 }; 8];
    let scales = target.metric().scales();

    let b0 = Interval::point(rhs[0]).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    let b1 = Interval::point(rhs[1]).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    let x0 = Interval::point(candidate[0]).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    let x1 = Interval::point(candidate[1]).map_err(|_| WitnessRejection::ArithmeticOverflow)?;

    match target.operator_kind() {
        OperatorKind::Represented { matrix } => {
            let delta = match target.assembly_uncertainty() {
                AssemblyUncertainty::Exact => 0.0,
                AssemblyUncertainty::EntrywiseAbs(d) => d,
                AssemblyUncertainty::Undeclared => return Err(WitnessRejection::TargetMismatch),
            };
            let make_int = |v: f64| -> Result<Interval, WitnessRejection> {
                if delta == 0.0 {
                    Interval::point(v).map_err(|_| WitnessRejection::ArithmeticOverflow)
                } else {
                    Interval::new(
                        sub_down(v, delta).map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                        add_up(v, delta).map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                    )
                    .map_err(|_| WitnessRejection::ArithmeticOverflow)
                }
            };
            let w00 = make_int(matrix[0][0])?;
            let w01 = make_int(matrix[0][1])?;
            let w10 = make_int(matrix[1][0])?;
            let w11 = make_int(matrix[1][1])?;

            let dot0 = w00
                .mul(x0)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                .add(
                    w01.mul(x1)
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                )
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let dot1 = w10
                .mul(x0)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                .add(
                    w11.mul(x1)
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                )
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            *flops += 6;

            res[0] = b0
                .sub(dot0)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            res[1] = b1
                .sub(dot1)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            *flops += 2;
        }
        OperatorKind::Shifted { jacobian, h, gamma } => {
            let delta = match target.assembly_uncertainty() {
                AssemblyUncertainty::Exact => 0.0,
                AssemblyUncertainty::EntrywiseAbs(d) => d,
                AssemblyUncertainty::Undeclared => return Err(WitnessRejection::TargetMismatch),
            };
            let a_int = Interval::point(*h)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                .mul(Interval::point(*gamma).map_err(|_| WitnessRejection::ArithmeticOverflow)?)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            *flops += 1;

            let mut w = [[Interval { lo: 0.0, hi: 0.0 }; 2]; 2];
            for i in 0..2 {
                for j in 0..2 {
                    let val = jacobian[i][j];
                    let j_int = if delta == 0.0 {
                        Interval::point(val).map_err(|_| WitnessRejection::ArithmeticOverflow)?
                    } else {
                        Interval::new(
                            sub_down(val, delta)
                                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                            add_up(val, delta).map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                        )
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                    };
                    let aj = a_int
                        .mul(j_int)
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
                    *flops += 1;
                    if i == j {
                        w[i][j] = Interval::point(1.0)
                            .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                            .sub(aj)
                            .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
                        *flops += 1;
                    } else {
                        w[i][j] = -aj;
                    }
                }
            }

            let dot0 = w[0][0]
                .mul(x0)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                .add(
                    w[0][1]
                        .mul(x1)
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                )
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let dot1 = w[1][0]
                .mul(x0)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                .add(
                    w[1][1]
                        .mul(x1)
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                )
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            *flops += 6;

            res[0] = b0
                .sub(dot0)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            res[1] = b1
                .sub(dot1)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            *flops += 2;
        }
    }

    corr[0] = inv[0][0]
        .mul(res[0])
        .map_err(|_| WitnessRejection::ArithmeticOverflow)?
        .add(
            inv[0][1]
                .mul(res[1])
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
        )
        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    corr[1] = inv[1][0]
        .mul(res[0])
        .map_err(|_| WitnessRejection::ArithmeticOverflow)?
        .add(
            inv[1][1]
                .mul(res[1])
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
        )
        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    *flops += 6;

    let s0 = Interval::point(scales[0]).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    let s1 = Interval::point(scales[1]).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    weighted[0] = corr[0]
        .div(s0)
        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    weighted[1] = corr[1]
        .div(s1)
        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    *flops += 2;

    let inf_upper = weighted[0].mag().max(weighted[1].mag());
    let wrms_upper = inf_upper;

    Ok(VerifiedCorrection {
        target_id: target.clone(),
        residual_intervals: res,
        correction_intervals: corr,
        weighted_correction_intervals: weighted,
        len: 2,
        weighted_inf_upper: inf_upper,
        wrms_upper,
        scaled_residual_gain_inf_upper: gain_bound,
        verification_flops: *flops,
    })
}

fn certify_isolated_blocks(
    target: &LinearTargetId,
    ranges: &[(usize, usize)],
    blocks_inv: &[Option<[[Interval; 2]; 2]>],
    rhs: &[f64],
    candidate: &[f64],
    flops: &mut u64,
) -> Result<VerifiedCorrection, WitnessRejection> {
    let n = target.dim();
    let scales = target.metric().scales();
    let mut res = [Interval { lo: 0.0, hi: 0.0 }; 8];
    let mut corr = [Interval { lo: 0.0, hi: 0.0 }; 8];
    let mut weighted = [Interval { lo: 0.0, hi: 0.0 }; 8];
    let mut inf_upper = 0.0_f64;

    for (b_idx, &(start, end)) in ranges.iter().enumerate() {
        let len = end - start;
        if len == 1 {
            let i = start;
            let bi = Interval::point(rhs[i]).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let xi =
                Interval::point(candidate[i]).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let w_ii = match target.operator_kind() {
                OperatorKind::Represented { matrix } => {
                    let delta = match target.assembly_uncertainty() {
                        AssemblyUncertainty::Exact => 0.0,
                        AssemblyUncertainty::EntrywiseAbs(d) => d,
                        AssemblyUncertainty::Undeclared => {
                            return Err(WitnessRejection::TargetMismatch);
                        }
                    };
                    if delta == 0.0 {
                        Interval::point(matrix[i][i])
                            .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                    } else {
                        Interval::new(
                            sub_down(matrix[i][i], delta)
                                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                            add_up(matrix[i][i], delta)
                                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                        )
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                    }
                }
                OperatorKind::Shifted { jacobian, h, gamma } => {
                    let a = Interval::point(*h)
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                        .mul(
                            Interval::point(*gamma)
                                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                        )
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
                    let j_ii = Interval::point(jacobian[i][i])
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
                    *flops += 2;
                    Interval::point(1.0)
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                        .sub(
                            a.mul(j_ii)
                                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                        )
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                }
            };
            if w_ii.contains_zero() {
                return Err(WitnessRejection::DeterminantContainsZero);
            }
            let dot = w_ii
                .mul(xi)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let r_i = bi
                .sub(dot)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let c_i = r_i
                .div(w_ii)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let si =
                Interval::point(scales[i]).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let wc_i = c_i
                .div(si)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            *flops += 4;

            res[i] = r_i;
            corr[i] = c_i;
            weighted[i] = wc_i;
            inf_upper = inf_upper.max(wc_i.mag());
        } else if len == 2 {
            let inv = blocks_inv[b_idx].ok_or(WitnessRejection::RouteNotImplemented)?;
            let b0 =
                Interval::point(rhs[start]).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let b1 = Interval::point(rhs[start + 1])
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let x0 = Interval::point(candidate[start])
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let x1 = Interval::point(candidate[start + 1])
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;

            let (w00, w01, w10, w11) = match target.operator_kind() {
                OperatorKind::Represented { matrix } => {
                    let delta = match target.assembly_uncertainty() {
                        AssemblyUncertainty::Exact => 0.0,
                        AssemblyUncertainty::EntrywiseAbs(d) => d,
                        AssemblyUncertainty::Undeclared => {
                            return Err(WitnessRejection::TargetMismatch);
                        }
                    };
                    let make_int = |v: f64| -> Result<Interval, WitnessRejection> {
                        if delta == 0.0 {
                            Interval::point(v).map_err(|_| WitnessRejection::ArithmeticOverflow)
                        } else {
                            Interval::new(
                                sub_down(v, delta)
                                    .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                                add_up(v, delta)
                                    .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                            )
                            .map_err(|_| WitnessRejection::ArithmeticOverflow)
                        }
                    };
                    (
                        make_int(matrix[start][start])?,
                        make_int(matrix[start][start + 1])?,
                        make_int(matrix[start + 1][start])?,
                        make_int(matrix[start + 1][start + 1])?,
                    )
                }
                OperatorKind::Shifted { jacobian, h, gamma } => {
                    let a = Interval::point(*h)
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                        .mul(
                            Interval::point(*gamma)
                                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                        )
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
                    *flops += 1;
                    let one =
                        Interval::point(1.0).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
                    let j00 = Interval::point(jacobian[start][start])
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
                    let j01 = Interval::point(jacobian[start][start + 1])
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
                    let j10 = Interval::point(jacobian[start + 1][start])
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
                    let j11 = Interval::point(jacobian[start + 1][start + 1])
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
                    *flops += 6;
                    (
                        one.sub(
                            a.mul(j00)
                                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                        )
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                        -a.mul(j01)
                            .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                        -a.mul(j10)
                            .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                        one.sub(
                            a.mul(j11)
                                .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                        )
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                    )
                }
            };

            let dot0 = w00
                .mul(x0)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                .add(
                    w01.mul(x1)
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                )
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let dot1 = w10
                .mul(x0)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                .add(
                    w11.mul(x1)
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                )
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            *flops += 6;

            let r0 = b0
                .sub(dot0)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let r1 = b1
                .sub(dot1)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            *flops += 2;

            let c0 = inv[0][0]
                .mul(r0)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                .add(
                    inv[0][1]
                        .mul(r1)
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                )
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let c1 = inv[1][0]
                .mul(r0)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                .add(
                    inv[1][1]
                        .mul(r1)
                        .map_err(|_| WitnessRejection::ArithmeticOverflow)?,
                )
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            *flops += 6;

            let s0 =
                Interval::point(scales[start]).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let s1 = Interval::point(scales[start + 1])
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let wc0 = c0
                .div(s0)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let wc1 = c1
                .div(s1)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            *flops += 2;

            res[start] = r0;
            res[start + 1] = r1;
            corr[start] = c0;
            corr[start + 1] = c1;
            weighted[start] = wc0;
            weighted[start + 1] = wc1;
            inf_upper = inf_upper.max(wc0.mag()).max(wc1.mag());
        }
    }

    let wrms_upper = inf_upper;
    Ok(VerifiedCorrection {
        target_id: target.clone(),
        residual_intervals: res,
        correction_intervals: corr,
        weighted_correction_intervals: weighted,
        len: n,
        weighted_inf_upper: inf_upper,
        wrms_upper,
        scaled_residual_gain_inf_upper: None,
        verification_flops: *flops,
    })
}

fn certify_lognorm(
    target: &LinearTargetId,
    lognorm: &LognormPrecomputed,
    rhs: &[f64],
    candidate: &[f64],
    flops: &mut u64,
) -> Result<VerifiedCorrection, WitnessRejection> {
    let n = target.dim();
    let scales = target.metric().scales();
    let mut res = [Interval { lo: 0.0, hi: 0.0 }; 8];
    let mut corr = [Interval { lo: 0.0, hi: 0.0 }; 8];
    let mut weighted = [Interval { lo: 0.0, hi: 0.0 }; 8];

    let (h, gamma) = match target.operator_kind() {
        OperatorKind::Shifted { h, gamma, .. } => (*h, *gamma),
        _ => return Err(WitnessRejection::TargetMismatch),
    };

    let a_int = Interval::point(h)
        .map_err(|_| WitnessRejection::ArithmeticOverflow)?
        .mul(Interval::point(gamma).map_err(|_| WitnessRejection::ArithmeticOverflow)?)
        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    *flops += 1;

    let mut dr_sq_up = 0.0_f64;

    for i in 0..n {
        let bi = Interval::point(rhs[i]).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
        let mut dot = Interval::point(0.0).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
        for j in 0..n {
            let xj =
                Interval::point(candidate[j]).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            let j_ij = lognorm.interval_j[i][j];
            let aj = a_int
                .mul(j_ij)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            *flops += 1;
            let w_ij = if i == j {
                Interval::point(1.0)
                    .map_err(|_| WitnessRejection::ArithmeticOverflow)?
                    .sub(aj)
                    .map_err(|_| WitnessRejection::ArithmeticOverflow)?
            } else {
                -aj
            };
            if i == j {
                *flops += 1;
            }
            let term = w_ij
                .mul(xj)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            dot = dot
                .add(term)
                .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
            *flops += 2;
        }
        let ri = bi
            .sub(dot)
            .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
        *flops += 1;
        res[i] = ri;

        let si = Interval::point(scales[i]).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
        let dri = ri
            .div(si)
            .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
        *flops += 1;

        let dri_mag = dri.mag();
        let dri_sq = mul_up(dri_mag, dri_mag).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
        dr_sq_up = add_up(dr_sq_up, dri_sq).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
        *flops += 2;
    }

    let norm_dr_up = sqrt_up(dr_sq_up).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    *flops += 1;

    // wrms_upper <= ||D r||_2 / (sqrt(n) * (1 - a mu_up))
    let denom_prod = mul_down(lognorm.sqrt_n_down, lognorm.denom_down)
        .map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    *flops += 1;
    let wrms_upper =
        div_up(norm_dr_up, denom_prod).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    *flops += 1;

    // Component bound: ||u* - x||_WRMS <= wrms_upper
    // => |(u* - x)_i / s_i| <= sqrt(n) * wrms_upper
    let sqrt_n_up = sqrt_up(n as f64).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    let radius = mul_up(sqrt_n_up, wrms_upper).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
    *flops += 2;

    for i in 0..n {
        let si = scales[i];
        let bound = mul_up(radius, si).map_err(|_| WitnessRejection::ArithmeticOverflow)?;
        *flops += 1;
        corr[i] = Interval {
            lo: -bound,
            hi: bound,
        };
        weighted[i] = Interval {
            lo: -radius,
            hi: radius,
        };
    }

    Ok(VerifiedCorrection {
        target_id: target.clone(),
        residual_intervals: res,
        correction_intervals: corr,
        weighted_correction_intervals: weighted,
        len: n,
        weighted_inf_upper: radius,
        wrms_upper,
        scaled_residual_gain_inf_upper: None,
        verification_flops: *flops,
    })
}

// -----------------------------------------------------------------------------
// Public Comparators and Estimators
// -----------------------------------------------------------------------------

/// Dense LU partial-pivot comparator (PREREGISTRATION Case E).
///
/// Computes `||c / s||_inf` as an [`EstimatedCorrection`].
pub fn dense_lu_comparator(
    matrix: &[Vec<f64>],
    rhs: &[f64],
    candidate: &[f64],
    scales: &[f64],
) -> CoreResult<EstimatedCorrection> {
    let n = scales.len();
    if matrix.len() != n
        || matrix.iter().any(|r| r.len() != n)
        || rhs.len() != n
        || candidate.len() != n
    {
        return Err(CoreError::Dimension(
            "dense_lu_comparator dimension mismatch".into(),
        ));
    }
    let m = DenseMatrix::new(n, n, matrix.iter().flatten().copied().collect())?;
    let lu = LuFactorization::new(&m)?;
    let res: Vec<f64> = (0..n)
        .map(|i| {
            let dot: f64 = (0..n).map(|j| matrix[i][j] * candidate[j]).sum();
            rhs[i] - dot
        })
        .collect();
    let sol = lu.solve(&res)?;
    let estimate = sol
        .iter()
        .zip(scales)
        .map(|(c, s)| (c / s).abs())
        .fold(0.0_f64, f64::max);
    Ok(EstimatedCorrection {
        estimate,
        source: EstimateSource::DenseLuComparator,
    })
}

/// Projected-nu estimator (PREREGISTRATION Case C / F105).
///
/// Always classified as [`EstimatedCorrection`], never `Verified`.
pub fn projected_nu_estimate(nu: f64) -> EstimatedCorrection {
    EstimatedCorrection {
        estimate: nu,
        source: EstimateSource::ProjectedNu,
    }
}

#[allow(dead_code)]
const _: () = {
    fn assert_send_sync<T: Send + Sync>() {}
    fn check() {
        assert_send_sync::<LinearTargetId>();
        assert_send_sync::<FrozenMetric>();
        assert_send_sync::<VerifiedCorrection>();
        assert_send_sync::<ResidualGainWitness>();
        assert_send_sync::<GainEvidence>();
    }
};
