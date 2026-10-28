#!/usr/bin/env python3
"""Relocate the already tested probe to a temporary directory; do not mutate repo.
Usage: source RUST_ENV; python3 run_probe.py --repo /path/to/vigilode --output /path/to/results
The wrapper itself has only been syntax checked; native evidence was obtained via
Cargo directly with the bundled source and original relative path dependencies.
"""
import argparse, pathlib, tempfile, shutil, subprocess, json, sys
p=argparse.ArgumentParser();p.add_argument('--repo',type=pathlib.Path,required=True);p.add_argument('--output',type=pathlib.Path,required=True);p.add_argument('--allow-network',action='store_true');a=p.parse_args()
repo=a.repo.resolve();here=pathlib.Path(__file__).resolve().parent;out=a.output.resolve();out.mkdir(parents=True,exist_ok=True)
sha=subprocess.check_output(['git','-C',str(repo),'rev-parse','HEAD'],text=True).strip()
if sha!='7708ef90554fc3986478d4602de6a01c7266b14f':
    print('NOTICE: current repo HEAD differs from audit source; results are a new execution, not a source-identical reproduction.',file=sys.stderr)
with tempfile.TemporaryDirectory(prefix='vigilode-statistics-') as work:
    work=pathlib.Path(work);shutil.copytree(here/'probe'/'src',work/'src');shutil.copyfile(here/'probe'/'Cargo.lock',work/'Cargo.lock')
    manifest=(here/'probe'/'Cargo.toml').read_text().replace('../../../vigilode',repo.as_posix())
    (work/'Cargo.toml').write_text(manifest)
    cmd=['cargo','run','--locked','--manifest-path',str(work/'Cargo.toml')]
    if not a.allow_network:cmd.append('--offline')
    with (out/'probe.log').open('w') as log:result=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT)
    (out/'execution.json').write_text(json.dumps({'repo':str(repo),'commit':sha,'command':cmd,'exit_code':result.returncode,'status':'PASS' if result.returncode==0 else 'EXECUTION_FAILED'},indent=2)+'\n')
    raise SystemExit(result.returncode)
