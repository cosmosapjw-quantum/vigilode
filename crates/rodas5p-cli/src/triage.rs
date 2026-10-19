//! Component-triage harness (research node `research/component_triage_20261019`).
//!
//! The binding interface is `HARNESS_CONTRACT.md` (CT-HARNESS v1) of that
//! node: `triage-list`, `triage-run` and `triage-profile-run` print the
//! problem registry, one raw row (`vigilode-triage-row-v1`) and the
//! callgrind workload summary; `triage-exact` and `triage-parity` give the
//! Python tooling the exact solutions and right-hand-side samples it checks
//! its reference models against. The arm registry is
//! [`crate::triage_arms`]; the problem builders and the existing arms are
//! the ones of [`crate::stiff_benchmark`] and `rodas5p_integrators`.
//!
//! Nothing here judges a component: rows are raw (PA-03).

use std::fmt;

use anyhow::Result;
use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, OdeProblem, OutputSamplingPlan, OutputSchedule,
    ScientificCaseSpec, ScientificCorpusV2, ScientificFamily, complex_dahlquist_problem,
    constant_affine_mass_problem, gray_scott_2d, manufactured_mass_nonlinear_problem,
    manufactured_vector_problem, oscillatory_prothero_robinson_problem, prothero_robinson_problem,
    robertson_problem, scalar_linear_problem, semilinear_advection_diffusion_problem,
    stiff_van_der_pol_problem,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::stiff_benchmark::{self, BenchmarkProblem};
use crate::triage_arms::{self, ArmOutcome, ArmSpec, PreparedArm, ProblemFeatures, SegmentOutput};

pub(crate) const LIST_SCHEMA: &str = "vigilode-triage-list-v1";
pub(crate) const ROW_SCHEMA: &str = "vigilode-triage-row-v1";
pub(crate) const PROFILE_SCHEMA: &str = "vigilode-triage-profile-v1";
pub(crate) const EXACT_SCHEMA: &str = "vigilode-triage-exact-v1";
pub(crate) const PARITY_SCHEMA: &str = "vigilode-triage-parity-v1";
pub(crate) const CONTRACT: &str = "CT-HARNESS v1";
/// `HARNESS_CONTRACT.md`: callbacks counted when attempted (DP-02).
pub(crate) const COUNTER_SEMANTICS: &str = "work-counters-v2-attempted";
/// `h0_base = H0_BASE_FRACTION (tf - t0)` unless the problem declares one.
pub(crate) const H0_BASE_FRACTION: f64 = 1.0e-6;
pub(crate) const H0_SCALES: [f64; 3] = [0.5, 1.0, 2.0];
pub(crate) const MIN_STEP: f64 = 1.0e-14;
pub(crate) const MAX_ATTEMPTS: usize = 1_000_000;
pub(crate) const UNIFORM_POINTS: usize = 101;

pub(crate) const APPLICABILITY_RULE: &str = "a problem is applicable to an arm iff: \
requires.jacobian is 'analytic' and problem.jacobian == 'analytic', or 'jvp' and problem.jvp, \
or 'any' (analytic or jvp); requires.structure is null or equals problem.structure; \
requires.mass is true (the arm accepts a non-identity mass matrix) or problem.mass is false; \
requires.fixed_dimension is null or contains problem.dimension. `applicable_problems` lists \
the result; `triage-run` exits 2 for an inapplicable pair or an unwired arm.";

/// A command-line error with exit code 2 (`HARNESS_CONTRACT.md`): unknown
/// arm or problem, inapplicable pair, unwired arm, invalid grid request.
#[derive(Debug)]
pub(crate) struct UsageError(pub String);

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for UsageError {}

fn usage(message: String) -> anyhow::Error {
    UsageError(message).into()
}

/// Print a handler's JSON, or exit with code 2 on a [`UsageError`].
pub(crate) fn print_or_exit(result: Result<Value>) -> Result<()> {
    match result {
        Ok(value) => {
            println!("{value}");
            Ok(())
        }
        Err(error) if error.downcast_ref::<UsageError>().is_some() => {
            eprintln!("error: {error}");
            std::process::exit(2);
        }
        Err(error) => Err(error),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum Grid {
    #[value(name = "endpoint")]
    Endpoint,
    #[value(name = "uniform101")]
    Uniform101,
}

impl Grid {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Endpoint => "endpoint",
            Self::Uniform101 => "uniform101",
        }
    }

    /// The requested output times on `t_span`: both ends, or 101 equispaced
    /// times `t0 + (tf - t0) i / 100` (the corpus v2 base grid) with both
    /// ends exact.
    pub(crate) fn times(self, (t0, tf): (f64, f64)) -> Vec<f64> {
        match self {
            Self::Endpoint => vec![t0, tf],
            Self::Uniform101 => {
                let last = (UNIFORM_POINTS - 1) as f64;
                let mut times: Vec<f64> = (0..UNIFORM_POINTS)
                    .map(|i| t0 + (tf - t0) * i as f64 / last)
                    .collect();
                times[0] = t0;
                times[UNIFORM_POINTS - 1] = tf;
                times
            }
        }
    }
}

/// The cell flags shared by `triage-run`, `triage-profile-run` and the
/// allocation audit's `triage` mode.
#[derive(Clone, Debug, clap::Args)]
pub(crate) struct CellArgs {
    #[arg(long)]
    pub arm: String,
    #[arg(long)]
    pub problem: String,
    #[arg(long)]
    pub rtol: f64,
    #[arg(long, value_enum)]
    pub grid: Grid,
    /// One of 0.5, 1, 2: `h0 = h0_scale * h0_base`.
    #[arg(long, default_value_t = 1.0)]
    pub h0_scale: f64,
}

// ------------------------------------------------------------------ problems

