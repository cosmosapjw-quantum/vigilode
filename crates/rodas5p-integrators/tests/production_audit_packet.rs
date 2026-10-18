//! New adversarial regressions for the 2026-10-18 packet audit.
use rodas5p_integrators::{
    CausalWeights, CollectiveModel, PacketWorkCounters, TrajectoryConfig, compile_packet,
    compute_certificate, evaluate_mode_at, generate_candidate, reconstruct_modes,
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
    assert!(compute_certificate(&cand, &packet, &table, &mut work).is_err());
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
