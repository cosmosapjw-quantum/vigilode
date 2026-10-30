#!/usr/bin/env python3
from pathlib import Path
import hashlib,json
r=Path(__file__).resolve().parent
m=json.loads((r/'MANIFEST.json').read_text())
files=m['files']; assert m['count']==len(files)
for f in files:
 p=(r/f['path']).resolve()
 assert p.is_relative_to(r) and p.is_file() and not p.is_symlink(),f
 data=p.read_bytes()
 assert len(data)==f['bytes'] and hashlib.sha256(data).hexdigest()==f['sha256'],f
print(f'N4C_MANIFEST_PASS={len(files)}/{len(files)}')
