//! Joint Chebyshev and Laguerre phi actions (re-audit R3 of 2026-10-01,
//! POLY-01 and POLY-02), against the independent oracle fixtures exported by
//! `tools/r3_export_polynomial_oracle_fixtures.py`.

use rodas5p_core::{
    DenseMatrix, WorkCounters,
    polynomial_action::{
        CoefficientCache, EnclosureEvidence, JointPhiInput, JointPhiReport,
        POLYNOMIAL_DOMAIN_UNSUPPORTED, PolynomialBasis, SymmetricNonpositiveOperator,
        TOTAL_ERROR_NOT_CERTIFIED, TotalErrorStatus, chebyshev_coefficient_enclosures,
        joint_phi_action, joint_phi_action_or_dense, laguerre_coefficient_enclosures,
    },
};
use serde_json::Value;

const FIXTURES: &str = include_str!("../../../fixtures/r3_polynomial_oracle_fixtures.json");
const BUDGET: f64 = 1.0e-12;

fn fixtures() -> Value {
    serde_json::from_str(FIXTURES).unwrap()
}

fn hex(value: &Value) -> f64 {
    let text = value.as_str().unwrap();
    let (negative, body) = text
        .strip_prefix('-')
        .map_or((false, text), |rest| (true, rest));
    let body = body.strip_prefix("0x").unwrap();
    let (mantissa, exponent) = body.split_once('p').unwrap();
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let mut value = u64::from_str_radix(int, 16).unwrap() as f64;
    let mut scale = 1.0 / 16.0;
    for digit in frac.chars() {
        value += digit.to_digit(16).unwrap() as f64 * scale;
        scale /= 16.0;
    }
    let value = value * 2.0_f64.powi(exponent.parse::<i32>().unwrap());
    if negative { -value } else { value }
}

fn matrix(value: &Value) -> Vec<Vec<f64>> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row.as_array().unwrap().iter().map(hex).collect())
        .collect()
}

fn dense(rows: &[Vec<f64>]) -> DenseMatrix {
    let refs = rows.iter().map(Vec::as_slice).collect::<Vec<_>>();
    DenseMatrix::from_rows(&refs).unwrap()
}

fn column(rows: &[Vec<f64>], k: usize) -> Vec<f64> {
    rows.iter().map(|row| row[k]).collect()
}

fn distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

fn run(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    w: &[Vec<f64>; 5],
    basis: PolynomialBasis,
    budget: f64,
) -> (JointPhiReport, WorkCounters) {
    let mut work = WorkCounters::default();
    let report = joint_phi_action(
        op,
        h,
        JointPhiInput::Distinct(w),
        basis,
        budget,
        None,
        &mut work,
    )
    .unwrap();
    (report, work)
}

