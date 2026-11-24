#!/usr/bin/env python3
"""Summary of the three diagnostic timing tries of the two-way banded client (node SL-C8).

Every try is a separate process run of the ignored test
`measure_two_way_stage_pattern_and_whole_solves` on a 2-vCPU shared host with the other
build paused. The summary keeps, per (kind, case, arm), the ratio to the protected arm of
the same try for each try, their minimum and maximum, and the median. A ratio is
`min(arm) / min(protected)` within one try (seven rounds for stage patterns, three for
whole solves, alternating order). No verdict is derived here; the protocol was not
preregistered and the host is not isolated. Usage: summarize_timing.py <results dir>
"""
from __future__ import annotations

import json
import pathlib
import statistics
import sys


def main() -> int:
    results = pathlib.Path(sys.argv[1])
    rows = []
    for t in (1, 2, 3):
        for line in (results / f"two_way_timing_try{t}.jsonl").read_text().splitlines():
            r = json.loads(line)
            r["try"] = t
            rows.append(r)
    groups: dict[tuple, list] = {}
    for r in rows:
        groups.setdefault((r["kind"], r["case"], r["arm"]), []).append(r)
    summary = []
    for (kind, case, arm), rs in sorted(groups.items()):
        ratios = [r["ratio_to_protected_min"] for r in sorted(rs, key=lambda r: r["try"])]
        entry = {
            "kind": kind, "case": case, "n": rs[0]["n"], "arm": arm,
            "ratios_by_try": ratios, "ratio_min": min(ratios), "ratio_max": max(ratios),
            "ratio_median": statistics.median(ratios),
        }
        if kind == "stage-pattern":
            entry["band"] = rs[0]["band"]
            entry["ns_per_row_min"] = min(r["min_ns_per_row"] for r in rs)
        else:
            entry["attempts"] = sorted({r["attempts"] for r in rs})
            entry["difference_in_tolerance_units_max"] = max(
                r["difference_in_tolerance_units"] for r in rs
            )
        summary.append(entry)
    out = {
        "schema": "vigilode.referee2.two-way-timing-summary.v1",
        "host": "2 vCPU Intel Xeon @ 2.10GHz (cloud sandbox), rustc 1.97.0 stable, profile measurement",
        "status": "diagnostic: not preregistered, shared host, three tries",
        "note": "try 1 used the first kernel revision (no contiguous back-substitution slices; "
                "whole-solve protected arm with the Indexed kernel); tries 2-3 the final revision "
                "with the facade's kernel choice for the protected whole solve",
        "rows": summary,
    }
    (results / "two_way_timing_summary.json").write_text(json.dumps(out, indent=1) + "\n")
    for e in summary:
        if e["arm"].startswith(("two-way", "tight", "client")):
            print(f"{e['kind']:13s} {e['case']:22s} {e['arm']:24s} ratios={['%.3f' % x for x in e['ratios_by_try']]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
