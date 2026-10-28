#!/usr/bin/env python3
"""Read-only verification of package and execution-source SHA256 inventories."""
import hashlib
import json
from pathlib import Path
import sys

PACKAGE = Path(__file__).resolve().parents[1]
REPO = PACKAGE.parents[1]

def main():
    failures = []
    counts = {}
    for name in ('ARTIFACT_MANIFEST.json', 'SOURCE_MANIFEST.json'):
        manifest = json.loads((PACKAGE / name).read_text())
        counts[name] = len(manifest['files'])
        for entry in manifest['files']:
            path = REPO / entry['path']
            if not path.is_relative_to(REPO) or '..' in Path(entry['path']).parts:
                failures.append({'path': entry['path'], 'reason': 'invalid relative path'})
                continue
            try:
                actual = hashlib.sha256(path.read_bytes()).hexdigest()
            except OSError as error:
                failures.append({'path': entry['path'], 'reason': str(error)})
                continue
            if actual != entry['sha256']:
                failures.append({'path': entry['path'], 'reason': 'SHA256 mismatch'})
    print(json.dumps({'status': 'PASS' if not failures else 'FAIL', 'counts': counts,
                      'failures': failures, 'scope': 'content identity; does not establish mathematical correctness or production readiness'}, indent=2))
    return int(bool(failures))

if __name__ == '__main__':
    sys.exit(main())
