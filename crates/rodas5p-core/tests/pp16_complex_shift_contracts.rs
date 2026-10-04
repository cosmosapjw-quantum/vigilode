//! Contract tests of research node `research/pp16_complex_shift_gain_20261004`
//! (RVJ DAG node PP16): complex-shift certificates with gain `|gamma| / Re gamma`.

use rodas5p_core::complex_shift_jet::{
    ComplexShiftJetConfig, PartialFractionTerm, certify_complex_shift_candidate,
    certify_partial_fraction, complex_shift_jet,
};
use rodas5p_core::shared_shift_jet::{
    CertificateStatus, SharedShiftJetConfig, certify_shift_candidate, shared_shift_jet,
};
use rodas5p_core::{DenseMatrix, LuFactorization};

const TOL: f64 = 1e-10;

fn skew_damped() -> DenseMatrix {
    DenseMatrix::from_rows(&[&[-1.0, 3.0], &[-3.0, -2.0]]).unwrap()
}

fn solve(j: &DenseMatrix, h: f64, g: (f64, f64), b: (&[f64], &[f64])) -> (Vec<f64>, Vec<f64>) {
    let n = j.nrows();
    let mut big = DenseMatrix::zeros(2 * n, 2 * n);
    for i in 0..n {
        for k in 0..n {
            let p = if i == k { 1.0 } else { 0.0 } - g.0 * h * j[(i, k)];
            let q = g.1 * h * j[(i, k)];
            big[(i, k)] = p;
            big[(i, n + k)] = q;
            big[(n + i, k)] = -q;
            big[(n + i, n + k)] = p;
        }
    }
    let rhs: Vec<f64> = b.0.iter().chain(b.1).copied().collect();
    let x = LuFactorization::new(&big).unwrap().solve(&rhs).unwrap();
    (x[..n].to_vec(), x[n..].to_vec())
}

#[test]
fn scalar_hand_case_and_gain() {
    // A = 1 - (1 + i)(-2) = 3 + 2i, x = (3 - 2i) / 13.
    let j = DenseMatrix::from_rows(&[&[-2.0]]).unwrap();
    let u = ([3.0 / 13.0], [-2.0 / 13.0]);
    let c = certify_complex_shift_candidate(&j, 1.0, 1.0, 1.0, &[1.0], &[0.0], &u.0, &u.1, TOL)
        .unwrap();
    assert_eq!(c.status(), CertificateStatus::Certified);
    assert!(c.error_upper() <= 1e-15);
    let sqrt2 = std::f64::consts::SQRT_2;
    assert!(c.gain_upper() >= sqrt2 && c.gain_upper() <= sqrt2 * (1.0 + 4.0 * f64::EPSILON));
    let zero =
        certify_complex_shift_candidate(&j, 1.0, 1.0, 1.0, &[1.0], &[0.0], &[0.0], &[0.0], TOL)
            .unwrap();
    assert_eq!(zero.status(), CertificateStatus::Rejected);
    assert_eq!(zero.residual_l2_upper(), 1.0);
    assert!(zero.error_upper() >= sqrt2 && zero.error_upper() >= 1.0 / 13f64.sqrt());
    // Purely real gamma: gain exactly one.
    let real =
        certify_complex_shift_candidate(&j, 1.0, 0.5, 0.0, &[2.0], &[0.0], &[1.0], &[0.0], TOL)
            .unwrap();
    assert_eq!(real.gain_upper(), 1.0);
    assert_eq!(real.error_upper(), 0.0);
}

#[test]
fn gain_one_would_under_bound_complex_shift() {
    // J skew with eigenvector (1, i) for i; gamma = 1/16 - i gives
    // (I - gamma J) v = -(i/16) v, so x = 16 i b and ||x|| = 16 ||b||.
    let j = DenseMatrix::from_rows(&[&[0.0, 1.0], &[-1.0, 0.0]]).unwrap();
    let gamma = (1.0 / 16.0, -1.0);
    let b = ([1.0, 0.0], [0.0, 1.0]);
    let exact = ([0.0, -16.0], [16.0, 0.0]);
    let x = certify_complex_shift_candidate(
        &j, 1.0, gamma.0, gamma.1, &b.0, &b.1, &exact.0, &exact.1, TOL,
    )
    .unwrap();
    assert_eq!(x.error_upper(), 0.0);
    assert_eq!(x.status(), CertificateStatus::Certified);
    let zero = [0.0, 0.0];
    let c =
        certify_complex_shift_candidate(&j, 1.0, gamma.0, gamma.1, &b.0, &b.1, &zero, &zero, TOL)
            .unwrap();
    let actual = 16.0 * std::f64::consts::SQRT_2;
    assert!(c.residual_l2_upper() < actual / 15.0);
    assert!(c.error_upper() >= actual);
    assert!(c.gain_upper() >= (1.0f64 + 256.0).sqrt());
}

