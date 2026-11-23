//! Preflight analysis: which entry point the router calls first, and why.
//!
//! The analysis reads declared facts. It asks the method registry which method a declared
//! regime selects, whether the explicit method admits the request and which driver family
//! of the RODAS5P facade is planned; it keeps no rule of its own about drivers. It makes
//! callbacks in one case only, the stiffness screen of a small problem that declares
//! nothing, and reports their number so that the router charges them.
//!
//! The analysis does not classify a problem as non-stiff. The logarithmic norm is not
//! computed: a contractive problem (the heat equation, `y' = -1e6 y`) is the standard
//! stiff case, not evidence for an explicit method.

use serde::Serialize;

use super::fallback::FallbackEngine;
use super::legs::CallbackStop;
use super::options::{SolverStrategy, VigilodeOptions};
use crate::error::{CallbackError, SolveError};
use crate::explicit::ExplicitSolveError;
use crate::problem::Problem;
use crate::{
    Callback, JacobianForm, MethodChoice, MethodFacts, MethodRefusalReason, MethodRegistry,
    MethodRoute, MethodRouteRefusal, ProblemRegime,
};

/// Largest dimension for which a problem that declares nothing gets an explicit trial.
/// Above it the request runs RODAS5P directly (the limit of the first unified router).
pub const EXPLICIT_TRIAL_MAX_DIMENSION: usize = 10;

/// Length of the negative real stability interval of DOPRI5, to two digits: an explicit
/// step `h` is stable on a decaying mode `lambda` only while `h |lambda| <= 3.3`.
pub const DOPRI5_REAL_STABILITY_BOUND: f64 = 3.3;

/// The entry point the router calls first. Each value has a driver of its own.
///
/// The first unified router also had `Tier2MatrixFreeKrylov`, `Tier3Exponential` and
/// `Tier4StagedRodas5p`. They named no driver: all of them ran `Rodas5p::solve`, which
/// chooses its driver family (dense, banded, matrix-free, with a continuous extension,
/// sequential) from the declared structure. They are removed; the family that ran is in
/// the run record ([`super::UnifiedLeg::driver_id`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SelectedTier {
    /// `Dopri5::solve`: Dormand-Prince 5(4), right-hand side only.
    Tier1Explicit,
    /// `Rodas5p::solve`: the protected RODAS5P facade with the driver family it selects.
    TierProtectedReference,
    /// `SwitchingSolver::solve`: windows of DOPRI5 and RODAS5P.
    Tier5AdaptiveSwitching,
}

impl SelectedTier {
    /// Every value.
    pub const ALL: [SelectedTier; 3] = [
        SelectedTier::Tier1Explicit,
        SelectedTier::TierProtectedReference,
        SelectedTier::Tier5AdaptiveSwitching,
    ];

    /// The facade entry point that the router calls for this value.
    pub fn entry_point(self) -> &'static str {
        match self {
            Self::Tier1Explicit => "vigilode::Dopri5::solve",
            Self::TierProtectedReference => "vigilode::Rodas5p::solve",
            Self::Tier5AdaptiveSwitching => "vigilode::SwitchingSolver::solve",
        }
    }
}

