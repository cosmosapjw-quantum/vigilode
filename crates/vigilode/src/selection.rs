//! Conservative, auditable selection for ordinary identity-mass ODEs.
//!
//! Selection uses declared information, not an uncharged timing pilot or an
//! inferred stiffness certificate. Experimental methods remain explicit APIs.

use crate::{
    DerivativeFallback, Dopri5, ExplicitSolution, ExplicitSolveError, FacadeRoute, Jacobian,
    Options, Problem, Rodas5p, Solution, SolveError, Unsupported,
};
use serde::{Deserialize, Serialize};

mod adapters;
mod frontend;
mod peer;

pub use adapters::{MethodCallbackCounts, MethodObservedRun};
pub use frontend::{
    JacobianForm, METHOD_CATALOG_VERSION, METHOD_ID_LAWSON, METHOD_ID_PEXPRB,
    METHOD_ID_RODAS5P_LAGGED, MethodAutoPolicy, MethodAvailability, MethodCandidate, MethodChoice,
    MethodChoiceKind, MethodCostEvidence, MethodEntry, MethodFacts, MethodFailure, MethodFallback,
    MethodOutputGrid, MethodParams, MethodRefusalReason, MethodResult, MethodRoute,
    MethodRoutePlan, MethodRouteRefusal, MethodRouteVariant, MethodSolution, MethodSolveError,
    MethodWork, SolutionView,
};
pub use peer::{
    PEER_STARTER_MAX_HALVINGS, PeerFailure, PeerOde, PeerPhase, PeerSolution, PeerSolveError,
    PeerSolveOptions, PeerStarterRecord,
};

/// User knowledge of the requested integration regime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProblemRegime {
    /// Try a budgeted explicit solve; no stiffness detection is implied.
    #[default]
    Unknown,
    /// The user expects an explicit method to be appropriate.
    Nonstiff,
    /// Require the Rosenbrock method and explicit derivative authorization.
    Stiff,
}

/// Resource and derivative permissions for [`GeneralOde`].
#[derive(Clone, Copy, Debug, Serialize)]
pub struct GeneralOdePolicy {
    /// Declared problem regime.
    pub regime: ProblemRegime,
    /// Permission to evaluate supplied derivatives or finite differences.
    pub derivatives: DerivativeFallback,
    /// Maximum dimension of dense matrix materialization or factorization. This is a resource ceiling,
    /// not an empirical or universal crossover estimate. Band storage is exempt.
    pub max_dense_dimension: usize,
    /// Explicit permission for the JVP/GMRES route; its benefit is problem dependent.
    pub allow_matrix_free: bool,
}

impl Default for GeneralOdePolicy {
    fn default() -> Self {
        Self {
            regime: ProblemRegime::Unknown,
            derivatives: DerivativeFallback::Forbidden,
            max_dense_dimension: 64,
            allow_matrix_free: false,
        }
    }
}

/// The two methods admitted to the ordinary selection surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProductionMethod {
    /// RHS-only Dormand--Prince 5(4), with FSAL reuse.
    Dopri5,
    /// Eight-stage Rosenbrock RODAS5P with an embedded local error estimate.
    Rodas5p,
}

/// Deterministic selection record; it is not a certificate of solution accuracy.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct GeneralSelection {
    /// Selected numerical method.
    pub method: ProductionMethod,
    /// User-declared regime.
    pub regime: ProblemRegime,
    /// Authorized derivative policy (explicit integration never uses it).
    pub derivatives: DerivativeFallback,
    /// Implicit route, absent for the RHS-only method.
    pub implicit_route: Option<FacadeRoute>,
}

