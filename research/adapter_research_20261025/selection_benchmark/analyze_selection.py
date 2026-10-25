#!/usr/bin/env python3
"""Read-only selection-window analysis. Never launches or reclassifies old runs.

This is a validator/aggregator of reported measurements, not a raw receipt verifier.
The caller supplies independently verified identities, errors and disjoint timings.
"""
from __future__ import annotations
import argparse
from collections import defaultdict
import hashlib
from fractions import Fraction
import json
import math
from pathlib import Path
import re

IDENTITY_FIELDS = (
    'problem_sha256', 'parameters_sha256', 'start_state_sha256',
    't_start_hex', 't_end_hex', 'physical_metric_sha256',
    'reference_sha256', 'reference_kind', 'error_ceiling_hex',
    'host_sha256', 'source_tree_sha256', 'build_config_sha256',
    'output_workload_sha256', 'timing_scope_sha256', 'environment_epoch',
)
COST_FIELDS = ('solve_seconds', 'setup_seconds', 'probe_seconds', 'transition_seconds')
STATUSES = {'completed', 'solver_failed', 'timeout', 'interrupted', 'not_run', 'reference_missing'}
HASH = re.compile(r'^[0-9a-f]{64}$')

def real(x):
    if isinstance(x, bool) or not isinstance(x, (int, float)):
        return False
    try:
        return math.isfinite(x)
    except (OverflowError, ValueError):
        return False

def hex_real(x):
    if not isinstance(x, str):
        raise ValueError('expected canonical binary64 hex string')
    try:
        y = float.fromhex(x)
    except (ValueError, OverflowError) as exc:
        raise ValueError('invalid hex number') from exc
    if not math.isfinite(y) or y.hex() != x:
        raise ValueError('expected finite canonical binary64 hex string')
    return y

def validate_identity(identity):
    if not isinstance(identity, dict) or set(IDENTITY_FIELDS) - identity.keys():
        return 'MISSING_IDENTITY'
    for key in IDENTITY_FIELDS:
        if key.endswith('_sha256') and (not isinstance(identity[key], str) or not HASH.fullmatch(identity[key])):
            return 'INVALID_IDENTITY_HASH'
    try:
        a, b = (hex_real(identity[k]) for k in ('t_start_hex', 't_end_hex'))
        target = hex_real(identity['error_ceiling_hex'])
    except ValueError:
        return 'INVALID_IDENTITY_NUMBER'
    if not b > a or not target > 0:
        return 'INVALID_INTERVAL_OR_TARGET'
    if identity['reference_kind'] not in {'analytic', 'certified', 'empirical'}:
        return 'INVALID_REFERENCE_KIND'
    epoch = identity['environment_epoch']
    if isinstance(epoch, bool) or not isinstance(epoch, int) or epoch < 0:
        return 'INVALID_ENVIRONMENT_EPOCH'
    return None

def assess(row, anchor):
    """Return one reproducible exclusion reason, or validated cost/error data."""
    identity = row.get('identity')
    reason = validate_identity(identity)
    if reason:
        return {'excluded': reason}
    differences = [k for k in IDENTITY_FIELDS if identity[k] != anchor[k]]
    if differences:
        return {'excluded': 'UNMATCHED_COMPARISON', 'different_fields': differences}
    if row.get('status') not in STATUSES:
        return {'excluded': 'INVALID_STATUS'}
    if row['status'] != 'completed':
        return {'excluded': row['status'].upper()}
    if row.get('final_time_hex') != identity['t_end_hex']:
        return {'excluded': 'INCOMPLETE_HORIZON'}
    if row.get('raw_verified') is not True:
        return {'excluded': 'UNVERIFIED_RECEIPT'}
    for field in ('receipt_sha256', 'binary_sha256'):
        if not isinstance(row.get(field), str) or not HASH.fullmatch(row[field]):
            return {'excluded': 'MISSING_RUN_IDENTITY'}
    if row.get('reference_admitted') is not True:
        return {'excluded': 'REFERENCE_UNADMITTED'}
    error, uncertainty = row.get('physical_error'), row.get('reference_uncertainty')
    if not real(error) or not real(uncertainty) or error < 0 or uncertainty < 0:
        return {'excluded': 'INVALID_ACCURACY_VALUE'}
    # Represent the sum conservatively; there is no claim this certifies empirical U.
    error_upper = math.nextafter(float(error) + float(uncertainty), math.inf)
    if not math.isfinite(error_upper):
        return {'excluded': 'INVALID_ACCURACY_VALUE'}
    exact_error_sum = Fraction(float(error)) + Fraction(float(uncertainty))
    if exact_error_sum > Fraction(hex_real(identity['error_ceiling_hex'])):
        return {'excluded': 'ACCURACY_FAILED', 'physical_error_plus_uncertainty_upper': error_upper}
    cost = row.get('cost')
    if not isinstance(cost, dict) or cost.get('disjoint_components') is not True:
        return {'excluded': 'UNKNOWN_COST_SCOPE'}
    if any(not real(cost.get(k)) or cost[k] < 0 for k in COST_FIELDS):
        return {'excluded': 'INVALID_COST'}
    if not real(cost.get('total_seconds')) or cost['total_seconds'] <= 0:
        return {'excluded': 'INVALID_COST'}
    try:
        summed = math.fsum(cost[k] for k in COST_FIELDS)
    except OverflowError:
        return {'excluded': 'INVALID_COST'}
    total = float(cost['total_seconds'])
    if not math.isfinite(summed) or not math.isclose(total, summed, rel_tol=2e-12, abs_tol=2e-15):
        return {'excluded': 'COST_ACCOUNTING_MISMATCH'}
    return {'total_seconds': total, 'physical_error_plus_uncertainty_upper': error_upper,
            'reported_physical_error': error, 'reported_reference_uncertainty': uncertainty}

