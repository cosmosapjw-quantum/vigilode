//! Reference ladder qualification contracts (V3-REFERENCE-LADDER).
//!
//! Validates:
//! 1. Multi-level independent reference ladder structure for spatial problems.
//! 2. Strict decreasing-tolerance refinement sequence (`rtols`, `atol_factors`).
//! 3. Cross-family cross-checking requirements (RADAU5 vs CVODE BDF).
//! 4. Rigorous separation: empirical reference agreement is an uncertainty estimate,
//!    never a continuous-time validated interval enclosure.

use vigilode_bench::arms::Arm;
use vigilode_bench::corpus::Corpus;
use vigilode_bench::reference_policy::{
    INDEPENDENT_TIER, independent_level_pass, validate_independent_policy,
};

const PROTOCOL_V2_PATH: &str = "../../bench/protocols/adversarial_spatial_grid_v2.json";

#[test]
fn test_spatial_problems_reference_ladder_configuration() {
    let content = std::fs::read_to_string(PROTOCOL_V2_PATH)
        .or_else(|_| std::fs::read_to_string("bench/protocols/adversarial_spatial_grid_v2.json"))
        .expect("adversarial_spatial_grid_v2.json must exist");

    let corpus = Corpus::from_json(&content).expect("Corpus JSON parsing must succeed");
    let protocol = corpus
        .reference
        .independent_protocol
        .as_ref()
        .expect("independent_protocol must be configured in v2");

    assert_eq!(protocol.tier, INDEPENDENT_TIER);
    assert_eq!(protocol.rtols, vec![1e-10, 1e-11, 1e-12]);
    assert_eq!(protocol.atol_factors, vec![1.0, 0.1, 0.01]);
    assert!(protocol.agreement_threshold <= 1e-10);
    assert!(protocol.refinement_threshold <= 1e-10);

    // Validate using policy validator
    validate_independent_policy(
        &protocol.tier,
        &protocol.rtols,
        &protocol.atol_factors,
        protocol.agreement_threshold,
        protocol.refinement_threshold,
    )
    .expect("reference ladder policy must pass validation");

    // Decisive cross-check requirements
    assert_eq!(corpus.reference.arm, Arm::Radau5);
    assert_eq!(corpus.reference.crosscheck_arm, Arm::CvodeBdfDense);
    assert_ne!(
        corpus.reference.arm.implementation_family(),
        corpus.reference.crosscheck_arm.implementation_family(),
        "Reference and crosscheck arm must belong to distinct implementation families"
    );
}

#[test]
fn test_independent_level_pass_logic_fail_closed() {
    // Both families succeed and cross-family discrepancy is within agreement_threshold
    assert!(independent_level_pass(
        true,
        Some(1e-12),
        None,
        1e-10,
        1e-10
    ));

    // Adjacent refinement check passes
    assert!(independent_level_pass(
        true,
        Some(1e-12),
        Some((Some(5e-11), Some(3e-11))),
        1e-10,
        1e-10
    ));

    // Fail closed if either arm failed
    assert!(!independent_level_pass(
        false,
        Some(1e-12),
        None,
        1e-10,
        1e-10
    ));

    // Fail closed if discrepancy exceeds agreement_threshold
    assert!(!independent_level_pass(
        true,
        Some(1e-9),
        None,
        1e-10,
        1e-10
    ));

    // Fail closed on nonfinite discrepancy
    assert!(!independent_level_pass(
        true,
        Some(f64::NAN),
        None,
        1e-10,
        1e-10
    ));
}

#[test]
fn test_empirical_uncertainty_vs_rigorous_enclosure_boundary() {
    // An empirical reference ladder compares two discrete floating-point numerical
    // ODE integrator families (e.g. Hairer RADAU5 Fortran vs LLNL SUNDIALS CVODE C).
    // This establishes an empirical reference discrepancy bound delta_ref.
    // It does NOT establish:
    // 1. Continuous interval containment y(t) in [y_lo(t), y_hi(t)] for all t in [t0, tf].
    // 2. Proof of existence and uniqueness of the true solution via Banach/Picard contraction.
    // 3. Directed rounding enclosure of local truncation errors.
    let crosscheck_family = Arm::CvodeBdfDense.implementation_family();
    let ref_family = Arm::Radau5.implementation_family();
    assert_ne!(crosscheck_family, ref_family);
    assert_eq!(ref_family, "hairer-radau5");
    assert_eq!(crosscheck_family, "sundials-cvode");
}
