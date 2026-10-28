"""Frozen campaign materialization and reference jobs; never changes solver policy."""
from __future__ import annotations

import argparse
import copy
import json
import os
import sys
import time
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from component_benchmark.cases import build_cases
    from component_benchmark.protocol import digest, strict_load, file_digest, physical_input_digest
    from component_benchmark.registry import inventory, write_reports
    from component_benchmark.runner import atomic_json, source_binding, validate_receipt
else:
    from .cases import build_cases
    from .protocol import digest, strict_load, file_digest, physical_input_digest
    from .registry import inventory, write_reports
    from .runner import atomic_json, source_binding, validate_receipt

BASE = "26f0b3a187ab41f7d68922491cc489b268c0caa1"
THREADS = {k: "1" for k in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "BLIS_NUM_THREADS", "RAYON_NUM_THREADS", "JULIA_NUM_THREADS")}


def expanded_cases():
    anchors = build_cases()
    cases = copy.deepcopy(anchors)
    for case in anchors:
        if not case["case_id"].startswith("ode-") or case["case_id"] in ("ode-pr101", "ode-robertson", "ode-sabr-scalar-fixture"):
            continue
        for rtol, atol, tag in ((1e-4, 1e-7, "rtol1e-4"), (1e-6, 1e-9, "rtol1e-6")):
            variant = copy.deepcopy(case)
            variant.update(case_id=case["case_id"]+"@"+tag, base_case_id=case["case_id"], rtol=rtol, atol=[atol]*len(case["atol"]))
            cases.append(variant)
    # Tight analytic controls do not introduce a new physical model or holdout.
    for cid in ("ode-scalar-decay", "ode-rotation-1"):
        variant = copy.deepcopy(next(c for c in anchors if c["case_id"] == cid))
        variant.update(case_id=cid+"@rtol1e-10", base_case_id=cid, rtol=1e-10, atol=[1e-13]*len(variant["atol"]))
        cases.append(variant)
    return cases


