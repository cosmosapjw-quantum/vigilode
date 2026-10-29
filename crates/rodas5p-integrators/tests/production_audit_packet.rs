//! New adversarial regressions for the 2026-10-18 packet audit.
use rodas5p_core::directed::Interval;
use rodas5p_core::directed::add_up;
use rodas5p_integrators::packet_window::outward_polynomial_sup_bound;
use rodas5p_integrators::{
    CausalWeights, CollectiveModel, OutwardPacketRequest, PacketError, PacketWorkCounters,
    TrajectoryConfig, compile_packet, compute_certificate, evaluate_mode_at, generate_candidate,
    reconstruct_modes, reconstruct_outward_at_times, run_packet_trajectory,
};

fn polynomial_intervals(coefficients: &[(f64, f64)]) -> Vec<(Interval, Interval)> {
    coefficients
        .iter()
        .map(|&(re, im)| (Interval::point(re).unwrap(), Interval::point(im).unwrap()))
        .collect()
}

#[test]
fn bernstein_false_rejection_savings_for_exact_one_minus_tau_power() {
    // (1-tau)^12 has coefficient L1 bound 4096 and exact supremum 1.
    let binomial = [
        1.0, 12.0, 66.0, 220.0, 495.0, 792.0, 924.0, 792.0, 495.0, 220.0, 66.0, 12.0, 1.0,
    ];
    let coefficients: Vec<_> = binomial
        .iter()
        .enumerate()
        .map(|(k, &v)| (if k % 2 == 0 { v } else { -v }, 0.0))
        .collect();
    let mut work = PacketWorkCounters::default();
    let bound =
        outward_polynomial_sup_bound(&polynomial_intervals(&coefficients), 1.01, 0, &mut work)
            .unwrap();
    assert!(bound.coefficient_upper >= 4096.0);
    assert!(
        bound.upper <= 1.01,
        "outward transformed bound={:e}",
        bound.upper
    );
    assert!(bound.transform_used);
    assert_eq!(bound.refinement_levels, 0);
    assert!(work.bernstein_transform_terms > 0);
    assert_eq!(work.bernstein_refinement_terms, 0);
}

#[test]
fn bernstein_complex_inclusion_and_dyadic_refinement_stop() {
    // p(tau) = tau(1-tau) + i*tau/4. Each sampled complex modulus
    // is below the verified L1 control-polygon bound for the full interval.
    let coefficients = polynomial_intervals(&[(0.0, 0.0), (1.0, 0.25), (-1.0, 0.0)]);
    let mut work = PacketWorkCounters::default();
    let bound = outward_polynomial_sup_bound(&coefficients, 0.4, 4, &mut work).unwrap();
    assert!(bound.transform_used);
    assert!(bound.refinement_levels > 0 && bound.refinement_levels <= 4);
    assert!(bound.upper <= 0.4);
    assert!(work.bernstein_refinement_terms > 0);
    for k in 0..=256 {
        let t = k as f64 / 256.0;
        let value = (t * (1.0 - t), t / 4.0);
        assert!(value.0.hypot(value.1) <= bound.upper);
    }
    let spent = work.bernstein_transform_terms;
    let trivial = outward_polynomial_sup_bound(&coefficients, 4.0, 4, &mut work).unwrap();
    assert!(!trivial.transform_used);
    assert_eq!(work.bernstein_transform_terms, spent);
}

#[test]
fn bernstein_componentwise_bounds_enclose_complex_vector_l1_norm() {
    let first = polynomial_intervals(&[(1.0, 0.0), (-2.0, 0.0), (1.0, 0.0)]);
    let second = polynomial_intervals(&[(0.0, 0.0), (0.0, 1.0), (0.0, -1.0)]);
    let mut work = PacketWorkCounters::default();
    let first_bound = outward_polynomial_sup_bound(&first, 1.01, 0, &mut work).unwrap();
    let second_bound = outward_polynomial_sup_bound(&second, 0.3, 2, &mut work).unwrap();
    let vector_upper = add_up(first_bound.upper, second_bound.upper).unwrap();
    for k in 0..=256 {
        let t = k as f64 / 256.0;
        let vector_l1 = (1.0 - t).powi(2) + t * (1.0 - t);
        assert!(vector_l1 <= vector_upper);
    }
}

