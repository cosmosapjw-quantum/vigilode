use std::{fs, process::Command};

fn temp_path(name: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "rodas5p-rs-allocation-{}-{name}",
        std::process::id()
    ));
    path
}

#[test]
fn allocation_audit_reports_end_to_end_solver_allocations_and_work() {
    let main_binary = env!("CARGO_BIN_EXE_rodas5p");
    let audit_binary = env!("CARGO_BIN_EXE_rodas5p-allocation-audit");
    let trace = temp_path("trace.json");
    let audit = temp_path("audit.json");

    let status = Command::new(main_binary)
        .args([
            "trace",
            "--kind",
            "fixed",
            "--dimension",
            "8",
            "--steps",
            "1",
            "--stages",
            "8",
            "--stiffness",
            "100",
            "--nonnormality",
            "0.05",
            "--output",
        ])
        .arg(&trace)
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new(audit_binary)
        .args(["--trace"])
        .arg(&trace)
        .args(["--output"])
        .arg(&audit)
        .args([
            "--repetitions",
            "1",
            "--restart",
            "6",
            "--recycle-dim",
            "2",
            "--operator-budget",
            "300",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let document: serde_json::Value = serde_json::from_slice(&fs::read(&audit).unwrap()).unwrap();
    assert_eq!(document["schema"], "rodas5p-rust-allocation-audit-v1");
    let rows = document["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 7);
    for row in rows {
        assert_eq!(row["failures"], 0);
        assert!(row["allocations"].as_u64().unwrap() > 0);
        assert!(row["allocated_bytes"].as_u64().unwrap() > 0);
        assert!(row["operator_total"].as_u64().unwrap() > 0);
    }

    let _ = fs::remove_file(trace);
    let _ = fs::remove_file(audit);
}
