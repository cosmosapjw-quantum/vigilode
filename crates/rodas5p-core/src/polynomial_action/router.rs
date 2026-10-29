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
    EXECUTION_CERTIFIED, EnclosureEvidence, JointPhiInput, JointPhiReport, PolynomialBasis,
    SymmetricNonpositiveOperator, TotalErrorAdmission, joint_phi_action,
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

/// Routing policy for [`route_joint_phi_single`] (RVJ DAG node PY03).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize)]
#[allow(clippy::manual_non_exhaustive)]
pub enum RoutePolicy {
    /// Preflight compares vector products, degree limit, and vector-independent error before executing.
    #[default]
    ChebyshevFirstPreflight,
    /// Force execution of Laguerre first (for testing failed preflight).
    #[doc(hidden)]
    ForceLaguerreFirst,
}

/// Detailed router-local work accounting (RVJ DAG node PY03).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct RouteCost {
    pub plan_tail_evaluations: u64,
    pub coefficient_setups: u64,
    pub coefficient_cache_hits: u64,
    pub coefficient_series_terms: u64,
    pub executed_backends: u64,
    pub failed_executions: u64,
    pub vector_products: u64,
    pub block_products: u64,
    pub certification_operations: u64,
}

impl RouteCost {
    pub fn accumulate(&mut self, other: RouteCost) {
        self.plan_tail_evaluations = self
            .plan_tail_evaluations
            .saturating_add(other.plan_tail_evaluations);
        self.coefficient_setups = self
            .coefficient_setups
            .saturating_add(other.coefficient_setups);
        self.coefficient_cache_hits = self
            .coefficient_cache_hits
            .saturating_add(other.coefficient_cache_hits);
        self.coefficient_series_terms = self
            .coefficient_series_terms
            .saturating_add(other.coefficient_series_terms);
        self.executed_backends = self
            .executed_backends
            .saturating_add(other.executed_backends);
        self.failed_executions = self
            .failed_executions
            .saturating_add(other.failed_executions);
        self.vector_products = self.vector_products.saturating_add(other.vector_products);
        self.block_products = self.block_products.saturating_add(other.block_products);
        self.certification_operations = self
            .certification_operations
            .saturating_add(other.certification_operations);
    }
}

/// Combined caches for the single backend router (RVJ DAG node PY03).
#[derive(Clone, Debug, Default)]
pub struct RouterCaches {
    pub coefficient: super::CoefficientCache,
    pub envelope: crate::laguerre_adjoint::LaguerreEnvelopeCache,
}

impl RouterCaches {
    pub fn clear(&mut self) {
        self.coefficient.tables.clear();
        self.envelope = crate::laguerre_adjoint::LaguerreEnvelopeCache::default();
    }
}

/// The outcome of [`route_joint_phi_single`] (RVJ DAG node PY03).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SingleRoutedPhi {
    choice: RouteChoice,
    policy: RoutePolicy,
    total_budget: f64,
    truncation_budget: f64,
    chebyshev_plan: Option<super::ActionPlan>,
    laguerre_plan: Option<super::ActionPlan>,
    plan_cost: RouteCost,
    execution_costs: Vec<RouteCost>,
    cost: RouteCost,
    fused: Option<Vec<f64>>,
    bound: Option<f64>,
    report: Option<JointPhiReport>,
    work: WorkCounters,
    first_attempt: Option<RouteAttempt>,
    second_attempt: Option<RouteAttempt>,
}

impl SingleRoutedPhi {
    pub fn choice(&self) -> &RouteChoice {
        &self.choice
    }

    pub fn policy(&self) -> RoutePolicy {
        self.policy
    }

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

    pub fn chebyshev_plan(&self) -> Option<&super::ActionPlan> {
        self.chebyshev_plan.as_ref()
    }

    pub fn laguerre_plan(&self) -> Option<&super::ActionPlan> {
        self.laguerre_plan.as_ref()
    }

    pub fn plan_cost(&self) -> RouteCost {
        self.plan_cost
    }

    pub fn execution_costs(&self) -> &[RouteCost] {
        &self.execution_costs
    }

