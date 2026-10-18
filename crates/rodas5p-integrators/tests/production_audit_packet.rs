//! New adversarial regressions for the 2026-10-18 packet audit.
use rodas5p_integrators::{
    CausalWeights, CollectiveModel, PacketError, PacketWorkCounters, TrajectoryConfig,
    compile_packet, compute_certificate, evaluate_mode_at, generate_candidate, reconstruct_modes,
    run_packet_trajectory,
};

fn one_mode(width: f64, chi: f64, kappa: f64) -> CollectiveModel {
    CollectiveModel::new(
        vec![1.0],
        vec![1.0],
        vec![(0.5, 0.0)],
        width,
        chi,
        0.0,
        kappa,
    )
    .unwrap()
}

#[test]
fn kappa_zero_packet_refuses_damped_model() {
    let mut work = PacketWorkCounters::new();
    assert!(compile_packet(&one_mode(0.0, 0.0, 1.0), 0.1, 4, &mut work).is_err());
}

#[test]
fn dense_output_restricts_normalized_time_waveform() {
    let model = one_mode(0.0, 1.0, 0.0);
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.1, 12, &mut work).unwrap();
    let table = CausalWeights::compile(12, 36);
    let cand = generate_candidate(&packet, 12, 6, &table, &mut work).unwrap();
    // Independent exact one-mode solution a(t)=a0 exp(i chi |a0|^2 t).
    for tau in [0.25, 0.5, 0.75, 1.0] {
        let actual = evaluate_mode_at(&cand, &packet, 0, tau, &mut work).unwrap();
        let phase: f64 = 0.25 * 0.1 * tau;
        let error = (actual.0 - 0.5 * phase.cos()).hypot(actual.1 - 0.5 * phase.sin());
        assert!(error < 1e-12, "tau={tau}, error={error:e}");
    }
}

#[test]
fn trajectory_tail_uses_force_bound_not_already_scaled_model_error() {
    let model = one_mode(0.3, 0.2, 0.0);
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.1, 0, &mut work).unwrap();
    let table = CausalWeights::compile(0, 18);
    let cand = generate_candidate(&packet, 6, 4, &table, &mut work).unwrap();
    let cert = compute_certificate(&cand, &packet, &table, &mut work).unwrap();
    assert!(!cert.roundoff_enclosed);
    let mut config = TrajectoryConfig::new(0.0, 0.1, 0.1, 1.0, 1.0).unwrap();
    config.order = 0;
    config.degree = 6;
    config.sweeps = 4;
    let out = run_packet_trajectory(&model, None, &config, &mut work).unwrap();
    assert!(!out.roundoff_enclosed);
    assert_eq!(out.accepted_steps, 1);
    assert!(out.steps[1].total_mean_bound >= cert.mean_bound * (1.0 - 1e-14));
    assert!(out.final_mode_radii[0] >= cert.mode_bounds[0] * (1.0 - 1e-14));
}

#[test]
fn inherited_uncertainty_must_fit_proved_unit_energy_domain() {
    let model = one_mode(0.0, 1.0, 0.0);
    let h = std::f64::consts::PI / (100.5_f64 * 100.5);
    let config = TrajectoryConfig::new(0.0, h, h, 200.0, 200.0).unwrap();
    let mut work = PacketWorkCounters::new();
    assert!(run_packet_trajectory(&model, Some(&[100.0]), &config, &mut work).is_err());
}

#[test]
fn nonfinite_candidate_cannot_receive_certificate() {
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&one_mode(0.0, 0.0, 0.0), 0.1, 2, &mut work).unwrap();
    let table = CausalWeights::compile(2, 6);
    let mut cand = generate_candidate(&packet, 2, 1, &table, &mut work).unwrap();
    cand.mean[0].0 = f64::NAN;
    assert!(matches!(
        compute_certificate(&cand, &packet, &table, &mut work),
        Err(PacketError::NonFinite("coefficient"))
    ));
    cand.mean[0].0 = f64::INFINITY;
    assert!(matches!(
        compute_certificate(&cand, &packet, &table, &mut work),
        Err(PacketError::NonFinite("coefficient"))
    ));
}

#[test]
fn coefficient_overflow_is_refused_and_subnormal_stays_diagnostic_only() {
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&one_mode(0.0, 0.0, 0.0), 0.1, 2, &mut work).unwrap();
    let table = CausalWeights::compile(2, 6);
    let mut candidate = generate_candidate(&packet, 2, 1, &table, &mut work).unwrap();
    candidate.mean[0] = (f64::MAX, f64::MAX);
    assert!(matches!(
        compute_certificate(&candidate, &packet, &table, &mut work),
        Err(PacketError::NonFinite("candidate domain bound"))
    ));

    candidate.mean[0] = (f64::from_bits(1), 0.0);
    let certificate = compute_certificate(&candidate, &packet, &table, &mut work).unwrap();
    assert!(!certificate.roundoff_enclosed);
}

