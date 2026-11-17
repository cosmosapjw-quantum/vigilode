#!/usr/bin/env python3
"""Orchestrator overlay on the V04 labels after the independent review.

The builder labelled a row authoritative when the three frozen criteria held
and the load average was recorded. The review (REVIEW.json, findings V04-R1,
V04-R2, V04-R4) showed that this is weaker than the pre-registered rule
("a recorded quiet host") and that the faer rows depend on the allocator
state. This script applies one stricter rule to every row and recomputes the
statements that were made "on authoritative rows". It only removes rows from
the authoritative set; RESULTS.json and AMDAHL.json are not modified.

Run: python3 -I research/production_loop_20261116/cost_anatomy/review_overlay.py
"""
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
MAX_LOAD_1MIN = 8.0  # of 24 hardware threads; timing core pinned and reserved
MAX_RELATIVE_SPREAD = 0.10  # (max - min) / median within one arm, 7 repetitions
MIN_FAER_RATIO = 0.96  # instrumented/plain below this on a faer row: allocator state


def strict(path):
    def reject(value):
        raise ValueError(f"non-finite constant {value} in {path}")

    return json.loads(path.read_text(), parse_constant=reject)


def max_load(cell):
    samples = cell.get("load_average") or {}
    values = [samples[key][0] for key in sorted(samples) if samples[key]]
    return max(values) if values else None


def leaves(prefix, value, out):
    if isinstance(value, bool):
        return
    if isinstance(value, (int, float)):
        out[prefix] = float(value)
    elif isinstance(value, dict):
        for key, item in value.items():
            leaves(f"{prefix}.{key}" if prefix else key, item, out)


def main():
    results = strict(HERE / "RESULTS.json")
    amdahl = strict(HERE / "AMDAHL.json")
    decisions = []
    kept = set()
    for cell in results["cells"]:
        label = cell.get("admissibility")
        reasons = []
        if label == "authoritative":
            wall = cell["wall"]
            ratio = wall["instrumented_over_plain"]["ratio_of_medians"]
            spread = max(wall["plain_relative_spread"], wall["instrumented_relative_spread"])
            load = max_load(cell)
            if load is None or load > MAX_LOAD_1MIN:
                reasons.append(f"1-minute load average {load} above {MAX_LOAD_1MIN}")
            if spread > MAX_RELATIVE_SPREAD:
                reasons.append(f"within-arm spread {spread:.3f} above {MAX_RELATIVE_SPREAD}")
            if "faer" in str(cell.get("lu_mode")).lower() and ratio < MIN_FAER_RATIO:
                reasons.append(
                    f"faer row with instrumented/plain {ratio:.3f} below {MIN_FAER_RATIO}: "
                    "the ratio follows the allocator state (V04-R1)"
                )
            if not reasons:
                kept.add(cell["cell"])
            decisions.append(
                {
                    "cell": cell["cell"],
                    "builder_label": label,
                    "overlay_label": "diagnostic_only" if reasons else "authoritative",
                    "reasons": reasons,
                    "instrumented_over_plain": ratio,
                    "max_within_arm_spread": spread,
                    "max_load_1min": load,
                    "lu_mode": cell.get("lu_mode"),
                }
            )

    regimes = amdahl["regimes"]
    by_cell = {regime["cell"]: regime for regime in regimes}
    largest = {}
    for name in sorted(kept):
        regime = by_cell[name]
        for lever, value in regime.items():
            if not (len(lever) > 2 and lever[1] == "_" and lever[0] in "abcdefg"):
                continue
            flat = {}
            leaves("", value, flat)
            for key, number in flat.items():
                if "bound" not in key and not key.startswith(("L", "P", "k")):
                    continue
                slot = largest.setdefault(lever, {}).setdefault(key, {"bound": 0.0, "cell": None})
                if number > slot["bound"]:
                    slot["bound"], slot["cell"] = number, name

    gate = []
    for regime in amdahl["v07_stage_b_gate"]["regimes"]:
        gate.append(
            {
                "cell": regime["cell"],
                "r": regime["r"],
                "bound": regime["bound"],
                "plain_wall_per_accepted_step_us": regime["plain_wall_per_accepted_step_us"],
                "overlay_label": "authoritative" if regime["cell"] in kept else "diagnostic_only",
            }
        )

    chain = {}
    for regime in regimes:
        fractions = regime.get("fractions") or {}
        if "f_chain" not in fractions:
            continue
        key = f"{regime['structure']} / jacobian {regime['jacobian_source']} / rhs x{regime['rhs_multiplier']}"
        entry = chain.setdefault(key, {"cells": 0, "authoritative_cells": 0, "f_chain_min": None, "f_chain_max": None})
        value = fractions["f_chain"]
        entry["cells"] += 1
        entry["authoritative_cells"] += int(regime["cell"] in kept)
        entry["f_chain_min"] = value if entry["f_chain_min"] is None else min(entry["f_chain_min"], value)
        entry["f_chain_max"] = value if entry["f_chain_max"] is None else max(entry["f_chain_max"], value)

    out = {
        "schema": "vigilode.loop.v04-review-overlay.v1",
        "inputs": ["RESULTS.json", "AMDAHL.json", "REVIEW.json"],
        "rule": {
            "applies_to": "rows the builder labelled authoritative",
            "max_load_1min": MAX_LOAD_1MIN,
            "max_within_arm_relative_spread": MAX_RELATIVE_SPREAD,
            "min_instrumented_over_plain_on_faer_rows": MIN_FAER_RATIO,
            "origin": "orchestrator ruling after review findings V04-R1, V04-R2 and V04-R4; quantifies the pre-registered words 'a recorded quiet host'; it can only remove rows from the authoritative set",
        },
        "counts": {
            "builder_authoritative": len(decisions),
            "overlay_authoritative": len(kept),
            "relabelled_diagnostic_only": len(decisions) - len(kept),
        },
        "rows": decisions,
        "v07_stage_b_gate": {
            "condition": amdahl["v07_stage_b_gate"]["condition"],
            "regimes": gate,
            "authoritative_regimes": sum(1 for item in gate if item["overlay_label"] == "authoritative"),
        },
        "largest_bounds_on_overlay_authoritative_rows": largest,
        "chain_share_by_class_all_rows": chain,
    }
    (HERE / "REVIEW_OVERLAY.json").write_text(json.dumps(out, indent=1, allow_nan=False) + "\n")
    print(json.dumps(out["counts"]))
    print("gate regimes on authoritative rows:", out["v07_stage_b_gate"]["authoritative_regimes"])
    for lever, slots in sorted(largest.items()):
        best = max(slots.items(), key=lambda item: item[1]["bound"])
        print(f"{lever}: max {best[1]['bound']:.3f} ({best[0]}) at {best[1]['cell']}")
    for key, entry in sorted(chain.items()):
        print(f"chain share {key}: {entry['f_chain_min']:.3f}-{entry['f_chain_max']:.3f} ({entry['authoritative_cells']}/{entry['cells']} authoritative)")


if __name__ == "__main__":
    main()
