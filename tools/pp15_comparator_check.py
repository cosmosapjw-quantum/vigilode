#!/usr/bin/env python3
"""Gate of research node research/pp15_fourier_comparator_20261004 (RVJ DAG node PP15).

Final errors at T = 1/2 against the PP05 reference (complex 1-norm, max over a and b). G1:
every certified arm's bound encloses its actual error. G2: for each omega, the RODAS5P
tolerance whose actual error is closest (in log) to the client's certified bound is named.
G3 (independent review statement) is recorded separately in REVIEW.md. No speed conclusion.
"""

import argparse
import importlib.util
import json
import math
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

spec = importlib.util.spec_from_file_location("pp05", Path(__file__).with_name("pp05_fourier_check.py"))
pp05 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pp05)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cases", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    rows = json.loads(args.cases.read_text())["rows"]
    g1 = True
    out = []
    for row in rows:
        omega = pp05.unhex(row["omega"])
        if abs(omega) <= 40:
            ref = pp05.reference_mp(omega, 1, [F(1, 2)])[F(1, 2)]
            refdiff = None
        else:
            r, refdiff = pp05.reference_scipy(omega, 1, [F(1, 2)])
            ref = r[F(1, 2)]
        mp.mp.dps = 30
        entry = {"omega": omega, "reference_difference": refdiff, "arms": {}}
        for arm in ("client_direct", "client_fft", "rodas_1e-8", "rodas_1e-10"):
            x = row[arm]
            if not x["ok"]:
                entry["arms"][arm] = {"ok": False, "error": x["error"]}
                continue
            y = [mp.mpf(pp05.unhex(v)) for v in x["final"]]
            t_final = pp05.unhex(x["t_final"])
            err = max(pp05.err1(mp.mpc(y[0], y[1]), ref[0]), pp05.err1(mp.mpc(y[2], y[3]), ref[1]))
            e = {"ok": True, "t_final": t_final, "actual_error": float(err), "certified": x["certified"]}
            if x["certified"]:
                bound = pp05.unhex(x["error_bound"])
                e["bound"] = bound
                e["encloses"] = bool(err <= bound)
                g1 &= e["encloses"] and t_final == 0.5
                for k in ("steps", "candidate_builds", "candidate_rejections", "predictor_calls",
                          "certificate_calls", "step_halvings"):
                    e[k] = x[k]
            else:
                c = x["counters"]
                e.update({"success": x["success"], "attempts": x["attempts"], "accepted": x["accepted"],
                          "rejected": x["rejected"], "rhs_evaluations": c.get("rhs_evaluations"),
                          "rhs_calls": c.get("rhs_calls"), "jacobian_builds": c.get("jacobian_builds"),
                          "direct_factorizations": c.get("direct_factorizations"),
                          "direct_solve_calls": c.get("direct_solve_calls")})
            entry["arms"][arm] = e
        cb = entry["arms"].get("client_direct", {}).get("bound")
        if cb:
            cands = [(abs(math.log10(entry["arms"][k]["actual_error"] / cb)), k)
                     for k in ("rodas_1e-8", "rodas_1e-10")
                     if entry["arms"].get(k, {}).get("ok") and entry["arms"][k]["actual_error"] > 0]
            entry["closest_rodas_to_client_bound"] = min(cands)[1] if cands else None
        out.append(entry)
    result = {"schema": "vigilode-pp15-check-v1", "rows": out,
              "gate": {"g1_certified_arms_enclose": g1, "g2_matched_error_named": all(
                  "closest_rodas_to_client_bound" in r for r in out),
                       "g3": "see REVIEW.md (independent reviewer)"},
              "speed_conclusion": "none (timing authority HOLD; counters are in different units)"}
    result["verdict_pending_g3"] = "PASS" if (g1 and result["gate"]["g2_matched_error_named"]) else "FAIL"
    args.output.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps(result, indent=1))


if __name__ == "__main__":
    main()
