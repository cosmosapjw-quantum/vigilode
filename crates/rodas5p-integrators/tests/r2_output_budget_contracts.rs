//! Mixed and step-power output budgets (re-audit R2 of 2026-09-30,
//! R2-POL-01).
//!
//! Before: `(h / h_ref)^p` overflowed to +inf and `f64::min` returned the
//! embedded term, so a WRMS of 5 was accepted against a true budget of about
//! 1; with epsilon_ref = 0 the step term was 0 * inf = NaN, which `min` also
//! dropped.

use rodas5p_core::{LinearMethod, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{
    HomotopyPathConfig, HomotopyPredictor, HomotopyStepConfig, OutputBudgetPolicy, homotopy_step,
    manufactured_vector_problem,
};

#[test]
fn the_step_term_is_evaluated_without_intermediate_overflow() {
    // Exact rational value of the represented inputs (audit oracle):
    // 1e-320 (subnormal) * (1 / 1e-160)^2 = 0.999988867182683...
    let mixed = OutputBudgetPolicy::mixed(10.0, 1.0e-320, 1.0e-160, 2).unwrap();
    let budget = mixed.budget(1.0, 1.0).unwrap();
    assert!(
        (budget / 0.999_988_867_182_683 - 1.0).abs() <= 1.0e-12,
        "{budget}"
    );
    assert!(!mixed.decide(5.0, 1.0, 1.0).unwrap().accepted);
    assert!(mixed.decide(0.99, 1.0, 1.0).unwrap().accepted);

    // epsilon_ref = 0: the step term is 0 for every finite ratio, not NaN.
    let zero = OutputBudgetPolicy::mixed(10.0, 0.0, 1.0e-308, 2).unwrap();
    assert_eq!(zero.budget(1.0, 1.0e308).unwrap(), 0.0);
    assert!(!zero.decide(5.0, 1.0, 1.0e308).unwrap().accepted);

    // A step term above f64::MAX does not bind: the embedded term does.
    let large = OutputBudgetPolicy::mixed(10.0, 1.0, 1.0e-160, 2).unwrap();
    assert_eq!(large.budget(1.0, 1.0).unwrap(), 10.0);
    // A step term below the smallest subnormal is 0.
    let small = OutputBudgetPolicy::step_power(1.0e-300, 1.0, 2).unwrap();
    assert_eq!(small.budget(0.0, 1.0e-20).unwrap(), 0.0);
}

#[test]
fn the_step_term_matches_the_direct_formula_where_that_is_exact() {
    for (epsilon_ref, h_ref, exponent, h) in [
        (0.1_f64, 0.04_f64, 6_u32, 0.02_f64),
        (1.0e-3, 1.0, 5, 0.5),
        (3.0, 2.0, 1, 7.0),
        (1.0e-8, 1.0e-3, 4, 3.0e-4),
        (2.5e-5, 0.125, 9, 1.0),
    ] {
        let direct = epsilon_ref * (h / h_ref).powi(exponent as i32);
        let policy = OutputBudgetPolicy::step_power(epsilon_ref, h_ref, exponent).unwrap();
        let value = policy.budget(0.0, h).unwrap();
        assert!(
            (value / direct - 1.0).abs() <= 1.0e-14,
            "{epsilon_ref} {h_ref} {exponent} {h}: {value} vs {direct}"
        );
    }
}

#[test]
fn the_homotopy_step_consumes_the_binding_step_budget_on_both_sides() {
    let fallback = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    };
    let (problem, y0) = manufactured_vector_problem(4, 80.0, 12.0, 0.35, 0.0).unwrap();
    let path = || HomotopyPathConfig::new(0.0, 0, 1, HomotopyPredictor::Euler, 0).unwrap();
    let h = 0.02;
    let run = |policy: OutputBudgetPolicy| {
        let config = HomotopyStepConfig::with_policy(path(), policy).unwrap();
        homotopy_step(
            &problem,
            0.0,
            &y0,
            h,
            &config,
            Some(&fallback),
            None,
            1.0e-7,
            1.0e-6,
            true,
            &mut WorkCounters::default(),
        )
        .unwrap()
    };
    let probe = run(OutputBudgetPolicy::absolute(f64::MAX).unwrap());
    let certificate = probe.output_certificate.clone().unwrap();
    let wrms = certificate.output_wrms;
    assert!(wrms > 0.0 && wrms.is_finite(), "{wrms}");
    // h / h_ref = 2^515, so (h / h_ref)^2 = 2^1030 overflows while the
    // budget epsilon_ref * 2^1030 is finite; eta is large enough that the
    // embedded term never binds.
    let h_ref = h * 2.0_f64.powi(-515);
    let eta = 1.0e300 / certificate.embedded_error.max(1.0e-300);
    for (factor, accepted) in [(1.01, true), (0.99, false)] {
        let epsilon_ref = factor * wrms * 2.0_f64.powi(-1000) * 2.0_f64.powi(-30);
        let policy = OutputBudgetPolicy::mixed(eta, epsilon_ref, h_ref, 2).unwrap();
        let decision = policy.decide(wrms, certificate.embedded_error, h).unwrap();
        assert!(
            (decision.budget / (factor * wrms) - 1.0).abs() <= 1.0e-9,
            "{} vs {}",
            decision.budget,
            factor * wrms
        );
        let report = run(policy);
        assert_eq!(
            report.output_certificate.unwrap().output_wrms,
            wrms,
            "same step, same certificate"
        );
        assert_eq!(report.fast_accepted, accepted, "factor {factor}");
    }
    // The embedded side: a huge step term, eta chosen so that
    // eta * embedded_error = factor * wrms binds.
    let embedded = certificate.embedded_error;
    assert!(embedded > 0.0, "{embedded}");
    for (factor, accepted) in [(1.01, true), (0.99, false)] {
        let eta = factor * wrms / embedded;
        let policy = OutputBudgetPolicy::mixed(eta, 1.0e300, h, 1).unwrap();
        let decision = policy.decide(wrms, embedded, h).unwrap();
        assert!((decision.budget / (factor * wrms) - 1.0).abs() <= 1.0e-12);
        assert_eq!(
            run(policy).fast_accepted,
            accepted,
            "embedded factor {factor}"
        );
    }
}
