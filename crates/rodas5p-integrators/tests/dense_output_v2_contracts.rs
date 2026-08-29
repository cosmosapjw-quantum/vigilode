use std::sync::Arc;

use rodas5p_core::{
    CoreError, DenseMatrix, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters,
    load_rodas5p_coefficients,
};
use rodas5p_integrators::{
    AdaptiveControllerState, AdaptiveStepConfig, BdfConfig, BdfOrder, ControllerKind,
    HomotopyPathConfig, HomotopyPredictor, HomotopyStepConfig, IntegrationMethod,
    OutputSamplingPlan, OutputSchedule, RadauConfig, RadauIiaStages, SabrConfig, StepResult,
    TransactionalQ1Q2Config, bdf_dense_output, integrate_adaptive_dense_observed_with_config,
    integrate_bdf_adaptive_dense_observed, integrate_bdf_adaptive_observed,
    integrate_bdf_fixed_dense_observed, integrate_fixed_dense_observed, integrate_fixed_observed,
    integrate_homotopy_adaptive_dense_observed, integrate_radau_adaptive_dense_observed,
    integrate_radau_adaptive_observed, integrate_radau_fixed_dense_observed,
    integrate_sequential_matrix_free_adaptive_dense_observed,
    integrate_transactional_q1_q2_adaptive_dense_observed, prothero_robinson_problem,
    radau_dense_output, radau_step, rodas_next_step_after_attempt, rodas5p_dense_output,
    scalar_linear_problem, sequential_step,
};

fn assert_bits_equal(left: &[f64], right: &[f64]) {
    assert_eq!(left.len(), right.len());
    for (actual, expected) in left.iter().zip(right) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
}

fn assert_close(left: f64, right: f64, tolerance: f64) {
    assert!(
        (left - right).abs() <= tolerance,
        "{left:.17e} != {right:.17e} (tol={tolerance:.3e})"
    );
}

fn halving_orders(errors: &[f64]) -> Vec<f64> {
    errors
        .windows(2)
        .map(|pair| (pair[0] / pair[1]).ln() / 2.0_f64.ln())
        .collect()
}

#[test]
fn analytic_dense_extensions_recover_their_expected_interior_orders() {
    let t0 = 0.3;
    let theta = 0.37;
    let (problem, y0) = prothero_robinson_problem(-2.0, 0.5, t0);
    let step_sizes = [0.2, 0.1, 0.05, 0.025];

    let mut rodas_errors = Vec::new();
    let mut radau_errors = Vec::new();
    let mut bdf_errors = Vec::new();
    for h in step_sizes {
        let exact = problem.exact(t0 + theta * h).unwrap()[0];

        let mut rodas_counters = WorkCounters::default();
        let rodas = sequential_step(
            &problem,
            t0,
            &y0,
            h,
            &LinearSolverConfig::default(),
            None,
            1.0e-14,
            1.0e-12,
            true,
            &mut rodas_counters,
        )
        .unwrap();
        rodas_errors.push((rodas5p_dense_output(&rodas, theta).unwrap()[0] - exact).abs());

        let mut radau_counters = WorkCounters::default();
        let radau = radau_step(
            &problem,
            t0,
            &y0,
            h,
            &RadauConfig::default(),
            &mut radau_counters,
        )
        .unwrap();
        radau_errors.push(
            (radau_dense_output(
                radau.stages,
                &y0,
                &radau.y_new,
                &radau.stage_increments,
                theta,
            )
            .unwrap()[0]
                - exact)
                .abs(),
        );

        let previous = problem.exact(t0 - h).unwrap();
        let mut history = rodas5p_integrators::BdfHistory::with_previous(previous, h).unwrap();
        let mut bdf_counters = WorkCounters::default();
        let bdf = rodas5p_integrators::bdf_step(
            &problem,
            t0,
            &y0,
            h,
            &BdfConfig {
                order: BdfOrder::Two,
                ..BdfConfig::default()
            },
            &mut history,
            &mut bdf_counters,
        )
        .unwrap();
        bdf_errors.push((bdf_dense_output(&bdf, &y0, theta).unwrap()[0] - exact).abs());
    }

    let rodas_orders = halving_orders(&rodas_errors);
    let radau_orders = halving_orders(&radau_errors);
    let bdf_orders = halving_orders(&bdf_errors);
    assert!(
        rodas_orders.iter().copied().fold(f64::INFINITY, f64::min) > 3.8,
        "RODAS5P dense orders={rodas_orders:?}, errors={rodas_errors:?}"
    );
    assert!(
        *radau_orders.last().unwrap() > 3.8,
        "Radau IIA3 dense orders={radau_orders:?}, errors={radau_errors:?}"
    );
    assert!(
        bdf_orders.iter().copied().fold(f64::INFINITY, f64::min) > 2.7,
        "BDF2 dense orders={bdf_orders:?}, errors={bdf_errors:?}"
    );
}

