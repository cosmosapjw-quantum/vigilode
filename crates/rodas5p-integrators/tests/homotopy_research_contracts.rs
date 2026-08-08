use rodas5p_core::{DenseMatrix, load_rodas5p_coefficients, safe_l2, wrms};
use rodas5p_integrators::{AffinePartialCouplingOracle, PartialCouplingParameters};

fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

fn noncommuting_oracle() -> AffinePartialCouplingOracle {
    let coeffs = load_rodas5p_coefficients().unwrap();
    let mass = DenseMatrix::from_rows(&[&[2.0, 0.3], &[-0.1, 1.5]]).unwrap();
    let jacobian = DenseMatrix::from_rows(&[&[-3.0, 2.0], &[0.5, -1.0]]).unwrap();
    let rhs_rows: Vec<Vec<f64>> = (0..coeffs.stages())
        .map(|i| {
            let x = i as f64 + 1.0;
            vec![0.1 * x.sin(), 0.1 * x.cos()]
        })
        .collect();
    AffinePartialCouplingOracle::new(
        mass,
        jacobian,
        coeffs.beta,
        coeffs.gamma,
        0.05,
        rhs_rows,
        coeffs.b,
    )
    .unwrap()
}

#[test]
fn partial_coupling_parameters_and_oracle_shapes_are_validated() {
    assert!(PartialCouplingParameters::new(0.0, 0.0).is_ok());
    assert!(PartialCouplingParameters::new(1.0, 1.0).is_ok());
    assert!(PartialCouplingParameters::new(-1e-12, 0.5).is_err());
    assert!(PartialCouplingParameters::new(0.5, 1.0 + 1e-12).is_err());
    assert!(PartialCouplingParameters::new(f64::NAN, 0.5).is_err());

    let coeffs = load_rodas5p_coefficients().unwrap();
    let bad_mass = DenseMatrix::identity(3);
    let jacobian = DenseMatrix::identity(2);
    let rhs_rows = vec![vec![0.0; 2]; coeffs.stages()];
    assert!(
        AffinePartialCouplingOracle::new(
            bad_mass,
            jacobian,
            coeffs.beta,
            coeffs.gamma,
            0.1,
            rhs_rows,
            coeffs.b,
        )
        .is_err()
    );
}

#[test]
fn affine_partial_coupling_endpoints_and_flrh_limit_hold() {
    let oracle = noncommuting_oracle();
    let target = oracle
        .solve_path(PartialCouplingParameters::new(0.0, 1.0).unwrap())
        .unwrap();

    for theta in [0.0, 0.25, 0.5, 1.0] {
        let endpoint = oracle
            .solve_path(PartialCouplingParameters::new(theta, 1.0).unwrap())
            .unwrap();
        assert!(max_abs_diff(&endpoint, &target) < 2e-12);
        assert!(safe_l2(&oracle.target_residual(&endpoint).unwrap()) < 2e-12);
        assert!(
            safe_l2(
                &oracle
                    .homotopy_residual(
                        PartialCouplingParameters::new(theta, 1.0).unwrap(),
                        &endpoint,
                    )
                    .unwrap()
            ) < 2e-12
        );
    }

    let flrh0 = oracle
        .solve_path(PartialCouplingParameters::new(1.0, 0.0).unwrap())
        .unwrap();
    for lambda in [0.25, 0.5, 0.75, 1.0] {
        let got = oracle
            .solve_path(PartialCouplingParameters::new(1.0, lambda).unwrap())
            .unwrap();
        assert!(max_abs_diff(&got, &flrh0) < 2e-12);
    }
}

#[test]
fn full_nilpotent_inverse_matches_direct_path_solve_for_noncommuting_mass_and_jacobian() {
    use rodas5p_core::LuFactorization;

    let oracle = noncommuting_oracle();
    let parameters = PartialCouplingParameters::new(0.4, 0.7).unwrap();
    let rhs: Vec<f64> = (0..oracle.stages() * oracle.dimension())
        .map(|i| ((i + 3) as f64).sin() / 7.0)
        .collect();
    let direct = LuFactorization::new(&oracle.path_operator(parameters).unwrap())
        .unwrap()
        .solve(&rhs)
        .unwrap();
    let nilpotent = oracle
        .truncated_inverse_apply(parameters, oracle.stages() - 1, &rhs)
        .unwrap();
    assert!(max_abs_diff(&nilpotent, &direct) < 3e-12);
    assert!(
        oracle
            .truncated_inverse_apply(parameters, oracle.stages(), &rhs)
            .is_err()
    );
}

