use rodas5p_core::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

use crate::{
    G4S5B0AttemptTraceReport, G4S5B0Family, G4S5B0FrozenFullEShadowHardGates,
    G4S5B0InnerToleranceLane, G4S5B0InnerTolerancePolicy, G4S5B0LinearToleranceArm, G4S5B0Profile,
    V36_FROZEN_ZETA34_TAU, g4_s5b0_regime_atlas::run_g4_s5b0_frozen_full_e_shadow_receipt_family,
    g4_s5b0_rjf_trace_digest,
};

pub const A1_TWO_ARM_RECEIPT_SCHEMA: &str = "vigilode-a1-two-arm-atomic-cell-v1";
pub const A1_TWO_ARM_RECEIPT_PROFILE: G4S5B0Profile = G4S5B0Profile::EnforcedBudgetHoldout320;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct A1ScientificExecutionIdentity {
    pub repository: String,
    pub pull_request: u64,
    pub scientific_execution_head_sha: String,
    pub scientific_execution_head_tree: String,
    pub base_sha: String,
    pub base_tree: String,
    pub tested_execution_merge_sha: String,
    pub tested_execution_merge_tree: String,
    pub execution_workflow_run_id: u64,
    pub execution_workflow_run_attempt: u64,
    pub rust_version: String,
    pub cargo_version: String,
}

