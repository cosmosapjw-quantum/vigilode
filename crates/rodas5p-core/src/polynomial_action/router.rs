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
