#!/usr/bin/env python3
"""Verify the exact published file bytes; does not rerun scientific experiments."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[3]
package = Path(__file__).resolve().parents[1]
manifest = json.loads((package / 'ARTIFACT_MANIFEST.json').read_text())
errors = []
for entry in manifest['files']:
    path = root / entry['path']
    if not path.is_file():
        errors.append({'path': entry['path'], 'reason': 'missing'})
        continue
    data = path.read_bytes()
    if len(data) != entry['bytes'] or hashlib.sha256(data).hexdigest() != entry['sha256']:
        errors.append({'path': entry['path'], 'reason': 'content mismatch'})
print(json.dumps({'schema': 'vigilode.package-byte-verification.v1',
                  'status': 'PASS' if not errors else 'FAIL',
                  'files': len(manifest['files']), 'errors': errors,
                  'scope': 'file identity; not a scientific rerun'}))
raise SystemExit(1 if errors else 0)
