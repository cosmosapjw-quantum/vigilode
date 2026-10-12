//! Current-operator residual gain witness (research node AS05,
//! `research/as05_residual_gain_core_20261011/PREREGISTRATION.md`), ported
//! from the re-audit prototype
//! `research/reaudit_accuracy_speed_20261010/gain_witness.rs`. Research
//! only: no driver calls this module and nothing here is a default.
//!
//! A [`GainEvidence::Verified`] result encloses the forward error `u* - u`
//! of one linear solve `W u* = b`, for one candidate `u`, in a frozen
//! diagonal metric `s` (`||v||_WRMS = sqrt(sum_k (v_k / s_k)^2 / n)`), for
//! the operator target it is bound to. It says nothing about stage
//! contamination, local or global ODE error, or order. Every bound comes
//! from the reviewed outward primitives of [`crate::directed`] and, for the
//! log-norm route, [`symmetric_part_upper`].
//!
//! The target identity [`LinearTargetId`] binds the operator (the exact real
//! matrix of binary64 entries, or the exact `I - h gamma J`), a SHA-256 of
//! its bits, the metric bits, an optional model epoch, the declared assembly
//! uncertainty and the declared structure. [`ResidualGainWitness::certify`]
//! refuses any target that differs from the witness's own binding, always
//! recomputes the residual `b - W u` itself, and reports an undeclared
//! assembly as [`GainEvidence::Estimated`], never as `Verified`.
//!
//! Routes:
//! - (a) [`ResidualGainWitness::represented`], dense, `n = 2`: the research
//!   algorithm (interval adjugate inverse, residual, correction, weighted
//!   correction) on [`Interval`].
//! - (b) declared isolated blocks of size 1 or 2; every off-block entry must
//!   be exactly zero.
//! - (c) [`ResidualGainWitness::shifted`]: `W` enclosed outward from `J`
//!   (widened entrywise by `delta`), `h` and `gamma`; the residual is
//!   recomputed with the interval `W`.
//! - (d) [`ResidualGainWitness::lognorm`], dense, `n <= 8`: with
//!   `a = h gamma >= 0`, `mu_up >= mu_2(D J D^-1)` for every `J` in
//!   `J +- delta` (`D = diag(1/s)`) and `1 - a mu_up > 0`,
//!   `||u* - u||_WRMS <= ||D r||_2 / (sqrt(n) (1 - a mu_up))`.
//! - The approximate-inverse defect route is not implemented:
//!   [`certify_by_approximate_inverse_defect`] returns
//!   `Unavailable(RouteNotImplemented)`.
//!
//! Authority types ([`VerifiedCorrection`], [`ResidualGainWitness`],
//! [`LinearTargetId`]) have private fields and no `Deserialize`
//! implementation; the witness API is their only constructor.
//!
//! ```
//! use rodas5p_core::residual_gain::{
//!     DeclaredStructure, FrozenMetric, GainEvidence, LinearTargetId, ResidualGainWitness,
//! };
//! let metric = FrozenMetric::new(&[1.0, 1.0]).unwrap();
//! let w = vec![vec![3.0, 1.0], vec![1.0, 2.0]];
//! let target =
//!     LinearTargetId::represented(&w, metric, Some(7), DeclaredStructure::Dense).unwrap();
//! let witness = ResidualGainWitness::represented(&target).unwrap();
//! match witness.certify(&target, &[1.0, 1.0], &[0.2, 0.4]) {
//!     GainEvidence::Verified(v) => assert!(v.wrms_upper() >= 0.0),
//!     other => panic!("{other:?}"),
//! }
//! ```
//!
//! A proof object cannot be deserialized:
//!
//! ```compile_fail
//! let _ = serde_json::from_str::<rodas5p_core::residual_gain::VerifiedCorrection>("{}");
//! ```
//!
//! nor written as a struct literal outside this module:
//!
//! ```compile_fail
//! use rodas5p_core::residual_gain::VerifiedCorrection;
//! let _ = VerifiedCorrection { wrms_upper: 0.0 };
//! ```
//!
//! ```compile_fail
//! use rodas5p_core::residual_gain::LinearTargetId;
//! let _ = LinearTargetId { dimension: 2, model_epoch: None };
//! ```
//!
//! ```compile_fail
//! use rodas5p_core::residual_gain::ResidualGainWitness;
//! let _ = ResidualGainWitness { setup_flops: 0 };
//! ```

use std::ops::Range;

use faer::Mat;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    CoreError, CoreResult, DenseMatrix, LuFactorization,
    directed::{
        Interval, add_up, div_down, div_up, mul_down, mul_up, sqrt_down, sqrt_up, sub_down,
    },
    nonnormal_certificate::symmetric_part_upper,
};

/// Largest dimension of the log-norm route (d).
pub const MAX_LOGNORM_DIMENSION: usize = 8;

/// Why no evidence was produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum WitnessRejection {
    /// A matrix entry (`W` or `J`) is not finite.
    NonfiniteMatrix,
    /// A metric entry is not positive and finite.
    InvalidMetric,
    /// `h` or `gamma` is not finite.
    InvalidShift,
    /// The declared `delta` is not finite and nonnegative.
    InvalidAssembly,
    /// Declared blocks do not partition `0..n` into blocks of size 1 or 2.
    InvalidStructure,
    NonfiniteRhs,
    NonfiniteCandidate,
    /// Shapes disagree (matrix, metric, right-hand side, candidate).
    DimensionMismatch,
    /// A directed operation or outward endpoint is not finite.
    ArithmeticOverflow,
    /// A determinant (or 1x1 pivot) interval contains zero.
    DeterminantContainsZero,
    /// Same operator kind, different operator bits.
    OperatorMismatch,
    /// Different operator kind, assembly or structure (e.g. a represented
    /// witness asked for a shifted target).
    TargetMismatch,
    MetricMismatch,
    StaleEpoch,
    /// A declared off-block entry is not exactly zero.
    OffBlockCoupling,
    /// A dense operator of a size the route does not certify.
    UndeclaredStructure,
    /// The route cannot be used for this declaration (log-norm with blocks,
    /// or `n` above [`MAX_LOGNORM_DIMENSION`]).
    RouteNotApplicable,
    /// The approximate-inverse defect route (`q < 1`) is not implemented.
    RouteNotImplemented,
    /// `h gamma < 0` (log-norm route).
    NegativeShift,
    /// `1 - a mu_up` is not verified positive (log-norm route).
    NonpositiveDenominator,
}

