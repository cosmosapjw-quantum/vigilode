"""Verify package bytes and explicit aggregate invariants, without solving ODEs."""
from pathlib import Path
import hashlib
import json
import re

ROOT=Path(__file__).resolve().parents[1]
REPO=ROOT.parents[1]


def main():
    manifest=json.loads((ROOT/'ARTIFACT_MANIFEST.json').read_text())
    for entry in manifest['files']:
        p=ROOT/entry['path']
        assert p.is_file(), entry['path']
        content=p.read_bytes()
        assert len(content)==entry['bytes'], entry['path']
        assert hashlib.sha256(content).hexdigest()==entry['sha256'],entry['path']
    for entry in manifest['patched_repository_files']:
        p=REPO/entry['path']
        assert hashlib.sha256(p.read_bytes()).hexdigest()==entry['sha256'],entry['path']
    d=json.loads((ROOT/'BENCHMARK_FINDINGS.json').read_text())
    for g in ['uninstrumented','instrumented']:
        row=d['local_tasks'][g]
        assert row['total']==sum(row[k] for k in ['endpoint_pass','endpoint_fail','solver_fail','timeout','reference_missing'])
    assert sum(d['local_tasks'][g]['total'] for g in ['uninstrumented','instrumented'])==1539
    assert d['new_ode_benchmark_runs']==0
    assert d['historical_corrections']['rotation_pass_count']==7
    images=re.findall(r'!\[[^]]*\]\(([^)]+)\)',(ROOT/'REPORT_KO.md').read_text())
    assert len(images)==8
    assert all((ROOT/p).is_file() for p in images)
    print(json.dumps({'package_files_verified':len(manifest['files']),
       'patched_repository_files_verified':len(manifest['patched_repository_files']),
       'figure_links':len(images),'scope':'byte identity and explicit aggregate arithmetic',
       'status':'PASS'},indent=2))


if __name__=='__main__':main()
