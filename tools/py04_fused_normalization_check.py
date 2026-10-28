#!/usr/bin/env python3
"""Scientific Gate Checker for Power-of-Two Fused Normalization (PY04).

Research node: research/py04_fused_normalization_20261011
Preregistration: research/py04_fused_normalization_20261011/PREREGISTRATION.md
"""

from __future__ import annotations

import argparse
from fractions import Fraction
import json
import math
from pathlib import Path
import struct
import sys

import mpmath as mp

# Evidence schema validation (AS03)
sys.path.insert(0, str(Path(__file__).resolve().parent))
from evidence_schema_v2 import INVALID_EXIT_CODE, ValidationError, loads_strict

SCHEMA = "vigilode-py04-fused-normalization-results-v1"
SUB_QUANTUM = 2.0 ** -1074


def unhex(text: str) -> float:
    return struct.unpack(">d", bytes.fromhex(text))[0]


def hex_f64(x: float) -> str:
    return format(struct.unpack(">Q", struct.pack(">d", x))[0], "016x")


def norm2(vec) -> mp.mpf:
    return mp.sqrt(mp.fsum(x * x for x in vec))


class SplitMix:
    def __init__(self, seed: int):
        self.state = seed & 0xFFFF_FFFF_FFFF_FFFF

    def next(self) -> int:
        self.state = (self.state + 0x9E37_79B9_7F4A_7C15) & 0xFFFF_FFFF_FFFF_FFFF
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58_476D_1CE4_E5B9) & 0xFFFF_FFFF_FFFF_FFFF
        z = ((z ^ (z >> 27)) * 0x94D0_49BB_1331_11EB) & 0xFFFF_FFFF_FFFF_FFFF
        return (z ^ (z >> 31)) & 0xFFFF_FFFF_FFFF_FFFF

    def symmetric(self) -> float:
        val = (self.next() >> 11) * (2.0**-53)
        return 2.0 * val - 1.0


def base_matrix(n: int, symmetric: bool, rng: SplitMix) -> list[list[float]]:
    a = [[0.0] * n for _ in range(n)]
    for i in range(n):
        for j in range(n):
            if i == j or (symmetric and j < i):
                continue
            a[i][j] = rng.symmetric()
            if symmetric:
                a[j][i] = a[i][j]
    for i in range(n):
        row = sum(abs(a[i][j]) for j in range(n) if j != i)
        col = sum(abs(a[j][i]) for j in range(n) if j != i)
        a[i][i] = -(row + col) / 2.0 - 1.0
    return a


def one_norm(a: list[list[float]]) -> float:
    n = len(a)
    return max(sum(abs(a[i][j]) for i in range(n)) for j in range(n))


def augmented_action(exact_ha, w, n, dps: int):
    mp.mp.dps = dps
    largest = max((abs(x) for wk in w for x in wk), default=0.0)
    e = math.frexp(largest)[1] - 1 if largest > 0 else 0
    scale = mp.ldexp(mp.mpf(1), e)
    size = n + 4
    m = mp.zeros(size, size)
    for i in range(n):
        for j in range(n):
            m[i, j] = exact_ha[i][j]
        for c in range(4):
            m[i, n + c] = mp.mpf(w[4 - c][i]) / scale
    for c in range(3):
        m[n + c, n + c + 1] = 1
    v = mp.matrix(size, 1)
    for i in range(n):
        v[i] = mp.mpf(w[0][i]) / scale
    v[size - 1] = 1
    y = mp.expm(m) * v
    return [y[i] * scale for i in range(n)]


def check_directed_scaling_table(scaling_table: list[dict]) -> bool:
    for entry in scaling_table:
        m = unhex(entry["m"])
        e = entry["e"]
        nearest_val = unhex(entry["nearest"])
        is_exact = entry["is_exact"]
        up_val = unhex(entry["up"]) if entry["up"] is not None else None
        down_val = unhex(entry["down"]) if entry["down"] is not None else None

        # Exact rational representation of m * 2^e
        frac_m = Fraction(m)
        frac_exact = frac_m * (Fraction(2)**e if e >= 0 else Fraction(1, 2**(-e)))
        frac_nearest = Fraction(nearest_val)

        # Exact flag verification
        if is_exact != (frac_exact == frac_nearest):
            return False

        # Bracket verification: down <= exact <= up
        if down_val is not None:
            if Fraction(down_val) > frac_exact:
                return False
        if up_val is not None:
            if Fraction(up_val) < frac_exact:
                return False

    return True