#[test]
fn all_joint_actions_match_the_independent_oracle() {
    let data = fixtures();
    let target = data["observed_target"].as_f64().unwrap();
    let mut actions = 0;
    let mut columns_checked = 0;
    let mut certified = 0;
    for row in data["actions"].as_array().unwrap() {
        let h = hex(&row["h"]);
        let (lambda, rho) = (hex(&row["lam"]), hex(&row["rho"]));
        let a = matrix(&row["A"]);
        let w_rows = matrix(&row["W"]);
        let reference = matrix(&row["ref_columns"]);
        let fused = row["ref_fused"]
            .as_array()
            .unwrap()
            .iter()
            .map(hex)
            .collect::<Vec<_>>();
        let w: [Vec<f64>; 5] = std::array::from_fn(|k| column(&w_rows, k));
        let op = SymmetricNonpositiveOperator::new(dense(&a), lambda, rho, "r3-fixture").unwrap();
        for basis in [PolynomialBasis::Chebyshev, PolynomialBasis::Laguerre] {
            let (report, work) = run(&op, h, &w, basis, BUDGET);
            let label = format!("{} h={h:e} {basis:?}", row["family"]);
            let error = distance(&report.fused, &fused);
            assert!(error <= target, "{label}: fused error {error:e}");
            actions += 1;
            for k in 0..5 {
                let column_error = distance(&report.columns[k], &column(&reference, k));
                assert!(
                    column_error <= target,
                    "{label} column {k}: {column_error:e}"
                );
                columns_checked += 1;
            }
            assert!(
                report.truncation_bound_exact_arithmetic <= BUDGET,
                "{label}"
            );
            // The probe chose its degree with a floating-point bound; the
            // rigorous bound may need one more term.
            let probe = match basis {
                PolynomialBasis::Chebyshev => row["probe_chebyshev_degree"].as_u64().unwrap(),
                PolynomialBasis::Laguerre => row["probe_laguerre_degree"].as_u64().unwrap(),
            } as usize;
            assert!(
                report.degree.abs_diff(probe) <= 1,
                "{label}: degree {} vs probe {probe}",
                report.degree
            );
            assert_eq!(work.poly_block_products, report.degree as u64);
            assert_eq!(work.poly_vector_products, 5 * report.degree as u64);
            assert_eq!(work.poly_coefficient_setups, 1);
            // Any bound the report gives must enclose the observed error
            // (the reference itself is rounded to binary64, half an ULP per
            // component).
            let reference_rounding = fused.iter().map(|x| x.abs()).fold(0.0, f64::max)
                * f64::EPSILON
                * (fused.len() as f64).sqrt();
            match &report.total_error {
                TotalErrorStatus::Certified { bound } => {
                    assert_eq!(basis, PolynomialBasis::Chebyshev);
                    assert_eq!(report.evidence, EnclosureEvidence::Gershgorin);
                    assert!(
                        error <= bound + reference_rounding,
                        "{label}: {error:e} > {bound:e}"
                    );
                    certified += 1;
                }
                TotalErrorStatus::EstimateOnly {
                    reason,
                    bounded_components,
                } => {
                    assert!(reason.starts_with(TOTAL_ERROR_NOT_CERTIFIED), "{reason}");
                    if basis == PolynomialBasis::Chebyshev {
                        // Only the evidence is missing: the enclosure is true.
                        assert!(matches!(
                            report.evidence,
                            EnclosureEvidence::Declared { .. }
                        ));
                        assert!(error <= bounded_components + reference_rounding, "{label}");
                    } else {
                        assert!(report.column_errors.iter().all(|c| c.recurrence.is_none()));
                    }
                }
            }
        }
    }
    assert_eq!((actions, columns_checked), (20, 100));
    // The diagonal family has verified (Gershgorin) enclosures.
    assert_eq!(certified, 4);
}

fn parse(value: &Value) -> f64 {
    value.as_str().unwrap().parse().unwrap()
}

/// `[lo, hi]` contains the real `x` rounded to `value` (so `x` lies within
/// one step of `value`).
fn encloses(lo: f64, hi: f64, value: f64) -> bool {
    lo <= value.next_up() && value.next_down() <= hi
}

#[test]
fn coefficient_enclosures_contain_independent_quadrature() {
    let data = fixtures();
    for row in data["chebyshev_coefficients"].as_array().unwrap() {
        let (h, lambda, rho) = (parse(&row["h"]), parse(&row["lam"]), parse(&row["rho"]));
        let n = row["n"].as_u64().unwrap() as usize;
        let table = chebyshev_coefficient_enclosures(h, lambda, rho, n).unwrap();
        for (k, expected) in row["c"].as_array().unwrap().iter().enumerate() {
            let value = parse(expected);
            let enclosure = table[n][k];
            assert!(
                encloses(enclosure.lo, enclosure.hi, value),
                "Chebyshev h={h} n={n} k={k}: {enclosure:?} vs {value:e}"
            );
            assert!(
                enclosure.hi - enclosure.lo <= 1.0e-12 * value.abs(),
                "{enclosure:?}"
            );
        }
    }
    for row in data["laguerre_coefficients"].as_array().unwrap() {
        let (h, rho, scale) = (parse(&row["h"]), parse(&row["rho"]), parse(&row["scale"]));
        let n = row["n"].as_u64().unwrap() as usize;
        let table = laguerre_coefficient_enclosures(h, rho / scale, n).unwrap();
        for (k, expected) in row["c"].as_array().unwrap().iter().enumerate() {
            let value = parse(expected);
            let enclosure = table[n][k];
            assert!(
                encloses(enclosure.lo, enclosure.hi, value),
                "Laguerre h={h} n={n} k={k}: {enclosure:?} vs {value:e}"
            );
            assert!(
                enclosure.hi - enclosure.lo <= 1.0e-12 * value.abs(),
                "{enclosure:?}"
            );
        }
    }
}