/// A feature or permission outside the selected ordinary ODE surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SelectionRefusal {
    /// Use an explicit mass-matrix API instead.
    #[error("the selected general ODE surface requires identity mass")]
    MassMatrix,
    /// Legacy events are postprocessed, not in-loop stopping.
    #[error("event termination is not implemented by the selected surface")]
    Events,
    /// Interpolation contracts remain method-specific APIs.
    #[error("continuous output requires an explicit method-specific API")]
    DenseOutput,
    /// The stiff method requires derivative action.
    #[error("stiff integration requires explicit derivative authorization")]
    DerivativesForbidden,
    /// A default finite-difference Jacobian needs separate permission.
    #[error("ProvidedOnly requires a supplied Jacobian or JVP")]
    MissingJacobian,
    /// A nonautonomous RHS requires a supplied partial time derivative.
    #[error("ProvidedOnly requires autonomous() or a supplied time derivative")]
    MissingTimeDerivative,
    /// The selected route exceeds an explicit resource ceiling.
    #[error("the dense factorization dimension exceeds the configured resource ceiling")]
    DenseDimensionLimit,
    /// GMRES use must be explicitly enabled.
    #[error("the matrix-free route requires allow_matrix_free = true")]
    MatrixFreeNotEnabled,
    /// A restriction inherited from the underlying implicit implementation.
    #[error("unsupported implicit feature combination: {0}")]
    Implicit(Unsupported),
}

