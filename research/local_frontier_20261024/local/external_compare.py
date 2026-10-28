#!/usr/bin/env python3
"""Matched external serial baselines. VODE and optional preinstalled Julia only.

Diffsol capability is reported NOT_RUN until a current-contract adapter is
implemented; older differently parameterized adapters are never substituted.
"""
import argparse
import importlib.metadata
import json
import platform
from pathlib import Path
import shutil
import subprocess
import sys
import time
from campaign import REPO,HERE,canonical,digest,eligible,json_new,run_process,source_identity,topology,utc,result_contract_errors,verify_receipt
from external_models import SUPPORTED,verify

def main():
    p=argparse.ArgumentParser()
    p.add_argument('--binary',type=Path,required=True)
    p.add_argument('--build-receipt',type=Path)
    p.add_argument('--expected-commit');p.add_argument('--expected-tree')
    p.add_argument('--allow-dirty-smoke',action='store_true')
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--cases',nargs='+',type=int,default=[1,10,15,16])
    p.add_argument('--size',choices=['smoke','standard'],default='smoke')
    p.add_argument('--repeats',type=int,default=3)
    p.add_argument('--rtol',type=float,default=1e-6);p.add_argument('--atol',type=float,default=1e-9)
    p.add_argument('--timeout',type=float,default=60);p.add_argument('--memory-gib',type=float,default=8)
    p.add_argument('--budget',type=float,default=900)
    p.add_argument('--julia-project',type=Path)
    a=p.parse_args()
    if a.repeats<1 or a.timeout<=0 or a.budget<=0 or a.memory_gib<=0 or a.rtol<=0 or a.atol<=0:raise ValueError('positive limits required')
    if len(set(a.cases))!=len(a.cases) or any(c not in SUPPORTED for c in a.cases):raise ValueError('only unique supported matched models permitted')
    if a.allow_dirty_smoke and a.size!='smoke':raise ValueError('dirty is smoke only')
    if not a.allow_dirty_smoke and (not a.expected_commit or not a.expected_tree or not a.build_receipt):raise ValueError('published run requires source commit/tree and build receipt')
    identity=source_identity(REPO,a.expected_commit,a.expected_tree,a.allow_dirty_smoke)
    binary_hash=digest(a.binary)
    if a.build_receipt:
        b=json.loads(a.build_receipt.read_text())
        if b['source']!=identity or b['binary_sha256']!=binary_hash or b['exit_code']!=0:raise ValueError('build source/binary mismatch')
    a.output.mkdir(parents=True,exist_ok=True)
    configuration={'source':identity,'binary_sha256':binary_hash,'cases':a.cases,'size':a.size,'repeats':a.repeats,'rtol':a.rtol,'atol':a.atol,'timeout':a.timeout,'memory_gib':a.memory_gib,'budget':a.budget,'julia_project':str(a.julia_project) if a.julia_project else None,'adapters':{n:digest(HERE/n) for n in ['external_models.py','vode_adapter.py','julia_adapter.jl']}}
    versions={}
    for package in ["numpy","scipy"]:
        try: versions[package]=importlib.metadata.version(package)
        except importlib.metadata.PackageNotFoundError: versions[package]=None
    configuration["packages"]=versions
    configuration["host_topology"]=topology()
    configuration["hostname"]=platform.node()
    configuration["platform"]=platform.platform()
    if a.julia_project:
        configuration["julia_project_hashes"]={f:digest(a.julia_project/f) for f in ["Project.toml","Manifest.toml"]}
        julia_executable=shutil.which("julia")
        configuration["julia_version"] = subprocess.check_output([julia_executable,"--version"],text=True).strip() if julia_executable else None
    marker=a.output/'IDENTITY.json'
    if marker.exists():
        if json.loads(marker.read_text())!=configuration:raise ValueError('output identity differs')
    else:json_new(marker,configuration)
    not_run=[{'method':'diffsol','status':'NOT_RUN','reason':'No matched current-contract diffsol adapter; old fixture adapter cannot supply this comparison.'}]
    methods=['dopri5','rodas5p-mf','switching','vode-adams','vode-bdf']
    try:import scipy
    except ImportError:
        methods=methods[:-2];not_run.append({'method':'vode','status':'NOT_RUN','reason':'SciPy unavailable'})
    julia=shutil.which('julia')
    if julia and a.julia_project:
        for f in ['Project.toml','Manifest.toml']:
            if not (a.julia_project/f).is_file():raise ValueError('Julia requires existing Project and Manifest')
        methods+=['julia-tsit5','julia-rodas5p']
    else:not_run.append({'method':'julia','status':'NOT_RUN','reason':'Existing Julia executable and explicit pinned project required'})
    cpus=topology()['physical_cpu_selection'][:1]
    env={k:'1' for k in ['RAYON_NUM_THREADS','OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','MKL_NUM_THREADS','BLIS_NUM_THREADS','JULIA_NUM_THREADS']}
    prior=sum(json.loads(p.read_text()).get('elapsed_process_seconds',0) for p in a.output.glob('runs/*/RECEIPT.json'))
    prior += sum(json.loads(p.read_text()).get('timeout_seconds',0) for p in a.output.glob('runs/*/START.json') if not (p.parent/'RECEIPT.json').exists())
    began=time.monotonic();rows=[];pending=[]
    for case in a.cases:
        path=a.output/f'model_{case}.json'
        d=subprocess.run([str(a.binary.resolve()),'--describe','--case',str(case),'--size',a.size],capture_output=True,text=True,check=True,timeout=30)
        contract=json.loads(d.stdout);verify(contract)
        if path.exists():
            if json.loads(path.read_text())!=contract:raise ValueError('model descriptor changed')
        else:json_new(path,contract)
        for repeat in range(a.repeats):
            ordered=methods[repeat%len(methods):]+methods[:repeat%len(methods)]
            for method in ordered:
                folder=a.output/'runs'/f'{case}_{repeat}_{method}'
                receipt_path=folder/'RECEIPT.json'
                task={'case':case,'method':method,'parallel_layer':'serial','threads':1,'repeat':repeat,'instrumented':False}
                if receipt_path.exists():rows.append(verify_receipt(receipt_path,identity,binary_hash,task));continue
                if folder.exists():
                    pending.append({'case':case,'method':method,'reason':'interrupted directory preserved; use a new campaign output for retry'});continue
                remaining=a.budget-prior-(time.monotonic()-began)
                if remaining<=0:pending.append({'case':case,'method':method,'reason':'budget exhausted'});continue
                if method.startswith('vode-'):
                    cmd=[sys.executable,str(HERE/'vode_adapter.py'),'--contract',str(path.resolve()),'--method',method,'--rtol',str(a.rtol),'--atol',str(a.atol)]
                elif method.startswith('julia-'):
                    cmd=[julia,'--startup-file=no','--threads=1','--project='+str(a.julia_project.resolve()),str(HERE/'julia_adapter.jl'),str(path.resolve()),method,str(a.rtol),str(a.atol),'200000']
                else:
                    cmd=[str(a.binary.resolve()),'--case',str(case),'--size',a.size,'--method',method,'--parallel-layer','serial','--threads','1','--rtol',str(a.rtol),'--atol',str(a.atol),'--max-steps','200000','--emit-state']
                r=run_process(cmd,folder,min(a.timeout,remaining),a.memory_gib,cpus,env)
                r.update(source=identity,task={'case':case,'method':method,'parallel_layer':'serial','threads':1,'repeat':repeat,'instrumented':False},contract_sha256=digest(path),binary_sha256=binary_hash)
                r['result_contract_errors']=result_contract_errors(r.get('result'),task)
                if r['result_contract_errors'] and r['execution_status']=='COMPLETED':r['execution_status']='RESULT_CONTRACT_MISMATCH'
                r['eligible_for_timing_comparison']=eligible(r)
                json_new(receipt_path,r);rows.append(r)
                print(canonical({'case':case,'method':method,'status':r['execution_status'],'scientific':(r.get('result')or{}).get('scientific_status')}),flush=True)
    summary={'created_utc':utc(),'configuration':configuration,'not_run':not_run,'pending':pending,'receipts':[{'task':r['task'],'execution_status':r['execution_status'],'result':r.get('result'),'eligible':r['eligible_for_timing_comparison']} for r in rows],
             'interpretation':'Only same IVP and maximum absolute endpoint error threshold; VODE includes Python callbacks, Julia explicitly excludes JIT warmup. No native-language speed parity or global phase certificate claimed.'}
    json_new(a.output/f'SUMMARY_{time.time_ns()}.json',summary)
if __name__=='__main__':main()
