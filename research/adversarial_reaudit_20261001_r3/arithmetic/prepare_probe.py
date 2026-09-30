#!/usr/bin/env python3
"""Prepare portable probe copy against byte-verified audited production files."""
import argparse,hashlib,json,shutil
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--repo',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
root=Path(__file__).resolve().parent;repo=a.repo.resolve();out=a.out.resolve()
receipt=json.loads((root/'EXECUTION_RECEIPT.json').read_text())
for name,expected in receipt['source_files'].items():
    actual=hashlib.sha256((repo/name).read_bytes()).hexdigest()
    if actual!=expected: raise SystemExit('SOURCE_MISMATCH: '+name)
if out.exists():raise SystemExit('OUTPUT_EXISTS: choose a new directory')
shutil.copytree(root/'probe',out)
manifest=out/'Cargo.toml';s=manifest.read_text()
for crate in ('rodas5p-core','rodas5p-integrators'):
    s=s.replace('"../../../vigilode/crates/'+crate+'"',json.dumps(str(repo/'crates'/crate)))
manifest.write_text(s)
print(json.dumps({'status':'PREPARED_ONLY','source_files_verified':len(receipt['source_files']),'manifest':str(manifest),'production_modified':False}))
