#!/usr/bin/env python3
"""Verify published bytes, archived raw evidence and document contracts; no solver runs."""
from pathlib import Path
import hashlib
import json
import tarfile

BASE = Path(__file__).resolve().parents[1]
REPO = BASE.parents[1]

def load(path):
    def reject(value):
        raise ValueError(f'non-JSON numeric constant: {value}')
    return json.loads(path.read_text(), parse_constant=reject)

def digest(data):
    return hashlib.sha256(data).hexdigest()

def main():
    manifest = load(BASE/'ARTIFACT_MANIFEST.json')
    seen = set()
    for item in manifest['files']:
        name = item['path']
        assert name not in seen and not Path(name).is_absolute() and '..' not in Path(name).parts, name
        seen.add(name)
        path = REPO/name
        assert path.is_file() and not path.is_symlink(), name
        data = path.read_bytes()
        assert len(data) == item['bytes'] and digest(data) == item['sha256'], name
    expected_package = {str(p.relative_to(REPO)) for p in BASE.rglob('*')
                        if p.is_file() and '__pycache__' not in p.parts and p.suffix != '.pyc'
                        and p.name != 'ARTIFACT_MANIFEST.json'}
    prefix = str(BASE.relative_to(REPO))+'/'
    assert expected_package == {name for name in seen if name.startswith(prefix)}, 'package coverage mismatch'
    json_count = 0
    for name in sorted(seen):
        if name.endswith('.json'):
            load(REPO/name)
            json_count += 1
    archive_count = 0
    raw_count = 0
    for path in sorted((BASE/'results').glob('*/RAW_MANIFEST.json')):
        info = load(path)
        assert info['archive'] == 'RAW.tar.gz'
        archive = path.parent/info['archive']
        expected = {item['path']: item for item in info['files']}
        assert len(expected) == len(info['files'])
        with tarfile.open(archive, 'r:gz') as tar:
            members = tar.getmembers()
            assert len(members) == len(expected)
            assert {m.name for m in members} == set(expected)
            for member in members:
                assert member.isfile() and not Path(member.name).is_absolute() and '..' not in Path(member.name).parts
                data = tar.extractfile(member).read()
                item = expected[member.name]
                assert len(data) == item['bytes'] and digest(data) == item['sha256'], member.name
                raw_count += 1
        archive_count += 1
    summary = load(BASE/'SUMMARY.json')
    assert summary['broad_production'] == 'HOLD'
    assert summary['switching']['completed'] == 114
    assert summary['switching']['same_error_cap_passes'] == 105
    assert summary['causal']['completed'] == 210 and summary['causal']['same_error_cap_passes'] == 195
    assert summary['pde']['final_native_passes'] == 135
    decisions = load(BASE/'DECISIONS.json')
    ids = {d['id'] for d in decisions['decisions']}
    assert len(ids) == len(decisions['decisions'])
    allowed = set(decisions['status_meanings'])
    assert all(d['status'] in allowed for d in decisions['decisions'])
    assert decisions['broad_production'] == 'HOLD' and decisions['external_comparators_status'] == 'NOT_RUN'
    tree = load(BASE/'RESEARCH_TREE.json')
    node_ids = {n['id'] for n in tree['nodes']}
    root_ids = {r['id'] for r in tree['roots']}
    assert len(node_ids) == len(tree['nodes'])
    assert all(n['parent'] in node_ids | root_ids for n in tree['nodes'])
    assert all(sum(n['parent'] == r for n in tree['nodes']) <= 3 for r in root_ids | node_ids)
    parents = {n['id']:n['parent'] for n in tree['nodes']}
    for node in node_ids:
        traversed = set()
        while node in parents:
            assert node not in traversed, 'research tree cycle'
            traversed.add(node)
            node = parents[node]
    for document in [decisions['decisions'], tree['nodes']]:
        for item in document:
            for path in item.get('evidence', []):
                assert (BASE/path).is_file(), path
    plan = load(BASE/'BENCHMARK_PLAN.json')
    assert len(plan['old_blocker_mapping']) == 40 and not plan['ncp']
    next_plan = load(BASE/'NEXT_STEPS.json')
    tasks = {t['id']:t for t in next_plan['tasks']}
    assert len(tasks) == len(next_plan['tasks']) == 19
    assert not next_plan['constraints']['ncp']
    assert {'SW02','SW03','DV01','QF01','CF01','CP01','QA01','SG01','PB01'} <= set(tasks)
    for task in tasks.values():
        assert set(task['depends_on']) <= set(tasks)
        for name in task['existing_files'] + task['evidence_to_reuse']:
            assert (REPO/name).is_file(), name
        for field in ['contract','implementation_steps','tests','budget','success_criteria','stop_criteria']:
            assert task[field], (task['id'], field)
    def visit(node, stack):
        assert node not in stack, 'task dependency cycle'
        for dep in tasks[node]['depends_on']:
            visit(dep, stack | {node})
    for node in tasks:
        visit(node, set())
    for theorem in load(BASE/'THEOREMS.json')['theorems']:
        assert (BASE/theorem['proof_document']).is_file()
        assert (REPO/theorem['code']).is_file()
    print(json.dumps({'status':'PASS','scope':'package integrity and document contracts; no new solver execution',
                      'manifest_sha256':digest((BASE/'ARTIFACT_MANIFEST.json').read_bytes()),
                      'files':len(seen),'json_documents':json_count,'archives':archive_count,
                      'archived_raw_files':raw_count,'research_roots':len(root_ids),'research_leaves_and_internal_nodes':len(node_ids),
                      'next_steps':len(tasks),'broad_production':'HOLD','external_comparators':'NOT_RUN'}))

if __name__ == '__main__':
    main()
