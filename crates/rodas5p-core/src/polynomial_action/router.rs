//! Opt-in Chebyshev/Laguerre total-budget router (RVJ DAG node PP08,
//! `research/pp08_laguerre_router_20261004`). Research only: no integrator
//! or default dispatch calls it.
//!
//! Both bases run with the truncation budget `total_budget / 4`; each result
//! is admitted only by its own total admission against the full
//! `total_budget` ([`route_admission`]: the Chebyshev certified total or the
//! Laguerre adjoint total), behind two router guards (certified execution,
//! verified enclosure). Among the admitted results the
//! one with fewer operator vector products wins, ties to Chebyshev; with none
//! admitted the route is [`RouteChoice::Fallback`] and the caller uses its
//! protected path. Both attempts stay charged in the caller's counters.

use serde::Serialize;

use super::{
    ActionPlan, EXECUTION_CERTIFIED, EnclosureEvidence, JointPhiInput, JointPhiReport, PlanRecord,
    PolynomialBasis, PolynomialCaches, SymmetricNonpositiveOperator, TotalErrorAdmission,
    execute_joint_phi_counted, joint_phi_action, laguerre_preflight_terms, plan_joint_phi,
};
use crate::{CoreError, CoreResult, WorkCounters};

/// An invalid total budget, step or input: an error, never a fallback.
pub const ROUTER_INPUT_UNSUPPORTED: &str = "PP08_ROUTER_INPUT_UNSUPPORTED";
/// Prefix of every rejection the router adds on top of the admission
/// functions.
pub const ROUTER_NOT_ADMITTED: &str = "PP08_ROUTER_NOT_ADMITTED";
/// The truncation budget of each attempt is `total_budget / 4`.
pub const ROUTER_TRUNCATION_FRACTION: f64 = 0.25;

/// The basis the router chose, or the fallback with both rejection reasons.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum RouteChoice {
    Chebyshev,
    Laguerre,
    /// Neither result is admitted; the caller uses its protected path.
    Fallback {
        chebyshev_reason: String,
        laguerre_reason: String,
    },
}

/// One basis attempt: its degree and scale, the admission (or the reason it
/// was not admitted, an action error included) and its work as the change of
/// the caller's counters.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RouteAttempt {
    basis: PolynomialBasis,
    degree: Option<usize>,
    laguerre_scale: Option<f64>,
    admission: TotalErrorAdmission,
    vector_products: u64,
    coefficient_setups: u64,
    work: WorkCounters,
    #[serde(skip)]
    report: Option<JointPhiReport>,
}

impl RouteAttempt {
    pub fn basis(&self) -> PolynomialBasis {
        self.basis
    }

    /// `None` when the action returned an error.
    pub fn degree(&self) -> Option<usize> {
        self.degree
    }

    pub fn laguerre_scale(&self) -> Option<f64> {
        self.laguerre_scale
    }

    pub fn admission(&self) -> &TotalErrorAdmission {
        &self.admission
    }

    pub fn is_admitted(&self) -> bool {
        matches!(self.admission, TotalErrorAdmission::Admitted { .. })
    }

    /// The admitted total bound.
    pub fn bound(&self) -> Option<f64> {
        match self.admission {
            TotalErrorAdmission::Admitted { bound, .. } => Some(bound),
            TotalErrorAdmission::Rejected { .. } => None,
        }
    }

    pub fn rejection_reason(&self) -> Option<&str> {
        match &self.admission {
            TotalErrorAdmission::Admitted { .. } => None,
            TotalErrorAdmission::Rejected { reason } => Some(reason),
        }
    }

    /// `poly_vector_products` added by this attempt.
    pub fn vector_products(&self) -> u64 {
        self.vector_products
    }

    /// `poly_coefficient_setups` added by this attempt.
    pub fn coefficient_setups(&self) -> u64 {
        self.coefficient_setups
    }

    /// Every counter this attempt added.
    pub fn work(&self) -> WorkCounters {
        self.work
    }

    /// The attempt's report; `None` when the action returned an error.
    pub fn report(&self) -> Option<&JointPhiReport> {
        self.report.as_ref()
    }
}

/// The outcome of [`route_joint_phi`]. Constructed only by the router (no
/// public fields, no `Deserialize`), so an admitted route always comes from
/// an admission against its own budget.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RoutedPhi {
    choice: RouteChoice,
    total_budget: f64,
    truncation_budget: f64,
    /// Chebyshev first, then Laguerre.
    attempts: Vec<RouteAttempt>,
    fused: Option<Vec<f64>>,
    bound: Option<f64>,
}