def analyze(packet):
    if not isinstance(packet, dict) or packet.get('schema_version') != 1 or isinstance(packet.get('schema_version'), bool):
        raise ValueError('schema_version must be 1')
    data_kind = packet.get('data_kind')
    if data_kind not in {'measured', 'synthetic_contract_test'}:
        raise ValueError('data_kind must identify measured or synthetic_contract_test')
    rows = packet.get('rows')
    if not isinstance(rows, list):
        raise ValueError('rows must be an array')
    groups, rejected = defaultdict(list), []
    for index, row in enumerate(rows):
        if not isinstance(row, dict):
            rejected.append({'row_index': index, 'reason': 'INVALID_ROW'}); continue
        if any(not isinstance(row.get(k), str) or not row[k] for k in ('window_id', 'paired_block_id', 'candidate_id')) or type(row.get('selected')) is not bool:
            rejected.append({'row_index': index, 'reason': 'MISSING_ROW_KEY'}); continue
        groups[(row['window_id'], row['paired_block_id'])].append((index, row))
    outputs = []
    for (window, block), group in sorted(groups.items()):
        out = {'window_id': window, 'paired_block_id': block, 'rows': len(group)}
        selected = [(i, r) for i, r in group if r['selected']]
        ids = [r['candidate_id'] for _, r in group]
        if len(selected) != 1 or len(ids) != len(set(ids)):
            out.update(status='INVALID_SELECTION_GROUP', cause='exactly one selected and unique candidate_id required')
            outputs.append(out); continue
        index, chosen = selected[0]
        reason = validate_identity(chosen.get('identity'))
        if reason:
            out.update(status='INVALID_SELECTED_IDENTITY', cause=reason)
            outputs.append(out); continue
        anchor = chosen['identity']
        checks = []
        for i, row in group:
            check = assess(row, anchor)
            checks.append({'row_index': i, 'candidate_id': row['candidate_id'], 'selected': row['selected'], **check})
        out['assessment'] = checks
        out['selected_candidate_id'] = chosen['candidate_id']
        out['reported_selection_reason'] = chosen.get('selection_reason')
        valid = [v for v in checks if 'excluded' not in v]
        selected_check = next(v for v in checks if v['selected'])
        out['eligible_candidates'] = len(valid)
        if 'excluded' in selected_check:
            out.update(status='SELECTED_NOT_QUALIFIED', cause=selected_check['excluded'])
        elif len(valid) < 2:
            out.update(status='NO_QUALIFIED_COUNTERFACTUAL')
        else:
            best = min(valid, key=lambda v: v['total_seconds'])
            ratio = selected_check['total_seconds'] / best['total_seconds']
            if not math.isfinite(ratio):
                out.update(status='NUMERIC_RANGE_EXCEEDED')
            else:
                out.update(status='COMPARABLE', best_measured_candidate_id=best['candidate_id'],
                           selected_total_seconds=selected_check['total_seconds'],
                           best_measured_total_seconds=best['total_seconds'],
                           observed_cost_ratio=ratio, observed_excess_seconds=selected_check['total_seconds']-best['total_seconds'],
                           interpretation='same physical error ceiling; achieved errors may differ; no causal diagnosis or statistical confidence implied')
        outputs.append(out)
    return {'schema_version': 1, 'data_kind': data_kind,
            'scope': 'same-checkpoint paired native candidates; not a universal best-method oracle',
            'performance_claim_authorized': False,
            'validation_scope': 'reported values and identities only; raw verification is a caller-provided attestation',
            'windows': outputs, 'rejected_rows': rejected,
            'summary': {status: sum(o['status'] == status for o in outputs) for status in sorted({o['status'] for o in outputs})}}

def reject_constant(x):
    raise ValueError('nonfinite JSON token: ' + x)

def unique_object(pairs):
    out = {}
    for k, v in pairs:
        if k in out:
            raise ValueError('duplicate JSON key: ' + k)
        out[k] = v
    return out

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('input', type=Path)
    p.add_argument('--output', type=Path, required=True, help='new JSON path, never overwritten')
    args = p.parse_args()
    raw = args.input.read_bytes()
    packet = json.loads(raw, parse_constant=reject_constant, object_pairs_hook=unique_object)
    result = analyze(packet)
    result['input_sha256'] = hashlib.sha256(raw).hexdigest()
    result['analyzer_sha256'] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    with args.output.open('x', encoding='utf8') as f:
        json.dump(result, f, ensure_ascii=False, indent=2, allow_nan=False)
        f.write('\n')
    return 0

if __name__ == '__main__':
    raise SystemExit(main())