/// Why the analysis chose its entry point.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
#[non_exhaustive]
pub enum RouteBasis {
    /// The strategy names the entry point ([`SolverStrategy::RobustStiff`],
    /// [`SolverStrategy::AdaptiveReentrant`]).
    Strategy {
        /// The strategy.
        strategy: SolverStrategy,
    },
    /// The caller declared the regime ([`VigilodeOptions::regime`]); the method is the one
    /// the registry selects for it.
    DeclaredRegime {
        /// The declared regime.
        regime: ProblemRegime,
    },
    /// The registry does not admit the explicit method for the declared facts (a mass
    /// matrix, dense output, events), so RODAS5P is the only candidate.
    ExplicitNotAdmitted {
        /// The registry's reasons.
        reasons: Vec<MethodRefusalReason>,
    },
    /// The caller supplied a Jacobian, a Jacobian-vector product or a band. RODAS5P can
    /// use it, and nothing shows that the problem is non-stiff.
    SuppliedDerivative,
    /// Nothing is declared and the dimension is above [`EXPLICIT_TRIAL_MAX_DIMENSION`].
    DimensionAboveTrialLimit {
        /// The dimension.
        dimension: usize,
        /// The limit.
        limit: usize,
    },
    /// Nothing is declared and the attempt budget leaves no explicit trial.
    NoBudgetForTrial {
        /// `Options::max_steps`.
        max_steps: usize,
    },
    /// Nothing is declared and the stiffness screen found that an explicit trial cannot
    /// cover the span within its attempt budget ([`StiffnessScreen`]).
    StiffnessScreen,
    /// Nothing is declared and the screen found nothing: a budgeted explicit trial, then
    /// RODAS5P from its last accepted state if it does not finish.
    UndeclaredRegimeTrial,
    /// A callback of the stiffness screen failed fatally; no leg is started.
    PreflightCallbackFailed,
}

impl RouteBasis {
    /// One sentence for the route history.
    pub fn describe(&self) -> String {
        match self {
            Self::Strategy { strategy } => format!("strategy {strategy:?}"),
            Self::DeclaredRegime { regime } => format!("declared regime {regime:?}"),
            Self::ExplicitNotAdmitted { reasons } => {
                format!("the explicit method is not admitted for the declared facts: {reasons:?}")
            }
            Self::SuppliedDerivative => {
                "a derivative or a band is supplied and no regime is declared".into()
            }
            Self::DimensionAboveTrialLimit { dimension, limit } => format!(
                "no declaration; dimension {dimension} is above the explicit trial limit {limit}"
            ),
            Self::NoBudgetForTrial { max_steps } => {
                format!("no declaration; an attempt budget of {max_steps} leaves no explicit trial")
            }
            Self::StiffnessScreen => {
                "no declaration; the charged stiffness screen shows that an explicit trial \
                 cannot cover the span within its attempt budget"
                    .into()
            }
            Self::UndeclaredRegimeTrial => {
                "no declaration and no evidence of stiffness at t0: budgeted explicit trial".into()
            }
            Self::PreflightCallbackFailed => {
                "a callback of the stiffness screen failed fatally".into()
            }
        }
    }
}

/// The stiffness screen: a lower bound of the Lipschitz constant of the right-hand side
/// at the initial point, compared with what an explicit trial can cover.
///
/// With `f0 = f(t0, y0)` and, for every coordinate `j`,
/// `delta_j = sqrt(eps) max(|y0_j|, 1)`, the screen forms
/// `L = max_j |f(t0, y0 + delta_j e_j) - f0|_2 / delta_j`. Every quotient is a difference
/// quotient of `f` along a segment, so `L` is a lower bound of the Lipschitz constant of
/// `f(t0, .)` near `y0`, up to rounding (for the linearisation, `L >= |J|_2 / sqrt(n)`).
/// DOPRI5 needs about `L (t1 - t0) / 3.3` steps for stability alone when `L` belongs to a
/// decaying mode. If that exceeds the attempt budget `B` of the trial,
/// `L (t1 - t0) > 3.3 B`, the trial is skipped and RODAS5P runs.
///
/// The screen is one-sided. It can move a request from the explicit trial to RODAS5P,
/// never the other way, and a negative result is not evidence of non-stiffness: stiffness
/// that develops after `t0` (Robertson) is left to the trial budget and the fallback. A
/// strongly oscillatory or non-normal problem can be moved to RODAS5P although DOPRI5
/// would have finished; the result is still error controlled.
///
/// Cost: `n + 1` right-hand-side calls and `O(n)` storage, with `n` at most
/// [`EXPLICIT_TRIAL_MAX_DIMENSION`]. Every call is charged.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct StiffnessScreen {
    /// `L`, when every difference quotient was formed and is finite.
    pub lipschitz_lower_bound: Option<f64>,
    /// `t1 - t0`.
    pub span: f64,
    /// Attempt budget `B` of the explicit trial.
    pub trial_attempts: usize,
    /// `3.3 B`.
    pub threshold: f64,
    /// `L (t1 - t0) > 3.3 B`.
    pub stiffness_evidence: bool,
    /// Whether every quotient was formed. A recoverable callback error or a non-finite
    /// value ends the screen early; it then shows nothing.
    pub completed: bool,
    /// Right-hand-side calls the screen made.
    pub rhs_calls: u64,
}

