#!/usr/bin/env python3
"""Verify this research package's artifact identities, not its scientific claims."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
manifest = json.loads((root/'ARTIFACT_MANIFEST.json').read_text())
failures = []
for item in manifest['files']:
    path = (root/item['path']).resolve()
    if not path.is_relative_to(root) or not path.is_file():
        failures.append({'path':item['path'], 'reason':'missing or out of package'})
        continue
    data = path.read_bytes()
    if len(data) != item['bytes'] or hashlib.sha256(data).hexdigest() != item['sha256']:
        failures.append({'path':item['path'], 'reason':'identity mismatch'})
print(json.dumps({'status':'PASS' if not failures else 'FAIL', 'checked':len(manifest['files']), 'failures':failures}))
raise SystemExit(bool(failures))