impl WitnessRejection {
    pub fn name(self) -> &'static str {
        match self {
            Self::NonfiniteMatrix => "NonfiniteMatrix",
            Self::InvalidMetric => "InvalidMetric",
            Self::InvalidShift => "InvalidShift",
            Self::InvalidAssembly => "InvalidAssembly",
            Self::InvalidStructure => "InvalidStructure",
            Self::NonfiniteRhs => "NonfiniteRhs",
            Self::NonfiniteCandidate => "NonfiniteCandidate",
            Self::DimensionMismatch => "DimensionMismatch",
            Self::ArithmeticOverflow => "ArithmeticOverflow",
            Self::DeterminantContainsZero => "DeterminantContainsZero",
            Self::OperatorMismatch => "OperatorMismatch",
            Self::TargetMismatch => "TargetMismatch",
            Self::MetricMismatch => "MetricMismatch",
            Self::StaleEpoch => "StaleEpoch",
            Self::OffBlockCoupling => "OffBlockCoupling",
            Self::UndeclaredStructure => "UndeclaredStructure",
            Self::RouteNotApplicable => "RouteNotApplicable",
            Self::RouteNotImplemented => "RouteNotImplemented",
            Self::NegativeShift => "NegativeShift",
            Self::NonpositiveDenominator => "NonpositiveDenominator",
        }
    }
}

fn overflow(_: CoreError) -> WitnessRejection {
    WitnessRejection::ArithmeticOverflow
}

/// A frozen diagonal error metric: positive finite scales `s`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FrozenMetric {
    scales: Vec<f64>,
}

impl FrozenMetric {
    pub fn new(scales: &[f64]) -> Result<Self, WitnessRejection> {
        if scales.is_empty() {
            return Err(WitnessRejection::DimensionMismatch);
        }
        if !scales.iter().all(|s| s.is_finite() && *s > 0.0) {
            return Err(WitnessRejection::InvalidMetric);
        }
        Ok(Self {
            scales: scales.to_vec(),
        })
    }

    pub fn scales(&self) -> &[f64] {
        &self.scales
    }

    fn same_bits(&self, other: &Self) -> bool {
        bits_equal(&self.scales, &other.scales)
    }
}

/// The declared uncertainty of the assembled `J` (shifted targets).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum AssemblyUncertainty {
    /// `J` is the exact real matrix of its binary64 entries.
    Exact,
    /// The true `J` lies entrywise within `+- delta` of the given one.
    EntrywiseAbs(f64),
    /// No bound: results are `Estimated`, never `Verified`.
    Undeclared,
}

impl AssemblyUncertainty {
    fn same_bits(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Exact, Self::Exact) | (Self::Undeclared, Self::Undeclared) => true,
            (Self::EntrywiseAbs(a), Self::EntrywiseAbs(b)) => a.to_bits() == b.to_bits(),
            _ => false,
        }
    }
}

/// The declared structure of the operator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum DeclaredStructure {
    Dense,
    /// Consecutive blocks of size 1 or 2 partitioning `0..n`; every entry
    /// outside them must be exactly zero.
    IsolatedBlocks(Vec<Range<usize>>),
}

/// The operator a target names (private: built only by the target API).
#[derive(Clone, Debug, Serialize)]
enum OperatorKind {
    /// The exact real matrix of the binary64 entries.
    Represented { w: Vec<Vec<f64>> },
    /// The exact `I - h gamma J`.
    Shifted {
        j: Vec<Vec<f64>>,
        h: f64,
        gamma: f64,
    },
}

impl OperatorKind {
    fn label(&self) -> &'static str {
        match self {
            Self::Represented { .. } => "represented",
            Self::Shifted { .. } => "shifted",
        }
    }

    fn same_bits(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Represented { w: a }, Self::Represented { w: b }) => matrix_bits_equal(a, b),
            (
                Self::Shifted {
                    j: a,
                    h: ha,
                    gamma: ga,
                },
                Self::Shifted {
                    j: b,
                    h: hb,
                    gamma: gb,
                },
            ) => {
                matrix_bits_equal(a, b)
                    && ha.to_bits() == hb.to_bits()
                    && ga.to_bits() == gb.to_bits()
            }
            _ => false,
        }
    }
}

fn bits_equal(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

fn matrix_bits_equal(a: &[Vec<f64>], b: &[Vec<f64>]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| bits_equal(x, y))
}

/// The identity of one linear target `W u = b`: operator, operator SHA-256,
/// metric, model epoch, assembly uncertainty and declared structure.
#[derive(Clone, Debug, Serialize)]
pub struct LinearTargetId {
    kind: OperatorKind,
    dimension: usize,
    operator_sha256: String,
    metric: FrozenMetric,
    model_epoch: Option<u64>,
    assembly: AssemblyUncertainty,
    structure: DeclaredStructure,
}

fn square(m: &[Vec<f64>]) -> Result<usize, WitnessRejection> {
    let n = m.len();
    if n == 0 || m.iter().any(|row| row.len() != n) {
        return Err(WitnessRejection::DimensionMismatch);
    }
    if !m.iter().flatten().all(|x| x.is_finite()) {
        return Err(WitnessRejection::NonfiniteMatrix);
    }
    Ok(n)
}

