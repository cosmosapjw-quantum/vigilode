#!/usr/bin/env python3
"""Source-pinned conditional kernel experiments, separate from whole-ODE ranking."""
from __future__ import annotations
import argparse
from collections import Counter, defaultdict
import hashlib
import json
import math
import platform
from pathlib import Path
import statistics
import sys
import time

from campaign import (REPO, canonical, checked_json_lines, digest, json_new,
                      run_process, source_identity, topology, utc,
                      validate_build_receipt, verify_receipt)

ROW_KINDS = ('result', 'sample', 'measurement', 'warmup', 'guard', 'negative_control', 'checks')

REGISTRY = {
    'conditional_chart_sweep': {'package': 'rodas5p-integrators', 'args': ['--repeats', '1', '--dimension', '32'], 'header_required': True, 'scope': 'conditioned linear-chart solves and original-space certificates'},
    'conditional_continuation_sweep': {'package': 'rodas5p-integrators', 'args': ['--repeats', '1'], 'header_required': True, 'scope': 'fixed-P nonlinear root solves and branch/admission controls'},
    'structured_residual_sweep': {'package': 'rodas5p-core', 'args': [], 'header_required': False, 'scope': 'structured directed original-space residual certificates, not whole ODE'},
    'invariant_module_bench': {'package': 'rodas5p-core', 'args': [], 'header_required': False, 'scope': 'finite invariant-module linear kernels with original-space certificates'},
}


def finite_number(value):
    return isinstance(value, (float, int)) and not isinstance(value, bool) and math.isfinite(value)


def template(target, receipts):
    return {'schema_version': 1, 'experiments': [
        {'example': name, 'binary': str(Path(target).resolve() / 'measurement/examples' / name),
         'build_receipt': str(Path(receipts).resolve() / ('build-' + name) / 'BUILD_RECEIPT.json'),
         'args': entry['args']}
        for name, entry in REGISTRY.items()]}


def validate_entry(entry, source):
    name = entry.get('example')
    if name not in REGISTRY:
        raise ValueError('unregistered conditional example: ' + str(name))
    if not isinstance(entry.get('args', []), list) or any(not isinstance(arg, str) for arg in entry.get('args', [])):
        raise ValueError('experiment args must be a list of literal strings')
    binary, path = Path(entry['binary']).resolve(), Path(entry['build_receipt']).resolve()
    receipt = json.loads(path.read_text())
    validate_build_receipt(receipt, source, binary)
    command = receipt.get('command', [])
    def argument(flag):
        return command[command.index(flag) + 1] if flag in command and command.index(flag) + 1 < len(command) else None
    if not command or Path(command[0]).name != 'cargo' or 'build' not in command or '--locked' not in command or argument('--profile') != 'measurement' or argument('--example') != name or argument('-p') != REGISTRY[name]['package']:
        raise ValueError('build command does not identify the registered package/example')
    if receipt.get('source_after') != source or receipt.get('source_changed_during_build') is not False or receipt.get('cargo_exit_code') != 0:
        raise ValueError('build receipt lacks stable successful source-after verification')
    if receipt.get('cargo_lock_sha256') != digest(REPO / 'Cargo.lock'):
        raise ValueError('Cargo.lock differs from the recorded build')
    if Path(receipt.get('binary', '')).resolve() != binary:
        raise ValueError('build receipt binary path differs from requested binary')
    return {'example': name, 'binary': str(binary), 'binary_sha256': digest(binary),
            'build_receipt': str(path), 'build_receipt_sha256': digest(path),
            'args': entry.get('args', []), 'scope': REGISTRY[name]['scope']}


def parse_rows(path, example):
    rows, errors = checked_json_lines(path)
    if any(not isinstance(row, dict) for row in rows):
        errors.append({'error': 'every JSONL row must be an object'})
    objects = [row for row in rows if isinstance(row, dict)]
    samples = [row for row in objects if row.get('kind') in ROW_KINDS]
    if not samples:
        errors.append({'error': 'no conditional measurements or controls emitted'})
    if REGISTRY[example]['header_required'] and not any(row.get('kind') == 'header' for row in objects):
        errors.append({'error': 'missing example header'})
    return rows, errors


