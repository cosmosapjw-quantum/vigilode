//! Contracts of the structural diagonal certificate pipeline (research node
//! `research/int02_structural_certificate_20261003`, integrated DAG node
//! INT-02): bitwise equality with the dense path on the R3 fixtures, O(n)
//! storage, and every stale, edited or mismatched input rejected.

use rodas5p_core::{CoreResult, WorkCounters, rodas5p_coefficients};
use rodas5p_integrators::{
    DIAGONAL_STRUCTURED, DiagonalQuadraticModel, DiagonalStageProblem, InverseWitness,
    ModelBinding, Q2Admission, Q2CertificateSource, QuadraticModel, QuadraticStageProblem,
    StageCertificate, StageTarget, TransactionalQ1Q2Config, certify_stage_target,
    certify_stage_target_diagonal, transactional_q1_q2_step_with_admission,
};
use serde::Deserialize;

const FIXTURES: &str = include_str!("../../../fixtures/r3_homotopy_certificate_fixtures.json");

#[derive(Deserialize)]
struct Fixtures {
    atol: f64,
    rtol: f64,
    rows: Vec<Row>,
}

#[derive(Deserialize)]
struct Row {
    jacobian: Vec<Vec<f64>>,
    y: Vec<f64>,
    q: Vec<f64>,
    h: f64,
    candidate: Vec<Vec<String>>,
    yhat: Vec<String>,
    ehat: Vec<String>,
}

