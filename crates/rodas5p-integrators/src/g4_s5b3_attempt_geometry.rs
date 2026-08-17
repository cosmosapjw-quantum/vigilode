use std::time::Instant;

use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use serde::{Deserialize, Serialize};

use crate::g3_fused_adaptive_gate::{RuntimeProblem, adaptive_config, build_problems, phi_config};
use crate::{
    AdaptiveEarlyFlowDefectAttempt, AdaptiveEarlyFlowDefectOutcome, AdaptiveFusedExponentialResult,
    EarlyFlowDefectDiagnosticWork, EarlyFlowDefectTelemetryMode, FusedOrthogonalization,
    FusedPhiKrylovConfig, G3FusedAdaptiveProfile, OdeProblem, OutputSchedule, ParallelExecution,
    integrate_pexprb54s4_fused_adaptive_observed_with_telemetry_mode,
    integrate_pexprb54s4_fused_adaptive_observed_with_tolerance_scaled_telemetry,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum G4S5B3Profile {
    Smoke,
    Canonical,
}

impl G4S5B3Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Canonical => "canonical",
        }
    }

    fn g3_profile(self) -> G3FusedAdaptiveProfile {
        match self {
            Self::Smoke => G3FusedAdaptiveProfile::Smoke,
            Self::Canonical => G3FusedAdaptiveProfile::Canonical,
        }
    }

    fn rtols(self) -> &'static [f64] {
        match self {
            Self::Smoke => &[1e-5],
            Self::Canonical => &[1e-4, 1e-6, 1e-8],
        }
    }

    fn overhead_protocol(self) -> OverheadProtocol {
        match self {
            Self::Smoke => OverheadProtocol {
                warmup_pairs: 0,
                measured_pairs: 1,
                minimum_wall_seconds: 0.0,
                maximum_repetitions: 1,
            },
            Self::Canonical => OverheadProtocol {
                warmup_pairs: 1,
                measured_pairs: 7,
                minimum_wall_seconds: 0.25,
                maximum_repetitions: 1024,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B3AttemptRow {
    pub trajectory_id: String,
    pub problem_id: String,
    pub rtol: f64,
    pub atol: f64,
    pub attempt_index: usize,
    pub t: f64,
    pub step_size: f64,
    pub abs_step_size: f64,
    pub output_clipped: bool,
    pub outcome: AdaptiveEarlyFlowDefectOutcome,
    pub eta_c2: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rho_c2_wrms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_scale_atol: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_scale_rtol: Option<f64>,
    pub stage_fraction: Option<f64>,
    pub state_dimension: Option<usize>,
    pub norm_component_count: Option<usize>,
    pub excluded_trailing_components: Option<usize>,
    pub stage_increment_l2: Option<f64>,
    pub nonlinear_remainder_l2: Option<f64>,
    pub zero_increment: Option<bool>,
    pub degenerate_nonzero_remainder: Option<bool>,
    pub nonfinite_normalization: Option<bool>,
    pub native_partial_t_sampled: Option<bool>,
    pub diagnostic_work: Option<EarlyFlowDefectDiagnosticWork>,
    pub time_error_norm: Option<f64>,
    pub phi_error_norm: Option<f64>,
    pub total_error_norm: Option<f64>,
    pub candidate_state_finite: Option<bool>,
    pub maximum_krylov_dimension: Option<usize>,
    pub phi_substeps: Option<usize>,
    pub trial_work: Option<WorkCounters>,
    pub failure: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B3TrajectorySummary {
    pub trajectory_id: String,
    pub problem_id: String,
    pub rtol: f64,
    pub atol: f64,
    pub physical_dimension: usize,
    pub integrated_dimension: usize,
    pub success: bool,
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub unscorable_attempts: usize,
    pub legacy_unclassified_attempts: usize,
    pub finite_eta_attempts: usize,
    pub null_eta_attempts: usize,
    pub degenerate_attempts: usize,
    pub nonfinite_normalization_attempts: usize,
    pub sum_abs_trial_step_size: f64,
    pub requested_outputs_bitwise_identical: bool,
    pub controller_histories_bitwise_identical: bool,
    pub accepted_rejected_sequences_identical: bool,
    pub existing_work_counters_identical: bool,
    pub per_trial_work_complete: bool,
    pub per_trial_work_matches_aggregate: bool,
    pub diagnostic_expensive_work_zero: bool,
    pub complete_attempt_coverage: bool,
    pub aggregate_work: WorkCounters,
    pub diagnostic_work: EarlyFlowDefectDiagnosticWork,
    pub all_required_gates_pass: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct G4S5B3HardGateSummary {
    pub expected_trajectories: usize,
    pub observed_trajectories: usize,
    pub all_trajectories_successful: bool,
    pub requested_outputs_bitwise_identical: bool,
    pub controller_histories_bitwise_identical: bool,
    pub accepted_rejected_sequences_identical: bool,
    pub existing_work_counters_identical: bool,
    pub per_trial_work_complete: bool,
    pub per_trial_work_matches_aggregate: bool,
    pub no_unscorable_attempts: bool,
    pub no_legacy_unclassified_attempts: bool,
    pub diagnostic_expensive_work_zero: bool,
    pub complete_attempt_coverage: bool,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B3CalibrationRow {
    pub repetitions: usize,
    pub wall_seconds: f64,
    pub proposed_interval: f64,
    pub gamma_seconds_per_interval: f64,
    pub all_suite_identities_passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B3OverheadArm {
    pub mode: String,
    pub repetitions: usize,
    pub wall_seconds: f64,
    pub proposed_interval: f64,
    pub gamma_seconds_per_interval: f64,
    pub all_suite_identities_passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B3OverheadPair {
    pub pair_index: usize,
    pub order: String,
    pub disabled: G4S5B3OverheadArm,
    pub read_only: G4S5B3OverheadArm,
    pub wall_ratio_read_only_over_disabled: f64,
    pub gamma_ratio_read_only_over_disabled: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B3OverheadReport {
    pub warmup_pairs: usize,
    pub measured_pairs: usize,
    pub frozen_repetitions: usize,
    pub minimum_calibration_wall_seconds: f64,
    pub maximum_calibration_repetitions: usize,
    pub calibration_rows: Vec<G4S5B3CalibrationRow>,
    pub warmup_rows: Vec<G4S5B3OverheadPair>,
    pub measured_rows: Vec<G4S5B3OverheadPair>,
    pub all_suite_identities_passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B3AttemptGeometryReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub contract_path: &'static str,
    pub active_switching: bool,
    pub early_abort: bool,
    pub threshold_selected: bool,
    pub selected_threshold: Option<f64>,
    pub orthogonalization: &'static str,
    pub threads: usize,
    pub requested_output: &'static str,
    pub attempts: Vec<G4S5B3AttemptRow>,
    pub trajectories: Vec<G4S5B3TrajectorySummary>,
    pub hard_gates: G4S5B3HardGateSummary,
    pub overhead: Option<G4S5B3OverheadReport>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Copy)]
struct OverheadProtocol {
    warmup_pairs: usize,
    measured_pairs: usize,
    minimum_wall_seconds: f64,
    maximum_repetitions: usize,
}

struct PreparedCase {
    trajectory_id: String,
    problem_id: String,
    rtol: f64,
    atol: f64,
    physical_dimension: usize,
    problem: OdeProblem,
    y0: Vec<f64>,
    t_span: (f64, f64),
    output: OutputSchedule,
    adaptive: crate::AdaptiveStepConfig,
    phi: FusedPhiKrylovConfig,
}

struct TrajectoryExecution {
    case_index: usize,
    result: AdaptiveFusedExponentialResult,
}

struct SuiteExecution {
    trajectories: Vec<TrajectoryExecution>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SuiteTelemetryMode {
    Disabled,
    RawReadOnly,
    ToleranceScaledReadOnly,
}

impl SuiteTelemetryMode {
    fn label(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::RawReadOnly => "read-only",
            Self::ToleranceScaledReadOnly => "tolerance-scaled-read-only",
        }
    }
}

fn prepare_cases(profile: G4S5B3Profile) -> CoreResult<Vec<PreparedCase>> {
    let runtimes = build_problems(profile.g3_profile())?;
    let mut cases = Vec::new();
    for runtime in runtimes {
        for &rtol in profile.rtols() {
            cases.push(prepare_case(&runtime, rtol)?);
        }
    }
    Ok(cases)
}

fn prepare_case(runtime: &RuntimeProblem, rtol: f64) -> CoreResult<PreparedCase> {
    let physical_dimension = runtime.problem.dimension;
    let (problem, y0) = if runtime.problem.autonomous {
        (runtime.problem.jvp_only_clone()?, runtime.y0.clone())
    } else {
        let problem = runtime.problem.jvp_only_clone()?.time_augmented_clone()?;
        let mut y0 = runtime.y0.clone();
        y0.push(runtime.t_span.0);
        (problem, y0)
    };
    let span = runtime.t_span.1 - runtime.t_span.0;
    let output = OutputSchedule::new(vec![runtime.t_span.0, runtime.t_span.1])?;
    let adaptive = adaptive_config(rtol, span);
    let phi = phi_config(rtol, FusedOrthogonalization::FullMgs, problem.dimension + 4);
    Ok(PreparedCase {
        trajectory_id: format!("{}|rtol={rtol:.0e}", runtime.id),
        problem_id: runtime.id.clone(),
        rtol,
        atol: 0.01 * rtol,
        physical_dimension,
        problem,
        y0,
        t_span: runtime.t_span,
        output,
        adaptive,
        phi,
    })
}

fn execute_suite(cases: &[PreparedCase], mode: SuiteTelemetryMode) -> CoreResult<SuiteExecution> {
    let execution = ParallelExecution::sequential();
    let mut trajectories = Vec::with_capacity(cases.len());
    for (case_index, case) in cases.iter().enumerate() {
        let result = match mode {
            SuiteTelemetryMode::Disabled => {
                integrate_pexprb54s4_fused_adaptive_observed_with_telemetry_mode(
                    &case.problem,
                    case.t_span,
                    &case.y0,
                    &case.adaptive,
                    &case.output,
                    case.phi,
                    &execution,
                    EarlyFlowDefectTelemetryMode::Disabled,
                )?
            }
            SuiteTelemetryMode::RawReadOnly => {
                integrate_pexprb54s4_fused_adaptive_observed_with_telemetry_mode(
                    &case.problem,
                    case.t_span,
                    &case.y0,
                    &case.adaptive,
                    &case.output,
                    case.phi,
                    &execution,
                    EarlyFlowDefectTelemetryMode::ReadOnly {
                        norm_component_count: case.physical_dimension,
                    },
                )?
            }
            SuiteTelemetryMode::ToleranceScaledReadOnly => {
                integrate_pexprb54s4_fused_adaptive_observed_with_tolerance_scaled_telemetry(
                    &case.problem,
                    case.t_span,
                    &case.y0,
                    &case.adaptive,
                    &case.output,
                    case.phi,
                    &execution,
                    case.physical_dimension,
                )?
            }
        };
        trajectories.push(TrajectoryExecution { case_index, result });
    }
    Ok(SuiteExecution { trajectories })
}

fn same_f64_bits(left: &[f64], right: &[f64]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| a.to_bits() == b.to_bits())
}

fn same_nested_f64_bits(left: &[Vec<f64>], right: &[Vec<f64>]) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(a, b)| same_f64_bits(a, b))
}

fn sorted_interval_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut values = values.into_iter().map(f64::abs).collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    values.into_iter().sum()
}

fn standard_interval_sum(result: &AdaptiveFusedExponentialResult) -> f64 {
    sorted_interval_sum(
        result
            .diagnostics
            .accepted_step_sizes
            .iter()
            .chain(&result.diagnostics.rejected_step_sizes)
            .copied(),
    )
}

fn attempt_interval_sum(attempts: &[AdaptiveEarlyFlowDefectAttempt]) -> f64 {
    sorted_interval_sum(attempts.iter().map(|attempt| attempt.step_size))
}

fn sum_diagnostic_work(
    attempts: &[AdaptiveEarlyFlowDefectAttempt],
) -> EarlyFlowDefectDiagnosticWork {
    let mut out = EarlyFlowDefectDiagnosticWork::default();
    for work in attempts
        .iter()
        .filter_map(|attempt| attempt.telemetry.as_ref())
        .map(|telemetry| telemetry.diagnostic_work)
    {
        out.conceptual_vector_differences = out
            .conceptual_vector_differences
            .saturating_add(work.conceptual_vector_differences);
        out.l2_norm_evaluations = out
            .l2_norm_evaluations
            .saturating_add(work.l2_norm_evaluations);
        out.scalar_normalizations = out
            .scalar_normalizations
            .saturating_add(work.scalar_normalizations);
        out.component_scale_evaluations = out
            .component_scale_evaluations
            .saturating_add(work.component_scale_evaluations);
        out.wrms_norm_evaluations = out
            .wrms_norm_evaluations
            .saturating_add(work.wrms_norm_evaluations);
        out.added_rhs_calls = out.added_rhs_calls.saturating_add(work.added_rhs_calls);
        out.added_jvp_calls = out.added_jvp_calls.saturating_add(work.added_jvp_calls);
        out.added_jvp_vectors = out.added_jvp_vectors.saturating_add(work.added_jvp_vectors);
        out.added_phi_actions = out.added_phi_actions.saturating_add(work.added_phi_actions);
        out.added_partial_t_calls = out
            .added_partial_t_calls
            .saturating_add(work.added_partial_t_calls);
        out.added_jacobian_builds = out
            .added_jacobian_builds
            .saturating_add(work.added_jacobian_builds);
        out.added_newton_iterations = out
            .added_newton_iterations
            .saturating_add(work.added_newton_iterations);
    }
    out
}

fn diagnostic_expensive_work_zero(work: EarlyFlowDefectDiagnosticWork) -> bool {
    work.added_rhs_calls == 0
        && work.added_jvp_calls == 0
        && work.added_jvp_vectors == 0
        && work.added_phi_actions == 0
        && work.added_partial_t_calls == 0
        && work.added_jacobian_builds == 0
        && work.added_newton_iterations == 0
}

fn compare_trajectory(
    case: &PreparedCase,
    disabled: &AdaptiveFusedExponentialResult,
    read_only: &AdaptiveFusedExponentialResult,
) -> G4S5B3TrajectorySummary {
    let requested_outputs_bitwise_identical =
        same_f64_bits(&disabled.observed.t, &read_only.observed.t)
            && same_nested_f64_bits(&disabled.observed.y, &read_only.observed.y)
            && disabled.observed.success == read_only.observed.success
            && disabled.observed.message == read_only.observed.message
            && disabled.observed.internal_steps == read_only.observed.internal_steps
            && disabled.observed.output_clipped_steps == read_only.observed.output_clipped_steps;
    let controller_histories_bitwise_identical = disabled.diagnostics.attempts
        == read_only.diagnostics.attempts
        && disabled.diagnostics.accepted_steps == read_only.diagnostics.accepted_steps
        && disabled.diagnostics.rejected_steps == read_only.diagnostics.rejected_steps
        && same_f64_bits(
            &disabled.diagnostics.accepted_step_sizes,
            &read_only.diagnostics.accepted_step_sizes,
        )
        && same_f64_bits(
            &disabled.diagnostics.rejected_step_sizes,
            &read_only.diagnostics.rejected_step_sizes,
        )
        && same_f64_bits(
            &disabled.diagnostics.time_error_norms,
            &read_only.diagnostics.time_error_norms,
        )
        && same_f64_bits(
            &disabled.diagnostics.phi_error_norms,
            &read_only.diagnostics.phi_error_norms,
        )
        && same_f64_bits(
            &disabled.diagnostics.total_error_norms,
            &read_only.diagnostics.total_error_norms,
        )
        && disabled.diagnostics.maximum_krylov_dimensions
            == read_only.diagnostics.maximum_krylov_dimensions
        && disabled.diagnostics.phi_substeps == read_only.diagnostics.phi_substeps;
    let attempts = &read_only.diagnostics.early_flow_defect_attempts;
    let accepted_from_attempts = attempts
        .iter()
        .filter(|row| row.outcome == AdaptiveEarlyFlowDefectOutcome::Accepted)
        .map(|row| row.step_size)
        .collect::<Vec<_>>();
    let rejected_from_attempts = attempts
        .iter()
        .filter(|row| row.outcome == AdaptiveEarlyFlowDefectOutcome::RejectedErrorControl)
        .map(|row| row.step_size)
        .collect::<Vec<_>>();
    let accepted_rejected_sequences_identical = attempts.len() == read_only.diagnostics.attempts
        && same_f64_bits(
            &accepted_from_attempts,
            &read_only.diagnostics.accepted_step_sizes,
        )
        && same_f64_bits(
            &rejected_from_attempts,
            &read_only.diagnostics.rejected_step_sizes,
        )
        && same_f64_bits(
            &disabled.diagnostics.accepted_step_sizes,
            &accepted_from_attempts,
        )
        && same_f64_bits(
            &disabled.diagnostics.rejected_step_sizes,
            &rejected_from_attempts,
        );
    let existing_work_counters_identical =
        disabled.observed.counters == read_only.observed.counters;
    let per_trial_work_complete = attempts.iter().all(|row| row.trial_work.is_some());
    let mut summed_trial_work = WorkCounters::default();
    for work in attempts.iter().filter_map(|row| row.trial_work) {
        summed_trial_work.accumulate(work);
    }
    let mut aggregate_numerical_work = read_only.observed.counters;
    aggregate_numerical_work.accepted_steps = 0;
    aggregate_numerical_work.rejected_steps = 0;
    let per_trial_work_matches_aggregate =
        per_trial_work_complete && summed_trial_work == aggregate_numerical_work;
    let unscorable_attempts = attempts
        .iter()
        .filter(|row| row.outcome == AdaptiveEarlyFlowDefectOutcome::TrialFailureUnscorable)
        .count();
    let legacy_unclassified_attempts = attempts
        .iter()
        .filter(|row| row.outcome == AdaptiveEarlyFlowDefectOutcome::LegacyUnclassified)
        .count();
    let finite_eta_attempts = attempts
        .iter()
        .filter(|row| {
            row.telemetry
                .as_ref()
                .and_then(|telemetry| telemetry.normalized_defect)
                .is_some_and(f64::is_finite)
        })
        .count();
    let null_eta_attempts = attempts.len().saturating_sub(finite_eta_attempts);
    let degenerate_attempts = attempts
        .iter()
        .filter(|row| {
            row.telemetry
                .as_ref()
                .is_some_and(|telemetry| telemetry.degenerate_nonzero_remainder)
        })
        .count();
    let nonfinite_normalization_attempts = attempts
        .iter()
        .filter(|row| {
            row.telemetry
                .as_ref()
                .is_some_and(|telemetry| telemetry.nonfinite_normalization)
        })
        .count();
    let diagnostic_work = sum_diagnostic_work(attempts);
    let diagnostic_expensive_work_zero = diagnostic_expensive_work_zero(diagnostic_work);
    let complete_attempt_coverage = attempts.len() == read_only.diagnostics.attempts
        && attempts.iter().all(|row| {
            row.failure.is_none()
                && row.telemetry.is_some()
                && row.time_error_norm.is_some()
                && row.phi_error_norm.is_some()
                && row.total_error_norm.is_some()
                && row.candidate_state_finite.is_some()
                && row.maximum_krylov_dimension.is_some()
                && row.phi_substeps.is_some()
                && row.trial_work.is_some()
        });
    let sum_abs_trial_step_size = attempt_interval_sum(attempts);
    let interval_match = sum_abs_trial_step_size.to_bits()
        == standard_interval_sum(read_only).to_bits()
        && sum_abs_trial_step_size.to_bits() == standard_interval_sum(disabled).to_bits();
    let all_required_gates_pass = disabled.observed.success
        && read_only.observed.success
        && requested_outputs_bitwise_identical
        && controller_histories_bitwise_identical
        && accepted_rejected_sequences_identical
        && existing_work_counters_identical
        && per_trial_work_complete
        && per_trial_work_matches_aggregate
        && unscorable_attempts == 0
        && legacy_unclassified_attempts == 0
        && diagnostic_expensive_work_zero
        && complete_attempt_coverage
        && interval_match;
    G4S5B3TrajectorySummary {
        trajectory_id: case.trajectory_id.clone(),
        problem_id: case.problem_id.clone(),
        rtol: case.rtol,
        atol: case.atol,
        physical_dimension: case.physical_dimension,
        integrated_dimension: case.problem.dimension,
        success: disabled.observed.success && read_only.observed.success,
        attempts: read_only.diagnostics.attempts,
        accepted_steps: read_only.diagnostics.accepted_steps,
        rejected_steps: read_only.diagnostics.rejected_steps,
        unscorable_attempts,
        legacy_unclassified_attempts,
        finite_eta_attempts,
        null_eta_attempts,
        degenerate_attempts,
        nonfinite_normalization_attempts,
        sum_abs_trial_step_size,
        requested_outputs_bitwise_identical,
        controller_histories_bitwise_identical,
        accepted_rejected_sequences_identical,
        existing_work_counters_identical,
        per_trial_work_complete,
        per_trial_work_matches_aggregate,
        diagnostic_expensive_work_zero,
        complete_attempt_coverage,
        aggregate_work: read_only.observed.counters,
        diagnostic_work,
        all_required_gates_pass,
    }
}

fn flatten_attempts(cases: &[PreparedCase], read_only: &SuiteExecution) -> Vec<G4S5B3AttemptRow> {
    let mut rows = Vec::new();
    for trajectory in &read_only.trajectories {
        let case = &cases[trajectory.case_index];
        for (attempt_index, attempt) in trajectory
            .result
            .diagnostics
            .early_flow_defect_attempts
            .iter()
            .enumerate()
        {
            let telemetry = attempt.telemetry.as_ref();
            rows.push(G4S5B3AttemptRow {
                trajectory_id: case.trajectory_id.clone(),
                problem_id: case.problem_id.clone(),
                rtol: case.rtol,
                atol: case.atol,
                attempt_index,
                t: attempt.t,
                step_size: attempt.step_size,
                abs_step_size: attempt.step_size.abs(),
                output_clipped: attempt.output_clipped,
                outcome: attempt.outcome,
                eta_c2: telemetry.and_then(|value| value.normalized_defect),
                rho_c2_wrms: telemetry.and_then(|value| value.tolerance_scaled_defect_wrms),
                tolerance_scale_atol: telemetry.and_then(|value| value.tolerance_scale_atol),
                tolerance_scale_rtol: telemetry.and_then(|value| value.tolerance_scale_rtol),
                stage_fraction: telemetry.map(|value| value.stage_fraction),
                state_dimension: telemetry.map(|value| value.state_dimension),
                norm_component_count: telemetry.map(|value| value.norm_component_count),
                excluded_trailing_components: telemetry
                    .map(|value| value.excluded_trailing_components),
                stage_increment_l2: telemetry.map(|value| value.stage_increment_l2),
                nonlinear_remainder_l2: telemetry.map(|value| value.nonlinear_remainder_l2),
                zero_increment: telemetry.map(|value| value.zero_increment),
                degenerate_nonzero_remainder: telemetry
                    .map(|value| value.degenerate_nonzero_remainder),
                nonfinite_normalization: telemetry.map(|value| value.nonfinite_normalization),
                native_partial_t_sampled: telemetry.map(|value| value.native_partial_t_sampled),
                diagnostic_work: telemetry.map(|value| value.diagnostic_work),
                time_error_norm: attempt.time_error_norm,
                phi_error_norm: attempt.phi_error_norm,
                total_error_norm: attempt.total_error_norm,
                candidate_state_finite: attempt.candidate_state_finite,
                maximum_krylov_dimension: attempt.maximum_krylov_dimension,
                phi_substeps: attempt.phi_substeps,
                trial_work: attempt.trial_work,
                failure: attempt.failure.clone(),
            });
        }
    }
    rows
}

fn hard_gate_summary(
    profile: G4S5B3Profile,
    trajectories: &[G4S5B3TrajectorySummary],
) -> G4S5B3HardGateSummary {
    let expected_trajectories = match profile {
        G4S5B3Profile::Smoke => 3,
        G4S5B3Profile::Canonical => 12,
    };
    let observed_trajectories = trajectories.len();
    let all_trajectories_successful = trajectories.iter().all(|row| row.success);
    let requested_outputs_bitwise_identical = trajectories
        .iter()
        .all(|row| row.requested_outputs_bitwise_identical);
    let controller_histories_bitwise_identical = trajectories
        .iter()
        .all(|row| row.controller_histories_bitwise_identical);
    let accepted_rejected_sequences_identical = trajectories
        .iter()
        .all(|row| row.accepted_rejected_sequences_identical);
    let existing_work_counters_identical = trajectories
        .iter()
        .all(|row| row.existing_work_counters_identical);
    let per_trial_work_complete = trajectories.iter().all(|row| row.per_trial_work_complete);
    let per_trial_work_matches_aggregate = trajectories
        .iter()
        .all(|row| row.per_trial_work_matches_aggregate);
    let no_unscorable_attempts = trajectories.iter().all(|row| row.unscorable_attempts == 0);
    let no_legacy_unclassified_attempts = trajectories
        .iter()
        .all(|row| row.legacy_unclassified_attempts == 0);
    let diagnostic_expensive_work_zero = trajectories
        .iter()
        .all(|row| row.diagnostic_expensive_work_zero);
    let complete_attempt_coverage = trajectories.iter().all(|row| row.complete_attempt_coverage);
    let passed = observed_trajectories == expected_trajectories
        && all_trajectories_successful
        && requested_outputs_bitwise_identical
        && controller_histories_bitwise_identical
        && accepted_rejected_sequences_identical
        && existing_work_counters_identical
        && per_trial_work_complete
        && per_trial_work_matches_aggregate
        && no_unscorable_attempts
        && no_legacy_unclassified_attempts
        && diagnostic_expensive_work_zero
        && complete_attempt_coverage
        && trajectories.iter().all(|row| row.all_required_gates_pass);
    G4S5B3HardGateSummary {
        expected_trajectories,
        observed_trajectories,
        all_trajectories_successful,
        requested_outputs_bitwise_identical,
        controller_histories_bitwise_identical,
        accepted_rejected_sequences_identical,
        existing_work_counters_identical,
        per_trial_work_complete,
        per_trial_work_matches_aggregate,
        no_unscorable_attempts,
        no_legacy_unclassified_attempts,
        diagnostic_expensive_work_zero,
        complete_attempt_coverage,
        passed,
    }
}

fn suites_numerically_identical(reference: &SuiteExecution, candidate: &SuiteExecution) -> bool {
    reference.trajectories.len() == candidate.trajectories.len()
        && reference
            .trajectories
            .iter()
            .zip(&candidate.trajectories)
            .all(|(left, right)| {
                left.case_index == right.case_index
                    && same_f64_bits(&left.result.observed.t, &right.result.observed.t)
                    && same_nested_f64_bits(&left.result.observed.y, &right.result.observed.y)
                    && left.result.observed.success == right.result.observed.success
                    && left.result.observed.message == right.result.observed.message
                    && left.result.observed.counters == right.result.observed.counters
                    && left.result.observed.internal_steps == right.result.observed.internal_steps
                    && left.result.observed.output_clipped_steps
                        == right.result.observed.output_clipped_steps
                    && left.result.diagnostics.attempts == right.result.diagnostics.attempts
                    && left.result.diagnostics.accepted_steps
                        == right.result.diagnostics.accepted_steps
                    && left.result.diagnostics.rejected_steps
                        == right.result.diagnostics.rejected_steps
                    && same_f64_bits(
                        &left.result.diagnostics.accepted_step_sizes,
                        &right.result.diagnostics.accepted_step_sizes,
                    )
                    && same_f64_bits(
                        &left.result.diagnostics.rejected_step_sizes,
                        &right.result.diagnostics.rejected_step_sizes,
                    )
                    && same_f64_bits(
                        &left.result.diagnostics.time_error_norms,
                        &right.result.diagnostics.time_error_norms,
                    )
                    && same_f64_bits(
                        &left.result.diagnostics.phi_error_norms,
                        &right.result.diagnostics.phi_error_norms,
                    )
                    && same_f64_bits(
                        &left.result.diagnostics.total_error_norms,
                        &right.result.diagnostics.total_error_norms,
                    )
                    && left.result.diagnostics.maximum_krylov_dimensions
                        == right.result.diagnostics.maximum_krylov_dimensions
                    && left.result.diagnostics.phi_substeps == right.result.diagnostics.phi_substeps
                    && left.result.diagnostics.early_flow_defect_attempts
                        == right.result.diagnostics.early_flow_defect_attempts
            })
}

fn suite_interval_sum(suite: &SuiteExecution) -> f64 {
    let mut values = Vec::new();
    for trajectory in &suite.trajectories {
        values.extend(
            trajectory
                .result
                .diagnostics
                .accepted_step_sizes
                .iter()
                .copied(),
        );
        values.extend(
            trajectory
                .result
                .diagnostics
                .rejected_step_sizes
                .iter()
                .copied(),
        );
    }
    sorted_interval_sum(values)
}

fn timed_arm(
    cases: &[PreparedCase],
    mode: SuiteTelemetryMode,
    repetitions: usize,
    reference: &SuiteExecution,
) -> CoreResult<G4S5B3OverheadArm> {
    let mut wall_seconds = 0.0;
    let mut proposed_interval = 0.0;
    let mut all_suite_identities_passed = true;
    for _ in 0..repetitions {
        let start = Instant::now();
        let suite = execute_suite(cases, mode)?;
        wall_seconds += start.elapsed().as_secs_f64();
        proposed_interval += suite_interval_sum(&suite);
        all_suite_identities_passed &= suites_numerically_identical(reference, &suite);
    }
    let gamma_seconds_per_interval = if proposed_interval > 0.0 {
        wall_seconds / proposed_interval
    } else {
        f64::INFINITY
    };
    Ok(G4S5B3OverheadArm {
        mode: mode.label().into(),
        repetitions,
        wall_seconds,
        proposed_interval,
        gamma_seconds_per_interval,
        all_suite_identities_passed,
    })
}

fn overhead_pair(
    cases: &[PreparedCase],
    repetitions: usize,
    pair_index: usize,
    disabled_first: bool,
    disabled_reference: &SuiteExecution,
    read_only_reference: &SuiteExecution,
    read_only_mode: SuiteTelemetryMode,
) -> CoreResult<G4S5B3OverheadPair> {
    let (disabled, read_only, order) = if disabled_first {
        let disabled = timed_arm(
            cases,
            SuiteTelemetryMode::Disabled,
            repetitions,
            disabled_reference,
        )?;
        let read_only = timed_arm(cases, read_only_mode, repetitions, read_only_reference)?;
        (disabled, read_only, "disabled-first")
    } else {
        let read_only = timed_arm(cases, read_only_mode, repetitions, read_only_reference)?;
        let disabled = timed_arm(
            cases,
            SuiteTelemetryMode::Disabled,
            repetitions,
            disabled_reference,
        )?;
        (disabled, read_only, "read-only-first")
    };
    if disabled.proposed_interval.to_bits() != read_only.proposed_interval.to_bits() {
        return Err(CoreError::InvalidInput(
            "paired overhead proposed intervals differ despite numerical identity".into(),
        ));
    }
    let wall_ratio_read_only_over_disabled = read_only.wall_seconds / disabled.wall_seconds;
    let gamma_ratio_read_only_over_disabled =
        read_only.gamma_seconds_per_interval / disabled.gamma_seconds_per_interval;
    Ok(G4S5B3OverheadPair {
        pair_index,
        order: order.into(),
        disabled,
        read_only,
        wall_ratio_read_only_over_disabled,
        gamma_ratio_read_only_over_disabled,
    })
}

fn run_overhead(
    profile: G4S5B3Profile,
    cases: &[PreparedCase],
    disabled_reference: &SuiteExecution,
    read_only_reference: &SuiteExecution,
    read_only_mode: SuiteTelemetryMode,
) -> CoreResult<G4S5B3OverheadReport> {
    let protocol = profile.overhead_protocol();
    let mut repetitions = 1_usize;
    let mut calibration_rows = Vec::new();
    loop {
        let arm = timed_arm(
            cases,
            SuiteTelemetryMode::Disabled,
            repetitions,
            disabled_reference,
        )?;
        calibration_rows.push(G4S5B3CalibrationRow {
            repetitions,
            wall_seconds: arm.wall_seconds,
            proposed_interval: arm.proposed_interval,
            gamma_seconds_per_interval: arm.gamma_seconds_per_interval,
            all_suite_identities_passed: arm.all_suite_identities_passed,
        });
        if arm.wall_seconds >= protocol.minimum_wall_seconds
            || repetitions >= protocol.maximum_repetitions
        {
            break;
        }
        repetitions = repetitions
            .saturating_mul(2)
            .min(protocol.maximum_repetitions);
    }
    let mut warmup_rows = Vec::new();
    for pair_index in 0..protocol.warmup_pairs {
        warmup_rows.push(overhead_pair(
            cases,
            repetitions,
            pair_index,
            pair_index % 2 == 0,
            disabled_reference,
            read_only_reference,
            read_only_mode,
        )?);
    }
    let mut measured_rows = Vec::new();
    for pair_index in 0..protocol.measured_pairs {
        measured_rows.push(overhead_pair(
            cases,
            repetitions,
            pair_index,
            pair_index % 2 == 0,
            disabled_reference,
            read_only_reference,
            read_only_mode,
        )?);
    }
    let all_suite_identities_passed = calibration_rows
        .iter()
        .all(|row| row.all_suite_identities_passed)
        && warmup_rows.iter().all(|row| {
            row.disabled.all_suite_identities_passed && row.read_only.all_suite_identities_passed
        })
        && measured_rows.iter().all(|row| {
            row.disabled.all_suite_identities_passed && row.read_only.all_suite_identities_passed
        });
    Ok(G4S5B3OverheadReport {
        warmup_pairs: protocol.warmup_pairs,
        measured_pairs: protocol.measured_pairs,
        frozen_repetitions: repetitions,
        minimum_calibration_wall_seconds: protocol.minimum_wall_seconds,
        maximum_calibration_repetitions: protocol.maximum_repetitions,
        calibration_rows,
        warmup_rows,
        measured_rows,
        all_suite_identities_passed,
    })
}

pub fn run_g4_s5b3_attempt_geometry(
    profile: G4S5B3Profile,
) -> CoreResult<G4S5B3AttemptGeometryReport> {
    let cases = prepare_cases(profile)?;
    let disabled = execute_suite(&cases, SuiteTelemetryMode::Disabled)?;
    let read_only = execute_suite(&cases, SuiteTelemetryMode::RawReadOnly)?;
    if disabled.trajectories.len() != cases.len() || read_only.trajectories.len() != cases.len() {
        return Err(CoreError::InvalidInput(
            "attempt-geometry suite trajectory count mismatch".into(),
        ));
    }
    let trajectories = cases
        .iter()
        .zip(&disabled.trajectories)
        .zip(&read_only.trajectories)
        .map(|((case, disabled), read_only)| {
            if disabled.case_index != read_only.case_index {
                return Err(CoreError::InvalidInput(
                    "attempt-geometry suite case order mismatch".into(),
                ));
            }
            Ok(compare_trajectory(
                case,
                &disabled.result,
                &read_only.result,
            ))
        })
        .collect::<CoreResult<Vec<_>>>()?;
    let hard_gates = hard_gate_summary(profile, &trajectories);
    let attempts = flatten_attempts(&cases, &read_only);
    let overhead = if hard_gates.passed {
        Some(run_overhead(
            profile,
            &cases,
            &disabled,
            &read_only,
            SuiteTelemetryMode::RawReadOnly,
        )?)
    } else {
        None
    };
    let overhead_pass = overhead
        .as_ref()
        .is_some_and(|report| report.all_suite_identities_passed);
    let status = if hard_gates.passed && overhead_pass {
        "pass"
    } else if hard_gates.passed {
        "hold-overhead-identity"
    } else {
        "blocked-hard-gate"
    };
    Ok(G4S5B3AttemptGeometryReport {
        schema: "g4-s5b3-attempt-geometry-raw-v1",
        status,
        profile: profile.as_str(),
        contract_path: "research/generic_event_reentry_shadow_v23/contracts/G4_S5B3_ATTEMPT_GEOMETRY_CONTRACT.json",
        active_switching: false,
        early_abort: false,
        threshold_selected: false,
        selected_threshold: None,
        orthogonalization: "full-mgs",
        threads: 1,
        requested_output: "endpoints-only",
        attempts,
        trajectories,
        hard_gates,
        overhead,
        limitations: vec![
            "eta_c2 is an admission diagnostic, not a local-error theorem".into(),
            "raw L2 normalization is component-scaling dependent".into(),
            "this node does not choose or recommend a threshold".into(),
            "this node does not implement active switching or early abort".into(),
            "canonical corpus is small/medium dense and not a large sparse holdout".into(),
        ],
    })
}

pub fn run_p1_00_tolerance_scaled_early_defect(
    profile: G4S5B3Profile,
) -> CoreResult<G4S5B3AttemptGeometryReport> {
    let cases = prepare_cases(profile)?;
    let disabled = execute_suite(&cases, SuiteTelemetryMode::Disabled)?;
    let read_only = execute_suite(&cases, SuiteTelemetryMode::ToleranceScaledReadOnly)?;
    if disabled.trajectories.len() != cases.len() || read_only.trajectories.len() != cases.len() {
        return Err(CoreError::InvalidInput(
            "tolerance-scaled early-defect suite trajectory count mismatch".into(),
        ));
    }
    let trajectories = cases
        .iter()
        .zip(&disabled.trajectories)
        .zip(&read_only.trajectories)
        .map(|((case, disabled), read_only)| {
            if disabled.case_index != read_only.case_index {
                return Err(CoreError::InvalidInput(
                    "tolerance-scaled early-defect suite case order mismatch".into(),
                ));
            }
            Ok(compare_trajectory(
                case,
                &disabled.result,
                &read_only.result,
            ))
        })
        .collect::<CoreResult<Vec<_>>>()?;
    let hard_gates = hard_gate_summary(profile, &trajectories);
    let attempts = flatten_attempts(&cases, &read_only);
    let scaled_coverage = attempts.iter().all(|row| {
        row.rho_c2_wrms.is_some_and(f64::is_finite)
            && row.tolerance_scale_atol == Some(row.atol)
            && row.tolerance_scale_rtol == Some(row.rtol)
    });
    let overhead = if hard_gates.passed && scaled_coverage {
        Some(run_overhead(
            profile,
            &cases,
            &disabled,
            &read_only,
            SuiteTelemetryMode::ToleranceScaledReadOnly,
        )?)
    } else {
        None
    };
    let overhead_pass = overhead
        .as_ref()
        .is_some_and(|report| report.all_suite_identities_passed);
    let status = if hard_gates.passed && scaled_coverage && overhead_pass {
        "pass"
    } else if hard_gates.passed && scaled_coverage {
        "hold-overhead-identity"
    } else {
        "blocked-hard-gate"
    };
    Ok(G4S5B3AttemptGeometryReport {
        schema: "p1-00-tolerance-scaled-early-defect-raw-v1",
        status,
        profile: profile.as_str(),
        contract_path: "research/generic_tolerance_scaled_early_defect_v24/contracts/P1_00_TOLERANCE_SCALED_EARLY_DEFECT_CONTRACT.json",
        active_switching: false,
        early_abort: false,
        threshold_selected: false,
        selected_threshold: None,
        orthogonalization: "full-mgs",
        threads: 1,
        requested_output: "endpoints-only",
        attempts,
        trajectories,
        hard_gates,
        overhead,
        limitations: vec![
            "rho_c2_wrms is an admission diagnostic, not a local-error theorem".into(),
            "the early scale uses U2 rather than the final candidate y_new".into(),
            "scalar atol assumes compatible component units or nondimensionalized state".into(),
            "this node does not choose or recommend a threshold".into(),
            "entry-window analysis is descriptive and is not a runtime event detector".into(),
            "this node does not implement active switching or early abort".into(),
        ],
    })
}
