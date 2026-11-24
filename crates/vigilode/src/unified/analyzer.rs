//! Preflight analysis: which entry point the router calls first, and why.
//!
//! The analysis reads declared facts and makes no callback. It asks the method registry
//! which method a declared regime selects, whether the explicit method admits the request
//! and which driver family of the RODAS5P facade is planned; it keeps no rule of its own
//! about drivers. A request that declares nothing runs the break-even route
//! ([`super::breakeven`]): DOPRI5 with forward break-even pilots of RODAS5P.
//!
//! The analysis does not classify a problem as stiff or non-stiff. The logarithmic norm is
//! not computed: a contractive problem (the heat equation, `y' = -1e6 y`) is the standard
//! stiff case, not evidence for an explicit method.

use serde::Serialize;

use super::breakeven::break_even_length;
use super::options::{SolverStrategy, VigilodeOptions};
use crate::error::SolveError;
use crate::explicit::ExplicitSolveError;
use crate::problem::Problem;
use crate::{
    JacobianForm, MethodChoice, MethodFacts, MethodRefusalReason, MethodRegistry, MethodRoute,
    MethodRouteRefusal, ProblemRegime,
};

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
    /// use it, and nothing shows that the problem is non-stiff (owner decision D2: the
    /// request starts on RODAS5P).
    SuppliedDerivative,
    /// Nothing is declared: DOPRI5 with forward break-even pilots ([`super::breakeven`]);
    /// RODAS5P continues after an accepted pilot or after a numerical DOPRI5 stop.
    BreakEven {
        /// The break-even length of the dimension ([`super::break_even_length`]).
        b: usize,
    },
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
            Self::BreakEven { b } => format!(
                "no declaration: DOPRI5 with forward break-even pilots of RODAS5P (b = {b})"
            ),
        }
    }
}

/// What the analysis decided. It makes no callback.
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
    /// Attempt budget of the first leg: `Options::max_steps`, the budget of the whole solve.
    pub first_leg_attempts: usize,
    /// Whether RODAS5P may continue from DOPRI5's last accepted state: after an accepted
    /// pilot or a numerical stop of DOPRI5 (the break-even route only).
    pub fallback_to_rodas5p: bool,
}

/// The preflight analysis of the router.
pub struct PreflightAnalyzer;

/// Method id that the registry's general surface selects for a declared regime
/// (`MethodRegistry::general_method`). The catalog is constant, so the two ids are looked
/// up once.
pub(super) fn method_id(regime: ProblemRegime) -> &'static str {
    static IDS: std::sync::OnceLock<[&'static str; 2]> = std::sync::OnceLock::new();
    let lookup = |regime, listed| {
        // the catalog lists one selectable method per regime; `listed` is its id
        MethodRegistry::general_method(regime).map_or(listed, |entry| entry.descriptor.method_id)
    };
    let [explicit, implicit] = *IDS.get_or_init(|| {
        [
            lookup(ProblemRegime::Nonstiff, "dopri5"),
            lookup(ProblemRegime::Stiff, "rodas5p"),
        ]
    });
    if regime == ProblemRegime::Stiff {
        implicit
    } else {
        explicit
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
    /// 5. Otherwise the break-even route ([`RouteBasis::BreakEven`]): DOPRI5 with forward
    ///    break-even pilots of RODAS5P, for every dimension.
    ///
    /// No rule makes a callback. `Err` is a refusal before any callback: malformed input,
    /// or a request that the RODAS5P facade does not support; for the break-even route both
    /// entry points are checked, so that a refusal never follows a callback.
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
        let analysis = |tier, basis, planned_route, fallback| PreflightAnalysis {
            suggested_tier: tier,
            basis,
            facts,
            regime: opts.regime,
            planned_route,
            first_leg_attempts: total,
            fallback_to_rodas5p: fallback,
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
                false,
            ));
        }

        // 4. a supplied derivative or band starts RODAS5P (owner decision D2)
        if facts.jacobian != JacobianForm::FiniteDifference || facts.declared_band {
            return implicit(RouteBasis::SuppliedDerivative);
        }

        // 5. the break-even route. Both entry points are checked before any callback.
        implicit_route()?;
        if let Err(error) = crate::explicit::validate(problem, span, y0, &opts.options) {
            return Err(SolveError::InvalidInput(match error {
                ExplicitSolveError::InvalidInput(message) => message,
                other => other.to_string(),
            }));
        }
        Ok(analysis(
            SelectedTier::Tier1Explicit,
            RouteBasis::BreakEven {
                b: break_even_length(n),
            },
            Some(explicit_route),
            true,
        ))
    }
}
