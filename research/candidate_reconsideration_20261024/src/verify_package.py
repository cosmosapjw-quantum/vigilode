#!/usr/bin/env python3
"""Check publication identities and recorded coverage; does not run a solver."""
import hashlib
import json
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
REPO = PACKAGE.parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    failures = []
    manifest = json.loads((PACKAGE / "ARTIFACT_MANIFEST.json").read_text())
    for name, expected in manifest["files"].items():
        p = REPO / name
        if not p.is_file() or sha(p) != expected:
            failures.append("manifest: " + name)
    freeze = json.loads((PACKAGE / "SOURCE_FREEZE.json").read_text())
    for name, expected in freeze["sha256"].items():
        if sha(REPO / name) != expected:
            failures.append("source: " + name)
    for receipt in (PACKAGE / "evidence/runs").glob("*.receipt.json"):
        r = json.loads(receipt.read_text())
        stem = receipt.name.removesuffix(".receipt.json")
        for stream in ["stdout", "stderr"]:
            if sha(receipt.parent / f"{stem}.{stream}") != r[f"{stream}_sha256"]:
                failures.append("receipt: " + stem + ":" + stream)
    inventory = json.loads((PACKAGE / "analysis/CANDIDATE_REASSESSMENT.json").read_text())
    original = json.loads((REPO / "research/general_ode_selection_20261024/analysis/PORTFOLIO.json").read_text())
    if {r["id"] for r in inventory["portfolio_families"]} != {r["id"] for r in original["families"]}:
        failures.append("portfolio coverage")
    validation = json.loads((PACKAGE / "VALIDATION.json").read_text())
    if sum(x["passed"] for x in validation["final_native_checks"]) != validation["unique_native_tests"]:
        failures.append("test count")
    for item in validation["final_native_checks"]:
        r = json.loads((PACKAGE / item["receipt"]).read_text())
        if r["exit_code"] or item["failed"]:
            failures.append("final native failure: " + item["id"])
    decisions = json.loads((PACKAGE / "DECISIONS.json").read_text())
    if any(x["completed_rounds"] > 3 for x in decisions["revision_units"]):
        failures.append("revision limit")
    print(json.dumps({"schema_version": 1, "status": "FAIL" if failures else "PASS",
        "manifest_files": len(manifest["files"]), "source_inputs": len(freeze["sha256"]),
        "inventory_families": len(inventory["portfolio_families"]),
        "native_tests": validation["unique_native_tests"],
        "scope": "Identity, receipt consistency and coverage only; scientific validity uses proofs and executed checks.",
        "failures": failures}, indent=2))
    return bool(failures)


if __name__ == "__main__":
    raise SystemExit(main())
