//! Arm registry of the component-triage harness (research node
//! `research/component_triage_20261019`, `HARNESS_CONTRACT.md` section 1).
//!
//! An arm is one integrator entry point with one configuration. Its
//! `components` are the ids of `COMPONENTS.json` it exercises; `requires`
//! says which problems it applies to. Every arm runs one integration segment
//! through [`run_arm`]; [`prepare`] builds everything that does not belong to
//! the integration itself (declared problems, bands, configurations), so the
//! allocation audit can count the integration call alone.
//!
//! The existing `stiff_benchmark` arms are reused through
//! `stiff_benchmark::run_arm_with`, `run_banded_with`, `run_sp03_with`,
//! `run_small_full` and `run_small_static_full`; nothing here changes them.

use rodas5p_core::{
    CoreError, CoreResult, InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind,
};
use rodas5p_integrators::{
    AdaptiveFusedExponentialDiagnostics, AdaptiveObservedIntegrationResult, AdaptiveStepConfig,
    BandedJacobian, BandedKernel, BandedWork, BdfConfig, BdfOrder, BlockMethod, ControllerKind,
    DenseErrorControl, DenseErrorReport, DenseOutputError, FusedPhiKrylovConfig,
    GcrodrRecyclePolicy, HomotopyPathConfig, HomotopyPredictor, HomotopyStepConfig,
    IntegrationMethod, KrylovWorkspaceOptions, MaxStepPolicy, ObservedIntegrationResult,
    OdeProblem, OutputSamplingPlan, OutputSchedule, ParallelExecution, RadauConfig, RadauIiaStages,
    Rodas5pFastLu, Rodas5pFastResult, Rodas5pMfFastResult, Rodas5pMfFastWorkspace, RoutingOptions,
    RoutingRecord, SabrConfig, StageSolveStatistics, StageTargetOptions, StageTargetPolicy,
    StructureDeclaration, TransactionalQ1Q2Config, TransactionalQ1Q2RunDiagnostics,
    integrate_adaptive_dense_observed_with_dense_error_control,
    integrate_adaptive_observed_with_config, integrate_bdf_adaptive_observed,
    integrate_homotopy_adaptive_observed, integrate_pexprb54s4_fused_adaptive_observed,
    integrate_radau_adaptive_observed, integrate_rodas5p_fast_observed,
    integrate_rodas5p_fast_observed_with_options, integrate_rodas5p_mf_fast_observed,
    integrate_rodas5p_mf_fast_observed_gmres_into,
    integrate_rodas5p_mf_fast_observed_with_gcrodr_policy,
    integrate_rodas5p_mf_fast_observed_with_stage_target,
    integrate_sequential_matrix_free_adaptive_dense_observed,
    integrate_transactional_q1_q2_adaptive_observed,
};
use serde_json::{Value, json};

use crate::stiff_benchmark::{self, BenchmarkProblem};

/// What an arm needs from the problem's Jacobian.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JacobianRequirement {
    /// An explicit (analytic) Jacobian callback.
    Analytic,
    /// A user JVP callback (strict matrix-free arms).
    Jvp,
    /// Either: the arm builds what it needs from what the problem has.
    Any,
}

impl JacobianRequirement {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Analytic => "analytic",
            Self::Jvp => "jvp",
            Self::Any => "any",
        }
    }
}

/// The problems an arm applies to (`HARNESS_CONTRACT.md`: a problem is
/// applicable iff every field is satisfied).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Requires {
    pub jacobian: JacobianRequirement,
    /// `None`, `banded` or `sparse_linear_part`: the problem's structure
    /// must equal it when set.
    pub structure: Option<&'static str>,
    /// Whether the arm accepts a non-identity mass matrix. `false`: only
    /// problems with `mass = false` apply.
    pub mass: bool,
    /// Compile-time dimensions of a fixed-dimension arm: the problem's
    /// dimension must be one of them.
    pub fixed_dimension: Option<&'static [usize]>,
}

impl Requires {
    pub(crate) fn json(&self) -> Value {
        json!({
            "jacobian": self.jacobian.id(),
            "structure": self.structure,
            "mass": self.mass,
            "fixed_dimension": self.fixed_dimension,
        })
    }
}

/// How an arm changes the cell's step configuration.
#[derive(Clone, Copy, Debug)]
pub(crate) enum AdaptiveOverride {
    None,
    Controller(ControllerKind),
    StrictMaxStep,
}

/// A set of switches of the U-form matrix-free workspace.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MfWorkspaceArm {
    method: LinearMethod,
    gcrodr: GcrodrRecyclePolicy,
    explicit_zero: bool,
    current_w: bool,
    workspace: KrylovWorkspaceOptions,
}

