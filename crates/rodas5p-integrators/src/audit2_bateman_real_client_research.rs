//! Source-bound, feature-gated real-client authority for the Audit-2 transaction.
//!
//! Construction validates a single canonical Bateman two-timescale manifest.
//! It does not execute a transactional candidate, alter the default dispatcher,
//! or admit a production/general accuracy claim.

use std::fmt;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, ExactPreconditionerIdentity, Preconditioner,
    WorkCounters, apply_counted, load_rodas5p_coefficients, sha256_hex,
};
use serde::{Deserialize, Serialize};

use crate::audit2_matrix_free_research::{
    Audit2MatrixFreeCommonWConfig, Audit2MatrixFreeCorrectionFailure,
    Audit2MatrixFreeCorrectionFailurePhase, Audit2MatrixFreeCorrectionSuccess,
};
use crate::audit2_reusable_transaction_research::{
    Audit2ExternalOutputReference, Audit2FrozenWSemanticIdentity, Audit2IndependentBudgetReceipt,
    Audit2IndependentStepBudget, Audit2ReferenceUncertaintyTreatment,
    Audit2ReusablePreconditionerCache, Audit2ReusablePreconditionerCacheSnapshot,
    Audit2ReusablePreconditionerIdentity, Audit2TransactionalAttemptConfig,
    Audit2TransactionalAttemptOutcome, Audit2TransactionalFailurePhase,
    Audit2TransactionalSelection, run_audit2_reusable_preconditioner_transactional_attempt,
};
use crate::{OdeProblem, StepContext, StepResult, build_step_context_matrix_free};

pub const AUDIT2_BATEMAN_CLIENT_ID: &str = "bateman-two-timescale-parent-stable-daughter-v1";
pub const AUDIT2_BATEMAN_NOMINAL_CASE_ID: &str = "nominal-h1e-3";
pub const AUDIT2_BATEMAN_CHANGED_W_CASE_ID: &str = "changed-w-h5e-4";
pub const AUDIT2_BATEMAN_FROZEN_W_SCHEMA: &str = "vigilode-audit2-bateman-frozen-w/v1";
pub const AUDIT2_BATEMAN_SCENARIO_IDS: [&str; 6] = [
    "same-live-context-reuse",
    "changed-w-invalidation",
    "nominal-independent-budget",
    "over-strict-budget-fallback",
    "late-preconditioner-failure",
    "terminal-rejection",
];
pub const AUDIT2_BATEMAN_AUTHORITY_MANIFEST_SHA256: &str =
    "673045bf6b9e723fceb6a3b8df8e9e9e9075c942cf1c438f0ebd03574dbac360";
pub const AUDIT2_BATEMAN_AUTHORITY_VERIFIER_SHA256: &str =
    "542715ca749efbf2060d608f2089ee8457e32f9c61fd0d35f613d5ecec26487d";
pub const AUDIT2_BATEMAN_AUTHORITY_PROOF_SHA256: &str =
    "057cceba92fed0d707db1d586b53adebee5aed00583b224811d091f1d453ab12";

const CHECKED_IN_MANIFEST_BYTES: &[u8] = include_bytes!(
    "../../../research/audit2_real_client_authority_construction_20260830/authority_manifest.json"
);
const CHECKED_IN_VERIFIER_BYTES: &[u8] = include_bytes!(
    "../../../research/audit2_real_client_authority_construction_20260830/verify_authority_manifest.py"
);
const CHECKED_IN_PROOF_BYTES: &[u8] = include_bytes!(
    "../../../research/audit2_real_client_authority_construction_20260830/evidence/AUTHORITY_VERIFICATION_RECEIPT.json"
);

