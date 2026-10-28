//! Contracts of the opt-in Newton-Leja phi action (RVJ DAG node PP10,
//! `research/pp10_leja_candidate_20261004`): closed forms, the status that
//! is always `EstimateOnly`, the degree cap, the work counters and the
//! fact that no certified API admits a Leja result.
//!
//! Type-level contract: `LejaReport` is not a `JointPhiReport`, so
//! `admit_total_error`, `admit_laguerre_total`, `route_admission` and
//! `route_joint_phi` cannot take it (passing one does not compile). The
//! test `no_certified_api_admits_a_leja_result` shows that even a
//! `JointPhiReport` carrying a Leja result and its status is rejected.

use rodas5p_core::{
    CoreError, DenseMatrix, WorkCounters, dense_phi_combination,
    leja_action::{
        LEJA_DEGREE_CAP, LEJA_INPUT_UNSUPPORTED, LEJA_TOTAL_NOT_CERTIFIED, LejaReport,
        leja_phi_action, leja_phi_action_capped,
    },
    polynomial_action::{
        JointPhiInput, PolynomialBasis, SymmetricNonpositiveOperator, TotalErrorAdmission,
        TotalErrorStatus, joint_phi_action, route_admission,
    },
};

const SCALES: [f64; 5] = [1.0, -0.5, 0.25, 2.0, -1.0];

fn splitmix(seed: u64, n: usize) -> Vec<f64> {
    let mut state = seed;
    (0..n)
        .map(|_| {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^= z >> 31;
            2.0 * ((z >> 11) as f64 / (1u64 << 53) as f64) - 1.0
        })
        .collect()
}

fn vectors(n: usize, seed: u64) -> [Vec<f64>; 5] {
    std::array::from_fn(|k| splitmix(seed + 31 * k as u64, n))
}

/// `phi_k(z)` for real `z <= 0`: Taylor for `|z| < 1`, the recursion
/// `phi_k = (phi_{k-1} - 1/(k-1)!) / z` otherwise.
fn phi(k: usize, z: f64) -> f64 {
    if z.abs() < 1.0 {
        let mut factorial = 1.0;
        for i in 1..=k {
            factorial *= i as f64;
        }
        let (mut total, mut term) = (0.0_f64, 1.0 / factorial);
        let mut j = 0;
        while term.abs() > 1e-20 * total.abs().max(1e-300) {
            total += term;
            j += 1;
            term *= z / (j + k) as f64;
        }
        return total;
    }
    let mut value = z.exp();
    let mut factorial = 1.0;
    for m in 1..=k {
        value = (value - 1.0 / factorial) / z;
        factorial *= m as f64;
    }
    value
}

fn diagonal(values: &[f64]) -> DenseMatrix {
    let n = values.len();
    let mut a = DenseMatrix::zeros(n, n);
    for (i, v) in values.iter().enumerate() {
        a[(i, i)] = *v;
    }
    a
}

/// `sum_k phi_k(h d_i) w_k[i]` for a diagonal `A = diag(d)`.
fn diagonal_target(d: &[f64], h: f64, w: &[Vec<f64>]) -> Vec<f64> {
    (0..d.len())
        .map(|i| (0..5).map(|k| phi(k, h * d[i]) * w[k][i]).sum())
        .collect()
}

fn distance(x: &[f64], y: &[f64]) -> f64 {
    x.iter()
        .zip(y)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
}

fn assert_estimate_only(report: &LejaReport) {
    match report.total_error() {
        TotalErrorStatus::EstimateOnly {
            reason,
            bounded_components,
        } => {
            assert!(reason.starts_with(LEJA_TOTAL_NOT_CERTIFIED), "{reason}");
            assert_eq!(*bounded_components, 0.0);
        }
        other => panic!("a Leja report is never certified: {other:?}"),
    }
}

/// Symmetric, diagonally dominant, Gershgorin discs inside `[-1.2 rho, 0)`
/// (the PP08 contract-test construction, not the study fixtures).
fn contract_matrix(n: usize, rho: f64, seed: u64) -> DenseMatrix {
    let d: Vec<f64> = splitmix(seed, n)
        .iter()
        .map(|u| rho * (0.65 + 0.35 * u))
        .collect();
    let raw = splitmix(seed + 1000, n * n);
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = -d[i];
        for j in 0..i {
            let v = 0.2 * d[i].min(d[j]) / n as f64 * raw[i * n + j];
            a[(i, j)] = v;
            a[(j, i)] = v;
        }
    }
    a
}

