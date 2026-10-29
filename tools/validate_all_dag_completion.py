#!/usr/bin/env python3
"""Verify the all-node handoff; optionally replay only the changed native scope.

No network, dependency installation, tolerance change, git mutation or promotion.
The default operation checks the source manifest and 12-node coverage only.
Native execution is explicit and preserves every command's failure log.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "research/all_dag_completion_20261018"
NODE_IDS = [f"A{i:02}" for i in range(1, 13)]


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def check_handoff():
    dag = json.loads((REPORT / "DAG.json").read_text())
    manifest = json.loads((REPORT / "SOURCE_MANIFEST.json").read_text())
    ids = [node["id"] for node in dag["nodes"]]
    errors = []
    if sorted(ids) != NODE_IDS:
        errors.append("DAG must contain each original node A01-A12 exactly once")
    for node in dag["nodes"]:
        if not node.get("owner") or not node.get("delivered") or not node.get("evidence"):
            errors.append(f"incomplete coverage for {node['id']}")
        for relative in node.get("evidence", []):
            if not (ROOT / relative).is_file():
                errors.append(f"missing evidence {relative}")
        if not set(node.get("depends_on", [])).issubset(NODE_IDS):
            errors.append(f"unknown dependency for {node['id']}")
    for relative, expected in manifest["files_sha256"].items():
        path = ROOT / relative
        if not path.is_file() or digest(path) != expected:
            errors.append(f"source changed or missing: {relative}")
    return {"schema_version": 1, "node_count": len(ids),
            "source_files_checked": len(manifest["files_sha256"]),
            "source_manifest_sha256": digest(REPORT / "SOURCE_MANIFEST.json"),
            "dag_sha256": digest(REPORT / "DAG.json"),
            "errors": errors, "passed": not errors,
            "production_promotion": False, "speed_promotion": False}


def native_commands():
    cargo = ["cargo", "test", "--locked", "--offline"]
    tests = [
        "all_dag_a01_packet", "production_audit_packet_adaptive",
        "production_audit_packet_verified", "production_audit_damped_packet",
        "all_dag_a05_verified", "all_dag_a06_direct",
        "collective_rodas5p_allocation_contract", "nonlinear_homotopy_contracts",
        "actual_k_homotopy_contracts", "all_dag_a10_bernstein",
        "all_dag_a11_verified",
    ]
    integrators = cargo + ["-p", "rodas5p-integrators"]
    for target in tests:
        integrators += ["--test", target]
    return [
        ("core_contracts", cargo + ["-p", "rodas5p-core", "--test",
         "leja_enclosed_contract", "--test", "guarded_polynomial_router", "--", "--test-threads=1"]),
        ("integrator_contracts", integrators + ["--", "--nocapture", "--test-threads=1"]),
        ("homotopy_unit", cargo + ["-p", "rodas5p-integrators", "--lib",
         "nonlinear_homotopy::tests"]),
        ("leja_token_doc", cargo + ["-p", "rodas5p-core", "--doc", "AdmittedLejaFused"]),
        ("polynomial_token_doc", cargo + ["-p", "rodas5p-core", "--doc", "GuardedPolynomialAction"]),
        ("tool_contracts", [sys.executable, "-m", "unittest", "discover",
         "-s", "tools/tests", "-p", "test_all_dag_*.py", "-v"]),
    ]


def execute(output):
    output = Path(output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        raise ValueError("execution output must be a new empty directory")
    before = check_handoff()
    if not before["passed"]:
        raise ValueError("handoff/source verification failed: " + repr(before["errors"]))
    commands = native_commands()
    protocol = {"schema_version": 1, "verification_before": before,
                "commands": [{"id": label, "argv": argv} for label, argv in commands],
                "selection": "changed native contracts only; historical campaigns reused",
                "performance_campaign": "separate optional all_dag_benchmark.py run",
                "promotion": "NONE"}
    (output / "PROTOCOL.json").write_text(json.dumps(protocol, indent=2) + "\n")
    results = []
    for label, argv in commands:
        started = time.perf_counter()
        print(f"Running {label}", flush=True)
        with (output / (label + ".log")).open("w") as stream:
            try:
                run = subprocess.run(argv, cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT,
                                     check=False)
                code = run.returncode
            except OSError as exc:
                stream.write(str(exc) + "\n")
                code = 127
        results.append({"id": label, "argv": argv, "exit_code": code,
                        "wall_seconds": time.perf_counter() - started,
                        "log": label + ".log", "log_sha256": digest(output / (label + ".log"))})
        (output / "COMMANDS.json").write_text(json.dumps(results, indent=2) + "\n")
    after = check_handoff()
    unchanged = all(before[key] == after[key] for key in ("source_manifest_sha256", "dag_sha256"))
    result = {"schema_version": 1, "verification_before": before, "verification_after": after,
              "source_and_dag_unchanged": unchanged,
              "commands": results,
              "passed": unchanged and before["passed"] and after["passed"] and all(r["exit_code"] == 0 for r in results),
              "production_promotion": False, "speed_promotion": False}
    (output / "RESULT.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--execute", action="store_true", help="compile and run the changed native scope")
    parser.add_argument("--output", help="new directory for source-bound execution logs")
    args = parser.parse_args()
    if args.execute and not args.output:
        parser.error("--execute requires --output")
    result = execute(args.output) if args.execute else check_handoff()
    print(json.dumps(result, indent=2))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError) as exc:
        print(json.dumps({"passed": False, "error": str(exc)}), file=sys.stderr)
        raise SystemExit(2)
