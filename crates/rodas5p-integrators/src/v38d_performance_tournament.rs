use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use serde::{Deserialize, Serialize};

pub const V38D_EXPLORATORY_PROBE_SCHEMA: &str = "vigilode-v38d-exploratory-probe-v1";
pub const V38D_EXPLORATORY_PROBE_STATUS: &str = "EXPLORATORY_NOT_TIMING_AUTHORITY";
pub const V38D_WARMUP_REPETITIONS: usize = 1;
pub const V38D_MEASURED_REPETITIONS: usize = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum V38dProbeCaseId {
    #[serde(rename = "stiff-diagonal-96")]
    StiffDiagonal96,
    #[serde(rename = "nonnormal-jordan-96")]
    NonnormalJordan96,
    #[serde(rename = "oscillatory-blocks-96")]
    OscillatoryBlocks96,
    #[serde(rename = "diffusion-like-192")]
    DiffusionLike192,
    #[serde(rename = "mixed-forcing-192")]
    MixedForcing192,
}

impl V38dProbeCaseId {
    pub const ALL: [Self; 5] = [
        Self::StiffDiagonal96,
        Self::NonnormalJordan96,
        Self::OscillatoryBlocks96,
        Self::DiffusionLike192,
        Self::MixedForcing192,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StiffDiagonal96 => "stiff-diagonal-96",
            Self::NonnormalJordan96 => "nonnormal-jordan-96",
            Self::OscillatoryBlocks96 => "oscillatory-blocks-96",
            Self::DiffusionLike192 => "diffusion-like-192",
            Self::MixedForcing192 => "mixed-forcing-192",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum V38dCandidateId {
    #[serde(rename = "full-mgs-authority")]
    FullMgsAuthority,
}

impl V38dCandidateId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FullMgsAuthority => "full-mgs-authority",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct V38dProbeSample {
    pub repetition: usize,
    pub wall_seconds: f64,
    pub allocations: u64,
    pub allocated_bytes: u64,
    pub work: WorkCounters,
    pub output_checksum: String,
    pub authority_wrms_defect: f64,
    pub residual_estimate: f64,
    pub converged: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct V38dProbeReport {
    pub schema: String,
    pub status: String,
    pub case_id: V38dProbeCaseId,
    pub candidate_id: V38dCandidateId,
    pub warmups: Vec<V38dProbeSample>,
    pub measured: Vec<V38dProbeSample>,
    pub timing_authority: bool,
    pub speedup_claim_authorized: bool,
    pub active_switching_authorized: bool,
    pub policy_retuning_authorized: bool,
    pub release_claim_authorized: bool,
    pub n2048_authorized: bool,
}

impl V38dProbeReport {
    pub fn new(
        case_id: V38dProbeCaseId,
        candidate_id: V38dCandidateId,
        warmups: Vec<V38dProbeSample>,
        measured: Vec<V38dProbeSample>,
    ) -> CoreResult<Self> {
        validate_sample_set("warmup", &warmups, V38D_WARMUP_REPETITIONS)?;
        validate_sample_set("measured", &measured, V38D_MEASURED_REPETITIONS)?;

        Ok(Self {
            schema: V38D_EXPLORATORY_PROBE_SCHEMA.into(),
            status: V38D_EXPLORATORY_PROBE_STATUS.into(),
            case_id,
            candidate_id,
            warmups,
            measured,
            timing_authority: false,
            speedup_claim_authorized: false,
            active_switching_authorized: false,
            policy_retuning_authorized: false,
            release_claim_authorized: false,
            n2048_authorized: false,
        })
    }
}

fn validate_sample_set(
    label: &str,
    samples: &[V38dProbeSample],
    required: usize,
) -> CoreResult<()> {
    if samples.len() != required {
        return Err(CoreError::InvalidInput(format!(
            "v3.8-D {label} sample count must be exactly {required}, got {}",
            samples.len()
        )));
    }
    for (expected, sample) in samples.iter().enumerate() {
        if sample.repetition != expected {
            return Err(CoreError::InvalidInput(format!(
                "v3.8-D {label} repetition mismatch at position {expected}: got {}",
                sample.repetition
            )));
        }
    }
    Ok(())
}

pub fn run_v38d_probe(
    _case_id: V38dProbeCaseId,
    _candidate_id: V38dCandidateId,
    warmups: usize,
    measured_repetitions: usize,
) -> CoreResult<V38dProbeReport> {
    if warmups != V38D_WARMUP_REPETITIONS {
        return Err(CoreError::InvalidInput(format!(
            "v3.8-D warmup count must be exactly {V38D_WARMUP_REPETITIONS}, got {warmups}"
        )));
    }
    if measured_repetitions != V38D_MEASURED_REPETITIONS {
        return Err(CoreError::InvalidInput(format!(
            "v3.8-D measured repetition count must be exactly {V38D_MEASURED_REPETITIONS}, got {measured_repetitions}"
        )));
    }
    Err(CoreError::InvalidInput(
        "v3.8-D probe case not implemented".into(),
    ))
}