impl RoutedPhi {
    pub fn choice(&self) -> &RouteChoice {
        &self.choice
    }

    /// The chosen basis; `None` for [`RouteChoice::Fallback`].
    pub fn basis(&self) -> Option<PolynomialBasis> {
        match self.choice {
            RouteChoice::Chebyshev => Some(PolynomialBasis::Chebyshev),
            RouteChoice::Laguerre => Some(PolynomialBasis::Laguerre),
            RouteChoice::Fallback { .. } => None,
        }
    }

    pub fn is_fallback(&self) -> bool {
        matches!(self.choice, RouteChoice::Fallback { .. })
    }

    pub fn total_budget(&self) -> f64 {
        self.total_budget
    }

    pub fn truncation_budget(&self) -> f64 {
        self.truncation_budget
    }

    /// Both attempts, Chebyshev first.
    pub fn attempts(&self) -> &[RouteAttempt] {
        &self.attempts
    }

    pub fn attempt(&self, basis: PolynomialBasis) -> &RouteAttempt {
        match basis {
            PolynomialBasis::Chebyshev => &self.attempts[0],
            PolynomialBasis::Laguerre => &self.attempts[1],
        }
    }

    /// The admitted fused result; `None` for a fallback.
    pub fn fused(&self) -> Option<&[f64]> {
        self.fused.as_deref()
    }

    /// The admitted total bound, at most [`Self::total_budget`].
    pub fn bound(&self) -> Option<f64> {
        self.bound
    }

    /// The chosen attempt's report.
    pub fn report(&self) -> Option<&JointPhiReport> {
        self.basis().and_then(|basis| self.attempt(basis).report())
    }

    /// The sum of both attempts' work, which the router added to the caller's
    /// counters.
    pub fn total_work(&self) -> WorkCounters {
        let mut total = WorkCounters::default();
        for attempt in &self.attempts {
            total.accumulate(attempt.work);
        }
        total
    }
}

/// The router's admission of one report against `total_budget`: rejected
/// unless the execution carried the rounding enclosures and the enclosure is
/// Gershgorin-verified; then [`JointPhiReport::admit_total_error`] for a
/// Chebyshev report and [`JointPhiReport::admit_laguerre_total`] for a
/// Laguerre one (which also refuses a degree above
/// [`crate::laguerre_adjoint::LAGUERRE_ADJOINT_DEGREE_LIMIT`] and a missing
/// adjoint total). The guards are needed because the scalar branch certifies
/// `h = 0` under a declared enclosure, and its bounds also under the timing
/// execution.
pub fn route_admission(report: &JointPhiReport, total_budget: f64) -> TotalErrorAdmission {
    if report.execution != EXECUTION_CERTIFIED {
        return TotalErrorAdmission::Rejected {
            reason: format!("{ROUTER_NOT_ADMITTED}: execution {}", report.execution),
        };
    }
    if !matches!(report.evidence, EnclosureEvidence::Gershgorin) {
        return TotalErrorAdmission::Rejected {
            reason: format!("{ROUTER_NOT_ADMITTED}: the spectral enclosure is not verified"),
        };
    }
    match report.basis {
        PolynomialBasis::Chebyshev => report.admit_total_error(total_budget),
        PolynomialBasis::Laguerre => report.admit_laguerre_total(total_budget),
    }
}

fn invalid(reason: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("{ROUTER_INPUT_UNSUPPORTED}: {reason}"))
}

fn attempt(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    basis: PolynomialBasis,
    truncation_budget: f64,
    total_budget: f64,
    work: &mut WorkCounters,
) -> CoreResult<RouteAttempt> {
    let before = *work;
    // No coefficient cache: each attempt's setup is part of its cost.
    let result = joint_phi_action(op, h, input, basis, truncation_budget, None, work);
    let delta = work
        .checked_delta(before)
        .ok_or_else(|| invalid("a work counter decreased during an attempt"))?;
    let (degree, laguerre_scale, admission, report) = match result {
        Ok(report) => (
            Some(report.degree),
            report.laguerre_scale,
            route_admission(&report, total_budget),
            Some(report),
        ),
        // An action error (degree or coefficient range, for example) only
        // means this basis is not admitted; the inputs were checked above.
        Err(error) => (
            None,
            None,
            TotalErrorAdmission::Rejected {
                reason: format!("{ROUTER_NOT_ADMITTED}: action failed: {error}"),
            },
            None,
        ),
    };
    Ok(RouteAttempt {
        basis,
        degree,
        laguerre_scale,
        admission,
        vector_products: delta.poly_vector_products,
        coefficient_setups: delta.poly_coefficient_setups,
        work: delta,
        report,
    })
}