/// Why the selected solve could not be initialized, or an explicitly checked
/// partial solution failed. Partial trajectories remain in the wrapped error.
#[derive(Debug, thiserror::Error)]
pub enum GeneralSolveError {
    /// Selection refused before callbacks or workspace allocation.
    #[error(transparent)]
    Selection(#[from] SelectionRefusal),
    /// Explicit initialization or checked integration failure.
    #[error(transparent)]
    Explicit(#[from] ExplicitSolveError),
    /// Implicit initialization or checked integration failure.
    #[error(transparent)]
    Implicit(#[from] SolveError),
}

/// The full method-specific result, including partial states and work counters.
// Keep the established public representation and avoid adding a heap allocation
// to every implicit result solely to equalize these payload sizes.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum GeneralResult {
    /// Dormand--Prince result.
    Explicit(ExplicitSolution),
    /// Rosenbrock result.
    Implicit(Solution),
}

/// Result and its deterministic selection receipt.
#[derive(Debug)]
pub struct GeneralSolution {
    /// How the method was selected.
    pub selection: GeneralSelection,
    /// Full result; callers can inspect method-specific diagnostics.
    pub result: GeneralResult,
}

impl GeneralSolution {
    /// Whether integration reached its requested end time.
    pub fn is_success(&self) -> bool {
        match &self.result {
            GeneralResult::Explicit(s) => s.success,
            GeneralResult::Implicit(s) => s.is_success(),
        }
    }
    /// Last accepted state, including for unsuccessful integration.
    pub fn final_state(&self) -> Option<&[f64]> {
        match &self.result {
            GeneralResult::Explicit(s) => Some(&s.final_state),
            GeneralResult::Implicit(s) => s.final_state(),
        }
    }
    /// Convert unsuccessful integration to a typed error retaining its partial result.
    pub fn ensure_success(self) -> Result<Self, GeneralSolveError> {
        let result = match self.result {
            GeneralResult::Explicit(s) => GeneralResult::Explicit(s.ensure_success()?),
            GeneralResult::Implicit(s) => GeneralResult::Implicit(s.ensure_success()?),
        };
        Ok(Self {
            selection: self.selection,
            result,
        })
    }
}

/// Explicitly scoped selection without hidden pilots or speculative methods.
pub struct GeneralOde;

impl GeneralOde {
    /// Select without calling user code or allocating numerical workspaces.
    /// Numerical input validity is checked by `solve`, not by this feature query.
    ///
    /// The decision is derived from the method catalog
    /// ([`MethodRegistry::general_selection`], phase B of research loop L1 node V03). It
    /// equals [`GeneralOde::select_reference`], the body this function had before, on
    /// every combination of the facts either of them reads; the contract test
    /// `unified_frontend_contracts` and the golden decision grid check that.
    pub fn select(
        problem: &Problem,
        options: &Options,
        policy: &GeneralOdePolicy,
    ) -> Result<GeneralSelection, SelectionRefusal> {
        MethodRegistry::general_selection(problem, options, policy)
    }

    /// The hard-coded selection that was [`GeneralOde::select`] before node V03, kept
    /// unchanged as the test oracle of the catalog-derived selection (and as its answer
    /// should the catalog ever fail to describe the general surface).
    #[doc(hidden)]
    pub fn select_reference(
        problem: &Problem,
        options: &Options,
        policy: &GeneralOdePolicy,
    ) -> Result<GeneralSelection, SelectionRefusal> {
        if problem.mass.is_some() {
            return Err(SelectionRefusal::MassMatrix);
        }
        if !problem.events.is_empty() {
            return Err(SelectionRefusal::Events);
        }
        if options.dense_output {
            return Err(SelectionRefusal::DenseOutput);
        }
        if policy.regime != ProblemRegime::Stiff {
            return Ok(GeneralSelection {
                method: ProductionMethod::Dopri5,
                regime: policy.regime,
                derivatives: policy.derivatives,
                implicit_route: None,
            });
        }
        match policy.derivatives {
            DerivativeFallback::Forbidden => return Err(SelectionRefusal::DerivativesForbidden),
            DerivativeFallback::ProvidedOnly => {
                if matches!(problem.jacobian, Jacobian::FiniteDifference) {
                    return Err(SelectionRefusal::MissingJacobian);
                }
                if !problem.autonomous && problem.time_derivative.is_none() {
                    return Err(SelectionRefusal::MissingTimeDerivative);
                }
            }
            DerivativeFallback::AllowFiniteDifference => {}
        }
        let caps = Rodas5p::capabilities(problem, options).map_err(SelectionRefusal::Implicit)?;
        if caps.route == FacadeRoute::MatrixFree && !policy.allow_matrix_free {
            return Err(SelectionRefusal::MatrixFreeNotEnabled);
        }
        // Dense callbacks with a band declaration still materialize/verify an
        // n-by-n matrix. Only native compact band or banded FD avoids that cost.
        let materializes_dense =
            caps.route == FacadeRoute::DenseFast || matches!(problem.jacobian, Jacobian::Dense(_));
        if materializes_dense && problem.dimension > policy.max_dense_dimension {
            return Err(SelectionRefusal::DenseDimensionLimit);
        }
        Ok(GeneralSelection {
            method: ProductionMethod::Rodas5p,
            regime: policy.regime,
            derivatives: policy.derivatives,
            implicit_route: Some(caps.route),
        })
    }

    /// Integrate using the recorded selection. Failed runs remain inspectable
    /// results; call [`GeneralSolution::ensure_success`] when failure must be an error.
    /// This does not automatically switch methods during a solve.
    pub fn solve(
        problem: &Problem,
        span: (f64, f64),
        y0: &[f64],
        options: &Options,
        policy: &GeneralOdePolicy,
    ) -> Result<GeneralSolution, GeneralSolveError> {
        let selection = Self::select(problem, options, policy)?;
        let result = match selection.method {
            ProductionMethod::Dopri5 => {
                GeneralResult::Explicit(Dopri5::solve(problem, span, y0, options)?)
            }
            ProductionMethod::Rodas5p => {
                GeneralResult::Implicit(Rodas5p::solve(problem, span, y0, options)?)
            }
        };
        Ok(GeneralSolution { selection, result })
    }
}

// ------------------------------------------------------------------------------------------
// Unified method capability registry (U06). Pure: no callback, allocation of numerical
// workspaces or solver attempt. Implemented is distinct from qualified; nothing here changes
// `GeneralOde::select` or any default routing.
// ------------------------------------------------------------------------------------------

/// Independent derivative and linear-algebra capabilities. In a [`MethodRequest`] they are
/// permissions; in a [`RequirementVariant`] they are needs. Flags are a set, not a ladder:
/// a JVP permission does not imply dense matrices, Newton or finite differences.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilitySet {
    /// Model-supplied Jacobian-vector products.
    pub provided_jvp: bool,
    /// Finite-difference derivative estimates from the RHS.
    pub finite_difference: bool,
    /// Automatic differentiation (no method in this repository uses it).
    pub automatic_differentiation: bool,
    /// A model-supplied explicit (dense or banded) Jacobian.
    pub explicit_jacobian: bool,
    /// Materialising and factorising dense matrices.
    pub dense_matrix: bool,
    /// Newton or simplified-Newton nonlinear iterations.
    pub newton: bool,
}

impl CapabilitySet {
    /// The empty set (RHS evaluations only).
    pub const NONE: Self = Self {
        provided_jvp: false,
        finite_difference: false,
        automatic_differentiation: false,
        explicit_jacobian: false,
        dense_matrix: false,
        newton: false,
    };