fn simple_w(n: usize) -> [Vec<f64>; 5] {
    std::array::from_fn(|k| (0..n).map(|i| (i * 5 + k) as f64 / 20.0).collect())
}

#[test]
fn zero_step_zero_operator_and_scalar_matrix_use_the_scalar_branch() {
    let w = simple_w(4);
    let factorials = [1.0, 1.0, 2.0, 6.0, 24.0];
    let minus_identity = dense(
        &(0..4)
            .map(|i| (0..4).map(|j| if i == j { -1.0 } else { 0.0 }).collect())
            .collect::<Vec<_>>(),
    );
    let zero = dense(&vec![vec![0.0; 4]; 4]);
    let e = (-1.0_f64).exp();
    let phi_minus_one = [e, 1.0 - e, e, 0.5 - e, e - 1.0 / 3.0];
    for basis in [PolynomialBasis::Chebyshev, PolynomialBasis::Laguerre] {
        for (op, h, expected) in [
            (
                SymmetricNonpositiveOperator::new(minus_identity.clone(), 1.0, 1.0, "test")
                    .unwrap(),
                0.0,
                factorials.map(|f| 1.0 / f),
            ),
            (
                SymmetricNonpositiveOperator::new(zero.clone(), 0.0, 0.0, "test").unwrap(),
                1.0,
                factorials.map(|f| 1.0 / f),
            ),
            (
                SymmetricNonpositiveOperator::new(minus_identity.clone(), 1.0, 1.0, "test")
                    .unwrap(),
                1.0,
                phi_minus_one,
            ),
        ] {
            let (report, work) = run(&op, h, &w, basis, BUDGET);
            assert_eq!(report.branch, "scalar");
            assert_eq!(report.degree, 0);
            assert_eq!(work.poly_block_products, 0);
            for k in 0..5 {
                for (value, input) in report.columns[k].iter().zip(&w[k]) {
                    assert!((value - expected[k] * input).abs() <= 4.0e-16 * input.abs().max(1.0));
                }
            }
            let TotalErrorStatus::Certified { bound } = report.total_error else {
                panic!("scalar branch must be certified: {:?}", report.total_error)
            };
            assert!(bound <= 1.0e-14);
        }
    }
}

#[test]
fn unsupported_operators_are_explicit_failures() {
    let jordan = dense(&[vec![-1.0, 100.0], vec![0.0, -1.0]]);
    for result in [
        SymmetricNonpositiveOperator::new(jordan.clone(), 1.0, 1.0, "test").map(|_| ()),
        SymmetricNonpositiveOperator::gershgorin(jordan.clone()).map(|_| ()),
    ] {
        let message = result.unwrap_err().to_string();
        assert!(message.contains(POLYNOMIAL_DOMAIN_UNSUPPORTED), "{message}");
    }
    // A diagonal entry outside the enclosure refutes it.
    let diagonal = dense(&[vec![-3.0, 0.0], vec![0.0, -1.0]]);
    assert!(SymmetricNonpositiveOperator::new(diagonal.clone(), 0.5, 2.0, "wrong").is_err());
    // A degenerate enclosure needs a scalar matrix.
    let off = dense(&[vec![-1.0, 0.5], vec![0.5, -1.0]]);
    let op = SymmetricNonpositiveOperator::new(off.clone(), 1.0, 1.0, "wrong").unwrap();
    let mut work = WorkCounters::default();
    let w = simple_w(2);
    assert!(
        joint_phi_action(
            &op,
            1.0,
            JointPhiInput::Distinct(&w),
            PolynomialBasis::Chebyshev,
            BUDGET,
            None,
            &mut work
        )
        .is_err()
    );
    // No spectral witness: Gershgorin cannot show A <= 0.
    let indefinite = dense(&[vec![-1.0, 2.0], vec![2.0, -1.0]]);
    assert!(SymmetricNonpositiveOperator::gershgorin(indefinite).is_err());
    // Coefficient range: |a| = h (rho + lambda) / 2 = 1000 > 600.
    let stiff = dense(&[vec![-2000.0, 0.0], vec![0.0, -1.0]]);
    let op = SymmetricNonpositiveOperator::gershgorin(stiff).unwrap();
    let error = joint_phi_action(
        &op,
        1.0,
        JointPhiInput::Distinct(&w),
        PolynomialBasis::Chebyshev,
        BUDGET,
        None,
        &mut work,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("coefficient range"), "{error}");
    // Nonpositive h and budgets are rejected.
    let op = SymmetricNonpositiveOperator::gershgorin(diagonal).unwrap();
    for (h, budget) in [(-1.0, BUDGET), (f64::NAN, BUDGET), (1.0, 0.0)] {
        assert!(
            joint_phi_action(
                &op,
                h,
                JointPhiInput::Distinct(&w),
                PolynomialBasis::Chebyshev,
                budget,
                None,
                &mut work
            )
            .is_err()
        );
    }
    // The dense fallback is counted and carries no bound.
    let mut work = WorkCounters::default();
    let (value, report) = joint_phi_action_or_dense(
        &jordan,
        1.0,
        1.0,
        0.5,
        &w,
        PolynomialBasis::Chebyshev,
        BUDGET,
        &mut work,
    )
    .unwrap();
    assert!(report.is_none());
    assert_eq!(work.poly_fallbacks, 1);
    assert!(value.iter().all(|x| x.is_finite()));
}

