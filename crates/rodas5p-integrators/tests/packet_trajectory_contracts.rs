use rodas5p_core::CoreError;
use rodas5p_integrators::collective_model::CollectiveModel;
use rodas5p_integrators::packet_trajectory::{
    OutputPolicy, TrajectoryConfig, run_packet_trajectory,
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
