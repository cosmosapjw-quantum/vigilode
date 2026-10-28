#!/usr/bin/env python3
"""Relocate the standalone R3 native probe, preserving source identity.

Run after selecting the supplied Rust/Cargo offline environment. Outputs must
be a new directory. A report-only descendant checkout is allowed if audited
production bytes match SOURCE_HASHES.json.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time

def main():
    p=argparse.ArgumentParser()
    p.add_argument('--repo',type=Path,required=True)
    p.add_argument('--out',type=Path,required=True)
    p.add_argument('--prepare-only',action='store_true')
    args=p.parse_args(); here=Path(__file__).resolve().parent; repo=args.repo.resolve()
    hashes=json.loads((here/'SOURCE_HASHES.json').read_text())
    for relative,expected in hashes['files'].items():
        actual=hashlib.sha256((repo/relative).read_bytes()).hexdigest()
        if actual!=expected: raise SystemExit('source mismatch: '+relative)
    args.out.mkdir(parents=True,exist_ok=False)
    shutil.copytree(here/'probe',args.out/'probe')
    manifest=args.out/'probe'/'Cargo.toml'
    text=manifest.read_text().replace('../../../vigilode/',str(repo)+'/')
    manifest.write_text(text)
    if args.prepare_only:
        print('PREPARED_ONLY; no native execution claimed'); return
    command=['cargo','run','--locked','--offline','--manifest-path',str(manifest.resolve())]
    started=time.time()
    with (args.out/'stdout.jsonl').open('w') as out, (args.out/'stderr.log').open('w') as err:
        result=subprocess.run(command,stdout=out,stderr=err)
    receipt=dict(command=command,exit_code=result.returncode,elapsed_seconds=time.time()-started,
                 source_commit=hashes['source_commit'],production_source_hashes_checked=True)
    (args.out/'EXECUTION_RECEIPT.json').write_text(json.dumps(receipt,indent=2)+'\n')
    raise SystemExit(result.returncode)

if __name__=='__main__': main()