    /// Every flag of `self` is also set in `allowed`.
    pub fn is_subset_of(&self, allowed: &Self) -> bool {
        (!self.provided_jvp || allowed.provided_jvp)
            && (!self.finite_difference || allowed.finite_difference)
            && (!self.automatic_differentiation || allowed.automatic_differentiation)
            && (!self.explicit_jacobian || allowed.explicit_jacobian)
            && (!self.dense_matrix || allowed.dense_matrix)
            && (!self.newton || allowed.newton)
    }

    /// Permissions implied by a facade problem and the existing [`GeneralOdePolicy`], without
    /// calling user code. A supplied derivative is usable only if the policy permits supplied
    /// derivatives; dense matrices only up to `max_dense_dimension`; Newton is never implied
    /// (the policy has no Newton permission), so Newton-based methods stay refused.
    pub fn from_problem_and_policy(problem: &Problem, policy: &GeneralOdePolicy) -> Self {
        let supplied = !matches!(policy.derivatives, DerivativeFallback::Forbidden);
        let mut s = Self::NONE;
        match problem.jacobian {
            Jacobian::Dense(_) | Jacobian::Banded { .. } => s.explicit_jacobian = supplied,
            Jacobian::Jvp(_) | Jacobian::SparseJvp { .. } => {
                s.provided_jvp = supplied && policy.allow_matrix_free
            }
            Jacobian::FiniteDifference => {}
        }
        s.finite_difference = matches!(
            policy.derivatives,
            DerivativeFallback::AllowFiniteDifference
        );
        s.dense_matrix = problem.dimension <= policy.max_dense_dimension;
        s
    }
}

/// One admissible way to run a method: what each phase needs and which features it supports.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RequirementVariant {
    /// Short label of the variant.
    pub label: &'static str,
    /// Needs of every accepted or rejected step.
    pub per_step: CapabilitySet,
    /// Needs of the starting procedure.
    pub startup: CapabilitySet,
    /// Needs of an automatic fallback, if the method has one (none currently do).
    pub fallback: Option<CapabilitySet>,
    /// Non-identity mass matrices.
    pub supports_mass: bool,
    /// Event location.
    pub supports_events: bool,
    /// Continuous (dense) output.
    pub supports_dense_output: bool,
}

/// Evidence level of a method; implemented is not qualified.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MethodQualification {
    /// Production method of the facade.
    Production,
    /// Qualified on calibration problems with recorded failure envelopes (not production).
    ScopedCalibration,
    /// Implemented and contract-tested, without method qualification.
    ImplementedUnqualified,
    /// Reference implementation used as a comparator only.
    ReferenceImplementationOnly,
}

/// How a caller reaches the method.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MethodAccess {
    /// May be chosen by [`GeneralOde::select`].
    GeneralOdeSelectable,
    /// Only through its explicit API path; never selected automatically.
    ExplicitApiOnly,
}

/// Descriptor of one method family in the registry.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MethodDescriptor {
    /// Stable method identifier.
    pub method_id: &'static str,
    /// Registry entry version.
    pub version: &'static str,
    /// Coefficient/tableau identity or citation.
    pub tableau_identity: &'static str,
    /// Stages per step.
    pub stages: usize,
    /// Formal (variable-step) order.
    pub order: usize,
    /// Number of steps of history the step map uses (1: one-step).
    pub history_steps: usize,
    /// Starting procedure.
    pub startup: &'static str,
    /// Admissible requirement variants, in preference order.
    pub variants: Vec<RequirementVariant>,
    /// Evidence level.
    pub qualification: MethodQualification,
    /// Access path.
    pub access: MethodAccess,
    /// Real exported entry point.
    pub route: &'static str,
    /// Where the qualification evidence lives.
    pub evidence: &'static str,
}