/// What the analysis decided and what it cost.
#[derive(Clone, Debug, Serialize)]
pub struct PreflightAnalysis {
    /// The entry point the router calls first.
    pub suggested_tier: SelectedTier,
    /// Why.
    pub basis: RouteBasis,
    /// The declared facts the registry routed on (no callback).
    pub facts: MethodFacts,
    /// The declared regime.
    pub regime: ProblemRegime,
    /// The registry's plan for the first leg, from declared facts; `None` for the
    /// switching adapter, which the registry does not list. It is a plan: the route that
    /// ran is read from the run record after the leg.
    pub planned_route: Option<MethodRoute>,
    /// Attempt budget of the first leg.
    pub first_leg_attempts: usize,
    /// Whether RODAS5P continues from the last accepted state when the explicit leg stops
    /// for a numerical reason.
    pub fallback_to_rodas5p: bool,
    /// The stiffness screen, when it ran.
    pub screen: Option<StiffnessScreen>,
    /// Right-hand-side calls the analysis made (those of the screen). The router adds
    /// them to the counters of the solve.
    pub rhs_calls: u64,
    /// The fatal error of a screen callback. With it no leg is started.
    pub callback_failure: Option<CallbackStop>,
}

/// The preflight analysis of the router.
pub struct PreflightAnalyzer;

/// Method id that the registry's general surface selects for a declared regime.
pub(super) fn method_id(regime: ProblemRegime) -> &'static str {
    match MethodRegistry::general_method(regime) {
        Some(entry) => entry.descriptor.method_id,
        // the catalog lists one selectable method per regime; these are their ids
        None if regime == ProblemRegime::Stiff => "rodas5p",
        None => "dopri5",
    }
}

/// A refusal of the registry as the facade error it stands for.
pub(super) fn refusal_error(refusal: MethodRouteRefusal) -> SolveError {
    if let MethodRouteRefusal::Method { reasons, .. } = &refusal {
        for reason in reasons {
            if let MethodRefusalReason::Implicit(unsupported) = reason {
                return SolveError::Unsupported(*unsupported);
            }
        }
    }
    SolveError::InvalidInput(refusal.to_string())
}