#[test]
fn scalar_operator_matches_the_closed_form() {
    let lambda = 3.0;
    let op = SymmetricNonpositiveOperator::gershgorin(diagonal(&[-lambda; 4])).unwrap();
    let w = vectors(4, 11);
    for h in [0.2, 1.0e-3, 5.0, 0.0] {
        let mut work = WorkCounters::default();
        let report =
            leja_phi_action(&op, h, JointPhiInput::Distinct(&w), 1.0e-12, &mut work).unwrap();
        assert_eq!(report.branch, "scalar");
        assert_eq!(report.degree, 0);
        assert_eq!(report.vector_products, 0);
        assert_estimate_only(&report);
        for (k, wk) in w.iter().enumerate() {
            let expected: Vec<f64> = wk.iter().map(|x| phi(k, -h * lambda) * x).collect();
            let error = distance(&report.columns[k], &expected);
            assert!(
                error <= 1e-14 * (1.0 + distance(&expected, &[0.0; 4])),
                "h {h} k {k}: {error:e}"
            );
        }
        let target = diagonal_target(&[-lambda; 4], h, &w);
        assert!(distance(&report.fused, &target) <= 1e-14, "h {h}");
        // Same vector with scales.
        let v = splitmix(12, 4);
        let report = leja_phi_action(
            &op,
            h,
            JointPhiInput::SameVector {
                vector: &v,
                scales: SCALES,
            },
            1.0e-12,
            &mut work,
        )
        .unwrap();
        let same: Vec<Vec<f64>> = SCALES
            .iter()
            .map(|s| v.iter().map(|x| s * x).collect())
            .collect();
        let target = diagonal_target(&[-lambda; 4], h, &same);
        assert!(distance(&report.fused, &target) <= 1e-14, "same h {h}");
        assert_estimate_only(&report);
    }
}

#[test]
fn diagonal_operators_meet_the_tolerance_against_closed_forms() {
    // Hand cases (not the study fixtures): spectra with h rho from 0.05 to 40.
    for (d, h) in [
        (vec![-1.0, -0.5, -0.1, -0.7], 0.05),
        (vec![-1.0, -5.0, -20.0, -50.0, -3.0], 0.1),
        (vec![-4.0, -40.0, -13.0, -27.5, -39.0, -4.5], 1.0),
        (vec![-1.0, -100.0, -60.0], 0.4),
    ] {
        let n = d.len();
        let op = SymmetricNonpositiveOperator::gershgorin(diagonal(&d)).unwrap();
        let w = vectors(n, 21);
        let target = diagonal_target(&d, h, &w);
        for tolerance in [1.0e-6, 1.0e-10, 1.0e-13] {
            let mut work = WorkCounters::default();
            let report =
                leja_phi_action(&op, h, JointPhiInput::Distinct(&w), tolerance, &mut work).unwrap();
            assert_eq!(report.branch, "newton");
            assert!(report.converged, "{d:?} h {h} tol {tolerance:e}");
            assert!(report.estimate <= tolerance);
            assert_estimate_only(&report);
            let error = distance(&report.fused, &target);
            assert!(
                error <= 10.0 * tolerance,
                "{d:?} h {h} tol {tolerance:e}: error {error:e}, degree {}",
                report.degree
            );
        }
    }
}

#[test]
fn symmetric_operator_agrees_with_the_dense_reference() {
    let a = contract_matrix(8, 30.0, 5);
    let op = SymmetricNonpositiveOperator::gershgorin(a.clone()).unwrap();
    let w = vectors(8, 41);
    for h in [1.0e-2, 0.3] {
        let reference = dense_phi_combination(&a, h, &w).unwrap();
        let mut work = WorkCounters::default();
        let report =
            leja_phi_action(&op, h, JointPhiInput::Distinct(&w), 1.0e-11, &mut work).unwrap();
        assert!(report.converged);
        let error = distance(&report.fused, &reference);
        assert!(error <= 1.0e-10, "h {h}: {error:e}");
        assert_estimate_only(&report);
    }
}