#[test]
fn subunit_weight_does_not_shrink_the_free_mode_error_bound() {
    let weight = 1.0 - 5.0e-13;
    let model = CollectiveModel::new(
        vec![1.0],
        vec![weight],
        vec![(0.5, 0.0)],
        1.0,
        0.0,
        0.0,
        0.0,
    )
    .unwrap();
    let h = 1.0e-8;
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, h, 0, &mut work).unwrap();
    let table = CausalWeights::compile(0, 2);
    let candidate = generate_candidate(&packet, 0, 1, &table, &mut work).unwrap();
    let certificate = compute_certificate(&candidate, &packet, &table, &mut work).unwrap();
    let actual = evaluate_mode_at(&candidate, &packet, 0, 1.0, &mut work).unwrap();
    let exact = (0.5 * h.cos(), 0.5 * h.sin());
    let error = (actual.0 - exact.0).hypot(actual.1 - exact.1);

    assert!(
        error <= certificate.mode_bounds[0],
        "mode error {error:e} exceeds reported bound {:e}",
        certificate.mode_bounds[0]
    );
}

#[test]
fn subunit_weight_inherited_radius_uses_microscopic_lipschitz_factor() {
    let weight = 1.0 - 5.0e-13;
    let model = CollectiveModel::new(
        vec![0.0],
        vec![weight],
        vec![(0.0, 0.0)],
        0.0,
        0.3,
        0.0,
        0.0,
    )
    .unwrap();
    let h = 0.5;
    let initial_radius = 0.9;
    let mut config = TrajectoryConfig::new(0.0, h, h, 10.0, 10.0).unwrap();
    config.order = 0;
    config.degree = 0;
    config.sweeps = 1;
    let mut work = PacketWorkCounters::new();
    let result =
        run_packet_trajectory(&model, Some(&[initial_radius]), &config, &mut work).unwrap();
    assert_eq!(result.accepted_steps, 1);

    let packet = compile_packet(&model, h, 0, &mut work).unwrap();
    let table = CausalWeights::compile(0, 0);
    let candidate = generate_candidate(&packet, 0, 1, &table, &mut work).unwrap();
    let certificate = compute_certificate(&candidate, &packet, &table, &mut work).unwrap();
    let inherited_mean = weight * initial_radius;
    let mean_denom = 1.0 - h * certificate.lipschitz_l;
    let expected = certificate.mode_bounds[0]
        + h * certificate.microscopic_lipschitz_l * inherited_mean / mean_denom
        + initial_radius;
    let old_wrong_transfer = certificate.mode_bounds[0]
        + h * certificate.lipschitz_l * inherited_mean / mean_denom
        + initial_radius;
    assert_eq!(result.final_mode_radii[0].to_bits(), expected.to_bits());
    assert!(result.final_mode_radii[0] > old_wrong_transfer);
}

#[test]
fn public_packet_descriptor_mutation_is_refused_by_certificate_and_output() {
    let mut work = PacketWorkCounters::new();
    let model = one_mode(0.2, 0.1, 0.0);
    let mut packet = compile_packet(&model, 0.1, 2, &mut work).unwrap();
    let table = CausalWeights::compile(2, 6);
    let cand = generate_candidate(&packet, 2, 2, &table, &mut work).unwrap();
    packet.kernel[0].0 = f64::from_bits(packet.kernel[0].0.to_bits() + 1);

    assert!(compute_certificate(&cand, &packet, &table, &mut work).is_err());
    assert!(reconstruct_modes(&cand, &packet, &mut work).is_err());
    assert!(evaluate_mode_at(&cand, &packet, 0, 0.5, &mut work).is_err());
}

#[test]
fn model_identity_compares_exact_float_bits_including_signed_zero() {
    let mut work = PacketWorkCounters::new();
    let model = one_mode(0.2, 0.1, 0.0);
    let mut packet = compile_packet(&model, 0.1, 2, &mut work).unwrap();
    let table = CausalWeights::compile(2, 6);
    let cand = generate_candidate(&packet, 2, 2, &table, &mut work).unwrap();
    packet.model =
        CollectiveModel::new(vec![1.0], vec![1.0], vec![(0.5, 0.0)], 0.2, 0.1, -0.0, 0.0).unwrap();

    assert!(compute_certificate(&cand, &packet, &table, &mut work).is_err());
    assert!(evaluate_mode_at(&cand, &packet, 0, 0.0, &mut work).is_err());
}

#[test]
fn candidate_cannot_be_reused_with_a_different_compiled_packet() {
    let mut work = PacketWorkCounters::new();
    let model = one_mode(0.2, 0.1, 0.0);
    let packet_a = compile_packet(&model, 0.1, 2, &mut work).unwrap();
    let packet_b = compile_packet(&model, 0.11, 2, &mut work).unwrap();
    let table = CausalWeights::compile(2, 6);
    let cand = generate_candidate(&packet_a, 2, 2, &table, &mut work).unwrap();

    assert!(compute_certificate(&cand, &packet_b, &table, &mut work).is_err());
    assert!(reconstruct_modes(&cand, &packet_b, &mut work).is_err());
    assert!(evaluate_mode_at(&cand, &packet_b, 0, 0.0, &mut work).is_err());
}

