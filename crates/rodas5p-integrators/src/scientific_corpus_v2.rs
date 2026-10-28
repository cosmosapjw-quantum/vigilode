//! Versioned scientific-validity corpus.
//!
//! This module is deliberately separate from the historical G4/S5B0 atlas.  Existing atlas
//! profiles and receipts continue to build their legacy problems; callers must opt in to this
//! v2 corpus explicitly.  Calibration contains only the six historical family names, now with
//! prefix-stable parameter diversification.  The four source-anchored families are holdout-only.

use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

use crate::OdeProblem;

const CALIBRATION_DIMENSIONS: [usize; 3] = [96, 384, 1536];
const CORPUS_RTOLS: [f64; 3] = [1.0e-4, 1.0e-6, 1.0e-8];
const UNIFORM_OUTPUT_POINTS: usize = 101;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CorpusPartition {
    Calibration,
    Holdout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScientificFamily {
    RobertsonRamped,
    HiresRamped,
    VanDerPolRamped,
    RotatingNonnormal,
    NonautonomousStiffForcing,
    SemilinearAdvectionDiffusionRamped,
    Oregonator,
    Pollution,
    MedicalAkzo,
    Brusselator2d,
}

impl ScientificFamily {
    pub const CALIBRATION: [Self; 6] = [
        Self::RobertsonRamped,
        Self::HiresRamped,
        Self::VanDerPolRamped,
        Self::RotatingNonnormal,
        Self::NonautonomousStiffForcing,
        Self::SemilinearAdvectionDiffusionRamped,
    ];

    pub const HOLDOUT: [Self; 4] = [
        Self::Oregonator,
        Self::Pollution,
        Self::MedicalAkzo,
        Self::Brusselator2d,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RobertsonRamped => "robertson-ramped",
            Self::HiresRamped => "hires-ramped",
            Self::VanDerPolRamped => "van-der-pol-ramped",
            Self::RotatingNonnormal => "rotating-nonnormal",
            Self::NonautonomousStiffForcing => "nonautonomous-stiff-forcing",
            Self::SemilinearAdvectionDiffusionRamped => "semilinear-advection-diffusion-ramped",
            Self::Oregonator => "oregonator",
            Self::Pollution => "pollution",
            Self::MedicalAkzo => "medical-akzo",
            Self::Brusselator2d => "brusselator-2d",
        }
    }

    pub const fn partition(self) -> CorpusPartition {
        match self {
            Self::RobertsonRamped
            | Self::HiresRamped
            | Self::VanDerPolRamped
            | Self::RotatingNonnormal
            | Self::NonautonomousStiffForcing
            | Self::SemilinearAdvectionDiffusionRamped => CorpusPartition::Calibration,
            Self::Oregonator | Self::Pollution | Self::MedicalAkzo | Self::Brusselator2d => {
                CorpusPartition::Holdout
            }
        }
    }

    pub const fn block_width(self) -> Option<usize> {
        match self {
            Self::RobertsonRamped => Some(3),
            Self::HiresRamped => Some(8),
            Self::VanDerPolRamped | Self::RotatingNonnormal => Some(2),
            Self::NonautonomousStiffForcing => Some(1),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScientificSourceProvenance {
    pub source_repository: String,
    pub source_revision: String,
    pub source_path: String,
    pub source_blob: Option<String>,
    pub source_sha256: Option<String>,
    pub license_or_terms: String,
    pub interpretation_note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScientificCaseSpec {
    pub id: String,
    pub family: ScientificFamily,
    pub partition: CorpusPartition,
    pub dimension: usize,
    /// Interior rectangular topology for physical 2-D scalar calibration
    /// problems.  This is typed metadata so reference bindings never infer a
    /// topology from a case-id string or from `dimension` alone.
    #[serde(default)]
    pub grid_shape: Option<[usize; 2]>,
    pub atol: f64,
    pub rtol: f64,
    pub t_span: (f64, f64),
    /// The source-neutral base grid has exactly 101 uniformly spaced points.  A mandatory
    /// breakpoint not already on that grid is inserted into this vector.
    pub output_times: Vec<f64>,
    pub uniform_output_points: usize,
    pub mandatory_breakpoints: Vec<f64>,
    pub provenance: ScientificSourceProvenance,
}

impl ScientificCaseSpec {
    pub fn build(&self) -> CoreResult<ScientificProblemCase> {
        ScientificCorpusV2::build(self)
    }
}

#[derive(Clone)]
pub struct ScientificProblemCase {
    pub spec: ScientificCaseSpec,
    /// Whole-domain source callback, retained for pointwise equation/JVP
    /// inspection.  Integrators must use `integration_segments` when it has
    /// more than one entry so endpoint stages see the correct one-sided RHS.
    pub problem: OdeProblem,
    pub y0: Vec<f64>,
    pub integration_segments: Vec<ScientificProblemSegment>,
}

#[derive(Clone)]
pub struct ScientificProblemSegment {
    pub t_span: (f64, f64),
    /// A branch-fixed problem on `t_span`; callbacks remain on this segment's
    /// side even when a method evaluates exactly at a shared endpoint.
    pub problem: OdeProblem,
}

pub struct ScientificCorpusV2;

impl ScientificCorpusV2 {
    pub const VERSION: &'static str = "scientific-corpus-v2.1";

    pub const fn calibration_dimensions() -> &'static [usize; 3] {
        &CALIBRATION_DIMENSIONS
    }

    pub const fn tolerances() -> &'static [f64; 3] {
        &CORPUS_RTOLS
    }

    pub fn calibration_specs() -> Vec<ScientificCaseSpec> {
        let mut specs = Vec::with_capacity(6 * 3 * 3);
        for family in ScientificFamily::CALIBRATION {
            let t_span = calibration_span(family);
            for dimension in CALIBRATION_DIMENSIONS {
                for rtol in CORPUS_RTOLS {
                    specs.push(case_spec(
                        family,
                        dimension,
                        rtol,
                        t_span,
                        &[],
                        calibration_provenance(family),
                    ));
                }
            }
        }
        specs
    }

    /// Inputs eligible to select or freeze calibration thresholds.
    ///
    /// Keeping this API partition-specific makes a holdout contribution an explicit type-level
    /// policy violation instead of an accidental filter over a mixed list.
    pub fn calibration_threshold_specs() -> Vec<ScientificCaseSpec> {
        Self::calibration_specs()
    }

    pub fn holdout_specs() -> Vec<ScientificCaseSpec> {
        let definitions = [
            (ScientificFamily::Oregonator, 3, (0.0, 360.0), &[][..]),
            (ScientificFamily::Pollution, 20, (0.0, 60.0), &[][..]),
            (ScientificFamily::MedicalAkzo, 400, (0.0, 20.0), &[5.0][..]),
            (
                ScientificFamily::Brusselator2d,
                512,
                (0.0, 11.5),
                &[1.1][..],
            ),
        ];
        let mut specs = Vec::with_capacity(4 * 3);
        for (family, dimension, t_span, breakpoints) in definitions {
            for rtol in CORPUS_RTOLS {
                specs.push(case_spec(
                    family,
                    dimension,
                    rtol,
                    t_span,
                    breakpoints,
                    holdout_provenance(family),
                ));
            }
        }
        specs
    }

    pub fn all_specs() -> Vec<ScientificCaseSpec> {
        let mut specs = Self::calibration_specs();
        specs.extend(Self::holdout_specs());
        specs
    }

    pub fn build(spec: &ScientificCaseSpec) -> CoreResult<ScientificProblemCase> {
        validate_spec(spec)?;
        let (problem, y0) = match spec.family {
            ScientificFamily::RobertsonRamped => robertson_v2(spec.dimension)?,
            ScientificFamily::HiresRamped => hires_v2(spec.dimension)?,
            ScientificFamily::VanDerPolRamped => van_der_pol_v2(spec.dimension)?,
            ScientificFamily::RotatingNonnormal => rotating_nonnormal_v2(spec.dimension)?,
            ScientificFamily::NonautonomousStiffForcing => {
                nonautonomous_forcing_v2(spec.dimension)?
            }
            ScientificFamily::SemilinearAdvectionDiffusionRamped => {
                semilinear_advection_diffusion_v2(spec.dimension)?
            }
            ScientificFamily::Oregonator => oregonator_holdout()?,
            ScientificFamily::Pollution => pollution_holdout()?,
            ScientificFamily::MedicalAkzo => medical_akzo_holdout()?,
            ScientificFamily::Brusselator2d => brusselator_2d_holdout()?,
        };
        let integration_segments = integration_segments(spec, &problem)?;
        Ok(ScientificProblemCase {
            spec: spec.clone(),
            problem,
            y0,
            integration_segments,
        })
    }
}

fn integration_segments(
    spec: &ScientificCaseSpec,
    whole_problem: &OdeProblem,
) -> CoreResult<Vec<ScientificProblemSegment>> {
    let segments = match spec.family {
        ScientificFamily::MedicalAkzo => vec![
            ScientificProblemSegment {
                t_span: (spec.t_span.0, 5.0),
                problem: medical_akzo_holdout_with_fixed_phi(Some(2.0))?.0,
            },
            ScientificProblemSegment {
                t_span: (5.0, spec.t_span.1),
                problem: medical_akzo_holdout_with_fixed_phi(Some(0.0))?.0,
            },
        ],
        ScientificFamily::Brusselator2d => vec![
            ScientificProblemSegment {
                t_span: (spec.t_span.0, 1.1),
                problem: brusselator_2d_holdout_with_fixed_forcing(Some(false))?.0,
            },
            ScientificProblemSegment {
                t_span: (1.1, spec.t_span.1),
                problem: brusselator_2d_holdout_with_fixed_forcing(Some(true))?.0,
            },
        ],
        _ => vec![ScientificProblemSegment {
            t_span: spec.t_span,
            problem: whole_problem.clone(),
        }],
    };
    Ok(segments)
}

/// Prefix-stable, nonperiodic scale in `[0.9, 1.1)` based on the base-two radical inverse.
pub fn v2_diversity_multiplier(index: usize) -> f64 {
    let mut value = index + 1;
    let mut fraction = 0.5;
    let mut radical_inverse = 0.0;
    while value != 0 {
        if value & 1 == 1 {
            radical_inverse += fraction;
        }
        value >>= 1;
        fraction *= 0.5;
    }
    0.9 + 0.2 * radical_inverse
}

fn calibration_span(family: ScientificFamily) -> (f64, f64) {
    match family {
        ScientificFamily::RobertsonRamped => (0.0, 0.10),
        ScientificFamily::HiresRamped
        | ScientificFamily::VanDerPolRamped
        | ScientificFamily::RotatingNonnormal
        | ScientificFamily::NonautonomousStiffForcing
        | ScientificFamily::SemilinearAdvectionDiffusionRamped => (0.0, 1.0),
        _ => unreachable!("holdout family has no calibration span"),
    }
}

fn case_spec(
    family: ScientificFamily,
    dimension: usize,
    rtol: f64,
    t_span: (f64, f64),
    breakpoints: &[f64],
    provenance: ScientificSourceProvenance,
) -> ScientificCaseSpec {
    ScientificCaseSpec {
        id: scientific_case_id(family, dimension, rtol),
        family,
        partition: family.partition(),
        dimension,
        grid_shape: if family == ScientificFamily::SemilinearAdvectionDiffusionRamped {
            let (nx, ny) = semilinear_grid_shape(dimension)
                .expect("calibration dimensions have a declared semilinear grid");
            Some([nx, ny])
        } else {
            None
        },
        atol: 0.01 * rtol,
        rtol,
        t_span,
        output_times: uniform_grid_with_breakpoints(t_span, breakpoints),
        uniform_output_points: UNIFORM_OUTPUT_POINTS,
        mandatory_breakpoints: breakpoints.to_vec(),
        provenance,
    }
}

fn scientific_case_id(family: ScientificFamily, dimension: usize, rtol: f64) -> String {
    if family == ScientificFamily::SemilinearAdvectionDiffusionRamped {
        let (nx, ny) = semilinear_grid_shape(dimension)
            .expect("calibration dimensions have a declared semilinear grid");
        format!(
            "{}-n{}-grid{}x{}-rtol-{rtol:.0e}-v2.1",
            family.as_str(),
            dimension,
            nx,
            ny
        )
    } else {
        format!("{}-n{}-rtol-{rtol:.0e}-v2.1", family.as_str(), dimension)
    }
}

fn uniform_grid_with_breakpoints(t_span: (f64, f64), breakpoints: &[f64]) -> Vec<f64> {
    let (t0, tf) = t_span;
    let mut times = (0..UNIFORM_OUTPUT_POINTS)
        .map(|index| t0 + (tf - t0) * index as f64 / (UNIFORM_OUTPUT_POINTS - 1) as f64)
        .collect::<Vec<_>>();
    for &breakpoint in breakpoints {
        if !times.contains(&breakpoint) {
            times.push(breakpoint);
        }
    }
    times.sort_by(f64::total_cmp);
    times
}

fn calibration_provenance(family: ScientificFamily) -> ScientificSourceProvenance {
    let interpretation_note = if family == ScientificFamily::SemilinearAdvectionDiffusionRamped {
        "VigilODE scientific-corpus-v2.1 manufactured equation: x-fast rectangular interior grid; zero Dirichlet boundary; five-point D=0.002 diffusion; backward-upwind x/y advection a(t)=0.5+3.5*r(t); mu(t)=2+48*r(t); exact phi=exp(-t)*sin(pi*x)*sin(pi*y)"
    } else {
        "scientific-corpus-v2.1 retains the v2 prefix-stable base-two radical-inverse parameter diversification; legacy atlas behavior remains separate"
    };
    ScientificSourceProvenance {
        source_repository: "VigilODE".into(),
        source_revision: ScientificCorpusV2::VERSION.into(),
        source_path: "crates/rodas5p-integrators/src/scientific_corpus_v2.rs".into(),
        source_blob: None,
        source_sha256: None,
        license_or_terms: "repository license".into(),
        interpretation_note: Some(interpretation_note.into()),
    }
}

fn holdout_provenance(family: ScientificFamily) -> ScientificSourceProvenance {
    match family {
        ScientificFamily::Oregonator => ScientificSourceProvenance {
            source_repository: "Bari stiff ODE test set".into(),
            source_revision: "orego.f file identity".into(),
            source_path: "orego.f".into(),
            source_blob: None,
            source_sha256: Some(
                "aa58d9090f1f581f2e60e29b02b409466197981f5399120ce66bfb2d34f41c27".into(),
            ),
            license_or_terms: "clean mathematical reimplementation; source distribution terms apply".into(),
            interpretation_note: None,
        },
        ScientificFamily::Pollution => ScientificSourceProvenance {
            source_repository: "Bari stiff ODE test set".into(),
            source_revision: "pollu.f file identity".into(),
            source_path: "pollu.f".into(),
            source_blob: None,
            source_sha256: Some(
                "2aba777ee6de34e0ee074951375e029ad5171e937dabb7ab4c6461c0736e6c20".into(),
            ),
            license_or_terms: "clean mathematical reimplementation; source distribution terms apply".into(),
            interpretation_note: None,
        },
        ScientificFamily::MedicalAkzo => ScientificSourceProvenance {
            source_repository: "Bari stiff ODE test set".into(),
            source_revision: "medakzo.f file identity".into(),
            source_path: "medakzo.f".into(),
            source_blob: None,
            source_sha256: Some(
                "3b5a4aa80769cd752e17a64a2ae15b4b07ba2a15f037aed48b7c2158d739861a".into(),
            ),
            license_or_terms: "clean mathematical reimplementation; source distribution terms apply".into(),
            interpretation_note: Some("mandatory t=5 split is source-declared".into()),
        },
        ScientificFamily::Brusselator2d => ScientificSourceProvenance {
            source_repository: "SciML/SciMLSensitivity.jl".into(),
            source_revision: "63a13a7301a17feb8cb5e3a4b3ccef4487ae0c52".into(),
            source_path: "docs/src/examples/pde/brusselator.md".into(),
            source_blob: Some("fea9aaa141f224a97f112e024082966a1a5ee6c2".into()),
            source_sha256: Some(
                "688e4642b669e4181cca67d0d7cd9d663e2322d70923daf0240e5a995627351e".into(),
            ),
            license_or_terms: "MIT".into(),
            interpretation_note: Some(
                "f64 translation of the source Float32 executable grid; h=1/15 follows its inclusive range; mandatory t=1.1 split is a VigilODE corpus policy for the discontinuous forcing, not a source tstops declaration"
                    .into(),
            ),
        },
        _ => unreachable!("calibration family has no holdout provenance"),
    }
}

fn validate_spec(spec: &ScientificCaseSpec) -> CoreResult<()> {
    if spec.partition != spec.family.partition() {
        return Err(CoreError::InvalidInput(
            "scientific corpus partition/family mismatch".into(),
        ));
    }
    if !CORPUS_RTOLS.contains(&spec.rtol) || spec.atol != 0.01 * spec.rtol {
        return Err(CoreError::InvalidInput(
            "scientific corpus tolerance is outside the v2 contract".into(),
        ));
    }
    let dimension_valid = match spec.family.partition() {
        CorpusPartition::Calibration => CALIBRATION_DIMENSIONS.contains(&spec.dimension),
        CorpusPartition::Holdout => match spec.family {
            ScientificFamily::Oregonator => spec.dimension == 3,
            ScientificFamily::Pollution => spec.dimension == 20,
            ScientificFamily::MedicalAkzo => spec.dimension == 400,
            ScientificFamily::Brusselator2d => spec.dimension == 512,
            _ => false,
        },
    };
    if !dimension_valid {
        return Err(CoreError::InvalidInput(
            "scientific corpus dimension is outside the v2 contract".into(),
        ));
    }
    let (expected_span, expected_breakpoints, expected_provenance) = match spec.partition {
        CorpusPartition::Calibration => (
            calibration_span(spec.family),
            &[][..],
            calibration_provenance(spec.family),
        ),
        CorpusPartition::Holdout => {
            let (span, breakpoints) = match spec.family {
                ScientificFamily::Oregonator => ((0.0, 360.0), &[][..]),
                ScientificFamily::Pollution => ((0.0, 60.0), &[][..]),
                ScientificFamily::MedicalAkzo => ((0.0, 20.0), &[5.0][..]),
                ScientificFamily::Brusselator2d => ((0.0, 11.5), &[1.1][..]),
                _ => unreachable!("partition validated above"),
            };
            (span, breakpoints, holdout_provenance(spec.family))
        }
    };
    let expected_id = scientific_case_id(spec.family, spec.dimension, spec.rtol);
    let expected_grid_shape = if spec.family == ScientificFamily::SemilinearAdvectionDiffusionRamped
    {
        let (nx, ny) = semilinear_grid_shape(spec.dimension)?;
        Some([nx, ny])
    } else {
        None
    };
    if spec.id != expected_id
        || spec.grid_shape != expected_grid_shape
        || spec.t_span != expected_span
        || spec.uniform_output_points != UNIFORM_OUTPUT_POINTS
        || spec.mandatory_breakpoints != expected_breakpoints
        || spec.output_times != uniform_grid_with_breakpoints(expected_span, expected_breakpoints)
        || spec.provenance != expected_provenance
    {
        return Err(CoreError::InvalidInput(
            "scientific corpus metadata is outside the v2 contract".into(),
        ));
    }
    Ok(())
}

fn smooth_ramp(t: f64, center: f64, width: f64) -> (f64, f64) {
    let z = (t - center) / width;
    let th = z.tanh();
    (0.5 * (1.0 + th), 0.5 * (1.0 - th * th) / width)
}

fn block_padding_rhs(y: &[f64], out: &mut [f64], first_padding: usize, rate: f64) {
    for index in first_padding..y.len() {
        out[index] = -rate * y[index];
    }
}

fn robertson_v2(n: usize) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let blocks = n / 3;
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.045, 0.010);
        let activity = 0.05 + 0.95 * ramp;
        for block in 0..blocks {
            let i = 3 * block;
            let scale = v2_diversity_multiplier(block);
            let k1 = 0.04 * scale;
            let k2 = 1.0e4 * activity * scale;
            let k3 = 3.0e7 * activity * scale;
            let y1 = y[i];
            let y2 = y[i + 1];
            let y3 = y[i + 2];
            out[i] = -k1 * y1 + k2 * y2 * y3;
            out[i + 1] = k1 * y1 - k2 * y2 * y3 - k3 * y2 * y2;
            out[i + 2] = k3 * y2 * y2;
        }
        block_padding_rhs(y, out, 3 * blocks, 20.0 * activity);
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.045, 0.010);
        let activity = 0.05 + 0.95 * ramp;
        for block in 0..blocks {
            let i = 3 * block;
            let scale = v2_diversity_multiplier(block);
            let k1 = 0.04 * scale;
            let k2 = 1.0e4 * activity * scale;
            let k3 = 3.0e7 * activity * scale;
            out[i] = -k1 * v[i] + k2 * y[i + 2] * v[i + 1] + k2 * y[i + 1] * v[i + 2];
            out[i + 1] = k1 * v[i] + (-k2 * y[i + 2] - 2.0 * k3 * y[i + 1]) * v[i + 1]
                - k2 * y[i + 1] * v[i + 2];
            out[i + 2] = 2.0 * k3 * y[i + 1] * v[i + 1];
        }
        for i in 3 * blocks..n {
            out[i] = -20.0 * activity * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (_, dramp) = smooth_ramp(t, 0.045, 0.010);
        let dactivity = 0.95 * dramp;
        out.fill(0.0);
        for block in 0..blocks {
            let i = 3 * block;
            let scale = v2_diversity_multiplier(block);
            let dk2 = 1.0e4 * dactivity * scale;
            let dk3 = 3.0e7 * dactivity * scale;
            let y2 = y[i + 1];
            let y3 = y[i + 2];
            out[i] = dk2 * y2 * y3;
            out[i + 1] = -dk2 * y2 * y3 - dk3 * y2 * y2;
            out[i + 2] = dk3 * y2 * y2;
        }
        for i in 3 * blocks..n {
            out[i] = -20.0 * dactivity * y[i];
        }
        Ok(())
    });
    let mut y0 = vec![0.0; n];
    for block in 0..blocks {
        y0[3 * block] = 1.0;
    }
    let problem = OdeProblem::new(
        format!("robertson-ramped-n{n}-v2"),
        n,
        rhs,
        None,
        None,
        Some(jvp),
        Some(partial_t),
        false,
        None,
        None,
    )?;
    Ok((problem, y0))
}

