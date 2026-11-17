#!/usr/bin/env python3
"""Check source/evidence identity and executable plan structure, not science truth."""
import hashlib
import fnmatch
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
PROGRAM = ROOT / 'docs/agent/production_solver_program_20261112'

def pairs(items):
    result = {}
    for key, value in items:
        if key in result:
            raise ValueError('duplicate JSON key: ' + key)
        result[key] = value
    return result

def strict(path):
    def invalid(value):
        raise ValueError('nonfinite JSON constant: ' + value)
    return json.loads(path.read_text(), object_pairs_hook=pairs, parse_constant=invalid)

def require(condition, message):
    if not condition:
        raise ValueError(message)

def graph(nodes):
    ids = [n['id'] for n in nodes]
    require(len(ids) == len(set(ids)), 'duplicate task ID')
    by_id = {n['id']: n for n in nodes}
    active, done = set(), set()
    def visit(node):
        require(node in by_id, 'missing dependency: ' + node)
        require(node not in active, 'cycle: ' + node)
        if node in done:
            return
        active.add(node)
        for dep in by_id[node]['depends']:
            visit(dep)
        active.remove(node)
        done.add(node)
    for node in ids:
        visit(node)

def git(*args):
    return subprocess.check_output(['git', '-C', str(ROOT), *args], text=True).strip()

def main():
    checkpoint = strict(HERE / 'CHECKPOINT.json')
    base = checkpoint['base_commit']
    require(git('rev-parse', base) == base, 'base git object unavailable')
    require(git('merge-base', base, 'HEAD') == base, 'HEAD does not descend from baseline')
    tasks = strict(PROGRAM / 'TASK_CONTRACTS.json')['tasks']
    dag = strict(PROGRAM / 'RESEARCH_DAG.json')
    graph(tasks)
    graph(dag['nodes'])
    by_id = {n['id']: n for n in dag['nodes']}
    for task in tasks:
        require(task['status'] == by_id[task['id']]['status'], 'task/DAG status drift: ' + task['id'])
        require(task['depends'] == by_id[task['id']]['depends'], 'task/DAG dependency drift: ' + task['id'])
    for parent, children in dag['realization_map'].items():
        require(parent in by_id and all(c in by_id for c in children), 'invalid realization map')
    require(dag['production_verdict'] == 'HOLD', 'unreviewed production promotion')
    require(dag['official_g0_mutated'] is False, 'official G0 mutation')
    for task in tasks:
        for key in ['purpose', 'write_allowlist', 'equations_and_design',
                    'acceptance_checks', 'completion_requires', 'failure_action']:
            require(bool(task[key]), task['id'] + ' lacks ' + key)
        if task['status'] == 'COMPLETE_SCOPED':
            require(bool(task['evidence']), task['id'] + ' lacks completion evidence')
            for ref in task['evidence']:
                require((ROOT / ref.split('#')[0]).is_file(), 'missing evidence ' + ref)
    for path, expected in checkpoint['protected_blob_sha1'].items():
        require(git('hash-object', path) == expected, 'protected file changed: ' + path)
    for path, expected in checkpoint['evidence_sha256'].items():
        actual = hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
        require(actual == expected, 'evidence bytes changed: ' + path)
    for path, expected in checkpoint['implemented_source_sha256'].items():
        require(hashlib.sha256((ROOT / path).read_bytes()).hexdigest() == expected,
                'implementation changed; rerun affected tests/update checkpoint: ' + path)
    for path in [*PROGRAM.glob('*.json'), *HERE.rglob('*.json')]:
        strict(path)
    allowed = strict(PROGRAM / 'MUTATION_GATES.json')['active_allowlist']
    changed = git('diff', '--name-only', base).splitlines()
    for path in changed:
        require(any(fnmatch.fnmatchcase(path, pattern) for pattern in allowed),
                'changed path outside active allowlist: ' + path)
    print('PASS: strict JSON, task/DAG structure, ancestry, protected and evidence identities')
    print('Scope: checkpoint reproducibility. Production HOLD; no new numerical qualification.')

if __name__ == '__main__':
    main()
