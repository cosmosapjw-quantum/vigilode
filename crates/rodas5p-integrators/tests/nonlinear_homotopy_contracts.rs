use rodas5p_integrators::{
    CollectiveModel, NonlinearHomotopyConfig, NonlinearHomotopyError, NonlinearHomotopyWork,
    integrate_nonlinear_homotopy,
};

fn config() -> NonlinearHomotopyConfig {
    NonlinearHomotopyConfig {
        split_substeps: 4,
        correction_rounds: 4,
        max_refinements: 1,
        lambdas: vec![0.5, 1.0],
        weighted_tolerance: 2e-5,
        packet_order: 5,
        packet_degree: 10,
        packet_sweeps: 8,
        damped_subdivision_levels: 6,
    }
}
fn model(n: usize, width: f64, chi: f64) -> CollectiveModel {
    let xi = (0..n)
        .map(|j| {
            if n == 1 {
                0.0
            } else {
                -1.0 + 2.0 * j as f64 / (n - 1) as f64
            }
        })
        .collect();
    let weights = vec![1.0 / n as f64; n];
    let initial = (0..n)
        .map(|j| (0.3 + 0.02 * j as f64, -0.1 + 0.03 * j as f64))
        .collect();
    CollectiveModel::new(xi, weights, initial, width, chi, 0.0, 0.0).unwrap()
}
#[test]
fn nonlinear_equal_frequency_orbit_is_inside_verified_endpoints() {
    // For zero width all modes receive the same exact collective nonlinear
    // displacement. This independent closed orbit is not passed to the solver.
    let model = model(3, 0.0, 0.7);
    let mesh = [0.0, 0.0625, 0.125, 0.1875, 0.25];
    let mut work = NonlinearHomotopyWork::default();
    let result = integrate_nonlinear_homotopy(&model, &mesh, &config(), &mut work).unwrap();
    assert_eq!(result.source_model(), &model);
    assert_eq!(result.mesh(), &mesh);
    let s: f64 = model.weights().iter().sum();
    let mean = model
        .initial_amplitudes()
        .iter()
        .zip(model.weights())
        .fold((0.0, 0.0), |(re, im), (z, w)| (re + w * z.0, im + w * z.1));
    for point in result.points() {
        let angle = model.chi() * s * (mean.0 * mean.0 + mean.1 * mean.1) * point.time();
        let delta = (
            (mean.0 * (angle.cos() - 1.0) - mean.1 * angle.sin()) / s,
            (mean.0 * angle.sin() + mean.1 * (angle.cos() - 1.0)) / s,
        );
        for ((&got, &initial), &radius) in point
            .modes()
            .iter()
            .zip(model.initial_amplitudes())
            .zip(point.mode_radii())
        {
            let error = (got.0 - initial.0 - delta.0).hypot(got.1 - initial.1 - delta.1);
            // Closed orbit evaluation is binary64, so only this independent
            // numerical comparison gets a tiny evaluation guard.
            assert!(error <= radius + 2e-15, "error {error} radius {radius}");
        }
        assert!(point.weighted_radius() <= config().weighted_tolerance);
    }
    assert!(work.fine_window_calls > 0);
    assert_eq!(work.packet_verification_work.len(), mesh.len() - 1);
}
#[test]
fn unequal_frequency_nonzero_coupling_is_thread_reproducible() {
    let model = model(4, 1.4, -0.8);
    let mesh = [0.0, 0.03125, 0.0625, 0.09375, 0.125];
    let run = |threads| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let mut work = NonlinearHomotopyWork::default();
                let result =
                    integrate_nonlinear_homotopy(&model, &mesh, &config(), &mut work).unwrap();
                (result, work)
            })
    };
    let (one, ow) = run(1);
    let (four, fw) = run(4);
    for (a, b) in one.points().iter().zip(four.points()) {
        assert_eq!(a.modes(), b.modes());
        assert_eq!(a.weighted_radius(), b.weighted_radius());
    }
    assert_eq!(ow.vector_affine_compositions, fw.vector_affine_compositions);
}
#[test]
fn target_and_mesh_refusals_never_return_partial_certificate() {
    let model = model(2, 1.0, 0.2);
    let mut cfg = config();
    let mut work = NonlinearHomotopyWork::default();
    cfg.lambdas = vec![0.0, 0.9];
    assert!(matches!(
        integrate_nonlinear_homotopy(&model, &[0.0, 0.1], &cfg, &mut work),
        Err(NonlinearHomotopyError::InvalidInput(_))
    ));
    assert_eq!(work.attempts, 0);
    cfg = config();
    assert!(matches!(
        integrate_nonlinear_homotopy(&model, &[0.1, 1.0], &cfg, &mut work),
        Err(NonlinearHomotopyError::InexactDuration { .. })
    ));
}
#[test]
fn unachievable_tolerance_preserves_attempted_work() {
    let model = model(3, 2.0, 0.8);
    let mut cfg = config();
    cfg.weighted_tolerance = 1e-30;
    cfg.max_refinements = 0;
    let mut work = NonlinearHomotopyWork::default();
    let answer = integrate_nonlinear_homotopy(&model, &[0.0, 0.125, 0.25], &cfg, &mut work);
    assert!(
        matches!(answer, Err(NonlinearHomotopyError::BudgetExceeded { .. })),
        "{answer:?}"
    );
    assert_eq!(work.attempts, 1);
    assert!(work.fine_window_calls > 0);
    assert_eq!(work.packet_verification_work.len(), 2);
}
#[test]
fn direct_target_and_explicit_homotopy_both_certify_original_source() {
    let model = model(2, 0.7, 0.5);
    let mesh = [0.0, 0.0625, 0.125];
    let mut cfg = config();
    cfg.lambdas = vec![1.0];
    let direct =
        integrate_nonlinear_homotopy(&model, &mesh, &cfg, &mut NonlinearHomotopyWork::default())
            .unwrap();
    cfg.lambdas = vec![0.0, 0.25, 1.0];
    let continued =
        integrate_nonlinear_homotopy(&model, &mesh, &cfg, &mut NonlinearHomotopyWork::default())
            .unwrap();
    for (a, b) in direct.points().iter().zip(continued.points()) {
        let d: f64 = a
            .modes()
            .iter()
            .zip(b.modes())
            .zip(model.weights())
            .map(|((a, b), w)| w * ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)))
            .sum::<f64>()
            .sqrt();
        assert!(d <= a.weighted_radius() + b.weighted_radius());
    }
    assert_eq!(continued.source_model().chi(), model.chi());
}