/// One registered problem: its declared features and its builder.
pub(crate) struct ProblemSpec {
    pub id: &'static str,
    pub tier: &'static str,
    /// `dense` (explicit Jacobian, no other declaration), `banded` (a band
    /// callback for the banded arms), `sparse_linear_part` (a declared
    /// sparse linear part) or `unstructured` (JVP only).
    pub structure: &'static str,
    pub oscillatory: bool,
    pub source: &'static str,
    build: fn() -> CoreResult<Built>,
}

struct Built {
    problem: OdeProblem,
    y0: Vec<f64>,
    t_span: (f64, f64),
    atol_scale: f64,
    breakpoints: Vec<f64>,
    /// Branch-fixed segments of a segmented corpus problem (empty: one).
    segments: Vec<((f64, f64), OdeProblem)>,
    band: Option<BandedJacobian>,
}

impl Built {
    fn plain((problem, y0): (OdeProblem, Vec<f64>), t_span: (f64, f64), atol_scale: f64) -> Self {
        Self {
            problem,
            y0,
            t_span,
            atol_scale,
            breakpoints: Vec::new(),
            segments: Vec::new(),
            band: None,
        }
    }
}

fn corpus(spec: ScientificCaseSpec) -> CoreResult<Built> {
    let case = spec.build()?;
    let segments = if case.integration_segments.len() > 1 {
        case.integration_segments
            .iter()
            .map(|segment| (segment.t_span, segment.problem.clone()))
            .collect()
    } else {
        Vec::new()
    };
    Ok(Built {
        problem: case.problem,
        y0: case.y0,
        t_span: spec.t_span,
        atol_scale: spec.atol / spec.rtol,
        breakpoints: spec.mandatory_breakpoints.clone(),
        segments,
        band: None,
    })
}

fn holdout(family: ScientificFamily) -> CoreResult<Built> {
    let spec = ScientificCorpusV2::holdout_specs()
        .into_iter()
        .find(|spec| spec.family == family)
        .ok_or_else(|| CoreError::InvalidInput(format!("no holdout spec for {family:?}")))?;
    corpus(spec)
}

fn semilinear_2d(dimension: usize) -> CoreResult<Built> {
    let spec = ScientificCorpusV2::calibration_specs()
        .into_iter()
        .find(|spec| {
            spec.family == ScientificFamily::SemilinearAdvectionDiffusionRamped
                && spec.dimension == dimension
        })
        .ok_or_else(|| {
            CoreError::InvalidInput(format!("no semilinear calibration spec n = {dimension}"))
        })?;
    corpus(spec)
}

fn brusselator_1d(cells: usize) -> CoreResult<Built> {
    let mut built = Built::plain(
        stiff_benchmark::brusselator_problem(cells)?,
        (0.0, 10.0),
        1.0,
    );
    built.band = Some(stiff_benchmark::brusselator_band(cells));
    Ok(built)
}

fn brusselator_1d_jvp(cells: usize) -> CoreResult<Built> {
    let (_, y0) = stiff_benchmark::brusselator_problem(cells)?;
    Ok(Built::plain(
        (stiff_benchmark::brusselator_jvp_only_problem(cells)?, y0),
        (0.0, 10.0),
        1.0,
    ))
}

fn gray_scott(grid: usize) -> CoreResult<Built> {
    Ok(Built::plain(gray_scott_2d(grid)?, (0.0, 20.0), 1.0))
}

