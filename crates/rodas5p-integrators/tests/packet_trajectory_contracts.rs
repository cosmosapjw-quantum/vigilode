use rodas5p_core::CoreError;
use rodas5p_core::directed::{Interval, add_up};
use rodas5p_integrators::collective_model::CollectiveModel;
use rodas5p_integrators::packet_trajectory::{
    LinearTrajectoryRefusal, OutputPolicy, TrajectoryConfig, run_linear_outward_trajectory,
    run_packet_trajectory,
};
use rodas5p_integrators::packet_work::PacketWorkCounters;

fn make_test_model() -> CollectiveModel {
    let xi = vec![-0.5, 0.0, 0.5];
    let weights = vec![0.25, 0.5, 0.25];
    let z = vec![(0.3, 0.0), (0.2, 0.1), (0.1, -0.2)];
    CollectiveModel::new(xi, weights, z, 1.0, 0.2, 0.0, 0.0).expect("valid collective model")
}

// ---------------------------------------------------------------------------
// VG-1: Zero Online Solver Counters Contract
// ---------------------------------------------------------------------------

#[test]
fn test_vg1_zero_online_solver_counters_across_trajectory() {
    let model = make_test_model();
    let config = TrajectoryConfig::new(0.0, 0.3, 0.1, 1e-3, 1e-2).expect("config");
    let mut work = PacketWorkCounters::default();

    let res = run_packet_trajectory(&model, None, &config, &mut work).expect("run trajectory");

    // Online solver counters MUST be strictly zero
    assert_eq!(work.jacobian_calls(), 0, "online jacobian calls must be 0");
    assert_eq!(work.jvp_calls(), 0, "online jvp calls must be 0");
    assert_eq!(
        work.krylov_iterations(),
        0,
        "online krylov iterations must be 0"
    );
    assert_eq!(work.linear_solves(), 0, "online linear solves must be 0");

    // Packet-specific work counters must be strictly positive
    assert!(work.moment_terms > 0, "moment terms must be tracked");
    assert!(
        work.poly_multiply_terms > 0,
        "poly multiply terms must be tracked"
    );
    assert!(work.volterra_terms > 0, "volterra terms must be tracked");
    assert!(
        work.nonlinear_evaluations > 0,
        "nonlinear evaluations must be tracked"
    );
    assert!(res.accepted_steps >= 2, "must accept multiple windows");
}

// ---------------------------------------------------------------------------
// VG-2: Multi-Window Overlap & Analytical Consistency
// ---------------------------------------------------------------------------

#[test]
fn test_vg2_multiwindow_overlap_and_dahlquist_linear_limit() {
    // Linear Dahlquist limit (chi = 0): exact solution is independent decoupled harmonic oscillators
    let xi = vec![-0.4, 0.0, 0.4];
    let weights = vec![0.3, 0.4, 0.3];
    let z = vec![(0.2, 0.0), (0.1, 0.2), (-0.1, 0.1)];
    let width = 1.0;
    let model = CollectiveModel::new(xi.clone(), weights, z.clone(), width, 0.0, 0.0, 0.0)
        .expect("chi = 0 model");

    let t_end = 0.2;
    let mut config = TrajectoryConfig::new(0.0, t_end, 0.05, 1e-4, 1e-3).expect("config");
    config.output_policy = OutputPolicy::AllModes;
    let mut work = PacketWorkCounters::default();

    let res = run_packet_trajectory(&model, None, &config, &mut work).expect("trajectory");

    // Compute exact analytical modes at t_end: a_j(t) = exp(i * width * xi_j * t) * z_j
    for (j, (&xi_j, &z_j)) in xi.iter().zip(z.iter()).enumerate() {
        let theta = width * xi_j * t_end;
        let c = theta.cos();
        let s = theta.sin();
        let exact_re = c * z_j.0 - s * z_j.1;
        let exact_im = s * z_j.0 + c * z_j.1;

        let num_mode = res.final_modes[j];
        let diff = ((num_mode.0 - exact_re).powi(2) + (num_mode.1 - exact_im).powi(2)).sqrt();
        let cert_bound = res.final_mode_radii[j];

        assert!(
            diff <= cert_bound + 1e-12,
            "mode {} error {:e} exceeds certified bound {:e}",
            j,
            diff,
            cert_bound
        );
    }
}

// ---------------------------------------------------------------------------
// VG-3: Transactional Atomicity & Output Policy Contracts
// ---------------------------------------------------------------------------