fn hires_v2(n: usize) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let blocks = n / 8;
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.45, 0.08);
        let activity = 0.1 + 0.9 * ramp;
        for block in 0..blocks {
            let i = 8 * block;
            let scale = v2_diversity_multiplier(block);
            let y1 = y[i];
            let y2 = y[i + 1];
            let y3 = y[i + 2];
            let y4 = y[i + 3];
            let y5 = y[i + 4];
            let y6 = y[i + 5];
            let y7 = y[i + 6];
            let y8 = y[i + 7];
            let q = 280.0 * activity * y6 * y8;
            out[i] = scale * (-1.71 * y1 + 0.43 * y2 + 8.32 * y3 + 0.0007);
            out[i + 1] = scale * (1.71 * y1 - 8.75 * y2);
            out[i + 2] = scale * (-10.03 * y3 + 0.43 * y4 + 0.035 * y5);
            out[i + 3] = scale * (8.32 * y2 + 1.71 * y3 - 1.12 * y4);
            out[i + 4] = scale * (-1.745 * y5 + 0.43 * y6 + 0.43 * y7);
            out[i + 5] = scale * (-q + 0.69 * y4 + 1.71 * y5 - 0.43 * y6 + 0.69 * y7);
            out[i + 6] = scale * (q - 1.81 * y7);
            out[i + 7] = scale * (-q + 1.81 * y7);
        }
        block_padding_rhs(y, out, 8 * blocks, 2.0 + 20.0 * activity);
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.45, 0.08);
        let activity = 0.1 + 0.9 * ramp;
        for block in 0..blocks {
            let i = 8 * block;
            let scale = v2_diversity_multiplier(block);
            let qv = 280.0 * activity * (y[i + 7] * v[i + 5] + y[i + 5] * v[i + 7]);
            out[i] = scale * (-1.71 * v[i] + 0.43 * v[i + 1] + 8.32 * v[i + 2]);
            out[i + 1] = scale * (1.71 * v[i] - 8.75 * v[i + 1]);
            out[i + 2] = scale * (-10.03 * v[i + 2] + 0.43 * v[i + 3] + 0.035 * v[i + 4]);
            out[i + 3] = scale * (8.32 * v[i + 1] + 1.71 * v[i + 2] - 1.12 * v[i + 3]);
            out[i + 4] = scale * (-1.745 * v[i + 4] + 0.43 * v[i + 5] + 0.43 * v[i + 6]);
            out[i + 5] = scale
                * (-qv + 0.69 * v[i + 3] + 1.71 * v[i + 4] - 0.43 * v[i + 5] + 0.69 * v[i + 6]);
            out[i + 6] = scale * (qv - 1.81 * v[i + 6]);
            out[i + 7] = scale * (-qv + 1.81 * v[i + 6]);
        }
        for i in 8 * blocks..n {
            out[i] = -(2.0 + 20.0 * activity) * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (_, dramp) = smooth_ramp(t, 0.45, 0.08);
        let dactivity = 0.9 * dramp;
        out.fill(0.0);
        for block in 0..blocks {
            let i = 8 * block;
            let dq = 280.0 * dactivity * y[i + 5] * y[i + 7];
            let scale = v2_diversity_multiplier(block);
            out[i + 5] = -scale * dq;
            out[i + 6] = scale * dq;
            out[i + 7] = -scale * dq;
        }
        for i in 8 * blocks..n {
            out[i] = -20.0 * dactivity * y[i];
        }
        Ok(())
    });
    let mut y0 = vec![0.0; n];
    for block in 0..blocks {
        y0[8 * block] = 1.0;
        y0[8 * block + 7] = 0.0057;
    }
    let problem = OdeProblem::new(
        format!("hires-ramped-n{n}-v2"),
        n,
        rhs,
        None,
        None,
        Some(jvp),
        Some(partial_t),
        false,
        None,
        None,
    )?;
    Ok((problem, y0))
}

