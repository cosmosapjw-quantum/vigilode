#!/usr/bin/env python3
"""Stage-1 admission gate checker for Laguerre adjoint bound (PY02).

Research node: research/py02_laguerre_adjoint_bound_20261011
Preregistration: research/py02_laguerre_adjoint_bound_20261011/PREREGISTRATION.md

Stage 1 evaluates three gates:
- A1 Dominance: In every one of PY01's 6 gated cases, recurrence_adjoint share >= 0.9.
- A2 Closability: In at least 1 of PY01's 6 gated cases, closability floor F <= budget.
- A3 Work feasibility: In at least 8 of the 32 stage-2 cases, Laguerre plan needs strictly
  fewer vector products than Chebyshev alone.

If any gate fails, the verdict is ABSTAIN with the recorded evidence and exit code 0.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess
import sys
import time
from typing import Any

import mpmath as mp

# Fail-closed evidence schema validation (re-audit AS03)
TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))
from evidence_schema_v2 import HEX64, INVALID_EXIT_CODE, ValidationError, decode_hex64, loads_strict

SCHEMA = "vigilode-py02-laguerre-admission-v1"
PLANS_SCHEMA = "vigilode-py02-laguerre-plans-v1"
PY01_SCHEMA = "vigilode-py01-laguerre-results-v1"
NODE = "research/py02_laguerre_adjoint_bound_20261011"
PY01_NODE = "research/py01_laguerre_component_export_20261011"

GATED_PY01_LABELS = (
    "n6-rho400-h0.1-distinct",
    "n6-rho400-h0.1-same",
    "n12-rho400-h0.1-distinct",
    "n12-rho400-h0.1-same",
    "n20-rho400-h0.1-distinct",
    "n20-rho400-h0.1-same",
)

A1_SHARE_THRESHOLD = 0.9
A2_CLOSABILITY_COUNT_MIN = 1
A3_FEWER_COUNT_MIN = 8
EXPECTED_STAGE2_CASE_COUNT = 32

STAGE2_VALID_N = {7, 14}
STAGE2_VALID_RHO = {100.0, 300.0}
STAGE2_VALID_H = {0.08, 0.16}
STAGE2_VALID_BUDGET_FACTORS = {1.0e-6, 1.0e-8}


def unhex(text: str) -> float:
    return struct.unpack(">d", bytes.fromhex(text))[0]


def phi(k: int, z):
    """phi_k(z) = sum_j z^j / (j + k)!, phi_0 = exp."""
    if abs(z) < mp.mpf("0.5"):
        total, term, j = mp.mpf(0), mp.mpf(1) / mp.factorial(k), 0
        while abs(term) > mp.mpf(10) ** (-mp.mp.dps - 5):
            total += term
            j += 1
            term = term * z / (j + k)
        return total
    value = mp.e ** z
    for m in range(1, k + 1):
        value = (value - 1 / mp.factorial(m - 1)) / z
    return value


def compute_reference_norm_f(case: dict[str, Any], dps: int) -> tuple[float, mp.mpf]:
    with mp.workdps(dps):
        n = case["n"]
        a = mp.matrix([[mp.mpf(unhex(x)) for x in row] for row in case["a"]])
        h_bits = case.get("h_bits")
        h = mp.mpf(unhex(h_bits) if h_bits else case["h"])
        w = [[mp.mpf(unhex(x)) for x in vec] for vec in case["w"]]
        eigenvalues, q = mp.eigsy(a)
        result = [mp.mpf(0)] * n
        for k in range(5):
            if all(x == 0.0 for x in w[k]):
                continue
            coords = [mp.fsum(q[i, j] * w[k][i] for i in range(n)) for j in range(n)]
            scaled = [phi(k, h * eigenvalues[j]) * coords[j] for j in range(n)]
            for i in range(n):
                result[i] += mp.fsum(q[i, j] * scaled[j] for j in range(n))
        norm_f = mp.sqrt(mp.fsum(r ** 2 for r in result))
        return float(norm_f), norm_f


def run_check(
    py01_path: Path,
    plans_path: Path,
    output_path: Path | None = None,
    dps: int = 50,
    verify_references: bool = True,
    verify_checker_blob: bool = True,
) -> int:
    t0 = time.time()
    errors: list[str] = []

    if not py01_path.exists():
        print(f"INVALID: PY01 results file not found: {py01_path}", file=sys.stderr)
        return INVALID_EXIT_CODE

    if not plans_path.exists():
        print(f"INVALID: PLANS file not found: {plans_path}", file=sys.stderr)
        return INVALID_EXIT_CODE

    try:
        py01_bytes = py01_path.read_bytes()
        py01_data = loads_strict(py01_bytes)
    except Exception as exc:
        print(f"INVALID: failed strict parsing of PY01 results: {exc}", file=sys.stderr)
        return INVALID_EXIT_CODE

    try:
        plans_bytes = plans_path.read_bytes()
        plans_data = loads_strict(plans_bytes)
    except Exception as exc:
        print(f"INVALID: failed strict parsing of PLANS: {exc}", file=sys.stderr)
        return INVALID_EXIT_CODE

    # Validate PLANS schema and metadata
    if plans_data.get("schema") != PLANS_SCHEMA:
        errors.append(f"plans schema mismatch: expected {PLANS_SCHEMA}, got {plans_data.get('schema')}")
    if plans_data.get("node") != NODE:
        errors.append(f"plans node mismatch: expected {NODE}, got {plans_data.get('node')}")
    if not plans_data.get("tree_clean", False):
        errors.append(f"plans recorded dirty tree: tree_clean={plans_data.get('tree_clean')}")

    source_commit = plans_data.get("source_commit")
    if not source_commit or not isinstance(source_commit, str):
        errors.append("plans missing valid source_commit")
    elif verify_checker_blob:
        try:
            repo_root = ROOT
            checker_rel = Path(__file__).resolve().relative_to(repo_root)
            proc = subprocess.run(
                ["git", "cat-file", "-p", f"{source_commit}:{checker_rel}"],
                cwd=repo_root,
                capture_output=True,
                text=True,
            )
            if proc.returncode == 0:
                current_text = Path(__file__).resolve().read_text(encoding="utf-8")
                if proc.stdout.strip() != current_text.strip():
                    errors.append(f"checker blob at source_commit {source_commit} differs from executed checker")
        except Exception as exc:
            errors.append(f"failed verifying checker blob: {exc}")

    # Validate PY01 schema and metadata
    if py01_data.get("schema") != PY01_SCHEMA:
        errors.append(f"py01 schema mismatch: expected {PY01_SCHEMA}, got {py01_data.get('schema')}")
    if py01_data.get("node") != PY01_NODE:
        errors.append(f"py01 node mismatch: expected {PY01_NODE}, got {py01_data.get('node')}")
    if not py01_data.get("tree_clean", False):
        errors.append(f"py01 recorded dirty tree: tree_clean={py01_data.get('tree_clean')}")

    cases = plans_data.get("cases")
    if not isinstance(cases, list) or len(cases) != EXPECTED_STAGE2_CASE_COUNT:
        errors.append(
            f"expected exactly {EXPECTED_STAGE2_CASE_COUNT} cases in plans, got {len(cases) if isinstance(cases, list) else type(cases)}"
        )

    if errors:
        print("INVALID evidence violations:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return INVALID_EXIT_CODE

    # Check PY01 completeness
    py01_verdict = py01_data.get("verdict")
    py01_gate1 = py01_data.get("gate", {}).get("1_completeness", False)
    if py01_verdict != "PASS" or not py01_gate1:
        admission_doc = {
            "schema": SCHEMA,
            "node": NODE,
            "verdict": "ABSTAIN",
            "decision": "ABSTAIN (no component evidence)",
            "stage2_admitted": False,
            "first_failed_gate": "no component evidence",
            "source_commit": source_commit,
            "tree_clean": True,
            "command": f"python3 tools/py02_laguerre_admission.py --py01 {py01_path} --plans {plans_path}"
            + (f" --output {output_path}" if output_path else ""),
            "runtime_seconds": round(time.time() - t0, 3),
            "inputs": {
                "py01": str(py01_path),
                "py01_sha256": hashlib.sha256(py01_bytes).hexdigest(),
                "plans": str(plans_path),
                "plans_sha256": hashlib.sha256(plans_bytes).hexdigest(),
            },
            "gate": {
                "A1_dominance": None,
                "A2_closability": None,
                "A3_work_feasibility": None,
            },
        }
        if output_path:
            output_path.parent.mkdir(parents=True, exist_ok=True)
            output_path.write_text(json.dumps(admission_doc, indent=2) + "\n")
            print(f"Wrote admission document to {output_path}")
        return 0

    # Evaluate A1 (Dominance)
    py01_cases = {c["label"]: c for c in py01_data.get("reported", {}).get("cases", [])}
    a1_cases = []
    a1_passed = True
    min_share = float("inf")

    for label in GATED_PY01_LABELS:
        if label not in py01_cases:
            print(f"INVALID: missing gated case {label} in PY01 reported cases", file=sys.stderr)
            return INVALID_EXIT_CODE
        c = py01_cases[label]
        share_val = float(c["shares"]["recurrence_adjoint"])
        min_share = min(min_share, share_val)
        meets = share_val >= A1_SHARE_THRESHOLD
        if not meets:
            a1_passed = False
        a1_cases.append({
            "label": label,
            "recurrence_adjoint_share": share_val,
            "meets_threshold": meets,
        })

    # Evaluate A2 (Closability)
    a2_cases = []
    meeting_closability_count = 0
    for label in GATED_PY01_LABELS:
        c = py01_cases[label]
        floor_f = float(c["closability_floor_F"])
        budget = float(c["budget"])
        meets = floor_f <= budget
        if meets:
            meeting_closability_count += 1
        a2_cases.append({
            "label": label,
            "closability_floor_F": floor_f,
            "budget": budget,
            "meets_budget": meets,
        })
    a2_passed = meeting_closability_count >= A2_CLOSABILITY_COUNT_MIN

    # Validate 32 stage-2 cases and Evaluate A3 (Work Feasibility)
    seen_labels = set()
    a3_cases = []
    laguerre_fewer_count = 0

    for case in cases:
        label = case.get("label")
        if not label or label in seen_labels:
            errors.append(f"invalid or duplicate case label in plans: {label}")
        seen_labels.add(label)

        n = case.get("n")
        rho = case.get("rho")
        h = case.get("h")
        budget_factor = case.get("budget_factor")
        norm_f = case.get("norm_f")
        total_budget = case.get("total_budget")
        truncation_budget = case.get("truncation_budget")

        if n not in STAGE2_VALID_N:
            errors.append(f"case {label}: n={n} not in {STAGE2_VALID_N}")
        if rho not in STAGE2_VALID_RHO:
            errors.append(f"case {label}: rho={rho} not in {STAGE2_VALID_RHO}")
        if h not in STAGE2_VALID_H:
            errors.append(f"case {label}: h={h} not in {STAGE2_VALID_H}")
        if budget_factor not in STAGE2_VALID_BUDGET_FACTORS:
            errors.append(f"case {label}: budget_factor={budget_factor} not in {STAGE2_VALID_BUDGET_FACTORS}")

        if verify_references:
            norm_f_50_flt, norm_f_50_mp = compute_reference_norm_f(case, dps=50)
            norm_f_70_flt, norm_f_70_mp = compute_reference_norm_f(case, dps=70)
            diff_50_70 = abs(norm_f_70_mp - norm_f_50_mp)
            if diff_50_70 > mp.mpf("1e-40") * norm_f_70_mp:
                errors.append(f"case {label}: 50- and 70-digit references differ by > 1e-40 ||F||: diff={diff_50_70}")

            if norm_f is None or not math.isfinite(norm_f):
                errors.append(f"case {label}: recorded norm_f is non-finite: {norm_f}")
            else:
                rel_err = abs(norm_f - norm_f_50_flt) / max(norm_f_50_flt, 1.0e-300)
                if rel_err > 1.0e-12:
                    errors.append(f"case {label}: norm_f mismatch with 50-digit EVD: rel_err={rel_err}")

            if total_budget is None or not math.isfinite(total_budget):
                errors.append(f"case {label}: invalid total_budget: {total_budget}")
            else:
                expected_total = budget_factor * max(norm_f, 1.0)
                if abs(total_budget - expected_total) > 1.0e-12 * expected_total:
                    errors.append(f"case {label}: total_budget {total_budget} != expected {expected_total}")

            if truncation_budget is None or not math.isfinite(truncation_budget):
                errors.append(f"case {label}: invalid truncation_budget: {truncation_budget}")
            else:
                expected_trunc = 0.25 * total_budget
                if abs(truncation_budget - expected_trunc) > 1.0e-12 * expected_trunc:
                    errors.append(f"case {label}: truncation_budget {truncation_budget} != expected {expected_trunc}")

        cheb_plan = case.get("chebyshev_plan", {})
        lag_plan = case.get("laguerre_plan", {})
        cheb_vp = cheb_plan.get("vector_products")
        lag_vp = lag_plan.get("vector_products")

        if not isinstance(cheb_vp, int) or not isinstance(lag_vp, int):
            errors.append(f"case {label}: missing or invalid vector_products in plans")
            fewer = False
        else:
            fewer = lag_vp < cheb_vp
            if fewer:
                laguerre_fewer_count += 1

        a3_cases.append({
            "label": label,
            "n": n,
            "rho": rho,
            "h": h,
            "same": case.get("same"),
            "budget_factor": budget_factor,
            "chebyshev_vector_products": cheb_vp,
            "laguerre_vector_products": lag_vp,
            "laguerre_fewer": fewer,
            "norm_f": norm_f,
        })

    if errors:
        print("INVALID plans evidence violations:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return INVALID_EXIT_CODE

    a3_passed = laguerre_fewer_count >= A3_FEWER_COUNT_MIN

    # Determine gate decision
    if not a1_passed:
        verdict = "ABSTAIN"
        decision = "ABSTAIN (A1)"
        first_failed = "A1"
        stage2_admitted = False
    elif not a2_passed:
        verdict = "ABSTAIN"
        decision = "ABSTAIN (A2)"
        first_failed = "A2"
        stage2_admitted = False
    elif not a3_passed:
        verdict = "ABSTAIN"
        decision = "ABSTAIN (A3)"
        first_failed = "A3"
        stage2_admitted = False
    else:
        verdict = "PASS"
        decision = "ADMIT"
        first_failed = None
        stage2_admitted = True

    admission_doc = {
        "schema": SCHEMA,
        "node": NODE,
        "verdict": verdict,
        "decision": decision,
        "stage2_admitted": stage2_admitted,
        "first_failed_gate": first_failed,
        "source_commit": source_commit,
        "tree_clean": True,
        "command": f"python3 tools/py02_laguerre_admission.py --py01 {py01_path} --plans {plans_path}"
        + (f" --output {output_path}" if output_path else ""),
        "runtime_seconds": round(time.time() - t0, 3),
        "inputs": {
            "py01": str(py01_path),
            "py01_sha256": hashlib.sha256(py01_bytes).hexdigest(),
            "plans": str(plans_path),
            "plans_sha256": hashlib.sha256(plans_bytes).hexdigest(),
        },
        "gate": {
            "A1_dominance": a1_passed,
            "A2_closability": a2_passed,
            "A3_work_feasibility": a3_passed,
        },
        "a1_dominance": {
            "passed": a1_passed,
            "threshold": A1_SHARE_THRESHOLD,
            "min_share": min_share,
            "cases": a1_cases,
        },
        "a2_closability": {
            "passed": a2_passed,
            "threshold_meeting_cases": A2_CLOSABILITY_COUNT_MIN,
            "meeting_cases_count": meeting_closability_count,
            "cases": a2_cases,
        },
        "a3_work_feasibility": {
            "passed": a3_passed,
            "threshold_fewer_count": A3_FEWER_COUNT_MIN,
            "laguerre_fewer_count": laguerre_fewer_count,
            "total_cases": len(cases),
            "cases": a3_cases,
        },
    }

    if output_path:
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(json.dumps(admission_doc, indent=2) + "\n")
        print(f"Wrote admission document to {output_path}")

    print(
        f"py02_laguerre_admission: {verdict} ({decision}) [A1={a1_passed}, A2={a2_passed}, A3={a3_passed}] in {admission_doc['runtime_seconds']}s"
    )
    return 0


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--py01",
        type=Path,
        default=ROOT / "research/py01_laguerre_component_export_20261011/RESULTS.json",
        help="Path to PY01 RESULTS.json",
    )
    parser.add_argument(
        "--plans",
        type=Path,
        default=ROOT / "research/py02_laguerre_adjoint_bound_20261011/PLANS.json",
        help="Path to stage-1 PLANS.json",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / "research/py02_laguerre_adjoint_bound_20261011/ADMISSION.json",
        help="Path to output ADMISSION.json",
    )
    parser.add_argument(
        "--dps",
        type=int,
        default=50,
        help="Precision for mpmath reference EVD",
    )
    parser.add_argument(
        "--no-verify-references",
        action="store_true",
        help="Skip mpmath reference recomputation",
    )
    args = parser.parse_args()

    exit_code = run_check(
        py01_path=args.py01,
        plans_path=args.plans,
        output_path=args.output,
        dps=args.dps,
        verify_references=not args.no_verify_references,
    )
    sys.exit(exit_code)


if __name__ == "__main__":
    main()