static PROBLEMS: &[ProblemSpec] = &[
    // ------------------------------------------------------------ tier G
    ProblemSpec {
        id: "robertson",
        tier: "G",
        structure: "dense",
        oscillatory: false,
        source: "rodas5p_integrators::problems::robertson_problem; t_span and atol_scale of rodas5p-cli stiff_benchmark::default_problems",
        build: || Ok(Built::plain(robertson_problem()?, (0.0, 40.0), 1.0e-4)),
    },
    ProblemSpec {
        id: "hires",
        tier: "G",
        structure: "dense",
        oscillatory: false,
        source: "rodas5p-cli stiff_benchmark::hires_problem; t_span and atol_scale of stiff_benchmark::default_problems",
        build: || {
            Ok(Built::plain(
                stiff_benchmark::hires_problem()?,
                (0.0, 321.8122),
                1.0e-4,
            ))
        },
    },
    ProblemSpec {
        id: "van-der-pol-mu1000",
        tier: "G",
        structure: "dense",
        oscillatory: true,
        source: "rodas5p_integrators::problems::stiff_van_der_pol_problem(1e3); t_span and atol_scale of stiff_benchmark::default_problems",
        build: || {
            Ok(Built::plain(
                stiff_van_der_pol_problem(1.0e3)?,
                (0.0, 2000.0),
                1.0,
            ))
        },
    },
    ProblemSpec {
        id: "oregonator",
        tier: "G",
        structure: "unstructured",
        oscillatory: true,
        source: "rodas5p_integrators::ScientificCorpusV2::holdout_specs() family oregonator (JVP only); atol_scale = spec.atol / spec.rtol",
        build: || holdout(ScientificFamily::Oregonator),
    },
    ProblemSpec {
        id: "pollution",
        tier: "G",
        structure: "unstructured",
        oscillatory: false,
        source: "rodas5p_integrators::ScientificCorpusV2::holdout_specs() family pollution (JVP only); atol_scale = spec.atol / spec.rtol",
        build: || holdout(ScientificFamily::Pollution),
    },
    ProblemSpec {
        id: "prothero-robinson-nl",
        tier: "G",
        structure: "dense",
        oscillatory: false,
        source: "rodas5p_integrators::problems::prothero_robinson_problem(-1e6, 10, 0): y' = -1e6 (y - sin t) + cos t + 10 (y - sin t)^2, exact sin t, analytic J, JVP, df/dt (the existing builder is this equation; no new builder needed); t_span [0, 1], atol_scale 1e-3 per the triage registration",
        build: || {
            Ok(Built::plain(
                prothero_robinson_problem(-1.0e6, 10.0, 0.0),
                (0.0, 1.0),
                1.0e-3,
            ))
        },
    },
    ProblemSpec {
        id: "scalar-linear",
        tier: "G",
        structure: "dense",
        oscillatory: false,
        source: "rodas5p_integrators::problems::scalar_linear_problem(-1e4, 1), exact; t_span [0, 1e-2] (100 time constants) and atol_scale 1e-3 chosen here: no existing adaptive test uses lambda = -1e4",
        build: || {
            Ok(Built::plain(
                scalar_linear_problem(-1.0e4, 1.0),
                (0.0, 1.0e-2),
                1.0e-3,
            ))
        },
    },
    ProblemSpec {
        id: "complex-dahlquist-32",
        tier: "G",
        structure: "dense",
        oscillatory: true,
        source: "rodas5p_integrators::problems::complex_dahlquist_problem(16 blocks = dimension 32, damping 120, frequency 180, t0 0) on t_span [0, 0.02] with atol = 0.01 rtol: the G1 adaptive global-error corpus (rodas5p_fair_ab::adaptive_global_error::g1_adaptive_corpus); exact",
        build: || {
            Ok(Built::plain(
                complex_dahlquist_problem(16, 120.0, 180.0, 0.0)?,
                (0.0, 0.02),
                0.01,
            ))
        },
    },
    ProblemSpec {
        id: "manufactured-vector-32",
        tier: "G",
        structure: "dense",
        oscillatory: false,
        source: "rodas5p_integrators::problems::manufactured_vector_problem(32, stiffness 1e3, nonlinearity 1e2, nonnormality 0.5, t0 0) on t_span [0, 0.02] with atol = 0.01 rtol: the G1 adaptive global-error corpus (rodas5p_fair_ab::adaptive_global_error::g1_adaptive_corpus); exact. Its linear part has eigenvalues up to about +362 (nonnormal Toeplitz), so errors grow like exp(362 t): longer spans fail for every arm",
        build: || {
            Ok(Built::plain(
                manufactured_vector_problem(32, 1.0e3, 1.0e2, 0.5, 0.0)?,
                (0.0, 0.02),
                0.01,
            ))
        },
    },
    ProblemSpec {
        id: "oscillatory-prothero-robinson",
        tier: "G",
        structure: "dense",
        oscillatory: true,
        source: "rodas5p_integrators::problems::oscillatory_prothero_robinson_problem(-1e4, 1e3, 140, 0) on t_span [0, 0.02] with atol = 0.01 rtol: the G1 adaptive global-error corpus (rodas5p_fair_ab::adaptive_global_error::g1_adaptive_corpus); exact",
        build: || {
            Ok(Built::plain(
                oscillatory_prothero_robinson_problem(-1.0e4, 1.0e3, 140.0, 0.0)?,
                (0.0, 0.02),
                0.01,
            ))
        },
    },
    ProblemSpec {
        id: "mass-affine",
        tier: "G",
        structure: "dense",
        oscillatory: false,
        source: "rodas5p_integrators::problems::constant_affine_mass_problem (constant noncommuting mass, quadratic forcing; no exact solution in Rust); t_span [0, 1] and atol_scale 1e-3 chosen here",
        build: || {
            let (problem, y0, _, _) = constant_affine_mass_problem();
            Ok(Built::plain((problem, y0), (0.0, 1.0), 1.0e-3))
        },
    },
    ProblemSpec {
        id: "mass-nonlinear",
        tier: "G",
        structure: "dense",
        oscillatory: false,
        source: "rodas5p_integrators::problems::manufactured_mass_nonlinear_problem(1e3, 1e2, 0.4, 0): the parameters of path_controller's mass case; exact; t_span [0, 1] and atol_scale 1e-3 chosen here",
        build: || {
            let (problem, y0, _, _) = manufactured_mass_nonlinear_problem(1.0e3, 1.0e2, 0.4, 0.0)?;
            Ok(Built::plain((problem, y0), (0.0, 1.0), 1.0e-3))
        },
    },
    ProblemSpec {
        id: "semilinear-advdiff-32",
        tier: "G",
        structure: "dense",
        oscillatory: false,
        source: "rodas5p_integrators::problems::semilinear_advection_diffusion_problem(32, 0.01, 5, -1, 10, 0) on t_span [0, 0.02] with atol = 0.01 rtol: the G1 adaptive global-error corpus (rodas5p_fair_ab::adaptive_global_error::g1_adaptive_corpus); exact",
        build: || {
            Ok(Built::plain(
                semilinear_advection_diffusion_problem(32, 0.01, 5.0, -1.0, 10.0, 0.0)?,
                (0.0, 0.02),
                0.01,
            ))
        },
    },
    // ------------------------------------------------------------ tier P
    ProblemSpec {
        id: "brusselator-1d-50",
        tier: "P",
        structure: "banded",
        oscillatory: true,
        source: "rodas5p-cli stiff_benchmark::brusselator_problem(50 cells, n = 100), analytic dense J, band stiff_benchmark::brusselator_band; t_span and atol_scale of stiff_benchmark",
        build: || brusselator_1d(50),
    },
    ProblemSpec {
        id: "brusselator-1d-250",
        tier: "P",
        structure: "banded",
        oscillatory: true,
        source: "rodas5p-cli stiff_benchmark::brusselator_problem(250 cells, n = 500), analytic dense J, band stiff_benchmark::brusselator_band; t_span and atol_scale of stiff_benchmark",
        build: || brusselator_1d(250),
    },
    ProblemSpec {
        id: "brusselator-1d-1000",
        tier: "P",
        structure: "banded",
        oscillatory: true,
        source: "rodas5p-cli stiff_benchmark::brusselator_problem(1000 cells, n = 2000), analytic dense J, band stiff_benchmark::brusselator_band; t_span and atol_scale of stiff_benchmark",
        build: || brusselator_1d(1000),
    },
    ProblemSpec {
        id: "brusselator-1d-jvp-250",
        tier: "P",
        structure: "unstructured",
        oscillatory: true,
        source: "rodas5p-cli stiff_benchmark::brusselator_jvp_only_problem(250 cells, n = 500), JVP only, y0 of brusselator_problem; t_span and atol_scale of stiff_benchmark",
        build: || brusselator_1d_jvp(250),
    },
    ProblemSpec {
        id: "brusselator-1d-jvp-1000",
        tier: "P",
        structure: "unstructured",
        oscillatory: true,
        source: "rodas5p-cli stiff_benchmark::brusselator_jvp_only_problem(1000 cells, n = 2000), JVP only, y0 of brusselator_problem; t_span and atol_scale of stiff_benchmark",
        build: || brusselator_1d_jvp(1000),
    },
    ProblemSpec {
        id: "brusselator-2d",
        tier: "P",
        structure: "unstructured",
        oscillatory: true,
        source: "rodas5p_integrators::ScientificCorpusV2::holdout_specs() family brusselator-2d (JVP only, segments [0, 1.1], [1.1, 11.5]); atol_scale = spec.atol / spec.rtol",
        build: || holdout(ScientificFamily::Brusselator2d),
    },
    ProblemSpec {
        id: "medical-akzo",
        tier: "P",
        structure: "unstructured",
        oscillatory: false,
        source: "rodas5p_integrators::ScientificCorpusV2::holdout_specs() family medical-akzo (JVP only, segments [0, 5], [5, 20]); atol_scale = spec.atol / spec.rtol",
        build: || holdout(ScientificFamily::MedicalAkzo),
    },
    ProblemSpec {
        id: "gray-scott-16",
        tier: "P",
        structure: "sparse_linear_part",
        oscillatory: false,
        source: "rodas5p_integrators::gray_scott_2d(16), n = 512, JVP and declared sparse linear part; t_span [0, 20] and atol = rtol of research/pc01_right_preconditioner_20261011 PREREGISTRATION F4",
        build: || gray_scott(16),
    },
    ProblemSpec {
        id: "gray-scott-32",
        tier: "P",
        structure: "sparse_linear_part",
        oscillatory: false,
        source: "rodas5p_integrators::gray_scott_2d(32), n = 2048, JVP and declared sparse linear part; t_span [0, 20] and atol = rtol of research/pc01_right_preconditioner_20261011 PREREGISTRATION F4",
        build: || gray_scott(32),
    },
    ProblemSpec {
        id: "gray-scott-64",
        tier: "P",
        structure: "sparse_linear_part",
        oscillatory: false,
        source: "rodas5p_integrators::gray_scott_2d(64), n = 8192, JVP and declared sparse linear part; t_span [0, 20] and atol = rtol of research/pc01_right_preconditioner_20261011 PREREGISTRATION F4",
        build: || gray_scott(64),
    },
    ProblemSpec {
        id: "semilinear-advdiff-2d-96",
        tier: "P",
        structure: "unstructured",
        oscillatory: false,
        source: "rodas5p_integrators::ScientificCorpusV2::calibration_specs() family semilinear-advection-diffusion-ramped n = 96 (JVP only, exact); atol_scale = spec.atol / spec.rtol",
        build: || semilinear_2d(96),
    },
    ProblemSpec {
        id: "semilinear-advdiff-2d-384",
        tier: "P",
        structure: "unstructured",
        oscillatory: false,
        source: "rodas5p_integrators::ScientificCorpusV2::calibration_specs() family semilinear-advection-diffusion-ramped n = 384 (JVP only, exact); atol_scale = spec.atol / spec.rtol",
        build: || semilinear_2d(384),
    },
    ProblemSpec {
        id: "semilinear-advdiff-2d-1536",
        tier: "P",
        structure: "unstructured",
        oscillatory: false,
        source: "rodas5p_integrators::ScientificCorpusV2::calibration_specs() family semilinear-advection-diffusion-ramped n = 1536 (JVP only, exact); atol_scale = spec.atol / spec.rtol",
        build: || semilinear_2d(1536),
    },
];

