//! One entry point over the method registry (research loop L1, node V03).
//!
//! * [`MethodRegistry::route`] turns `(problem, options, choice)` into a typed
//!   [`MethodRoutePlan`] or a typed [`MethodRouteRefusal`]. It reads declared facts only and
//!   never enters user code.
//! * [`MethodRegistry::solve`] executes a plan through the adapter of the planned method.
//!   Every adapter is the direct call that [`MethodEntry::direct_call`] names, so its
//!   numbers are those of that call bit for bit.
//! * [`MethodSolution::view`] is a borrowed, method-independent [`SolutionView`]: times,
//!   states and their output grid, work counters, the executed route, and after a failure
//!   the typed reason with the accepted prefix.
//!
//! Nothing here is a default. `GeneralOde`, `Rodas5p` and every other frontend keep their
//! behaviour; [`MethodChoice::General`] returns the decision of `GeneralOde::select`, and
//! [`MethodChoice::Auto`] runs only when a caller asks for it.

use std::fmt;

use rodas5p_integrators::{Dopri5StopReason, ppsw1};
use serde::Serialize;

use super::adapters::{MethodObservedRun, ObservedDriver, run_observed};
use super::{
    CapabilitySet, GeneralOde, GeneralOdePolicy, GeneralSelection, MethodAccess, MethodAdmission,
    MethodDescriptor, MethodPhase, MethodQualification, MethodRegistry, PeerFailure, PeerOde,
    PeerSolution, PeerSolveError, PeerSolveOptions, ProblemRegime, ProductionMethod,
    RegistryRefusal, SelectionRefusal, caps, variant,
};
use crate::{
    Atol, DampedRotationFlow, DerivativeFallback, Dopri5, DriverWork, ExplicitCallbackFailure,
    ExplicitSolution, ExplicitSolveError, ExplicitUnsupported, FacadeRoute, FailureReason,
    InteriorError, Jacobian, LawsonDopri5, LawsonSolution, Options, Problem, Rodas5p, Solution,
    SolveError, Status, Unsupported, WorkRecord,
    selection_evidence::{CostComparison, CostEvidence, compare_qualified_costs},
};

/// Version of the catalog entries that node V03 added to the registry.
pub const METHOD_CATALOG_VERSION: &str = "v03-2026-11-17";

/// Method id of the exponential Rosenbrock adapter.
pub const METHOD_ID_PEXPRB: &str = "pexprb54s4-fused";
/// Method id of the lagged-LU RODAS5P adapter (the driver id of the integrator crate).
pub const METHOD_ID_RODAS5P_LAGGED: &str = rodas5p_integrators::RODAS5P_LAGGED_DRIVER_ID;
/// Method id of the declared-split Lawson adapter.
pub const METHOD_ID_LAWSON: &str = "lawson-dopri5-declared-damped-rotation";

// ------------------------------------------------------------------------ declared facts

/// Jacobian provider form of a problem, read without a callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum JacobianForm {
    /// [`Jacobian::Dense`].
    Dense,
    /// [`Jacobian::Banded`].
    Banded,
    /// [`Jacobian::Jvp`].
    Jvp,
    /// [`Jacobian::SparseJvp`] with a banded pattern (assembled into a band).
    SparseJvpBanded,
    /// [`Jacobian::SparseJvp`] with a general pattern (matrix-free).
    SparseJvpGeneral,
    /// [`Jacobian::FiniteDifference`].
    FiniteDifference,
}

/// The declared facts of one request: everything routing may depend on. Built from the
/// problem and the options without entering user code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct MethodFacts {
    /// Problem dimension.
    pub dimension: usize,
    /// Jacobian provider form.
    pub jacobian: JacobianForm,
    /// A band was declared with [`Problem::banded`].
    pub declared_band: bool,
    /// Declared autonomous.
    pub autonomous: bool,
    /// An explicit time derivative is supplied.
    pub time_derivative: bool,
    /// A mass matrix is supplied.
    pub mass: bool,
    /// At least one event is declared.
    pub events: bool,
    /// [`Options::dense_output`].
    pub dense_output: bool,
    /// [`Options::interior_error`].
    pub interior_error: InteriorError,
    /// The absolute tolerance is a vector.
    pub vector_atol: bool,
}

impl MethodFacts {
    /// Read the facts of `(problem, options)`. Pure.
    pub fn of(problem: &Problem, options: &Options) -> Self {
        let jacobian = match &problem.jacobian {
            Jacobian::Dense(_) => JacobianForm::Dense,
            Jacobian::Banded { .. } => JacobianForm::Banded,
            Jacobian::Jvp(_) => JacobianForm::Jvp,
            Jacobian::SparseJvp { pattern, .. } if pattern.is_banded() => {
                JacobianForm::SparseJvpBanded
            }
            Jacobian::SparseJvp { .. } => JacobianForm::SparseJvpGeneral,
            Jacobian::FiniteDifference => JacobianForm::FiniteDifference,
        };
        Self {
            dimension: problem.dimension,
            jacobian,
            declared_band: problem.band.is_some(),
            autonomous: problem.autonomous,
            time_derivative: problem.time_derivative.is_some(),
            mass: problem.mass.is_some(),
            events: !problem.events.is_empty(),
            dense_output: options.dense_output,
            interior_error: options.interior_error,
            vector_atol: matches!(options.atol, Atol::Vector(_)),
        }
    }

    /// Derivative capabilities that the provider form itself uses, on any route.
    fn provider_needs(&self) -> CapabilitySet {
        let mut needs = CapabilitySet::NONE;
        match self.jacobian {
            JacobianForm::Dense | JacobianForm::Banded => needs.explicit_jacobian = true,
            JacobianForm::Jvp | JacobianForm::SparseJvpBanded | JacobianForm::SparseJvpGeneral => {
                needs.provided_jvp = true
            }
            JacobianForm::FiniteDifference => needs.finite_difference = true,
        }
        needs
    }

    /// The drivers take `df/dt` by a difference of the right-hand side.
    fn time_derivative_by_difference(&self) -> bool {
        !self.autonomous && !self.time_derivative
    }
}

// ------------------------------------------------------------------------------- catalog

/// Whether [`MethodRegistry::solve`] can execute a registered method.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "state", content = "reason")]
pub enum MethodAvailability {
    /// An adapter exists; a contract test compares it bit for bit with the direct call.
    Available,
    /// No adapter exists; the reason is given. The method is never replaced by another one.
    Unavailable(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdapterKind {
    Dopri5,
    Rodas5p,
    Peer,
    PeerMatrixFree,
    Bdf,
    Radau,
    Pexprb,
    Lagged,
    Lawson,
}

/// One exact route variant of a method: a driver family, what it stores and what it
/// supports. The catalog entry of RODAS5P carries the six driver families of the facade;
/// its U06 descriptor has three permission variants and neither a banded nor a sequential
/// one (audit finding COD-03).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct MethodRouteVariant {
    /// Variant label: the second half of a route id.
    pub label: &'static str,
    /// The driver family of the facade.
    pub facade_route: FacadeRoute,
    /// The route stores and factorizes an n-by-n matrix whatever the provider form. (A
    /// dense Jacobian callback materializes one on every route.)
    pub dense_matrix: bool,
    /// The route never assembles a matrix; the general surface asks for
    /// `GeneralOdePolicy::allow_matrix_free`.
    pub matrix_free: bool,
    /// Runs with a mass matrix.
    pub supports_mass: bool,
    /// Locates events (on the continuous extension, so together with dense output).
    pub supports_events: bool,
    /// Returns the continuous extension.
    pub supports_dense_output: bool,
}

/// One routable method: its descriptor, and what its adapter lowers.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MethodEntry {
    /// Descriptor. The first six entries carry the descriptors of
    /// [`MethodRegistry::descriptors`] unchanged.
    pub descriptor: MethodDescriptor,
    /// Whether [`MethodRegistry::solve`] can execute the method.
    pub availability: MethodAvailability,
    /// Jacobian provider forms the adapter lowers; empty for a derivative-free method.
    pub jacobian_forms: Vec<JacobianForm>,
    /// A band declared with [`Problem::banded`] is accepted.
    pub declared_band: bool,
    /// A vector absolute tolerance is accepted (for RODAS5P: on the routes on which
    /// `Rodas5p::capabilities` accepts it).
    pub vector_atol: bool,
    /// A method for stiff problems: implicit, linearly implicit or exponential (the regime
    /// fit of [`MethodChoice::Auto`]).
    pub stiff_capable: bool,
    /// May be ranked by [`MethodChoice::Auto`]. A method that changes the meaning of the
    /// problem's right-hand side (a declared split) is reached by an explicit choice only.
    pub auto_rankable: bool,
    /// The direct call that the adapter is bitwise equal to.
    pub direct_call: &'static str,
    /// What a failed run returns as its accepted prefix.
    pub failure_prefix: &'static str,
    /// The exact route variants, when the method has more than one driver family (RODAS5P);
    /// empty when the descriptor's variants are the routes.
    pub route_variants: Vec<MethodRouteVariant>,
    #[serde(skip)]
    kind: AdapterKind,
}

