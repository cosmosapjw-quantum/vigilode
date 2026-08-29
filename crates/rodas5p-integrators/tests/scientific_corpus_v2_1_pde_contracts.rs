use rodas5p_core::WorkCounters;
use rodas5p_integrators::{ScientificCorpusV2, ScientificFamily};
use serde::Deserialize;

#[derive(Deserialize)]
struct Oracle {
    schema_version: String,
    origin: OracleOrigin,
    cases: Vec<OracleCase>,
}

#[derive(Deserialize)]
struct OracleOrigin {
    engine: String,
    version: String,
    decimal_digits: usize,
}

#[derive(Deserialize)]
struct OracleCase {
    dimension: usize,
    nx: usize,
    ny: usize,
    time: String,
    expected_half_bandwidth: usize,
    seam_cross_jacobian: String,
    samples: Vec<OracleSample>,
}

#[derive(Deserialize)]
struct OracleSample {
    index: usize,
    phi: String,
    phi_f64_bits: u64,
    rhs_at_phi: String,
    rhs_perturbed: String,
    jvp: String,
    partial_t: String,
}

fn oracle_value(value: &str) -> f64 {
    value.parse().unwrap()
}

fn assert_oracle_close(actual: f64, expected: &str, context: &str) -> f64 {
    let expected = oracle_value(expected);
    let error = (actual - expected).abs();
    let scaled_error = error / expected.abs().max(1.0);
    assert!(
        scaled_error <= 5.0e-14,
        "{context}: actual={actual:.17e}, expected={expected:.17e}, error={error:.3e}"
    );
    scaled_error
}

fn semilinear_case(dimension: usize) -> rodas5p_integrators::ScientificProblemCase {
    ScientificCorpusV2::calibration_specs()
        .into_iter()
        .find(|spec| {
            spec.family == ScientificFamily::SemilinearAdvectionDiffusionRamped
                && spec.dimension == dimension
                && spec.rtol == 1.0e-6
        })
        .unwrap()
        .build()
        .unwrap()
}

#[test]
fn corpus_v2_1_identity_distinguishes_the_new_grid_topology() {
    // Defect caught: a v2 case ID can silently retain the old 1-D operator while
    // claiming the new dimension-growing 2-D corpus.
    assert_eq!(ScientificCorpusV2::VERSION, "scientific-corpus-v2.1");
    let cases = [
        (96, "grid8x12", [8, 12]),
        (384, "grid16x24", [16, 24]),
        (1536, "grid32x48", [32, 48]),
    ];
    for (dimension, grid_id, grid_shape) in cases {
        let case = semilinear_case(dimension);
        assert_eq!(case.spec.grid_shape, Some(grid_shape));
        assert!(case.spec.id.contains(grid_id), "{}", case.spec.id);
        assert!(case.spec.id.ends_with("-v2.1"), "{}", case.spec.id);
        assert!(case.problem.name.contains(grid_id), "{}", case.problem.name);
    }
    assert!(
        ScientificCorpusV2::calibration_specs()
            .iter()
            .filter(|spec| spec.family != ScientificFamily::SemilinearAdvectionDiffusionRamped)
            .all(|spec| spec.grid_shape.is_none())
    );
    assert!(
        ScientificCorpusV2::holdout_specs()
            .iter()
            .all(|spec| spec.grid_shape.is_none())
    );
}

#[test]
fn semilinear_v2_1_is_a_zero_dirichlet_five_point_manufactured_pde() {
    // Defects caught: row-wrap coupling at an x seam, retention of the old
    // tridiagonal operator, or a forcing term inconsistent with phi_t=-phi.
    let case = semilinear_case(96);
    let (nx, ny) = (8, 12);
    let hx = 1.0 / (nx + 1) as f64;
    let hy = 1.0 / (ny + 1) as f64;
    let t = 0.37_f64;
    let phi = (0..ny)
        .flat_map(|j| {
            (0..nx).map(move |i| {
                (-t).exp()
                    * (std::f64::consts::PI * (i + 1) as f64 * hx).sin()
                    * (std::f64::consts::PI * (j + 1) as f64 * hy).sin()
            })
        })
        .collect::<Vec<_>>();
    let rhs = case
        .problem
        .eval_rhs(t, &phi, &mut WorkCounters::default())
        .unwrap();
    for (actual, exact) in rhs.iter().zip(&phi) {
        assert!((actual + exact).abs() <= 4.0e-15 * exact.abs().max(1.0));
    }

    let operator = case.problem.linearize_matrix_free(t, &phi).unwrap();
    let mut seam_direction = vec![0.0; 96];
    seam_direction[nx] = 1.0; // (i=0,j=1), not a neighbor of (i=nx-1,j=0)
    let mut seam_image = vec![0.0; 96];
    operator.apply(&seam_direction, &mut seam_image).unwrap();
    assert_eq!(seam_image[nx - 1].to_bits(), 0.0_f64.to_bits());

    let mut bandwidth = 0usize;
    for column in 0..96 {
        let mut basis = vec![0.0; 96];
        basis[column] = 1.0;
        let mut image = vec![0.0; 96];
        operator.apply(&basis, &mut image).unwrap();
        for (row, value) in image.into_iter().enumerate() {
            if value != 0.0 {
                bandwidth = bandwidth.max(row.abs_diff(column));
            }
        }
    }
    assert_eq!(bandwidth, nx);
}