#[test]
fn common_damping_uses_original_ode_verifier() {
    let original = model(2, 0.0, 0.6);
    let damped = CollectiveModel::new(
        original.xi().to_vec(),
        original.weights().to_vec(),
        original.initial_amplitudes().to_vec(),
        0.0,
        0.6,
        0.0,
        0.4,
    )
    .unwrap();
    let mesh = [0.0, 0.03125, 0.0625];
    let mut cfg = config();
    cfg.weighted_tolerance = 5e-5;
    let mut work = NonlinearHomotopyWork::default();
    let result = integrate_nonlinear_homotopy(&damped, &mesh, &cfg, &mut work).unwrap();
    let s: f64 = damped.weights().iter().sum();
    let mean = damped
        .initial_amplitudes()
        .iter()
        .zip(damped.weights())
        .fold((0.0, 0.0), |(re, im), (z, w)| (re + w * z.0, im + w * z.1));
    for point in result.points() {
        let t = point.time();
        let attenuation = (-damped.kappa() * t).exp();
        let theta = -(-2.0 * damped.kappa() * t).exp_m1() / (2.0 * damped.kappa());
        let angle = damped.chi() * s * (mean.0 * mean.0 + mean.1 * mean.1) * theta;
        let shift = (
            (mean.0 * (angle.cos() - 1.0) - mean.1 * angle.sin()) / s,
            (mean.0 * angle.sin() + mean.1 * (angle.cos() - 1.0)) / s,
        );
        for ((&got, &initial), &radius) in point
            .modes()
            .iter()
            .zip(damped.initial_amplitudes())
            .zip(point.mode_radii())
        {
            let exact = (
                attenuation * (initial.0 + shift.0),
                attenuation * (initial.1 + shift.1),
            );
            assert!((got.0 - exact.0).hypot(got.1 - exact.1) <= radius + 2e-15);
        }
    }
    assert!(
        work.damped_verification_work
            .iter()
            .all(|w| w.window_attempts > 0)
    );
    assert_eq!(result.source_model().kappa(), 0.4);
}

#[test]
fn budget_driven_refinement_reduces_split_error_without_changing_tolerance() {
    let source = model(4, 1.4, -0.8);
    let mut cfg = config();
    cfg.split_substeps = 1;
    cfg.correction_rounds = 1;
    cfg.max_refinements = 5;
    cfg.lambdas = vec![1.0];
    cfg.weighted_tolerance = 1e-8;
    let mut work = NonlinearHomotopyWork::default();
    let result = integrate_nonlinear_homotopy(
        &source,
        &[0.0, 0.03125, 0.0625, 0.09375, 0.125],
        &cfg,
        &mut work,
    )
    .unwrap();
    assert!(work.attempts > 1);
    assert!(result.final_substeps() > 1);
    assert!(result.points().iter().all(|p| p.weighted_radius() <= 1e-8));
    assert_eq!(result.config().weighted_tolerance, 1e-8);
    assert_eq!(work.verification_window_calls, 4 * work.attempts);
}