#[test]
fn bernstein_interval_coefficients_include_perturbed_polynomials() {
    let eps = 1e-6;
    let coefficients = [
        (
            Interval::new(1.0 - eps, 1.0 + eps).unwrap(),
            Interval::new(-eps, eps).unwrap(),
        ),
        (
            Interval::new(-2.0 - eps, -2.0 + eps).unwrap(),
            Interval::new(0.25 - eps, 0.25 + eps).unwrap(),
        ),
        (
            Interval::new(1.0 - eps, 1.0 + eps).unwrap(),
            Interval::new(-eps, eps).unwrap(),
        ),
    ];
    let mut work = PacketWorkCounters::default();
    let bound = outward_polynomial_sup_bound(&coefficients, 1.3, 4, &mut work).unwrap();
    assert!(bound.transform_used);
    for k in 0..=256 {
        let t = k as f64 / 256.0;
        for signs in [-1.0, 1.0] {
            let re = (1.0 + signs * eps) + (-2.0 + signs * eps) * t + (1.0 + signs * eps) * t * t;
            let im = signs * eps + (0.25 + signs * eps) * t + signs * eps * t * t;
            assert!(re.hypot(im) <= bound.upper);
        }
    }
}

#[test]
fn bernstein_typed_budget_refusal_preserves_work() {
    let coefficients = polynomial_intervals(&[(0.0, 0.0), (1.0, 0.0), (-1.0, 0.0)]);
    let mut work = PacketWorkCounters::default();
    let error = outward_polynomial_sup_bound(&coefficients, 0.2, 2, &mut work).unwrap_err();
    assert!(matches!(
        error,
        PacketError::BernsteinBudgetExhausted { .. }
    ));
    assert!(work.bernstein_transform_terms > 0);
    assert!(work.bernstein_refinement_terms > 0);
}

#[test]
fn bernstein_rejects_public_invalid_interval_before_work() {
    let coefficients = [(
        Interval {
            lo: f64::NAN,
            hi: 1.0,
        },
        Interval::point(0.0).unwrap(),
    )];
    let mut work = PacketWorkCounters::default();
    let result = outward_polynomial_sup_bound(&coefficients, 1.0, 1, &mut work);
    assert!(matches!(result, Err(PacketError::OutwardRefusal(_))));
    assert_eq!(work.total_packet_terms(), 0);
}

#[test]
fn bernstein_subdivision_overflow_retains_attempted_refinement_work() {
    let huge = 2.0_f64.powi(1023);
    let coefficients = polynomial_intervals(&[(huge, 0.0), (0.0, 0.0)]);
    let mut work = PacketWorkCounters::default();
    let result = outward_polynomial_sup_bound(&coefficients, 1.0, 1, &mut work);
    assert!(matches!(result, Err(PacketError::OutwardRefusal(_))));
    assert!(work.bernstein_transform_terms > 0);
    assert!(work.bernstein_refinement_terms > 0);
}

#[test]
fn bernstein_candidate_gate_recovers_a_real_linear_packet() {
    let model = one_mode(1.0, 0.0, 0.0);
    let mut work = PacketWorkCounters::default();
    let packet = compile_packet(&model, 2.0, 8, &mut work).unwrap();
    let table = CausalWeights::compile(8, 24);
    let candidate = generate_candidate(&packet, 8, 1, &table, &mut work).unwrap();
    let coefficient_sum: f64 = candidate.mean.iter().map(|&(re, im)| re.hypot(im)).sum();
    assert!(coefficient_sum > 1.0);
    let certificate = compute_certificate(&candidate, &packet, &table, &mut work).unwrap();
    assert!(certificate.candidate_domain_bound <= 1.0);
    assert!(work.bernstein_transform_terms > 0);
    assert!(!certificate.roundoff_enclosed);
}

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
fn outward_linear_physical_times_enclose_returned_center_at_midpoint_and_endpoint() {
    let model = one_mode(0.3, 0.0, 0.0);
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.1, 8, &mut work).unwrap();
    let after_setup = work;
    let table = CausalWeights::compile(8, 24);
    let cand = generate_candidate(&packet, 8, 2, &table, &mut work).unwrap();
    let after_candidate = work;
    let out = reconstruct_outward_at_times(
        &cand,
        &packet,
        OutwardPacketRequest {
            times: &[0.05, 0.1],
            initial_radii: &[0.0],
        },
        &mut work,
    )
    .unwrap();
    eprintln!(
        "A01_COUNTER_RECEIPT setup={after_setup:?} candidate={after_candidate:?} outward={work:?}"
    );
    assert_eq!(out.modes.len(), 2);
    assert_eq!(
        work.outward_spectral_power_terms - after_candidate.outward_spectral_power_terms,
        8
    );
    for i in 0..2 {
        assert!(out.mode_radii[i].is_finite() && out.mode_radii[i] >= 0.0);
    }
    assert!(!out.roundoff_enclosed);
}