/// A declarative request; no callbacks are attached.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MethodRequest {
    /// Permitted capabilities, for every phase including startup and fallback.
    pub allowed: CapabilitySet,
    /// A non-identity mass matrix is required.
    pub needs_mass: bool,
    /// Event location is required.
    pub needs_events: bool,
    /// Continuous output is required.
    pub needs_dense_output: bool,
    /// Admit methods that are only implemented or reference implementations.
    pub admit_unqualified: bool,
}

/// Phase whose needs exceed the permitted set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MethodPhase {
    /// Per-step work.
    PerStep,
    /// Starting procedure.
    Startup,
    /// Automatic fallback.
    Fallback,
}

/// Why a method was not admitted for a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RegistryRefusal {
    /// The qualification is below the request's admission level.
    Unqualified,
    /// No variant supports a non-identity mass matrix.
    MassUnsupported,
    /// No variant supports events.
    EventsUnsupported,
    /// No variant supports continuous output.
    DenseOutputUnsupported,
    /// No single variant supports the requested combination of features.
    FeatureCombinationUnsupported,
    /// A variant needs capabilities outside the permitted set in this phase.
    Permission {
        /// Variant label.
        variant: &'static str,
        /// Phase that needs more.
        phase: MethodPhase,
    },
    /// The Jacobian provider form of the problem (or its declared band) is one that no
    /// variant of the method lowers. Reported by admissions that read the problem
    /// ([`PeerOde::admission`], [`MethodRegistry::route`]), never by [`MethodRegistry::query`],
    /// whose [`MethodRequest`] does not carry the provider form.
    ProviderFormUnsupported,
}

/// Outcome for one method.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MethodAdmission {
    /// Admitted with the first satisfying variant.
    Admitted {
        /// Method identifier.
        method_id: &'static str,
        /// Variant label.
        variant: &'static str,
        /// Evidence level.
        qualification: MethodQualification,
        /// Access path.
        access: MethodAccess,
    },
    /// Not admitted, with every reason found.
    Refused {
        /// Method identifier.
        method_id: &'static str,
        /// Reasons.
        reasons: Vec<RegistryRefusal>,
    },
}

const fn caps(jvp: bool, explicit: bool, fd: bool, dense: bool, newton: bool) -> CapabilitySet {
    CapabilitySet {
        provided_jvp: jvp,
        finite_difference: fd,
        automatic_differentiation: false,
        explicit_jacobian: explicit,
        dense_matrix: dense,
        newton,
    }
}

fn variant(
    label: &'static str,
    per_step: CapabilitySet,
    startup: CapabilitySet,
    features: (bool, bool, bool),
) -> RequirementVariant {
    RequirementVariant {
        label,
        per_step,
        startup,
        fallback: None,
        supports_mass: features.0,
        supports_events: features.1,
        supports_dense_output: features.2,
    }
}

/// Registry of the methods present in this repository.
pub struct MethodRegistry;

/// Version of the registry entries.
pub const METHOD_REGISTRY_VERSION: &str = "u06-fd-2026-11-24";