/// The entry point and configuration of an arm.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ArmKind {
    /// A `stiff_benchmark::run_arm_with` arm id.
    Bench(&'static str),
    /// `integrate_rodas5p_fast_observed` (`None`) or
    /// `integrate_rodas5p_fast_observed_with_options` with the options of a
    /// `stiff_benchmark::option_arm` id.
    Fast(Option<&'static str>),
    /// `stiff_benchmark::run_small_full` with the default options (`None`)
    /// or those of a small `stiff_benchmark::option_arm` id.
    Small(Option<&'static str>),
    /// `stiff_benchmark::run_small_static_full` with the options of a
    /// `stiff_benchmark::small_static_arm` id.
    SmallStatic(&'static str),
    /// Sequential RODAS5P with an explicit Jacobian and a Krylov method.
    SeqKrylov(LinearMethod, PreconditionerKind),
    /// The sequential direct dense-output driver with a dense error mode.
    SeqDense(DenseErrorControl),
    /// The sequential matrix-free dense-output driver (WRMS inner forcing).
    SeqMf(LinearMethod, InitialGuess),
    Banded(BandedKernel),
    /// The router; `declare` attaches the problem's structure first.
    Routed {
        declare: bool,
    },
    /// A `stiff_benchmark::run_sp03_with` matrix-free arm id.
    Me01(&'static str),
    MfStageTarget(StageTargetPolicy),
    MfGcrodr(GcrodrRecyclePolicy),
    /// `integrate_rodas5p_mf_fast_observed` with another Krylov method.
    MfMethod(LinearMethod),
    /// `integrate_rodas5p_mf_fast_observed_gmres_into` with another start.
    MfX0(InitialGuess),
    MfWorkspace(MfWorkspaceArm),
    Bdf(BdfOrder),
    Radau(RadauIiaStages),
    Sabr(BlockMethod),
    Homotopy,
    Transactional,
    Pexprb,
    Unwired,
}

pub(crate) struct ArmSpec {
    pub id: &'static str,
    pub components: &'static [&'static str],
    pub requires: Requires,
    pub wired: bool,
    pub unwired_reason: Option<&'static str>,
    pub adaptive: AdaptiveOverride,
    pub kind: ArmKind,
}

const ANALYTIC: Requires = Requires {
    jacobian: JacobianRequirement::Analytic,
    structure: None,
    mass: true,
    fixed_dimension: None,
};
const ANALYTIC_NO_MASS: Requires = Requires {
    mass: false,
    ..ANALYTIC
};
const JVP: Requires = Requires {
    jacobian: JacobianRequirement::Jvp,
    structure: None,
    mass: true,
    fixed_dimension: None,
};
const JVP_NO_MASS: Requires = Requires { mass: false, ..JVP };
const ANY: Requires = Requires {
    jacobian: JacobianRequirement::Any,
    structure: None,
    mass: true,
    fixed_dimension: None,
};
const ANY_NO_MASS: Requires = Requires { mass: false, ..ANY };
const BANDED: Requires = Requires {
    structure: Some("banded"),
    ..ANALYTIC_NO_MASS
};
/// The compile-time dimensions of the small driver's problems (van der
/// Pol 2, Robertson 3, HIRES 8); the driver itself refuses every other
/// problem.
const SMALL_DIMENSIONS: &[usize] = &[2, 3, 8];
const SMALL: Requires = Requires {
    fixed_dimension: Some(SMALL_DIMENSIONS),
    ..ANALYTIC_NO_MASS
};

const fn wired(
    id: &'static str,
    components: &'static [&'static str],
    requires: Requires,
    kind: ArmKind,
) -> ArmSpec {
    ArmSpec {
        id,
        components,
        requires,
        wired: true,
        unwired_reason: None,
        adaptive: AdaptiveOverride::None,
        kind,
    }
}

const fn with_adaptive(mut arm: ArmSpec, adaptive: AdaptiveOverride) -> ArmSpec {
    arm.adaptive = adaptive;
    arm
}

const fn mf_ws(
    method: LinearMethod,
    gcrodr: GcrodrRecyclePolicy,
    explicit_zero: bool,
    current_w: bool,
    workspace: KrylovWorkspaceOptions,
) -> ArmKind {
    ArmKind::MfWorkspace(MfWorkspaceArm {
        method,
        gcrodr,
        explicit_zero,
        current_w,
        workspace,
    })
}

const FAST_LU: [&str; 3] = ["la.faer_lu", "la.small_lu", "la.sparse_density_switch"];

static ARMS: &[ArmSpec] = &[
    // ---------------------------------------------- sequential, explicit J
    wired(
        "seq-direct",
        &[
            "drv.seq_direct",
            "la.faer_lu",
            "ctl.integral",
            "out.collector",
            "ctl.max_step_slack",
        ],
        ANALYTIC,
        ArmKind::Bench("rodas5p"),
    ),
    wired(
        "seq-direct-dense-off",
        &[
            "drv.seq_direct",
            "la.faer_lu",
            "ctl.integral",
            "ctl.max_step_slack",
            "out.dense_off",
        ],
        ANALYTIC,
        ArmKind::SeqDense(DenseErrorControl::Off),
    ),
    wired(
        "seq-direct-dense-report-defect",
        &[
            "drv.seq_direct",
            "la.faer_lu",
            "ctl.integral",
            "ctl.max_step_slack",
            "out.dense_report_defect",
        ],
        ANALYTIC,
        ArmKind::SeqDense(DenseErrorControl::ReportDefect),
    ),
    wired(
        "seq-direct-dense-report",
        &[
            "drv.seq_direct",
            "la.faer_lu",
            "ctl.integral",
            "ctl.max_step_slack",
            "out.dense_report",
        ],
        ANALYTIC,
        ArmKind::SeqDense(DenseErrorControl::Report),
    ),
    wired(
        "seq-direct-dense-enforce",
        &[
            "drv.seq_direct",
            "la.faer_lu",
            "ctl.integral",
            "ctl.max_step_slack",
            "out.dense_enforce",
        ],
        ANALYTIC,
        ArmKind::SeqDense(DenseErrorControl::Enforce),
    ),
    with_adaptive(
        wired(
            "seq-direct-maxstep-strict",
            &[
                "drv.seq_direct",
                "la.faer_lu",
                "ctl.integral",
                "out.collector",
                "ctl.max_step_strict",
            ],
            ANALYTIC,
            ArmKind::Bench("rodas5p"),
        ),
        AdaptiveOverride::StrictMaxStep,
    ),
    wired(
        "seq-gmres-pc-none",
        &[
            "drv.seq_krylov",
            "kr.gmres",
            "kr.pc_none",
            "ctl.integral",
            "out.collector",
        ],
        ANALYTIC,
        ArmKind::SeqKrylov(LinearMethod::Gmres, PreconditionerKind::None),
    ),
    wired(
        "seq-gmres-pc-jacobi",
        &[
            "drv.seq_krylov",
            "kr.gmres",
            "kr.pc_jacobi",
            "ctl.integral",
            "out.collector",
        ],
        ANALYTIC,
        ArmKind::SeqKrylov(LinearMethod::Gmres, PreconditionerKind::Jacobi),
    ),
    wired(
        "seq-gmres-pc-direct",
        &[
            "drv.seq_krylov",
            "kr.gmres",
            "kr.pc_direct",
            "ctl.integral",
            "out.collector",
        ],
        ANALYTIC,
        ArmKind::SeqKrylov(LinearMethod::Gmres, PreconditionerKind::Direct),
    ),
    wired(
        "seq-lgmres",
        &[
            "drv.seq_krylov",
            "kr.lgmres",
            "kr.pc_none",
            "ctl.integral",
            "out.collector",
        ],
        ANALYTIC,
        ArmKind::SeqKrylov(LinearMethod::Lgmres, PreconditionerKind::None),
    ),
    wired(
        "seq-gcrodr",
        &[
            "drv.seq_krylov",
            "kr.gcrodr",
            "kr.pc_none",
            "ctl.integral",
            "out.collector",
        ],
        ANALYTIC,
        ArmKind::SeqKrylov(LinearMethod::Gcrodr, PreconditionerKind::None),
    ),
    // ------------------------------------------ sequential matrix-free
    wired(
        "seq-mf-gmres",
        &[
            "drv.seq_mf",
            "ctl.inner_tolerance",
            "kr.gmres",
            "kr.accounting_recompute_final",
            "ctl.integral",
        ],
        JVP,
        ArmKind::SeqMf(LinearMethod::Gmres, InitialGuess::Previous),
    ),
    wired(
        "seq-mf-lgmres",
        &[
            "drv.seq_mf",
            "ctl.inner_tolerance",
            "kr.lgmres",
            "ctl.integral",
        ],
        JVP,
        ArmKind::SeqMf(LinearMethod::Lgmres, InitialGuess::Previous),
    ),
    wired(
        "seq-mf-gcrodr",
        &[
            "drv.seq_mf",
            "ctl.inner_tolerance",
            "kr.gcrodr",
            "ctl.integral",
        ],
        JVP,
        ArmKind::SeqMf(LinearMethod::Gcrodr, InitialGuess::Previous),
    ),
    wired(
        "seq-mf-gmres-x0-zero",
        &[
            "drv.seq_mf",
            "ctl.inner_tolerance",
            "kr.gmres",
            "kr.accounting_recompute_final",
            "ctl.integral",
            "kr.x0_zero",
        ],
        JVP,
        ArmKind::SeqMf(LinearMethod::Gmres, InitialGuess::Zero),
    ),
    wired(
        "seq-mf-gmres-x0-previous",
        &[
            "drv.seq_mf",
            "ctl.inner_tolerance",
            "kr.gmres",
            "kr.accounting_recompute_final",
            "ctl.integral",
            "kr.x0_previous",
        ],
        JVP,
        ArmKind::SeqMf(LinearMethod::Gmres, InitialGuess::Previous),
    ),
    wired(
        "seq-mf-gmres-x0-prevstep",
        &[
            "drv.seq_mf",
            "ctl.inner_tolerance",
            "kr.gmres",
            "kr.accounting_recompute_final",
            "ctl.integral",
            "kr.x0_prevstep",
        ],
        JVP,
        ArmKind::SeqMf(LinearMethod::Gmres, InitialGuess::PreviousStep),
    ),
    wired(
        "seq-mf-gmres-x0-prevstep-scaled",
        &[
            "drv.seq_mf",
            "ctl.inner_tolerance",
            "kr.gmres",
            "kr.accounting_recompute_final",
            "ctl.integral",
            "kr.x0_prevstep_scaled",
        ],
        JVP,
        ArmKind::SeqMf(LinearMethod::Gmres, InitialGuess::PreviousStepScaled),
    ),
    // ------------------------------------------------------- fast driver
    wired(
        "fast",
        &[
            "drv.fast",
            FAST_LU[0],
            FAST_LU[1],
            FAST_LU[2],
            "ctl.integral",
        ],
        ANALYTIC_NO_MASS,
        ArmKind::Fast(None),
    ),
    with_adaptive(
        wired(
            "fast-ctl-pi",
            &["drv.fast", FAST_LU[0], FAST_LU[1], FAST_LU[2], "ctl.pi"],
            ANALYTIC_NO_MASS,
            ArmKind::Fast(None),
        ),
        AdaptiveOverride::Controller(ControllerKind::Pi),
    ),
    with_adaptive(
        wired(
            "fast-ctl-predictive",
            &[
                "drv.fast",
                FAST_LU[0],
                FAST_LU[1],
                FAST_LU[2],
                "ctl.predictive",
            ],
            ANALYTIC_NO_MASS,
            ArmKind::Fast(None),
        ),
        AdaptiveOverride::Controller(ControllerKind::Predictive),
    ),
    with_adaptive(
        wired(
            "fast-ctl-predictive-capped",
            &[
                "drv.fast",
                FAST_LU[0],
                FAST_LU[1],
                FAST_LU[2],
                "ctl.predictive_capped",
            ],
            ANALYTIC_NO_MASS,
            ArmKind::Fast(None),
        ),
        AdaptiveOverride::Controller(ControllerKind::PredictiveCapped),
    ),
    with_adaptive(
        wired(
            "fast-ctl-predictive-capped2",
            &[
                "drv.fast",
                FAST_LU[0],
                FAST_LU[1],
                FAST_LU[2],
                "ctl.predictive_capped2",
            ],
            ANALYTIC_NO_MASS,
            ArmKind::Fast(None),
        ),
        AdaptiveOverride::Controller(ControllerKind::PredictiveCapped2),
    ),
    // The option arms of `stiff_benchmark::OPTION_ARMS`, under their ids.
    wired(
        "rodas5p-fast-val",
        &[
            "drv.fast",
            FAST_LU[0],
            FAST_LU[1],
            FAST_LU[2],
            "ctl.integral",
            "ctl.fast_prevalidated",
        ],
        ANALYTIC_NO_MASS,
        ArmKind::Fast(Some("rodas5p-fast-val")),
    ),
    wired(
        "rodas5p-fast-land",
        &[
            "drv.fast",
            FAST_LU[0],
            FAST_LU[1],
            FAST_LU[2],
            "ctl.integral",
            "ctl.fast_fused_landing",
        ],
        ANALYTIC_NO_MASS,
        ArmKind::Fast(Some("rodas5p-fast-land")),
    ),
    wired(
        "rodas5p-fast-ovh",
        &[
            "drv.fast",
            FAST_LU[0],
            FAST_LU[1],
            FAST_LU[2],
            "ctl.integral",
            "ctl.fast_prevalidated",
            "ctl.fast_fused_landing",
        ],
        ANALYTIC_NO_MASS,
        ArmKind::Fast(Some("rodas5p-fast-ovh")),
    ),
    wired(
        "rodas5p-fast-small-val",
        &["drv.fast_small", "ctl.integral", "ctl.fast_prevalidated"],
        SMALL,
        ArmKind::Small(Some("rodas5p-fast-small-val")),
    ),
    wired(
        "rodas5p-fast-small-land",
        &["drv.fast_small", "ctl.integral", "ctl.fast_fused_landing"],
        SMALL,
        ArmKind::Small(Some("rodas5p-fast-small-land")),
    ),
    wired(
        "rodas5p-fast-small-ovh",
        &[
            "drv.fast_small",
            "ctl.integral",
            "ctl.fast_prevalidated",
            "ctl.fast_fused_landing",
        ],
        SMALL,
        ArmKind::Small(Some("rodas5p-fast-small-ovh")),
    ),
    wired(
        "rodas5p-fast-colext",
        &["drv.fast", "la.lu_colext", "ctl.integral"],
        ANALYTIC_NO_MASS,
        ArmKind::Fast(Some("rodas5p-fast-colext")),
    ),
    wired(
        "rodas5p-fast-colext64",
        &["drv.fast", "la.small_lu", "la.lu_colext64", "ctl.integral"],
        ANALYTIC_NO_MASS,
        ArmKind::Fast(Some("rodas5p-fast-colext64")),
    ),
    // ---------------------------------------------------- banded, small
    wired(
        "fast-banded-indexed",
        &["drv.fast_banded", "la.banded_indexed", "ctl.integral"],
        BANDED,
        ArmKind::Banded(BandedKernel::Indexed),
    ),
    wired(
        "fast-banded-slices",
        &["drv.fast_banded", "la.banded_slices", "ctl.integral"],
        BANDED,
        ArmKind::Banded(BandedKernel::Slices),
    ),
    wired(
        "fast-small",
        &["drv.fast_small", "ctl.integral"],
        SMALL,
        ArmKind::Small(None),
    ),
    wired(
        "rodas5p-fast-small-static",
        &[
            "drv.fast_small",
            "ctl.integral",
            "sm.static_stages",
            "sm.static_solves",
        ],
        SMALL,
        ArmKind::SmallStatic("rodas5p-fast-small-static"),
    ),
    wired(
        "rodas5p-fast-small-static-stages",
        &["drv.fast_small", "ctl.integral", "sm.static_stages"],
        SMALL,
        ArmKind::SmallStatic("rodas5p-fast-small-static-stages"),
    ),
    wired(
        "rodas5p-fast-small-static-ovh",
        &[
            "drv.fast_small",
            "ctl.integral",
            "sm.static_stages",
            "sm.static_solves",
            "ctl.fast_prevalidated",
            "ctl.fast_fused_landing",
        ],
        SMALL,
        ArmKind::SmallStatic("rodas5p-fast-small-static-ovh"),
    ),
    ArmSpec {
        id: "fast-small-batch4",
        components: &["drv.fast_small_batch"],
        requires: SMALL,
        wired: false,
        unwired_reason: Some(
            "integrate_rodas5p_fast_small_batch integrates B ensemble members on lanes at once; \
             it has no single-problem entry point (stiff-ensemble-run covers the van der Pol \
             ensemble)",
        ),
        adaptive: AdaptiveOverride::None,
        kind: ArmKind::Unwired,
    },
    // ------------------------------------------------------------ routed
    wired(
        "routed",
        &["drv.routed"],
        ANY_NO_MASS,
        ArmKind::Routed { declare: true },
    ),
    wired(
        "routed-undeclared",
        &["drv.routed"],
        ANY_NO_MASS,
        ArmKind::Routed { declare: false },
    ),
    // ------------------------------------- U-form matrix-free (ME01 ...)
    wired(
        "mf",
        &["drv.mf_fast", "kr.gmres", "kr.accounting_recompute_final"],
        JVP_NO_MASS,
        ArmKind::Me01(stiff_benchmark::MF_ARM),
    ),
    wired(
        "mf-into",
        &[
            "drv.mf_fast",
            "kr.gmres_into",
            "kr.accounting_recompute_final",
        ],
        JVP_NO_MASS,
        ArmKind::Me01(stiff_benchmark::MF_INTO_ARM),
    ),
    wired(
        "mf-ls-ws",
        &[
            "drv.mf_fast",
            "kr.gmres_into",
            "kr.ls_workspace",
            "kr.accounting_recompute_final",
        ],
        JVP_NO_MASS,
        ArmKind::Me01(stiff_benchmark::MF_LS_WS_ARM),
    ),
    wired(
        "mf-legacy-v2",
        &[
            "drv.mf_fast",
            "kr.gmres_into",
            "kr.accounting_reuse_confirmed",
        ],
        JVP_NO_MASS,
        ArmKind::Me01(stiff_benchmark::MF_LEGACY_V2_ARM),
    ),
    wired(
        "mf-stage-target-legacy",
        &["drv.mf_fast", "kr.gmres_into", "kr.stage_target_legacy"],
        JVP_NO_MASS,
        ArmKind::MfStageTarget(StageTargetPolicy::Legacy),
    ),
    wired(
        "mf-stage-target-projl2",
        &["drv.mf_fast", "kr.gmres_staged", "kr.stage_target_projl2"],
        JVP_NO_MASS,
        ArmKind::MfStageTarget(StageTargetPolicy::ProjL2),
    ),
    wired(
        "mf-stage-target-l2coupled",
        &[
            "drv.mf_fast",
            "kr.gmres_staged",
            "kr.stage_target_l2coupled",
        ],
        JVP_NO_MASS,
        ArmKind::MfStageTarget(StageTargetPolicy::L2Coupled),
    ),
    wired(
        "mf-stage-target-coupled",
        &["drv.mf_fast", "kr.gmres_staged", "kr.stage_target_coupled"],
        JVP_NO_MASS,
        ArmKind::MfStageTarget(StageTargetPolicy::Coupled),
    ),
    wired(
        "mf-stage-target-coupled-guarded",
        &[
            "drv.mf_fast",
            "kr.gmres_staged",
            "kr.stage_target_coupled_guarded",
        ],
        JVP_NO_MASS,
        ArmKind::MfStageTarget(StageTargetPolicy::CoupledGuarded),
    ),
    wired(
        "mf-stage-target-dupfix",
        &["drv.mf_fast", "kr.gmres_into", "kr.stage_target_dupfix"],
        JVP_NO_MASS,
        ArmKind::MfStageTarget(StageTargetPolicy::DupFix),
    ),
    wired(
        "mf-stage-target-coupled-guarded2",
        &[
            "drv.mf_fast",
            "kr.gmres_staged",
            "kr.stage_target_coupled_guarded2",
        ],
        JVP_NO_MASS,
        ArmKind::MfStageTarget(StageTargetPolicy::CoupledGuarded2),
    ),
    wired(
        "mf-x0-prevstep",
        &[
            "drv.mf_fast",
            "kr.gmres_into",
            "kr.accounting_recompute_final",
            "kr.mf_warm_start",
        ],
        JVP_NO_MASS,
        ArmKind::MfX0(InitialGuess::PreviousStep),
    ),
    wired(
        "mf-lgmres",
        &["drv.mf_fast", "kr.lgmres"],
        JVP_NO_MASS,
        ArmKind::MfMethod(LinearMethod::Lgmres),
    ),
    wired(
        "mf-lgmres-into",
        &["drv.mf_fast", "kr.lgmres_into", "kr.workspace_options"],
        JVP_NO_MASS,
        mf_ws(
            LinearMethod::Lgmres,
            GcrodrRecyclePolicy::Legacy,
            false,
            false,
            KrylovWorkspaceOptions::lgmres_into(),
        ),
    ),
    wired(
        "mf-lgmres-into-ls-ws",
        &[
            "drv.mf_fast",
            "kr.lgmres_into",
            "kr.ls_workspace",
            "kr.workspace_options",
        ],
        JVP_NO_MASS,
        mf_ws(
            LinearMethod::Lgmres,
            GcrodrRecyclePolicy::Legacy,
            false,
            false,
            KrylovWorkspaceOptions::all(),
        ),
    ),
    wired(
        "mf-lgmres-start-explicit-zero",
        &["drv.mf_fast", "kr.lgmres", "kr.start_explicit_zero"],
        JVP_NO_MASS,
        mf_ws(
            LinearMethod::Lgmres,
            GcrodrRecyclePolicy::Legacy,
            true,
            false,
            KrylovWorkspaceOptions::none(),
        ),
    ),
    wired(
        "mf-gcrodr-recycle-legacy",
        &["drv.mf_fast", "kr.gcrodr", "kr.recycle_legacy"],
        JVP_NO_MASS,
        ArmKind::MfGcrodr(GcrodrRecyclePolicy::Legacy),
    ),
    wired(
        "mf-gcrodr-recycle-refresh",
        &[
            "drv.mf_fast",
            "kr.gcrodr",
            "kr.recycle_refresh_after_update",
        ],
        JVP_NO_MASS,
        ArmKind::MfGcrodr(GcrodrRecyclePolicy::RefreshAfterUpdate),
    ),
    wired(
        "mf-gcrodr-recycle-cold",
        &["drv.mf_fast", "kr.gcrodr", "kr.recycle_cold"],
        JVP_NO_MASS,
        ArmKind::MfGcrodr(GcrodrRecyclePolicy::Cold),
    ),
    wired(
        "mf-gcrodr-start-explicit-zero",
        &[
            "drv.mf_fast",
            "kr.gcrodr",
            "kr.recycle_refresh_after_update",
            "kr.start_explicit_zero",
        ],
        JVP_NO_MASS,
        mf_ws(
            LinearMethod::Gcrodr,
            GcrodrRecyclePolicy::RefreshAfterUpdate,
            true,
            false,
            KrylovWorkspaceOptions::none(),
        ),
    ),
    wired(
        "mf-gcrodr-recycle-scope-currentw",
        &[
            "drv.mf_fast",
            "kr.gcrodr",
            "kr.recycle_refresh_after_update",
            "kr.start_explicit_zero",
            "kr.recycle_scope_currentw",
        ],
        JVP_NO_MASS,
        mf_ws(
            LinearMethod::Gcrodr,
            GcrodrRecyclePolicy::RefreshAfterUpdate,
            true,
            true,
            KrylovWorkspaceOptions::none(),
        ),
    ),
    // --------------------------------------------- BDF and Radau IIA
    wired(
        "bdf2",
        &["drv.bdf", "nl.fixed"],
        ANY,
        ArmKind::Bench("repo-bdf2"),
    ),
    wired(
        "bdf2-tier-a",
        &["drv.bdf", "nl.scaled"],
        ANY,
        ArmKind::Bench("repo-bdf2-tier-a"),
    ),
    wired(
        "bdf1",
        &["drv.bdf", "nl.fixed"],
        ANY,
        ArmKind::Bdf(BdfOrder::One),
    ),
    wired(
        "radau5",
        &["drv.radau", "nl.fixed"],
        ANY,
        ArmKind::Bench("repo-radau5"),
    ),
    wired(
        "radau5-tier-a",
        &["drv.radau", "nl.scaled"],
        ANY,
        ArmKind::Bench("repo-radau5-tier-a"),
    ),
    wired(
        "radau1",
        &["drv.radau", "nl.fixed"],
        ANY,
        ArmKind::Radau(RadauIiaStages::One),
    ),
    // ------------------------------------------------------------- SABR
    wired(
        "sabr-gmres",
        &["drv.sabr", "blk.gmres"],
        ANALYTIC,
        ArmKind::Sabr(BlockMethod::Gmres),
    ),
    wired(
        "sabr-forward",
        &["drv.sabr", "blk.forward"],
        ANALYTIC,
        ArmKind::Sabr(BlockMethod::Forward),
    ),
    wired(
        "sabr-explicit",
        &["drv.sabr", "blk.explicit"],
        ANALYTIC,
        ArmKind::Sabr(BlockMethod::Explicit),
    ),
    wired(
        "sabr-nilpotent",
        &["drv.sabr", "blk.nilpotent"],
        ANALYTIC,
        ArmKind::Sabr(BlockMethod::Nilpotent),
    ),
    // ------------------------------------------------------------ others
    wired("homotopy", &["drv.homotopy"], ANALYTIC, ArmKind::Homotopy),
    wired(
        "transactional-q1q2",
        &["drv.transactional_q1q2", "cert.outward"],
        JVP,
        ArmKind::Transactional,
    ),
    wired(
        "pexprb54s4-fused",
        &["drv.exp_pexprb54s4", "phi.fused", "phi.pade13"],
        JVP_NO_MASS,
        ArmKind::Pexprb,
    ),
];

/// Every arm of the registry, wired or not, in listing order.
pub(crate) fn arms() -> &'static [ArmSpec] {
    ARMS
}

pub(crate) fn find_arm(id: &str) -> Option<&'static ArmSpec> {
    ARMS.iter().find(|arm| arm.id == id)
}

