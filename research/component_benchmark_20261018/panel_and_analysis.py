#!/usr/bin/env python3
"""Read-only P2 panel assembly. Rankings are coverage based, never speed based."""
from __future__ import annotations

import argparse
import csv
import hashlib
import importlib.util
import json
import math
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from component_benchmark.analysis import paired_log_cost, reference_gate, same_achieved_accuracy
from component_benchmark.protocol import digest, file_digest, physical_input_digest, strict_load
from component_benchmark.runner import source_binding, validate_receipt


def cli_reference_validator(root):
    spec = importlib.util.spec_from_file_location("_p2_frozen_cli", root / "tools/component_benchmark.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.validated_reference


def rel(root, path):
    try:
        return str(Path(path).resolve().relative_to(root))
    except ValueError:
        return str(path)


def read_plan(folder):
    path = folder / "plan.json"
    return strict_load(path.read_text()), file_digest(path)


def read_receipt(root, folder, job, source_sha):
    path = folder / "jobs" / job["job_id"] / "receipt.json"
    if not path.is_file():
        return None, "MISSING_RECEIPT", rel(root, path), None
    try:
        raw = path.read_bytes()
        sha = hashlib.sha256(raw).hexdigest()
        row = strict_load(raw.decode())
        if not validate_receipt(path.parent, row, job, source_sha):
            return None, "INVALID_RECEIPT_IDENTITY", rel(root, path), sha
        return row, row.get("status", "MISSING_STATUS"), rel(root, path), sha
    except (OSError, ValueError, KeyError, TypeError):
        return None, "INVALID_RECEIPT_IDENTITY", rel(root, path), locals().get("sha")


def allocation_state(receipt, status):
    if status != "OK" or not isinstance(receipt, dict):
        return "HOLD_" + status
    alloc = (receipt.get("result") or {}).get("allocations")
    if not isinstance(alloc, dict):
        return "HOLD_UNMEASURED"
    count = alloc.get("hot_loop_allocations", alloc.get("solve"))
    control = alloc.get("positive_control_allocations", alloc.get("positive_control"))
    if type(count) is not int or count < 0:
        return "HOLD_UNMEASURED"
    if count > 0:
        return "NONZERO_ALLOCATIONS"
    return "OBSERVED_ZERO_WITH_POSITIVE_CONTROL" if type(control) is int and control > 0 else "HOLD_POSITIVE_CONTROL_MISSING"


def arm_rows(root, p2, refs, provenance, source_sha, validator, prerequisites, probes):
    screening, screening_sha = read_plan(p2 / "screening")
    allocation, allocation_sha = read_plan(p2 / "allocation")
    if screening["kind"] != "screening" or allocation["kind"] != "screening":
        raise RuntimeError("P2_PLAN_KIND_MISMATCH")
    if digest(screening["source_binding"]) != source_sha or digest(allocation["source_binding"]) != source_sha:
        raise RuntimeError("P2_SOURCE_BINDING_MISMATCH")
    reference_cache = {}
    allocations = {}
    all_rows = []
    for queue, plan, plan_sha in (("allocation", allocation, allocation_sha), ("screening", screening, screening_sha)):
        for job in plan["jobs"]:
            receipt, status, receipt_path, receipt_sha = read_receipt(root, p2 / queue, job, source_sha)
            case = job["request"]["case"]
            cid = job["case_id"]
            reference = refs.get(cid)
            prov = provenance.get(cid)
            if cid in reference_cache and reference_cache[cid][2] != digest(case):
                raise RuntimeError("CASE_ID_INPUT_CONFLICT: " + cid)
            if cid not in reference_cache:
                try:
                    valid = bool(validator(case, reference, prov, root, source_sha))
                except (OSError, ValueError, TypeError, KeyError):
                    valid = False
                usable = (valid and isinstance(reference, dict) and reference.get("status") == "OK"
                          and reference.get("reference_converged") is True
                          and reference.get("times") == case["output_times"])
                reference_cache[cid] = (usable, valid, digest(case))
            ref_usable, ref_identity_valid, _ = reference_cache[cid]
            result = receipt.get("result") if receipt else None
            if not isinstance(result, dict):
                result = None
            try:
                gate = reference_gate(case, result.get("states") if result and status == "OK" else None,
                                      reference.get("states") if ref_usable else None,
                                      reference.get("uncertainty_abs") if ref_usable else None)
            except (KeyError, ValueError, TypeError, ArithmeticError):
                gate = {"status": "HOLD", "reason": "INVALID_NUMERICAL_INPUT"}
            attempts = receipt.get("attempts", []) if receipt else []
            attempt = attempts[-1] if attempts else {}
            spec = plan["solvers"][job["solver_id"]]
            prerequisite = prerequisites.get((cid, job["solver_id"]), {})
            if job.get("prerequisite") and job["prerequisite"].get("evidence_sha256") != prerequisite.get("evidence_sha256"):
                raise RuntimeError("P2_PREREQUISITE_EVIDENCE_MISMATCH: " + cid + "/" + job["solver_id"])
            probe = probes.get((physical_input_digest(case), prerequisite.get("probe_adapter")), {})
            timing = result.get("timing") if result else None
            counters = result.get("counters") if result else None
            key = (cid, job["solver_id"], job["config_id"])
            row = {
                "queue": queue, "plan_sha256": plan_sha, "job_id": job["job_id"],
                "case_id": cid, "family": case.get("family"), "solver_id": job["solver_id"],
                "config_id": job["config_id"], "measurement_mode": job["measurement_mode"],
                "capability": job["capability"], "status": status,
                "receipt_path": receipt_path, "receipt_sha256": receipt_sha,
                "request_sha256": receipt.get("request_sha256") if receipt else None,
                "source_binding_sha256": source_sha, "physical_input_sha256": physical_input_digest(case),
                "case_sha256": digest(case), "reference_path": "research/component_benchmark_20261018/ncp_execution_01/references.json",
                "reference_provenance_path": "research/component_benchmark_20261018/ncp_execution_01/references.json.provenance.json",
                "reference_receipt_path": prov.get("receipt_path") if isinstance(prov, dict) else None,
                "reference_receipt_sha256": prov.get("receipt_sha256") if isinstance(prov, dict) else None,
                "prerequisite_evidence_path": rel(root, p2 / "prerequisites/prerequisite_evidence.json"),
                "prerequisite_evidence_sha256": prerequisite.get("evidence_sha256"),
                "probe_adapter": prerequisite.get("probe_adapter"),
                "probe_receipt_path": probe.get("path"),
                "probe_receipt_sha256": prerequisite.get("probe_receipt_sha256"),
                "probe_status": prerequisite.get("probe_reason"),
                "reference_identity_valid": ref_identity_valid,
                "reference_converged": reference.get("reference_converged") if isinstance(reference, dict) else None,
                "gate_status": gate.get("status"), "gate_reason": gate.get("reason"),
                "L": gate.get("achieved_error_lower"), "U": gate.get("achieved_error_upper"),
                "max_error_budget_ratio": gate.get("max_error_budget_ratio"),
                "max_uncertainty_budget_ratio": gate.get("max_uncertainty_budget_ratio"),
                "timing_scope": result.get("timing_scope") if result else None,
                "timed_seconds": timing.get("timed_seconds") if isinstance(timing, dict) else None,
                "warmup_seconds": timing.get("warmup_seconds") if isinstance(timing, dict) else None,
                "process_wall_seconds": (attempt.get("ended_unix") - attempt.get("started_unix")
                    if isinstance(attempt.get("ended_unix"), (int,float)) and isinstance(attempt.get("started_unix"), (int,float)) else None),
                "peak_rss_bytes": attempt.get("peak_rss_bytes"),
                "requested_threads": spec.get("threads", job["request"].get("solver_config", {}).get("threads", 1)),
                "observed_thread_environment": attempt.get("observed_thread_environment"),
                "counters": counters, "allocations": result.get("allocations") if result else None,
                "gate": gate,
            }
            if queue == "allocation":
                state = allocation_state(receipt, status)
                row["allocation_status"] = state
                allocations[key] = {"status": state, "job_id": job["job_id"],
                                    "receipt_path": receipt_path, "receipt_sha256": receipt_sha,
                                    "allocations": row["allocations"]}
            all_rows.append(row)
    for row in all_rows:
        if row["queue"] == "screening":
            link = allocations.get((row["case_id"], row["solver_id"], row["config_id"]))
            row["allocation_status"] = link["status"] if link else "HOLD_UNMEASURED"
            row["allocation_link"] = link
        else:
            row["allocation_link"] = None
    return all_rows, {"screening": screening_sha, "allocation": allocation_sha}


def pair_candidates(rows):
    by_case = defaultdict(list)
    for row in rows:
        if row["queue"] == "screening" and row["measurement_mode"] == "timing":
            by_case[row["case_id"]].append(row)
    out = []
    for cid, arms in sorted(by_case.items()):
        arms.sort(key=lambda x: (x["solver_id"], x["config_id"]))
        for i, base in enumerate(arms):
            for candidate in arms[i+1:]:
                # A primary contrast compares distinct solver IDs or explicit configurations.
                if (base["solver_id"], base["config_id"]) == (candidate["solver_id"], candidate["config_id"]):
                    continue
                if base["status"] != "OK" or candidate["status"] != "OK":
                    continue
                if base["gate_status"] != "PASS" or candidate["gate_status"] != "PASS":
                    continue
                br = {"measurement_mode": "timing", "timing_scope": base["timing_scope"],
                      "timing": {"timed_seconds": base["timed_seconds"], "warmup_seconds": base["warmup_seconds"]}}
                cr = {"measurement_mode": "timing", "timing_scope": candidate["timing_scope"],
                      "timing": {"timed_seconds": candidate["timed_seconds"], "warmup_seconds": candidate["warmup_seconds"]}}
                log_cost = paired_log_cost(br, cr)
                if log_cost is None:
                    continue
                match = same_achieved_accuracy(base["gate"], candidate["gate"])
                out.append({
                    "case_id": cid, "family": base["family"],
                    "physical_input_sha256": base["physical_input_sha256"],
                    "source_binding_sha256": base["source_binding_sha256"],
                    "baseline": base["solver_id"], "baseline_config_id": base["config_id"],
                    "candidate": candidate["solver_id"], "candidate_config_id": candidate["config_id"],
                    "baseline_job_id": base["job_id"], "candidate_job_id": candidate["job_id"],
                    "baseline_receipt_path": base["receipt_path"], "candidate_receipt_path": candidate["receipt_path"],
                    "baseline_receipt_sha256": base["receipt_sha256"], "candidate_receipt_sha256": candidate["receipt_sha256"],
                    "baseline_reference_receipt_path": base["reference_receipt_path"],
                    "candidate_reference_receipt_path": candidate["reference_receipt_path"],
                    "reference_receipt_sha256": base["reference_receipt_sha256"],
                    "baseline_probe_receipt_path": base["probe_receipt_path"],
                    "candidate_probe_receipt_path": candidate["probe_receipt_path"],
                    "baseline_probe_receipt_sha256": base["probe_receipt_sha256"],
                    "candidate_probe_receipt_sha256": candidate["probe_receipt_sha256"],
                    "baseline_prerequisite_evidence_sha256": base["prerequisite_evidence_sha256"],
                    "candidate_prerequisite_evidence_sha256": candidate["prerequisite_evidence_sha256"],
                    "baseline_L": base["L"], "baseline_U": base["U"],
                    "candidate_L": candidate["L"], "candidate_U": candidate["U"],
                    "matched_achieved_status": match["status"],
                    "matched_achieved_reason": match.get("reason"),
                    "matched_achieved_worst_ratio": match.get("worst_ratio"),
                    "same_target_status": "PASS",
                    "baseline_timed_seconds": base["timed_seconds"],
                    "candidate_timed_seconds": candidate["timed_seconds"],
                    "screening_speed_ratio_diagnostic": math.exp(log_cost),
                    "baseline_peak_rss_bytes": base["peak_rss_bytes"],
                    "candidate_peak_rss_bytes": candidate["peak_rss_bytes"],
                    "baseline_threads": base["requested_threads"],
                    "candidate_threads": candidate["requested_threads"],
                    "baseline_allocation_status": base["allocation_status"],
                    "candidate_allocation_status": candidate["allocation_status"],
                    "baseline_allocation_job_id": (base["allocation_link"] or {}).get("job_id"),
                    "candidate_allocation_job_id": (candidate["allocation_link"] or {}).get("job_id"),
                    "timing_authority": "diagnostic_only", "production_admission": "HOLD",
                })
    return out


def rank_prospective(pairs, limit=16):
    coverage = defaultdict(lambda: {"cases": set(), "families": set(), "matched": set()})
    for p in pairs:
        key = (p["baseline"], p["baseline_config_id"], p["candidate"], p["candidate_config_id"])
        cell = coverage[key]
        cell["cases"].add(p["case_id"])
        cell["families"].add(p["family"])
        if p["matched_achieved_status"] == "PASS":
            cell["matched"].add(p["case_id"])
    for p in pairs:
        key = (p["baseline"], p["baseline_config_id"], p["candidate"], p["candidate_config_id"])
        c = coverage[key]
        p["solver_pair_same_target_case_coverage"] = len(c["cases"])
        p["solver_pair_matched_case_coverage"] = len(c["matched"])
        p["solver_pair_family_coverage"] = len(c["families"])
    eligible = [p for p in pairs if p["matched_achieved_status"] == "PASS"
                and type(p["baseline_peak_rss_bytes"]) is int and p["baseline_peak_rss_bytes"] > 0
                and type(p["candidate_peak_rss_bytes"]) is int and p["candidate_peak_rss_bytes"] > 0]
    eligible.sort(key=lambda p: (-p["solver_pair_matched_case_coverage"],
                                 -p["solver_pair_same_target_case_coverage"],
                                 -p["solver_pair_family_coverage"],
                                 p["baseline"], p["baseline_config_id"], p["candidate"],
                                 p["candidate_config_id"], p["case_id"]))
    selected = []
    used_pairs = set()
    for p in eligible:
        key = (p["baseline"], p["baseline_config_id"], p["candidate"], p["candidate_config_id"])
        if key in used_pairs:
            continue
        selected.append(p)
        used_pairs.add(key)
        if len(selected) == limit:
            break
    return selected


def write_csv(path, rows, fields):
    with path.open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields, extrasaction="ignore")
        writer.writeheader()
        for row in rows:
            writer.writerow({k: json.dumps(row[k], sort_keys=True, allow_nan=False) if isinstance(row.get(k), (dict,list)) else row.get(k)
                             for k in fields})