fn check_structure(structure: &DeclaredStructure, n: usize) -> Result<(), WitnessRejection> {
    if let DeclaredStructure::IsolatedBlocks(blocks) = structure {
        let mut next = 0;
        for block in blocks {
            let size = block.end.wrapping_sub(block.start);
            if block.start != next || block.end <= block.start || size > 2 {
                return Err(WitnessRejection::InvalidStructure);
            }
            next = block.end;
        }
        if next != n {
            return Err(WitnessRejection::InvalidStructure);
        }
    }
    Ok(())
}

fn operator_hash(kind: &OperatorKind, n: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"vigilode-as05-linear-target-v1\0");
    let (tag, m) = match kind {
        OperatorKind::Represented { w } => (0_u8, w),
        OperatorKind::Shifted { j, .. } => (1_u8, j),
    };
    let shift = match kind {
        OperatorKind::Represented { .. } => None,
        OperatorKind::Shifted { h, gamma, .. } => Some((*h, *gamma)),
    };
    hasher.update([tag]);
    hasher.update((n as u64).to_le_bytes());
    for x in m.iter().flatten() {
        hasher.update(x.to_bits().to_le_bytes());
    }
    if let Some((h, gamma)) = shift {
        hasher.update(h.to_bits().to_le_bytes());
        hasher.update(gamma.to_bits().to_le_bytes());
    }
    hex::encode(hasher.finalize())
}

impl LinearTargetId {
    /// A represented target: `W` is the exact real matrix of its entries.
    pub fn represented(
        w: &[Vec<f64>],
        metric: FrozenMetric,
        model_epoch: Option<u64>,
        structure: DeclaredStructure,
    ) -> Result<Self, WitnessRejection> {
        let n = square(w)?;
        if metric.scales.len() != n {
            return Err(WitnessRejection::DimensionMismatch);
        }
        check_structure(&structure, n)?;
        let kind = OperatorKind::Represented { w: w.to_vec() };
        Ok(Self {
            operator_sha256: operator_hash(&kind, n),
            kind,
            dimension: n,
            metric,
            model_epoch,
            assembly: AssemblyUncertainty::Exact,
            structure,
        })
    }

    /// A shifted target: the exact `W = I - h gamma J`.
    pub fn shifted(
        j: &[Vec<f64>],
        h: f64,
        gamma: f64,
        metric: FrozenMetric,
        model_epoch: Option<u64>,
        assembly: AssemblyUncertainty,
        structure: DeclaredStructure,
    ) -> Result<Self, WitnessRejection> {
        let n = square(j)?;
        if !(h.is_finite() && gamma.is_finite()) {
            return Err(WitnessRejection::InvalidShift);
        }
        if let AssemblyUncertainty::EntrywiseAbs(delta) = assembly
            && !(delta.is_finite() && delta >= 0.0)
        {
            return Err(WitnessRejection::InvalidAssembly);
        }
        if metric.scales.len() != n {
            return Err(WitnessRejection::DimensionMismatch);
        }
        check_structure(&structure, n)?;
        let kind = OperatorKind::Shifted {
            j: j.to_vec(),
            h,
            gamma,
        };
        Ok(Self {
            operator_sha256: operator_hash(&kind, n),
            kind,
            dimension: n,
            metric,
            model_epoch,
            assembly,
            structure,
        })
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// `represented` or `shifted`.
    pub fn operator_kind(&self) -> &'static str {
        self.kind.label()
    }

    /// SHA-256 (hex) of the operator kind, dimension, entry bits and, for a
    /// shifted target, the `h` and `gamma` bits.
    pub fn operator_sha256(&self) -> &str {
        &self.operator_sha256
    }

    pub fn metric(&self) -> &FrozenMetric {
        &self.metric
    }

    pub fn model_epoch(&self) -> Option<u64> {
        self.model_epoch
    }

    pub fn assembly(&self) -> AssemblyUncertainty {
        self.assembly
    }

    pub fn structure(&self) -> &DeclaredStructure {
        &self.structure
    }

    /// The first difference between this binding and `other`, if any.
    fn mismatch(&self, other: &Self) -> Option<WitnessRejection> {
        if self.kind.label() != other.kind.label() {
            return Some(WitnessRejection::TargetMismatch);
        }
        if self.dimension != other.dimension
            || self.operator_sha256 != other.operator_sha256
            || !self.kind.same_bits(&other.kind)
        {
            return Some(WitnessRejection::OperatorMismatch);
        }
        if !self.assembly.same_bits(&other.assembly) || self.structure != other.structure {
            return Some(WitnessRejection::TargetMismatch);
        }
        if !self.metric.same_bits(&other.metric) {
            return Some(WitnessRejection::MetricMismatch);
        }
        if self.model_epoch != other.model_epoch {
            return Some(WitnessRejection::StaleEpoch);
        }
        None
    }

    /// The binary64 operator `fl(W)`: the entries, or
    /// `fl(delta_ij - fl(fl(h gamma) J_ij))` (comparators and estimates).
    fn rounded_operator(&self) -> Vec<Vec<f64>> {
        match &self.kind {
            OperatorKind::Represented { w } => w.clone(),
            OperatorKind::Shifted { j, h, gamma } => {
                let hg = h * gamma;
                j.iter()
                    .enumerate()
                    .map(|(i, row)| {
                        row.iter()
                            .enumerate()
                            .map(|(k, x)| f64::from(u8::from(i == k)) - hg * x)
                            .collect()
                    })
                    .collect()
            }
        }
    }
}

/// Which witness route produced a result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum WitnessRoute {
    /// (a) represented, dense, `n = 2`.
    RepresentedDense2,
    /// (b) represented, declared isolated blocks.
    RepresentedBlocks,
    /// (c) shifted, `W` enclosed from `J`, `h`, `gamma` (dense `n = 2` or
    /// declared isolated blocks).
    ShiftedEnclosure,
    /// (d) log-norm, dense, `n <= 8`.
    Lognorm,
}

