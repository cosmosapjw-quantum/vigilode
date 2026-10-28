#!/usr/bin/env python3
"""60-digit check of the SAFE-ENCLOSURE probes (research node
research/safe_enclosure_composition_20261004, RVJ DAG node SAFE-ENCLOSURE).

Every input is the exact binary64 value. C1: e^{-kappa tau} must lie in the reported
interval. C2: e^{count h a} must not exceed the reported upper bound. C3: the distance
max(m - lo, hi - m) from the rounded midpoint m must not exceed the reported radius.
Base and repaired values are checked separately; base violations are listed verbatim.
"""

from __future__ import annotations

import argparse
import json
import struct
from pathlib import Path

import mpmath as mp

mp.mp.dps = 60


def unhex(text: str) -> mp.mpf:
    return mp.mpf(struct.unpack(">d", bytes.fromhex(text))[0])


def rel(gap, ref):
    return float(gap / abs(ref)) if ref != 0 else float("inf")


def check_c1(rows):
    out = {"rows": len(rows), "base_violations": [], "repaired_violations": [],
           "max_base_under_relative": 0.0}
    for kappa_h, tau_h, blo, bhi, rlo, rhi in rows:
        truth = mp.e ** (-(unhex(kappa_h) * unhex(tau_h)))
        for arm, lo, hi in (("base", blo, bhi), ("repaired", rlo, rhi)):
            lo_v, hi_v = unhex(lo), unhex(hi)
            if not (lo_v <= truth <= hi_v):
                gap = truth - hi_v if truth > hi_v else lo_v - truth
                entry = {"kappa": kappa_h, "tau": tau_h, "lo": lo, "hi": hi,
                         "truth": mp.nstr(truth, 25), "under_relative": rel(gap, truth)}
                out[f"{arm}_violations"].append(entry)
                if arm == "base":
                    out["max_base_under_relative"] = max(out["max_base_under_relative"],
                                                         entry["under_relative"])
    return out


def check_c2(rows):
    out = {"rows": len(rows), "base_violations": [], "repaired_violations": [],
           "max_base_under_relative": 0.0, "negative_rate_rows": 0}
    for count, h_h, a_h, base_h, rep_h in rows:
        a = unhex(a_h)
        if a < 0:
            out["negative_rate_rows"] += 1
        truth = mp.e ** (count * unhex(h_h) * a)
        for arm, value in (("base", base_h), ("repaired", rep_h)):
            v = unhex(value)
            if truth > v:
                entry = {"count": count, "h": h_h, "a": a_h, "upper": value,
                         "truth": mp.nstr(truth, 25), "under_relative": rel(truth - v, truth)}
                out[f"{arm}_violations"].append(entry)
                if arm == "base":
                    out["max_base_under_relative"] = max(out["max_base_under_relative"],
                                                         entry["under_relative"])
    return out


def check_c3(rows):
    out = {"rows": len(rows), "base_violations": [], "repaired_violations": [],
           "max_base_under_ulps": 0.0}
    for lo_h, hi_h, bm, br, rm, rr in rows:
        lo, hi = unhex(lo_h), unhex(hi_h)
        for arm, m_h, r_h in (("base", bm, br), ("repaired", rm, rr)):
            m, r = unhex(m_h), unhex(r_h)
            distance = max(m - lo, hi - m)
            if distance > r:
                ulp = mp.mpf(2) ** (mp.floor(mp.log(abs(m), 2)) - 52) if m != 0 else mp.mpf(2) ** -1074
                entry = {"lo": lo_h, "hi": hi_h, "m": m_h, "r": r_h,
                         "distance": mp.nstr(distance, 25), "under_ulps": float((distance - r) / ulp)}
                out[f"{arm}_violations"].append(entry)
                if arm == "base":
                    out["max_base_under_ulps"] = max(out["max_base_under_ulps"], entry["under_ulps"])
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probes", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    data = json.loads(args.probes.read_text())
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    result = {"schema": "vigilode-safe-enclosure-check-v1",
              "c1": check_c1(data["c1"]["rows"]),
              "c2": check_c2(data["c2"]["rows"]),
              "c3": check_c3(data["c3"]["rows"])}
    summary = {}
    for key in ("c1", "c2", "c3"):
        part = result[key]
        summary[key] = {"rows": part["rows"], "base_violations": len(part["base_violations"]),
                        "repaired_violations": len(part["repaired_violations"])}
        # Keep at most 50 verbatim base counterexamples in the file; the count is complete.
        part["base_violations_total"] = len(part["base_violations"])
        part["base_violations"] = part["base_violations"][:50]
    result["summary"] = summary
    result["gate_g1_repaired_zero_violations"] = all(
        v["repaired_violations"] == 0 for v in summary.values())
    args.output.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps(summary))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
