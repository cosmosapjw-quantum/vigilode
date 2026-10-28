use rodas5p_core::invariant_module::*;
use rodas5p_core::{DenseMatrix, direct_solve};

fn matrix(n: usize, m: usize, a: &[f64]) -> DenseMatrix {
    DenseMatrix::new(n, m, a.to_vec()).unwrap()
}
fn pair() -> (DenseMatrix, DenseMatrix, DenseMatrix, DenseMatrix) {
    // Two identical damped Jordan blocks; repeated eigenvalue is retained.
    (
        matrix(
            4,
            4,
            &[
                -2., 1., 0., 0., 0., -2., 0., 0., 0., 0., -2., 1., 0., 0., 0., -2.,
            ],
        ),
        matrix(4, 2, &[1., 0., 0., 1., 1., 0., 0., 1.]),
        matrix(2, 4, &[1., 0., 0., 0., 0., 1., 0., 0.]),
        matrix(2, 2, &[-2., 1., 0., -2.]),
    )
}
fn plan() -> InvariantModulePlan {
    let (b, v, w, h) = pair();
    InvariantModulePlan::new(7, b, v, w, h, InvariantModuleConfig::default()).unwrap()
}
fn eval(
    p: &InvariantModulePlan,
    shifts: &[f64],
    rhs: Vec<f64>,
    work: &mut InvariantModuleWork,
) -> InvariantModuleReport {
    p.evaluate(
        7,
        shifts,
        &[rhs],
        1e-11,
        InvariantModuleSolveBudget::default(),
        work,
    )
    .unwrap()
}
#[test]
fn jordan_module_matches_direct_without_diagonalization() {
    let p = plan();
    let mut work = InvariantModuleWork::default();
    let rhs = vec![1., 2., 1., 2.];
    let report = eval(&p, &[0., 0.25, 1.], rhs.clone(), &mut work);
    let (b, _, _, _) = pair();
    for c in &report.candidates {
        let mut a = DenseMatrix::identity(4);
        for i in 0..4 {
            for j in 0..4 {
                a[(i, j)] -= c.alpha() * b[(i, j)];
            }
        }
        let reference = direct_solve(&a, &rhs).unwrap();
        let actual = &c.accepted_values().unwrap()[0];
        assert!(
            reference
                .iter()
                .zip(actual)
                .all(|(x, y)| (x - y).abs() < 1e-12)
        );
    }
    assert_eq!(work.factorizations, 3);
    assert_eq!(p.evidence().reduced_dimension, 2);
}
#[test]
fn repeated_actual_shifts_are_counted_once() {
    let mut work = InvariantModuleWork::default();
    let r = eval(&plan(), &[0.25, 0.25, 0.5, 0.25], vec![1.; 4], &mut work);
    assert_eq!(r.distinct_shifts, 2);
    assert_eq!(work.factorizations, 2);
    assert_eq!(work.reused_shift_factorizations, 2);
    assert_eq!(work.original_residual_products, 64);
    assert_eq!(work.dissipativity_symmetric_entries, 0);
}
#[test]
fn signed_zero_is_one_shift() {
    let mut work = InvariantModuleWork::default();
    let r = eval(&plan(), &[0.0, -0.0], vec![1.; 4], &mut work);
    assert_eq!(r.distinct_shifts, 1);
    assert_eq!(work.factorizations, 1);
}
#[test]
fn out_of_module_rhs_is_refused_by_original_residual() {
    let mut work = InvariantModuleWork::default();
    let r = eval(&plan(), &[1.], vec![1., 2., 3., 4.], &mut work);
    assert!(r.candidates[0].accepted_values().is_none());
    assert!(r.candidates[0].error_l2_upper()[0] > 1.);
    assert_eq!(work.refused_shift_groups, 1);
}
#[test]
fn stale_epoch_refuses_before_work() {
    let mut work = InvariantModuleWork::default();
    assert!(
        plan()
            .evaluate(
                8,
                &[1.],
                &[vec![1.; 4]],
                1e-10,
                InvariantModuleSolveBudget::default(),
                &mut work
            )
            .is_err()
    );
    assert_eq!(work.factorizations, 0);
    assert_eq!(work.rhs_restriction_products, 0);
}
#[test]
fn rank_change_refuses_at_admission() {
    let (b, mut v, w, h) = pair();
    for i in 0..4 {
        v[(i, 1)] = 0.;
    }
    assert!(InvariantModulePlan::new(7, b, v, w, h, InvariantModuleConfig::default()).is_err());
}
#[test]
fn leakage_refuses_at_admission() {
    let (mut b, v, w, h) = pair();
    b[(2, 2)] -= 0.1;
    assert!(InvariantModulePlan::new(7, b, v, w, h, InvariantModuleConfig::default()).is_err());
}
#[test]
fn approximate_module_still_needs_current_residual() {
    let (mut b, v, w, h) = pair();
    b[(2, 2)] -= 1e-6;
    let p = InvariantModulePlan::new(
        7,
        b,
        v,
        w,
        h,
        InvariantModuleConfig {
            invariance_tolerance: 2e-6,
            ..Default::default()
        },
    )
    .unwrap();
    let mut work = InvariantModuleWork::default();
    let report = eval(&p, &[1.], vec![1.; 4], &mut work);
    assert!(report.candidates[0].accepted_values().is_none());
}
#[test]
fn oblique_chart_amplification_is_not_discarded() {
    let b = matrix(2, 2, &[-2., 0., 0., -2.]);
    let v = matrix(2, 1, &[1., 0.]);
    let w = matrix(1, 2, &[1., 1000.]);
    let h = matrix(1, 1, &[-2.]);
    let err = InvariantModulePlan::new(
        1,
        b.clone(),
        v.clone(),
        w.clone(),
        h.clone(),
        InvariantModuleConfig {
            max_projector_amplification: 100.,
            ..Default::default()
        },
    );
    assert!(err.is_err());
    let p = InvariantModulePlan::new(1, b, v, w, h, InvariantModuleConfig::default()).unwrap();
    assert_eq!(p.evidence().projector_amplification_inf_upper, 1001.);
}
#[test]
fn unproved_full_space_inverse_is_refused() {
    let b = matrix(2, 2, &[1., 0., 0., 1.]);
    assert!(
        InvariantModulePlan::new(
            1,
            b.clone(),
            DenseMatrix::identity(2),
            DenseMatrix::identity(2),
            b,
            InvariantModuleConfig::default()
        )
        .is_err()
    );
}
#[test]
fn storage_and_shift_budgets_refuse_before_work() {
    let (b, v, w, h) = pair();
    assert!(
        InvariantModulePlan::new(
            1,
            b,
            v,
            w,
            h,
            InvariantModuleConfig {
                max_stored_scalars: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    for budget in [
        InvariantModuleSolveBudget {
            max_distinct_shifts: 0,
            ..Default::default()
        },
        InvariantModuleSolveBudget {
            max_targets: 0,
            ..Default::default()
        },
        InvariantModuleSolveBudget {
            max_output_scalars: 1,
            ..Default::default()
        },
        InvariantModuleSolveBudget {
            max_work_units: 1,
            ..Default::default()
        },
    ] {
        let mut work = InvariantModuleWork::default();
        assert!(
            plan()
                .evaluate(7, &[1.], &[vec![1.; 4]], 1e-10, budget, &mut work)
                .is_err()
        );
        assert_eq!(work.rhs_restriction_products, 0);
    }
}
#[test]
fn invalid_targets_and_nonfinite_rhs_refuse_before_work() {
    for shift in [-1., f64::NAN, f64::INFINITY] {
        let mut work = InvariantModuleWork::default();
        assert!(
            plan()
                .evaluate(
                    7,
                    &[shift],
                    &[vec![1.; 4]],
                    1e-10,
                    InvariantModuleSolveBudget::default(),
                    &mut work
                )
                .is_err()
        );
        assert_eq!(work.factorizations, 0);
    }
}

#[test]
fn matrix_mutation_before_admission_cannot_forge_evidence() {
    let (mut b, v, w, h) = pair();
    b[(3, 3)] = f64::NAN;
    assert!(InvariantModulePlan::new(1, b, v, w, h, InvariantModuleConfig::default()).is_err());
}

#[test]
fn cached_full_space_certifier_accepts_an_independent_full_solve() {
    let rhs = vec![vec![1., 2., 3., 4.]];
    let (b, _, _, _) = pair();
    let mut a = DenseMatrix::identity(4);
    for i in 0..4 {
        for j in 0..4 {
            a[(i, j)] -= b[(i, j)];
        }
    }
    let x = vec![direct_solve(&a, &rhs[0]).unwrap()];
    let c = plan().certify_candidates(7, 1., &rhs, &x, 1e-11).unwrap();
    assert_eq!(
        c.status(),
        rodas5p_core::shared_shift_jet::CertificateStatus::Certified
    );
}

#[test]
fn cached_certifier_cannot_accept_missing_or_malformed_columns() {
    let p = plan();
    let rhs = vec![vec![1.; 4]];
    for x in [
        Vec::new(),
        vec![vec![0.; 3]],
        vec![vec![f64::NAN; 4]],
        vec![vec![0.; 4], vec![0.; 4]],
    ] {
        assert!(p.certify_candidates(7, 1., &rhs, &x, 1e-10).is_err());
    }
    assert!(p.certify_candidates(8, 1., &rhs, &rhs, 1e-10).is_err());
    assert!(p.certify_candidates(7, -1., &rhs, &rhs, 1e-10).is_err());
    assert!(p.certify_candidates(7, 1., &[], &[], 1e-10).is_err());
    assert!(p.certify_candidates(7, 1., &rhs, &rhs, 0.).is_err());
    assert_eq!(
        p.certify_candidates(7, 1., &rhs, &rhs, 1e-10)
            .unwrap()
            .status(),
        rodas5p_core::shared_shift_jet::CertificateStatus::Rejected
    );
}