/// The problem features an applicability decision reads.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ProblemFeatures<'a> {
    pub analytic: bool,
    pub jvp: bool,
    pub structure: &'a str,
    pub mass: bool,
    pub dimension: usize,
}

/// `HARNESS_CONTRACT.md`: a problem is applicable to an arm iff every
/// `requires` field is satisfied.
pub(crate) fn applicable(requires: &Requires, problem: &ProblemFeatures<'_>) -> bool {
    let jacobian = match requires.jacobian {
        JacobianRequirement::Analytic => problem.analytic,
        JacobianRequirement::Jvp => problem.jvp,
        JacobianRequirement::Any => problem.analytic || problem.jvp,
    };
    let structure = requires
        .structure
        .is_none_or(|structure| structure == problem.structure);
    let mass = requires.mass || !problem.mass;
    let dimension = requires
        .fixed_dimension
        .is_none_or(|dimensions| dimensions.contains(&problem.dimension));
    jacobian && structure && mass && dimension
}

/// Arm-specific work outside `WorkCounters`, kept typed until the window
/// closes (serializing it would allocate inside the allocation window).
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum ExtraWork {
    Fast {
        jacobian_reuses: usize,
        lu: Rodas5pFastLu,
        driver: &'static str,
    },
    Small {
        jacobian_reuses: usize,
        driver: &'static str,
    },
    Banded(BandedWork),
    Routed(RoutingRecord, Option<BandedWork>),
    Dense(DenseErrorReport),
    MfFast(MfFastExtra),
    Transactional(TransactionalQ1Q2RunDiagnostics),
    Exponential(AdaptiveFusedExponentialDiagnostics),
}