/// Route `sum_k phi_k(h A) w_k` between the Chebyshev and the Laguerre
/// basis under the absolute `total_budget` (preregistered rule of
/// `research/pp08_laguerre_router_20261004`). A NaN, negative or infinite
/// budget, a negative or non-finite `h` and a malformed input are errors.
/// A zero budget is valid and ends in a fallback (no positive truncation
/// budget).
pub fn route_joint_phi(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    total_budget: f64,
    work: &mut WorkCounters,
) -> CoreResult<RoutedPhi> {
    if !(total_budget.is_finite() && total_budget >= 0.0) {
        return Err(invalid(format!(
            "total budget {total_budget:e} outside [0, inf)"
        )));
    }
    if !(h.is_finite() && h >= 0.0) {
        return Err(invalid(format!("step h = {h:e} must be finite and >= 0")));
    }
    let n = op.dimension();
    let finite_input = match input {
        JointPhiInput::Distinct(vectors) => {
            vectors.iter().all(|w| w.len() == n) && vectors.iter().flatten().all(|x| x.is_finite())
        }
        JointPhiInput::SameVector { vector, scales } => {
            vector.len() == n && vector.iter().chain(&scales).all(|x| x.is_finite())
        }
    };
    if !finite_input {
        return Err(invalid(
            "every input vector needs the operator dimension and finite entries",
        ));
    }
    let truncation_budget = total_budget * ROUTER_TRUNCATION_FRACTION;
    let chebyshev = attempt(
        op,
        h,
        input,
        PolynomialBasis::Chebyshev,
        truncation_budget,
        total_budget,
        work,
    )?;
    let laguerre = attempt(
        op,
        h,
        input,
        PolynomialBasis::Laguerre,
        truncation_budget,
        total_budget,
        work,
    )?;
    let choice = match (chebyshev.is_admitted(), laguerre.is_admitted()) {
        (true, true) if laguerre.vector_products < chebyshev.vector_products => {
            RouteChoice::Laguerre
        }
        (true, _) => RouteChoice::Chebyshev,
        (false, true) => RouteChoice::Laguerre,
        (false, false) => RouteChoice::Fallback {
            chebyshev_reason: chebyshev.rejection_reason().unwrap_or_default().into(),
            laguerre_reason: laguerre.rejection_reason().unwrap_or_default().into(),
        },
    };
    let chosen = match choice {
        RouteChoice::Chebyshev => Some(&chebyshev),
        RouteChoice::Laguerre => Some(&laguerre),
        RouteChoice::Fallback { .. } => None,
    };
    let fused = chosen.and_then(|a| a.report.as_ref().map(|r| r.fused.clone()));
    let bound = chosen.and_then(RouteAttempt::bound);
    Ok(RoutedPhi {
        choice,
        total_budget,
        truncation_budget,
        attempts: vec![chebyshev, laguerre],
        fused,
        bound,
    })
}

// ---------------------------------------------------------------------------
// Plan first, execute one backend (research node PY03)
// ---------------------------------------------------------------------------

/// The operator's enclosure is declared, not Gershgorin-verified: no
/// execution could pass [`route_admission`], so none is run.
pub const ROUTER_ENCLOSURE_NOT_VERIFIED: &str = "PY03_ROUTER_ENCLOSURE_NOT_VERIFIED";
/// The basis could not be planned, so it was not executed.
pub const ROUTER_PLAN_FAILED: &str = "PY03_ROUTER_PLAN_FAILED";
/// The basis was not executed (the other was admitted first, or the policy
/// is Chebyshev alone).
pub const ROUTER_NOT_ATTEMPTED: &str = "PY03_ROUTER_NOT_ATTEMPTED";
/// Preflight abstentions (Laguerre not preselected).
pub const PREFLIGHT_LAGUERRE_PLAN_FAILED: &str = "PY03_PREFLIGHT_LAGUERRE_PLAN_FAILED";
pub const PREFLIGHT_CHEBYSHEV_PLAN_FAILED: &str = "PY03_PREFLIGHT_CHEBYSHEV_PLAN_FAILED";
pub const PREFLIGHT_PRODUCTS_NOT_FEWER: &str = "PY03_PREFLIGHT_PRODUCTS_NOT_FEWER";
pub const PREFLIGHT_DEGREE_ABOVE_LIMIT: &str = "PY03_PREFLIGHT_DEGREE_ABOVE_LIMIT";
pub const PREFLIGHT_TERMS_ABOVE_BUDGET: &str = "PY03_PREFLIGHT_TERMS_ABOVE_BUDGET";

