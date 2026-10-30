#!/usr/bin/env python3
from __future__ import annotations
import hashlib,json,pathlib,sys
ROOT=pathlib.Path(__file__).resolve().parents[1]
manifest=ROOT/'FILE_MANIFEST.json'
expected=json.loads(manifest.read_text())
files={p.relative_to(ROOT).as_posix():p for p in ROOT.rglob('*') if p.is_file() and p!=manifest and not p.name.endswith('.pyc') and '__pycache__' not in p.parts and '.pytest_cache' not in p.parts}
if set(files)!=set(expected):
  print('MANIFEST_MEMBERS_MISMATCH',{'missing':sorted(set(expected)-set(files)),'unexpected':sorted(set(files)-set(expected))});sys.exit(1)
for k,info in sorted(expected.items()):
  h=hashlib.sha256(files[k].read_bytes()).hexdigest()
  if info['sha256']!=h or info['bytes']!=files[k].stat().st_size:
    print('MANIFEST_HASH_FAIL',k,info['sha256'],h);sys.exit(1)
print('MANIFEST_PASS',len(files),'/',len(expected))