def telemetry_summary(path):
    if not path.is_file():
        return {"status": "MISSING_TELEMETRY", "path": str(path)}
    samples = [strict_load(line) for line in path.read_text().splitlines() if line.strip()]
    if not samples:
        return {"status": "EMPTY_TELEMETRY", "path": str(path)}
    samples.sort(key=lambda x: x["unix"])
    def maximum(key):
        values = [x[key] for x in samples if type(x.get(key)) in (int,float) and math.isfinite(x[key])]
        return max(values) if values else None
    def weighted(key):
        weighted_sum = duration = 0.0
        for a,b in zip(samples,samples[1:]):
            dt = max(0.0,b["unix"]-a["unix"])
            value = a.get(key)
            if type(value) in (int,float) and math.isfinite(value):
                weighted_sum += value*dt
                duration += dt
        return weighted_sum/duration if duration else None
    return {"status": "OBSERVED", "path": str(path), "samples": len(samples),
            "elapsed_first_to_last_seconds": samples[-1]["unix"]-samples[0]["unix"],
            "host": samples[0].get("host"), "cpuset": samples[0].get("cpuset"),
            "worker_cpuset": samples[0].get("worker_cpuset"),
            "reserved_cpuset": samples[0].get("reserved_cpuset"),
            "memory_budget_bytes": samples[0].get("memory_budget_bytes"),
            "max_allocated_cores": maximum("allocated_cores"),
            "time_weighted_mean_allocated_cores": weighted("allocated_cores"),
            "max_allocated_memory_bytes": maximum("allocated_memory_bytes"),
            "max_running_jobs": maximum("running"),
            "max_completed_jobs": maximum("completed"),
            "cpu_busy_fraction_peak_host_observed": maximum("cpu_busy_fraction"),
            "cpu_busy_fraction_time_weighted_mean_host_observed": weighted("cpu_busy_fraction"),
            "cpu_steal_fraction_peak_host_observed": maximum("cpu_steal_fraction"),
            "cpu_steal_fraction_time_weighted_mean_host_observed": weighted("cpu_steal_fraction"),
            "own_child_rss_bytes_peak": maximum("own_child_rss_bytes"),
            "external_background_status": sorted({x.get("external_background_status", "UNKNOWN") for x in samples})}