/// The work of a plan, a preflight or an execution in the units of research
/// node PY03 (`research/py03_single_backend_router_20261011`): counted
/// work, no wall time. Router-local: [`WorkCounters`] is not extended; the
/// fields it shares with it (vector and block products, coefficient setups
/// and cache hits) are counted separately and must agree with it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct RouteCost {
    /// Tail evaluations of the degree searches.
    pub plan_tail_evaluations: u64,
    /// Coefficient tables built (preflight or execution).
    pub coefficient_setups: u64,
    /// Coefficient tables read from the cache.
    pub coefficient_cache_hits: u64,
    /// Terms of the positive series of the tables built (and of the scalar
    /// branch's `phi_k`).
    pub coefficient_series_terms: u64,
    /// Executions run (admitted or not).
    pub executed_backends: u64,
    /// Executions not admitted, a basis that could not be planned included.
    pub failed_executions: u64,
    /// Operator vector products.
    pub vector_products: u64,
    /// Operator block products.
    pub block_products: u64,
    /// Certification: terms of the Chebyshev rounding-propagation loop.
    pub chebyshev_propagation_terms: u64,
    /// Certification: interval operations of the Laguerre adjoint envelopes
    /// (zero on a cache hit).
    pub envelope_interval_operations: u64,
    /// Envelope cache hits and misses (one per column polynomial).
    pub envelope_cache_hits: u64,
    pub envelope_cache_misses: u64,
    /// Certification: terms `beta_j eps_(j-1)` of the adjoint sums.
    pub adjoint_sum_terms: u64,
}

impl RouteCost {
    /// Propagation-loop terms, envelope interval operations and adjoint-sum
    /// terms.
    pub fn certification_operations(&self) -> u64 {
        self.chebyshev_propagation_terms
            .saturating_add(self.envelope_interval_operations)
            .saturating_add(self.adjoint_sum_terms)
    }

    /// Field-by-field sum.
    pub fn accumulate(&mut self, other: Self) {
        macro_rules! add { ($($f:ident),*) => { $(self.$f = self.$f.saturating_add(other.$f);)* } }
        add!(
            plan_tail_evaluations,
            coefficient_setups,
            coefficient_cache_hits,
            coefficient_series_terms,
            executed_backends,
            failed_executions,
            vector_products,
            block_products,
            chebyshev_propagation_terms,
            envelope_interval_operations,
            envelope_cache_hits,
            envelope_cache_misses,
            adjoint_sum_terms
        );
    }

    /// `self - before`, field by field; `None` if any field decreased.
    pub fn checked_delta(self, before: Self) -> Option<Self> {
        macro_rules! sub { ($($f:ident),*) => {
            Some(Self { $($f: self.$f.checked_sub(before.$f)?,)* })
        } }
        sub!(
            plan_tail_evaluations,
            coefficient_setups,
            coefficient_cache_hits,
            coefficient_series_terms,
            executed_backends,
            failed_executions,
            vector_products,
            block_products,
            chebyshev_propagation_terms,
            envelope_interval_operations,
            envelope_cache_hits,
            envelope_cache_misses,
            adjoint_sum_terms
        )
    }
}

/// The execution order of [`route_joint_phi_single`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoutePolicy {
    /// Plan both bases; execute Laguerre first only when the preflight
    /// preselects it, else Chebyshev; the other only on rejection.
    #[default]
    ChebyshevFirstPreflight,
    /// Execute Laguerre first whatever the preflight (contract tests of a
    /// failed preflight only).
    ForceLaguerreFirst,
    /// The comparator "Chebyshev alone": plan and execute Chebyshev only.
    ChebyshevOnly,
}

/// One basis plan: the plan or its error, and its cost.
#[derive(Clone, Debug, Serialize)]
pub struct SinglePlan {
    basis: PolynomialBasis,
    record: Option<PlanRecord>,
    error: Option<String>,
    cost: RouteCost,
    #[serde(skip)]
    plan: Option<ActionPlan>,
}

impl SinglePlan {
    pub fn basis(&self) -> PolynomialBasis {
        self.basis
    }

    pub fn plan(&self) -> Option<&ActionPlan> {
        self.plan.as_ref()
    }

