//! One common represented output and budget for all three polynomial families.
use rodas5p_core::polynomial_action::{JointPhiInput, SymmetricNonpositiveOperator};
use rodas5p_core::polynomial_router::{
    GuardedPolynomialConfig, GuardedPolynomialWorkspace, PolynomialAttemptStatus,
    PolynomialBackend as B, PolynomialRefusalStage, PolynomialRoutingMode,
    guarded_polynomial_action,
};
use rodas5p_core::{DenseMatrix, WorkCounters};
use serde_json::{Value, json};

fn bits(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}
fn vector_bits(v: &[f64]) -> Vec<String> {
    v.iter().copied().map(bits).collect()
}
type SpectralFixture = (DenseMatrix, Vec<Vec<f64>>, Vec<f64>, [Vec<f64>; 5]);

fn fixture() -> SpectralFixture {
    let q = vec![
        vec![0.5, 0.5, 0.5, 0.5],
        vec![0.5, -0.5, 0.5, -0.5],
        vec![0.5, 0.5, -0.5, -0.5],
        vec![0.5, -0.5, -0.5, 0.5],
    ];
    let eigen = vec![-4.0, -5.0, -6.0, -7.0];
    let mut matrix = DenseMatrix::zeros(4, 4);
    for i in 0..4 {
        for j in 0..4 {
            matrix[(i, j)] = (0..4).map(|a| q[i][a] * eigen[a] * q[j][a]).sum();
        }
    }
    let vectors = std::array::from_fn(|k| {
        (0..4)
            .map(|i| ((i + 1) as f64) * 0.125 * if (i + k) % 2 == 0 { 1.0 } else { -1.0 })
            .collect()
    });
    (matrix, q, eigen, vectors)
}
fn all_config() -> GuardedPolynomialConfig {
    GuardedPolynomialConfig {
        mode: PolynomialRoutingMode::CompareAll,
        ..GuardedPolynomialConfig::default()
    }
}

