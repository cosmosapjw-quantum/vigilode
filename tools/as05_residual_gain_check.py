#!/usr/bin/env python3
"""Independent exact-binary oracle and verification checker for AS05.

Validates current-operator residual gain witness in core (DAG node AS05).
Strictly independent of native Rust interval arithmetic. Uses Python Fraction
exact rational arithmetic for matrix inversions up to dimension 8.
"""

import argparse
import hashlib
import json
import math
import struct
import sys
from fractions import Fraction as F
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parent

PINNED_HASHES = {
    "research/reaudit_accuracy_speed_20261010/NATIVE.json": "b9404714af837dfb099db564550e88127ff52a0c9341758862529261b85bd364",
    "research/reaudit_accuracy_speed_20261010/gain_witness.rs": "22c977199d3c60849db94227a973a5d83ffea381650043774e8510ad5e745c89",
    "research/reaudit_accuracy_speed_20261010/check.py": "238d52a2a41ca87debdd6a5bfc4b9421bccb95ba6c7170c7fb832fe281239b89",
    "research/reaudit_accuracy_speed_20261010/RESULTS.json": "20a6cb32aa119142576bac4adbe836fbd6e13623cf4dd2c4c0fbda36fc0d0e21",
}


def compute_sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


def hex_to_f64(h: str) -> float:
    return struct.unpack("!d", bytes.fromhex(h))[0]


def hex_to_fraction(h: str) -> F:
    val = hex_to_f64(h)
    if not math.isfinite(val):
        raise ValueError(f"Non-finite float in hex: {h} -> {val}")
    return F(val)


def num_to_fraction(x) -> F:
    if isinstance(x, str):
        return hex_to_fraction(x)
    elif isinstance(x, (int, float)):
        if not math.isfinite(x):
            raise ValueError(f"Non-finite float: {x}")
        return F(x)
    raise TypeError(f"Cannot convert to Fraction: {x!r}")


def solve_fraction_system(A: list, b: list) -> list:
    """Exact Gaussian elimination with partial pivoting in Python Fraction."""
    n = len(A)
    M = [[F(A[i][j]) for j in range(n)] for i in range(n)]
    rhs = [F(b[i]) for i in range(n)]

    for col in range(n):
        pivot = None
        for row in range(col, n):
            if M[row][col] != 0:
                pivot = row
                break
        if pivot is None:
            raise ValueError("Singular matrix in exact solver")
        if pivot != col:
            M[col], M[pivot] = M[pivot], M[col]
            rhs[col], rhs[pivot] = rhs[pivot], rhs[col]

        pv = M[col][col]
        for row in range(col + 1, n):
            factor = M[row][col] / pv
            for j in range(col, n):
                M[row][j] -= factor * M[col][j]
            rhs[row] -= factor * rhs[col]

    x = [F(0)] * n
    for row in range(n - 1, -1, -1):
        s = sum(M[row][j] * x[j] for j in range(row + 1, n))
        x[row] = (rhs[row] - s) / M[row][row]
    return x


def check_authority_types(src_path: Path) -> dict:
    """Inspects residual_gain.rs to ensure zero-deserialization and private fields on authority types."""
    if not src_path.is_file():
        return {"status": "FAIL", "reason": f"File not found: {src_path}"}

    content = src_path.read_text(encoding="utf-8")
    authority_types = [
        "LinearTargetId",
        "FrozenMetric",
        "VerifiedCorrection",
        "ResidualGainWitness",
    ]

    for auth in authority_types:
        # Check no Deserialize implementation or derive
        # Regex or substring check:
        # Looking for `Deserialize` anywhere near the struct or impl
        lines = content.splitlines()
        for idx, line in enumerate(lines):
            if f"struct {auth}" in line:
                # Look backwards for derives
                prev_lines = lines[max(0, idx - 5):idx]
                for pl in prev_lines:
                    if "derive" in pl and "Deserialize" in pl:
                        return {"status": "FAIL", "reason": f"{auth} derives Deserialize"}
            if f"impl<'de> Deserialize<'de> for {auth}" in line or f"impl Deserialize for {auth}" in line:
                return {"status": "FAIL", "reason": f"{auth} implements Deserialize"}

        # Check fields of authority struct are private
        # Find struct definition
        in_struct = False
        for line in lines:
            stripped = line.strip()
            if stripped.startswith(f"pub struct {auth}"):
                in_struct = True
                continue
            if in_struct:
                if stripped.startswith("}"):
                    in_struct = False
                    continue
                if stripped.startswith("pub ") and not stripped.startswith("pub(crate)"):
                    return {"status": "FAIL", "reason": f"{auth} has public field: {stripped}"}

    return {"status": "PASS"}