impl MethodRegistry {
    /// All descriptors. Requirements were read from the implementations: RODAS5P per
    /// `capabilities.rs`; PPSW1 per `ppsw1.rs`/`ppsw1_linear_backend.rs` (dense J or `n`
    /// JVP columns, dense starter with simplified Newton); BDF and Radau assemble a dense
    /// Jacobian (provided, from `n` JVPs, or by RHS differences) and run Newton.
    pub fn descriptors() -> Vec<MethodDescriptor> {
        let none = CapabilitySet::NONE;
        let v = METHOD_REGISTRY_VERSION;
        vec![
            MethodDescriptor {
                method_id: "dopri5",
                version: v,
                tableau_identity: "Dormand-Prince 5(4), FSAL",
                stages: 7,
                order: 5,
                history_steps: 1,
                startup: "none",
                variants: vec![variant("rhs-only", none, none, (false, false, false))],
                qualification: MethodQualification::Production,
                access: MethodAccess::GeneralOdeSelectable,
                route: "vigilode::Dopri5::solve",
                evidence: "vigilode explicit contracts",
            },
            MethodDescriptor {
                method_id: "rodas5p",
                version: v,
                tableau_identity: "RODAS5P, fixtures/rodas5p_coefficients_snapshot.json",
                stages: 8,
                order: 5,
                history_steps: 1,
                startup: "none",
                variants: vec![
                    variant(
                        "explicit-jacobian-dense",
                        caps(false, true, false, true, false),
                        none,
                        (true, true, true),
                    ),
                    variant(
                        "finite-difference-dense",
                        caps(false, false, true, true, false),
                        none,
                        (true, true, true),
                    ),
                    variant(
                        "jvp-matrix-free",
                        caps(true, false, false, false, false),
                        none,
                        (false, false, false),
                    ),
                ],
                qualification: MethodQualification::Production,
                access: MethodAccess::GeneralOdeSelectable,
                route: "vigilode::Rodas5p::solve",
                evidence: "vigilode facade and capabilities contracts",
            },
            MethodDescriptor {
                method_id: rodas5p_integrators::ppsw1::METHOD_ID,
                version: v,
                tableau_identity: "Schmitt-Weiner SINUM 42(1) 2004, method 6b; route-R1 oracle",
                stages: rodas5p_integrators::ppsw1::STAGES,
                order: rodas5p_integrators::ppsw1::FORMAL_ORDER,
                history_steps: 2,
                startup: "Radau IIA (5 stages) over 2 h0, dense 5n LU, simplified Newton",
                variants: vec![
                    variant(
                        "explicit-jacobian-dense",
                        caps(false, true, false, true, false),
                        caps(false, true, false, true, true),
                        (false, false, false),
                    ),
                    variant(
                        "jvp-columns-dense",
                        caps(true, false, false, true, false),
                        caps(true, false, false, true, true),
                        (false, false, false),
                    ),
                ],
                qualification: MethodQualification::ScopedCalibration,
                access: MethodAccess::ExplicitApiOnly,
                route: "rodas5p_integrators::ppsw1::Session",
                evidence: "research/production_unification_20261115/peer_qualification",
            },
            MethodDescriptor {
                method_id: "ppsw1-6b-matrix-free-common-w",
                version: v,
                tableau_identity: "as ppsw1-6b; JVP W = I - gamma h J(t_m, y_m), GMRES",
                stages: rodas5p_integrators::ppsw1::STAGES,
                order: rodas5p_integrators::ppsw1::FORMAL_ORDER,
                history_steps: 2,
                startup: "dense PPSW1 starter (no matrix-free starter)",
                variants: vec![
                    variant(
                        "jvp-steps-explicit-jacobian-startup",
                        caps(true, false, false, false, false),
                        caps(false, true, false, true, true),
                        (false, false, false),
                    ),
                    variant(
                        "jvp-steps-jvp-columns-startup",
                        caps(true, false, false, false, false),
                        caps(true, false, false, true, true),
                        (false, false, false),
                    ),
                ],
                qualification: MethodQualification::ImplementedUnqualified,
                access: MethodAccess::ExplicitApiOnly,
                route: "rodas5p_integrators::ppsw1::linear_backend::LinearBackend",
                evidence: "research/production_unification_20261115/matrix_free_w",
            },
            MethodDescriptor {
                method_id: "bdf1-bdf2-variable",
                version: v,
                tableau_identity: "variable-step BDF1/BDF2 (bdf.rs)",
                stages: 1,
                order: 2,
                history_steps: 2,
                startup: "BDF1 steps",
                variants: vec![
                    variant(
                        "explicit-jacobian-dense-newton",
                        caps(false, true, false, true, true),
                        caps(false, true, false, true, true),
                        (false, false, false),
                    ),
                    variant(
                        "jvp-columns-dense-newton",
                        caps(true, false, false, true, true),
                        caps(true, false, false, true, true),
                        (false, false, false),
                    ),
                    variant(
                        "finite-difference-dense-newton",
                        caps(false, false, true, true, true),
                        caps(false, false, true, true, true),
                        (false, false, false),
                    ),
                ],
                qualification: MethodQualification::ImplementedUnqualified,
                access: MethodAccess::ExplicitApiOnly,
                route: "rodas5p_integrators::integrate_bdf_adaptive_observed",
                evidence: "crates/rodas5p-integrators/src/bdf.rs contracts",
            },
            MethodDescriptor {
                method_id: "radau-iia3",
                version: v,
                // `RadauConfig::default()` runs `RadauIiaStages::Three`: three stages, order
                // five, with the embedded third-order estimate (the U06 entry said two
                // stages and order three; corrected by node V03 against radau.rs).
                tableau_identity: "3-stage Radau IIA, order 5, embedded order-3 estimate (radau.rs)",
                stages: 3,
                order: 5,
                history_steps: 1,
                startup: "none",
                variants: vec![
                    variant(
                        "explicit-jacobian-dense-newton",
                        caps(false, true, false, true, true),
                        none,
                        (false, false, false),
                    ),
                    variant(
                        "jvp-columns-dense-newton",
                        caps(true, false, false, true, true),
                        none,
                        (false, false, false),
                    ),
                    variant(
                        "finite-difference-dense-newton",
                        caps(false, false, true, true, true),
                        none,
                        (false, false, false),
                    ),
                ],
                qualification: MethodQualification::ReferenceImplementationOnly,
                access: MethodAccess::ExplicitApiOnly,
                route: "rodas5p_integrators::integrate_radau_adaptive_observed",
                evidence: "CLI comparator fidelity: reference-implementation-only",
            },
        ]
    }

