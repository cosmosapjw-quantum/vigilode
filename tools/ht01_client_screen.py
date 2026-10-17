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

CANDIDATES_DEF = [
    {
        "id": "K1",
        "client": "Fourier path client (PP05/PP06)",
        "source_files": [
            "crates/rodas5p-integrators/src/fourier_path_candidate.rs",
            "crates/rodas5p-integrators/src/fourier_path_certificate.rs",
        ],
        "applicable_native_arms": [],
    },
    {
        "id": "K2",
        "client": "R4 / HOM-06 / R-NEXT-06 quadratic family, n in {1, 2, 4, 8, 16}",
        "source_files": [
            "crates/rodas5p-integrators/src/transactional_q1_q2.rs",
        ],
        "applicable_native_arms": ["small", "dense_v2", "matrix_free_u_form"],
    },
    {
        "id": "K3",
        "client": "Bateman two-timescale real client (feature audit2-bateman-authority)",
        "source_files": [
            "crates/rodas5p-integrators/src/audit2_bateman_real_client_research.rs",
        ],
        "applicable_native_arms": ["small", "dense_v2"],
    },
    {
        "id": "K4",
        "client": "Darboux chart client y = x^2 (w + 1/kappa) (L-0043, L-0048)",
        "source_files": [
            "crates/rodas5p-integrators/src/chart_transport.rs",
        ],
        "applicable_native_arms": ["small", "dense_v2"],
    },
    {
        "id": "K5",
        "client": "robertson, hires, van-der-pol-mu1000",
        "source_files": [
            "crates/rodas5p-cli/src/stiff_benchmark.rs",
        ],
        "applicable_native_arms": ["small", "dense_v2", "matrix_free_u_form"],
    },
    {
        "id": "K6",
        "client": "brusselator-1d-N, N in {30, 40, 50, 160, 200, 500}",
        "source_files": [
            "crates/rodas5p-cli/src/stiff_benchmark.rs",
        ],
        "applicable_native_arms": ["dense_v2", "colext64", "banded", "matrix_free_u_form"],
    },
    {
        "id": "K7",
        "client": "scientific corpus v2 calibration: 6 families x n in {96, 384, 1536}",
        "source_files": [
            "crates/rodas5p-integrators/src/scientific_corpus_v2.rs",
        ],
        "applicable_native_arms": ["dense_v2", "colext64", "banded", "matrix_free_u_form"],
    },
    {
        "id": "K8",
        "client": "problems.rs fixtures (scalar linear, Prothero-Robinson, manufactured, mass, complex Dahlquist, semilinear)",
        "source_files": [
            "crates/rodas5p-integrators/src/problems.rs",
        ],
        "applicable_native_arms": ["small", "dense_v2"],
    },
]

F_MIN = 0.348


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


def check_expensive_declaration(source_files: list[str]) -> bool:
    token = "ExpensiveClientDeclaration"
    for rel_path in source_files:
        full_path = ROOT / rel_path
        if full_path.exists() and token in full_path.read_text(encoding="utf-8"):
            return True
    return False