#[derive(Clone, Debug)]
pub(crate) struct MfFastExtra {
    state_reuses: usize,
    driver: &'static str,
    gcrodr_policy: &'static str,
    krylov_workspace: &'static str,
    start_policy: &'static str,
    recycle_scope: &'static str,
    stage_statistics: Option<StageSolveStatistics>,
    fallback_charges: Option<usize>,
}

impl ExtraWork {
    pub(crate) fn json(&self) -> Value {
        match self {
            Self::Fast {
                jacobian_reuses,
                lu,
                driver,
            } => json!({
                "kind": "fast",
                "jacobian_reuses": jacobian_reuses,
                "lu": lu,
                "driver": driver,
            }),
            Self::Small {
                jacobian_reuses,
                driver,
            } => json!({
                "kind": "fast-small",
                "jacobian_reuses": jacobian_reuses,
                "driver": driver,
            }),
            Self::Banded(work) => json!({ "kind": "banded", "banded_work": work }),
            Self::Routed(routing, work) => json!({
                "kind": "routed",
                "routing": routing,
                "banded_work": work,
            }),
            Self::Dense(report) => json!({
                "kind": "dense-error-control",
                "control": report.control,
                "samples": report.samples.len(),
                "rejected_samples": report.samples.iter().filter(|s| !s.accepted).count(),
                "unavailable": report.unavailable,
                "max_accepted_estimate_wrms": report.max_accepted_estimate(),
                "estimate_counters": report.counters,
            }),
            Self::MfFast(extra) => json!({
                "kind": "mf-fast",
                "state_reuses": extra.state_reuses,
                "driver": extra.driver,
                "gcrodr_policy": extra.gcrodr_policy,
                "krylov_workspace": extra.krylov_workspace,
                "start_policy": extra.start_policy,
                "recycle_scope": extra.recycle_scope,
                "stage_statistics": extra.stage_statistics,
                "fallback_charges": extra.fallback_charges,
            }),
            Self::Transactional(diagnostics) => json!({
                "kind": "transactional-q1q2",
                "diagnostics": diagnostics,
            }),
            Self::Exponential(diagnostics) => json!({
                "kind": "pexprb54s4-fused",
                "phi_substeps_total": diagnostics.phi_substeps.iter().sum::<usize>(),
                "maximum_krylov_dimension": diagnostics.maximum_krylov_dimensions.iter().max(),
            }),
        }
    }
}