#[test]
fn fixed_dense_observation_samples_four_inner_times_without_clipping_the_macro_step() {
    let (problem, y0) = scalar_linear_problem(-2.0, 1.0);
    let output = OutputSchedule::new(vec![0.0, 0.025, 0.05, 0.075, 0.1]).unwrap();

    let observed = integrate_fixed_dense_observed(
        &problem,
        (0.0, 0.1),
        &y0,
        0.1,
        IntegrationMethod::Sequential,
        Some(&LinearSolverConfig::default()),
        None,
        1.0e-12,
        1.0e-10,
        &OutputSamplingPlan::dense(output.clone()),
    )
    .unwrap();

    assert_eq!(observed.t, output.times());
    assert_eq!(observed.internal_steps, 1);
    assert_eq!(observed.output_clipped_steps, 0);
}

#[test]
fn rodas5p_h_gamma_extension_uses_transformed_k_without_an_extra_h() {
    let coefficients = load_rodas5p_coefficients().unwrap();
    assert_eq!(coefficients.dense_h.nrows(), 3);
    assert_eq!(coefficients.dense_h.ncols(), 8);
    assert_eq!(coefficients.dense_d.nrows(), 3);
    assert_eq!(coefficients.dense_d.ncols(), 8);
    for row in 0..3 {
        for column in 0..8 {
            let expected = (0..8)
                .map(|index| {
                    coefficients.dense_h[(row, index)] * coefficients.gamma_matrix[(index, column)]
                })
                .sum::<f64>();
            assert_close(coefficients.dense_d[(row, column)], expected, 2.0e-13);
        }
    }

    let stages = (0..8)
        .map(|index| vec![0.01 * (index + 1) as f64, -0.02 * (index + 1) as f64])
        .collect::<Vec<_>>();
    let step = StepResult {
        t_old: 3.0,
        t_new: 10.5,
        y_old: vec![1.25, -0.75],
        y_new: vec![1.5, -0.5],
        // A deliberately large h catches an accidental second h multiplier.
        h: 7.5,
        stages: stages.clone(),
        error_vector: vec![0.0; 2],
        error_norm: 0.0,
        accepted: true,
        method: "test".into(),
        used_fallback: false,
        certificate: None,
        counters: WorkCounters::default(),
    };
    assert_bits_equal(&rodas5p_dense_output(&step, 0.0).unwrap(), &step.y_old);
    assert_bits_equal(&rodas5p_dense_output(&step, 1.0).unwrap(), &step.y_new);

    let theta = 0.37;
    let complement = 1.0 - theta;
    let expected = (0..2)
        .map(|component| {
            let d = (0..3)
                .map(|row| {
                    coefficients
                        .dense_d
                        .row(row)
                        .iter()
                        .zip(&stages)
                        .map(|(coefficient, stage)| coefficient * stage[component])
                        .sum::<f64>()
                })
                .collect::<Vec<_>>();
            complement * step.y_old[component]
                + theta
                    * (step.y_new[component] + complement * (d[0] + theta * (d[1] + theta * d[2])))
        })
        .collect::<Vec<_>>();
    let actual = rodas5p_dense_output(&step, theta).unwrap();
    for (actual, expected) in actual.iter().zip(&expected) {
        assert_close(*actual, *expected, 2.0e-13);
    }
}