fn van_der_pol_v2(n: usize) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let blocks = n / 2;
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
        let mu = 10.0 + 490.0 * ramp;
        for block in 0..blocks {
            let i = 2 * block;
            let local_mu = mu * v2_diversity_multiplier(block);
            out[i] = y[i + 1];
            out[i + 1] = local_mu * (1.0 - y[i] * y[i]) * y[i + 1] - y[i];
        }
        block_padding_rhs(y, out, 2 * blocks, 5.0 + mu);
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
        let mu = 10.0 + 490.0 * ramp;
        for block in 0..blocks {
            let i = 2 * block;
            let local_mu = mu * v2_diversity_multiplier(block);
            out[i] = v[i + 1];
            out[i + 1] = (-2.0 * local_mu * y[i] * y[i + 1] - 1.0) * v[i]
                + local_mu * (1.0 - y[i] * y[i]) * v[i + 1];
        }
        for i in 2 * blocks..n {
            out[i] = -(5.0 + mu) * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (_, dramp) = smooth_ramp(t, 0.50, 0.08);
        let dmu = 490.0 * dramp;
        out.fill(0.0);
        for block in 0..blocks {
            let i = 2 * block;
            let local_dmu = dmu * v2_diversity_multiplier(block);
            out[i + 1] = local_dmu * (1.0 - y[i] * y[i]) * y[i + 1];
        }
        for i in 2 * blocks..n {
            out[i] = -dmu * y[i];
        }
        Ok(())
    });
    let mut y0 = vec![0.0; n];
    for block in 0..blocks {
        y0[2 * block] = 2.0;
    }
    let problem = OdeProblem::new(
        format!("van-der-pol-ramped-n{n}-v2"),
        n,
        rhs,
        None,
        None,
        Some(jvp),
        Some(partial_t),
        false,
        None,
        None,
    )?;
    Ok((problem, y0))
}

