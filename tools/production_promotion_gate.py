#!/usr/bin/env python3
"""Read source-bound NCP receipts. Exit zero means analysis completed, not promotion.

This consumer cannot establish a continuous error theorem or controlled-host
timing from sampled work-precision rows. It reports those obligations explicitly.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import math
from pathlib import Path


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def finite(x):
    return type(x) in (int, float) and math.isfinite(x)


def safe_file(base, name, expected):
    if not isinstance(name, str) or not isinstance(expected, str):
        raise ValueError('missing receipt path/hash')
    p = (base / name).resolve()
    if not p.is_relative_to(base.resolve()) or not p.is_file():
        raise ValueError('receipt path missing or outside output directory')
    if sha(p) != expected:
        raise ValueError('receipt hash mismatch: ' + name)
    return p


def check_receipt(row, summary, base):
    path = safe_file(base, row.get('receipt_path'), row.get('receipt_sha256'))
    receipt = json.loads(path.read_text())
    if receipt.get('id') != row.get('id') or receipt.get('status') != 'PASS':
        raise ValueError('receipt id/status mismatch')
    if receipt.get('source') != summary.get('source') or receipt.get('source_after') != summary.get('source'):
        raise ValueError('source changed during command')
    if not summary.get('execution_identity_sha256') or receipt.get('execution_identity_sha256') != summary.get('execution_identity_sha256'):
        raise ValueError('execution identity mismatch')
    process = receipt.get('process', {})
    if process.get('exit_code') != 0 or process.get('timed_out') is not False:
        raise ValueError('command failed or timed out')
    artifacts = receipt.get('artifacts', {})
    if not artifacts:
        raise ValueError('empty command artifact receipt')
    for name, expected in artifacts.items():
        safe_file(base, name, expected)
    return receipt


def assess_wp(raw, corpus, corpus_sha, commit, binary_sha=None):
    reasons = []
    if raw.get('schema') != 'vigilode-bench.wp.v1':
        return {'status': 'HOLD', 'reasons': ['UNSUPPORTED_RAW_SCHEMA']}
    if raw.get('corpus') != corpus:
        reasons.append('CORPUS_CHANGED')
    provenance = raw.get('provenance', {})
    identity = provenance.get('source_identity', {})
    if provenance.get('git_commit') != commit:
        reasons.append('BUILD_COMMIT_MISMATCH')
    if identity.get('status') != 'VERIFIED_BUILD_SOURCE':
        reasons.append('UNVERIFIED_BUILD_SOURCE')
    if identity.get('corpus_sha256') != corpus_sha:
        reasons.append('CORPUS_HASH_MISMATCH')
    if binary_sha is not None and identity.get('binary_sha256') != binary_sha:
        reasons.append('BUILD_BINARY_HASH_MISMATCH')
    if identity.get('git_dirty') is not False or identity.get('dirty_source_allowed') is not False:
        reasons.append('DIRTY_SOURCE_NOT_PROMOTABLE')
    if set(identity.get('verified_boundaries', [])) != {'before_run', 'after_run'}:
        reasons.append('SOURCE_BOUNDARIES_NOT_VERIFIED')
    manifest = identity.get('manifest')
    if not isinstance(manifest, str) or hashlib.sha256(manifest.encode()).hexdigest() != identity.get('manifest_sha256'):
        reasons.append('SOURCE_MANIFEST_HASH_MISMATCH')
    protocol = corpus.get('reference', {}).get('independent_protocol', {})
    if protocol.get('tier') != 'radau5_cvode_refinement_v1':
        reasons.append('MISSING_STRICT_INDEPENDENT_REFERENCE_POLICY')
    refs = raw.get('references', [])
    problems = {p['id']: p for p in corpus.get('problems', [])}
    if len(refs) != len(problems) or {r.get('problem') for r in refs} != set(problems):
        reasons.append('REFERENCE_COVERAGE_MISMATCH')
    for ref in refs:
        receipt = ref.get('independent_protocol', {})
        if ref.get('disputed') is not False or receipt.get('status') != 'PASS':
            reasons.append('REFERENCE_DISPUTED_OR_UNVERIFIED')
        if receipt.get('tier') != protocol.get('tier'):
            reasons.append('REFERENCE_TIER_MISMATCH')
        for key in ('agreement_threshold', 'refinement_threshold'):
            value = receipt.get(key)
            if not finite(value) or not 0 < value <= 1e-10 or value != protocol.get(key):
                reasons.append('REFERENCE_THRESHOLD_MISMATCH')
        levels = receipt.get('levels', [])
        rtols, factors = protocol.get('rtols', []), protocol.get('atol_factors', [])
        problem = problems.get(ref.get('problem'), {})
        reference_atol = problem.get('reference_atol')
        if len(rtols) < 2 or len(factors) != len(rtols) or len(levels) != len(rtols) or not finite(reference_atol):
            reasons.append('REFERENCE_LADDER_COVERAGE')
            continue
        for index, (level, rtol, factor) in enumerate(zip(levels, rtols, factors)):
            if not finite(rtol) or not finite(factor) or rtol <= 0 or factor <= 0:
                reasons.append('REFERENCE_LADDER_POLICY')
                continue
            if index and (rtol >= rtols[index - 1] or factor >= factors[index - 1]):
                reasons.append('REFERENCE_LADDER_POLICY')
            if level.get('rtol') != rtol or level.get('atol') != reference_atol * factor:
                reasons.append('REFERENCE_LADDER_INPUT_MISMATCH')
            for arm in ('radau5', 'cvode'):
                if level.get(arm + '_status', {}).get('kind') != 'success' or level.get(arm + '_grid_points') != corpus.get('grid_points'):
                    reasons.append('REFERENCE_LEVEL_FAILED_OR_SHORT')
                previous = level.get(arm + '_previous_level_disagreement')
                if index and (not finite(previous) or not 0 <= previous <= protocol.get('refinement_threshold', 0)):
                    reasons.append('REFERENCE_REFINEMENT_FAILED')
            difference = level.get('cross_family_disagreement')
            if not finite(difference) or not 0 <= difference <= protocol.get('agreement_threshold', 0):
                reasons.append('REFERENCE_CROSSCHECK_FAILED')
        if (ref.get('arm') != 'radau5' or ref.get('rtol') != rtols[-1]
                or ref.get('atol') != reference_atol * factors[-1]
                or ref.get('status', {}).get('kind') != 'success'
                or len(ref.get('states', [])) != corpus.get('grid_points')):
            reasons.append('REFERENCE_FINEST_BASELINE_MISMATCH')
    required = set()
    for problem, item in problems.items():
        for arm in ('vigilode-dense', 'vigilode-banded-dense'):
            if arm in corpus.get('arms', []) and (arm != 'vigilode-banded-dense' or item.get('banded')):
                required.update((problem, arm, rtol) for rtol in corpus.get('rtols', []))
    if not required:
        reasons.append('EMPTY_CANDIDATE_SCOPE')
    seen = set()
    excess = []
    rows = raw.get('rows', [])
    for row in rows:
        key = (row.get('problem'), row.get('arm'), row.get('rtol'))
        if key not in required:
            continue
        if key in seen:
            reasons.append('DUPLICATE_CANDIDATE_ROW')
        seen.add(key)
        if row.get('status', {}).get('kind') != 'success':
            reasons.append('CANDIDATE_FAILED')
        if row.get('grid_points_completed') != corpus.get('grid_points'):
            reasons.append('INCOMPLETE_CANDIDATE_GRID')
        error = row.get('error_wrms_own')
        if not finite(error) or error < 0 or error > 1:
            excess.append({'problem': key[0], 'arm': key[1], 'rtol': key[2], 'error_wrms_own': error if finite(error) else None})
        reps = row.get('repetitions', [])
        if len(reps) != corpus.get('repetitions'):
            reasons.append('REPETITION_COUNT_MISMATCH')
        for rep in reps:
            if rep.get('status', {}).get('kind') != 'success' or rep.get('grid_points_completed') != corpus.get('grid_points') or not isinstance(rep.get('work'), dict):
                reasons.append('FAILED_OR_INCOMPLETE_REPETITION')
        if row.get('deterministic') is not True:
            reasons.append('NONDETERMINISTIC_COUNTED_WORK')
    if seen != required:
        reasons.append('CANDIDATE_COVERAGE_MISMATCH')
    if raw.get('protocol_violation_rows') != 0:
        reasons.append('PROTOCOL_VIOLATION')
    if excess:
        reasons.append('GLOBAL_GRID_ERROR_ABOVE_ORIGINAL_UNIT')
    return {'status': 'EMPIRICAL_GRID_PASS' if not reasons else 'HOLD',
            'reasons': sorted(set(reasons)), 'candidate_rows': len(seen),
            'required_candidate_rows': len(required), 'excess_error_rows': excess,
            'retained_failed_rows': sum(r.get('status', {}).get('kind') != 'success' for r in rows),
            'continuous_error_certificate': 'NOT_INFERRED_FROM_GRID',
            'speed_promotion': 'HOLD_REQUIRES_PREREGISTERED_PAIRED_FULL_COST_EVIDENCE',
            'timing_authority_observed': raw.get('timing_authority')}


def assess_comparison(raw):
    reasons = []
    suite = raw.get('suite')
    expected_scope = 'STAGE_FUNCTIONAL_ONLY' if suite == 'actual-k' else 'REQUESTED_ODE_ENDPOINTS_ONLY'
    expected_status = 'PASS_CERTIFIED_STAGE_FUNCTIONAL' if suite == 'actual-k' else 'PASS_CERTIFIED_ENDPOINTS'
    expected_budget = {'polynomial': 1e-7, 'homotopy': 2e-5, 'actual-k': 1e-9}.get(suite)
    if raw.get('schema') != 'vigilode.reaudit-comparison.v1' or expected_budget is None:
        reasons.append('UNSUPPORTED_COMPARISON_SCHEMA_OR_SUITE')
    if raw.get('scope') != expected_scope or raw.get('full_curve_certified') is not False:
        reasons.append('CERTIFICATE_SCOPE_MISMATCH')
    if raw.get('status') != expected_status:
        reasons.append('CANDIDATE_REFUSED')
    budget = raw.get('original_budget')
    bound = raw.get('achieved_bound')
    if budget != expected_budget or not finite(bound) or not 0 <= bound <= budget:
        reasons.append('ORIGINAL_BUDGET_NOT_SATISFIED')
    if not raw.get('outputs') or not isinstance(raw.get('work'), dict) or not raw['work']:
        reasons.append('MISSING_OUTPUT_OR_WORK_RECEIPT')
    if raw.get('solver_transaction_timed') is not True or not finite(raw.get('elapsed_ns')) or raw['elapsed_ns'] <= 0:
        reasons.append('MISSING_TRANSACTION_TIMING')
    return {'status': 'SCOPED_CERTIFICATE_PASS' if not reasons else 'HOLD',
            'reasons': sorted(set(reasons)), 'scope': expected_scope,
            'input_sha256': raw.get('input_sha256'), 'original_budget': budget,
            'fallback_used': raw.get('fallback_used'),
            'speed_promotion': 'HOLD_REQUIRES_PREREGISTERED_PAIRED_FULL_COST_EVIDENCE'}


def assess_bound_client(raw):
    contract = {
        'diagonal': (1e-7, 'REQUESTED_ODE_ENDPOINTS_ONLY', 'PASS_CERTIFIED_ENDPOINTS', False),
        'global': (1e-4, 'BOUND_MODEL_FULL_CURVE', 'PASS_CERTIFIED_FULL_CURVE', True),
        'allocation': (0.5, 'ALGEBRAIC_ALLOCATION_ONLY', 'PASS_VALIDATED_ALLOCATION', False),
    }.get(raw.get('case'))
    reasons = []
    if contract is None or raw.get('schema') != 'vigilode.reaudit-bound-clients.v1':
        return {'status': 'HOLD', 'reasons': ['UNSUPPORTED_BOUND_CLIENT']}
    budget, scope, status, full_curve = contract
    if raw.get('scope') != scope or raw.get('full_curve_certified') is not full_curve:
        reasons.append('CERTIFICATE_SCOPE_MISMATCH')
    if raw.get('status') != status:
        reasons.append('CANDIDATE_REFUSED')
    bound = raw.get('achieved_bound')
    if raw.get('original_budget') != budget or not finite(bound) or not 0 <= bound <= budget:
        reasons.append('ORIGINAL_BUDGET_NOT_SATISFIED')
    if not raw.get('work') or not raw.get('outputs'):
        reasons.append('MISSING_OUTPUT_OR_WORK_RECEIPT')
    return {'status': 'SCOPED_CERTIFICATE_PASS' if not reasons else 'HOLD',
            'reasons': reasons, 'scope': scope,
            'speed_promotion': 'HOLD_REQUIRES_PREREGISTERED_PAIRED_FULL_COST_EVIDENCE'}


def case_contracts(config, base):
    contracts = {}
    for case in config.get('comparison_cases', []):
        bound = case['suite'] == 'bound-clients'
        binary = base / 'targets/native/release/examples' / ('reaudit_bound_clients' if bound else 'reaudit_comparison')
        for threads in config['threads']:
            for rep in range(-config['warmups'], config['repetitions']):
                phase = f"warmup{rep + config['warmups']:02}" if rep < 0 else f"repeat{rep:02}"
                for arm in case['arms']:
                    name = f"{case['suite']}-{case['case']}-t{threads}-{phase}-{arm}"
                    argv = [str(binary), *([] if bound else ['--suite', case['suite']]),
                            '--case', case['case'], '--arm', arm, '--threads', str(threads)]
                    contracts[name] = (case, arm, threads, argv)
    return contracts


def evaluate(summary, base, corpus, corpus_sha, expected_commit, expected_config=None):
    reasons = []
    if summary.get('schema') != 'vigilode.ncp-receipt.v1':
        reasons.append('UNSUPPORTED_SUMMARY_SCHEMA')
    source = summary.get('source', {})
    if source.get('commit') != expected_commit or source.get('clean') is not True:
        reasons.append('SOURCE_IDENTITY_OR_CLEANLINESS')
    for key in ('tree', 'closure_sha256', 'closure_policy_sha256'):
        if not isinstance(source.get(key), str) or len(source[key]) != (40 if key == 'tree' else 64):
            reasons.append('MISSING_SOURCE_' + key.upper())
    planned_steps, planned_cases, config = {}, [], {}
    try:
        from run_ncp_reaudit import canonical, planned_case_ids, steps
        plan = json.loads(safe_file(base, summary.get('plan_path'), summary.get('plan_sha256')).read_text())
        config = plan['config']
        if expected_config is None or config != expected_config:
            raise ValueError('plan differs from the independently supplied committed config')
        locked_corpus = config.get('locked_inputs', {}).get(config.get('corpus'))
        if not isinstance(locked_corpus, str) or corpus_sha != locked_corpus:
            raise ValueError('supplied corpus hash differs from frozen config lock')
        if hashlib.sha256(canonical(plan['identity'])).hexdigest() != summary.get('execution_identity_sha256'):
            raise ValueError('plan execution identity hash mismatch')
        if plan['identity'].get('source') != source or plan['identity'].get('config_sha256') != hashlib.sha256(canonical(config)).hexdigest():
            raise ValueError('plan source/config mismatch')
        canonical_steps = steps(config)
        if plan['steps'] != [list(s) for s in canonical_steps]:
            raise ValueError('validation plan commands changed')
        planned_steps = {s[0]: s[1] for s in canonical_steps}
        planned_cases = planned_case_ids(config)
    except (OSError, ValueError, KeyError, TypeError) as exc:
        reasons.append('INVALID_FROZEN_PLAN:' + str(exc))
    validation = summary.get('validation', {})
    if validation.get('all_passed') is not True or not validation.get('required_steps'):
        reasons.append('INCOMPLETE_NATIVE_VALIDATION')
    validations = validation.get('receipts', [])
    validation_ids = [r.get('id') for r in validations]
    if (len(set(validation_ids)) != len(validation_ids) or set(validation_ids) != set(validation.get('required_steps', []))
            or set(validation_ids) != set(planned_steps)):
        reasons.append('VALIDATION_COVERAGE_MISMATCH')
    build_artifacts, build_libraries = {}, {}
    for row in validations:
        try:
            receipt = check_receipt(row, summary, base)
            if receipt.get('argv') != planned_steps.get(row.get('id')):
                raise ValueError('validation command differs from frozen plan')
            if row.get('id') in ('comparison_build', 'bench_build'):
                build_artifacts.update(receipt.get('artifacts', {}))
                build_libraries.update(receipt.get('runtime_libraries', {}))
        except (OSError, ValueError, TypeError) as exc:
            reasons.append('INVALID_VALIDATION_RECEIPT:' + str(exc))
    required = summary.get('benchmarks', {}).get('required_cases', [])
    cases = summary.get('benchmarks', {}).get('cases', [])
    if not required or len(set(required)) != len(required) or required != planned_cases:
        reasons.append('EMPTY_OR_DUPLICATE_CAMPAIGN')
    ids = [c.get('id') for c in cases]
    if len(ids) != len(set(ids)) or set(ids) != set(required):
        reasons.append('CAMPAIGN_COVERAGE_MISMATCH')
    results = []
    comparison_inputs = {}
    contracts = case_contracts(config, base)
    for case in cases:
        try:
            receipt = check_receipt(case, summary, base)
            if case.get('status') != 'EXECUTED' or case.get('issues'):
                raise ValueError('benchmark has retained holds')
            from run_ncp_reaudit import memory_issues
            if memory_issues(receipt, config):
                raise ValueError('benchmark resource evidence is unavailable or failed')
            if receipt.get('artifacts', {}).get(case.get('raw_path')) != case.get('raw_sha256'):
                raise ValueError('raw output is not bound to its own execution receipt')
            binary_binding = receipt.get('executable_binding') or {}
            binary_path = binary_binding.get('path')
            if (not binary_path or build_artifacts.get(binary_path) != binary_binding.get('sha256')
                    or not binary_binding.get('runtime_libraries')
                    or build_libraries.get(binary_path) != binary_binding.get('runtime_libraries')
                    or receipt.get('executable_unchanged_after') is not True):
                raise ValueError('executable/runtime differs from source-bound build')
            raw_path = safe_file(base, case.get('raw_path'), case.get('raw_sha256'))
            raw = json.loads(raw_path.read_text())
            if case.get('id') != 'work_precision':
                from run_ncp_reaudit import numeric_row_issues
                expected_case, arm, threads, argv = contracts[case['id']]
                expected_kind = 'bound_client' if expected_case['suite'] == 'bound-clients' else 'comparison'
                if (case.get('kind') != expected_kind or receipt.get('argv') != argv
                        or Path(argv[0]).resolve() != (base / binary_path).resolve()):
                    raise ValueError('benchmark arm command or executable binding differs from frozen case')
                # A numerical refusal is legitimate scientific data; identity,
                # budget and scope mismatches are receipt-integrity failures.
                row_issues = numeric_row_issues(raw, expected_case, arm, threads)
                if any(x.startswith('ROW_MISMATCH:') and x != 'ROW_MISMATCH:status' or x in ('ROW_SCHEMA', 'FROZEN_BUDGET_CHANGED', 'INPUT_IDENTITY_INVALID', 'FULL_CURVE_FLAG_MISMATCH') for x in row_issues):
                    raise ValueError('raw case/arm/thread/input differs from frozen case')
                result = assess_bound_client(raw) if expected_kind == 'bound_client' else assess_comparison(raw)
                key = (expected_case['suite'], raw.get('case'))
                binding = (raw.get('input_sha256'), raw.get('original_budget'), raw.get('scope'))
                if not isinstance(binding[0], str) or len(binding[0]) != 64:
                    reasons.append('MISSING_COMPARISON_INPUT_HASH')
                if key in comparison_inputs and comparison_inputs[key] != binding:
                    reasons.append('COMPARISON_REQUEST_REBOUND_ACROSS_ARMS')
                comparison_inputs[key] = binding
                if not finite(case.get('process_elapsed_ns')) or case['process_elapsed_ns'] <= 0:
                    reasons.append('MISSING_PROCESS_END_TO_END_TIMING')
            else:
                argv = receipt.get('argv', [])
                if (case.get('kind') != 'work_precision' or len(argv) != 9 or argv[1:3] != ['wp', '--corpus']
                        or argv[4] != '--out' or argv[6:8] != ['--require-independent-reference', '--source-root']
                        or Path(argv[0]).resolve() != (base / binary_path).resolve()
                        or Path(argv[5]).resolve() != raw_path
                        or not argv[3].endswith(config['corpus'])):
                    raise ValueError('work-precision command differs from frozen policy')
                result = assess_wp(raw, corpus, corpus_sha, expected_commit, binary_binding['sha256'])
        except (OSError, ValueError, KeyError, TypeError) as exc:
            result = {'status': 'HOLD', 'reasons': ['INVALID_RAW_RECEIPT'], 'detail': str(exc)}
            reasons.append('INVALID_RAW_RECEIPT:' + str(case.get('id')))
        results.append({'id': case.get('id'), **result})
    return {'schema': 'vigilode.ncp-gate.v1', 'source_commit': expected_commit,
            'receipt_integrity': 'PASS' if not reasons else 'HOLD', 'reasons': sorted(set(reasons)),
            'cases': results, 'broad_production': 'HOLD',
            'scope': 'Mechanical source/coverage/refinement/grid audit. No automatic router or release change.',
            'next_action': 'Review the preserved case-level failures; do not change original budgets or replay unchanged published campaigns.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--summary', type=Path, required=True)
    parser.add_argument('--corpus', type=Path, required=True)
    parser.add_argument('--expected-commit', required=True)
    parser.add_argument('--config', type=Path, default=Path(__file__).resolve().parents[1] / 'research/production_reaudit_20261021/NCP_CONFIG.json')
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    result = evaluate(json.loads(args.summary.read_text()), args.summary.resolve().parent,
                      json.loads(args.corpus.read_text()), sha(args.corpus), args.expected_commit,
                      json.loads(args.config.read_text()))
    args.out.write_text(json.dumps(result, ensure_ascii=False, indent=2, allow_nan=False) + '\n')
    print(json.dumps({'output': str(args.out), 'receipt_integrity': result['receipt_integrity'],
                      'broad_production': result['broad_production']}))


if __name__ == '__main__':
    main()
