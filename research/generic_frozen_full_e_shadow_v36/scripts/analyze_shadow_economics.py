#!/usr/bin/env python3
"""Validate and summarize v3.6 optimized paired-wall economics."""

from __future__ import annotations

import argparse
import csv
import json
import math
import statistics
import struct
from pathlib import Path
from typing import Any

from verify_runtime_shadow_against_preflight import (
    FAMILIES,
    HARD_GATE_FIELDS,
    PROFILES,
    VerificationError,
    exact_equal,
    load_json,
    require,
    require_exact,
    sha256,
)


def validate_arm(arm: dict[str, Any], mode: str, repetitions: int, label: str) -> None:
    require(arm.get("mode") == mode, f"mode mismatch: {label}")
    require(arm.get("repetitions") == repetitions, f"repetition mismatch: {label}")
    require(arm.get("family_count") == 6, f"family count mismatch: {label}")
    require(arm.get("all_suite_identities_passed") is True, f"identity failure: {label}")
    wall = arm.get("wall_seconds")
    interval = arm.get("proposed_interval")
    gamma = arm.get("gamma_seconds_per_interval")
    require(
        isinstance(wall, (int, float)) and math.isfinite(wall) and wall > 0.0,
        f"invalid wall time: {label}",
    )
    require(
        isinstance(interval, (int, float)) and math.isfinite(interval) and interval > 0.0,
        f"invalid interval: {label}",
    )
    require(isinstance(gamma, (int, float)) and math.isfinite(gamma), f"invalid Gamma: {label}")
    require_exact(gamma, wall / interval, f"Gamma identity: {label}")


def positive_ulp_distance(left: float, right: float) -> int:
    require(left > 0.0 and right > 0.0, "ULP comparison requires positive values")
    left_bits = struct.unpack(">Q", struct.pack(">d", left))[0]
    right_bits = struct.unpack(">Q", struct.pack(">d", right))[0]
    return abs(left_bits - right_bits)


def validate_pair(
    pair: dict[str, Any], pair_index: int, repetitions: int, label: str
) -> float:
    expected_order = "rjf-first" if pair_index % 2 == 0 else "shadow-first"
    require(pair.get("pair_index") == pair_index, f"pair index: {label}")
    require(pair.get("order") == expected_order, f"pair order: {label}")
    rjf = pair.get("rjf_only", {})
    shadow = pair.get("frozen_full_e_shadow", {})
    validate_arm(rjf, "rjf-only", repetitions, f"{label}/rjf")
    validate_arm(shadow, "frozen-full-e-shadow", repetitions, f"{label}/shadow")
    require_exact(
        rjf.get("proposed_interval"),
        shadow.get("proposed_interval"),
        f"paired denominator: {label}",
    )
    wall_ratio = shadow["wall_seconds"] / rjf["wall_seconds"]
    gamma_ratio = shadow["gamma_seconds_per_interval"] / rjf["gamma_seconds_per_interval"]
    require_exact(pair.get("wall_ratio_shadow_over_rjf"), wall_ratio, f"wall ratio: {label}")
    require_exact(pair.get("gamma_ratio_shadow_over_rjf"), gamma_ratio, f"Gamma ratio: {label}")
    require(
        positive_ulp_distance(wall_ratio, gamma_ratio) <= 1,
        f"wall/Gamma ratio exceeds one ULP: {label}",
    )
    return wall_ratio


def runtime_profile_totals(runtime_root: Path, profile: str) -> dict[str, Any]:
    reports = [load_json(runtime_root / profile / f"{family}.json") for family in FAMILIES]
    committed = sum(report["committed_rjf_jvp_vectors"] for report in reports)
    prefix = sum(report["prefix_speculative_work"]["jvp_vectors"] for report in reports)
    continuation = sum(report["continuation_work"]["jvp_vectors"] for report in reports)
    return {
        "recommendations": sum(report["recommendations"] for report in reports),
        "completions": sum(report["shadow_full_e_completions"] for report in reports),
        "unsafe": sum(report["unsafe_recommendations"] for report in reports),
        "committed_rjf_jvp_vectors": committed,
        "prefix_jvp_vectors": prefix,
        "continuation_jvp_vectors": continuation,
        "total_speculative_jvp_vectors": prefix + continuation,
        "realized_prefix_over_committed_rjf_jvp": prefix / committed,
        "realized_continuation_over_committed_rjf_jvp": continuation / committed,
        "realized_total_speculative_over_committed_rjf_jvp": (prefix + continuation)
        / committed,
    }