#[test]
fn test_vg3_transactional_atomicity_on_step_rejection() {
    let model = make_test_model();
    // Demand an impossibly tight tolerance so that steps fail and retry with smaller h
    let mut config = TrajectoryConfig::new(0.0, 0.05, 0.02, 1e-14, 1e-14).expect("config");
    config.min_h = 1e-3;
    config.max_steps = 15;
    let mut work = PacketWorkCounters::default();

    let prev_terms = work.total_packet_terms();
    let res = run_packet_trajectory(&model, None, &config, &mut work);

    // Regardless of whether it finishes or fails due to min_h underflow, work strictly increases
    assert!(
        work.total_packet_terms() > prev_terms,
        "rejected attempts must monotonically accumulate work"
    );

    // If it succeeded or hit step limit, verify accepted state was never corrupted
    if let Ok(traj) = res {
        for step in &traj.steps {
            assert!(step.mean.0.is_finite() && step.mean.1.is_finite());
            assert!(step.total_mean_bound.is_finite() && step.total_mean_bound >= 0.0);
        }
    }
}

#[test]
fn test_vg3_output_policy_separation() {
    let model = make_test_model();

    // 1. MeanOnly policy: modes and mode_radii must be None
    let mut config_mean = TrajectoryConfig::new(0.0, 0.1, 0.05, 1e-3, 1e-2).expect("config");
    config_mean.output_policy = OutputPolicy::MeanOnly;
    let mut work = PacketWorkCounters::default();
    let res_mean = run_packet_trajectory(&model, None, &config_mean, &mut work).expect("mean-only");

    for step in &res_mean.steps {
        assert!(step.modes.is_none(), "MeanOnly must not store mode vectors");
        assert!(
            step.mode_radii.is_none(),
            "MeanOnly must not store mode radii vectors"
        );
    }

    // 2. AllModes policy: modes and mode_radii must be Some(N)
    let mut config_all = TrajectoryConfig::new(0.0, 0.1, 0.05, 1e-3, 1e-2).expect("config");
    config_all.output_policy = OutputPolicy::AllModes;
    let mut work = PacketWorkCounters::default();
    let res_all = run_packet_trajectory(&model, None, &config_all, &mut work).expect("all-modes");

    for step in &res_all.steps {
        assert_eq!(
            step.modes.as_ref().map(|v| v.len()),
            Some(model.num_modes()),
            "AllModes must store all mode amplitudes"
        );
        assert_eq!(
            step.mode_radii.as_ref().map(|v| v.len()),
            Some(model.num_modes()),
            "AllModes must store all mode radii"
        );
    }
}

#[test]
fn test_vg3_inherited_radii_propagation() {
    let model = make_test_model();
    let initial_radii = vec![1e-5, 2e-5, 1e-5];

    let config = TrajectoryConfig::new(0.0, 0.1, 0.05, 1e-3, 1e-2).expect("config");
    let mut work = PacketWorkCounters::default();

    let res = run_packet_trajectory(&model, Some(&initial_radii), &config, &mut work)
        .expect("trajectory with initial radii");

    // Final mode radii must strictly exceed initial radii (non-zero accumulated uncertainty)
    for (final_r, &init_r) in res.final_mode_radii.iter().zip(initial_radii.iter()) {
        assert!(
            *final_r >= init_r,
            "final radius {:e} must carry initial radius {:e}",
            final_r,
            init_r
        );
    }
}

// ---------------------------------------------------------------------------
// Negative Admission Gates
// ---------------------------------------------------------------------------

#[test]
fn test_negative_admission_gates() {
    let model = make_test_model();

    // 1. t_end <= t_start
    let res = TrajectoryConfig::new(1.0, 0.5, 0.1, 1e-3, 1e-2);
    assert!(matches!(res, Err(CoreError::InvalidInput(_))));

    // 2. Non-positive step
    let res = TrajectoryConfig::new(0.0, 1.0, 0.0, 1e-3, 1e-2);
    assert!(matches!(res, Err(CoreError::InvalidInput(_))));

    // 3. Non-positive tolerance
    let res = TrajectoryConfig::new(0.0, 1.0, 0.1, -1e-3, 1e-2);
    assert!(matches!(res, Err(CoreError::InvalidInput(_))));

    // 4. Initial radii length mismatch
    let config = TrajectoryConfig::new(0.0, 0.1, 0.05, 1e-3, 1e-2).expect("config");
    let wrong_radii = vec![1e-5, 1e-5]; // 2 elements != 3 modes
    let mut work = PacketWorkCounters::default();
    let res = run_packet_trajectory(&model, Some(&wrong_radii), &config, &mut work);
    assert!(matches!(res, Err(CoreError::InvalidInput(_))));
}

