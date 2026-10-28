//! Contracts of the lean RODAS5P driver (research node
//! `research/stiff_rodas5p_fast_20261002`): it is the sequential RODAS5P
//! step in transformed variables, with the sequential driver's controller.

use std::sync::Arc;

use rodas5p_core::{CoreError, DenseMatrix, LinearMethod, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, IntegrationMethod, OdeProblem, OutputSchedule, Rodas5pFastLu,
    constant_affine_mass_problem, integrate_adaptive_observed_with_config,
    integrate_rodas5p_fast_observed, manufactured_vector_problem, prothero_robinson_problem,
    robertson_problem, rodas5p_fast_step, semilinear_advection_diffusion_problem, sequential_step,
    stiff_van_der_pol_problem,
};

fn direct() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    }
}

/// y' = A y with a dense, diagonally dominant A of `n` rows.
fn dense_linear_problem(n: usize) -> (OdeProblem, Vec<f64>) {
    let a: Vec<f64> = (0..n * n)
        .map(|k| {
            let (i, j) = (k / n, k % n);
            if i == j {
                -10.0 - i as f64
            } else {
                0.05 * (((i * 7 + j * 3) % 11) as f64 - 5.0) / n as f64
            }
        })
        .collect();
    let matrix = Arc::new(DenseMatrix::new(n, n, a).unwrap());
    let rhs_matrix = matrix.clone();
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| rhs_matrix.matvec_into(y, out));
    let jac_matrix = matrix.clone();
    let jacobian = Arc::new(move |_t: f64, _y: &[f64]| Ok((*jac_matrix).clone()));
    let problem = OdeProblem::new(
        format!("dense-linear-{n}"),
        n,
        rhs,
        None,
        Some(jacobian),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    (problem, (0..n).map(|i| 1.0 + 0.01 * i as f64).collect())
}

