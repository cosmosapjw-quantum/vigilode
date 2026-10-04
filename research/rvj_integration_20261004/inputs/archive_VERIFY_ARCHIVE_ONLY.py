#!/usr/bin/env python3
"""Verify byte identities only. Does not execute archived scientific code."""
import hashlib,json
from pathlib import Path
root=Path(__file__).resolve().parent
manifest=json.loads((root/'SOURCE_FILE_MANIFEST.json').read_text())
fail=[]
for item in manifest['files']:
    p=root/item['path']
    if not p.is_file():
        fail.append({'path':item['path'],'reason':'missing'});continue
    b=p.read_bytes()
    if len(b)!=item['bytes'] or hashlib.sha256(b).hexdigest()!=item['sha256']:
        fail.append({'path':item['path'],'reason':'identity_mismatch'})
print(json.dumps({'scope':'archive-byte-integrity-only','checked':len(manifest['files']),
                  'failures':fail,'scientific_tests_executed':False},ensure_ascii=False))
raise SystemExit(1 if fail else 0)