pub(crate) fn find_problem(id: &str) -> Option<&'static ProblemSpec> {
    PROBLEMS.iter().find(|problem| problem.id == id)
}

/// A built problem: the whole-domain problem (parity, exact solution,
/// features) and its integration segments, each a [`BenchmarkProblem`]
/// whose `y0` is the segment's initial-state buffer.
pub(crate) struct TriageProblem {
    pub spec: &'static ProblemSpec,
    pub problem: OdeProblem,
    pub y0: Vec<f64>,
    pub t_span: (f64, f64),
    pub atol_scale: f64,
    pub breakpoints: Vec<f64>,
    pub band: Option<BandedJacobian>,
    pub segments: Vec<BenchmarkProblem>,
    /// `None`: the default `h0_base = 1e-6 (tf - t0)`.
    pub h0_base: Option<f64>,
}

impl TriageProblem {
    pub(crate) fn build(spec: &'static ProblemSpec) -> CoreResult<Self> {
        let built = (spec.build)()?;
        let segment = |t_span: (f64, f64), problem: OdeProblem| BenchmarkProblem {
            id: spec.id,
            problem,
            y0: built.y0.clone(),
            t_span,
            atol_scale: built.atol_scale,
        };
        let segments = if built.segments.is_empty() {
            vec![segment(built.t_span, built.problem.clone())]
        } else {
            built
                .segments
                .iter()
                .map(|(t_span, problem)| segment(*t_span, problem.clone()))
                .collect()
        };
        Ok(Self {
            spec,
            problem: built.problem,
            y0: built.y0,
            t_span: built.t_span,
            atol_scale: built.atol_scale,
            breakpoints: built.breakpoints,
            band: built.band,
            segments,
            h0_base: None,
        })
    }

    pub(crate) fn analytic(&self) -> bool {
        self.problem.has_explicit_jacobian()
    }

    pub(crate) fn exact_solution(&self) -> bool {
        self.problem.exact(self.t_span.0).is_some()
    }