fn rotating_exact_shape(n: usize, t: f64, derivative: usize) -> Vec<f64> {
    (0..n)
        .map(|i| {
            let block = i / 2;
            let frequency = (1.0 + (i % 7) as f64) * v2_diversity_multiplier(block);
            match derivative {
                0 => 0.4 * (frequency * t).sin() + 0.2 * (0.5 * frequency * t).cos(),
                1 => {
                    0.4 * frequency * (frequency * t).cos()
                        - 0.1 * frequency * (0.5 * frequency * t).sin()
                }
                2 => {
                    -0.4 * frequency * frequency * (frequency * t).sin()
                        - 0.05 * frequency * frequency * (0.5 * frequency * t).cos()
                }
                _ => unreachable!("only two exact-shape derivatives are defined"),
            }
        })
        .collect()
}

fn apply_rotating_v2(t: f64, x: &[f64], out: &mut [f64]) {
    let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
    let base_stiffness = 20.0 + 480.0 * ramp;
    let eta = 0.1 + 0.8 * ramp;
    let base_theta = 8.0 * t + 0.4 * (4.0 * t).sin();
    let blocks = x.len() / 2;
    for block in 0..blocks {
        let i = 2 * block;
        let scale = v2_diversity_multiplier(block);
        let stiffness = base_stiffness * scale;
        let theta = base_theta * scale;
        let c = theta.cos();
        let s = theta.sin();
        let xr0 = c * x[i] + s * x[i + 1];
        let xr1 = -s * x[i] + c * x[i + 1];
        let ar0 = -stiffness * xr0 + eta * stiffness * xr1;
        let ar1 = -0.35 * stiffness * xr1;
        out[i] = c * ar0 - s * ar1;
        out[i + 1] = s * ar0 + c * ar1;
    }
    for i in 2 * blocks..x.len() {
        out[i] = -base_stiffness * v2_diversity_multiplier(i) * x[i];
    }
}

