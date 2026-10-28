#!/usr/bin/env python3
"""Independent scientific gate checker for single backend router (PY03).

Research node: research/py03_single_backend_router_20261011
Preregistration: research/py03_single_backend_router_20261011/PREREGISTRATION.md
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import struct
import sys

import mpmath as mp

# Import schema validator from AS03
sys.path.insert(0, str(Path(__file__).resolve().parent))
from evidence_schema_v2 import INVALID_EXIT_CODE, ValidationError, loads_strict

SCHEMA = "vigilode-py03-single-backend-router-results-v1"
DEGREE_LIMIT = 128


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


def exact_target(case: dict, dps: int) -> list:
    mp.mp.dps = dps
    n = case["n"]
    a = mp.matrix([[mp.mpf(unhex(x)) for x in row] for row in case["a"]])
    h = mp.mpf(unhex(case["h"]))
    w = [[mp.mpf(unhex(x)) for x in vec] for vec in case["w"]]
    eigenvalues, q = mp.eigsy(a)
    result = [mp.mpf(0)] * n
    for k in range(5):
        if all(x == 0 for x in w[k]):
            continue
        coords = [mp.fsum(q[i, j] * w[k][i] for i in range(n)) for j in range(n)]
        scaled = [phi(k, h * eigenvalues[j]) * coords[j] for j in range(n)]
        for i in range(n):
            result[i] += mp.fsum(q[i, j] * scaled[j] for j in range(n))
    return result


def distance(values_hex: list[str], exact: list) -> mp.mpf:
    values = [mp.mpf(unhex(x)) for x in values_hex]
    return mp.sqrt(mp.fsum((v - e) ** 2 for v, e in zip(values, exact)))


def cost_sum_matches(total: dict, parts: list[dict]) -> bool:
    keys = [
        "plan_tail_evaluations",
        "coefficient_setups",
        "coefficient_cache_hits",
        "coefficient_series_terms",
        "executed_backends",
        "failed_executions",
        "vector_products",
        "block_products",
        "certification_operations",
    ]
    for key in keys:
        expected = sum(p.get(key, 0) for p in parts)
        if total.get(key, 0) != expected:
            return False
    return True


def main():
    parser = argparse.ArgumentParser(description="PY03 scientific gate checker")
    parser.add_argument("--cases", type=Path, required=True, help="Path to cases.json")
    parser.add_argument("--output", type=Path, required=True, help="Path to RESULTS.json")
    args = parser.parse_args()

    if args.output.exists():
        print(f"Error: Output file already exists: {args.output}", file=sys.stderr)
        sys.exit(INVALID_EXIT_CODE)

    if not args.cases.exists():
        print(f"Error: Cases file not found: {args.cases}", file=sys.stderr)
        sys.exit(INVALID_EXIT_CODE)

    # 1. AS03 Strict JSON validation
    try:
        raw_bytes = args.cases.read_bytes()
        data = loads_strict(raw_bytes)
    except ValidationError as err:
        print(f"AS03 ValidationError: {err}", file=sys.stderr)
        sys.exit(INVALID_EXIT_CODE)
    except Exception as err:
        print(f"JSON load error: {err}", file=sys.stderr)
        sys.exit(INVALID_EXIT_CODE)

    cases = data.get("cases", [])
    if len(cases) != 103:
        print(f"Validity error: expected 103 cases, found {len(cases)}", file=sys.stderr)
        sys.exit(INVALID_EXIT_CODE)

    labels = set()
    grid_cases = []
    niche_cases = []

    # Gates flags
    gate_1_certification = True
    gate_1_reasons = []

    gate_2_parity = True
    gate_2_reasons = []

    gate_3_accounting = True
    gate_3_reasons = []

    gate_4_one_execution = True
    gate_4_reasons = []

    for case in cases:
        label = case.get("label", "")
        if label in labels:
            print(f"Validity error: duplicate case label {label}", file=sys.stderr)
            sys.exit(INVALID_EXIT_CODE)
        labels.add(label)

        group = case.get("group", "")
        if group == "grid":
            grid_cases.append(case)

        # 50-digit and 70-digit reference verification
        target_50 = exact_target(case, dps=50)
        target_70 = exact_target(case, dps=70)

        diff = mp.sqrt(mp.fsum((a - b) ** 2 for a, b in zip(target_50, target_70)))
        norm_f = mp.sqrt(mp.fsum(x ** 2 for x in target_50))
        if norm_f > 0 and diff > mp.mpf("1e-40") * norm_f:
            print(f"Validity error: 50/70 digit discrepancy on {label}: diff={diff:e}, norm={norm_f:e}", file=sys.stderr)
            sys.exit(INVALID_EXIT_CODE)

        runs = case.get("runs", {})
        if set(runs.keys()) != {"cold", "warm"}:
            print(f"Validity error: runs keys must be exactly cold and warm on {label}", file=sys.stderr)
            sys.exit(INVALID_EXIT_CODE)

        for cache_state, arm_map in runs.items():
            if set(arm_map.keys()) != {"single", "cheb", "pp08"}:
                print(f"Validity error: arm keys must be single, cheb, pp08 on {label}/{cache_state}", file=sys.stderr)
                sys.exit(INVALID_EXIT_CODE)

            for arm_name, arm in arm_map.items():
                # Arithmetic consistency: total_cost == plan_cost + sum(execution_costs)
                total_cost = arm.get("cost", {})
                plan_cost = arm.get("plan_cost", {})
                execution_costs = arm.get("execution_costs", [])
                if not cost_sum_matches(total_cost, [plan_cost] + execution_costs):
                    print(f"Validity error: cost sum mismatch on {label}/{cache_state}/{arm_name}", file=sys.stderr)
                    sys.exit(INVALID_EXIT_CODE)

            single = arm_map["single"]
            cheb = arm_map["cheb"]
            pp08 = arm_map["pp08"]
            total_budget = unhex(case["total_budget"])

            # Check Niche determination (cold preflight of single on grid cases)
            if group == "grid" and cache_state == "cold":
                if single.get("preselected_laguerre", False):
                    niche_cases.append(case)

            # Gate 1: Certification
            if single.get("admitted", False):
                bound_hex = single.get("bound")
                if bound_hex is None:
                    gate_1_certification = False
                    gate_1_reasons.append(f"{label}/{cache_state}: admitted but bound is None")
                else:
                    bound = unhex(bound_hex)
                    if bound > total_budget:
                        gate_1_certification = False
                        gate_1_reasons.append(f"{label}/{cache_state}: bound {bound:e} > budget {total_budget:e}")
                    fused_hex = single.get("fused", [])
                    err_50 = float(distance(fused_hex, target_50))
                    if bound < err_50:
                        gate_1_certification = False
                        gate_1_reasons.append(f"{label}/{cache_state}: bound {bound:e} < error {err_50:e}")

                enclosure_ev = case.get("enclosure", {}).get("evidence")
                if enclosure_ev == "Declared":
                    gate_1_certification = False
                    gate_1_reasons.append(f"{label}/{cache_state}: declared enclosure was admitted")
                if "timing" in label:
                    gate_1_certification = False
                    gate_1_reasons.append(f"{label}/{cache_state}: timing execution was admitted")
                if single.get("choice") == "Laguerre":
                    lag_deg = single.get("laguerre_plan", {}).get("degree", 0)
                    if lag_deg > DEGREE_LIMIT:
                        gate_1_certification = False
                        gate_1_reasons.append(f"{label}/{cache_state}: Laguerre degree {lag_deg} > {DEGREE_LIMIT} was admitted")
            else:
                if single.get("choice") != "Fallback":
                    gate_1_certification = False
                    gate_1_reasons.append(f"{label}/{cache_state}: not admitted but choice is not Fallback")

            # Gate 2: Parity
            if single.get("choice") == "Chebyshev" and cheb.get("admitted", False):
                if single.get("fused") != cheb.get("fused"):
                    gate_2_parity = False
                    gate_2_reasons.append(f"{label}/{cache_state}: fused bitwise mismatch with cheb")
                if single.get("bound") != cheb.get("bound"):
                    gate_2_parity = False
                    gate_2_reasons.append(f"{label}/{cache_state}: bound bitwise mismatch with cheb")

            # Gate 3: Accounting
            caller_counters = single.get("caller_counters", {})
            cost = single.get("cost", {})
            if cost.get("vector_products", 0) != caller_counters.get("poly_vector_products", 0):
                gate_3_accounting = False
                gate_3_reasons.append(f"{label}/{cache_state}: vector_products mismatch")
            if cost.get("block_products", 0) != caller_counters.get("poly_block_products", 0):
                gate_3_accounting = False
                gate_3_reasons.append(f"{label}/{cache_state}: block_products mismatch")
            if cost.get("coefficient_setups", 0) != caller_counters.get("poly_coefficient_setups", 0):
                gate_3_accounting = False
                gate_3_reasons.append(f"{label}/{cache_state}: setups mismatch")
            if cost.get("coefficient_cache_hits", 0) != caller_counters.get("poly_coefficient_reuses", 0):
                gate_3_accounting = False
                gate_3_reasons.append(f"{label}/{cache_state}: cache hits mismatch")

            if "force-laguerre" in label:
                if cost.get("failed_executions", 0) != 1 or cost.get("executed_backends", 0) != 2:
                    gate_3_accounting = False
                    gate_3_reasons.append(f"{label}/{cache_state}: force-laguerre failed execution not charged properly")

            # Gate 4: One execution
            if single.get("admitted", False) and cost.get("failed_executions", 0) == 0:
                if cost.get("executed_backends", 0) != 1:
                    gate_4_one_execution = False
                    gate_4_reasons.append(f"{label}/{cache_state}: admitted but executed_backends != 1")
                single_vps = cost.get("vector_products", 0)
                pp08_vps = pp08.get("cost", {}).get("vector_products", 0)
                if single_vps > pp08_vps:
                    gate_4_one_execution = False
                    gate_4_reasons.append(f"{label}/{cache_state}: vector_products {single_vps} > pp08 {pp08_vps}")
                if pp08.get("choice") in ("Chebyshev", "Laguerre") and pp08_vps > 0 and pp08.get("cost", {}).get("executed_backends", 0) >= 2:
                    if single_vps >= pp08_vps:
                        gate_4_one_execution = False
                        gate_4_reasons.append(f"{label}/{cache_state}: vector_products {single_vps} not strictly lower than pp08 {pp08_vps}")

    # Gate 5: Niche benefit
    niche_size = len(niche_cases)
    gate_5_niche_benefit = niche_size >= 6
    gate_5_reasons = []
    if not gate_5_niche_benefit:
        gate_5_reasons.append("niche empty")

    all_gates_pass = (
        gate_1_certification
        and gate_2_parity
        and gate_3_accounting
        and gate_4_one_execution
        and gate_5_niche_benefit
    )

    verdict = "PASS" if all_gates_pass else "FAIL"
    failed_gates = []
    if not gate_1_certification:
        failed_gates.append("item_1_certification")
    if not gate_2_parity:
        failed_gates.append("item_2_parity")
    if not gate_3_accounting:
        failed_gates.append("item_3_accounting")
    if not gate_4_one_execution:
        failed_gates.append("item_4_one_execution")
    if not gate_5_niche_benefit:
        failed_gates.append("item_5_niche_benefit")

    results = {
        "schema": SCHEMA,
        "verdict": verdict,
        "failed_gates": failed_gates,
        "gate_results": {
            "item_1_certification": gate_1_certification,
            "item_2_parity": gate_2_parity,
            "item_3_accounting": gate_3_accounting,
            "item_4_one_execution": gate_4_one_execution,
            "item_5_niche_benefit": gate_5_niche_benefit,
        },
        "niche_size": niche_size,
        "niche_reasons": gate_5_reasons,
        "failure_details": {
            "gate_1": gate_1_reasons[:20],
            "gate_2": gate_2_reasons[:20],
            "gate_3": gate_3_reasons[:20],
            "gate_4": gate_4_reasons[:20],
        },
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(results, indent=2))
    print(f"Results written to {args.output}")
    print(f"Verdict: {verdict}")
    print(f"Gate 1 Certification: {gate_1_certification}")
    print(f"Gate 2 Parity: {gate_2_parity}")
    print(f"Gate 3 Accounting: {gate_3_accounting}")
    print(f"Gate 4 One Execution: {gate_4_one_execution}")
    print(f"Gate 5 Niche Benefit: {gate_5_niche_benefit} ({gate_5_reasons})")

    # Exit code: 0 for PASS, 1 for FAIL
    sys.exit(0 if all_gates_pass else 1)


if __name__ == "__main__":
    main()