    pub(crate) fn features(&self) -> ProblemFeatures<'_> {
        ProblemFeatures {
            analytic: self.analytic(),
            jvp: self.problem.has_jvp(),
            structure: self.spec.structure,
            mass: self.problem.mass_matrix.is_some(),
            dimension: self.problem.dimension,
        }
    }

    pub(crate) fn h0_base(&self) -> f64 {
        self.h0_base
            .unwrap_or(H0_BASE_FRACTION * (self.t_span.1 - self.t_span.0))
    }

    pub(crate) fn json(&self) -> Value {
        let spec = self.spec;
        json!({
            "id": spec.id,
            "tier": spec.tier,
            "dimension": self.problem.dimension,
            "structure": spec.structure,
            "jacobian": if self.analytic() { "analytic" } else { "jvp" },
            "jvp": self.problem.has_jvp(),
            "mass": self.problem.mass_matrix.is_some(),
            "exact_solution": self.exact_solution(),
            "oscillatory": spec.oscillatory,
            "t_span": [self.t_span.0, self.t_span.1],
            "atol_scale": self.atol_scale,
            "breakpoints": self.breakpoints,
            "h0_base": self.h0_base(),
            "source": spec.source,
        })
    }
}

fn build_problem(id: &str) -> Result<TriageProblem> {
    let spec = find_problem(id).ok_or_else(|| usage(format!("unknown problem {id}")))?;
    Ok(TriageProblem::build(spec)?)
}

// --------------------------------------------------------------------- list

pub(crate) fn list_json() -> Result<Value> {
    let built = PROBLEMS
        .iter()
        .map(TriageProblem::build)
        .collect::<CoreResult<Vec<_>>>()?;
    let arms = triage_arms::arms()
        .iter()
        .map(|arm| {
            let applicable: Vec<&str> = built
                .iter()
                .filter(|p| triage_arms::applicable(&arm.requires, &p.features()))
                .map(|p| p.spec.id)
                .collect();
            json!({
                "id": arm.id,
                "components": arm.components,
                "wired": arm.wired,
                "unwired_reason": arm.unwired_reason,
                "requires": arm.requires.json(),
                "applicable_problems": applicable,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "schema": LIST_SCHEMA,
        "contract": CONTRACT,
        "applicability_rule": APPLICABILITY_RULE,
        "counter_semantics": COUNTER_SEMANTICS,
        "cell_defaults": {
            "h0_base": "1e-6 * (tf - t0) unless the problem declares h0_base",
            "h0_scales": H0_SCALES,
            "min_step": MIN_STEP,
            "max_step": "tf - t0",
            "max_attempts_per_segment": MAX_ATTEMPTS,
            "grids": ["endpoint", "uniform101"],
        },
        "arms": arms,
        "problems": built.iter().map(TriageProblem::json).collect::<Vec<_>>(),
    }))
}

// -------------------------------------------------------------------- cells

/// Row status (`HARNESS_CONTRACT.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    Ok,
    SolverFailure,
    Refused,
    Error,
}

impl Status {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::SolverFailure => "solver_failure",
            Self::Refused => "refused",
            Self::Error => "error",
        }
    }
}

fn core_error_kind(error: &CoreError) -> &'static str {
    match error {
        CoreError::Dimension(_) => "dimension",
        CoreError::InvalidInput(_) => "invalid-input",
        CoreError::NonFinite(_) => "non-finite",
        CoreError::LinearSolve(_) => "linear-solve",
        CoreError::NonlinearSolve(_) => "nonlinear-solve",
        CoreError::Coefficients(_) => "coefficients",
    }
}

/// Whether a typed error of the first segment is an entry-point refusal: a
/// dimension or invalid-input error other than the clock's time-resolution
/// errors, which are raised during the run. The integrators do not report
/// whether work had started, so this is classified by error kind.
fn is_refusal(error: &CoreError) -> bool {
    match error {
        CoreError::Dimension(_) => true,
        CoreError::InvalidInput(message) => !message.starts_with("time resolution"),
        _ => false,
    }
}

/// The typed result of one segment run (or of a failed preparation).
pub(crate) type SegmentRun = CoreResult<ArmOutcome>;

/// One cell: an arm on a problem at one tolerance, grid and initial step,
/// with everything built that is not the integration itself.
pub(crate) struct Cell {
    pub arm: &'static ArmSpec,
    pub problem: TriageProblem,
    pub rtol: f64,
    pub atol: f64,
    pub grid: Grid,
    pub h0_scale: f64,
    pub h0: f64,
    pub output_times: Vec<f64>,
    prepared: std::result::Result<PreparedArm, (String, &'static str)>,
    outputs: Vec<SegmentOutput>,
}

impl Cell {
    /// Build a cell. Unknown arms or problems, unwired arms, inapplicable
    /// pairs and an `h0_scale` outside {0.5, 1, 2} are [`UsageError`]s.
    pub(crate) fn new(args: &CellArgs) -> Result<Self> {
        let arm = triage_arms::find_arm(&args.arm)
            .ok_or_else(|| usage(format!("unknown arm {}", args.arm)))?;
        let problem = build_problem(&args.problem)?;
        if !arm.wired {
            return Err(usage(format!(
                "arm {} is not wired: {}",
                arm.id,
                arm.unwired_reason.unwrap_or("")
            )));
        }
        if !triage_arms::applicable(&arm.requires, &problem.features()) {
            return Err(usage(format!(
                "arm {} is not applicable to problem {}",
                arm.id, problem.spec.id
            )));
        }
        if !H0_SCALES.contains(&args.h0_scale) {
            return Err(usage(format!(
                "h0-scale must be one of {H0_SCALES:?}, got {}",
                args.h0_scale
            )));
        }
        let (t0, tf) = problem.t_span;
        let h0 = args.h0_scale * problem.h0_base();
        let adaptive = AdaptiveStepConfig {
            atol: args.rtol * problem.atol_scale,
            rtol: args.rtol,
            initial_step: h0,
            min_step: MIN_STEP,
            max_step: tf - t0,
            max_attempts: MAX_ATTEMPTS,
            ..AdaptiveStepConfig::default()
        };
        let output_times = args.grid.times(problem.t_span);
        let outputs = problem
            .segments
            .iter()
            .map(|segment| {
                let (a, b) = segment.t_span;
                let mut times = vec![a];
                times.extend(output_times.iter().copied().filter(|t| *t > a && *t < b));
                times.push(b);
                let schedule = OutputSchedule::new(times)?;
                Ok(SegmentOutput {
                    sampling: OutputSamplingPlan::dense(schedule.clone()),
                    schedule,
                })
            })
            .collect::<CoreResult<Vec<_>>>()?;
        let prepared = triage_arms::prepare(
            arm,
            &problem.segments,
            problem.spec.structure,
            problem.band.clone(),
            adaptive.clone(),
        )
        .map_err(|e| (e.to_string(), core_error_kind(&e)));
        Ok(Self {
            arm,
            atol: adaptive.atol,
            problem,
            rtol: args.rtol,
            grid: args.grid,
            h0_scale: args.h0_scale,
            h0,
            output_times,
            prepared,
            outputs,
        })
    }