#[test]
fn degree_cap_stops_without_convergence() {
    let a = contract_matrix(6, 200.0, 7);
    let op = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let w = vectors(6, 51);
    let v = splitmix(52, 6);
    for (input, width) in [
        (JointPhiInput::Distinct(&w), 5_u64),
        (
            JointPhiInput::SameVector {
                vector: &v,
                scales: SCALES,
            },
            1,
        ),
    ] {
        let mut work = WorkCounters::default();
        let report = leja_phi_action(&op, 0.2, input, 1.0e-300, &mut work).unwrap();
        assert!(!report.converged);
        assert_eq!(report.degree, LEJA_DEGREE_CAP);
        assert_eq!(report.degree_cap, LEJA_DEGREE_CAP);
        assert_eq!(report.vector_products, width * LEJA_DEGREE_CAP as u64);
        assert!(report.estimate > 1.0e-300);
        assert_estimate_only(&report);
        let mut work = WorkCounters::default();
        let report = leja_phi_action_capped(&op, 0.2, input, 1.0e-12, 3, &mut work).unwrap();
        assert!(!report.converged);
        assert_eq!(report.degree, 3);
        assert_eq!(work.poly_vector_products, 3 * width);
        assert_estimate_only(&report);
    }
}

#[test]
fn counters_charge_every_operator_product_and_nothing_else() {
    let a = contract_matrix(7, 30.0, 9);
    let op = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let w = vectors(7, 61);
    let v = splitmix(62, 7);
    for (input, width) in [
        (JointPhiInput::Distinct(&w), 5_u64),
        (
            JointPhiInput::SameVector {
                vector: &v,
                scales: SCALES,
            },
            1,
        ),
    ] {
        let mut work = WorkCounters {
            jvp_calls: 4,
            poly_vector_products: 100,
            ..WorkCounters::default()
        };
        let before = work;
        let report = leja_phi_action(&op, 0.03, input, 1.0e-10, &mut work).unwrap();
        assert!(report.degree >= 1);
        assert_eq!(report.block_products, report.degree as u64);
        assert_eq!(report.vector_products, width * report.degree as u64);
        assert_eq!(report.coefficient_setups, 1);
        let expected = WorkCounters {
            poly_block_products: before.poly_block_products + report.block_products,
            poly_vector_products: before.poly_vector_products + report.vector_products,
            poly_coefficient_setups: before.poly_coefficient_setups + 1,
            ..before
        };
        assert_eq!(work, expected);
        // Chebyshev at the same tolerance charges the same counter kinds.
        let mut cheb = WorkCounters::default();
        let r = joint_phi_action(
            &op,
            0.03,
            input,
            PolynomialBasis::Chebyshev,
            1.0e-10,
            None,
            &mut cheb,
        )
        .unwrap();
        assert_eq!(cheb.poly_vector_products, width * r.degree as u64);
    }
}

#[test]
fn zero_input_and_zero_step_charge_no_products() {
    let a = contract_matrix(5, 30.0, 13);
    let op = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let zero: [Vec<f64>; 5] = std::array::from_fn(|_| vec![0.0; 5]);
    let mut work = WorkCounters::default();
    let report =
        leja_phi_action(&op, 0.1, JointPhiInput::Distinct(&zero), 1.0e-8, &mut work).unwrap();
    assert_eq!(report.branch, "zero-input");
    assert!(report.fused.iter().all(|x| *x == 0.0));
    assert_eq!(work, WorkCounters::default());
    assert_estimate_only(&report);
    let w = vectors(5, 71);
    let report = leja_phi_action(&op, 0.0, JointPhiInput::Distinct(&w), 1.0e-8, &mut work).unwrap();
    assert_eq!(report.branch, "scalar");
    assert_eq!(work.poly_vector_products, 0);
    let expected: Vec<f64> = (0..5)
        .map(|i| w[0][i] + w[1][i] + w[2][i] / 2.0 + w[3][i] / 6.0 + w[4][i] / 24.0)
        .collect();
    assert!(distance(&report.fused, &expected) <= 1e-15);
    assert_estimate_only(&report);
}