    pub fn record(&self) -> Option<&PlanRecord> {
        self.record.as_ref()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn cost(&self) -> RouteCost {
        self.cost
    }
}

/// The preflight: which basis executes first and why.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Preflight {
    policy: RoutePolicy,
    /// `None` when nothing executes (an unverified enclosure).
    preselected: Option<PolynomialBasis>,
    /// Why Laguerre was not preselected (the abstention to Chebyshev), or
    /// why nothing executes.
    abstain_reason: Option<String>,
    chebyshev_products: Option<u64>,
    laguerre_products: Option<u64>,
    laguerre_degree: Option<usize>,
    /// The Laguerre table was built (or read from the cache) for the
    /// vector-independent terms.
    laguerre_table_used: bool,
    vector_independent_terms: Option<f64>,
    cost: RouteCost,
    work: WorkCounters,
}

impl Preflight {
    pub fn policy(&self) -> RoutePolicy {
        self.policy
    }

    pub fn preselected(&self) -> Option<PolynomialBasis> {
        self.preselected
    }

    pub fn abstain_reason(&self) -> Option<&str> {
        self.abstain_reason.as_deref()
    }

    pub fn laguerre_table_used(&self) -> bool {
        self.laguerre_table_used
    }

    pub fn vector_independent_terms(&self) -> Option<f64> {
        self.vector_independent_terms
    }

    pub fn cost(&self) -> RouteCost {
        self.cost
    }

    pub fn work(&self) -> WorkCounters {
        self.work
    }
}

/// One execution: the admission of its report (or why it was not run) and
/// its work.
#[derive(Clone, Debug, Serialize)]
pub struct SingleExecution {
    basis: PolynomialBasis,
    /// False when the basis could not be planned.
    executed: bool,
    degree: Option<usize>,
    laguerre_scale: Option<f64>,
    admission: TotalErrorAdmission,
    cost: RouteCost,
    work: WorkCounters,
    #[serde(skip)]
    report: Option<JointPhiReport>,
}

impl SingleExecution {
    pub fn basis(&self) -> PolynomialBasis {
        self.basis
    }

    pub fn executed(&self) -> bool {
        self.executed
    }

    pub fn degree(&self) -> Option<usize> {
        self.degree
    }

    pub fn laguerre_scale(&self) -> Option<f64> {
        self.laguerre_scale
    }

    pub fn admission(&self) -> &TotalErrorAdmission {
        &self.admission
    }

    pub fn is_admitted(&self) -> bool {
        matches!(self.admission, TotalErrorAdmission::Admitted { .. })
    }

    pub fn rejection_reason(&self) -> Option<&str> {
        match &self.admission {
            TotalErrorAdmission::Admitted { .. } => None,
            TotalErrorAdmission::Rejected { reason } => Some(reason),
        }
    }

    pub fn cost(&self) -> RouteCost {
        self.cost
    }

    /// The change of the caller's counters during this execution.
    pub fn work(&self) -> WorkCounters {
        self.work
    }

    /// `None` when the basis was not executed or the action failed.
    pub fn report(&self) -> Option<&JointPhiReport> {
        self.report.as_ref()
    }
}

/// The outcome of [`route_joint_phi_single`]: constructed only by the
/// router, so an admitted route always comes from [`route_admission`] on an
/// executed report.
#[derive(Clone, Debug, Serialize)]
pub struct SingleRoutedPhi {
    policy: RoutePolicy,
    choice: RouteChoice,
    total_budget: f64,
    truncation_budget: f64,
    /// Chebyshev first, then Laguerre (Chebyshev only for
    /// [`RoutePolicy::ChebyshevOnly`]).
    plans: Vec<SinglePlan>,
    preflight: Preflight,
    /// In execution order.
    executions: Vec<SingleExecution>,
    /// The running total of every plan, the preflight and every execution.
    cost: RouteCost,
    /// The change of the caller's counters during the route.
    work: WorkCounters,
    fused: Option<Vec<f64>>,
    bound: Option<f64>,
}

impl SingleRoutedPhi {
    pub fn policy(&self) -> RoutePolicy {
        self.policy
    }

    pub fn choice(&self) -> &RouteChoice {
        &self.choice
    }

    /// The admitted basis; `None` for a fallback.
    pub fn basis(&self) -> Option<PolynomialBasis> {
        match self.choice {
            RouteChoice::Chebyshev => Some(PolynomialBasis::Chebyshev),
            RouteChoice::Laguerre => Some(PolynomialBasis::Laguerre),
            RouteChoice::Fallback { .. } => None,
        }
    }

    pub fn is_fallback(&self) -> bool {
        matches!(self.choice, RouteChoice::Fallback { .. })
    }