#[test]
fn real_gamma_reproduces_real_shift_status() {
    let j = skew_damped();
    let b = [1.0, -0.5];
    let zero = [0.0, 0.0];
    for (h, gamma) in [(0.25, 0.5), (1.0, 0.125), (0.0, 2.0)] {
        let (good, _) = solve(&j, h, (gamma, 0.0), (&b, &zero));
        let mut off = good.clone();
        off[0] += 1e-9;
        for u in [good.clone(), off, zero.to_vec(), b.to_vec()] {
            let real =
                certify_shift_candidate(&j, h, gamma, &[b.to_vec()], std::slice::from_ref(&u), TOL)
                    .unwrap();
            let complex =
                certify_complex_shift_candidate(&j, h, gamma, 0.0, &b, &zero, &u, &zero, TOL)
                    .unwrap();
            assert_eq!(
                real.status(),
                complex.status(),
                "h {h} gamma {gamma} u {u:?}"
            );
            assert_eq!(complex.gain_upper(), 1.0);
        }
    }
    // The jet with real targets and real B reproduces the real jet.
    let config = SharedShiftJetConfig::default();
    let gammas = [0.4375, 0.5, 0.5625];
    let real = shared_shift_jet(&j, 0.25, 0.5, &[b.to_vec()], &gammas, config).unwrap();
    let targets: Vec<_> = gammas.iter().map(|g| (*g, 0.0)).collect();
    let complex = complex_shift_jet(
        &j,
        0.25,
        0.5,
        &[b.to_vec()],
        &[zero.to_vec()],
        &targets,
        ComplexShiftJetConfig::default(),
    )
    .unwrap();
    for (r, c) in real.candidates().iter().zip(complex.candidates()) {
        assert_eq!(r.rhs_columns()[0], c.columns_re()[0]);
        assert!(c.columns_im()[0].iter().all(|v| *v == 0.0));
        assert_eq!(r.certificate().status(), c.status());
    }
}

#[test]
fn complex_jet_candidates_and_work() {
    let j = skew_damped();
    let rhs_re = vec![vec![1.0, -0.5], vec![0.0, 1.0]];
    let rhs_im = vec![vec![0.25, 0.75], vec![0.0, 0.0]];
    let targets = [(1.25, 0.25), (0.875, -0.375), (1.0, 0.0)];
    let config = ComplexShiftJetConfig {
        degree: 40,
        ..ComplexShiftJetConfig::default()
    };
    let report = complex_shift_jet(&j, 0.25, 1.0, &rhs_re, &rhs_im, &targets, config).unwrap();
    let work = report.work();
    assert_eq!(work.factorizations, 1);
    assert_eq!(work.solve_batches, 41);
    assert_eq!(work.rhs_solves, 4 * 41);
    assert_eq!(work.supplied_rhs_columns, 2);
    assert_eq!(work.target_shifts, 3);
    assert_eq!(work.evaluation_multiply_add_pairs, 4 * 3 * 2 * 2 * 40);
    assert_eq!(work.residual_matrix_entry_products, 4 * 3 * 2 * 4);
    for (candidate, target) in report.candidates().iter().zip(targets) {
        assert_eq!(candidate.gamma(), target);
        assert!(candidate.z_modulus_upper() < 1.0);
        assert_eq!(candidate.status(), CertificateStatus::Certified);
        for (c, ((ur, ui), (br, bi))) in candidate.certificates().iter().zip(
            candidate
                .columns_re()
                .iter()
                .zip(candidate.columns_im())
                .zip(rhs_re.iter().zip(&rhs_im)),
        ) {
            let (xr, xi) = solve(&j, 0.25, target, (br, bi));
            let diff: f64 = ur
                .iter()
                .zip(&xr)
                .chain(ui.iter().zip(&xi))
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>()
                .sqrt();
            assert!(diff <= c.error_upper() + 1e-15);
        }
    }
    // Under-resolved degree: a valid bound that is rejected.
    let low = complex_shift_jet(
        &j,
        0.25,
        1.0,
        &rhs_re,
        &rhs_im,
        &[(1.5, 0.25)],
        ComplexShiftJetConfig {
            degree: 1,
            ..config
        },
    )
    .unwrap();
    assert_eq!(low.candidates()[0].status(), CertificateStatus::Rejected);
}