/// Counts directed-primitive calls (an interval add or sub is 2 calls, a
/// mul or div 8, a scalar directed operation 1).
#[derive(Default)]
struct Ops {
    calls: u64,
}

impl Ops {
    fn point(&self, x: f64) -> Result<Interval, WitnessRejection> {
        Interval::point(x).map_err(overflow)
    }

    fn add(&mut self, a: Interval, b: Interval) -> Result<Interval, WitnessRejection> {
        self.calls += 2;
        a.add(b).map_err(overflow)
    }

    fn sub(&mut self, a: Interval, b: Interval) -> Result<Interval, WitnessRejection> {
        self.calls += 2;
        a.sub(b).map_err(overflow)
    }

    fn mul(&mut self, a: Interval, b: Interval) -> Result<Interval, WitnessRejection> {
        self.calls += 8;
        a.mul(b).map_err(overflow)
    }

    fn div(&mut self, a: Interval, b: Interval) -> Result<Interval, WitnessRejection> {
        if b.contains_zero() {
            return Err(WitnessRejection::DeterminantContainsZero);
        }
        self.calls += 8;
        a.div(b).map_err(overflow)
    }

    fn scalar(&mut self, value: CoreResult<f64>) -> Result<f64, WitnessRejection> {
        self.calls += 1;
        value.map_err(overflow)
    }
}

#[derive(Clone, Debug)]
struct Block {
    range: Range<usize>,
    /// The interval inverse of the block, indexed from the block start.
    inverse: Vec<Vec<Interval>>,
}

/// The data of the log-norm route (d), reported with each result.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct LognormSummary {
    /// Enclosure `[lo, hi]` of `a = h gamma` (lower end clamped at 0).
    pub shift: [f64; 2],
    /// Verified `mu_up >= mu_2(D J D^-1)` for every admissible `J`.
    pub mu_up: f64,
    /// `interval-cholesky` or `gershgorin` ([`symmetric_part_upper`]).
    pub mu_source: &'static str,
    /// Lower bound on `1 - a mu_up` over the enclosed `a`.
    pub denominator_lower: f64,
}

/// An immutable witness bound to one target: the enclosed operator and,
/// for routes (a)-(c), the interval inverse of each block.
#[derive(Clone, Debug)]
pub struct ResidualGainWitness {
    target: LinearTargetId,
    route: WitnessRoute,
    w: Vec<Vec<Interval>>,
    blocks: Vec<Block>,
    lognorm: Option<LognormSummary>,
    residual_gain_inf_upper: Option<f64>,
    setup_flops: u64,
}

/// The interval `J` (widened entrywise by the declared `delta`; an
/// undeclared assembly uses the point `J`, and its results are estimates).
fn j_enclosure(
    ops: &mut Ops,
    j: &[Vec<f64>],
    assembly: AssemblyUncertainty,
) -> Result<Vec<Vec<Interval>>, WitnessRejection> {
    j.iter()
        .map(|row| {
            row.iter()
                .map(|x| match assembly {
                    AssemblyUncertainty::EntrywiseAbs(delta) => {
                        let lo = ops.scalar(sub_down(*x, delta))?;
                        let hi = ops.scalar(add_up(*x, delta))?;
                        Interval::new(lo, hi).map_err(overflow)
                    }
                    AssemblyUncertainty::Exact | AssemblyUncertainty::Undeclared => ops.point(*x),
                })
                .collect()
        })
        .collect()
}

/// `W = I - (h gamma) J` enclosed outward.
fn shifted_enclosure(
    ops: &mut Ops,
    j: &[Interval],
    n: usize,
    hg: Interval,
) -> Result<Vec<Vec<Interval>>, WitnessRejection> {
    let mut w = Vec::with_capacity(n);
    for i in 0..n {
        let mut row = Vec::with_capacity(n);
        for k in 0..n {
            let identity = ops.point(f64::from(u8::from(i == k)))?;
            let product = ops.mul(hg, j[i * n + k])?;
            row.push(ops.sub(identity, product)?);
        }
        w.push(row);
    }
    Ok(w)
}

fn is_exact_zero(x: &Interval) -> bool {
    x.lo == 0.0 && x.hi == 0.0
}

impl ResidualGainWitness {
    /// Routes (a) and (b): a represented target, dense `n = 2` or declared
    /// isolated blocks. A dense target of another size is
    /// `UndeclaredStructure`; a shifted target is `TargetMismatch`.
    pub fn represented(target: &LinearTargetId) -> Result<Self, WitnessRejection> {
        let OperatorKind::Represented { w } = &target.kind else {
            return Err(WitnessRejection::TargetMismatch);
        };
        let ops = Ops::default();
        let wi = w
            .iter()
            .map(|row| row.iter().map(|x| ops.point(*x)).collect())
            .collect::<Result<Vec<Vec<_>>, _>>()?;
        let route = match target.structure {
            DeclaredStructure::Dense => WitnessRoute::RepresentedDense2,
            DeclaredStructure::IsolatedBlocks(_) => WitnessRoute::RepresentedBlocks,
        };
        Self::with_blocks(ops, target, wi, route)
    }

    /// Route (c): a shifted target, dense `n = 2` or declared isolated
    /// blocks; `W` is enclosed from `J` (widened by the declared `delta`),
    /// `h` and `gamma`.
    pub fn shifted(target: &LinearTargetId) -> Result<Self, WitnessRejection> {
        let OperatorKind::Shifted { j, h, gamma } = &target.kind else {
            return Err(WitnessRejection::TargetMismatch);
        };
        let mut ops = Ops::default();
        let n = target.dimension;
        let hg = ops.mul(ops.point(*h)?, ops.point(*gamma)?)?;
        let ji: Vec<Interval> = j_enclosure(&mut ops, j, target.assembly)?
            .into_iter()
            .flatten()
            .collect();
        let wi = shifted_enclosure(&mut ops, &ji, n, hg)?;
        Self::with_blocks(ops, target, wi, WitnessRoute::ShiftedEnclosure)
    }

