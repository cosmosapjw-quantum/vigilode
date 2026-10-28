//! WU-6 (audit F-007, F-029): an attainable two-arm admissibility criterion
//! under a new protocol id.  The frozen rule `gap <= 0.1 * dense error`
//! compares two independently stepped O(tol) trajectories whose difference is
//! itself O(tol); the audit found SciPy Radau violating it in 17/18 pairs.
//! The frozen rule stays in place for the committed v2 records.

use std::path::PathBuf;

use rodas5p_core::WorkCounters;
use rodas5p_fair_ab::{
    ArmBudget, CommonOutputGrid, DualOutputPolicyEvidence, ExternalErrorScale, GlobalErrorMetrics,
    IntegratorWorkReport, OutputArmExceedance, OutputPolicyDominance, OutputPolicyRunEvidence,
    ReferenceWrmsBasis, TWO_ARM_ADMISSIBILITY_PROTOCOL_ID, TWO_ARM_GLOBAL_ERROR_BUDGET,
    TWO_ARM_INTERPOLANT_DELTA_LIMIT, TwoArmRowStatus, check_policy_gap_triangle,
    classify_arm_budget, classify_output_policy_dominance, classify_two_arm_row,
};
use serde_json::Value;

#[test]
fn protocol_id_declares_budget_and_interpolant_limit_before_any_run() {
    assert_eq!(TWO_ARM_GLOBAL_ERROR_BUDGET, 10.0);
    assert_eq!(TWO_ARM_INTERPOLANT_DELTA_LIMIT, 2.0);
    assert!(TWO_ARM_ADMISSIBILITY_PROTOCOL_ID.contains("budget=10"));
    assert!(TWO_ARM_ADMISSIBILITY_PROTOCOL_ID.contains("interpolant-delta<=2"));
    assert!(TWO_ARM_ADMISSIBILITY_PROTOCOL_ID.contains("basis=case-tolerance"));
    assert!(!TWO_ARM_ADMISSIBILITY_PROTOCOL_ID.contains("0.1"));
}

#[test]
fn arm_budget_has_three_bands_with_reference_uncertainty() {
    let b = 10.0;
    assert_eq!(
        classify_arm_budget(9.0, 1.0, b).unwrap(),
        ArmBudget::WithinBudget
    );
    assert_eq!(
        classify_arm_budget(3.0, 0.0, b).unwrap(),
        ArmBudget::WithinBudget
    );
    assert_eq!(
        classify_arm_budget(11.5, 1.0, b).unwrap(),
        ArmBudget::ExceedsBudget
    );
    assert_eq!(
        classify_arm_budget(11.0, 1.0, b).unwrap(),
        ArmBudget::ReferenceUndecidable,
        "E - U == B is not a strict excess"
    );
    assert_eq!(
        classify_arm_budget(9.5, 1.0, b).unwrap(),
        ArmBudget::ReferenceUndecidable
    );
    for (e, u, budget) in [
        (f64::NAN, 0.0, b),
        (1.0, -1.0, b),
        (-1.0, 0.0, b),
        (1.0, 0.0, 0.0),
        (1.0, f64::INFINITY, b),
    ] {
        assert!(
            classify_arm_budget(e, u, budget).is_err(),
            "{e} {u} {budget}"
        );
    }
}