#[test]
fn negative_controls_fail_closed() {
    let j = skew_damped();
    let b = ([1.0, -0.5], [0.25, 0.75]);
    let g = (0.25, 1.5);
    let (ur, ui) = solve(&j, 0.5, g, (&b.0, &b.1));
    let good =
        certify_complex_shift_candidate(&j, 0.5, g.0, g.1, &b.0, &b.1, &ur, &ui, TOL).unwrap();
    assert_eq!(good.status(), CertificateStatus::Certified);
    let err = |j: &DenseMatrix,
               h: f64,
               g: (f64, f64),
               br: &[f64],
               bi: &[f64],
               ur: &[f64],
               ui: &[f64],
               tol: f64| {
        certify_complex_shift_candidate(j, h, g.0, g.1, br, bi, ur, ui, tol).is_err()
    };
    for re in [
        0.0,
        -0.0,
        -0.25,
        f64::NAN,
        f64::INFINITY,
        -f64::MIN_POSITIVE,
    ] {
        assert!(
            err(&j, 0.5, (re, g.1), &b.0, &b.1, &ur, &ui, TOL),
            "Re gamma {re}"
        );
    }
    assert!(err(&j, 0.5, (g.0, f64::NAN), &b.0, &b.1, &ur, &ui, TOL));
    let nondissipative = DenseMatrix::from_rows(&[&[-1.0, 100.0], &[0.0, -1.0]]).unwrap();
    assert!(err(&nondissipative, 0.5, g, &b.0, &b.1, &ur, &ui, TOL));
    // DenseMatrix constructors refuse NaN; the entry is set afterwards.
    let mut nan_j = j.clone();
    nan_j[(0, 1)] = f64::NAN;
    assert!(err(&nan_j, 0.5, g, &b.0, &b.1, &ur, &ui, TOL));
    assert!(err(&j, f64::NAN, g, &b.0, &b.1, &ur, &ui, TOL));
    assert!(err(&j, -0.5, g, &b.0, &b.1, &ur, &ui, TOL));
    assert!(err(&j, 0.5, g, &[1.0, f64::INFINITY], &b.1, &ur, &ui, TOL));
    assert!(err(&j, 0.5, g, &b.0, &b.1, &[ur[0], f64::NAN], &ui, TOL));
    assert!(err(&j, 0.5, g, &b.0, &b.1, &ur[..1], &ui, TOL));
    assert!(err(&j, 0.5, g, &b.0, &b.1, &ur, &ui, 0.0));
    assert!(err(&j, 0.5, g, &b.0, &b.1, &ur, &ui, f64::NAN));
    // Wrong candidates are rejected.
    let mut off = ur.clone();
    off[1] += 1e-6;
    let (cr, ci) = solve(&j, 0.5, (g.0, -g.1), (&b.0, &b.1));
    for (wr, wi) in [
        (off.as_slice(), ui.as_slice()),
        (&cr, &ci),
        (&ui, &ur),
        (&[0.0, 0.0], &[0.0, 0.0]),
    ] {
        let c =
            certify_complex_shift_candidate(&j, 0.5, g.0, g.1, &b.0, &b.1, wr, wi, TOL).unwrap();
        assert_eq!(c.status(), CertificateStatus::Rejected);
    }
    // Jet radius |z| >= 1 and other refusals.
    let config = ComplexShiftJetConfig::default();
    let jet = |j: &DenseMatrix, g0: f64, targets: &[(f64, f64)], config| {
        complex_shift_jet(
            j,
            0.5,
            g0,
            &[b.0.to_vec()],
            &[b.1.to_vec()],
            targets,
            config,
        )
        .is_err()
    };
    for t in [
        (1.0, 1.0),
        (2.0, 0.0),
        (0.5, 0.9),
        (0.4, 0.8),
        (1.0, -1.0),
        (0.0, 0.5),
    ] {
        assert!(jet(&j, 1.0, &[(1.25, 0.25), t], config), "target {t:?}");
    }
    assert!(jet(&nondissipative, 1.0, &[(1.0, 0.25)], config));
    assert!(jet(&j, 1.0, &[(1.0, f64::NAN)], config));
    assert!(jet(&j, 0.0, &[(1.0, 0.25)], config));
    assert!(jet(&j, 1.0, &[], config));
    assert!(jet(
        &j,
        1.0,
        &[(1.0, 0.25)],
        ComplexShiftJetConfig {
            degree: 129,
            ..config
        }
    ));
    assert!(jet(
        &j,
        1.0,
        &[(1.0, 0.25)],
        ComplexShiftJetConfig {
            max_work_units: 0,
            ..config
        }
    ));
    assert!(complex_shift_jet(&j, 0.5, 1.0, &[b.0.to_vec()], &[], &[(1.0, 0.25)], config).is_err());
}