def analyze_profile(
    economics: dict[str, Any], runtime: dict[str, Any], profile: str
) -> dict[str, Any]:
    label = profile
    require(economics.get("schema") == "g4-s5b0-frozen-full-e-shadow-economics-v1", f"schema: {label}")
    require(economics.get("status") == "complete", f"status: {label}")
    require(economics.get("profile") == PROFILES[profile], f"profile label: {label}")
    require(economics.get("switching_active") is False, f"switching active: {label}")
    require_exact(economics.get("frozen_zeta34_tau"), 13.39706618860016, f"tau: {label}")
    require(economics.get("all_six_families_present") is True, f"family set: {label}")
    require(
        economics.get("reference_recommendations") == runtime["recommendations"],
        f"reference recommendation count: {label}",
    )
    require(
        economics.get("reference_shadow_completions") == runtime["completions"],
        f"reference completion count: {label}",
    )
    require(economics.get("reference_unsafe_recommendations") == 0, f"unsafe: {label}")
    require(runtime["unsafe"] == 0, f"runtime unsafe: {label}")
    require(
        all(economics.get("reference_hard_gates", {}).get(key) is True for key in HARD_GATE_FIELDS),
        f"reference hard gate: {label}",
    )

    wall = economics.get("paired_wall", {})
    require(wall.get("required_build_profile") == "measurement", f"build profile: {label}")
    require(wall.get("measurement_build_verified") is True, f"build attestation: {label}")
    require(wall.get("compiled_profile_directory") == "measurement", f"profile directory: {label}")
    require(wall.get("compiled_cargo_profile") == "release", f"raw Cargo profile: {label}")
    require(wall.get("suite_scope") == "all-six-families", f"suite scope: {label}")
    require(wall.get("calibration_arm") == "rjf-only", f"calibration arm: {label}")
    require(
        wall.get("gamma_denominator") == "sum-absolute-proposed-attempt-h",
        f"Gamma denominator: {label}",
    )
    require(wall.get("warmup_pairs") == 1, f"warm-up count: {label}")
    require(wall.get("measured_pairs") == 7, f"measured count: {label}")
    require(wall.get("all_suite_identities_passed") is True, f"suite identity: {label}")
    repetitions = wall.get("frozen_repetitions")
    require(isinstance(repetitions, int) and repetitions > 0, f"frozen repetitions: {label}")

    calibration = wall.get("calibration_rows")
    require(isinstance(calibration, list) and calibration, f"calibration rows: {label}")
    expected_repetitions = 1
    for index, row in enumerate(calibration):
        require(row.get("repetitions") == expected_repetitions, f"calibration doubling: {label}/{index}")
        require(row.get("all_suite_identities_passed") is True, f"calibration identity: {label}/{index}")
        require_exact(
            row.get("gamma_seconds_per_interval"),
            row["wall_seconds"] / row["proposed_interval"],
            f"calibration Gamma: {label}/{index}",
        )
        expected_repetitions = min(
            expected_repetitions * 2, wall["maximum_calibration_repetitions"]
        )
    require(calibration[-1]["repetitions"] == repetitions, f"frozen calibration row: {label}")
    require(
        calibration[-1]["wall_seconds"] >= wall["minimum_calibration_wall_seconds"]
        or repetitions == wall["maximum_calibration_repetitions"],
        f"calibration stopping rule: {label}",
    )

    warmups = wall.get("warmup_rows")
    measured = wall.get("measured_rows")
    require(isinstance(warmups, list) and len(warmups) == 1, f"warm-up rows: {label}")
    require(isinstance(measured, list) and len(measured) == 7, f"measured rows: {label}")
    validate_pair(warmups[0], 0, repetitions, f"{label}/warmup-0")
    ratios = [
        validate_pair(pair, index, repetitions, f"{label}/measured-{index}")
        for index, pair in enumerate(measured)
    ]
    rjf_walls = [pair["rjf_only"]["wall_seconds"] for pair in measured]
    shadow_walls = [pair["frozen_full_e_shadow"]["wall_seconds"] for pair in measured]
    median = statistics.median(ratios)
    rjf_first = [ratio for index, ratio in enumerate(ratios) if index % 2 == 0]
    shadow_first = [ratio for index, ratio in enumerate(ratios) if index % 2 == 1]
    absolute_deviations = [abs(ratio - median) for ratio in ratios]
    return {
        "profile": profile,
        "dimension": int("".join(character for character in profile if character.isdigit())),
        **runtime,
        "calibration_repetitions": repetitions,
        "calibration_rjf_wall_seconds": calibration[-1]["wall_seconds"],
        "measured_pairs": measured,
        "measured_wall_ratios_shadow_over_rjf": ratios,
        "measured_rjf_only_wall_seconds": rjf_walls,
        "measured_shadow_wall_seconds": shadow_walls,
        "median_wall_ratio_shadow_over_rjf": median,
        "mean_wall_ratio_shadow_over_rjf": statistics.fmean(ratios),
        "minimum_wall_ratio_shadow_over_rjf": min(ratios),
        "maximum_wall_ratio_shadow_over_rjf": max(ratios),
        "median_absolute_deviation": statistics.median(absolute_deviations),
        "rjf_first_median_wall_ratio": statistics.median(rjf_first),
        "shadow_first_median_wall_ratio": statistics.median(shadow_first),
        "order_median_absolute_gap": abs(
            statistics.median(rjf_first) - statistics.median(shadow_first)
        ),
        "maximum_over_minimum_wall_ratio": max(ratios) / min(ratios),
        "rjf_only_wall_maximum_over_minimum": max(rjf_walls) / min(rjf_walls),
        "shadow_wall_maximum_over_minimum": max(shadow_walls) / min(shadow_walls),
    }


