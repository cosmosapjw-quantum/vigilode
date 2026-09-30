#!/usr/bin/env python3
"""Relocate the recorded probe, verify source file identity, and run offline.

Use an already configured Rust1.94.1 environment. This is a reproducibility
runner, not an installer; a caller should serialize concurrent Cargo builds.
"""
import argparse,hashlib,json,re,shutil,subprocess,sys,time
from pathlib import Path

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--repo',type=Path,required=True)
    ap.add_argument('--out',type=Path,required=True)
    ap.add_argument('--prepare-only',action='store_true')
    a=ap.parse_args(); root=Path(__file__).resolve().parent
    repo=a.repo.resolve(); out=a.out.resolve()
    contract=json.loads((root/'SOURCE_CONTRACT.json').read_text())
    mismatches=[]
    for row in contract['files']:
        p=repo/row['path']
        if not p.is_file() or hashlib.sha256(p.read_bytes()).hexdigest()!=row['sha256']:
            mismatches.append(row['path'])
    if mismatches:raise SystemExit('Source identity mismatch: '+', '.join(mismatches))
    out.mkdir(parents=True,exist_ok=False)
    probe=out/'probe';shutil.copytree(root/'probe',probe)
    manifest=(probe/'Cargo.toml').read_text()
    for name in ('rodas5p-core','rodas5p-integrators'):
        manifest=manifest.replace('"../../../vigilode/crates/'+name+'"',json.dumps(str(repo/'crates'/name)))
    (probe/'Cargo.toml').write_text(manifest)
    receipt={'source_commit':contract['source_commit'],'source_files_verified':len(contract['files']),
             'prepared_probe':str(probe),'execution':'NOT_RUN','commands':[]}
    if not a.prepare_only:
        cmd=['cargo','run','--locked','--offline','--jobs','1','--manifest-path',str(probe/'Cargo.toml')]
        start=time.monotonic()
        with (out/'native_raw.jsonl').open('w') as stdout,(out/'native_build.log').open('w') as stderr:
            p=subprocess.run(cmd,stdout=stdout,stderr=stderr,timeout=600)
        receipt['commands'].append({'argv':cmd,'exit_code':p.returncode,'elapsed_seconds':time.monotonic()-start})
        if p.returncode==0:
            cmd=[sys.executable,str(root/'exact_oracle.py'),'--raw',str(out/'native_raw.jsonl'),'--out',str(out/'exact_oracle.json')]
            p=subprocess.run(cmd,capture_output=True,text=True,timeout=30)
            (out/'oracle.log').write_text(p.stdout+p.stderr)
            receipt['commands'].append({'argv':cmd,'exit_code':p.returncode})
        receipt['execution']='COMPLETED' if p.returncode==0 else 'FAILED'
    (out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print(json.dumps(receipt))
    if receipt['execution']=='FAILED':raise SystemExit(1)

if __name__=='__main__':main()
