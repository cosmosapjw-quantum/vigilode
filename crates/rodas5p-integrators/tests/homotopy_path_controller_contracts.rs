use rodas5p_integrators::{HomotopyPredictor, HomotopyRoundSpec, HomotopyScheduleConfig};

#[test]
fn schedule_contract_accepts_a_monotone_three_round_path() {
    let rounds = vec![
        HomotopyRoundSpec::new(1.0 / 3.0, 0.0, 0, 1.0, 0).unwrap(),
        HomotopyRoundSpec::new(2.0 / 3.0, 0.0, 1, 1.0, 0).unwrap(),
        HomotopyRoundSpec::new(1.0, 0.0, 2, 1.0, 1).unwrap(),
    ];
    let schedule = HomotopyScheduleConfig::new(rounds, HomotopyPredictor::Euler).unwrap();
    assert_eq!(schedule.rounds().len(), 3);
    assert_eq!(schedule.predictor(), HomotopyPredictor::Euler);
    assert_eq!(schedule.rounds()[2].q(), 2);
    assert_eq!(schedule.rounds()[2].corrections(), 1);
}

#[test]
fn schedule_contract_rejects_nonmonotone_or_incomplete_lambda_paths() {
    let nonmonotone = vec![
        HomotopyRoundSpec::new(0.7, 0.0, 1, 1.0, 0).unwrap(),
        HomotopyRoundSpec::new(0.6, 0.0, 1, 1.0, 0).unwrap(),
        HomotopyRoundSpec::new(1.0, 0.0, 1, 1.0, 0).unwrap(),
    ];
    assert!(HomotopyScheduleConfig::new(nonmonotone, HomotopyPredictor::Euler).is_err());

    let incomplete = vec![
        HomotopyRoundSpec::new(0.4, 0.0, 1, 1.0, 0).unwrap(),
        HomotopyRoundSpec::new(0.9, 0.0, 1, 1.0, 0).unwrap(),
    ];
    assert!(HomotopyScheduleConfig::new(incomplete, HomotopyPredictor::Euler).is_err());
}

#[test]
fn schedule_contract_rejects_invalid_round_parameters() {
    assert!(HomotopyRoundSpec::new(0.5, -0.1, 1, 1.0, 0).is_err());
    assert!(HomotopyRoundSpec::new(0.5, 0.0, 8, 1.0, 0).is_err());
    assert!(HomotopyRoundSpec::new(0.5, 0.0, 1, 0.0, 0).is_err());
    assert!(HomotopyRoundSpec::new(0.5, 0.0, 1, 1.0, 3).is_err());
    assert!(HomotopyRoundSpec::new(f64::NAN, 0.0, 1, 1.0, 0).is_err());
}

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use rodas5p_core::{DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    OdeProblem, StructuredBlockSystem, build_step_context, manufactured_vector_problem,
    run_fixed_homotopy_path, run_scheduled_homotopy_path,
};

fn maximum_stage_difference(left: &[Vec<f64>], right: &[Vec<f64>]) -> f64 {
    left.iter()
        .zip(right)
        .flat_map(|(a, b)| a.iter().zip(b).map(|(x, y)| (x - y).abs()))
        .fold(0.0, f64::max)
}

#[test]
fn scheduled_fixed_parameters_reproduce_the_existing_fixed_path() {
    let (problem, y0) = manufactured_vector_problem(4, 80.0, 10.0, 0.2, 0.0).unwrap();
    let mut fixed_counters = WorkCounters::default();
    let fixed_context = build_step_context(&problem, 0.0, &y0, 0.01, &mut fixed_counters).unwrap();
    let fixed_block = StructuredBlockSystem::new(&fixed_context);
    let fixed_config = rodas5p_integrators::HomotopyPathConfig::new(
        0.5,
        2,
        3,
        HomotopyPredictor::AdamsBashforth2,
        1,
    )
    .unwrap();
    let fixed = run_fixed_homotopy_path(&fixed_block, &fixed_config, &mut fixed_counters).unwrap();

    let schedule = HomotopyScheduleConfig::new(
        vec![
            HomotopyRoundSpec::new(1.0 / 3.0, 0.5, 2, 1.0, 1).unwrap(),
            HomotopyRoundSpec::new(2.0 / 3.0, 0.5, 2, 1.0, 1).unwrap(),
            HomotopyRoundSpec::new(1.0, 0.5, 2, 1.0, 1).unwrap(),
        ],
        HomotopyPredictor::AdamsBashforth2,
    )
    .unwrap();
    let mut scheduled_counters = WorkCounters::default();
    let scheduled_context =
        build_step_context(&problem, 0.0, &y0, 0.01, &mut scheduled_counters).unwrap();
    let scheduled_block = StructuredBlockSystem::new(&scheduled_context);
    let scheduled =
        run_scheduled_homotopy_path(&scheduled_block, &schedule, &mut scheduled_counters).unwrap();

    assert!(scheduled.completed, "failure={:?}", scheduled.failure);
    assert!(maximum_stage_difference(&fixed.stages, &scheduled.stages) < 2.0e-13);
    assert_eq!(fixed.points.len(), scheduled.points.len());
    for (a, b) in fixed.points.iter().zip(&scheduled.points) {
        assert!((a.lambda - b.lambda_end).abs() < 1.0e-15);
        assert!((a.target_residual_norm - b.target_residual_after).abs() < 2.0e-12);
    }
}

