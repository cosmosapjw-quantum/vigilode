#!/usr/bin/env python3
"""A07 collective campaign: fixed physical tolerances, retained failures, generated report.

This complements the independently locked Robertson/Diffsol A07 campaign. It
never tunes physical tolerances or promotes speed. No third-party Python package
is required. Native timing includes the example's allocator instrumentation.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import random
import re
import shutil
import statistics
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = "collective-structured-comparison-v1"
ARMS = ("dense_fast", "structured")
TOLS = (1e-6, 1e-8, 1e-10)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_json(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True, allow_nan=False)+"\n")


def finite(value, positive=False):
    try:
        return isinstance(value, (float, int)) and not isinstance(value, bool) and math.isfinite(value) and (value > 0 if positive else value >= 0)
    except OverflowError:
        return False


def source_identity():
    files = [ROOT/"Cargo.toml", ROOT/"Cargo.lock", ROOT/"rust-toolchain.toml", Path(__file__).resolve()]
    files.extend(p for p in (ROOT/".cargo/config",ROOT/".cargo/config.toml") if p.is_file())
    for package in ("rodas5p-core", "rodas5p-integrators"):
        base = ROOT/"crates"/package
        files.append(base/"Cargo.toml")
        files.extend(base.rglob("*.rs"))
    # Compile-time literal assets are part of the executable target, notably
    # the actual floating RODAS coefficient snapshot; hashing only .rs misses it.
    for path in list(files):
        if path.suffix == ".rs":
            for relative in re.findall(r'include_(?:str|bytes)!\s*\(\s*"([^"\n]+)"\s*\)', path.read_text()):
                files.append((path.parent/relative).resolve())
    return {str(p.relative_to(ROOT)):digest(p) for p in sorted(set(files))}


def capture(argv, cwd=ROOT):
    started=time.perf_counter()
    try:
        result = subprocess.run(argv, cwd=cwd, text=True, capture_output=True, check=False)
    except OSError as exc:
        return {"argv":argv,"returncode":127,"stdout":"","stderr":str(exc),"wall_seconds":time.perf_counter()-started}
    return {"argv":argv,"returncode":result.returncode,"stdout":result.stdout,"stderr":result.stderr,"wall_seconds":time.perf_counter()-started}


def validate_row(row):
    errors = []
    if not isinstance(row, dict):
        return ["row_not_object"]
    for key, expected in (("schema",SCHEMA),("reference_authority","empirical_two_resolution"),
        ("reference","independent_classical_rk4_10240_steps"),
        ("measurement_scope","solver_and_adapter_setup_plus_integration_and_output"),
        ("coefficient_cache","warmed"),("promotion","DIAGNOSTIC_ONLY")):
        if row.get(key) != expected:
            errors.append("unexpected_"+key)
    if row.get("arm") not in ARMS:
        errors.append("unknown_arm")
    for key in ("elapsed_ns","atol","rtol"):
        if not finite(row.get(key), positive=True): errors.append("invalid_"+key)
    for key in ("repeat","modes"):
        if type(row.get(key)) is not int or row[key] < (1 if key=="modes" else 0): errors.append("invalid_"+key)
    if row.get("rtol") not in TOLS or not finite(row.get("rtol"),positive=True) or row.get("atol") != row["rtol"]*1e-3:
        errors.append("changed_physical_tolerance")
    if type(row.get("success")) is not bool:
        errors.append("invalid_success")
    calibrated = row.get("allocation_calibration_valid")
    calls = row.get("allocation_calibration_library_clone_calls")
    if type(calibrated) is not bool or type(calls) is not int or calls < 0 or calibrated != (calls >= 3):
        errors.append("invalid_allocator_positive_control")
    for key in ("allocation_calls","allocated_bytes"):
        v = row.get(key)
        if calibrated is True:
            if type(v) is not int or v < 0: errors.append("invalid_"+key)
        elif v is not None:
            errors.append("uncalibrated_allocation_claim")
    for key in ("reference_refinement_max_abs","reference_refinement_max_grid_wrms"):
        if not finite(row.get(key)): errors.append("invalid_"+key)
    if row.get("success") is True:
        if row.get("output_count") != 11: errors.append("incomplete_output_grid")
        for key in ("max_abs_reference_error","max_grid_wrms_reference_error","empirical_accuracy_gate_upper"):
            if not finite(row.get(key)): errors.append("invalid_"+key)
        e, u = row.get("max_grid_wrms_reference_error"), row.get("reference_refinement_max_grid_wrms")
        if finite(e) and finite(u):
            upper = e+u
            if row.get("empirical_accuracy_gate_upper") != upper: errors.append("false_accuracy_sum")
            if row.get("empirical_accuracy_gate_pass") is not (upper <= 1): errors.append("false_accuracy_gate")
    elif row.get("success") is False:
        if row.get("empirical_accuracy_gate_pass") is not False: errors.append("failed_arm_accuracy_claim")
    return errors


def load_rows(path, process_id=0):
    rows = []
    def reject(value): raise ValueError("nonstandard JSON constant: "+value)
    for index, line in enumerate(Path(path).read_text().splitlines()):
        try:
            row=json.loads(line, parse_constant=reject)
            errors=validate_row(row)
        except (ValueError,TypeError) as exc:
            row={"raw_line":line}
            errors=[str(exc)]
        rows.append({"process_id":process_id,"line":index+1,"raw":row,"errors":errors})
    return rows


def paired_interval(values, seed=71018):
    if not values: return None
    rng=random.Random(seed)
    med=statistics.median(values)
    boot=sorted(statistics.median(rng.choices(values,k=len(values))) for _ in range(2000))
    return {"median":med,"bootstrap_percentile_95":[boot[50],boot[1949]],
        "unit_count":len(values),"method":"seeded percentile bootstrap of paired process medians; diagnostic only",
        "insufficient_independent_processes":len(values)<3}


def summarize(entries, manifest, max_achieved_ratio=1.05):
    if not finite(max_achieved_ratio,positive=True) or max_achieved_ratio < 1:
        raise ValueError("achieved ratio must be finite and >=1")
    groups={}
    invalid=[e for e in entries if e["errors"]]
    duplicates=[]
    seen=set()
    for entry in entries:
        row=entry["raw"]
        if entry["errors"]: continue
        identity=(entry["process_id"],row["modes"],row["rtol"],row["repeat"],row["arm"])
        if identity in seen: duplicates.append(identity)
        seen.add(identity)
        key=(row["modes"],row["rtol"],row["atol"])
        groups.setdefault(key,[]).append(entry)
    expected=set()
    for process in manifest["processes"]:
        for tol in TOLS:
            for rep in range(manifest["repeats"]):
                for arm in ARMS: expected.add((process["id"],process["modes"],tol,rep,arm))
    missing=sorted(expected-seen)
    unexpected=sorted(seen-expected)
    valid_schema=not invalid and not duplicates and not missing and not unexpected
    source_bound=manifest.get("source_binding") == "captured_before_build_and_unchanged_after_runs"
    summaries=[]
    for (modes,rtol,atol), records in sorted(groups.items()):
        arms={}
        for arm in ARMS:
            selected=[e["raw"] for e in records if e["raw"]["arm"]==arm]
            successful=[r for r in selected if r["success"]]
            arms[arm]={"rows":len(selected),"successful":len(successful),
              "failed":len(selected)-len(successful),
              "max_abs_error":max((r["max_abs_reference_error"] for r in successful),default=None),
              "max_grid_wrms":max((r["max_grid_wrms_reference_error"] for r in successful),default=None),
              "empirical_accuracy_pass":bool(selected) and all(r["empirical_accuracy_gate_pass"] for r in selected),
              "allocator_calibrated":bool(selected) and all(r["allocation_calibration_valid"] for r in selected),
              "allocation_calls":sorted({r["allocation_calls"] for r in selected if r["allocation_calls"] is not None}),
              "timing_median_ns":statistics.median([r["elapsed_ns"] for r in successful]) if successful else None}
        err=[arms[a]["max_abs_error"] for a in ARMS]
        uncertainty=max(e["raw"]["reference_refinement_max_abs"] for e in records)
        # Conservative closeness check relative to empirical reference spread.
        # This is deliberately not a rigorous interval: uncertainty is empirical.
        ratio=(max(err)+uncertainty)/(min(err)-uncertainty) if all(v is not None for v in err) and min(err)>uncertainty else None
        matched=all(arms[a]["empirical_accuracy_pass"] for a in ARMS) and ratio is not None and ratio<=max_achieved_ratio
        pair_map={}
        for e in records:
            r=e["raw"]
            pair_map.setdefault((e["process_id"],r["repeat"]),{})[r["arm"]]=r
        process_ratios={}
        for (proc,_), pair in pair_map.items():
            if set(pair)==set(ARMS) and all(r["success"] for r in pair.values()):
                process_ratios.setdefault(proc,[]).append(pair["dense_fast"]["elapsed_ns"]/pair["structured"]["elapsed_ns"])
        paired=paired_interval([statistics.median(v) for _,v in sorted(process_ratios.items())])
        summaries.append({"modes":modes,"rtol":rtol,"atol":atol,"arms":arms,
          "achieved_absolute_error_ratio_with_empirical_spread":ratio,
          "achieved_error_ratio_limit":max_achieved_ratio,"matched_achieved_accuracy":matched,
          "paired_dense_over_structured_timing":paired,
          "matched_accuracy_comparison_admitted":valid_schema and source_bound and matched,
          "speed_promotion":"HOLD"})
    return {"schema_version":1,"node":"A07","schema_valid":valid_schema,
      "source_bound":source_bound,"rows_retained":len(entries),"invalid_rows":invalid,
      "duplicate_identities":duplicates,"missing_identities":missing,"unexpected_identities":unexpected,
      "raw_rows":entries,"groups":summaries,"speed_promotion":"HOLD",
      "reference_authority":"empirical_two_resolution_not_certified",
      "timing_instrumentation":"original example includes allocator counting; every arm identically instrumented; no uninstrumented speed authority",
      "scope":"collective structured-versus-dense; Robertson/Diffsol evidence is separate and reused unchanged"}


def markdown(data):
    lines=["# A07 collective matched-accuracy report","",
      f"Rows retained: {data['rows_retained']}. Schema valid: {data['schema_valid']}. Source binding: {data['source_bound']}.",
      "Speed promotion: **HOLD**. The reference and uncertainty are empirical; timings are instrumented diagnostics.","",
      "| Modes | rtol | dense / structured rows | Absolute-error ratio | Matched achieved accuracy | Paired median time ratio | Independent process units |",
      "|---:|---:|---:|---:|:---:|---:|---:|"]
    for g in data["groups"]:
        p=g["paired_dense_over_structured_timing"]
        lines.append(f"| {g['modes']} | {g['rtol']:.3g} | {g['arms']['dense_fast']['rows']} / {g['arms']['structured']['rows']} | {g['achieved_absolute_error_ratio_with_empirical_spread']} | {g['matched_achieved_accuracy']} | {p['median'] if p else None} | {p['unit_count'] if p else 0} |")
    lines.extend(["","Physical tolerances and output grid are fixed; no failed arm is removed. Reference calculation is excluded from every timed arm. Solver/adapter setup, integration and output are included; coefficient tables are prewarmed. New campaigns record complete process wall time and build time separately; process wall time includes startup and reference calculation, with no isolated startup measurement.","",
      "A source-bound matched pair is an admissible diagnostic comparison, not a production or speed promotion. The uncertainty interval uses independent process medians; one process is explicitly insufficient. Detailed failures, allocator calibration, raw measurements and intervals are in VALIDATED.json.",""])
    return "\n".join(lines)


def emit(out, manifest, raw_paths, ratio):
    entries=[]
    for process, path in zip(manifest["processes"],raw_paths):
        entries.extend(load_rows(path,process["id"]))
    result=summarize(entries,manifest,ratio)
    result["input_sha256"]={str(Path(p).name):digest(p) for p in raw_paths}
    result["manifest_sha256"]=digest(out/"MANIFEST.json")
    write_json(out/"VALIDATED.json",result)
    (out/"REPORT.md").write_text(markdown(result))
    assert (out/"REPORT.md").read_text()==markdown(json.loads((out/"VALIDATED.json").read_text()))
    return result


def run_campaign(args):
    out=Path(args.output).resolve()
    out.mkdir(parents=True,exist_ok=True)
    if any(out.iterdir()): raise ValueError("output directory must be empty")
    (out/".gitignore").write_text("collective_structured_compare\n")
    if any(m<=0 or m&(m-1) for m in args.modes): raise ValueError("modes must be powers of two")
    if args.repeats<1 or args.processes<1: raise ValueError("counts must be positive")
    if not finite(args.max_achieved_ratio,positive=True) or args.max_achieved_ratio<1:
        raise ValueError("achieved ratio must be finite and >=1")
    processes=[{"id":i,"modes":m} for i,m in enumerate(m for m in args.modes for _ in range(args.processes))]
    manifest={"schema_version":1,"processes":processes,"repeats":args.repeats,
      "physical_tolerances":{"rtol":TOLS,"atol":"rtol*1e-3"},"output_grid":"0.0 through 1.0 every 0.1, exact OutputSchedule implementation pinned by source",
      "achieved_error_ratio_limit":args.max_achieved_ratio,"source_sha256":source_identity(),
      "git_head":capture(["git","rev-parse","HEAD"]),"git_status":capture(["git","status","--porcelain=v1"]),
      "environment":{"platform":platform.platform(),"python":platform.python_version(),"cpu_count":os.cpu_count(),
        "RUSTFLAGS":os.environ.get("RUSTFLAGS"),"RAYON_NUM_THREADS":os.environ.get("RAYON_NUM_THREADS")},
      "rustc":capture(["rustc","-Vv"]),"cargo":capture(["cargo","-V"]),
      "source_binding":"PENDING","publication_or_speed_promotion":False}
    write_json(out/"MANIFEST.json",manifest) # Frozen protocol before measurements.
    argv=["cargo","build","--locked"]+([] if args.online else ["--offline"])+["--profile",args.profile,"-p","rodas5p-integrators","--example","collective_structured_compare","--message-format=json"]
    build=capture(argv)
    write_json(out/"BUILD.json",build)
    artifacts=[]
    for line in build["stdout"].splitlines():
        try: item=json.loads(line)
        except ValueError: continue
        if item.get("reason")=="compiler-artifact" and item.get("target",{}).get("name")=="collective_structured_compare" and item.get("executable"):
            artifacts.append(Path(item["executable"]))
    if build["returncode"] or not artifacts:
        write_json(out/"STATUS.json",{"status":"BLOCKED_BY_BUILD","build_returncode":build["returncode"]})
        return 2
    if source_identity()!=manifest["source_sha256"]:
        write_json(out/"STATUS.json",{"status":"BLOCKED_SOURCE_CHANGED_DURING_BUILD"})
        return 2
    binary=out/"collective_structured_compare"
    shutil.copy2(artifacts[-1],binary)
    manifest["binary_sha256"]=digest(binary)
    raw_paths=[]
    executions=[]
    for proc in processes:
        command=capture([str(binary),str(proc["modes"]),str(args.repeats)])
        name=f"process_{proc['id']:03d}"
        raw=out/(name+".jsonl")
        raw.write_text(command.pop("stdout"))
        (out/(name+".stderr")).write_text(command.pop("stderr"))
        executions.append(command)
        raw_paths.append(raw)
    unchanged=source_identity()==manifest["source_sha256"]
    manifest["source_binding"]="captured_before_build_and_unchanged_after_runs" if unchanged else "SOURCE_CHANGED_DURING_RUN"
    write_json(out/"MANIFEST.json",manifest)
    write_json(out/"EXECUTIONS.json",executions)
    result=emit(out,manifest,raw_paths,args.max_achieved_ratio)
    return 0 if result["schema_valid"] and unchanged and all(e["returncode"]==0 for e in executions) else 4


def import_retained(args):
    out=Path(args.output).resolve()
    out.mkdir(parents=True,exist_ok=True)
    if any(out.iterdir()): raise ValueError("output directory must be empty")
    rows=load_rows(args.input)
    valid=[r["raw"] for r in rows if not r["errors"]]
    modes=sorted({r["modes"] for r in valid})
    if len(modes)!=1: raise ValueError("retained import requires one mode-count process")
    manifest={"schema_version":1,"processes":[{"id":0,"modes":modes[0]}],
      "repeats":1+max(r["repeat"] for r in valid),"source_binding":"RETAINED_HISTORICAL_ROWS_NO_NEW_BUILD_BINDING",
      "input_path":str(Path(args.input)),"input_sha256":digest(args.input),
      "reused_without_execution":True,"achieved_error_ratio_limit":args.max_achieved_ratio}
    if args.receipt:
        manifest["historical_receipt_sha256"]=digest(args.receipt)
        receipt=json.loads(Path(args.receipt).read_text())
        manifest["historical_executed_head"]=receipt.get("executed_head")
        manifest["historical_published_source_commit"]=receipt.get("published_source_commit")
    write_json(out/"MANIFEST.json",manifest)
    target=out/"retained.jsonl"
    shutil.copyfile(args.input,target)
    result=emit(out,manifest,[target],args.max_achieved_ratio)
    return 0 if result["schema_valid"] else 4


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    commands=parser.add_subparsers(dest="command",required=True)
    run=commands.add_parser("run")
    run.add_argument("--modes",type=int,nargs="+",default=[8,32])
    run.add_argument("--repeats",type=int,default=5)
    run.add_argument("--processes",type=int,default=3)
    run.add_argument("--profile",choices=["measurement","release"],default="measurement")
    run.add_argument("--online",action="store_true")
    run.set_defaults(function=run_campaign)
    imp=commands.add_parser("import-retained")
    imp.add_argument("--input",required=True)
    imp.add_argument("--receipt")
    imp.set_defaults(function=import_retained)
    for command in (run,imp):
        command.add_argument("--output",required=True)
        command.add_argument("--max-achieved-ratio",type=float,default=1.05)
    args=parser.parse_args()
    return args.function(args)


if __name__=="__main__":
    sys.exit(main())
