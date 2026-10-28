#!/usr/bin/env python3
"""Serial, bounded paired runner. It deliberately never invokes Cargo."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

CASES = ("dense-fd", "banded-fd", "time-jvp", "autonomous-jvp")
VALIDATION_CASES = ("nonautonomous-dense-fd", "span-parameter-control")
ACCURACY_CAP = 1e-5
TIMEOUT = 90
CAMPAIGN_CAP = 1200
_STARTED = None

def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for b in iter(lambda: f.read(1024 * 1024), b""):
            h.update(b)
    return h.hexdigest()

def execute(cmd, dest, timeout, cpu):
    env = os.environ.copy()
    env.update({"RAYON_NUM_THREADS":"1", "OMP_NUM_THREADS":"1",
                "OPENBLAS_NUM_THREADS":"1", "MKL_NUM_THREADS":"1"})
    preexec = None
    if hasattr(os, "sched_setaffinity"):
        def pin(): os.sched_setaffinity(0, {cpu})
        preexec = pin
    remaining = CAMPAIGN_CAP - (time.monotonic() - _STARTED)
    record = {"argv": cmd, "timeout_seconds": min(timeout, max(0, remaining)), "cpu": cpu}
    start = time.monotonic()
    if remaining <= 0:
        record.update(exit_code=None, status="FAILED_BUDGET", elapsed_seconds=0.0)
        dest.with_suffix(".stdout").write_bytes(b"")
        dest.with_suffix(".stderr").write_bytes(b"cumulative 1200-second measurement budget exhausted\n")
        dest.with_suffix(".json").write_text(json.dumps(record, indent=2)+"\n")
        record["stdout_path"] = str(dest.with_suffix(".stdout")); record["stderr_path"] = str(dest.with_suffix(".stderr"))
        return record, None
    try:
        p = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                           timeout=min(timeout, remaining), env=env, preexec_fn=preexec,
                           check=False)
        record.update(exit_code=p.returncode, status="EXITED",
                      elapsed_seconds=time.monotonic()-start)
        out, err = p.stdout, p.stderr
    except subprocess.TimeoutExpired as e:
        record.update(exit_code=None, status="TIMEOUT",
                      elapsed_seconds=time.monotonic()-start)
        out = e.stdout or b""; err = e.stderr or b""
    except Exception as e:  # includes unavailable affinity/cpuset
        record.update(exit_code=None, status="LAUNCH_ERROR", error=repr(e),
                      elapsed_seconds=time.monotonic()-start)
        out, err = b"", repr(e).encode()
    dest.with_suffix(".stdout").write_bytes(out)
    dest.with_suffix(".stderr").write_bytes(err)
    dest.with_suffix(".json").write_text(json.dumps(record, indent=2)+"\n")
    record["stdout_path"] = str(dest.with_suffix(".stdout"))
    record["stderr_path"] = str(dest.with_suffix(".stderr"))
    parsed = None
    if record["status"] == "EXITED" and record["exit_code"] == 0:
        try: parsed = json.loads(out)
        except Exception as e: record.update(status="INVALID_JSON", parse_error=repr(e))
    dest.with_suffix(".json").write_text(json.dumps(record, indent=2)+"\n")
    return record, parsed

def comparable(v):
    x = json.loads(json.dumps(v))
    x.pop("mode", None)
    x.pop("setup_ns", None)
    for s in x.get("samples", []):
        s.pop("solver_ns", None); s.pop("solve_plus_drop_ns", None); s.pop("solve_allocations", None)
    return x

def callgrind_summary(path):
    events = None; total = None
    for line in path.read_text(errors="replace").splitlines():
        if line.startswith("events:"): events = line.partition(":")[2].split()
        elif line.startswith("summary:"):
            try: total = [int(x) for x in line.partition(":")[2].split()]
            except ValueError: pass
    return {"events":events,"summary":total}

def invoke(binpath, case, mode, repeats, outdir, tag, cpu, span=0.25, model_param=0.2):
    cmd = [binpath, "--case", case, "--mode", mode, "--repeats", str(repeats),
           "--span", str(span), "--model-param", str(model_param)]
    return execute(cmd, outdir/tag, TIMEOUT, cpu)

def failure_kind(record):
    if record.get("status") == "EXITED": return "PROCESS_EXIT_NONZERO"
    return record.get("status", "PROCESS_FAILURE")

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("--baseline", required=True); ap.add_argument("--baseline-sha", required=True)
    ap.add_argument("--baseline-binary-sha", required=True)
    ap.add_argument("--candidate", required=True); ap.add_argument("--candidate-sha", required=True)
    ap.add_argument("--candidate-binary-sha", required=True)
    ap.add_argument("--output", required=True); ap.add_argument("--cpu", type=int, default=0)
    ap.add_argument("--repeats", type=int, default=64, help="fixed per-process solve repetitions")
    ap.add_argument("--profile-repeats", default="4,12")
    a=ap.parse_args()
    global _STARTED
    out=Path(a.output)
    out.mkdir(parents=True, exist_ok=False)
    bins={"baseline":str(Path(a.baseline).resolve()),"candidate":str(Path(a.candidate).resolve())}
    expected={"baseline":a.baseline_binary_sha,"candidate":a.candidate_binary_sha}
    for k,p in bins.items():
        if not Path(p).is_file(): raise SystemExit(f"missing binary: {p}")
        actual=sha(p)
        if actual != expected[k]: raise SystemExit(f"{k} binary sha mismatch: expected {expected[k]} got {actual}")
    manifest={"schema":"rust-opt-light-v1","status":"RUNNING","baseline_source_sha":a.baseline_sha,
      "candidate_source_sha":a.candidate_sha,"binary_sha256":{k:sha(v) for k,v in bins.items()},
      "cpu":a.cpu,"thread_environment":1,"per_process_timeout_seconds":TIMEOUT,
      "accuracy_cap":ACCURACY_CAP,"measurement_budget_seconds":CAMPAIGN_CAP,"cases":{},"allocation":{},"validation_controls":{},
        "profile":{"status":"NOT_RUN"},"failures":[]}
    (out/"manifest.json").write_text(json.dumps(manifest,indent=2)+"\n")
    def save(): (out/"manifest.json").write_text(json.dumps(manifest,indent=2)+"\n")
    _STARTED=time.monotonic()
    def runpair(case, mode, repeats, tag, span=0.25, model_param=0.2):
        pair={}
        for k in ("baseline","candidate"):
            rec,val=invoke(bins[k],case,mode,repeats,out,f"{tag}-{k}",a.cpu,span,model_param)
            pair[k]={"receipt":rec,"value":val}
            if rec.get("status") != "EXITED" or rec.get("exit_code") != 0:
                manifest["failures"].append({"case":case,"mode":mode,"kind":failure_kind(rec),"binary":k,"receipt":rec})
        if any(x["value"] is None for x in pair.values()):
            manifest["failures"].append({"case":case,"mode":mode,"kind":"PROCESS_OR_JSON_FAILURE","pair":{k:v["receipt"] for k,v in pair.items()}})
        elif comparable(pair["baseline"]["value"]) != comparable(pair["candidate"]["value"]):
            manifest["failures"].append({"case":case,"mode":mode,"kind":"EXACT_PARITY_FAILURE","baseline":pair["baseline"]["value"],"candidate":pair["candidate"]["value"]})
        elif mode=="alloc":
            ba=pair["baseline"]["value"].get("samples",[]); ca=pair["candidate"]["value"].get("samples",[])
            baseline=[s.get("solve_allocations") for s in ba]; candidate=[s.get("solve_allocations") for s in ca]
            delta=[None if b is None or c is None else c-b for b,c in zip(baseline,candidate)]
            ratio=[None if b in (None,0) or c is None else c/b for b,c in zip(baseline,candidate)]
            pair["allocation_observation"]={"baseline":baseline,"candidate":candidate,"candidate_minus_baseline":delta,"candidate_over_baseline":ratio,"interpretation":"observed optimization target; differences are retained, not treated as parity failure"}
        return pair
    def accuracy_check(case, mode, span, param, pair):
        for k, item in pair.items():
            value=item["value"]
            if value is None: continue
            for i,s in enumerate(value.get("samples", [])):
                err=s.get("endpoint_max_abs_error")
                if err is not None and err > ACCURACY_CAP:
                    manifest["failures"].append({"case":case,"mode":mode,"kind":"ACCURACY_CAP_FAILURE","binary":k,"span":span,"model_param":param,"sample":i,"observed_error":err,"cap":ACCURACY_CAP})
                status=s.get("status",{})
                if status.get("status") != "success":
                    manifest["failures"].append({"case":case,"mode":mode,"kind":"SOLVER_STATUS_FAILURE","binary":k,"span":span,"model_param":param,"sample":i,"status":status})
    for case in CASES:
        for mode in ("probe","alloc"):
            pair=runpair(case,mode,1,f"{case}-{mode}")
            accuracy_check(case,mode,0.25,0.2,pair)
            manifest.setdefault("validation_controls",{}).setdefault(case,{})[mode]=pair; save()
    for case in VALIDATION_CASES:
        configs=[(0.25,0.2)] if case=="nonautonomous-dense-fd" else [(0.125,0.0),(0.125,0.4),(0.25,0.0),(0.25,0.4)]
        for span,param in configs:
            key=f"span={span};parameter={param}"
            for mode in ("probe","alloc"):
                pair=runpair(case,mode,1,f"{case}-{mode}-s{span}-p{param}",span,param)
                accuracy_check(case,mode,span,param,pair)
                manifest["validation_controls"].setdefault(case,{}).setdefault(key,{})[mode]=pair; save()
    for case in CASES:
        rows=[]
        # One warmup per executable before five alternating paired blocks.
        for k in ("baseline","candidate"):
            rec,val=invoke(bins[k],case,"timing",a.repeats,out,f"{case}-warmup-{k}",a.cpu)
            rows.append({"phase":"warmup","order":k,"receipt":rec,"value":val})
        for block in range(1,6):
            order=("baseline","candidate","candidate","baseline") if block%2 else ("candidate","baseline","baseline","candidate")
            for j,k in enumerate(order):
                rec,val=invoke(bins[k],case,"timing",a.repeats,out,f"{case}-block{block}-{j}-{k}",a.cpu)
                rows.append({"phase":"timed","block":block,"order":k,"receipt":rec,"value":val})
        for row in rows:
            rec=row["receipt"]
            if rec.get("status")!="EXITED" or rec.get("exit_code")!=0:
                manifest["failures"].append({"case":case,"mode":"timing","kind":failure_kind(rec),"receipt":rec})
        manifest["cases"][case]=rows; save()
    valgrind=shutil.which("valgrind"); annotate=shutil.which("callgrind_annotate")
    if not valgrind or not annotate:
        manifest["profile"]={"status":"NOT_RUN","reason":"valgrind or callgrind_annotate unavailable","valgrind":valgrind,"annotate":annotate}
    else:
        try: levels=[int(x) for x in a.profile_repeats.split(",")]
        except ValueError: raise SystemExit("--profile-repeats must be comma-separated positive integers")
        if len(levels)!=2 or any(x<1 for x in levels) or levels[0]>=levels[1]: raise SystemExit("profile requires two increasing positive repeat levels")
        manifest["profile"]={"status":"RUNNING","levels":levels,"runs":[]}
        for case in CASES:
            for k in bins:
                for n in levels:
                    cg=out/f"callgrind-{case}-{k}-r{n}.out"
                    cmd=[valgrind,"--tool=callgrind",f"--callgrind-out-file={cg}",bins[k],"--case",case,"--mode","timing","--repeats",str(n),"--span","0.25","--model-param","0.2"]
                    rec,val=execute(cmd,out/f"profile-{case}-{k}-r{n}",TIMEOUT,a.cpu)
                    if rec.get("status")!="EXITED" or rec.get("exit_code")!=0:
                        manifest["failures"].append({"case":case,"mode":"callgrind","kind":failure_kind(rec),"binary":k,"repeats":n,"receipt":rec})
                    ann=out/f"annotate-inclusive-{case}-{k}-r{n}.txt"
                    if rec.get("exit_code")==0 and cg.exists():
                        for inclusive, label in (("yes","inclusive"),("no","self")):
                            target=out/f"annotate-{label}-{case}-{k}-r{n}.txt"
                            left=CAMPAIGN_CAP-(time.monotonic()-_STARTED)
                            try:
                                q=subprocess.run([annotate,f"--inclusive={inclusive}","--threshold=0",str(cg)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=min(TIMEOUT,max(0,left)),check=False)
                                target.write_bytes(q.stdout); target.with_suffix(".stderr").write_bytes(q.stderr)
                                rec[f"{label}_annotation_exit_code"]=q.returncode
                                if q.returncode!=0: manifest["failures"].append({"case":case,"mode":"callgrind-annotate","kind":"ANNOTATION_FAILURE","binary":k,"repeats":n,"inclusive":inclusive,"exit_code":q.returncode})
                            except subprocess.TimeoutExpired as e:
                                target.write_bytes(e.stdout or b""); target.with_suffix(".stderr").write_bytes(e.stderr or b"")
                                rec[f"{label}_annotation_status"]="TIMEOUT"
                                manifest["failures"].append({"case":case,"mode":"callgrind-annotate","kind":"TIMEOUT","binary":k,"repeats":n,"inclusive":inclusive})
                            except Exception as e:
                                target.write_bytes(b""); target.with_suffix(".stderr").write_text(repr(e)+"\n")
                                rec[f"{label}_annotation_status"]="LAUNCH_ERROR"
                                manifest["failures"].append({"case":case,"mode":"callgrind-annotate","kind":"LAUNCH_ERROR","binary":k,"repeats":n,"inclusive":inclusive,"error":repr(e)})
                    manifest["profile"]["runs"].append({"case":case,"binary":k,"repeats":n,"receipt":rec,"callgrind_file":str(cg),"annotation":str(ann)})
                    save()
        manifest["profile"]["instruction_summaries"]={}
        for case in CASES:
            for k in bins:
                files=[(n,callgrind_summary(out/f"callgrind-{case}-{k}-r{n}.out")) for n in levels if (out/f"callgrind-{case}-{k}-r{n}.out").exists()]
                slope=None
                if len(files)==2 and files[0][1]["summary"] and files[1][1]["summary"]:
                    if files[0][1]["events"]==files[1][1]["events"] and "Ir" in files[0][1]["events"]:
                        idx=files[0][1]["events"].index("Ir")
                        slope=(files[1][1]["summary"][idx]-files[0][1]["summary"][idx])/(levels[1]-levels[0])
                manifest["profile"]["instruction_summaries"][f"{case}/{k}"]={"levels":files,"Ir_per_solve_slope":slope,"note":"slope is diagnostic and subtracts fixed process/setup instruction cost by differencing repeat levels"}
        manifest["profile"]["status"]="COMPLETE_WITH_FAILURES" if any(x.get("mode") in ("callgrind","callgrind-annotate") or x.get("kind") in ("TIMEOUT","LAUNCH_ERROR","PROFILE_FAILURE") for x in manifest["failures"]) else "COMPLETE"
    manifest["status"]="FAIL" if manifest["failures"] else "COMPLETE"
    save()
    print(json.dumps({"status":manifest["status"],"output":str(out),"failure_count":len(manifest["failures"])}))
    return 1 if manifest["failures"] else 0

if __name__=="__main__": sys.exit(main())
