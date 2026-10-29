//! Certified output-budget enclosures (re-audit R3 of 2026-10-01, ARITH-04).

use rodas5p_integrators::{CertifiedBudgetStatus, OutputBudgetPolicy, step_power_enclosure};
use serde::Deserialize;

const FIXTURES: &str = include_str!("../../../fixtures/r3_certified_budget_fixtures.json");

#[derive(Deserialize)]
struct Fixtures {
    rows: Vec<Row>,
}

#[derive(Deserialize)]
struct Row {
    case: String,
    epsilon_ref: String,
    h: String,
    h_ref: String,
    exponent: u32,
    current_above_exact: bool,
    exact_floor: String,
    exact_ceil: String,
}

fn bits(hex: &str) -> f64 {
    if hex == "inf" {
        return f64::INFINITY;
    }
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

#[test]
fn every_enclosure_contains_the_exact_budget() {
    let fixtures: Fixtures = serde_json::from_str(FIXTURES).unwrap();
    assert_eq!(fixtures.rows.len(), 18);
    for row in &fixtures.rows {
        let (eps, h, h_ref) = (bits(&row.epsilon_ref), bits(&row.h), bits(&row.h_ref));
        let enclosure = step_power_enclosure(eps, h, h_ref, row.exponent).unwrap();
        let (floor, ceil) = (bits(&row.exact_floor), bits(&row.exact_ceil));
        assert!(
            enclosure.lower <= floor,
            "{}: lower {:e} > {floor:e}",
            row.case,
            enclosure.lower
        );
        assert!(
            enclosure.upper >= ceil,
            "{}: upper {:e} < {ceil:e}",
            row.case,
            enclosure.upper
        );
        if row.exponent <= 64 && ceil.is_finite() && floor > 0.0 {
            // Tight: within a few ulps of the exact value.
            let mut probe = floor;
            for _ in 0..4 {
                probe = probe.next_down();
            }
            assert!(enclosure.lower >= probe, "{}: loose lower", row.case);
        }
        if row.current_above_exact {
            // The round-to-nearest budget exceeds the exact one; the
            // certified decision rejects an error equal to it.
            let policy = OutputBudgetPolicy::step_power(eps, h_ref, row.exponent);
            if let Ok(policy) = policy {
                let current = policy.budget(0.0, h).unwrap();
                let decision = policy.certified_decide(current, 0.0, h).unwrap();
                assert!(
                    !decision.accepted,
                    "{}: certified accept of {current:e}",
                    row.case
                );
            }
        }
    }
}

#[test]
fn zero_underflow_and_overflow_have_explicit_semantics() {
    let zero = step_power_enclosure(0.0, 1.0, 1.0e-160, 2).unwrap();
    assert_eq!(
        (zero.lower, zero.upper, zero.status),
        (0.0, 0.0, CertifiedBudgetStatus::ExactZero)
    );
    let underflow = step_power_enclosure(f64::from_bits(1), 1.0e-300, 1.0, 1).unwrap();
    assert_eq!(underflow.status, CertifiedBudgetStatus::UnderflowLowerZero);
    assert_eq!(underflow.lower, 0.0);
    let overflow = step_power_enclosure(1.0, 1.0e300, 1.0e-300, 2).unwrap();
    assert_eq!(overflow.status, CertifiedBudgetStatus::OverflowNonBinding);
    assert_eq!(overflow.lower, f64::MAX);
    // R2-POL-01: 0.999988867182683 is enclosed; WRMS 5 is rejected and a
    // WRMS below the lower bound accepted.
    let policy = OutputBudgetPolicy::mixed(1.0e300, 1.0e-320, 1.0e-160, 2).unwrap();
    assert!(!policy.certified_decide(5.0, 1.0, 1.0).unwrap().accepted);
    assert!(policy.certified_decide(0.99998, 1.0, 1.0).unwrap().accepted);
}