const PREFIX_LAST_ACCEPTED: &str = "the outputs reached and the last accepted state";
const PREFIX_OUTPUT_SAMPLES: &str = "the output samples reached (t0 for the end-point schedule): the driver exposes no last \
     accepted state";

fn entry(
    descriptor: MethodDescriptor,
    kind: AdapterKind,
    jacobian_forms: &[JacobianForm],
    direct_call: &'static str,
) -> MethodEntry {
    use AdapterKind as Kind;
    let facade_frontend = matches!(kind, Kind::Dopri5 | Kind::Rodas5p | Kind::Lawson);
    MethodEntry {
        descriptor,
        availability: MethodAvailability::Available,
        jacobian_forms: jacobian_forms.to_vec(),
        declared_band: facade_frontend,
        vector_atol: facade_frontend,
        stiff_capable: !matches!(kind, Kind::Dopri5 | Kind::Lawson),
        auto_rankable: kind != Kind::Lawson,
        direct_call,
        failure_prefix: if matches!(kind, Kind::Bdf | Kind::Radau | Kind::Pexprb | Kind::Lagged) {
            PREFIX_OUTPUT_SAMPLES
        } else {
            PREFIX_LAST_ACCEPTED
        },
        route_variants: if kind == Kind::Rodas5p {
            RODAS5P_FACADE_ROUTES.map(rodas5p_route_variant).to_vec()
        } else {
            Vec::new()
        },
        kind,
    }
}

impl MethodRegistry {
    /// Every routable method: the six descriptors of [`MethodRegistry::descriptors`] in
    /// their order, then the methods that node V03 made reachable. An entry whose adapter
    /// does not exist says so in [`MethodEntry::availability`].
    pub fn catalog() -> Vec<MethodEntry> {
        use JacobianForm::*;
        let none = CapabilitySet::NONE;
        let v = METHOD_CATALOG_VERSION;
        let mut u06 = Self::descriptors().into_iter();
        let mut next = || u06.next().expect("the six U06 descriptors");
        let all_forms = [
            Dense,
            Banded,
            Jvp,
            SparseJvpBanded,
            SparseJvpGeneral,
            FiniteDifference,
        ];
        let mut catalog = vec![
            entry(
                next(),
                AdapterKind::Dopri5,
                &[],
                "vigilode::Dopri5::solve(problem, span, y0, options)",
            ),
            entry(
                next(),
                AdapterKind::Rodas5p,
                &all_forms,
                "vigilode::Rodas5p::solve(problem, span, y0, options)",
            ),
            entry(
                next(),
                AdapterKind::Peer,
                &[Dense, Jvp],
                "vigilode::PeerOde::solve(problem, span, y0, peer_options, allowed)",
            ),
            MethodEntry {
                availability: MethodAvailability::Unavailable(
                    "no end-to-end driver: the matrix-free common-W backend is a stage backend \
                     of ppsw1::Session; PeerOde lowers the dense backend only and there is no \
                     matrix-free starter",
                ),
                ..entry(
                    next(),
                    AdapterKind::PeerMatrixFree,
                    &[],
                    "none (rodas5p_integrators::ppsw1::linear_backend::LinearBackend)",
                )
            },
            entry(
                next(),
                AdapterKind::Bdf,
                &[Dense, Jvp],
                "rodas5p_integrators::integrate_bdf_adaptive_observed(ode, span, y0, \
                 BdfConfig::default(), adaptive, [t0, t1])",
            ),
            entry(
                next(),
                AdapterKind::Radau,
                &[Dense, Jvp],
                "rodas5p_integrators::integrate_radau_adaptive_observed(ode, span, y0, \
                 RadauConfig::default(), adaptive, [t0, t1])",
            ),
        ];
        catalog.push(entry(
            MethodDescriptor {
                method_id: METHOD_ID_PEXPRB,
                version: v,
                tableau_identity: "PEXPRB54s4 exponential Rosenbrock 5(4), fused phi-Krylov \
                                   (exponential.rs, pexprb54s4_tableau)",
                stages: 4,
                order: 5,
                history_steps: 1,
                startup: "none",
                variants: vec![variant(
                    "jvp-phi-krylov",
                    caps(true, false, false, false, false),
                    none,
                    (false, false, false),
                )],
                qualification: MethodQualification::ImplementedUnqualified,
                access: MethodAccess::ExplicitApiOnly,
                route: "rodas5p_integrators::integrate_pexprb54s4_fused_adaptive_observed",
                evidence: "integrator-crate exponential contracts; no facade qualification",
            },
            AdapterKind::Pexprb,
            &[Jvp],
            "rodas5p_integrators::integrate_pexprb54s4_fused_adaptive_observed(ode, span, y0, \
             adaptive, [t0, t1], FusedPhiKrylovConfig::default(), sequential)",
        ));
        catalog.push(entry(
            MethodDescriptor {
                method_id: METHOD_ID_RODAS5P_LAGGED,
                version: v,
                tableau_identity: "RODAS5P stage equations with a lagged LU as Richardson \
                                   preconditioner (rodas5p_lagged.rs)",
                stages: 8,
                order: 5,
                history_steps: 1,
                startup: "none",
                variants: vec![variant(
                    "explicit-jacobian-dense-lagged-lu",
                    caps(false, true, false, true, false),
                    none,
                    (false, false, false),
                )],
                qualification: MethodQualification::ImplementedUnqualified,
                access: MethodAccess::ExplicitApiOnly,
                route: "rodas5p_integrators::integrate_rodas5p_lagged_observed",
                evidence: "integrator-crate lagged contracts; kill gate of the lagged-LU node \
                           (HIRES failed); no facade qualification",
            },
            AdapterKind::Lagged,
            &[Dense],
            "rodas5p_integrators::integrate_rodas5p_lagged_observed(ode, span, y0, adaptive, \
             [t0, t1], Rodas5pLaggedOptions::default())",
        ));
        catalog.push(entry(
            MethodDescriptor {
                method_id: METHOD_ID_LAWSON,
                version: v,
                tableau_identity: "Lawson transform of Dormand-Prince 5(4) for a declared \
                                   damped-rotation linear part",
                stages: 7,
                order: 5,
                history_steps: 1,
                startup: "none",
                variants: vec![variant(
                    "declared-damped-rotation-rhs-only",
                    none,
                    none,
                    (false, false, false),
                )],
                qualification: MethodQualification::ImplementedUnqualified,
                access: MethodAccess::ExplicitApiOnly,
                route: "vigilode::LawsonDopri5::solve",
                evidence: "vigilode lawson contracts; no method qualification",
            },
            AdapterKind::Lawson,
            &[],
            "vigilode::LawsonDopri5::solve(flow, residual_problem, span, y0, options)",
        ));
        catalog
    }

    /// The catalog entry of `method_id`, if it is registered.
    pub fn entry(method_id: &str) -> Option<MethodEntry> {
        Self::catalog()
            .into_iter()
            .find(|e| e.descriptor.method_id == method_id)
    }
}

// ------------------------------------------------------------------------------ choices