/// One integration segment of an arm.
#[derive(Clone, Debug)]
pub(crate) struct ArmOutcome {
    pub observed: ObservedIntegrationResult,
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub extra: Option<ExtraWork>,
}

impl ArmOutcome {
    fn adaptive(result: AdaptiveObservedIntegrationResult, extra: Option<ExtraWork>) -> Self {
        let d = result.diagnostics;
        Self {
            observed: result.observed,
            attempts: d.attempts,
            accepted_steps: d.accepted_macro_steps,
            rejected_steps: d.rejected_macro_steps,
            extra,
        }
    }

    fn fast(run: Rodas5pFastResult) -> Self {
        Self {
            observed: run.observed,
            attempts: run.attempts,
            accepted_steps: run.accepted_steps,
            rejected_steps: run.rejected_steps,
            extra: Some(ExtraWork::Fast {
                jacobian_reuses: run.jacobian_reuses,
                lu: run.lu,
                driver: run.driver,
            }),
        }
    }

    fn mf(
        run: Rodas5pMfFastResult,
        stage_statistics: Option<StageSolveStatistics>,
        fallback_charges: Option<usize>,
    ) -> Self {
        Self {
            observed: run.observed,
            attempts: run.attempts,
            accepted_steps: run.accepted_steps,
            rejected_steps: run.rejected_steps,
            extra: Some(ExtraWork::MfFast(MfFastExtra {
                state_reuses: run.state_reuses,
                driver: run.driver,
                gcrodr_policy: run.gcrodr_policy,
                krylov_workspace: run.krylov_workspace,
                start_policy: run.start_policy,
                recycle_scope: run.recycle_scope,
                stage_statistics,
                fallback_charges,
            })),
        }
    }
}