#[test]
fn linear_outward_fixed_grid_encloses_independent_exact_modes_and_mean() {
    let weights = vec![0.25, 0.5, 0.25];
    let xi = vec![-0.5, 0.0, 0.5];
    let z = vec![(0.25, -0.125), (0.125, 0.0625), (-0.0625, 0.25)];
    let model = CollectiveModel::new(xi.clone(), weights.clone(), z.clone(), 1.0, 0.0, 0.0, 0.0)
        .expect("linear model");
    let mut config = TrajectoryConfig::new(0.0, 0.25, 0.0625, 1e-5, 1e-5).unwrap();
    config.output_policy = OutputPolicy::AllModes;
    let times = [0.03125, 0.09375, 0.15625, 0.25];
    let mut work = PacketWorkCounters::default();
    let result = run_linear_outward_trajectory(&model, None, &config, &times, &mut work)
        .expect("linear outward trajectory");
    assert!(!result.roundoff_enclosed);
    assert_eq!(result.observations.len(), times.len() + 1);
    assert_eq!(result.endpoints.last().unwrap().t, 0.25);
    // Frozen 120-digit analytic oracle. Each real/imaginary component is
    // bracketed by adjacent binary64 values, so the check includes oracle
    // conversion error rather than relying on ordinary sin/cos evaluation.
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/production_audit_20261018/evidence/continuation_a02/linear_oracle_mpmath.json"
    ))
    .unwrap();
    let input = &oracle["input_bits"];
    assert_eq!(input["width"].as_u64().unwrap(), 1.0_f64.to_bits());
    for j in 0..xi.len() {
        assert_eq!(input["xi"][j].as_u64().unwrap(), xi[j].to_bits());
        assert_eq!(input["weights"][j].as_u64().unwrap(), weights[j].to_bits());
        assert_eq!(input["z"][j][0].as_u64().unwrap(), z[j].0.to_bits());
        assert_eq!(input["z"][j][1].as_u64().unwrap(), z[j].1.to_bits());
    }
    let rows = oracle["rows"].as_array().unwrap();
    assert_eq!(rows.len(), result.observations.len());
    for (point, row) in result.observations.iter().zip(rows) {
        assert_eq!(row["t_bits"].as_u64().unwrap(), point.t.to_bits());
        for j in 0..xi.len() {
            let re = Interval::new(
                row["modes"][j][0][0].as_f64().unwrap(),
                row["modes"][j][0][1].as_f64().unwrap(),
            )
            .unwrap();
            let im = Interval::new(
                row["modes"][j][1][0].as_f64().unwrap(),
                row["modes"][j][1][1].as_f64().unwrap(),
            )
            .unwrap();
            let center = point.modes.as_ref().unwrap()[j];
            let re_error = re.sub(Interval::point(center.0).unwrap()).unwrap().mag();
            let im_error = im.sub(Interval::point(center.1).unwrap()).unwrap().mag();
            let error = add_up(re_error, im_error).unwrap();
            assert!(
                error <= point.mode_radii.as_ref().unwrap()[j],
                "t={} j={j} error={error:e} radius={:e}",
                point.t,
                point.mode_radii.as_ref().unwrap()[j]
            );
        }
        let mean_re = Interval::new(
            row["mean"][0][0].as_f64().unwrap(),
            row["mean"][0][1].as_f64().unwrap(),
        )
        .unwrap();
        let mean_im = Interval::new(
            row["mean"][1][0].as_f64().unwrap(),
            row["mean"][1][1].as_f64().unwrap(),
        )
        .unwrap();
        let mean_re_error = mean_re
            .sub(Interval::point(point.mean.0).unwrap())
            .unwrap()
            .mag();
        let mean_im_error = mean_im
            .sub(Interval::point(point.mean.1).unwrap())
            .unwrap()
            .mag();
        let mean_error = add_up(mean_re_error, mean_im_error).unwrap();
        assert!(
            mean_error <= point.mean_radius,
            "t={} mean error={mean_error:e} radius={:e}",
            point.t,
            point.mean_radius
        );
    }
    assert!(work.output_value_items > 0);
}