#[test]
fn radau_iia3_collocation_cubic_has_the_bk_endpoint_oracle_and_dense_sampling() {
    let y0 = vec![1.0, -2.0];
    let increments = vec![vec![0.2, -0.1], vec![-0.3, 0.4], vec![0.5, 0.2]];
    let sqrt6 = 6.0_f64.sqrt();
    let endpoint_weights = [(16.0 - sqrt6) / 36.0, (16.0 + sqrt6) / 36.0, 1.0 / 9.0];
    let y1 = (0..2)
        .map(|component| {
            y0[component]
                + endpoint_weights
                    .iter()
                    .zip(&increments)
                    .map(|(weight, increment)| weight * increment[component])
                    .sum::<f64>()
        })
        .collect::<Vec<_>>();
    assert_bits_equal(
        &radau_dense_output(RadauIiaStages::Three, &y0, &y1, &increments, 0.0).unwrap(),
        &y0,
    );
    assert_bits_equal(
        &radau_dense_output(RadauIiaStages::Three, &y0, &y1, &increments, 1.0).unwrap(),
        &y1,
    );

    let theta = 0.42;
    let c1 = (4.0 - sqrt6) / 10.0;
    let c2 = (4.0 + sqrt6) / 10.0;
    let theta2 = theta * theta;
    let theta3 = theta2 * theta;
    let weights = [
        25.0 / (3.0 * (1.0 + sqrt6)) * (theta3 / 3.0 - (1.0 + c2) * theta2 / 2.0 + c2 * theta),
        25.0 / (3.0 * (1.0 - sqrt6)) * (theta3 / 3.0 - (1.0 + c1) * theta2 / 2.0 + c1 * theta),
        10.0 * theta3 / 9.0 - 4.0 * theta2 / 3.0 + theta / 3.0,
    ];
    let expected = (0..2)
        .map(|component| {
            y0[component]
                + weights
                    .iter()
                    .zip(&increments)
                    .map(|(weight, increment)| weight * increment[component])
                    .sum::<f64>()
        })
        .collect::<Vec<_>>();
    let actual = radau_dense_output(RadauIiaStages::Three, &y0, &y1, &increments, theta).unwrap();
    for (actual, expected) in actual.iter().zip(&expected) {
        assert_close(*actual, *expected, 2.0e-14);
    }

    let (problem, initial) = scalar_linear_problem(-2.0, 1.0);
    let output = OutputSchedule::new(vec![0.0, 0.025, 0.05, 0.075, 0.1]).unwrap();
    let observed = integrate_radau_fixed_dense_observed(
        &problem,
        (0.0, 0.1),
        &initial,
        0.1,
        &RadauConfig::default(),
        &OutputSamplingPlan::dense(output.clone()),
    )
    .unwrap();
    assert_eq!(observed.t, output.times());
    assert_eq!(observed.internal_steps, 1);
    assert_eq!(observed.output_clipped_steps, 0);
}

