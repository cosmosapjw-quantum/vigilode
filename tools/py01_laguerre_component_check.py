#!/usr/bin/env python3
"""Independent scientific gate checker for Laguerre component export (PY01).

Research node: research/py01_laguerre_component_export_20261011
Preregistration: research/py01_laguerre_component_export_20261011/PREREGISTRATION.md
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import multiprocessing
import os
from pathlib import Path
import struct
import subprocess
import sys
import time
from fractions import Fraction
from typing import Any

import mpmath

# Import schema validator from AS03
sys.path.insert(0, str(Path(__file__).resolve().parent))
from evidence_schema_v2 import HEX64, INVALID_EXIT_CODE, ValidationError, decode_hex64, loads_strict

SCHEMA = "vigilode-py01-laguerre-results-v1"
NODE = "research/py01_laguerre_component_export_20261011"
REGISTERED_CASES_SHA256 = "700726f0bd2d58d84fbd1e42bf4daa66efd80312057021bc1e69515f28a01b7d"
REGISTERED_PP09_SHA256 = "c50789bc3fab65a4f600a329723d48e422c60f8162a9ff39cec0dbd63e318beb"

PROOF_VERSION = "laguerre-adjoint-bernstein-interval-v1"
ADJOINT_DEPTH = 3
NUM_SAMPLED_POINTS = 4097
EXACT_FMA_FLOOR = 1.0e-289

TARGET_LABELS = [
    "n6-rho400-h0.1-distinct",
    "n6-rho400-h0.1-same",
    "n12-rho400-h0.1-distinct",
    "n12-rho400-h0.1-same",
    "n20-rho400-h0.1-distinct",
    "n20-rho400-h0.1-same",
    "n6-rho400-h0.01-distinct",
    "subnormal-1e-310",
    "large-1e300",
]

COMPONENT_TERMS = ["truncation", "coefficient", "recurrence_adjoint", "summation", "normalization"]


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def next_up(x: float) -> float:
    return math.nextafter(x, float("inf"))


def next_down(x: float) -> float:
    return math.nextafter(x, float("-inf"))


def add_up(a: float, b: float) -> float:
    s = a + b
    if not math.isfinite(s):
        raise ArithmeticError(f"add_up non-finite: {a} + {b}")
    return next_up(s) if Fraction(a) + Fraction(b) > Fraction(s) else s


def mul_up(a: float, b: float) -> float:
    p = a * b
    if not math.isfinite(p):
        raise ArithmeticError(f"mul_up non-finite: {a} * {b}")
    if a == 0.0 or b == 0.0:
        return p
    if abs(p) < EXACT_FMA_FLOOR:
        return next_up(p)
    return next_up(p) if Fraction(a) * Fraction(b) > Fraction(p) else p


def scale_bound_up(bound: float, shift: int) -> float:
    if bound == 0.0 or shift == 0:
        return bound
    exact = Fraction(bound) * (Fraction(2) ** shift)
    s = float(exact)
    if not math.isfinite(s):
        raise ArithmeticError(f"scale_bound_up non-finite: {bound} * 2^{shift}")
    if Fraction(s) < exact:
        s = next_up(s)
    return s


def canonical_identity_sha256(identity: dict[str, Any]) -> str:
    envelope_keys_str = ",".join(identity["envelope_key_digests"])
    canonical = (
        f"{identity['basis']}:{identity['branch']}:{identity['execution']}:{identity['input_kind']}:"
        f"{identity['degree']}:{identity['laguerre_scale']}:{identity['beta']}:{identity['laguerre_extent']}:"
        f"{identity['h']}:{identity['truncation_budget']}:{identity['enclosure']['lambda']}:"
        f"{identity['enclosure']['rho']}:{identity['enclosure']['evidence']}:{identity['operator_fingerprint']}:"
        f"{identity['normalization_shift']}:{identity['adjoint_proof_version']}:{identity['adjoint_depth']}:"
        f"{envelope_keys_str}"
    )
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def canonical_envelope_key_digest(coef_hex_list: list[str], extent_hex: str, depth: int) -> str:
    text = ",".join(f"{c}:{c}" for c in coef_hex_list)
    coef_sha = hashlib.sha256(text.encode("utf-8")).hexdigest()
    canonical = f"{PROOF_VERSION}:{coef_sha}:{extent_hex}:{depth}"
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def _eval_column_worker(args: tuple[list[str], str]) -> list[float]:
    """Sample lower bounds l_j on [0, L'] using mpmath.iv backward recurrence."""
    coef_hex_list, extent_hex = args
    mp = mpmath.mp
    mp.dps = 60
    m = len(coef_hex_list) - 1
    extent = decode_hex64(extent_hex)
    c_iv = [mpmath.iv.mpf(decode_hex64(x)) for x in coef_hex_list]
    alphas = [mpmath.iv.mpf(2 * j + 1) / (j + 1) for j in range(m + 1)]
    inv_j = [mpmath.iv.mpf(1) / (j + 1) for j in range(m + 1)]
    dampings = [mpmath.iv.mpf(j + 1) / (j + 2) for j in range(m + 1)]

    max_lower = [mp.mpf(0) for _ in range(m + 1)]
    for i in range(NUM_SAMPLED_POINTS):
        x = mpmath.iv.mpf(extent * i / (NUM_SAMPLED_POINTS - 1))
        z1 = mpmath.iv.mpf(0)
        z2 = mpmath.iv.mpf(0)
        for j in range(m, -1, -1):
            z = c_iv[j] + (alphas[j] - x * inv_j[j]) * z1 - dampings[j] * z2
            z2 = z1
            z1 = z
            lo = mp.mpf(abs(z).a)
            if lo > max_lower[j]:
                max_lower[j] = lo
    return [float(x) for x in max_lower]


def run_check(
    published_path: Path,
    pp09_path: Path,
    components_path: Path,
    output_path: Path | None = None,
    num_workers: int | None = None,
) -> int:
    started = time.time()
    errors: list[str] = []
    gate_failures: list[str] = []

    # 1. Strict loading & SHA-256 bindings
    if not published_path.exists():
        print(f"INVALID: published cases file missing: {published_path}", file=sys.stderr)
        return INVALID_EXIT_CODE
    published_bytes = published_path.read_bytes()
    published_sha = sha256_bytes(published_bytes)
    if published_sha != REGISTERED_CASES_SHA256:
        print(
            f"INVALID: published cases SHA-256 mismatch: got {published_sha}, expected {REGISTERED_CASES_SHA256}",
            file=sys.stderr,
        )
        return INVALID_EXIT_CODE

    if not pp09_path.exists():
        print(f"INVALID: PP09 results file missing: {pp09_path}", file=sys.stderr)
        return INVALID_EXIT_CODE
    pp09_bytes = pp09_path.read_bytes()
    pp09_sha = sha256_bytes(pp09_bytes)
    if pp09_sha != REGISTERED_PP09_SHA256:
        print(
            f"INVALID: PP09 results SHA-256 mismatch: got {pp09_sha}, expected {REGISTERED_PP09_SHA256}",
            file=sys.stderr,
        )
        return INVALID_EXIT_CODE

    if not components_path.exists():
        print(f"INVALID: components export file missing: {components_path}", file=sys.stderr)
        return INVALID_EXIT_CODE
    components_bytes = components_path.read_bytes()
    components_sha = sha256_bytes(components_bytes)

    try:
        published_data = loads_strict(published_bytes)
        pp09_data = loads_strict(pp09_bytes)
        comp_data = loads_strict(components_bytes)
    except ValidationError as exc:
        print(f"INVALID: strict JSON validation failed: {exc}", file=sys.stderr)
        return INVALID_EXIT_CODE

    # Check source_commit and tree_clean
    if not comp_data.get("tree_clean", False):
        errors.append(f"components export recorded dirty tree: tree_clean={comp_data.get('tree_clean')}")

    source_commit = comp_data.get("source_commit")
    if not source_commit:
        errors.append("components export missing source_commit")
    else:
        # Check checker blob at source commit
        try:
            repo_root = Path(__file__).resolve().parent.parent
            checker_rel = Path(__file__).resolve().relative_to(repo_root)
            proc = subprocess.run(
                ["git", "cat-file", "-p", f"{source_commit}:{checker_rel}"],
                cwd=repo_root,
                capture_output=True,
                text=True,
            )
            if proc.returncode == 0:
                current_text = Path(__file__).resolve().read_text()
                if proc.stdout.strip() != current_text.strip():
                    errors.append(f"checker blob at source_commit {source_commit} differs from executed checker")
        except Exception as exc:
            errors.append(f"failed verifying checker blob: {exc}")

    # Check case set: exactly the 9 registered cases
    comp_cases = comp_data.get("cases", [])
    if len(comp_cases) != len(TARGET_LABELS):
        errors.append(f"expected {len(TARGET_LABELS)} cases, got {len(comp_cases)}")

    comp_labels = [c.get("label") for c in comp_cases]
    if len(set(comp_labels)) != len(comp_labels):
        errors.append("duplicate case labels in components export")
    if set(comp_labels) != set(TARGET_LABELS):
        errors.append(f"case labels do not match registered target cases: diff={set(comp_labels) ^ set(TARGET_LABELS)}")

    if errors:
        print("INVALID evidence violations:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return INVALID_EXIT_CODE

    # Build lookup maps
    published_cases = {c["label"]: c for c in published_data["cases"] if c["label"] in set(TARGET_LABELS)}
    pp09_rows = {r["label"]: r for r in pp09_data.get("g3", {}).get("rows", [])}

    # Validity: Binding checks & identity uniqueness
    seen_identities: set[str] = set()
    for case in comp_cases:
        label = case["label"]
        pub = published_cases.get(label)
        if not pub:
            errors.append(f"case {label} not found in published cases.json")
            continue

        exp = case.get("export")
        if not exp:
            errors.append(f"case {label} missing export object")
            continue

        identity = exp.get("identity")
        if not identity:
            errors.append(f"case {label} missing identity in export")
            continue

        stamp = identity.get("identity_sha256")
        recomputed_stamp = canonical_identity_sha256(identity)
        if stamp != recomputed_stamp:
            errors.append(f"case {label} identity_sha256 mismatch: recorded {stamp}, recomputed {recomputed_stamp}")

        if stamp in seen_identities:
            errors.append(f"duplicate identity_sha256 across cases: {stamp}")
        seen_identities.add(stamp)

        # Check stamps on all term blocks
        for col in exp.get("columns", []):
            if col.get("identity_sha256") != stamp:
                errors.append(f"case {label} column {col.get('column')} stamp mismatch")
        if exp.get("assembly", {}).get("identity_sha256") != stamp:
            errors.append(f"case {label} assembly stamp mismatch")
        for adj in exp.get("adjoint_columns", []):
            if adj.get("identity_sha256") != stamp:
                errors.append(f"case {label} adjoint column {adj.get('column')} stamp mismatch")

        # Binding to published bits: A, h, w
        if case.get("a") != pub.get("a"):
            errors.append(f"case {label} matrix A bits do not match published cases.json")
        if case.get("h") != pub.get("h"):
            errors.append(f"case {label} step h bits do not match published cases.json")
        if case.get("w") != pub.get("w"):
            errors.append(f"case {label} vectors w bits do not match published cases.json")

        # Binding to published results: degree, scale, shift, fused, coverage.total
        if case.get("degree") != pub.get("degree") or identity.get("degree") != pub.get("degree"):
            errors.append(f"case {label} degree mismatch against published")
        if case.get("laguerre_scale") != pub.get("laguerre_scale"):
            errors.append(f"case {label} laguerre_scale mismatch against published")
        if case.get("normalization_shift") != pub.get("normalization_shift"):
            errors.append(f"case {label} normalization_shift mismatch against published")
        if case.get("fused") != pub.get("fused"):
            errors.append(f"case {label} fused vector bits mismatch against published")

        pub_total_hex = f"{struct.unpack('>Q', struct.pack('>d', pub['coverage']['total']))[0]:016x}"
        if case.get("laguerre_adjoint_total") != pub_total_hex:
            errors.append(f"case {label} laguerre_adjoint_total mismatch against published")
        if exp.get("assembly", {}).get("total_upper") != pub_total_hex:
            errors.append(f"case {label} assembly total_upper mismatch against published")

    if errors:
        print("INVALID evidence violations:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return INVALID_EXIT_CODE

    # -------------------------------------------------------------------------
    # Scientific Gates
    # -------------------------------------------------------------------------
    # Gate 1: Completeness
    # Gate 2: Enclosure
    # Gate 3: Source binding of adjoint
    for case in comp_cases:
        label = case["label"]
        exp = case["export"]
        m = exp["identity"]["degree"]
        shift = exp["identity"]["normalization_shift"]
        n = case["n"]
        total_upper = decode_hex64(exp["assembly"]["total_upper"])

        # Item 1: Completeness
        cols = exp.get("columns", [])
        adj_cols = exp.get("adjoint_columns", [])
        if len(cols) != 5:
            gate_failures.append(f"{label}: columns length {len(cols)} != 5")
        if len(adj_cols) != 5:
            gate_failures.append(f"{label}: adjoint_columns length {len(adj_cols)} != 5")

        for k in range(5):
            col = cols[k]
            for term in COMPONENT_TERMS:
                val = decode_hex64(col[term])
                if not math.isfinite(val) or val < 0.0:
                    gate_failures.append(f"{label}: column {k} term {term} non-finite or negative: {val}")

            adj = adj_cols[k]
            coefs = [decode_hex64(x) for x in adj.get("coefficients", [])]
            envs = [decode_hex64(x) for x in adj.get("envelopes", [])]
            res = [decode_hex64(x) for x in adj.get("local_residuals", [])]
            abs_s = decode_hex64(adj.get("abs_scale"))

            if len(coefs) != m + 1:
                gate_failures.append(f"{label}: column {k} coefficients length {len(coefs)} != m+1 ({m+1})")
            if len(envs) != m + 1:
                gate_failures.append(f"{label}: column {k} envelopes length {len(envs)} != m+1 ({m+1})")
            if len(res) != m:
                gate_failures.append(f"{label}: column {k} local_residuals length {len(res)} != m ({m})")
            if not math.isfinite(abs_s) or abs_s < 0.0:
                gate_failures.append(f"{label}: column {k} abs_scale non-finite or negative: {abs_s}")

            for c_val in coefs:
                if not math.isfinite(c_val):
                    gate_failures.append(f"{label}: column {k} non-finite coefficient: {c_val}")
            for b_val in envs:
                if not math.isfinite(b_val) or b_val < 0.0:
                    gate_failures.append(f"{label}: column {k} envelope non-finite or negative: {b_val}")
            for r_val in res:
                if not math.isfinite(r_val) or r_val < 0.0:
                    gate_failures.append(f"{label}: column {k} residual non-finite or negative: {r_val}")

        fused_sum = decode_hex64(exp["assembly"]["fused_summation"])
        if not math.isfinite(fused_sum) or fused_sum < 0.0:
            gate_failures.append(f"{label}: assembly fused_summation non-finite or negative: {fused_sum}")

        # Item 2: Enclosure
        upward_sum = fused_sum
        for col in cols:
            upward_sum = add_up(upward_sum, decode_hex64(col["truncation"]))
            upward_sum = add_up(upward_sum, decode_hex64(col["coefficient"]))
            upward_sum = add_up(upward_sum, decode_hex64(col["summation"]))
        for col in cols:
            upward_sum = add_up(upward_sum, decode_hex64(col["recurrence_adjoint"]))
        for col in cols:
            upward_sum = add_up(upward_sum, decode_hex64(col["normalization"]))

        if not (upward_sum >= total_upper):
            gate_failures.append(f"{label}: upward sum {upward_sum} < total_upper {total_upper}")

        if shift == 0:
            if upward_sum != total_upper:
                gate_failures.append(f"{label}: in-window upward sum {upward_sum} != total_upper {total_upper}")
        else:
            limit = total_upper * (1.0 + 1.0e-12) + (5.0 * math.sqrt(n) + 64.0) * (2.0 ** -1074)
            if not (upward_sum <= limit):
                gate_failures.append(f"{label}: out-of-window upward sum {upward_sum} > limit {limit}")

        # Item 3: Source binding of adjoint
        extent_hex = exp["identity"]["laguerre_extent"]
        for k in range(5):
            adj = adj_cols[k]
            betas = [decode_hex64(x) for x in adj["envelopes"]]
            eps = [decode_hex64(x) for x in adj["local_residuals"]]
            abs_scale = decode_hex64(adj["abs_scale"])

            adj_tot = 0.0
            for j in range(1, m + 1):
                prod = mul_up(betas[j], eps[j - 1])
                adj_tot = add_up(adj_tot, prod)
            adj_bound = mul_up(abs_scale, adj_tot)
            if shift != 0:
                adj_bound = scale_bound_up(adj_bound, shift)

            expected_adj = decode_hex64(cols[k]["recurrence_adjoint"])
            if adj_bound != expected_adj:
                gate_failures.append(
                    f"{label}: column {k} adjoint sum {adj_bound} != exported recurrence_adjoint {expected_adj}"
                )

            expected_key_digest = canonical_envelope_key_digest(adj["coefficients"], extent_hex, ADJOINT_DEPTH)
            recorded_key_digest = exp["identity"]["envelope_key_digests"][k]
            if recorded_key_digest != expected_key_digest:
                gate_failures.append(
                    f"{label}: column {k} envelope key digest mismatch: recorded {recorded_key_digest}, recomputed {expected_key_digest}"
                )

    gate_1 = not any("Completeness" in f or "length" in f or "non-finite" in f for f in gate_failures)
    gate_2 = not any("upward sum" in f for f in gate_failures)
    gate_3 = not any("adjoint sum" in f or "envelope key digest" in f for f in gate_failures)

    verdict = "PASS" if not gate_failures else "FAIL"

    # -------------------------------------------------------------------------
    # Reported, not gated metrics
    # -------------------------------------------------------------------------
    # Prepare multiprocessing for sampling l_j
    worker_count = num_workers if num_workers is not None else min(24, os.cpu_count() or 4)
    tasks = []
    task_keys = []
    for case in comp_cases:
        label = case["label"]
        exp = case["export"]
        extent_hex = exp["identity"]["laguerre_extent"]
        for k in range(5):
            tasks.append((exp["adjoint_columns"][k]["coefficients"], extent_hex))
            task_keys.append((label, k))

    with multiprocessing.Pool(processes=min(worker_count, len(tasks))) as pool:
        sampled_results = pool.map(_eval_column_worker, tasks)

    sampled_map: dict[tuple[str, int], list[float]] = dict(zip(task_keys, sampled_results))

    reported_cases = []
    for case in comp_cases:
        label = case["label"]
        exp = case["export"]
        m = exp["identity"]["degree"]
        shift = exp["identity"]["normalization_shift"]
        budget = case.get("budget", published_cases[label]["budget"])

        cols = exp["columns"]
        adj_cols = exp["adjoint_columns"]
        fused_sum = decode_hex64(exp["assembly"]["fused_summation"])

        # Exact rational arithmetic for S and shares
        fused_frac = Fraction(fused_sum)
        comp_fracs = {term: sum(Fraction(decode_hex64(col[term])) for col in cols) for term in COMPONENT_TERMS}
        comp_fracs["fused_summation"] = fused_frac
        comp_fracs["assembly"] = fused_frac + comp_fracs["normalization"]

        total_S_frac = (
            fused_frac
            + comp_fracs["truncation"]
            + comp_fracs["coefficient"]
            + comp_fracs["recurrence_adjoint"]
            + comp_fracs["summation"]
            + comp_fracs["normalization"]
        )

        shares = {k: float(v / total_S_frac) if total_S_frac > 0 else 0.0 for k, v in comp_fracs.items()}
        bottleneck = max(shares, key=shares.get)

        # Comparison with PP09
        pp09_entry = pp09_rows.get(label)
        pp09_comp = None
        if pp09_entry and "derived_supplementary" in pp09_entry:
            supp = pp09_entry["derived_supplementary"]
            pp09_comp = {
                "pp09_truncation": supp["truncation"],
                "native_truncation": f"{float(comp_fracs['truncation']):.8e}",
                "pp09_coefficient": supp["coefficient"],
                "native_coefficient": f"{float(comp_fracs['coefficient']):.8e}",
            }

        # Per column metrics
        column_reports = []
        adj_floor_frac = Fraction(0)
        for k in range(5):
            adj = adj_cols[k]
            betas = [decode_hex64(x) for x in adj["envelopes"]]
            eps = [decode_hex64(x) for x in adj["local_residuals"]]
            abs_scale = decode_hex64(adj["abs_scale"])

            l_j = sampled_map[(label, k)]

            # Largest beta_j * eps_{j-1} for j = 1..m
            products = [betas[j] * eps[j - 1] for j in range(1, m + 1)]
            max_prod_idx = 1 + max(range(m), key=lambda idx: products[idx])

            overestimates = [
                float(betas[j] / l_j[j]) if l_j[j] > 0 else None for j in range(1, m + 1)
            ]
            valid_overestimates = [x for x in overestimates if x is not None]
            max_overestimate = max(valid_overestimates) if valid_overestimates else None

            # Closability floor contribution: |s_k| * sum_j l_j * eps_{j-1}
            col_l_sum_frac = sum(Fraction(l_j[j]) * Fraction(eps[j - 1]) for j in range(1, m + 1))
            adj_floor_frac += Fraction(abs_scale) * col_l_sum_frac

            column_reports.append(
                {
                    "column": k,
                    "max_beta": max(betas),
                    "max_product_index": max_prod_idx,
                    "max_overestimate_ratio": max_overestimate,
                    "sampled_lower_bounds_l_j": l_j,
                }
            )

        # Non-adjoint terms sum N
        N_frac = (
            fused_frac
            + comp_fracs["truncation"]
            + comp_fracs["coefficient"]
            + comp_fracs["summation"]
            + comp_fracs["normalization"]
        )

        if shift != 0:
            adj_floor_frac *= Fraction(2) ** shift

        F_frac = N_frac + adj_floor_frac
        F_float = float(F_frac)
        F_meets_budget = F_float <= budget

        reported_cases.append(
            {
                "label": label,
                "n": case["n"],
                "degree": m,
                "laguerre_scale": case["laguerre_scale"],
                "normalization_shift": shift,
                "budget": budget,
                "exact_sum_S": f"{float(total_S_frac):.16e}",
                "non_adjoint_sum_N": f"{float(N_frac):.16e}",
                "components": {k: f"{float(v):.16e}" for k, v in comp_fracs.items()},
                "shares": {k: f"{v:.6e}" for k, v in shares.items()},
                "bottleneck": bottleneck,
                "pp09_comparison": pp09_comp,
                "columns": column_reports,
                "closability_floor_F": f"{F_float:.16e}",
                "F_meets_budget": F_meets_budget,
                "F_over_budget": f"{(F_float / budget):.8e}",
            }
        )

    results_doc = {
        "schema": SCHEMA,
        "node": NODE,
        "verdict": verdict,
        "source_commit": source_commit,
        "tree_clean": comp_data.get("tree_clean", False),
        "command": " ".join(sys.argv),
        "runtime_seconds": round(time.time() - started, 2),
        "inputs": {
            "published": str(published_path),
            "published_sha256": published_sha,
            "pp09": str(pp09_path),
            "pp09_sha256": pp09_sha,
            "components": str(components_path),
            "components_sha256": components_sha,
        },
        "gate": {
            "1_completeness": gate_1,
            "2_enclosure": gate_2,
            "3_source_binding": gate_3,
        },
        "gate_failures": gate_failures,
        "reported": {
            "cases": reported_cases,
        },
    }

    if output_path:
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(json.dumps(results_doc, indent=2) + "\n")
        print(f"Wrote results to {output_path}")

    print(f"PY01 Check completed in {results_doc['runtime_seconds']}s with verdict: {verdict}")
    if gate_failures:
        print("Gate failures:", file=sys.stderr)
        for gf in gate_failures:
            print(f"  - {gf}", file=sys.stderr)
        return 1

    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Scientific gate checker for PY01 Laguerre component export.")
    parser.add_argument(
        "--published",
        type=Path,
        default=Path("research/rnext04_laguerre_admission_20261003/cases.json"),
        help="Path to published R-NEXT-04 cases.json",
    )
    parser.add_argument(
        "--pp09",
        type=Path,
        default=Path("research/pp09_laguerre_envelope_20261004/RESULTS.json"),
        help="Path to PP09 RESULTS.json",
    )
    parser.add_argument(
        "--components",
        type=Path,
        default=Path("research/py01_laguerre_component_export_20261011/COMPONENTS.json"),
        help="Path to PY01 COMPONENTS.json export",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("research/py01_laguerre_component_export_20261011/RESULTS.json"),
        help="Path to output RESULTS.json",
    )
    parser.add_argument(
        "--workers",
        type=int,
        default=None,
        help="Number of multiprocessing workers for mpmath sampled lower bounds",
    )
    args = parser.parse_args()

    return run_check(
        published_path=args.published,
        pp09_path=args.pp09,
        components_path=args.components,
        output_path=args.output,
        num_workers=args.workers,
    )


if __name__ == "__main__":
    sys.exit(main())