fn apply_rotating_v2_partial_t(t: f64, x: &[f64], out: &mut [f64]) {
    let (ramp, dramp) = smooth_ramp(t, 0.50, 0.08);
    let base_stiffness = 20.0 + 480.0 * ramp;
    let base_dstiffness = 480.0 * dramp;
    let eta = 0.1 + 0.8 * ramp;
    let deta = 0.8 * dramp;
    let base_theta = 8.0 * t + 0.4 * (4.0 * t).sin();
    let base_dtheta = 8.0 + 1.6 * (4.0 * t).cos();
    let blocks = x.len() / 2;
    for block in 0..blocks {
        let i = 2 * block;
        let scale = v2_diversity_multiplier(block);
        let stiffness = base_stiffness * scale;
        let dstiffness = base_dstiffness * scale;
        let theta = base_theta * scale;
        let dtheta = base_dtheta * scale;
        let c = theta.cos();
        let s = theta.sin();
        let xr0 = c * x[i] + s * x[i + 1];
        let xr1 = -s * x[i] + c * x[i + 1];
        let dxr0 = dtheta * xr1;
        let dxr1 = -dtheta * xr0;
        let ar0 = -stiffness * xr0 + eta * stiffness * xr1;
        let ar1 = -0.35 * stiffness * xr1;
        let dar0 = -dstiffness * xr0 - stiffness * dxr0
            + deta * stiffness * xr1
            + eta * dstiffness * xr1
            + eta * stiffness * dxr1;
        let dar1 = -0.35 * (dstiffness * xr1 + stiffness * dxr1);
        out[i] = c * dar0 - s * dar1 - dtheta * (s * ar0 + c * ar1);
        out[i + 1] = s * dar0 + c * dar1 + dtheta * (c * ar0 - s * ar1);
    }
    for i in 2 * blocks..x.len() {
        out[i] = -base_dstiffness * v2_diversity_multiplier(i) * x[i];
    }
}

fn rotating_nonnormal_v2(n: usize) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let phi = rotating_exact_shape(n, t, 0);
        let dphi = rotating_exact_shape(n, t, 1);
        let defect = y.iter().zip(&phi).map(|(a, b)| a - b).collect::<Vec<_>>();
        apply_rotating_v2(t, &defect, out);
        let (ramp, _) = smooth_ramp(t, 0.60, 0.06);
        let nonlinear = 40.0 * ramp;
        for i in 0..n {
            out[i] += dphi[i] + nonlinear * (y[i] * y[i] - phi[i] * phi[i]);
        }
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        apply_rotating_v2(t, v, out);
        let (ramp, _) = smooth_ramp(t, 0.60, 0.06);
        let nonlinear = 40.0 * ramp;
        for i in 0..n {
            out[i] += 2.0 * nonlinear * y[i] * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let phi = rotating_exact_shape(n, t, 0);
        let dphi = rotating_exact_shape(n, t, 1);
        let ddphi = rotating_exact_shape(n, t, 2);
        let defect = y.iter().zip(&phi).map(|(a, b)| a - b).collect::<Vec<_>>();
        apply_rotating_v2_partial_t(t, &defect, out);
        let mut operator_phi_t = vec![0.0; n];
        apply_rotating_v2(t, &dphi, &mut operator_phi_t);
        let (ramp, dramp) = smooth_ramp(t, 0.60, 0.06);
        let nonlinear = 40.0 * ramp;
        let dnonlinear = 40.0 * dramp;
        for i in 0..n {
            out[i] += -operator_phi_t[i] + ddphi[i] + dnonlinear * (y[i] * y[i] - phi[i] * phi[i])
                - 2.0 * nonlinear * phi[i] * dphi[i];
        }
        Ok(())
    });
    let y0 = rotating_exact_shape(n, 0.0, 0);
    let exact = Arc::new(move |t: f64| rotating_exact_shape(n, t, 0));
    let problem = OdeProblem::new(
        format!("rotating-nonnormal-n{n}-v2"),
        n,
        rhs,
        None,
        None,
        Some(jvp),
        Some(partial_t),
        false,
        None,
        Some(exact),
    )?;
    Ok((problem, y0))
}