/// The output requests of one segment: the schedule of the clipping drivers
/// and the sampling plan of the dense-output drivers (no hard stops; the
/// segments are the breakpoints).
pub(crate) struct SegmentOutput {
    pub schedule: OutputSchedule,
    pub sampling: OutputSamplingPlan,
}

/// Everything an arm needs besides the integration call, built once per
/// cell.
pub(crate) struct PreparedArm {
    pub spec: &'static ArmSpec,
    pub adaptive: AdaptiveStepConfig,
    /// The declared problem of each segment (routed arm only).
    declared: Vec<OdeProblem>,
    band: Option<BandedJacobian>,
    linear: LinearSolverConfig,
    homotopy: Option<HomotopyStepConfig>,
    sabr: Option<SabrConfig>,
    execution: Option<ParallelExecution>,
}

/// The matrix-free configuration of the U-form arms: the router's
/// production stage target (`RoutingOptions::default().matrix_free`, the
/// ME01 arms' configuration).
fn mf_linear(method: LinearMethod, x0: InitialGuess) -> LinearSolverConfig {
    LinearSolverConfig {
        method,
        x0_strategy: x0,
        ..RoutingOptions::default().matrix_free
    }
}

/// `structure` of a triage problem as the router's declaration; dense for
/// every other problem with an explicit Jacobian, nothing otherwise.
fn declared_problem(
    segment: &OdeProblem,
    structure: &str,
    band: Option<&BandedJacobian>,
) -> CoreResult<OdeProblem> {
    let n = segment.dimension;
    let invalid = |e: rodas5p_integrators::StructureError| CoreError::InvalidInput(e.to_string());
    match (structure, band) {
        ("banded", Some(band)) => segment
            .clone()
            .with_declared_structure(StructureDeclaration::banded(
                n,
                band.lower,
                band.upper,
                band.fill.clone(),
            ))
            .map_err(invalid),
        _ if segment.has_explicit_jacobian() && segment.declared_band().is_none() => segment
            .clone()
            .with_declared_structure(StructureDeclaration::dense(n))
            .map_err(invalid),
        _ => Ok(segment.clone()),
    }
}