impl PreflightAnalyzer {
    /// Decide the first entry point for `(problem, span, y0, opts)`.
    ///
    /// The rules, in order; the first that applies decides:
    ///
    /// 1. [`SolverStrategy::RobustStiff`] runs RODAS5P;
    ///    [`SolverStrategy::AdaptiveReentrant`] runs the switching adapter.
    /// 2. A declared regime ([`VigilodeOptions::regime`]): `Stiff` runs RODAS5P;
    ///    `Nonstiff` runs DOPRI5 with the whole attempt budget and no second method, if
    ///    the registry admits it.
    /// 3. The registry does not admit DOPRI5 for the declared facts (a mass matrix, dense
    ///    output, events): RODAS5P.
    /// 4. A Jacobian, a Jacobian-vector product or a band is supplied: RODAS5P.
    /// 5. The dimension is above [`EXPLICIT_TRIAL_MAX_DIMENSION`], or the attempt budget
    ///    leaves no trial: RODAS5P with a finite-difference Jacobian.
    /// 6. Otherwise the [`StiffnessScreen`]. With evidence of stiffness RODAS5P runs;
    ///    without it DOPRI5 gets a trial of
    ///    [`FallbackEngine::explicit_trial_attempts`] attempts, and RODAS5P continues from
    ///    its last accepted state if it does not finish.
    ///
    /// Rules 1 to 5 make no callback. `Err` is a refusal before any callback: malformed
    /// input, or a request that the RODAS5P facade does not support.
    pub fn analyze(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        opts: &VigilodeOptions,
    ) -> Result<PreflightAnalysis, SolveError> {
        let n = problem.dimension();
        if n != y0.len() {
            return Err(SolveError::InvalidInput(format!(
                "initial state length {} does not match problem dimension {}",
                y0.len(),
                n
            )));
        }
        if !span.0.is_finite() || !span.1.is_finite() || span.1 <= span.0 {
            return Err(SolveError::InvalidInput(format!(
                "invalid integration interval ({}, {})",
                span.0, span.1
            )));
        }

        let facts = MethodFacts::of(problem, &opts.options);
        let total = opts.options.max_steps;
        let analysis = |tier, basis, planned_route, attempts, fallback| PreflightAnalysis {
            suggested_tier: tier,
            basis,
            facts,
            regime: opts.regime,
            planned_route,
            first_leg_attempts: attempts,
            fallback_to_rodas5p: fallback,
            screen: None,
            rhs_calls: 0,
            callback_failure: None,
        };
        let implicit_route = || {
            let choice = MethodChoice::Explicit(method_id(ProblemRegime::Stiff));
            MethodRegistry::route(problem, &opts.options, &choice)
                .map(|plan| plan.route)
                .map_err(refusal_error)
        };
        let implicit = |basis| -> Result<PreflightAnalysis, SolveError> {
            let route = implicit_route()?;
            Ok(analysis(
                SelectedTier::TierProtectedReference,
                basis,
                Some(route),
                total,
                false,
            ))
        };

        // 1. the strategy names the entry point
        match opts.strategy {
            SolverStrategy::RobustStiff => {
                return implicit(RouteBasis::Strategy {
                    strategy: opts.strategy,
                });
            }
            SolverStrategy::AdaptiveReentrant => {
                return Ok(analysis(
                    SelectedTier::Tier5AdaptiveSwitching,
                    RouteBasis::Strategy {
                        strategy: opts.strategy,
                    },
                    None,
                    total,
                    false,
                ));
            }
            SolverStrategy::Auto | SolverStrategy::HighPerformance => {}
        }

        // 2. a declared regime
        let declared = RouteBasis::DeclaredRegime {
            regime: opts.regime,
        };
        if opts.regime == ProblemRegime::Stiff {
            return implicit(declared);
        }

        // 3. what the explicit method admits
        let explicit_choice = MethodChoice::Explicit(method_id(ProblemRegime::Nonstiff));
        let explicit_route = match MethodRegistry::route(problem, &opts.options, &explicit_choice) {
            Ok(plan) => plan.route,
            Err(MethodRouteRefusal::Method { reasons, .. }) => {
                return implicit(RouteBasis::ExplicitNotAdmitted { reasons });
            }
            Err(_) => {
                return implicit(RouteBasis::ExplicitNotAdmitted {
                    reasons: Vec::new(),
                });
            }
        };
        if opts.regime == ProblemRegime::Nonstiff {
            return Ok(analysis(
                SelectedTier::Tier1Explicit,
                declared,
                Some(explicit_route),
                total,
                false,
            ));
        }

        // 4. and 5.: no regime is declared
        if facts.jacobian != JacobianForm::FiniteDifference || facts.declared_band {
            return implicit(RouteBasis::SuppliedDerivative);
        }
        if n > EXPLICIT_TRIAL_MAX_DIMENSION {
            return implicit(RouteBasis::DimensionAboveTrialLimit {
                dimension: n,
                limit: EXPLICIT_TRIAL_MAX_DIMENSION,
            });
        }
        let trial = FallbackEngine::explicit_trial_attempts(total);
        if trial == 0 {
            return implicit(RouteBasis::NoBudgetForTrial { max_steps: total });
        }

        // 6. the screen. Its callbacks are made only for a request that both methods
        // accept, so that a refusal never follows a callback.
        let fallback_route = implicit_route()?;
        let mut trial_options = opts.options.clone();
        trial_options.max_steps = trial;
        if let Err(error) = crate::explicit::validate(problem, span, y0, &trial_options) {
            return Err(SolveError::InvalidInput(match error {
                ExplicitSolveError::InvalidInput(message) => message,
                other => other.to_string(),
            }));
        }
        let (screen, failure) = stiffness_screen(problem, span, y0, trial);
        let mut decided = if failure.is_some() {
            analysis(
                SelectedTier::Tier1Explicit,
                RouteBasis::PreflightCallbackFailed,
                Some(explicit_route),
                0,
                false,
            )
        } else if screen.stiffness_evidence {
            analysis(
                SelectedTier::TierProtectedReference,
                RouteBasis::StiffnessScreen,
                Some(fallback_route),
                total,
                false,
            )
        } else {
            analysis(
                SelectedTier::Tier1Explicit,
                RouteBasis::UndeclaredRegimeTrial,
                Some(explicit_route),
                trial,
                true,
            )
        };
        decided.screen = Some(screen);
        decided.rhs_calls = screen.rhs_calls;
        decided.callback_failure = failure;
        Ok(decided)
    }
}

