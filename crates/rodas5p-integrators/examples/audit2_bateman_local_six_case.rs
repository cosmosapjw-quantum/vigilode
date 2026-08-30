//! Local-only entry point for the frozen Audit-2 Bateman six-case suite.
//!
//! Standard output contains only the JSON report. Cargo diagnostics remain on
//! standard error, so the handoff command can redirect stdout to an artifact.

use rodas5p_integrators::{
    admit_audit2_bateman_real_client_authority, run_audit2_bateman_local_six_case_suite,
};

const MANIFEST_BYTES: &[u8] = include_bytes!(
    "../../../research/audit2_real_client_authority_construction_20260830/authority_manifest.json"
);
const VERIFIER_BYTES: &[u8] = include_bytes!(
    "../../../research/audit2_real_client_authority_construction_20260830/verify_authority_manifest.py"
);
const PROOF_BYTES: &[u8] = include_bytes!(
    "../../../research/audit2_real_client_authority_construction_20260830/evidence/AUTHORITY_VERIFICATION_RECEIPT.json"
);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let authority =
        admit_audit2_bateman_real_client_authority(MANIFEST_BYTES, VERIFIER_BYTES, PROOF_BYTES)?;
    let report = run_audit2_bateman_local_six_case_suite(authority);
    let passed = report.all_six_executed
        && report.all_contracts_satisfied
        && report.terminal_failure.is_none();
    serde_json::to_writer_pretty(std::io::stdout().lock(), &report)?;
    println!();
    if !passed {
        return Err(std::io::Error::other(
            "Audit-2 Bateman local suite did not satisfy all six frozen contracts",
        )
        .into());
    }
    Ok(())
}
