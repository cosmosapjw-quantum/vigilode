#!/usr/bin/env python3
"""Inventory and audit VigilODE A1 tolerance-policy call sites.

This script is read-only and uses only the Python standard library.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


EXPECTED_HEAD = "67ec3ad77d0a88f3ff9c096b309d3a12da72b600"
ATLAS_REL = Path("crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs")
POLICY_REL = Path("crates/rodas5p-integrators/src/g4_s5b0_inner_tolerance.rs")
LIB_REL = Path("crates/rodas5p-integrators/src/lib.rs")
TEST_REL = Path("crates/rodas5p-integrators/tests/a1_inner_tolerance_parity_contracts.rs")
WORKFLOW_REL = Path(".github/workflows/a1-inner-tolerance-parity.yml")


def read_required(repo: Path, rel: Path) -> str:
    path = repo / rel
    if not path.is_file():
        raise FileNotFoundError(f"missing required file: {rel}")
    return path.read_text(encoding="utf-8")


def line_inventory(rel: Path, text: str) -> list[dict[str, Any]]:
    needles = (
        "G4S5B0InnerTolerancePolicy",
        "inner_tolerance_policy(",
        "phi_config(",
        "linear_config(",
        "relative_tolerance",
        "absolute_tolerance",
        "rtol: 1.0e-10",
        "atol: 1.0e-12",
    )
    rows: list[dict[str, Any]] = []
    for number, line in enumerate(text.splitlines(), start=1):
        matched = [needle for needle in needles if needle in line]
        if matched:
            rows.append(
                {
                    "file": rel.as_posix(),
                    "line": number,
                    "matches": matched,
                    "source": line.rstrip(),
                }
            )
    return rows


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", required=True, help="implementation worktree root")
    args = parser.parse_args()
    repo = Path(args.repo).expanduser().resolve()

    checks: list[dict[str, Any]] = []

    def check(name: str, condition: bool, detail: Any) -> None:
        checks.append({"name": name, "pass": bool(condition), "detail": detail})

    try:
        atlas = read_required(repo, ATLAS_REL)
        policy = read_required(repo, POLICY_REL)
        lib = read_required(repo, LIB_REL)
        test = read_required(repo, TEST_REL)
        workflow = read_required(repo, WORKFLOW_REL)
    except (OSError, UnicodeError) as exc:
        print(json.dumps({"audit_pass": False, "error": str(exc)}, indent=2))
        return 2

    inventory: list[dict[str, Any]] = []
    for rel, text in (
        (ATLAS_REL, atlas),
        (POLICY_REL, policy),
        (LIB_REL, lib),
        (TEST_REL, test),
    ):
        inventory.extend(line_inventory(rel, text))

    adaptive_calls = atlas.count("linear_config(adaptive.rtol)")
    legacy_rtol = atlas.count("rtol: 1.0e-10")
    legacy_atol = atlas.count("atol: 1.0e-12")
    no_arg_linear_defs = len(re.findall(r"fn\s+linear_config\s*\(\s*\)", atlas))
    rtol_linear_defs = len(re.findall(r"fn\s+linear_config\s*\(\s*rtol\s*:\s*f64\s*\)", atlas))

    check(
        "shared policy type exists",
        "pub struct G4S5B0InnerTolerancePolicy" in policy,
        POLICY_REL.as_posix(),
    )
    check(
        "checked constructor rejects invalid outer tolerance",
        "!outer_rtol.is_finite() || outer_rtol <= 0.0" in policy,
        "constructor must fail closed for nonfinite/nonpositive input",
    )
    check(
        "pre-A1 relative arithmetic preserved",
        "INNER_RELATIVE_FRACTION * outer_rtol" in policy
        and "INNER_RELATIVE_FLOOR" in policy
        and "const INNER_RELATIVE_FRACTION: f64 = 3.0e-2;" in policy
        and "const INNER_RELATIVE_FLOOR: f64 = 1.0e-12;" in policy,
        "max(3.0e-2 * outer_rtol, 1.0e-12)",
    )
    check(
        "pre-A1 absolute arithmetic preserved",
        "INNER_ABSOLUTE_FRACTION * outer_rtol" in policy
        and "INNER_ABSOLUTE_FLOOR" in policy
        and "const INNER_ABSOLUTE_FRACTION: f64 = 3.0e-4;" in policy
        and "const INNER_ABSOLUTE_FLOOR: f64 = 1.0e-14;" in policy,
        "max(3.0e-4 * outer_rtol, 1.0e-14)",
    )
    check(
        "linear and phi builders consume stored relative value",
        policy.count("self.relative_tolerance") >= 2,
        policy.count("self.relative_tolerance"),
    )
    check(
        "linear and phi builders consume stored absolute value",
        policy.count("self.absolute_tolerance") >= 2,
        policy.count("self.absolute_tolerance"),
    )
    check(
        "atlas phi adapter uses shared policy",
        "inner_tolerance_policy(rtol).phi_config(dimension)" in atlas,
        "phi adapter",
    )
    check(
        "atlas linear adapter uses shared policy",
        "inner_tolerance_policy(rtol).linear_config()" in atlas,
        "linear adapter",
    )
    check(
        "all protected linear lanes pass adaptive rtol",
        adaptive_calls == 6,
        adaptive_calls,
    )
    check(
        "legacy fixed linear rtol absent from atlas",
        legacy_rtol == 0,
        legacy_rtol,
    )
    check(
        "legacy fixed linear atol absent from atlas",
        legacy_atol == 0,
        legacy_atol,
    )
    check(
        "no no-argument linear config remains",
        no_arg_linear_defs == 0,
        no_arg_linear_defs,
    )
    check(
        "one rtol-accepting linear adapter exists",
        rtol_linear_defs == 1,
        rtol_linear_defs,
    )
    check(
        "policy exported from crate root",
        "pub use g4_s5b0_inner_tolerance::G4S5B0InnerTolerancePolicy;" in lib,
        LIB_REL.as_posix(),
    )
    check(
        "wiring contract covers all six lanes",
        'matches("linear_config(adaptive.rtol)").count()' in test
        and "6" in test,
        TEST_REL.as_posix(),
    )
    check(
        "focused workflow executes A1 and behavioral contracts",
        "a1_inner_tolerance_parity_contracts" in workflow
        and "g4_s5b0_regime_atlas_contracts" in workflow,
        WORKFLOW_REL.as_posix(),
    )

    payload = {
        "schema": "vigilode-a1-tolerance-site-inventory-v1",
        "repo": str(repo),
        "intake_expected_head": EXPECTED_HEAD,
        "counts": {
            "linear_config_adaptive_rtol_calls": adaptive_calls,
            "fixed_legacy_linear_rtol_literals_in_atlas": legacy_rtol,
            "fixed_legacy_linear_atol_literals_in_atlas": legacy_atol,
            "no_argument_linear_config_definitions": no_arg_linear_defs,
            "rtol_linear_config_definitions": rtol_linear_defs,
        },
        "checks": checks,
        "inventory": inventory,
        "audit_pass": all(item["pass"] for item in checks),
    }
    print(json.dumps(payload, indent=2, sort_keys=True))
    if not payload["audit_pass"]:
        failed = [item["name"] for item in checks if not item["pass"]]
        print("FAILED CHECKS: " + "; ".join(failed), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