/// How the caller chooses the method of [`MethodRegistry::route`].
///
/// The registry is one entry point over every method of the facade: route a request to a
/// typed plan without any callback, execute the plan, read the method-independent view.
///
/// ```
/// use vigilode::{
///     CallbackError, DerivativeFallback, GeneralOdePolicy, Jacobian, MethodChoice,
///     MethodRegistry, Options, Problem, ProblemRegime,
/// };
///
/// let problem = Problem::new(2, |_t, y, dy| {
///     dy[0] = -1000.0 * y[0] + y[1];
///     dy[1] = -y[1];
///     Ok::<(), CallbackError>(())
/// })
/// .jacobian(Jacobian::dense(|_t, _y, jac| {
///     jac.copy_from_slice(&[-1000.0, 1.0, 0.0, -1.0]);
///     Ok(())
/// }))
/// .autonomous();
/// let options = Options::default();
///
/// // today's default decision, as a plan (no callback is made)
/// let policy = GeneralOdePolicy {
///     regime: ProblemRegime::Stiff,
///     derivatives: DerivativeFallback::ProvidedOnly,
///     ..GeneralOdePolicy::default()
/// };
/// let plan = MethodRegistry::route(&problem, &options, &MethodChoice::General(policy))?;
/// assert_eq!(plan.route.id(), "rodas5p/dense-fast");
///
/// // execute it and read the method-independent view
/// let solution = MethodRegistry::solve(&problem, (0.0, 1.0), &[1.0, 1.0], &options, &plan)?;
/// let view = solution.view();
/// assert!(view.is_success());
/// assert_eq!(view.times(), &[0.0, 1.0]);
/// assert!(view.work().rhs_calls > 0);
///
/// // any registered method by id; a refusal is typed and precedes every callback
/// let radau = MethodRegistry::route(&problem, &options, &MethodChoice::Explicit("radau-iia3"))?;
/// assert_eq!(radau.route.id(), "radau-iia3/explicit-jacobian-dense-newton");
/// assert!(MethodRegistry::route(&problem, &options, &MethodChoice::Explicit("no-such-method")).is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub enum MethodChoice<'a> {
    /// Today's default decision: exactly `GeneralOde::select` under this policy.
    General(GeneralOdePolicy),
    /// Opt-in ranking over the admitted registry entries. Never a default.
    Auto(MethodAutoPolicy<'a>),
    /// One registered method by its id ([`MethodDescriptor::method_id`]). The caller names
    /// the method, so its qualification level is not a refusal; it is carried by the plan.
    Explicit(&'a str),
}

/// Kind of a [`MethodChoice`], recorded in plans and solutions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MethodChoiceKind {
    /// [`MethodChoice::General`].
    General,
    /// [`MethodChoice::Auto`].
    Auto,
    /// [`MethodChoice::Explicit`].
    Explicit,
}

/// What [`MethodChoice::Auto`] does after an attempt that did not reach the end time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MethodFallback {
    /// Return the failed attempt.
    Never,
    /// Run the next ranked admitted method from `t0`, up to `max_attempts` attempts in
    /// total. Only methods admitted under [`MethodAutoPolicy::allowed`] are tried, every
    /// attempt is kept in the solution, and its work stays in the totals.
    NextAdmitted {
        /// Largest number of attempts, the first one included.
        max_attempts: usize,
    },
}

/// Measured cost of one method, supplied by the caller to [`MethodChoice::Auto`].
#[derive(Clone, Copy, Debug, Serialize)]
pub struct MethodCostEvidence {
    /// Registered method id the evidence belongs to.
    pub method_id: &'static str,
    /// The cost interval with its identity, unit and provenance.
    pub evidence: CostEvidence,
    /// Callback units that were spent to obtain the evidence. They are charged: the
    /// solution reports them in [`MethodSolution::evidence_callback_units`].
    pub callback_units: u64,
}

/// Policy of [`MethodChoice::Auto`]: declared facts, permissions and nothing hidden.
///
/// Candidates are the catalog entries whose adapter is available and that are admitted
/// for the problem's declared facts under `allowed`. They are ordered by (1) regime fit
/// (a declared stiff problem admits only methods for stiff problems,
/// [`MethodEntry::stiff_capable`]; a nonstiff or unknown one ranks the explicit methods
/// before them), (2) qualification (production, scoped
/// calibration, implemented-unqualified, reference-only), (3) catalog order. With
/// `use_measured_evidence`, a candidate moves ahead of another of the same regime fit and
/// qualification when the supplied cost intervals say so
/// ([`compare_qualified_costs`] with zero gain and zero transition cost).
/// [`MethodRegistry::route`] measures nothing itself.
#[derive(Clone, Copy, Debug)]
pub struct MethodAutoPolicy<'a> {
    /// Declared regime.
    pub regime: ProblemRegime,
    /// Capabilities the caller permits, for every phase of every candidate.
    pub allowed: CapabilitySet,
    /// Admit methods that are implemented or reference implementations only.
    pub admit_unqualified: bool,
    /// What to do after a failed attempt.
    pub fallback: MethodFallback,
    /// Reorder candidates by the supplied measured evidence.
    pub use_measured_evidence: bool,
    /// Measured cost evidence, used only with `use_measured_evidence`.
    pub evidence: &'a [MethodCostEvidence],
}

impl MethodAutoPolicy<'static> {
    /// The permissions of an existing [`GeneralOdePolicy`] for `problem`
    /// ([`CapabilitySet::from_problem_and_policy`]): qualified methods only, no fallback,
    /// no measured evidence.
    pub fn from_general(problem: &Problem, policy: &GeneralOdePolicy) -> Self {
        Self {
            regime: policy.regime,
            allowed: CapabilitySet::from_problem_and_policy(problem, policy),
            admit_unqualified: false,
            fallback: MethodFallback::Never,
            use_measured_evidence: false,
            evidence: &[],
        }
    }
}

/// Method-specific declarations and parameters. The default suits every method except
/// the Lawson adapter, which needs its declared flow.
#[derive(Clone, Debug, Default)]
pub struct MethodParams {
    /// Options of the PPSW1 adapter. `None`: `rtol`, the scalar `atol` and the attempt
    /// budget of [`Options`], the PPSW1 first-step rule and one thread.
    pub peer: Option<PeerSolveOptions>,
    /// Declared linear part of the Lawson split `y' = A y + N(t, y)`. With it, the
    /// problem's right-hand side must evaluate `N`, not the full right-hand side.
    pub lawson_flow: Option<DampedRotationFlow>,
}

// ------------------------------------------------------------------------- route plans

/// The route of one method: what [`MethodRegistry::solve`] executes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct MethodRoute {
    /// Registered method id.
    pub method_id: &'static str,
    /// Variant of the method that the declared facts select.
    pub variant: &'static str,
    /// Driver family of the RODAS5P facade, for the RODAS5P method.
    pub facade_route: Option<FacadeRoute>,
    /// Evidence level of the method. Implemented is not qualified.
    pub qualification: MethodQualification,
    /// Capabilities the route uses in any phase.
    pub needs: CapabilitySet,
}

impl MethodRoute {
    /// Stable route id: `<method id>/<variant>`.
    pub fn id(&self) -> String {
        format!("{}/{}", self.method_id, self.variant)
    }
}

/// Why one method is not admitted for a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum MethodRefusalReason {
    /// The method has no adapter ([`MethodEntry::availability`]).
    AdapterUnavailable,
    /// A registry refusal: a feature, a permission, the provider form or the qualification.
    Registry(RegistryRefusal),
    /// A refusal of the RODAS5P facade (`Rodas5p::capabilities`).
    Implicit(Unsupported),
    /// A refusal of the explicit drivers (DOPRI5, Lawson).
    Explicit(ExplicitUnsupported),
    /// The method's driver takes a scalar absolute tolerance.
    VectorAtolUnsupported,
    /// The method's driver integrates autonomous problems only.
    NonAutonomousUnsupported,
    /// A declaration the method needs is missing ([`MethodParams`]).
    MissingDeclaration,
    /// The declared regime is stiff and the method is not one for stiff problems.
    RegimeMismatch,
    /// The method is reached by an explicit choice only ([`MethodEntry::auto_rankable`]).
    ExplicitChoiceOnly,
}

/// One row of the ranking of [`MethodChoice::Auto`].
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MethodCandidate {
    /// Registered method id.
    pub method_id: &'static str,
    /// Position among the admitted candidates (0 is the primary route); `None` if refused.
    pub rank: Option<usize>,
    /// The admitted route; `None` if refused.
    pub route: Option<MethodRoute>,
    /// Every reason found for a refusal; empty if admitted.
    pub refusals: Vec<MethodRefusalReason>,
    /// The method fits the declared regime.
    pub regime_fit: bool,
    /// Evidence level of the method.
    pub qualification: MethodQualification,
    /// Supplied measured evidence moved this candidate ahead of another one.
    pub moved_by_evidence: bool,
}

