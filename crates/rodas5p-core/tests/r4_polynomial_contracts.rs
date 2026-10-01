//! Joint phi polynomial action contracts of re-audit R4 of 2026-10-01
//! (POLY-DEV-01..03), against the oracle exported by
//! `tools/r4_export_polynomial_range_fixtures.py`.

use rodas5p_core::{
    DenseMatrix, WorkCounters,
    directed::sub_up,
    polynomial_action::{
        EXECUTION_CERTIFIED, EXECUTION_UNBOUNDED_TIMING, JointPhiInput, NORMALIZATION_WINDOW,
        POLYNOMIAL_RANGE_UNSUPPORTED, PolynomialBasis, SymmetricNonpositiveOperator,
        TOTAL_ERROR_ABOVE_BUDGET, TOTAL_ERROR_NOT_CERTIFIED, TotalErrorAdmission, TotalErrorStatus,
        joint_phi_action, joint_phi_action_laguerre_scales, joint_phi_action_unbounded,
    },
    transform_bound::ExpBound,
};
use serde_json::Value;
use std::cmp::Ordering;

const FIXTURES: &str = include_str!("../../../fixtures/r4_polynomial_range_fixtures.json");

fn fixtures() -> Value {
    serde_json::from_str(FIXTURES).unwrap()
}

fn bits(value: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(value.as_str().unwrap(), 16).unwrap())
}

fn vector(value: &Value) -> Vec<f64> {
    value.as_array().unwrap().iter().map(bits).collect()
}

fn diagonal(eigenvalues: &[f64]) -> SymmetricNonpositiveOperator {
    let n = eigenvalues.len();
    let rows = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| if i == j { eigenvalues[i] } else { 0.0 })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let matrix =
        DenseMatrix::from_rows(&rows.iter().map(Vec::as_slice).collect::<Vec<_>>()).unwrap();
    SymmetricNonpositiveOperator::gershgorin(matrix).unwrap()
}

#[test]
fn scaled_norms_enclose_the_exact_norm_across_the_range() {
    for row in fixtures()["norms"].as_array().unwrap() {
        let values = vector(&row["vector"]);
        let exact = ExpBound {
            mantissa: row["norm_upper"]["mantissa"].as_f64().unwrap(),
            exponent: row["norm_upper"]["exponent"].as_i64().unwrap(),
        };
        let bound = ExpBound::l2_norm_upper(&values).unwrap();
        assert_ne!(bound.total_cmp(&exact), Ordering::Less, "{values:?}");
        if exact.is_zero() {
            assert!(bound.is_zero());
        } else {
            // Tight: within a relative 1e-14 of the exact norm.
            let ratio = bound.mantissa / exact.mantissa
                * 2.0_f64.powi((bound.exponent - exact.exponent) as i32);
            assert!(ratio < 1.0 + 1.0e-14, "{values:?}: {bound:?} vs {exact:?}");
        }
    }
}

