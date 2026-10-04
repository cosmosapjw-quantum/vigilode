#!/usr/bin/env python3
"""50-digit check of research node research/pp11_taylor_fused_total_20261004 (RVJ DAG node
PP11): total-target certificates of the scaled-Taylor fused phi action.

Reference: F = sum_{k=0}^{4} phi_k(hA) w_k with the binary64 A, h, w_k taken as exact reals,
computed as the top block of mpmath.expm of the exact augmented matrix
M = [[hA, W], [0, J]] times v = [w_0; e_4] (W = [w_4, w_3, w_2, w_1], J the 4x4 upper shift)
at 50 digits. hA is formed exactly (a product of two binary64 numbers has at most 106
significant bits). F is linear in (w_0, ..., w_4) jointly, so the w_k are first divided by a
power of two 2^e near their largest magnitude (exact in mpmath's binary arithmetic), the
expm is taken of the normalized matrix (v = [w_0 / 2^e; e_4]), and the top block is
multiplied back by 2^e (exact). This keeps the 1e100 and 1e-310 variants inside expm's
well-scaled range. The reference is recomputed at 70 digits and the largest relative
difference is reported.

Gates (preregistration): G1 every reported bound >= the 50-digit error of the candidate;
G2 the perturbation term is > 0 whenever hA rounds and == 0 when it does not (the
power-of-two control must not round); type separation is recorded by the contract test.
G3 is reported only. Not gated, also reported: whether E1 encloses ||y - exp(M~) v|| and the
perturbation encloses ||exp(M) v - exp(M~) v|| on the full augmented vector.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
from pathlib import Path

import mpmath as mp

SCHEMA = "vigilode-pp11-taylor-fused-total-check-v1"
TYPE_SEPARATION = (
    "contract test pp11_taylor_fused_total_contracts::"
    "an_exp_certificate_is_a_different_type_from_a_fused_certificate and the compile_fail "
    "doctest of crates/rodas5p-core/src/taylor_phi_total.rs"
)
USEFUL = mp.mpf("1e-10")


def f64(text: str) -> float:
    return struct.unpack(">d", bytes.fromhex(text))[0]


def norm2(x) -> mp.mpf:
    return mp.sqrt(mp.fsum(xi * xi for xi in x))


def augmented_action(top_left, w, n):
    """exp(M) v for M = [[top_left, W], [0, J]], v = [w_0; e_4], via the normalization in the
    module docstring. Returns the full augmented vector (top block rescaled)."""
    largest = max((abs(x) for wk in w for x in wk), default=0.0)
    e = math.frexp(largest)[1] - 1 if largest > 0 else 0
    scale = mp.ldexp(mp.mpf(1), e)
    size = n + 4
    m = mp.zeros(size, size)
    for i in range(n):
        for j in range(n):
            m[i, j] = top_left[i][j]
        for c in range(4):
            m[i, n + c] = mp.mpf(w[4 - c][i]) / scale
    for c in range(3):
        m[n + c, n + c + 1] = 1
    v = mp.matrix(size, 1)
    for i in range(n):
        v[i] = mp.mpf(w[0][i]) / scale
    v[size - 1] = 1
    y = mp.expm(m) * v
    return [y[i] * scale for i in range(n)] + [y[i] for i in range(n, size)]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    data = json.loads(args.cases.read_text())
    rows = []
    g1 = True
    g2 = True
    control_rounds = False
    aux_e1 = True
    aux_pert = True
    status_consistent = True
    worst_reference_difference = mp.mpf(0)
    for case in data["cases"]:
        mp.mp.dps = 50
        n = case["n"]
        h = f64(case["h"])
        a = [[f64(x) for x in row] for row in case["a"]]
        w = [[f64(x) for x in wk] for wk in case["w"]]
        u = [mp.mpf(f64(x)) for x in case["candidate"]]
        cert = case["certificate"]
        exact_ha = [[mp.mpf(h) * mp.mpf(a[i][j]) for j in range(n)] for i in range(n)]
        stored_ha = [[mp.mpf(h * a[i][j]) for j in range(n)] for i in range(n)]
        rounds = any(exact_ha[i][j] != stored_ha[i][j] for i in range(n) for j in range(n))
        full = augmented_action(exact_ha, w, n)
        full_stored = augmented_action(stored_ha, w, n)
        target = full[:n]
        target_norm = norm2(target)
        error = norm2([u[i] - target[i] for i in range(n)])
        # Reference self-check at 70 digits.
        mp.mp.dps = 70
        exact70 = [[mp.mpf(h) * mp.mpf(a[i][j]) for j in range(n)] for i in range(n)]
        target70 = augmented_action(exact70, w, n)[:n]
        diff = norm2([target70[i] - target[i] for i in range(n)])
        rel = diff / norm2(target70) if norm2(target70) > 0 else diff
        mp.mp.dps = 50
        worst_reference_difference = max(worst_reference_difference, mp.mpf(rel))

        status = cert["status"]
        bound = f64(cert["bound"])
        perturbation = f64(cert["perturbation"])
        e1 = f64(cert["stepped_error"])
        distance = f64(cert["candidate_distance"])
        encloses = bool(error <= mp.mpf(bound)) if math.isfinite(bound) else True
        g1 &= encloses
        status_consistent &= math.isfinite(bound) == (status == "bounded")
        g2_case = (perturbation > 0) if rounds else (perturbation == 0)
        g2 &= bool(g2_case)
        if case["h_power_of_two"]:
            control_rounds |= rounds
        # Auxiliary (not gated): the two inner inequalities on the full vector.
        y = [mp.mpf(f64(x)) if math.isfinite(f64(x)) else None for x in cert["stepped_candidate"]]
        e1_actual = None
        if all(yi is not None for yi in y):
            e1_actual = norm2([y[i] - full_stored[i] for i in range(n + 4)])
            if math.isfinite(e1):
                aux_e1 &= bool(e1_actual <= mp.mpf(e1))
        pert_actual = norm2([full[i] - full_stored[i] for i in range(n + 4)])
        if math.isfinite(perturbation):
            aux_pert &= bool(pert_actual <= mp.mpf(perturbation))
        useful = bool(math.isfinite(bound) and mp.mpf(bound) <= USEFUL * target_norm)
        rows.append({
            "label": case["label"], "variant": case["variant"], "symmetric": case["symmetric"],
            "n": n, "target_hA_one_norm": case["target_hA_one_norm"],
            "h_power_of_two": case["h_power_of_two"], "hA_rounds": bool(rounds),
            "taylor_accepted": case["taylor"]["accepted"],
            "taylor_rejection": case["taylor"].get("rejection"),
            "candidate_source": case["candidate_source"],
            "status": status, "unbounded_reason": cert["unbounded_reason"],
            "chosen_metric": cert["chosen_metric"],
            "steps": [att["steps"] for att in cert["attempts"]],
            "F_norm": mp.nstr(target_norm, 8),
            "true_error": mp.nstr(error, 8),
            "bound": repr(bound), "candidate_distance": repr(distance),
            "stepped_error": repr(e1), "perturbation": repr(perturbation),
            "omega": repr(f64(cert["omega"])), "delta_two_norm": repr(f64(cert["delta_two_norm"])),
            "dominant": cert["dominant"],
            "bound_over_error": mp.nstr(mp.mpf(bound) / error, 6)
            if math.isfinite(bound) and error > 0 else None,
            "relative_bound": mp.nstr(mp.mpf(bound) / target_norm, 6)
            if math.isfinite(bound) and target_norm > 0 else None,
            "relative_error": mp.nstr(error / target_norm, 6) if target_norm > 0 else None,
            "encloses": encloses, "g2": bool(g2_case), "useful_1e-10": useful,
            "aux_stepped_actual": mp.nstr(e1_actual, 6) if e1_actual is not None else None,
            "aux_perturbation_actual": mp.nstr(pert_actual, 6),
            "reference_70_digit_relative_difference": mp.nstr(rel, 3),
        })
        print(f"{case['label']}: {status} bound {bound:.3e} error {mp.nstr(error, 4)} "
              f"||F|| {mp.nstr(target_norm, 4)} dominant {cert['dominant']} encloses {encloses}")
    g2 &= not control_rounds
    bounded = [r for r in rows if r["status"] == "bounded"]
    ratios = [mp.mpf(r["bound_over_error"]) for r in bounded if r["bound_over_error"] is not None]
    dominant = {}
    for r in bounded:
        dominant[r["dominant"]] = dominant.get(r["dominant"], 0) + 1
    by_variant = {}
    for r in rows:
        entry = by_variant.setdefault(r["variant"], {"cases": 0, "bounded": 0, "useful_1e-10": 0,
                                                     "taylor_accepted": 0})
        entry["cases"] += 1
        entry["bounded"] += r["status"] == "bounded"
        entry["useful_1e-10"] += r["useful_1e-10"]
        entry["taylor_accepted"] += bool(r["taylor_accepted"])
    g3 = {
        "cases": len(rows), "bounded": len(bounded),
        "unbounded": len(rows) - len(bounded),
        "useful_1e-10": sum(r["useful_1e-10"] for r in rows),
        "bound_over_error_min": mp.nstr(min(ratios), 4) if ratios else None,
        "bound_over_error_median": mp.nstr(sorted(ratios)[len(ratios) // 2], 4) if ratios else None,
        "bound_over_error_max": mp.nstr(max(ratios), 4) if ratios else None,
        "dominant_term_counts_bounded": dominant,
        "by_variant": by_variant,
    }
    gate = {"G1_enclosure": bool(g1), "G2_perturbation_iff_rounding": bool(g2),
            "G2_type_separation": TYPE_SEPARATION, "G3_reported": g3}
    verdict = "PASS" if gate["G1_enclosure"] and gate["G2_perturbation_iff_rounding"] else "FAIL"
    report = {
        "schema": SCHEMA, "mpmath": mp.__version__, "dps": 50,
        "reference": "mpmath.expm of the exact augmented matrix (normalized by a power of two), "
                     "50 digits; recomputed at 70 digits",
        "worst_reference_70_digit_relative_difference": mp.nstr(worst_reference_difference, 3),
        "auxiliary_not_gated": {"stepped_E1_encloses_full_vector": bool(aux_e1),
                                "perturbation_encloses_full_vector": bool(aux_pert),
                                "status_matches_finiteness": bool(status_consistent)},
        "rows": rows, "gate": gate, "verdict": verdict,
    }
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({"G1": gate["G1_enclosure"], "G2": gate["G2_perturbation_iff_rounding"],
                      "G3": g3, "verdict": verdict}))


if __name__ == "__main__":
    main()