    pub fn total_budget(&self) -> f64 {
        self.total_budget
    }

    pub fn truncation_budget(&self) -> f64 {
        self.truncation_budget
    }

    pub fn plans(&self) -> &[SinglePlan] {
        &self.plans
    }

    pub fn plan(&self, basis: PolynomialBasis) -> Option<&SinglePlan> {
        self.plans.iter().find(|p| p.basis == basis)
    }

    pub fn preflight(&self) -> &Preflight {
        &self.preflight
    }

    pub fn executions(&self) -> &[SingleExecution] {
        &self.executions
    }

    /// The cost of every plan and of the preflight.
    pub fn plan_cost(&self) -> RouteCost {
        let mut total = self.preflight.cost;
        for plan in &self.plans {
            total.accumulate(plan.cost);
        }
        total
    }

    /// The running total the router charged.
    pub fn cost(&self) -> RouteCost {
        self.cost
    }

    /// The change of the caller's counters.
    pub fn work(&self) -> WorkCounters {
        self.work
    }

    /// The admitted fused result; `None` for a fallback.
    pub fn fused(&self) -> Option<&[f64]> {
        self.fused.as_deref()
    }

    /// The admitted total bound, at most [`Self::total_budget`].
    pub fn bound(&self) -> Option<f64> {
        self.bound
    }

    /// The admitted execution's report.
    pub fn report(&self) -> Option<&JointPhiReport> {
        let basis = self.basis()?;
        self.executions
            .iter()
            .find(|e| e.basis == basis && e.is_admitted())
            .and_then(SingleExecution::report)
    }
}

fn check_route_inputs(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    total_budget: f64,
) -> CoreResult<()> {
    if !(total_budget.is_finite() && total_budget >= 0.0) {
        return Err(invalid(format!(
            "total budget {total_budget:e} outside [0, inf)"
        )));
    }
    if !(h.is_finite() && h >= 0.0) {
        return Err(invalid(format!("step h = {h:e} must be finite and >= 0")));
    }
    let n = op.dimension();
    let finite_input = match input {
        JointPhiInput::Distinct(vectors) => {
            vectors.iter().all(|w| w.len() == n) && vectors.iter().flatten().all(|x| x.is_finite())
        }
        JointPhiInput::SameVector { vector, scales } => {
            vector.len() == n && vector.iter().chain(&scales).all(|x| x.is_finite())
        }
    };
    if !finite_input {
        return Err(invalid(
            "every input vector needs the operator dimension and finite entries",
        ));
    }
    Ok(())
}

fn rejected(reason: String) -> TotalErrorAdmission {
    TotalErrorAdmission::Rejected { reason }
}

