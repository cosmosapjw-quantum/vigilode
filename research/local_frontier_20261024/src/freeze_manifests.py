#!/usr/bin/env python3
"""Maintainer-only inventory generation; consumers should use verify_package.py."""
from pathlib import Path
import hashlib
import json
import subprocess

PACKAGE=Path(__file__).resolve().parents[1]
REPO=PACKAGE.parents[1]

def entry(f):
    return {'path':f.relative_to(REPO).as_posix(),'sha256':hashlib.sha256(f.read_bytes()).hexdigest(),'bytes':f.stat().st_size}

paths=subprocess.check_output(['git','-C',str(REPO),'ls-files','--cached','--others','--exclude-standard','-z']).decode().split('\0')
sources=[]
for name in sorted(set(paths)):
    f=REPO/name
    if not name or not f.is_file():continue
    if (name in ('Cargo.toml','Cargo.lock','rust-toolchain.toml','rust-toolchain') or
        (name.startswith('crates/') and (name.endswith('.rs') or name.endswith('Cargo.toml'))) or
        (f.is_relative_to(PACKAGE/'local') or f.is_relative_to(PACKAGE/'src')) and f.suffix in ('.py','.sh','.jl','.json')):
        sources.append(entry(f))
(PACKAGE/'SOURCE_MANIFEST.json').write_text(json.dumps({'schema_version':1,'scope':'Rust workspace source/manifests plus local campaign and research replay sources; dependencies pinned by Cargo.lock','files':sources},indent=2)+'\n')
artifacts=[entry(f)for f in sorted(PACKAGE.rglob('*'))if f.is_file() and '__pycache__'not in f.parts and f.name!='ARTIFACT_MANIFEST.json']
(PACKAGE/'ARTIFACT_MANIFEST.json').write_text(json.dumps({'schema_version':1,'scope':'All package files except this self-referential manifest; trust its bytes through the published Git commit tree','files':artifacts},indent=2)+'\n')
print(json.dumps({'source_files':len(sources),'artifact_files':len(artifacts)}))
