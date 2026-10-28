"""Portable stdlib audit-artifact validator; not a scientific theorem checker."""
from pathlib import Path
import hashlib
import json
import re
import sys

BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[1]


def pairs(items):
    out = {}
    for key, value in items:
        if key in out:
            raise ValueError(f"duplicate key {key}")
        out[key] = value
    return out


def read(path):
    return json.loads(path.read_text(), object_pairs_hook=pairs,
                      parse_constant=lambda x: (_ for _ in ()).throw(ValueError(x)))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    # This check may redirect its own receipt; do not read a file being written.
    files = [path for path in BASE.rglob('*.json') if path.name != 'package_check.json']
    for path in files:
        read(path)
    decision = read(BASE / 'DECISION.json')
    assert decision['decision'] == 'HOLD_UNQUALIFIED_PRODUCTION'
    for key in ['production_promoted', 'global_accuracy_promoted', 'measured_speed_promoted',
                'certified_packet_promoted', 'leja_total_binary64_certificate_promoted']:
        assert decision[key] is False, key
    nodes = read(BASE / 'NEXT_STEPS.json')['nodes']
    by_id = {node['id']: node for node in nodes}
    assert len(by_id) == len(nodes)
    done, active = set(), set()

    def visit(key):
        assert key not in active, f'cycle: {key}'
        if key in done:
            return
        active.add(key)
        for dependency in by_id[key]['depends_on']:
            assert dependency in by_id, dependency
            visit(dependency)
        active.remove(key)
        done.add(key)

    for key in by_id:
        visit(key)
    for node in nodes:
        assert all(node[key] for key in ['source','implementation','interface','acceptance','cost_contract'])
        for dependencies in node.get('scope_dependencies', {}).values():
            assert all(key in by_id for key in dependencies)
    bindings = read(BASE / 'REVIEW_BINDINGS.json')['bindings']
    for row in bindings:
        assert sha(ROOT / row['durable_path']) == row['sha256'], row['durable_path']
    source = read(BASE / 'SOURCE_IDENTITY.json')
    for row in source['final_changed_files']:
        assert sha(ROOT / row['path']) == row['sha256'], row['path']
    for row in read(BASE / 'EVIDENCE_INDEX.json')['files']:
        path = ROOT / row['path']
        assert path.stat().st_size == row['bytes'], row['path']
        assert sha(path) == row['sha256'], row['path']
    report = ROOT / 'docs/reviews/20261018_production_audit/REPORT_KO.md'
    links = re.findall(r'\]\(([^)]+)\)', report.read_text())
    for target in links:
        if '://' not in target and not target.startswith('#'):
            assert (report.parent / target.split('#')[0]).resolve().is_file(), target
    result = {'status':'PASS_SCOPED','scope':'JSON/path/hash/DAG/decision consistency only',
              'json_files':len(files),'dag_nodes':len(nodes),'review_bindings':len(bindings),
              'final_source_files':len(source['final_changed_files']),'report_links':len(links)}
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
