use rodas5p_integrators::{
    G4S5B0Family, G4S5B0Profile, committed_g4_s5b0_linear_tolerance_arm, g4_s5b0_rjf_trace_digest,
    run_g4_s5b0_rjf_attempt_trace_family_with_linear_tolerance_arm,
};

// Captured from the deliberate RED run at compile-closure head
// 82e35ead9126d5965dc60922e490aa9ff1ecf7e6, GitHub Actions run
// 32875101031, using the committed `legacy-fixed` arm. Wall-clock fields are
// excluded by the canonical digest contract.
const HISTORICAL_A1_SMOKE_HIRES_TRACE_SHA256: &str =
    "04cd9202b25df52357045754c8f10ffcb93d35f234ecace6a0c947ca306049de";
// Non-authoritative regression baseline after the scientific-validity-v2
// problem, derivative, inner-forcing, and work-accounting transition.  The
// historical A1 digest above remains recorded and is not rewritten.
const SCIENTIFIC_VALIDITY_V2_SMOKE_HIRES_TRACE_SHA256: &str =
    "1df89540c3b1374a47b554d0f0f2f31747ff1316aa5799eec3f94a76ec1f9b61";

#[test]
fn historical_external_digest_is_preserved_while_v2_trace_has_its_own_baseline() {
    let arm = committed_g4_s5b0_linear_tolerance_arm();
    let report = run_g4_s5b0_rjf_attempt_trace_family_with_linear_tolerance_arm(
        G4S5B0Profile::Smoke,
        G4S5B0Family::HiresRamped,
        arm,
    )
    .unwrap();
    let digest = g4_s5b0_rjf_trace_digest(&report);

    assert_ne!(
        SCIENTIFIC_VALIDITY_V2_SMOKE_HIRES_TRACE_SHA256, HISTORICAL_A1_SMOKE_HIRES_TRACE_SHA256,
        "the v2 transition must not masquerade as the historical A1 receipt"
    );
    assert_eq!(
        digest,
        SCIENTIFIC_VALIDITY_V2_SMOKE_HIRES_TRACE_SHA256,
        "scientific-validity-v2 G4/S5B0 regression trace changed; actual digest={digest}, arm={}",
        arm.as_str()
    );
}