    pub(crate) fn segment_count(&self) -> usize {
        self.problem.segments.len()
    }

    /// One integration of the cell: every segment in order, each starting
    /// from the end state of the previous one, until one fails. `runs` must
    /// be empty and should have capacity [`Self::segment_count`]; nothing
    /// else is allocated here besides what the arm allocates.
    pub(crate) fn integrate_into(&mut self, runs: &mut Vec<SegmentRun>) {
        let prepared = match &self.prepared {
            Ok(prepared) => prepared,
            Err((message, _)) => {
                runs.push(Err(CoreError::InvalidInput(message.clone())));
                return;
            }
        };
        for (index, segment) in self.problem.segments.iter_mut().enumerate() {
            if index == 0 {
                segment.y0.copy_from_slice(&self.problem.y0);
            } else {
                match runs.last() {
                    Some(Ok(previous))
                        if previous.observed.success
                            && previous.observed.t.last() == Some(&segment.t_span.0) =>
                    {
                        let end = previous.observed.y.last().expect("a successful run");
                        segment.y0.copy_from_slice(end);
                    }
                    _ => return,
                }
            }
            runs.push(triage_arms::run_arm(
                prepared,
                index,
                segment,
                &self.outputs[index],
            ));
        }
    }

    pub(crate) fn integrate(&mut self) -> CellRun {
        let mut runs = Vec::with_capacity(self.segment_count());
        self.integrate_into(&mut runs);
        self.merge(runs)
    }

    /// The row data of one integration: the outputs on the requested grid
    /// (segment boundaries that are not requested are dropped), the summed
    /// work, and the status.
    pub(crate) fn merge(&self, runs: Vec<SegmentRun>) -> CellRun {
        let requested = &self.output_times;
        let ran = runs.len();
        let mut out = CellRun {
            status: Status::Ok,
            error: None,
            error_kind: None,
            success: false,
            attempts: 0,
            accepted_steps: 0,
            rejected_steps: 0,
            output_times: Vec::with_capacity(requested.len()),
            states: Vec::with_capacity(requested.len()),
            counters: WorkCounters::default(),
            extra: Vec::with_capacity(ran),
            segments: self.segment_count(),
            segments_completed: 0,
        };
        if let Err((_, kind)) = &self.prepared {
            out.error_kind = Some(*kind);
        }
        let mut failed = false;
        for (index, run) in runs.into_iter().enumerate() {
            match run {
                Err(error) => {
                    out.status = if index == 0 && is_refusal(&error) {
                        Status::Refused
                    } else {
                        Status::Error
                    };
                    if out.error_kind.is_none() {
                        out.error_kind = Some(core_error_kind(&error));
                    }
                    out.error = Some(error.to_string());
                    failed = true;
                    break;
                }
                Ok(run) => {
                    out.counters.accumulate(run.observed.counters);
                    out.attempts += run.attempts;
                    out.accepted_steps += run.accepted_steps;
                    out.rejected_steps += run.rejected_steps;
                    for (t, y) in run.observed.t.iter().zip(run.observed.y) {
                        let is_requested = requested.binary_search_by(|x| x.total_cmp(t)).is_ok();
                        if is_requested && out.output_times.last() != Some(t) {
                            out.output_times.push(*t);
                            out.states.push(y);
                        }
                    }
                    out.extra.push(run.extra);
                    if !run.observed.success {
                        out.status = Status::SolverFailure;
                        out.error = Some(run.observed.message);
                        failed = true;
                        break;
                    }
                    out.segments_completed += 1;
                }
            }
        }
        if !failed {
            let complete = out.segments_completed == out.segments
                && out.output_times.len() == requested.len()
                && out
                    .output_times
                    .iter()
                    .zip(requested)
                    .all(|(a, b)| a.to_bits() == b.to_bits());
            if complete {
                out.success = true;
            } else {
                out.status = Status::SolverFailure;
                out.error = Some(format!(
                    "{} of {} requested outputs observed in {} of {} segments",
                    out.output_times.len(),
                    requested.len(),
                    out.segments_completed,
                    out.segments
                ));
            }
        }
        out
    }

    pub(crate) fn status_json(&self) -> Value {
        json!({
            "arm": self.arm.id,
            "problem": self.problem.spec.id,
            "rtol": self.rtol,
            "grid": self.grid.id(),
            "h0_scale": self.h0_scale,
        })
    }
}

/// The merged result of one integration of a cell.
#[derive(Debug)]
pub(crate) struct CellRun {
    pub status: Status,
    pub error: Option<String>,
    pub error_kind: Option<&'static str>,
    pub success: bool,
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub output_times: Vec<f64>,
    pub states: Vec<Vec<f64>>,
    pub counters: WorkCounters,
    pub extra: Vec<Option<triage_arms::ExtraWork>>,
    pub segments: usize,
    pub segments_completed: usize,
}

impl CellRun {
    /// Bitwise equality of the observed output arrays (times and states)
    /// and of the status.
    pub(crate) fn same_output(&self, other: &Self) -> bool {
        let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        self.status == other.status
            && self.success == other.success
            && bits(&self.output_times) == bits(&other.output_times)
            && self.states.len() == other.states.len()
            && self
                .states
                .iter()
                .zip(&other.states)
                .all(|(a, b)| bits(a) == bits(b))
    }

