use rodas5p_core::DenseMatrix;
use rodas5p_core::shared_shift_jet::{
    CertificateStatus, SharedShiftJetConfig, certify_shift_candidate, shared_shift_jet,
};

fn scalar(value: f64) -> DenseMatrix {
    DenseMatrix::new(1, 1, vec![value]).unwrap()
}

#[test]
fn exact_current_target_certificate_rejects_wrong_candidate() {
    let j = scalar(-1.0);
    let good = certify_shift_candidate(&j, 1.0, 1.0, &[vec![2.0]], &[vec![1.0]], 1e-10).unwrap();
    assert_eq!(good.status(), CertificateStatus::Certified);
    assert_eq!(good.rhs_error_upper(), &[0.0]);
    let wrong = certify_shift_candidate(&j, 1.0, 0.5, &[vec![2.0]], &[vec![1.0]], 1e-10).unwrap();
    assert_eq!(wrong.status(), CertificateStatus::Rejected);
    assert_eq!(wrong.rhs_error_upper(), &[0.5]);
}

#[test]
fn independent_columns_and_executed_work_are_preserved() {
    let j = DenseMatrix::from_rows(&[&[-2.0, 3.0], &[-3.0, -2.0]]).unwrap();
    let rhs = vec![vec![1.0, 0.0], vec![0.0, 2.0]];
    let gammas = [0.4375, 0.5, 0.5625];
    let config = SharedShiftJetConfig::default();
    let report = shared_shift_jet(&j, 0.25, 0.5, &rhs, &gammas, config).unwrap();
    assert_eq!(report.candidates().len(), 3);
    assert_eq!(report.work().factorizations, 1);
    assert_eq!(report.work().rhs_solves, 2 * (config.degree + 1));
    assert_eq!(report.work().solve_batches, config.degree + 1);
    assert_eq!(report.work().recurrence_depth, config.degree);
    assert_eq!(report.work().supplied_rhs_columns, 2);
    assert_eq!(
        report.work().evaluation_multiply_add_pairs,
        3 * 2 * 2 * config.degree
    );
    assert_eq!(report.work().residual_matrix_entry_products, 3 * 2 * 2 * 2);
    for candidate in report.candidates() {
        assert_eq!(candidate.rhs_columns().len(), 2);
        assert_eq!(
            candidate.certificate().status(),
            CertificateStatus::Certified
        );
        assert!(
            candidate
                .certificate()
                .rhs_error_upper()
                .iter()
                .all(|b| *b <= 1e-10)
        );
    }
}

#[test]
fn under_resolution_and_too_small_tolerance_fail_closed() {
    let config = SharedShiftJetConfig {
        degree: 0,
        ..SharedShiftJetConfig::default()
    };
    let report =
        shared_shift_jet(&scalar(-8.0), 1.0, 0.5, &[vec![1.0]], &[0.5625], config).unwrap();
    assert_eq!(
        report.candidates()[0].certificate().status(),
        CertificateStatus::Rejected
    );
    let config = SharedShiftJetConfig {
        absolute_tolerance: f64::MIN_POSITIVE,
        ..SharedShiftJetConfig::default()
    };
    let report = shared_shift_jet(&scalar(-2.0), 0.5, 0.5, &[vec![1.0]], &[0.5], config).unwrap();
    assert_eq!(
        report.candidates()[0].certificate().status(),
        CertificateStatus::Rejected
    );
}

#[test]
fn identity_limits_and_extreme_center_scaling() {
    for (j, h) in [(scalar(0.0), 1.0), (scalar(-2.0), 0.0)] {
        let report = shared_shift_jet(
            &j,
            h,
            0.5,
            &[vec![3.0]],
            &[0.4375, 0.5625],
            SharedShiftJetConfig::default(),
        )
        .unwrap();
        for candidate in report.candidates() {
            assert_eq!(candidate.rhs_columns(), &[vec![3.0]]);
            assert_eq!(
                candidate.certificate().status(),
                CertificateStatus::Certified
            );
        }
    }
    for exponent in [-400, 400] {
        let gamma0 = 2_f64.powi(exponent);
        let j = scalar(-2_f64.powi(-exponent));
        let report = shared_shift_jet(
            &j,
            1.0,
            gamma0,
            &[vec![1.0]],
            &[gamma0 * 0.875, gamma0 * 1.125],
            SharedShiftJetConfig::default(),
        )
        .unwrap();
        assert!(
            report
                .candidates()
                .iter()
                .all(|c| c.certificate().status() == CertificateStatus::Certified)
        );
    }
}