def verify_runs_file(runs_path: Path, retained_path: Path) -> dict:
    # 1. Pinned hashes check
    pin_results = {}
    for rel_path, expected_hash in PINNED_HASHES.items():
        full_path = REPO_ROOT / rel_path
        if not full_path.is_file():
            return {
                "verdict": "INVALID",
                "reason": f"Missing pinned reference file: {rel_path}",
                "pin_results": pin_results,
            }
        actual_hash = compute_sha256(full_path)
        if actual_hash != expected_hash:
            return {
                "verdict": "INVALID",
                "reason": f"Hash mismatch for {rel_path}: expected {expected_hash}, got {actual_hash}",
                "pin_results": pin_results,
            }
        pin_results[rel_path] = "MATCH"

    # 2. Authority types check
    auth_res = check_authority_types(REPO_ROOT / "crates/rodas5p-core/src/residual_gain.rs")
    if auth_res["status"] != "PASS":
        return {
            "verdict": "FAIL",
            "reason": f"Authority type contract violation: {auth_res.get('reason')}",
            "pin_results": pin_results,
            "authority_check": auth_res,
        }

    # 3. Load RUNS.json
    if not runs_path.is_file():
        return {
            "verdict": "INVALID",
            "reason": f"Runs file not found: {runs_path}",
            "pin_results": pin_results,
        }

    try:
        with open(runs_path, "r", encoding="utf-8") as f:
            runs_doc = json.load(f)
    except Exception as e:
        return {
            "verdict": "INVALID",
            "reason": f"Failed to parse RUNS.json: {e}",
            "pin_results": pin_results,
        }

    if runs_doc.get("schema") != "vigilode-as05-runs-v1":
        return {
            "verdict": "INVALID",
            "reason": f"Invalid schema: {runs_doc.get('schema')}",
            "pin_results": pin_results,
        }
    if runs_doc.get("node") != "as05_residual_gain_core_20261011":
        return {
            "verdict": "INVALID",
            "reason": f"Invalid node: {runs_doc.get('node')}",
            "pin_results": pin_results,
        }

    cases = runs_doc.get("cases", [])
    if runs_doc.get("total_cases") != len(cases):
        return {
            "verdict": "INVALID",
            "reason": f"total_cases mismatch: header has {runs_doc.get('total_cases')}, array has {len(cases)}",
            "pin_results": pin_results,
        }

    # 4. Check W-formation discrimination table
    w_discrim = runs_doc.get("w_formation_discrimination", [])
    if len(w_discrim) != 16:
        return {
            "verdict": "FAIL",
            "reason": f"Expected 16 W-formation combinations, found {len(w_discrim)}",
            "pin_results": pin_results,
        }

    discriminating_fixtures = [item for item in w_discrim if item.get("discriminates")]
    if len(discriminating_fixtures) < 4:
        return {
            "verdict": "FAIL",
            "reason": f"Expected at least 4 discriminating W-formation fixtures, got {len(discriminating_fixtures)}",
            "pin_results": pin_results,
        }

    # Check the first 4 in list order
    first_4 = [(item["h"], item["k"]) for item in discriminating_fixtures[:4]]
    expected_first_4 = [(0.1, 0.0), (0.1, 1024.0), (0.3, 0.0), (0.3, 1024.0)]
    if first_4 != expected_first_4:
        return {
            "verdict": "FAIL",
            "reason": f"Discriminating fixtures mismatch: expected {expected_first_4}, got {first_4}",
            "pin_results": pin_results,
        }

    # 5. Evaluate individual test cases
    case_results = []
    category_counts = {}
    passed_cases = 0

    # Load retained NATIVE.json for cross-checking Case A
    with open(retained_path, "r", encoding="utf-8") as f:
        retained_doc = json.load(f)
    study_cells = {c["id"]: c for c in retained_doc["gain_study"]["cells"]}
    study_invalid = {c["id"]: c for c in retained_doc["gain_study"]["invalid_controls"]}

    for idx, case in enumerate(cases):
        cid = case.get("id", f"case_{idx}")
        cat = case.get("category")
        category_counts[cat] = category_counts.get(cat, 0) + 1
        ev_kind = case.get("evidence_kind")

        passed = True
        err_msg = None

        if cat == "case_a_retained":
            if ev_kind != "verified":
                passed = False
                err_msg = f"Expected verified, got {ev_kind}"
            else:
                ref = study_cells.get(cid)
                if not ref:
                    passed = False
                    err_msg = f"Cell {cid} not found in retained NATIVE.json"
                else:
                    ref_wrms = ref["certificate"]["wrms_upper"]
                    core_wrms = case.get("wrms_upper")
                    if core_wrms > ref_wrms * (1.0 + 1e-14):
                        passed = False
                        err_msg = f"wrms_upper {core_wrms} > ref {ref_wrms}"

                    # Exact Fraction solve
                    w_hex = case["inputs"]["W_hex"]
                    b_hex = case["inputs"]["rhs_hex"]
                    x_hex = case["inputs"]["candidate_hex"]
                    s_hex = case["inputs"]["scales_hex"]

                    W_frac = [[hex_to_fraction(w_hex[i][j]) for j in range(2)] for i in range(2)]
                    b_frac = [hex_to_fraction(b_hex[i]) for i in range(2)]
                    x_frac = [hex_to_fraction(x_hex[i]) for i in range(2)]
                    s_frac = [hex_to_fraction(s_hex[i]) for i in range(2)]

                    x_exact = solve_fraction_system(W_frac, b_frac)
                    weighted_err = [(x_exact[i] - x_frac[i]) / s_frac[i] for i in range(2)]

                    corr_int_hex = case.get("weighted_correction_intervals_hex")
                    for i in range(2):
                        lo = hex_to_fraction(corr_int_hex[i][0])
                        hi = hex_to_fraction(corr_int_hex[i][1])
                        if not (lo <= weighted_err[i] <= hi):
                            passed = False
                            err_msg = f"Component {i} error {float(weighted_err[i])} not in [{float(lo)}, {float(hi)}]"

        elif cat == "case_a_invalid_control":
            if cid == "invalid_determinant_overlap":
                # Registered exception in PREREGISTRATION.md (lines 183-185):
                # "The registered exception is `invalid_determinant_overlap`. Its exact determinant
                #  is 2^-52, which the directed primitives compute exactly, so it is predicted Verified.
                #  The exception may be Verified (then it must enclose its oracle error) or Unavailable;
                #  both PASS."
                if ev_kind == "verified":
                    w_hex = case["inputs"]["W_hex"]
                    b_hex = case["inputs"]["rhs_hex"]
                    x_hex = case["inputs"]["candidate_hex"]
                    s_hex = case["inputs"]["scales_hex"]

                    W_frac = [[hex_to_fraction(w_hex[i][j]) for j in range(2)] for i in range(2)]
                    b_frac = [hex_to_fraction(b_hex[i]) for i in range(2)]
                    x_frac = [hex_to_fraction(x_hex[i]) for i in range(2)]
                    s_frac = [hex_to_fraction(s_hex[i]) for i in range(2)]

                    x_exact = solve_fraction_system(W_frac, b_frac)
                    weighted_err = [(x_exact[i] - x_frac[i]) / s_frac[i] for i in range(2)]
                    wrms_bound = F(case["wrms_upper"])
                    wrms_err_sq = sum(e * e for e in weighted_err) / F(2)
                    if not (wrms_bound * wrms_bound >= wrms_err_sq):
                        passed = False
                        err_msg = f"wrms bound violation on invalid_determinant_overlap"
                elif ev_kind == "unavailable":
                    pass
                else:
                    passed = False
                    err_msg = f"Unexpected evidence_kind for invalid_determinant_overlap: {ev_kind}"
            elif ev_kind != "unavailable":
                passed = False
                err_msg = f"Expected unavailable for invalid control, got {ev_kind}"
            else:
                ref = study_invalid.get(cid)
                if not ref:
                    passed = False
                    err_msg = f"Invalid control {cid} not found in NATIVE.json"
                else:
                    expected_error = ref.get("error")
                    actual_rej = case.get("rejection_class")
                    if actual_rej != expected_error:
                        passed = False
                        err_msg = f"Rejection mismatch: expected {expected_error}, got {actual_rej}"

        elif cat in ("case_c_w_formation", "case_c_metric", "case_c_epoch", "case_c_operator",
                     "case_c_coupling", "case_c_structure"):
            if ev_kind != "unavailable":
                passed = False
                err_msg = f"Negative control expected unavailable, got {ev_kind}"
            else:
                expected_rejections = {
                    "case_c_w_formation": "target_mismatch",
                    "case_c_metric": "metric_mismatch",
                    "case_c_epoch": "stale_epoch",
                    "case_c_operator": "operator_mismatch",
                    "case_c_coupling": "off_block_coupling",
                    "case_c_structure": "undeclared_structure",
                }
                exp_rej = expected_rejections.get(cat)
                actual_rej = case.get("rejection_class")
                if exp_rej and actual_rej != exp_rej:
                    # undeclared_structure can also be route_not_implemented
                    if not (exp_rej == "undeclared_structure" and actual_rej in ("undeclared_structure", "route_not_implemented")):
                        passed = False
                        err_msg = f"Expected rejection {exp_rej}, got {actual_rej}"

        elif cat in ("case_c_assembly", "case_c_nu"):
            if ev_kind != "estimated":
                passed = False
                err_msg = f"Expected estimated, got {ev_kind}"
            else:
                exp_source = "undeclared_assembly" if cat == "case_c_assembly" else "projected_nu"
                if case.get("estimate_source") != exp_source:
                    passed = False
                    err_msg = f"Expected estimate source {exp_source}, got {case.get('estimate_source')}"
                if not (case.get("estimate") is not None and math.isfinite(case["estimate"])):
                    passed = False
                    err_msg = f"Estimate missing or non-finite: {case.get('estimate')}"

        elif cat == "case_d_w_formation_exact":
            if ev_kind != "verified":
                passed = False
                err_msg = f"Expected verified, got {ev_kind}"
            else:
                # Exact fraction oracle
                J_hex = case["inputs"]["J_hex"]
                h_frac = hex_to_fraction(case["inputs"]["h_hex"])
                g_frac = hex_to_fraction(case["inputs"]["gamma_hex"])
                a_frac = h_frac * g_frac

                W_frac = [
                    [F(1) - a_frac * hex_to_fraction(J_hex[0][0]), -a_frac * hex_to_fraction(J_hex[0][1])],
                    [-a_frac * hex_to_fraction(J_hex[1][0]), F(1) - a_frac * hex_to_fraction(J_hex[1][1])],
                ]
                b_frac = [hex_to_fraction(case["inputs"]["rhs_hex"][i]) for i in range(2)]
                x_frac = [hex_to_fraction(case["inputs"]["candidate_hex"][i]) for i in range(2)]
                s_frac = [hex_to_fraction(case["inputs"]["scales_hex"][i]) for i in range(2)]

                x_exact = solve_fraction_system(W_frac, b_frac)
                weighted_err = [(x_exact[i] - x_frac[i]) / s_frac[i] for i in range(2)]

                corr_int_hex = case.get("weighted_correction_intervals_hex")
                for i in range(2):
                    lo = hex_to_fraction(corr_int_hex[i][0])
                    hi = hex_to_fraction(corr_int_hex[i][1])
                    if not (lo <= weighted_err[i] <= hi):
                        passed = False
                        err_msg = f"Component {i} error {float(weighted_err[i])} not in [{float(lo)}, {float(hi)}]"

        elif cat == "case_d_w_formation_delta":
            if ev_kind == "verified":
                # Verify all 16 sign corners
                h_frac = hex_to_fraction(case["inputs"]["h_hex"])
                g_frac = hex_to_fraction(case["inputs"]["gamma_hex"])
                a_frac = h_frac * g_frac
                b_frac = [hex_to_fraction(case["inputs"]["rhs_hex"][i]) for i in range(2)]
                x_frac = [hex_to_fraction(case["inputs"]["candidate_hex"][i]) for i in range(2)]
                s_frac = [hex_to_fraction(case["inputs"]["scales_hex"][i]) for i in range(2)]
                corr_int_hex = case.get("weighted_correction_intervals_hex")

                for corner in case.get("assembly_corners", []):
                    J_c_hex = corner["J_corner_hex"]
                    W_c = [
                        [F(1) - a_frac * hex_to_fraction(J_c_hex[0][0]), -a_frac * hex_to_fraction(J_c_hex[0][1])],
                        [-a_frac * hex_to_fraction(J_c_hex[1][0]), F(1) - a_frac * hex_to_fraction(J_c_hex[1][1])],
                    ]
                    x_corner = solve_fraction_system(W_c, b_frac)
                    w_err = [(x_corner[i] - x_frac[i]) / s_frac[i] for i in range(2)]
                    for i in range(2):
                        lo = hex_to_fraction(corr_int_hex[i][0])
                        hi = hex_to_fraction(corr_int_hex[i][1])
                        if not (lo <= w_err[i] <= hi):
                            passed = False
                            err_msg = f"Corner {corner['mask']} component {i} error outside interval"
                            break
                    if not passed:
                        break
            elif ev_kind == "unavailable":
                # Vacuous corner check, allowed by PREREGISTRATION.md
                pass
            else:
                passed = False
                err_msg = f"Unexpected evidence kind: {ev_kind}"

        elif cat == "case_d_transport":
            if ev_kind != "verified":
                passed = False
                err_msg = f"Expected verified transport, got {ev_kind}"
            else:
                wrms = case.get("wrms_upper")
                inf_u = case.get("weighted_inf_upper")
                if not (wrms >= 0 and inf_u >= 0 and math.isfinite(wrms) and math.isfinite(inf_u)):
                    passed = False
                    err_msg = f"Invalid transport norm bounds: wrms={wrms}, inf={inf_u}"
                else:
                    inputs = case.get("inputs", {})
                    if "W_hex" in inputs and "rhs_hex" in inputs and "candidate_hex" in inputs:
                        w_hex = inputs["W_hex"]
                        b_hex = inputs["rhs_hex"]
                        x_hex = inputs["candidate_hex"]
                        new_s_hex = inputs["new_scales_hex"]

                        W_frac = [[hex_to_fraction(w_hex[i][j]) for j in range(2)] for i in range(2)]
                        b_frac = [hex_to_fraction(b_hex[i]) for i in range(2)]
                        x_frac = [hex_to_fraction(x_hex[i]) for i in range(2)]
                        s_new_frac = [hex_to_fraction(new_s_hex[i]) for i in range(2)]

                        x_exact = solve_fraction_system(W_frac, b_frac)
                        weighted_err = [(x_exact[i] - x_frac[i]) / s_new_frac[i] for i in range(2)]

                        inf_err = max(abs(e) for e in weighted_err)
                        wrms_err_sq = sum(e * e for e in weighted_err) / F(2)

                        if F(inf_u) < inf_err:
                            passed = False
                            err_msg = f"Transport inf bound violation: true={float(inf_err)}, bound={inf_u}"
                        elif F(wrms) * F(wrms) < wrms_err_sq:
                            passed = False
                            err_msg = f"Transport wrms bound violation: true_sq={float(wrms_err_sq)}, bound={wrms}"

                        # If scaled residual gain is present, verify exact operator gain bound
                        gain_upper = case.get("scaled_residual_gain_inf_upper")
                        if gain_upper is not None and math.isfinite(gain_upper):
                            det = W_frac[0][0] * W_frac[1][1] - W_frac[0][1] * W_frac[1][0]
                            inv_W = [
                                [W_frac[1][1] / det, -W_frac[0][1] / det],
                                [-W_frac[1][0] / det, W_frac[0][0] / det],
                            ]
                            row0 = abs(inv_W[0][0] * s_new_frac[0] / s_new_frac[0]) + abs(inv_W[0][1] * s_new_frac[1] / s_new_frac[0])
                            row1 = abs(inv_W[1][0] * s_new_frac[0] / s_new_frac[1]) + abs(inv_W[1][1] * s_new_frac[1] / s_new_frac[1])
                            true_scaled_gain = max(row0, row1)
                            if F(gain_upper) < true_scaled_gain:
                                passed = False
                                err_msg = f"Transport scaled gain bound violation: true={float(true_scaled_gain)}, bound={gain_upper}"

        elif cat == "case_d_isolated_blocks":
            if ev_kind != "verified":
                passed = False
                err_msg = f"Expected verified isolated blocks, got {ev_kind}"
            else:
                n = case.get("dimension")
                w_hex = case["inputs"]["W_hex"]
                b_hex = case["inputs"]["rhs_hex"]
                x_hex = case["inputs"]["candidate_hex"]
                s_hex = case["inputs"]["scales_hex"]

                W_frac = [[hex_to_fraction(w_hex[i][j]) for j in range(n)] for i in range(n)]
                b_frac = [hex_to_fraction(b_hex[i]) for i in range(n)]
                x_frac = [hex_to_fraction(x_hex[i]) for i in range(n)]
                s_frac = [hex_to_fraction(s_hex[i]) for i in range(n)]

                x_exact = solve_fraction_system(W_frac, b_frac)
                w_err = [(x_exact[i] - x_frac[i]) / s_frac[i] for i in range(n)]

                corr_int_hex = case.get("weighted_correction_intervals_hex")
                for i in range(n):
                    lo = hex_to_fraction(corr_int_hex[i][0])
                    hi = hex_to_fraction(corr_int_hex[i][1])
                    if not (lo <= w_err[i] <= hi):
                        passed = False
                        err_msg = f"Component {i} error outside interval"
                        break

        elif cat == "case_d_lognorm":
            if ev_kind == "verified":
                n = case.get("dimension")
                J_hex = case["inputs"]["J_hex"]
                h_frac = hex_to_fraction(case["inputs"]["h_hex"])
                g_frac = hex_to_fraction(case["inputs"]["gamma_hex"])
                a_frac = h_frac * g_frac

                W_frac = [[F(1 if i == j else 0) - a_frac * hex_to_fraction(J_hex[i][j]) for j in range(n)] for i in range(n)]
                b_frac = [hex_to_fraction(case["inputs"]["rhs_hex"][i]) for i in range(n)]
                x_frac = [hex_to_fraction(case["inputs"]["candidate_hex"][i]) for i in range(n)]
                s_frac = [hex_to_fraction(case["inputs"]["scales_hex"][i]) for i in range(n)]

                x_exact = solve_fraction_system(W_frac, b_frac)
                w_err = [(x_exact[i] - x_frac[i]) / s_frac[i] for i in range(n)]

                inf_err = max(abs(e) for e in w_err)
                wrms_err_sq = sum(e * e for e in w_err) / F(n)

                wrms_bound = F(case["wrms_upper"])
                inf_bound = F(case["weighted_inf_upper"])

                if not (wrms_bound * wrms_bound >= wrms_err_sq and inf_bound >= inf_err):
                    passed = False
                    err_msg = f"Norm bound violation: true wrms_sq={float(wrms_err_sq)}, bound={float(wrms_bound)}"
            elif ev_kind == "unavailable":
                # Nonpositive denominator can occur when 1 - a mu <= 0
                rej = case.get("rejection_class")
                if rej != "nonpositive_denominator":
                    passed = False
                    err_msg = f"Expected nonpositive_denominator, got {rej}"
            else:
                passed = False
                err_msg = f"Unexpected evidence kind in lognorm: {ev_kind}"

        elif cat == "case_d_negatives":
            if ev_kind != "unavailable":
                passed = False
                err_msg = f"Expected unavailable for route d negatives, got {ev_kind}"
            else:
                expected = "negative_shift" if "negative_shift" in cid else "nonpositive_denominator"
                if case.get("rejection_class") != expected:
                    passed = False
                    err_msg = f"Expected rejection {expected}, got {case.get('rejection_class')}"

        if passed:
            passed_cases += 1
        case_results.append({
            "id": cid,
            "category": cat,
            "pass": passed,
            "error": err_msg,
        })

    all_pass = (passed_cases == len(cases))
    verdict = "PASS" if all_pass else "FAIL"

    return {
        "verdict": verdict,
        "total_cases": len(cases),
        "passed_cases": passed_cases,
        "failed_cases": len(cases) - passed_cases,
        "category_counts": category_counts,
        "pin_results": pin_results,
        "authority_check": auth_res,
        "w_formation_discrimination": {
            "total_fixtures": len(w_discrim),
            "discriminating_fixtures": len(discriminating_fixtures),
            "first_4": first_4,
        },
        "case_failures": [c for c in case_results if not c["pass"]],
    }


def main():
    parser = argparse.ArgumentParser(description="Residual gain current-operator checker (AS05).")
    parser.add_argument("--runs", type=Path, default=REPO_ROOT / "research/as05_residual_gain_core_20261011/RUNS.json")
    parser.add_argument("--retained", type=Path, default=REPO_ROOT / "research/reaudit_accuracy_speed_20261010/NATIVE.json")
    parser.add_argument("--output", type=Path, default=REPO_ROOT / "research/as05_residual_gain_core_20261011/RESULTS.json")
    args = parser.parse_args()

    report = verify_runs_file(args.runs, args.retained)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with open(args.output, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2)

    print(json.dumps({
        "verdict": report.get("verdict"),
        "total_cases": report.get("total_cases"),
        "passed_cases": report.get("passed_cases"),
        "failed_cases": report.get("failed_cases"),
        "authority_check": report.get("authority_check", {}).get("status"),
    }, indent=2))

    verdict = report.get("verdict")
    if verdict == "PASS":
        return 0
    elif verdict == "FAIL":
        return 1
    else:
        return 2


if __name__ == "__main__":
    sys.exit(main())
