#!/usr/bin/env python3
"""One source-bound validation lane; every outcome, including failures, is retained."""
import argparse
import atexit
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

ROOT = Path(__file__).resolve().parents[3]
NODE = Path(__file__).resolve().parents[1]
CASES = ('dense-fd', 'banded-fd', 'time-jvp', 'autonomous-jvp')


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--source-sha', required=True)
    p.add_argument('--remote-ref', required=True)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--baseline-dir', type=Path, required=True)
    p.add_argument('--candidate-dir', type=Path, required=True)
    p.add_argument('--phase', choices=('checks', 'bench', 'deployment', 'all'), default='all')
    p.add_argument('--wall-cap', type=float, default=900)
    p.add_argument('--prior-run', type=Path, action='append', default=[])
    a = p.parse_args()
    if git('rev-parse', 'HEAD') != a.source_sha or git('rev-parse', a.remote_ref) != a.source_sha:
        p.error('HEAD and fetched remote ref must equal published source')
    subprocess.run(['git', 'diff', '--exit-code', 'HEAD', '--', 'crates', 'Cargo.lock', str(NODE.relative_to(ROOT))], cwd=ROOT, check=True)
    out = a.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    env = os.environ | dict(CARGO_BUILD_JOBS='2', RAYON_NUM_THREADS='1', OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', PYTHONDONTWRITEBYTECODE='1')
    prior_wall = sum(json.loads(path.read_text())['wall_seconds'] for path in a.prior_run)
    if prior_wall >= a.wall_cap:
        p.error('aggregate prior phase wall already exhausted campaign cap')
    start = time.monotonic()
    rows = []
    state = dict(schema='vigilode.synchronized-lane.v1', source=a.source_sha,
                 tree=git('rev-parse', 'HEAD^{tree}'), remote_ref=a.remote_ref,
                 started_utc=dt.datetime.now(dt.timezone.utc).isoformat(), phase=a.phase,
                 cargo_lock_sha256=digest(ROOT/'Cargo.lock'), runner_sha256=digest(__file__),
                 affinity=sorted(os.sched_getaffinity(0)), commands=rows)
    state.update(execution='running', prior_runs=[str(x) for x in a.prior_run], prior_wall_seconds=prior_wall)

    def finish():
        state['finished_utc'] = dt.datetime.now(dt.timezone.utc).isoformat()
        state['wall_seconds'] = time.monotonic()-start
        state['aggregate_wall_seconds'] = prior_wall+state['wall_seconds']
        if state['execution'] == 'running':
            state['execution'] = 'failed_or_interrupted'
        (out/'RUN.json').write_text(json.dumps(state, indent=2)+'\n')
    atexit.register(finish)

    def run(label, argv, cap=300):
        if git('rev-parse', 'HEAD') != a.source_sha:
            raise RuntimeError('source changed while running')
        remaining = a.wall_cap - prior_wall - (time.monotonic()-start)
        if remaining <= 0:
            raise TimeoutError('global lane wall cap exhausted')
        directory = out/label
        directory.mkdir()
        t = time.monotonic()
        before = resource.getrusage(resource.RUSAGE_CHILDREN)
        with (directory/'stdout.txt').open('wb') as so, (directory/'stderr.txt').open('wb') as se:
            proc = subprocess.Popen(argv, cwd=ROOT, env=env, stdout=so, stderr=se, start_new_session=True)
            timed_out = False
            try:
                rc = proc.wait(timeout=min(cap, remaining))
            except subprocess.TimeoutExpired:
                os.killpg(proc.pid, signal.SIGKILL)
                rc = proc.wait()
                timed_out = True
        after = resource.getrusage(resource.RUSAGE_CHILDREN)
        row = dict(label=label, argv=list(map(str, argv)), exit_code=rc, timed_out=timed_out,
                   wall_seconds=time.monotonic()-t,
                   child_cpu_seconds=after.ru_utime+after.ru_stime-before.ru_utime-before.ru_stime,
                   cumulative_child_maxrss_kib=after.ru_maxrss,
                   stdout_sha256=digest(directory/'stdout.txt'), stderr_sha256=digest(directory/'stderr.txt'))
        (directory/'RECEIPT.json').write_text(json.dumps(row, indent=2)+'\n')
        rows.append(row)
        (out/'RUN.json').write_text(json.dumps(state, indent=2)+'\n')
        print(json.dumps(dict(label=label, exit_code=rc, wall_seconds=row['wall_seconds'])), flush=True)
        return row, directory/'stdout.txt'

    common = ['--profile', 'measurement', '--locked', '--offline']
    if a.phase in ('checks', 'all'):
        commands = [
            ('proof-oracles', [sys.executable, '-m', 'unittest', 'discover', '-s', str(NODE/'code'), '-p', 'test_*.py', '-v']),
            ('libraries', ['cargo', 'test', *common, '-p', 'vigilode', '-p', 'rodas5p-krylov', '-p', 'vigilode-coupled', '--lib']),
            ('facade-contracts', ['cargo', 'test', *common, '-p', 'vigilode', '--test', '*', '--no-fail-fast']),
            ('krylov-contracts', ['cargo', 'test', *common, '-p', 'rodas5p-krylov', '--test', '*', '--no-fail-fast']),
            ('time-fd-contract', ['cargo', 'test', *common, '-p', 'rodas5p-integrators', '--test', 'time_fd_workspace_contracts']),
            ('focused-clippy', ['cargo', 'clippy', *common, '-p', 'vigilode', '-p', 'rodas5p-krylov', '--all-targets', '--no-deps', '--', '-D', 'warnings']),
            ('time-fd-clippy', ['cargo', 'clippy', *common, '-p', 'rodas5p-integrators', '--test', 'time_fd_workspace_contracts', '--no-deps', '--', '-D', 'warnings']),
        ]
        for label, argv in commands:
            receipt, path = run(label, argv, 600)
            if label == 'proof-oracles' and receipt['exit_code'] == 0:
                import re
                log = (path.parent/'stderr.txt').read_text()
                match = re.search(r'Ran (\d+) tests?', log)
                if not match or int(match[1]) == 0:
                    raise RuntimeError('no executed proof tests')
        if any(r['exit_code'] != 0 for r in rows) and a.phase == 'all':
            state['execution'] = 'checks_failed_later_phases_not_run'
            state['all_commands_exit_zero'] = False
            return 1

    if a.phase in ('bench', 'all'):
        bins = {arm: directory.resolve()/'rust_allocation_probe' for arm, directory in [('baseline', a.baseline_dir), ('candidate', a.candidate_dir)]}
        bindings = {}
        for arm, directory in [('baseline', a.baseline_dir), ('candidate', a.candidate_dir)]:
            bindings[arm] = json.loads((directory/'BINDINGS.json').read_text())
            for name in ('rust_allocation_probe', 'ownership_projection_probe'):
                binding = bindings[arm][name]
                if digest(directory/name) != binding['binary_sha256']:
                    raise RuntimeError('binary does not match build binding')
                if not git('branch', '-r', '--contains', binding['source']):
                    raise RuntimeError('binary source is not remotely reachable')
                if arm == 'candidate' and binding['source'] != a.source_sha:
                    raise RuntimeError('candidate binary source mismatch')
        state['build_bindings'] = bindings
        state['binary_sha256'] = {k: digest(v) for k, v in bins.items()}
        pairs = []
        timings = []
        ownership = {}
        for arm, directory in [('baseline', a.baseline_dir), ('candidate', a.candidate_dir)]:
            rec, path = run('ownership-'+arm, [str(directory.resolve()/'ownership_projection_probe'), '--case', 'all', '--mode', 'alloc', '--repeats', '3'], 60)
            if rec['exit_code']:
                raise RuntimeError('ownership probe failed')
            ownership[arm] = json.loads(path.read_text())
        ownership_pairs = []
        for left, right in zip(ownership['baseline']['rows'], ownership['candidate']['rows'], strict=True):
            excluded = ('solve_allocations','solve_ns','drop_ns')
            same = {k:v for k,v in left.items() if k not in excluded} == {k:v for k,v in right.items() if k not in excluded}
            ownership_pairs.append(dict(case=left['case'], repeat=left['repeat'], exact_parity=same,
                                        baseline_allocations=left['solve_allocations'], candidate_allocations=right['solve_allocations']))
        (out/'OWNERSHIP_PAIRS.json').write_text(json.dumps(ownership_pairs, indent=2)+'\n')
        if len(ownership_pairs) != 33 or not all(row['exact_parity'] for row in ownership_pairs):
            raise RuntimeError('ownership observations differ or incomplete')
        for case in CASES:
            data = {}
            for mode in ('probe', 'alloc'):
                for arm, binary in bins.items():
                    receipt, path = run(f'{case}-{mode}-{arm}', [str(binary), '--case', case, '--mode', mode, '--repeats', '3'], 30)
                    if receipt['exit_code']:
                        raise RuntimeError('probe failure retained; timing refused')
                    data[arm] = json.loads(path.read_text())
                same = data['baseline']['callback_counts'] == data['candidate']['callback_counts']
                same &= all(data['baseline'][k] == data['candidate'][k] for k in ('case','mode','span','model_param','repeats'))
                for left, right in zip(data['baseline']['samples'], data['candidate']['samples'], strict=True):
                    same &= {k:v for k,v in left.items() if k != 'solve_allocations'} == {k:v for k,v in right.items() if k != 'solve_allocations'}
                    for row in (left,right):
                        err = row['endpoint_max_abs_error']
                        same &= row['status'] == {'status':'success'} and err is not None and math.isfinite(err) and err <= 1e-5
                pairs.append(dict(case=case, mode=mode, exact_returned_parity=same,
                                  baseline_allocations=[s['solve_allocations'] for s in data['baseline']['samples']],
                                  candidate_allocations=[s['solve_allocations'] for s in data['candidate']['samples']],
                                  endpoint_error=max(s['endpoint_max_abs_error'] or 0 for s in data['candidate']['samples'])))
                if not same:
                    (out/'PAIRS.json').write_text(json.dumps(pairs, indent=2)+'\n')
                    raise RuntimeError('exact returned parity gate failed; timing refused')
            for arm, binary in bins.items():
                rec, path = run(f'{case}-warmup-{arm}', [str(binary), '--case', case, '--mode', 'timing', '--repeats', '16'], 30)
                if rec['exit_code'] or not all(s['success'] for s in json.loads(path.read_text())['samples']):
                    raise RuntimeError('warmup failure retained')
            for block in range(7):
                order = ('baseline','candidate','candidate','baseline') if block%2 == 0 else ('candidate','baseline','baseline','candidate')
                sample = {'baseline': [], 'candidate': []}
                for pos, arm in enumerate(order):
                    rec, path = run(f'{case}-b{block}-{pos}-{arm}', [str(bins[arm]), '--case', case, '--mode', 'timing', '--repeats', '16'], 30)
                    payload = json.loads(path.read_text())
                    if rec['exit_code'] or not all(s['success'] for s in payload['samples']):
                        raise RuntimeError('timed failure retained')
                    sample[arm].append(statistics.median(s['solve_plus_drop_ns'] for s in payload['samples']))
                timings.append(dict(case=case, block=block, order=order, child_medians=sample,
                                    ratio=sum(sample['baseline'])/sum(sample['candidate'])))
                (out/'TIMINGS.json').write_text(json.dumps(timings, indent=2)+'\n')
            (out/'PAIRS.json').write_text(json.dumps(pairs, indent=2)+'\n')
        (out/'PAIRS.json').write_text(json.dumps(pairs, indent=2)+'\n')
        (out/'TIMINGS.json').write_text(json.dumps(timings, indent=2)+'\n')
        state['comparison_scope'] = 'Identical returned checkpoints/Stats/callback counts in separate probe+allocation arms; timed arms assert status. Endpoint analytic binary64 diagnostic cap1e-5, no rigorous reference uncertainty or all-interior trajectory claim.'

    if a.phase in ('deployment', 'all'):
        binary = a.candidate_dir.resolve()/'production_from_json'
        binding = json.loads((a.candidate_dir/'BINDINGS.json').read_text())['production_from_json']
        if binding['source'] != a.source_sha or binding['binary_sha256'] != digest(binary):
            raise RuntimeError('deployment executable source/hash mismatch')
        state['deployment_build_binding'] = binding
        receipt, config = run('single-config', [str(binary), '--template', 'selective-jvp'])
        if receipt['exit_code'] == 0:
            for rate in ('1', '1000'):
                run('production-rate-'+rate, [str(binary), str(config), rate], 60)
        state['deployment_scope'] = 'One existing selective-jvp ProductionConfig and dispatcher, two manufactured invocation checks; physical client/deployment qualification remains open.'

    state['finished_utc'] = dt.datetime.now(dt.timezone.utc).isoformat()
    state['wall_seconds'] = time.monotonic()-start
    state['all_commands_exit_zero'] = all(r['exit_code'] == 0 for r in rows)
    state['production_qualification'] = 'HOLD'
    state['execution'] = 'completed'
    (out/'RUN.json').write_text(json.dumps(state, indent=2)+'\n')
    return 0 if state['all_commands_exit_zero'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