#[test]
fn rust_callbacks_match_the_shared_mpmath_oracle_for_every_grid() {
    // The JSON is the cross-language oracle: Rust and the independent Python
    // generator both consume these same high-precision decimal observations.
    let oracle: Oracle = serde_json::from_str(include_str!(
        "../../../fixtures/scientific_corpus_v2_1_semilinear_oracle.json"
    ))
    .unwrap();
    assert_eq!(
        oracle.schema_version,
        "scientific-corpus-v2.1-semilinear-mpmath-oracle-v1"
    );
    assert_eq!(oracle.origin.engine, "mpmath");
    assert_eq!(oracle.origin.version, "1.3.0");
    assert_eq!(oracle.origin.decimal_digits, 80);

    for expected in oracle.cases {
        assert_eq!(expected.dimension, expected.nx * expected.ny);
        let case = semilinear_case(expected.dimension);
        let time = oracle_value(&expected.time);
        let phi = case.problem.exact(time).unwrap();
        let state = phi
            .iter()
            .enumerate()
            .map(|(index, value)| value + 0.01 * (0.17 * (index + 1) as f64).cos())
            .collect::<Vec<_>>();
        let direction = (0..expected.dimension)
            .map(|index| (0.23 * (index + 1) as f64).sin())
            .collect::<Vec<_>>();
        let rhs_at_phi = case
            .problem
            .eval_rhs(time, &phi, &mut WorkCounters::default())
            .unwrap();
        let rhs = case
            .problem
            .eval_rhs(time, &state, &mut WorkCounters::default())
            .unwrap();
        let partial_t = case
            .problem
            .eval_partial_t(time, &state, &mut WorkCounters::default())
            .unwrap();
        let operator = case.problem.linearize_matrix_free(time, &state).unwrap();
        let mut jvp = vec![0.0; expected.dimension];
        operator.apply(&direction, &mut jvp).unwrap();

        let mut max_scaled_error = 0.0_f64;
        for sample in &expected.samples {
            let index = sample.index;
            assert_eq!(
                phi[index].to_bits(),
                sample.phi_f64_bits,
                "Rust/Python exact-state evaluation lost bit parity at n={} index={index}",
                expected.dimension
            );
            for scaled_error in [
                assert_oracle_close(phi[index], &sample.phi, "phi"),
                assert_oracle_close(rhs_at_phi[index], &sample.rhs_at_phi, "rhs(phi)"),
                assert_oracle_close(rhs[index], &sample.rhs_perturbed, "rhs(perturbed)"),
                assert_oracle_close(jvp[index], &sample.jvp, "Jv"),
                assert_oracle_close(partial_t[index], &sample.partial_t, "partial_t"),
            ] {
                max_scaled_error = max_scaled_error.max(scaled_error);
            }
        }
        println!(
            "grid={}x{} max scaled mpmath-oracle error={max_scaled_error:.3e}",
            expected.nx, expected.ny
        );

        let seam_left = expected.nx - 1;
        let seam_right = expected.nx;
        let mut seam_direction = vec![0.0; expected.dimension];
        seam_direction[seam_right] = 1.0;
        let mut seam_image = vec![0.0; expected.dimension];
        operator.apply(&seam_direction, &mut seam_image).unwrap();
        assert_eq!(
            seam_image[seam_left].to_bits(),
            oracle_value(&expected.seam_cross_jacobian).to_bits()
        );

        let mut bandwidth = 0usize;
        let mut basis = vec![0.0; expected.dimension];
        let mut image = vec![0.0; expected.dimension];
        for column in 0..expected.dimension {
            basis.fill(0.0);
            basis[column] = 1.0;
            image.fill(0.0);
            operator.apply(&basis, &mut image).unwrap();
            for (row, value) in image.iter().copied().enumerate() {
                if value != 0.0 {
                    bandwidth = bandwidth.max(row.abs_diff(column));
                }
            }
        }
        assert_eq!(bandwidth, expected.expected_half_bandwidth);
    }
}