#[test]
fn identical_fused_request_exact_oracle_export() {
    let (matrix, q, eigen, vectors) = fixture();
    let op = SymmetricNonpositiveOperator::gershgorin(matrix.clone()).unwrap();
    let mut workspace = GuardedPolynomialWorkspace::default();
    let scales = [1.0, -0.5, 0.25, 0.0, -0.125];
    let mut rows: Vec<Value> = vec![];
    for h in [0.0, 0.03125, 0.125] {
        for same in [false, true] {
            let input = if same {
                JointPhiInput::SameVector {
                    vector: &vectors[0],
                    scales,
                }
            } else {
                JointPhiInput::Distinct(&vectors)
            };
            let mut work = WorkCounters::default();
            let result = guarded_polynomial_action(
                &op,
                h,
                input,
                1e-7,
                all_config(),
                &mut workspace,
                &mut work,
            )
            .unwrap();
            assert_eq!(result.attempts().len(), 3);
            assert_eq!(work, result.work());
            assert_eq!(result.total_budget(), 1e-7);
            let mut sum = WorkCounters::default();
            for attempt in result.attempts() {
                assert!(attempt.bound().is_some(), "h={h}, same={same}, {attempt:?}");
                assert!(attempt.bound().unwrap() <= 1e-7);
                assert!(attempt.cost().measured_allocated_bytes.is_none());
                sum.accumulate(attempt.cost().work);
                rows.push(json!({
                "name":format!("{:?}-h{h}",attempt.backend()),
                "same_vector":same, "eigenvalues":vector_bits(&eigen),
                "q":q.iter().map(|v|vector_bits(v)).collect::<Vec<_>>(),
                "matrix":(0..4).map(|i|(0..4).map(|j|bits(matrix[(i,j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
                "vectors":vectors.iter().map(|v|vector_bits(v)).collect::<Vec<_>>(),
                "scales":vector_bits(&scales), "h":bits(h),
                "fused":vector_bits(attempt.fused().unwrap()), "bound":bits(attempt.bound().unwrap()),
                "degree":attempt.degree(), "converged":true,
                "request_sha256":result.request_sha256(),"total_budget":bits(result.total_budget()),
                "route_cost":attempt.cost(),
            }));
            }
            assert_eq!(sum, result.work());
        }
    }
    if let Ok(path) = std::env::var("POLYNOMIAL_ROUTER_RUNS") {
        std::fs::write(
            path,
            serde_json::to_string_pretty(&json!({
                "schema":"vigilode-polynomial-router-fixtures-v1", "rows":rows
            }))
            .unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn first_admitted_stops_and_retains_owned_output_across_reuse() {
    let (matrix, _, _, vectors) = fixture();
    let op = SymmetricNonpositiveOperator::gershgorin(matrix).unwrap();
    let mut workspace = GuardedPolynomialWorkspace::default();
    let config = GuardedPolynomialConfig::default();
    let mut work = WorkCounters::default();
    let first = guarded_polynomial_action(
        &op,
        0.125,
        JointPhiInput::Distinct(&vectors),
        1e-7,
        config,
        &mut workspace,
        &mut work,
    )
    .unwrap();
    assert_eq!(first.attempts().len(), 1);
    assert_eq!(first.selected().unwrap().backend(), B::EnclosedLeja);
    let stored = first.fused().unwrap().to_vec();
    let before = work;
    let same = guarded_polynomial_action(
        &op,
        0.125,
        JointPhiInput::Distinct(&vectors),
        1e-7,
        config,
        &mut workspace,
        &mut work,
    )
    .unwrap();
    assert_eq!(first.request_sha256(), same.request_sha256());
    assert_eq!(same.work(), work.checked_delta(before).unwrap());
    assert_eq!(same.fused(), first.fused());
    assert!(
        same.selected()
            .unwrap()
            .cost()
            .leja_setup
            .unwrap()
            .subtotal()
            .cache_hits()
            > 0
    );
    let changed: [Vec<f64>; 5] =
        std::array::from_fn(|k| vectors[k].iter().map(|x| 2.0 * x).collect());
    let later = guarded_polynomial_action(
        &op,
        0.125,
        JointPhiInput::Distinct(&changed),
        1e-7,
        config,
        &mut workspace,
        &mut work,
    )
    .unwrap();
    assert_ne!(first.request_sha256(), later.request_sha256());
    assert_eq!(first.fused().unwrap(), stored);
}

#[test]
fn declared_spectrum_abstains_and_malformed_requests_are_errors() {
    let (matrix, _, _, vectors) = fixture();
    let op =
        SymmetricNonpositiveOperator::with_declared_enclosure(matrix, 0.0, 10.0, "test").unwrap();
    let mut workspace = GuardedPolynomialWorkspace::default();
    let mut work = WorkCounters::default();
    let result = guarded_polynomial_action(
        &op,
        0.0,
        JointPhiInput::Distinct(&vectors),
        1e-7,
        all_config(),
        &mut workspace,
        &mut work,
    )
    .unwrap();
    assert!(result.is_abstained());
    assert!(result.fused().is_none());
    assert_eq!(work, WorkCounters::default());
    for a in result.attempts() {
        assert!(matches!(
            a.status(),
            PolynomialAttemptStatus::Abstained {
                stage: PolynomialRefusalStage::Domain,
                ..
            }
        ));
    }
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            guarded_polynomial_action(
                &op,
                0.0,
                JointPhiInput::Distinct(&vectors),
                bad,
                all_config(),
                &mut workspace,
                &mut work
            )
            .is_err()
        );
    }
    let mut duplicate = all_config();
    duplicate.order[1] = duplicate.order[0];
    assert!(
        guarded_polynomial_action(
            &op,
            0.0,
            JointPhiInput::Distinct(&vectors),
            1e-7,
            duplicate,
            &mut workspace,
            &mut work
        )
        .is_err()
    );
    let mut nonfinite = vectors.clone();
    nonfinite[2][1] = f64::NAN;
    assert!(
        guarded_polynomial_action(
            &op,
            0.0,
            JointPhiInput::Distinct(&nonfinite),
            1e-7,
            all_config(),
            &mut workspace,
            &mut work
        )
        .is_err()
    );
}

#[test]
fn tight_budget_is_not_relaxed_and_failures_keep_work() {
    let (matrix, _, _, vectors) = fixture();
    let op = SymmetricNonpositiveOperator::gershgorin(matrix).unwrap();
    let mut workspace = GuardedPolynomialWorkspace::default();
    let mut work = WorkCounters::default();
    let config = GuardedPolynomialConfig {
        leja_degree_cap: 1,
        ..all_config()
    };
    let result = guarded_polynomial_action(
        &op,
        0.5,
        JointPhiInput::Distinct(&vectors),
        1e-24,
        config,
        &mut workspace,
        &mut work,
    )
    .unwrap();
    assert!(result.is_abstained(), "{result:?}");
    assert_eq!(result.total_budget(), 1e-24);
    assert!(result.fused().is_none());
    assert!(work.poly_vector_products > 0);
    let mut sum = WorkCounters::default();
    for a in result.attempts() {
        sum.accumulate(a.cost().work);
        assert!(a.fused().is_none());
    }
    assert_eq!(sum, work);
}

#[test]
fn unsupported_laguerre_degree_skips_all_matrix_products() {
    let (matrix, _, _, vectors) = fixture();
    let op = SymmetricNonpositiveOperator::gershgorin(matrix).unwrap();
    let mut workspace = GuardedPolynomialWorkspace::default();
    let mut work = WorkCounters::default();
    let config = GuardedPolynomialConfig {
        order: [B::Laguerre, B::Chebyshev, B::EnclosedLeja],
        leja_degree_cap: 1,
        ..all_config()
    };
    let result = guarded_polynomial_action(
        &op,
        10.0,
        JointPhiInput::Distinct(&vectors),
        1e-10,
        config,
        &mut workspace,
        &mut work,
    )
    .unwrap();
    let lag = &result.attempts()[0];
    assert!(lag.degree().unwrap() > 128);
    assert!(matches!(
        lag.status(),
        PolynomialAttemptStatus::Abstained {
            stage: PolynomialRefusalStage::Preflight,
            ..
        }
    ));
    assert_eq!(lag.cost().work.poly_vector_products, 0);
    assert_eq!(lag.cost().polynomial.executed_backends, 0);
    assert!(lag.cost().polynomial.plan_tail_evaluations > 0);
}

#[test]
fn each_family_can_be_the_guarded_selected_backend() {
    let (matrix, _, _, vectors) = fixture();
    let op = SymmetricNonpositiveOperator::gershgorin(matrix).unwrap();
    let mut workspace = GuardedPolynomialWorkspace::default();
    for order in [
        [B::EnclosedLeja, B::Laguerre, B::Chebyshev],
        [B::Laguerre, B::EnclosedLeja, B::Chebyshev],
        [B::Chebyshev, B::Laguerre, B::EnclosedLeja],
    ] {
        let mut work = WorkCounters::default();
        let config = GuardedPolynomialConfig {
            order,
            ..GuardedPolynomialConfig::default()
        };
        let result = guarded_polynomial_action(
            &op,
            0.03125,
            JointPhiInput::Distinct(&vectors),
            1e-7,
            config,
            &mut workspace,
            &mut work,
        )
        .unwrap();
        assert_eq!(result.attempts().len(), 1);
        assert_eq!(result.selected().unwrap().backend(), order[0]);
        assert!(result.bound().unwrap() <= 1e-7);
    }
}