def configurations(root, env_root):
    root, env_root = Path(root), Path(env_root)
    native = str(root/"target/measurement/examples/component_benchmark_native")
    diffsol = str(root/"tools/benchmarks/diffsol/a07/target/release/component_benchmark")
    python = str(env_root/"python/bin/python")
    julia = str(env_root/"julia-1.12.7+0.x64.linux.gnu/bin/julia")
    julia_project = str(root/"tools/benchmarks/component_julia")
    julia_depot = str(env_root/"vigilode-component-julia-depot-20261018")
    module = lambda crate, name: crate+":src::"+name
    I = lambda name: module("rodas5p-integrators", name)
    cases = expanded_cases()
    selected_fixed = [c["case_id"] for c in cases if c["case_id"].split("@")[0] in (
        "ode-scalar-decay", "ode-logistic", "ode-pr-linear-1e2", "ode-pr-linear-1e4", "ode-diagonal-stiff", "ode-rotation-1", "ode-damping-critical", "ode-jordan") and "@" not in c["case_id"]]
    # Allocation census runs independently of timed processes on representative
    # complete solves. Its scope is total solve, not a hot-loop assertion.
    solvers = {}
    def add(sid, command, components=(), models=None, max_dimension=512, forms=("ODE",), **kwargs):
        spec = dict(command=command, cwd=str(root), env=dict(THREADS), equation_forms=list(forms), components=list(components), max_dimension=max_dimension,
                    derivative_lane="adapter_declared", timing_scope="warmed_solver_setup_integration_output_certificate", **kwargs)
        if models is not None:
            spec["models"] = models
        solvers[sid] = spec
    for sid, component in (
        ("native-rodas5p-fast", "rodas5p_fast"), ("native-rodas5p-small", "rodas5p_fast_small"),
        ("native-rodas5p-banded", "rodas5p_fast"),
        ("native-rodas5p-matrix-free", "dense_output_v2"), ("native-rodas5p-mf-fast", "rodas5p_matrix_free_fast"),
        ("native-rodas5p-mf-gmres-into", "rodas5p_matrix_free_fast"), ("native-rodas5p-mf-ls-workspace", "rodas5p_matrix_free_fast"),
        ("native-pexprb54s4", "exponential")):
        add(sid, [native], [I(component)], max_dimension=8 if sid.endswith("small") else (4096 if "mf-" in sid or "matrix-free" in sid else 512), measurement_modes=["timing"])
    for sid, component, variant in (
        ("native-bdf1","bdf","BDF1"),("native-bdf2","bdf","BDF2"),
        ("native-radau1","radau","RadauIIA1"),("native-radau3","radau","RadauIIA3"),
        ("native-exprb2","exponential","EXPRB2"),("native-exprb43","exponential","EXPRB43")):
        add(sid,[native],[I(component), I(component)+"#"+variant],case_ids=selected_fixed,
            fixed_h_divisors=[1000,2000,4000,8000],measurement_modes=["timing"])
    add("native-sabr",[native],[I("sabr")],case_ids=["ode-sabr-scalar-fixture"],measurement_modes=["timing"])
    for name in ("bdf","tr_bdf2","esdirk34","tsit45"):
        add("diffsol-"+name,[diffsol],measurement_modes=["timing"],bind_files=[str(root/"tools/benchmarks/diffsol/a07/Cargo.lock")])
    for name in ("rk45","dop853","radau","bdf","lsoda"):
        add("scipy-"+name,[python,str(root/"tools/component_benchmark/scipy_adapter.py")],max_dimension=4096 if name in ("rk45","dop853") else 512,forms=("ODE","MASS_MATRIX"))
    for name in ("adams","bdf"):
        add("vode-"+name,[python,str(root/"tools/component_benchmark/scipy_adapter.py")],forms=("ODE","MASS_MATRIX"))
    for name,library in (("radau5","radau"),("rodas","rodas")):
        add("hairer-"+name,[python,str(root/"tools/component_benchmark/hairer_adapter.py")],
            bind_files=[str(env_root/"vigilode-component-hairer-20261018"/("libhairer_"+library+".so"))])
        solvers["hairer-"+name]['env']['VIGILODE_HAIRER_LIBDIR']=str(env_root/"vigilode-component-hairer-20261018")
    for name in ("tsit5","vern9","rodas5p","radauiia5","fbdf","dfbdf","ida","cvode-bdf"):
        add("julia-"+name,[julia,"--startup-file=no","--project="+julia_project,str(root/"tools/component_benchmark/julia_adapter.jl")],
            forms=("ODE","MASS_MATRIX","DAE"), max_dimension=4096 if name in ("tsit5","vern9") else 512,
            bind_files=[str(root/"tools/benchmarks/component_julia/Manifest.toml")])
        solvers["julia-"+name]["env"]["JULIA_DEPOT_PATH"] = julia_depot
    for name in ('dense','matrix-free'):
        add('native-rodas5p-routed-'+name,[native],[I('routing')],max_dimension=512)
    # Configuration arms share frozen CaseSpecs, but have separate action/route
    # identities. They are not aliases for algorithms that were never executed.
    representative=[c['case_id'] for c in cases if '@' not in c['case_id'] and c['case_id'] in (
        'ode-scalar-decay','ode-logistic','ode-kepler','ode-robertson','ode-pr101',
        'ode-damping-underdamped','ode-diagonal-stiff','ode-periodic-heat','ode-brusselator-1d-50','ode-gray-scott-2d-4')]
    def configured(sid,base,options,components,threads=1):
        command=[python,str(root/'tools/component_benchmark/configured_adapter.py'),'--adapter',native,
                 '--solver-id',base,'--solver-config-json',json.dumps(options,sort_keys=True)]
        add(sid,command,components,case_ids=representative,measurement_modes=['timing'],
            executable_artifact=native,threads=threads,alias_of=base,configuration=options)
        solvers[sid]['env'].update({k:str(threads) for k in THREADS})
    for name,variant in (('pi','PI'),('predictive','Predictive'),('predictive-capped','PredictiveCapped'),('predictive-capped2','PredictiveCapped2')):
        configured('native-rodas5p-fast@'+name,'native-rodas5p-fast',{'controller':name},[I('adaptive')+'#'+variant])
    for policy in ('column-extents','column-extents-above64'):
        configured('native-rodas5p-fast@'+policy,'native-rodas5p-fast',{'lu_policy':policy},[I('rodas5p_fast')])
    for threads in (2,4,8):
        configured('native-pexprb54s4@threads'+str(threads),'native-pexprb54s4',{'threads':threads},[I('exponential')],threads)
    precise = {
        'native-rodas5p-fast': ['rodas5p_fast#FastDenseDriver'],
        'native-rodas5p-small': ['rodas5p_fast_small#SmallFixedArray'],
        'native-rodas5p-banded': ['rodas5p_fast#FastBandedDriver'],
        'native-rodas5p-mf-fast': ['rodas5p_matrix_free_fast#MatrixFreeDefault'],
        'native-rodas5p-mf-gmres-into': ['rodas5p_matrix_free_fast#MatrixFreeGMRESInto'],
        'native-rodas5p-mf-ls-workspace': ['rodas5p_matrix_free_fast#MatrixFreeLSWorkspace'],
        'native-pexprb54s4': ['exponential#PEXPRB54S4','adaptive_exponential'],
        'native-sabr': ['sabr#SabrStep'],
        'native-rodas5p-fast@column-extents': ['rodas5p_fast#DenseColumnExtentsLU'],
        'native-rodas5p-fast@column-extents-above64': ['rodas5p_fast#DenseColumnExtentsAbove64'],
    }
    for name in ('dense','matrix-free'):
        precise['native-rodas5p-routed-'+name]=['routing#RoutedDenseOrMatrixFree']
    for count in (2,4,8):
        precise['native-pexprb54s4@threads'+str(count)]=['parallel#ParallelExecution','exponential#PEXPRB54S4']
    for sid, links in precise.items():
        solvers[sid]['components']=list(dict.fromkeys(solvers[sid]['components']+[I(link) for link in links]))
    # Detect stale component names instead of silently leaving an unlinked arm.
    known = {r["component_id"] for r in inventory(root)}
    errors = [(sid,c) for sid,s in solvers.items() for c in s["components"] if c not in known]
    if errors:
        raise ValueError("component mapping must be explicit: "+repr(errors))
    return {"campaign":"component_benchmark_20261018", "baseline":BASE, "random_seed":20261018,
            "solvers":solvers, "cases":cases, "contrasts":[], "timing_authority":"diagnostic_only"}


