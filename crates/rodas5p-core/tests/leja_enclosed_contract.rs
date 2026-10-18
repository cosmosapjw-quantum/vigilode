//! Independent rational-oracle fixtures for the complete represented Leja action.
//! Set LEJA_ENCLOSURE_RUNS to export the actual binary64 inputs and report bits;
//! research/cloud_completion_20261018/leja_fraction_oracle.py checks them using
//! exact Fraction intervals and spectral projectors, independently of Leja.
use rodas5p_core::leja_action::{
    CertifiedLejaCoeffCache, CertifiedLejaWorkspace, LEJA_ENCLOSED_SCHEMA, LejaAdmissionError,
    leja_phi_action_certified_capped,
    leja_phi_action_enclosed_with_workspace,
};
use rodas5p_core::polynomial_action::{
    JointPhiInput, SymmetricNonpositiveOperator, TotalErrorStatus,
};
use rodas5p_core::{DenseMatrix, WorkCounters};
use serde_json::{Value, json};

fn bits(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}
fn vec_bits(x: &[f64]) -> Vec<String> {
    x.iter().copied().map(bits).collect()
}

#[test]
fn exact_spectral_fixtures_export() {
    let mut rows: Vec<Value> = Vec::new();
    // Orthogonal matrices are exact binary64 and have rational entries.
    let h4: Vec<Vec<f64>> = vec![
        vec![0.5, 0.5, 0.5, 0.5],
        vec![0.5, -0.5, 0.5, -0.5],
        vec![0.5, 0.5, -0.5, -0.5],
        vec![0.5, -0.5, -0.5, 0.5],
    ];
    let id4: Vec<Vec<f64>> = (0..4)
        .map(|i| (0..4).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
        .collect();
    let cases = [
        (
            "diagonal",
            vec![-0.25, -1.0, -3.0, -10.0],
            id4.clone(),
            0.5,
            1e-8,
            64,
            1.0,
        ),
        (
            "dense-cancellation",
            vec![-4.0, -5.0, -6.0, -7.0],
            h4.clone(),
            0.5,
            1e-8,
            64,
            1.0,
        ),
        (
            "zero-step",
            vec![-4.0, -5.0, -6.0, -7.0],
            h4.clone(),
            0.0,
            1e-8,
            64,
            1.0,
        ),
        (
            "smallest-input",
            vec![-4.0, -5.0, -6.0, -7.0],
            h4.clone(),
            0.5,
            1e-8,
            32,
            f64::from_bits(1),
        ),
        (
            "subnormal-step",
            vec![-0.25, -1.0, -3.0, -10.0],
            id4.clone(),
            f64::from_bits(1),
            1e-8,
            32,
            1.0,
        ),
        (
            "unmet-budget",
            vec![-0.25, -1.0, -3.0, -10.0],
            id4.clone(),
            0.5,
            1e-20,
            1,
            1.0,
        ),
        (
            "semidefinite",
            vec![0.0, -1.0, -3.0, -10.0],
            id4.clone(),
            0.1,
            1e-8,
            64,
            1.0,
        ),
        (
            "scalar-nonbinary-product",
            vec![-0.1],
            vec![vec![1.0]],
            0.3,
            1e-8,
            32,
            1.0,
        ),
        (
            "large-output",
            vec![-4.0, -5.0, -6.0, -7.0],
            h4,
            0.5,
            1e93,
            64,
            1e100,
        ),
    ];
    for (name, eigenvalues, q, h, budget, cap, magnitude) in cases {
        let n = eigenvalues.len();
        let mut matrix = DenseMatrix::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                matrix[(i, j)] = (0..n).map(|a| q[i][a] * eigenvalues[a] * q[j][a]).sum();
            }
        }
        let matrix_bits: Vec<Vec<String>> = (0..n).map(|i| vec_bits(matrix.row(i))).collect();
        let operator = SymmetricNonpositiveOperator::gershgorin(matrix).unwrap();
        let vectors: [Vec<f64>; 5] = std::array::from_fn(|k| {
            (0..n)
                .map(|i| {
                    let sign = if (k + i) % 2 == 0 { 1.0 } else { -1.0 };
                    sign * magnitude * (i + 1) as f64 / (k + 1) as f64
                })
                .collect()
        });
        for same in [false, true] {
            let scales = [1.0, -0.5, 0.25, 2.0, -1.0];
            let input = if same {
                JointPhiInput::SameVector {
                    vector: &vectors[0],
                    scales,
                }
            } else {
                JointPhiInput::Distinct(&vectors)
            };
            let historical = leja_phi_action_certified_capped(
                &operator,
                h,
                input,
                budget,
                cap,
                &mut WorkCounters::default(),
            )
            .unwrap();
            assert!(matches!(
                historical.total_error(),
                TotalErrorStatus::EstimateOnly { .. }
            ));
            let mut workspace = CertifiedLejaWorkspace::new(n, if same { 1 } else { 5 });
            let report = leja_phi_action_enclosed_with_workspace(
                &operator,
                h,
                input,
                budget,
                cap,
                None,
                &mut WorkCounters::default(),
                &mut workspace,
                None,
            )
            .unwrap();
            assert_eq!(report.schema(), LEJA_ENCLOSED_SCHEMA);
            assert!(
                matches!(report.total_error(), TotalErrorStatus::Certified { bound } if *bound == report.total_bound())
            );
            assert_eq!(historical.fused(), report.fused());
            assert_eq!(
                historical.total_bound().to_bits(),
                report.total_bound().to_bits()
            );
            assert!(report.total_bound().is_finite() && report.total_bound() >= 0.0);
            if name == "unmet-budget" {
                assert!(!report.converged());
            }
            rows.push(json!({
                "name": name, "same_vector": same, "eigenvalues": vec_bits(&eigenvalues),
                "q": q.iter().map(|v| vec_bits(v)).collect::<Vec<_>>(), "matrix": matrix_bits,
                "h": bits(h), "vectors": vectors.iter().map(|v| vec_bits(v)).collect::<Vec<_>>(),
                "scales": vec_bits(&scales), "fused": vec_bits(report.fused()),
                "bound": bits(report.total_bound()), "degree": report.degree(),
                "converged": report.converged(), "total_error": report.total_error(),
                "certification_ops": report.certification_ops(),
            }));
        }
    }
    assert_eq!(rows.len(), 18);
    if let Ok(path) = std::env::var("LEJA_ENCLOSURE_RUNS") {
        let path = std::path::PathBuf::from(path);
        let path = if path.is_absolute() {
            path
        } else {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path)
        };
        std::fs::write(
            path,
            serde_json::to_string_pretty(
                &json!({"schema": "vigilode-leja-enclosure-fixtures-v1", "rows": rows}),
            )
            .unwrap()
                + "\n",
        )
        .unwrap();
    }
}