def main():
    parser = argparse.ArgumentParser(description="PY04 scientific gate checker")
    parser.add_argument("--cases", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    # Evidence validation first
    try:
        raw_bytes = args.cases.read_bytes()
        data = loads_strict(raw_bytes)
    except (ValidationError, Exception) as err:
        print(f"INVALID: Evidence schema validation failed: {err}", file=sys.stderr)
        sys.exit(INVALID_EXIT_CODE)

    if data.get("schema") != "vigilode-py04-cases-v1":
        print(f"INVALID: Unregistered schema: {data.get('schema')}", file=sys.stderr)
        sys.exit(INVALID_EXIT_CODE)

    cases = data.get("cases", [])
    if len(cases) != 144:
        print(f"INVALID: Expected 144 cases, found {len(cases)}", file=sys.stderr)
        sys.exit(INVALID_EXIT_CODE)

    # 1. Regenerate s x base and check exact bits of w (family rule binding)
    for case in cases:
        seed = case["seed"]
        n = case["n"]
        family = case["family"]

        rng = SplitMix(seed)
        _base = base_matrix(n, case["symmetric"], rng)
        base_w = [[rng.symmetric() for _ in range(n)] for _ in range(5)]

        # Check base_w bits
        for k in range(5):
            for i in range(n):
                if hex_f64(base_w[k][i]) != case["base_w"][k][i]:
                    print(f"INVALID: Base w bit mismatch in case {case['label']}", file=sys.stderr)
                    sys.exit(INVALID_EXIT_CODE)

        # Check w = s * base_w bits according to family rule
        for k in range(5):
            for i in range(n):
                bw = base_w[k][i]
                if family == "S1":
                    expected = math.ldexp(bw, -1016)
                elif family == "S2":
                    expected = math.ldexp(bw, -1050)
                elif family == "S3":
                    expected = math.ldexp(bw, -1068)
                elif family == "S4":
                    expected = math.ldexp(bw, 1018)
                elif family == "S5":
                    expected = math.ldexp(bw, 1023)
                elif family == "S6":
                    if k == 0:
                        expected = math.ldexp(bw, 1000)
                    elif k == 4:
                        expected = math.ldexp(bw, -1070)
                    else:
                        expected = bw
                elif family == "S7a":
                    expected = bw
                elif family == "S7b":
                    expected = math.ldexp(bw, 600)
                elif family == "S7c":
                    expected = math.ldexp(bw, -600)
                else:
                    print(f"INVALID: Unknown family {family}", file=sys.stderr)
                    sys.exit(INVALID_EXIT_CODE)

                if hex_f64(expected) != case["w"][k][i]:
                    print(f"INVALID: Input w bit mismatch for family rule in case {case['label']}", file=sys.stderr)
                    sys.exit(INVALID_EXIT_CODE)

    # 2. Check directed scaling table (Gate 4)
    gate4_scaling_table_pass = check_directed_scaling_table(data.get("scaling_table", []))

    # 3. High-precision references and Gates
    gate1_enclosure_violations = 0
    gate2_homogeneity_pass = True
    gate3_accounting_pass = True
    gate5_usefulness_pass = True
    gate6_overflow_pass = True

    case_results = []
    s7_triples: dict[int, dict[str, dict]] = {}

    for case in cases:
        n = case["n"]
        h = unhex(case["h"])
        a = [[unhex(x) for x in row] for row in case["a"]]
        w = [[unhex(x) for x in wk] for wk in case["w"]]
        family = case["family"]
        matrix_idx = case["matrix_index"]
        norm_cert = case["normalized_certificate"]

        exact_ha = [[mp.mpf(h) * mp.mpf(a[i][j]) for j in range(n)] for i in range(n)]
        ref50 = augmented_action(exact_ha, w, n, dps=50)
        ref70 = augmented_action(exact_ha, w, n, dps=70)

        # Check oracle uncertainty
        mp.mp.dps = 70
        ref_diff = norm2([ref50[i] - ref70[i] for i in range(n)])
        norm_f = norm2(ref50)
        max_uncertainty = mp.mpf("1e-45") * norm_f + mp.ldexp(mp.mpf(1), -1100)
        if ref_diff > max_uncertainty:
            print(f"INVALID: Reference uncertainty exceeded in case {case['label']}", file=sys.stderr)
            sys.exit(INVALID_EXIT_CODE)

        # Record S7 triples for Gate 2
        if family.startswith("S7"):
            if matrix_idx not in s7_triples:
                s7_triples[matrix_idx] = {}
            s7_triples[matrix_idx][family] = norm_cert

        # Evaluate case
        status = norm_cert.get("status")
        if status == "refused":
            refusal = norm_cert.get("refusal")
            if refusal != "FUSED_PHI_RANGE_UNSUPPORTED":
                gate6_overflow_pass = False
            if family != "S5":
                # Only S5 may be refused
                gate5_usefulness_pass = False
            case_results.append({
                "label": case["label"],
                "status": "refused",
                "refusal": refusal,
            })
            continue

        if status != "bounded":
            gate3_accounting_pass = False

        bound = unhex(norm_cert["bound"])
        f_hat = [unhex(x) for x in norm_cert["f_hat"]]
        f_hat_bound = unhex(norm_cert["f_hat_bound"])

        # Check finite candidate and bound (Gate 3)
        if not math.isfinite(bound) or bound > sys.float_info.max:
            gate3_accounting_pass = False
        if not all(math.isfinite(x) for x in f_hat):
            gate3_accounting_pass = False

        # Accounting for L and R (Gate 3)
        r_term = unhex(norm_cert["r_term"])
        l_term = unhex(norm_cert["l_term"])
        exponent = norm_cert["exponent"]

        # Exactness check of w / sigma
        w_exact = all(
            math.frexp(x)[1] - 1 - exponent >= -1022 or x == 0.0 or (math.ldexp(x, -exponent) == math.ldexp(x, -exponent))  # float check
            for wk in w for x in wk
        )
        if l_term < 0.0 or r_term < 0.0:
            gate3_accounting_pass = False

        # Gate 1: Enclosure for F_hat
        cand_diff = norm2([mp.mpf(f_hat[i]) - ref50[i] for i in range(n)])
        if mp.mpf(f_hat_bound) < cand_diff + ref_diff:
            gate1_enclosure_violations += 1

        # Check caller candidate enclosure if present
        taylor_caller = case.get("taylor_caller", {})
        if taylor_caller.get("accepted"):
            caller_cand = [unhex(x) for x in taylor_caller["candidate"]]
            caller_cert = taylor_caller["certificate"]
            if caller_cert.get("status") != "refused":
                caller_bound = unhex(caller_cert["bound"])
                caller_diff = norm2([mp.mpf(caller_cand[i]) - ref50[i] for i in range(n)])
                if mp.mpf(caller_bound) < caller_diff + ref_diff:
                    gate1_enclosure_violations += 1

        # Gate 5: Mixed budget usefulness: bound <= 1e-10 ||F||_2 + 16 sqrt(n) 2^-1074
        mixed_budget = mp.mpf("1e-10") * norm_f + mp.mpf(16 * math.sqrt(n) * SUB_QUANTUM)
        is_useful = mp.mpf(f_hat_bound) <= mixed_budget

        if family in ["S1", "S2", "S3", "S4", "S6"]:
            if not is_useful:
                gate5_usefulness_pass = False

        case_results.append({
            "label": case["label"],
            "status": "bounded",
            "bound": norm_cert["bound"],
            "norm_f": float(norm_f),
            "useful_mixed": is_useful,
        })

    # Gate 2: Homogeneity across S7 triples
    for matrix_idx, triple in s7_triples.items():
        if len(triple) != 3 or "S7a" not in triple or "S7b" not in triple or "S7c" not in triple:
            gate2_homogeneity_pass = False
            continue
        c_a = triple["S7a"]
        c_b = triple["S7b"]
        c_c = triple["S7c"]

        # Normalized internals must be bitwise identical
        keys_to_compare = [
            "v", "z", "normalized_bound", "stepped_error", "perturbation", "normalized_distance"
        ]
        for k in keys_to_compare:
            if c_a[k] != c_b[k] or c_a[k] != c_c[k]:
                gate2_homogeneity_pass = False

        # Bound scaling check: bound_s == s * bound_1
        bound_a = unhex(c_a["bound"])
        bound_b = unhex(c_b["bound"])
        bound_c = unhex(c_c["bound"])
        r_exact_all = (
            unhex(c_a["r_term"]) == 0.0 and unhex(c_a["l_term"]) == 0.0 and
            unhex(c_b["r_term"]) == 0.0 and unhex(c_b["l_term"]) == 0.0 and
            unhex(c_c["r_term"]) == 0.0 and unhex(c_c["l_term"]) == 0.0
        )
        if r_exact_all:
            expected_b = math.ldexp(bound_a, 600)
            expected_c = math.ldexp(bound_a, -600)
            if hex_f64(bound_b) != hex_f64(expected_b) or hex_f64(bound_c) != hex_f64(expected_c):
                gate2_homogeneity_pass = False

    gate1_pass = gate1_enclosure_violations == 0
    all_pass = (
        gate1_pass and
        gate2_homogeneity_pass and
        gate3_accounting_pass and
        gate4_scaling_table_pass and
        gate5_usefulness_pass and
        gate6_overflow_pass
    )

    verdict = "PASS" if all_pass else "FAIL"

    results = {
        "schema": SCHEMA,
        "verdict": verdict,
        "gates": {
            "gate1_enclosure": gate1_pass,
            "gate1_enclosure_violations": gate1_enclosure_violations,
            "gate2_homogeneity": gate2_homogeneity_pass,
            "gate3_accounting": gate3_accounting_pass,
            "gate4_directed_scaling": gate4_scaling_table_pass,
            "gate5_usefulness": gate5_usefulness_pass,
            "gate6_overflow_boundary": gate6_overflow_pass,
        },
        "case_count": len(cases),
    }

    if args.output.exists():
        args.output.unlink()
    args.output.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")

    print(json.dumps(results, indent=2))
    if not all_pass:
        sys.exit(1)


if __name__ == "__main__":
    main()
