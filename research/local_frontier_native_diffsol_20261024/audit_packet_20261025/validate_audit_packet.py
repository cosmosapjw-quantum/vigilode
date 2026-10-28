#!/usr/bin/env python3
"""Validate compact packet identities and counts without large raw artifacts."""
from __future__ import annotations

import collections
import hashlib
import json
from pathlib import Path
import re
import sys

HERE = Path(__file__).resolve().parent
HEX64 = re.compile(r'[0-9a-f]{64}\Z')


def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda: f.read(1 << 20), b''):
            h.update(block)
    return h.hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def main():
    packet = json.loads((HERE/'AUDIT_INDEX.json').read_text())
    compact_path = HERE/packet['legacy']['status_index_file']
    require(sha(compact_path) == packet['legacy']['status_index_sha256'], 'compact index SHA')
    compact = json.loads(compact_path.read_text())
    rows = compact['rows']
    require(len(rows) == packet['legacy']['task_count'] == 1539, 'task count')
    require(len({r['task_id'] for r in rows}) == 1539, 'unique task identities')
    require(all(HEX64.fullmatch(r['receipt_sha256']) for r in rows), 'receipt SHA format')
    require(dict(collections.Counter(r['science'] for r in rows)) ==
            packet['legacy']['final_status_counts'], 'science status histogram')
    require(sum(r['science'] == 'UNVALIDATED_REFERENCE' for r in rows) == 805,
            'original unvalidated count')
    require(sum(r['original_final_uninstrumented_timeout'] for r in rows) == 199,
            'original final timeout count')
    require(sum(r['original_final_typed_step_budget_failure'] for r in rows) == 54,
            'original final typed step-budget count')
    require(len(packet['external_reference_cases']) == 16 and
            len({r['case'] for r in packet['external_reference_cases']}) == 16,
            'external reference case count')
    require(sum(f['count'] for f in packet['timeout']['families']) == 199,
            'timeout family count')
    require(len(packet['typed_step_budget']['failures']) == 54,
            'typed step-budget count')
    require(packet['cost']['event_count'] == 174 and
            HEX64.fullmatch(packet['cost']['events_sha256']) is not None and
            packet['cost']['conservative_fixture']['charged_seconds'] == 120,
            'cost summary shape')
    for item in packet['copied_sources']:
        local = HERE/item['name']
        require(local.is_file() and local.stat().st_size == item['bytes'] and
                sha(local) == item['sha256'], 'copied source mismatch: '+item['name'])
    for item in packet['evidence_copies']:
        local = HERE/item['packet_path']
        require(local.is_file() and local.stat().st_size == item['bytes'] and
                sha(local) == item['sha256'], 'evidence copy mismatch: '+item['packet_path'])
    for case in (2, 3, 5):
        ref = json.loads((HERE/f'evidence/references/case{case:02}/EXTERNAL_REFERENCE_PRECOMMIT.json').read_text())
        require(ref['model_descriptor']['case_id'] == case and
                len(ref['reference_state']) == ref['model_descriptor']['model_contract']['dimension'] and
                ref['uncertainty'] >= 0, 'candidate full-vector reference contract')
    crosswalk = json.loads((HERE/'evidence/references/case01/CASE1_RAW_DESCRIPTOR_ROBERTSON_V2_CROSSWALK.json').read_text())
    require(crosswalk['status'] == 'DERIVED_BINDING_CROSSWALK_PENDING_INDEPENDENT_REVIEW',
            'case1 crosswalk claim status')
    triage = json.loads((HERE/'evidence/timeout/TIMEOUT_FAMILY_TRIAGE.json').read_text())
    require(triage['final_uninstrumented_timeouts'] == 199 and
            triage['time_typed_solver_failures_cases_5_12'] == 54,
            'copied timeout triage counts')
    representative_receipts = list((HERE/'evidence/failures').glob('*/RECEIPT.json'))
    require(len(representative_receipts) == 6, 'representative raw receipt count')
    for receipt in representative_receipts:
        row = json.loads(receipt.read_text())
        expected = 'TIMEOUT' if 'timeout' in receipt.parent.name else 'COMPLETED'
        require(row['execution_status'] == expected,
                'representative raw failure execution status')
    original = Path(compact['source_path'])
    original_available = original.is_file()
    if original_available:
        require(sha(original) == compact['source_sha256'] ==
                packet['legacy']['index_sha256'], 'original index SHA')
    cost_file = Path(packet['cost']['events_path'])
    cost_available = cost_file.is_file()
    if cost_available:
        require(sha(cost_file) == packet['cost']['events_sha256'] and
                len(cost_file.read_bytes().splitlines()) == 174,
                'local cost chain byte identity or count')
    require({r['case'] for r in packet['inventory_component_prose_defects']} == {21, 23},
            'case21/23 erratum')
    result = {'status': 'PASS_PACKET_HASH_AND_COUNT_CHECKS',
              'compact_rows': len(rows), 'references': 16, 'timeouts': 199,
              'typed_step_budget_failures': 54,
              'small_evidence_copies': len(packet['evidence_copies']),
              'original_index_bytes_available': original_available,
              'cost_events_bytes_available': cost_available,
              'external_raw_receipts': 'HASH_ONLY_NOT_REQUIRED_FOR_PACKET_VALIDATION'}
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        print(f'PACKET_VALIDATION_FAILED: {type(error).__name__}: {error}', file=sys.stderr)
        raise SystemExit(1)