#[test]
fn explicit_enclosure_rejects_declared_domain_and_keeps_success_transaction() {
    let mut a = DenseMatrix::zeros(1, 1);
    a[(0, 0)] = -1.0;
    let verified = SymmetricNonpositiveOperator::gershgorin(a.clone()).unwrap();
    let declared =
        SymmetricNonpositiveOperator::with_declared_enclosure(a, 1.0, 1.0, "unverified").unwrap();
    let input = JointPhiInput::SameVector {
        vector: &[1.0],
        scales: [1.0, 0.0, 0.0, 0.0, 0.0],
    };
    let mut workspace = CertifiedLejaWorkspace::new(1, 1);
    let mut work = WorkCounters::default();
    let committed = leja_phi_action_enclosed_with_workspace(
        &verified,
        1.0,
        input,
        1e-8,
        8,
        None,
        &mut work,
        &mut workspace,
        None,
    )
    .unwrap()
    .clone();
    assert!(
        leja_phi_action_enclosed_with_workspace(
            &declared,
            1.0,
            input,
            1e-8,
            8,
            None,
            &mut work,
            &mut workspace,
            None,
        )
        .is_err()
    );
    assert_eq!(workspace.report().unwrap(), &committed);
}

#[test]
fn admitted_fused_output_keeps_bound_budget_and_owner_together() {
    let mut a = DenseMatrix::zeros(2, 2);
    a[(0, 0)] = -1.0;
    a[(1, 1)] = -8.0;
    let operator = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let input = JointPhiInput::SameVector {
        vector: &[1.0, -2.0],
        scales: [1.0, 0.0, 0.0, 0.0, 0.0],
    };
    let mut workspace = CertifiedLejaWorkspace::new(2, 1);
    let report = leja_phi_action_enclosed_with_workspace(
        &operator,
        0.5,
        input,
        1e-20,
        1,
        None,
        &mut WorkCounters::default(),
        &mut workspace,
        None,
    )
    .unwrap();
    assert!(!report.converged());
    assert!(matches!(
        report.admit_requested_budget(),
        Err(LejaAdmissionError::BoundExceedsBudget { .. })
    ));
    let original_budget = report.total_budget();
    let bound = report.total_bound();
    let token = report.admit_enclosed_fused(bound).unwrap();
    assert_eq!(token.bound(), bound);
    assert_eq!(token.budget(), bound);
    assert_eq!(token.fused().as_ptr(), report.fused().as_ptr());
    assert_eq!(report.total_budget(), original_budget);
    assert!(!report.converged());
    let smaller = f64::from_bits(bound.to_bits() - 1);
    assert!(matches!(
        report.admit_enclosed_fused(smaller),
        Err(LejaAdmissionError::BoundExceedsBudget { .. })
    ));
}