#[test]
fn legacy_clipped_wrappers_are_bit_stable_and_new_hard_stops_do_not_cross() {
    let (problem, y0) = scalar_linear_problem(-2.0, 1.0);
    let output = OutputSchedule::new(vec![0.0, 0.025, 0.05, 0.075, 0.1]).unwrap();
    let legacy_a = integrate_fixed_observed(
        &problem,
        (0.0, 0.1),
        &y0,
        0.1,
        IntegrationMethod::Sequential,
        Some(&LinearSolverConfig::default()),
        None,
        1.0e-12,
        1.0e-10,
        &output,
    )
    .unwrap();
    let legacy_b = integrate_fixed_observed(
        &problem,
        (0.0, 0.1),
        &y0,
        0.1,
        IntegrationMethod::Sequential,
        Some(&LinearSolverConfig::default()),
        None,
        1.0e-12,
        1.0e-10,
        &output,
    )
    .unwrap();
    assert_eq!(legacy_a.internal_steps, 4);
    assert_eq!(legacy_a.output_clipped_steps, 3);
    assert_eq!(legacy_a.t, legacy_b.t);
    for (left, right) in legacy_a.y.iter().zip(&legacy_b.y) {
        assert_bits_equal(left, right);
    }

    let stopped = integrate_fixed_dense_observed(
        &problem,
        (0.0, 0.1),
        &y0,
        0.1,
        IntegrationMethod::Sequential,
        Some(&LinearSolverConfig::default()),
        None,
        1.0e-12,
        1.0e-10,
        &OutputSamplingPlan::new(output.clone(), vec![0.05]).unwrap(),
    )
    .unwrap();
    assert_eq!(stopped.t, output.times());
    assert_eq!(stopped.internal_steps, 2);
    assert_eq!(stopped.counters.accepted_steps, 2);
    assert_eq!(stopped.output_clipped_steps, 0);
}

#[test]
fn hard_stop_landing_is_controller_neutral_while_ordinary_outputs_are_not_landings() {
    let config = AdaptiveStepConfig {
        controller: ControllerKind::Pi,
        safety: 0.5,
        min_factor: 0.1,
        max_factor: 10.0,
        ..AdaptiveStepConfig::default()
    };
    let mut controller = AdaptiveControllerState::default();
    // `trial_h` is the hard-stop landing; the dense path wires this boolean
    // only for a true breakpoint, never for an ordinary output time.
    let next =
        rodas_next_step_after_attempt(&mut controller, &config, 0.1, 0.05, 1.0e-12, true, true)
            .unwrap();
    assert_eq!(next.to_bits(), 0.1_f64.to_bits());
    assert_eq!(controller.previous_accepted_error(), None);
}

#[test]
fn protected_adaptive_rodas_and_low_order_implicit_dense_sampling_are_available() {
    let (problem, y0) = scalar_linear_problem(-2.0, 1.0);
    let matrix_free = problem.jvp_only_clone().unwrap();
    let output = OutputSchedule::new(vec![0.0, 0.025, 0.05, 0.075, 0.1]).unwrap();
    let adaptive = AdaptiveStepConfig {
        atol: 1.0,
        rtol: 0.0,
        initial_step: 0.1,
        min_step: 1.0e-12,
        max_step: 0.1,
        max_attempts: 8,
        ..AdaptiveStepConfig::default()
    };
    let observed = integrate_sequential_matrix_free_adaptive_dense_observed(
        &matrix_free,
        (0.0, 0.1),
        &y0,
        &LinearSolverConfig {
            method: LinearMethod::Gmres,
            preconditioner: PreconditionerKind::None,
            ..LinearSolverConfig::default()
        },
        &adaptive,
        &OutputSamplingPlan::dense(output.clone()),
    )
    .unwrap()
    .observed;
    assert!(observed.success);
    assert_eq!(observed.t, output.times());
    assert_eq!(observed.internal_steps, 1);
    assert_eq!(observed.output_clipped_steps, 0);

    assert_eq!(
        radau_dense_output(RadauIiaStages::One, &[1.0], &[3.0], &[vec![2.0]], 0.25).unwrap(),
        vec![1.5]
    );

    let bdf1 = integrate_bdf_fixed_dense_observed(
        &problem,
        (0.0, 0.1),
        &y0,
        0.1,
        &BdfConfig {
            order: BdfOrder::One,
            ..BdfConfig::default()
        },
        &OutputSamplingPlan::dense(output.clone()),
    )
    .unwrap();
    assert_eq!(bdf1.t, output.times());
    assert_eq!(bdf1.internal_steps, 1);
    assert_eq!(bdf1.output_clipped_steps, 0);

    let radau1 = integrate_radau_fixed_dense_observed(
        &problem,
        (0.0, 0.1),
        &y0,
        0.1,
        &RadauConfig {
            stages: RadauIiaStages::One,
            ..RadauConfig::default()
        },
        &OutputSamplingPlan::dense(output.clone()),
    )
    .unwrap();
    assert_eq!(radau1.t, output.times());
    assert_eq!(radau1.internal_steps, 1);
    assert_eq!(radau1.output_clipped_steps, 0);
}

