//! Outward stage-target certificates against the R3 exact-root fixtures
//! (re-audit R3 of 2026-10-01: HOM-02, HOM-03, HOM-04).
//!
//! 24 rows (four families, h in {0.1, 1, 10}, candidates q1 and q2) from
//! `certified_majorant.py` on the strict-lower-projection target: candidate
//! stage bits, rational exact-root brackets, and the recorded decisions
//! (output WRMS <= 0.1: 12 of 24; combined proxy <= 1: 8; radius 1e-4
//! closes: 22).

use rodas5p_core::rodas5p_coefficients;
use rodas5p_integrators::{
    CertificateKind, InverseWitness, PastStepData, QuadraticStageProblem, StageTarget,
    certify_stage_target, doubling_certificate, predict_state_radius,
};
use serde::Deserialize;

const FIXTURES: &str = include_str!("../../../fixtures/r3_homotopy_certificate_fixtures.json");

#[derive(Deserialize)]
struct Fixtures {
    atol: f64,
    rtol: f64,
    doubling_radius: f64,
    rows: Vec<Row>,
}

#[derive(Deserialize)]
struct Row {
    family: String,
    h: f64,
    method: String,
    jacobian: Vec<Vec<f64>>,
    y: Vec<f64>,
    q: Vec<f64>,
    candidate: Vec<Vec<String>>,
    #[allow(dead_code)]
    exact_root: Vec<Vec<[String; 2]>>,
    yhat: Vec<String>,
    ehat: Vec<String>,
    stage_distance: Vec<Vec<[String; 2]>>,
    output_distance: Vec<[String; 2]>,
    embedded_distance: Vec<[String; 2]>,
    python_output_wrms: f64,
    python_combined_proxy_upper: f64,
    accept_output_01: bool,
    accept_combined_1: bool,
    doubling_closes: bool,
}