#[test]
fn invalid_inputs_unsupported_operators_and_budgets_are_errors() {
    let config = SharedShiftJetConfig::default();
    let valid = scalar(-1.0);
    for j in [
        scalar(1.0),
        DenseMatrix::from_rows(&[&[-1.0, 100.0], &[0.0, -1.0]]).unwrap(),
    ] {
        let rhs = vec![vec![1.0; j.nrows()]];
        assert!(shared_shift_jet(&j, 1.0, 0.5, &rhs, &[0.5], config).is_err());
    }
    let mut nonfinite = valid.clone();
    nonfinite[(0, 0)] = f64::NAN;
    assert!(shared_shift_jet(&nonfinite, 1.0, 0.5, &[vec![1.0]], &[0.5], config).is_err());
    assert!(
        shared_shift_jet(
            &DenseMatrix::zeros(0, 0),
            1.0,
            0.5,
            &[vec![]],
            &[0.5],
            config
        )
        .is_err()
    );
    assert!(shared_shift_jet(&valid, 1.0, 0.5, &[], &[0.5], config).is_err());
    assert!(shared_shift_jet(&valid, 1.0, 0.5, &[vec![1.0]], &[], config).is_err());
    assert!(shared_shift_jet(&valid, 1.0, 0.5, &[vec![]], &[0.5], config).is_err());
    assert!(shared_shift_jet(&valid, 1.0, 0.5, &[vec![f64::INFINITY]], &[0.5], config).is_err());
    for h in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(shared_shift_jet(&valid, h, 0.5, &[vec![1.0]], &[0.5], config).is_err());
    }
    for center in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(shared_shift_jet(&valid, 1.0, center, &[vec![1.0]], &[0.5], config).is_err());
    }
    for target in [0.0, -1.0, 1.0, f64::NAN, f64::INFINITY] {
        assert!(shared_shift_jet(&valid, 1.0, 0.5, &[vec![1.0]], &[target], config).is_err());
    }
    for bad_config in [
        SharedShiftJetConfig {
            degree: usize::MAX,
            ..config
        },
        SharedShiftJetConfig {
            absolute_tolerance: 0.0,
            ..config
        },
        SharedShiftJetConfig {
            absolute_tolerance: f64::NAN,
            ..config
        },
        SharedShiftJetConfig {
            max_stored_scalars: 0,
            ..config
        },
        SharedShiftJetConfig {
            max_work_units: 0,
            ..config
        },
    ] {
        assert!(shared_shift_jet(&valid, 1.0, 0.5, &[vec![1.0]], &[0.5], bad_config).is_err());
    }
    assert!(
        certify_shift_candidate(&valid, 1.0, 1.0, &[vec![1.0]], &[vec![f64::NAN]], 1e-10).is_err()
    );
    assert!(certify_shift_candidate(&valid, 1.0, 1.0, &[vec![1.0]], &[], 1e-10).is_err());
    assert!(certify_shift_candidate(&valid, 1.0, 1.0, &[vec![1.0]], &[vec![1.0]], -1.0).is_err());
    assert!(
        shared_shift_jet(
            &scalar(-f64::MAX),
            f64::MAX,
            0.5,
            &[vec![1.0]],
            &[0.5],
            config
        )
        .is_err()
    );
}

#[test]
fn large_target_set_budget_includes_coordinate_gamma_and_tolerance_slots() {
    let gammas = vec![0.5; 64];
    let config = SharedShiftJetConfig {
        degree: 0,
        max_stored_scalars: 392,
        ..SharedShiftJetConfig::default()
    };
    let report = shared_shift_jet(&scalar(-1.0), 1.0, 0.5, &[vec![1.0]], &gammas, config).unwrap();
    assert_eq!(
        report.work().explicit_storage_upper_scalars_excluding_lu,
        392
    );
    let too_small = SharedShiftJetConfig {
        max_stored_scalars: 391,
        ..config
    };
    assert!(shared_shift_jet(&scalar(-1.0), 1.0, 0.5, &[vec![1.0]], &gammas, too_small).is_err());
}