#[test]
fn bdf2_quadratic_dense_interpolant_honors_all_three_nodes() {
    use rodas5p_integrators::{BdfHistory, bdf_step};

    let (problem, _) = scalar_linear_problem(0.0, 1.0);
    let mut history = BdfHistory::with_previous(vec![0.0], 1.0).unwrap();
    let mut counters = WorkCounters::default();
    let report = bdf_step(
        &problem,
        0.0,
        &[1.0],
        1.0,
        &BdfConfig {
            order: BdfOrder::Two,
            ..BdfConfig::default()
        },
        &mut history,
        &mut counters,
    )
    .unwrap();
    assert_eq!(bdf_dense_output(&report, &[1.0], 0.0).unwrap(), vec![1.0]);
    assert_eq!(
        bdf_dense_output(&report, &[1.0], 1.0).unwrap(),
        report.y_new
    );
}

#[test]
fn adaptive_radau_and_bdf_dense_paths_are_output_grid_invariant() {
    let (problem, y0) = scalar_linear_problem(-5.0, 1.0);
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-8,
        rtol: 1.0e-6,
        initial_step: 0.08,
        min_step: 1.0e-12,
        max_step: 0.2,
        max_attempts: 1024,
        ..AdaptiveStepConfig::default()
    };
    let coarse = OutputSamplingPlan::dense(OutputSchedule::new(vec![0.0, 0.2]).unwrap());
    let fine = OutputSamplingPlan::dense(
        OutputSchedule::new((0..=20).map(|index| index as f64 * 0.01).collect()).unwrap(),
    );

    for stages in [RadauIiaStages::One, RadauIiaStages::Three] {
        let radau_config = RadauConfig {
            stages,
            ..RadauConfig::default()
        };
        let radau_coarse = integrate_radau_adaptive_dense_observed(
            &problem,
            (0.0, 0.2),
            &y0,
            &radau_config,
            &adaptive,
            &coarse,
        )
        .unwrap();
        let radau_fine = integrate_radau_adaptive_dense_observed(
            &problem,
            (0.0, 0.2),
            &y0,
            &radau_config,
            &adaptive,
            &fine,
        )
        .unwrap();
        assert_eq!(
            radau_coarse.observed.internal_steps,
            radau_fine.observed.internal_steps
        );
        assert_eq!(radau_coarse.observed.counters, radau_fine.observed.counters);
    }

    for order in [BdfOrder::One, BdfOrder::Two] {
        let bdf_config = BdfConfig {
            order,
            ..BdfConfig::default()
        };
        let bdf_coarse = integrate_bdf_adaptive_dense_observed(
            &problem,
            (0.0, 0.2),
            &y0,
            &bdf_config,
            &adaptive,
            &coarse,
        )
        .unwrap();
        let bdf_fine = integrate_bdf_adaptive_dense_observed(
            &problem,
            (0.0, 0.2),
            &y0,
            &bdf_config,
            &adaptive,
            &fine,
        )
        .unwrap();
        assert_eq!(
            bdf_coarse.observed.internal_steps,
            bdf_fine.observed.internal_steps
        );
        assert_eq!(bdf_coarse.observed.counters, bdf_fine.observed.counters);
    }
}