/// Name, problem, y0, t0, h and the LU the driver must choose.
type Case = (&'static str, OdeProblem, Vec<f64>, f64, f64, Rodas5pFastLu);

fn cases() -> Vec<Case> {
    let (robertson, robertson_y0) = robertson_problem().unwrap();
    let (vdp, vdp_y0) = stiff_van_der_pol_problem(1.0e3).unwrap();
    let (pr, pr_y0) = prothero_robinson_problem(-50.0, 1.0, 0.3);
    let (vector, vector_y0) = manufactured_vector_problem(80, 50.0, 1.0, 0.2, 0.0).unwrap();
    let (semilinear, semilinear_y0) =
        semilinear_advection_diffusion_problem(8, 0.01, 5.0, -1.0, 10.0, 0.0).unwrap();
    let (dense, dense_y0) = dense_linear_problem(80);
    let small = Rodas5pFastLu::InPlaceZeroSkipping;
    vec![
        ("robertson", robertson, robertson_y0, 0.0, 1.0e-3, small),
        ("van-der-pol", vdp, vdp_y0, 0.0, 1.0e-2, small),
        (
            "prothero-robinson-nonautonomous",
            pr,
            pr_y0,
            0.3,
            0.05,
            small,
        ),
        ("tridiagonal-80", vector, vector_y0, 0.0, 0.01, small),
        ("semilinear-8", semilinear, semilinear_y0, 0.0, 0.01, small),
        ("dense-80", dense, dense_y0, 0.0, 0.01, Rodas5pFastLu::Faer),
    ]
}

#[test]
fn one_step_is_the_sequential_step_in_transformed_variables() {
    for (name, problem, y0, t0, h, lu) in cases() {
        let (atol, rtol) = (1.0e-8, 1.0e-6);
        let mut seq_counters = WorkCounters::default();
        let reference = sequential_step(
            &problem,
            t0,
            &y0,
            h,
            &direct(),
            None,
            atol,
            rtol,
            false,
            &mut seq_counters,
        )
        .unwrap();
        let mut counters = WorkCounters::default();
        let (y_new, error, used) =
            rodas5p_fast_step(&problem, t0, &y0, h, atol, rtol, &mut counters).unwrap();
        assert_eq!(used, lu, "{name}");
        let scale = y0
            .iter()
            .chain(&reference.y_new)
            .fold(0.0_f64, |m, v| m.max(v.abs()))
            .max(1.0e-300);
        let diff = y_new
            .iter()
            .zip(&reference.y_new)
            .fold(0.0_f64, |m, (a, b)| m.max((a - b).abs()));
        assert!(
            diff <= 1.0e-12 * scale,
            "{name}: state differs by {diff:e} (scale {scale:e})"
        );
        // The k-form recovers the embedded error u_s = sum_j Gamma_sj k_j by
        // cancellation; the transformed form solves for it. They agree to
        // roundoff of the stages, far below the tolerance unit that decides
        // acceptance.
        let gap = (error - reference.error_norm).abs();
        assert!(
            gap <= 1.0e-7 * reference.error_norm + 1.0e-8,
            "{name}: error norm {error:e} vs {:e}",
            reference.error_norm
        );
        // The same right-hand sides and one factorization; no residual products.
        assert_eq!(counters.direct_factorizations, 1, "{name}");
        assert_eq!(counters.linear_solves, 8, "{name}");
        assert_eq!(counters.jacobian_matvecs, 0, "{name}");
        assert_eq!(counters.diagnostic_matvecs, 0, "{name}");
    }
}

#[test]
fn a_jvp_only_problem_assembles_the_same_jacobian() {
    // The sequential direct path needs an explicit matrix; the fast driver
    // assembles one from JVPs, which for Robertson is exact.
    let (robertson, y0) = robertson_problem().unwrap();
    let jvp_only = robertson.jvp_only_clone().unwrap();
    let mut counters = WorkCounters::default();
    let explicit =
        rodas5p_fast_step(&robertson, 0.0, &y0, 1.0e-3, 1.0e-8, 1.0e-6, &mut counters).unwrap();
    let mut jvp_counters = WorkCounters::default();
    let assembled = rodas5p_fast_step(
        &jvp_only,
        0.0,
        &y0,
        1.0e-3,
        1.0e-8,
        1.0e-6,
        &mut jvp_counters,
    )
    .unwrap();
    assert_eq!(explicit.0, assembled.0);
    assert_eq!(explicit.1.to_bits(), assembled.1.to_bits());
    assert_eq!(jvp_counters.jvp_vectors, 3);
}

#[test]
fn adaptive_runs_follow_the_sequential_driver() {
    let (robertson, robertson_y0) = robertson_problem().unwrap();
    let (vdp, vdp_y0) = stiff_van_der_pol_problem(1.0e3).unwrap();
    let (pr, pr_y0) = prothero_robinson_problem(-50.0, 1.0, 0.0);
    for (name, problem, y0, tf, atol_scale) in [
        ("robertson", robertson, robertson_y0, 40.0, 1.0e-4),
        ("van-der-pol", vdp, vdp_y0, 2000.0, 1.0),
        ("prothero-robinson", pr, pr_y0, 2.0, 1.0),
    ] {
        for rtol in [1.0e-4, 1.0e-7] {
            let adaptive = AdaptiveStepConfig {
                atol: rtol * atol_scale,
                rtol,
                initial_step: 1.0e-6,
                min_step: 1.0e-14,
                max_step: tf,
                max_attempts: 1_000_000,
                ..AdaptiveStepConfig::default()
            };
            let output = OutputSchedule::new(vec![0.0, tf]).unwrap();
            let sequential = integrate_adaptive_observed_with_config(
                &problem,
                (0.0, tf),
                &y0,
                IntegrationMethod::Sequential,
                Some(&direct()),
                None,
                &adaptive,
                &output,
            )
            .unwrap();
            let fast =
                integrate_rodas5p_fast_observed(&problem, (0.0, tf), &y0, &adaptive, &output)
                    .unwrap();
            assert!(fast.observed.success, "{name} {rtol:e}");
            assert_eq!(fast.attempts, fast.accepted_steps + fast.rejected_steps);
            assert_eq!(
                fast.observed.counters.accepted_steps as usize,
                fast.accepted_steps
            );
            // Roundoff alone separates the two step sequences.
            let seq_steps = sequential.diagnostics.accepted_macro_steps as f64;
            let steps_gap = (fast.accepted_steps as f64 - seq_steps).abs() / seq_steps;
            assert!(
                steps_gap <= 0.02,
                "{name} {rtol:e}: {} vs {seq_steps}",
                fast.accepted_steps
            );
            let (a, b) = (
                fast.observed.y.last().unwrap(),
                sequential.observed.y.last().unwrap(),
            );
            for (i, (x, z)) in a.iter().zip(b).enumerate() {
                let tol = 10.0 * (rtol * atol_scale + rtol * z.abs());
                assert!(
                    (x - z).abs() <= tol,
                    "{name} {rtol:e} component {i}: {x:e} vs {z:e}"
                );
            }
            // A rejection from the same state reuses its Jacobian.
            assert_eq!(
                fast.observed.counters.jacobian_builds as usize,
                fast.attempts - fast.jacobian_reuses,
                "{name}"
            );
        }
    }
}

#[test]
fn a_mass_matrix_is_refused() {
    let (problem, y0, _, _) = constant_affine_mass_problem();
    let adaptive = AdaptiveStepConfig::default();
    let output = OutputSchedule::new(vec![0.0, 0.1]).unwrap();
    let error =
        integrate_rodas5p_fast_observed(&problem, (0.0, 0.1), &y0, &adaptive, &output).unwrap_err();
    assert!(matches!(error, CoreError::InvalidInput(_)), "{error}");
}
