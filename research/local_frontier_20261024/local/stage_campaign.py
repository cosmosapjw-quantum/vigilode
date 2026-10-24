#!/usr/bin/env python3
"""Separate causal-stage experiment; never joins the data/ensemble timing table."""
import argparse
import json
import platform
from pathlib import Path
import time
from campaign import REPO,canonical,checked_json_lines,digest,json_new,run_process,source_identity,topology,verify_receipt

def main():
    p=argparse.ArgumentParser()
    p.add_argument('--binary',type=Path,required=True);p.add_argument('--build-receipt',type=Path,required=True)
    p.add_argument('--expected-commit',required=True);p.add_argument('--expected-tree',required=True)
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--threads',type=int,nargs='+',default=[1,2,4,8,12]);p.add_argument('--repeats',type=int,default=3)
    p.add_argument('--timeout',type=float,default=120);p.add_argument('--budget',type=float,default=1800);p.add_argument('--memory-gib',type=float,default=16)
    a=p.parse_args()
    if any(t not in [1,2,4,8,12] for t in a.threads) or len(set(a.threads))!=len(a.threads) or not 1<=a.repeats<=9 or min(a.timeout,a.budget,a.memory_gib)<=0:raise ValueError('invalid limits')
    identity=source_identity(REPO,a.expected_commit,a.expected_tree)
    build=json.loads(a.build_receipt.read_text())
    if build['source']!=identity or build['binary_sha256']!=digest(a.binary) or build['exit_code']!=0:raise ValueError('stage build/source mismatch')
    cpus=topology()['physical_cpu_selection'];a.output.mkdir(parents=True,exist_ok=True)
    config={'source':identity,'binary_sha256':digest(a.binary),'threads':a.threads,'repeats':a.repeats,'budget':a.budget,'timeout':a.timeout,'memory_gib':a.memory_gib,'host_topology':topology(),'hostname':platform.node(),'platform':platform.platform(),'cases':['few-mode','nonlinear-changing-rank','nonlinear-step-regime','meanfield-manufactured']}
    marker=a.output/'IDENTITY.json'
    if marker.exists():
        if json.loads(marker.read_text())!=config:raise ValueError('campaign identity changed')
    else:json_new(marker,config)
    prior=sum(json.loads(p.read_text()).get('elapsed_process_seconds',0) for p in a.output.glob('runs/*/RECEIPT.json'))
    prior += sum(json.loads(p.read_text()).get('timeout_seconds',0) for p in a.output.glob('runs/*/START.json') if not (p.parent/'RECEIPT.json').exists())
    start=time.monotonic();rows=[];pending=[]
    for repeat in range(a.repeats):
        for case in config['cases']:
            for threads in a.threads:
                task={'case':case,'parallel_layer':'stage','threads':threads,'process_repeat':repeat}
                if threads>len(cpus):pending.append({**task,'reason':'insufficient physical cores'});continue
                folder=a.output/'runs'/f'{case}_{threads}_{repeat}'
                if (folder/'RECEIPT.json').exists():rows.append(verify_receipt(folder/'RECEIPT.json',identity,config['binary_sha256'],task));continue
                if folder.exists():pending.append({**task,'reason':'interrupted attempt preserved; retry in a new output'});continue
                remaining=a.budget-prior-(time.monotonic()-start)
                if remaining<=0:pending.append({**task,'reason':'budget exhausted'});continue
                command=[str(a.binary.resolve()),'--case',case,'--root-variants','--reference-guard','--threads',str(threads),'--repeats','1']
                environment={'RAYON_NUM_THREADS':str(threads),'OMP_NUM_THREADS':'1','OPENBLAS_NUM_THREADS':'1','MKL_NUM_THREADS':'1','VIGILODE_SOURCE_COMMIT':identity['commit']}
                receipt=run_process(command,folder,min(a.timeout,remaining),a.memory_gib,cpus[:threads],environment,expect_result=False)
                parsed,errors=checked_json_lines(folder/'stdout.jsonl')
                receipt.update(task=task,source=identity,binary_sha256=config["binary_sha256"],header=next((r for r in parsed if r.get('kind')=='header'),None),samples=[r for r in parsed if r.get('kind')=='sample'])
                if not receipt['samples'] or not receipt['header']:
                    if receipt['execution_status']=='COMPLETED':receipt['execution_status']='MISSING_STAGE_SAMPLES'
                json_new(folder/'RECEIPT.json',receipt);rows.append(receipt)
                print(canonical({'task':task,'status':receipt['execution_status']}),flush=True)
    json_new(a.output/f'SUMMARY_{time.time_ns()}.json',{'configuration':config,'receipts':rows,'pending':pending,'interpretation':'Independent process repeats; each process warms and cyclically runs all modes. Stage parallelism and fixed 17 physical landings differ from endpoint-only data/ensemble suite. Never combine their latency ratios. A pool is created once per process; report startup separately. No synthetic RHS burn in meanfield-manufactured.'})
if __name__=='__main__':main()
