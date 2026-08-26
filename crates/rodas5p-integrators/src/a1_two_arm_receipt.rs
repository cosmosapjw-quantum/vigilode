use std::collections::BTreeMap;

use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use serde::{Deserialize, Serialize};

use crate::{
    G4S5B0AttemptTraceReport, G4S5B0Family, G4S5B0FrozenFullEShadowHardGates,
    G4S5B0InnerToleranceLane, G4S5B0InnerTolerancePolicy, G4S5B0LinearToleranceArm, G4S5B0Profile,
    V36_FROZEN_ZETA34_TAU,
    g4_s5b0_regime_atlas::{
        run_g4_s5b0_frozen_full_e_shadow_receipt_family,
        run_g4_s5b0_stage_growth_safety_receipt_audit_family,
    },
    g4_s5b0_rjf_trace_digest,
};

pub const A1_TWO_ARM_RECEIPT_SCHEMA: &str = "vigilode-a1-two-arm-atomic-cell-v2";
pub const A1_TWO_ARM_RECEIPT_PROFILE: G4S5B0Profile = G4S5B0Profile::EnforcedBudgetHoldout320;
const INVALIDATED_EXECUTION_WORKFLOW_RUN_ID: u64 = 32_906_175_896;

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
        if self.execution_workflow_run_id == INVALIDATED_EXECUTION_WORKFLOW_RUN_ID {
            return Err(CoreError::InvalidInput(
                "A1 receipt workflow run 32906175896 is diagnostic-only and invalid for authority"
                    .into(),
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
    pub t_start: f64,
    pub h: f64,
    pub quadratic_drift_zeta34: Option<f64>,
    pub zeta34_signed_margin: Option<f64>,
    pub recommended: bool,
    pub shadow_full_e_completed: bool,
    pub shadow_full_e_locally_admissible: bool,
    pub audit_arm: String,
    pub audit_family: String,
    pub audit_event_key: String,
    pub audit_full_e_eligible: bool,
    pub audit_full_e_attempted: bool,
    pub audit_full_e_completed: bool,
    pub audit_full_e_total_error: Option<f64>,
    pub audit_full_e_locally_admissible: Option<bool>,
    pub audit_full_e_failure: Option<String>,
    pub audit_full_e_work: Option<WorkCounters>,
    pub audit_unsafe: Option<bool>,
    pub audit_evidence_status: String,
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
    let audit_report = run_g4_s5b0_stage_growth_safety_receipt_audit_family(family, arm)?;
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
    let audit_trace = G4S5B0AttemptTraceReport {
        schema: "g4-s5b0-rjf-attempt-trace-v1",
        status: "read-only-rjf-attempt-trace",
        profile: A1_TWO_ARM_RECEIPT_PROFILE.as_str(),
        switching_active: false,
        committed_method: "protected-sequential-matrix-free-rodas5p",
        attempt_rows: audit_report.attempt_rows.clone(),
        accepted_rows: audit_report.accepted_rows.clone(),
        trajectories: audit_report.trajectories.clone(),
        limitations: Vec::new(),
    };
    let trace_digest = g4_s5b0_rjf_trace_digest(&trace);
    let audit_trace_digest = g4_s5b0_rjf_trace_digest(&audit_trace);
    if audit_trace_digest != trace_digest {
        return Err(CoreError::InvalidInput(format!(
            "A1 audit R-JF trace identity mismatch for arm {} family {}",
            arm.as_str(),
            family.as_str()
        )));
    }

    let mut audit_rows = BTreeMap::new();
    for row in &audit_report.rows {
        let key = format!(
            "{}:{}:{}:{}",
            row.trajectory_id,
            row.decision_accepted_step,
            row.target_attempt_index,
            row.target_accepted_steps_before
        );
        if audit_rows.insert(key.clone(), row).is_some() {
            return Err(CoreError::InvalidInput(format!(
                "duplicate A1 audit event identity {key} for arm {} family {}",
                arm.as_str(),
                family.as_str()
            )));
        }
    }

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
        .map(|row| -> CoreResult<A1ToleranceReceiptEventRow> {
            let zeta34 = row.quadratic_drift_zeta34.filter(|value| value.is_finite());
            let event_key = format!(
                "{}:{}:{}:{}",
                row.trajectory_id,
                row.decision_accepted_step,
                row.target_attempt_index,
                row.target_accepted_steps_before
            );
            let audit = audit_rows.remove(&event_key).ok_or_else(|| {
                CoreError::InvalidInput(format!(
                    "missing A1 audit event identity {event_key} for arm {} family {}",
                    arm.as_str(),
                    family.as_str()
                ))
            })?;
            let aligned = audit.family == family.as_str()
                && audit.trajectory_id == row.trajectory_id
                && audit.decision_accepted_step == row.decision_accepted_step
                && audit.target_attempt_index == row.target_attempt_index
                && audit.target_accepted_steps_before == row.target_accepted_steps_before
                && audit.t_start.to_bits() == row.t_start.to_bits()
                && audit.h.to_bits() == row.h.to_bits()
                && audit.feature_value.map(f64::to_bits) == row.feature_value.map(f64::to_bits)
                && audit.quadratic_drift_zeta34.map(f64::to_bits)
                    == row.quadratic_drift_zeta34.map(f64::to_bits)
                && audit.target_r_attempt_accepted == row.target_r_attempt_accepted
                && audit.target_r_error_norm.map(f64::to_bits)
                    == row.target_r_error_norm.map(f64::to_bits)
                && audit.committed_rjf_jvp_before_target == row.committed_rjf_jvp_before_target
                && audit.speculative_jvp_before_target
                    == row.prefix_speculative_jvp_before_target
                && audit.budget_cap_jvp == row.budget_cap_jvp
                && audit.budget_fraction.to_bits() == row.budget_fraction.to_bits()
                && audit.budget_admitted == row.budget_admitted
                && audit.budget_exhausted == row.budget_exhausted
                && audit.budget_breached == row.budget_breached
                && audit.prefix_succeeded == row.prefix_succeeded
                && audit.actual_prefix_jvp_vectors == row.actual_prefix_jvp_vectors
                && audit.prefix_work == row.prefix_work;
            if !aligned {
                return Err(CoreError::InvalidInput(format!(
                    "A1 audit arm/family/event state or budget mismatch for arm {} family {} event {event_key}",
                    arm.as_str(),
                    family.as_str()
                )));
            }

            let (audit_full_e_eligible, audit_full_e_attempted, audit_evidence_status) =
                if audit.audit_full_e_completed {
                    (true, true, "complete")
                } else if audit.prefix_succeeded || audit.audit_full_e_failure.is_some() {
                    (true, true, "failed")
                } else {
                    (false, false, "ineligible")
                };
            let audit_full_e_failure = if audit.audit_full_e_completed {
                None
            } else if let Some(failure) = &audit.audit_full_e_failure {
                Some(failure.clone())
            } else if audit.budget_exhausted {
                Some("audit-ineligible-prefix-budget-exhausted".into())
            } else if !audit.budget_admitted {
                Some("audit-ineligible-prefix-budget-denied".into())
            } else if let Some(failure) = &audit.prefix_failure {
                Some(format!("audit-ineligible-prefix-failure: {failure}"))
            } else {
                Some("audit-full-e-incomplete-without-explicit-solver-result".into())
            };
            let audit_full_e_locally_admissible = audit
                .audit_full_e_completed
                .then_some(audit.audit_full_e_locally_admissible);
            let audit_unsafe = audit_full_e_locally_admissible.map(|admissible| !admissible);
            Ok(A1ToleranceReceiptEventRow {
                event_key: event_key.clone(),
                trajectory_id: row.trajectory_id.clone(),
                decision_accepted_step: row.decision_accepted_step,
                target_attempt_index: row.target_attempt_index,
                target_accepted_steps_before: row.target_accepted_steps_before,
                t_start: row.t_start,
                h: row.h,
                quadratic_drift_zeta34: zeta34,
                zeta34_signed_margin: zeta34.map(|value| value - V36_FROZEN_ZETA34_TAU),
                recommended: row.recommended,
                shadow_full_e_completed: row.shadow_full_e_completed,
                shadow_full_e_locally_admissible: row.shadow_full_e_locally_admissible,
                audit_arm: arm.as_str().into(),
                audit_family: family.as_str().into(),
                audit_event_key: event_key,
                audit_full_e_eligible,
                audit_full_e_attempted,
                audit_full_e_completed: audit.audit_full_e_completed,
                audit_full_e_total_error: audit.audit_full_e_total_error,
                audit_full_e_locally_admissible,
                audit_full_e_failure,
                audit_full_e_work: audit.audit_full_e_work,
                audit_unsafe,
                audit_evidence_status: audit_evidence_status.into(),
            })
        })
        .collect::<CoreResult<Vec<_>>>()?;
    if let Some(extra) = audit_rows.keys().next() {
        return Err(CoreError::InvalidInput(format!(
            "extra A1 audit event identity {extra} for arm {} family {}",
            arm.as_str(),
            family.as_str()
        )));
    }
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
        trace_digest,
        switching_active: false,
        frozen_zeta34_tau: V36_FROZEN_ZETA34_TAU,
        event_rows,
        recommendation_rows,
        hard_gates: report.hard_gates,
        limitations,
    })
}