    fn with_blocks(
        mut ops: Ops,
        target: &LinearTargetId,
        w: Vec<Vec<Interval>>,
        route: WitnessRoute,
    ) -> Result<Self, WitnessRejection> {
        let n = target.dimension;
        let ranges = match &target.structure {
            DeclaredStructure::Dense => {
                if n != 2 {
                    return Err(WitnessRejection::UndeclaredStructure);
                }
                vec![Range { start: 0, end: 2 }]
            }
            DeclaredStructure::IsolatedBlocks(blocks) => {
                let mut owner = vec![0; n];
                for (b, block) in blocks.iter().enumerate() {
                    for i in block.clone() {
                        owner[i] = b;
                    }
                }
                for i in 0..n {
                    for k in 0..n {
                        if owner[i] != owner[k] && !is_exact_zero(&w[i][k]) {
                            return Err(WitnessRejection::OffBlockCoupling);
                        }
                    }
                }
                blocks.clone()
            }
        };
        let scales = target.metric.scales();
        let mut blocks = Vec::with_capacity(ranges.len());
        let mut gain: f64 = 0.0;
        for range in ranges {
            let s = range.start;
            let inverse = if range.len() == 1 {
                vec![vec![ops.div(ops.point(1.0)?, w[s][s])?]]
            } else {
                let (a, b, c, d) = (w[s][s], w[s][s + 1], w[s + 1][s], w[s + 1][s + 1]);
                let ad = ops.mul(a, d)?;
                let bc = ops.mul(b, c)?;
                let determinant = ops.sub(ad, bc)?;
                if determinant.contains_zero() {
                    return Err(WitnessRejection::DeterminantContainsZero);
                }
                vec![
                    vec![ops.div(d, determinant)?, ops.div(-b, determinant)?],
                    vec![ops.div(-c, determinant)?, ops.div(a, determinant)?],
                ]
            };
            // || S^-1 W^-1 S ||_inf restricted to the block, S = diag(s).
            for (i, row) in inverse.iter().enumerate() {
                let mut sum = ops.point(0.0)?;
                for (k, entry) in row.iter().enumerate() {
                    let term = ops.mul(ops.point(entry.mag())?, ops.point(scales[s + k])?)?;
                    let term = ops.div(term, ops.point(scales[s + i])?)?;
                    sum = ops.add(sum, term)?;
                }
                gain = gain.max(sum.hi);
            }
            blocks.push(Block { range, inverse });
        }
        Ok(Self {
            target: target.clone(),
            route,
            w,
            blocks,
            lognorm: None,
            residual_gain_inf_upper: Some(gain),
            setup_flops: ops.calls,
        })
    }

    /// Route (d): a shifted dense target with `n <= 8`.
    pub fn lognorm(target: &LinearTargetId) -> Result<Self, WitnessRejection> {
        let OperatorKind::Shifted { j, h, gamma } = &target.kind else {
            return Err(WitnessRejection::TargetMismatch);
        };
        let n = target.dimension;
        if target.structure != DeclaredStructure::Dense || n > MAX_LOGNORM_DIMENSION {
            return Err(WitnessRejection::RouteNotApplicable);
        }
        // The exact sign of h gamma is the product of the signs.
        if *h != 0.0 && *gamma != 0.0 && (*h < 0.0) != (*gamma < 0.0) {
            return Err(WitnessRejection::NegativeShift);
        }
        let mut ops = Ops::default();
        let hg = ops.mul(ops.point(*h)?, ops.point(*gamma)?)?;
        // h gamma >= 0 exactly: an outward step below zero is clamped.
        let a = Interval::new(hg.lo.max(0.0), hg.hi).map_err(overflow)?;
        let ji: Vec<Interval> = j_enclosure(&mut ops, j, target.assembly)?
            .into_iter()
            .flatten()
            .collect();
        let s = target.metric.scales();
        // B = D J D^-1 with D = diag(1/s): b_ik = J_ik s_k / s_i.
        let mut b = vec![vec![ops.point(0.0)?; n]; n];
        for i in 0..n {
            for k in 0..n {
                let t = ops.mul(ji[i * n + k], ops.point(s[k])?)?;
                b[i][k] = ops.div(t, ops.point(s[i])?)?;
            }
        }
        let (mu_up, mu_source) = symmetric_part_upper(&b).map_err(overflow)?;
        if !mu_up.is_finite() {
            return Err(WitnessRejection::ArithmeticOverflow);
        }
        let am = ops.mul(a, ops.point(mu_up)?)?;
        let denominator_lower = ops.scalar(sub_down(1.0, am.hi))?;
        if denominator_lower.is_nan() || denominator_lower <= 0.0 {
            return Err(WitnessRejection::NonpositiveDenominator);
        }
        let w = shifted_enclosure(&mut ops, &ji, n, hg)?;
        Ok(Self {
            target: target.clone(),
            route: WitnessRoute::Lognorm,
            w,
            blocks: Vec::new(),
            lognorm: Some(LognormSummary {
                shift: [a.lo, a.hi],
                mu_up,
                mu_source,
                denominator_lower,
            }),
            residual_gain_inf_upper: None,
            setup_flops: ops.calls,
        })
    }

    pub fn target(&self) -> &LinearTargetId {
        &self.target
    }

    pub fn route(&self) -> WitnessRoute {
        self.route
    }

    /// Directed-primitive calls of the construction (the internal search
    /// of [`symmetric_part_upper`] is not counted).
    pub fn setup_flops(&self) -> u64 {
        self.setup_flops
    }

