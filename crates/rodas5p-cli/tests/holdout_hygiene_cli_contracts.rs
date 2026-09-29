use std::process::Command;

/// Audit F-047: the replayed k=1 table is refused on holdout profiles from
/// the CLI too, before any trajectory runs.
#[test]
fn replayed_k1_table_is_refused_on_holdout_profiles() {
    for (command, profile) in [
        ("generic-policy-redesign-actual-prefix", "holdout"),
        ("generic-policy-redesign-level2-prefix", "holdout"),
    ] {
        for policy in ["replayed-k1", "frozen-k1"] {
            let output = Command::new(env!("CARGO_BIN_EXE_rodas5p"))
                .args([
                    command,
                    "--profile",
                    profile,
                    "--family",
                    "robertson",
                    "--policy",
                    policy,
                    "--output",
                ])
                .arg(std::env::temp_dir().join(format!(
                    "rodas5p-holdout-{}-{command}-{policy}.json",
                    std::process::id()
                )))
                .output()
                .unwrap();
            assert!(!output.status.success(), "{command} {policy}");
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains("calibration-only"),
                "{command} {policy}: {stderr}"
            );
        }
    }
}
