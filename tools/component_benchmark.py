#!/usr/bin/env python3
"""Component benchmark validation, planning, execution and conservative report."""
from __future__ import annotations

import argparse
import json
import math
import os
from pathlib import Path

from component_benchmark.analysis import paired_log_cost, paired_median_ci, recommendation, reference_gate, same_achieved_accuracy
from component_benchmark.budget import CAP_SECONDS, close, reserve
from component_benchmark.protocol import canonical, digest, file_digest, physical_input_digest, strict_load, validate_case
from component_benchmark.registry import inventory, write_reports
from component_benchmark.runner import load_config, make_plan, run_plan, source_binding, validate_receipt

ROOT = Path(__file__).resolve().parents[1]


def cases_from_config(config):
    if "cases" in config:
        return config["cases"]
    from component_benchmark.cases import build_cases
    return build_cases()


def validated_reference(case, ref, provenance, root, source_hash):
    if not isinstance(ref,dict) or not isinstance(provenance,dict):
        return False
    path = Path(provenance.get("receipt_path", ""))
    if not path.is_absolute(): path = Path(root) / path
    if not path.is_file() or file_digest(path) != provenance.get("receipt_sha256"):
        return False
    receipt = strict_load(path.read_text())
    job = receipt.get("job")
    if receipt.get("status") != "OK" or not isinstance(job,dict) or job.get("measurement_mode") != "reference":
        return False
    if not validate_receipt(path.parent,receipt,job,source_hash):
        return False
    original_case = job.get("request",{}).get("case")
    if not isinstance(original_case,dict): return False
    if (physical_input_digest(case) != physical_input_digest(original_case) or
        provenance.get("physical_input_sha256") != physical_input_digest(original_case) or
        ref.get("physical_input_sha256") != physical_input_digest(original_case)):
        return False
    if provenance.get("source_binding_sha256") != source_hash or ref.get("reference_source_sha256") != source_hash:
        return False
    if provenance.get("request_sha256") != receipt.get("request_sha256"):
        return False
    if provenance.get("input_sha256") != digest(original_case) or ref.get("input_sha256") != digest(original_case):
        return False
    original_result = receipt.get("result")
    expected = dict(ref)
    expected.pop("reused_from",None)
    expected.pop("reuse_contract",None)
    return canonical(expected) == canonical(original_result)