def write_csv(path: Path, profiles: list[dict[str, Any]]) -> None:
    fields = (
        "profile",
        "dimension",
        "recommendations",
        "committed_rjf_jvp_vectors",
        "prefix_jvp_vectors",
        "continuation_jvp_vectors",
        "total_speculative_jvp_vectors",
        "realized_prefix_over_committed_rjf_jvp",
        "realized_continuation_over_committed_rjf_jvp",
        "realized_total_speculative_over_committed_rjf_jvp",
        "calibration_repetitions",
        "median_wall_ratio_shadow_over_rjf",
        "mean_wall_ratio_shadow_over_rjf",
        "minimum_wall_ratio_shadow_over_rjf",
        "maximum_wall_ratio_shadow_over_rjf",
        "median_absolute_deviation",
        "rjf_first_median_wall_ratio",
        "shadow_first_median_wall_ratio",
        "order_median_absolute_gap",
        "maximum_over_minimum_wall_ratio",
        "rjf_only_wall_maximum_over_minimum",
        "shadow_wall_maximum_over_minimum",
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        for row in profiles:
            writer.writerow({field: row[field] for field in fields})


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--economics-root", type=Path, required=True)
    parser.add_argument("--runtime-root", type=Path, required=True)
    parser.add_argument("--runtime-verification", type=Path, required=True)
    parser.add_argument("--output-json", type=Path, required=True)
    parser.add_argument("--output-csv", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    expected = {Path(f"{profile}.json") for profile in PROFILES}
    found = {path.relative_to(args.economics_root) for path in args.economics_root.rglob("*.json")}
    require(found == expected, "economics file set is not exactly the five consumed profiles")
    profiles = []
    hashes = {}
    runtime_hashes = {
        str(path.relative_to(args.runtime_root)): sha256(path)
        for path in sorted(args.runtime_root.rglob("*.json"))
    }
    runtime_verification = load_json(args.runtime_verification)
    require(
        runtime_verification.get("schema") == "vigilode-v36-runtime-shadow-verification-v1"
        and runtime_verification.get("verdict") == "PASS",
        "runtime verification authority is not PASS",
    )
    require_exact(
        runtime_verification.get("input_sha256", {}).get("runtime"),
        runtime_hashes,
        "runtime verification input hashes",
    )
    for profile in PROFILES:
        path = args.economics_root / f"{profile}.json"
        economics = load_json(path)
        runtime = runtime_profile_totals(args.runtime_root, profile)
        profiles.append(analyze_profile(economics, runtime, profile))
        hashes[path.name] = sha256(path)
    profile_medians = [row["median_wall_ratio_shadow_over_rjf"] for row in profiles]
    largest_rjf_span = max(profiles, key=lambda row: row["rjf_only_wall_maximum_over_minimum"])
    output = {
        "schema": "vigilode-v36-frozen-full-e-shadow-economics-summary-v1",
        "verdict": "PASS_DESCRIPTIVE_ECONOMICS",
        "measurement_profile_verified": True,
        "all_pair_identities_passed": True,
        "active_switching": False,
        "speedup_claim_authorized": False,
        "fresh_safety_claim_authorized": False,
        "timing_exclusion_rule_precommitted": False,
        "measured_pairs_excluded": 0,
        "profile_count": len(profiles),
        "measured_pair_count": sum(len(row["measured_wall_ratios_shadow_over_rjf"]) for row in profiles),
        "median_of_profile_medians_shadow_over_rjf": statistics.median(profile_medians),
        "minimum_profile_median_shadow_over_rjf": min(profile_medians),
        "maximum_profile_median_shadow_over_rjf": max(profile_medians),
        "largest_rjf_wall_span": {
            "profile": largest_rjf_span["profile"],
            "maximum_over_minimum": largest_rjf_span["rjf_only_wall_maximum_over_minimum"],
        },
        "profiles": profiles,
        "input_sha256": {
            "economics": hashes,
            "runtime": runtime_hashes,
            "runtime_verification": sha256(args.runtime_verification),
        },
        "interpretation": [
            "All ratios are descriptive optimized whole-suite measurements with all pairs retained.",
            "Ratios near or on both sides of one do not authorize an active-polyalgorithm speedup claim.",
            "No post-hoc timing exclusion is applied; N=384 is retained despite host-noise-dominated wall variation.",
            "The consumed profiles are not fresh safety evidence; N=2048 remains sealed.",
        ],
    }
    args.output_json.parent.mkdir(parents=True, exist_ok=True)
    args.output_json.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_csv(args.output_csv, profiles)
    print("SHADOW_ECONOMICS_ANALYSIS_PASS")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except VerificationError as error:
        print(f"SHADOW_ECONOMICS_ANALYSIS_FAIL: {error}")
        raise SystemExit(1) from error
