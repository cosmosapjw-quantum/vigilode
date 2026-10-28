use rodas5p_integrators::CollectiveModel;
use rodas5p_integrators::damped_packet::{
    DampedPacketConfig, DampedPacketDescriptor, DampedPacketError, DampedPacketWork,
    integrate_verified_damped_packet, verify_damped_packet_window,
};
use rodas5p_integrators::packet_window::VerifiedPacketConfig;

fn config(tolerance: f64) -> DampedPacketConfig {
    DampedPacketConfig {
        packet: VerifiedPacketConfig {
            order: 8,
            degree: 12,
            sweeps: 6,
            mean_tolerance: tolerance,
            mode_tolerance: tolerance,
        },
        max_subdivision_levels: 8,
        moment_width_limit: 1.0e-12,
    }
}

fn one(kappa: f64, chi: f64, omega: f64) -> CollectiveModel {
    CollectiveModel::new(
        vec![1.0],
        vec![1.0],
        vec![(0.4, -0.2)],
        omega,
        chi,
        0.0,
        kappa,
    )
    .unwrap()
}

#[test]
fn one_mode_cubic_and_confluent_limit() {
    for kappa in [0.0, 1.0e-10, 0.7, 8.0] {
        let source = one(kappa, 0.8, 0.3);
        let h = 0.125;
        let mut work = DampedPacketWork::default();
        let point =
            verify_damped_packet_window(&source, h, 0.0, config(1.0e-10), &mut work).unwrap();
        let clock = if kappa == 0.0 {
            h
        } else {
            -(-2.0 * kappa * h).exp_m1() / (2.0 * kappa)
        };
        let phase = 0.3 * h + 0.8 * 0.2 * clock;
        let decay = (-kappa * h).exp();
        let expected = (
            decay * (0.4 * phase.cos() + 0.2 * phase.sin()),
            decay * (0.4 * phase.sin() - 0.2 * phase.cos()),
        );
        let z = point.modes()[0];
        let error = (z.0 - expected.0).hypot(z.1 - expected.1);
        assert!(
            error <= point.mode_radii()[0] + 2.0e-16,
            "kappa={kappa}, {error:e}"
        );
        assert!(point.weighted_radius() <= 1.0e-10);
        assert_eq!(point.subwindows(), 1);
        assert!(work.subwindow_attempts >= 1);
    }
}

#[test]
fn actual_nonunit_stored_weight_sum_is_used() {
    let s = 1.0_f64 + 5.0e-13;
    let source =
        CollectiveModel::new(vec![0.0], vec![s], vec![(0.4, -0.2)], 0.0, 0.8, 0.0, 0.7).unwrap();
    let h = 0.125;
    let point = verify_damped_packet_window(
        &source,
        h,
        0.0,
        config(1.0e-10),
        &mut DampedPacketWork::default(),
    )
    .unwrap();
    let clock = -(-1.4_f64 * h).exp_m1() / 1.4;
    let phase = 0.8 * s * s * s * 0.2 * clock;
    let decay = (-0.7_f64 * h).exp();
    let expected = (
        decay * (0.4 * phase.cos() + 0.2 * phase.sin()),
        decay * (0.4 * phase.sin() - 0.2 * phase.cos()),
    );
    let z = point.modes()[0];
    assert!((z.0 - expected.0).hypot(z.1 - expected.1) <= point.mode_radii()[0] + 2e-16);
}

#[test]
fn linear_large_damping_remains_finite() {
    let source = one(128.0, 0.0, 0.1);
    let point = verify_damped_packet_window(
        &source,
        0.125,
        0.0,
        config(1.0e-10),
        &mut DampedPacketWork::default(),
    )
    .unwrap();
    let phase = 0.1_f64 * 0.125;
    let decay = (-16.0_f64).exp();
    let expected = (
        decay * (0.4 * phase.cos() + 0.2 * phase.sin()),
        decay * (0.4 * phase.sin() - 0.2 * phase.cos()),
    );
    let z = point.modes()[0];
    assert!((z.0 - expected.0).hypot(z.1 - expected.1) <= point.mode_radii()[0] + 1.0e-22);
}