#[test]
fn outward_nonlinear_physical_reconstruction_returns_finite_midpoint_and_endpoint() {
    let model = one_mode(0.0, 1.0, 0.0);
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.01, 8, &mut work).unwrap();
    let table = CausalWeights::compile(8, 24);
    let cand = generate_candidate(&packet, 8, 5, &table, &mut work).unwrap();
    let out = reconstruct_outward_at_times(
        &cand,
        &packet,
        OutwardPacketRequest {
            times: &[0.005, 0.01],
            initial_radii: &[0.0],
        },
        &mut work,
    )
    .unwrap();
    assert!(
        out.modes
            .iter()
            .all(|&(re, im)| re.is_finite() && im.is_finite())
    );
    assert!(out.mode_radii.iter().all(|r| r.is_finite() && *r >= 0.0));
    assert!(out.mean_radii.iter().all(|r| r.is_finite() && *r >= 0.0));
}

#[test]
fn outward_one_mode_cubic_radius_contains_exact_midpoint_and_endpoint() {
    let model = one_mode(0.2, 0.7, 0.0);
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.01, 10, &mut work).unwrap();
    let table = CausalWeights::compile(10, 30);
    let cand = generate_candidate(&packet, 10, 6, &table, &mut work).unwrap();
    let times = [0.005, 0.01];
    let out = reconstruct_outward_at_times(
        &cand,
        &packet,
        OutwardPacketRequest {
            times: &times,
            initial_radii: &[0.0],
        },
        &mut work,
    )
    .unwrap();
    for (i, &t) in times.iter().enumerate() {
        let phase = (0.2 + 0.7 * 0.25) * t;
        let exact = (0.5 * phase.cos(), 0.5 * phase.sin());
        let distance = (out.modes[i].0 - exact.0).hypot(out.modes[i].1 - exact.1);
        assert!(
            distance <= out.mode_radii[i],
            "t={t}, center={:?}, exact={exact:?}, radius={:e}, mean={:?}, mean_radius={:e}, candidate={:?}",
            out.modes[i],
            out.mode_radii[i],
            out.means[i],
            out.mean_radii[i],
            cand.mean
        );
    }
}

#[test]
fn outward_multimode_encloses_frozen_mpmath_reference_at_midpoint_and_endpoint() {
    let model = CollectiveModel::new(
        vec![-0.4, 0.7],
        vec![0.35, 0.65],
        vec![(0.3, 0.1), (-0.2, 0.25)],
        0.2,
        0.1,
        0.0,
        0.0,
    )
    .unwrap();
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.01, 10, &mut work).unwrap();
    let table = CausalWeights::compile(10, 30);
    let cand = generate_candidate(&packet, 10, 6, &table, &mut work).unwrap();
    let before_power_terms = work.outward_spectral_power_terms;
    let out = reconstruct_outward_at_times(
        &cand,
        &packet,
        OutwardPacketRequest {
            times: &[0.005, 0.01],
            initial_radii: &[0.0, 0.0],
        },
        &mut work,
    )
    .unwrap();
    assert_eq!(work.outward_spectral_power_terms - before_power_terms, 20);
    let reference = [
        [
            (0.30003606598313826, 0.099_879_496_665_868_6),
            (-0.20017886072855837, 0.24985944127300486),
        ],
        [
            (0.300_072_091_095_823_3, 0.09975897745945944),
            (-0.20035761568763077, 0.24971875592645612),
        ],
    ];
    for (t, row) in reference.iter().enumerate() {
        for (j, &value) in row.iter().enumerate() {
            let i = t * 2 + j;
            let d = (out.modes[i].0 - value.0).hypot(out.modes[i].1 - value.1);
            assert!(
                d <= out.mode_radii[i],
                "t={t} j={j} d={d:e} r={:e}",
                out.mode_radii[i]
            );
        }
    }
    for (t, row) in reference.iter().enumerate() {
        let mean = (
            0.35 * row[0].0 + 0.65 * row[1].0,
            0.35 * row[0].1 + 0.65 * row[1].1,
        );
        let d = (out.means[t].0 - mean.0).hypot(out.means[t].1 - mean.1);
        assert!(
            d <= out.mean_radii[t],
            "mean t={t} d={d:e} r={:e}",
            out.mean_radii[t]
        );
    }
}