/// The [`StiffnessScreen`] of `(problem, span, y0)` for a trial of `trial_attempts`
/// attempts, and the fatal callback error that ended it, if any.
fn stiffness_screen(
    problem: &Problem,
    span: (f64, f64),
    y0: &[f64],
    trial_attempts: usize,
) -> (StiffnessScreen, Option<CallbackStop>) {
    let n = y0.len();
    let length = span.1 - span.0;
    let threshold = DOPRI5_REAL_STABILITY_BOUND * trial_attempts as f64;
    let mut screen = StiffnessScreen {
        lipschitz_lower_bound: None,
        span: length,
        trial_attempts,
        threshold,
        stiffness_evidence: false,
        completed: false,
        rhs_calls: 0,
    };
    let rhs = problem.rhs_fn();
    // One call of the right-hand side; `Ok(true)` if the output is finite.
    let call = |y: &[f64], out: &mut [f64], calls: &mut u64| {
        out.fill(f64::NAN);
        *calls += 1;
        match rhs(span.0, y, out) {
            Ok(()) => Ok(out.iter().all(|v| v.is_finite())),
            Err(CallbackError::Recoverable(_)) => Ok(false),
            Err(CallbackError::Fatal(message)) => Err(CallbackStop {
                callback: Callback::Rhs,
                t: span.0,
                message,
            }),
        }
    };

    let mut f0 = vec![0.0; n];
    match call(y0, &mut f0, &mut screen.rhs_calls) {
        Ok(true) => {}
        Ok(false) => return (screen, None),
        Err(failure) => return (screen, Some(failure)),
    }
    let mut shifted = y0.to_vec();
    let mut f1 = vec![0.0; n];
    let mut bound = 0.0f64;
    for j in 0..n {
        let delta = f64::EPSILON.sqrt() * y0[j].abs().max(1.0);
        shifted[j] = y0[j] + delta;
        // the represented increment, as the finite-difference Jacobian of the facade takes it
        let step = shifted[j] - y0[j];
        let outcome = call(&shifted, &mut f1, &mut screen.rhs_calls);
        shifted[j] = y0[j];
        match outcome {
            Ok(true) => {}
            Ok(false) => return (screen, None),
            Err(failure) => return (screen, Some(failure)),
        }
        let difference = f1
            .iter()
            .zip(&f0)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt();
        bound = bound.max(difference / step.abs());
    }
    if bound.is_finite() {
        screen.completed = true;
        screen.lipschitz_lower_bound = Some(bound);
        screen.stiffness_evidence = bound * length > threshold;
    }
    (screen, None)
}
