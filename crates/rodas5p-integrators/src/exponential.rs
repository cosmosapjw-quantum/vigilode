use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use rodas5p_core::{
    ClosureOperator, CoreError, CoreResult, DenseMatrix, LinearOperator, WorkCounters,
    dense_phi_action, error_scale, safe_l2, wrms,
};
use serde::{Deserialize, Serialize};

use crate::{OdeProblem, ParallelExecution};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExponentialKrylovConfig {
    pub minimum_dimension: usize,
    pub maximum_dimension: usize,
    pub dimension_increment: usize,
    pub relative_tolerance: f64,
    pub absolute_tolerance: f64,
    pub reorthogonalize: bool,
}

impl Default for ExponentialKrylovConfig {
    fn default() -> Self {
        Self {
            minimum_dimension: 4,
            maximum_dimension: 24,
            dimension_increment: 2,
            relative_tolerance: 1.0e-11,
            absolute_tolerance: 1.0e-13,
            reorthogonalize: true,
        }
    }
}

impl ExponentialKrylovConfig {
    fn validate(self, n: usize) -> CoreResult<Self> {
        if self.minimum_dimension == 0
            || self.maximum_dimension < self.minimum_dimension
            || self.dimension_increment == 0
            || self.maximum_dimension == 0
        {
            return Err(CoreError::InvalidInput(
                "invalid exponential Krylov dimension contract".into(),
            ));
        }
        if !(self.relative_tolerance >= 0.0
            && self.relative_tolerance.is_finite()
            && self.absolute_tolerance >= 0.0
            && self.absolute_tolerance.is_finite())
        {
            return Err(CoreError::InvalidInput(
                "invalid exponential Krylov tolerance".into(),
            ));
        }
        let maximum_dimension = self.maximum_dimension.min(n.max(1));
        let minimum_dimension = self.minimum_dimension.min(maximum_dimension);
        Ok(Self {
            minimum_dimension,
            maximum_dimension,
            ..self
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FusedOrthogonalization {
    FullMgs,
    Incomplete { length: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FusedPhiKrylovConfig {
    pub minimum_dimension: usize,
    pub maximum_dimension: usize,
    pub dimension_increment: usize,
    pub relative_tolerance: f64,
    pub absolute_tolerance: f64,
    pub orthogonalization: FusedOrthogonalization,
    pub maximum_substeps: usize,
}

impl Default for FusedPhiKrylovConfig {
    fn default() -> Self {
        Self {
            minimum_dimension: 4,
            maximum_dimension: 24,
            dimension_increment: 2,
            relative_tolerance: 1.0e-10,
            absolute_tolerance: 1.0e-13,
            orthogonalization: FusedOrthogonalization::FullMgs,
            maximum_substeps: 16,
        }
    }
}

impl FusedPhiKrylovConfig {
    fn validate(self, dimension: usize) -> CoreResult<Self> {
        if self.minimum_dimension == 0
            || self.maximum_dimension < self.minimum_dimension
            || self.dimension_increment == 0
            || self.maximum_substeps == 0
        {
            return Err(CoreError::InvalidInput(
                "invalid fused phi Krylov dimension/substep contract".into(),
            ));
        }
        if !(self.relative_tolerance >= 0.0
            && self.relative_tolerance.is_finite()
            && self.absolute_tolerance >= 0.0
            && self.absolute_tolerance.is_finite())
        {
            return Err(CoreError::InvalidInput(
                "invalid fused phi Krylov tolerance".into(),
            ));
        }
        if matches!(
            self.orthogonalization,
            FusedOrthogonalization::Incomplete { length: 0 }
        ) {
            return Err(CoreError::InvalidInput(
                "incomplete orthogonalization length must be positive".into(),
            ));
        }
        let maximum_dimension = self.maximum_dimension.min(dimension.max(1));
        let minimum_dimension = self.minimum_dimension.min(maximum_dimension);
        Ok(Self {
            minimum_dimension,
            maximum_dimension,
            ..self
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FusedPhiSubstepReport {
    pub substep_index: usize,
    pub krylov_dimension: usize,
    pub converged: bool,
    pub happy_breakdown: bool,
    /// KIOPS/Saad first-term estimate
    /// `|tau h_{m+1,m} e_m^T phi_1(tau H_m) beta e_1|`.
    pub error_estimate: f64,
    /// Nested-dimension difference retained only as an independent diagnostic.
    pub nested_difference_estimate: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FusedPhiActionReport {
    pub scale: f64,
    pub highest_phi_index: usize,
    pub substeps: usize,
    pub converged: bool,
    pub maximum_krylov_dimension: usize,
    /// Sum of the per-substep residual-based estimates.
    pub error_estimate: f64,
    /// Sum of finite nested-dimension differences; diagnostic only.
    pub nested_difference_estimate: f64,
    pub action_norm: f64,
    pub value: Vec<f64>,
    pub substep_reports: Vec<FusedPhiSubstepReport>,
}

#[derive(Clone, Copy, Debug)]
pub struct FusedPhiTerm<'a> {
    pub coefficient: f64,
    pub phi_index: usize,
    pub vector: &'a [f64],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhiActionReport {
    pub phi_index: usize,
    pub scale: f64,
    pub krylov_dimension: usize,
    pub converged: bool,
    pub happy_breakdown: bool,
    pub error_estimate: f64,
    pub action_norm: f64,
    pub value: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExponentialStepReport {
    pub method: String,
    pub y_new: Vec<f64>,
    pub y_embedded: Option<Vec<f64>>,
    pub error_estimate: Option<Vec<f64>>,
    pub logical_critical_depth: usize,
    pub phi_reports: Vec<PhiActionReport>,
    pub work: WorkCounters,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FusedExponentialStepReport {
    pub method: String,
    pub y_new: Vec<f64>,
    pub y_embedded: Option<Vec<f64>>,
    pub error_estimate: Option<Vec<f64>>,
    pub logical_critical_depth: usize,
    pub fused_phi_reports: Vec<FusedPhiActionReport>,
    pub work: WorkCounters,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub early_flow_defect: Option<EarlyFlowDefectTelemetry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EarlyFlowDefectTelemetryMode {
    Disabled,
    ReadOnly { norm_component_count: usize },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EarlyFlowDefectDiagnosticWork {
    pub conceptual_vector_differences: u64,
    pub l2_norm_evaluations: u64,
    pub scalar_normalizations: u64,
    #[serde(default)]
    pub component_scale_evaluations: u64,
    #[serde(default)]
    pub wrms_norm_evaluations: u64,
    pub added_rhs_calls: u64,
    pub added_jvp_calls: u64,
    pub added_jvp_vectors: u64,
    pub added_phi_actions: u64,
    pub added_partial_t_calls: u64,
    pub added_jacobian_builds: u64,
    pub added_newton_iterations: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EarlyFlowDefectTelemetry {
    pub stage_fraction: f64,
    pub state_dimension: usize,
    pub norm_component_count: usize,
    pub excluded_trailing_components: usize,
    pub abs_h: f64,
    pub stage_increment_l2: f64,
    pub nonlinear_remainder_l2: f64,
    pub normalized_defect: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_scaled_defect_wrms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_scale_atol: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_scale_rtol: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_scaled_nonfinite: Option<bool>,
    pub zero_increment: bool,
    pub degenerate_nonzero_remainder: bool,
    pub nonfinite_normalization: bool,
    pub native_partial_t_sampled: bool,
    pub diagnostic_work: EarlyFlowDefectDiagnosticWork,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pexprb54s4Level1PrefixReport {
    pub method: String,
    pub t: f64,
    pub h: f64,
    pub logical_critical_depth: usize,
    pub fused_phi_reports: Vec<FusedPhiActionReport>,
    pub work: WorkCounters,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub early_flow_defect: Option<EarlyFlowDefectTelemetry>,
}

/// Retained pexprb54s4 dependency-level-1 state.
///
/// The public report is serializable for research telemetry.  The numerical
/// base state, matrix-free operator, F0, and D2 remain private so callers can
/// only continue through the checked resume function rather than reconstructing
/// a partial method state by hand.
pub struct Pexprb54s4Level1Prefix {
    problem: OdeProblem,
    t: f64,
    y: Vec<f64>,
    h: f64,
    config: FusedPhiKrylovConfig,
    f0: Vec<f64>,
    operator: Arc<dyn LinearOperator>,
    d2: Vec<f64>,
    u2_action: FusedPhiActionReport,
    telemetry_mode: EarlyFlowDefectTelemetryMode,
    tolerance_scale: Option<EarlyFlowDefectToleranceScale>,
    report: Pexprb54s4Level1PrefixReport,
}

impl Pexprb54s4Level1Prefix {
    pub fn report(&self) -> &Pexprb54s4Level1PrefixReport {
        &self.report
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pexprb54s4RemainderVectorGeometry {
    pub state_dimension: usize,
    pub norm_component_count: usize,
    pub excluded_trailing_components: usize,
    pub chi23: Option<f64>,
    pub chi34: Option<f64>,
    pub chi24: Option<f64>,
    pub q34_perp: Option<f64>,
    pub delta_chi: Option<f64>,
}

fn normalized_direction_cosine(a: &[f64], b: &[f64]) -> Option<f64> {
    let norm_a = safe_l2(a);
    let norm_b = safe_l2(b);
    if !(norm_a > 0.0 && norm_a.is_finite() && norm_b > 0.0 && norm_b.is_finite()) {
        return None;
    }
    let cosine = a
        .iter()
        .zip(b)
        .map(|(&x, &y)| (x / norm_a) * (y / norm_b))
        .sum::<f64>();
    cosine.is_finite().then(|| cosine.clamp(-1.0, 1.0))
}

pub fn pexprb54s4_remainder_vector_geometry(
    d2: &[f64],
    d3: &[f64],
    d4: &[f64],
    norm_component_count: usize,
) -> CoreResult<Pexprb54s4RemainderVectorGeometry> {
    if d2.len() != d3.len() || d2.len() != d4.len() {
        return Err(CoreError::Dimension(
            "pexprb54s4 remainder-vector shape mismatch".into(),
        ));
    }
    if norm_component_count == 0 || norm_component_count > d2.len() {
        return Err(CoreError::Dimension(
            "invalid pexprb54s4 remainder-vector component contract".into(),
        ));
    }

    let state_dimension = d2.len();
    let d2 = &d2[..norm_component_count];
    let d3 = &d3[..norm_component_count];
    let d4 = &d4[..norm_component_count];
    let chi23 = normalized_direction_cosine(d2, d3);
    let chi34 = normalized_direction_cosine(d3, d4);
    let chi24 = normalized_direction_cosine(d2, d4);
    let q34_perp = chi34.map(|chi| (1.0 - chi * chi).max(0.0).sqrt());
    let delta_chi = chi34.zip(chi23).map(|(later, earlier)| later - earlier);

    Ok(Pexprb54s4RemainderVectorGeometry {
        state_dimension,
        norm_component_count,
        excluded_trailing_components: state_dimension - norm_component_count,
        chi23,
        chi34,
        chi24,
        q34_perp,
        delta_chi,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pexprb54s4QuadraticRemainderDrift {
    pub state_dimension: usize,
    pub norm_component_count: usize,
    pub excluded_trailing_components: usize,
    pub zeta23: Option<f64>,
    pub zeta34: Option<f64>,
    pub relative_drift: Option<f64>,
}

#[allow(clippy::too_many_arguments)]
fn pairwise_quadratic_drift_zeta(
    y: &[f64],
    ui: &[f64],
    uj: &[f64],
    di: &[f64],
    dj: &[f64],
    ci: f64,
    cj: f64,
    h: f64,
    norm_component_count: usize,
    atol: f64,
    rtol: f64,
) -> Option<f64> {
    let mut scaled_square_sum = 0.0;
    for k in 0..norm_component_count {
        let values = [y[k], ui[k], uj[k], di[k], dj[k]];
        if values.iter().any(|value| !value.is_finite()) {
            return None;
        }
        let scale = atol + rtol * y[k].abs().max(ui[k].abs()).max(uj[k].abs());
        if !(scale > 0.0 && scale.is_finite()) {
            return None;
        }
        let drift = dj[k] / (cj * cj) - di[k] / (ci * ci);
        let scaled = drift / scale;
        if !scaled.is_finite() {
            return None;
        }
        scaled_square_sum += scaled * scaled;
        if !scaled_square_sum.is_finite() {
            return None;
        }
    }
    let wrms = (scaled_square_sum / norm_component_count as f64).sqrt();
    let zeta = h.abs() * wrms;
    zeta.is_finite().then_some(zeta)
}

#[allow(clippy::too_many_arguments)]
pub fn pexprb54s4_quadratic_remainder_drift(
    y: &[f64],
    u2: &[f64],
    u3: &[f64],
    u4: &[f64],
    d2: &[f64],
    d3: &[f64],
    d4: &[f64],
    h: f64,
    norm_component_count: usize,
    atol: f64,
    rtol: f64,
) -> CoreResult<Pexprb54s4QuadraticRemainderDrift> {
    let n = y.len();
    if [u2.len(), u3.len(), u4.len(), d2.len(), d3.len(), d4.len()]
        .iter()
        .any(|&len| len != n)
    {
        return Err(CoreError::Dimension(
            "pexprb54s4 quadratic-remainder drift shape mismatch".into(),
        ));
    }
    if norm_component_count == 0 || norm_component_count > n {
        return Err(CoreError::Dimension(
            "invalid pexprb54s4 quadratic-remainder drift component contract".into(),
        ));
    }
    if !(h.is_finite() && atol > 0.0 && atol.is_finite() && rtol >= 0.0 && rtol.is_finite()) {
        return Err(CoreError::InvalidInput(
            "invalid pexprb54s4 quadratic-remainder drift scale contract".into(),
        ));
    }

    let tableau = pexprb54s4_tableau();
    let zeta23 = pairwise_quadratic_drift_zeta(
        y,
        u2,
        u3,
        d2,
        d3,
        tableau.c2,
        tableau.c3,
        h,
        norm_component_count,
        atol,
        rtol,
    );
    let zeta34 = pairwise_quadratic_drift_zeta(
        y,
        u3,
        u4,
        d3,
        d4,
        tableau.c3,
        tableau.c4,
        h,
        norm_component_count,
        atol,
        rtol,
    );
    let relative_drift = zeta23.zip(zeta34).and_then(|(earlier, later)| {
        let sum = earlier + later;
        if !sum.is_finite() {
            None
        } else if sum == 0.0 {
            Some(0.0)
        } else {
            let value = (later - earlier) / sum;
            value.is_finite().then(|| value.clamp(-1.0, 1.0))
        }
    });

    Ok(Pexprb54s4QuadraticRemainderDrift {
        state_dimension: n,
        norm_component_count,
        excluded_trailing_components: n - norm_component_count,
        zeta23,
        zeta34,
        relative_drift,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pexprb54s4Level2PrefixReport {
    pub method: String,
    pub t: f64,
    pub h: f64,
    pub logical_critical_depth: usize,
    pub level1_report: Pexprb54s4Level1PrefixReport,
    pub level2_fused_phi_reports: Vec<FusedPhiActionReport>,
    pub level2_incremental_work: WorkCounters,
    pub cumulative_work: WorkCounters,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage3_flow_defect: Option<EarlyFlowDefectTelemetry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage4_flow_defect: Option<EarlyFlowDefectTelemetry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remainder_vector_geometry: Option<Pexprb54s4RemainderVectorGeometry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quadratic_remainder_drift: Option<Pexprb54s4QuadraticRemainderDrift>,
}

pub struct Pexprb54s4Level2Prefix {
    y: Vec<f64>,
    h: f64,
    config: FusedPhiKrylovConfig,
    f0: Vec<f64>,
    operator: Arc<dyn LinearOperator>,
    d2: Vec<f64>,
    d3: Vec<f64>,
    d4: Vec<f64>,
    u2_action: FusedPhiActionReport,
    u3_action: FusedPhiActionReport,
    u4_action: FusedPhiActionReport,
    report: Pexprb54s4Level2PrefixReport,
}

impl Pexprb54s4Level2Prefix {
    pub fn report(&self) -> &Pexprb54s4Level2PrefixReport {
        &self.report
    }
}

const PEXPRB_PREFIX_JVP_BUDGET_EXHAUSTED: &str =
    "pexprb54s4 speculative prefix JVP budget exhausted";
const PEXPRB_CONTINUATION_JVP_BUDGET_EXHAUSTED: &str =
    "pexprb54s4 retained level-2 continuation JVP budget exhausted";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pexprb54s4BudgetExhaustedPrefixReport {
    pub jvp_cap: u64,
    pub used_jvp_vectors: u64,
    pub work: WorkCounters,
}

#[derive(Debug)]
pub struct Pexprb54s4FailedPrefixReport {
    pub error: CoreError,
    pub work: WorkCounters,
}

pub enum Pexprb54s4BudgetedLevel2PrefixOutcome {
    Complete(Box<Pexprb54s4Level2Prefix>),
    BudgetExhausted(Box<Pexprb54s4BudgetExhaustedPrefixReport>),
}

/// Accounted form of a budgeted level-2 prefix transaction.
///
/// Unlike the compatibility API, runtime failures are values so the caller can charge every
/// operation that completed before the error.
pub enum Pexprb54s4AccountedBudgetedLevel2PrefixOutcome {
    Complete(Box<Pexprb54s4Level2Prefix>),
    BudgetExhausted(Box<Pexprb54s4BudgetExhaustedPrefixReport>),
    Failed(Box<Pexprb54s4FailedPrefixReport>),
}

/// Exact work split for a retained-level-2 endpoint continuation.
///
/// Keeping all three ledgers explicit makes both successful and failed speculative continuations
/// auditable without relying on saturating subtraction alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pexprb54s4Level2ContinuationLedger {
    pub prefix_work: WorkCounters,
    pub continuation_work: WorkCounters,
    pub cumulative_work: WorkCounters,
}

/// Accounted result of consuming a retained level-2 prefix exactly once.
///
/// Endpoint failures are values rather than outer errors so all independently completed endpoint
/// work can be merged before the deterministic first error is surfaced. The outer [`CoreResult`]
/// of the accounted API remains reserved for execution/invariant failures.
#[derive(Debug)]
pub enum Pexprb54s4Level2ContinuationOutcome {
    Complete {
        report: Box<FusedExponentialStepReport>,
        ledger: Box<Pexprb54s4Level2ContinuationLedger>,
    },
    BudgetExhausted {
        jvp_cap: u64,
        used_jvp_vectors: u64,
        ledger: Box<Pexprb54s4Level2ContinuationLedger>,
    },
    Failed {
        error: CoreError,
        ledger: Box<Pexprb54s4Level2ContinuationLedger>,
    },
}

#[derive(Debug)]
struct Pexprb54s4JvpBudget {
    limit: u64,
    used: AtomicU64,
    exhaustion_message: &'static str,
}

impl Pexprb54s4JvpBudget {
    fn new(limit: u64) -> Self {
        Self::with_exhaustion_message(limit, PEXPRB_PREFIX_JVP_BUDGET_EXHAUSTED)
    }

    fn new_continuation(limit: u64) -> Self {
        Self::with_exhaustion_message(limit, PEXPRB_CONTINUATION_JVP_BUDGET_EXHAUSTED)
    }

    fn with_exhaustion_message(limit: u64, exhaustion_message: &'static str) -> Self {
        Self {
            limit,
            used: AtomicU64::new(0),
            exhaustion_message,
        }
    }

    fn used(&self) -> u64 {
        self.used.load(Ordering::Relaxed)
    }

    fn exhausted(&self) -> CoreError {
        CoreError::LinearSolve(self.exhaustion_message.into())
    }

    fn reserve(&self) -> CoreResult<()> {
        self.reserve_many(1)
    }

    fn reserve_many(&self, requested: u64) -> CoreResult<()> {
        if requested == 0 {
            return Ok(());
        }
        let mut current = self.used.load(Ordering::Relaxed);
        loop {
            let Some(next) = current.checked_add(requested) else {
                return Err(self.exhausted());
            };
            if next > self.limit {
                return Err(self.exhausted());
            }
            match self.used.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(()),
                Err(observed) => current = observed,
            }
        }
    }
}

struct Pexprb54s4BudgetedOperator {
    inner: Arc<dyn LinearOperator>,
    budget: Arc<Pexprb54s4JvpBudget>,
}

impl Pexprb54s4BudgetedOperator {
    fn new(inner: Arc<dyn LinearOperator>, budget: Arc<Pexprb54s4JvpBudget>) -> Self {
        Self { inner, budget }
    }
}

impl LinearOperator for Pexprb54s4BudgetedOperator {
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        self.budget.reserve()?;
        self.inner.apply(x, y)
    }

    fn apply_rows(&self, inputs: &[Vec<f64>], outputs: &mut [Vec<f64>]) -> CoreResult<()> {
        let dimension = self.dimension();
        if inputs.len() != outputs.len()
            || inputs.iter().any(|row| row.len() != dimension)
            || outputs.iter().any(|row| row.len() != dimension)
        {
            return Err(CoreError::Dimension(
                "linear-operator row batch shape mismatch".into(),
            ));
        }
        let requested = u64::try_from(inputs.len()).map_err(|_| {
            CoreError::InvalidInput("JVP row batch length exceeds u64 accounting".into())
        })?;
        self.budget.reserve_many(requested)?;
        self.inner.apply_rows(inputs, outputs)
    }

    fn explicit(&self) -> Option<&DenseMatrix> {
        self.inner.explicit()
    }

    fn token(&self) -> u64 {
        self.inner.token()
    }
}

fn is_pexprb_prefix_budget_exhaustion(error: &CoreError) -> bool {
    matches!(error, CoreError::LinearSolve(message) if message.contains(PEXPRB_PREFIX_JVP_BUDGET_EXHAUSTED))
}

fn is_pexprb_continuation_budget_exhaustion(error: &CoreError) -> bool {
    matches!(error, CoreError::LinearSolve(message) if message.contains(PEXPRB_CONTINUATION_JVP_BUDGET_EXHAUSTED))
}

fn budget_exhausted_outcome(
    cap: u64,
    budget: &Pexprb54s4JvpBudget,
    work: WorkCounters,
) -> Pexprb54s4AccountedBudgetedLevel2PrefixOutcome {
    debug_assert_eq!(budget.used(), work.jvp_vectors);
    Pexprb54s4AccountedBudgetedLevel2PrefixOutcome::BudgetExhausted(Box::new(
        Pexprb54s4BudgetExhaustedPrefixReport {
            jvp_cap: cap,
            used_jvp_vectors: budget.used(),
            work,
        },
    ))
}

fn failed_budgeted_prefix_outcome(
    error: CoreError,
    work: WorkCounters,
) -> Pexprb54s4AccountedBudgetedLevel2PrefixOutcome {
    Pexprb54s4AccountedBudgetedLevel2PrefixOutcome::Failed(Box::new(Pexprb54s4FailedPrefixReport {
        error,
        work,
    }))
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pexprb54s4Tableau {
    pub c2: f64,
    pub c3: f64,
    pub c4: f64,
    pub a32_phi3: f64,
    pub a42_phi3: f64,
    pub a43: f64,
    pub b3_phi3: f64,
    pub b3_phi4: f64,
    pub b4_phi3: f64,
    pub b4_phi4: f64,
    pub embedded_b2_phi3: f64,
    pub embedded_b2_phi4: f64,
    pub embedded_b3_phi3: f64,
    pub embedded_b3_phi4: f64,
    pub embedded_b4_phi3: f64,
    pub embedded_b4_phi4: f64,
}

pub fn pexprb54s4_tableau() -> Pexprb54s4Tableau {
    Pexprb54s4Tableau {
        c2: 1.0 / 4.0,
        c3: 1.0 / 2.0,
        c4: 9.0 / 10.0,
        a32_phi3: 4.0,
        a42_phi3: 2916.0 / 125.0,
        a43: 0.0,
        b3_phi3: 18.0,
        b3_phi4: -60.0,
        b4_phi3: -250.0 / 81.0,
        b4_phi4: 500.0 / 27.0,
        embedded_b2_phi3: 64.0,
        embedded_b2_phi4: -60.0,
        embedded_b3_phi3: -8.0,
        embedded_b3_phi4: -285.0 / 8.0,
        embedded_b4_phi3: 0.0,
        embedded_b4_phi4: 125.0 / 8.0,
    }
}

fn dot(x: &[f64], y: &[f64]) -> f64 {
    x.iter().zip(y).map(|(a, b)| a * b).sum()
}

fn axpy(alpha: f64, x: &[f64], y: &mut [f64]) {
    for (value, source) in y.iter_mut().zip(x) {
        *value += alpha * source;
    }
}

fn projected_action(
    basis: &[Vec<f64>],
    hessenberg: &[Vec<f64>],
    beta: f64,
    dimension: usize,
    scale: f64,
    phi_index: usize,
    counters: &mut WorkCounters,
) -> CoreResult<Vec<f64>> {
    let mut h = DenseMatrix::zeros(dimension, dimension);
    for i in 0..dimension {
        for j in 0..dimension {
            h[(i, j)] = hessenberg[i][j];
        }
    }
    let mut reduced_input = vec![0.0; dimension];
    reduced_input[0] = beta;
    let reduced = dense_phi_action(&h, scale, phi_index, &reduced_input)?;
    counters.phi_projected_exponentials += 1;
    counters.phi_dense_oracle_calls += 1;
    let mut out = vec![0.0; basis[0].len()];
    for (coefficient, vector) in reduced.iter().zip(&basis[..dimension]) {
        axpy(*coefficient, vector, &mut out);
    }
    Ok(out)
}

/// Matrix-free Arnoldi approximation of `phi_k(scale*A) v`.
///
/// The current G2 foundation uses full modified Gram--Schmidt with optional reorthogonalization
/// and a nested-dimension convergence diagnostic.  It is a counted correctness reference, not yet
/// KIOPS: incomplete orthogonalization, adaptive substepping and restart remain a later node.
pub fn krylov_phi_action(
    operator: Arc<dyn LinearOperator>,
    scale: f64,
    phi_index: usize,
    vector: &[f64],
    config: ExponentialKrylovConfig,
    counters: &mut WorkCounters,
) -> CoreResult<PhiActionReport> {
    let n = operator.dimension();
    if vector.len() != n {
        return Err(CoreError::Dimension(
            "Krylov phi-action vector shape mismatch".into(),
        ));
    }
    if !scale.is_finite() || !vector.iter().all(|value| value.is_finite()) {
        return Err(CoreError::NonFinite(
            "Krylov phi-action input contains NaN/Inf".into(),
        ));
    }
    let config = config.validate(n)?;
    counters.phi_actions += 1;
    let beta = safe_l2(vector);
    if beta == 0.0 {
        return Ok(PhiActionReport {
            phi_index,
            scale,
            krylov_dimension: 0,
            converged: true,
            happy_breakdown: true,
            error_estimate: 0.0,
            action_norm: 0.0,
            value: vec![0.0; n],
        });
    }

    let maximum = config.maximum_dimension;
    let mut basis: Vec<Vec<f64>> = Vec::with_capacity(maximum + 1);
    basis.push(vector.iter().map(|value| value / beta).collect());
    let mut hessenberg = vec![vec![0.0; maximum]; maximum + 1];
    let mut previous: Option<Vec<f64>> = None;
    let mut latest = vec![0.0; n];
    let mut latest_error = f64::INFINITY;
    let mut latest_dimension = 0;
    let mut happy_breakdown = false;
    let breakdown_tolerance = 64.0 * f64::EPSILON.sqrt();

    for column in 0..maximum {
        let mut work = vec![0.0; n];
        operator.apply(&basis[column], &mut work)?;
        counters.jvp_calls += 1;
        counters.jvp_vectors += 1;
        counters.phi_krylov_vectors += 1;

        for (row, basis_vector) in basis.iter().take(column + 1).enumerate() {
            let coefficient = dot(basis_vector, &work);
            hessenberg[row][column] += coefficient;
            axpy(-coefficient, basis_vector, &mut work);
            counters.orthogonalization_inner_products += 1;
            counters.orthogonalization_vector_updates += 1;
        }
        if config.reorthogonalize {
            for (row, basis_vector) in basis.iter().take(column + 1).enumerate() {
                let coefficient = dot(basis_vector, &work);
                hessenberg[row][column] += coefficient;
                axpy(-coefficient, basis_vector, &mut work);
                counters.orthogonalization_inner_products += 1;
                counters.orthogonalization_vector_updates += 1;
            }
        }
        let next_norm = safe_l2(&work);
        hessenberg[column + 1][column] = next_norm;
        let scale_norm = hessenberg
            .iter()
            .take(column + 1)
            .map(|row| row[column].abs())
            .fold(1.0, f64::max);
        happy_breakdown = next_norm <= breakdown_tolerance * scale_norm;
        if !happy_breakdown && column + 1 < maximum {
            basis.push(work.iter().map(|value| value / next_norm).collect());
        }

        let dimension = column + 1;
        let checkpoint = dimension >= config.minimum_dimension
            && ((dimension - config.minimum_dimension) % config.dimension_increment == 0
                || dimension == maximum
                || happy_breakdown);
        if checkpoint {
            let current = projected_action(
                &basis,
                &hessenberg,
                beta,
                dimension,
                scale,
                phi_index,
                counters,
            )?;
            latest_error = previous.as_ref().map_or(f64::INFINITY, |prior| {
                let difference: Vec<f64> = current.iter().zip(prior).map(|(a, b)| a - b).collect();
                safe_l2(&difference)
            });
            latest_dimension = dimension;
            latest = current.clone();
            let threshold =
                config.absolute_tolerance + config.relative_tolerance * safe_l2(&current).max(beta);
            let full_space = dimension == n;
            if happy_breakdown || full_space || latest_error <= threshold {
                return Ok(PhiActionReport {
                    phi_index,
                    scale,
                    krylov_dimension: dimension,
                    converged: true,
                    happy_breakdown,
                    error_estimate: if latest_error.is_finite() {
                        latest_error
                    } else {
                        0.0
                    },
                    action_norm: safe_l2(&current),
                    value: current,
                });
            }
            previous = Some(current);
        }
        if happy_breakdown {
            break;
        }
    }

    Ok(PhiActionReport {
        phi_index,
        scale,
        krylov_dimension: latest_dimension,
        converged: false,
        happy_breakdown,
        error_estimate: latest_error,
        action_norm: safe_l2(&latest),
        value: latest,
    })
}

fn projected_exponential_action_with_residual_estimate(
    basis: &[Vec<f64>],
    hessenberg: &[Vec<f64>],
    beta: f64,
    dimension: usize,
    scale: f64,
    counters: &mut WorkCounters,
) -> CoreResult<(Vec<f64>, f64)> {
    let mut h = DenseMatrix::zeros(dimension, dimension);
    for i in 0..dimension {
        for j in 0..dimension {
            h[(i, j)] = hessenberg[i][j];
        }
    }
    let mut reduced_input = vec![0.0; dimension];
    reduced_input[0] = beta;

    let reduced_exp = dense_phi_action(&h, scale, 0, &reduced_input)?;
    counters.phi_projected_exponentials += 1;
    counters.phi_dense_oracle_calls += 1;
    let mut out = vec![0.0; basis[0].len()];
    for (coefficient, vector) in reduced_exp.iter().zip(&basis[..dimension]) {
        axpy(*coefficient, vector, &mut out);
    }

    // First term of the Arnoldi error expansion used by KIOPS/phipm:
    // |tau h_{m+1,m} e_m^T phi_1(tau H_m) beta e_1|.
    // Unlike a nested-dimension difference this is available at the first checkpoint and is tied
    // directly to the Arnoldi projection residual.  It remains an estimator, not a rigorous upper
    // bound for arbitrary nonnormal operators.
    let reduced_phi1 = dense_phi_action(&h, scale, 1, &reduced_input)?;
    counters.phi_projected_exponentials += 1;
    counters.phi_dense_oracle_calls += 1;
    let h_next = hessenberg[dimension][dimension - 1].abs();
    let residual_error_estimate =
        (scale.abs() * h_next * reduced_phi1[dimension - 1].abs()).max(0.0);
    Ok((out, residual_error_estimate))
}

fn fused_orthogonalize(
    basis: &[Vec<f64>],
    column: usize,
    work: &mut [f64],
    hessenberg: &mut [Vec<f64>],
    mode: FusedOrthogonalization,
    counters: &mut WorkCounters,
) {
    let end = column + 1;
    let start = match mode {
        FusedOrthogonalization::FullMgs => 0,
        FusedOrthogonalization::Incomplete { length } => end.saturating_sub(length),
    };
    let passes = usize::from(matches!(mode, FusedOrthogonalization::FullMgs)) + 1;
    for _ in 0..passes {
        for row in start..end {
            let coefficient = dot(&basis[row], work);
            hessenberg[row][column] += coefficient;
            axpy(-coefficient, &basis[row], work);
            counters.orthogonalization_inner_products += 1;
            counters.orthogonalization_vector_updates += 1;
        }
    }
}

fn krylov_exponential_once(
    operator: Arc<dyn LinearOperator>,
    scale: f64,
    vector: &[f64],
    config: FusedPhiKrylovConfig,
    counters: &mut WorkCounters,
) -> CoreResult<(Vec<f64>, FusedPhiSubstepReport)> {
    let dimension = operator.dimension();
    if vector.len() != dimension {
        return Err(CoreError::Dimension(
            "fused Krylov exponential vector shape mismatch".into(),
        ));
    }
    let config = config.validate(dimension)?;
    let beta = safe_l2(vector);
    if beta == 0.0 {
        return Ok((
            vec![0.0; dimension],
            FusedPhiSubstepReport {
                substep_index: 0,
                krylov_dimension: 0,
                converged: true,
                happy_breakdown: true,
                error_estimate: 0.0,
                nested_difference_estimate: 0.0,
            },
        ));
    }

    let maximum = config.maximum_dimension;
    let mut basis = Vec::with_capacity(maximum + 1);
    basis.push(vector.iter().map(|value| value / beta).collect::<Vec<_>>());
    let mut hessenberg = vec![vec![0.0; maximum]; maximum + 1];
    let mut previous: Option<Vec<f64>> = None;
    let mut latest = vector.to_vec();
    let mut latest_residual_error = f64::INFINITY;
    let mut latest_nested_difference = f64::INFINITY;
    let mut latest_dimension = 0;
    let mut latest_breakdown = false;
    let breakdown_tolerance = 64.0 * f64::EPSILON.sqrt();

    for column in 0..maximum {
        let mut work = vec![0.0; dimension];
        operator.apply(&basis[column], &mut work)?;
        counters.jvp_calls += 1;
        counters.jvp_vectors += 1;
        counters.phi_krylov_vectors += 1;
        fused_orthogonalize(
            &basis,
            column,
            &mut work,
            &mut hessenberg,
            config.orthogonalization,
            counters,
        );
        let next_norm = safe_l2(&work);
        hessenberg[column + 1][column] = next_norm;
        let column_scale = hessenberg
            .iter()
            .take(column + 1)
            .map(|row| row[column].abs())
            .fold(1.0, f64::max);
        let happy_breakdown = next_norm <= breakdown_tolerance * column_scale;
        if !happy_breakdown && column + 1 < maximum {
            basis.push(work.iter().map(|value| value / next_norm).collect());
        }

        let krylov_dimension = column + 1;
        // A true invariant-subspace breakdown is a valid checkpoint even before the requested
        // minimum dimension; otherwise zero or affine combinations can burn the entire budget.
        let checkpoint = happy_breakdown
            || krylov_dimension == maximum
            || (krylov_dimension >= config.minimum_dimension
                && (krylov_dimension - config.minimum_dimension) % config.dimension_increment == 0);
        if checkpoint {
            let (current, residual_error_estimate) =
                projected_exponential_action_with_residual_estimate(
                    &basis,
                    &hessenberg,
                    beta,
                    krylov_dimension,
                    scale,
                    counters,
                )?;
            let nested_difference_estimate = previous.as_ref().map_or(f64::INFINITY, |prior| {
                let difference = current
                    .iter()
                    .zip(prior)
                    .map(|(a, b)| a - b)
                    .collect::<Vec<_>>();
                safe_l2(&difference)
            });
            latest_residual_error = residual_error_estimate;
            latest_nested_difference = nested_difference_estimate;
            latest_dimension = krylov_dimension;
            latest_breakdown = happy_breakdown;
            latest = current.clone();
            let threshold =
                config.absolute_tolerance + config.relative_tolerance * safe_l2(&current).max(beta);
            let full_space_exact = krylov_dimension == dimension
                && matches!(config.orthogonalization, FusedOrthogonalization::FullMgs);
            if happy_breakdown || full_space_exact || residual_error_estimate <= threshold {
                return Ok((
                    current,
                    FusedPhiSubstepReport {
                        substep_index: 0,
                        krylov_dimension,
                        converged: true,
                        happy_breakdown,
                        error_estimate: if happy_breakdown || full_space_exact {
                            0.0
                        } else {
                            residual_error_estimate
                        },
                        nested_difference_estimate,
                    },
                ));
            }
            previous = Some(current);
        }
        if happy_breakdown {
            break;
        }
    }
    Ok((
        latest,
        FusedPhiSubstepReport {
            substep_index: 0,
            krylov_dimension: latest_dimension,
            converged: false,
            happy_breakdown: latest_breakdown,
            error_estimate: latest_residual_error,
            nested_difference_estimate: latest_nested_difference,
        },
    ))
}

fn augmented_fused_operator(
    operator: Arc<dyn LinearOperator>,
    vectors: &[Vec<f64>],
) -> CoreResult<(Arc<dyn LinearOperator>, Vec<f64>, usize)> {
    if vectors.is_empty() {
        return Err(CoreError::InvalidInput(
            "fused phi action requires at least b0".into(),
        ));
    }
    let n = operator.dimension();
    if vectors.iter().any(|vector| vector.len() != n) {
        return Err(CoreError::Dimension(
            "fused phi action vector shape mismatch".into(),
        ));
    }
    let p = vectors.len() - 1;
    if p == 0 {
        return Ok((operator, vectors[0].clone(), n));
    }
    let owned = vectors.to_vec();
    let augmented_dimension = n + p;
    let augmented = Arc::new(ClosureOperator::new(
        augmented_dimension,
        move |input, output| {
            if input.len() != augmented_dimension || output.len() != augmented_dimension {
                return Err(CoreError::Dimension(
                    "augmented fused operator shape mismatch".into(),
                ));
            }
            operator.apply(&input[..n], &mut output[..n])?;
            for column in 0..p {
                let coefficient = input[n + column];
                if coefficient != 0.0 {
                    axpy(coefficient, &owned[p - column], &mut output[..n]);
                }
            }
            for row in 0..p {
                output[n + row] = if row + 1 < p { input[n + row + 1] } else { 0.0 };
            }
            Ok(())
        },
    )) as Arc<dyn LinearOperator>;
    let mut start = vec![0.0; augmented_dimension];
    start[..n].copy_from_slice(&vectors[0]);
    start[augmented_dimension - 1] = 1.0;
    Ok((augmented, start, n))
}

/// One augmented matrix-free Krylov process for a linear combination of phi functions.
///
/// The `vectors` input follows the scaled convention
/// `b0 + scale*phi1(scale*A)b1 + ... + scale^p*phi_p(scale*A)bp`.
pub fn fused_phi_action(
    operator: Arc<dyn LinearOperator>,
    scale: f64,
    vectors: &[Vec<f64>],
    config: FusedPhiKrylovConfig,
    counters: &mut WorkCounters,
) -> CoreResult<FusedPhiActionReport> {
    if !scale.is_finite()
        || vectors.is_empty()
        || !vectors
            .iter()
            .flat_map(|v| v.iter())
            .all(|value| value.is_finite())
    {
        return Err(CoreError::InvalidInput(
            "invalid fused phi-action input".into(),
        ));
    }
    counters.phi_actions += 1;
    let n = operator.dimension();
    if vectors.iter().any(|vector| vector.len() != n) {
        return Err(CoreError::Dimension(
            "fused phi-action vector shape mismatch".into(),
        ));
    }
    if vectors.iter().all(|vector| safe_l2(vector) == 0.0) {
        return Ok(FusedPhiActionReport {
            scale,
            highest_phi_index: vectors.len() - 1,
            substeps: 0,
            converged: true,
            maximum_krylov_dimension: 0,
            error_estimate: 0.0,
            nested_difference_estimate: 0.0,
            action_norm: 0.0,
            value: vec![0.0; n],
            substep_reports: Vec::new(),
        });
    }
    let (augmented, initial, physical_dimension) = augmented_fused_operator(operator, vectors)?;
    let config = config.validate(augmented.dimension())?;
    let mut substeps = 1usize;
    loop {
        let delta = scale / substeps as f64;
        let mut state = initial.clone();
        let mut reports = Vec::with_capacity(substeps);
        let mut total_error = 0.0;
        let mut total_nested_difference = 0.0;
        let mut maximum_dimension = 0;
        let mut completed = true;
        for index in 0..substeps {
            let (next, mut report) =
                krylov_exponential_once(augmented.clone(), delta, &state, config, counters)?;
            report.substep_index = index;
            maximum_dimension = maximum_dimension.max(report.krylov_dimension);
            if report.error_estimate.is_finite() {
                total_error += report.error_estimate;
            }
            if report.nested_difference_estimate.is_finite() {
                total_nested_difference += report.nested_difference_estimate;
            }
            completed &= report.converged;
            reports.push(report);
            state = next;
            if !completed {
                break;
            }
        }
        if completed {
            let value = state[..physical_dimension].to_vec();
            return Ok(FusedPhiActionReport {
                scale,
                highest_phi_index: vectors.len() - 1,
                substeps,
                converged: true,
                maximum_krylov_dimension: maximum_dimension,
                error_estimate: total_error,
                nested_difference_estimate: total_nested_difference,
                action_norm: safe_l2(&value),
                value,
                substep_reports: reports,
            });
        }
        if substeps >= config.maximum_substeps {
            let value = state[..physical_dimension].to_vec();
            return Ok(FusedPhiActionReport {
                scale,
                highest_phi_index: vectors.len() - 1,
                substeps,
                converged: false,
                maximum_krylov_dimension: maximum_dimension,
                error_estimate: total_error,
                nested_difference_estimate: total_nested_difference,
                action_norm: safe_l2(&value),
                value,
                substep_reports: reports,
            });
        }
        counters.phi_restarts += 1;
        substeps = (substeps * 2).min(config.maximum_substeps);
    }
}

/// Fused unscaled combination `sum coefficient*phi_k(scale*A)vector`.
pub fn fused_phi_linear_combination(
    operator: Arc<dyn LinearOperator>,
    scale: f64,
    terms: &[FusedPhiTerm<'_>],
    config: FusedPhiKrylovConfig,
    counters: &mut WorkCounters,
) -> CoreResult<FusedPhiActionReport> {
    if scale == 0.0 {
        return Err(CoreError::InvalidInput(
            "fused unscaled phi combination requires nonzero scale".into(),
        ));
    }
    let n = operator.dimension();
    let highest = terms.iter().map(|term| term.phi_index).max().unwrap_or(0);
    let mut vectors = vec![vec![0.0; n]; highest + 1];
    for term in terms {
        if term.vector.len() != n || !term.coefficient.is_finite() {
            return Err(CoreError::Dimension(
                "fused phi combination term shape mismatch".into(),
            ));
        }
        let divisor = scale.powi(term.phi_index as i32);
        if divisor == 0.0 || !divisor.is_finite() {
            return Err(CoreError::NonFinite(
                "fused phi combination scale power is invalid".into(),
            ));
        }
        axpy(
            term.coefficient / divisor,
            term.vector,
            &mut vectors[term.phi_index],
        );
    }
    fused_phi_action(operator, scale, &vectors, config, counters)
}

/// Advisory cost prediction from an actually computed fused-phi Arnoldi prefix.
///
/// Delayed Krylov convergence is possible for nonnormal operators, so this prediction never
/// replaces the residual-based completion certificate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FusedPhiPrefixPrediction {
    pub prefix_dimension: usize,
    pub predicted_total_dimension: usize,
    pub residual_error_estimate: f64,
    pub target_error: f64,
    pub observed_contraction: f64,
}

fn predict_krylov_dimension(
    history: &[f64],
    target: f64,
    current: usize,
    maximum: usize,
) -> (usize, f64) {
    let last = history.last().copied().unwrap_or(f64::INFINITY);
    if last <= target {
        return (current, 0.0);
    }
    let ratios = history
        .windows(2)
        .rev()
        .take(2)
        .filter_map(|pair| {
            let ratio = pair[1] / pair[0];
            (pair[0] > 0.0 && ratio.is_finite() && ratio > 0.0).then_some(ratio)
        })
        .collect::<Vec<_>>();
    if ratios.is_empty() {
        return (maximum, 1.0);
    }
    let observed = ratios
        .into_iter()
        .fold(0.0_f64, f64::max)
        .clamp(1.0e-3, 0.999);
    if observed >= 0.999 || !last.is_finite() || !target.is_finite() || target <= 0.0 {
        return (maximum, observed);
    }
    let remaining = ((target / last).ln() / observed.ln()).ceil().max(0.0) as usize;
    (current.saturating_add(remaining).min(maximum), observed)
}

/// Full-MGS fused-phi Krylov session whose first Arnoldi vectors can be retained and extended.
///
/// G4-S4 intentionally restricts this research kernel to one Krylov substep and full MGS.  The
/// existing adaptive-substep/IOP implementation remains the production research candidate until
/// prefix reuse passes its own cost and parity gates.
pub struct FusedPhiPrefixSession {
    operator: Arc<dyn LinearOperator>,
    scale: f64,
    highest_phi_index: usize,
    physical_dimension: usize,
    config: FusedPhiKrylovConfig,
    beta: f64,
    basis: Vec<Vec<f64>>,
    hessenberg: Vec<Vec<f64>>,
    current_dimension: usize,
    residual_history: Vec<f64>,
    latest_value_augmented: Vec<f64>,
    latest_residual_error: f64,
    latest_nested_difference: f64,
    previous_projected: Option<Vec<f64>>,
    converged: bool,
    happy_breakdown: bool,
}

impl FusedPhiPrefixSession {
    pub fn begin(
        operator: Arc<dyn LinearOperator>,
        scale: f64,
        vectors: &[Vec<f64>],
        config: FusedPhiKrylovConfig,
        prefix_dimension: usize,
        counters: &mut WorkCounters,
    ) -> CoreResult<Self> {
        if !scale.is_finite()
            || vectors.is_empty()
            || !vectors
                .iter()
                .flat_map(|vector| vector.iter())
                .all(|value| value.is_finite())
        {
            return Err(CoreError::InvalidInput(
                "invalid fused-phi prefix input".into(),
            ));
        }
        if !matches!(config.orthogonalization, FusedOrthogonalization::FullMgs)
            || config.maximum_substeps != 1
        {
            return Err(CoreError::InvalidInput(
                "G4-S4 fused-phi prefix requires full MGS and exactly one substep".into(),
            ));
        }
        let n = operator.dimension();
        if vectors.iter().any(|vector| vector.len() != n) {
            return Err(CoreError::Dimension(
                "fused-phi prefix vector shape mismatch".into(),
            ));
        }
        counters.phi_actions += 1;
        if vectors.iter().all(|vector| safe_l2(vector) == 0.0) {
            return Ok(Self {
                operator,
                scale,
                highest_phi_index: vectors.len() - 1,
                physical_dimension: n,
                config,
                beta: 0.0,
                basis: Vec::new(),
                hessenberg: Vec::new(),
                current_dimension: 0,
                residual_history: vec![0.0],
                latest_value_augmented: vec![0.0; n],
                latest_residual_error: 0.0,
                latest_nested_difference: 0.0,
                previous_projected: None,
                converged: true,
                happy_breakdown: true,
            });
        }
        let highest_phi_index = vectors.len() - 1;
        let (augmented, initial, physical_dimension) = augmented_fused_operator(operator, vectors)?;
        let config = config.validate(augmented.dimension())?;
        let beta = safe_l2(&initial);
        if !(beta > f64::MIN_POSITIVE && beta.is_finite()) {
            return Err(CoreError::LinearSolve(
                "fused-phi prefix initial-vector breakdown".into(),
            ));
        }
        let maximum = config.maximum_dimension;
        let mut session = Self {
            operator: augmented,
            scale,
            highest_phi_index,
            physical_dimension,
            config,
            beta,
            basis: vec![initial.iter().map(|value| value / beta).collect()],
            hessenberg: vec![vec![0.0; maximum]; maximum + 1],
            current_dimension: 0,
            residual_history: Vec::new(),
            latest_value_augmented: initial,
            latest_residual_error: f64::INFINITY,
            latest_nested_difference: f64::INFINITY,
            previous_projected: None,
            converged: false,
            happy_breakdown: false,
        };
        for _ in 0..prefix_dimension.min(maximum) {
            if session.converged || session.happy_breakdown {
                break;
            }
            session.extend_one(counters)?;
        }
        Ok(session)
    }

    fn extend_one(&mut self, counters: &mut WorkCounters) -> CoreResult<()> {
        let column = self.current_dimension;
        let maximum = self.config.maximum_dimension;
        if column >= maximum {
            return Ok(());
        }
        if column >= self.basis.len() {
            return Err(CoreError::LinearSolve(
                "fused-phi prefix basis exhausted before continuation".into(),
            ));
        }
        let augmented_dimension = self.operator.dimension();
        let mut work = vec![0.0; augmented_dimension];
        self.operator.apply(&self.basis[column], &mut work)?;
        counters.jvp_calls += 1;
        counters.jvp_vectors += 1;
        counters.phi_krylov_vectors += 1;
        fused_orthogonalize(
            &self.basis,
            column,
            &mut work,
            &mut self.hessenberg,
            FusedOrthogonalization::FullMgs,
            counters,
        );
        let next_norm = safe_l2(&work);
        self.hessenberg[column + 1][column] = next_norm;
        let column_scale = self
            .hessenberg
            .iter()
            .take(column + 1)
            .map(|row| row[column].abs())
            .fold(1.0, f64::max);
        let breakdown_tolerance = 64.0 * f64::EPSILON.sqrt();
        self.happy_breakdown = next_norm <= breakdown_tolerance * column_scale;
        if !self.happy_breakdown && column + 1 < maximum {
            self.basis
                .push(work.iter().map(|value| value / next_norm).collect());
        }
        self.current_dimension += 1;
        let (current, residual_error) = projected_exponential_action_with_residual_estimate(
            &self.basis,
            &self.hessenberg,
            self.beta,
            self.current_dimension,
            self.scale,
            counters,
        )?;
        let nested = self
            .previous_projected
            .as_ref()
            .map_or(f64::INFINITY, |previous| {
                let difference = current
                    .iter()
                    .zip(previous)
                    .map(|(a, b)| a - b)
                    .collect::<Vec<_>>();
                safe_l2(&difference)
            });
        self.latest_value_augmented = current.clone();
        self.latest_residual_error = residual_error;
        self.latest_nested_difference = nested;
        self.previous_projected = Some(current);
        self.residual_history.push(residual_error);
        let threshold = self.config.absolute_tolerance
            + self.config.relative_tolerance * safe_l2(&self.latest_value_augmented).max(self.beta);
        let full_space = self.current_dimension == augmented_dimension;
        self.converged = self.happy_breakdown || full_space || residual_error <= threshold;
        Ok(())
    }

    pub fn prediction(&self) -> FusedPhiPrefixPrediction {
        let target = self.config.absolute_tolerance
            + self.config.relative_tolerance * safe_l2(&self.latest_value_augmented).max(self.beta);
        let (predicted, contraction) = predict_krylov_dimension(
            &self.residual_history,
            target,
            self.current_dimension,
            self.config.maximum_dimension,
        );
        FusedPhiPrefixPrediction {
            prefix_dimension: self.current_dimension,
            predicted_total_dimension: predicted,
            residual_error_estimate: self.latest_residual_error,
            target_error: target,
            observed_contraction: contraction,
        }
    }

    pub fn finish(mut self, counters: &mut WorkCounters) -> CoreResult<FusedPhiActionReport> {
        while !self.converged && self.current_dimension < self.config.maximum_dimension {
            self.extend_one(counters)?;
            if self.happy_breakdown && !self.converged {
                break;
            }
        }
        let physical = self.latest_value_augmented[..self.physical_dimension].to_vec();
        Ok(FusedPhiActionReport {
            scale: self.scale,
            highest_phi_index: self.highest_phi_index,
            substeps: usize::from(self.current_dimension > 0),
            converged: self.converged,
            maximum_krylov_dimension: self.current_dimension,
            error_estimate: if self.converged && self.happy_breakdown {
                0.0
            } else {
                self.latest_residual_error
            },
            nested_difference_estimate: self.latest_nested_difference,
            action_norm: safe_l2(&physical),
            value: physical,
            substep_reports: if self.current_dimension == 0 {
                Vec::new()
            } else {
                vec![FusedPhiSubstepReport {
                    substep_index: 0,
                    krylov_dimension: self.current_dimension,
                    converged: self.converged,
                    happy_breakdown: self.happy_breakdown,
                    error_estimate: self.latest_residual_error,
                    nested_difference_estimate: self.latest_nested_difference,
                }]
            },
        })
    }
}

pub fn fused_phi_action_incremental(
    operator: Arc<dyn LinearOperator>,
    scale: f64,
    vectors: &[Vec<f64>],
    config: FusedPhiKrylovConfig,
    counters: &mut WorkCounters,
) -> CoreResult<FusedPhiActionReport> {
    FusedPhiPrefixSession::begin(operator, scale, vectors, config, 0, counters)?.finish(counters)
}

fn validate_problem(problem: &OdeProblem, y: &[f64]) -> CoreResult<()> {
    if y.len() != problem.dimension {
        return Err(CoreError::Dimension(
            "exponential step state shape mismatch".into(),
        ));
    }
    if !problem.autonomous {
        return Err(CoreError::InvalidInput(
            "G2 foundation currently supports autonomous problems only".into(),
        ));
    }
    if problem.mass_matrix.is_some() {
        return Err(CoreError::InvalidInput(
            "G2 foundation currently supports identity mass only".into(),
        ));
    }
    Ok(())
}

fn require_action(report: PhiActionReport) -> CoreResult<PhiActionReport> {
    if report.converged {
        Ok(report)
    } else {
        Err(CoreError::LinearSolve(format!(
            "phi_{} Krylov action failed to converge at dimension {} (estimate {})",
            report.phi_index, report.krylov_dimension, report.error_estimate
        )))
    }
}

fn phi_combination(
    operator: Arc<dyn LinearOperator>,
    scale: f64,
    terms: &[(f64, usize, &[f64])],
    config: ExponentialKrylovConfig,
    counters: &mut WorkCounters,
) -> CoreResult<(Vec<f64>, Vec<PhiActionReport>)> {
    let n = operator.dimension();
    let mut value = vec![0.0; n];
    let mut reports = Vec::with_capacity(terms.len());
    for &(coefficient, phi_index, vector) in terms {
        if coefficient == 0.0 {
            continue;
        }
        let report = require_action(krylov_phi_action(
            operator.clone(),
            scale,
            phi_index,
            vector,
            config,
            counters,
        )?)?;
        axpy(coefficient, &report.value, &mut value);
        reports.push(report);
    }
    Ok((value, reports))
}

fn nonlinear_remainder(
    problem: &OdeProblem,
    operator: &dyn LinearOperator,
    t: f64,
    y: &[f64],
    f0: &[f64],
    stage: &[f64],
    counters: &mut WorkCounters,
) -> CoreResult<Vec<f64>> {
    let f_stage = problem.eval_rhs(t, stage, counters)?;
    let increment: Vec<f64> = stage.iter().zip(y).map(|(a, b)| a - b).collect();
    let mut linear = vec![0.0; increment.len()];
    operator.apply(&increment, &mut linear)?;
    counters.jvp_calls += 1;
    counters.jvp_vectors += 1;
    Ok(f_stage
        .iter()
        .zip(f0)
        .zip(linear)
        .map(|((stage_value, base_value), linear_value)| stage_value - base_value - linear_value)
        .collect())
}

fn base_stage(y: &[f64], h: f64, c: f64, phi1: &[f64]) -> Vec<f64> {
    y.iter()
        .zip(phi1)
        .map(|(state, action)| state + c * h * action)
        .collect()
}

#[derive(Clone, Copy, Debug)]
struct EarlyFlowDefectToleranceScale {
    atol: f64,
    rtol: f64,
}

fn early_flow_defect_telemetry(
    mode: EarlyFlowDefectTelemetryMode,
    stage_fraction: f64,
    h: f64,
    y: &[f64],
    stage: &[f64],
    nonlinear_remainder: &[f64],
    tolerance_scale: Option<EarlyFlowDefectToleranceScale>,
) -> CoreResult<Option<EarlyFlowDefectTelemetry>> {
    let EarlyFlowDefectTelemetryMode::ReadOnly {
        norm_component_count,
    } = mode
    else {
        return Ok(None);
    };
    if y.len() != stage.len() || y.len() != nonlinear_remainder.len() {
        return Err(CoreError::Dimension(
            "early-flow-defect telemetry shape mismatch".into(),
        ));
    }
    if norm_component_count == 0 || norm_component_count > y.len() {
        return Err(CoreError::InvalidInput(format!(
            "early-flow-defect norm component count {norm_component_count} is outside 1..={}",
            y.len()
        )));
    }
    let increment = stage[..norm_component_count]
        .iter()
        .zip(&y[..norm_component_count])
        .map(|(stage_value, base_value)| stage_value - base_value)
        .collect::<Vec<_>>();
    let stage_increment_l2 = safe_l2(&increment);
    let nonlinear_remainder_l2 = safe_l2(&nonlinear_remainder[..norm_component_count]);
    let zero_increment = stage_increment_l2 == 0.0;
    let degenerate_nonzero_remainder = zero_increment && nonlinear_remainder_l2 != 0.0;
    let (normalized_defect, scalar_normalizations, nonfinite_normalization) = if zero_increment {
        ((!degenerate_nonzero_remainder).then_some(0.0), 0, false)
    } else {
        let value = h.abs() * nonlinear_remainder_l2 / stage_increment_l2;
        (value.is_finite().then_some(value), 1, !value.is_finite())
    };
    let (
        tolerance_scaled_defect_wrms,
        tolerance_scale_atol,
        tolerance_scale_rtol,
        tolerance_scaled_nonfinite,
        component_scale_evaluations,
        wrms_norm_evaluations,
    ) = if let Some(scale_contract) = tolerance_scale {
        if !(scale_contract.atol >= 0.0 && scale_contract.atol.is_finite()) {
            return Err(CoreError::InvalidInput(
                "early-flow-defect atol must be finite and nonnegative".into(),
            ));
        }
        let scales = error_scale(
            &y[..norm_component_count],
            &stage[..norm_component_count],
            &[scale_contract.atol],
            scale_contract.rtol,
        )?;
        let scaled_remainder = nonlinear_remainder[..norm_component_count]
            .iter()
            .map(|value| h.abs() * value)
            .collect::<Vec<_>>();
        let value = wrms(&scaled_remainder, &scales)?;
        (
            value.is_finite().then_some(value),
            Some(scale_contract.atol),
            Some(scale_contract.rtol),
            Some(!value.is_finite()),
            norm_component_count as u64,
            1,
        )
    } else {
        (None, None, None, None, 0, 0)
    };
    Ok(Some(EarlyFlowDefectTelemetry {
        stage_fraction,
        state_dimension: y.len(),
        norm_component_count,
        excluded_trailing_components: y.len() - norm_component_count,
        abs_h: h.abs(),
        stage_increment_l2,
        nonlinear_remainder_l2,
        normalized_defect,
        tolerance_scaled_defect_wrms,
        tolerance_scale_atol,
        tolerance_scale_rtol,
        tolerance_scaled_nonfinite,
        zero_increment,
        degenerate_nonzero_remainder,
        nonfinite_normalization,
        native_partial_t_sampled: false,
        diagnostic_work: EarlyFlowDefectDiagnosticWork {
            conceptual_vector_differences: 1,
            l2_norm_evaluations: 2,
            scalar_normalizations,
            component_scale_evaluations,
            wrms_norm_evaluations,
            ..EarlyFlowDefectDiagnosticWork::default()
        },
    }))
}

pub fn exprb2_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: ExponentialKrylovConfig,
) -> CoreResult<ExponentialStepReport> {
    validate_problem(problem, y)?;
    let mut counters = WorkCounters::default();
    let f0 = problem.eval_rhs(t, y, &mut counters)?;
    let operator = problem.linearize_matrix_free(t, y)?;
    let phi1 = require_action(krylov_phi_action(
        operator,
        h,
        1,
        &f0,
        config,
        &mut counters,
    )?)?;
    let y_new: Vec<f64> = y
        .iter()
        .zip(&phi1.value)
        .map(|(state, action)| state + h * action)
        .collect();
    Ok(ExponentialStepReport {
        method: "exprb2".into(),
        y_new,
        y_embedded: None,
        error_estimate: None,
        logical_critical_depth: 1,
        phi_reports: vec![phi1],
        work: counters,
    })
}

pub fn exprb43_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: ExponentialKrylovConfig,
) -> CoreResult<ExponentialStepReport> {
    validate_problem(problem, y)?;
    let mut counters = WorkCounters::default();
    let f0 = problem.eval_rhs(t, y, &mut counters)?;
    let operator = problem.linearize_matrix_free(t, y)?;
    let phi_half = require_action(krylov_phi_action(
        operator.clone(),
        0.5 * h,
        1,
        &f0,
        config,
        &mut counters,
    )?)?;
    let u2 = base_stage(y, h, 0.5, &phi_half.value);
    let d2 = nonlinear_remainder(problem, operator.as_ref(), t, y, &f0, &u2, &mut counters)?;

    let phi_full = require_action(krylov_phi_action(
        operator.clone(),
        h,
        1,
        &f0,
        config,
        &mut counters,
    )?)?;
    let a32 = require_action(krylov_phi_action(
        operator.clone(),
        h,
        1,
        &d2,
        config,
        &mut counters,
    )?)?;
    let mut u3 = base_stage(y, h, 1.0, &phi_full.value);
    axpy(h, &a32.value, &mut u3);
    let d3 = nonlinear_remainder(problem, operator.as_ref(), t, y, &f0, &u3, &mut counters)?;

    let (main_correction, mut main_reports) = phi_combination(
        operator.clone(),
        h,
        &[
            (16.0, 3, &d2),
            (-48.0, 4, &d2),
            (-2.0, 3, &d3),
            (12.0, 4, &d3),
        ],
        config,
        &mut counters,
    )?;
    let (embedded_correction, mut embedded_reports) = phi_combination(
        operator,
        h,
        &[(16.0, 3, &d2), (-2.0, 3, &d3)],
        config,
        &mut counters,
    )?;
    let mut y_new = base_stage(y, h, 1.0, &phi_full.value);
    axpy(h, &main_correction, &mut y_new);
    let mut y_embedded = base_stage(y, h, 1.0, &phi_full.value);
    axpy(h, &embedded_correction, &mut y_embedded);
    let error_estimate: Vec<f64> = y_new.iter().zip(&y_embedded).map(|(a, b)| a - b).collect();
    let mut phi_reports = vec![phi_half, phi_full, a32];
    phi_reports.append(&mut main_reports);
    phi_reports.append(&mut embedded_reports);
    Ok(ExponentialStepReport {
        method: "exprb43".into(),
        y_new,
        y_embedded: Some(y_embedded),
        error_estimate: Some(error_estimate),
        logical_critical_depth: 3,
        phi_reports,
        work: counters,
    })
}

pub fn pexprb54s4_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: ExponentialKrylovConfig,
    execution: &ParallelExecution,
) -> CoreResult<ExponentialStepReport> {
    validate_problem(problem, y)?;
    let tableau = pexprb54s4_tableau();
    let mut counters = WorkCounters::default();
    let f0 = problem.eval_rhs(t, y, &mut counters)?;
    let operator = problem.linearize_matrix_free(t, y)?;

    // Dependency level 1: the U2 path and the endpoint base action are independent.
    // Running both inside the same local execution context makes the published three-level
    // dependency graph an actual code-path property rather than metadata only.
    let level_one_ids = [0_usize, 1_usize];
    let level_one = execution.map_ordered(&level_one_ids, |task_id| {
        let mut local = WorkCounters::default();
        if *task_id == 0 {
            let phi_c2 = require_action(krylov_phi_action(
                operator.clone(),
                tableau.c2 * h,
                1,
                &f0,
                config,
                &mut local,
            )?)?;
            let u2 = base_stage(y, h, tableau.c2, &phi_c2.value);
            let d2 = nonlinear_remainder(problem, operator.as_ref(), t, y, &f0, &u2, &mut local)?;
            Ok((Some(d2), Some(phi_c2), None, local))
        } else {
            let phi_full = require_action(krylov_phi_action(
                operator.clone(),
                h,
                1,
                &f0,
                config,
                &mut local,
            )?)?;
            Ok((None, None, Some(phi_full), local))
        }
    })?;
    for result in &level_one {
        counters.accumulate(result.3);
    }
    let d2 = level_one[0]
        .0
        .clone()
        .ok_or_else(|| CoreError::InvalidInput("pexprb54s4 level-one U2 result missing".into()))?;
    let phi_c2 = level_one[0]
        .1
        .clone()
        .ok_or_else(|| CoreError::InvalidInput("pexprb54s4 level-one c2 action missing".into()))?;
    let phi_full = level_one[1].2.clone().ok_or_else(|| {
        CoreError::InvalidInput("pexprb54s4 level-one endpoint action missing".into())
    })?;

    let stage_ids = [3_usize, 4_usize];
    let stage_results = execution.map_ordered(&stage_ids, |stage_id| {
        let mut local = WorkCounters::default();
        let (c, coefficient) = if *stage_id == 3 {
            (tableau.c3, tableau.a32_phi3)
        } else {
            (tableau.c4, tableau.a42_phi3)
        };
        let phi1 = require_action(krylov_phi_action(
            operator.clone(),
            c * h,
            1,
            &f0,
            config,
            &mut local,
        )?)?;
        let phi3 = require_action(krylov_phi_action(
            operator.clone(),
            c * h,
            3,
            &d2,
            config,
            &mut local,
        )?)?;
        let mut stage = base_stage(y, h, c, &phi1.value);
        axpy(h * coefficient, &phi3.value, &mut stage);
        let remainder =
            nonlinear_remainder(problem, operator.as_ref(), t, y, &f0, &stage, &mut local)?;
        Ok((stage, remainder, vec![phi1, phi3], local))
    })?;
    let d3 = stage_results[0].1.clone();
    let d4 = stage_results[1].1.clone();
    let mut phi_reports = vec![phi_c2];
    for result in &stage_results {
        counters.accumulate(result.3);
        phi_reports.extend(result.2.clone());
    }

    let endpoint_ids = [0_usize, 1_usize];
    let endpoint_results = execution.map_ordered(&endpoint_ids, |endpoint_id| {
        let mut local = WorkCounters::default();
        let (correction, reports) = if *endpoint_id == 0 {
            phi_combination(
                operator.clone(),
                h,
                &[
                    (tableau.b3_phi3, 3, &d3),
                    (tableau.b3_phi4, 4, &d3),
                    (tableau.b4_phi3, 3, &d4),
                    (tableau.b4_phi4, 4, &d4),
                ],
                config,
                &mut local,
            )?
        } else {
            phi_combination(
                operator.clone(),
                h,
                &[
                    (tableau.embedded_b2_phi3, 3, &d2),
                    (tableau.embedded_b2_phi4, 4, &d2),
                    (tableau.embedded_b3_phi3, 3, &d3),
                    (tableau.embedded_b3_phi4, 4, &d3),
                    (tableau.embedded_b4_phi3, 3, &d4),
                    (tableau.embedded_b4_phi4, 4, &d4),
                ],
                config,
                &mut local,
            )?
        };
        Ok((correction, reports, local))
    })?;
    for result in &endpoint_results {
        counters.accumulate(result.2);
        phi_reports.extend(result.1.clone());
    }
    phi_reports.push(phi_full.clone());
    let mut y_new = base_stage(y, h, 1.0, &phi_full.value);
    axpy(h, &endpoint_results[0].0, &mut y_new);
    let mut y_embedded = base_stage(y, h, 1.0, &phi_full.value);
    axpy(h, &endpoint_results[1].0, &mut y_embedded);
    let error_estimate: Vec<f64> = y_new.iter().zip(&y_embedded).map(|(a, b)| a - b).collect();

    Ok(ExponentialStepReport {
        method: "pexprb54s4".into(),
        y_new,
        y_embedded: Some(y_embedded),
        error_estimate: Some(error_estimate),
        logical_critical_depth: 3,
        phi_reports,
        work: counters,
    })
}

fn require_fused(report: FusedPhiActionReport) -> CoreResult<FusedPhiActionReport> {
    if report.converged {
        Ok(report)
    } else {
        Err(CoreError::LinearSolve(format!(
            "fused phi action failed after {} substeps at dimension {} (estimate {})",
            report.substeps, report.maximum_krylov_dimension, report.error_estimate
        )))
    }
}

fn add_scaled_state(y: &[f64], h: f64, action: &[f64]) -> Vec<f64> {
    y.iter()
        .zip(action)
        .map(|(state, delta)| state + h * delta)
        .collect()
}

pub fn exprb2_fused_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: FusedPhiKrylovConfig,
) -> CoreResult<FusedExponentialStepReport> {
    validate_problem(problem, y)?;
    let mut work = WorkCounters::default();
    let f0 = problem.eval_rhs(t, y, &mut work)?;
    let operator = problem.linearize_matrix_free(t, y)?;
    let action = require_fused(fused_phi_linear_combination(
        operator,
        h,
        &[FusedPhiTerm {
            coefficient: 1.0,
            phi_index: 1,
            vector: &f0,
        }],
        config,
        &mut work,
    )?)?;
    Ok(FusedExponentialStepReport {
        method: "exprb2-fused".into(),
        y_new: add_scaled_state(y, h, &action.value),
        y_embedded: None,
        error_estimate: None,
        logical_critical_depth: 1,
        fused_phi_reports: vec![action],
        work,
        early_flow_defect: None,
    })
}

pub fn exprb43_fused_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: FusedPhiKrylovConfig,
    execution: &ParallelExecution,
) -> CoreResult<FusedExponentialStepReport> {
    validate_problem(problem, y)?;
    let mut work = WorkCounters::default();
    let f0 = problem.eval_rhs(t, y, &mut work)?;
    let operator = problem.linearize_matrix_free(t, y)?;

    let u2_action = require_fused(fused_phi_linear_combination(
        operator.clone(),
        0.5 * h,
        &[FusedPhiTerm {
            coefficient: 0.5,
            phi_index: 1,
            vector: &f0,
        }],
        config,
        &mut work,
    )?)?;
    let u2 = add_scaled_state(y, h, &u2_action.value);
    let d2 = nonlinear_remainder(problem, operator.as_ref(), t, y, &f0, &u2, &mut work)?;

    let u3_action = require_fused(fused_phi_linear_combination(
        operator.clone(),
        h,
        &[
            FusedPhiTerm {
                coefficient: 1.0,
                phi_index: 1,
                vector: &f0,
            },
            FusedPhiTerm {
                coefficient: 1.0,
                phi_index: 1,
                vector: &d2,
            },
        ],
        config,
        &mut work,
    )?)?;
    let u3 = add_scaled_state(y, h, &u3_action.value);
    let d3 = nonlinear_remainder(problem, operator.as_ref(), t, y, &f0, &u3, &mut work)?;

    let endpoint_ids = [0usize, 1usize];
    let endpoints = execution.map_ordered(&endpoint_ids, |id| {
        let mut local = WorkCounters::default();
        let terms = if *id == 0 {
            vec![
                FusedPhiTerm {
                    coefficient: 1.0,
                    phi_index: 1,
                    vector: &f0,
                },
                FusedPhiTerm {
                    coefficient: 16.0,
                    phi_index: 3,
                    vector: &d2,
                },
                FusedPhiTerm {
                    coefficient: -48.0,
                    phi_index: 4,
                    vector: &d2,
                },
                FusedPhiTerm {
                    coefficient: -2.0,
                    phi_index: 3,
                    vector: &d3,
                },
                FusedPhiTerm {
                    coefficient: 12.0,
                    phi_index: 4,
                    vector: &d3,
                },
            ]
        } else {
            vec![
                FusedPhiTerm {
                    coefficient: 1.0,
                    phi_index: 1,
                    vector: &f0,
                },
                FusedPhiTerm {
                    coefficient: 16.0,
                    phi_index: 3,
                    vector: &d2,
                },
                FusedPhiTerm {
                    coefficient: -2.0,
                    phi_index: 3,
                    vector: &d3,
                },
            ]
        };
        let action = require_fused(fused_phi_linear_combination(
            operator.clone(),
            h,
            &terms,
            config,
            &mut local,
        )?)?;
        Ok((action, local))
    })?;
    for (_, local) in &endpoints {
        work.accumulate(*local);
    }
    let y_new = add_scaled_state(y, h, &endpoints[0].0.value);
    let y_embedded = add_scaled_state(y, h, &endpoints[1].0.value);
    let error_estimate = y_new.iter().zip(&y_embedded).map(|(a, b)| a - b).collect();
    Ok(FusedExponentialStepReport {
        method: "exprb43-fused".into(),
        y_new,
        y_embedded: Some(y_embedded),
        error_estimate: Some(error_estimate),
        logical_critical_depth: 3,
        fused_phi_reports: vec![
            u2_action,
            u3_action,
            endpoints[0].0.clone(),
            endpoints[1].0.clone(),
        ],
        work,
        early_flow_defect: None,
    })
}

pub fn pexprb54s4_fused_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: FusedPhiKrylovConfig,
    execution: &ParallelExecution,
) -> CoreResult<FusedExponentialStepReport> {
    pexprb54s4_fused_step_with_telemetry_mode(
        problem,
        t,
        y,
        h,
        config,
        execution,
        EarlyFlowDefectTelemetryMode::Disabled,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn pexprb54s4_fused_step_with_telemetry_mode(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: FusedPhiKrylovConfig,
    execution: &ParallelExecution,
    telemetry_mode: EarlyFlowDefectTelemetryMode,
) -> CoreResult<FusedExponentialStepReport> {
    pexprb54s4_fused_step_with_telemetry_config(
        problem,
        t,
        y,
        h,
        config,
        execution,
        telemetry_mode,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn pexprb54s4_fused_step_with_tolerance_scaled_telemetry(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: FusedPhiKrylovConfig,
    execution: &ParallelExecution,
    norm_component_count: usize,
    atol: f64,
    rtol: f64,
) -> CoreResult<FusedExponentialStepReport> {
    pexprb54s4_fused_step_with_telemetry_config(
        problem,
        t,
        y,
        h,
        config,
        execution,
        EarlyFlowDefectTelemetryMode::ReadOnly {
            norm_component_count,
        },
        Some(EarlyFlowDefectToleranceScale { atol, rtol }),
    )
}

#[allow(clippy::too_many_arguments)]
fn pexprb54s4_level1_prefix_with_telemetry_config(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: FusedPhiKrylovConfig,
    telemetry_mode: EarlyFlowDefectTelemetryMode,
    tolerance_scale: Option<EarlyFlowDefectToleranceScale>,
) -> CoreResult<Pexprb54s4Level1Prefix> {
    validate_problem(problem, y)?;
    if let EarlyFlowDefectTelemetryMode::ReadOnly {
        norm_component_count,
    } = telemetry_mode
        && (norm_component_count == 0 || norm_component_count > y.len())
    {
        return Err(CoreError::InvalidInput(format!(
            "early-flow-defect norm component count {norm_component_count} is outside 1..={}",
            y.len()
        )));
    }

    let tableau = pexprb54s4_tableau();
    let mut work = WorkCounters::default();
    let f0 = problem.eval_rhs(t, y, &mut work)?;
    let operator = problem.linearize_matrix_free(t, y)?;

    // Dependency level 1: U2 and D2 only.  No U3/U4 or endpoint work is allowed here.
    let u2_action = require_fused(fused_phi_linear_combination(
        operator.clone(),
        tableau.c2 * h,
        &[FusedPhiTerm {
            coefficient: tableau.c2,
            phi_index: 1,
            vector: &f0,
        }],
        config,
        &mut work,
    )?)?;
    let u2 = add_scaled_state(y, h, &u2_action.value);
    let d2 = nonlinear_remainder(problem, operator.as_ref(), t, y, &f0, &u2, &mut work)?;
    let early_flow_defect =
        early_flow_defect_telemetry(telemetry_mode, tableau.c2, h, y, &u2, &d2, tolerance_scale)?;

    let report = Pexprb54s4Level1PrefixReport {
        method: "pexprb54s4-fused-level1".into(),
        t,
        h,
        logical_critical_depth: 1,
        fused_phi_reports: vec![u2_action.clone()],
        work,
        early_flow_defect,
    };

    Ok(Pexprb54s4Level1Prefix {
        problem: problem.clone(),
        t,
        y: y.to_vec(),
        h,
        config,
        f0,
        operator,
        d2,
        u2_action,
        telemetry_mode,
        tolerance_scale,
        report,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: FusedPhiKrylovConfig,
    norm_component_count: usize,
    atol: f64,
    rtol: f64,
) -> CoreResult<Pexprb54s4Level1Prefix> {
    pexprb54s4_level1_prefix_with_telemetry_config(
        problem,
        t,
        y,
        h,
        config,
        EarlyFlowDefectTelemetryMode::ReadOnly {
            norm_component_count,
        },
        Some(EarlyFlowDefectToleranceScale { atol, rtol }),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget_accounted(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: FusedPhiKrylovConfig,
    norm_component_count: usize,
    atol: f64,
    rtol: f64,
    jvp_cap: u64,
) -> CoreResult<Pexprb54s4AccountedBudgetedLevel2PrefixOutcome> {
    validate_problem(problem, y)?;
    if norm_component_count == 0 || norm_component_count > y.len() {
        return Err(CoreError::InvalidInput(format!(
            "early-flow-defect norm component count {norm_component_count} is outside 1..={}",
            y.len()
        )));
    }

    let telemetry_mode = EarlyFlowDefectTelemetryMode::ReadOnly {
        norm_component_count,
    };
    let tolerance_scale = Some(EarlyFlowDefectToleranceScale { atol, rtol });
    let tableau = pexprb54s4_tableau();
    let mut level1_work = WorkCounters::default();
    let f0 = match problem.eval_rhs(t, y, &mut level1_work) {
        Ok(value) => value,
        Err(error) => {
            return Ok(failed_budgeted_prefix_outcome(error, level1_work));
        }
    };
    let inner_operator = match problem.linearize_matrix_free(t, y) {
        Ok(operator) => operator,
        Err(error) => {
            return Ok(failed_budgeted_prefix_outcome(error, level1_work));
        }
    };
    let budget = Arc::new(Pexprb54s4JvpBudget::new(jvp_cap));
    let guarded_operator = Arc::new(Pexprb54s4BudgetedOperator::new(
        inner_operator.clone(),
        Arc::clone(&budget),
    )) as Arc<dyn LinearOperator>;

    let u2_action = match fused_phi_linear_combination(
        guarded_operator.clone(),
        tableau.c2 * h,
        &[FusedPhiTerm {
            coefficient: tableau.c2,
            phi_index: 1,
            vector: &f0,
        }],
        config,
        &mut level1_work,
    ) {
        Err(error) if is_pexprb_prefix_budget_exhaustion(&error) => {
            return Ok(budget_exhausted_outcome(jvp_cap, &budget, level1_work));
        }
        Err(error) => return Ok(failed_budgeted_prefix_outcome(error, level1_work)),
        Ok(report) => match require_fused(report) {
            Ok(action) => action,
            Err(error) => return Ok(failed_budgeted_prefix_outcome(error, level1_work)),
        },
    };
    let u2 = add_scaled_state(y, h, &u2_action.value);
    let d2 = match nonlinear_remainder(
        problem,
        guarded_operator.as_ref(),
        t,
        y,
        &f0,
        &u2,
        &mut level1_work,
    ) {
        Err(error) if is_pexprb_prefix_budget_exhaustion(&error) => {
            return Ok(budget_exhausted_outcome(jvp_cap, &budget, level1_work));
        }
        Err(error) => return Ok(failed_budgeted_prefix_outcome(error, level1_work)),
        Ok(remainder) => remainder,
    };
    let early_flow_defect = match early_flow_defect_telemetry(
        telemetry_mode,
        tableau.c2,
        h,
        y,
        &u2,
        &d2,
        tolerance_scale,
    ) {
        Ok(telemetry) => telemetry,
        Err(error) => return Ok(failed_budgeted_prefix_outcome(error, level1_work)),
    };
    let level1_report = Pexprb54s4Level1PrefixReport {
        method: "pexprb54s4-fused-level1".into(),
        t,
        h,
        logical_critical_depth: 1,
        fused_phi_reports: vec![u2_action.clone()],
        work: level1_work,
        early_flow_defect,
    };

    let mut stages = Vec::with_capacity(2);
    let mut level2_incremental_work = WorkCounters::default();
    for id in [3usize, 4usize] {
        let mut local = WorkCounters::default();
        let (c, a) = if id == 3 {
            (tableau.c3, tableau.a32_phi3)
        } else {
            (tableau.c4, tableau.a42_phi3)
        };
        let action = match fused_phi_linear_combination(
            guarded_operator.clone(),
            c * h,
            &[
                FusedPhiTerm {
                    coefficient: c,
                    phi_index: 1,
                    vector: &f0,
                },
                FusedPhiTerm {
                    coefficient: a,
                    phi_index: 3,
                    vector: &d2,
                },
            ],
            config,
            &mut local,
        ) {
            Err(error) if is_pexprb_prefix_budget_exhaustion(&error) => {
                level2_incremental_work.accumulate(local);
                let mut spent = level1_report.work;
                spent.accumulate(level2_incremental_work);
                return Ok(budget_exhausted_outcome(jvp_cap, &budget, spent));
            }
            Err(error) => {
                level2_incremental_work.accumulate(local);
                let mut spent = level1_report.work;
                spent.accumulate(level2_incremental_work);
                return Ok(failed_budgeted_prefix_outcome(error, spent));
            }
            Ok(report) => match require_fused(report) {
                Ok(action) => action,
                Err(error) => {
                    level2_incremental_work.accumulate(local);
                    let mut spent = level1_report.work;
                    spent.accumulate(level2_incremental_work);
                    return Ok(failed_budgeted_prefix_outcome(error, spent));
                }
            },
        };
        let stage = add_scaled_state(y, h, &action.value);
        let remainder = match nonlinear_remainder(
            problem,
            guarded_operator.as_ref(),
            t,
            y,
            &f0,
            &stage,
            &mut local,
        ) {
            Err(error) if is_pexprb_prefix_budget_exhaustion(&error) => {
                level2_incremental_work.accumulate(local);
                let mut spent = level1_report.work;
                spent.accumulate(level2_incremental_work);
                return Ok(budget_exhausted_outcome(jvp_cap, &budget, spent));
            }
            Err(error) => {
                level2_incremental_work.accumulate(local);
                let mut spent = level1_report.work;
                spent.accumulate(level2_incremental_work);
                return Ok(failed_budgeted_prefix_outcome(error, spent));
            }
            Ok(remainder) => remainder,
        };
        level2_incremental_work.accumulate(local);
        stages.push((action, stage, remainder));
    }

    let mut cumulative_work = level1_report.work;
    cumulative_work.accumulate(level2_incremental_work);
    debug_assert_eq!(budget.used(), cumulative_work.jvp_vectors);

    let d3 = stages[0].2.clone();
    let d4 = stages[1].2.clone();
    let stage3_flow_defect = match early_flow_defect_telemetry(
        telemetry_mode,
        tableau.c3,
        h,
        y,
        &stages[0].1,
        &d3,
        tolerance_scale,
    ) {
        Ok(telemetry) => telemetry,
        Err(error) => return Ok(failed_budgeted_prefix_outcome(error, cumulative_work)),
    };
    let stage4_flow_defect = match early_flow_defect_telemetry(
        telemetry_mode,
        tableau.c4,
        h,
        y,
        &stages[1].1,
        &d4,
        tolerance_scale,
    ) {
        Ok(telemetry) => telemetry,
        Err(error) => return Ok(failed_budgeted_prefix_outcome(error, cumulative_work)),
    };
    let remainder_vector_geometry =
        match pexprb54s4_remainder_vector_geometry(&d2, &d3, &d4, norm_component_count) {
            Ok(geometry) => Some(geometry),
            Err(error) => return Ok(failed_budgeted_prefix_outcome(error, cumulative_work)),
        };
    let quadratic_remainder_drift = match pexprb54s4_quadratic_remainder_drift(
        y,
        &u2,
        &stages[0].1,
        &stages[1].1,
        &d2,
        &d3,
        &d4,
        h,
        norm_component_count,
        atol,
        rtol,
    ) {
        Ok(drift) => Some(drift),
        Err(error) => return Ok(failed_budgeted_prefix_outcome(error, cumulative_work)),
    };
    let u3_action = stages[0].0.clone();
    let u4_action = stages[1].0.clone();
    let report = Pexprb54s4Level2PrefixReport {
        method: "pexprb54s4-fused-level2".into(),
        t,
        h,
        logical_critical_depth: 2,
        level1_report,
        level2_fused_phi_reports: vec![u3_action.clone(), u4_action.clone()],
        level2_incremental_work,
        cumulative_work,
        stage3_flow_defect,
        stage4_flow_defect,
        remainder_vector_geometry,
        quadratic_remainder_drift,
    };
    Ok(Pexprb54s4AccountedBudgetedLevel2PrefixOutcome::Complete(
        Box::new(Pexprb54s4Level2Prefix {
            y: y.to_vec(),
            h,
            config,
            f0,
            operator: inner_operator,
            d2,
            d3,
            d4,
            u2_action,
            u3_action,
            u4_action,
            report,
        }),
    ))
}

/// Compatibility API retaining the v3.5 `CoreResult` failure surface.
#[allow(clippy::too_many_arguments)]
pub fn pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: FusedPhiKrylovConfig,
    norm_component_count: usize,
    atol: f64,
    rtol: f64,
    jvp_cap: u64,
) -> CoreResult<Pexprb54s4BudgetedLevel2PrefixOutcome> {
    match pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget_accounted(
        problem,
        t,
        y,
        h,
        config,
        norm_component_count,
        atol,
        rtol,
        jvp_cap,
    )? {
        Pexprb54s4AccountedBudgetedLevel2PrefixOutcome::Complete(prefix) => {
            Ok(Pexprb54s4BudgetedLevel2PrefixOutcome::Complete(prefix))
        }
        Pexprb54s4AccountedBudgetedLevel2PrefixOutcome::BudgetExhausted(report) => Ok(
            Pexprb54s4BudgetedLevel2PrefixOutcome::BudgetExhausted(report),
        ),
        Pexprb54s4AccountedBudgetedLevel2PrefixOutcome::Failed(report) => Err(report.error),
    }
}

pub fn pexprb54s4_level2_prefix_resume_level1(
    prefix: Pexprb54s4Level1Prefix,
    execution: &ParallelExecution,
) -> CoreResult<Pexprb54s4Level2Prefix> {
    let Pexprb54s4Level1Prefix {
        problem,
        t,
        y,
        h,
        config,
        f0,
        operator,
        d2,
        u2_action,
        telemetry_mode,
        tolerance_scale,
        report: level1_report,
    } = prefix;
    let tableau = pexprb54s4_tableau();

    // Dependency level 2: U3 and U4 are independent once D2 is known.
    let stage_ids = [3usize, 4usize];
    let stages = execution.map_ordered(&stage_ids, |id| {
        let mut local = WorkCounters::default();
        let (c, a) = if *id == 3 {
            (tableau.c3, tableau.a32_phi3)
        } else {
            (tableau.c4, tableau.a42_phi3)
        };
        let action = require_fused(fused_phi_linear_combination(
            operator.clone(),
            c * h,
            &[
                FusedPhiTerm {
                    coefficient: c,
                    phi_index: 1,
                    vector: &f0,
                },
                FusedPhiTerm {
                    coefficient: a,
                    phi_index: 3,
                    vector: &d2,
                },
            ],
            config,
            &mut local,
        )?)?;
        let stage = add_scaled_state(&y, h, &action.value);
        let remainder =
            nonlinear_remainder(&problem, operator.as_ref(), t, &y, &f0, &stage, &mut local)?;
        Ok((action, stage, remainder, local))
    })?;

    let mut level2_incremental_work = WorkCounters::default();
    for (_, _, _, local) in &stages {
        level2_incremental_work.accumulate(*local);
    }
    let mut cumulative_work = level1_report.work;
    cumulative_work.accumulate(level2_incremental_work);

    let d3 = stages[0].2.clone();
    let d4 = stages[1].2.clone();
    let stage3_flow_defect = early_flow_defect_telemetry(
        telemetry_mode,
        tableau.c3,
        h,
        &y,
        &stages[0].1,
        &d3,
        tolerance_scale,
    )?;
    let stage4_flow_defect = early_flow_defect_telemetry(
        telemetry_mode,
        tableau.c4,
        h,
        &y,
        &stages[1].1,
        &d4,
        tolerance_scale,
    )?;

    let remainder_vector_geometry = match telemetry_mode {
        EarlyFlowDefectTelemetryMode::Disabled => None,
        EarlyFlowDefectTelemetryMode::ReadOnly {
            norm_component_count,
        } => Some(pexprb54s4_remainder_vector_geometry(
            &d2,
            &d3,
            &d4,
            norm_component_count,
        )?),
    };
    let quadratic_remainder_drift = match (telemetry_mode, tolerance_scale) {
        (
            EarlyFlowDefectTelemetryMode::ReadOnly {
                norm_component_count,
            },
            Some(scale),
        ) => {
            let u2 = add_scaled_state(&y, h, &u2_action.value);
            Some(pexprb54s4_quadratic_remainder_drift(
                &y,
                &u2,
                &stages[0].1,
                &stages[1].1,
                &d2,
                &d3,
                &d4,
                h,
                norm_component_count,
                scale.atol,
                scale.rtol,
            )?)
        }
        _ => None,
    };

    let u3_action = stages[0].0.clone();
    let u4_action = stages[1].0.clone();
    let report = Pexprb54s4Level2PrefixReport {
        method: "pexprb54s4-fused-level2".into(),
        t,
        h,
        logical_critical_depth: 2,
        level1_report,
        level2_fused_phi_reports: vec![u3_action.clone(), u4_action.clone()],
        level2_incremental_work,
        cumulative_work,
        stage3_flow_defect,
        stage4_flow_defect,
        remainder_vector_geometry,
        quadratic_remainder_drift,
    };

    Ok(Pexprb54s4Level2Prefix {
        y,
        h,
        config,
        f0,
        operator,
        d2,
        d3,
        d4,
        u2_action,
        u3_action,
        u4_action,
        report,
    })
}

fn pexprb54s4_level2_endpoint_terms<'a>(
    endpoint: usize,
    tableau: Pexprb54s4Tableau,
    f0: &'a [f64],
    d2: &'a [f64],
    d3: &'a [f64],
    d4: &'a [f64],
) -> Vec<FusedPhiTerm<'a>> {
    if endpoint == 0 {
        vec![
            FusedPhiTerm {
                coefficient: 1.0,
                phi_index: 1,
                vector: f0,
            },
            FusedPhiTerm {
                coefficient: tableau.b3_phi3,
                phi_index: 3,
                vector: d3,
            },
            FusedPhiTerm {
                coefficient: tableau.b3_phi4,
                phi_index: 4,
                vector: d3,
            },
            FusedPhiTerm {
                coefficient: tableau.b4_phi3,
                phi_index: 3,
                vector: d4,
            },
            FusedPhiTerm {
                coefficient: tableau.b4_phi4,
                phi_index: 4,
                vector: d4,
            },
        ]
    } else {
        vec![
            FusedPhiTerm {
                coefficient: 1.0,
                phi_index: 1,
                vector: f0,
            },
            FusedPhiTerm {
                coefficient: tableau.embedded_b2_phi3,
                phi_index: 3,
                vector: d2,
            },
            FusedPhiTerm {
                coefficient: tableau.embedded_b2_phi4,
                phi_index: 4,
                vector: d2,
            },
            FusedPhiTerm {
                coefficient: tableau.embedded_b3_phi3,
                phi_index: 3,
                vector: d3,
            },
            FusedPhiTerm {
                coefficient: tableau.embedded_b3_phi4,
                phi_index: 4,
                vector: d3,
            },
            FusedPhiTerm {
                coefficient: tableau.embedded_b4_phi3,
                phi_index: 3,
                vector: d4,
            },
            FusedPhiTerm {
                coefficient: tableau.embedded_b4_phi4,
                phi_index: 4,
                vector: d4,
            },
        ]
    }
}

#[allow(clippy::too_many_arguments)]
fn complete_pexprb54s4_level2_report(
    y: Vec<f64>,
    h: f64,
    u2_action: FusedPhiActionReport,
    u3_action: FusedPhiActionReport,
    u4_action: FusedPhiActionReport,
    prefix_report: Pexprb54s4Level2PrefixReport,
    main_endpoint: FusedPhiActionReport,
    embedded_endpoint: FusedPhiActionReport,
    work: WorkCounters,
) -> FusedExponentialStepReport {
    let y_new = add_scaled_state(&y, h, &main_endpoint.value);
    let y_embedded = add_scaled_state(&y, h, &embedded_endpoint.value);
    let error_estimate = y_new.iter().zip(&y_embedded).map(|(a, b)| a - b).collect();
    FusedExponentialStepReport {
        method: "pexprb54s4-fused".into(),
        y_new,
        y_embedded: Some(y_embedded),
        error_estimate: Some(error_estimate),
        logical_critical_depth: 3,
        fused_phi_reports: vec![
            u2_action,
            u3_action,
            u4_action,
            main_endpoint,
            embedded_endpoint,
        ],
        work,
        early_flow_defect: prefix_report.level1_report.early_flow_defect,
    }
}

pub fn pexprb54s4_fused_step_resume_level2_accounted(
    prefix: Pexprb54s4Level2Prefix,
    execution: &ParallelExecution,
) -> CoreResult<Pexprb54s4Level2ContinuationOutcome> {
    pexprb54s4_fused_step_resume_level2_accounted_impl(prefix, execution, None)
}

/// Consume a retained level-2 prefix exactly once under an event-local continuation JVP cap.
///
/// The bounded authority path is deliberately sequential in v3.7. This keeps one prospective
/// counter authoritative for both endpoint actions; parallel shared-budget execution remains
/// deferred until it has a separate deterministic contract.
pub fn pexprb54s4_fused_step_resume_level2_accounted_jvp_budget(
    prefix: Pexprb54s4Level2Prefix,
    execution: &ParallelExecution,
    jvp_cap: u64,
) -> CoreResult<Pexprb54s4Level2ContinuationOutcome> {
    if execution.threads() != 1 {
        return Err(CoreError::InvalidInput(
            "bounded retained level-2 continuation requires sequential execution".into(),
        ));
    }
    pexprb54s4_fused_step_resume_level2_accounted_impl(prefix, execution, Some(jvp_cap))
}

fn pexprb54s4_fused_step_resume_level2_accounted_impl(
    prefix: Pexprb54s4Level2Prefix,
    execution: &ParallelExecution,
    jvp_cap: Option<u64>,
) -> CoreResult<Pexprb54s4Level2ContinuationOutcome> {
    let Pexprb54s4Level2Prefix {
        y,
        h,
        config,
        f0,
        operator,
        d2,
        d3,
        d4,
        u2_action,
        u3_action,
        u4_action,
        report,
    } = prefix;
    let tableau = pexprb54s4_tableau();
    let prefix_work = report.cumulative_work;
    let continuation_budget =
        jvp_cap.map(|cap| Arc::new(Pexprb54s4JvpBudget::new_continuation(cap)));
    let continuation_operator: Arc<dyn LinearOperator> = match &continuation_budget {
        Some(budget) => Arc::new(Pexprb54s4BudgetedOperator::new(
            operator,
            Arc::clone(budget),
        )),
        None => operator,
    };

    // Dependency level 3: main and embedded endpoints are independent.
    //
    // The inner action result is deliberately carried as data. This makes `map_ordered` finish
    // both scheduled endpoint jobs and preserves each local counter even when an endpoint action
    // fails. Ordered collection then gives a deterministic main-before-embedded error choice.
    let endpoint_ids = [0usize, 1usize];
    let endpoints = execution.map_ordered(&endpoint_ids, |id| {
        let mut local = WorkCounters::default();
        let terms = pexprb54s4_level2_endpoint_terms(*id, tableau, &f0, &d2, &d3, &d4);
        let action = fused_phi_linear_combination(
            Arc::clone(&continuation_operator),
            h,
            &terms,
            config,
            &mut local,
        )
        .and_then(require_fused);
        Ok((action, local))
    })?;

    let mut continuation_work = WorkCounters::default();
    let mut endpoint_actions = Vec::with_capacity(2);
    let mut first_hard_error = None;
    let mut budget_exhausted = false;
    for (action, local) in endpoints {
        continuation_work.accumulate(local);
        match action {
            Ok(action) => endpoint_actions.push(action),
            Err(error) if is_pexprb_continuation_budget_exhaustion(&error) => {
                budget_exhausted = true;
            }
            Err(error) if first_hard_error.is_none() => first_hard_error = Some(error),
            Err(_) => {}
        }
    }

    let mut cumulative_work = prefix_work;
    cumulative_work.accumulate(continuation_work);
    let ledger = Pexprb54s4Level2ContinuationLedger {
        prefix_work,
        continuation_work,
        cumulative_work,
    };
    if let Some(error) = first_hard_error {
        return Ok(Pexprb54s4Level2ContinuationOutcome::Failed {
            error,
            ledger: Box::new(ledger),
        });
    }
    if budget_exhausted {
        let budget = continuation_budget.ok_or_else(|| {
            CoreError::LinearSolve(
                "unbounded retained level-2 continuation reported budget exhaustion".into(),
            )
        })?;
        if budget.used() != continuation_work.jvp_vectors {
            return Err(CoreError::LinearSolve(
                "retained level-2 continuation budget/work ledger mismatch".into(),
            ));
        }
        return Ok(Pexprb54s4Level2ContinuationOutcome::BudgetExhausted {
            jvp_cap: jvp_cap.expect("budget exhaustion requires a configured cap"),
            used_jvp_vectors: budget.used(),
            ledger: Box::new(ledger),
        });
    }

    let [main_endpoint, embedded_endpoint] = endpoint_actions.try_into().map_err(|_| {
        CoreError::LinearSolve(
            "pexprb54s4 retained level-2 continuation lost an endpoint result".into(),
        )
    })?;
    let report = complete_pexprb54s4_level2_report(
        y,
        h,
        u2_action,
        u3_action,
        u4_action,
        report,
        main_endpoint,
        embedded_endpoint,
        cumulative_work,
    );
    Ok(Pexprb54s4Level2ContinuationOutcome::Complete {
        report: Box::new(report),
        ledger: Box::new(ledger),
    })
}

/// Compatibility wrapper for the original retained-level-2 resume API.
///
/// Callers that need failure-path work accounting should use
/// [`pexprb54s4_fused_step_resume_level2_accounted`].
pub fn pexprb54s4_fused_step_resume_level2(
    prefix: Pexprb54s4Level2Prefix,
    execution: &ParallelExecution,
) -> CoreResult<FusedExponentialStepReport> {
    let Pexprb54s4Level2Prefix {
        y,
        h,
        config,
        f0,
        operator,
        d2,
        d3,
        d4,
        u2_action,
        u3_action,
        u4_action,
        report,
    } = prefix;
    let tableau = pexprb54s4_tableau();
    let mut work = report.cumulative_work;
    let endpoint_ids = [0usize, 1usize];
    let endpoints = execution.map_ordered(&endpoint_ids, |id| {
        let mut local = WorkCounters::default();
        let terms = pexprb54s4_level2_endpoint_terms(*id, tableau, &f0, &d2, &d3, &d4);
        let action = require_fused(fused_phi_linear_combination(
            operator.clone(),
            h,
            &terms,
            config,
            &mut local,
        )?)?;
        Ok((action, local))
    })?;
    for (_, local) in &endpoints {
        work.accumulate(*local);
    }
    let [main, embedded] = endpoints.try_into().map_err(|_| {
        CoreError::LinearSolve(
            "pexprb54s4 retained level-2 continuation lost an endpoint result".into(),
        )
    })?;
    Ok(complete_pexprb54s4_level2_report(
        y, h, u2_action, u3_action, u4_action, report, main.0, embedded.0, work,
    ))
}

pub fn pexprb54s4_fused_step_resume_level1(
    prefix: Pexprb54s4Level1Prefix,
    execution: &ParallelExecution,
) -> CoreResult<FusedExponentialStepReport> {
    let level2 = pexprb54s4_level2_prefix_resume_level1(prefix, execution)?;
    pexprb54s4_fused_step_resume_level2(level2, execution)
}

#[allow(clippy::too_many_arguments)]
fn pexprb54s4_fused_step_with_telemetry_config(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: FusedPhiKrylovConfig,
    execution: &ParallelExecution,
    telemetry_mode: EarlyFlowDefectTelemetryMode,
    tolerance_scale: Option<EarlyFlowDefectToleranceScale>,
) -> CoreResult<FusedExponentialStepReport> {
    let prefix = pexprb54s4_level1_prefix_with_telemetry_config(
        problem,
        t,
        y,
        h,
        config,
        telemetry_mode,
        tolerance_scale,
    )?;
    pexprb54s4_fused_step_resume_level1(prefix, execution)
}

#[cfg(test)]
mod continuation_budget_guard_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct CountingOperator {
        calls: Arc<AtomicU64>,
    }

    impl CountingOperator {
        fn new(calls: Arc<AtomicU64>) -> Self {
            Self { calls }
        }
    }

    impl LinearOperator for CountingOperator {
        fn dimension(&self) -> usize {
            1
        }

        fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
            if x.len() != 1 || y.len() != 1 {
                return Err(CoreError::Dimension(
                    "counting operator shape mismatch".into(),
                ));
            }
            self.calls.fetch_add(1, Ordering::Relaxed);
            y[0] = x[0];
            Ok(())
        }

        fn token(&self) -> u64 {
            0xC017
        }
    }

    fn continuation_guard(
        cap: u64,
        calls: Arc<AtomicU64>,
    ) -> (Pexprb54s4BudgetedOperator, Arc<Pexprb54s4JvpBudget>) {
        let budget = Arc::new(Pexprb54s4JvpBudget::new_continuation(cap));
        let inner = Arc::new(CountingOperator::new(calls)) as Arc<dyn LinearOperator>;
        (
            Pexprb54s4BudgetedOperator::new(inner, Arc::clone(&budget)),
            budget,
        )
    }

    #[test]
    fn continuation_cap_admits_exactly_eighty_scalars_and_denies_jvp_81_before_call() {
        let calls = Arc::new(AtomicU64::new(0));
        let (guarded, budget) = continuation_guard(80, Arc::clone(&calls));
        let mut output = [0.0];
        for _ in 0..80 {
            guarded.apply(&[1.0], &mut output).unwrap();
        }
        let error = guarded.apply(&[1.0], &mut output).unwrap_err();
        assert!(is_pexprb_continuation_budget_exhaustion(&error));
        assert_eq!(budget.used(), 80);
        assert_eq!(calls.load(Ordering::Relaxed), 80);
    }

    #[test]
    fn zero_continuation_budget_invokes_no_underlying_jvp() {
        let calls = Arc::new(AtomicU64::new(0));
        let (guarded, budget) = continuation_guard(0, Arc::clone(&calls));
        let error = guarded.apply(&[1.0], &mut [0.0]).unwrap_err();
        assert!(is_pexprb_continuation_budget_exhaustion(&error));
        assert_eq!(budget.used(), 0);
        assert_eq!(calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn over_cap_row_batch_is_denied_atomically_without_partial_execution() {
        let calls = Arc::new(AtomicU64::new(0));
        let (guarded, budget) = continuation_guard(4, Arc::clone(&calls));
        let mut output = [0.0];
        for _ in 0..3 {
            guarded.apply(&[1.0], &mut output).unwrap();
        }
        let inputs = vec![vec![1.0], vec![2.0]];
        let mut outputs = vec![vec![0.0], vec![0.0]];
        let error = guarded.apply_rows(&inputs, &mut outputs).unwrap_err();
        assert!(is_pexprb_continuation_budget_exhaustion(&error));
        assert_eq!(budget.used(), 3);
        assert_eq!(calls.load(Ordering::Relaxed), 3);
        assert_eq!(outputs, vec![vec![0.0], vec![0.0]]);
    }

    #[test]
    fn invalid_row_shape_is_rejected_before_budget_reservation() {
        let calls = Arc::new(AtomicU64::new(0));
        let (guarded, budget) = continuation_guard(4, Arc::clone(&calls));
        let inputs = vec![vec![1.0, 2.0]];
        let mut outputs = vec![vec![0.0]];
        let error = guarded.apply_rows(&inputs, &mut outputs).unwrap_err();
        assert!(matches!(error, CoreError::Dimension(_)));
        assert_eq!(budget.used(), 0);
        assert_eq!(calls.load(Ordering::Relaxed), 0);
    }
}
