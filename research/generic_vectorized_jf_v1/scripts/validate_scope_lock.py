#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
STATE = ROOT / "research" / "generic_vectorized_jf_v1" / "state"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args: str) -> str:
    return subprocess.check_output(["git", "-C", str(ROOT), *args], text=True).strip()


def production_rust_sources() -> list[Path]:
    return sorted(p for p in (ROOT / "crates").glob("*/src/**/*.rs") if p.is_file())


def main() -> None:
    provenance = json.loads((STATE / "BASELINE_PROVENANCE.json").read_text())
    dag = json.loads((STATE / "DAG_STATUS.json").read_text())

    base = provenance["base_commit"]
    subprocess.run(
        ["git", "-C", str(ROOT), "merge-base", "--is-ancestor", base, "HEAD"],
        check=True,
    )

    checked_hashes: dict[str, str] = {}
    checked_build_hashes: dict[str, str] = {}
    for rel, expected in provenance.get("build_lock_hashes", {}).items():
        actual = sha256(ROOT / rel)
        assert actual == expected, f"build lock drift: {rel}: {actual} != {expected}"
        checked_build_hashes[rel] = actual
    for rel, expected in provenance["frozen_comparator_hashes"].items():
        actual = sha256(ROOT / rel)
        assert actual == expected, f"frozen comparator drift: {rel}: {actual} != {expected}"
        checked_hashes[rel] = actual

    required = list(provenance["required_generic_modules"])
    for rel in required:
        assert (ROOT / rel).is_file(), f"missing generic core module: {rel}"

    forbidden_tokens = (
        "rec_bianchi",
        "rei_bianchi",
        "RABBIT_v1.0.5_PRODUCTION",
        "AcceptedRadiationParent",
        "CoupledCollisionTransportProblem",
        "FLEX_JF_TX",
    )
    violations: list[dict[str, str]] = []
    for path in production_rust_sources():
        text = path.read_text(encoding="utf-8")
        for token in forbidden_tokens:
            if token in text:
                violations.append({"file": str(path.relative_to(ROOT)), "token": token})
    assert not violations, f"physical-client binding leaked into generic production source: {violations}"

    assert dag["current_node"]["id"] == "G0_SCOPE_RESET"
    assert dag["current_node"]["status"] == "COMPLETE"
    assert dag["next_node"]["id"] == "G1_TRANSACTIONAL_Q1_Q2_ENDPOINT_ENGINE"

    report = {
        "schema": "generic-vectorized-jf-scope-validation-v1",
        "head": git("rev-parse", "HEAD"),
        "tree": git("rev-parse", "HEAD^{tree}"),
        "base_commit": base,
        "base_is_ancestor": True,
        "frozen_comparator_hashes": checked_hashes,
        "build_lock_hashes": checked_build_hashes,
        "required_generic_modules": required,
        "production_rust_sources_scanned": len(production_rust_sources()),
        "physical_client_binding_violations": violations,
        "current_node": dag["current_node"],
        "next_node": dag["next_node"],
        "status": "PASS",
    }
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
