#!/usr/bin/env python3
"""Serial, bounded verification of this follow-up; no NCP or full-suite claims."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent

def snapshot():
    paths = {ROOT/'Cargo.toml', ROOT/'Cargo.lock'}
    for package in ('rodas5p-core', 'rodas5p-krylov', 'rodas5p-integrators'):
        base = ROOT/'crates'/package
        paths.add(base/'Cargo.toml')
        for directory in ('src','tests','examples'):
            paths.update((base/directory).rglob('*.rs'))
    paths.update((ROOT/'fixtures').glob('*.json'))
    paths.update(HERE.rglob('*.py'))
    return {str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(paths) if p.is_file()}

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args=parser.parse_args()
    out=args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    before=snapshot()
    (out/'SOURCE_BEFORE.json').write_text(json.dumps(before,indent=2)+'\n')
    tasks=[
      ('core', ['cargo','test','-p','rodas5p-core','--test','bordered_schur_contracts','--test','bordered_allocation_contract','--locked','--offline','-j','1']),
      ('krylov', ['cargo','test','-p','rodas5p-krylov','--test','bordered_preconditioner_contracts','--locked','--offline','-j','1']),
      ('adapter', ['cargo','test','-p','rodas5p-integrators','--example','component_benchmark_native','--locked','--offline','-j','1']),
      ('remap_build', ['rustc','--edition=2024','--test','crates/rodas5p-integrators/src/conservative_remap.rs','-o',str(out/'remap_tests')]),
      ('remap', [str(out/'remap_tests')]),
      ('bordered_probe', ['cargo','run','-p','rodas5p-core','--example','bordered_probe','--locked','--offline','-j','1']),
      ('physics', [sys.executable,str(HERE/'BASS_REI_FVM_REFERENCE.py'),str(out/'PHYSICS.json')]),
      ('rotation_contract', [sys.executable,'-m','unittest','discover','-s',str(HERE/'accuracy'),'-p','test_*.py','-v']),
      ('rotation_endpoint', [sys.executable,str(HERE/'accuracy/rotation_diagnostic.py'),'--bundle',str(ROOT/'external_audits/vigilode_ncp_intensive_120m_v1_20261021'),'--out',str(out/'ROTATION.json')]),
    ]
    records=[]
    env=dict(os.environ, CARGO_BUILD_JOBS='1', PYTHONDONTWRITEBYTECODE='1')
    for name,argv in tasks:
        started=time.monotonic()
        with (out/(name+'.stdout.log')).open('w') as stdout, (out/(name+'.stderr.log')).open('w') as stderr:
            try:
                result=subprocess.run(argv,cwd=ROOT,env=env,stdout=stdout,stderr=stderr,timeout=900)
                code=result.returncode; reason=None
            except subprocess.TimeoutExpired:
                code=None; reason='timeout'
            except OSError as error:
                code=None; reason=str(error)
        records.append(dict(id=name,command=argv,exit_code=code,reason=reason,
                            elapsed_seconds=time.monotonic()-started))
        (out/'EXECUTION.json').write_text(json.dumps(records,indent=2)+'\n')
        print(name,code,flush=True)
        if code != 0:
            break
    after=snapshot()
    (out/'SOURCE_AFTER.json').write_text(json.dumps(after,indent=2)+'\n')
    ok=len(records)==len(tasks) and all(r['exit_code']==0 for r in records) and before==after
    summary=dict(status='PASS_SCOPED' if ok else 'FAIL',commands_planned=len(tasks),
                 commands_executed=len(records),source_unchanged=before==after,
                 source_files=len(before),broad_production='HOLD',ncp_execution='NOT_RUN',
                 scope='Changed-dependency contracts, manufactured linear solve, formula oracle and preserved NCP endpoint analysis')
    (out/'SUMMARY.json').write_text(json.dumps(summary,indent=2)+'\n')
    return 0 if ok else 1

if __name__=='__main__':
    sys.exit(main())
