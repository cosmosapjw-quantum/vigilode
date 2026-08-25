use rodas5p_core::{CoreError, CoreResult, sha256_hex};

use crate::{
    G4S5B0AttemptTraceReport, G4S5B0Family, G4S5B0LinearToleranceArm, G4S5B0Profile,
    committed_g4_s5b0_linear_tolerance_arm, run_g4_s5b0_rjf_attempt_trace_family,
};

/// Run the currently committed G4/S5B0 R-JF attempt trace through an explicit
/// tolerance-arm boundary.
///
/// `OuterScaledNumericParity` remains intentionally unavailable here until the
/// dedicated two-arm authority runner is implemented. Rejecting it prevents a
/// caller from treating an unreceipted trajectory generator as production
/// authority during the compile-closure phase.
pub fn run_g4_s5b0_rjf_attempt_trace_family_with_linear_tolerance_arm(
    profile: G4S5B0Profile,
    family: G4S5B0Family,
    arm: G4S5B0LinearToleranceArm,
) -> CoreResult<G4S5B0AttemptTraceReport> {
    let committed = committed_g4_s5b0_linear_tolerance_arm();
    if arm != committed {
        return Err(CoreError::InvalidInput(format!(
            "G4/S5B0 runtime arm '{}' is not authority-enabled; committed arm is '{}'",
            arm.as_str(),
            committed.as_str()
        )));
    }
    run_g4_s5b0_rjf_attempt_trace_family(profile, family)
}

/// Canonical SHA-256 digest of the deterministic, load-bearing R-JF trace.
///
/// Wall-clock fields and prose limitations are intentionally excluded. The
/// encoding is explicitly tagged, length-prefixed, ordered, and uses IEEE-754
/// bit patterns for every floating-point value.
pub fn g4_s5b0_rjf_trace_digest(report: &G4S5B0AttemptTraceReport) -> String {
    let mut bytes = Vec::new();
    push_str(&mut bytes, "g4-s5b0-rjf-trace-digest-v1");
    push_str(&mut bytes, report.schema);
    push_str(&mut bytes, report.status);
    push_str(&mut bytes, report.profile);
    push_bool(&mut bytes, report.switching_active);
    push_str(&mut bytes, report.committed_method);

    push_usize(&mut bytes, report.attempt_rows.len());
    for row in &report.attempt_rows {
        push_str(&mut bytes, &row.trajectory_id);
        push_str(&mut bytes, &row.family);
        push_usize(&mut bytes, row.dimension);
        push_f64(&mut bytes, row.rtol);
        push_usize(&mut bytes, row.attempt_index);
        push_usize(&mut bytes, row.accepted_steps_before);
        push_f64(&mut bytes, row.t_start);
        push_f64(&mut bytes, row.h);
        push_option_f64(&mut bytes, row.error_norm);
        push_bool(&mut bytes, row.accepted);
        push_bool(&mut bytes, row.recoverable_failure);
        push_option_str(&mut bytes, row.failure.as_deref());
        push_u64(&mut bytes, row.rhs_evaluations);
        push_u64(&mut bytes, row.jvp_vectors);
        push_u64(&mut bytes, row.linear_matvecs);
    }

    push_usize(&mut bytes, report.accepted_rows.len());
    for row in &report.accepted_rows {
        push_str(&mut bytes, &row.trajectory_id);
        push_str(&mut bytes, &row.family);
        push_usize(&mut bytes, row.dimension);
        push_f64(&mut bytes, row.rtol);
        push_usize(&mut bytes, row.step_index);
        push_f64(&mut bytes, row.t_start);
        push_f64(&mut bytes, row.h);
        push_f64(&mut bytes, row.transition_level);
        push_f64(&mut bytes, row.rodas_embedded_error);
        push_u64(&mut bytes, row.rodas_rhs_evaluations);
        push_u64(&mut bytes, row.rodas_jvp_vectors);
        push_u64(&mut bytes, row.rodas_linear_matvecs);
        push_bool(&mut bytes, row.exponential_completed);
        push_option_f64(&mut bytes, row.exponential_total_error);
        push_bool(&mut bytes, row.exponential_locally_admissible);
        push_option_u64(&mut bytes, row.exponential_rhs_evaluations);
        push_option_u64(&mut bytes, row.exponential_jvp_vectors);
        push_option_usize(&mut bytes, row.exponential_maximum_krylov_dimension);
        push_option_usize(&mut bytes, row.exponential_phi_substeps);
        push_option_str(&mut bytes, row.exponential_failure.as_deref());
    }

    push_usize(&mut bytes, report.trajectories.len());
    for trajectory in &report.trajectories {
        push_str(&mut bytes, &trajectory.trajectory_id);
        push_str(&mut bytes, &trajectory.family);
        push_usize(&mut bytes, trajectory.dimension);
        push_f64(&mut bytes, trajectory.rtol);
        push_bool(&mut bytes, trajectory.success);
        push_option_str(&mut bytes, trajectory.failure.as_deref());
        push_usize(&mut bytes, trajectory.attempts);
        push_usize(&mut bytes, trajectory.accepted_steps);
        push_usize(&mut bytes, trajectory.rejected_steps);
        push_f64(&mut bytes, trajectory.endpoint_time);
        push_u64(&mut bytes, trajectory.explicit_jacobian_builds);
        push_u64(&mut bytes, trajectory.direct_factorizations);
        push_u64(&mut bytes, trajectory.newton_iterations);
    }

    sha256_hex(&bytes)
}

fn push_bool(bytes: &mut Vec<u8>, value: bool) {
    bytes.push(u8::from(value));
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_usize(bytes: &mut Vec<u8>, value: usize) {
    push_u64(bytes, value as u64);
}

fn push_f64(bytes: &mut Vec<u8>, value: f64) {
    push_u64(bytes, value.to_bits());
}

fn push_str(bytes: &mut Vec<u8>, value: &str) {
    push_usize(bytes, value.len());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_option_f64(bytes: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(value) => {
            bytes.push(1);
            push_f64(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn push_option_u64(bytes: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            bytes.push(1);
            push_u64(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn push_option_usize(bytes: &mut Vec<u8>, value: Option<usize>) {
    match value {
        Some(value) => {
            bytes.push(1);
            push_usize(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn push_option_str(bytes: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            bytes.push(1);
            push_str(bytes, value);
        }
        None => bytes.push(0),
    }
}