#[test]
fn linear_outward_transports_nonzero_initial_disks() {
    let model = CollectiveModel::new(
        vec![-0.5, 0.5],
        vec![0.5, 0.5],
        vec![(0.25, 0.0), (0.0, 0.125)],
        1.0,
        0.0,
        0.0,
        0.0,
    )
    .unwrap();
    let initial = [1e-6, 2e-6];
    let mut config = TrajectoryConfig::new(0.0, 0.125, 0.0625, 1e-4, 1e-4).unwrap();
    config.output_policy = OutputPolicy::AllModes;
    let mut work = PacketWorkCounters::default();
    let result = run_linear_outward_trajectory(
        &model,
        Some(&initial),
        &config,
        &[0.03125, 0.125],
        &mut work,
    )
    .unwrap();
    for point in &result.observations {
        for (radius, inherited) in point.mode_radii.as_ref().unwrap().iter().zip(initial) {
            assert!(*radius >= inherited);
        }
        assert!(point.mean_radius >= 1.5e-6);
    }
}

#[test]
fn linear_outward_rejections_preserve_attempted_work_and_return_typed_refusal() {
    let model = make_test_model();
    let mut work = PacketWorkCounters::default();
    let config = TrajectoryConfig::new(0.0, 0.125, 0.0625, 1e-5, 1e-5).unwrap();
    assert_eq!(
        run_linear_outward_trajectory(&model, None, &config, &[], &mut work),
        Err(LinearTrajectoryRefusal::UnsupportedModel)
    );
    assert_eq!(work.total_packet_terms(), 0);
    let linear =
        CollectiveModel::new(vec![0.5], vec![1.0], vec![(0.25, 0.0)], 1.0, 0.0, 0.0, 0.0).unwrap();
    let mut tight = config.clone();
    tight.mean_tolerance = 1e-16;
    tight.mode_tolerance = 1e-16;
    tight.min_h = 0.015625;
    tight.max_steps = 8;
    let result = run_linear_outward_trajectory(&linear, Some(&[1e-4]), &tight, &[], &mut work);
    assert!(matches!(
        result,
        Err(LinearTrajectoryRefusal::MinimumStep | LinearTrajectoryRefusal::StepLimit)
    ));
    assert!(
        work.total_packet_terms() > 0,
        "rejected packet work is retained"
    );
    let before = work.total_packet_terms();
    let mut invalid = config;
    invalid.initial_h = f64::NAN;
    assert!(matches!(
        run_linear_outward_trajectory(&linear, None, &invalid, &[], &mut work),
        Err(LinearTrajectoryRefusal::InvalidInput(_))
    ));
    assert!(matches!(
        run_linear_outward_trajectory(&linear, None, &tight, &[f64::INFINITY], &mut work),
        Err(LinearTrajectoryRefusal::InvalidInput(_))
    ));
    assert_eq!(work.total_packet_terms(), before);
    let nonexact = TrajectoryConfig::new(0.0, 1.0, 0.1, 1e-3, 1e-3).unwrap();
    assert_eq!(
        run_linear_outward_trajectory(&linear, None, &nonexact, &[0.9], &mut work),
        Err(LinearTrajectoryRefusal::NonExactTimeGrid)
    );
}

#[test]
fn linear_outward_rejected_trial_does_not_shift_accepted_center() {
    let model =
        CollectiveModel::new(vec![1.0], vec![1.0], vec![(0.25, 0.0)], 1.0, 0.0, 0.0, 0.0).unwrap();
    let mut config = TrajectoryConfig::new(0.0, 0.25, 0.25, 2e-4, 2e-4).unwrap();
    config.order = 2;
    config.degree = 3;
    config.min_h = 0.015625;
    config.output_policy = OutputPolicy::AllModes;
    let mut work = PacketWorkCounters::default();
    let result =
        run_linear_outward_trajectory(&model, None, &config, &[0.125, 0.25], &mut work).unwrap();
    assert!(result.rejected_steps > 0, "large trial must be rejected");
    assert!(result.accepted_steps >= 2);
    assert_eq!(result.work, work);
    let endpoint = result.endpoints.last().unwrap();
    let exact = (0.25 * 0.25_f64.cos(), 0.25 * 0.25_f64.sin());
    let center = endpoint.modes.as_ref().unwrap()[0];
    assert!(
        (exact.0 - center.0).hypot(exact.1 - center.1) <= endpoint.mode_radii.as_ref().unwrap()[0]
    );
    assert_eq!(model.initial_amplitudes()[0], (0.25, 0.0));
}