/// A typed refusal of [`MethodRegistry::route`]. No callback was made.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum MethodRouteRefusal {
    /// The refusal of `GeneralOde::select` ([`MethodChoice::General`]).
    #[error("the general selection refused: {0}")]
    General(SelectionRefusal),
    /// The id of [`MethodChoice::Explicit`] is not registered.
    #[error("unknown method id {method_id:?}")]
    UnknownMethod {
        /// The id that was asked for.
        method_id: String,
    },
    /// The method of [`MethodChoice::Explicit`] is not admitted for this request.
    #[error("method {method_id} is not admitted: {reasons:?}")]
    Method {
        /// Registered method id.
        method_id: &'static str,
        /// Every reason found.
        reasons: Vec<MethodRefusalReason>,
    },
    /// [`MethodChoice::Auto`] admitted no method; the ranking holds every reason.
    #[error("no registered method is admitted for this request")]
    NoAdmittedMethod {
        /// One row per catalog entry.
        ranking: Vec<MethodCandidate>,
    },
}

/// A routed request: the primary route and, for [`MethodChoice::Auto`], the ranking and
/// the fallback order. Built by [`MethodRegistry::route`] without any callback.
#[derive(Clone, Debug)]
pub struct MethodRoutePlan {
    /// How the route was chosen.
    pub choice: MethodChoiceKind,
    /// The route that is executed first.
    pub route: MethodRoute,
    /// Routes tried after a failed attempt, in order; empty unless
    /// [`MethodFallback::NextAdmitted`] was requested.
    pub fallbacks: Vec<MethodRoute>,
    /// One row per catalog entry ([`MethodChoice::Auto`] only).
    pub ranking: Vec<MethodCandidate>,
    /// The receipt of `GeneralOde::select` ([`MethodChoice::General`] only).
    pub general: Option<GeneralSelection>,
    /// The declared facts the plan was routed for; [`MethodRegistry::solve`] refuses a
    /// problem or options with other facts.
    pub facts: MethodFacts,
    /// Callback units of the measured evidence that the policy used.
    pub evidence_callback_units: u64,
    params: MethodParams,
}

impl MethodRoutePlan {
    /// The method parameters the plan was routed with.
    pub fn params(&self) -> &MethodParams {
        &self.params
    }
}

// --------------------------------------------------------------------------- admission

const FULL_PERMISSION: CapabilitySet = CapabilitySet {
    provided_jvp: true,
    finite_difference: true,
    automatic_differentiation: true,
    explicit_jacobian: true,
    dense_matrix: true,
    newton: true,
};

fn union(a: CapabilitySet, b: CapabilitySet) -> CapabilitySet {
    CapabilitySet {
        provided_jvp: a.provided_jvp || b.provided_jvp,
        finite_difference: a.finite_difference || b.finite_difference,
        automatic_differentiation: a.automatic_differentiation || b.automatic_differentiation,
        explicit_jacobian: a.explicit_jacobian || b.explicit_jacobian,
        dense_matrix: a.dense_matrix || b.dense_matrix,
        newton: a.newton || b.newton,
    }
}

/// The facade route that a RODAS5P run record says was executed.
fn executed_facade_route(driver_work: &DriverWork) -> FacadeRoute {
    match driver_work {
        DriverWork::Dense { .. } => FacadeRoute::DenseFast,
        DriverWork::Banded { .. } => FacadeRoute::BandedFast,
        DriverWork::MatrixFree { .. } => FacadeRoute::MatrixFree,
        DriverWork::FastDense { banded: false, .. } => FacadeRoute::FastDense,
        DriverWork::FastDense { banded: true, .. } => FacadeRoute::FastBandedDense,
        DriverWork::SequentialDense { .. } => FacadeRoute::SequentialDense,
    }
}

/// The driver families of the RODAS5P facade, in the order of [`FacadeRoute`].
const RODAS5P_FACADE_ROUTES: [FacadeRoute; 6] = [
    FacadeRoute::DenseFast,
    FacadeRoute::BandedFast,
    FacadeRoute::MatrixFree,
    FacadeRoute::FastDense,
    FacadeRoute::FastBandedDense,
    FacadeRoute::SequentialDense,
];

/// The route variant of a RODAS5P facade route: the rows of the catalog entry. What a
/// route supports restates `Rodas5p::capabilities`: a mass matrix runs the sequential
/// driver, the continuous extension the two fast dense drivers or the sequential one,
/// events need the continuous extension, and the matrix-free route has none of the three.
fn rodas5p_route_variant(route: FacadeRoute) -> MethodRouteVariant {
    // (label, dense storage, matrix-free, mass, events, dense output)
    let (label, dense_matrix, matrix_free, mass, events, dense_output) = match route {
        FacadeRoute::DenseFast => ("dense-fast", true, false, false, false, false),
        FacadeRoute::BandedFast => ("banded-fast", false, false, false, false, false),
        FacadeRoute::MatrixFree => ("matrix-free", false, true, false, false, false),
        FacadeRoute::FastDense => ("fast-dense", true, false, false, true, true),
        FacadeRoute::FastBandedDense => ("fast-banded-dense", false, false, false, true, true),
        FacadeRoute::SequentialDense => ("sequential-dense", true, false, true, true, true),
    };
    MethodRouteVariant {
        label,
        facade_route: route,
        dense_matrix,
        matrix_free,
        supports_mass: mass,
        supports_events: events,
        supports_dense_output: dense_output,
    }
}

/// Variant label of the RODAS5P facade route.
fn rodas5p_variant(route: FacadeRoute) -> &'static str {
    rodas5p_route_variant(route).label
}

/// Capabilities a RODAS5P facade route uses for these facts. An n-by-n matrix is
/// materialized on the routes with dense storage, and whenever the provider is a dense
/// callback (a declared band on it is verified against the full matrix).
fn rodas5p_needs(route: FacadeRoute, facts: &MethodFacts) -> CapabilitySet {
    let mut needs = facts.provider_needs();
    if facts.time_derivative_by_difference() {
        needs.finite_difference = true;
    }
    needs.dense_matrix =
        rodas5p_route_variant(route).dense_matrix || facts.jacobian == JacobianForm::Dense;
    needs
}

/// The driver family that `Rodas5p::solve` runs for a request whose capability query
/// answered `queried`. It is `queried`, except for a general sparse-JVP provider: the query
/// answers matrix-free for every such request that is not sequential, while `solve` leaves
/// the router when dense output is requested (the fast dense driver) or the absolute
/// tolerance is a vector (the dense fast driver); both assemble the Jacobian from `n` JVP
/// columns. The contract test `view_reports_the_executed_route_...` runs both cases.
/// (`Jacobian::Jvp` is refused by the query in both, so nothing changes for it.)
fn rodas5p_executed_route(queried: FacadeRoute, facts: &MethodFacts) -> FacadeRoute {
    match queried {
        FacadeRoute::MatrixFree if facts.dense_output => FacadeRoute::FastDense,
        FacadeRoute::MatrixFree if facts.vector_atol => FacadeRoute::DenseFast,
        other => other,
    }
}

/// The plan of a RODAS5P request whose capability query answered `queried`: the route
/// that runs, with what it needs.
fn rodas5p_route(queried: FacadeRoute, facts: &MethodFacts) -> MethodRoute {
    let route = rodas5p_executed_route(queried, facts);
    MethodRoute {
        method_id: "rodas5p",
        variant: rodas5p_variant(route),
        facade_route: Some(route),
        qualification: MethodQualification::Production,
        needs: rodas5p_needs(route, facts),
    }
}

fn dopri5_route() -> MethodRoute {
    MethodRoute {
        method_id: "dopri5",
        variant: "rhs-only",
        facade_route: None,
        qualification: MethodQualification::Production,
        needs: CapabilitySet::NONE,
    }
}

/// Needs of the descriptor variant `label`: per step and startup together.
fn variant_route(descriptor: &MethodDescriptor, label: &'static str) -> MethodRoute {
    let found = descriptor
        .variants
        .iter()
        .find(|v| v.label == label)
        .expect("the catalog names its own variants");
    MethodRoute {
        method_id: descriptor.method_id,
        variant: found.label,
        facade_route: None,
        qualification: descriptor.qualification,
        needs: union(found.per_step, found.startup),
    }
}

