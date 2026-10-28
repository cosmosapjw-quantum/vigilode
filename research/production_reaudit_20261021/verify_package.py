#!/usr/bin/env python3
"""Verify this audit's file identities; this does not rerun or certify a solver."""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    errors = []
    counts = {}
    for name in ('SOURCE_MANIFEST.json', 'EVIDENCE_MANIFEST.json'):
        manifest = json.loads((HERE / name).read_text())
        files = manifest['files']
        counts[name] = len(files)
        if len({row['path'] for row in files}) != len(files):
            errors.append(name + ': duplicate paths')
        for row in files:
            path = ROOT / row['path']
            if not path.resolve().is_relative_to(ROOT.resolve()) or not path.is_file():
                errors.append(row['path'] + ': missing or outside repository')
            elif digest(path) != row['sha256']:
                errors.append(row['path'] + ': digest mismatch')
    execution = json.loads((HERE / 'EXECUTION.json').read_text())
    current = digest(HERE / 'SOURCE_MANIFEST.json')
    for field in ('source_manifest_before_sha256', 'source_manifest_after_sha256'):
        if execution.get(field) != current:
            errors.append('execution/source manifest binding mismatch: ' + field)
    dag = json.loads((HERE / 'DAG.json').read_text())
    for scope, expected in [('original_audit', 12), ('production_plan', 15), ('wave4', 8)]:
        item = dag['namespaces'][scope]
        if item['count'] != expected or len(set(item['ids'])) != expected:
            errors.append('DAG coverage mismatch: ' + scope)
    print(json.dumps({'status': 'PASS' if not errors else 'FAIL', 'files_checked': counts,
                      'errors': errors, 'scope': 'identity_and_coverage_only'}, indent=2))
    return bool(errors)

if __name__ == '__main__':
    raise SystemExit(main())