impl A1ScientificExecutionIdentity {
    fn validate(&self) -> CoreResult<()> {
        if self.repository.trim().is_empty() {
            return Err(CoreError::InvalidInput(
                "A1 receipt repository must be non-empty".into(),
            ));
        }
        if self.pull_request == 0 {
            return Err(CoreError::InvalidInput(
                "A1 receipt pull_request must be positive".into(),
            ));
        }
        for (name, value) in [
            (
                "scientific_execution_head_sha",
                &self.scientific_execution_head_sha,
            ),
            (
                "scientific_execution_head_tree",
                &self.scientific_execution_head_tree,
            ),
            ("base_sha", &self.base_sha),
            ("base_tree", &self.base_tree),
            (
                "tested_execution_merge_sha",
                &self.tested_execution_merge_sha,
            ),
            (
                "tested_execution_merge_tree",
                &self.tested_execution_merge_tree,
            ),
        ] {
            if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(CoreError::InvalidInput(format!(
                    "A1 receipt {name} must be a 40-digit hexadecimal Git object identity"
                )));
            }
        }
        if self.execution_workflow_run_id == 0 || self.execution_workflow_run_attempt == 0 {
            return Err(CoreError::InvalidInput(
                "A1 receipt execution workflow run ID and attempt must be positive".into(),
            ));
        }
        if self.rust_version.trim().is_empty() || self.cargo_version.trim().is_empty() {
            return Err(CoreError::InvalidInput(
                "A1 receipt Rust and Cargo versions must be non-empty".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct A1ToleranceReceiptEventRow {
    pub event_key: String,
    pub trajectory_id: String,
    pub decision_accepted_step: usize,
    pub target_attempt_index: usize,
    pub target_accepted_steps_before: usize,
    pub quadratic_drift_zeta34: Option<f64>,
    pub zeta34_signed_margin: Option<f64>,
    pub recommended: bool,
    pub shadow_full_e_completed: bool,
    pub shadow_full_e_locally_admissible: bool,
    pub audit_unsafe: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct A1ToleranceReceiptRecommendationRow {
    pub event_key: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct A1ToleranceReceiptCell {
    pub schema: &'static str,
    #[serde(flatten)]
    pub scientific_execution_identity: A1ScientificExecutionIdentity,
    pub profile: &'static str,
    pub family: &'static str,
    pub arm: &'static str,
    pub outer_rtol: f64,
    pub linear_rtol: f64,
    pub linear_atol: f64,
    pub phi_relative_tolerance: f64,
    pub phi_absolute_tolerance: f64,
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub rhs_evaluations: u64,
    pub jvp_vectors: u64,
    pub linear_matvecs: u64,
    pub trace_digest: String,
    pub switching_active: bool,
    pub frozen_zeta34_tau: f64,
    pub event_rows: Vec<A1ToleranceReceiptEventRow>,
    pub recommendation_rows: Vec<A1ToleranceReceiptRecommendationRow>,
    pub hard_gates: G4S5B0FrozenFullEShadowHardGates,
    pub limitations: Vec<String>,
}

/// Generate one deterministic receipt-only A1 replay cell.
///
/// This is the sole API that admits `OuterScaledNumericParity`. It is fixed to
/// `EnforcedBudgetHoldout320`, never switches methods, and does not alter the
/// ordinary committed runtime arm.
pub fn run_a1_two_arm_receipt_cell(
    scientific_execution_identity: A1ScientificExecutionIdentity,
    family: G4S5B0Family,
    arm: G4S5B0LinearToleranceArm,
) -> CoreResult<A1ToleranceReceiptCell> {
    scientific_execution_identity.validate()?;
    let report = run_g4_s5b0_frozen_full_e_shadow_receipt_family(family, arm)?;
    let (_, outer_rtol) = A1_TWO_ARM_RECEIPT_PROFILE.tolerances();
    let tolerance = G4S5B0InnerTolerancePolicy::try_for_lane(
        G4S5B0InnerToleranceLane::FrozenFullEShadow,
        arm,
        outer_rtol,
    )?;

    let trace = G4S5B0AttemptTraceReport {
        schema: "g4-s5b0-rjf-attempt-trace-v1",
        status: "read-only-rjf-attempt-trace",
        profile: A1_TWO_ARM_RECEIPT_PROFILE.as_str(),
        switching_active: false,
        committed_method: "protected-sequential-matrix-free-rodas5p",
        attempt_rows: report.attempt_rows.clone(),
        accepted_rows: report.accepted_rows.clone(),
        trajectories: report.trajectories.clone(),
        limitations: Vec::new(),
    };

    let attempts = report.attempt_rows.len();
    let accepted_steps = report
        .attempt_rows
        .iter()
        .filter(|row| row.accepted)
        .count();
    let rejected_steps = attempts.saturating_sub(accepted_steps);
    let rhs_evaluations = report
        .attempt_rows
        .iter()
        .map(|row| row.rhs_evaluations)
        .sum();
    let jvp_vectors = report.attempt_rows.iter().map(|row| row.jvp_vectors).sum();
    let linear_matvecs = report
        .attempt_rows
        .iter()
        .map(|row| row.linear_matvecs)
        .sum();

    let mut event_rows = report
        .rows
        .iter()
        .map(|row| {
            let zeta34 = row.quadratic_drift_zeta34.filter(|value| value.is_finite());
            let event_key = format!(
                "{}:{}:{}:{}",
                row.trajectory_id,
                row.decision_accepted_step,
                row.target_attempt_index,
                row.target_accepted_steps_before
            );
            A1ToleranceReceiptEventRow {
                event_key,
                trajectory_id: row.trajectory_id.clone(),
                decision_accepted_step: row.decision_accepted_step,
                target_attempt_index: row.target_attempt_index,
                target_accepted_steps_before: row.target_accepted_steps_before,
                quadratic_drift_zeta34: zeta34,
                zeta34_signed_margin: zeta34.map(|value| value - V36_FROZEN_ZETA34_TAU),
                recommended: row.recommended,
                shadow_full_e_completed: row.shadow_full_e_completed,
                shadow_full_e_locally_admissible: row.shadow_full_e_locally_admissible,
                audit_unsafe: row.shadow_full_e_completed && !row.shadow_full_e_locally_admissible,
            }
        })
        .collect::<Vec<_>>();
    event_rows.sort_by(|left, right| left.event_key.cmp(&right.event_key));
    let recommendation_rows = event_rows
        .iter()
        .filter(|row| row.recommended)
        .map(|row| A1ToleranceReceiptRecommendationRow {
            event_key: row.event_key.clone(),
        })
        .collect();

    let mut limitations = report.limitations;
    limitations.push(
        "The outer-scaled arm matches preserved phi tolerance numbers only; this receipt makes no equal-error-contribution claim."
            .into(),
    );
    limitations.push(
        "Wall time, ranking, speedup, active switching, and candidate activation are outside this receipt."
            .into(),
    );

    Ok(A1ToleranceReceiptCell {
        schema: A1_TWO_ARM_RECEIPT_SCHEMA,
        scientific_execution_identity,
        profile: A1_TWO_ARM_RECEIPT_PROFILE.as_str(),
        family: family.as_str(),
        arm: arm.as_str(),
        outer_rtol,
        linear_rtol: tolerance.linear_relative_tolerance(),
        linear_atol: tolerance.linear_absolute_tolerance(),
        phi_relative_tolerance: tolerance.phi_relative_tolerance(),
        phi_absolute_tolerance: tolerance.phi_absolute_tolerance(),
        attempts,
        accepted_steps,
        rejected_steps,
        rhs_evaluations,
        jvp_vectors,
        linear_matvecs,
        trace_digest: g4_s5b0_rjf_trace_digest(&trace),
        switching_active: false,
        frozen_zeta34_tau: V36_FROZEN_ZETA34_TAU,
        event_rows,
        recommendation_rows,
        hard_gates: report.hard_gates,
        limitations,
    })
}