#[test]
fn row_status_order_is_reference_then_budget_then_interpolant() {
    use ArmBudget::*;
    assert_eq!(
        classify_two_arm_row(ReferenceUndecidable, ExceedsBudget, Some(0.0)).unwrap(),
        TwoArmRowStatus::ReferenceDominated
    );
    assert_eq!(
        classify_two_arm_row(WithinBudget, ExceedsBudget, Some(99.0)).unwrap(),
        TwoArmRowStatus::GlobalErrorExceedsBudget(OutputArmExceedance {
            clipped: false,
            dense: true
        })
    );
    assert_eq!(
        classify_two_arm_row(ExceedsBudget, ExceedsBudget, None).unwrap(),
        TwoArmRowStatus::GlobalErrorExceedsBudget(OutputArmExceedance {
            clipped: true,
            dense: true
        })
    );
    assert_eq!(
        classify_two_arm_row(WithinBudget, WithinBudget, Some(2.5)).unwrap(),
        TwoArmRowStatus::InterpolantDominated
    );
    assert_eq!(
        classify_two_arm_row(WithinBudget, WithinBudget, Some(2.0)).unwrap(),
        TwoArmRowStatus::Pass
    );
    assert_eq!(
        classify_two_arm_row(WithinBudget, WithinBudget, None).unwrap(),
        TwoArmRowStatus::InterpolantUnchecked,
        "a row without the same-step interpolant check must not pass"
    );
    assert!(classify_two_arm_row(WithinBudget, WithinBudget, Some(f64::NAN)).is_err());
}

#[test]
fn gap_triangle_is_an_integrity_assertion() {
    assert!(check_policy_gap_triangle(2.0, 1.0, 1.0, 8).is_ok());
    assert!(check_policy_gap_triangle(0.0, 0.0, 0.0, 8).is_ok());
    assert!(check_policy_gap_triangle(2.0 + 1.0e-9, 1.0, 1.0, 8).is_err());
    assert!(check_policy_gap_triangle(f64::NAN, 1.0, 1.0, 8).is_err());
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
    }
}

fn arm(times: &[f64], states: Vec<Vec<f64>>) -> OutputPolicyRunEvidence {
    OutputPolicyRunEvidence {
        output_times: times.to_vec(),
        states,
        errors: GlobalErrorMetrics {
            endpoint_l2: 0.0,
            max_grid_l2: 0.0,
            rms_grid_l2: 0.0,
            endpoint_wrms: 0.0,
            max_grid_wrms: 0.0,
            rms_grid_wrms: 0.0,
            reference_uncertainty_wrms: 0.0,
            conservative_max_wrms: 0.0,
        },
        work: IntegratorWorkReport {
            counters: WorkCounters::default(),
            internal_steps: 1,
            output_clipped_steps: 0,
            stored_state_bytes: 0,
        },
    }
}