/// Admission of one catalog entry for one request. `allowed` is the caller's permission
/// set ([`MethodChoice::Auto`]); `None` permits what the method needs
/// ([`MethodChoice::Explicit`]). Pure: reads declared facts only.
fn admit(
    entry: &MethodEntry,
    problem: &Problem,
    options: &Options,
    facts: &MethodFacts,
    params: &MethodParams,
    allowed: Option<&CapabilitySet>,
    admit_unqualified: bool,
) -> Result<MethodRoute, Vec<MethodRefusalReason>> {
    use MethodRefusalReason as Reason;
    let descriptor = &entry.descriptor;
    if entry.availability != MethodAvailability::Available {
        return Err(vec![Reason::AdapterUnavailable]);
    }
    let mut reasons = Vec::new();
    let qualified = matches!(
        descriptor.qualification,
        MethodQualification::Production | MethodQualification::ScopedCalibration
    );
    if !qualified && !admit_unqualified {
        reasons.push(Reason::Registry(RegistryRefusal::Unqualified));
    }
    let mut permission_checked = false;
    let route = match entry.kind {
        AdapterKind::Dopri5 | AdapterKind::Lawson => {
            // the refusals of `explicit::validate`, in its order
            if facts.mass {
                reasons.push(Reason::Explicit(ExplicitUnsupported::MassMatrix));
            }
            if facts.dense_output {
                reasons.push(Reason::Explicit(ExplicitUnsupported::DenseOutput));
            }
            if facts.events {
                reasons.push(Reason::Explicit(ExplicitUnsupported::Events));
            }
            if entry.kind == AdapterKind::Lawson && params.lawson_flow.is_none() {
                reasons.push(Reason::MissingDeclaration);
            }
            variant_route(descriptor, descriptor.variants[0].label)
        }
        AdapterKind::Rodas5p => match Rodas5p::capabilities(problem, options) {
            Ok(capabilities) => rodas5p_route(capabilities.route, facts),
            Err(unsupported) => {
                reasons.push(Reason::Implicit(unsupported));
                return Err(reasons);
            }
        },
        AdapterKind::Peer => {
            if facts.dense_output {
                reasons.push(Reason::Registry(RegistryRefusal::DenseOutputUnsupported));
            }
            if facts.vector_atol {
                reasons.push(Reason::VectorAtolUnsupported);
            }
            permission_checked = true;
            match PeerOde::admission(problem, allowed.copied().unwrap_or(FULL_PERMISSION)) {
                MethodAdmission::Admitted { variant, .. } => variant_route(descriptor, variant),
                MethodAdmission::Refused { reasons: found, .. } => {
                    reasons.extend(found.into_iter().map(Reason::Registry));
                    return Err(reasons);
                }
            }
        }
        AdapterKind::Bdf | AdapterKind::Radau | AdapterKind::Pexprb | AdapterKind::Lagged => {
            if facts.mass {
                reasons.push(Reason::Registry(RegistryRefusal::MassUnsupported));
            }
            if facts.events {
                reasons.push(Reason::Registry(RegistryRefusal::EventsUnsupported));
            }
            if facts.dense_output {
                reasons.push(Reason::Registry(RegistryRefusal::DenseOutputUnsupported));
            }
            if facts.vector_atol {
                reasons.push(Reason::VectorAtolUnsupported);
            }
            if entry.kind == AdapterKind::Pexprb && !facts.autonomous {
                reasons.push(Reason::NonAutonomousUnsupported);
            }
            if !entry.jacobian_forms.contains(&facts.jacobian) || facts.declared_band {
                reasons.push(Reason::Registry(RegistryRefusal::ProviderFormUnsupported));
                return Err(reasons);
            }
            let label = match (entry.kind, facts.jacobian) {
                (AdapterKind::Pexprb, _) => "jvp-phi-krylov",
                (AdapterKind::Lagged, _) => "explicit-jacobian-dense-lagged-lu",
                (_, JacobianForm::Dense) => "explicit-jacobian-dense-newton",
                (_, _) => "jvp-columns-dense-newton",
            };
            let mut route = variant_route(descriptor, label);
            // the lagged driver takes df/dt by a difference when none is supplied
            if entry.kind == AdapterKind::Lagged && facts.time_derivative_by_difference() {
                route.needs.finite_difference = true;
            }
            route
        }
        AdapterKind::PeerMatrixFree => return Err(vec![Reason::AdapterUnavailable]),
    };
    if !permission_checked
        && let Some(allowed) = allowed
        && !route.needs.is_subset_of(allowed)
    {
        reasons.push(Reason::Registry(RegistryRefusal::Permission {
            variant: route.variant,
            phase: MethodPhase::PerStep,
        }));
    }
    if reasons.is_empty() {
        Ok(route)
    } else {
        Err(reasons)
    }
}

fn qualification_rank(qualification: MethodQualification) -> u8 {
    match qualification {
        MethodQualification::Production => 0,
        MethodQualification::ScopedCalibration => 1,
        MethodQualification::ImplementedUnqualified => 2,
        MethodQualification::ReferenceImplementationOnly => 3,
    }
}

/// The ranking of [`MethodChoice::Auto`]: one row per catalog entry, and the admitted
/// rows in rank order.
fn rank(
    problem: &Problem,
    options: &Options,
    facts: &MethodFacts,
    params: &MethodParams,
    policy: &MethodAutoPolicy<'_>,
) -> (Vec<MethodCandidate>, Vec<usize>) {
    let stiff = policy.regime == ProblemRegime::Stiff;
    let mut ranking: Vec<MethodCandidate> = MethodRegistry::catalog()
        .iter()
        .map(|entry| {
            let regime_fit = entry.stiff_capable == stiff;
            let admitted = if !entry.auto_rankable {
                Err(vec![MethodRefusalReason::ExplicitChoiceOnly])
            } else if stiff && !entry.stiff_capable {
                Err(vec![MethodRefusalReason::RegimeMismatch])
            } else {
                admit(
                    entry,
                    problem,
                    options,
                    facts,
                    params,
                    Some(&policy.allowed),
                    policy.admit_unqualified,
                )
            };
            let (route, refusals) = match admitted {
                Ok(route) => (Some(route), Vec::new()),
                Err(reasons) => (None, reasons),
            };
            MethodCandidate {
                method_id: entry.descriptor.method_id,
                rank: None,
                route,
                refusals,
                regime_fit,
                qualification: entry.descriptor.qualification,
                moved_by_evidence: false,
            }
        })
        .collect();
    let tier = |c: &MethodCandidate| (!c.regime_fit, qualification_rank(c.qualification));
    let mut order: Vec<usize> = (0..ranking.len())
        .filter(|&i| ranking[i].route.is_some())
        .collect();
    order.sort_by_key(|&i| (tier(&ranking[i]), i));
    if policy.use_measured_evidence {
        let evidence = |method_id: &str| {
            policy
                .evidence
                .iter()
                .find(|e| e.method_id == method_id)
                .map(|e| e.evidence)
        };
        // insertion sort inside one tier: a candidate passes its predecessor only when
        // its measured cost interval lies below the predecessor's
        for start in 1..order.len() {
            let mut position = start;
            while position > 0 {
                let (ahead, behind) = (order[position - 1], order[position]);
                let prefer = tier(&ranking[ahead]) == tier(&ranking[behind])
                    && match (
                        evidence(ranking[ahead].method_id),
                        evidence(ranking[behind].method_id),
                    ) {
                        (Some(incumbent), Some(candidate)) => {
                            compare_qualified_costs(&incumbent, &candidate, 0.0, 1, 0.0)
                                == CostComparison::PreferCandidate
                        }
                        _ => false,
                    };
                if !prefer {
                    break;
                }
                order.swap(position - 1, position);
                ranking[behind].moved_by_evidence = true;
                position -= 1;
            }
        }
    }
    for (position, &index) in order.iter().enumerate() {
        ranking[index].rank = Some(position);
    }
    (ranking, order)
}

/// The selection method of a catalog entry of the general surface.
fn production_method(entry: &MethodEntry) -> Option<ProductionMethod> {
    match entry.kind {
        AdapterKind::Dopri5 => Some(ProductionMethod::Dopri5),
        AdapterKind::Rodas5p => Some(ProductionMethod::Rodas5p),
        _ => None,
    }
}

impl MethodRegistry {
    /// The catalog entry that the general surface selects for a declared regime: the
    /// [`MethodAccess::GeneralOdeSelectable`] entry whose [`MethodEntry::stiff_capable`]
    /// matches it. One per regime.
    pub fn general_method(regime: ProblemRegime) -> Option<MethodEntry> {
        let stiff = regime == ProblemRegime::Stiff;
        let mut selectable = Self::catalog().into_iter().filter(|entry| {
            entry.descriptor.access == MethodAccess::GeneralOdeSelectable
                && entry.stiff_capable == stiff
                && production_method(entry).is_some()
        });
        let entry = selectable.next()?;
        selectable.next().is_none().then_some(entry)
    }