#[test]
fn low_depth_truncation_exposes_unpropagated_stage_chain() {
    let stages = 8;
    let gamma = 0.25;
    let mut beta = DenseMatrix::identity(stages).scale(gamma);
    for i in 1..stages {
        beta[(i, i - 1)] = 2.0;
    }
    let oracle = AffinePartialCouplingOracle::new(
        DenseMatrix::identity(1),
        DenseMatrix::identity(1),
        beta,
        gamma,
        0.1,
        vec![vec![0.0]; stages],
        vec![1.0; stages],
    )
    .unwrap();
    let parameters = PartialCouplingParameters::new(0.0, 1.0).unwrap();
    let mut rhs = vec![0.0; stages];
    rhs[0] = 1.0;

    let q0 = oracle.truncated_inverse_apply(parameters, 0, &rhs).unwrap();
    let q1 = oracle.truncated_inverse_apply(parameters, 1, &rhs).unwrap();
    let exact = oracle
        .truncated_inverse_apply(parameters, stages - 1, &rhs)
        .unwrap();

    assert_eq!(q0[1], 0.0);
    assert!(q1[1].abs() > 0.0);
    assert_eq!(q1[2], 0.0);
    assert!(exact[7].abs() > 0.0);
    assert!(
        safe_l2(
            &exact
                .iter()
                .zip(&q1)
                .map(|(a, b)| a - b)
                .collect::<Vec<_>>()
        ) > 0.0
    );
}

#[test]
fn original_output_certificate_matches_an_exact_affine_target_correction() {
    let oracle = noncommuting_oracle();
    let endpoint = oracle
        .solve_path(PartialCouplingParameters::new(0.3, 1.0).unwrap())
        .unwrap();
    let mut perturbation = vec![0.0; endpoint.len()];
    for stage in 0..oracle.stages() {
        let start = stage * oracle.dimension();
        perturbation[start] = 1e-5 * (stage as f64 + 1.0);
        perturbation[start + 1] = -5e-6 * (stage as f64 + 1.0).powi(2);
    }
    let perturbed: Vec<f64> = endpoint
        .iter()
        .zip(&perturbation)
        .map(|(value, delta)| value + delta)
        .collect();
    let scale = vec![2e-4, 3e-4];
    let certificate = oracle.certify_target(&perturbed, &scale).unwrap();

    let mut expected_output = vec![0.0; oracle.dimension()];
    for stage in 0..oracle.stages() {
        for component in 0..oracle.dimension() {
            expected_output[component] += oracle.output_weights()[stage]
                * perturbation[stage * oracle.dimension() + component];
        }
    }
    let expected_wrms = wrms(&expected_output, &scale).unwrap();
    assert!((certificate.output_wrms - expected_wrms).abs() < 2e-11);
    assert!((certificate.correction_norm - safe_l2(&perturbation)).abs() < 2e-12);
    assert!(certificate.residual_norm > 0.0);
    assert!(certificate.relative_residual > 0.0);

    let exact_certificate = oracle.certify_target(&endpoint, &scale).unwrap();
    assert!(exact_certificate.output_wrms < 2e-11);
    assert!(exact_certificate.correction_norm < 2e-12);
}

#[test]
fn original_output_certificate_rejects_invalid_vectors_and_scales() {
    let oracle = noncommuting_oracle();
    let endpoint = oracle
        .solve_path(PartialCouplingParameters::new(0.0, 1.0).unwrap())
        .unwrap();
    assert!(
        oracle
            .certify_target(&endpoint[..endpoint.len() - 1], &[1.0, 1.0])
            .is_err()
    );
    assert!(oracle.certify_target(&endpoint, &[1.0]).is_err());
    assert!(oracle.certify_target(&endpoint, &[1.0, 0.0]).is_err());
    let mut nonfinite = endpoint;
    nonfinite[0] = f64::NAN;
    assert!(oracle.certify_target(&nonfinite, &[1.0, 1.0]).is_err());

    let mut finite_perturbation = oracle
        .solve_path(PartialCouplingParameters::new(0.0, 1.0).unwrap())
        .unwrap();
    finite_perturbation[0] += 1e-6;
    assert!(
        oracle
            .certify_target(
                &finite_perturbation,
                &[f64::from_bits(1), f64::from_bits(1)],
            )
            .is_err()
    );
}