fn synthetic_evidence(seed: u64, dimension: usize, points: usize) -> DualOutputPolicyEvidence {
    let mut rng = Lcg(seed);
    let times = (0..points)
        .map(|i| i as f64 / (points - 1) as f64)
        .collect::<Vec<_>>();
    let reference = (0..points)
        .map(|_| {
            (0..dimension)
                .map(|_| 10.0 * rng.next())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let perturb = |rng: &mut Lcg, scale: f64| {
        reference
            .iter()
            .map(|state| state.iter().map(|v| v + scale * rng.next()).collect())
            .collect::<Vec<Vec<f64>>>()
    };
    let clipped = perturb(&mut rng, 1.0e-6);
    let dense = perturb(&mut rng, 3.0e-6);
    let basis = ReferenceWrmsBasis::new(
        CommonOutputGrid::new(times.clone()).unwrap(),
        reference,
        ExternalErrorScale::with_reference_uncertainty(vec![1.0e-10; dimension], 1.0e-8, 1.0e-3)
            .unwrap(),
    )
    .unwrap();
    DualOutputPolicyEvidence::new(basis, arm(&times, clipped), arm(&times, dense)).unwrap()
}

#[test]
fn two_arm_classification_holds_the_triangle_on_random_trajectories() {
    for seed in 1..=64 {
        let evidence = synthetic_evidence(seed, 7, 11);
        let row = evidence
            .classify_two_arm_v3(1.0e-6, 1.0e-4, Some(0.5))
            .unwrap();
        assert_eq!(row.protocol_id, TWO_ARM_ADMISSIBILITY_PROTOCOL_ID);
        assert!(row.gap_case_wrms <= row.clipped_case_wrms + row.dense_case_wrms);
        // The frozen rule is untouched and still reachable on the same evidence.
        assert!(matches!(
            evidence.classify().unwrap(),
            OutputPolicyDominance::Admissible | OutputPolicyDominance::Dominated
        ));
    }
}

#[test]
fn case_tolerance_basis_scales_exactly_when_weights_are_proportional() {
    let evidence = synthetic_evidence(7, 5, 9);
    let tight = &evidence.dense.errors;
    for rtol in [1.0e-4, 1.0e-6, 1.0e-8] {
        let row = evidence
            .classify_two_arm_v3(0.01 * rtol, rtol, Some(0.0))
            .unwrap();
        let expected = tight.max_grid_wrms * 1.0e-8 / rtol;
        assert!(
            (row.dense_case_wrms - expected).abs() <= 1.0e-12 * expected,
            "rtol {rtol}: {} vs {expected}",
            row.dense_case_wrms
        );
        let u_expected = tight.reference_uncertainty_wrms * 1.0e-8 / rtol;
        assert!((row.reference_uncertainty_case_wrms - u_expected).abs() <= 1.0e-12 * u_expected);
        assert!(row.basis_id.contains("case-tolerance"));
    }
    // A different atol/rtol ratio is not a scalar rescaling.
    let row = evidence
        .classify_two_arm_v3(1.0e-2, 1.0e-4, Some(0.0))
        .unwrap();
    let scalar = tight.max_grid_wrms * 1.0e-8 / 1.0e-4;
    assert!((row.dense_case_wrms - scalar).abs() > 1.0e-6 * scalar);
    assert!(evidence.classify_two_arm_v3(0.0, 1.0e-4, None).is_err());
    assert!(
        evidence
            .classify_two_arm_v3(1.0e-6, f64::NAN, None)
            .is_err()
    );
}

fn audit_experiment(path: &str) -> Value {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/adversarial_audit_20260927/experiments")
        .join(path);
    serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}

fn case_rtol(case_id: &str) -> f64 {
    let tail = case_id.split("-rtol-").nth(1).unwrap();
    tail.split("-v2").next().unwrap().parse().unwrap()
}

/// Offline regression on the audit's own records (E-02 reproduces the
/// committed ab8fbcd arms bitwise; E-03 is the SciPy Radau control).  The
/// campaign's tight basis is (1e-10, 1e-8) and the case basis is
/// (0.01 rtol, rtol), so case-tolerance WRMS = tight WRMS * 1e-8 / rtol.
#[test]
fn audit_records_classify_as_predeclared_under_the_new_protocol() {
    let e02 = audit_experiment("E-02/results.json");
    let e03 = audit_experiment("E-03/results.json");
    let b = TWO_ARM_GLOBAL_ERROR_BUDGET;

    let mut dense_within = Vec::new();
    let mut dense_exceeds = Vec::new();
    for row in e02["rows"].as_array().unwrap() {
        let case_id = row["case_id"].as_str().unwrap();
        let scale = 1.0e-8 / row["rtol"].as_f64().unwrap();
        let dense = &row["dense"];
        let e = dense["max_grid_wrms"]["committed"].as_f64().unwrap() * scale;
        let u = dense["reference_uncertainty_wrms"].as_f64().unwrap() * scale;
        match classify_arm_budget(e, u, b).unwrap() {
            ArmBudget::WithinBudget => dense_within.push(case_id.to_owned()),
            ArmBudget::ExceedsBudget => dense_exceeds.push(case_id.to_owned()),
            ArmBudget::ReferenceUndecidable => panic!("{case_id} undecidable"),
        }
    }
    assert_eq!(dense_within.len(), 15);
    assert_eq!(dense_exceeds.len(), 3);
    assert!(
        dense_exceeds
            .iter()
            .all(|id| id.starts_with("semilinear-advection-diffusion-ramped"))
    );

    let mut scipy_within = 0;
    let mut scipy_max_case = 0.0_f64;
    for row in e03["e03c_scipy_rows"].as_array().unwrap() {
        let case_id = row["case_id"].as_str().unwrap();
        let scale = 1.0e-8 / case_rtol(case_id);
        let e = row["max_grid_wrms_tight"].as_f64().unwrap() * scale;
        let u = row["ref_interp_uncertainty_max"].as_f64().unwrap() * scale;
        scipy_max_case = scipy_max_case.max(e);
        if classify_arm_budget(e, u, b).unwrap() == ArmBudget::WithinBudget {
            scipy_within += 1;
        }
    }
    assert_eq!(
        scipy_within, 18,
        "the new criterion is attainable by SciPy Radau"
    );
    assert!(scipy_max_case < 1.7);

    // The frozen 0.1 rule rejects SciPy Radau's own first-step pair in 17/18.
    let pairs = e03["e03b_pairs"]["A_vs_B_first_step"].as_array().unwrap();
    let dominated = pairs
        .iter()
        .filter(|pair| {
            classify_output_policy_dominance(
                pair["gap_wrms_tight"].as_f64().unwrap(),
                pair["denominator_max_grid_wrms"].as_f64().unwrap(),
            )
            .unwrap()
                == OutputPolicyDominance::Dominated
        })
        .count();
    assert_eq!((dominated, pairs.len()), (17, 18));
}

/// External audit VIG-A07 (2026-09-29): the bands are defined on the exact
/// values `E + U` and `E - U`; a rounded `10 + 1e-16 = 10` made a boundary
/// row WithinBudget.
#[test]
fn arm_budget_bands_are_decided_on_the_exact_sum_and_difference() {
    let b = 10.0_f64;
    let ulp = b - b.next_down(); // 2^-49 just below 10
    let above = b.next_up() - b; // 2^-49 just above 10
    let cases = [
        (b, 1.0e-16, ArmBudget::ReferenceUndecidable),
        (b, 0.0, ArmBudget::WithinBudget),
        (b.next_down(), ulp, ArmBudget::WithinBudget),
        (b.next_down(), ulp / 2.0, ArmBudget::WithinBudget),
        (
            b.next_down(),
            ulp + 2.0_f64.powi(-60),
            ArmBudget::ReferenceUndecidable,
        ),
        (b.next_up(), above, ArmBudget::ReferenceUndecidable),
        (
            b.next_up(),
            above - 2.0_f64.powi(-80),
            ArmBudget::ExceedsBudget,
        ),
        (b.next_up(), 0.0, ArmBudget::ExceedsBudget),
        (f64::MAX, f64::MAX, ArmBudget::ReferenceUndecidable),
    ];
    for (error, uncertainty, expected) in cases {
        let budget = if error == f64::MAX { f64::MAX } else { b };
        assert_eq!(
            classify_arm_budget(error, uncertainty, budget).unwrap(),
            expected,
            "E = {error:e}, U = {uncertainty:e}, B = {budget:e}"
        );
    }
}

#[test]
fn case_tolerance_uncertainty_is_rounded_upward() {
    // Weight ratio tight / case = 1 / 3, so U_case = U / 3 is inexact.
    for u in [1.0, 2.0, 1.0e-300, 7.0e10] {
        let basis = ReferenceWrmsBasis::new(
            CommonOutputGrid::new(vec![0.0, 1.0]).unwrap(),
            vec![vec![0.0], vec![0.0]],
            ExternalErrorScale::with_reference_uncertainty(vec![1.0], 0.0, u).unwrap(),
        )
        .unwrap();
        let case = basis.with_case_tolerance(3.0, 0.0).unwrap();
        let u_case = case.error_scale.reference_uncertainty_wrms;
        assert!(
            u_case.mul_add(3.0, -u) >= 0.0,
            "U = {u:e}: {u_case:e} is below U / 3"
        );
    }
}