def lease_summary(p2):
    context_path = p2 / "context.json"
    if not context_path.is_file():
        return {"status": "MISSING_CONTEXT"}
    context = strict_load(context_path.read_text())
    ledger_path = Path(context["ledger"])
    if not ledger_path.is_file():
        return {"status": "MISSING_LEDGER", "context_sha256": file_digest(context_path)}
    ledger = strict_load(ledger_path.read_text())
    matches = [x for x in ledger.get("leases", []) if x.get("id") == context.get("lease_id")]
    if len(matches) != 1 or matches[0].get("phase") != "P2":
        return {"status": "LEASE_IDENTITY_MISMATCH", "context_sha256": file_digest(context_path),
                "ledger_sha256": file_digest(ledger_path)}
    lease = matches[0]
    return {"status": "OBSERVED", "context_sha256": file_digest(context_path),
            "ledger_sha256": file_digest(ledger_path), "lease_id": lease["id"],
            "host": lease.get("host"), "phase": lease["phase"],
            "reserved_seconds": lease.get("reserved_seconds"),
            "start_unix": lease.get("start"), "deadline_unix": lease.get("deadline"),
            "end_unix": lease.get("end"), "persistent": lease.get("persistent")}


def command_summary(p2):
    rows = []
    for path in sorted((p2 / "commands").glob("*.json")):
        try:
            item = strict_load(path.read_text())
            rows.append({"path": str(path), "sha256": file_digest(path),
                         "exit_code": item.get("exit_code"),
                         "elapsed_seconds": item.get("elapsed_seconds")})
        except (OSError, ValueError, TypeError):
            rows.append({"path": str(path), "status": "INVALID_COMMAND_RECEIPT"})
    return rows