#[test]
fn outward_initial_mode_radii_are_transport_included() {
    let model = one_mode(0.0, 0.2, 0.0);
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.01, 8, &mut work).unwrap();
    let table = CausalWeights::compile(8, 24);
    let cand = generate_candidate(&packet, 8, 5, &table, &mut work).unwrap();
    let out = reconstruct_outward_at_times(
        &cand,
        &packet,
        OutwardPacketRequest {
            times: &[0.01],
            initial_radii: &[1e-4],
        },
        &mut work,
    )
    .unwrap();
    assert!(out.mode_radii[0] >= 1e-4);
}

#[test]
fn outward_packet_refuses_nonfinite_uncertainty() {
    let model = one_mode(0.0, 0.0, 0.0);
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.01, 4, &mut work).unwrap();
    let table = CausalWeights::compile(4, 12);
    let cand = generate_candidate(&packet, 4, 1, &table, &mut work).unwrap();
    assert!(matches!(
        reconstruct_outward_at_times(
            &cand,
            &packet,
            OutwardPacketRequest {
                times: &[0.01],
                initial_radii: &[f64::NAN]
            },
            &mut work
        ),
        Err(PacketError::OutwardRefusal(_))
    ));
}

#[test]
fn outward_packet_refuses_nonfinite_physical_time() {
    let model = one_mode(0.0, 0.0, 0.0);
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.01, 4, &mut work).unwrap();
    let table = CausalWeights::compile(4, 12);
    let cand = generate_candidate(&packet, 4, 1, &table, &mut work).unwrap();
    assert!(matches!(
        reconstruct_outward_at_times(
            &cand,
            &packet,
            OutwardPacketRequest {
                times: &[f64::INFINITY],
                initial_radii: &[0.0]
            },
            &mut work
        ),
        Err(PacketError::OutwardRefusal(_))
    ));
}

#[test]
fn outward_partial_time_request_refusal_keeps_first_output_work() {
    let model = one_mode(0.0, 0.0, 0.0);
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.01, 4, &mut work).unwrap();
    let table = CausalWeights::compile(4, 12);
    let cand = generate_candidate(&packet, 4, 1, &table, &mut work).unwrap();
    let before = work;
    assert!(matches!(
        reconstruct_outward_at_times(
            &cand,
            &packet,
            OutwardPacketRequest {
                times: &[0.005, f64::NAN],
                initial_radii: &[0.0]
            },
            &mut work
        ),
        Err(PacketError::OutwardRefusal(_))
    ));
    assert!(work.output_value_items > before.output_value_items);
    assert!(work.output_copy_items > before.output_copy_items);
    assert!(work.reconstruction_terms > before.reconstruction_terms);
}

#[test]
fn outward_overflow_refusal_preserves_attempted_certificate_work() {
    let model = one_mode(0.0, 0.1, 0.0);
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.01, 4, &mut work).unwrap();
    let table = CausalWeights::compile(4, 12);
    let cand = generate_candidate(&packet, 4, 2, &table, &mut work).unwrap();
    let before = work.certificate_items;
    assert!(matches!(
        reconstruct_outward_at_times(
            &cand,
            &packet,
            OutwardPacketRequest {
                times: &[0.01],
                initial_radii: &[f64::MAX]
            },
            &mut work
        ),
        Err(PacketError::OutwardRefusal(_))
    ));
    assert!(work.certificate_items > before);
}

#[test]
fn outward_refuses_overflowing_candidate_degree_before_allocation() {
    let model = one_mode(0.0, 0.0, 0.0);
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.01, 4, &mut work).unwrap();
    let table = CausalWeights::compile(4, 12);
    let mut cand = generate_candidate(&packet, 4, 1, &table, &mut work).unwrap();
    cand.degree = usize::MAX;
    assert!(matches!(
        reconstruct_outward_at_times(
            &cand,
            &packet,
            OutwardPacketRequest {
                times: &[0.01],
                initial_radii: &[0.0]
            },
            &mut work
        ),
        Err(PacketError::OutwardRefusal(_))
    ));
}

#[test]
fn packet_compiler_and_candidate_refuse_usize_max_before_allocation() {
    let model = one_mode(0.0, 0.0, 0.0);
    let mut work = PacketWorkCounters::new();
    assert!(matches!(
        compile_packet(&model, 0.01, usize::MAX, &mut work),
        Err(PacketError::SizeOverflow(_))
    ));
    let packet = compile_packet(&model, 0.01, 4, &mut work).unwrap();
    let table = CausalWeights::compile(4, 12);
    assert!(matches!(
        generate_candidate(&packet, usize::MAX, 1, &table, &mut work),
        Err(PacketError::SizeOverflow(_))
    ));
}