    /// Upper bound on `||S^-1 W^-1 S||_inf` (routes (a)-(c)).
    pub fn residual_gain_inf_upper(&self) -> Option<f64> {
        self.residual_gain_inf_upper
    }

    pub fn lognorm_summary(&self) -> Option<LognormSummary> {
        self.lognorm
    }

    /// The enclosure of the operator `W` the witness certifies with.
    pub fn operator_enclosure(&self) -> Vec<Vec<Interval>> {
        self.w.clone()
    }

    /// Certify `candidate` for `W u = rhs` on `target`, which must equal the
    /// witness's own binding. The residual is always recomputed here.
    pub fn certify(&self, target: &LinearTargetId, rhs: &[f64], candidate: &[f64]) -> GainEvidence {
        match self.certify_inner(target, rhs, candidate) {
            Ok(evidence) => evidence,
            Err(rejection) => GainEvidence::Unavailable(rejection),
        }
    }

    fn certify_inner(
        &self,
        target: &LinearTargetId,
        rhs: &[f64],
        candidate: &[f64],
    ) -> Result<GainEvidence, WitnessRejection> {
        if let Some(rejection) = self.target.mismatch(target) {
            return Err(rejection);
        }
        let n = self.target.dimension;
        if rhs.len() != n || candidate.len() != n {
            return Err(WitnessRejection::DimensionMismatch);
        }
        if !rhs.iter().all(|x| x.is_finite()) {
            return Err(WitnessRejection::NonfiniteRhs);
        }
        if !candidate.iter().all(|x| x.is_finite()) {
            return Err(WitnessRejection::NonfiniteCandidate);
        }
        let mut ops = Ops::default();
        let s = self.target.metric.scales();
        let (residual, correction, weighted, weighted_inf_upper, wrms_upper) = match self.route {
            WitnessRoute::Lognorm => {
                let summary = self.lognorm.expect("log-norm route has its summary");
                let mut residual = Vec::with_capacity(n);
                let mut norm_sq = 0.0;
                for i in 0..n {
                    let mut dot = ops.mul(self.w[i][0], ops.point(candidate[0])?)?;
                    for (wk, xk) in self.w[i].iter().zip(candidate).skip(1) {
                        let term = ops.mul(*wk, ops.point(*xk)?)?;
                        dot = ops.add(dot, term)?;
                    }
                    let r = ops.sub(ops.point(rhs[i])?, dot)?;
                    let scaled = ops.div(r, ops.point(s[i])?)?.mag();
                    let square = ops.scalar(mul_up(scaled, scaled))?;
                    norm_sq = ops.scalar(add_up(norm_sq, square))?;
                    residual.push(r);
                }
                let norm = ops.scalar(sqrt_up(norm_sq))?;
                let inf_upper = ops.scalar(div_up(norm, summary.denominator_lower))?;
                let root_n = ops.scalar(sqrt_down(n as f64))?;
                let denominator = ops.scalar(mul_down(root_n, summary.denominator_lower))?;
                let wrms = ops.scalar(div_up(norm, denominator))?;
                (residual, None, None, inf_upper, wrms)
            }
            _ => {
                let mut residual = vec![ops.point(0.0)?; n];
                let mut correction = vec![ops.point(0.0)?; n];
                let mut weighted = vec![ops.point(0.0)?; n];
                let mut inf_upper: f64 = 0.0;
                for block in &self.blocks {
                    let cols = block.range.clone();
                    for i in cols.clone() {
                        let mut dot: Option<Interval> = None;
                        for k in cols.clone() {
                            let term = ops.mul(self.w[i][k], ops.point(candidate[k])?)?;
                            dot = Some(match dot {
                                None => term,
                                Some(sum) => ops.add(sum, term)?,
                            });
                        }
                        let dot = dot.expect("blocks are nonempty");
                        residual[i] = ops.sub(ops.point(rhs[i])?, dot)?;
                    }
                    for (bi, i) in cols.clone().enumerate() {
                        let mut sum: Option<Interval> = None;
                        for (bk, k) in cols.clone().enumerate() {
                            let term = ops.mul(block.inverse[bi][bk], residual[k])?;
                            sum = Some(match sum {
                                None => term,
                                Some(acc) => ops.add(acc, term)?,
                            });
                        }
                        correction[i] = sum.expect("blocks are nonempty");
                        weighted[i] = ops.div(correction[i], ops.point(s[i])?)?;
                        inf_upper = inf_upper.max(weighted[i].mag());
                    }
                }
                // The weighted infinity bound also bounds the weighted RMS.
                (
                    residual,
                    Some(correction),
                    Some(weighted),
                    inf_upper,
                    inf_upper,
                )
            }
        };
        if self.target.assembly == AssemblyUncertainty::Undeclared {
            return Ok(GainEvidence::Estimated(EstimatedCorrection {
                estimate: wrms_upper,
                source: EstimateSource::UndeclaredAssembly,
            }));
        }
        let pairs = |v: Vec<Interval>| v.into_iter().map(|x| [x.lo, x.hi]).collect();
        Ok(GainEvidence::Verified(VerifiedCorrection {
            target: self.target.clone(),
            route: self.route,
            residual: pairs(residual),
            correction: correction.map(pairs),
            weighted_correction: weighted.map(pairs),
            weighted_inf_upper,
            wrms_upper,
            residual_gain_inf_upper: self.residual_gain_inf_upper,
            lognorm: self.lognorm,
            setup_flops: self.setup_flops,
            verification_flops: ops.calls,
            transport_factor: None,
        }))
    }
}