    pub fn fused(&self) -> Option<&[f64]> {
        self.fused.as_deref()
    }

    pub fn bound(&self) -> Option<f64> {
        self.bound
    }

    pub fn report(&self) -> Option<&JointPhiReport> {
        self.report.as_ref()
    }

    pub fn cost(&self) -> RouteCost {
        self.cost
    }

    pub fn work(&self) -> WorkCounters {
        self.work
    }

    pub fn first_attempt(&self) -> Option<&RouteAttempt> {
        self.first_attempt.as_ref()
    }

    pub fn second_attempt(&self) -> Option<&RouteAttempt> {
        self.second_attempt.as_ref()
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_one_basis(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    basis: PolynomialBasis,
    plan: Option<&super::ActionPlan>,
    truncation_budget: f64,
    total_budget: f64,
    caches: &mut RouterCaches,
    work: &mut WorkCounters,
) -> CoreResult<(RouteAttempt, RouteCost)> {
    let before_work = *work;
    let mut exec_cost = RouteCost {
        executed_backends: 1,
        ..RouteCost::default()
    };
    let report_result = match plan {
        Some(p) => super::execute_joint_phi_with_cost(
            p,
            op,
            h,
            input,
            Some(caches),
            None,
            work,
            &mut exec_cost,
        ),
        None => super::joint_phi_action_with_cost(
            op,
            h,
            input,
            basis,
            truncation_budget,
            Some(caches),
            work,
            &mut exec_cost,
        ),
    };
    let delta = work
        .checked_delta(before_work)
        .ok_or_else(|| invalid("work decreased during execution"))?;
    exec_cost.vector_products = exec_cost
        .vector_products
        .saturating_add(delta.poly_vector_products);
    exec_cost.block_products = exec_cost
        .block_products
        .saturating_add(delta.poly_block_products);

    match report_result {
        Ok(report) => {
            let admission = route_admission(&report, total_budget);
            if !matches!(admission, TotalErrorAdmission::Admitted { .. }) {
                exec_cost.failed_executions = exec_cost.failed_executions.saturating_add(1);
            }
            Ok((
                RouteAttempt {
                    basis,
                    degree: Some(report.degree),
                    laguerre_scale: report.laguerre_scale,
                    admission,
                    vector_products: delta.poly_vector_products,
                    coefficient_setups: delta.poly_coefficient_setups,
                    work: delta,
                    report: Some(report),
                },
                exec_cost,
            ))
        }
        Err(err) => {
            exec_cost.failed_executions = exec_cost.failed_executions.saturating_add(1);
            Ok((
                RouteAttempt {
                    basis,
                    degree: None,
                    laguerre_scale: None,
                    admission: TotalErrorAdmission::Rejected {
                        reason: format!("{ROUTER_NOT_ADMITTED}: action failed: {err}"),
                    },
                    vector_products: delta.poly_vector_products,
                    coefficient_setups: delta.poly_coefficient_setups,
                    work: delta,
                    report: None,
                },
                exec_cost,
            ))
        }
    }
}

/// Route `sum_k phi_k(h A) w_k` by planning first and executing one backend (RVJ DAG node PY03).
pub fn route_joint_phi_single(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    total_budget: f64,
    policy: RoutePolicy,
    caches: Option<&mut RouterCaches>,
    work: &mut WorkCounters,
) -> CoreResult<SingleRoutedPhi> {
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
    if total_budget == 0.0 {
        return Ok(SingleRoutedPhi {
            choice: RouteChoice::Fallback {
                chebyshev_reason: format!(
                    "{ROUTER_NOT_ADMITTED}: plan failed: truncation budget must be positive"
                ),
                laguerre_reason: format!(
                    "{ROUTER_NOT_ADMITTED}: plan failed: truncation budget must be positive"
                ),
            },
            policy,
            total_budget: 0.0,
            truncation_budget: 0.0,
            chebyshev_plan: None,
            laguerre_plan: None,
            plan_cost: RouteCost::default(),
            execution_costs: Vec::new(),
            cost: RouteCost::default(),
            fused: None,
            bound: None,
            report: None,
            work: WorkCounters::default(),
            first_attempt: None,
            second_attempt: None,
        });
    }

    let mut local_caches = RouterCaches::default();
    let caches = match caches {
        Some(c) => c,
        None => &mut local_caches,
    };

    let truncation_budget = total_budget * ROUTER_TRUNCATION_FRACTION;
    let mut plan_cost = RouteCost::default();

    let cheb_plan_res = super::plan_joint_phi(
        op,
        h,
        input,
        PolynomialBasis::Chebyshev,
        truncation_budget,
        Some(caches),
        &mut plan_cost,
    );
    let lag_plan_res = super::plan_joint_phi(
        op,
        h,
        input,
        PolynomialBasis::Laguerre,
        truncation_budget,
        Some(caches),
        &mut plan_cost,
    );

    let preselect_laguerre = match policy {
        RoutePolicy::ForceLaguerreFirst => true,
        RoutePolicy::ChebyshevFirstPreflight => match (&cheb_plan_res, &lag_plan_res) {
            (Ok(cheb), Ok(lag)) => {
                if lag.predicted_vector_products() >= cheb.predicted_vector_products()
                    || lag.degree() > crate::laguerre_adjoint::LAGUERRE_ADJOINT_DEGREE_LIMIT
                {
                    false
                } else {
                    let key = super::CoefficientKey {
                        basis: PolynomialBasis::Laguerre,
                        operator_fingerprint: op.fingerprint.clone(),
                        h_bits: h.to_bits(),
                        degree: lag.degree(),
                        laguerre_scale_bits: lag.laguerre_scale().unwrap_or(0.0).to_bits(),
                    };
                    let (table, is_hit) = if let Some(t) = caches.coefficient.tables.get(&key) {
                        (t.clone(), true)
                    } else {
                        let mut terms = 0;
                        let a = lag.a().ok_or_else(|| {
                            super::unsupported("Laguerre plan missing interval a")
                        })?;
                        let t = super::laguerre_coefficients_counted(a, lag.degree(), &mut terms)?;
                        plan_cost.coefficient_setups =
                            plan_cost.coefficient_setups.saturating_add(1);
                        plan_cost.coefficient_series_terms =
                            plan_cost.coefficient_series_terms.saturating_add(terms);
                        work.poly_coefficient_setups += 1;
                        caches.coefficient.tables.insert(key, t.clone());
                        (t, false)
                    };
                    if is_hit {
                        plan_cost.coefficient_cache_hits =
                            plan_cost.coefficient_cache_hits.saturating_add(1);
                    }
                    let scales = match input {
                        JointPhiInput::Distinct(_) => [1.0; super::JOINT_PHI_TERMS],
                        JointPhiInput::SameVector { scales, .. } => scales,
                    };
                    let mut total_coeff_bound = 0.0;
                    let lag_norm = lag
                        .laguerre_norm()
                        .ok_or_else(|| super::unsupported("Laguerre plan missing laguerre_norm"))?;
                    for k in 0..super::JOINT_PHI_TERMS {
                        let source_norm = match input {
                            JointPhiInput::Distinct(w) => super::norm_up_scaled(&w[k], 0)?,
                            JointPhiInput::SameVector { vector, .. } => {
                                super::norm_up_scaled(vector, 0)?
                            }
                        };
                        let mut coeff_err = 0.0;
                        for row in &table {
                            let chosen_val = super::midpoint(row[k]);
                            let radius = super::distance_up(chosen_val, row[k])?;
                            coeff_err = super::add_up(coeff_err, super::mul_up(radius, lag_norm)?)?;
                        }
                        coeff_err = super::mul_up(coeff_err, source_norm)?;
                        total_coeff_bound = super::add_up(
                            total_coeff_bound,
                            super::mul_up(scales[k].abs(), coeff_err)?,
                        )?;
                    }
                    let vector_independent_terms =
                        super::add_up(lag.truncation_bound(), total_coeff_bound)?;
                    vector_independent_terms <= total_budget
                }
            }
            _ => false,
        },
    };

    let first_basis = if preselect_laguerre {
        PolynomialBasis::Laguerre
    } else {
        PolynomialBasis::Chebyshev
    };
    let second_basis = match first_basis {
        PolynomialBasis::Chebyshev => PolynomialBasis::Laguerre,
        PolynomialBasis::Laguerre => PolynomialBasis::Chebyshev,
    };

    let first_plan = match first_basis {
        PolynomialBasis::Chebyshev => cheb_plan_res.as_ref().ok(),
        PolynomialBasis::Laguerre => lag_plan_res.as_ref().ok(),
    };
    let mut execution_costs = Vec::new();
    let (first_attempt, first_cost) = execute_one_basis(
        op,
        h,
        input,
        first_basis,
        first_plan,
        truncation_budget,
        total_budget,
        caches,
        work,
    )?;
    execution_costs.push(first_cost);

    let (choice, attempts) = if first_attempt.is_admitted() {
        let ch = match first_basis {
            PolynomialBasis::Chebyshev => RouteChoice::Chebyshev,
            PolynomialBasis::Laguerre => RouteChoice::Laguerre,
        };
        (ch, vec![first_attempt])
    } else {
        let second_plan = match second_basis {
            PolynomialBasis::Chebyshev => cheb_plan_res.as_ref().ok(),
            PolynomialBasis::Laguerre => lag_plan_res.as_ref().ok(),
        };
        let (second_attempt, second_cost) = execute_one_basis(
            op,
            h,
            input,
            second_basis,
            second_plan,
            truncation_budget,
            total_budget,
            caches,
            work,
        )?;
        execution_costs.push(second_cost);
        let ch = if second_attempt.is_admitted() {
            match second_basis {
                PolynomialBasis::Chebyshev => RouteChoice::Chebyshev,
                PolynomialBasis::Laguerre => RouteChoice::Laguerre,
            }
        } else {
            let (cheb_reason, lag_reason) = match first_basis {
                PolynomialBasis::Chebyshev => (
                    first_attempt
                        .rejection_reason()
                        .unwrap_or_default()
                        .to_string(),
                    second_attempt
                        .rejection_reason()
                        .unwrap_or_default()
                        .to_string(),
                ),
                PolynomialBasis::Laguerre => (
                    second_attempt
                        .rejection_reason()
                        .unwrap_or_default()
                        .to_string(),
                    first_attempt
                        .rejection_reason()
                        .unwrap_or_default()
                        .to_string(),
                ),
            };
            RouteChoice::Fallback {
                chebyshev_reason: cheb_reason,
                laguerre_reason: lag_reason,
            }
        };
        (ch, vec![first_attempt, second_attempt])
    };

    let chosen_attempt = match choice {
        RouteChoice::Chebyshev => attempts
            .iter()
            .find(|a| a.basis() == PolynomialBasis::Chebyshev),
        RouteChoice::Laguerre => attempts
            .iter()
            .find(|a| a.basis() == PolynomialBasis::Laguerre),
        RouteChoice::Fallback { .. } => None,
    };

    let fused = chosen_attempt.and_then(|a| a.report().map(|r| r.fused.clone()));
    let bound = chosen_attempt.and_then(RouteAttempt::bound);
    let report = chosen_attempt.and_then(|a| a.report().cloned());
    let total_work = attempts.iter().fold(WorkCounters::default(), |mut acc, a| {
        acc.accumulate(a.work());
        acc
    });

    let mut total_cost = plan_cost;
    for c in &execution_costs {
        total_cost.accumulate(*c);
    }

    Ok(SingleRoutedPhi {
        choice,
        policy,
        total_budget,
        truncation_budget,
        chebyshev_plan: cheb_plan_res.ok(),
        laguerre_plan: lag_plan_res.ok(),
        plan_cost,
        execution_costs,
        cost: total_cost,
        fused,
        bound,
        report,
        work: total_work,
        first_attempt: attempts.first().cloned(),
        second_attempt: attempts.get(1).cloned(),
    })
}