fn nonautonomous_forcing_v2(n: usize) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.45, 0.07);
        let stiffness = 30.0 + 470.0 * ramp;
        let frequency = 2.0 + 28.0 * ramp;
        for i in 0..n {
            let scale = v2_diversity_multiplier(i);
            let phase = (i % 11) as f64 * 0.17;
            let argument = frequency * t + phase;
            let phi = argument.sin();
            let forcing = scale * frequency * argument.cos();
            let defect = y[i] - phi;
            out[i] = -scale * stiffness * defect + forcing + 20.0 * ramp * defect * defect;
        }
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.45, 0.07);
        let stiffness = 30.0 + 470.0 * ramp;
        let frequency = 2.0 + 28.0 * ramp;
        for i in 0..n {
            let scale = v2_diversity_multiplier(i);
            let phase = (i % 11) as f64 * 0.17;
            let phi = (frequency * t + phase).sin();
            let defect = y[i] - phi;
            out[i] = (-scale * stiffness + 40.0 * ramp * defect) * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (ramp, dramp) = smooth_ramp(t, 0.45, 0.07);
        let stiffness = 30.0 + 470.0 * ramp;
        let dstiffness = 470.0 * dramp;
        let frequency = 2.0 + 28.0 * ramp;
        let dfrequency = 28.0 * dramp;
        for i in 0..n {
            let scale = v2_diversity_multiplier(i);
            let phase = (i % 11) as f64 * 0.17;
            let argument = frequency * t + phase;
            let phi = argument.sin();
            let dargument = frequency + t * dfrequency;
            let defect = y[i] - phi;
            let ddefect = -dargument * argument.cos();
            let dforcing =
                scale * (dfrequency * argument.cos() - frequency * argument.sin() * dargument);
            out[i] = -scale * dstiffness * defect - scale * stiffness * ddefect
                + dforcing
                + 20.0 * dramp * defect * defect
                + 40.0 * ramp * defect * ddefect;
        }
        Ok(())
    });
    let y0 = (0..n)
        .map(|i| ((i % 11) as f64 * 0.17).sin())
        .collect::<Vec<_>>();
    let problem = OdeProblem::new(
        format!("nonautonomous-stiff-forcing-n{n}-v2"),
        n,
        rhs,
        None,
        None,
        Some(jvp),
        Some(partial_t),
        false,
        None,
        None,
    )?;
    Ok((problem, y0))
}

fn semilinear_grid_shape(n: usize) -> CoreResult<(usize, usize)> {
    match n {
        96 => Ok((8, 12)),
        384 => Ok((16, 24)),
        1536 => Ok((32, 48)),
        _ => Err(CoreError::InvalidInput(format!(
            "no scientific-corpus-v2.1 semilinear grid for dimension {n}"
        ))),
    }
}

fn semilinear_exact_state(nx: usize, ny: usize, t: f64) -> Vec<f64> {
    let hx = 1.0 / (nx + 1) as f64;
    let hy = 1.0 / (ny + 1) as f64;
    let decay = (-t).exp();
    let mut state = vec![0.0; nx * ny];
    for j in 0..ny {
        let y = (j + 1) as f64 * hy;
        let sy = (std::f64::consts::PI * y).sin();
        for i in 0..nx {
            let x = (i + 1) as f64 * hx;
            state[i + nx * j] = decay * (std::f64::consts::PI * x).sin() * sy;
        }
    }
    state
}

fn apply_advection_diffusion_2d(nx: usize, ny: usize, t: f64, x: &[f64], out: &mut [f64]) {
    const DIFFUSION: f64 = 0.002;
    let hx = 1.0 / (nx + 1) as f64;
    let hy = 1.0 / (ny + 1) as f64;
    let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
    let advection = 0.5 + 3.5 * ramp;
    for j in 0..ny {
        for i in 0..nx {
            let index = i + nx * j;
            let center = x[index];
            let left = if i == 0 { 0.0 } else { x[index - 1] };
            let right = if i + 1 == nx { 0.0 } else { x[index + 1] };
            let down = if j == 0 { 0.0 } else { x[index - nx] };
            let up = if j + 1 == ny { 0.0 } else { x[index + nx] };
            let laplacian =
                (left - 2.0 * center + right) / (hx * hx) + (down - 2.0 * center + up) / (hy * hy);
            let backward_upwind = (center - left) / hx + (center - down) / hy;
            out[index] = DIFFUSION * laplacian - advection * backward_upwind - center;
        }
    }
}

fn semilinear_advection_diffusion_v2(n: usize) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let (nx, ny) = semilinear_grid_shape(n)?;
    let exact = Arc::new(move |t: f64| semilinear_exact_state(nx, ny, t));
    let rhs_exact = exact.clone();
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let phi = rhs_exact(t);
        let defect = y.iter().zip(&phi).map(|(a, b)| a - b).collect::<Vec<_>>();
        apply_advection_diffusion_2d(nx, ny, t, &defect, out);
        let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
        let nonlinear = 2.0 + 48.0 * ramp;
        for i in 0..n {
            out[i] += -phi[i] + nonlinear * (y[i] * y[i] - phi[i] * phi[i]);
        }
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        apply_advection_diffusion_2d(nx, ny, t, v, out);
        let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
        let nonlinear = 2.0 + 48.0 * ramp;
        for i in 0..n {
            out[i] += 2.0 * nonlinear * y[i] * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let phi = semilinear_exact_state(nx, ny, t);
        let defect = y.iter().zip(&phi).map(|(a, b)| a - b).collect::<Vec<_>>();
        let mut operator_phi = vec![0.0; n];
        apply_advection_diffusion_2d(nx, ny, t, &phi, &mut operator_phi);
        let (ramp, dramp) = smooth_ramp(t, 0.50, 0.08);
        let dadvection = 3.5 * dramp;
        let mut operator_t = vec![0.0; n];
        let hx = 1.0 / (nx + 1) as f64;
        let hy = 1.0 / (ny + 1) as f64;
        for j in 0..ny {
            for i in 0..nx {
                let index = i + nx * j;
                let left = if i == 0 { 0.0 } else { defect[index - 1] };
                let down = if j == 0 { 0.0 } else { defect[index - nx] };
                operator_t[index] =
                    -dadvection * ((defect[index] - left) / hx + (defect[index] - down) / hy);
            }
        }
        let nonlinear = 2.0 + 48.0 * ramp;
        let dnonlinear = 48.0 * dramp;
        for i in 0..n {
            out[i] = operator_t[i]
                + operator_phi[i]
                + phi[i]
                + dnonlinear * (y[i] * y[i] - phi[i] * phi[i])
                + 2.0 * nonlinear * phi[i] * phi[i];
        }
        Ok(())
    });
    let y0 = exact(0.0);
    let problem = OdeProblem::new(
        format!("semilinear-advection-diffusion-ramped-n{n}-grid{nx}x{ny}-v2.1"),
        n,
        rhs,
        None,
        None,
        Some(jvp),
        Some(partial_t),
        false,
        None,
        Some(exact),
    )?;
    Ok((problem, y0))
}