#[test]
fn implicit_dense_paths_land_on_hard_stops_before_interpolating_later_outputs() {
    let (problem, y0) = scalar_linear_problem(-2.0, 1.0);
    let output = OutputSchedule::new(vec![0.0, 0.025, 0.05, 0.075, 0.1]).unwrap();
    let sampling = OutputSamplingPlan::new(output.clone(), vec![0.05]).unwrap();
    let adaptive = AdaptiveStepConfig {
        atol: 1.0,
        rtol: 0.0,
        initial_step: 0.1,
        min_step: 1.0e-12,
        max_step: 0.1,
        max_attempts: 32,
        ..AdaptiveStepConfig::default()
    };
    let radau = integrate_radau_adaptive_dense_observed(
        &problem,
        (0.0, 0.1),
        &y0,
        &RadauConfig::default(),
        &adaptive,
        &sampling,
    )
    .unwrap();
    let bdf = integrate_bdf_adaptive_dense_observed(
        &problem,
        (0.0, 0.1),
        &y0,
        &BdfConfig::default(),
        &adaptive,
        &sampling,
    )
    .unwrap();
    assert_eq!(radau.observed.t, output.times());
    assert_eq!(bdf.observed.t, output.times());
    assert!(radau.diagnostics.accepted_step_sizes.len() >= 2);
    assert!(bdf.diagnostics.accepted_step_sizes.len() >= 2);
    assert_eq!(
        radau.diagnostics.accepted_step_sizes[0].to_bits(),
        0.05_f64.to_bits()
    );
    assert_eq!(
        bdf.diagnostics.accepted_step_sizes[0].to_bits(),
        0.05_f64.to_bits()
    );
    assert_eq!(radau.observed.output_clipped_steps, 0);
    assert_eq!(bdf.observed.output_clipped_steps, 0);
    assert_eq!(
        bdf.diagnostics.estimator_ids,
        vec![
            "bdf-explicit-startup-step-doubling",
            "bdf-explicit-startup-step-doubling"
        ],
        "a declared hard stop must discard pre-discontinuity BDF history"
    );
}

#[test]
fn bdf_history_restarts_when_the_natural_step_exactly_matches_a_hard_stop() {
    let (problem, y0) = scalar_linear_problem(-2.0, 1.0);
    let output = OutputSchedule::new(vec![0.0, 0.025, 0.05, 0.075, 0.1]).unwrap();
    let sampling = OutputSamplingPlan::new(output, vec![0.05]).unwrap();
    let adaptive = AdaptiveStepConfig {
        atol: 1.0,
        rtol: 0.0,
        initial_step: 0.05,
        min_step: 1.0e-12,
        max_step: 0.05,
        max_attempts: 8,
        ..AdaptiveStepConfig::default()
    };
    let result = integrate_bdf_adaptive_dense_observed(
        &problem,
        (0.0, 0.1),
        &y0,
        &BdfConfig::default(),
        &adaptive,
        &sampling,
    )
    .unwrap();

    assert!(result.observed.success);
    assert_eq!(result.diagnostics.accepted_step_sizes, vec![0.05, 0.05]);
    assert_eq!(
        result.diagnostics.estimator_ids,
        vec![
            "bdf-explicit-startup-step-doubling",
            "bdf-explicit-startup-step-doubling"
        ],
        "hard-stop identity, not only step shortening, defines the BDF restart boundary"
    );
}

