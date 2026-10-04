#!/usr/bin/env python3
"""Gate of research node research/pp06_fourier_fft_candidate_20261004 (RVJ DAG node PP06).

G1: the FFT right side equals the direct noncyclic one within 1e-12 relative on every
predictor call of the two padded runs. G2: every FFT-built committed state and interior
point encloses the PP05 reference (same functions as tools/pp05_fourier_check.py). G3: the
under-padded grid is refused without the override, and with it no candidate is committed
(rejected or budget not met). G4: unit test fourier_path_candidate::roundtrip_and_
conjugation_order (recorded in the node, not re-run here).
"""

import argparse
import importlib.util
import json
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

spec = importlib.util.spec_from_file_location("pp05", Path(__file__).with_name("pp05_fourier_check.py"))
pp05 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pp05)


def check_run(entry):
    run = entry["run"]
    if not run["ok"]:
        return {"ok": False, "error": run["error"]}, False
    omega = pp05.unhex(entry["omega"])
    recs = run["records"]
    times = []
    for r in recs:
        t0, h = pp05.fr(r["start"]["t"]), pp05.fr(r["trial"]["h"])
        times += [t0 + h] + [t0 + tau * h for tau in (F(1, 4), F(1, 2), F(3, 4))]
    if abs(omega) <= 40:
        ref = pp05.reference_mp(omega, 1, times)
        diff = None
    else:
        ref, diff = pp05.reference_scipy(omega, 1, times)
    mp.mp.dps = 30
    ok = True
    worst = worst_i = 0.0
    for r in recs:
        tr = r["trial"]
        t0, h = pp05.fr(r["start"]["t"]), pp05.fr(tr["h"])
        bound = mp.mpf(pp05.unhex(r["state"]["error"]))
        a_ = [mp.mpc(pp05.unhex(z[0]), pp05.unhex(z[1])) for z in r["state"]["a"]]
        e = max(pp05.err1(a_[0], ref[t0 + h][0]), pp05.err1(a_[1], ref[t0 + h][1]))
        worst = max(worst, float(e / bound))
        ok &= bool(e <= bound) and tr["accepted"]
        ib = mp.mpf(pp05.unhex(tr["growth"])) * (mp.mpf(pp05.unhex(r["start"]["error"]))
                                                 + mp.mpf(pp05.unhex(tr["start_mismatch"]))
                                                 + mp.mpf(pp05.unhex(tr["residual"])))
        a = mp.mpf(omega) * pp05.mq(h)
        for tau in (F(1, 4), F(1, 2), F(3, 4)):
            vals = [pp05.path_value(p, a, tau) for p in tr["path"]]
            ei = max(pp05.err1(vals[0], ref[t0 + tau * h][0]), pp05.err1(vals[1], ref[t0 + tau * h][1]))
            worst_i = max(worst_i, float(ei / ib))
            ok &= bool(ei <= ib)
    return {"ok": True, "steps": len(recs), "counts": run["counts"], "fft_work": entry["fft_work"],
            "predictor_calls": entry["predictor_calls"],
            "worst_relative_difference_fft_vs_direct": entry["worst_relative_difference_fft_vs_direct"],
            "worst_error_over_bound": worst, "worst_interior_error_over_bound": worst_i,
            "reference_difference": diff,
            "final_error_bound": pp05.unhex(recs[-1]["state"]["error"])}, ok


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cases", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    runs = json.loads(args.cases.read_text())["runs"]
    out = {}
    g1 = all(runs[k]["worst_relative_difference_fft_vs_direct"] <= 1e-12 for k in ("forty", "fast"))
    g2 = True
    for k in ("forty", "fast"):
        out[k], ok = check_run(runs[k])
        g2 &= ok
    alias = runs["forty_alias"]
    alias_summary, alias_enclosed = check_run(alias)
    committed = alias_summary.get("steps", 0) if alias["run"]["ok"] else 0
    alias_summary["refused_without_override"] = alias["refused_without_override"]
    alias_summary["committed_steps"] = committed
    alias_summary["committed_steps_enclose_reference"] = alias_enclosed if committed else None
    out["forty_alias"] = alias_summary
    g3 = alias["refused_without_override"] > 0 and committed == 0
    result = {"schema": "vigilode-pp06-check-v1", "runs": out,
              "gate": {"g1_fft_equals_direct": g1, "g2_enclosure": g2, "g3_alias_refused_and_rejected": g3,
                       "g4": "unit test fourier_path_candidate::tests::roundtrip_and_conjugation_order"}}
    result["verdict"] = "PASS" if (g1 and g2 and g3) else "FAIL"
    args.output.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps(result, indent=1))


if __name__ == "__main__":
    main()