fn oregonator_holdout() -> CoreResult<(OdeProblem, Vec<f64>)> {
    const S: f64 = 77.27;
    const Q: f64 = 8.375e-6;
    const W: f64 = 0.161;
    let rhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = S * (y[1] + y[0] * (1.0 - Q * y[0] - y[1]));
        out[1] = (y[2] - (1.0 + y[0]) * y[1]) / S;
        out[2] = W * (y[0] - y[2]);
        Ok(())
    });
    let jvp = Arc::new(|_t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        out[0] = S * ((1.0 - 2.0 * Q * y[0] - y[1]) * v[0] + (1.0 - y[0]) * v[1]);
        out[1] = (-y[1] * v[0] - (1.0 + y[0]) * v[1] + v[2]) / S;
        out[2] = W * (v[0] - v[2]);
        Ok(())
    });
    let y0 = vec![1.0, 2.0, 3.0];
    let problem = OdeProblem::new(
        "oregonator-holdout-v2",
        3,
        rhs,
        None,
        None,
        Some(jvp),
        None,
        true,
        None,
        None,
    )?;
    Ok((problem, y0))
}

const POLLUTION_K: [f64; 25] = [
    0.35, 26.6, 12_300.0, 8.6e-4, 8.2e-4, 15_000.0, 1.3e-4, 24_000.0, 16_500.0, 9_000.0, 0.022,
    12_000.0, 1.88, 16_300.0, 4.8e6, 3.5e-4, 0.0175, 1.0e8, 4.44e11, 1_240.0, 2.1, 5.78, 0.0474,
    1_780.0, 3.12,
];

fn pollution_rates(y: &[f64]) -> [f64; 25] {
    let k = &POLLUTION_K;
    [
        k[0] * y[0],
        k[1] * y[1] * y[3],
        k[2] * y[4] * y[1],
        k[3] * y[6],
        k[4] * y[6],
        k[5] * y[6] * y[5],
        k[6] * y[8],
        k[7] * y[8] * y[5],
        k[8] * y[10] * y[1],
        k[9] * y[10] * y[0],
        k[10] * y[12],
        k[11] * y[9] * y[1],
        k[12] * y[13],
        k[13] * y[0] * y[5],
        k[14] * y[2],
        k[15] * y[3],
        k[16] * y[3],
        k[17] * y[15],
        k[18] * y[15],
        k[19] * y[16] * y[5],
        k[20] * y[18],
        k[21] * y[18],
        k[22] * y[0] * y[3],
        k[23] * y[18] * y[0],
        k[24] * y[19],
    ]
}

fn pollution_rate_derivatives(y: &[f64], v: &[f64]) -> [f64; 25] {
    let k = &POLLUTION_K;
    [
        k[0] * v[0],
        k[1] * (v[1] * y[3] + y[1] * v[3]),
        k[2] * (v[4] * y[1] + y[4] * v[1]),
        k[3] * v[6],
        k[4] * v[6],
        k[5] * (v[6] * y[5] + y[6] * v[5]),
        k[6] * v[8],
        k[7] * (v[8] * y[5] + y[8] * v[5]),
        k[8] * (v[10] * y[1] + y[10] * v[1]),
        k[9] * (v[10] * y[0] + y[10] * v[0]),
        k[10] * v[12],
        k[11] * (v[9] * y[1] + y[9] * v[1]),
        k[12] * v[13],
        k[13] * (v[0] * y[5] + y[0] * v[5]),
        k[14] * v[2],
        k[15] * v[3],
        k[16] * v[3],
        k[17] * v[15],
        k[18] * v[15],
        k[19] * (v[16] * y[5] + y[16] * v[5]),
        k[20] * v[18],
        k[21] * v[18],
        k[22] * (v[0] * y[3] + y[0] * v[3]),
        k[23] * (v[18] * y[0] + y[18] * v[0]),
        k[24] * v[19],
    ]
}

fn assemble_pollution(r: &[f64; 25], out: &mut [f64]) {
    out[0] =
        -r[0] - r[9] - r[13] - r[22] - r[23] + r[1] + r[2] + r[8] + r[10] + r[11] + r[21] + r[24];
    out[1] = -r[1] - r[2] - r[8] - r[11] + r[0] + r[20];
    out[2] = -r[14] + r[0] + r[16] + r[18] + r[21];
    out[3] = -r[1] - r[15] - r[16] - r[22] + r[14];
    out[4] = -r[2] + 2.0 * r[3] + r[5] + r[6] + r[12] + r[19];
    out[5] = -r[5] - r[7] - r[13] - r[19] + r[2] + 2.0 * r[17];
    out[6] = -r[3] - r[4] - r[5] + r[12];
    out[7] = r[3] + r[4] + r[5] + r[6];
    out[8] = -r[6] - r[7];
    out[9] = -r[11] + r[6] + r[8];
    out[10] = -r[8] - r[9] + r[7] + r[10];
    out[11] = r[8];
    out[12] = -r[10] + r[9];
    out[13] = -r[12] + r[11];
    out[14] = r[13];
    out[15] = -r[17] - r[18] + r[15];
    out[16] = -r[19];
    out[17] = r[19];
    out[18] = -r[20] - r[21] - r[23] + r[22] + r[24];
    out[19] = -r[24] + r[23];
}

fn pollution_holdout() -> CoreResult<(OdeProblem, Vec<f64>)> {
    let rhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        assemble_pollution(&pollution_rates(y), out);
        Ok(())
    });
    let jvp = Arc::new(|_t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        assemble_pollution(&pollution_rate_derivatives(y, v), out);
        Ok(())
    });
    let y0 = vec![
        0.0, 0.2, 0.0, 0.04, 0.0, 0.0, 0.1, 0.3, 0.01, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.007,
        0.0, 0.0, 0.0,
    ];
    let problem = OdeProblem::new(
        "pollution-holdout-v2",
        20,
        rhs,
        None,
        None,
        Some(jvp),
        None,
        true,
        None,
        None,
    )?;
    Ok((problem, y0))
}

fn medical_akzo_holdout() -> CoreResult<(OdeProblem, Vec<f64>)> {
    medical_akzo_holdout_with_fixed_phi(None)
}