def analyze(out, root):
    plan = strict_load((out / "plan.json").read_text())
    rows = []
    for job in plan["jobs"]:
        receipt = out / "jobs" / job["job_id"] / "receipt.json"
        if receipt.exists():
            row = strict_load(receipt.read_text())
            if not validate_receipt(receipt.parent,row,job,digest(plan["source_binding"])):
                row = {"job":job,"status":"INVALID_RECEIPT","result":None}
        else:
            row = {"job": job, "status": "MISSING", "result": None}
        rows.append(row)
    source_valid = source_binding(root) == plan["source_binding"]
    ref_path = out / "references.json"
    refs = strict_load(ref_path.read_text()) if ref_path.exists() else {}
    provenance_path = Path(str(ref_path) + ".provenance.json")
    provenance = strict_load(provenance_path.read_text()) if provenance_path.exists() else {}
    gates = {}
    for row in rows:
        job = row["job"]
        ref = refs.get(job["case_id"])
        result = row.get("result")
        key = job["job_id"]
        ref_usable = (validated_reference(job["request"]["case"],ref,provenance.get(job["case_id"]),root,digest(plan["source_binding"])) and
                      ref.get("status") == "OK" and
                      ref.get("reference_converged") is True and ref.get("times") == job["request"]["case"]["output_times"])
        gates[key] = reference_gate(job["request"]["case"], result.get("states") if result and row["status"] == "OK" else None,
                                    ref.get("states") if ref_usable else None,
                                    ref.get("uncertainty_abs") if ref_usable else None)
        allocation = result.get("allocations") if result else None
        if isinstance(allocation, dict) and (allocation.get("hot_loop_allocations") == 0 or allocation.get("solve") == 0):
            control = allocation.get("positive_control_allocations", allocation.get("positive_control"))
            if type(control) is not int or control <= 0:
                gates[key]["allocation_status"] = "HOLD_POSITIVE_CONTROL_MISSING"
            else:
                gates[key]["allocation_status"] = "OBSERVED_ZERO_WITH_POSITIVE_CONTROL"
        elif isinstance(allocation, dict) and (type(allocation.get("hot_loop_allocations")) is int and allocation["hot_loop_allocations"] > 0 or type(allocation.get("solve")) is int and allocation["solve"] > 0):
            gates[key]["allocation_status"] = "NONZERO_ALLOCATIONS"
        else:
            gates[key]["allocation_status"] = "HOLD_UNMEASURED"
    allocation_by_configuration = {}
    for row in rows:
        job = row["job"]
        if job["measurement_mode"] == "allocation":
            allocation_by_configuration[(job["case_id"], job["solver_id"], job["config_id"])] = gates[job["job_id"]]["allocation_status"]
    for row in rows:
        job = row["job"]
        if job["measurement_mode"] == "timing":
            gates[job["job_id"]]["allocation_status"] = allocation_by_configuration.get((job["case_id"], job["solver_id"], job["config_id"]), "HOLD_UNMEASURED")
    comparisons = []
    for contrast in plan.get("contrasts", []):
        ratios = []
        achieved = []
        for round_index in range(17):
            pair = {}
            for row in rows:
                j = row["job"]
                if (j["case_id"] == contrast["case_id"] and j["round"] == round_index and j["measurement_mode"] == "timing" and
                    j["solver_id"] in (contrast["baseline"], contrast["candidate"]) and
                    j["config_id"] == contrast.get(("baseline" if j["solver_id"] == contrast["baseline"] else "candidate") + "_config_id", "default")):
                    pair[j["solver_id"]] = row
            if len(pair) != 2 or any(x["status"] != "OK" or gates[x["job"]["job_id"]]["status"] != "PASS" for x in pair.values()):
                continue
            base_result = pair[contrast["baseline"]]["result"]
            cand_result = pair[contrast["candidate"]]["result"]
            log_cost = paired_log_cost(base_result, cand_result)
            if log_cost is not None:
                ratios.append(log_cost)
                achieved.append(same_achieved_accuracy(gates[pair[contrast["baseline"]]["job"]["job_id"]],
                                                       gates[pair[contrast["candidate"]]["job"]["job_id"]]))
        ci = paired_median_ci(ratios, plan.get("global_multiplicity",len(plan["contrasts"])))
        accuracy_matched = len(achieved) == 17 and all(x["status"] == "PASS" for x in achieved)
        candidate_allocation = [gates[r["job"]["job_id"]]["allocation_status"] for r in rows if r["job"]["case_id"] == contrast["case_id"] and r["job"]["solver_id"] == contrast["candidate"] and r["job"]["config_id"] == contrast.get("candidate_config_id", "default") and r["job"]["measurement_mode"] == "timing"]
        zero_allocation = len(candidate_allocation) == 17 and all(x == "OBSERVED_ZERO_WITH_POSITIVE_CONTROL" for x in candidate_allocation)
        comparisons.append({"contrast": contrast, "interval": ci,
                            "achieved_accuracy_ratio_max": max((x.get("worst_ratio",math.inf) for x in achieved),default=None),
                            "achieved_accuracy_gates": achieved,
                            "same_achieved_accuracy": accuracy_matched,
                            "candidate_zero_allocation_observed": zero_allocation,
                            "same_target": len(ratios) == 17,
                            "decision": recommendation(len(ratios) == 17 and source_valid and zero_allocation,
                                                       "PASS" if accuracy_matched else "HOLD", ci,
                                                       compensating_benefit=contrast.get("compensating_benefit"))})
    for comparison in comparisons:
        candidate = comparison["contrast"]["candidate"]
        critical = [x for x in comparisons if x["contrast"]["candidate"] == candidate and x["contrast"].get("critical") is True]
        complete = bool(critical) and all(x["interval"].get("status") == "INTERVAL" for x in critical)
        regression = any(x["interval"].get("status") == "INTERVAL" and x["interval"]["ci_speed_ratio"][1] <= .8 for x in critical)
        if comparison["decision"]["action"] == "promote" and (not complete or regression):
            comparison["decision"] = {"action": "need to optimize", "evidence_status": "HOLD", "reason": "CRITICAL_STRATUM_INCOMPLETE_OR_REGRESSION", "production_admission": "HOLD"}
    components = inventory(root)
    mapping = {r["component_id"]:r for r in components}
    for solver_id, spec in plan["solvers"].items():
        linked = spec.get("components", [])
        for component_id in linked:
            if component_id not in mapping:
                raise ValueError("UNKNOWN_COMPONENT_MAPPING: " + component_id)
            record = mapping[component_id]
            linked_rows = [r for r in rows if r["job"]["solver_id"] == solver_id]
            record.setdefault("solver_ids", []).append(solver_id)
            record["observed_jobs"] = record.get("observed_jobs",0) + len(linked_rows)
            record["passing_gates"] = record.get("passing_gates",0) + sum(gates[r["job"]["job_id"]]["status"] == "PASS" for r in linked_rows)
            record["failed_or_unsupported_jobs"] = record.get("failed_or_unsupported_jobs",0) + sum(r["status"] != "OK" for r in linked_rows)
            record["reason"] = "MAPPED_INCOMPLETE_EVIDENCE"
    for comparison in comparisons:
        decision = comparison["decision"]
        if decision["evidence_status"] != "CONFIRMED_DIAGNOSTIC": continue
        for component_id in plan["solvers"][comparison["contrast"]["candidate"]].get("components", []):
            mapping[component_id]["recommended_action"] = decision["action"]
            mapping[component_id]["evidence_status"] = decision["evidence_status"]
            mapping[component_id]["reason"] = "CONFIRMED_CONTRAST_" + comparison["contrast"]["case_id"]
    write_reports(components, out)
    report = {"schema": plan["schema"], "source_binding_valid": source_valid, "receipts": len(rows),
              "status_counts": {s: sum(r["status"] == s for r in rows) for s in sorted({r["status"] for r in rows})},
              "scientific_gates": gates, "comparisons": comparisons, "components": len(components),
              "reference_input_sha256": file_digest(ref_path) if ref_path.exists() else None,
              "reference_provenance_sha256": file_digest(provenance_path) if provenance_path.exists() else None,
              "timing_authority": "diagnostic_only", "production_admission": "HOLD"}
    (out / "analysis.json").write_text(json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n")
    return report


def main():
    p = argparse.ArgumentParser()
    p.add_argument("command", choices=("validate", "preflight", "plan", "run", "resume", "analyze", "lease-open", "lease-close"))
    p.add_argument("--config", type=Path)
    p.add_argument("--plan", type=Path)
    p.add_argument("--output", type=Path)
    p.add_argument("--phase", default="P1")
    p.add_argument("--kind", choices=("screening", "confirmation", "reference"), default="screening")
    p.add_argument("--concurrency", type=int, default=1)
    p.add_argument("--hpc", action="store_true", help="resource-admitted P1/P2 screening/reference scheduler; confirmation remains serial")
    p.add_argument("--ledger", type=Path)
    p.add_argument("--lease-id")
    p.add_argument("--seconds", type=float)
    p.add_argument("--retry-diagnosis")
    p.add_argument("--max-jobs", type=int)
    args = p.parse_args()
    if args.command == "lease-open":
        if args.ledger is None or args.seconds is None: p.error("--ledger and --seconds required")
        import platform
        lease_id = reserve(args.ledger,args.phase,args.seconds,platform.node(),persistent=True)
        print(json.dumps({"status":"LEASE_OPEN", "lease_id":lease_id,"ledger":str(args.ledger)}))
        return
    if args.command == "lease-close":
        if args.ledger is None or args.lease_id is None: p.error("--ledger and --lease-id required")
        print(json.dumps({"status":"LEASE_CLOSED","lease":close(args.ledger,args.lease_id)}))
        return
    if args.command in ("validate", "preflight", "plan"):
        if args.config is None: p.error("--config required")
        config = load_config(args.config)
        cases = cases_from_config(config)
        errors = {c.get("case_id", str(i)): validate_case(c) for i,c in enumerate(cases)}
        errors = {k:v for k,v in errors.items() if v}
        if errors: raise ValueError(errors)
        if args.command == "validate":
            print(json.dumps({"status": "VALID", "cases": len(cases), "solvers": len(config["solvers"])}))
            return
        if args.command == "preflight":
            available = {}
            for name,spec in config["solvers"].items():
                exe = spec["command"][0]
                available[name] = {"environment": "AVAILABLE" if (Path(exe).exists() or __import__("shutil").which(exe)) else "ENVIRONMENT_FAILURE",
                                   "equation_forms": spec["equation_forms"]}
            print(json.dumps({"status": "PREFLIGHT", "solvers": available, "source_binding_sha256": digest(source_binding(ROOT)), "cap_seconds": CAP_SECONDS}, indent=2))
            return
        plan = make_plan(cases, config, source_binding(ROOT), args.phase, args.kind)
        plan["contrasts"] = config.get("contrasts", [])
        if args.output is None: p.error("--output required")
        args.output.mkdir(parents=True, exist_ok=True)
        target = args.output / "plan.json"
        if target.exists(): raise FileExistsError(target)
        target.write_text(json.dumps(plan, indent=2, sort_keys=True, allow_nan=False) + "\n")
        print(json.dumps({"status": "PLANNED", "jobs": len(plan["jobs"]), "plan": str(target)}))
        return
    if args.command in ("run", "resume"):
        if args.plan is None or args.output is None: p.error("--plan and --output required")
        plan = strict_load(args.plan.read_text())
        if args.ledger is None: p.error("--ledger shared across all campaign phases is required")
        results = run_plan(plan, args.output, ROOT, args.ledger, args.concurrency, args.command == "resume", lease_id=args.lease_id, retry_diagnosis=args.retry_diagnosis, max_jobs=args.max_jobs, hpc=args.hpc)
        print(json.dumps({"status": "EXECUTED", "jobs": len(results), "counts": {s: sum(r["status"] == s for r in results) for s in sorted({r["status"] for r in results})}}))
        return
    if args.output is None: p.error("--output required")
    print(json.dumps(analyze(args.output, ROOT), indent=2))


if __name__ == "__main__":
    main()
