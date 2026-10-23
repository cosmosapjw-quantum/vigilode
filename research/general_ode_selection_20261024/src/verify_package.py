#!/usr/bin/env python3
"""Read-only identity and consistency check; does not execute a solver."""
import hashlib
import json
import re
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
REPO = PACKAGE.parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    manifest = json.loads((PACKAGE / 'ARTIFACT_MANIFEST.json').read_text())
    failures = []
    for name, expected in manifest['files'].items():
        path = REPO / name
        if not path.is_file() or sha(path) != expected:
            failures.append('manifest mismatch: ' + name)
    identity = json.loads((PACKAGE / 'SOURCE_IDENTITY.json').read_text())
    for name, expected in identity['files'].items():
        if sha(REPO / name) != expected:
            failures.append('numerical source mismatch: ' + name)
    validation = json.loads((PACKAGE / 'VALIDATION.json').read_text())
    if any(c['exit_code'] or c['failed'] for c in validation['native_checks']):
        failures.append('nonpassing final native validation')
    if sum(c['passed'] for c in validation['native_checks']) != validation['unique_test_total']:
        failures.append('test count mismatch')
    for name in ['core-survey', 'session-65', 'session-1025']:
        receipt = json.loads((PACKAGE / f'evidence/runs/{name}.receipt.json').read_text())
        if receipt['exit_code']:
            failures.append('run failed: ' + name)
        for stream in ['stdout', 'stderr']:
            if sha(PACKAGE / f'evidence/runs/{name}.{stream}') != receipt[f'{stream}_sha256']:
                failures.append('run receipt mismatch: ' + name + ':' + stream)
        if receipt['source_identity_sha256'] != sha(PACKAGE / 'SOURCE_IDENTITY.json'):
            failures.append('source receipt mismatch: ' + name)
    docdir = REPO / 'docs/general_ode_design_20261024'
    rendered = json.loads((docdir / 'editorial/RENDER_CHECK.json').read_text())
    for name, expected in rendered['sha256'].items():
        if sha(docdir / name) != expected:
            failures.append('rendered document identity mismatch: ' + name)
    text = (docdir / 'DESIGN_KO.md').read_text()
    if re.search(r'\b(?:BASS|rei|DAG|claim gate|PRUNE|HOLD)\b', text, re.I):
        failures.append('excluded scope/status term in external design')
    print(json.dumps({'schema': 'vigilode.package-check.v1',
                      'status': 'FAIL' if failures else 'PASS',
                      'manifest_files': len(manifest['files']),
                      'numerical_inputs': len(identity['files']),
                      'tests_in_final_receipts': validation['unique_test_total'],
                      'limits': 'Identity/consistency only; not an independent numerical revalidation.',
                      'failures': failures}, indent=2))
    return bool(failures)


if __name__ == '__main__':
    raise SystemExit(main())