    /// Admission of every registered method for `request`. Pure.
    pub fn query(request: &MethodRequest) -> Vec<MethodAdmission> {
        Self::descriptors()
            .iter()
            .map(|d| Self::admit(d, request))
            .collect()
    }

    /// Admission of one descriptor. Pure.
    pub fn admit(d: &MethodDescriptor, request: &MethodRequest) -> MethodAdmission {
        let mut reasons = Vec::new();
        let qualified = matches!(
            d.qualification,
            MethodQualification::Production | MethodQualification::ScopedCalibration
        );
        if !qualified && !request.admit_unqualified {
            reasons.push(RegistryRefusal::Unqualified);
        }
        let feature_ok = |v: &RequirementVariant| {
            (!request.needs_mass || v.supports_mass)
                && (!request.needs_events || v.supports_events)
                && (!request.needs_dense_output || v.supports_dense_output)
        };
        if request.needs_mass && !d.variants.iter().any(|v| v.supports_mass) {
            reasons.push(RegistryRefusal::MassUnsupported);
        }
        if request.needs_events && !d.variants.iter().any(|v| v.supports_events) {
            reasons.push(RegistryRefusal::EventsUnsupported);
        }
        if request.needs_dense_output && !d.variants.iter().any(|v| v.supports_dense_output) {
            reasons.push(RegistryRefusal::DenseOutputUnsupported);
        }
        let mut chosen = None;
        for v in d.variants.iter().filter(|v| feature_ok(v)) {
            let mut ok = true;
            for (phase, need) in [
                (MethodPhase::PerStep, Some(v.per_step)),
                (MethodPhase::Startup, Some(v.startup)),
                (MethodPhase::Fallback, v.fallback),
            ] {
                if let Some(need) = need
                    && !need.is_subset_of(&request.allowed)
                {
                    ok = false;
                    reasons.push(RegistryRefusal::Permission {
                        variant: v.label,
                        phase,
                    });
                }
            }
            if ok && chosen.is_none() {
                chosen = Some(v.label);
            }
        }
        match chosen {
            Some(label)
                if reasons
                    .iter()
                    .all(|r| matches!(r, RegistryRefusal::Permission { .. })) =>
            {
                MethodAdmission::Admitted {
                    method_id: d.method_id,
                    variant: label,
                    qualification: d.qualification,
                    access: d.access,
                }
            }
            _ => {
                if reasons.is_empty() {
                    // each requested feature exists in some variant, but no single variant
                    // offers the requested combination
                    reasons.push(RegistryRefusal::FeatureCombinationUnsupported);
                }
                MethodAdmission::Refused {
                    method_id: d.method_id,
                    reasons,
                }
            }
        }
    }
}