#[test]
fn adaptive_implicit_failures_preserve_the_committed_output_prefix() {
    let (problem, y0) = scalar_linear_problem(0.0, 1.0);
    let adaptive = AdaptiveStepConfig {
        atol: 1.0,
        rtol: 0.0,
        initial_step: 0.1,
        min_step: 1.0e-12,
        max_step: 0.1,
        max_attempts: 1,
        ..AdaptiveStepConfig::default()
    };
    let clipped_output = OutputSchedule::new(vec![0.0, 0.03, 1.0]).unwrap();
    let dense_output = OutputSchedule::new(vec![0.0, 0.05, 0.1, 1.0]).unwrap();

    let clipped_bdf = integrate_bdf_adaptive_observed(
        &problem,
        (0.0, 1.0),
        &y0,
        &BdfConfig::default(),
        &adaptive,
        &clipped_output,
    )
    .unwrap();
    let clipped_radau = integrate_radau_adaptive_observed(
        &problem,
        (0.0, 1.0),
        &y0,
        &RadauConfig::default(),
        &adaptive,
        &clipped_output,
    )
    .unwrap();
    assert!(!clipped_bdf.observed.success && !clipped_radau.observed.success);
    assert_eq!(clipped_bdf.observed.t, vec![0.0, 0.03]);
    assert_eq!(clipped_radau.observed.t, vec![0.0, 0.03]);
    assert_eq!(clipped_bdf.observed.y.len(), 2);
    assert_eq!(clipped_radau.observed.y.len(), 2);
    assert_eq!(clipped_bdf.observed.output_clipped_steps, 1);
    assert_eq!(clipped_radau.observed.output_clipped_steps, 1);

    let dense_bdf = integrate_bdf_adaptive_dense_observed(
        &problem,
        (0.0, 1.0),
        &y0,
        &BdfConfig::default(),
        &adaptive,
        &OutputSamplingPlan::dense(dense_output.clone()),
    )
    .unwrap();
    let dense_radau = integrate_radau_adaptive_dense_observed(
        &problem,
        (0.0, 1.0),
        &y0,
        &RadauConfig::default(),
        &adaptive,
        &OutputSamplingPlan::dense(dense_output),
    )
    .unwrap();
    assert!(!dense_bdf.observed.success && !dense_radau.observed.success);
    assert_eq!(dense_bdf.observed.t, vec![0.0, 0.05, 0.1]);
    assert_eq!(dense_radau.observed.t, vec![0.0, 0.05, 0.1]);
    assert_eq!(dense_bdf.observed.y.len(), 3);
    assert_eq!(dense_radau.observed.y.len(), 3);
}

#[test]
fn dense_implicit_failure_kinds_match_work_counters() {
    let rhs = Arc::new(|_t: f64, _y: &[f64], _out: &mut [f64]| {
        Err(CoreError::NonFinite(
            "intentional dense implicit failure".into(),
        ))
    });
    let jacobian = Arc::new(|_t: f64, _y: &[f64]| DenseMatrix::new(1, 1, vec![-1.0]));
    let jvp = Arc::new(|_t: f64, _y: &[f64], direction: &[f64], out: &mut [f64]| {
        out[0] = -direction[0];
        Ok(())
    });
    let problem = rodas5p_integrators::OdeProblem::new(
        "dense-implicit-failure",
        1,
        rhs,
        None,
        Some(jacobian),
        Some(jvp),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let adaptive = AdaptiveStepConfig {
        initial_step: 0.1,
        max_step: 0.1,
        max_attempts: 1,
        ..AdaptiveStepConfig::default()
    };
    let sampling = OutputSamplingPlan::dense(OutputSchedule::new(vec![0.0, 0.1]).unwrap());
    let bdf = integrate_bdf_adaptive_dense_observed(
        &problem,
        (0.0, 0.1),
        &[1.0],
        &BdfConfig::default(),
        &adaptive,
        &sampling,
    )
    .unwrap();
    let radau = integrate_radau_adaptive_dense_observed(
        &problem,
        (0.0, 0.1),
        &[1.0],
        &RadauConfig::default(),
        &adaptive,
        &sampling,
    )
    .unwrap();
    let rodas = integrate_sequential_matrix_free_adaptive_dense_observed(
        &problem.jvp_only_clone().unwrap(),
        (0.0, 0.1),
        &[1.0],
        &LinearSolverConfig {
            method: LinearMethod::Gmres,
            preconditioner: PreconditionerKind::None,
            ..LinearSolverConfig::default()
        },
        &adaptive,
        &sampling,
    )
    .unwrap();
    for result in [bdf, radau, rodas] {
        assert!(!result.observed.success);
        assert_eq!(result.diagnostics.non_finite_failures, 1);
        assert_eq!(result.observed.counters.nonfinite_step_failures, 1);
        assert_eq!(result.observed.t, vec![0.0]);
        assert_eq!(result.observed.y, vec![vec![1.0]]);
    }
}

#[test]
fn every_adaptive_rodas_family_samples_dense_outputs_without_grid_clipping() {
    let (problem, y0) = scalar_linear_problem(-5.0, 1.0);
    let matrix_free = problem.jvp_only_clone().unwrap();
    let schedule = OutputSchedule::new(vec![0.0, 0.025, 0.05, 0.075, 0.1]).unwrap();
    let sampling = OutputSamplingPlan::dense(schedule.clone());
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-8,
        rtol: 1.0e-6,
        initial_step: 0.05,
        min_step: 1.0e-12,
        max_step: 0.1,
        max_attempts: 1_000,
        ..AdaptiveStepConfig::default()
    };
    let direct = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    };

    let sequential = integrate_adaptive_dense_observed_with_config(
        &problem,
        (0.0, 0.1),
        &y0,
        IntegrationMethod::Sequential,
        Some(&direct),
        None,
        &adaptive,
        &sampling,
    )
    .unwrap();
    let sabr = integrate_adaptive_dense_observed_with_config(
        &problem,
        (0.0, 0.1),
        &y0,
        IntegrationMethod::Sabr,
        Some(&direct),
        Some(SabrConfig::default()),
        &adaptive,
        &sampling,
    )
    .unwrap();
    let path = HomotopyPathConfig::new(1.0, 7, 2, HomotopyPredictor::AdamsBashforth2, 1).unwrap();
    let homotopy = integrate_homotopy_adaptive_dense_observed(
        &problem,
        (0.0, 0.1),
        &y0,
        &HomotopyStepConfig::new(path, 0.1).unwrap(),
        Some(&direct),
        &adaptive,
        &sampling,
    )
    .unwrap();
    let transactional = integrate_transactional_q1_q2_adaptive_dense_observed(
        &matrix_free,
        (0.0, 0.1),
        &y0,
        &TransactionalQ1Q2Config::default(),
        &adaptive,
        &sampling,
    )
    .unwrap();

    for result in [sequential, sabr, homotopy] {
        assert!(result.observed.success);
        assert_eq!(result.observed.t, schedule.times());
        assert_eq!(result.observed.output_clipped_steps, 0);
        assert!(result.observed.internal_steps > 0);
    }
    assert!(transactional.observed.success);
    assert_eq!(transactional.observed.t, schedule.times());
    assert_eq!(transactional.observed.output_clipped_steps, 0);
    assert_eq!(
        transactional.transactional.accepted_steps(),
        transactional.observed.internal_steps
    );
}

