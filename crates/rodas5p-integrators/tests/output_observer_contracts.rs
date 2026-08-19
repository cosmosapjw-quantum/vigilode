use rodas5p_core::LinearSolverConfig;
use rodas5p_integrators::{
    BdfConfig, BdfOrder, IntegrationMethod, NewtonConfig, OutputSchedule, RadauConfig,
    RadauIiaStages, integrate_adaptive_observed, integrate_bdf_fixed_observed, integrate_fixed,
    integrate_fixed_observed, integrate_radau_fixed_observed, scalar_linear_problem,
};

fn assert_state_close(left: &[f64], right: &[f64], tolerance: f64) {
    assert_eq!(left.len(), right.len());
    for (a, b) in left.iter().zip(right) {
        assert!((a - b).abs() <= tolerance, "{a:.17e} != {b:.17e}");
    }
}

#[test]
fn requested_output_schedule_removes_internal_step_storage_without_changing_states() {
    let (problem, y0) = scalar_linear_problem(-2.0, 1.0);
    let schedule = OutputSchedule::uniform(0.0, 0.2, 0.04).unwrap();
    let linear = LinearSolverConfig::default();
    let full = integrate_fixed(
        &problem,
        (0.0, 0.2),
        &y0,
        0.01,
        IntegrationMethod::Sequential,
        Some(&linear),
        None,
        1.0e-12,
        1.0e-10,
    )
    .unwrap();
    let observed = integrate_fixed_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        0.01,
        IntegrationMethod::Sequential,
        Some(&linear),
        None,
        1.0e-12,
        1.0e-10,
        &schedule,
    )
    .unwrap();

    assert_eq!(observed.t, schedule.times());
    assert_eq!(observed.y.len(), schedule.times().len());
    assert!(observed.y.len() < full.y.len());
    assert_eq!(
        observed.internal_steps,
        full.counters.accepted_steps as usize
    );
    assert_eq!(observed.output_clipped_steps, 0);

    for (time, state) in observed.t.iter().zip(&observed.y) {
        let index = full
            .t
            .iter()
            .position(|candidate| (candidate - time).abs() <= 1.0e-13)
            .unwrap();
        assert_state_close(state, &full.y[index], 1.0e-13);
    }
}

#[test]
fn all_fixed_anchor_families_land_on_the_same_requested_grid() {
    let (problem, y0) = scalar_linear_problem(-2.0, 1.0);
    let schedule = OutputSchedule::uniform(0.0, 0.2, 0.04).unwrap();
    let linear = LinearSolverConfig::default();

    let sequential = integrate_fixed_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        0.03,
        IntegrationMethod::Sequential,
        Some(&linear),
        None,
        1.0e-12,
        1.0e-10,
        &schedule,
    )
    .unwrap();
    let bdf1 = integrate_bdf_fixed_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        0.03,
        &BdfConfig {
            order: BdfOrder::One,
            newton: NewtonConfig::default(),
        },
        &schedule,
    )
    .unwrap();
    let bdf2 = integrate_bdf_fixed_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        0.03,
        &BdfConfig {
            order: BdfOrder::Two,
            newton: NewtonConfig::default(),
        },
        &schedule,
    )
    .unwrap();
    let radau1 = integrate_radau_fixed_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        0.03,
        &RadauConfig {
            stages: RadauIiaStages::One,
            ..RadauConfig::default()
        },
        &schedule,
    )
    .unwrap();
    let radau3 = integrate_radau_fixed_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        0.03,
        &RadauConfig {
            stages: RadauIiaStages::Three,
            ..RadauConfig::default()
        },
        &schedule,
    )
    .unwrap();

    for result in [&sequential, &bdf1, &bdf2, &radau1, &radau3] {
        assert_eq!(result.t, schedule.times());
        assert_eq!(result.y.len(), schedule.times().len());
        assert!(result.output_clipped_steps > 0);
        assert_eq!(
            result.internal_steps,
            result.counters.accepted_steps as usize
        );
    }
}

#[test]
fn adaptive_observer_keeps_only_requested_outputs_across_retries_and_clips() {
    let (problem, y0) = scalar_linear_problem(-100.0, 1.0);
    let schedule = OutputSchedule::uniform(0.0, 0.2, 0.04).unwrap();
    let linear = LinearSolverConfig::default();
    let observed = integrate_adaptive_observed(
        &problem,
        (0.0, 0.2),
        &y0,
        0.15,
        IntegrationMethod::Sequential,
        Some(&linear),
        None,
        1.0e-12,
        1.0e-8,
        10_000,
        0.15,
        &schedule,
    )
    .unwrap();

    assert!(observed.success);
    assert!(observed.counters.rejected_steps > 0);
    assert_eq!(observed.t, schedule.times());
    assert_eq!(observed.y.len(), schedule.times().len());
    assert_eq!(
        observed.internal_steps,
        observed.counters.accepted_steps as usize
    );
    assert!(observed.output_clipped_steps > 0);
}