#[test]
fn large_moment_uses_stable_alternate_or_refuses_width() {
    let source = one(64.0, 0.4, 0.0);
    let mut work = DampedPacketWork::default();
    let point =
        verify_damped_packet_window(&source, 0.125, 0.0, config(1.0e-9), &mut work).unwrap();
    assert!(point.modes()[0].0.is_finite());
    assert!(point.mode_radii()[0] <= 1.0e-9);
    assert!(work.moments.kernel_attempts > 0);
    assert!(work.moment_fallback_attempts > 0);
    let mut strict = config(1.0e-9);
    strict.max_subdivision_levels = 0;
    strict.moment_width_limit = 0.0;
    assert!(
        verify_damped_packet_window(
            &source,
            0.125,
            0.0,
            strict,
            &mut DampedPacketWork::default()
        )
        .is_err()
    );
}

#[test]
fn unequal_frequencies_refine_without_relaxing_tolerance() {
    let source = CollectiveModel::new(
        vec![-1.0, 1.0],
        vec![0.25, 0.75],
        vec![(0.4, 0.1), (-0.2, 0.3)],
        0.4,
        0.3,
        0.0,
        0.2,
    )
    .unwrap();
    let mut work = DampedPacketWork::default();
    let point =
        verify_damped_packet_window(&source, 0.125, 0.0, config(2.0e-6), &mut work).unwrap();
    assert!(point.subwindows() > 1);
    assert!(work.rejected_windows > 0);
    assert!(point.mean_radius() <= 2.0e-6);
    assert!(point.mode_radii().iter().all(|&r| r <= 2.0e-6));
}

#[test]
fn immutable_multimesh_trajectory_carries_nonzero_ball() {
    let source = one(0.7, 0.8, 0.3);
    let snapshot = source.clone();
    let mesh = [2.0, 2.0625, 2.125, 2.25];
    let run = integrate_verified_damped_packet(
        &source,
        &[1.0e-9],
        &mesh,
        config(1.0e-7),
        &mut DampedPacketWork::default(),
    )
    .unwrap();
    assert_eq!(source, snapshot);
    assert_eq!(run.source_model(), &source);
    assert_eq!(run.initial_time(), 2.0);
    assert_eq!(run.initial_radii(), &[1.0e-9]);
    assert_eq!(run.points().len(), 3);
    for point in run.points() {
        let t = point.time() - 2.0;
        for perturbation in [-1.0e-9, 1.0e-9] {
            let re = 0.4 + perturbation;
            let energy = re * re + 0.04;
            let clock = -(-1.4 * t).exp_m1() / 1.4;
            let phase = 0.3 * t + 0.8 * energy * clock;
            let decay = (-0.7 * t).exp();
            let expected = (
                decay * (re * phase.cos() + 0.2 * phase.sin()),
                decay * (re * phase.sin() - 0.2 * phase.cos()),
            );
            let z = point.modes()[0];
            assert!((z.0 - expected.0).hypot(z.1 - expected.1) <= point.mode_radii()[0] + 2e-16);
        }
    }
}

#[test]
fn source_descriptor_is_owned_and_wrong_frame_refuses() {
    let source = one(0.3, 0.4, 0.2);
    let descriptor = DampedPacketDescriptor::new(
        &source,
        0.125,
        config(1.0e-9),
        &mut DampedPacketWork::default(),
    )
    .unwrap();
    assert_eq!(descriptor.source_model(), &source);
    descriptor
        .reconstruct(0.0, &mut DampedPacketWork::default())
        .unwrap();
    let wrong =
        CollectiveModel::new(vec![0.0], vec![1.0], vec![(0.4, 0.0)], 0.0, 0.2, 10.0, 0.3).unwrap();
    assert!(matches!(
        verify_damped_packet_window(
            &wrong,
            0.1,
            0.0,
            config(1e-9),
            &mut DampedPacketWork::default()
        ),
        Err(DampedPacketError::UnsupportedCarrier)
    ));
}

#[test]
fn failed_budgets_and_inexact_mesh_are_typed() {
    let source = one(0.7, 0.8, 0.3);
    let mut strict = config(1.0e-30);
    strict.max_subdivision_levels = 0;
    let mut work = DampedPacketWork::default();
    assert!(matches!(
        verify_damped_packet_window(&source, 0.125, 0.0, strict, &mut work),
        Err(DampedPacketError::BudgetExceeded { .. })
    ));
    assert_eq!(work.rejected_windows, 1);
    assert!(matches!(
        integrate_verified_damped_packet(
            &source,
            &[0.0],
            &[-1.0, 0.1],
            config(1.0e-8),
            &mut DampedPacketWork::default()
        ),
        Err(DampedPacketError::InexactDuration)
    ));
}
