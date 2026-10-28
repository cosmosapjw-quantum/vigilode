//! Contract tests for the diagnostic packet comparison (LP09).
//!
//! Validates:
//! - VG-1: Zero online solver counters on packet candidate arm vs positive solver counters on baseline.
//! - Agreement diagnostics are distinct from independent physical accuracy evidence.
//! - VG-3: Proper hoisting verification; refusal of unhoisted unfair comparisons; negative gates.

use rodas5p_core::WorkCounters;
use rodas5p_fair_ab::{
    PacketBenchmarkConfig, build_hoisted_collective_ode_problem, run_packet_benchmark,
};
use rodas5p_integrators::CollectiveModel;

fn create_test_model() -> CollectiveModel {
    // 4-mode collective model with weak coupling
    CollectiveModel::new(
        vec![-0.5, -0.2, 0.2, 0.5],
        vec![0.25, 0.25, 0.25, 0.25],
        vec![(0.3, 0.0), (0.2, 0.2), (-0.2, 0.2), (0.0, -0.3)],
        1.0,
        0.05,
        0.0,
        0.0,
    )
    .expect("valid model")
}

#[test]
fn test_vg1_zero_online_solver_counters_on_candidate() {
    let model = create_test_model();
    let config = PacketBenchmarkConfig {
        t_start: 0.0,
        t_end: 0.3,
        tolerance: 1e-5,
        ..Default::default()
    };

    let report = run_packet_benchmark(&model, &config).expect("benchmark run succeeds");

    // Candidate arm has zero Jacobian and LU/linear solve counters
    assert_eq!(report.packet_candidate.work_counters.jacobian_builds, 0);
    assert_eq!(report.packet_candidate.work_counters.jvp_calls, 0);
    assert_eq!(
        report.packet_candidate.work_counters.direct_factorizations,
        0
    );
    assert_eq!(report.packet_candidate.work_counters.linear_solves, 0);

    // Baseline arm has positive Jacobian and LU/linear solve counters
    assert!(report.direct_baseline.work_counters.jacobian_builds > 0);
    assert!(report.direct_baseline.work_counters.direct_factorizations > 0);
    assert!(report.direct_baseline.work_counters.linear_solves > 0);
    assert!(report.work_reduction.online_solvers_eliminated);
}

#[test]
fn test_nominal_tolerance_and_agreement_do_not_promote_physical_accuracy() {
    let model = create_test_model();
    let config = PacketBenchmarkConfig {
        t_start: 0.0,
        t_end: 0.3,
        tolerance: 1e-6,
        order: 5,
        degree: 10,
        sweeps: 3,
        ..Default::default()
    };

    let report = run_packet_benchmark(&model, &config).expect("benchmark run succeeds");

    let packet = report.packet_candidate.final_mean;
    let direct = report.direct_baseline.final_mean;
    let expected_discrepancy =
        ((packet.0 - direct.0).powi(2) + (packet.1 - direct.1).powi(2)).sqrt();
    assert_eq!(report.mean_discrepancy_diagnostic, expected_discrepancy);
    assert!(report.mean_discrepancy_diagnostic.is_finite());
    assert!(report.packet_candidate.reported_mean_radius.is_some());
    assert!(report.direct_baseline.reported_mean_radius.is_none());
    assert!(report.packet_candidate.error_vs_reference.is_none());
    assert!(report.direct_baseline.error_vs_reference.is_none());
    assert_eq!(
        report.physical_accuracy_status,
        "UNVERIFIED_NO_INDEPENDENT_REFERENCE"
    );
    assert!(!report.fairness_verified);
    assert_eq!(report.verdict, "DIAGNOSTIC_ONLY");

    // Unmeasured data must survive serialization as null, rather than numeric zero.
    let json = serde_json::to_value(&report).expect("serialize diagnostic report");
    for arm in ["packet_candidate", "direct_baseline"] {
        assert!(json[arm]["hot_loop_allocations"].is_null());
        assert!(json[arm]["error_vs_reference"].is_null());
    }
    assert_eq!(
        report.packet_candidate.work_counters_authority,
        "structural_zero_not_instrumented"
    );
    assert_eq!(
        report.direct_baseline.work_counters_authority,
        "instrumented"
    );
}

#[test]
fn test_vg3_hoisted_ode_problem_structure() {
    let model = create_test_model();
    let problem = build_hoisted_collective_ode_problem(&model).expect("problem builds");
    assert_eq!(problem.dimension, 8); // 4 modes * 2 coords = 8

    let y: Vec<f64> = vec![0.1; 8];
    let mut dy: Vec<f64> = vec![0.0; 8];
    let mut counters = WorkCounters::default();
    problem
        .eval_rhs_into(0.0, &y, &mut dy, &mut counters)
        .expect("rhs succeeds");
    for &val in &dy {
        assert!(val.is_finite());
    }
}

#[test]
fn test_refuse_unhoisted_unfair_baseline() {
    let model = create_test_model();
    let config = PacketBenchmarkConfig {
        allow_unhoisted: true, // Should be refused!
        ..Default::default()
    };

    let res = run_packet_benchmark(&model, &config);
    assert!(res.is_err(), "Must refuse unhoisted baseline comparison");
}

#[test]
fn test_negative_parameter_gates() {
    let model = create_test_model();

    // Reversed interval
    let bad_interval = PacketBenchmarkConfig {
        t_start: 1.0,
        t_end: 0.0,
        ..Default::default()
    };
    assert!(run_packet_benchmark(&model, &bad_interval).is_err());

    // Non-positive tolerance
    let bad_tol = PacketBenchmarkConfig {
        tolerance: -1e-6,
        ..Default::default()
    };
    assert!(run_packet_benchmark(&model, &bad_tol).is_err());
}
