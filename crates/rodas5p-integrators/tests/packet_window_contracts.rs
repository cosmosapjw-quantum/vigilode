use rodas5p_integrators::{
    CausalWeights, CollectiveModel, PacketError, PacketWorkCounters, compile_packet,
    compute_certificate, evaluate_mode_at, generate_candidate, reconstruct_modes,
};

fn sample_packet_model() -> CollectiveModel {
    let xi = vec![-0.75, -0.25, 0.25, 0.75];
    let weights = vec![0.25, 0.25, 0.25, 0.25];
    let initial = vec![(0.3, 0.2), (0.2, -0.3), (-0.2, 0.3), (0.1, 0.1)];
    CollectiveModel::new(xi, weights, initial, 1.2, 0.2, 0.0, 0.0).expect("valid model")
}

#[test]
fn test_vg1_zero_online_solver_counters_contract() {
    let model = sample_packet_model();
    let mut work = PacketWorkCounters::new();

    let packet = compile_packet(&model, 0.2, 6, &mut work).expect("compile packet");
    let table = CausalWeights::compile(packet.order(), 10);
    let candidate =
        generate_candidate(&packet, 10, 4, &table, &mut work).expect("generate candidate");
    let cert = compute_certificate(&candidate, &packet, &table, &mut work).expect("compute cert");
    let _modes = reconstruct_modes(&candidate, &packet, &mut work).unwrap();

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
    assert!(
        work.total_packet_terms() > 0,
        "total packet terms must be positive"
    );
    assert!(cert.mean_bound > 0.0 && cert.mean_bound.is_finite());
}

#[test]
fn test_vg2_picard_sweeps_convergence_and_defect_reduction() {
    let model = sample_packet_model();
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.15, 6, &mut work).expect("compile packet");
    let table = CausalWeights::compile(packet.order(), 12);

    let mut prev_defect = f64::INFINITY;
    for sweeps in 1..=5 {
        let cand = generate_candidate(&packet, 10, sweeps, &table, &mut work).unwrap();
        let cert = compute_certificate(&cand, &packet, &table, &mut work).unwrap();

        assert!(
            cert.volterra_defect <= prev_defect + 1e-12,
            "Volterra defect must decrease with Picard sweeps: sweep {sweeps} defect {} > prev {}",
            cert.volterra_defect,
            prev_defect
        );
        prev_defect = cert.volterra_defect;
    }
}

#[test]
fn test_vg2_dahlquist_linear_boundary_zero_defect() {
    // When chi = 0, the nonlinear force is zero, so the exact Picard solution
    // is simply the free linear evolution.
    let xi = vec![-0.5, 0.5];
    let weights = vec![0.5, 0.5];
    let initial = vec![(0.4, 0.0), (0.0, 0.4)];
    let model = CollectiveModel::new(xi, weights, initial, 1.0, 0.0, 0.0, 0.0).unwrap();

    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.2, 4, &mut work).unwrap();
    let table = CausalWeights::compile(packet.order(), 8);

    let cand = generate_candidate(&packet, 8, 2, &table, &mut work).unwrap();
    let cert = compute_certificate(&cand, &packet, &table, &mut work).unwrap();

    // Defect should be zero to machine precision
    assert!(
        cert.volterra_defect < 1e-15,
        "Dahlquist linear boundary defect must be 0, got {}",
        cert.volterra_defect
    );
}

#[test]
fn test_vg3_reconstruction_initial_state_consistency() {
    let model = sample_packet_model();
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.2, 6, &mut work).unwrap();
    let table = CausalWeights::compile(packet.order(), 10);
    let cand = generate_candidate(&packet, 10, 4, &table, &mut work).unwrap();

    // At tau = 0, evaluate_mode_at must match initial amplitudes z_j exactly
    for (j, &z0) in model.initial_amplitudes().iter().enumerate() {
        let val_at_0 = evaluate_mode_at(&cand, &packet, j, 0.0, &mut work).unwrap();
        let diff_u = (val_at_0.0 - z0.0).abs();
        let diff_v = (val_at_0.1 - z0.1).abs();
        assert!(
            diff_u < 1e-13 && diff_v < 1e-13,
            "Reconstruction at tau=0 mismatch for mode {j}: got {val_at_0:?}, expected {z0:?}"
        );
    }
}

#[test]
fn test_vg3_certificate_mode_bounds_enclosure_consistency() {
    let model = sample_packet_model();
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.1, 6, &mut work).unwrap();
    let table = CausalWeights::compile(packet.order(), 10);
    let cand = generate_candidate(&packet, 10, 4, &table, &mut work).unwrap();
    let cert = compute_certificate(&cand, &packet, &table, &mut work).unwrap();

    assert_eq!(cert.mode_bounds.len(), model.num_modes());
    for (j, &bound) in cert.mode_bounds.iter().enumerate() {
        assert!(
            bound > 0.0 && bound.is_finite(),
            "Mode bound {j} must be strictly positive and finite, got {bound}"
        );
        let z = model.initial_amplitudes()[j];
        let z_upper = z.0.abs() + z.1.abs();
        assert!(
            bound >= cert.tail * z_upper,
            "Mode bound {j} ({bound}) must be >= tail * |z_j| ({})",
            cert.tail * z_upper
        );
    }
}

#[test]
fn test_negative_refusal_contracts() {
    let model = sample_packet_model();
    let mut work = PacketWorkCounters::new();

    // 1. Negative or zero horizon
    let res = compile_packet(&model, 0.0, 4, &mut work);
    assert!(matches!(res, Err(PacketError::InvalidHorizon(_))));
    let res = compile_packet(&model, -0.5, 4, &mut work);
    assert!(matches!(res, Err(PacketError::InvalidHorizon(_))));

    // 2. Degree < order
    let packet = compile_packet(&model, 0.1, 6, &mut work).unwrap();
    let table = CausalWeights::compile(packet.order(), 10);
    let res = generate_candidate(&packet, 4, 3, &table, &mut work);
    assert!(matches!(res, Err(PacketError::DegreeLessThanOrder { .. })));

    // 3. Sweeps == 0
    let res = generate_candidate(&packet, 8, 0, &table, &mut work);
    assert_eq!(res, Err(PacketError::ZeroSweeps));

    // 4. Gronwall condition violated: h * L >= 1.0
    // With chi = 0.2, L = 0.6. Choose h = 2.0 -> h * L = 1.2 >= 1.0.
    let packet_large_h = compile_packet(&model, 2.0, 4, &mut work).unwrap();
    let table_large = CausalWeights::compile(packet_large_h.order(), 10);
    let cand = generate_candidate(&packet_large_h, 8, 2, &table_large, &mut work).unwrap();
    let res = compute_certificate(&cand, &packet_large_h, &table_large, &mut work);
    assert!(matches!(
        res,
        Err(PacketError::GronwallConditionViolated { .. })
    ));

    // 5. evaluate_mode_at out of bounds
    let res = evaluate_mode_at(&cand, &packet_large_h, 999, 0.5, &mut work);
    assert!(res.is_err());
}