#[test]
fn dense_output_grid_density_does_not_change_adaptive_internal_steps() {
    let (problem, y0) = scalar_linear_problem(-20.0, 1.0);
    let matrix_free = problem.jvp_only_clone().unwrap();
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-9,
        rtol: 1.0e-7,
        initial_step: 0.08,
        min_step: 1.0e-12,
        max_step: 0.2,
        max_attempts: 256,
        ..AdaptiveStepConfig::default()
    };
    let linear = LinearSolverConfig {
        method: LinearMethod::Gmres,
        preconditioner: PreconditionerKind::None,
        ..LinearSolverConfig::default()
    };
    let coarse = integrate_sequential_matrix_free_adaptive_dense_observed(
        &matrix_free,
        (0.0, 0.2),
        &y0,
        &linear,
        &adaptive,
        &OutputSamplingPlan::dense(OutputSchedule::new(vec![0.0, 0.2]).unwrap()),
    )
    .unwrap();
    let fine_times = (0..=20).map(|index| index as f64 * 0.01).collect();
    let fine = integrate_sequential_matrix_free_adaptive_dense_observed(
        &matrix_free,
        (0.0, 0.2),
        &y0,
        &linear,
        &adaptive,
        &OutputSamplingPlan::dense(OutputSchedule::new(fine_times).unwrap()),
    )
    .unwrap();

    assert_eq!(coarse.observed.internal_steps, fine.observed.internal_steps);
    assert_eq!(coarse.observed.counters, fine.observed.counters);
    assert_eq!(
        coarse
            .diagnostics
            .accepted_step_sizes
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        fine.diagnostics
            .accepted_step_sizes
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>()
    );
    assert_bits_equal(
        coarse.observed.y.last().unwrap(),
        fine.observed.y.last().unwrap(),
    );
}
