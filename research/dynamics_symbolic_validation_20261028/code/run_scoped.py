#!/usr/bin/env python3
"""Run the published bounded campaign; preserve each independent outcome."""
import argparse
import datetime as dt
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
NODE = Path(__file__).resolve().parents[1]

def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-sha', required=True)
    parser.add_argument('--remote-ref', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--cargo', default='cargo')
    parser.add_argument('--lock', default='/tmp/vigilode-cargo-20261027.lock')
    parser.add_argument('--only', action='append', default=[])
    args = parser.parse_args()
    if git('rev-parse','HEAD') != args.source_sha or git('rev-parse',args.remote_ref) != args.source_sha:
        parser.error('actual HEAD and fetched remote ref must equal the supplied published source')
    subprocess.run(['git','diff','--exit-code','HEAD','--','crates','Cargo.lock',str(NODE.relative_to(ROOT))],cwd=ROOT,check=True)
    output = args.output.resolve()
    output.mkdir(parents=True,exist_ok=False)
    environment = os.environ.copy()
    environment.update(CARGO_BUILD_JOBS='2',RAYON_NUM_THREADS='1',OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1')
    cargo = args.cargo
    common = ['--profile','measurement','--locked','--offline']
    commands = [
        ('coupled-lib',900,[cargo,'test',*common,'-p','vigilode-coupled','--lib'],{}),
        ('facade-lib',900,[cargo,'test',*common,'-p','vigilode','--lib'],{}),
        ('givens-exact',300,[cargo,'test',*common,'-p','rodas5p-krylov','--test','givens_production_differential_contracts','--','--nocapture'],{'Q_GIVENS_ORACLE_DIR':str(output/'givens-bits'),'Q_GIVENS_ORACLE_SCRIPT':str(ROOT/'tools/q_givens_exact_residual_oracle.py')}),
        ('trace-support',180,[cargo,'run',*common,'-p','vigilode-coupled','--example','trace_support_campaign'],{}),
        ('switch-timing',180,[cargo,'test',*common,'-p','vigilode-coupled','--lib','switching::tests::benchmark_snapshot_transfer_against_legacy','--','--ignored','--exact','--nocapture'],{'VIGILODE_SWITCH_BENCH_OUT':str(output/'switch-timing.json')}),
        ('coupled-release',600,[cargo,'test','--release','--locked','--offline','-p','vigilode-coupled','--lib'],{}),
        ('all-targets-scoped',300,[cargo,'check',*common,'-p','vigilode-coupled','-p','vigilode','--all-targets'],{}),
        ('docs-scoped',300,[cargo,'doc','--locked','--offline','-p','vigilode-coupled','--no-deps'],{'RUSTDOCFLAGS':'-D warnings'}),
    ]
    known={row[0] for row in commands}
    if set(args.only)-known:
        parser.error('unknown --only label')
    record=ROOT/'research/production_solver_closure_20261027/record_command.py'
    receipt={'schema':'vigilode.dynamics-scoped-run.v1','source_sha':args.source_sha,'source_tree':git('rev-parse','HEAD^{tree}'),'remote_ref':args.remote_ref,'started_utc':dt.datetime.now(dt.timezone.utc).isoformat(),'commands':[]}
    with open(args.lock,'w') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        for label,cap,argv,extra in commands:
            if args.only and label not in args.only: continue
            if label=='givens-exact': (output/'givens-bits').mkdir()
            env=environment|extra
            wrapper=[sys.executable,str(record),'--output',str(output/label),'--timeout',str(cap),'--input',str(NODE/'PREREGISTRATION.md'),'--input',str(Path(__file__)),'--',*argv]
            proc=subprocess.run(wrapper,cwd=ROOT,env=env)
            row={'label':label,'argv':argv,'wrapper_exit':proc.returncode,'receipt':str((output/label/'RECEIPT.json').relative_to(output))}
            receipt['commands'].append(row)
            (output/'RUN.json').write_text(json.dumps(receipt,indent=2)+'\n')
            print(json.dumps(row),flush=True)
            if git('rev-parse','HEAD')!=args.source_sha: raise RuntimeError('source changed during run')
    receipt['finished_utc']=dt.datetime.now(dt.timezone.utc).isoformat()
    receipt['all_commands_exit_zero']=all(r['wrapper_exit']==0 for r in receipt['commands'])
    receipt['scientific_acceptance']='See per-command tests, exact oracle and numeric results; exit status alone is insufficient.'
    (output/'RUN.json').write_text(json.dumps(receipt,indent=2)+'\n')
    return 0 if receipt['all_commands_exit_zero'] else 1

if __name__=='__main__':
    raise SystemExit(main())
