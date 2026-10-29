#!/usr/bin/env python3
"""Source-bound, same-binary structural-proposal experiment. No build is performed."""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import math
import os
from pathlib import Path
import resource
import signal
import statistics
import subprocess
import sys
import time


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path: Path, value) -> None:
    temporary = path.with_suffix(path.suffix + '.tmp')
    temporary.write_text(json.dumps(value, indent=2, allow_nan=False) + '\n')
    temporary.replace(path)


def load_json(path: Path):
    def reject(value):
        raise ValueError(f'non-finite JSON token: {value}')
    return json.loads(path.read_text(), parse_constant=reject)


def finite_number(value, label: str, nonnegative: bool = False) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
        raise ValueError(f'{label} must be finite numeric')
    if nonnegative and value < 0:
        raise ValueError(f'{label} must be nonnegative')
    return value


def nonnegative_integer(value, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise ValueError(f'{label} must be a nonnegative integer')
    return value


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--protocol', type=Path, required=True)
    parser.add_argument('--source-sha', required=True)
    parser.add_argument('--remote-ref', required=True)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--binding', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--prior-run', type=Path, action='append', default=[])
    parser.add_argument('--phase', choices=('checks', 'bench', 'all'), default='all')
    args = parser.parse_args()
    repo = args.repo.resolve()
    protocol_path = args.protocol.resolve()
    binary = args.binary.resolve()
    binding_path = args.binding.resolve()
    out = args.output.resolve()

    def git(*words) -> str:
        return subprocess.check_output(['git', *words], cwd=repo, text=True).strip()

    if git('rev-parse', 'HEAD') != args.source_sha or git('rev-parse', args.remote_ref) != args.source_sha:
        raise ValueError('HEAD and fetched remote ref must equal the published source')
    protocol = load_json(protocol_path)
    runner = Path(__file__).resolve()
    tracked = [runner.relative_to(repo), protocol_path.relative_to(repo)]
    for relative in tracked:
        committed = subprocess.check_output(['git', 'show', f'{args.source_sha}:{relative.as_posix()}'], cwd=repo)
        if committed != (repo / relative).read_bytes():
            raise ValueError(f'runner/protocol bytes are not the published bytes: {relative}')
    subprocess.run(['git', 'diff', '--exit-code', 'HEAD', '--', 'crates', 'Cargo.lock', 'Cargo.toml', *map(str, tracked)], cwd=repo, check=True)
    if protocol['scientific_wall_cap_seconds'] != 900 or protocol['inherited_phase1_wall_seconds'] != 491.53919015300016:
        raise ValueError('registered cumulative budget changed')
    inherited_path = repo / protocol['inherited_results_path']
    inherited = load_json(inherited_path)
    if inherited['aggregate_campaign_wall_seconds'] != protocol['inherited_phase1_wall_seconds']:
        raise ValueError('inherited phase1 charge does not match source-bound results')
    inherited_blob = subprocess.check_output(['git', 'show', f'{args.source_sha}:{protocol["inherited_results_path"]}'], cwd=repo)
    if inherited_blob != inherited_path.read_bytes():
        raise ValueError('inherited results are not source-bound')

    prior = []
    seen = set()
    for item in args.prior_run:
        path = item.resolve()
        if path in seen:
            raise ValueError('duplicate structural prior run')
        seen.add(path)
        data = load_json(path)
        if data['schema'] != 'vigilode.structural-quadratic-run.v1':
            raise ValueError('only structural phase receipts belong in --prior-run; phase1 is charged once by protocol')
        receipt_digest = sha256(path)
        finite_number(data['wall_seconds'], 'prior run wall', True)
        same_source = data['source'] == args.source_sha
        if not same_source:
            matches = [record for record in protocol.get('registered_prior_source_runs', [])
                       if record['source'] == data['source']
                       and record['receipt_sha256'] == receipt_digest
                       and record['wall_seconds'] == data['wall_seconds']
                       and record['checks_result'] == data.get('checks_result')]
            if len(matches) != 1:
                raise ValueError('prior source receipt not exactly preregistered')
            subprocess.run(['git', 'merge-base', '--is-ancestor', data['source'], args.source_sha], cwd=repo, check=True)
        prior.append({'path': str(path), 'sha256': receipt_digest, 'wall_seconds': data['wall_seconds'],
                      'source': data['source'], 'same_source': same_source, 'checks_result': data.get('checks_result')})
    for record in protocol.get('registered_prior_source_runs', []):
        if sum(row['sha256'] == record['receipt_sha256'] for row in prior) != 1:
            raise ValueError('registered prior source cost must be charged exactly once')
    used_before = protocol['inherited_phase1_wall_seconds'] + sum(row['wall_seconds'] for row in prior)
    if used_before >= protocol['scientific_wall_cap_seconds']:
        raise ValueError('cumulative campaign wall budget exhausted')
    if args.phase == 'bench' and not any(row['same_source'] and row['checks_result'] == 'pass_scoped' for row in prior):
        raise ValueError('bench-only needs a source-matched passing structural checks receipt')

    binding_packet = load_json(binding_path)
    binding = binding_packet.get('structural_quadratic_probe', binding_packet)
    if binding['source'] != args.source_sha or binding['binary_sha256'] != sha256(binary):
        raise ValueError('candidate source or binary digest does not match actual build binding')
    if not git('branch', '-r', '--contains', binding['source']):
        raise ValueError('binary source is not remotely reachable')
    build_receipt = Path(binding['build_receipt'])
    if not build_receipt.is_file() or sha256(build_receipt) != binding['build_receipt_sha256']:
        raise ValueError('build receipt unavailable or digest mismatch')
    build = load_json(build_receipt)
    if build['exit_code'] != 0 or not build['source_file_hashes_unchanged'] or build['source'] != args.source_sha:
        raise ValueError('build receipt does not establish successful unchanged-source compilation')

    out.mkdir(parents=True, exist_ok=False)
    env = os.environ | {'CARGO_BUILD_JOBS': '2', 'RAYON_NUM_THREADS': '1', 'OMP_NUM_THREADS': '1', 'OPENBLAS_NUM_THREADS': '1', 'PYTHONDONTWRITEBYTECODE': '1', 'VIGILODE_SOURCE_COMMIT': args.source_sha}
    started = time.monotonic()
    state = {'schema': 'vigilode.structural-quadratic-run.v1', 'source': args.source_sha,
             'tree': git('rev-parse', 'HEAD^{tree}'), 'remote_ref': args.remote_ref,
             'phase': args.phase, 'started_utc': dt.datetime.now(dt.timezone.utc).isoformat(),
             'protocol_sha256': sha256(protocol_path), 'runner_sha256': sha256(runner),
             'binary_sha256': sha256(binary), 'build_binding': binding,
             'binding_file_sha256': sha256(binding_path), 'cargo_lock_sha256': sha256(repo / 'Cargo.lock'),
             'inherited_results_sha256': sha256(inherited_path), 'inherited_phase1_wall_seconds': protocol['inherited_phase1_wall_seconds'],
             'prior_structural_runs': prior, 'used_wall_seconds_before_this_run': used_before,
             'scientific_wall_cap_seconds': protocol['scientific_wall_cap_seconds'],
             'affinity': sorted(os.sched_getaffinity(0)), 'commands': [], 'execution': 'running',
             'checks_result': 'not_run', 'benchmark_result': 'not_run', 'production': 'HOLD',
             'comparison_scope': 'Same binary and fixed binary64 controls; proposal and ordinary GMRES paths. Accuracy is an analytic binary64 diagnostic on every one of 17 returned grid points, not a whole-trajectory enclosure.'}
    trials = []
    pair_rows = []
    case_rows = {}
    fixture_metadata = {}
    def interrupted(signum, frame):
        raise InterruptedError(f'runner received signal {signum}')
    signal.signal(signal.SIGTERM, interrupted)

    def source_hashes():
        paths = [repo / 'Cargo.lock', repo / 'Cargo.toml', runner, protocol_path]
        paths += sorted((repo / 'crates').rglob('*.rs'))
        paths += sorted((repo / 'crates').rglob('Cargo.toml'))
        return {str(path.relative_to(repo)): sha256(path) for path in paths}

    before_hashes = source_hashes()
    write_json(out / 'SOURCE_HASHES_BEFORE.json', before_hashes)

    def checkpoint():
        state['wall_seconds'] = time.monotonic() - started
        state['aggregate_campaign_wall_seconds'] = used_before + state['wall_seconds']
        state['remaining_campaign_wall_seconds'] = max(0, protocol['scientific_wall_cap_seconds'] - state['aggregate_campaign_wall_seconds'])
        write_json(out / 'RUN.json', state)
        write_json(out / 'TRIALS.json', trials)
        write_json(out / 'PAIRED_TIMINGS.json', pair_rows)
        write_json(out / 'CASE_SUMMARY.json', case_rows)

    def run(label, argv, command_cap):
        if git('rev-parse', 'HEAD') != args.source_sha:
            raise RuntimeError('HEAD changed during immutable-source execution')
        if sha256(binary) != state['binary_sha256']:
            raise RuntimeError('executable bytes changed during execution')
        remaining = protocol['scientific_wall_cap_seconds'] - used_before - (time.monotonic() - started)
        if remaining <= 0:
            raise TimeoutError('aggregate scientific campaign wall cap exhausted')
        target = out / label
        target.mkdir()
        start = time.monotonic()
        cpu_before = resource.getrusage(resource.RUSAGE_CHILDREN)
        timeout = False
        interruption = None
        with (target / 'stdout.txt').open('wb') as stdout, (target / 'stderr.txt').open('wb') as stderr:
            process = subprocess.Popen(list(map(str, argv)), cwd=repo, env=env, stdout=stdout, stderr=stderr, start_new_session=True)
            try:
                code = process.wait(timeout=min(command_cap, remaining))
            except subprocess.TimeoutExpired:
                timeout = True
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                code = process.wait()
            except BaseException as exc:
                interruption = exc
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                code = process.wait()
        cpu_after = resource.getrusage(resource.RUSAGE_CHILDREN)
        row = {'label': label, 'argv': list(map(str, argv)), 'exit_code': code, 'timed_out': timeout,
               'wall_seconds': time.monotonic() - start,
               'child_cpu_seconds': cpu_after.ru_utime + cpu_after.ru_stime - cpu_before.ru_utime - cpu_before.ru_stime,
               'cumulative_child_maxrss_kib': cpu_after.ru_maxrss,
               'stdout_sha256': sha256(target / 'stdout.txt'), 'stderr_sha256': sha256(target / 'stderr.txt')}
        write_json(target / 'RECEIPT.json', row)
        state['commands'].append(row)
        checkpoint()
        print(json.dumps({'label': label, 'exit_code': code, 'wall_seconds': row['wall_seconds']}), flush=True)
        if interruption is not None:
            raise interruption
        return row, target

    def validate(payload, case, arm, start, mode, repeats):
        for key, value in [('case', case['case']), ('dimension', case['dimension']), ('arm', arm), ('start', start), ('mode', mode), ('repeats', repeats), ('span', protocol['span'])]:
            if payload.get(key) != value:
                raise ValueError(f'fixture metadata mismatch: {key}')
        if payload.get('output_grid') != protocol['output_grid']:
            raise ValueError('returned-grid request metadata mismatch')
        if payload.get('physical_error_cap') != protocol['max_abs_error_cap']:
            raise ValueError('declared physical diagnostic cap differs')
        if payload.get('solve_log_enabled') != (mode == 'probe'):
            raise ValueError('solve-log mode changes timed work scope')
        if payload.get('tolerances') != protocol['tolerances']:
            raise ValueError('actual tolerance settings differ from preregistration')
        samples = payload['samples']
        if len(samples) != repeats:
            raise ValueError('wrong sample count')
        for row in samples:
            if row.get('finite') is not True:
                raise ValueError('nonfinite returned observations')
            if row.get('success') is not True:
                raise ValueError('unsuccessful full-IVP solve retained; timing qualification refused')
            if row.get('gridpoint_count') != len(protocol['output_grid']):
                raise ValueError('not all requested grid points returned')
            if finite_number(row['max_abs_error'], 'max_abs_error', True) > protocol['max_abs_error_cap']:
                raise ValueError('same-accuracy diagnostic cap exceeded')
            finite_number(row['scaled_rms_error'], 'scaled_rms_error', True)
            for key in ['attempts', 'accepted_steps', 'rejected_steps', 'gmres_cycles', 'elapsed_ns', 'drop_ns', 'strict_work_units']:
                nonnegative_integer(row[key], key)
            for key in ['rhs', 'jvp']:
                nonnegative_integer(row['callback_counts'][key], 'callback_counts.' + key)
            if not isinstance(row['work'], dict) or not isinstance(row['proposal_statistics'], dict):
                raise ValueError('work/proposal accounting omitted')
            if mode == 'probe':
                finite_number(row['max_logged_relative_residual'], 'max_logged_relative_residual', True)
            elif row['max_logged_relative_residual'] is not None:
                raise ValueError('timing mode unexpectedly recorded residual-log work')
            if row['solve_plus_drop_ns'] != row['elapsed_ns'] + row['drop_ns']:
                raise ValueError('timing scope sum does not match')
            if mode == 'probe':
                if row['t'] != protocol['output_grid'] or len(row['y']) != len(protocol['output_grid']):
                    raise ValueError('probe returned row grid mismatch')
                if len(row['t_bits']) != len(protocol['output_grid']) or len(row['y_bits']) != len(protocol['output_grid']):
                    raise ValueError('probe bit observations omitted')
                for values, bits in zip(row['y'], row['y_bits'], strict=True):
                    if len(values) != case['dimension'] or len(bits) != case['dimension']:
                        raise ValueError('probe state shape mismatch')
                    for value in values:
                        finite_number(value, 'returned state')
        return samples

    def invoke(case, arm, start, mode, repeats, label):
        argv = [str(binary), '--case', case['case'], '--arm', arm, '--start', start, '--mode', mode,
                '--repeats', str(repeats), '--span', str(protocol['span'])]
        receipt, target = run(label, argv, protocol['benchmark_command_timeout_seconds'])
        if receipt['exit_code'] != 0:
            raise RuntimeError(f'probe process failed: {label}')
        payload = load_json(target / 'stdout.txt')
        samples = validate(payload, case, arm, start, mode, repeats)
        metadata_key = (case['case'], start)
        metadata = {key: payload[key] for key in ('case', 'dimension', 'start', 'span', 'tolerances', 'output_grid', 'physical_error_cap')}
        if metadata_key in fixture_metadata and fixture_metadata[metadata_key] != metadata:
            raise ValueError('fixture controls differ between arms or between probe and timing')
        fixture_metadata[metadata_key] = metadata
        trials.append({'label': label, 'case': case['case'], 'arm': arm, 'start': start, 'mode': mode,
                       'repeats': repeats, 'max_grid_error': max(row['max_abs_error'] for row in samples),
                       'strict_work_units': [row['strict_work_units'] for row in samples],
                       'callback_counts': [row['callback_counts'] for row in samples],
                       'proposal_statistics': [row['proposal_statistics'] for row in samples],
                       'gmres_cycles': [row['gmres_cycles'] for row in samples],
                       'accepted_steps': [row['accepted_steps'] for row in samples],
                       'rejected_steps': [row['rejected_steps'] for row in samples],
                       'solve_plus_drop_ns': [row['elapsed_ns'] + row['drop_ns'] for row in samples]})
        checkpoint()
        return payload, samples

    result_code = 0
    try:
        if args.phase in ('checks', 'all'):
            for item in protocol['checks']:
                argv = [word.replace('{output}', str(out)).replace('{source}', args.source_sha).replace('{repo}', str(repo)) for word in item['argv']]
                row, target = run(item['label'], argv, item['timeout_seconds'])
                if item.get('requires_nonzero_test_count') and row['exit_code'] == 0:
                    import re
                    text = (target / 'stdout.txt').read_text() + (target / 'stderr.txt').read_text()
                    counts = re.findall(r'test result: .*? (\d+) passed;', text)
                    if not counts or sum(map(int, counts)) == 0:
                        raise ValueError('successful command did not execute registered tests')
            state['checks_result'] = 'pass_scoped' if all(row['exit_code'] == 0 for row in state['commands']) else 'fail'
            if state['checks_result'] != 'pass_scoped':
                state['execution'] = 'checks_failed_benchmark_not_run'
                return 1
        if args.phase in ('bench', 'all'):
            for case in protocol['cases']:
                metadata = {}
                initial_bits = {}
                for start in protocol['correctness_starts']:
                    for arm in protocol['arms']:
                        payload, samples = invoke(case, arm, start, 'probe', protocol['probe_repeats'], f"{case['case']}-{start}-probe-{arm}")
                        metadata[arm] = {key: payload[key] for key in ('case', 'dimension', 'start', 'span', 'tolerances', 'output_grid')}
                        initial_bits[arm] = (samples[0]['t_bits'][0], samples[0]['y_bits'][0])
                    if initial_bits['gmres'] != initial_bits['proposal']:
                        raise ValueError('arms do not start from identical represented state/time')
                    if metadata['gmres'] != metadata['proposal']:
                        raise ValueError('arms differ in fixed problem/settings/grid')
                for arm in protocol['arms']:
                    invoke(case, arm, protocol['timing_start'], 'timing', protocol['warmup_solves'], f"{case['case']}-warmup-{arm}")
                ratios = []
                for block in range(protocol['paired_blocks']):
                    order = ['gmres', 'proposal', 'proposal', 'gmres'] if block % 2 == 0 else ['proposal', 'gmres', 'gmres', 'proposal']
                    medians = {arm: [] for arm in protocol['arms']}
                    for position, arm in enumerate(order):
                        _, samples = invoke(case, arm, protocol['timing_start'], 'timing', protocol['timed_solves_per_child'], f"{case['case']}-b{block}-{position}-{arm}")
                        medians[arm].append(statistics.median(row['elapsed_ns'] + row['drop_ns'] for row in samples))
                    denominator = sum(medians['proposal'])
                    if denominator <= 0:
                        raise ValueError('nonpositive candidate timing')
                    ratio = sum(medians['gmres']) / denominator
                    ratios.append(ratio)
                    pair_rows.append({'case': case['case'], 'block': block, 'order': order, 'child_medians_ns': medians,
                                      'gmres_over_proposal_ratio': ratio})
                    case_rows[case['case']] = {'classification': case['classification'], 'completed_blocks': len(ratios),
                                             'ratios': list(ratios), 'median': statistics.median(ratios),
                                             'minimum': min(ratios), 'maximum': max(ratios),
                                             'range_is_confidence_interval': False, 'timing_authority': 'diagnostic_only'}
                    checkpoint()
            state['benchmark_result'] = 'pass_scoped_all_registered_cases'
        state['execution'] = 'completed'
    except BaseException as exc:
        result_code = 1
        state['execution'] = 'failed_or_interrupted'
        state['failure'] = {'type': type(exc).__name__, 'message': str(exc)}
    finally:
        after_hashes = source_hashes()
        write_json(out / 'SOURCE_HASHES_AFTER.json', after_hashes)
        state['source_hashes_unchanged'] = before_hashes == after_hashes
        if before_hashes != after_hashes:
            result_code = 1
            state['execution'] = 'invalidated_source_mutation'
        state['finished_utc'] = dt.datetime.now(dt.timezone.utc).isoformat()
        checkpoint()
        manifest = {str(path.relative_to(out)): {'bytes': path.stat().st_size, 'sha256': sha256(path)}
                    for path in sorted(out.rglob('*')) if path.is_file() and path.name != 'MANIFEST.json'}
        write_json(out / 'MANIFEST.json', manifest)
    return result_code


if __name__ == '__main__':
    sys.exit(main())