#[test]
fn coefficient_tables_are_reused_only_for_the_same_key() {
    let rows = (0..6)
        .map(|i| {
            (0..6)
                .map(|j| if i == j { -(i as f64 + 1.0) } else { 0.0 })
                .collect()
        })
        .collect::<Vec<Vec<f64>>>();
    let op = SymmetricNonpositiveOperator::gershgorin(dense(&rows)).unwrap();
    let w = simple_w(6);
    let mut cache = CoefficientCache::default();
    let mut work = WorkCounters::default();
    let call = |op: &SymmetricNonpositiveOperator,
                h: f64,
                w: &[Vec<f64>; 5],
                cache: &mut CoefficientCache,
                work: &mut WorkCounters| {
        joint_phi_action(
            op,
            h,
            JointPhiInput::Distinct(w),
            PolynomialBasis::Chebyshev,
            BUDGET,
            Some(cache),
            work,
        )
        .unwrap()
    };
    let first = call(&op, 0.5, &w, &mut cache, &mut work);
    let second = call(&op, 0.5, &w, &mut cache, &mut work);
    assert!(!first.coefficient_cache_hit && second.coefficient_cache_hit);
    assert_eq!(first.fused, second.fused);
    assert_eq!(
        (work.poly_coefficient_setups, work.poly_coefficient_reuses),
        (1, 1)
    );
    // One ULP of h is a different key.
    call(&op, 0.5_f64.next_up(), &w, &mut cache, &mut work);
    assert_eq!(work.poly_coefficient_setups, 2);
    // A different enclosure of the same matrix is a different operator.
    let wider = SymmetricNonpositiveOperator::new(dense(&rows), 0.5, 7.0, "wider").unwrap();
    assert_ne!(wider.fingerprint(), op.fingerprint());
    call(&wider, 0.5, &w, &mut cache, &mut work);
    assert_eq!(work.poly_coefficient_setups, 3);
    // A changed matrix entry too.
    let mut changed = rows.clone();
    changed[0][0] = -1.0_f64.next_up();
    let changed =
        SymmetricNonpositiveOperator::new(dense(&changed), 1.0_f64.next_down(), 6.0, "changed")
            .unwrap();
    call(&changed, 0.5, &w, &mut cache, &mut work);
    assert_eq!(work.poly_coefficient_setups, 4);
    // The bound is recomputed for new weights, not reused with the table.
    let small: [Vec<f64>; 5] = std::array::from_fn(|k| w[k].iter().map(|x| x * 1.0e-3).collect());
    let scaled = call(&op, 0.5, &small, &mut cache, &mut work);
    assert!(scaled.truncation_bound_exact_arithmetic < first.truncation_bound_exact_arithmetic);
}