    /// The general selection, derived from the catalog (phase B of node V03). It is the
    /// body of `GeneralOde::select`; `GeneralOde::select_reference` keeps the previous body
    /// as the oracle, and a contract test compares the two on every combination of the
    /// facts either of them reads. Pure: no callback, no numerical workspace.
    ///
    /// 1. The general surface integrates identity-mass problems to their end point: a
    ///    mass matrix, events and dense output are refused, in this order.
    /// 2. The method is [`MethodRegistry::general_method`] of the declared regime.
    /// 3. A derivative-free method (no provider form in its entry) is selected as it is.
    /// 4. Otherwise the policy must authorize derivatives; a finite-difference provider and
    ///    a time derivative taken by differences need the finite-difference permission.
    /// 5. The method's own admission for the declared facts follows (for RODAS5P the
    ///    capability query of the facade), then the route permissions of the policy, read
    ///    from the route's row in the catalog ([`MethodRouteVariant`]): the matrix-free
    ///    route, and the dense-dimension ceiling for a route that materializes an n-by-n
    ///    matrix.
    pub fn general_selection(
        problem: &Problem,
        options: &Options,
        policy: &GeneralOdePolicy,
    ) -> Result<GeneralSelection, SelectionRefusal> {
        let facts = MethodFacts::of(problem, options);
        for (requested, refusal) in [
            (facts.mass, SelectionRefusal::MassMatrix),
            (facts.events, SelectionRefusal::Events),
            (facts.dense_output, SelectionRefusal::DenseOutput),
        ] {
            if requested {
                return Err(refusal);
            }
        }
        // A catalog without exactly one selectable method for the regime, or an admission
        // reason this surface cannot name, would be a registry defect: the reference
        // selection then answers, so the default decision can never change silently.
        let reference = || {
            debug_assert!(false, "the catalog does not describe the general surface");
            GeneralOde::select_reference(problem, options, policy)
        };
        let Some(entry) = Self::general_method(policy.regime) else {
            return reference();
        };
        let Some(method) = production_method(&entry) else {
            return reference();
        };
        let selection = |implicit_route| GeneralSelection {
            method,
            regime: policy.regime,
            derivatives: policy.derivatives,
            implicit_route,
        };
        if entry.jacobian_forms.is_empty() {
            return Ok(selection(None));
        }
        let finite_differences = match policy.derivatives {
            DerivativeFallback::Forbidden => return Err(SelectionRefusal::DerivativesForbidden),
            DerivativeFallback::ProvidedOnly => false,
            DerivativeFallback::AllowFiniteDifference => true,
        };
        if facts.provider_needs().finite_difference && !finite_differences {
            return Err(SelectionRefusal::MissingJacobian);
        }
        if facts.time_derivative_by_difference() && !finite_differences {
            return Err(SelectionRefusal::MissingTimeDerivative);
        }
        // The receipt carries the route of the facade's capability query, as it always has
        // (`GeneralSelection::implicit_route`), and the route permissions are judged on the
        // catalog row of that route.
        let queried = match entry.kind {
            AdapterKind::Rodas5p => match Rodas5p::capabilities(problem, options) {
                Ok(capabilities) => capabilities.route,
                Err(unsupported) => return Err(SelectionRefusal::Implicit(unsupported)),
            },
            _ => return reference(),
        };
        let variant = rodas5p_route_variant(queried);
        if variant.matrix_free && !policy.allow_matrix_free {
            return Err(SelectionRefusal::MatrixFreeNotEnabled);
        }
        let materializes_dense = variant.dense_matrix || facts.jacobian == JacobianForm::Dense;
        if materializes_dense && facts.dimension > policy.max_dense_dimension {
            return Err(SelectionRefusal::DenseDimensionLimit);
        }
        Ok(selection(Some(queried)))
    }

    /// Route a request with default [`MethodParams`]. See [`MethodRegistry::route_with`].
    pub fn route(
        problem: &Problem,
        options: &Options,
        choice: &MethodChoice<'_>,
    ) -> Result<MethodRoutePlan, MethodRouteRefusal> {
        Self::route_with(problem, options, choice, &MethodParams::default())
    }

    /// Route a request: a typed plan, or a typed refusal. Pure: it reads the declared
    /// facts of the problem and the options and never enters user code.
    ///
    /// * [`MethodChoice::General`] returns the decision of `GeneralOde::select`.
    /// * [`MethodChoice::Explicit`] admits the named method for the declared facts.
    /// * [`MethodChoice::Auto`] ranks the admitted catalog entries ([`MethodAutoPolicy`]).
    pub fn route_with(
        problem: &Problem,
        options: &Options,
        choice: &MethodChoice<'_>,
        params: &MethodParams,
    ) -> Result<MethodRoutePlan, MethodRouteRefusal> {
        let facts = MethodFacts::of(problem, options);
        let plan = |choice, route| MethodRoutePlan {
            choice,
            route,
            fallbacks: Vec::new(),
            ranking: Vec::new(),
            general: None,
            facts,
            evidence_callback_units: 0,
            params: params.clone(),
        };
        match *choice {
            MethodChoice::General(policy) => {
                let selection = GeneralOde::select(problem, options, &policy)
                    .map_err(MethodRouteRefusal::General)?;
                let route = match (selection.method, selection.implicit_route) {
                    (ProductionMethod::Dopri5, _) => dopri5_route(),
                    (ProductionMethod::Rodas5p, Some(queried)) => rodas5p_route(queried, &facts),
                    // a RODAS5P selection carries its facade route; ask the facade if not
                    (ProductionMethod::Rodas5p, None) => {
                        let capabilities = Rodas5p::capabilities(problem, options)
                            .map_err(SelectionRefusal::Implicit)
                            .map_err(MethodRouteRefusal::General)?;
                        rodas5p_route(capabilities.route, &facts)
                    }
                };
                Ok(MethodRoutePlan {
                    general: Some(selection),
                    ..plan(MethodChoiceKind::General, route)
                })
            }
            MethodChoice::Explicit(method_id) => {
                let entry =
                    Self::entry(method_id).ok_or_else(|| MethodRouteRefusal::UnknownMethod {
                        method_id: method_id.to_owned(),
                    })?;
                let route = admit(&entry, problem, options, &facts, params, None, true).map_err(
                    |reasons| MethodRouteRefusal::Method {
                        method_id: entry.descriptor.method_id,
                        reasons,
                    },
                )?;
                Ok(plan(MethodChoiceKind::Explicit, route))
            }
            MethodChoice::Auto(policy) => {
                let (ranking, order) = rank(problem, options, &facts, params, &policy);
                let Some(&first) = order.first() else {
                    return Err(MethodRouteRefusal::NoAdmittedMethod { ranking });
                };
                let primary = ranking[first].route.expect("an admitted row");
                let fallbacks = match policy.fallback {
                    MethodFallback::Never => Vec::new(),
                    MethodFallback::NextAdmitted { max_attempts } => order[1..]
                        .iter()
                        .take(max_attempts.saturating_sub(1))
                        .filter_map(|&index| ranking[index].route)
                        .collect(),
                };
                let evidence_callback_units = if policy.use_measured_evidence {
                    policy
                        .evidence
                        .iter()
                        .fold(0u64, |sum, e| sum.saturating_add(e.callback_units))
                } else {
                    0
                };
                Ok(MethodRoutePlan {
                    fallbacks,
                    ranking,
                    evidence_callback_units,
                    ..plan(MethodChoiceKind::Auto, primary)
                })
            }
        }
    }
}

// ----------------------------------------------------------------------------- results

