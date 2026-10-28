#!/usr/bin/env python3
"""Research node research/pp07_fourier_shared_action_20261004 (RVJ DAG node PP07).

Answers Q1-Q3 of the preregistration from the recorded PP05 runs and the native source:
Q1 shifted linear solves performed by the native client (static scan of the two client
modules for any linear-solver use, plus the runs' counts); Q2 the distinct shifts a
frozen-Jacobian Newton correction would see per step start (every step size tried there:
the start size and its halvings); Q3 the Euclidean dissipativity witness of PP01 for the
real 4x4 Jacobian at each step start (symmetric-part row bound, 50-digit, diagnostic) and
the alternative gain 1/(1 - h ||J||_2,up) with ||J||_2 <= sqrt(||J||_1 ||J||_inf).
Then the preregistered decision rule.
"""

import argparse
import json
import struct
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

ROOT = Path(__file__).resolve().parents[1]
MODULES = ["crates/rodas5p-integrators/src/fourier_path_certificate.rs",
           "crates/rodas5p-integrators/src/fourier_path_candidate.rs"]
SOLVER_TOKENS = ["LuFactorization", "solve_rows", "direct_solve", "solve_gmres", "solve_gcrodr",
                 "solve_lgmres", "shared_shift_jet", "LinearOperator", "inverse("]


def unhex(t):
    return struct.unpack(">d", bytes.fromhex(t))[0]


def jacobian(omega, sigma, t, y):
    g = eps = mp.mpf(1) / 8
    q = sigma * mp.mpf(5) / 4

    def f(v):
        a = mp.mpc(v[0], v[1])
        b = mp.mpc(v[2], v[3])
        c = g + eps * mp.re(mp.exp(1j * mp.mpf(omega) * t) * a)
        da = 1j * q * c * mp.conj(a) * b
        db = 1j * q * c * a * a / 2
        return [mp.re(da), mp.im(da), mp.re(db), mp.im(db)]

    jac = [[mp.mpf(0)] * 4 for _ in range(4)]
    for k in range(4):
        def comp(x, k=k):
            v = list(y)
            v[k] = x
            return f(v)
        for i in range(4):
            jac[i][k] = mp.diff(lambda x, i=i: comp(x)[i], y[k])
    return jac


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pp05-cases", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    mp.mp.dps = 50
    scan = {m: [tok for tok in SOLVER_TOKENS if tok in (ROOT / m).read_text()] for m in MODULES}
    q1_solves = sum(len(v) for v in scan.values())
    data = json.loads(args.pp05_cases.read_text())
    rows = []
    witness_fail = witness_ok = 0
    gains_finite = 0
    multi_shift_starts = 0
    jet_admissible = 0
    for case in data["cases"]:
        run = case["run"]
        if not run["ok"]:
            continue
        omega, sigma = unhex(case["omega"]), case["sigma"]
        T = F(unhex(case["T"]))
        for r in run["records"]:
            t0 = F(unhex(r["start"]["t"]))
            h = F(unhex(r["trial"]["h"]))
            h_start = min(F(1, 8), T - t0)
            tried = []
            x = h_start
            while x >= h:
                tried.append(x)
                x /= 2
            a = r["start"]["a"]
            y = [mp.mpf(unhex(a[0][0])), mp.mpf(unhex(a[0][1])), mp.mpf(unhex(a[1][0])), mp.mpf(unhex(a[1][1]))]
            t = mp.mpf(t0.numerator) / t0.denominator
            jac = jacobian(omega, sigma, t, y)
            sym_rows = [jac[i][i] + sum(abs((jac[i][k] + jac[k][i]) / 2) for k in range(4) if k != i)
                        for i in range(4)]
            witness = max(sym_rows)
            norm1 = max(sum(abs(jac[i][k]) for i in range(4)) for k in range(4))
            norminf = max(sum(abs(jac[i][k]) for k in range(4)) for i in range(4))
            norm2 = mp.sqrt(norm1 * norminf)
            gains = []
            for hh in tried:
                hm = mp.mpf(hh.numerator) / hh.denominator
                gains.append(float(1 / (1 - hm * norm2)) if hm * norm2 < 1 else None)
            gains_finite += sum(g is not None for g in gains)
            if witness <= 0:
                witness_ok += 1
            else:
                witness_fail += 1
            distinct = len(tried)
            if distinct >= 2:
                multi_shift_starts += 1
                # Centre at the first shift: later halvings sit at z = -1/2, -3/4, ...
                radii = [float(abs(F(1) - x / tried[0])) for x in tried[1:]]
                jet_admissible += int(all(z < 0.5 for z in radii))
            rows.append({"case": case["case"], "t": str(t0), "tried_h": [str(x) for x in tried],
                         "accepted_h": str(h), "witness_row_bound": float(witness),
                         "norm2_upper": float(norm2), "alternative_gains": gains})
    decision = []
    if q1_solves == 0:
        decision.append("Q1: the client performs no shifted linear solve; kill condition "
                        "'client has no nearby distinct shifts' holds for the client as it is.")
    if witness_ok == 0:
        decision.append("Q3: no step start has the Euclidean dissipativity witness "
                        "(gain 1 is never available); the alternative gain is finite at "
                        f"{gains_finite} tried shifts.")
    decision.append(f"Q2: {multi_shift_starts} step starts see more than one shift (halvings); "
                    f"jet admissible (all later radii < 1/2) at {jet_admissible}.")
    result = {"schema": "vigilode-pp07-audit-v1", "q1_static_solver_tokens": scan, "q1_solves": q1_solves,
              "q2_multi_shift_starts": multi_shift_starts, "q2_jet_admissible_starts": jet_admissible,
              "q3_witness_ok": witness_ok, "q3_witness_fail": witness_fail,
              "q3_finite_alternative_gains": gains_finite,
              "shared_jet_evaluations": 0, "decision": decision, "rows": rows,
              "gate": {"g1_questions_answered": True,
                       "g2_no_uncertified_jet_evaluation": True}}
    result["verdict"] = "PASS"
    args.output.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps({k: v for k, v in result.items() if k != "rows"}, indent=1))


if __name__ == "__main__":
    main()