#[test]
fn packet_compiler_and_candidate_refuse_unallocatable_but_nonoverflowing_lengths() {
    let model = one_mode(0.0, 0.0, 0.0);
    let mut work = PacketWorkCounters::new();
    // Both +1 operations fit usize. A Vec cannot represent these capacities.
    assert!(matches!(
        compile_packet(&model, 0.01, usize::MAX - 1, &mut work),
        Err(PacketError::SizeOverflow(_))
    ));
    let packet = compile_packet(&model, 0.01, 4, &mut work).unwrap();
    let table = CausalWeights::compile(4, 12);
    assert!(matches!(
        generate_candidate(&packet, usize::MAX - 1, 1, &table, &mut work),
        Err(PacketError::SizeOverflow(_))
    ));
}

#[test]
fn outward_subnormal_physical_time_is_finite_or_typed_refusal() {
    let model = one_mode(0.0, 0.0, 0.0);
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.01, 4, &mut work).unwrap();
    let table = CausalWeights::compile(4, 12);
    let cand = generate_candidate(&packet, 4, 1, &table, &mut work).unwrap();
    match reconstruct_outward_at_times(
        &cand,
        &packet,
        OutwardPacketRequest {
            times: &[f64::from_bits(1)],
            initial_radii: &[0.0],
        },
        &mut work,
    ) {
        Ok(out) => assert!(
            out.modes
                .iter()
                .all(|&(a, b)| a.is_finite() && b.is_finite())
        ),
        Err(PacketError::OutwardRefusal(_)) => {}
        Err(e) => panic!("unexpected {e:?}"),
    }
}

#[test]
fn outward_uncertainty_radius_contains_perturbed_one_mode_solution() {
    let model = one_mode(0.0, 0.2, 0.0);
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&model, 0.01, 8, &mut work).unwrap();
    let table = CausalWeights::compile(8, 24);
    let cand = generate_candidate(&packet, 8, 5, &table, &mut work).unwrap();
    let out = reconstruct_outward_at_times(
        &cand,
        &packet,
        OutwardPacketRequest {
            times: &[0.01],
            initial_radii: &[0.1],
        },
        &mut work,
    )
    .unwrap();
    let amp: f64 = 0.6;
    let phase = 0.2 * amp * amp * 0.01;
    let d = (out.modes[0].0 - amp * phase.cos()).hypot(out.modes[0].1 - amp * phase.sin());
    assert!(d <= out.mode_radii[0], "d={d:e} r={:e}", out.mode_radii[0]);
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
fn compiled_descriptor_is_read_only_and_cross_packet_candidate_is_refused() {
    let mut work = PacketWorkCounters::new();
    let model = one_mode(0.2, 0.1, 0.0);
    let packet = compile_packet(&model, 0.1, 2, &mut work).unwrap();
    let other_packet = compile_packet(&model, 0.11, 2, &mut work).unwrap();
    let table = CausalWeights::compile(2, 6);
    let cand = generate_candidate(&packet, 2, 2, &table, &mut work).unwrap();

    // Packet descriptor fields are private: external callers have no mutable
    // path to model, moments, free coefficients, kernel, horizon, or order.
    assert_eq!(packet.order(), 2);
    assert!(compute_certificate(&cand, &other_packet, &table, &mut work).is_err());
    assert!(reconstruct_modes(&cand, &other_packet, &mut work).is_err());
    assert!(evaluate_mode_at(&cand, &other_packet, 0, 0.5, &mut work).is_err());
}

#[test]
fn distinct_signed_zero_model_cannot_reuse_a_candidate() {
    let mut work = PacketWorkCounters::new();
    let packet = compile_packet(&one_mode(0.2, 0.1, 0.0), 0.1, 2, &mut work).unwrap();
    let signed_zero_model =
        CollectiveModel::new(vec![1.0], vec![1.0], vec![(0.5, 0.0)], 0.2, 0.1, -0.0, 0.0).unwrap();
    let signed_zero_packet = compile_packet(&signed_zero_model, 0.1, 2, &mut work).unwrap();
    let table = CausalWeights::compile(2, 6);
    let cand = generate_candidate(&packet, 2, 2, &table, &mut work).unwrap();

    assert!(compute_certificate(&cand, &signed_zero_packet, &table, &mut work).is_err());
    assert!(evaluate_mode_at(&cand, &signed_zero_packet, 0, 0.0, &mut work).is_err());
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
    assert!(work.model_bounds_snapshot_items > 0);
    assert!(work.model_bounds_comparison_items > 0);
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
