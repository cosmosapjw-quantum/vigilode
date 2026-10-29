#!/usr/bin/env python3
"""Analyze new adapter runs and a provenance-preserving copy of the frozen baseline."""

import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

from run_external import SOURCE, sha256, write_json
from verify_adapters import FROZEN_ANALYZER_SHA256, verify_campaign


BASELINE_BINARY_SHA256 = "50534f6dc5da3c7c123e74c83c2d6d437474a94282decd274711534292586ac1"
BASELINE_TREE = "b43b69b1ec5b86f3577d7cd6efdefe2d3acb9bad"


def frozen_script():
    path = Path(__file__).resolve().parents[1] / "nonstiff_explicit_20261023" / "analyze_bench.py"
    if sha256(path) != FROZEN_ANALYZER_SHA256:
        raise RuntimeError("frozen 450 analyzer byte identity mismatch")
    return path


def copy_raw(src_folder, dst_folder, receipt, prefix):
    copied = {}
    for field in ("stdout", "stderr"):
        original_name = receipt.get(field)
        if not original_name:
            continue
        name = prefix + original_name
        copied[field] = name
        original = src_folder / original_name
        if not original.is_file():
            continue
        want = receipt.get(field + "_sha256")
        observed = sha256(original)
        if want != observed:
            raise ValueError(f"{prefix} raw {field} hash mismatch: {original}")
        target = dst_folder / name
        shutil.copyfile(original, target)
        if sha256(target) != observed:
            raise ValueError(f"copy hash mismatch: {target}")
    return copied


def merge(baseline, external, merged):
    if merged.exists():
        raise FileExistsError("merged output already exists; preserve prior attempt")
    base_plan = json.loads((baseline / "PLAN.json").read_text())
    base_receipts = json.loads((baseline / "RECEIPTS.json").read_text())
    ext_plan = json.loads((external / "PLAN.json").read_text())
    ext_receipts = json.loads((external / "RECEIPTS.json").read_text())
    summary, replay = verify_campaign(external, write=False)
    if base_plan.get("source_commit") != SOURCE or base_plan.get("binary_sha256") != BASELINE_BINARY_SHA256:
        raise ValueError("frozen baseline source/binary mismatch")
    if ext_plan.get("source_commit") != SOURCE or len(base_plan["configurations"]) != 144:
        raise ValueError("source or baseline configuration count mismatch")
    if len(base_receipts) != 576 or len(ext_receipts) != 384:
        raise ValueError("receipt count mismatch")
    if base_plan.get("warmups_per_configuration") != 1 or base_plan.get("timed_repeats") != 3:
        raise ValueError("baseline round count mismatch")
    merged.mkdir(parents=True)
    configs = []
    receipts = []
    mapping = []
    for cohort, folder, plan, source_receipts, offset, prefix in (
        ("frozen-baseline", baseline, base_plan, base_receipts, 0, "baseline_"),
        ("new-external", external, ext_plan, ext_receipts, 144, "external_"),
    ):
        for cfg in plan["configurations"]:
            configs.append({**cfg, "id": cfg["id"] + offset})
        for receipt in source_receipts:
            copied = copy_raw(folder, merged, receipt, prefix)
            remapped = {**receipt, "id": receipt["id"] + offset, **copied}
            receipts.append(remapped)
            mapping.append({
                "cohort": cohort,
                "original_campaign": str(folder.resolve()),
                "original_config_id": receipt["id"],
                "merged_config_id": remapped["id"],
                "phase": receipt["phase"], "repeat": receipt["repeat"],
                "source_commit": SOURCE,
                "adapter_source_commit": ext_plan["adapter_source_commit"] if cohort == "new-external" else None,
                "files": {field: {"original": receipt.get(field), "merged": remapped.get(field),
                                  "sha256": receipt.get(field + "_sha256")}
                          for field in ("stdout", "stderr")},
            })
    ids = [c["id"] for c in configs]
    if ids != list(range(240)):
        raise ValueError("merged configuration IDs not contiguous")
    merged_plan = {**base_plan, "configurations": configs,
                   "cohorts": {
                       "frozen-baseline": {"source_commit": SOURCE, "source_tree": BASELINE_TREE,
                                           "binary_sha256": BASELINE_BINARY_SHA256,
                                           "plan_sha256": sha256(baseline / "PLAN.json"),
                                           "receipts_sha256": sha256(baseline / "RECEIPTS.json"),
                                           "timing_session": "2026-10-23 original local campaign"},
                       "new-external": {"source_commit": SOURCE,
                                        "adapter_source_commit": ext_plan["adapter_source_commit"],
                                        "adapters": ext_plan["adapters"],
                                        "plan_sha256": sha256(external / "PLAN.json"),
                                        "receipts_sha256": sha256(external / "RECEIPTS.json"),
                                        "timing_session": "subsequent local adapter campaign"}},
                   "cross_session_timing_authority": "diagnostic_only",
                   "cross_session_note": "Ratios across baseline and new adapter sessions are exploratory; each row retains its cohort and raw process evidence."}
    write_json(merged / "PLAN.json", merged_plan)
    write_json(merged / "RECEIPTS.json", receipts)
    write_json(merged / "BASELINE_MAPPING.json", mapping)
    write_json(merged / "SOURCE_CAMPAIGNS.json", {
        "frozen_problem_source": SOURCE,
        "frozen_baseline": {"path": str(baseline.resolve()), "plan_sha256": sha256(baseline / "PLAN.json"),
                            "receipts_sha256": sha256(baseline / "RECEIPTS.json"),
                            "summary_sha256": sha256(baseline / "SUMMARY.json"),
                            "raw_manifest_sha256": sha256(baseline / "RAW_MANIFEST.json")},
        "new_external": {"path": str(external.resolve()), "adapter_source_commit": ext_plan["adapter_source_commit"],
                         "plan_sha256": sha256(external / "PLAN.json"),
                         "receipts_sha256": sha256(external / "RECEIPTS.json"),
                         "adapter_replay_summary": summary},
        "baseline_reused_without_rerun": True,
        "cross_session_timing_authority": "diagnostic_only",
    })
    return summary, replay