fn bits(hex: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

fn vector(hexes: &[String]) -> Vec<f64> {
    hexes.iter().map(|hex| bits(hex)).collect()
}

fn same_bits(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

/// The bound fields of two certificates are bitwise equal.
fn same_bounds(a: &StageCertificate, b: &StageCertificate) -> bool {
    a.stage_bound.len() == b.stage_bound.len()
        && a.stage_bound
            .iter()
            .zip(&b.stage_bound)
            .all(|(x, y)| same_bits(x, y))
        && same_bits(&a.output_bound, &b.output_bound)
        && same_bits(&a.embedded_difference_bound, &b.embedded_difference_bound)
        && a.output_wrms_upper.to_bits() == b.output_wrms_upper.to_bits()
        && a.embedded_difference_wrms_upper.to_bits() == b.embedded_difference_wrms_upper.to_bits()
        && a.embedded_target_wrms_upper.to_bits() == b.embedded_target_wrms_upper.to_bits()
        && a.embedded_target_wrms_lower.to_bits() == b.embedded_target_wrms_lower.to_bits()
}

fn diagonal_of(jacobian: &[Vec<f64>]) -> Option<Vec<f64>> {
    let n = jacobian.len();
    (0..n)
        .all(|a| (0..n).all(|b| a == b || jacobian[a][b] == 0.0))
        .then(|| (0..n).map(|a| jacobian[a][a]).collect())
}

#[test]
fn diagonal_fixtures_give_bitwise_the_dense_bounds_with_fewer_slots_and_operations() {
    let fixtures: Fixtures = serde_json::from_str(FIXTURES).unwrap();
    let coeffs = rodas5p_coefficients().unwrap();
    let target = StageTarget::strict_lower_projection(coeffs).unwrap();
    let mut compared = 0;
    for row in &fixtures.rows {
        let Some(diagonal) = diagonal_of(&row.jacobian) else {
            continue;
        };
        let candidate: Vec<Vec<f64>> = row.candidate.iter().map(|s| vector(s)).collect();
        let (y_hat, e_hat) = (vector(&row.yhat), vector(&row.ehat));
        let dense = QuadraticStageProblem {
            jacobian: row.jacobian.clone(),
            y: row.y.clone(),
            h: row.h,
            q: row.q.clone(),
        };
        let structured = DiagonalStageProblem {
            diagonal,
            y: row.y.clone(),
            h: row.h,
            q: row.q.clone(),
        };
        assert_eq!(structured.to_dense(), dense);
        let dense_witness = InverseWitness::diagonal(&dense, target.gamma).unwrap();
        let witness = InverseWitness::diagonal_structured(&structured, target.gamma).unwrap();
        assert_eq!(&*witness.upper(), &*dense_witness.upper());
        let n = row.y.len();
        assert_eq!(witness.stored_slots(), n);
        assert_eq!(dense_witness.stored_slots(), n * n);
        let a = certify_stage_target(
            &target,
            &dense,
            &candidate,
            &y_hat,
            &e_hat,
            &dense_witness,
            fixtures.atol,
            fixtures.rtol,
        )
        .unwrap();
        let b = certify_stage_target_diagonal(
            &target,
            &structured,
            &candidate,
            &y_hat,
            &e_hat,
            &witness,
            fixtures.atol,
            fixtures.rtol,
        )
        .unwrap();
        assert!(same_bounds(&a, &b));
        assert!(b.directed_operations <= a.directed_operations);
        assert!(b.is_bound_to_diagonal(
            &target,
            &structured,
            &candidate,
            &y_hat,
            &e_hat,
            witness.identity(),
            fixtures.atol,
            fixtures.rtol,
        ));
        compared += 1;
    }
    assert!(compared >= 6, "{compared} diagonal fixture rows");
}

fn small_case() -> (
    StageTarget,
    DiagonalStageProblem,
    Vec<Vec<f64>>,
    Vec<f64>,
    Vec<f64>,
) {
    let coeffs = rodas5p_coefficients().unwrap();
    let target = StageTarget::sequential(coeffs).unwrap();
    let problem = DiagonalStageProblem {
        diagonal: vec![-1.0, -2.5, -4.0],
        y: vec![1.0, 0.5, -0.25],
        h: 0.1,
        q: vec![-0.05, -0.1, 0.0],
    };
    let candidate: Vec<Vec<f64>> = (0..8)
        .map(|i| {
            (0..3)
                .map(|a| 1.0e-3 * (i as f64 + 1.0) * (a as f64 - 1.0))
                .collect()
        })
        .collect();
    let y_hat = problem.y.clone();
    let e_hat = vec![1.0e-6; 3];
    (target, problem, candidate, y_hat, e_hat)
}

#[test]
fn a_witness_for_another_operator_or_of_the_other_storage_is_rejected() {
    let (target, problem, candidate, y_hat, e_hat) = small_case();
    let certify = |p: &DiagonalStageProblem, w: &InverseWitness| {
        certify_stage_target_diagonal(&target, p, &candidate, &y_hat, &e_hat, w, 1e-6, 1e-6)
    };
    let good = InverseWitness::diagonal_structured(&problem, target.gamma).unwrap();
    assert!(certify(&problem, &good).is_ok());
    // Another h, gamma or diagonal.
    let mut other_h = problem.clone();
    other_h.h = 0.2;
    let mut other_d = problem.clone();
    other_d.diagonal[1] = -2.0;
    for witness in [
        InverseWitness::diagonal_structured(&other_h, target.gamma).unwrap(),
        InverseWitness::diagonal_structured(&other_d, target.gamma).unwrap(),
        InverseWitness::diagonal_structured(&problem, target.gamma * 1.5).unwrap(),
    ] {
        assert!(certify(&problem, &witness).is_err());
    }
    // A dense witness on the diagonal path, and the converse.
    let dense = problem.to_dense();
    let dense_witness = InverseWitness::diagonal(&dense, target.gamma).unwrap();
    assert!(certify(&problem, &dense_witness).is_err());
    assert!(
        certify_stage_target(
            &target, &dense, &candidate, &y_hat, &e_hat, &good, 1e-6, 1e-6
        )
        .is_err()
    );
}

#[test]
fn an_edited_wire_witness_is_rejected_and_an_intact_one_is_rebuilt() {
    let (target, problem, ..) = small_case();
    let good = InverseWitness::diagonal_structured(&problem, target.gamma).unwrap();
    let wire = good.to_unverified();
    assert_eq!(wire.identity.structure, DIAGONAL_STRUCTURED);
    let rebuilt = wire
        .clone()
        .verify_diagonal(&problem, target.gamma)
        .unwrap();
    assert_eq!(rebuilt, good);
    let mut zeroed = wire.clone();
    zeroed.upper[1][1] = 0.0;
    let mut off_diagonal = wire.clone();
    off_diagonal.upper[0][2] = 1.0;
    let mut renamed = wire.clone();
    renamed.identity.structure = "diagonal".into();
    for edited in [zeroed, off_diagonal, renamed] {
        assert!(edited.verify_diagonal(&problem, target.gamma).is_err());
    }
    let mut other = problem.clone();
    other.y[0] = 2.0; // J is the declared diagonal; y does not enter the witness
    assert!(wire.clone().verify_diagonal(&other, target.gamma).is_ok());
    other.diagonal[0] = -1.5;
    assert!(wire.verify_diagonal(&other, target.gamma).is_err());
}

#[test]
fn bad_shapes_and_non_finite_data_are_rejected() {
    let (target, problem, candidate, y_hat, e_hat) = small_case();
    let mut short = problem.clone();
    short.diagonal.pop();
    let mut nan = problem.clone();
    nan.diagonal[2] = f64::NAN;
    let mut bad_q = problem.clone();
    bad_q.q.push(0.0);
    for p in [&short, &nan, &bad_q] {
        assert!(InverseWitness::diagonal_structured(p, target.gamma).is_err());
    }
    let witness = InverseWitness::diagonal_structured(&problem, target.gamma).unwrap();
    let mut wrong_candidate = candidate.clone();
    wrong_candidate[3].pop();
    assert!(
        certify_stage_target_diagonal(
            &target,
            &problem,
            &wrong_candidate,
            &y_hat,
            &e_hat,
            &witness,
            1e-6,
            1e-6
        )
        .is_err()
    );
    let mut singular = problem.clone();
    singular.diagonal[0] = 1.0 / (singular.h * target.gamma);
    assert!(InverseWitness::diagonal_structured(&singular, target.gamma).is_err());
}

#[test]
fn a_stale_certificate_is_not_bound() {
    let (target, problem, candidate, y_hat, e_hat) = small_case();
    let witness = InverseWitness::diagonal_structured(&problem, target.gamma).unwrap();
    let cert = certify_stage_target_diagonal(
        &target, &problem, &candidate, &y_hat, &e_hat, &witness, 1e-6, 1e-6,
    )
    .unwrap();
    let bound = |p: &DiagonalStageProblem, c: &[Vec<f64>], y: &[f64]| {
        cert.is_bound_to_diagonal(&target, p, c, y, &e_hat, witness.identity(), 1e-6, 1e-6)
    };
    assert!(bound(&problem, &candidate, &y_hat));
    let mut moved = candidate.clone();
    moved[4][1] = f64::from_bits(moved[4][1].to_bits() + 1);
    assert!(!bound(&problem, &moved, &y_hat));
    let mut other_y = y_hat.clone();
    other_y[2] += 1.0e-12;
    assert!(!bound(&problem, &candidate, &other_y));
    let mut other_q = problem.clone();
    other_q.q[0] = -0.06;
    assert!(!bound(&other_q, &candidate, &y_hat));
    // The dense binding of the same problem is a different subject.
    assert!(!cert.is_bound_to(
        &target,
        &problem.to_dense(),
        &candidate,
        &y_hat,
        &e_hat,
        witness.identity(),
        1e-6,
        1e-6
    ));
}

#[test]
fn a_non_diagonal_model_is_not_a_diagonal_model() {
    let a = vec![vec![-1.0, 0.1], vec![0.0, -2.0]];
    let model = QuadraticModel::new("upper", a, vec![-0.05, -0.05]).unwrap();
    assert!(DiagonalQuadraticModel::new(model).is_err());
}

/// A source whose diagonal problem is for another state.
struct Lying(DiagonalQuadraticModel);

impl Q2CertificateSource for Lying {
    fn stage_problem(&self, t: f64, y: &[f64], h: f64) -> CoreResult<QuadraticStageProblem> {
        self.0.stage_problem(t, y, h)
    }
    fn binding(&self) -> ModelBinding {
        self.0.binding()
    }
    fn diagonal_stage_problem(
        &self,
        t: f64,
        y: &[f64],
        h: f64,
    ) -> Option<CoreResult<DiagonalStageProblem>> {
        let shifted: Vec<f64> = y.iter().map(|v| v * (1.0 + 1.0e-9)).collect();
        self.0.diagonal_stage_problem(t, &shifted, h)
    }
}

#[test]
fn a_prepared_problem_for_another_state_is_not_used() {
    let n = 4;
    let a = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| if i == j { -1.0 - i as f64 } else { 0.0 })
                .collect()
        })
        .collect();
    let q = (0..n).map(|i| -0.05 * (1 + i % 3) as f64).collect();
    let model = QuadraticModel::new("diag-4", a, q).unwrap();
    let problem = model.ode_problem().unwrap();
    let y: Vec<f64> = (0..n).map(|i| 1.0 + 0.1 * i as f64).collect();
    let honest = DiagonalQuadraticModel::new(model.clone()).unwrap();
    let lying = Lying(DiagonalQuadraticModel::new(model).unwrap());
    let config = TransactionalQ1Q2Config::default();
    let step = |source: &dyn Q2CertificateSource| {
        transactional_q1_q2_step_with_admission(
            &problem,
            0.0,
            &y,
            0.05,
            &config,
            1e-6,
            1e-6,
            false,
            Q2Admission::PreparedStructuredCertificate(source),
            &mut WorkCounters::default(),
        )
        .unwrap()
    };
    let good = step(&honest);
    let bad = step(&lying);
    let good_q2 = good.q2_certificate.expect("q=2 candidate certified");
    assert!(good_q2.certificate.is_some(), "{}", good_q2.reason);
    let bad_q2 = bad.q2_certificate.expect("q=2 candidate considered");
    assert!(!bad_q2.accepted && bad_q2.certificate.is_none());
    assert!(
        bad_q2.reason.contains("not this step's (y, h)"),
        "{}",
        bad_q2.reason
    );
}