fn bits(hex: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

fn vector(hexes: &[String]) -> Vec<f64> {
    hexes.iter().map(|hex| bits(hex)).collect()
}

/// `bound >= |x - x*|`, given the exact distance `|x - x*|` enclosed in
/// `[down, up]` by the rational oracle. An undecidable case (the bound
/// inside a nondegenerate one-ULP enclosure) fails the test too.
fn encloses(bound: f64, distance: &[String; 2]) -> bool {
    let (down, up) = (bits(&distance[0]), bits(&distance[1]));
    bound >= up || (down == up && bound >= down)
}

fn load() -> Fixtures {
    serde_json::from_str(FIXTURES).unwrap()
}

#[test]
fn every_fixture_is_enclosed_and_every_decision_matches() {
    let fixtures = load();
    let coeffs = rodas5p_coefficients().unwrap();
    let target = StageTarget::strict_lower_projection(coeffs).unwrap();
    assert_eq!(fixtures.rows.len(), 24);
    for row in &fixtures.rows {
        let label = format!("{} h={} {}", row.family, row.h, row.method);
        let problem = QuadraticStageProblem {
            jacobian: row.jacobian.clone(),
            y: row.y.clone(),
            h: row.h,
            q: row.q.clone(),
        };
        let candidate = row
            .candidate
            .iter()
            .map(|stage| vector(stage))
            .collect::<Vec<_>>();
        let (y_hat, e_hat) = (vector(&row.yhat), vector(&row.ehat));
        let witness = InverseWitness::small(&problem, target.gamma).unwrap();
        let certificate = certify_stage_target(
            &target,
            &problem,
            &candidate,
            &y_hat,
            &e_hat,
            &witness,
            fixtures.atol,
            fixtures.rtol,
        )
        .unwrap();
        assert_eq!(certificate.kind, CertificateKind::StageTargetBound);
        for i in 0..candidate.len() {
            for a in 0..problem.y.len() {
                assert!(
                    encloses(certificate.stage_bound[i][a], &row.stage_distance[i][a]),
                    "{label}: stage {i}.{a} bound {:e} vs exact distance {:?}",
                    certificate.stage_bound[i][a],
                    row.stage_distance[i][a]
                );
            }
        }
        for a in 0..problem.y.len() {
            assert!(
                encloses(certificate.output_bound[a], &row.output_distance[a]),
                "{label}: output {a}"
            );
            assert!(
                encloses(
                    certificate.embedded_difference_bound[a],
                    &row.embedded_distance[a]
                ),
                "{label}: embedded {a}"
            );
        }
        // Decisions agree with the Python reference; the Rust bound is at
        // most the Python one (it rounds outward only when needed).
        assert_eq!(
            certificate.output_wrms_upper <= 0.1,
            row.accept_output_01,
            "{label}"
        );
        assert_eq!(
            certificate.combined_proxy_upper <= 1.0,
            row.accept_combined_1,
            "{label}"
        );
        assert!(
            certificate.output_wrms_upper <= row.python_output_wrms * (1.0 + 1.0e-9),
            "{label}: {} vs {}",
            certificate.output_wrms_upper,
            row.python_output_wrms
        );
        assert!(
            certificate.combined_proxy_upper <= row.python_combined_proxy_upper * (1.0 + 1.0e-9),
            "{label}"
        );
        assert!(
            certificate.embedded_target_wrms_lower <= certificate.embedded_target_wrms_upper,
            "{label}"
        );

        let doubling = doubling_certificate(
            &target,
            &problem,
            &candidate,
            &y_hat,
            &e_hat,
            &witness,
            fixtures.atol,
            fixtures.rtol,
            fixtures.doubling_radius,
            1,
            1,
        )
        .unwrap();
        assert_eq!(doubling.attempts.len(), 1);
        assert_eq!(doubling.attempts[0].closes, row.doubling_closes, "{label}");
        if let Some(doubled) = &doubling.certificate {
            for i in 0..candidate.len() {
                for a in 0..problem.y.len() {
                    assert!(
                        encloses(doubled.stage_bound[i][a], &row.stage_distance[i][a]),
                        "{label}: doubling stage {i}.{a}"
                    );
                }
            }
        }
    }
}

#[test]
fn the_path_sum_is_bitwise_identical_for_every_worker_count() {
    let fixtures = load();
    let target = StageTarget::strict_lower_projection(rodas5p_coefficients().unwrap()).unwrap();
    let row = fixtures
        .rows
        .iter()
        .find(|row| row.family == "nonnormal_quadratic" && row.h == 1.0 && row.method == "q2")
        .unwrap();
    let problem = QuadraticStageProblem {
        jacobian: row.jacobian.clone(),
        y: row.y.clone(),
        h: row.h,
        q: row.q.clone(),
    };
    let candidate = row
        .candidate
        .iter()
        .map(|stage| vector(stage))
        .collect::<Vec<_>>();
    let witness = InverseWitness::small(&problem, target.gamma).unwrap();
    let run = |workers| {
        doubling_certificate(
            &target,
            &problem,
            &candidate,
            &vector(&row.yhat),
            &vector(&row.ehat),
            &witness,
            fixtures.atol,
            fixtures.rtol,
            fixtures.doubling_radius,
            1,
            workers,
        )
        .unwrap()
    };
    let serial = run(1);
    for workers in [2, 4, 8] {
        let parallel = run(workers);
        assert_eq!(
            parallel.certificate, serial.certificate,
            "{workers} workers"
        );
    }
}

#[test]
fn witnesses_bind_their_operator_and_fail_closed() {
    let target = StageTarget::strict_lower_projection(rodas5p_coefficients().unwrap()).unwrap();
    let problem = QuadraticStageProblem {
        jacobian: vec![vec![-1.0, 100.0], vec![0.0, -1.0]],
        y: vec![0.0, 1.0],
        h: 1.0,
        q: vec![0.0, 0.0],
    };
    let candidate = vec![vec![0.0, -0.5]; 8];
    let small = InverseWitness::small(&problem, target.gamma).unwrap();
    // A witness built for another step size is refused.
    let other = QuadraticStageProblem {
        h: 2.0,
        ..problem.clone()
    };
    let wrong = InverseWitness::small(&other, target.gamma).unwrap();
    assert!(
        certify_stage_target(
            &target,
            &problem,
            &candidate,
            &[0.0, 0.5],
            &[0.0, 0.0],
            &wrong,
            1e-8,
            1e-6
        )
        .unwrap_err()
        .to_string()
        .contains("another operator")
    );
    // A diagonal witness is unavailable for a nondiagonal Jacobian.
    assert!(InverseWitness::diagonal(&problem, target.gamma).is_err());
    // An approximate inverse V with a verified ||I - V W|| < 1 bounds the
    // exact small witness from above.
    let w = [
        [1.0 + target.gamma, -100.0 * target.gamma],
        [0.0, 1.0 + target.gamma],
    ];
    let det = w[0][0] * w[1][1];
    let v = vec![
        vec![w[1][1] / det, -w[0][1] / det],
        vec![0.0, w[0][0] / det],
    ];
    let approximate =
        InverseWitness::approximate(&problem, target.gamma, &v, "dense-approximate").unwrap();
    assert!(approximate.residual_norm_upper() < 1.0e-12);
    for a in 0..2 {
        for b in 0..2 {
            assert!(approximate.upper()[a][b] >= small.upper()[a][b] * (1.0 - 1.0e-12));
        }
    }
    let bad_v = vec![vec![0.0, 0.0], vec![0.0, 0.0]];
    assert!(InverseWitness::approximate(&problem, target.gamma, &bad_v, "zero").is_err());
    // A target whose rows are not strictly lower is rejected.
    let mut broken = target.clone();
    broken.alpha_rows[2].push(0.5);
    assert!(
        certify_stage_target(
            &broken,
            &problem,
            &candidate,
            &[0.0, 0.5],
            &[0.0, 0.0],
            &small,
            1e-8,
            1e-6
        )
        .is_err()
    );
    // Overflow fails closed.
    let huge = QuadraticStageProblem {
        y: vec![1.0e300, 1.0e300],
        q: vec![1.0e300, 0.0],
        ..problem
    };
    let witness = InverseWitness::small(&huge, target.gamma).unwrap();
    assert!(
        certify_stage_target(
            &target,
            &huge,
            &candidate,
            &[1.0e300, 1.0e300],
            &[0.0, 0.0],
            &witness,
            1e-8,
            1e-6
        )
        .is_err()
    );
}

#[test]
fn the_radius_predictor_sees_only_past_steps() {
    // First step: the floor.
    let first = PastStepData {
        previous_max_residual: None,
        previous_h: None,
        order: 5,
    };
    assert_eq!(
        predict_state_radius(&first, 0.1, 1.0e-4, 10.0).unwrap(),
        1.0e-4
    );
    let later = PastStepData {
        previous_max_residual: Some(1.0e-3),
        previous_h: Some(0.05),
        order: 1,
    };
    // kappa max|r| (h / h_prev)^p = 10 * 1e-3 * 2.
    let radius = predict_state_radius(&later, 0.1, 1.0e-4, 10.0).unwrap();
    assert!((radius / 2.0e-2 - 1.0).abs() < 1.0e-15);
}