def reference_job(request):
    from component_benchmark.references import build_reference
    start=time.monotonic()
    result=build_reference(request["case"])
    for key, value in (("times",[]),("states",[]),("counters",None),("allocations",None)):
        result.setdefault(key,value)
    result["timing"]={"elapsed_seconds":time.monotonic()-start,"timing_authority":"diagnostic_only"}
    result["measurement_mode"]="reference"
    result["input_sha256"]=digest(request["case"])
    result["physical_input_sha256"]=physical_input_digest(request["case"])
    result["reference_source_sha256"]=digest(source_binding(Path(__file__).resolve().parents[2]))
    return result


def collect_references(run, target, cases):
    refs={}
    receipts={}
    originals={}
    root=Path(__file__).resolve().parents[2]
    source_hash=digest(source_binding(root))
    for path in sorted(Path(run).glob("jobs/*/receipt.json")):
        row=strict_load(path.read_text())
        cid=row["job"]["case_id"]
        if not validate_receipt(path.parent,row,row["job"],source_hash):
            raise ValueError("invalid reference receipt "+str(path))
        if row.get("result") is not None:
            refs[cid]=row["result"]
            original=row['job']['request']['case']
            originals[cid]=original
            receipts[cid] = {"receipt_path":str(path.resolve().relative_to(root)),"receipt_sha256":file_digest(path),
                            "source_binding_sha256":source_hash,"request_sha256":row['request_sha256'],
                            "physical_input_sha256":physical_input_digest(original),"input_sha256":digest(original)}
    for case in cases:
        cid=case["case_id"]
        base=case.get("base_case_id",cid)
        if cid not in refs and base in refs:
            if physical_input_digest(case)!=physical_input_digest(originals[base]):
                raise ValueError("reference reuse physical input mismatch: "+cid)
            refs[cid]=copy.deepcopy(refs[base])
            receipts[cid]=copy.deepcopy(receipts[base])
            refs[cid]["reused_from"]=base
            refs[cid]["reuse_contract"]="identical equation/IC/BC/mesh; error gate reapplied to this tolerance"
    atomic_json(target,refs)
    atomic_json(str(target)+".provenance.json",receipts)
    return {"references":len(refs),"sha256":file_digest(target)}


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("command",choices=("freeze","reference","collect-references"))
    parser.add_argument("--root",type=Path,default=Path(__file__).resolve().parents[2])
    parser.add_argument("--env-root",type=Path)
    parser.add_argument("--output",type=Path)
    parser.add_argument("--request",type=Path)
    parser.add_argument("--run",type=Path)
    parser.add_argument("--measurement-mode",default="reference")
    args=parser.parse_args()
    if args.command=="reference":
        result=reference_job(strict_load(args.request.read_text()))
        print(json.dumps(result,allow_nan=False,separators=(",",":")))
        return
    if args.command=="collect-references":
        print(json.dumps(collect_references(args.run,args.output,expanded_cases())))
        return
    if args.env_root is None or args.output is None:
        parser.error("freeze requires --env-root and --output")
    args.output.mkdir(parents=True,exist_ok=False)
    config=configurations(args.root,args.env_root)
    atomic_json(args.output/"screening_config.json",config)
    allocation_config=copy.deepcopy(config)
    allocation_config['cases']=[c for c in config['cases'] if c['case_id'] in (
        'ode-scalar-decay','ode-robertson','ode-pr101','ode-periodic-heat','ode-sabr-scalar-fixture')]
    allocation_config['solvers']={sid:spec for sid,spec in allocation_config['solvers'].items()
                                   if sid.startswith(('native-','diffsol-'))}
    for spec in allocation_config['solvers'].values():
        spec['measurement_modes']=['allocation']
    atomic_json(args.output/'allocation_config.json',allocation_config)
    reference_cases=build_cases()+[c for c in expanded_cases() if c["case_id"].endswith("@rtol1e-10")]
    refspec={"command":[str(args.env_root/"python/bin/python"),str(args.root/"tools/component_benchmark/campaign.py"),"reference"],
             "cwd":str(args.root),"env":THREADS,"equation_forms":["ODE","MASS_MATRIX","DAE"],"components":[],"measurement_modes":["reference"]}
    atomic_json(args.output/"reference_config.json",{"cases":reference_cases,"solvers":{"independent-reference":refspec},"contrasts":[]})
    write_reports(inventory(args.root),args.output/"inventory")
    atomic_json(args.output/"manifest.json",{"baseline":BASE,"cases_sha256":digest(config["cases"]),
        "screening_config_sha256":file_digest(args.output/"screening_config.json"),
        "source_binding_sha256":digest(source_binding(args.root)),"runtime_identity":"NOT_INDEPENDENTLY_OBSERVED",
        "scope":"pre-execution frozen inputs, no scientific decision"})
    print(json.dumps({"status":"FROZEN_INPUTS","cases":len(config["cases"]),"solvers":len(config["solvers"]),"output":str(args.output)}))


if __name__=="__main__":
    main()
