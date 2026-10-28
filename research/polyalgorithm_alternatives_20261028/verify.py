#!/usr/bin/env python3
"""Verify artifact hashes, DAG preservation, candidate mapping and recorded checks.

This packaging check is not a theorem prover or native solver validation.
It never reruns or overwrites the registered numerical outputs.
"""
from pathlib import Path
import hashlib
import json
import subprocess


NODE = Path(__file__).resolve().parent
REPO = NODE.parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    return json.loads(path.read_text())


def confined(name):
    path = Path(name)
    if path.is_absolute() or '..' in path.parts:
        raise ValueError(f'Nonrelative artifact path: {name}')
    target = (REPO / path).resolve()
    if not target.is_relative_to(REPO.resolve()):
        raise ValueError(f'Artifact outside repository: {name}')
    return target


def main():
    manifest = read(NODE / 'MANIFEST.json')
    for name, expected in manifest['files_sha256'].items():
        assert digest(confined(name)) == expected, name
    plan = read(NODE / 'LOCAL_DEVELOPMENT_PLAN.json')
    inherited = read(confined(plan['inherited_plan']['path']))
    assert digest(confined(plan['inherited_plan']['path'])) == plan['inherited_plan']['sha256']
    nodes = {n['id']: n for n in plan['nodes']}
    old = {n['id']: n for n in inherited['nodes']}
    assert len(nodes) == len(plan['nodes']) == 23
    assert nodes.keys() == old.keys()
    active, done = set(), set()

    def visit(key):
        assert key in nodes and key not in active, key
        if key in done:
            return
        active.add(key)
        for parent in nodes[key]['dependencies']:
            visit(parent)
        active.remove(key)
        done.add(key)

    for key in nodes:
        visit(key)
        assert nodes[key]['dependencies'] == old[key]['dependencies'], key
        assert nodes[key]['status'] == old[key]['status'], key
        for field in ['correction_loops_used', 'correction_loops_max']:
            assert nodes[key].get(field) == old[key].get(field), (key, field)
    assert plan['execution_suites'] == inherited['execution_suites']
    candidates = {}
    for name in plan['candidate_catalog']['files']:
        for candidate in read(confined(name))['candidates']:
            assert candidate['id'] not in candidates
            candidates[candidate['id']] = candidate
    assert len(candidates) == plan['candidate_catalog']['count'] == 17
    mapped = set()
    for pr, ids in plan['candidate_catalog']['membership'].items():
        assert nodes[pr]['status'] == 'pending_local'
        assert set(ids) <= candidates.keys()
        mapped.update(ids)
    assert mapped == candidates.keys()
    for lane in ['POLYNOMIAL', 'HOMOTOPY', 'GEOMETRY']:
        receipt = read(NODE / 'evidence' / f'{lane}_RECEIPT.json')
        assert receipt['exit_code'] == 0
        assert receipt['source_commit'] == manifest['supporting_source_commit']
        assert digest(confined(receipt['stdout'])) == receipt['stdout_sha256']
        assert digest(confined(receipt['stderr'])) == receipt['stderr_sha256']
        assert digest(confined(receipt['argv'][1])) == receipt['source_sha256']
        result = read(confined(receipt['stdout']))
        if lane == 'GEOMETRY':
            assert all(c['status'] == 'pass' for c in result['checks'])
            assert result['native_solver_executed'] is False
        else:
            assert result['status'] == 'PASS'
    # Numerical runtime and dependency bytes are unchanged from the inspected base.
    for prefix in ['crates', 'Cargo.toml', 'Cargo.lock', '.github']:
        diff = subprocess.check_output(['git', 'diff', manifest['runtime_baseline_commit'],
                                        '--', prefix], cwd=REPO, text=True)
        assert not diff, f'Runtime/CI change relative to baseline: {prefix}'
    result = {'status': 'PASS', 'artifact_files': len(manifest['files_sha256']),
              'dag_nodes': len(nodes), 'dag_edges': sum(len(n['dependencies']) for n in nodes.values()),
              'candidate_count': len(candidates), 'supporting_lanes_passed': 3,
              'runtime_bytes_unchanged': True, 'native_solver_validation': 'NOT_RUN',
              'scope': 'artifact identity and recorded-check consistency only'}
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