/// Build the per-cell state of `spec` for `segments` (the problem's
/// integration segments), with the cell's step configuration `adaptive`.
/// `structure` and `band` are the problem's declared structure and band.
pub(crate) fn prepare(
    spec: &'static ArmSpec,
    segments: &[BenchmarkProblem],
    structure: &str,
    band: Option<BandedJacobian>,
    mut adaptive: AdaptiveStepConfig,
) -> CoreResult<PreparedArm> {
    match spec.adaptive {
        AdaptiveOverride::None => {}
        AdaptiveOverride::Controller(controller) => adaptive.controller = controller,
        AdaptiveOverride::StrictMaxStep => {
            adaptive.max_step_policy = MaxStepPolicy::StrictRepresentedCap
        }
    }
    let direct = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    };
    let linear = match spec.kind {
        ArmKind::SeqKrylov(method, preconditioner) => LinearSolverConfig {
            method,
            preconditioner,
            ..LinearSolverConfig::default()
        },
        ArmKind::SeqMf(method, x0) => LinearSolverConfig {
            method,
            x0_strategy: x0,
            ..LinearSolverConfig::default()
        },
        ArmKind::MfStageTarget(_) => mf_linear(LinearMethod::Gmres, InitialGuess::Zero),
        ArmKind::MfGcrodr(_) => mf_linear(LinearMethod::Gcrodr, InitialGuess::Zero),
        ArmKind::MfMethod(method) => mf_linear(method, InitialGuess::Zero),
        ArmKind::MfX0(x0) => mf_linear(LinearMethod::Gmres, x0),
        ArmKind::MfWorkspace(ws) => mf_linear(ws.method, InitialGuess::Zero),
        _ => direct,
    };
    let declared = match spec.kind {
        ArmKind::Routed { declare: true } => segments
            .iter()
            .map(|segment| declared_problem(&segment.problem, structure, band.as_ref()))
            .collect::<CoreResult<Vec<_>>>()?,
        _ => Vec::new(),
    };
    // The homotopy candidate of the adaptive global-error screen
    // (`rodas5p_fair_ab::adaptive_global_error`, `AdaptiveCandidate::Homotopy`):
    // theta 1, q 7, two path rounds, AB2 predictor, one correction, output
    // budget 0.1, direct fallback.
    let homotopy = match spec.kind {
        ArmKind::Homotopy => Some(HomotopyStepConfig::new(
            HomotopyPathConfig::new(1.0, 7, 2, HomotopyPredictor::AdamsBashforth2, 1)?,
            0.1,
        )?),
        _ => None,
    };
    let sabr = match spec.kind {
        ArmKind::Sabr(block_method) => Some(SabrConfig {
            block_method,
            ..SabrConfig::default()
        }),
        _ => None,
    };
    let execution = matches!(spec.kind, ArmKind::Pexprb).then(ParallelExecution::sequential);
    if matches!(spec.kind, ArmKind::Banded(_)) && band.is_none() {
        return Err(CoreError::InvalidInput(format!(
            "arm {} needs a declared band",
            spec.id
        )));
    }
    Ok(PreparedArm {
        spec,
        adaptive,
        declared,
        band,
        linear,
        homotopy,
        sabr,
        execution,
    })
}

fn dense_error(error: DenseOutputError) -> CoreError {
    match error {
        DenseOutputError::Core(error) => error,
        other => CoreError::InvalidInput(other.to_string()),
    }
}

fn unwired(id: &str) -> CoreError {
    CoreError::InvalidInput(format!("arm {id} is not wired"))
}