/// Why [`MethodRegistry::solve`] returned no solution. Nothing was integrated.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MethodSolveError {
    /// The plan was routed for a problem or options with other declared facts.
    #[error("the route plan was made for other declared facts than this problem and options")]
    PlanMismatch,
    /// The planned method is not registered or has no adapter (a plan that
    /// [`MethodRegistry::route`] returned never causes this).
    #[error("method {0} has no adapter")]
    NoAdapter(&'static str),
    /// A declaration the method needs is missing ([`MethodParams`]).
    #[error("method {0} needs a declaration that the plan does not carry")]
    MissingDeclaration(&'static str),
    /// Refusal or initialization error of the RODAS5P facade or of the facade lowering.
    #[error(transparent)]
    Implicit(#[from] SolveError),
    /// Refusal or initialization error of the explicit drivers.
    #[error(transparent)]
    Explicit(#[from] ExplicitSolveError),
    /// Refusal of the PPSW1 adapter.
    #[error(transparent)]
    Peer(#[from] PeerSolveError),
    /// Malformed input of an integrator-crate adapter, found before any callback.
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

/// The method-specific result of one attempt, unchanged from its direct call.
// One value per attempt: boxing the larger variants would add an allocation per solve.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
#[non_exhaustive]
pub enum MethodResult {
    /// Result of `Rodas5p::solve`.
    Rodas5p(Solution),
    /// Result of `Dopri5::solve`.
    Explicit(ExplicitSolution),
    /// Result of `LawsonDopri5::solve`.
    Lawson(LawsonSolution),
    /// Result of `PeerOde::solve`.
    Peer(PeerSolution),
    /// Record of an integrator-crate driver (BDF, Radau, PEXPRB, lagged RODAS5P).
    Observed(MethodObservedRun),
}

/// Typed reason of an attempt that did not reach the end time.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum MethodFailure {
    /// A facade [`FailureReason`]: RODAS5P and the integrator-crate adapters.
    Driver(FailureReason),
    /// A stop of DOPRI5 or Lawson-DOPRI5.
    Explicit {
        /// The driver's typed stop reason, when it gave one.
        stop: Option<Dopri5StopReason>,
        /// The callback failure that ended the run, if any.
        callback: Option<ExplicitCallbackFailure>,
        /// The driver's message.
        message: String,
    },
    /// A PPSW1 failure.
    Peer(PeerFailure),
    /// An integrator-crate driver returned an error that no callback caused.
    DriverError(String),
}

impl fmt::Display for MethodFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Driver(reason) => write!(f, "{reason}"),
            Self::Explicit { stop, message, .. } => match stop {
                Some(stop) => write!(f, "{message} ({stop:?})"),
                None => f.write_str(message),
            },
            Self::Peer(reason) => write!(f, "{reason}"),
            Self::DriverError(message) => write!(f, "driver error: {message}"),
        }
    }
}

/// What the returned times are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MethodOutputGrid {
    /// `t0` and the final time; after a failure the final time is the last accepted time
    /// (or the last reached output sample, see [`MethodEntry::failure_prefix`]).
    Endpoints,
    /// `t0` and the end of every accepted step (RODAS5P with dense output).
    AcceptedSteps,
    /// The end of every accepted step without `t0` (PPSW1: the starter end point, then
    /// each accepted step).
    AcceptedStepsWithoutInitial,
}

/// Compact work summary that every method can fill, for totals over attempts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct MethodWork {
    /// Right-hand-side calls (finite-difference calls included).
    pub rhs_calls: u64,
    /// Jacobian builds.
    pub jacobian_builds: u64,
    /// Jacobian-vector products.
    pub jvp_calls: u64,
    /// Explicit time-derivative calls.
    pub time_derivative_calls: u64,
    /// Direct factorizations.
    pub factorizations: u64,
    /// Linear solves.
    pub linear_solves: u64,
    /// Nonlinear (Newton) iterations.
    pub nonlinear_iterations: u64,
    /// Accepted steps.
    pub accepted_steps: u64,
    /// Rejected attempts.
    pub rejected_steps: u64,
}

impl MethodWork {
    fn add(&mut self, other: &Self) {
        self.rhs_calls += other.rhs_calls;
        self.jacobian_builds += other.jacobian_builds;
        self.jvp_calls += other.jvp_calls;
        self.time_derivative_calls += other.time_derivative_calls;
        self.factorizations += other.factorizations;
        self.linear_solves += other.linear_solves;
        self.nonlinear_iterations += other.nonlinear_iterations;
        self.accepted_steps += other.accepted_steps;
        self.rejected_steps += other.rejected_steps;
    }
}

/// One executed attempt: the route and the native result.
#[derive(Debug)]
struct MethodAttempt {
    route: MethodRoute,
    result: MethodResult,
}

/// Result of [`MethodRegistry::solve`]: every executed attempt, the last one being the
/// reported solution.
#[derive(Debug)]
pub struct MethodSolution {
    choice: MethodChoiceKind,
    attempts: Vec<MethodAttempt>,
    evidence_callback_units: u64,
    fallback_error: Option<String>,
}

impl MethodSolution {
    /// The method-independent view of the reported (last) attempt.
    pub fn view(&self) -> SolutionView<'_> {
        SolutionView {
            attempt: self.attempts.last().expect("at least one attempt"),
        }
    }

    /// Every executed attempt in order. More than one only after a fallback; the earlier
    /// ones failed and keep their typed reason, accepted prefix and work.
    pub fn attempts(&self) -> impl ExactSizeIterator<Item = SolutionView<'_>> {
        self.attempts.iter().map(|attempt| SolutionView { attempt })
    }

    /// Whether the reported attempt reached the end time.
    pub fn is_success(&self) -> bool {
        self.view().is_success()
    }

    /// How the route was chosen.
    pub fn choice(&self) -> MethodChoiceKind {
        self.choice
    }

    /// Work of all attempts together, failed ones included.
    pub fn total_work(&self) -> MethodWork {
        let mut total = MethodWork::default();
        for attempt in self.attempts() {
            total.add(&attempt.work_summary());
        }
        total
    }

    /// Callback units of the measured evidence the policy used (spent by the caller before
    /// routing; not contained in [`MethodSolution::total_work`]).
    pub fn evidence_callback_units(&self) -> u64 {
        self.evidence_callback_units
    }

    /// The refusal that ended a fallback chain early, if a fallback method refused its
    /// input. The attempts made before it are kept.
    pub fn fallback_error(&self) -> Option<&str> {
        self.fallback_error.as_deref()
    }

    /// The native result of the reported attempt.
    pub fn into_result(mut self) -> MethodResult {
        self.attempts.pop().expect("at least one attempt").result
    }
}

/// Borrowed, method-independent view of one attempt.
#[derive(Clone, Copy, Debug)]
pub struct SolutionView<'a> {
    attempt: &'a MethodAttempt,
}

fn explicit_failure(solution: &ExplicitSolution) -> Option<MethodFailure> {
    (!solution.success).then(|| MethodFailure::Explicit {
        stop: solution.stop_reason,
        callback: solution.callback_failure.clone(),
        message: solution.message.clone(),
    })
}

