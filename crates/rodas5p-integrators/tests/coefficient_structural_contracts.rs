//! Stage target coefficient semantics (re-audit R3 of 2026-10-01, HOM-01).

use rodas5p_core::{DenseMatrix, RODAS5P_COEFFICIENT_SNAPSHOT_SHA256, rodas5p_coefficients};
use rodas5p_integrators::{
    STAGE_TARGET_SEQUENTIAL, STAGE_TARGET_STRICT_LOWER_PROJECTION, StageTarget, StageTargetBits,
    block_sequential_allowance, native_coefficient_leakage,
};

const SEMANTICS: &str = include_str!("../../../fixtures/stage_target_semantics.json");

fn hex(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

#[test]
fn native_coefficient_bits_and_target_ids_are_pinned() {
    let coeffs = rodas5p_coefficients().unwrap();
    assert_eq!(hex(coeffs.gamma), "3fcb20c5235b5100");
    assert_eq!(hex(coeffs.alpha[(1, 0)]), "3fe45893da847cbf");
    assert_eq!(hex(coeffs.l[(1, 0)]), "bca0000000000000");
    // The rounded inverse makes Gamma_77 = btilde_7 differ from gamma.
    assert_eq!(hex(coeffs.gamma_matrix[(7, 7)]), "3fcb20c5235b5104");
    let sequential = StageTarget::sequential(coeffs).unwrap();
    let projection = StageTarget::strict_lower_projection(coeffs).unwrap();
    assert_eq!(sequential.id, STAGE_TARGET_SEQUENTIAL);
    assert_eq!(projection.id, STAGE_TARGET_STRICT_LOWER_PROJECTION);
    assert!(STAGE_TARGET_SEQUENTIAL.contains(&RODAS5P_COEFFICIENT_SNAPSHOT_SHA256[..8]));
    let pinned: Vec<StageTargetBits> = serde_json::from_str(SEMANTICS).unwrap();
    assert_eq!(
        pinned,
        vec![sequential.coefficient_bits(), projection.coefficient_bits()],
        "fixtures/stage_target_semantics.json"
    );
}

#[test]
fn the_leakage_census_is_pinned() {
    // Any change to the derivation must fail here and be re-reviewed.
    let leakage = native_coefficient_leakage(rodas5p_coefficients().unwrap());
    assert_eq!(leakage.alpha_nonzero, 28);
    assert_eq!(leakage.alpha_max, 5.577737968635803e-16);
    assert_eq!(leakage.l_nonzero, 34);
    assert_eq!(leakage.l_max, 3.7683229960916457e-16);
}

#[test]
fn nilpotency_is_a_property_of_the_target_not_of_the_native_matrices() {
    let coeffs = rodas5p_coefficients().unwrap();
    for target in [
        StageTarget::sequential(coeffs).unwrap(),
        StageTarget::strict_lower_projection(coeffs).unwrap(),
    ] {
        assert!(target.strictly_lower_nilpotent(), "{}", target.id);
        for (i, row) in target.alpha_rows.iter().enumerate() {
            assert_eq!(row.len(), i);
            assert_eq!(target.coupling_rows[i].len(), i);
        }
    }
    // Documentation: the native full alpha is not nilpotent as reals in
    // binary64 evaluation (its eighth power has nonzero entries).
    let mut power = coeffs.alpha.clone();
    for _ in 1..8 {
        power = power.matmul(&coeffs.alpha).unwrap();
    }
    let s = coeffs.stages();
    assert!((0..s).any(|i| (0..s).any(|j| power[(i, j)] != 0.0)));
    // A target whose rows are too long is not accepted as nilpotent.
    let mut broken = StageTarget::sequential(coeffs).unwrap();
    broken.alpha_rows[3].push(1.0);
    assert!(!broken.strictly_lower_nilpotent());
}

/// Linear f(y) = J y: the sequential residual of stage i,
/// `(I - h gamma J) K_i - h J (y + sum_{j<i} alpha_ij K_j) - h J sum_{j<i} Gamma_ij K_j`,
/// and the full-block residual
/// `K_i - h J sum_j (beta_ij - alpha_ij) K_j - h J (y + sum_j alpha_ij K_j)`.
fn residuals(
    coeffs: &rodas5p_core::Rodas5pCoefficients,
    jacobian: &DenseMatrix,
    y: &[f64],
    h: f64,
    stages: &[Vec<f64>],
) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let n = y.len();
    let s = coeffs.stages();
    let combine = |weights: &dyn Fn(usize) -> f64, upto: usize| {
        let mut out = vec![0.0; n];
        for (j, stage) in stages.iter().enumerate().take(upto) {
            for (value, k) in out.iter_mut().zip(stage) {
                *value += weights(j) * k;
            }
        }
        out
    };
    let jv = |v: &[f64]| jacobian.matvec(v).unwrap();
    let mut sequential = Vec::new();
    let mut block = Vec::new();
    for (i, stage) in stages.iter().enumerate().take(s) {
        let delta = combine(&|j| coeffs.alpha[(i, j)], i);
        let mix = combine(&|j| coeffs.gamma_matrix[(i, j)], i);
        let state = y.iter().zip(&delta).map(|(a, b)| a + b).collect::<Vec<_>>();
        let (jk, js, jm) = (jv(stage), jv(&state), jv(&mix));
        sequential.push(
            (0..n)
                .map(|q| stage[q] - h * coeffs.gamma * jk[q] - h * js[q] - h * jm[q])
                .collect(),
        );
        let coupling = combine(&|j| coeffs.beta[(i, j)] - coeffs.alpha[(i, j)], s);
        let full_delta = combine(&|j| coeffs.alpha[(i, j)], s);
        let full_state = y
            .iter()
            .zip(&full_delta)
            .map(|(a, b)| a + b)
            .collect::<Vec<_>>();
        let (jc, jf) = (jv(&coupling), jv(&full_state));
        block.push((0..n).map(|q| stage[q] - h * jc[q] - h * jf[q]).collect());
    }
    (sequential, block)
}

