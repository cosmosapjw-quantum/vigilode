#!/usr/bin/env python3
"""Relocate the native probe without modifying the original repository."""
import argparse, hashlib, json, os, shutil, subprocess
from pathlib import Path

p=argparse.ArgumentParser()
p.add_argument('--repo',required=True,type=Path)
p.add_argument('--out',required=True,type=Path)
p.add_argument('--target-dir',required=True,type=Path)
p.add_argument('--prepare-only',action='store_true')
a=p.parse_args()
here=Path(__file__).resolve().parent
receipt=json.loads((here/'EXECUTION_RECEIPT.json').read_text())
repo=a.repo.resolve()
for name,expected in receipt['source_sha256'].items():
    actual=hashlib.sha256((repo/name).read_bytes()).hexdigest()
    if actual!=expected:
        raise SystemExit(f'Source identity mismatch: {name}; refuse to reuse this receipt.')
out=a.out.resolve()
if out.exists():
    raise SystemExit('Choose a new --out directory; existing evidence is not overwritten.')
out.mkdir(parents=True)
shutil.copytree(here/'probe',out/'probe')
manifest=(out/'probe/Cargo.toml').read_text()
for crate in ['rodas5p-core','rodas5p-integrators']:
    manifest=manifest.replace(f'"../../../vigilode/crates/{crate}"',json.dumps(str(repo/'crates'/crate)))
(out/'probe/Cargo.toml').write_text(manifest)
if a.prepare_only:
    print(json.dumps({'prepared':str(out/'probe'),'source_identity':'matched','execution':'NOT_RUN'}))
    raise SystemExit(0)
env=os.environ.copy()
env.update(CARGO_TARGET_DIR=str(a.target_dir.resolve()),CARGO_BUILD_JOBS='1')
base=['cargo','--offline','--locked','--manifest-path',str(out/'probe/Cargo.toml')]
commands=[('native', ['run',*base[1:],'--bin','vigilode-output-cert-reaudit']),
          ('regression', ['test',*base[1:],'--tests']),
          ('candidate', ['run',*base[1:],'--bin','safe_budget'])]
results=[]
for name,args in commands:
    with (out/f'{name}.stdout').open('w') as stdout,(out/f'{name}.stderr').open('w') as stderr:
        result=subprocess.run(['cargo',*args],env=env,stdout=stdout,stderr=stderr,timeout=300)
    results.append({'name':name,'argv':['cargo',*args],'exit_code':result.returncode})
(out/'rerun_receipt.json').write_text(json.dumps(results,indent=2)+'\n')
print(json.dumps(results,indent=2))
raise SystemExit(any(r['exit_code']!=0 for r in results))
