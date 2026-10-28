#!/usr/bin/env python3
"""Replay only changed behavior; output receipts belong outside the checkout."""
import argparse, hashlib, json, os, pathlib, platform, shutil, subprocess, sys, time
ROOT=pathlib.Path(__file__).resolve().parents[2]

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--out',type=pathlib.Path,required=True)
    ap.add_argument('--expected-commit',required=True)
    args=ap.parse_args(); out=args.out.resolve()
    if out==ROOT or ROOT in out.parents:
        ap.error('--out must be outside the checkout')
    actual=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    if actual!=args.expected_commit:
        ap.error('HEAD differs from expected commit; use a fresh detached worktree')
    dirty=subprocess.check_output(['git','status','--porcelain'],cwd=ROOT,text=True)
    if dirty:
        ap.error('checkout is not clean; preserve it and use a new worktree')
    out.mkdir(parents=True,exist_ok=False)
    env=os.environ.copy(); env.setdefault('CARGO_BUILD_JOBS','2')
    env.setdefault('CARGO_PROFILE_DEV_DEBUG','0');env.setdefault('CARGO_PROFILE_TEST_DEBUG','0')
    env['PYTHONDONTWRITEBYTECODE']='1'
    commands=[
        ('low_rank_core',['cargo','test','--locked','--offline','-p','rodas5p-core','--test','low_rank_bordered_contracts','--test','low_rank_bordered_allocation']),
        ('low_rank_gmres',['cargo','test','--locked','--offline','-p','rodas5p-krylov','--test','low_rank_bordered_preconditioner']),
        ('structured_client',['cargo','test','--locked','--offline','-p','rodas5p-integrators','--test','lrs_block_solver_failure_contracts','--test','lrs_block_solver_contracts']),
        ('reference_metric',[sys.executable,'research/production_review_20261022/benchmark/test_metric_reference_domain.py']),
        ('affine_norm',[sys.executable,'research/lifted_error_norm_20261022/code/verify_norm.py','--output',str(out/'AFFINE_NORM.json')]),
    ]
    summary={'schema_version':1,'commit':actual,'platform':platform.platform(),'hostname':platform.node(),'commands':[],'status':'RUNNING','new_ncp_execution':False}
    tracked=subprocess.check_output(['git','ls-files','-z','crates','Cargo.toml','Cargo.lock','external_audits/vigilode_bassrei_ncp_c1_20261022/harness/scenarios.py'],cwd=ROOT).decode().split('\0')
    summary['source_sha256']={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in tracked if p}
    def save(): (out/'SUMMARY.json').write_text(json.dumps(summary,indent=2)+'\n')
    save()
    for name,cmd in commands:
        start=time.monotonic()
        try:
            with (out/(name+'.log')).open('w') as log:
                result=subprocess.run(cmd,cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=1200)
            code=result.returncode
        except (OSError,subprocess.TimeoutExpired) as exc:
            code=None;(out/(name+'.error.txt')).write_text(str(exc)+'\n')
        summary['commands'].append({'name':name,'command':cmd,'exit':code,'seconds':time.monotonic()-start})
        save()
        if code!=0:
            summary['status']='STOPPED';save();return 1
    after={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in tracked if p}
    summary['source_unchanged']=after==summary['source_sha256']
    summary['status']='PASS' if summary['source_unchanged'] else 'SOURCE_CHANGED';save()
    return 0 if summary['source_unchanged'] else 1
if __name__=='__main__': raise SystemExit(main())