def flatten_fields(value, predicate, path=()):
    fields = {}
    if isinstance(value, dict):
        for key, item in value.items():
            fields.update(flatten_fields(item, predicate, (*path, key)))
    elif path and predicate(path, value):
        fields['.'.join(path)] = value
    return fields


def scenario_identity(row):
    # Unknown fields are retained conservatively: a new scenario parameter must
    # never silently merge distinct cases. Explicit telemetry alone is removed.
    observed = {'repeat', 'repetition', 'timing_iterations', 'raw', 'conditional',
                'bisection_final_only', 'work', 'chart_work', 'setup_work',
                'corrections', 'fallback_solves', 'final_original_interval_checks',
                'final_residual_scalar_products', 'actual_error_upper', 'actual_error',
                'original_residual_upper', 'certified_error_upper', 'error_l2_upper',
                'actual_error_lower', 'true_error_l2', 'residual_operations', 'fallback_count', 'certified', 'last_chart_error_upper', 'last_chart_residual_upper',
                'last_chart_reported_residual', 'last_chart_conditional_error_upper',
                'conditional_over_raw_time', 'bisection_over_conditional_time',
                'observed_ratio_to_calibration_direct', 'empirical_pass_cost_proxy',
                'refusal_reason', 'accepted', 'chart_accepted', 'final_accepted',
                'status', 'scientific_status', 'speed_comparison_eligible'}
    return {k: v for k, v in row.items() if k not in observed and 'seconds' not in k}


def summarize_rows(receipts):
    """Report raw statuses/costs; no promotion decision or whole-ODE speed ranking."""
    buckets = defaultdict(list)
    statuses = Counter()
    for receipt in receipts:
        example = receipt['task']['example']
        repeat = receipt['task']['process_repeat']
        for row in receipt.get('conditional_rows', []):
            if not isinstance(row, dict) or row.get('kind') not in ROW_KINDS:
                continue
            status = row.get('scientific_status', row.get('status'))
            if status is None:
                status = 'ACCEPTED' if row.get('accepted', row.get('certified')) is True else 'REFUSED' if row.get('accepted', row.get('certified')) is False else 'BUNDLED_METHOD_RESULTS' if any(k in row for k in ('raw', 'conditional')) else 'UNCLASSIFIED'
            statuses[str(status)] += 1
            nested_statuses = flatten_fields(row, lambda path, value: path[-1] in ('status', 'scientific_status', 'success', 'accepted', 'guard_pass'))
            for key, value in nested_statuses.items():
                if '.' in key:
                    statuses[key + ':' + str(value)] += 1
            key = {'example': example, 'case': row.get('case', row.get('case_id', row.get('family', row.get('fixture')))),
                   'method': row.get('method', row.get('mode', 'bundled-method-results')),
                   'scenario': scenario_identity(row), 'row_kind': row.get('kind'), 'status': str(status)}
            timings = flatten_fields(row, lambda path, value: any('seconds' in part for part in path) and finite_number(value))
            errors = flatten_fields(row, lambda path, value: any('error' in part or 'residual' in part for part in path))
            buckets[canonical(key)].append({'process_repeat': repeat, 'execution_status': receipt['execution_status'],
                                           'timings': timings, 'error_and_residual_fields': errors,
                                           'negative_control': row.get('negative_control', row.get('expected_refusal', row.get('kind') == 'negative_control')),
                                           'refusal_reason': row.get('refusal_reason'),
                                           'method_statuses': nested_statuses,
                                           'accepted': row.get('accepted', row.get('certified', row.get('final_accepted', row.get('chart_accepted'))))})
    groups = []
    for encoded, observations in sorted(buckets.items()):
        # Collapse internal iterations per process first. They are not independent process repetitions.
        process_values = defaultdict(lambda: defaultdict(list))
        for o in observations:
            for field, value in o['timings'].items():
                process_values[o['process_repeat']][field].append(value)
        per_process = [{'process_repeat': r, 'median_timings': {field: statistics.median(values) for field, values in fields.items()}}
                       for r, fields in sorted(process_values.items())]
        groups.append({'identity': json.loads(encoded), 'raw_observation_count': len(observations),
                       'independent_process_count': len(set(o['process_repeat'] for o in observations)),
                       'per_process_median_timings': per_process, 'observations': observations})
    return {'raw_status_counts': dict(statuses), 'groups': groups,
            'whole_ode_ranking_eligible': False, 'automatic_dispatch_enabled': False,
            'interpretation': 'Kernel/root experiments only. Keep original-space errors, refused cases, all setup/certificate/fallback costs, and process failures. Internal iterations are collapsed within each independent process; no speedup claim is generated.'}