/// One integration of segment `index` of a cell: `segment` carries the
/// segment's problem, span and initial state.
pub(crate) fn run_arm(
    prepared: &PreparedArm,
    index: usize,
    segment: &BenchmarkProblem,
    output: &SegmentOutput,
) -> CoreResult<ArmOutcome> {
    let spec = prepared.spec;
    let adaptive = &prepared.adaptive;
    let (p, span, y0) = (&segment.problem, segment.t_span, segment.y0.as_slice());
    let schedule = &output.schedule;
    match spec.kind {
        ArmKind::Bench(arm) => stiff_benchmark::run_arm_with(arm, segment, adaptive, schedule)
            .map(|r| ArmOutcome::adaptive(r, None)),
        ArmKind::Fast(None) => {
            integrate_rodas5p_fast_observed(p, span, y0, adaptive, schedule).map(ArmOutcome::fast)
        }
        ArmKind::Fast(Some(arm)) => {
            let (_, options) = stiff_benchmark::option_arm(arm).ok_or_else(|| unwired(spec.id))?;
            integrate_rodas5p_fast_observed_with_options(p, span, y0, adaptive, schedule, options)
                .map(ArmOutcome::fast)
        }
        ArmKind::Small(arm) => {
            let options = match arm {
                None => Default::default(),
                Some(arm) => {
                    stiff_benchmark::option_arm(arm)
                        .ok_or_else(|| unwired(spec.id))?
                        .1
                }
            };
            stiff_benchmark::run_small_full(segment, adaptive, schedule, options).map(|run| {
                let (jacobian_reuses, driver) = (run.jacobian_reuses, run.driver);
                ArmOutcome::adaptive(
                    stiff_benchmark::small_result(run),
                    Some(ExtraWork::Small {
                        jacobian_reuses,
                        driver,
                    }),
                )
            })
        }
        ArmKind::SmallStatic(arm) => {
            let options = stiff_benchmark::small_static_arm(arm).ok_or_else(|| unwired(spec.id))?;
            stiff_benchmark::run_small_static_full(segment, adaptive, schedule, options).map(
                |run| {
                    let (jacobian_reuses, driver) = (run.jacobian_reuses, run.driver);
                    ArmOutcome::adaptive(
                        stiff_benchmark::small_result(run),
                        Some(ExtraWork::Small {
                            jacobian_reuses,
                            driver,
                        }),
                    )
                },
            )
        }
        ArmKind::SeqKrylov(..) => integrate_adaptive_observed_with_config(
            p,
            span,
            y0,
            IntegrationMethod::Sequential,
            Some(&prepared.linear),
            None,
            adaptive,
            schedule,
        )
        .map(|r| ArmOutcome::adaptive(r, None)),
        ArmKind::SeqDense(control) => integrate_adaptive_dense_observed_with_dense_error_control(
            p,
            span,
            y0,
            IntegrationMethod::Sequential,
            Some(&prepared.linear),
            None,
            adaptive,
            &output.sampling,
            control,
        )
        .map(|(r, report)| ArmOutcome::adaptive(r, Some(ExtraWork::Dense(report))))
        .map_err(dense_error),
        ArmKind::SeqMf(..) => integrate_sequential_matrix_free_adaptive_dense_observed(
            p,
            span,
            y0,
            &prepared.linear,
            adaptive,
            &output.sampling,
        )
        .map(|r| ArmOutcome::adaptive(r, None))
        .map_err(dense_error),
        ArmKind::Banded(kernel) => {
            let band = prepared.band.as_ref().ok_or_else(|| unwired(spec.id))?;
            stiff_benchmark::run_banded_with(segment, band, adaptive, schedule, kernel)
                .map(|(r, work)| ArmOutcome::adaptive(r, Some(ExtraWork::Banded(work))))
        }
        ArmKind::Routed { declare } => {
            let problem = if declare {
                prepared
                    .declared
                    .get(index)
                    .ok_or_else(|| unwired(spec.id))?
            } else {
                p
            };
            stiff_benchmark::run_sp03_with(
                stiff_benchmark::ROUTED_ARM,
                problem,
                span,
                y0,
                adaptive,
                schedule,
            )
            .map(|(r, routing, work)| {
                let extra = routing.map(|routing| ExtraWork::Routed(routing, work));
                ArmOutcome::adaptive(r, extra)
            })
        }
        ArmKind::Me01(arm) => stiff_benchmark::run_sp03_with(arm, p, span, y0, adaptive, schedule)
            .map(|(r, _, _)| ArmOutcome::adaptive(r, None)),
        ArmKind::MfStageTarget(policy) => integrate_rodas5p_mf_fast_observed_with_stage_target(
            p,
            span,
            y0,
            &prepared.linear,
            adaptive,
            schedule,
            StageTargetOptions::new(policy),
        )
        .map(|run| ArmOutcome::mf(run.result, Some(run.statistics), Some(run.charges.len()))),
        ArmKind::MfGcrodr(policy) => integrate_rodas5p_mf_fast_observed_with_gcrodr_policy(
            p,
            span,
            y0,
            &prepared.linear,
            adaptive,
            schedule,
            policy,
        )
        .map(|run| ArmOutcome::mf(run, None, None)),
        ArmKind::MfMethod(_) => {
            integrate_rodas5p_mf_fast_observed(p, span, y0, &prepared.linear, adaptive, schedule)
                .map(|run| ArmOutcome::mf(run, None, None))
        }
        ArmKind::MfX0(_) => integrate_rodas5p_mf_fast_observed_gmres_into(
            p,
            span,
            y0,
            &prepared.linear,
            adaptive,
            schedule,
        )
        .map(|run| ArmOutcome::mf(run, None, None)),
        ArmKind::MfWorkspace(ws) => {
            // The SP02/SP04 switches on one workspace, as
            // `examples/sp04_zero_start_profile.rs` sets them.
            let mut work = Rodas5pMfFastWorkspace::new(p, &prepared.linear)?;
            if ws.method == LinearMethod::Gcrodr {
                work.set_gcrodr_policy(ws.gcrodr);
            }
            work.set_explicit_zero(ws.explicit_zero);
            work.set_current_w(ws.current_w);
            if ws.workspace != KrylovWorkspaceOptions::none() {
                work.set_krylov_workspace(ws.workspace)?;
            }
            work.integrate_observed(p, span, y0, adaptive, schedule)
                .map(|run| ArmOutcome::mf(run, None, None))
        }
        ArmKind::Bdf(order) => integrate_bdf_adaptive_observed(
            p,
            span,
            y0,
            &BdfConfig {
                order,
                ..BdfConfig::default()
            },
            adaptive,
            schedule,
        )
        .map(|r| ArmOutcome::adaptive(r, None)),
        ArmKind::Radau(stages) => integrate_radau_adaptive_observed(
            p,
            span,
            y0,
            &RadauConfig {
                stages,
                ..RadauConfig::default()
            },
            adaptive,
            schedule,
        )
        .map(|r| ArmOutcome::adaptive(r, None)),
        ArmKind::Sabr(_) => integrate_adaptive_observed_with_config(
            p,
            span,
            y0,
            IntegrationMethod::Sabr,
            None,
            prepared.sabr.clone(),
            adaptive,
            schedule,
        )
        .map(|r| ArmOutcome::adaptive(r, None)),
        ArmKind::Homotopy => {
            let homotopy = prepared.homotopy.as_ref().ok_or_else(|| unwired(spec.id))?;
            integrate_homotopy_adaptive_observed(
                p,
                span,
                y0,
                homotopy,
                Some(&prepared.linear),
                adaptive,
                schedule,
            )
            .map(|r| ArmOutcome::adaptive(r, None))
        }
        ArmKind::Transactional => integrate_transactional_q1_q2_adaptive_observed(
            p,
            span,
            y0,
            &TransactionalQ1Q2Config::default(),
            adaptive,
            schedule,
        )
        .map(|run| {
            ArmOutcome::adaptive(
                AdaptiveObservedIntegrationResult {
                    observed: run.observed,
                    diagnostics: run.diagnostics,
                },
                Some(ExtraWork::Transactional(run.transactional)),
            )
        }),
        ArmKind::Pexprb => {
            let execution = prepared
                .execution
                .as_ref()
                .ok_or_else(|| unwired(spec.id))?;
            integrate_pexprb54s4_fused_adaptive_observed(
                p,
                span,
                y0,
                adaptive,
                schedule,
                FusedPhiKrylovConfig::default(),
                execution,
            )
            .map(|run| ArmOutcome {
                observed: run.observed,
                attempts: run.diagnostics.attempts,
                accepted_steps: run.diagnostics.accepted_steps,
                rejected_steps: run.diagnostics.rejected_steps,
                extra: Some(ExtraWork::Exponential(run.diagnostics)),
            })
        }
        ArmKind::Unwired => Err(unwired(spec.id)),
    }
}