#[test]
fn amplitude_scaled_actions_are_enclosed_or_rejected_never_zero() {
    let data = fixtures();
    for case in data["actions"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let op = diagonal(&vector(&case["eigenvalues"]));
        let h = bits(&case["h"]);
        let vectors: [Vec<f64>; 5] = case["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .map(vector)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let lo = vector(&case["exact_lo"]);
        let hi = vector(&case["exact_hi"]);
        let amplitude = vectors
            .iter()
            .flatten()
            .fold(0.0_f64, |a, x| a.max(x.abs()));
        let budget = (amplitude * 1.0e-12).max(f64::from_bits(1));
        let mut work = WorkCounters::default();
        let report = joint_phi_action(
            &op,
            h,
            JointPhiInput::Distinct(&vectors),
            PolynomialBasis::Chebyshev,
            budget,
            None,
            &mut work,
        )
        .unwrap_or_else(|error| panic!("{id}: {error}"));
        let TotalErrorStatus::Certified { bound } = report.total_error else {
            panic!("{id}: {:?}", report.total_error);
        };
        // Upper estimate of the true error from the outward reference.
        let distances = (0..2)
            .map(|i| {
                let fused = report.fused[i];
                sub_up(fused, lo[i])
                    .unwrap()
                    .max(sub_up(hi[i], fused).unwrap())
            })
            .collect::<Vec<_>>();
        let error = ExpBound::l2_norm_upper(&distances).unwrap().to_f64_up();
        assert!(
            bound >= error,
            "{id}: bound {bound:e} below error {error:e}"
        );
        assert!(bound > 0.0, "{id}: zero certificate");
        // Useful: the bound is a small fraction of the amplitude (or of the
        // subnormal rounding when the result itself is subnormal).
        assert!(
            bound <= (amplitude * 1.0e-9).max(1.0e-321),
            "{id}: bound {bound:e} for amplitude {amplitude:e}"
        );
        let shift = report.normalization_shift;
        assert_eq!(
            shift != 0,
            amplitude.log2().abs() > NORMALIZATION_WINDOW as f64,
            "{id}: shift {shift}"
        );
        if id == "mixed_lossy" {
            assert!(report.column_errors[0].normalization > 0.0);
        }
        // R4 POLY-DEV-03: the Laguerre majorant total encloses the same
        // error at every scale, and the status stays an estimate.
        for scale in [1.0, 2.0, 4.0, 8.0, 16.0] {
            let mut work = WorkCounters::default();
            let laguerre = joint_phi_action_laguerre_scales(
                &op,
                h,
                JointPhiInput::Distinct(&vectors),
                &[scale],
                budget,
                &mut work,
            )
            .unwrap_or_else(|error| panic!("{id} L={scale}: {error}"));
            assert!(matches!(
                laguerre.total_error,
                TotalErrorStatus::EstimateOnly { .. }
            ));
            assert!(
                laguerre
                    .column_errors
                    .iter()
                    .all(|c| c.recurrence.is_none())
            );
            let distances = (0..2)
                .map(|i| {
                    let fused = laguerre.fused[i];
                    sub_up(fused, lo[i])
                        .unwrap()
                        .max(sub_up(hi[i], fused).unwrap())
                })
                .collect::<Vec<_>>();
            let error = ExpBound::l2_norm_upper(&distances).unwrap().to_f64_up();
            let majorant = laguerre.laguerre_majorant_total.unwrap();
            assert!(
                majorant >= error,
                "{id} L={scale}: {majorant:e} < {error:e}"
            );
            assert!(majorant > 0.0);
        }
    }
}

#[test]
fn an_out_of_range_result_or_budget_is_a_typed_rejection() {
    // phi_0 of a zero operator with w at f64::MAX is in range; a budget that
    // underflows after scaling down by 2^1024 is not representable.
    let op = diagonal(&[-1.0, -2.0]);
    let big = [
        vec![f64::MAX, 0.0],
        vec![0.0; 2],
        vec![0.0; 2],
        vec![0.0; 2],
        vec![0.0; 2],
    ];
    let mut work = WorkCounters::default();
    let error = joint_phi_action(
        &op,
        0.1,
        JointPhiInput::Distinct(&big),
        PolynomialBasis::Chebyshev,
        f64::from_bits(1),
        None,
        &mut work,
    )
    .unwrap_err();
    assert!(
        error.to_string().contains(POLYNOMIAL_RANGE_UNSUPPORTED),
        "{error}"
    );
}

fn unit_vectors() -> [Vec<f64>; 5] {
    [
        vec![1.0, 1.0],
        vec![-1.0, 0.5],
        vec![0.25, 0.0],
        vec![0.0, -0.125],
        vec![0.0625, 0.0],
    ]
}

#[test]
fn truncation_success_is_not_total_error_admission() {
    // R4 POLY-DEV-02: the degree meets a 1e-30 truncation budget, but the
    // rounding bound (about 1e-16) does not; admission against 1e-30 fails.
    let op = diagonal(&[-1.0, -2.0]);
    let vectors = unit_vectors();
    let mut work = WorkCounters::default();
    let report = joint_phi_action(
        &op,
        0.1,
        JointPhiInput::Distinct(&vectors),
        PolynomialBasis::Chebyshev,
        1.0e-30,
        None,
        &mut work,
    )
    .unwrap();
    assert_eq!(report.execution, EXECUTION_CERTIFIED);
    assert!(report.truncation_bound_exact_arithmetic <= 1.0e-30);
    let TotalErrorStatus::Certified { bound } = report.total_error else {
        panic!("{:?}", report.total_error);
    };
    assert!(bound > 1.0e-30);
    match report.admit_total_error(1.0e-30) {
        TotalErrorAdmission::Rejected { reason } => {
            assert!(reason.starts_with(TOTAL_ERROR_ABOVE_BUDGET), "{reason}")
        }
        other => panic!("{other:?}"),
    }
    let TotalErrorAdmission::Admitted {
        bound: admitted,
        relative_bound,
        condition_proxy,
        ..
    } = report.admit_total_error(1.0e-12)
    else {
        panic!("not admitted at 1e-12");
    };
    assert_eq!(admitted, bound);
    assert!(relative_bound > 0.0 && relative_bound < 1.0e-12);
    assert!(condition_proxy >= 1.0);
    for budget in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(matches!(
            report.admit_total_error(budget),
            TotalErrorAdmission::Rejected { .. }
        ));
    }
}