const MODEL_ID: &str = "bateman_two_pair_v1";
const SCALAR_FORMAT: &str = "IEEE754_BINARY64";
const MATRIX_LAYOUT: &str = "ROW_MAJOR";
const W_CONVENTION: &str = "W=I-(h*gamma)*J; h_gamma rounded binary64 before row construction";
const FROZEN_W_DOMAIN: &[u8] = b"VIGILODE\0AUDIT2\0FROZEN_W\0";
const FAST_RATE_BITS: u64 = 0x408f_4000_0000_0000;
const SLOW_RATE_BITS: u64 = 0x3ff0_0000_0000_0000;
const GAMMA_BITS: u64 = 0x3fcb_20c5_235b_5100;
const INITIAL_STATE_BITS: [u64; 4] = [
    0x3fe0_0000_0000_0000,
    0x0000_0000_0000_0000,
    0x3fe0_0000_0000_0000,
    0x0000_0000_0000_0000,
];
const NOMINAL_REFERENCE_BITS: [u64; 4] = [
    0x3fc7_8b56_362c_ef38,
    0x3fd4_3a54_e4e9_8864,
    0x3fdf_f7cf_e56f_1a9e,
    0x3f40_6035_21ca_c48b,
];
const CHANGED_REFERENCE_BITS: [u64; 4] = [
    0x3fd3_68b2_fc6f_960a,
    0x3fc9_2e9a_0720_d3ec,
    0x3fdf_fbe7_afa4_452e,
    0x3f30_6141_6eeb_48bc,
];
const NOMINAL_DIGEST: &str = "8de6e2cb36fa3f899bae98ba36e9f28f5f293f3ca1259267f8d3ef6c12e22b58";
const CHANGED_DIGEST: &str = "907c709cf96d60ff2372a1e3dc3e9ef7bc098e9499654292146cb23c98fe8386";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Audit2BatemanOperatorAuthority {
    pub case_id: String,
    pub t: f64,
    pub h: f64,
    pub attempt_config: Audit2TransactionalAttemptConfig,
    pub reference: Audit2ExternalOutputReference,
    pub budget: Audit2IndependentStepBudget,
    pub frozen_w_semantic: Audit2FrozenWSemanticIdentity,
    pub preconditioner_identity: Audit2ReusablePreconditionerIdentity,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Audit2BatemanRealClientManifest {
    pub schema: String,
    pub client_id: String,
    pub model_classification: String,
    pub equation_authority_doi: String,
    pub stiffness_authority: String,
    pub state_order: Vec<String>,
    pub rate_bits: Vec<u64>,
    pub initial_state_bits: Vec<u64>,
    pub coefficient_gamma_bits: u64,
    pub frozen_w_serialization: String,
    pub reference_method: String,
    pub reference_verifier: String,
    pub trial_stage_policy: String,
    pub candidate_execution_during_construction: String,
    pub holdout_access: String,
    pub operator_cases: Vec<Audit2BatemanOperatorAuthority>,
    pub execution_scenarios: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Audit2BatemanAuthorityVerificationReceipt {
    schema: String,
    status: String,
    authority_manifest_sha256: String,
    exact_verifier_sha256: String,
    reference_method: String,
    verified_operator_cases: usize,
    execution_scenarios: usize,
    candidate_executions: usize,
    declared_reference_l2_uncertainty: f64,
    max_reference_l2_bound: f64,
    fast_exponent_exceeds_one: bool,
    uncertainty_treatment: String,
    output_admission_rule: String,
    holdout_access: String,
    local_six_case_status: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Audit2BatemanScenarioKind {
    SameLiveContextCacheProbe,
    ChangedWCacheProbe,
    TransactionalNominal,
    TransactionalStrictFallback,
    TransactionalLateApplyFailure,
    TransactionalTerminalRejection,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Audit2BatemanScenarioPlan {
    pub ordinal: usize,
    pub scenario_id: String,
    pub operator_case_id: String,
    pub kind: Audit2BatemanScenarioKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Audit2BatemanScenarioDisposition {
    CacheReuseObserved,
    ChangedWInvalidationObserved,
    ContractMismatch,
    Candidate,
    ProtectedFallback,
    Rejected,
    AttemptFailed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audit2BatemanStepReceipt {
    pub method: String,
    pub accepted: bool,
    pub used_fallback: bool,
    pub error_norm: f64,
    pub y_new_bits: Vec<u64>,
    pub counters: WorkCounters,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audit2BatemanScenarioReceipt {
    pub ordinal: usize,
    pub scenario_id: String,
    pub operator_case_id: String,
    pub kind: Audit2BatemanScenarioKind,
    pub disposition: Audit2BatemanScenarioDisposition,
    pub contract_satisfied: bool,
    pub committed: Option<bool>,
    pub committed_state_bits: Option<Vec<u64>>,
    pub committed_state_sha256: Option<String>,
    pub candidate_step: Option<Audit2BatemanStepReceipt>,
    pub selected_step: Option<Audit2BatemanStepReceipt>,
    pub fallback_step: Option<Audit2BatemanStepReceipt>,
    pub candidate_budget: Option<Audit2IndependentBudgetReceipt>,
    pub candidate_correction: Option<Audit2MatrixFreeCorrectionSuccess>,
    pub candidate_failure: Option<Audit2MatrixFreeCorrectionFailure>,
    pub candidate_failure_phase: Option<Audit2MatrixFreeCorrectionFailurePhase>,
    pub transaction_failure_phase: Option<Audit2TransactionalFailurePhase>,
    pub transaction_failure_message: Option<String>,
    pub cache: Audit2ReusablePreconditionerCacheSnapshot,
    pub work: WorkCounters,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audit2BatemanPartialFailure {
    pub scenario_id: String,
    pub message: String,
    pub cache: Option<Audit2ReusablePreconditionerCacheSnapshot>,
    pub work: WorkCounters,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audit2BatemanSixCaseReport {
    pub schema: String,
    pub claim_scope: String,
    pub client_id: String,
    pub authority_manifest_sha256: String,
    pub exact_verifier_sha256: String,
    pub authority_proof_sha256: String,
    pub scenario_plan: Vec<Audit2BatemanScenarioPlan>,
    pub scenario_receipts: Vec<Audit2BatemanScenarioReceipt>,
    pub all_six_executed: bool,
    pub all_contracts_satisfied: bool,
    pub terminal_failure: Option<Audit2BatemanPartialFailure>,
}

pub struct Audit2BatemanRealClientAuthority {
    manifest: Audit2BatemanRealClientManifest,
    problem: OdeProblem,
    initial_state: Vec<f64>,
    manifest_sha256: String,
    verifier_sha256: String,
    proof_sha256: String,
}

impl fmt::Debug for Audit2BatemanRealClientAuthority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Audit2BatemanRealClientAuthority")
            .field("client_id", &self.manifest.client_id)
            .field("manifest_sha256", &self.manifest_sha256)
            .field("verifier_sha256", &self.verifier_sha256)
            .field("proof_sha256", &self.proof_sha256)
            .finish()
    }
}

impl Audit2BatemanRealClientAuthority {
    pub fn manifest(&self) -> &Audit2BatemanRealClientManifest {
        &self.manifest
    }

    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }

    pub fn verifier_sha256(&self) -> &str {
        &self.verifier_sha256
    }

    pub fn proof_sha256(&self) -> &str {
        &self.proof_sha256
    }

    fn case(&self, case_id: &str) -> CoreResult<&Audit2BatemanOperatorAuthority> {
        self.manifest
            .operator_cases
            .iter()
            .find(|case| case.case_id == case_id)
            .ok_or_else(|| {
                CoreError::InvalidInput(format!(
                    "Audit-2 Bateman authority has no operator case {case_id}"
                ))
            })
    }
}

pub fn audit2_bateman_six_case_plan() -> Vec<Audit2BatemanScenarioPlan> {
    use Audit2BatemanScenarioKind::{
        ChangedWCacheProbe, SameLiveContextCacheProbe, TransactionalLateApplyFailure,
        TransactionalNominal, TransactionalStrictFallback, TransactionalTerminalRejection,
    };
    [
        (
            AUDIT2_BATEMAN_SCENARIO_IDS[0],
            AUDIT2_BATEMAN_NOMINAL_CASE_ID,
            SameLiveContextCacheProbe,
        ),
        (
            AUDIT2_BATEMAN_SCENARIO_IDS[1],
            AUDIT2_BATEMAN_CHANGED_W_CASE_ID,
            ChangedWCacheProbe,
        ),
        (
            AUDIT2_BATEMAN_SCENARIO_IDS[2],
            AUDIT2_BATEMAN_NOMINAL_CASE_ID,
            TransactionalNominal,
        ),
        (
            AUDIT2_BATEMAN_SCENARIO_IDS[3],
            AUDIT2_BATEMAN_NOMINAL_CASE_ID,
            TransactionalStrictFallback,
        ),
        (
            AUDIT2_BATEMAN_SCENARIO_IDS[4],
            AUDIT2_BATEMAN_NOMINAL_CASE_ID,
            TransactionalLateApplyFailure,
        ),
        (
            AUDIT2_BATEMAN_SCENARIO_IDS[5],
            AUDIT2_BATEMAN_NOMINAL_CASE_ID,
            TransactionalTerminalRejection,
        ),
    ]
    .into_iter()
    .enumerate()
    .map(
        |(ordinal, (scenario_id, operator_case_id, kind))| Audit2BatemanScenarioPlan {
            ordinal: ordinal + 1,
            scenario_id: scenario_id.into(),
            operator_case_id: operator_case_id.into(),
            kind,
        },
    )
    .collect()
}

pub struct Audit2BatemanOperatorBinding {
    pub frozen_w_semantic: Audit2FrozenWSemanticIdentity,
    pub preconditioner_identity: Audit2ReusablePreconditionerIdentity,
    pub preconditioner: Arc<dyn Preconditioner>,
}

/// Candidate-free proof that one admitted operator case binds to the runtime
/// matrix-free context and exact analytic diagonal named by the manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Audit2BatemanRuntimeBindingReceipt {
    pub case_id: String,
    pub dimension: usize,
    pub frozen_w_sha256: String,
    pub inverse_diagonal_bits: Vec<u64>,
    pub rhs_calls: u64,
    pub diagnostic_matvecs: u64,
    pub jvp_calls: u64,
    pub candidate_executions: u64,
}

struct Audit2BatemanAnalyticDiagonal {
    inverse_diagonal: Vec<f64>,
}

impl Preconditioner for Audit2BatemanAnalyticDiagonal {
    fn dimension(&self) -> usize {
        self.inverse_diagonal.len()
    }

    fn apply(&self, input: &[f64], output: &mut [f64]) -> CoreResult<()> {
        if input.len() != self.dimension() || output.len() != input.len() {
            return Err(CoreError::Dimension(
                "Audit-2 Bateman analytic diagonal shape mismatch".into(),
            ));
        }
        for ((value, inverse), input) in output.iter_mut().zip(&self.inverse_diagonal).zip(input) {
            *value = inverse * input;
        }
        Ok(())
    }

    fn exact_identity(&self) -> Option<ExactPreconditionerIdentity> {
        Some(ExactPreconditionerIdentity::Jacobi {
            inverse_diagonal_bits: self
                .inverse_diagonal
                .iter()
                .map(|value| value.to_bits())
                .collect(),
        })
    }
}

fn from_bits(bits: &[u64]) -> Vec<f64> {
    bits.iter().map(|bits| f64::from_bits(*bits)).collect()
}

fn push_u32(bytes: &mut Vec<u8>, value: usize) {
    bytes.extend_from_slice(&(value as u32).to_be_bytes());
}

fn push_frame(bytes: &mut Vec<u8>, value: &str) {
    push_u32(bytes, value.len());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_f64(bytes: &mut Vec<u8>, value: f64) {
    bytes.extend_from_slice(&value.to_bits().to_be_bytes());
}

fn shifted_w_entries(h_gamma: f64) -> [f64; 16] {
    let fast = h_gamma * f64::from_bits(FAST_RATE_BITS);
    let slow = h_gamma * f64::from_bits(SLOW_RATE_BITS);
    [
        1.0 + fast,
        0.0,
        0.0,
        0.0,
        -fast,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0 + slow,
        0.0,
        0.0,
        0.0,
        -slow,
        1.0,
    ]
}

fn frozen_w_digest(case_id: &str, t: f64, h: f64, gamma: f64, h_gamma: f64) -> String {
    let mut bytes = FROZEN_W_DOMAIN.to_vec();
    push_u32(&mut bytes, 1);
    for field in [
        MODEL_ID,
        SCALAR_FORMAT,
        MATRIX_LAYOUT,
        W_CONVENTION,
        case_id,
    ] {
        push_frame(&mut bytes, field);
    }
    push_u32(&mut bytes, 4);
    for value in [t, h, gamma] {
        push_f64(&mut bytes, value);
    }
    push_u32(&mut bytes, 2);
    push_f64(&mut bytes, f64::from_bits(FAST_RATE_BITS));
    push_f64(&mut bytes, f64::from_bits(SLOW_RATE_BITS));
    push_u32(&mut bytes, INITIAL_STATE_BITS.len());
    for value in from_bits(&INITIAL_STATE_BITS) {
        push_f64(&mut bytes, value);
    }
    let entries = shifted_w_entries(h_gamma);
    push_u32(&mut bytes, entries.len());
    for value in entries {
        push_f64(&mut bytes, value);
    }
    sha256_hex(&bytes)
}

fn preconditioner_identity(
    h: f64,
    gamma: f64,
    h_gamma: f64,
) -> Audit2ReusablePreconditionerIdentity {
    let entries = shifted_w_entries(h_gamma);
    let inverse = [1.0 / entries[0], 1.0, 1.0 / entries[10], 1.0];
    Audit2ReusablePreconditionerIdentity {
        provider: "analytic-bateman-jacobi-inverse-multiply".into(),
        revision: 1,
        configuration_bits: vec![
            h.to_bits(),
            gamma.to_bits(),
            FAST_RATE_BITS,
            SLOW_RATE_BITS,
            entries[0].to_bits(),
            entries[10].to_bits(),
        ],
        expected_inverse_diagonal_bits: inverse.iter().map(|value| value.to_bits()).collect(),
    }
}

fn operator_case(
    case_id: &str,
    h: f64,
    reference_bits: &[u64; 4],
    expected_digest: &str,
    gamma: f64,
) -> CoreResult<Audit2BatemanOperatorAuthority> {
    let h_gamma = h * gamma;
    let digest = frozen_w_digest(case_id, 0.0, h, gamma, h_gamma);
    if digest != expected_digest {
        return Err(CoreError::InvalidInput(format!(
            "Audit-2 Bateman frozen-W digest drift for {case_id}: {digest}"
        )));
    }
    Ok(Audit2BatemanOperatorAuthority {
        case_id: case_id.into(),
        t: 0.0,
        h,
        attempt_config: Audit2TransactionalAttemptConfig {
            common_w: Audit2MatrixFreeCommonWConfig::default(),
            outer_atol: 1.0e-4,
            outer_rtol: 1.0e-6,
        },
        reference: Audit2ExternalOutputReference {
            source: "bateman-taylor-lagrange-fraction-v1".into(),
            state: from_bits(reference_bits),
            uncertainty_l2: f64::from_bits(0x3cd2_03af_9ee7_5616),
            uncertainty_treatment: Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound,
        },
        budget: Audit2IndependentStepBudget {
            identifier: format!("frozen-bateman-independent-budget-{case_id}-v1"),
            output_atol_l2: 1.0e-4,
            output_rtol: 1.0e-6,
            max_embedded_l2: 2.0e-4,
            max_original_target_residual_l2: 1.0e-10,
            max_original_target_contraction: 1.0e-8,
        },
        frozen_w_semantic: Audit2FrozenWSemanticIdentity {
            schema: AUDIT2_BATEMAN_FROZEN_W_SCHEMA.into(),
            sha256: digest,
        },
        preconditioner_identity: preconditioner_identity(h, gamma, h_gamma),
    })
}

pub fn audit2_bateman_real_client_manifest() -> CoreResult<Audit2BatemanRealClientManifest> {
    let coefficients = load_rodas5p_coefficients()?;
    if coefficients.gamma.to_bits() != GAMMA_BITS {
        return Err(CoreError::InvalidInput(
            "Audit-2 Bateman authority requires the frozen coefficient gamma bits".into(),
        ));
    }
    let gamma = coefficients.gamma;
    Ok(Audit2BatemanRealClientManifest {
        schema: "vigilode-audit2-bateman-real-client-authority/v1".into(),
        client_id: AUDIT2_BATEMAN_CLIENT_ID.into(),
        model_classification: "REAL_NUCLEAR_DECAY_PHYSICS_BATEMAN_TWO_PAIR".into(),
        equation_authority_doi: "10.1063/1.2715785;10.1119/1.5064446".into(),
        stiffness_authority: "Hykes-and-Ferrer-2013-Bateman-stiff-implicit-ODE".into(),
        state_order: [
            "fast-parent",
            "fast-stable-daughter",
            "slow-parent",
            "slow-stable-daughter",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        rate_bits: vec![FAST_RATE_BITS, SLOW_RATE_BITS],
        initial_state_bits: INITIAL_STATE_BITS.to_vec(),
        coefficient_gamma_bits: GAMMA_BITS,
        frozen_w_serialization: format!(
            "{W_CONVENTION}; domain=VIGILODE\\0AUDIT2\\0FROZEN_W\\0; u32/f64 big-endian; length-framed UTF-8"
        ),
        reference_method: "Bateman analytic solution enclosed by exact-rational Taylor-Lagrange S41/S40 endpoint bounds".into(),
        reference_verifier: "stdlib Python Fraction.from_float; exact squared-L2 comparison; Wolfram cross-check only".into(),
        trial_stage_policy: "eight all-zero stage increments; work starts before target preparation".into(),
        candidate_execution_during_construction: "FORBIDDEN".into(),
        holdout_access: "NOT_OPENED_OR_EXECUTED".into(),
        operator_cases: vec![
            operator_case(
                AUDIT2_BATEMAN_NOMINAL_CASE_ID,
                f64::from_bits(0x3f50_624d_d2f1_a9fc),
                &NOMINAL_REFERENCE_BITS,
                NOMINAL_DIGEST,
                gamma,
            )?,
            operator_case(
                AUDIT2_BATEMAN_CHANGED_W_CASE_ID,
                f64::from_bits(0x3f40_624d_d2f1_a9fc),
                &CHANGED_REFERENCE_BITS,
                CHANGED_DIGEST,
                gamma,
            )?,
        ],
        execution_scenarios: AUDIT2_BATEMAN_SCENARIO_IDS
            .into_iter()
            .map(str::to_owned)
            .collect(),
    })
}

fn bateman_problem() -> CoreResult<(OdeProblem, Vec<f64>)> {
    let fast_rate = f64::from_bits(FAST_RATE_BITS);
    let slow_rate = f64::from_bits(SLOW_RATE_BITS);
    let rhs = Arc::new(move |_t: f64, y: &[f64], output: &mut [f64]| {
        output[0] = -fast_rate * y[0];
        output[1] = fast_rate * y[0];
        output[2] = -slow_rate * y[2];
        output[3] = slow_rate * y[2];
        Ok(())
    });
    let batch = Arc::new(move |_times: &[f64], states: &[Vec<f64>]| {
        Ok(states
            .iter()
            .map(|y| {
                vec![
                    -fast_rate * y[0],
                    fast_rate * y[0],
                    -slow_rate * y[2],
                    slow_rate * y[2],
                ]
            })
            .collect())
    });
    let jvp = Arc::new(
        move |_t: f64, _y: &[f64], vector: &[f64], output: &mut [f64]| {
            output[0] = -fast_rate * vector[0];
            output[1] = fast_rate * vector[0];
            output[2] = -slow_rate * vector[2];
            output[3] = slow_rate * vector[2];
            Ok(())
        },
    );
    Ok((
        OdeProblem::new(
            AUDIT2_BATEMAN_CLIENT_ID,
            4,
            rhs,
            Some(batch),
            None,
            Some(jvp),
            None,
            true,
            None,
            None,
        )?,
        from_bits(&INITIAL_STATE_BITS),
    ))
}

pub fn admit_audit2_bateman_real_client_authority(
    manifest_bytes: &[u8],
    verifier_bytes: &[u8],
    proof_bytes: &[u8],
) -> CoreResult<Audit2BatemanRealClientAuthority> {
    let manifest_sha256 = sha256_hex(manifest_bytes);
    let verifier_sha256 = sha256_hex(verifier_bytes);
    let proof_sha256 = sha256_hex(proof_bytes);
    if manifest_bytes != CHECKED_IN_MANIFEST_BYTES
        || verifier_bytes != CHECKED_IN_VERIFIER_BYTES
        || proof_bytes != CHECKED_IN_PROOF_BYTES
        || manifest_sha256 != AUDIT2_BATEMAN_AUTHORITY_MANIFEST_SHA256
        || verifier_sha256 != AUDIT2_BATEMAN_AUTHORITY_VERIFIER_SHA256
        || proof_sha256 != AUDIT2_BATEMAN_AUTHORITY_PROOF_SHA256
    {
        return Err(CoreError::InvalidInput(
            "Audit-2 Bateman admission requires the exact checked-in manifest, verifier, and proof receipt bytes".into(),
        ));
    }
    let manifest: Audit2BatemanRealClientManifest = serde_json::from_slice(manifest_bytes)
        .map_err(|error| {
            CoreError::InvalidInput(format!("Audit-2 Bateman manifest JSON is invalid: {error}"))
        })?;
    let canonical = audit2_bateman_real_client_manifest()?;
    if manifest != canonical {
        return Err(CoreError::InvalidInput(
            "Audit-2 Bateman authority requires the exact canonical manifest".into(),
        ));
    }
    let proof: Audit2BatemanAuthorityVerificationReceipt = serde_json::from_slice(proof_bytes)
        .map_err(|error| {
            CoreError::InvalidInput(format!(
                "Audit-2 Bateman proof receipt JSON is invalid: {error}"
            ))
        })?;
    let proof_valid = proof.schema == "vigilode-audit2-bateman-authority-verification-receipt/v1"
        && proof.status == "AUTHORITY_CONSTRUCTION_VERIFIED"
        && proof.authority_manifest_sha256 == manifest_sha256
        && proof.exact_verifier_sha256 == verifier_sha256
        && proof.reference_method == "exact-binary-fraction-taylor-lagrange-s41-s40"
        && proof.verified_operator_cases == 2
        && proof.execution_scenarios == AUDIT2_BATEMAN_SCENARIO_IDS.len()
        && proof.candidate_executions == 0
        && proof.declared_reference_l2_uncertainty.to_bits() == 1.0e-15f64.to_bits()
        && proof.max_reference_l2_bound.is_finite()
        && proof.max_reference_l2_bound >= 0.0
        && proof.max_reference_l2_bound < proof.declared_reference_l2_uncertainty
        && proof.fast_exponent_exceeds_one
        && proof.uncertainty_treatment == "DECLARED_UPPER_BOUND"
        && proof.output_admission_rule == "E_ref + u <= B_abs + B_rel * norm2(reference)"
        && proof.holdout_access == "NOT_OPENED_OR_EXECUTED"
        && proof.local_six_case_status == "NOT_RUN_DURING_AUTHORITY_CONSTRUCTION";
    if !proof_valid {
        return Err(CoreError::InvalidInput(
            "Audit-2 Bateman proof receipt does not bind the frozen candidate-free authority"
                .into(),
        ));
    }
    let (problem, initial_state) = bateman_problem()?;
    Ok(Audit2BatemanRealClientAuthority {
        manifest,
        problem,
        initial_state,
        manifest_sha256,
        verifier_sha256,
        proof_sha256,
    })
}

fn bind_audit2_bateman_operator_authority(
    authority: &Audit2BatemanRealClientAuthority,
    case: &Audit2BatemanOperatorAuthority,
    context: &StepContext<'_>,
) -> CoreResult<Audit2BatemanOperatorBinding> {
    let exact_state = authority
        .initial_state
        .iter()
        .zip(&context.y)
        .all(|(expected, actual)| expected.to_bits() == actual.to_bits());
    if context.problem.name != AUDIT2_BATEMAN_CLIENT_ID
        || context.problem.dimension != 4
        || context.t.to_bits() != case.t.to_bits()
        || context.h.to_bits() != case.h.to_bits()
        || !exact_state
    {
        return Err(CoreError::InvalidInput(
            "Audit-2 Bateman context differs from the admitted operator case".into(),
        ));
    }
    let gamma = f64::from_bits(authority.manifest.coefficient_gamma_bits);
    let digest = frozen_w_digest(
        &case.case_id,
        context.t,
        context.h,
        gamma,
        context.shifted.h_gamma(),
    );
    if digest != case.frozen_w_semantic.sha256 {
        return Err(CoreError::InvalidInput(
            "Audit-2 Bateman runtime frozen-W digest differs from the manifest".into(),
        ));
    }
    let identity = preconditioner_identity(context.h, gamma, context.shifted.h_gamma());
    if identity != case.preconditioner_identity {
        return Err(CoreError::InvalidInput(
            "Audit-2 Bateman runtime diagonal differs from the manifest".into(),
        ));
    }
    let preconditioner = Arc::new(Audit2BatemanAnalyticDiagonal {
        inverse_diagonal: from_bits(&identity.expected_inverse_diagonal_bits),
    });
    let preconditioner: Arc<dyn Preconditioner> = preconditioner;
    Ok(Audit2BatemanOperatorBinding {
        frozen_w_semantic: case.frozen_w_semantic.clone(),
        preconditioner_identity: identity,
        preconditioner,
    })
}

/// Rebind both frozen operator cases without constructing trial stages or
/// calling the transactional candidate.
///
/// The diagnostic basis actions prove that the live shifted operator has the
/// exact canonical W entries and that the runtime preconditioner applies the
/// exact admitted diagonal map. The function exposes no caller-selected case,
/// parameter, budget, or threshold.
pub fn audit2_bateman_verify_runtime_operator_bindings_candidate_free(
    authority: &Audit2BatemanRealClientAuthority,
) -> CoreResult<Vec<Audit2BatemanRuntimeBindingReceipt>> {
    let mut receipts = Vec::with_capacity(authority.manifest.operator_cases.len());
    for case in &authority.manifest.operator_cases {
        let mut work = WorkCounters::default();
        let context = build_step_context_matrix_free(
            &authority.problem,
            case.t,
            &authority.initial_state,
            case.h,
            &mut work,
        )?;
        let binding = bind_audit2_bateman_operator_authority(authority, case, &context)?;
        let expected_w = shifted_w_entries(context.shifted.h_gamma());
        for column in 0..4 {
            let mut basis = vec![0.0; 4];
            basis[column] = 1.0;
            let mut shifted_output = vec![0.0; 4];
            apply_counted(
                &context.shifted,
                &basis,
                &mut shifted_output,
                &mut work,
                ApplyCategory::Diagnostic,
            )?;
            for (row, actual) in shifted_output.iter().enumerate() {
                if actual.to_bits() != expected_w[row * 4 + column].to_bits() {
                    return Err(CoreError::InvalidInput(format!(
                        "Audit-2 Bateman runtime shifted action differs from canonical W for {}",
                        case.case_id
                    )));
                }
            }

            let mut preconditioned = vec![0.0; 4];
            binding.preconditioner.apply(&basis, &mut preconditioned)?;
            for (row, actual) in preconditioned.iter().enumerate() {
                let expected = if row == column {
                    f64::from_bits(
                        binding
                            .preconditioner_identity
                            .expected_inverse_diagonal_bits[row],
                    )
                } else {
                    0.0
                };
                if actual.to_bits() != expected.to_bits() {
                    return Err(CoreError::InvalidInput(format!(
                        "Audit-2 Bateman runtime preconditioner differs from admitted map for {}",
                        case.case_id
                    )));
                }
            }
        }
        receipts.push(Audit2BatemanRuntimeBindingReceipt {
            case_id: case.case_id.clone(),
            dimension: context.problem.dimension,
            frozen_w_sha256: binding.frozen_w_semantic.sha256,
            inverse_diagonal_bits: binding
                .preconditioner_identity
                .expected_inverse_diagonal_bits,
            rhs_calls: work.rhs_calls,
            diagnostic_matvecs: work.diagnostic_matvecs,
            jvp_calls: work.jvp_calls,
            candidate_executions: 0,
        });
    }
    Ok(receipts)
}

struct Audit2BatemanSecondApplyFailure {
    inverse_diagonal: Vec<f64>,
    apply_attempts: AtomicUsize,
}

impl Preconditioner for Audit2BatemanSecondApplyFailure {
    fn dimension(&self) -> usize {
        self.inverse_diagonal.len()
    }

    fn apply(&self, input: &[f64], output: &mut [f64]) -> CoreResult<()> {
        let attempt = self.apply_attempts.fetch_add(1, Ordering::SeqCst);
        if attempt == 1 {
            return Err(CoreError::LinearSolve(
                "frozen Bateman second-apply failure injection".into(),
            ));
        }
        if input.len() != self.dimension() || output.len() != input.len() {
            return Err(CoreError::Dimension(
                "Audit-2 Bateman failure-injection diagonal shape mismatch".into(),
            ));
        }
        for ((value, inverse), input) in output.iter_mut().zip(&self.inverse_diagonal).zip(input) {
            *value = inverse * input;
        }
        Ok(())
    }

    fn exact_identity(&self) -> Option<ExactPreconditionerIdentity> {
        Some(ExactPreconditionerIdentity::Jacobi {
            inverse_diagonal_bits: self
                .inverse_diagonal
                .iter()
                .map(|value| value.to_bits())
                .collect(),
        })
    }
}

fn cache_probe_receipt(
    plan: &Audit2BatemanScenarioPlan,
    cache: &Audit2ReusablePreconditionerCache,
    work: WorkCounters,
    contract_satisfied: bool,
) -> Audit2BatemanScenarioReceipt {
    let disposition = if !contract_satisfied {
        Audit2BatemanScenarioDisposition::ContractMismatch
    } else {
        match plan.kind {
            Audit2BatemanScenarioKind::SameLiveContextCacheProbe => {
                Audit2BatemanScenarioDisposition::CacheReuseObserved
            }
            Audit2BatemanScenarioKind::ChangedWCacheProbe => {
                Audit2BatemanScenarioDisposition::ChangedWInvalidationObserved
            }
            _ => unreachable!("cache probe receipt requires a cache-probe plan"),
        }
    };
    Audit2BatemanScenarioReceipt {
        ordinal: plan.ordinal,
        scenario_id: plan.scenario_id.clone(),
        operator_case_id: plan.operator_case_id.clone(),
        kind: plan.kind,
        disposition,
        contract_satisfied,
        committed: None,
        committed_state_bits: None,
        committed_state_sha256: None,
        candidate_step: None,
        selected_step: None,
        fallback_step: None,
        candidate_budget: None,
        candidate_correction: None,
        candidate_failure: None,
        candidate_failure_phase: None,
        transaction_failure_phase: None,
        transaction_failure_message: None,
        cache: cache.snapshot(),
        work,
    }
}

fn step_receipt(step: &StepResult) -> Audit2BatemanStepReceipt {
    Audit2BatemanStepReceipt {
        method: step.method.clone(),
        accepted: step.accepted,
        used_fallback: step.used_fallback,
        error_norm: step.error_norm,
        y_new_bits: step.y_new.iter().map(|value| value.to_bits()).collect(),
        counters: step.counters,
    }
}

fn state_bits_and_digest(scenario_id: &str, state: &[f64]) -> (Vec<u64>, String) {
    let bits = state
        .iter()
        .map(|value| value.to_bits())
        .collect::<Vec<_>>();
    let mut bytes = b"VIGILODE\0AUDIT2\0BATEMAN_STATE\0".to_vec();
    push_frame(&mut bytes, scenario_id);
    push_u32(&mut bytes, bits.len());
    for value in &bits {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
    (bits, sha256_hex(&bytes))
}

fn zero_trial(context: &StepContext<'_>) -> CoreResult<Vec<Vec<f64>>> {
    if context.coeffs.stages() != 8 {
        return Err(CoreError::InvalidInput(
            "Audit-2 Bateman authority requires exactly eight zero trial stages".into(),
        ));
    }
    Ok(vec![vec![0.0; context.problem.dimension]; 8])
}

fn strict_budget(
    case: &Audit2BatemanOperatorAuthority,
    suffix: &str,
) -> Audit2IndependentStepBudget {
    Audit2IndependentStepBudget {
        identifier: format!("frozen-bateman-{suffix}-{}-v1", case.case_id),
        output_atol_l2: 0.0,
        output_rtol: 0.0,
        max_embedded_l2: 0.0,
        max_original_target_residual_l2: 0.0,
        max_original_target_contraction: 0.0,
    }
}

fn transaction_receipt(
    plan: &Audit2BatemanScenarioPlan,
    outcome: Audit2TransactionalAttemptOutcome,
) -> Audit2BatemanScenarioReceipt {
    match outcome {
        Audit2TransactionalAttemptOutcome::Completed(completed) => {
            let disposition = match completed.selection {
                Audit2TransactionalSelection::Candidate => {
                    Audit2BatemanScenarioDisposition::Candidate
                }
                Audit2TransactionalSelection::ProtectedFallback => {
                    Audit2BatemanScenarioDisposition::ProtectedFallback
                }
                Audit2TransactionalSelection::Rejected => {
                    Audit2BatemanScenarioDisposition::Rejected
                }
            };
            let candidate_budget = completed
                .candidate
                .as_ref()
                .map(|candidate| candidate.budget.clone());
            let candidate_step = completed
                .candidate
                .as_ref()
                .map(|candidate| step_receipt(&candidate.step));
            let candidate_correction = completed
                .candidate
                .as_ref()
                .map(|candidate| candidate.correction.clone());
            let candidate_failure = completed.candidate_failure.clone();
            let candidate_failure_phase = completed
                .candidate_failure
                .as_ref()
                .map(|failure| failure.phase);
            let late_apply_ledger_satisfied = completed
                .candidate_failure
                .as_ref()
                .and_then(|failure| failure.work.session.as_ref())
                .is_some_and(|session| {
                    session.preconditioner_apply_attempts == 2
                        && session.preconditioner_apply_completed == 1
                });
            let candidate_step_accepted_but_budget_rejected = completed
                .candidate
                .as_ref()
                .is_some_and(|candidate| candidate.step.accepted && !candidate.budget.accepted);
            let contract_satisfied = match plan.kind {
                Audit2BatemanScenarioKind::TransactionalNominal => {
                    completed.selection == Audit2TransactionalSelection::Candidate
                        && completed.committed
                        && completed.candidate.is_some()
                }
                Audit2BatemanScenarioKind::TransactionalStrictFallback => {
                    completed.selection == Audit2TransactionalSelection::ProtectedFallback
                        && completed.committed
                        && candidate_step_accepted_but_budget_rejected
                }
                Audit2BatemanScenarioKind::TransactionalLateApplyFailure => {
                    completed.selection == Audit2TransactionalSelection::ProtectedFallback
                        && completed.committed
                        && late_apply_ledger_satisfied
                        && completed.cache.commits == 0
                        && completed.cache.rollbacks == 1
                }
                Audit2BatemanScenarioKind::TransactionalTerminalRejection => {
                    completed.selection == Audit2TransactionalSelection::Rejected
                        && !completed.committed
                        && completed.selected_step.is_none()
                        && completed.candidate.is_some()
                        && completed.cache.commits == 0
                        && completed.cache.rollbacks == 1
                }
                _ => false,
            };
            let (committed_state_bits, committed_state_sha256) =
                state_bits_and_digest(&plan.scenario_id, &completed.committed_state);
            Audit2BatemanScenarioReceipt {
                ordinal: plan.ordinal,
                scenario_id: plan.scenario_id.clone(),
                operator_case_id: plan.operator_case_id.clone(),
                kind: plan.kind,
                disposition,
                contract_satisfied,
                committed: Some(completed.committed),
                committed_state_bits: Some(committed_state_bits),
                committed_state_sha256: Some(committed_state_sha256),
                candidate_step,
                selected_step: completed.selected_step.as_ref().map(step_receipt),
                fallback_step: completed.fallback_step.as_ref().map(step_receipt),
                candidate_budget,
                candidate_correction,
                candidate_failure,
                candidate_failure_phase,
                transaction_failure_phase: None,
                transaction_failure_message: None,
                cache: completed.cache.clone(),
                work: completed.work,
            }
        }
        Audit2TransactionalAttemptOutcome::Failed(failure) => {
            let (committed_state_bits, committed_state_sha256) =
                state_bits_and_digest(&plan.scenario_id, &failure.committed_state);
            Audit2BatemanScenarioReceipt {
                ordinal: plan.ordinal,
                scenario_id: plan.scenario_id.clone(),
                operator_case_id: plan.operator_case_id.clone(),
                kind: plan.kind,
                disposition: Audit2BatemanScenarioDisposition::AttemptFailed,
                contract_satisfied: false,
                committed: Some(false),
                committed_state_bits: Some(committed_state_bits),
                committed_state_sha256: Some(committed_state_sha256),
                candidate_step: None,
                selected_step: None,
                fallback_step: None,
                candidate_budget: None,
                candidate_correction: None,
                candidate_failure: failure.candidate_failure.clone(),
                candidate_failure_phase: failure
                    .candidate_failure
                    .as_ref()
                    .map(|candidate| candidate.phase),
                transaction_failure_phase: Some(failure.phase),
                transaction_failure_message: Some(failure.message.clone()),
                cache: failure.cache.clone(),
                work: failure.work,
            }
        }
    }
}

fn run_transactional_scenario(
    authority: &Audit2BatemanRealClientAuthority,
    plan: &Audit2BatemanScenarioPlan,
) -> Result<Audit2BatemanScenarioReceipt, Box<Audit2BatemanPartialFailure>> {
    let mut cache = Audit2ReusablePreconditionerCache::default();
    let mut work = WorkCounters::default();
    let result = (|| -> CoreResult<Audit2BatemanScenarioReceipt> {
        let case = authority.case(&plan.operator_case_id)?;
        let context = build_step_context_matrix_free(
            &authority.problem,
            case.t,
            &authority.initial_state,
            case.h,
            &mut work,
        )?;
        let trial = zero_trial(&context)?;
        let binding = bind_audit2_bateman_operator_authority(authority, case, &context)?;
        let mut config = case.attempt_config.clone();
        let budget = match plan.kind {
            Audit2BatemanScenarioKind::TransactionalNominal
            | Audit2BatemanScenarioKind::TransactionalLateApplyFailure => case.budget.clone(),
            Audit2BatemanScenarioKind::TransactionalStrictFallback => {
                strict_budget(case, "over-strict-budget-fallback")
            }
            Audit2BatemanScenarioKind::TransactionalTerminalRejection => {
                config.outer_atol = 1.0e-30;
                config.outer_rtol = 0.0;
                strict_budget(case, "terminal-rejection")
            }
            _ => {
                return Err(CoreError::InvalidInput(
                    "Audit-2 Bateman transaction runner received a cache-only scenario".into(),
                ));
            }
        };
        let preconditioner: Arc<dyn Preconditioner> =
            if plan.kind == Audit2BatemanScenarioKind::TransactionalLateApplyFailure {
                Arc::new(Audit2BatemanSecondApplyFailure {
                    inverse_diagonal: from_bits(
                        &binding
                            .preconditioner_identity
                            .expected_inverse_diagonal_bits,
                    ),
                    apply_attempts: AtomicUsize::new(0),
                })
            } else {
                binding.preconditioner
            };
        let outcome = run_audit2_reusable_preconditioner_transactional_attempt(
            &context,
            &trial,
            &config,
            &budget,
            &case.reference,
            &mut cache,
            binding.frozen_w_semantic,
            binding.preconditioner_identity,
            move |_, _| Ok(preconditioner),
            &mut work,
        );
        Ok(transaction_receipt(plan, outcome))
    })();
    result.map_err(|error| {
        Box::new(Audit2BatemanPartialFailure {
            scenario_id: plan.scenario_id.clone(),
            message: error.to_string(),
            cache: Some(cache.snapshot()),
            work,
        })
    })
}

/// Execute the one-shot, local-only, preregistered Bateman six-case suite.
///
/// The authority token is consumed, so a caller cannot reuse one admission to
/// select additional cases or alter any numerical knob. The report preserves
/// every completed receipt and a terminal setup/attempt failure when the suite
/// cannot reach all six scenarios. This remains a non-production research
/// exercise and does not open or execute the holdout corpus.
pub fn run_audit2_bateman_local_six_case_suite(
    authority: Audit2BatemanRealClientAuthority,
) -> Audit2BatemanSixCaseReport {
    let plan = audit2_bateman_six_case_plan();
    let mut report = Audit2BatemanSixCaseReport {
        schema: "vigilode-audit2-bateman-local-six-case-report/v1".into(),
        claim_scope: "LOCAL_ONLY_EXPLORATORY_NONAUTHORITATIVE_REAL_CLIENT_VALIDATION".into(),
        client_id: authority.manifest.client_id.clone(),
        authority_manifest_sha256: authority.manifest_sha256.clone(),
        exact_verifier_sha256: authority.verifier_sha256.clone(),
        authority_proof_sha256: authority.proof_sha256.clone(),
        scenario_plan: plan.clone(),
        scenario_receipts: Vec::with_capacity(plan.len()),
        all_six_executed: false,
        all_contracts_satisfied: false,
        terminal_failure: None,
    };

    let mut cache = Audit2ReusablePreconditionerCache::default();
    let mut probe_work = WorkCounters::default();
    let probe_result = (|| -> CoreResult<()> {
        let nominal_case = authority.case(AUDIT2_BATEMAN_NOMINAL_CASE_ID)?;
        let nominal_context = build_step_context_matrix_free(
            &authority.problem,
            nominal_case.t,
            &authority.initial_state,
            nominal_case.h,
            &mut probe_work,
        )?;
        let binding =
            bind_audit2_bateman_operator_authority(&authority, nominal_case, &nominal_context)?;
        cache.begin_attempt(
            &nominal_context,
            binding.frozen_w_semantic.clone(),
            binding.preconditioner_identity.clone(),
            move |_, _| Ok(binding.preconditioner),
        )?;
        cache.commit_attempt()?;
        cache.begin_attempt(
            &nominal_context,
            nominal_case.frozen_w_semantic.clone(),
            nominal_case.preconditioner_identity.clone(),
            |_, _| {
                Err(CoreError::InvalidInput(
                    "same-W cache probe unexpectedly reran setup".into(),
                ))
            },
        )?;
        cache.commit_attempt()?;
        let snapshot = cache.snapshot();
        let same_contract = snapshot.attempts == 2
            && snapshot.setup_attempts == 1
            && snapshot.setup_completed == 1
            && snapshot.same_binding_reuses == 1
            && snapshot.commits == 2
            && snapshot.pending_binding.is_none();
        report.scenario_receipts.push(cache_probe_receipt(
            &plan[0],
            &cache,
            probe_work,
            same_contract,
        ));

        let changed_case = authority.case(AUDIT2_BATEMAN_CHANGED_W_CASE_ID)?;
        probe_work = WorkCounters::default();
        let changed_context = build_step_context_matrix_free(
            &authority.problem,
            changed_case.t,
            &authority.initial_state,
            changed_case.h,
            &mut probe_work,
        )?;
        let changed_binding =
            bind_audit2_bateman_operator_authority(&authority, changed_case, &changed_context)?;
        cache.begin_attempt(
            &changed_context,
            changed_binding.frozen_w_semantic,
            changed_binding.preconditioner_identity,
            move |_, _| Ok(changed_binding.preconditioner),
        )?;
        cache.commit_attempt()?;
        let changed_snapshot = cache.snapshot();
        let changed_contract = changed_snapshot.attempts == 3
            && changed_snapshot.setup_attempts == 2
            && changed_snapshot.setup_completed == 2
            && changed_snapshot.same_binding_reuses == 1
            && changed_snapshot.changed_operator_invalidations == 1
            && changed_snapshot.changed_preconditioner_invalidations == 1
            && changed_snapshot.commits == 3
            && changed_snapshot.pending_binding.is_none();
        report.scenario_receipts.push(cache_probe_receipt(
            &plan[1],
            &cache,
            probe_work,
            changed_contract,
        ));
        Ok(())
    })();
    if let Err(error) = probe_result {
        let scenario_id = plan
            .get(report.scenario_receipts.len())
            .map(|scenario| scenario.scenario_id.clone())
            .unwrap_or_else(|| "cache-probe-orchestration".into());
        report.terminal_failure = Some(Audit2BatemanPartialFailure {
            scenario_id,
            message: error.to_string(),
            cache: Some(cache.snapshot()),
            work: probe_work,
        });
        return report;
    }

    for scenario in &plan[2..] {
        match run_transactional_scenario(&authority, scenario) {
            Ok(receipt) => {
                let attempt_failed =
                    receipt.disposition == Audit2BatemanScenarioDisposition::AttemptFailed;
                let failure_message = receipt.transaction_failure_message.clone();
                let failure_cache = receipt.cache.clone();
                let failure_work = receipt.work;
                report.scenario_receipts.push(receipt);
                if attempt_failed {
                    report.terminal_failure = Some(Audit2BatemanPartialFailure {
                        scenario_id: scenario.scenario_id.clone(),
                        message: failure_message
                            .unwrap_or_else(|| "transactional attempt failed".into()),
                        cache: Some(failure_cache),
                        work: failure_work,
                    });
                    break;
                }
            }
            Err(failure) => {
                report.terminal_failure = Some(*failure);
                break;
            }
        }
    }
    report.all_six_executed = report.scenario_receipts.len() == plan.len();
    report.all_contracts_satisfied = report.all_six_executed
        && report
            .scenario_receipts
            .iter()
            .all(|receipt| receipt.contract_satisfied);
    report
}