/// Route `sum_k phi_k(h A) w_k` under the absolute `total_budget` by
/// planning first and executing one backend (preregistered rule of
/// `research/py03_single_backend_router_20261011`). Opt-in and research
/// only; [`route_joint_phi`] and every default dispatch are unchanged.
///
/// 1. **Plan** both bases at the truncation budget `total_budget / 4`
///    ([`plan_joint_phi`]; Chebyshev only for [`RoutePolicy::ChebyshevOnly`]).
/// 2. **Guard.** An unverified enclosure cannot pass [`route_admission`]:
///    nothing executes and the route is a fallback.
/// 3. **Preflight** ([`RoutePolicy::ChebyshevFirstPreflight`]). Laguerre is
///    preselected only if its plan needs strictly fewer vector products
///    (ties to Chebyshev), its degree is at most
///    [`crate::laguerre_adjoint::LAGUERRE_ADJOINT_DEGREE_LIMIT`], and its
///    vector-independent terms ([`laguerre_preflight_terms`], which builds
///    and charges the table) are at most `total_budget`; otherwise the
///    preflight abstains to Chebyshev.
/// 4. **Execute** the preselected basis and admit it by [`route_admission`];
///    on rejection execute the other, charged, and admit it; with neither
///    admitted the route is [`RouteChoice::Fallback`] with both reasons.
///
/// The coefficient tables and envelopes come from `caches`. Every plan, the
/// preflight and every execution (failed ones included) are charged to the
/// route's [`RouteCost`] and their counted work to `work`. A NaN, negative
/// or infinite budget, a negative or non-finite `h` and a malformed input
/// are errors before any work; a zero budget is valid and ends in a
/// fallback.
pub fn route_joint_phi_single(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    total_budget: f64,
    policy: RoutePolicy,
    caches: &mut PolynomialCaches,
    work: &mut WorkCounters,
) -> CoreResult<SingleRoutedPhi> {
    check_route_inputs(op, h, input, total_budget)?;
    let work_before = *work;
    let truncation_budget = total_budget * ROUTER_TRUNCATION_FRACTION;
    let mut cost = RouteCost::default();

    // 1. Plans.
    let bases: &[PolynomialBasis] = match policy {
        RoutePolicy::ChebyshevOnly => &[PolynomialBasis::Chebyshev],
        _ => &[PolynomialBasis::Chebyshev, PolynomialBasis::Laguerre],
    };
    let mut plans = Vec::with_capacity(bases.len());
    for &basis in bases {
        let before = cost;
        let result = plan_joint_phi(op, h, input, basis, truncation_budget, caches, &mut cost);
        let delta = cost
            .checked_delta(before)
            .ok_or_else(|| invalid("a cost decreased during a plan"))?;
        let (plan, error) = match result {
            Ok(plan) => (Some(plan), None),
            Err(error) => (None, Some(error.to_string())),
        };
        plans.push(SinglePlan {
            basis,
            record: plan.as_ref().map(ActionPlan::record),
            error,
            cost: delta,
            plan,
        });
    }
    let find = |basis: PolynomialBasis| plans.iter().find(|p| p.basis == basis);
    let products =
        |basis| find(basis).and_then(|p| p.plan.as_ref().map(|p| p.predicted_vector_products()));

    // 2-3. Guard and preflight.
    let preflight_cost_before = cost;
    let preflight_work_before = *work;
    let mut preflight = Preflight {
        policy,
        preselected: None,
        abstain_reason: None,
        chebyshev_products: products(PolynomialBasis::Chebyshev),
        laguerre_products: products(PolynomialBasis::Laguerre),
        laguerre_degree: find(PolynomialBasis::Laguerre)
            .and_then(|p| p.plan.as_ref().map(ActionPlan::degree)),
        laguerre_table_used: false,
        vector_independent_terms: None,
        cost: RouteCost::default(),
        work: WorkCounters::default(),
    };
    let verified = matches!(op.enclosure().evidence, EnclosureEvidence::Gershgorin);
    if !verified {
        preflight.abstain_reason = Some(format!(
            "{ROUTER_ENCLOSURE_NOT_VERIFIED}: the spectral enclosure is declared, so no execution could be admitted"
        ));
    } else {
        preflight.preselected = Some(match policy {
            RoutePolicy::ChebyshevOnly => PolynomialBasis::Chebyshev,
            RoutePolicy::ForceLaguerreFirst => PolynomialBasis::Laguerre,
            RoutePolicy::ChebyshevFirstPreflight => {
                let chebyshev = find(PolynomialBasis::Chebyshev).expect("planned");
                let laguerre = find(PolynomialBasis::Laguerre).expect("planned");
                let abstain = match (&chebyshev.plan, &laguerre.plan) {
                    (_, None) => Some(format!(
                        "{PREFLIGHT_LAGUERRE_PLAN_FAILED}: {}",
                        laguerre.error.as_deref().unwrap_or_default()
                    )),
                    (None, Some(_)) => Some(format!(
                        "{PREFLIGHT_CHEBYSHEV_PLAN_FAILED}: {}",
                        chebyshev.error.as_deref().unwrap_or_default()
                    )),
                    (Some(c), Some(l)) => {
                        let (pc, pl) =
                            (c.predicted_vector_products(), l.predicted_vector_products());
                        if pl >= pc {
                            Some(format!(
                                "{PREFLIGHT_PRODUCTS_NOT_FEWER}: Laguerre {pl} >= Chebyshev {pc}"
                            ))
                        } else if l.degree()
                            > crate::laguerre_adjoint::LAGUERRE_ADJOINT_DEGREE_LIMIT
                        {
                            Some(format!(
                                "{PREFLIGHT_DEGREE_ABOVE_LIMIT}: degree {} above {}",
                                l.degree(),
                                crate::laguerre_adjoint::LAGUERRE_ADJOINT_DEGREE_LIMIT
                            ))
                        } else {
                            preflight.laguerre_table_used = true;
                            match laguerre_preflight_terms(l, op, caches, work, &mut cost) {
                                Ok(terms) => {
                                    preflight.vector_independent_terms = Some(terms);
                                    // A NaN sum is not within the budget either.
                                    if terms <= total_budget {
                                        None
                                    } else {
                                        Some(format!(
                                            "{PREFLIGHT_TERMS_ABOVE_BUDGET}: {terms:e} > {total_budget:e}"
                                        ))
                                    }
                                }
                                Err(error) => {
                                    Some(format!("{PREFLIGHT_TERMS_ABOVE_BUDGET}: {error}"))
                                }
                            }
                        }
                    }
                };
                let laguerre_first = abstain.is_none();
                preflight.abstain_reason = abstain;
                if laguerre_first {
                    PolynomialBasis::Laguerre
                } else {
                    PolynomialBasis::Chebyshev
                }
            }
        });
    }
    preflight.cost = cost
        .checked_delta(preflight_cost_before)
        .ok_or_else(|| invalid("a cost decreased during the preflight"))?;
    preflight.work = work
        .checked_delta(preflight_work_before)
        .ok_or_else(|| invalid("a work counter decreased during the preflight"))?;

    // 4. Executions.
    let order: Vec<PolynomialBasis> = match (policy, preflight.preselected) {
        (_, None) => Vec::new(),
        (RoutePolicy::ChebyshevOnly, Some(_)) => vec![PolynomialBasis::Chebyshev],
        (_, Some(PolynomialBasis::Chebyshev)) => {
            vec![PolynomialBasis::Chebyshev, PolynomialBasis::Laguerre]
        }
        (_, Some(PolynomialBasis::Laguerre)) => {
            vec![PolynomialBasis::Laguerre, PolynomialBasis::Chebyshev]
        }
    };
    let mut executions: Vec<SingleExecution> = Vec::new();
    for basis in order {
        let entry = find(basis).expect("planned");
        let cost_before = cost;
        let work_before_execution = *work;
        let (executed, report, admission) = match &entry.plan {
            None => (
                false,
                None,
                rejected(format!(
                    "{ROUTER_PLAN_FAILED}: {}",
                    entry.error.as_deref().unwrap_or_default()
                )),
            ),
            Some(plan) => {
                match execute_joint_phi_counted(plan, op, h, input, caches, work, &mut cost) {
                    Ok(report) => {
                        let admission = route_admission(&report, total_budget);
                        (true, Some(report), admission)
                    }
                    Err(error) => (
                        true,
                        None,
                        rejected(format!("{ROUTER_NOT_ADMITTED}: action failed: {error}")),
                    ),
                }
            }
        };
        let admitted = matches!(admission, TotalErrorAdmission::Admitted { .. });
        if !admitted {
            cost.failed_executions += 1;
        }
        executions.push(SingleExecution {
            basis,
            executed,
            degree: report.as_ref().map(|r| r.degree),
            laguerre_scale: report.as_ref().and_then(|r| r.laguerre_scale),
            admission,
            cost: cost
                .checked_delta(cost_before)
                .ok_or_else(|| invalid("a cost decreased during an execution"))?,
            work: work
                .checked_delta(work_before_execution)
                .ok_or_else(|| invalid("a work counter decreased during an execution"))?,
            report,
        });
        if admitted {
            break;
        }
    }

    let admitted = executions.iter().find(|e| e.is_admitted());
    let reason = |basis: PolynomialBasis| -> String {
        if let Some(reason) = preflight
            .abstain_reason
            .as_deref()
            .filter(|_| preflight.preselected.is_none())
        {
            return reason.to_string();
        }
        executions
            .iter()
            .find(|e| e.basis == basis)
            .and_then(SingleExecution::rejection_reason)
            .map_or_else(
                || format!("{ROUTER_NOT_ATTEMPTED}: {basis:?} was not executed"),
                str::to_string,
            )
    };
    let choice = match admitted.map(|e| e.basis) {
        Some(PolynomialBasis::Chebyshev) => RouteChoice::Chebyshev,
        Some(PolynomialBasis::Laguerre) => RouteChoice::Laguerre,
        None => RouteChoice::Fallback {
            chebyshev_reason: reason(PolynomialBasis::Chebyshev),
            laguerre_reason: reason(PolynomialBasis::Laguerre),
        },
    };
    let fused = admitted.and_then(|e| e.report.as_ref().map(|r| r.fused.clone()));
    let bound = admitted.and_then(|e| match e.admission {
        TotalErrorAdmission::Admitted { bound, .. } => Some(bound),
        TotalErrorAdmission::Rejected { .. } => None,
    });
    let route_work = work
        .checked_delta(work_before)
        .ok_or_else(|| invalid("a work counter decreased during the route"))?;
    Ok(SingleRoutedPhi {
        policy,
        choice,
        total_budget,
        truncation_budget,
        plans,
        preflight,
        executions,
        cost,
        work: route_work,
        fused,
        bound,
    })
}
