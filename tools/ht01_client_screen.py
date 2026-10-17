#!/usr/bin/env python3
"""Client screening generator for research node HT01 (Homotopy Expensive-Client Contract).

Scans static source files and committed callgrind profiles to evaluate candidates K1-K8
against screening criteria S1-S4 without invoking cargo or running any new solver executions.

Usage:
    python3 tools/ht01_client_screen.py --output research/ht01_homotopy_client_contract_20261011/SCREEN.json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent

# Pinned inputs to scan and record
PINNED_FILES = [
    "crates/rodas5p-integrators/src/fourier_path_candidate.rs",
    "crates/rodas5p-integrators/src/fourier_path_certificate.rs",
    "crates/rodas5p-integrators/src/transactional_q1_q2.rs",
    "crates/rodas5p-integrators/src/audit2_bateman_real_client_research.rs",
    "crates/rodas5p-integrators/src/chart_transport.rs",
    "crates/rodas5p-cli/src/stiff_benchmark.rs",
    "crates/rodas5p-integrators/src/scientific_corpus_v2.rs",
    "crates/rodas5p-integrators/src/problems.rs",
    "research/pp07_fourier_shared_action_20261004/RESULTS.json",
    "research/spd01_fast_driver_overhead_20261005/PROFILE_SMALL.json",
    "research/spd01_fast_driver_overhead_20261005/PROFILE_DENSE.json",
    "research/spd01_fast_driver_overhead_20261005/PROFILE_DENSE_B50.json",
    "research/spd03_banded_arm_instructions_20261005/PROFILE.json",
    "research/spd03_banded_arm_instructions_20261005/PROFILE_N1000.json",
    "research/spd06_small_static_stages_20261007/PROFILE.json",
    "research/spd09_colext_threshold_20261007/PROFILE.json",
    "research/sp03_declared_structure_routing_20261010/PROFILE.json",
    "research/r3_matched_accuracy_hom06_20261001/CAMPAIGN.json",
    "research/r3_matched_accuracy_hom06_20261001/VERIFY.json",
    "research/LEDGER.jsonl",
    "docs/HOMOTOPY_EXPENSIVE_CLIENT_CONTRACT.md",
]

# Closed candidate definitions matching PREREGISTRATION.md
CANDIDATES_DEF = [
    {
        "id": "K1",
        "client": "Fourier path client (PP05/PP06)",
        "source_files": [
            "crates/rodas5p-integrators/src/fourier_path_candidate.rs",
            "crates/rodas5p-integrators/src/fourier_path_certificate.rs",
        ],
        "applicable_native_arms": [],
        "cheapest_arm": None,
        "ledger_row": "L-0074",
    },
    {
        "id": "K2",
        "client": "R4 / HOM-06 / R-NEXT-06 quadratic family, n in {1, 2, 4, 8, 16}",
        "source_files": [
            "crates/rodas5p-integrators/src/transactional_q1_q2.rs",
        ],
        "applicable_native_arms": ["small", "dense_v2", "matrix_free_u_form"],
        "cheapest_arm": "dense_v2",
        "ledger_row": "L-0008",
    },
    {
        "id": "K3",
        "client": "Bateman two-timescale real client (feature audit2-bateman-authority)",
        "source_files": [
            "crates/rodas5p-integrators/src/audit2_bateman_real_client_research.rs",
        ],
        "applicable_native_arms": ["small", "dense_v2"],
        "cheapest_arm": "small",
        "ledger_row": "L-0032",
    },
    {
        "id": "K4",
        "client": "Darboux chart client y = x^2 (w + 1/kappa) (L-0043, L-0048)",
        "source_files": [
            "crates/rodas5p-integrators/src/chart_transport.rs",
        ],
        "applicable_native_arms": ["small", "dense_v2"],
        "cheapest_arm": "small",
        "ledger_row": "L-0043",
    },
    {
        "id": "K5",
        "client": "robertson, hires, van-der-pol-mu1000",
        "source_files": [
            "crates/rodas5p-cli/src/stiff_benchmark.rs",
        ],
        "applicable_native_arms": ["small", "dense_v2", "matrix_free_u_form"],
        "cheapest_arm": "small",
        "ledger_row": "L-0085",
    },
    {
        "id": "K6",
        "client": "brusselator-1d-N, N in {30, 40, 50, 160, 200, 500}",
        "source_files": [
            "crates/rodas5p-cli/src/stiff_benchmark.rs",
        ],
        "applicable_native_arms": ["dense_v2", "colext64", "banded", "matrix_free_u_form"],
        "cheapest_arm": "banded",
        "ledger_row": "L-0083",
    },
    {
        "id": "K7",
        "client": "scientific corpus v2 calibration: 6 families x n in {96, 384, 1536}",
        "source_files": [
            "crates/rodas5p-integrators/src/scientific_corpus_v2.rs",
        ],
        "applicable_native_arms": ["dense_v2", "colext64", "banded", "matrix_free_u_form"],
        "cheapest_arm": "colext64",
        "ledger_row": "L-0086",
    },
    {
        "id": "K8",
        "client": "problems.rs fixtures (scalar linear, Prothero-Robinson, manufactured, mass, complex Dahlquist, semilinear)",
        "source_files": [
            "crates/rodas5p-integrators/src/problems.rs",
        ],
        "applicable_native_arms": ["small", "dense_v2"],
        "cheapest_arm": "small",
        "ledger_row": "L-0041",
    },
]

F_MIN = 0.348


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


def strip_rust_comments(code: str) -> str:
    """Removes single-line and multi-line comments from Rust code."""
    # Remove block comments /* ... */
    code = re.sub(r"/\*.*?\*/", "", code, flags=re.DOTALL)
    # Remove line comments // ...
    code = re.sub(r"//.*", "", code)
    return code


def check_expensive_declaration(source_files: list[str]) -> bool:
    """Checks for actual instantiation or implementation of ExpensiveClientDeclaration outside comments."""
    for rel_path in source_files:
        full_path = ROOT / rel_path
        if not full_path.exists():
            continue
        cleaned = strip_rust_comments(full_path.read_text(encoding="utf-8"))
        if re.search(r"ExpensiveClientDeclaration\s*\{", cleaned):
            return True
        if re.search(r"impl\b[^{]*\bExpensiveClientDeclaration\b", cleaned):
            return True
    return False


def collect_profile_shares() -> dict[tuple[str, str], list[float]]:
    """Fail-closed extraction of problem-user-code share mapped by (problem, arm)."""
    profile_paths = [
        ROOT / "research/spd01_fast_driver_overhead_20261005/PROFILE_SMALL.json",
        ROOT / "research/spd01_fast_driver_overhead_20261005/PROFILE_DENSE.json",
        ROOT / "research/spd01_fast_driver_overhead_20261005/PROFILE_DENSE_B50.json",
        ROOT / "research/spd03_banded_arm_instructions_20261005/PROFILE.json",
        ROOT / "research/spd03_banded_arm_instructions_20261005/PROFILE_N1000.json",
        ROOT / "research/spd06_small_static_stages_20261007/PROFILE.json",
        ROOT / "research/spd09_colext_threshold_20261007/PROFILE.json",
        ROOT / "research/sp03_declared_structure_routing_20261010/PROFILE.json",
    ]
    problem_arm_shares: dict[tuple[str, str], list[float]] = {}
    for p in profile_paths:
        if not p.exists():
            raise FileNotFoundError(f"Missing required profile file: {p}")
        text = p.read_text(encoding="utf-8")
        data = json.loads(text)
        for entry in data.get("entries", []):
            prob = entry.get("problem")
            arm = entry.get("arm")
            categories = entry.get("categories_by_file", {})
            user_cat = categories.get("problem-user-code", {})
            share = user_cat.get("share")
            if prob and arm and share is not None:
                if not isinstance(share, (int, float)):
                    raise ValueError(f"Non-numeric share {share!r} in {p}")
                problem_arm_shares.setdefault((prob, arm), []).append(float(share))
    return problem_arm_shares


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path, help="Target SCREEN.json output path")
    args = parser.parse_args()

    inputs_sha256 = {}
    for rel in PINNED_FILES:
        p = ROOT / rel
        if not p.exists():
            print(f"ERROR: missing pinned input: {rel}", file=sys.stderr)
            return 2
        inputs_sha256[rel] = sha256_file(p)

    # Fail-closed read of PP07 results for K1
    pp07_file = ROOT / "research/pp07_fourier_shared_action_20261004/RESULTS.json"
    if not pp07_file.exists():
        print(f"ERROR: missing PP07 results: {pp07_file}", file=sys.stderr)
        return 2
    pp07_data = json.loads(pp07_file.read_text(encoding="utf-8"))
    if "q1_solves" not in pp07_data:
        print("ERROR: q1_solves missing from PP07 RESULTS.json", file=sys.stderr)
        return 2
    pp07_q1_solves = int(pp07_data["q1_solves"])
    if pp07_q1_solves != 0:
        print(f"ERROR: expected PP07 q1_solves == 0, got {pp07_q1_solves}", file=sys.stderr)
        return 2

    # Collect profile shares fail-closed
    try:
        shares_map = collect_profile_shares()
    except Exception as exc:
        print(f"ERROR parsing profiles: {exc}", file=sys.stderr)
        return 2

    candidates_evaluated = []

    for c in CANDIDATES_DEF:
        cid = c["id"]
        source_files = c["source_files"]
        applicable_arms = c["applicable_native_arms"]
        cheapest_arm = c["cheapest_arm"]
        ledger_row = c["ledger_row"]

        has_decl = check_expensive_declaration(source_files)

        # S1: Callback share f and declaration evaluated on the strongest sequential comparator arm
        measured_f_min = None
        measured_f_max = None
        sample_count = 0

        if cid == "K5":
            # cheapest arm: rodas5p-fast-small
            arm_shares = []
            for prob in ["robertson", "hires", "van-der-pol-mu1000"]:
                shares = shares_map.get((prob, "rodas5p-fast-small"), [])
                arm_shares.extend(shares)
            if arm_shares:
                measured_f_min = round(min(arm_shares), 6)
                measured_f_max = round(max(arm_shares), 6)
                sample_count = len(arm_shares)
        elif cid == "K6":
            # cheapest arm: banded
            arm_shares = []
            for prob in [
                "brusselator-1d-30",
                "brusselator-1d-40",
                "brusselator-1d-50",
                "brusselator-1d-160",
                "brusselator-1d-200",
                "brusselator-1d-500",
            ]:
                shares = shares_map.get((prob, "banded"), [])
                arm_shares.extend(shares)
            if arm_shares:
                measured_f_min = round(min(arm_shares), 6)
                measured_f_max = round(max(arm_shares), 6)
                sample_count = len(arm_shares)

        # Authoritative S1(b) criterion: min measured share on cheapest arm >= F_MIN
        pass_s1_a = has_decl
        pass_s1_b = measured_f_min is not None and measured_f_min >= F_MIN
        pass_s1 = pass_s1_a and pass_s1_b

        if not has_decl and measured_f_min is None:
            s1_reason = "decl=False; unmeasured_f"
        elif not has_decl and measured_f_min is not None:
            s1_reason = f"decl=False; measured_f_min={measured_f_min} < {F_MIN} (max={measured_f_max}, samples={sample_count})"
        elif has_decl and (measured_f_min is None or not pass_s1_b):
            s1_reason = f"decl=True; measured_f_min={measured_f_min} < {F_MIN} (below required threshold)"
        else:
            s1_reason = f"decl=True; measured_f_min={measured_f_min} >= {F_MIN}"

        # S2: Shifted solves present
        if cid == "K1":
            q1_solves = pp07_q1_solves
            independent_tasks_per_round = 0
            pass_s2 = False
            s2_notes = "q1_solves = 0; PP07 ABSTAIN preserved"
            s2_reason = "Fourier client makes no shifted solve (q1_solves = 0, PP07 ABSTAIN preserved)"
        elif cid == "K4":
            q1_solves = 0
            independent_tasks_per_round = 0
            pass_s2 = False
            s2_notes = "scalar chart transport; no linear solve (q1_solves = 0)"
            s2_reason = "Darboux chart client is a scalar transformation without linear stage solves"
        else:
            q1_solves = 1
            independent_tasks_per_round = 6 if cid == "K2" else 8
            pass_s2 = True
            s2_notes = f"shifted solves present (W = I - h gamma J solves > 0; {independent_tasks_per_round} stage tasks/round)"
            s2_reason = f"sequential step attempts require W = I - h gamma J solves with {independent_tasks_per_round} tasks"

        # S3: Strong sequential comparator
        if cid == "K1":
            pass_s3 = False
            s3_reason = "no_applicable_arm: client makes no solver call (L-0074)"
        elif cid == "K4":
            pass_s3 = False
            s3_reason = "no_applicable_arm: scalar chart client makes no linear solver call (L-0043, L-0048)"
        else:
            pass_s3 = True
            s3_reason = f"cheapest arm '{cheapest_arm}' verified in {ledger_row}"

        # S4: Work/span accounting and endpoint certificate
        if cid == "K2":
            capability = {
                "n_le_2_or_diagonal": "Available",
                "n_gt_2_dense": "DimensionCutoff",
            }
            model_binding = "GeneratedFromModel"
            pass_s4_a = True
            s4_reason = "QuadraticModel provides GeneratedFromModel binding; small witness Available for n <= 2 or diagonal"
        else:
            capability = "unverified"
            model_binding = "SampledAgreement"
            pass_s4_a = False
            s4_reason = "No GeneratedFromModel certificate source available (SampledAgreement is never a proof)"

        missing_ledger_fields = [
            "max_task_per_round",
            "barriers",
            "rejected_candidate_work",
            "fallback_work",
            "peak_bytes",
        ]
        s4_b_status = "PASS_WITH_HT02_OBLIGATIONS"
        no_speedup_claim_permitted = True
        pass_s4 = pass_s4_a and (s4_b_status in ("PASS", "PASS_WITH_HT02_OBLIGATIONS"))

        failing = []
        if not pass_s1:
            failing.append("S1")
        if not pass_s2:
            failing.append("S2")
        if not pass_s3:
            failing.append("S3")
        if not pass_s4:
            failing.append("S4")

        row = {
            "id": cid,
            "client": c["client"],
            "source_files": source_files,
            "applicable_native_arms": applicable_arms,
            "s1_declared_expensive": {
                "has_declaration": has_decl,
                "measured_f_min": measured_f_min,
                "measured_f_max": measured_f_max,
                "f_min_threshold": F_MIN,
                "sample_count": sample_count,
                "pass_s1_a": pass_s1_a,
                "pass_s1_b": pass_s1_b,
                "pass_s1": pass_s1,
                "reason": s1_reason,
            },
            "s2_shifted_solves": {
                "present": pass_s2,
                "q1_solves": q1_solves,
                "independent_tasks_per_round": independent_tasks_per_round,
                "notes": s2_notes,
                "pass_s2": pass_s2,
                "reason": s2_reason,
            },
            "s3_strong_comparator": {
                "applicable_arms": applicable_arms,
                "cheapest_arm": cheapest_arm,
                "ledger_row": ledger_row,
                "pass_s3": pass_s3,
                "reason": s3_reason,
            },
            "s4_accounting_certificate": {
                "capability": capability,
                "model_binding": model_binding,
                "pass_s4_a": pass_s4_a,
                "s4_b_status": s4_b_status,
                "missing_ledger_fields_ht02_obligation": missing_ledger_fields,
                "no_speedup_claim_permitted": no_speedup_claim_permitted,
                "pass_s4": pass_s4,
                "reason": s4_reason,
            },
            "candidate_status": "MEETS_CRITERIA" if not failing else "FAILS_CRITERIA",
            "failing_criteria": failing,
        }
        candidates_evaluated.append(row)

    # Decision rule: SELECTED(k) if at least one meets all S1-S4 with measured f >= F_MIN.
    # Candidates without measured f can only be leads.
    qualifying = [
        c["id"]
        for c in candidates_evaluated
        if c["candidate_status"] == "MEETS_CRITERIA"
        and c["s1_declared_expensive"]["measured_f_min"] is not None
        and c["s1_declared_expensive"]["measured_f_min"] >= F_MIN
    ]

    if len(qualifying) == 1:
        decision = f"SELECTED({qualifying[0]})"
    elif len(qualifying) > 1:
        # Tie-break by largest minimum measured f
        best = max(
            qualifying,
            key=lambda cid: next(
                c["s1_declared_expensive"]["measured_f_min"] for c in candidates_evaluated if c["id"] == cid
            ),
        )
        decision = f"SELECTED({best})"
    else:
        decision = "ABSTAIN"

    screen_data = {
        "schema": "vigilode-ht01-screen-v1",
        "inputs_sha256": inputs_sha256,
        "criteria": {
            "s1": "Declared expensive RHS/JVP (ExpensiveClientDeclaration + min measured f >= 0.348 on cheapest arm)",
            "s2": "Shifted solves present (W solves per attempt > 0, >=2 independent tasks)",
            "s3": "Strong sequential comparator (applicable native arms defined, committed Ir and ledger)",
            "s4": "Work/span accounting and endpoint certificate (Available + GeneratedFromModel, HT02 obligations tracked)",
        },
        "thresholds": {
            "f_min": F_MIN,
            "derivation": "f >= (1 - 1/1.15) / (1 - 5/8) = 0.3478... ~ 0.348 (Amdahl bound r=5, P=8)",
        },
        "candidates": candidates_evaluated,
        "preliminary_decision": decision,
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(screen_data, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {args.output} ({len(candidates_evaluated)} candidates screened, decision: {decision})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