    pub(crate) fn extra_json(&self) -> Value {
        if self.extra.iter().all(Option::is_none) {
            return Value::Null;
        }
        let one = |extra: &Option<triage_arms::ExtraWork>| {
            extra
                .as_ref()
                .map_or(Value::Null, triage_arms::ExtraWork::json)
        };
        if self.segments == 1 {
            one(&self.extra[0])
        } else {
            json!({ "segments": self.extra.iter().map(one).collect::<Vec<_>>() })
        }
    }
}

pub(crate) fn hex(value: f64) -> String {
    stiff_benchmark::f64_bits_hex(value)
}

pub(crate) fn hex_vec(values: &[f64]) -> Vec<String> {
    values.iter().map(|v| hex(*v)).collect()
}

/// The field names of `WorkCounters`, read from its `Deserialize` impl, so
/// the row lists every field even when serde skips a zero one.
pub(crate) fn work_counter_fields() -> &'static [&'static str] {
    use serde::de::{self, Visitor};

    struct Probe<'a>(&'a mut Option<&'static [&'static str]>);

    impl<'de> de::Deserializer<'de> for Probe<'_> {
        type Error = de::value::Error;

        fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
            Err(de::Error::custom("field-name probe"))
        }

        fn deserialize_struct<V: Visitor<'de>>(
            self,
            _: &'static str,
            fields: &'static [&'static str],
            _: V,
        ) -> Result<V::Value, Self::Error> {
            *self.0 = Some(fields);
            Err(de::Error::custom("field-name probe"))
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map enum identifier ignored_any
        }
    }

    let mut fields = None;
    let _ = WorkCounters::deserialize(Probe(&mut fields));
    fields.unwrap_or(&[])
}

/// Every field of `WorkCounters`, raw: serde's serialization with the
/// fields it skips while zero filled in as 0.
pub(crate) fn counters_json(counters: &WorkCounters) -> Value {
    let mut value = serde_json::to_value(counters).expect("WorkCounters serializes");
    if let Value::Object(map) = &mut value {
        for field in work_counter_fields() {
            map.entry(*field).or_insert(json!(0));
        }
    }
    value
}

fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// `binary`: the sha256 of the running executable (or
/// `VIGILODE_BINARY_SHA256` when the driver has hashed it already) and the
/// build profile (`VIGILODE_BUILD_PROFILE`, otherwise inferred: cargo builds
/// the `bench` and `release` profiles into the same `target/release`, so the
/// binary cannot tell them apart).
pub(crate) fn binary_json() -> Value {
    let (sha256, sha256_source) = match env_nonempty("VIGILODE_BINARY_SHA256") {
        Some(sha) => (Some(sha), "env"),
        None => (
            std::env::current_exe()
                .ok()
                .and_then(|exe| std::fs::read(exe).ok())
                .map(|bytes| rodas5p_core::sha256_hex(&bytes)),
            "computed",
        ),
    };
    let (profile, profile_source) = match env_nonempty("VIGILODE_BUILD_PROFILE") {
        Some(profile) => (profile, "env"),
        None if cfg!(debug_assertions) => ("dev".to_owned(), "inferred"),
        None => {
            let dir = std::env::current_exe().ok().and_then(|exe| {
                exe.parent()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
            });
            let profile = match dir.as_deref() {
                Some("release") => "release-or-bench".to_owned(),
                Some(other) => other.to_owned(),
                None => "unknown".to_owned(),
            };
            (profile, "inferred")
        }
    };
    json!({
        "sha256": sha256,
        "sha256_source": sha256_source,
        "profile": profile,
        "profile_source": profile_source,
    })
}

/// `source`: from `VIGILODE_SOURCE_COMMIT` and `VIGILODE_TREE_CLEAN` (the
/// binary does not shell out to git); `null` when unset.
pub(crate) fn source_json() -> Value {
    let tree_clean =
        env_nonempty("VIGILODE_TREE_CLEAN").and_then(|v| match v.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" => Some(true),
            "0" | "false" | "no" => Some(false),
            _ => None,
        });
    json!({
        "commit": env_nonempty("VIGILODE_SOURCE_COMMIT"),
        "tree_clean": tree_clean,
    })
}

/// The `vigilode-triage-row-v1` row of the first run of a cell; `binary`
/// and `source` are [`binary_json`] and [`source_json`].
pub(crate) fn row_json(
    cell: &Cell,
    run: &CellRun,
    deterministic: bool,
    repetitions: usize,
    campaign: Option<&str>,
    binary: Value,
    source: Value,
) -> Value {
    let problem = &cell.problem;
    json!({
        "schema": ROW_SCHEMA,
        "campaign": campaign,
        "arm": cell.arm.id,
        "components": cell.arm.components,
        "problem": problem.spec.id,
        "tier": problem.spec.tier,
        "dimension": problem.problem.dimension,
        "rtol": cell.rtol,
        "atol": cell.atol,
        "grid": cell.grid.id(),
        "h0_scale": cell.h0_scale,
        "h0": cell.h0,
        "status": run.status.id(),
        "error": run.error,
        "error_kind": run.error_kind,
        "success": run.success,
        "attempts": run.attempts,
        "accepted_steps": run.accepted_steps,
        "rejected_steps": run.rejected_steps,
        "segments": run.segments,
        "segments_completed": run.segments_completed,
        "output_times": run.output_times,
        "states_hex": run.states.iter().map(|y| hex_vec(y)).collect::<Vec<_>>(),
        "counters": counters_json(&run.counters),
        "extra_work": run.extra_json(),
        "counter_semantics": COUNTER_SEMANTICS,
        "deterministic": deterministic,
        "repetitions": repetitions,
        "binary": binary,
        "source": source,
    })
}

