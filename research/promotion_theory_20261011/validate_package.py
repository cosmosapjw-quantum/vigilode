#!/usr/bin/env python3
"""Validate delivery identities, scope separation and port dependencies.

This is not a theorem prover or production scientific-result checker.
"""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read(path):
    def invalid_constant(value):
        raise ValueError(f"nonfinite JSON token {value} in {path}")
    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, f"duplicate JSON key {key} in {path}")
            result[key] = value
        return result
    return json.loads(path.read_text(), parse_constant=invalid_constant, object_pairs_hook=unique)


def main():
    json_paths = list(HERE.rglob('*.json'))
    for path in json_paths:
        read(path)
    binding = read(HERE / 'SOURCE_BINDING.json')
    require(binding['production_edits'] is False, 'production changes outside theory scope')
    require(binding['native_execution_this_turn'] is False, 'unexpected native execution claim')
    decision = read(HERE / 'DECISION.json')
    for key in ['current_production', 'current_executable_global_accuracy', 'measured_wall_time']:
        require(decision['scope_decisions'][key] == 'HOLD', f'unsupported promotion: {key}')
    require(decision['scope_decisions']['derived_conditional_theorems'] == 'PROMOTE', 'missing independent theorem decision')
    claims = read(HERE / 'THEOREMS.json')['claims']
    ids = [x['id'] for x in claims]
    require(len(ids) == len(set(ids)), 'duplicate theorem id')
    for claim in claims:
        require(claim['assumptions'], f'missing assumptions: {claim["id"]}')
        require(claim['decision'] == 'PROMOTE', f'unresolved claim: {claim["id"]}')
        require((ROOT / claim['proof']['path']).is_file(), f'missing proof: {claim["id"]}')
        require((ROOT / claim['independent_review']).is_file(), f'missing review: {claim["id"]}')

    dag = read(HERE / 'PORTING_DAG.json')
    nodes = dag['nodes']
    node_ids = [node['id'] for node in nodes]
    require(len(node_ids) == len(set(node_ids)), 'duplicate DAG id')
    edges = {node['id']: node.get('depends_on', []) for node in nodes}
    visiting, seen = set(), set()
    def visit(node_id):
        require(node_id not in visiting, f'DAG cycle: {node_id}')
        if node_id in seen:
            return
        visiting.add(node_id)
        for dep in edges[node_id]:
            require(dep in edges, f'unknown dependency: {dep}')
            visit(dep)
        visiting.remove(node_id)
        seen.add(node_id)
    for node in nodes:
        require(node['status'] == 'PROPOSED_NOT_EXECUTED', f'unsupported execution: {node["id"]}')
        for theorem_id in node['theorem_ids']:
            require(theorem_id in ids, f'unknown theorem reference: {theorem_id}')
        for seam in node.get('current_source_seams', []):
            if seam['status'] == 'EXISTS_AT_BASELINE':
                require((ROOT / seam['path']).is_file(), f'missing current source seam: {seam["path"]}')
        visit(node['id'])

    aliases = read(HERE / 'PATH_ALIASES.json')['paths']
    hash_checks = 0
    for item in read(HERE / 'REVIEW_INPUT_BINDINGS.json')['inputs']:
        path = ROOT / item['delivery_path']
        require(path.is_file(), f'missing reviewed delivery file: {path}')
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        require(digest == item['sha256'], f'reviewer hash mismatch: {path}')
        hash_checks += 1
    for claim in claims:
        require(claim['promotion_scope'] not in ['production', 'measured_wall_time'], 'theory scope conflation')
    return {'schema_version': '1.0', 'status': 'PASS', 'json_files_checked': len(json_paths),
            'theorems': len(claims), 'port_nodes': len(nodes), 'reviewed_hashes': hash_checks,
            'historical_campaigns_rerun': 0, 'validation_kind': 'delivery_semantics_and_identity_only',
            'proves_mathematical_theorems': False}


if __name__ == '__main__':
    result = main()
    (HERE / 'VALIDATION.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result))
