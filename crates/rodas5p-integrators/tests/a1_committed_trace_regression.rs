use rodas5p_integrators::{
    G4S5B0Family, G4S5B0Profile, committed_g4_s5b0_linear_tolerance_arm,
    g4_s5b0_rjf_trace_digest,
    run_g4_s5b0_rjf_attempt_trace_family_with_linear_tolerance_arm,
};

// Captured from the deliberate RED run at compile-closure head
// 82e35ead9126d5965dc60922e490aa9ff1ecf7e6, GitHub Actions run
// 32875101031, using the committed `legacy-fixed` arm. Wall-clock fields are
// excluded by the canonical digest contract.
const EXPECTED_COMMITTED_SMOKE_HIRES_TRACE_SHA256: &str =
    "04cd9202b25df52357045754c8f10ffcb93d35f234ecace6a0c947ca306049de";

#[test]
fn committed_smoke_hires_rjf_trace_is_frozen_against_an_external_digest() {
    let arm = committed_g4_s5b0_linear_tolerance_arm();
    let report = run_g4_s5b0_rjf_attempt_trace_family_with_linear_tolerance_arm(
        G4S5B0Profile::Smoke,
        G4S5B0Family::HiresRamped,
        arm,
    )
    .unwrap();
    let digest = g4_s5b0_rjf_trace_digest(&report);

    assert_eq!(
        digest, EXPECTED_COMMITTED_SMOKE_HIRES_TRACE_SHA256,
        "committed G4/S5B0 trace changed; actual digest={digest}, arm={}",
        arm.as_str()
    );
}