def run_strict_analysis(folder, script, external_replay, offset):
    """Feed frozen analyzer derived receipt statuses; retain original receipts/raw."""
    staging = folder / "strict-analysis-input"
    if staging.exists():
        raise FileExistsError("strict analysis input already exists")
    staging.mkdir()
    shutil.copyfile(folder / "PLAN.json", staging / "PLAN.json")
    original_receipts = json.loads((folder / "RECEIPTS.json").read_text())
    replay_by_key = {(item["id"] + offset, item["phase"], item["repeat"]): item
                     for item in external_replay}
    effective = []
    for receipt in original_receipts:
        derived = dict(receipt)
        key = (receipt["id"], receipt["phase"], receipt["repeat"])
        strict = replay_by_key.get(key)
        if strict is not None and not strict["valid"] and receipt["status"] == "COMPLETED":
            derived["original_status"] = receipt["status"]
            derived["status"] = "STRICT_REPLAY_INVALID"
            derived["strict_replay_reasons"] = strict["reasons"]
        effective.append(derived)
    write_json(staging / "RECEIPTS.json", effective)
    shutil.copyfile(staging / "RECEIPTS.json", folder / "STRICT_ANALYSIS_RECEIPTS.json")
    for raw in [*folder.glob("*.stdout.json"), *folder.glob("*.stderr.log")]:
        os.symlink(raw.resolve(), staging / raw.name)
    command = [sys.executable, str(script), str(staging)]
    result = subprocess.run(command, check=False)
    if result.returncode:
        return result.returncode, command
    names = ("SUMMARY.json", "REPLAY.json", "RESULTS.md", "WORK_PRECISION.png",
             "WORK_PRECISION.pdf", "WORK_PRECISION.svg", "RAW_OUTPUTS.tar.gz",
             "RAW_MANIFEST.json")
    for name in names:
        shutil.copyfile(staging / name, folder / name)
    frozen_replay = json.loads((folder / "REPLAY.json").read_text())
    shutil.copyfile(folder / "REPLAY.json", folder / "FROZEN_ANALYZER_REPLAY.json")
    for entry in frozen_replay:
        strict = replay_by_key.get((entry["id"], entry["phase"], entry["repeat"]))
        if strict is not None:
            entry["frozen_analyzer_valid"] = entry["valid"]
            entry["strict_adapter_valid"] = strict["valid"]
            entry["valid"] = entry["valid"] and strict["valid"]
            entry["reasons"] = [*entry["reasons"], *strict["reasons"]]
    write_json(folder / "REPLAY.json", frozen_replay)
    summary = json.loads((folder / "SUMMARY.json").read_text())
    for row in summary["rows"]:
        strict_runs = [item for key, item in replay_by_key.items() if key[0] == row["id"]]
        if strict_runs:
            passed = len(strict_runs) == 4 and all(
                item["valid"] and item["record_success"] and item["status"] == "COMPLETED"
                for item in strict_runs)
            row["strict_adapter_config_passed"] = passed
            if not passed:
                assert not row["all_runs_successful"], "strict-invalid adapter entered eligibility"
    summary["replay"]["valid_replays"] = sum(item["valid"] for item in frozen_replay)
    summary["replay"]["invalid_replays"] = sum(not item["valid"] for item in frozen_replay)
    summary["strict_replay_gate"] = {
        "effective_receipts": "STRICT_ANALYSIS_RECEIPTS.json",
        "original_receipts_unchanged": "RECEIPTS.json",
        "strict_invalid_completed_receipts": sum(
            item.get("status") == "STRICT_REPLAY_INVALID" for item in effective),
        "rule": "Every warmup and timed receipt must be completed, solver-successful, and strict-valid before cost eligibility.",
    }
    write_json(folder / "SUMMARY.json", summary)
    return 0, command


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--external", type=Path, required=True)
    parser.add_argument("--merged-out", type=Path, required=True)
    args = parser.parse_args()
    script = frozen_script()
    verify_summary, external_replay = verify_campaign(args.external)
    external_exit, external_command = run_strict_analysis(args.external, script, external_replay, 0)
    write_json(args.external / "ANALYSIS_EXECUTION.json", {
        "command": external_command,
        "frozen_analyzer_sha256": FROZEN_ANALYZER_SHA256,
        "exit_code": external_exit,
        "adapter_replay_summary": verify_summary,
    })
    if external_exit:
        return external_exit
    replay_summary, external_replay = merge(args.baseline, args.external, args.merged_out)
    # The frozen analyzer independently re-scores all copied saved states and
    # creates the merged graphs and raw-output archive without running solvers.
    merged_exit, merged_command = run_strict_analysis(args.merged_out, script, external_replay, 144)
    write_json(args.merged_out / "ANALYSIS_EXECUTION.json", {
        "command": merged_command,
        "frozen_analyzer_sha256": FROZEN_ANALYZER_SHA256,
        "exit_code": merged_exit,
        "external_replay_summary": replay_summary,
    })
    if merged_exit:
        return merged_exit
    merged_summary = json.loads((args.merged_out / "SUMMARY.json").read_text())
    for row in merged_summary["rows"]:
        row["cohort"] = "frozen-baseline" if row["id"] < 144 else "new-external"
    for comparison in merged_summary["common_error_comparisons"]:
        cross_session = comparison["comparator"] in ("diffsol", "julia")
        comparison["timing_cohort_relation"] = "cross-session" if cross_session else "same-session"
        comparison["timing_authority"] = "diagnostic_only"
    merged_summary["cohort_policy"] = {
        "frozen_baseline_configurations": 144,
        "new_external_configurations": 96,
        "same_problem_source_commit": SOURCE,
        "cross_session_timing_authority": "diagnostic_only",
        "comparison_rule": "fastest configuration satisfying the same sampled-error cap; achieved errors may differ",
    }
    write_json(args.merged_out / "SUMMARY.json", merged_summary)
    print(json.dumps({"configurations": len(merged_summary["rows"]),
                      "comparisons": len(merged_summary["common_error_comparisons"]),
                      "replay": merged_summary["replay"]}, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
