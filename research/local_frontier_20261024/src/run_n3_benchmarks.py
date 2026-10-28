#!/usr/bin/env python3
"""Small, serialized native discrimination campaign; preserve every outcome."""
import fcntl,hashlib,json,os,subprocess,sys
from pathlib import Path

ROOT=Path(__file__).resolve().parents[3]
PKG=ROOT/'research/local_frontier_20261024'
BIN=Path(os.environ['CARGO_TARGET_DIR'])/'measurement/examples/reconsideration_bench'
LOCK=Path(os.environ.get('VIGILODE_CARGO_LOCK',str(ROOT.parent/'runtime1024/cargo-frontier.lock')))

def run(name,args):
    with LOCK.open('a') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        binary_sha=hashlib.sha256(BIN.read_bytes()).hexdigest()
        command=[sys.executable,str(PKG/'src/run_receipt.py'),'--name',name,'--timeout','240','--',
                 str(BIN),*args]
        r=subprocess.run(command,cwd=ROOT)
    path=PKG/'evidence/runs'/f'{name}.receipt.json'
    if path.exists():
        receipt=json.loads(path.read_text())
        receipt['measured_binary']={'path':str(BIN),'sha256':binary_sha}
        receipt['measurement_lock_scope']='cargo-frontier lock prevents simultaneous project builds/benchmarks that honor this lock'
        path.write_text(json.dumps(receipt,indent=2)+'\n')
    return r.returncode

def main():
    cases=['few-mode','expensive-few-mode','broad','riccati','nonnormal','oscillatory','changing-rate',
           'few-mode-basis1','nonlinear-changing-rank','nonlinear-step-regime']
    errors=[]
    for case in cases:
        args=['--case','few-mode' if case=='few-mode-basis1' else case,'--reference-guard','--repeats','3']
        if case=='few-mode-basis1':args+=['--basis-limit','1']
        code=run('n3-final-'+case,args)
        if code:errors.append({'case':case,'exit_code':code})
    for threads in [1,2]:
        code=run(f'n5-final-meanfield-{threads}',['--case','meanfield-manufactured','--root-variants',
            '--threads',str(threads),'--repeats','3'])
        if code:errors.append({'case':f'meanfield-{threads}','exit_code':code})
    print(json.dumps({'process_failures':errors}))
    return bool(errors)

if __name__=='__main__':raise SystemExit(main())