#[test]
fn partial_fraction_contracts() {
    // J = 0: every resolvent is the identity, so u_i = b is exact.
    let j = DenseMatrix::zeros(2, 2);
    let b = ([1.0, -0.5], [0.0, 0.0]);
    let terms = [
        PartialFractionTerm {
            gamma_re: 0.25,
            gamma_im: 0.5,
            weight_re: 0.1,
            weight_im: 0.3,
            candidate_re: &b.0,
            candidate_im: &b.1,
        },
        PartialFractionTerm {
            gamma_re: 0.25,
            gamma_im: -0.5,
            weight_re: 0.1,
            weight_im: -0.3,
            candidate_re: &b.0,
            candidate_im: &b.1,
        },
    ];
    let c = certify_partial_fraction(&j, 1.0, -1.0, 0.0, &terms, &b.0, &b.1, TOL).unwrap();
    assert_eq!(c.status(), CertificateStatus::Certified);
    assert!(c.weighted_term_error_upper() == 0.0);
    assert!(c.error_upper() <= 1e-15);
    // (c0 + 0.2) b, the imaginary parts cancel exactly in interval arithmetic.
    assert!((c.output_re()[0] + 0.8).abs() <= c.error_upper() + 1e-16);
    assert!(c.output_im().iter().all(|v| v.abs() <= c.error_upper()));
    assert_eq!(c.work().terms, 2);
    assert_eq!(c.work().residual_matrix_entry_products, 4 * 2 * 4);
    assert_eq!(c.work().combination_interval_products, 4 * 3 * 2);
    // One conjugate member alone is not merged: the output keeps its imaginary part.
    let alone = certify_partial_fraction(&j, 1.0, -1.0, 0.0, &terms[..1], &b.0, &b.1, TOL).unwrap();
    assert!(alone.output_im()[0] > 0.29);
    // A wrong term candidate is rejected; invalid inputs are errors.
    let wrong = [1.0, -0.4];
    let mut bad = terms;
    bad[1].candidate_re = &wrong;
    let rejected = certify_partial_fraction(&j, 1.0, -1.0, 0.0, &bad, &b.0, &b.1, TOL).unwrap();
    assert_eq!(rejected.status(), CertificateStatus::Rejected);
    assert!(rejected.error_upper() >= 0.0316);
    assert!(certify_partial_fraction(&j, 1.0, -1.0, 0.0, &[], &b.0, &b.1, TOL).is_err());
    assert!(certify_partial_fraction(&j, 1.0, f64::NAN, 0.0, &terms, &b.0, &b.1, TOL).is_err());
    let mut nan_weight = terms;
    nan_weight[0].weight_im = f64::INFINITY;
    assert!(certify_partial_fraction(&j, 1.0, -1.0, 0.0, &nan_weight, &b.0, &b.1, TOL).is_err());
    let mut left = terms;
    left[0].gamma_re = -0.25;
    assert!(certify_partial_fraction(&j, 1.0, -1.0, 0.0, &left, &b.0, &b.1, TOL).is_err());
    let nondissipative = DenseMatrix::from_rows(&[&[-1.0, 100.0], &[0.0, -1.0]]).unwrap();
    assert!(
        certify_partial_fraction(&nondissipative, 1.0, -1.0, 0.0, &terms, &b.0, &b.1, TOL).is_err()
    );
}