#[test]
fn cancellation_reports_absolute_and_relative_accuracy_separately() {
    // phi_0(-0.1) - c phi_1(-0.1) with c chosen to cancel the first
    // component to about 1e-17: the absolute bound stays small, the relative
    // bound and the conditioning are large, and admission is absolute.
    let op = diagonal(&[-1.0, -2.0]);
    let z: f64 = -0.1;
    let c = z.exp() / ((z.exp() - 1.0) / z);
    let vectors = [
        vec![1.0, 0.0],
        vec![-c, 0.0],
        vec![0.0; 2],
        vec![0.0; 2],
        vec![0.0; 2],
    ];
    let mut work = WorkCounters::default();
    let report = joint_phi_action(
        &op,
        0.1,
        JointPhiInput::Distinct(&vectors),
        PolynomialBasis::Chebyshev,
        1.0e-14,
        None,
        &mut work,
    )
    .unwrap();
    assert!(report.condition_proxy > 1.0e6, "{}", report.condition_proxy);
    let TotalErrorAdmission::Admitted {
        bound,
        relative_bound,
        ..
    } = report.admit_total_error(1.0e-12)
    else {
        panic!("{:?}", report.total_error);
    };
    assert!(bound < 1.0e-12);
    assert!(relative_bound > bound);
}

#[test]
fn estimates_and_the_timing_path_never_admit() {
    let op = diagonal(&[-1.0, -2.0]);
    let vectors = unit_vectors();
    let mut work = WorkCounters::default();
    let laguerre = joint_phi_action(
        &op,
        0.1,
        JointPhiInput::Distinct(&vectors),
        PolynomialBasis::Laguerre,
        1.0e-12,
        None,
        &mut work,
    )
    .unwrap();
    assert!(matches!(
        laguerre.total_error,
        TotalErrorStatus::EstimateOnly { .. }
    ));
    assert!(matches!(
        laguerre.admit_total_error(1.0),
        TotalErrorAdmission::Rejected { .. }
    ));
    let timing = joint_phi_action_unbounded(
        &op,
        0.1,
        JointPhiInput::Distinct(&vectors),
        PolynomialBasis::Chebyshev,
        1.0e-12,
        None,
        &mut work,
    )
    .unwrap();
    assert_eq!(timing.execution, EXECUTION_UNBOUNDED_TIMING);
    match timing.admit_total_error(1.0) {
        TotalErrorAdmission::Rejected { reason } => {
            assert!(reason.starts_with(TOTAL_ERROR_NOT_CERTIFIED), "{reason}")
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn laguerre_scales_outside_the_cap_are_rejected() {
    let op = diagonal(&[-1.0, -2.0]);
    let vectors = unit_vectors();
    for scales in [&[][..], &[0.0][..], &[17.0][..], &[f64::NAN][..]] {
        let mut work = WorkCounters::default();
        assert!(
            joint_phi_action_laguerre_scales(
                &op,
                0.1,
                JointPhiInput::Distinct(&vectors),
                scales,
                1.0e-12,
                &mut work
            )
            .is_err(),
            "{scales:?}"
        );
    }
}

#[test]
fn the_continuous_laguerre_scale_minimizes_the_analytic_tail() {
    use rodas5p_core::polynomial_action::{LAGUERRE_SCALE_CAP, laguerre_scale_for_degree};
    // log T(beta) up to log W.
    let log_tail = |h: f64, rho: f64, r: f64, beta: f64| {
        rho / (2.0 * beta) + r * (h * beta / (1.0 + h * beta)).ln()
    };
    for (h, rho) in [(0.1, 2.0), (1.0, 3.0), (0.01, 50.0), (2.0, 1.0)] {
        for degree in [2_usize, 5, 10, 40] {
            let r = (degree + 1) as f64;
            match laguerre_scale_for_degree(h, rho, degree) {
                None => assert!(2.0 * r <= h * rho),
                Some(scale) => {
                    let exact = 2.0 * r - h * rho;
                    assert_eq!(scale, exact.min(LAGUERRE_SCALE_CAP));
                    if exact <= LAGUERRE_SCALE_CAP {
                        let beta = rho / scale;
                        let at = log_tail(h, rho, r, beta);
                        for factor in [0.9, 0.99, 1.01, 1.1] {
                            assert!(at <= log_tail(h, rho, r, beta * factor) + 1.0e-12);
                        }
                    }
                }
            }
        }
    }
    // Exact zero branches stay separate.
    assert_eq!(laguerre_scale_for_degree(0.0, 1.0, 3), None);
    assert_eq!(laguerre_scale_for_degree(1.0, 0.0, 3), None);
}