#[test]
fn the_same_vector_path_reuses_one_recurrence() {
    let rows = (0..5_usize)
        .map(|i| {
            (0..5)
                .map(|j| {
                    if i == j {
                        -2.0 * (i as f64 + 1.0)
                    } else if i.abs_diff(j) == 1 {
                        0.25
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect::<Vec<Vec<f64>>>();
    let op = SymmetricNonpositiveOperator::gershgorin(dense(&rows)).unwrap();
    let v = (0..5).map(|i| 1.0 + i as f64 * 0.3).collect::<Vec<_>>();
    let scales = [1.0, -0.5, 0.25, 2.0, -1.0];
    let w: [Vec<f64>; 5] = std::array::from_fn(|k| v.iter().map(|x| scales[k] * x).collect());
    let (distinct, distinct_work) = run(&op, 0.7, &w, PolynomialBasis::Chebyshev, BUDGET);
    let mut same_work = WorkCounters::default();
    let same = joint_phi_action(
        &op,
        0.7,
        JointPhiInput::SameVector { vector: &v, scales },
        PolynomialBasis::Chebyshev,
        BUDGET,
        None,
        &mut same_work,
    )
    .unwrap();
    assert_eq!(same.degree, distinct.degree);
    assert!(distance(&same.fused, &distinct.fused) <= 1.0e-14);
    assert_eq!(
        same_work.poly_block_products,
        distinct_work.poly_block_products
    );
    assert_eq!(
        same_work.poly_vector_products * 5,
        distinct_work.poly_vector_products
    );
    assert!(matches!(
        same.total_error,
        TotalErrorStatus::Certified { .. }
    ));
}

#[test]
fn the_degree_near_the_budget_never_understates_the_bound() {
    let rows = (0..8)
        .map(|i| {
            (0..8)
                .map(|j| if i == j { -(1.5_f64).powi(i) } else { 0.0 })
                .collect()
        })
        .collect::<Vec<Vec<f64>>>();
    let op = SymmetricNonpositiveOperator::gershgorin(dense(&rows)).unwrap();
    let w = simple_w(8);
    for basis in [PolynomialBasis::Chebyshev, PolynomialBasis::Laguerre] {
        for h in [0.05, 0.4, 3.0] {
            let (report, _) = run(&op, h, &w, basis, 1.0e-10);
            let bound = report.truncation_bound_exact_arithmetic;
            assert!(bound <= 1.0e-10);
            // The bound itself as the budget keeps the degree.
            let (at, _) = run(&op, h, &w, basis, bound);
            assert_eq!(at.degree, report.degree);
            assert!(at.truncation_bound_exact_arithmetic <= bound);
            // One ULP below needs more terms, and the bound still holds.
            let (below, _) = run(&op, h, &w, basis, bound.next_down());
            assert!(below.degree > report.degree, "{basis:?} h={h}");
            assert!(below.truncation_bound_exact_arithmetic <= bound.next_down());
        }
    }
}

#[test]
fn the_unbounded_mode_returns_the_same_values_without_a_bound() {
    let rows = (0..6)
        .map(|i| {
            (0..6)
                .map(|j| if i == j { -(i as f64 + 1.0) * 3.0 } else { 0.0 })
                .collect()
        })
        .collect::<Vec<Vec<f64>>>();
    let op = SymmetricNonpositiveOperator::gershgorin(dense(&rows)).unwrap();
    let w = simple_w(6);
    for basis in [PolynomialBasis::Chebyshev, PolynomialBasis::Laguerre] {
        let (bounded, _) = run(&op, 0.8, &w, basis, BUDGET);
        let mut work = WorkCounters::default();
        let plain = rodas5p_core::polynomial_action::joint_phi_action_unbounded(
            &op,
            0.8,
            JointPhiInput::Distinct(&w),
            basis,
            BUDGET,
            None,
            &mut work,
        )
        .unwrap();
        assert_eq!(plain.degree, bounded.degree);
        assert_eq!(plain.fused, bounded.fused, "{basis:?}");
        assert_eq!(plain.columns, bounded.columns, "{basis:?}");
        let TotalErrorStatus::EstimateOnly { reason, .. } = &plain.total_error else {
            panic!("an unbounded run is never certified")
        };
        assert!(reason.contains("not computed"));
        assert_eq!(work.poly_block_products, plain.degree as u64);
    }
}
