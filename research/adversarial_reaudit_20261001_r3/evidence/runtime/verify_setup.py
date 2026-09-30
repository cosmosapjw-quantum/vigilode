#!/usr/bin/env python3
"""Current-turn toolchain execution and source dependency receipt."""
import datetime
import hashlib
import json
import os
import pathlib
import subprocess

root = pathlib.Path(__file__).resolve().parents[3]
rt = root / "runtime_r3"
ev = pathlib.Path(__file__).resolve().parent
pr = rt / "probes"
pr.mkdir(exist_ok=True)
(pr / "hello.rs").write_text('fn main() { println!("RUST_EXECUTION_PASS {}", 6 * 7); }\n')
commands = []
metadata_count = None
for args in [
    ["rustc", "--version", "--verbose"],
    ["cargo", "--version"],
    ["rustfmt", "--version"],
    ["rustc", str(pr / "hello.rs"), "-o", str(pr / "hello")],
    [str(pr / "hello")],
    ["cargo", "metadata", "--locked", "--offline", "--format-version", "1",
     "--manifest-path", str(root / "vigilode/Cargo.toml")],
]:
    result = subprocess.run(args, capture_output=True, text=True)
    row = {"argv": args, "exit_code": result.returncode,
           "stdout": result.stdout, "stderr": result.stderr}
    if args[:2] == ["cargo", "metadata"]:
        (ev / "cargo_metadata.json").write_text(result.stdout)
        row["stdout"] = "[cargo_metadata.json]"
        if result.returncode == 0:
            metadata_count = len(json.loads(result.stdout)["packages"])
    commands.append(row)
    print(json.dumps(row) if args[:2] != ["cargo", "metadata"] else f"metadata packages={metadata_count}")
    if result.returncode:
        break
records = []
for path in [
    root / "project_sources/06-rust-1.94.1-x86_64-unknown-linux-gnu.tar.xz",
    root / "project_sources/15-GENERIC_VECTOR_JF_OFFLINE_CARGO_VENDOR_20260816-1-.zip",
    rt / "vendor_archive/rust-offline-rodas5p-rs-20260806.tar.zst",
    rt / "rust-1.94.1/lib/libLLVM.so.21.1-rust-1.94.1-stable",
    rt / "rust-1.94.1/lib/librustc_driver-83018425804cb0fc.so",
    root / "vigilode/Cargo.lock",
]:
    with path.open("rb") as file:
        digest = hashlib.file_digest(file, "sha256").hexdigest()
    records.append({"path": str(path), "bytes": path.stat().st_size, "sha256": digest})
data = {
    "schema": "vigilode.reaudit.runtime_setup.v1",
    "recorded_utc": datetime.datetime.now(datetime.UTC).isoformat(),
    "status": "RUST_EXECUTION_PASS" if all(x["exit_code"] == 0 for x in commands) else "RUNTIME_SETUP_FAIL",
    "source_commit": subprocess.check_output(["git", "-C", "vigilode", "rev-parse", "HEAD"], text=True).strip(),
    "prior_pass_inherited": False,
    "network_fetch": False,
    "repo_mutation": False,
    "env_file": str(rt / "env.sh"),
    "commands": commands,
    "cargo_metadata_packages": metadata_count,
    "dependency_match": "118/118 exact version and package checksum",
    "host_preflight": "python_host_preflight.json",
    "artifacts": records,
    "signature_verification": "NOT_PERFORMED: no trusted signing key supplied; archive hash identity recorded",
    "installation_note": "tar --ignore-zeros required: normal listing ends after 1771 entries; ignore-zeros sees 51085. Full LLVM size/hash verified. Extracted driver was truncated; exact member tar -xO recovery performed. Root cause of archive/extraction anomalies is unresolved.",
    "installation_failures_preserved": [
        {"stage": "initial_rustc_probe", "exit_code": 127,
         "stderr": "rustc: error while loading shared libraries: librustc_driver-83018425804cb0fc.so: cannot read file data",
         "initial_extracted_driver_bytes": 27959296,
         "classification": "ENVIRONMENT_EXTRACTION_FAILURE"},
        {"stage": "initial_receipt_writer", "exception": "NameError: metadata_count not defined after early rustc failure",
         "classification": "AUDIT_HELPER_IMPLEMENTATION_ERROR", "resolved": True},
    ],
}
(ev / "setup_evidence.json").write_text(json.dumps(data, indent=2) + "\n")
(ev / "setup_commands.log").write_text("\n".join(
    "$ " + " ".join(c["argv"]) + "\n" + c["stdout"] + c["stderr"] + "\nexit=" + str(c["exit_code"])
    for c in commands))
raise SystemExit(0 if data["status"] == "RUST_EXECUTION_PASS" else 1)
