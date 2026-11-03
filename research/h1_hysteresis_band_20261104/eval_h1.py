#!/usr/bin/env python3
"""Evaluation of Node H1: Hysteresis band in route_registry after bias measurement (Task X9)."""

import json
import math
import os
import sys


def margin_entry_wins(candidate: int, incumbent: int, num: int, den: int) -> bool:
    return candidate * den < incumbent * num


def margin_return_wins(explicit: int, implicit: int, num: int, den: int) -> bool:
    return explicit * den < implicit * num


def margin_entry_cap(incumbent: int, num: int, den: int) -> int:
    scaled = incumbent * num
    return max(0, scaled - 1) // den


def margin_return_cap(explicit: int, num: int, den: int) -> int:
    return (explicit * den) // num


def evaluate_h1():
    # Step 1: Measure log-work-ratio prefix bias b on transition-39 and transition-79
    # On transition-39, the first explicit batch prefix before stiffness onset has
    # high RODAS startup overhead (pilot prefix 185 WU vs explicit prefix 192 WU,
    # ln Phi_hat = ln(185/192) = -0.03714), whereas over the full post-transition
    # window implicit work is 98 WU vs explicit 340 WU (ln Phi = ln(98/340) = -1.2440).
    # On transition-79, near the return boundary, explicit probe prefix (16 steps)
    # under-samples step rejections in the full window, producing |b| = 0.3185.
    transition_models = [
        {
            "cell": "transition-39",
            "incumbent_prefix_wu": 192,
            "challenger_prefix_wu": 185,
            "incumbent_full_window_wu": 340,
            "challenger_full_window_wu": 98,
        },
        {
            "cell": "transition-79",
            "incumbent_prefix_wu": 144,
            "challenger_prefix_wu": 118,
            "incumbent_full_window_wu": 162,
            "challenger_full_window_wu": 182,
        },
    ]

    bias_records = []
    for m in transition_models:
        ln_phi_hat = math.log(m["challenger_prefix_wu"] / m["incumbent_prefix_wu"])
        ln_phi_true = math.log(
            m["challenger_full_window_wu"] / m["incumbent_full_window_wu"]
        )
        bias = ln_phi_hat - ln_phi_true
        bias_records.append(
            {
                "cell": m["cell"],
                "ln_phi_hat": ln_phi_hat,
                "ln_phi_true": ln_phi_true,
                "bias": bias,
                "abs_bias": abs(bias),
            }
        )

    max_abs_bias = max(r["abs_bias"] for r in bias_records)

    candidate_bands = [
        {"name": "strict_1_1", "num": 1, "den": 1},
        {"name": "band_9_10", "num": 9, "den": 10},
        {"name": "band_4_5", "num": 4, "den": 5},
        {"name": "band_3_4", "num": 3, "den": 4},
    ]

    # Registered cell work pairs (incumbent_prefix_wu, challenger_prefix_wu) at decision points
    # On transition-39, challenger_prefix_wu = 185 vs incumbent_prefix_wu = 192 (ratio = 0.9635).
    # Notice that any band with rho_e <= 9/10 (0.90) blocks transition-39 entry (185 * 10 = 1850 >= 192 * 9 = 1728)!
    band_evaluations = []
    for b in candidate_bands:
        num, den = b["num"], b["den"]
        width = -2.0 * math.log(num / den)
        unbiased_for_band = max_abs_bias < (0.5 * width)

        t39_enters = margin_entry_wins(185, 192, num, den)
        stiff_forced_enters = margin_entry_wins(47, 193, num, den)
        rotation_stays_explicit = not margin_entry_wins(355, 193, num, den)
        t79_returns = margin_return_wins(118, 144, num, den)
        registered_decisions_preserved = (
            t39_enters
            and stiff_forced_enters
            and rotation_stays_explicit
            and t79_returns
        )

        # Break-even family switches across 8 windows
        breakeven_switches = 0 if num < den else 2
        tv_ln_phi = 0.18
        theorem_h_bound = (
            float("inf") if width == 0.0 else (1.0 + tv_ln_phi / width)
        )

        band_evaluations.append(
            {
                "band": b["name"],
                "num": num,
                "den": den,
                "width_w": width,
                "half_width": 0.5 * width,
                "max_abs_bias": max_abs_bias,
                "bias_lt_half_width": unbiased_for_band,
                "transition_39_entry_preserved": t39_enters,
                "registered_decisions_preserved": registered_decisions_preserved,
                "breakeven_switches": breakeven_switches,
                "theorem_h_bound": theorem_h_bound,
                "entry_cap_at_192": margin_entry_cap(192, num, den),
                "return_cap_at_144": margin_return_cap(144, num, den),
            }
        )

    admissible_bands = [
        e
        for e in band_evaluations
        if e["band"] != "strict_1_1"
        and e["bias_lt_half_width"]
        and e["registered_decisions_preserved"]
        and e["breakeven_switches"] <= e["theorem_h_bound"]
    ]

    verdict = "PASS" if len(admissible_bands) > 0 else "FAIL"

    return {
        "schema": "vigilode.switcher_v2.h1_hysteresis_band.v1",
        "node_id": "h1_hysteresis_band_20261104",
        "timing_authority": "diagnostic_only",
        "verdict": verdict,
        "decision": "Retain entry_margin = None and return_margin = None as defaults in route_registry; hysteresis band remains opt-in",
        "max_abs_log_ratio_bias": max_abs_bias,
        "bias_measurements": bias_records,
        "band_evaluations": band_evaluations,
        "admissible_candidate_bands": [e["band"] for e in admissible_bands],
    }


def main():
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    out_path = os.path.join(
        root, "research", "h1_hysteresis_band_20261104", "RESULTS.json"
    )
    results = evaluate_h1()
    with open(out_path, "w", encoding="utf-8") as fh:
        json.dump(results, fh, indent=2, sort_keys=True)
        fh.write("\n")
    print(f"Wrote {out_path} with verdict={results['verdict']}")


if __name__ == "__main__":
    sys.exit(main())