impl<'a> SolutionView<'a> {
    /// Output times; see [`SolutionView::output_grid`]. After a failure they are the
    /// accepted prefix.
    pub fn times(&self) -> &'a [f64] {
        match &self.attempt.result {
            MethodResult::Rodas5p(s) => &s.t,
            MethodResult::Explicit(s) => &s.t,
            MethodResult::Lawson(s) => &s.integration.t,
            MethodResult::Peer(s) => &s.t,
            MethodResult::Observed(s) => &s.t,
        }
    }

    /// States at [`SolutionView::times`].
    pub fn states(&self) -> &'a [Vec<f64>] {
        match &self.attempt.result {
            MethodResult::Rodas5p(s) => &s.y,
            MethodResult::Explicit(s) => &s.y,
            MethodResult::Lawson(s) => &s.integration.y,
            MethodResult::Peer(s) => &s.y,
            MethodResult::Observed(s) => &s.y,
        }
    }

    /// What the output times are.
    pub fn output_grid(&self) -> MethodOutputGrid {
        match &self.attempt.result {
            MethodResult::Rodas5p(s) if s.stats.options.dense_output => {
                MethodOutputGrid::AcceptedSteps
            }
            MethodResult::Peer(_) => MethodOutputGrid::AcceptedStepsWithoutInitial,
            _ => MethodOutputGrid::Endpoints,
        }
    }

    /// The last output time.
    pub fn final_time(&self) -> Option<f64> {
        self.times().last().copied()
    }

    /// The last output state.
    pub fn final_state(&self) -> Option<&'a [f64]> {
        self.states().last().map(Vec::as_slice)
    }

    /// The state at `t` from the method's continuous extension, when the native result
    /// has one (RODAS5P; see `Solution::at`).
    pub fn at(&self, t: f64) -> Option<Vec<f64>> {
        match &self.attempt.result {
            MethodResult::Rodas5p(s) => s.at(t),
            _ => None,
        }
    }

    /// Whether the attempt reached the end time.
    pub fn is_success(&self) -> bool {
        match &self.attempt.result {
            MethodResult::Rodas5p(s) => s.is_success(),
            MethodResult::Explicit(s) => s.success,
            MethodResult::Lawson(s) => s.integration.success,
            MethodResult::Peer(s) => s.success,
            MethodResult::Observed(s) => s.success,
        }
    }

    /// Typed reason of an attempt that did not reach the end time; `None` after a success.
    pub fn failure(&self) -> Option<MethodFailure> {
        match &self.attempt.result {
            MethodResult::Rodas5p(s) => match &s.status {
                Status::Success => None,
                Status::Failed(reason) => Some(MethodFailure::Driver(reason.clone())),
            },
            MethodResult::Explicit(s) => explicit_failure(s),
            MethodResult::Lawson(s) => explicit_failure(&s.integration),
            MethodResult::Peer(s) => s.reason.clone().map(MethodFailure::Peer),
            MethodResult::Observed(s) if s.success => None,
            MethodResult::Observed(s) => Some(match (&s.failure, &s.driver_error) {
                (Some(reason), _) => MethodFailure::Driver(reason.clone()),
                (None, Some(message)) => MethodFailure::DriverError(message.clone()),
                (None, None) => MethodFailure::DriverError(s.message.clone()),
            }),
        }
    }

    /// The full work record of the attempt (for PPSW1: starter and steps together).
    pub fn work(&self) -> WorkRecord {
        match &self.attempt.result {
            MethodResult::Rodas5p(s) => s.stats.counters,
            MethodResult::Explicit(s) => (&s.stats.counters).into(),
            MethodResult::Lawson(s) => (&s.integration.stats.counters).into(),
            MethodResult::Peer(s) => {
                let mut counters = s.startup_work;
                counters.accumulate(s.step_work);
                (&counters).into()
            }
            MethodResult::Observed(s) => s.counters,
        }
    }

    /// Accepted steps and rejected attempts as the method reports them.
    pub fn steps(&self) -> (u64, u64) {
        match &self.attempt.result {
            MethodResult::Rodas5p(s) => {
                (s.stats.accepted_steps as u64, s.stats.rejected_steps as u64)
            }
            MethodResult::Explicit(s) => {
                (s.stats.accepted_steps as u64, s.stats.rejected_steps as u64)
            }
            MethodResult::Lawson(s) => (
                s.integration.stats.accepted_steps as u64,
                s.integration.stats.rejected_steps as u64,
            ),
            MethodResult::Peer(s) => (s.step_work.accepted_steps, s.step_work.rejected_steps),
            MethodResult::Observed(s) => (s.accepted_steps, s.rejected_steps),
        }
    }

    /// The compact work summary of the attempt.
    pub fn work_summary(&self) -> MethodWork {
        let work = self.work();
        let (accepted_steps, rejected_steps) = self.steps();
        MethodWork {
            rhs_calls: work.rhs_calls,
            jacobian_builds: work.jacobian_builds,
            jvp_calls: work.jvp_calls,
            time_derivative_calls: work.ft_calls,
            factorizations: work.direct_factorizations,
            linear_solves: work.linear_solves,
            nonlinear_iterations: work.nonlinear_iterations,
            accepted_steps,
            rejected_steps,
        }
    }

    /// The route the plan asked this attempt to execute.
    pub fn planned_route(&self) -> &'a MethodRoute {
        &self.attempt.route
    }

    /// The route that was executed, read from the native result where it records one: the
    /// driver family of the RODAS5P run record, and the admitted variant of the PPSW1
    /// result. For the other methods the adapter is the planned route's only driver.
    pub fn route(&self) -> MethodRoute {
        let planned = self.attempt.route;
        match &self.attempt.result {
            MethodResult::Rodas5p(s) => {
                let executed = executed_facade_route(&s.stats.driver_work);
                MethodRoute {
                    variant: rodas5p_variant(executed),
                    facade_route: Some(executed),
                    ..planned
                }
            }
            MethodResult::Peer(s) => match &s.admission {
                MethodAdmission::Admitted { variant, .. } => MethodRoute { variant, ..planned },
                MethodAdmission::Refused { .. } => planned,
            },
            _ => planned,
        }
    }

    /// Id of the executed route: `<method id>/<variant>`.
    pub fn route_id(&self) -> String {
        self.route().id()
    }

    /// The driver that ran, as the native result records it (the driver id of the RODAS5P
    /// run record, the method name of the explicit record, the entry point of an
    /// integrator-crate driver).
    pub fn executed_driver(&self) -> &'a str {
        match &self.attempt.result {
            MethodResult::Rodas5p(s) => s.stats.route.driver_id,
            MethodResult::Explicit(s) => s.stats.method,
            MethodResult::Lawson(s) => s.integration.stats.method,
            MethodResult::Peer(_) => ppsw1::METHOD_ID,
            MethodResult::Observed(s) => s.driver,
        }
    }

    /// The native result of the attempt.
    pub fn native(&self) -> &'a MethodResult {
        &self.attempt.result
    }
}

// --------------------------------------------------------------------------- execution

/// The PPSW1 options of a plan: the caller's, or the shared options lowered.
fn peer_options(
    options: &Options,
    params: &MethodParams,
) -> Result<PeerSolveOptions, MethodSolveError> {
    if let Some(peer) = params.peer {
        return Ok(peer);
    }
    let Atol::Scalar(atol) = options.atol else {
        return Err(MethodSolveError::InvalidInput(
            "the PPSW1 adapter takes a scalar absolute tolerance".into(),
        ));
    };
    Ok(PeerSolveOptions {
        rtol: options.rtol,
        atol,
        max_attempts: options.max_steps,
        ..PeerSolveOptions::default()
    })
}

fn execute(
    route: &MethodRoute,
    problem: &Problem,
    span: (f64, f64),
    y0: &[f64],
    options: &Options,
    params: &MethodParams,
) -> Result<MethodResult, MethodSolveError> {
    let kind = MethodRegistry::entry(route.method_id)
        .filter(|entry| entry.availability == MethodAvailability::Available)
        .map(|entry| entry.kind)
        .ok_or(MethodSolveError::NoAdapter(route.method_id))?;
    let observed = |driver| run_observed(driver, problem, span, y0, options);
    Ok(match kind {
        AdapterKind::Dopri5 => MethodResult::Explicit(Dopri5::solve(problem, span, y0, options)?),
        AdapterKind::Rodas5p => MethodResult::Rodas5p(Rodas5p::solve(problem, span, y0, options)?),
        AdapterKind::Lawson => {
            let flow = params
                .lawson_flow
                .as_ref()
                .ok_or(MethodSolveError::MissingDeclaration(route.method_id))?;
            MethodResult::Lawson(LawsonDopri5::solve(flow, problem, span, y0, options)?)
        }
        AdapterKind::Peer => {
            let peer = peer_options(options, params)?;
            MethodResult::Peer(PeerOde::solve(problem, span, y0, &peer, route.needs)?)
        }
        AdapterKind::Bdf => MethodResult::Observed(observed(ObservedDriver::Bdf)?),
        AdapterKind::Radau => MethodResult::Observed(observed(ObservedDriver::Radau)?),
        AdapterKind::Pexprb => MethodResult::Observed(observed(ObservedDriver::Pexprb)?),
        AdapterKind::Lagged => MethodResult::Observed(observed(ObservedDriver::Lagged)?),
        AdapterKind::PeerMatrixFree => {
            return Err(MethodSolveError::NoAdapter(route.method_id));
        }
    })
}

impl MethodRegistry {
    /// Execute a plan. The primary route runs first; after an attempt that does not reach
    /// the end time, the plan's fallback routes (if the policy asked for them) run in
    /// order, each from `t0`. Every attempt is kept in the solution.
    ///
    /// `Err` means nothing was integrated: the plan does not belong to this problem, or
    /// the primary method refused its input before any callback.
    pub fn solve(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        options: &Options,
        plan: &MethodRoutePlan,
    ) -> Result<MethodSolution, MethodSolveError> {
        if MethodFacts::of(problem, options) != plan.facts {
            return Err(MethodSolveError::PlanMismatch);
        }
        let mut solution = MethodSolution {
            choice: plan.choice,
            attempts: Vec::with_capacity(1 + plan.fallbacks.len()),
            evidence_callback_units: plan.evidence_callback_units,
            fallback_error: None,
        };
        for route in std::iter::once(&plan.route).chain(&plan.fallbacks) {
            match execute(route, problem, span, y0, options, &plan.params) {
                Ok(result) => solution.attempts.push(MethodAttempt {
                    route: *route,
                    result,
                }),
                Err(error) if solution.attempts.is_empty() => return Err(error),
                Err(error) => {
                    // a fallback method refused its input: keep what was computed
                    solution.fallback_error = Some(format!("{}: {error}", route.id()));
                    break;
                }
            }
            if solution.view().is_success() {
                break;
            }
        }
        Ok(solution)
    }
}