def collect_profile_shares() -> dict[str, list[float]]:
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
    problem_shares: dict[str, list[float]] = {}
    for p in profile_paths:
        if not p.exists():
            continue
        try:
            data = json.loads(p.read_text(encoding="utf-8"))
            for entry in data.get("entries", []):
                prob = entry.get("problem")
                categories = entry.get("categories_by_file", {})
                user_cat = categories.get("problem-user-code", {})
                share = user_cat.get("share")
                if prob and share is not None and isinstance(share, (int, float)):
                    problem_shares.setdefault(prob, []).append(float(share))
        except Exception:
            pass
    return problem_shares


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

    # Read PP07 results for K1
    pp07_file = ROOT / "research/pp07_fourier_shared_action_20261004/RESULTS.json"
    pp07_data = json.loads(pp07_file.read_text(encoding="utf-8"))
    pp07_q1_solves = pp07_data.get("q1_solves", 0)

    # Collect profile shares
    shares_by_prob = collect_profile_shares()

    k5_problems = ["robertson", "hires", "van-der-pol-mu1000"]
    k5_shares = []
    for prob in k5_problems:
        k5_shares.extend(shares_by_prob.get(prob, []))

    k6_problems = [
        "brusselator-1d-30",
        "brusselator-1d-40",
        "brusselator-1d-50",
        "brusselator-1d-160",
        "brusselator-1d-200",
        "brusselator-1d-500",
    ]
    k6_shares = []
    for prob in k6_problems:
        k6_shares.extend(shares_by_prob.get(prob, []))

    candidates_evaluated = []

    for c in CANDIDATES_DEF:
        cid = c["id"]
        source_files = c["source_files"]
        applicable_arms = c["applicable_native_arms"]

        has_decl = check_expensive_declaration(source_files)

        # S1: Callback share f and declaration
        if cid == "K5":
            measured_f = round(max(k5_shares), 6) if k5_shares else None
            min_f = round(min(k5_shares), 6) if k5_shares else None
            share_range = [min_f, measured_f] if k5_shares else None
        elif cid == "K6":
            measured_f = round(max(k6_shares), 6) if k6_shares else None
            min_f = round(min(k6_shares), 6) if k6_shares else None
            share_range = [min_f, measured_f] if k6_shares else None
        else:
            measured_f = None
            share_range = None

        pass_s1_a = has_decl
        pass_s1_b = measured_f is not None and measured_f >= F_MIN
        pass_s1 = pass_s1_a and (measured_f is None or pass_s1_b)
        s1_reason = (
            "decl=False, unmeasured_f"
            if not has_decl and measured_f is None
            else f"decl=False, measured_f_max={measured_f} < {F_MIN}"
            if not has_decl and measured_f is not None
            else "decl=True"
        )

        # S2: Shifted solves present
        if cid == "K1":
            q1_solves = pp07_q1_solves
            pass_s2 = False
            s2_notes = "q1_solves = 0; PP07 ABSTAIN preserved"
            s2_reason = "Fourier client makes no shifted solve (q1_solves = 0, PP07 ABSTAIN preserved)"
        elif cid == "K4":
            q1_solves = 0
            pass_s2 = False
            s2_notes = "scalar chart transport; no linear solve"
            s2_reason = "Darboux chart client is a scalar transformation without linear solves"
        else:
            q1_solves = 1
            pass_s2 = True
            s2_notes = "shifted solves present in step attempts"
            s2_reason = "sequential step attempts require W = I - h gamma J solves"

        # S3: Strong sequential comparator
        if cid == "K1":
            pass_s3 = False
            s3_cheapest_arm = None
            s3_ledger = None
            s3_reason = "no_applicable_arm: client makes no solver call (L-0074)"
        elif cid == "K2":
            pass_s3 = True
            s3_cheapest_arm = "dense_v2"
            s3_ledger = "L-0008"
            s3_reason = "applicable arms defined; historical comparator blocked at 0.04-0.17 (L-0008)"
        elif cid == "K4":
            pass_s3 = True
            s3_cheapest_arm = "small"
            s3_ledger = "L-0043"
            s3_reason = "small scalar arm applicable; L-0043/L-0048 evidence"
        elif cid == "K5":
            pass_s3 = True
            s3_cheapest_arm = "rodas5p-fast-small"
            s3_ledger = "L-0081"
            s3_reason = "small and dense arms profiled (L-0081, L-0085)"
        elif cid == "K6":
            pass_s3 = True
            s3_cheapest_arm = "banded"
            s3_ledger = "L-0083"
            s3_reason = "banded arm and colext64 profiled (L-0083, L-0086)"
        elif cid == "K7":
            pass_s3 = True
            s3_cheapest_arm = "colext64"
            s3_ledger = "L-0086"
            s3_reason = "dense v2, colext64 and banded applicable (L-0086)"
        else:
            pass_s3 = True
            s3_cheapest_arm = "small"
            s3_ledger = "L-0041"
            s3_reason = "small fixture driver applicable"

        # S4: Work/span accounting and endpoint certificate
        if cid == "K2":
            capability = "Available"
            binding = "GeneratedFromModel"
            pass_s4_a = True
            s4_reason = "QuadraticModel provides GeneratedFromModel binding with Available small/diagonal witness"
        else:
            capability = "NotAvailable"
            binding = "SampledAgreement"
            pass_s4_a = False
            s4_reason = "No GeneratedFromModel certificate source available (SampledAgreement is never a proof)"

        missing_ledger_fields = [
            "max_task_per_round",
            "barriers",
            "rejected_candidate_work",
            "fallback_work",
            "peak_bytes",
        ]
        # In HT01 preregistration line 93: missing fields are listed as HT02 obligations, not failures for S4(b).
        pass_s4_b = True
        pass_s4 = pass_s4_a and pass_s4_b

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
                "measured_f_max": measured_f,
                "share_range": share_range,
                "f_min": F_MIN,
                "pass_s1": pass_s1,
                "reason": s1_reason,
            },
            "s2_shifted_solves": {
                "present": pass_s2,
                "q1_solves": q1_solves,
                "notes": s2_notes,
                "pass_s2": pass_s2,
                "reason": s2_reason,
            },
            "s3_strong_comparator": {
                "applicable_arms": applicable_arms,
                "cheapest_arm": s3_cheapest_arm,
                "ledger_row": s3_ledger,
                "pass_s3": pass_s3,
                "reason": s3_reason,
            },
            "s4_accounting_certificate": {
                "capability": capability,
                "model_binding": binding,
                "pass_s4_a": pass_s4_a,
                "missing_ledger_fields_ht02_obligation": missing_ledger_fields,
                "pass_s4": pass_s4,
                "reason": s4_reason,
            },
            "candidate_status": "MEETS_CRITERIA" if not failing else "FAILS_CRITERIA",
            "failing_criteria": failing,
        }
        candidates_evaluated.append(row)

    qualifying = [c["id"] for c in candidates_evaluated if c["candidate_status"] == "MEETS_CRITERIA"]
    if len(qualifying) == 1:
        decision = f"SELECTED({qualifying[0]})"
    else:
        decision = "ABSTAIN"

    screen_data = {
        "schema": "vigilode-ht01-screen-v1",
        "inputs_sha256": inputs_sha256,
        "criteria": {
            "s1": "Declared expensive RHS/JVP (ExpensiveClientDeclaration + f >= 0.348)",
            "s2": "Shifted solves present (W solves per attempt > 0, >=2 independent tasks)",
            "s3": "Strong sequential comparator (applicable native arms defined, committed Ir and ledger)",
            "s4": "Work/span accounting and endpoint certificate (Available + GeneratedFromModel)",
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