fn delayed_nonfinite_problem() -> (OdeProblem, Vec<f64>) {
    let batch_calls = Arc::new(AtomicUsize::new(0));
    let rhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = -10.0 * y[0];
        out[1] = -20.0 * y[1];
        Ok(())
    });
    let batch_counter = batch_calls.clone();
    let batch = Arc::new(move |_times: &[f64], states: &[Vec<f64>]| {
        let call = batch_counter.fetch_add(1, Ordering::SeqCst);
        let mut rows = states
            .iter()
            .map(|y| vec![-10.0 * y[0], -20.0 * y[1]])
            .collect::<Vec<_>>();
        if call >= 1 {
            rows[0][0] = f64::NAN;
        }
        Ok(rows)
    });
    let jac =
        Arc::new(|_t: f64, _y: &[f64]| DenseMatrix::from_rows(&[&[-10.0, 0.0], &[0.0, -20.0]]));
    (
        OdeProblem::new(
            "delayed-nonfinite",
            2,
            rhs,
            Some(batch),
            Some(jac),
            None,
            None,
            true,
            None,
            None,
        )
        .unwrap(),
        vec![1.0, -0.5],
    )
}

#[test]
fn scheduled_path_preserves_partial_points_and_work_on_numerical_failure() {
    let (problem, y0) = delayed_nonfinite_problem();
    let schedule = HomotopyScheduleConfig::new(
        vec![
            HomotopyRoundSpec::new(0.5, 0.0, 1, 1.0, 0).unwrap(),
            HomotopyRoundSpec::new(1.0, 0.0, 1, 1.0, 0).unwrap(),
        ],
        HomotopyPredictor::Euler,
    )
    .unwrap();
    let mut counters = WorkCounters::default();
    let context = build_step_context(&problem, 0.0, &y0, 0.01, &mut counters).unwrap();
    let block = StructuredBlockSystem::new(&context);
    let report = run_scheduled_homotopy_path(&block, &schedule, &mut counters).unwrap();
    assert!(!report.completed);
    assert!(
        report
            .failure
            .as_deref()
            .is_some_and(|text| text.contains("NaN/Inf"))
    );
    assert!(!report.points.is_empty());
    assert!(report.work.w_solve_vectors > 0);
    assert!(counters.rhs_evaluations > 0);
}

#[test]
fn path_controller_smoke_report_is_deterministic_and_preserves_controls() {
    let first = rodas5p_integrators::run_path_controller_screen(
        rodas5p_integrators::PathControllerProfile::Smoke,
    )
    .unwrap();
    let second = rodas5p_integrators::run_path_controller_screen(
        rodas5p_integrators::PathControllerProfile::Smoke,
    )
    .unwrap();
    assert_eq!(first, second);
    assert_eq!(first.schema, "rodas5p-path-controller-screen-v1");
    assert_eq!(first.profile, "smoke");
    assert!(!first.cases.is_empty());
    assert!(!first.controls.is_empty());
    assert!(!first.rows.is_empty());
    assert!(first.rows.iter().any(|row| row.schedule_id == "fixed-q2"));
    assert!(
        first
            .rows
            .iter()
            .any(|row| row.schedule_id == "fixed-q1-final-correction")
    );
    assert!(
        first
            .rows
            .iter()
            .any(|row| row.schedule_id == "escalate-q012")
    );
    assert!(first.controls.iter().all(|row| row.completed));
}

#[test]
fn path_controller_separates_algebraic_path_success_from_timestep_acceptance() {
    let report = rodas5p_integrators::run_path_controller_screen(
        rodas5p_integrators::PathControllerProfile::Smoke,
    )
    .unwrap();
    let q7 = report
        .rows
        .iter()
        .find(|row| row.case_id == "complex-dahlquist-s120-w180" && row.schedule_id == "fixed-q7")
        .unwrap();
    assert!(q7.algebraic_accepted);
    assert!(!q7.full_step_accepted);
    assert!(!q7.false_accept);
}