def verify_conditional_receipt(path, source, entry, task):
    receipt = verify_receipt(path, source, entry['binary_sha256'], task)
    rows, errors = parse_rows(path.parent / 'stdout.jsonl', entry['example'])
    if receipt.get('conditional_rows') != rows or receipt.get('conditional_parse_errors') != errors:
        raise ValueError('stored conditional rows differ from the immutable raw JSONL')
    if receipt.get('build_receipt_sha256') != entry['build_receipt_sha256']:
        raise ValueError('conditional receipt build linkage mismatch')
    return receipt


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--write-template', type=Path)
    p.add_argument('--target-dir', type=Path)
    p.add_argument('--receipts-dir', type=Path)
    p.add_argument('--manifest', type=Path)
    p.add_argument('--expected-commit')
    p.add_argument('--expected-tree')
    p.add_argument('--output', type=Path)
    p.add_argument('--repeats', type=int, default=3, help='separate processes, not inner loop iterations')
    p.add_argument('--timeout', type=float, default=120)
    p.add_argument('--budget', type=float, default=900, help='cumulative seconds across resumptions')
    p.add_argument('--memory-gib', type=float, default=16)
    a = p.parse_args(argv)
    if a.write_template:
        if a.target_dir is None or a.receipts_dir is None:
            raise ValueError('--write-template requires --target-dir and --receipts-dir')
        json_new(a.write_template, template(a.target_dir, a.receipts_dir))
        print(canonical({'status': 'TEMPLATE_WRITTEN', 'path': str(a.write_template)}))
        return 0
    if not all([a.manifest, a.expected_commit, a.expected_tree, a.output]):
        raise ValueError('execution requires --manifest, --expected-commit, --expected-tree and --output')
    if not 1 <= a.repeats <= 9 or any(not math.isfinite(v) or v <= 0 for v in (a.timeout, a.budget, a.memory_gib)):
        raise ValueError('invalid finite positive resource limits or repeats outside1..9')
    output = a.output.resolve()
    if output == REPO.resolve() or REPO.resolve() in output.parents:
        raise ValueError('place campaign output outside the clean source checkout')
    source = source_identity(REPO, a.expected_commit, a.expected_tree)
    data = json.loads(a.manifest.read_text())
    entries = [validate_entry(e, source) for e in data['experiments']]
    if not entries or len({e['example'] for e in entries}) != len(entries):
        raise ValueError('choose a nonempty set with one task per registered example')
    host_topology = topology()
    cpu = host_topology['physical_cpu_selection'][:1]
    host_identity = {'hostname': platform.node(), 'platform': platform.platform(),
                     'physical_cpu_selection': host_topology['physical_cpu_selection'],
                     'allowed_logical_cpus': host_topology['allowed_logical_cpus']}
    if not cpu:
        raise ValueError('no admitted CPU')
    config = {'schema_version': 1, 'source': source, 'experiments': entries, 'manifest_sha256': digest(a.manifest),
              'repeats': a.repeats, 'timeout_seconds': a.timeout, 'cumulative_budget_seconds': a.budget,
              'memory_gib': a.memory_gib, 'cpus': cpu, 'host_identity': host_identity, 'ncp_used': False, 'threads': 1,
              'parallel_layer': 'serial-conditional-kernel', 'whole_ode_ranking_eligible': False}
    output.mkdir(parents=True, exist_ok=True)
    marker = output / 'IDENTITY.json'
    if marker.exists():
        if json.loads(marker.read_text()) != config:
            raise ValueError('output belongs to a different immutable campaign configuration')
    else:
        json_new(marker, config)
    prior = sum(json.loads(path.read_text()).get('elapsed_process_seconds', 0) for path in output.glob('runs/*/RECEIPT.json'))
    prior += sum(json.loads(path.read_text()).get('timeout_seconds', 0) for path in output.glob('runs/*/START.json') if not (path.parent / 'RECEIPT.json').exists())
    started = time.monotonic()
    receipts, pending = [], []
    source_changed = False
    for repeat in range(a.repeats):
        for entry in entries:
            task = {'example': entry['example'], 'process_repeat': repeat, 'parallel_layer': 'serial-conditional-kernel', 'threads': 1}
            folder = output / 'runs' / (entry['example'] + '_' + str(repeat))
            if (folder / 'RECEIPT.json').exists():
                receipts.append(verify_conditional_receipt(folder / 'RECEIPT.json', source, entry, task))
                continue
            if folder.exists():
                pending.append({**task, 'reason': 'interrupted attempt preserved; use a new campaign output to retry'})
                continue
            remaining = a.budget - prior - (time.monotonic() - started)
            if source_changed or remaining <= 0:
                pending.append({**task, 'reason': 'source changed' if source_changed else 'cumulative budget exhausted'})
                continue
            if source_identity(REPO, a.expected_commit, a.expected_tree) != source or digest(entry['binary']) != entry['binary_sha256'] or digest(entry['build_receipt']) != entry['build_receipt_sha256']:
                raise ValueError('source, binary or build receipt changed after campaign planning')
            environment = {'RAYON_NUM_THREADS': '1', 'OMP_NUM_THREADS': '1', 'OPENBLAS_NUM_THREADS': '1', 'MKL_NUM_THREADS': '1', 'VIGILODE_SOURCE_COMMIT': source['commit']}
            receipt = run_process([entry['binary'], *entry['args']], folder, min(a.timeout, remaining), a.memory_gib, cpu, environment, expect_result=False)
            rows, errors = parse_rows(folder / 'stdout.jsonl', entry['example'])
            after = source_identity(REPO, allow_dirty=True)
            source_changed = after != source
            binary_changed = digest(entry['binary']) != entry['binary_sha256']
            receipt.update(task=task, source=source, source_after=after, source_changed_during_run=source_changed,
                           binary_sha256=entry['binary_sha256'], binary_changed_during_run=binary_changed,
                           build_receipt_sha256=entry['build_receipt_sha256'], conditional_rows=rows, conditional_parse_errors=errors,
                           evidence_scope=entry['scope'], whole_ode_ranking_eligible=False)
            if source_changed or binary_changed:
                receipt['execution_status'] = 'IDENTITY_CHANGED_DURING_RUN'
                source_changed = True
            elif errors and receipt['execution_status'] == 'COMPLETED':
                receipt['execution_status'] = 'MALFORMED_CONDITIONAL_OUTPUT'
            json_new(folder / 'RECEIPT.json', receipt)
            receipts.append(receipt)
            print(canonical({'task': task, 'status': receipt['execution_status'], 'rows': len(rows)}), flush=True)
    summary = {'schema_version': 1, 'finished_utc': utc(), 'configuration': config, 'receipts': receipts, 'pending': pending,
               'summary': summarize_rows(receipts), 'process_statuses': dict(Counter(r['execution_status'] for r in receipts)),
               'execution_complete': len(receipts) == len(entries) * a.repeats and not pending}
    json_new(output / ('SUMMARY_' + str(time.time_ns()) + '.json'), summary)
    return 0 if all(r['execution_status'] == 'COMPLETED' for r in receipts) and not pending else 2


if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError) as e:
        print(canonical({'status': 'BLOCKED', 'error': str(e)}), file=sys.stderr)
        raise SystemExit(2)