def generate(root, p1, p2, output, validator=None):
    root, p1, p2, output = map(lambda p: Path(p).resolve(), (root, p1, p2, output))
    if not (p2 / "P2_QUEUES_FINISHED.json").is_file():
        raise RuntimeError("P2_QUEUES_NOT_FINISHED")
    if output.exists():
        raise FileExistsError("output must be a new directory")
    source_sha = digest(source_binding(root))
    refs_path, prov_path = p2 / "screening/references.json", p2 / "screening/references.json.provenance.json"
    if not refs_path.is_file() or not prov_path.is_file():
        raise RuntimeError("P2_REFERENCE_COPY_MISSING")
    if (file_digest(refs_path) != file_digest(p1 / "references.json") or
        file_digest(prov_path) != file_digest(p1 / "references.json.provenance.json")):
        raise RuntimeError("P1_P2_REFERENCE_COPY_MISMATCH")
    refs, provenance = strict_load(refs_path.read_text()), strict_load(prov_path.read_text())
    prerequisite_path = p2 / "prerequisites/prerequisite_evidence.json"
    prerequisite_data = strict_load(prerequisite_path.read_text())
    prerequisites = {(x["case_id"], x["solver_id"]): x for x in prerequisite_data["entries"]}
    probe_plan = strict_load((p1 / "probes/plan.json").read_text())
    probes = {(x["physical_input_sha256"], x["solver_id"]):
              {"path": rel(root, p1 / "probes/jobs" / x["job_id"] / "receipt.json")}
              for x in probe_plan["jobs"]}
    validator = validator or cli_reference_validator(root)
    rows, plans = arm_rows(root, p2, refs, provenance, source_sha, validator, prerequisites, probes)
    pairs = pair_candidates(rows)
    prospective = rank_prospective(pairs)
    output.mkdir(parents=True)
    with (output / "all_arms.jsonl").open("w") as stream:
        for row in rows:
            stream.write(json.dumps(row, sort_keys=True, allow_nan=False) + "\n")
    arm_fields = ("queue", "job_id", "case_id", "family", "solver_id", "config_id", "measurement_mode",
                  "capability", "status", "gate_status", "gate_reason", "L", "U",
                  "max_error_budget_ratio", "max_uncertainty_budget_ratio", "timed_seconds",
                  "warmup_seconds", "process_wall_seconds", "peak_rss_bytes", "requested_threads",
                  "allocation_status", "receipt_path", "receipt_sha256", "source_binding_sha256",
                  "physical_input_sha256", "reference_receipt_path", "reference_receipt_sha256",
                  "prerequisite_evidence_sha256", "probe_adapter", "probe_receipt_path",
                  "probe_receipt_sha256", "probe_status")
    write_csv(output / "all_arms.csv", rows, arm_fields)
    with (output / "all_valid_same_target_pairs.jsonl").open("w") as stream:
        for pair in pairs:
            stream.write(json.dumps(pair, sort_keys=True, allow_nan=False) + "\n")
    pair_fields = ("case_id", "family", "baseline", "baseline_config_id", "candidate", "candidate_config_id",
                   "same_target_status", "matched_achieved_status", "matched_achieved_reason",
                   "matched_achieved_worst_ratio", "baseline_L", "baseline_U", "candidate_L", "candidate_U",
                   "solver_pair_matched_case_coverage", "solver_pair_same_target_case_coverage",
                   "solver_pair_family_coverage", "baseline_timed_seconds", "candidate_timed_seconds",
                   "screening_speed_ratio_diagnostic", "baseline_peak_rss_bytes", "candidate_peak_rss_bytes",
                   "baseline_threads", "candidate_threads", "baseline_allocation_status", "candidate_allocation_status",
                   "baseline_job_id", "candidate_job_id", "baseline_receipt_path", "candidate_receipt_path",
                   "baseline_receipt_sha256", "candidate_receipt_sha256", "reference_receipt_sha256",
                   "baseline_probe_receipt_path", "candidate_probe_receipt_path",
                   "baseline_probe_receipt_sha256", "candidate_probe_receipt_sha256",
                   "baseline_prerequisite_evidence_sha256", "candidate_prerequisite_evidence_sha256",
                   "physical_input_sha256", "source_binding_sha256")
    write_csv(output / "all_valid_same_target_pairs.csv", pairs, pair_fields)
    scaling = [row for row in rows if row["queue"] == "screening" and
               row["solver_id"].startswith("native-pexprb54s4")]
    write_csv(output / "thread_scaling_arms.csv", scaling, arm_fields)
    telemetry = {queue: telemetry_summary(p2 / queue / "hpc_telemetry.jsonl")
                 for queue in ("screening", "allocation")}
    summary = {"schema": "vigilode-p2-mechanical-panel-v1", "source_binding_sha256": source_sha,
               "p2_plan_sha256": plans, "p2_finished_marker_sha256": file_digest(p2 / "P2_QUEUES_FINISHED.json"),
               "references_sha256": file_digest(refs_path), "reference_provenance_sha256": file_digest(prov_path),
               "arm_count": len(rows), "arm_status_counts": dict(Counter(row["status"] for row in rows)),
               "arm_gate_counts": dict(Counter(row["gate_status"] for row in rows)),
               "same_target_pair_count": len(pairs),
               "matched_achieved_pair_count": sum(p["matched_achieved_status"] == "PASS" for p in pairs),
               "prospective_count": len(prospective), "prospective_for_independent_review": prospective,
               "hpc_telemetry": telemetry, "host_lease_reservation": lease_summary(p2),
               "command_receipts": command_summary(p2),
               "thread_scaling_arm_count": len(scaling),
               "selection_rule": "coverage counts, then lexical IDs; distinct solver/config pair; no speed ranking",
               "selection_authority": "REVIEW_REQUIRED", "timing_authority": "diagnostic_only",
               "production_admission": "HOLD", "p3_confirmation": "NOT_MEASURED"}
    (output / "panel_summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True, allow_nan=False) + "\n")
    return {k: summary[k] for k in ("arm_count", "arm_status_counts", "same_target_pair_count",
                                   "matched_achieved_pair_count", "prospective_count")}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--p1", type=Path, required=True)
    parser.add_argument("--p2", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(generate(args.root, args.p1, args.p2, args.output), sort_keys=True))


if __name__ == "__main__":
    main()