/// Where an estimate came from. Estimates are never certificates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum EstimateSource {
    /// `nu ||D r||_2 / sqrt(n)` with `nu = 1/sigma_min` of a projected
    /// Arnoldi Hessenberg matrix: not a full-space inverse-norm bound.
    ProjectedNu,
    /// `||c / s||_inf` for the partial-pivot LU solution `c` of
    /// `fl(W) c = fl(b - fl(W) u)`.
    DenseLuComparator,
    /// The route's bound computed with the point `J`: the assembly
    /// uncertainty was not declared.
    UndeclaredAssembly,
}

/// A plain estimate (weighted RMS for `ProjectedNu` and
/// `UndeclaredAssembly`, weighted infinity for `DenseLuComparator`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct EstimatedCorrection {
    pub estimate: f64,
    pub source: EstimateSource,
}

/// A verified enclosure of `u* - u` for the bound target.
#[derive(Clone, Debug, Serialize)]
pub struct VerifiedCorrection {
    target: LinearTargetId,
    route: WitnessRoute,
    residual: Vec<[f64; 2]>,
    correction: Option<Vec<[f64; 2]>>,
    weighted_correction: Option<Vec<[f64; 2]>>,
    weighted_inf_upper: f64,
    wrms_upper: f64,
    residual_gain_inf_upper: Option<f64>,
    lognorm: Option<LognormSummary>,
    setup_flops: u64,
    verification_flops: u64,
    transport_factor: Option<f64>,
}

fn intervals(v: &[[f64; 2]]) -> Vec<Interval> {
    v.iter().map(|p| Interval { lo: p[0], hi: p[1] }).collect()
}

impl VerifiedCorrection {
    pub fn target(&self) -> &LinearTargetId {
        &self.target
    }

    pub fn route(&self) -> WitnessRoute {
        self.route
    }

    /// Enclosure of the residual `b - W u` (for every admissible `W`).
    pub fn residual(&self) -> Vec<Interval> {
        intervals(&self.residual)
    }

    /// Componentwise enclosure of `u* - u` (routes (a)-(c)).
    pub fn correction(&self) -> Option<Vec<Interval>> {
        self.correction.as_deref().map(intervals)
    }

    /// Componentwise enclosure of `(u* - u) / s` (routes (a)-(c)).
    pub fn weighted_correction(&self) -> Option<Vec<Interval>> {
        self.weighted_correction.as_deref().map(intervals)
    }

    /// Upper bound on `||(u* - u) / s||_inf`.
    pub fn weighted_inf_upper(&self) -> f64 {
        self.weighted_inf_upper
    }

    /// Upper bound on `||u* - u||_WRMS`.
    pub fn wrms_upper(&self) -> f64 {
        self.wrms_upper
    }

    pub fn residual_gain_inf_upper(&self) -> Option<f64> {
        self.residual_gain_inf_upper
    }

    pub fn lognorm_summary(&self) -> Option<LognormSummary> {
        self.lognorm
    }

    pub fn setup_flops(&self) -> u64 {
        self.setup_flops
    }

    pub fn verification_flops(&self) -> u64 {
        self.verification_flops
    }

    /// The accumulated `kappa` of [`Self::transport_to`], if any.
    pub fn transport_factor(&self) -> Option<f64> {
        self.transport_factor
    }

    /// The only metric change: the bounds are multiplied by
    /// `kappa = max_k s_old,k / s_new,k` (rounded up), each weighted
    /// component by its enclosed ratio `s_old,k / s_new,k`, the residual gain
    /// by `kappa max_k s_new,k / s_old,k`; the target is rebound to the new
    /// metric (same operator, epoch, assembly and structure).
    pub fn transport_to(&self, new_metric: &FrozenMetric) -> Result<Self, WitnessRejection> {
        let old = self.target.metric.scales();
        let new = new_metric.scales();
        if old.len() != new.len() {
            return Err(WitnessRejection::DimensionMismatch);
        }
        let mut kappa: f64 = 0.0;
        let mut back: f64 = 0.0;
        let mut ratios = Vec::with_capacity(old.len());
        for (so, sn) in old.iter().zip(new) {
            let hi = div_up(*so, *sn).map_err(overflow)?;
            let lo = div_down(*so, *sn).map_err(overflow)?;
            kappa = kappa.max(hi);
            back = back.max(div_up(*sn, *so).map_err(overflow)?);
            ratios.push(Interval::new(lo, hi).map_err(overflow)?);
        }
        let weighted_correction = match &self.weighted_correction {
            None => None,
            Some(w) => Some(
                intervals(w)
                    .into_iter()
                    .zip(&ratios)
                    .map(|(x, r)| x.mul(*r).map(|y| [y.lo, y.hi]))
                    .collect::<CoreResult<Vec<_>>>()
                    .map_err(overflow)?,
            ),
        };
        let residual_gain_inf_upper = match self.residual_gain_inf_upper {
            None => None,
            Some(g) => Some(mul_up(mul_up(g, kappa).map_err(overflow)?, back).map_err(overflow)?),
        };
        let transport_factor = match self.transport_factor {
            None => kappa,
            Some(k) => mul_up(k, kappa).map_err(overflow)?,
        };
        let mut target = self.target.clone();
        target.metric = new_metric.clone();
        Ok(Self {
            target,
            route: self.route,
            residual: self.residual.clone(),
            correction: self.correction.clone(),
            weighted_correction,
            weighted_inf_upper: mul_up(self.weighted_inf_upper, kappa).map_err(overflow)?,
            wrms_upper: mul_up(self.wrms_upper, kappa).map_err(overflow)?,
            residual_gain_inf_upper,
            lognorm: self.lognorm,
            setup_flops: self.setup_flops,
            verification_flops: self.verification_flops,
            transport_factor: Some(transport_factor),
        })
    }
}

/// The result of one certification request.
// The verified variant carries its enclosures by value (registered shape).
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Serialize)]
pub enum GainEvidence {
    Verified(VerifiedCorrection),
    Estimated(EstimatedCorrection),
    Unavailable(WitnessRejection),
}