#[test]
fn admission_refuses_legacy_initial_workspace_and_invalid_budgets() {
    let initial_workspace = CertifiedLejaWorkspace::new(1, 1);
    assert!(matches!(
        initial_workspace.report().unwrap().admit_enclosed_fused(1.0),
        Err(LejaAdmissionError::Uncertified)
    ));
    let mut a = DenseMatrix::zeros(1, 1);
    a[(0, 0)] = -1.0;
    let operator = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let input = JointPhiInput::SameVector {
        vector: &[1.0],
        scales: [1.0, 0.0, 0.0, 0.0, 0.0],
    };
    let legacy = leja_phi_action_certified_capped(
        &operator,
        1.0,
        input,
        1e-8,
        8,
        &mut WorkCounters::default(),
    )
    .unwrap();
    assert!(legacy.converged());
    assert!(matches!(
        legacy.admit_enclosed_fused(f64::MAX),
        Err(LejaAdmissionError::Uncertified)
    ));
    let mut workspace = CertifiedLejaWorkspace::new(1, 1);
    let report = leja_phi_action_enclosed_with_workspace(
        &operator,
        1.0,
        input,
        1e-8,
        8,
        None,
        &mut WorkCounters::default(),
        &mut workspace,
        None,
    )
    .unwrap();
    assert!(report.admit_requested_budget().is_ok());
    for invalid in [-1.0, f64::NEG_INFINITY, f64::INFINITY, f64::NAN] {
        assert!(matches!(
            report.admit_enclosed_fused(invalid),
            Err(LejaAdmissionError::InvalidBudget { .. })
        ));
    }
}

#[test]
fn coefficient_cache_reuse_does_not_reuse_another_operators_output() {
    // The operators have identical Gershgorin enclosures and coefficient
    // cache keys, but opposite off-diagonal signs and different actions.
    let operators: Vec<_> = [1.0, -1.0]
        .into_iter()
        .map(|off| {
            let mut a = DenseMatrix::zeros(2, 2);
            a[(0, 0)] = -3.0;
            a[(1, 1)] = -3.0;
            a[(0, 1)] = off;
            a[(1, 0)] = off;
            SymmetricNonpositiveOperator::gershgorin(a).unwrap()
        })
        .collect();
    let input = JointPhiInput::SameVector {
        vector: &[1.0, 0.0],
        scales: [1.0, 0.0, 0.0, 0.0, 0.0],
    };
    let mut cache = CertifiedLejaCoeffCache::new();
    let mut workspace = CertifiedLejaWorkspace::new(2, 1);
    let mut first = Vec::new();
    for (i, operator) in operators.iter().enumerate() {
        let report = leja_phi_action_enclosed_with_workspace(
            operator,
            0.125,
            input,
            1e-8,
            32,
            Some(&mut cache),
            &mut WorkCounters::default(),
            &mut workspace,
            None,
        )
        .unwrap();
        let admitted = report.admit_requested_budget().unwrap();
        if i == 0 {
            assert_eq!(report.coefficient_setups(), 1);
            assert!(admitted.fused()[1] > admitted.bound());
            first = admitted.fused().to_vec();
        } else {
            assert_eq!(report.coefficient_setups(), 0);
            assert!(admitted.fused()[1] < -admitted.bound());
            assert_eq!(admitted.fused()[0], first[0]);
            assert_eq!(admitted.fused()[1], -first[1]);
        }
    }
}

#[test]
fn refused_overflow_attempt_does_not_change_admissible_report() {
    let mut a = DenseMatrix::zeros(1, 1);
    a[(0, 0)] = -1.0;
    let operator = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let mut workspace = CertifiedLejaWorkspace::new(1, 1);
    let mut work = WorkCounters::default();
    let committed = leja_phi_action_enclosed_with_workspace(
        &operator,
        0.0,
        JointPhiInput::SameVector {
            vector: &[1.0],
            scales: [1.0, 0.0, 0.0, 0.0, 0.0],
        },
        1e-8,
        8,
        None,
        &mut work,
        &mut workspace,
        None,
    )
    .unwrap()
    .clone();
    assert!(committed.admit_requested_budget().is_ok());
    let result = leja_phi_action_enclosed_with_workspace(
        &operator,
        0.0,
        JointPhiInput::SameVector {
            vector: &[f64::MAX],
            scales: [2.0, 0.0, 0.0, 0.0, 0.0],
        },
        1e-8,
        8,
        None,
        &mut work,
        &mut workspace,
        None,
    );
    assert!(result.is_err());
    assert_eq!(workspace.report(), Some(&committed));
    assert!(workspace.report().unwrap().admit_requested_budget().is_ok());
}

#[test]
fn exact_zero_fused_action_can_be_admitted_at_zero_budget() {
    let mut a = DenseMatrix::zeros(1, 1);
    a[(0, 0)] = -1.0;
    let operator = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let mut workspace = CertifiedLejaWorkspace::new(1, 1);
    let report = leja_phi_action_enclosed_with_workspace(
        &operator,
        0.5,
        JointPhiInput::SameVector {
            vector: &[0.0],
            scales: [1.0; 5],
        },
        1e-8,
        8,
        None,
        &mut WorkCounters::default(),
        &mut workspace,
        None,
    )
    .unwrap();
    let token = report.admit_enclosed_fused(0.0).unwrap();
    assert_eq!(token.bound(), 0.0);
    assert_eq!(token.fused(), &[0.0]);
}
