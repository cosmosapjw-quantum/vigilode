#!/usr/bin/env python3
"""Check reused evidence and policy consistency. Does not execute any solver."""
import argparse
import hashlib
import json
from pathlib import Path


def check(root: Path) -> dict:
    path = root / 'research/general_ode_selection_20261024/analysis/PORTFOLIO.json'
    data = json.loads(path.read_text())
    errors = []
    for key, source in data['sources'].items():
        source_path = root / source['path']
        digest = hashlib.sha256(source_path.read_bytes()).hexdigest() if source_path.is_file() else None
        if digest != source['sha256']:
            errors.append(f'{key}: evidence file missing or changed')
    families = data['families']
    ids = [x['id'] for x in families]
    if len(ids) != len(set(ids)):
        errors.append('duplicate algorithm family')
    selected = [x['id'] for x in families if x['eligible_for_new_general_selector']]
    if set(selected) != {'EXPLICIT', 'RODAS_DENSE', 'RODAS_BANDED', 'RODAS_MATRIX_FREE'}:
        errors.append('unexpected default algorithm family')
    for family in families:
        if not family['evidence'] or any(k not in data['sources'] for k in family['evidence']):
            errors.append(f"{family['id']}: unresolved evidence")
        if family['eligible_for_new_general_selector'] and family['disposition'] != 'SELECT_SCOPED_CORE':
            errors.append(f"{family['id']}: inconsistent automatic-selection status")
    cyclic = json.loads((root / data['sources']['L']['path']).read_text())
    if cyclic['timing_authority'] != 'diagnostic_only':
        errors.append('LP01 timing authority changed; review required')
    if cyclic['all_statuses'] != {'PASS_SAMPLED': 36}:
        errors.append('LP01 sampled accuracy population changed; review required')
    if data['new_solver_runs'] != 0 or data['new_performance_claims']:
        errors.append('this evidence reuse analysis cannot claim new solver results')
    if len(data['search']['candidates']) > 3:
        errors.append('policy search exceeded three variants')
    return {
        'schema': 'vigilode.portfolio-analysis-check.v1',
        'status': 'PASS' if not errors else 'FAIL',
        'evidence_files_checked': len(data['sources']),
        'algorithm_families': len(families),
        'automatic_selection_families': selected,
        'policy_variants': len(data['search']['candidates']),
        'new_solver_runs': 0,
        'lp01_timing_authority': cyclic['timing_authority'],
        'errors': errors,
        'meaning': 'Evidence identity and selection-policy consistency only; no proof of executable selector behavior or production qualification.'
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[3])
    args = parser.parse_args()
    result = check(args.root)
    print(json.dumps(result, ensure_ascii=False, indent=2))
    raise SystemExit(0 if result['status'] == 'PASS' else 1)