impl GainEvidence {
    /// `verified`, `estimated` or `unavailable`.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Verified(_) => "verified",
            Self::Estimated(_) => "estimated",
            Self::Unavailable(_) => "unavailable",
        }
    }
}

/// The approximate-inverse defect route (`||I - X W|| <= q < 1`) is not
/// implemented in this node.
pub fn certify_by_approximate_inverse_defect(
    _target: &LinearTargetId,
    _rhs: &[f64],
    _candidate: &[f64],
) -> GainEvidence {
    GainEvidence::Unavailable(WitnessRejection::RouteNotImplemented)
}

fn check_vectors(
    target: &LinearTargetId,
    rhs: &[f64],
    candidate: &[f64],
) -> Result<usize, WitnessRejection> {
    let n = target.dimension;
    if rhs.len() != n || candidate.len() != n {
        return Err(WitnessRejection::DimensionMismatch);
    }
    if !rhs.iter().all(|x| x.is_finite()) {
        return Err(WitnessRejection::NonfiniteRhs);
    }
    if !candidate.iter().all(|x| x.is_finite()) {
        return Err(WitnessRejection::NonfiniteCandidate);
    }
    Ok(n)
}

fn rounded_residual(w: &[Vec<f64>], rhs: &[f64], candidate: &[f64]) -> Vec<f64> {
    w.iter()
        .zip(rhs)
        .map(|(row, b)| b - row.iter().zip(candidate).map(|(x, u)| x * u).sum::<f64>())
        .collect()
}

/// The projected-`nu` estimate (finding F105): Arnoldi on `fl(W)` from
/// `start` for at most `max_steps` steps, `nu = 1 / sigma_min(H)` of the
/// Hessenberg matrix (square after a breakdown), times `||D r||_2 /
/// sqrt(n)`. A projected `nu` is not a full-space inverse-norm bound: the
/// result is always `Estimated` (or `Unavailable` on invalid input).
pub fn projected_nu_estimate(
    target: &LinearTargetId,
    rhs: &[f64],
    candidate: &[f64],
    start: &[f64],
    max_steps: usize,
) -> GainEvidence {
    let n = match check_vectors(target, rhs, candidate) {
        Ok(n) => n,
        Err(rejection) => return GainEvidence::Unavailable(rejection),
    };
    let start_norm = start.iter().map(|x| x * x).sum::<f64>().sqrt();
    if start.len() != n || !start_norm.is_finite() || start_norm == 0.0 || max_steps == 0 {
        return GainEvidence::Unavailable(WitnessRejection::DimensionMismatch);
    }
    let w = target.rounded_operator();
    let mut basis = vec![start.iter().map(|x| x / start_norm).collect::<Vec<f64>>()];
    let steps = max_steps.min(n);
    let mut h = vec![vec![0.0; steps]; steps + 1];
    let mut columns = 0;
    let mut square = false;
    for k in 0..steps {
        let mut v: Vec<f64> = w
            .iter()
            .map(|row| row.iter().zip(&basis[k]).map(|(a, b)| a * b).sum())
            .collect();
        let scale = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        for (i, q) in basis.iter().enumerate() {
            let c: f64 = q.iter().zip(&v).map(|(a, b)| a * b).sum();
            h[i][k] = c;
            for (x, y) in v.iter_mut().zip(q) {
                *x -= c * y;
            }
        }
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        h[k + 1][k] = norm;
        columns = k + 1;
        if norm <= 1e-14 * scale.max(f64::MIN_POSITIVE) {
            square = true;
            break;
        }
        basis.push(v.iter().map(|x| x / norm).collect());
    }
    let rows = if square { columns } else { columns + 1 };
    let hm = Mat::<f64>::from_fn(rows, columns, |i, j| h[i][j]);
    let sigma_min = match hm.singular_values() {
        Ok(s) => s.last().copied().unwrap_or(0.0),
        Err(_) => return GainEvidence::Unavailable(WitnessRejection::ArithmeticOverflow),
    };
    let nu = 1.0 / sigma_min;
    let s = target.metric.scales();
    let r = rounded_residual(&w, rhs, candidate);
    let dr = r
        .iter()
        .zip(s)
        .map(|(x, si)| (x / si) * (x / si))
        .sum::<f64>()
        .sqrt();
    let estimate = nu * dr / (n as f64).sqrt();
    if !estimate.is_finite() {
        return GainEvidence::Unavailable(WitnessRejection::ArithmeticOverflow);
    }
    GainEvidence::Estimated(EstimatedCorrection {
        estimate,
        source: EstimateSource::ProjectedNu,
    })
}

/// Nominal flops of the dense LU comparator: `2/3 n^3 + 2 n^2`.
pub fn lu_comparator_flops(n: usize) -> f64 {
    let n = n as f64;
    2.0 / 3.0 * n * n * n + 2.0 * n * n
}

/// The dense LU comparator (reported, never verified): faer's partial-pivot
/// LU solution `c` of `fl(W) c = r`, `r = fl(b - fl(W) u)`, and
/// `||c / s||_inf`.
pub fn dense_lu_comparator(
    target: &LinearTargetId,
    rhs: &[f64],
    candidate: &[f64],
) -> Result<EstimatedCorrection, WitnessRejection> {
    check_vectors(target, rhs, candidate)?;
    let w = target.rounded_operator();
    let r = rounded_residual(&w, rhs, candidate);
    let matrix = DenseMatrix::from_vec_rows(w).map_err(overflow)?;
    let c = LuFactorization::new(&matrix)
        .and_then(|lu| lu.solve(&r))
        .map_err(overflow)?;
    let estimate = c
        .iter()
        .zip(target.metric.scales())
        .map(|(x, s)| (x / s).abs())
        .fold(0.0_f64, f64::max);
    Ok(EstimatedCorrection {
        estimate,
        source: EstimateSource::DenseLuComparator,
    })
}