/// `triage-run`: the cell `repetitions` times in-process; the row of the
/// first run, `deterministic` comparing every run's outputs bitwise.
pub(crate) fn run_row(
    args: &CellArgs,
    repetitions: usize,
    campaign: Option<&str>,
) -> Result<Value> {
    if repetitions == 0 {
        return Err(usage("triage-run needs at least one repetition".into()));
    }
    let mut cell = Cell::new(args)?;
    let first = cell.integrate();
    let mut deterministic = true;
    for _ in 1..repetitions {
        let again = cell.integrate();
        deterministic &= first.same_output(&again);
    }
    Ok(row_json(
        &cell,
        &first,
        deterministic,
        repetitions,
        campaign,
        binary_json(),
        source_json(),
    ))
}

/// `triage-profile-run`: the callgrind workload. Builds the cell, runs it
/// `repetitions` (possibly 0) times and prints a compact summary; nothing
/// else (no hashing, no environment reads).
pub(crate) fn profile_run(args: &CellArgs, repetitions: usize) -> Result<Value> {
    let mut cell = Cell::new(args)?;
    let mut first: Option<CellRun> = None;
    let mut deterministic = true;
    for _ in 0..repetitions {
        let run = cell.integrate();
        match &first {
            None => first = Some(run),
            Some(first) => deterministic &= first.same_output(&run),
        }
    }
    let mut out = cell.status_json();
    out["schema"] = json!(PROFILE_SCHEMA);
    out["repetitions"] = json!(repetitions);
    out["deterministic"] = json!(deterministic);
    match &first {
        Some(run) => {
            out["status"] = json!(run.status.id());
            out["success"] = json!(run.success);
            out["attempts"] = json!(run.attempts);
            out["counters"] = counters_json(&run.counters);
        }
        None => {
            out["status"] = Value::Null;
            out["success"] = Value::Null;
            out["attempts"] = Value::Null;
            out["counters"] = Value::Null;
        }
    }
    Ok(out)
}

// ------------------------------------------------------- exact and parity

/// `triage-exact`: the exact solution at the grid, as IEEE hex.
pub(crate) fn exact_json(problem_id: &str, grid: Grid) -> Result<Value> {
    let problem = build_problem(problem_id)?;
    if !problem.exact_solution() {
        return Err(usage(format!("problem {problem_id} has no exact solution")));
    }
    let times = grid.times(problem.t_span);
    let states = times
        .iter()
        .map(|t| {
            problem
                .problem
                .exact(*t)
                .map(|y| hex_vec(&y))
                .ok_or_else(|| anyhow::anyhow!("exact solution missing at t = {t}"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({
        "schema": EXACT_SCHEMA,
        "problem": problem_id,
        "grid": grid.id(),
        "dimension": problem.problem.dimension,
        "output_times": times,
        "output_times_hex": hex_vec(&times),
        "states_hex": states,
    }))
}

fn matrix_hex(rows: usize, cols: usize, entry: impl Fn(usize, usize) -> f64) -> Vec<Vec<String>> {
    (0..rows)
        .map(|i| (0..cols).map(|j| hex(entry(i, j))).collect())
        .collect()
}

/// `triage-parity`: `f(t, y)`, the analytic Jacobian (dense rows up to
/// `jacobian_max_dense`, nonzero triplets `[i, j, hex]` above) and a JVP
/// `J v` with `v_i = sin(i + 1)` at the deterministic states of
/// `stiff_benchmark::parity_states` (`y0` and a perturbation) at `t0` and
/// the span midpoint, all as IEEE hex. The whole-domain problem is used
/// for segmented corpus problems.
pub(crate) fn parity_json(problem_id: &str, jacobian_max_dense: usize) -> Result<Value> {
    let problem = build_problem(problem_id)?;
    let p = &problem.problem;
    let n = p.dimension;
    let (t0, tf) = problem.t_span;
    let times = [t0, t0 + 0.5 * (tf - t0)];
    let v: Vec<f64> = (0..n).map(|i| ((i + 1) as f64).sin()).collect();
    let mut samples = Vec::new();
    for t in times {
        for y in stiff_benchmark::parity_states(&problem.y0) {
            let mut counters = WorkCounters::default();
            let f = p.eval_rhs(t, &y, &mut counters).map(|f| hex_vec(&f));
            let jacobian = if p.has_explicit_jacobian() {
                let j = p.dense_jacobian(t, &y, &mut counters)?;
                if n <= jacobian_max_dense {
                    json!({"format": "dense", "rows_hex": matrix_hex(n, n, |i, k| j[(i, k)])})
                } else {
                    let mut triplets = Vec::new();
                    for i in 0..n {
                        for k in 0..n {
                            let x = j[(i, k)];
                            if x.to_bits() != 0 {
                                triplets.push(json!([i, k, hex(x)]));
                            }
                        }
                    }
                    json!({"format": "coo", "nonzeros_hex": triplets})
                }
            } else {
                Value::Null
            };
            let jvp = match p.jvp_callback() {
                Some(jvp) => {
                    let mut out = vec![0.0; n];
                    jvp(t, &y, &v, &mut out).map(|()| hex_vec(&out)).ok()
                }
                None => None,
            };
            samples.push(json!({
                "t_hex": hex(t),
                "y_hex": hex_vec(&y),
                "f_hex": f.as_ref().ok(),
                "f_error": f.as_ref().err().map(ToString::to_string),
                "jacobian": jacobian,
                "jvp_hex": jvp,
            }));
        }
    }
    let mass = p
        .mass_matrix
        .as_ref()
        .map(|m| matrix_hex(n, n, |i, k| m[(i, k)]));
    Ok(json!({
        "schema": PARITY_SCHEMA,
        "problem": problem_id,
        "dimension": n,
        "autonomous": p.autonomous,
        "jacobian": if p.has_explicit_jacobian() { "analytic" } else { "jvp" },
        "jvp_direction_hex": hex_vec(&v),
        "mass_hex": mass,
        "samples": samples,
    }))
}
