#!/usr/bin/env python3
"""60-digit check of research node research/safe_enclosure_exp_floor_20261004:
e^x must lie in the exported exp_interval(x) for every row (x exact binary64)."""
import argparse
import json
import struct
from pathlib import Path

import mpmath as mp

mp.mp.dps = 60


def unhex(text):
    return mp.mpf(struct.unpack(">d", bytes.fromhex(text))[0])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rows", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    rows = json.loads(args.rows.read_text())["rows"]
    groups = {}
    violations = []
    for group, x_h, lo_h, hi_h in rows:
        x = unhex(x_h)
        truth = mp.e ** x
        g = groups.setdefault(group, {"rows": 0, "violations": 0})
        g["rows"] += 1
        if not (unhex(lo_h) <= truth <= unhex(hi_h)):
            g["violations"] += 1
            violations.append({"group": group, "x": x_h, "lo": lo_h, "hi": hi_h,
                               "truth": mp.nstr(truth, 25)})
    result = {"schema": "vigilode-safe-enclosure-exp-floor-check-v1", "groups": groups,
              "violations": violations, "g1_zero_violations": not violations}
    args.output.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps({"groups": groups, "violations": len(violations)}))


if __name__ == "__main__":
    main()