#[test]
fn certificate_uses_actual_weight_sum_and_spectral_support_and_charges_work() {
    let model = CollectiveModel::new(
        vec![0.25, -0.5],
        vec![0.4, 0.6000000000005],
        vec![(0.2, 0.0), (0.1, 0.0)],
        0.8,
        0.05,
        0.0,
        0.0,
    )
    .unwrap();
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.1, 2, &mut work).unwrap();
    let table = CausalWeights::compile(2, 12);
    let candidate = generate_candidate(&packet, 4, 2, &table, &mut work).unwrap();
    let cert = compute_certificate(&candidate, &packet, &table, &mut work).unwrap();
    let _ = reconstruct_modes(&candidate, &packet, &mut work).unwrap();

    let s = 1.0000000000005_f64;
    let expected_bare_tail = (0.8_f64 * 0.5 * 0.1).powi(3) / 6.0;
    let expected_kernel_tail = expected_bare_tail * s;
    let expected_free_tail = expected_bare_tail * (0.4 * 0.2 + 0.6000000000005 * 0.1);
    assert!((cert.bare_tail - expected_bare_tail).abs() <= 2.0 * f64::EPSILON * expected_bare_tail);
    assert!(
        (cert.kernel_tail - expected_kernel_tail).abs()
            <= 2.0 * f64::EPSILON * expected_kernel_tail
    );
    assert!((cert.tail - expected_kernel_tail).abs() <= 2.0 * f64::EPSILON * expected_kernel_tail);
    assert!((cert.free_tail - expected_free_tail).abs() <= 2.0 * f64::EPSILON * expected_free_tail);
    assert!(!cert.roundoff_enclosed);
    assert!(work.identity_snapshot_items > 0);
    assert!(work.identity_comparison_items > 0);
    assert!(work.model_copy_items > 0);
    assert!(work.certificate_items > 0);
    assert!(work.reconstruction_terms > 0);
    assert!(work.output_value_items >= 4);
}

#[test]
fn multimode_high_precision_linear_oracle_is_close_at_intermediate_and_endpoints() {
    // 100-digit mpmath oracle for the exact binary inputs below. This checks
    // the diagnostic center only; no center/radius API exists yet, so it does
    // not establish certified inclusion.
    let model = CollectiveModel::new(
        vec![0.25, -0.75],
        vec![0.25, 0.75],
        vec![(0.2, 0.1), (-0.15, 0.05)],
        0.8,
        0.0,
        0.0,
        0.0,
    )
    .unwrap();
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.2, 3, &mut work).unwrap();
    let table = CausalWeights::compile(3, 12);
    let candidate = generate_candidate(&packet, 6, 3, &table, &mut work).unwrap();
    let certificate = compute_certificate(&candidate, &packet, &table, &mut work).unwrap();
    assert!(!certificate.roundoff_enclosed);
    let reference = [
        [
            (0.198_990_016_749_916_4, 0.10199496670849986),
            (-0.14843273005222334, 0.05447682671782373),
        ],
        [
            (0.19796013466398225, 0.10397973400532439),
            (-0.1467318806663084, 0.0589046279686769),
        ],
        [
            (0.19584108791353217, 0.10791787750342463),
            (-0.14293568501363397, 0.06759726288603122),
        ],
    ];
    for (time_index, tau) in [0.25, 0.5, 1.0].into_iter().enumerate() {
        for (mode, expected) in reference[time_index].iter().enumerate() {
            let actual = evaluate_mode_at(&candidate, &packet, mode, tau, &mut work).unwrap();
            let error = (actual.0 - expected.0).hypot(actual.1 - expected.1);
            assert!(error <= certificate.mode_bounds[mode]);
        }
    }
}

#[test]
fn mutable_config_is_validated_at_run_boundary() {
    let mut config = TrajectoryConfig::new(0.0, 0.1, 0.1, 1e-6, 1e-6).unwrap();
    config.t_end = f64::NAN;
    let mut work = PacketWorkCounters::new();
    assert!(run_packet_trajectory(&one_mode(0.0, 0.0, 0.0), None, &config, &mut work).is_err());
}

#[test]
fn tiny_nonempty_interval_is_actually_advanced() {
    let h = 1e-15;
    let mut config = TrajectoryConfig::new(0.0, h, h, 1e-6, 1e-6).unwrap();
    config.min_h = 1e-16;
    let mut work = PacketWorkCounters::new();
    let out = run_packet_trajectory(&one_mode(0.0, 0.0, 0.0), None, &config, &mut work).unwrap();
    assert_eq!(out.accepted_steps, 1);
    assert_eq!(out.final_t, h);
}

#[test]
fn admitted_small_uncertainty_remains_supported() {
    let model = one_mode(0.0, 0.0, 0.0);
    let config = TrajectoryConfig::new(0.0, 0.1, 0.1, 1e-3, 1e-3).unwrap();
    let mut work = PacketWorkCounters::new();
    let out = run_packet_trajectory(&model, Some(&[1e-4]), &config, &mut work).unwrap();
    assert_eq!(out.final_modes[0], (0.5, 0.0));
    assert!(out.final_mode_radii[0] >= 1e-4);
}