#[test]
fn invalid_inputs_are_errors_without_work() {
    let a = contract_matrix(5, 30.0, 17);
    let op = SymmetricNonpositiveOperator::gershgorin(a.clone()).unwrap();
    let diagonal_max = (0..5).map(|i| a[(i, i)].abs()).fold(0.0, f64::max);
    let declared =
        SymmetricNonpositiveOperator::new(a, 0.0, diagonal_max, "pp10-declared").unwrap();
    let w = vectors(5, 81);
    let short: [Vec<f64>; 5] = std::array::from_fn(|_| vec![1.0; 4]);
    let mut nan = w.clone();
    nan[2][1] = f64::NAN;
    let mut work = WorkCounters::default();
    let cases: Vec<(&str, CoreError)> = vec![
        (
            "tolerance nan",
            leja_phi_action(&op, 0.1, JointPhiInput::Distinct(&w), f64::NAN, &mut work)
                .unwrap_err(),
        ),
        (
            "tolerance 0",
            leja_phi_action(&op, 0.1, JointPhiInput::Distinct(&w), 0.0, &mut work).unwrap_err(),
        ),
        (
            "tolerance -1",
            leja_phi_action(&op, 0.1, JointPhiInput::Distinct(&w), -1.0, &mut work).unwrap_err(),
        ),
        (
            "h -1",
            leja_phi_action(&op, -1.0, JointPhiInput::Distinct(&w), 1e-8, &mut work).unwrap_err(),
        ),
        (
            "h inf",
            leja_phi_action(
                &op,
                f64::INFINITY,
                JointPhiInput::Distinct(&w),
                1e-8,
                &mut work,
            )
            .unwrap_err(),
        ),
        (
            "declared",
            leja_phi_action(&declared, 0.1, JointPhiInput::Distinct(&w), 1e-8, &mut work)
                .unwrap_err(),
        ),
        (
            "dimension",
            leja_phi_action(&op, 0.1, JointPhiInput::Distinct(&short), 1e-8, &mut work)
                .unwrap_err(),
        ),
        (
            "cap 0",
            leja_phi_action_capped(&op, 0.1, JointPhiInput::Distinct(&w), 1e-8, 0, &mut work)
                .unwrap_err(),
        ),
        (
            "cap above",
            leja_phi_action_capped(
                &op,
                0.1,
                JointPhiInput::Distinct(&w),
                1e-8,
                LEJA_DEGREE_CAP + 1,
                &mut work,
            )
            .unwrap_err(),
        ),
        (
            "h rho above the range",
            leja_phi_action(&op, 100.0, JointPhiInput::Distinct(&w), 1e-8, &mut work).unwrap_err(),
        ),
    ];
    for (label, error) in &cases {
        let text = error.to_string();
        assert!(text.contains(LEJA_INPUT_UNSUPPORTED), "{label}: {text}");
    }
    let error =
        leja_phi_action(&op, 0.1, JointPhiInput::Distinct(&nan), 1e-8, &mut work).unwrap_err();
    assert!(matches!(error, CoreError::NonFinite(_)));
    assert_eq!(work, WorkCounters::default());
}

#[test]
fn no_certified_api_admits_a_leja_result() {
    let a = contract_matrix(6, 30.0, 19);
    let op = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let w = vectors(6, 91);
    let mut work = WorkCounters::default();
    let leja = leja_phi_action(&op, 0.05, JointPhiInput::Distinct(&w), 1.0e-10, &mut work).unwrap();
    assert_estimate_only(&leja);
    // The most admissible-looking carrier: a certified, verified Chebyshev
    // report of the same target with the Leja result and status put in.
    let mut carrier = joint_phi_action(
        &op,
        0.05,
        JointPhiInput::Distinct(&w),
        PolynomialBasis::Chebyshev,
        1.0e-10,
        None,
        &mut work,
    )
    .unwrap();
    assert!(matches!(
        carrier.admit_total_error(1.0e300),
        TotalErrorAdmission::Admitted { .. }
    ));
    carrier.fused = leja.fused.clone();
    carrier.columns = leja.columns.clone();
    carrier.degree = leja.degree;
    carrier.total_error = leja.total_error().clone();
    for (api, admission) in [
        ("admit_total_error", carrier.admit_total_error(1.0e300)),
        (
            "admit_laguerre_total",
            carrier.admit_laguerre_total(1.0e300),
        ),
        ("route_admission", route_admission(&carrier, 1.0e300)),
    ] {
        match admission {
            TotalErrorAdmission::Rejected { reason } => {
                if api != "admit_laguerre_total" {
                    assert!(reason.contains(LEJA_TOTAL_NOT_CERTIFIED), "{api}: {reason}");
                }
            }
            TotalErrorAdmission::Admitted { .. } => panic!("{api} admitted a Leja result"),
        }
    }
    // As a Laguerre carrier the status is still not admitted.
    carrier.basis = PolynomialBasis::Laguerre;
    assert!(matches!(
        carrier.admit_laguerre_total(1.0e300),
        TotalErrorAdmission::Rejected { .. }
    ));
    assert!(matches!(
        route_admission(&carrier, 1.0e300),
        TotalErrorAdmission::Rejected { .. }
    ));
}