fn medical_akzo_holdout_with_fixed_phi(
    fixed_phi: Option<f64>,
) -> CoreResult<(OdeProblem, Vec<f64>)> {
    const N: usize = 200;
    const H: f64 = 0.005;
    const K: f64 = 100.0;
    const C: f64 = 4.0;
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let phi = fixed_phi.unwrap_or(if t <= 5.0 { 2.0 } else { 0.0 });
        for j in 0..N {
            let i = 2 * j;
            let zeta = (j + 1) as f64 * H;
            let u = y[i];
            let v = y[i + 1];
            let um = if j == 0 { phi } else { y[i - 2] };
            let up = if j + 1 == N { u } else { y[i + 2] };
            let a = 2.0 * (zeta - 1.0).powi(3) / (C * C);
            let b = (zeta - 1.0).powi(4) / (C * C);
            let reaction = K * u * v;
            out[i] = a * (up - um) / (2.0 * H) + b * (um - 2.0 * u + up) / (H * H) - reaction;
            out[i + 1] = -reaction;
        }
        Ok(())
    });
    let jvp = Arc::new(|_t: f64, y: &[f64], direction: &[f64], out: &mut [f64]| {
        for j in 0..N {
            let i = 2 * j;
            let zeta = (j + 1) as f64 * H;
            let pu = direction[i];
            let pv = direction[i + 1];
            let pm = if j == 0 { 0.0 } else { direction[i - 2] };
            let pp = if j + 1 == N { pu } else { direction[i + 2] };
            let a = 2.0 * (zeta - 1.0).powi(3) / (C * C);
            let b = (zeta - 1.0).powi(4) / (C * C);
            let reaction = K * (y[i + 1] * pu + y[i] * pv);
            out[i] = a * (pp - pm) / (2.0 * H) + b * (pm - 2.0 * pu + pp) / (H * H) - reaction;
            out[i + 1] = -reaction;
        }
        Ok(())
    });
    let partial_t = Arc::new(|_t: f64, _y: &[f64], out: &mut [f64]| {
        // The RHS is autonomous on each side of the mandatory t=5 split.  The distributional
        // derivative at the jump is not sampled by a step that honors the corpus breakpoint.
        out.fill(0.0);
        Ok(())
    });
    let mut y0 = vec![0.0; 2 * N];
    for j in 0..N {
        y0[2 * j + 1] = 1.0;
    }
    let name = match fixed_phi {
        Some(2.0) => "medical-akzo-holdout-v2-left-segment",
        Some(0.0) => "medical-akzo-holdout-v2-right-segment",
        Some(_) => "medical-akzo-holdout-v2-invalid-segment",
        None => "medical-akzo-holdout-v2",
    };
    let problem = OdeProblem::new(
        name,
        2 * N,
        rhs,
        None,
        None,
        Some(jvp),
        Some(partial_t),
        false,
        None,
        None,
    )?;
    Ok((problem, y0))
}

fn brusselator_offset(i: usize, j: usize) -> usize {
    i + 16 * j
}

fn brusselator_2d_holdout() -> CoreResult<(OdeProblem, Vec<f64>)> {
    brusselator_2d_holdout_with_fixed_forcing(None)
}

fn brusselator_2d_holdout_with_fixed_forcing(
    fixed_forcing: Option<bool>,
) -> CoreResult<(OdeProblem, Vec<f64>)> {
    const SIDE: usize = 16;
    const PLANE: usize = SIDE * SIDE;
    const A: f64 = 3.4;
    const B: f64 = 1.0;
    const ALPHA: f64 = 10.0;
    const H: f64 = 1.0 / 15.0;
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let diffusion = ALPHA / (H * H);
        for j in 0..SIDE {
            let jm = if j == 0 { SIDE - 1 } else { j - 1 };
            let jp = if j + 1 == SIDE { 0 } else { j + 1 };
            for i in 0..SIDE {
                let im = if i == 0 { SIDE - 1 } else { i - 1 };
                let ip = if i + 1 == SIDE { 0 } else { i + 1 };
                let index = brusselator_offset(i, j);
                let u = y[index];
                let v = y[PLANE + index];
                let lap_u = y[brusselator_offset(im, j)]
                    + y[brusselator_offset(ip, j)]
                    + y[brusselator_offset(i, jm)]
                    + y[brusselator_offset(i, jp)]
                    - 4.0 * u;
                let lap_v = y[PLANE + brusselator_offset(im, j)]
                    + y[PLANE + brusselator_offset(ip, j)]
                    + y[PLANE + brusselator_offset(i, jm)]
                    + y[PLANE + brusselator_offset(i, jp)]
                    - 4.0 * v;
                let x = i as f64 * H;
                let yy = j as f64 * H;
                let forcing_enabled = fixed_forcing.unwrap_or(t >= 1.1);
                let forcing = if forcing_enabled && (x - 0.3).powi(2) + (yy - 0.6).powi(2) <= 0.01 {
                    5.0
                } else {
                    0.0
                };
                let uv2 = u * u * v;
                out[index] = diffusion * lap_u + B + uv2 - (A + 1.0) * u + forcing;
                out[PLANE + index] = diffusion * lap_v + A * u - uv2;
            }
        }
        Ok(())
    });
    let jvp = Arc::new(|_t: f64, y: &[f64], direction: &[f64], out: &mut [f64]| {
        let diffusion = ALPHA / (H * H);
        for j in 0..SIDE {
            let jm = if j == 0 { SIDE - 1 } else { j - 1 };
            let jp = if j + 1 == SIDE { 0 } else { j + 1 };
            for i in 0..SIDE {
                let im = if i == 0 { SIDE - 1 } else { i - 1 };
                let ip = if i + 1 == SIDE { 0 } else { i + 1 };
                let index = brusselator_offset(i, j);
                let u = y[index];
                let v = y[PLANE + index];
                let pu = direction[index];
                let pv = direction[PLANE + index];
                let lap_pu = direction[brusselator_offset(im, j)]
                    + direction[brusselator_offset(ip, j)]
                    + direction[brusselator_offset(i, jm)]
                    + direction[brusselator_offset(i, jp)]
                    - 4.0 * pu;
                let lap_pv = direction[PLANE + brusselator_offset(im, j)]
                    + direction[PLANE + brusselator_offset(ip, j)]
                    + direction[PLANE + brusselator_offset(i, jm)]
                    + direction[PLANE + brusselator_offset(i, jp)]
                    - 4.0 * pv;
                out[index] = diffusion * lap_pu + (2.0 * u * v - A - 1.0) * pu + u * u * pv;
                out[PLANE + index] = diffusion * lap_pv + (A - 2.0 * u * v) * pu - u * u * pv;
            }
        }
        Ok(())
    });
    let partial_t = Arc::new(|_t: f64, _y: &[f64], out: &mut [f64]| {
        // Smooth-side derivative; the t=1.1 jump is isolated by the mandatory corpus split.
        out.fill(0.0);
        Ok(())
    });
    let mut y0 = vec![0.0; 2 * PLANE];
    for j in 0..SIDE {
        let yy = j as f64 * H;
        for i in 0..SIDE {
            let x = i as f64 * H;
            let index = brusselator_offset(i, j);
            y0[index] = 22.0 * (yy * (1.0 - yy)).powf(1.5);
            y0[PLANE + index] = 27.0 * (x * (1.0 - x)).powf(1.5);
        }
    }
    let name = match fixed_forcing {
        Some(false) => "brusselator-2d-holdout-v2-left-segment",
        Some(true) => "brusselator-2d-holdout-v2-right-segment",
        None => "brusselator-2d-holdout-v2",
    };
    let problem = OdeProblem::new(
        name,
        2 * PLANE,
        rhs,
        None,
        None,
        Some(jvp),
        Some(partial_t),
        false,
        None,
        None,
    )?;
    Ok((problem, y0))
}
