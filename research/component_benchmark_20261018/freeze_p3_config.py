#!/usr/bin/env python3
"""Materialize reviewer-selected P3 contrasts from immutable P2 evidence.

This utility makes no numerical or selection decisions. It never runs a solver.
The selection JSON contains ``rows`` with exact panel pair identity fields and
reviewer rationale, component_mapping, critical_flag, and comparison_scope.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from component_benchmark.protocol import digest, file_digest, physical_input_digest, strict_load, validate_case
from component_benchmark.runner import source_binding

IDENTITY = ("case_id", "baseline", "baseline_config_id", "candidate", "candidate_config_id")
ARM_FIELDS = ("baseline", "candidate")


def _required_hash(value, label):
    if not isinstance(value, str) or len(value) != 64 or any(c not in "0123456789abcdef" for c in value):
        raise ValueError("INVALID_SHA256:" + label)
    return value


def _identity(row):
    return tuple(row.get(k) for k in IDENTITY)


def _linked_file(root, path, expected, label, embedded_sha=False):
    if not isinstance(path, str) or not path or Path(path).is_absolute():
        raise ValueError("INVALID_LINK_PATH:" + label)
    full = (root / path).resolve(strict=True)
    if not full.is_relative_to(root):
        raise ValueError("LINK_OUTSIDE_ROOT:" + label)
    if not full.is_file():
        raise ValueError("LINK_MISSING:" + label)
    actual = strict_load(full.read_text()).get("receipt_sha256") if embedded_sha else file_digest(full)
    if actual != _required_hash(expected, label):
        raise ValueError("LINK_SHA_MISMATCH:" + label)
    return full


def _panel_rows(path):
    with path.open(newline="") as stream:
        rows = list(csv.DictReader(stream))
    by_key = {}
    for row in rows:
        key = _identity(row)
        if any(not isinstance(part, str) or not part for part in key) or key in by_key:
            raise ValueError("DUPLICATE_OR_INVALID_PANEL_PAIR")
        by_key[key] = row
    return by_key


def _arm_rows(path):
    with path.open(newline="") as stream:
        rows = list(csv.DictReader(stream))
    selected = {}
    for row in rows:
        if row.get("queue") != "screening" or row.get("measurement_mode") != "timing":
            continue
        key = row.get("job_id")
        if not key or key in selected:
            raise ValueError("DUPLICATE_OR_INVALID_PANEL_ARM")
        selected[key] = row
    return selected


def _panel_pair(panel, entry):
    key = _identity(entry)
    if key in panel:
        return panel[key], False
    reversed_key = (entry.get("case_id"), entry.get("candidate"), entry.get("candidate_config_id"),
                    entry.get("baseline"), entry.get("baseline_config_id"))
    if reversed_key in panel:
        return panel[reversed_key], True
    raise ValueError("SELECTED_PAIR_MISSING_FROM_FULL_PANEL")


def _panel_arm(row, label, reversed_pair):
    return "candidate" if reversed_pair and label == "baseline" else "baseline" if reversed_pair else label


def _job_index(plan):
    if plan.get("kind") != "screening" or plan.get("phase") != "P2":
        raise ValueError("P2_SCREENING_PLAN_REQUIRED")
    indexed = {}
    for job in plan.get("jobs", []):
        if job.get("measurement_mode") != "timing":
            continue
        key = (job.get("case_id"), job.get("solver_id"), job.get("config_id"))
        if key in indexed:
            raise ValueError("DUPLICATE_SCREENING_JOB")
        indexed[key] = job
    return indexed


def _positive_rss(value, label):
    try:
        parsed = int(value)
    except (TypeError, ValueError):
        raise ValueError("MISSING_SCREENED_RSS:" + label) from None
    if parsed <= 0 or str(parsed) != str(value):
        raise ValueError("INVALID_SCREENED_RSS:" + label)
    return parsed


def materialize(root, selection_path, panel_path, arms_path, plan_path):
    root = root.resolve(strict=True)
    selected = strict_load(selection_path.read_text())
    entries = selected.get("rows")
    if not isinstance(entries, list) or not 1 <= len(entries) <= 16:
        raise ValueError("ONE_TO_SIXTEEN_REVIEWER_ROWS_REQUIRED")
    panel = _panel_rows(panel_path)
    arms = _arm_rows(arms_path)
    plan = strict_load(plan_path.read_text())
    source_sha = digest(plan["source_binding"])
    if selected.get("source_binding_sha256") != source_sha:
        raise ValueError("SELECTION_SOURCE_BINDING_MISMATCH")
    if digest(source_binding(root)) != source_sha:
        raise ValueError("CURRENT_SOURCE_BINDING_MISMATCH")
    if (selected.get("panel_sha256") != file_digest(panel_path) or
            selected.get("arms_sha256") != file_digest(arms_path) or
            selected.get("screening_plan_sha256") != file_digest(plan_path)):
        raise ValueError("SELECTION_INPUT_SHA_MISMATCH")
    jobs = _job_index(plan)
    config_cases = {}
    config_solvers = {}
    contrasts = []
    peaks = defaultdict(list)
    links = []
    seen = set()
    seen_ids = set()
    for entry in entries:
        if not isinstance(entry, dict):
            raise ValueError("INVALID_SELECTION_ROW")
        key = _identity(entry)
        if key in seen:
            raise ValueError("DUPLICATE_SELECTED_PAIR")
        seen.add(key)
        row, reversed_pair = _panel_pair(panel, entry)
        if not isinstance(entry.get("id"), str) or not entry["id"]:
            raise ValueError("MISSING_REVIEWER_CONTRAST_ID")
        if entry["id"] in seen_ids:
            raise ValueError("DUPLICATE_REVIEWER_CONTRAST_ID")
        seen_ids.add(entry["id"])
        scope = entry.get("comparison_scope")
        if scope not in ("same_target", "matched_achieved"):
            raise ValueError("EXPLICIT_COMPARISON_SCOPE_REQUIRED")
        if row.get("same_target_status") != "PASS" or (scope == "matched_achieved" and row.get("matched_achieved_status") != "PASS"):
            raise ValueError("SELECTED_PAIR_SCOPE_NOT_PASS")
        if not isinstance(entry.get("rationale"), str) or not entry["rationale"].strip():
            raise ValueError("MISSING_REVIEWER_RATIONALE")
        if not isinstance(entry.get("component_mapping"), list) or any(
                not isinstance(value, str) or not value for value in entry["component_mapping"]):
            raise ValueError("INVALID_REVIEWER_COMPONENT_MAPPING")
        if type(entry.get("critical_flag")) is not bool:
            raise ValueError("EXPLICIT_CRITICAL_FLAG_REQUIRED")
        if row.get("source_binding_sha256") != source_sha:
            raise ValueError("PANEL_SOURCE_BINDING_MISMATCH")
        if row.get("baseline_threads") != "1" or row.get("candidate_threads") != "1":
            raise ValueError("P3_SINGLE_THREAD_LANE_REQUIRED")
        for arm in ARM_FIELDS:
            panel_arm = _panel_arm(row, arm, reversed_pair)
            sid, cid = row[panel_arm], row[panel_arm + "_config_id"]
            job = jobs.get((row["case_id"], sid, cid))
            if job is None or job.get("job_id") != row.get(panel_arm + "_job_id") or job.get("capability") != "SUPPORTED":
                raise ValueError("SELECTED_SCREENING_JOB_MISMATCH:" + arm)
            arm_row = arms.get(job["job_id"])
            if arm_row is None or any(arm_row.get(field) != row.get(panel_arm + "_" + field) for field in
                    ("receipt_path", "receipt_sha256", "probe_receipt_path", "probe_receipt_sha256",
                     "prerequisite_evidence_sha256", "peak_rss_bytes")):
                raise ValueError("PANEL_ARM_LINK_MISMATCH:" + arm)
            if (arm_row.get("status") != "OK" or arm_row.get("gate_status") != "PASS" or
                    arm_row.get("capability") != "SUPPORTED" or arm_row.get("requested_threads") != "1" or
                    arm_row.get("probe_status") != "PROBE_PASS_DIAGNOSTIC" or
                    arm_row.get("source_binding_sha256") != source_sha):
                raise ValueError("PANEL_ARM_PREREQUISITE_NOT_PASS:" + arm)
            if job.get("request", {}).get("solver_id") != sid:
                raise ValueError("SCREENING_REQUEST_SOLVER_MISMATCH:" + arm)
            case = job["request"]["case"]
            if validate_case(case) or physical_input_digest(case) != row.get("physical_input_sha256"):
                raise ValueError("SCREENING_CASE_MISMATCH:" + arm)
            previous = config_cases.setdefault(row["case_id"], case)
            if digest(previous) != digest(case):
                raise ValueError("CASE_ID_INPUT_CONFLICT")
            spec = plan["solvers"][sid]
            if spec.get("threads", 1) != 1 or any(spec.get("env", {}).get(k, "1") != "1" for k in
                    ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "RAYON_NUM_THREADS", "JULIA_NUM_THREADS")):
                raise ValueError("P3_SINGLE_THREAD_SOLVER_SPEC_REQUIRED")
            config_solvers[sid] = spec
            receipt = _linked_file(root, arm_row.get("receipt_path"), arm_row.get("receipt_sha256"), arm + "_receipt")
            raw = strict_load(receipt.read_text())
            if (raw.get("status") != "OK" or raw.get("job") != job or
                    raw.get("source_binding_sha256") != source_sha or
                    raw.get("result", {}).get("measurement_mode") != "timing"):
                raise ValueError("SCREENING_RECEIPT_NOT_PASS:" + arm)
            probe = _linked_file(root, arm_row.get("probe_receipt_path"), arm_row.get("probe_receipt_sha256"), arm + "_probe_receipt", embedded_sha=True)
            probe_raw = strict_load(probe.read_text())
            if (probe_raw.get("parity", {}).get("status") != "PASS_DIAGNOSTIC" or
                    probe_raw.get("adapter_status") != "OK" or
                    probe_raw.get("source_binding_sha256") != source_sha or
                    probe_raw.get("job", {}).get("solver_id") != arm_row.get("probe_adapter")):
                raise ValueError("PROBE_RECEIPT_NOT_PASS:" + arm)
            if not _required_hash(arm_row.get("prerequisite_evidence_sha256"), arm + "_prerequisite") == job.get("prerequisite", {}).get("evidence_sha256"):
                raise ValueError("PREREQUISITE_LINK_MISMATCH:" + arm)
            rss = _positive_rss(arm_row.get("peak_rss_bytes"), arm)
            attempt_peaks = [a.get("peak_rss_bytes") for a in raw.get("attempts", [])]
            if rss not in attempt_peaks:
                raise ValueError("SCREENED_RSS_RECEIPT_MISMATCH:" + arm)
            peaks[sid + "#" + cid].append(rss)
            links.append({"arm": arm, "case_id": row["case_id"], "solver_id": sid,
                          "config_id": cid, "job_id": job["job_id"], "receipt_sha256": file_digest(receipt),
                          "probe_receipt_sha256": arm_row["probe_receipt_sha256"],
                          "probe_receipt_file_sha256": file_digest(probe), "peak_rss_bytes": rss})
        base_arm = arms[row["baseline_job_id"]]
        candidate_arm = arms[row["candidate_job_id"]]
        if base_arm.get("reference_receipt_path") != candidate_arm.get("reference_receipt_path"):
            raise ValueError("REFERENCE_RECEIPT_PATH_CONFLICT")
        ref = _linked_file(root, base_arm.get("reference_receipt_path"), row.get("reference_receipt_sha256"), "reference_receipt")
        ref_raw = strict_load(ref.read_text())
        if ref_raw.get("status") != "OK":
            raise ValueError("REFERENCE_RECEIPT_NOT_PASS")
        contrast = {k: entry[k] for k in IDENTITY}
        contrast["id"] = entry["id"]
        contrast.update({"rationale": entry["rationale"], "component_mapping": entry["component_mapping"],
                         "critical_flag": entry["critical_flag"], "comparison_scope": scope,
                         "screening_pair_sha256": digest(row),
                         "matched_achieved_status": row["matched_achieved_status"],
                         "same_target_status": row["same_target_status"]})
        contrasts.append(contrast)
    config = {"schema": "vigilode-component-p3-frozen-selection-v1",
              "cases": [config_cases[k] for k in sorted(config_cases)],
              "solvers": {k: config_solvers[k] for k in sorted(config_solvers)},
              "global_contrasts": contrasts, "screened_peak_rss_bytes": {k: max(v) for k,v in sorted(peaks.items())},
              "random_seed": 20261018, "timing_authority": "diagnostic_only",
              "selection_authority": "FRESH_ASTRA_CONFIG_REVIEW_REQUIRED",
              "source_binding_sha256": source_sha}
    receipt = {"schema": "vigilode-component-p3-selection-receipt-v1",
               "selection_sha256": file_digest(selection_path), "panel_sha256": file_digest(panel_path),
               "arms_sha256": file_digest(arms_path),
               "screening_plan_sha256": file_digest(plan_path), "source_binding_sha256": source_sha,
               "selected_pair_count": len(contrasts), "selected_pair_sha256": [digest(row) for row in contrasts],
               "screening_arm_links": links, "screened_peak_rss_bytes": config["screened_peak_rss_bytes"],
               "p3_launch_admission": "HOLD_PENDING_FRESH_ASTRA_CONFIG_REVIEW"}
    return config, receipt


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--selection", type=Path, required=True)
    parser.add_argument("--panel", type=Path, required=True)
    parser.add_argument("--arms", type=Path, required=True)
    parser.add_argument("--screening-plan", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists() or args.receipt.exists() or args.output == args.receipt:
        raise FileExistsError("P3_FROZEN_OUTPUT_ALREADY_EXISTS")
    config, receipt = materialize(args.root, args.selection, args.panel, args.arms, args.screening_plan)
    rendered = json.dumps(config, indent=2, sort_keys=True, allow_nan=False) + "\n"
    receipt["config_sha256"] = hashlib.sha256(rendered.encode()).hexdigest()
    with args.output.open("x", encoding="utf-8") as stream:
        stream.write(rendered)
    with args.receipt.open("x", encoding="utf-8") as stream:
        stream.write(json.dumps(receipt, indent=2, sort_keys=True, allow_nan=False) + "\n")
    print(json.dumps({"status": "FROZEN_PENDING_REVIEW", "config_sha256": receipt["config_sha256"],
                      "selected_pair_count": receipt["selected_pair_count"]}, sort_keys=True))


if __name__ == "__main__":
    main()