#[test]
fn block_and_sequential_residuals_differ_within_the_quantified_allowance() {
    let coeffs = rodas5p_coefficients().unwrap();
    let (discrepancy, upper_alpha) = block_sequential_allowance(coeffs).unwrap();
    let jacobian = DenseMatrix::from_rows(&[&[-2.0, 3.0], &[0.0, -0.5]]).unwrap();
    let absolute = DenseMatrix::from_rows(&[&[2.0, 3.0], &[0.0, 0.5]]).unwrap();
    let y = [0.125, -0.0625];
    let h = 0.25;
    let stages = (0..coeffs.stages())
        .map(|i| vec![0.1 * (1.0 + i as f64).sin(), -0.05 * (2.0 + i as f64).cos()])
        .collect::<Vec<_>>();
    let (sequential, block) = residuals(coeffs, &jacobian, &y, h, &stages);
    let s = coeffs.stages();
    for i in 0..s {
        let mut weighted = [0.0_f64; 2];
        for (j, stage) in stages.iter().enumerate() {
            for q in 0..2 {
                // Linear f: the Lipschitz bound is |J| as well.
                weighted[q] += (discrepancy[i][j] + upper_alpha[i][j]) * stage[q].abs();
            }
        }
        let allowance = absolute.matvec(&weighted).unwrap();
        for q in 0..2 {
            let difference = (block[i][q] - sequential[i][q]).abs();
            // Evaluation rounding of the two residuals: a few ulps of their
            // terms, all below 1.
            let rounding = 64.0 * f64::EPSILON;
            assert!(
                difference <= h * allowance[q] + rounding,
                "stage {i} component {q}: {difference:e} > {:e}",
                h * allowance[q] + rounding
            );
        }
    }
}

#[test]
#[ignore = "writes fixtures/stage_target_semantics.json; run with VIGILODE_WRITE_STAGE_TARGET=1"]
fn write_stage_target_semantics() {
    if std::env::var("VIGILODE_WRITE_STAGE_TARGET").as_deref() != Ok("1") {
        return;
    }
    let coeffs = rodas5p_coefficients().unwrap();
    let bits = vec![
        StageTarget::sequential(coeffs).unwrap().coefficient_bits(),
        StageTarget::strict_lower_projection(coeffs)
            .unwrap()
            .coefficient_bits(),
    ];
    std::fs::write(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/stage_target_semantics.json"
        ),
        serde_json::to_string_pretty(&bits).unwrap() + "\n",
    )
    .unwrap();
    let _ = hex;
}
