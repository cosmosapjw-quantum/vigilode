#!/usr/bin/env python3
"""Locked, serial Julia endpoint campaign runner.

This runner deliberately accepts a hash-bound plan rather than inventing
scientific tolerances or identities. It never installs packages, builds Rust,
or compares timing against a different source/host cohort.
"""
from __future__ import annotations

import argparse
import datetime as dt
import fcntl
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import time
import traceback

SCHEMA = 'vigilode.julia-comparison-plan.v1'
METHODS = ('julia-tsit5', 'julia-rodas5p')
CASES = (1, 10, 15, 16)
TIMED_REPEATS = 5
WARMUP_REPEATS = 1
ERROR_LIMIT = 1e-4
GLOBAL_COST_ROOT = Path('/tmp/vigilode-local-run-20261024.geLVCp9Q/continuation_20261025T080718Z')
GLOBAL_DEADLINE = '2026-10-27T08:07:00Z'
GLOBAL_CAP_SECONDS = 172800.0
HOUSEKEEPING_RESERVE_SECONDS = 60.0
THREAD_ENV = ('JULIA_NUM_THREADS', 'OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS',
              'MKL_NUM_THREADS', 'BLIS_NUM_THREADS', 'VECLIB_MAXIMUM_THREADS',
              'RAYON_NUM_THREADS')


def sha256_file(path: str | Path) -> str:
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1 << 20), b''):
            h.update(block)
    return h.hexdigest()


def write_new(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x', encoding='utf-8') as stream:
        json.dump(value, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')


def write_bytes_new(path: Path, value: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('xb') as stream:
        stream.write(value)


class BudgetExhausted(RuntimeError):
    pass


def utc_now():
    return dt.datetime.now(dt.timezone.utc)


def validate_global_binding(plan):
    root = Path(plan.get('cost_root', '')).resolve()
    if (root != GLOBAL_COST_ROOT.resolve() or not (root/'COST_EVENTS.jsonl').is_file()
            or plan.get('deadline_utc') != GLOBAL_DEADLINE
            or plan.get('new_cap_seconds') != GLOBAL_CAP_SECONDS):
        raise ValueError('plan must bind the existing global cost root, 48h cap, and absolute deadline')
    return root


def _cost_module():
    spec = importlib.util.spec_from_file_location(
        '_julia_continuation_costs', Path(__file__).with_name('continuation_runner.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _canonical_sha(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':'),
                                     allow_nan=False).encode()).hexdigest()


def _chain_head(root):
    lines = (root/'COST_EVENTS.jsonl').read_text().splitlines()
    return _canonical_sha(json.loads(lines[-1])) if lines else '0'*64


def _append_cost_locked(root, row, expected_head):
    if _chain_head(root) != expected_head:
        raise ValueError('global cost chain changed before append')
    row = dict(row, schema='vigilode.local-continuation-cost-event.v1',
               prev_event_sha256=expected_head, observed_utc=utc_now().isoformat(), category='julia')
    with (root/'COST_EVENTS.jsonl').open('a') as stream:
        stream.write(json.dumps(row, sort_keys=True, separators=(',', ':'), allow_nan=False)+'\n')
        stream.flush()
        os.fsync(stream.fileno())


class CampaignBudget:
    """Reserve one serial invocation; all children and exclusive overhead fit inside it.

    An interrupted invocation keeps its full STARTED reservation. A new output
    directory cannot replace that event or reset the owner's cumulative cap.
    """
    def __init__(self, plan, output, started):
        self.root = validate_global_binding(plan)
        self.started = started
        self.requested = plan['limits']['campaign_budget_seconds']
        self.attempt_id = 'julia-comparison-'+hashlib.sha256(str(output).encode()).hexdigest()
        self.reserved = None
        self.execution_budget = 0.0
        self.costs = _cost_module()

    def _charged(self):
        return self.costs.cumulative_new_charge(self.root)

    def _deadline_remaining(self):
        end = dt.datetime.fromisoformat(GLOBAL_DEADLINE.replace('Z', '+00:00'))
        return (end-utc_now()).total_seconds()

    def begin(self):
        with (self.root/'COST_EVENTS.lock').open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            charged = self._charged()
            head = _chain_head(self.root)
            upper = min(self.requested+HOUSEKEEPING_RESERVE_SECONDS,
                        GLOBAL_CAP_SECONDS-charged, self._deadline_remaining())
            if upper <= HOUSEKEEPING_RESERVE_SECONDS:
                raise BudgetExhausted('global cap or absolute deadline leaves no safe campaign budget')
            _append_cost_locked(self.root, {'attempt_id': self.attempt_id,
                'event_id': self.attempt_id+'-started', 'phase': 'STARTED',
                'charge_upper_seconds': upper}, head)
            self.reserved = upper
            self.execution_budget = upper-HOUSEKEEPING_RESERVE_SECONDS

    def remaining(self):
        # The active invocation is already fully reserved in the shared chain.
        # Revalidate it on every launch, including other controllers' costs.
        with (self.root/'COST_EVENTS.lock').open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            if self._charged() > GLOBAL_CAP_SECONDS:
                raise BudgetExhausted('global cumulative charge exceeds the fixed cap')
        return max(0.0, min(self.execution_budget-(time.monotonic()-self.started),
                            self._deadline_remaining()-HOUSEKEEPING_RESERVE_SECONDS))

    def finish(self, receipt_path, charge):
        if self.reserved is None:
            return
        with (self.root/'COST_EVENTS.lock').open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            self._charged()
            _append_cost_locked(self.root, {'attempt_id': self.attempt_id,
                'event_id': self.attempt_id+'-terminal', 'phase': 'TERMINAL',
                'charged_seconds': charge, 'charge_basis': 'serial_invocation_wall_including_all_children_and_controller_overhead',
                'exact_duration_known': True, 'evidence_path': str(receipt_path),
                'evidence_sha256': sha256_file(receipt_path)}, _chain_head(self.root))


def file_record(path: str | Path, expected: str) -> dict:
    resolved = Path(path).resolve(strict=True)
    actual = sha256_file(resolved)
    if actual != expected:
        raise ValueError(f'locked file hash mismatch: {resolved}')
    return {'path': str(resolved), 'sha256': actual, 'bytes': resolved.stat().st_size}


def git_identity(repo: str | Path) -> dict:
    def git(*args):
        return subprocess.check_output(['git', '-C', str(repo), *args], text=True, timeout=10).strip()
    return {'commit': git('rev-parse', 'HEAD'), 'tree': git('rev-parse', 'HEAD^{tree}'),
            'status_porcelain': git('status', '--porcelain=v1')}


def _selector(value, dotted: str):
    current = value
    for part in dotted.split('.'):
        if isinstance(current, list):
            current = current[int(part)]
        else:
            current = current[part]
    return current


def _finite_vector(value, size: int, label: str) -> list[float]:
    if not isinstance(value, list) or len(value) != size:
        raise ValueError(f'{label} must be a vector of length {size}')
    out = []
    for item in value:
        if isinstance(item, bool) or not isinstance(item, (int, float)) or not math.isfinite(item):
            raise ValueError(f'{label} contains a non-finite or non-numeric value')
        out.append(float(item))
    return out


def resolve_case(case_plan: dict) -> dict:
    case_id = case_plan.get('case')
    if type(case_id) is not int or case_id not in CASES or case_plan.get('size') != 'standard':
        raise ValueError('case must be one of 1, 10, 15, 16 at standard size')
    descriptor = file_record(case_plan['descriptor_path'], case_plan['descriptor_sha256'])
    contract = json.loads(Path(descriptor['path']).read_text())
    model = contract.get('model_contract')
    if (not isinstance(model, dict) or contract.get('kind') != 'model_contract' or
            contract.get('case_id') != case_id or contract.get('size') != 'standard' or
            model.get('case') != case_id or model.get('reference_kind') not in
            ('analytic-semidiscrete-endpoint', 'external-reference-required')):
        raise ValueError(f'case {case_id} descriptor contract mismatch')
    state = _finite_vector(contract.get('initial_state'), model.get('dimension_standard'),
                           f'case {case_id} initial state')
    end = contract.get('t_end')
    if not isinstance(end, (int, float)) or isinstance(end, bool) or not math.isfinite(end) or end <= 0:
        raise ValueError(f'case {case_id} has invalid endpoint')
    reference_plan = case_plan.get('reference')
    if not isinstance(reference_plan, dict):
        raise ValueError(f'case {case_id} missing reference contract')
    reference_file = file_record(reference_plan['path'], reference_plan['sha256'])
    reference = json.loads(Path(reference_file['path']).read_text())
    reference_state = _finite_vector(_selector(reference, reference_plan['state_path']),
                                     len(state), f'case {case_id} reference state')
    uncertainty = _selector(reference, reference_plan['uncertainty_path'])
    if (isinstance(uncertainty, bool) or not isinstance(uncertainty, (int, float)) or
            not math.isfinite(uncertainty) or not 0 <= uncertainty <= ERROR_LIMIT):
        raise ValueError(f'case {case_id} invalid reference uncertainty U')
    provenance = _selector(reference, reference_plan['provenance_path'])
    if not isinstance(provenance, str) or not provenance.strip():
        raise ValueError(f'case {case_id} missing reference provenance')
    uncertainty_interpretation = reference_plan.get('uncertainty_interpretation')
    if not isinstance(uncertainty_interpretation, str) or not uncertainty_interpretation.strip():
        raise ValueError(f'case {case_id} missing uncertainty interpretation')
    binding = reference_plan.get('descriptor_binding')
    if not isinstance(binding, dict):
        raise ValueError(f'case {case_id} missing descriptor binding')
    if binding.get('kind') == 'descriptor_sha256':
        if _selector(reference, binding['path']) != descriptor['sha256']:
            raise ValueError(f'case {case_id} reference descriptor hash mismatch')
    elif binding.get('kind') == 'descriptor_exact_final':
        ref_exact = _finite_vector(_selector(reference, binding['path']), len(state),
                                   f'case {case_id} reference descriptor endpoint')
        desc_exact = contract.get('exact_final')
        if desc_exact is None or ref_exact != _finite_vector(desc_exact, len(state),
                                                              f'case {case_id} descriptor endpoint'):
            raise ValueError(f'case {case_id} reference endpoint does not bind descriptor')
    else:
        raise ValueError(f'case {case_id} unsupported descriptor binding')
    if model['reference_kind'] == 'analytic-semidiscrete-endpoint':
        exact = _finite_vector(contract.get('exact_final'), len(state),
                               f'case {case_id} analytic descriptor endpoint')
        if max(abs(a-b) for a, b in zip(reference_state, exact)) > uncertainty:
            raise ValueError(f'case {case_id} selected reference state conflicts with analytic descriptor endpoint')
    rtol, atol = case_plan.get('rtol'), case_plan.get('atol')
    max_steps = case_plan.get('max_steps')
    if any(isinstance(x, bool) or not isinstance(x, (int, float)) or not math.isfinite(x) or x <= 0
           for x in (rtol, atol)) or type(max_steps) is not int or not 1 <= max_steps <= 2_000_000:
        raise ValueError(f'case {case_id} invalid tolerances or max_steps')
    return {'case': case_id, 'size': 'standard', 'descriptor': descriptor,
            'descriptor_data': contract, 'dimension': len(state), 't_end': float(end),
            'reference_file': reference_file, 'reference_state': reference_state,
            'uncertainty_U': float(uncertainty), 'reference_provenance': provenance,
            'uncertainty_interpretation': uncertainty_interpretation,
            'rtol': float(rtol), 'atol': float(atol), 'max_steps': max_steps}


def validate_plan(plan: dict) -> dict:
    validate_global_binding(plan)
    if plan.get('schema') != SCHEMA:
        raise ValueError('plan schema mismatch')
    expected_repeats = plan.get('repeats')
    if expected_repeats != {'warmup': WARMUP_REPEATS, 'timed': TIMED_REPEATS}:
        raise ValueError('plan must specify exactly one warmup and five timed attempts')
    if plan.get('methods') != list(METHODS) or plan.get('error_limit') != ERROR_LIMIT:
        raise ValueError('methods or fixed endpoint error cap mismatch')
    source = plan.get('source')
    if not isinstance(source, dict):
        raise ValueError('missing source identity')
    actual_source = git_identity(source['repo'])
    expected_source = {'commit': source.get('commit'), 'tree': source.get('tree'),
                       'status_porcelain': ''}
    if actual_source != expected_source:
        raise ValueError('locked source checkout identity mismatch or dirty source')
    project = file_record(plan['project_path'], plan['project_sha256'])
    manifest = file_record(plan['manifest_path'], plan['manifest_sha256'])
    if Path(project['path']).parent != Path(manifest['path']).parent:
        raise ValueError('Project.toml and Manifest.toml must share the pinned environment')
    adapter = file_record(plan['adapter_path'], plan['adapter_sha256'])
    try:
        Path(adapter['path']).relative_to(Path(source['repo']).resolve())
    except ValueError as exc:
        raise ValueError('adapter must be inside the locked source checkout') from exc
    julia = file_record(plan['julia_path'], plan['julia_sha256'])
    binary = file_record(plan['binary_path'], plan['binary_sha256'])
    build_receipt = file_record(plan['build_receipt_path'], plan['build_receipt_sha256'])
    build = json.loads(Path(build_receipt['path']).read_text())
    if (build.get('source', {}).get('commit') != source['commit'] or
            build.get('source', {}).get('tree') != source['tree'] or
            build.get('binary_sha256') != binary['sha256']):
        raise ValueError('binary build receipt does not bind source and binary')
    depot = Path(plan['depot_path']).resolve()
    user_depot = (Path.home()/'.julia').resolve()
    if (not depot.is_dir() or depot == Path.home().resolve() or
            depot == user_depot or user_depot in depot.parents):
        raise ValueError('Julia depot must be isolated from the user default depot')
    limits = plan.get('limits')
    required_limits = ('cpu', 'attempt_timeout_seconds', 'idle_timeout_seconds',
                       'campaign_budget_seconds', 'setup_timeout_seconds',
                       'max_rss_bytes', 'max_output_bytes')
    if not isinstance(limits, dict) or any(k not in limits for k in required_limits):
        raise ValueError('incomplete timeout, resource, or CPU limits')
    if type(limits['cpu']) is not int or limits['cpu'] not in os.sched_getaffinity(0):
        raise ValueError('requested CPU is outside current affinity')
    for key in required_limits[1:]:
        value = limits[key]
        if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value <= 0:
            raise ValueError(f'invalid positive limit: {key}')
    if type(limits['max_rss_bytes']) is not int or type(limits['max_output_bytes']) is not int:
        raise ValueError('memory/output byte limits must be integers')
    raw_cases = plan.get('cases')
    if not isinstance(raw_cases, list) or {c.get('case') for c in raw_cases if isinstance(c, dict)} != set(CASES) or len(raw_cases) != len(CASES):
        raise ValueError('plan must contain cases 1, 10, 15, and 16 exactly once')
    cases = [resolve_case(c) for c in raw_cases]
    return {'source': actual_source, 'project': project, 'manifest': manifest,
            'adapter': adapter, 'julia': julia, 'binary': binary,
            'build_receipt': build_receipt, 'cases': sorted(cases, key=lambda c: c['case']),
            'limits': limits}


def _proc_group(pgid: int) -> tuple[int, int, int]:
    """Return group CPU ticks, resident bytes, and process count from /proc."""
    ticks = rss = count = 0
    page = os.sysconf('SC_PAGE_SIZE')
    for stat_path in Path('/proc').glob('[0-9]*/stat'):
        try:
            raw = stat_path.read_text()
            rest = raw[raw.rfind(')') + 2:].split()
            if int(rest[2]) != pgid:
                continue
            ticks += int(rest[11]) + int(rest[12])
            statm = stat_path.with_name('statm').read_text().split()
            rss += int(statm[1]) * page
            count += 1
        except (OSError, ValueError, IndexError):
            continue
    return ticks, rss, count


def _terminate_group(proc: subprocess.Popen, grace: float = 1.0) -> None:
    try:
        os.killpg(proc.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        proc.wait(timeout=grace)
    except subprocess.TimeoutExpired:
        pass
    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    try:
        proc.wait(timeout=grace)
    except subprocess.TimeoutExpired:
        pass


def run_bounded(command: list[str], env: dict[str, str], cpu: int, *,
                timeout_seconds: float, idle_timeout_seconds: float,
                max_rss_bytes: int, max_output_bytes: int, cwd: str | None = None,
                on_spawn=None) -> dict:
    """Retain partial output and kill/reap the exact group on every failure."""
    started = time.monotonic()
    proc = None
    selector = selectors.DefaultSelector()
    output = {'stdout': bytearray(), 'stderr': bytearray()}
    resources = []
    status, first_failure = 'COMPLETED', None
    hz = os.sysconf('SC_CLK_TCK')
    max_ticks = ticks = 0
    last_cpu_progress = started
    last_sample = 0.0
    try:
        proc = subprocess.Popen(command, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                bufsize=0, start_new_session=True,
                                preexec_fn=lambda: os.sched_setaffinity(0, {cpu}))
        if on_spawn is not None:
            on_spawn(proc)
        selector.register(proc.stdout, selectors.EVENT_READ, 'stdout')
        selector.register(proc.stderr, selectors.EVENT_READ, 'stderr')
        while True:
            now = time.monotonic()
            elapsed = now-started
            for key, _ in selector.select(timeout=min(0.1, max(0.001, timeout_seconds-elapsed))):
                chunk = os.read(key.fileobj.fileno(), 65536)
                if not chunk:
                    selector.unregister(key.fileobj)
                else:
                    output[key.data].extend(chunk)
            group_ticks, group_rss, group_count = _proc_group(proc.pid)
            max_ticks = max(max_ticks, group_ticks)
            if group_ticks > ticks:
                ticks = group_ticks
                last_cpu_progress = now
            if elapsed-last_sample >= 1.0 or proc.poll() is not None:
                resources.append({'elapsed_seconds': elapsed, 'process_group_cpu_seconds': group_ticks/hz,
                                  'process_group_rss_bytes_sum': group_rss,
                                  'process_group_process_count': group_count})
                last_sample = elapsed
            if sum(map(len, output.values())) > max_output_bytes:
                status = 'FAILED_OUTPUT_LIMIT'
            elif group_rss > max_rss_bytes:
                status = 'FAILED_MEMORY_LIMIT'
            elif elapsed >= timeout_seconds:
                status = 'FAILED_ATTEMPT_DEADLINE'
            elif now-last_cpu_progress >= idle_timeout_seconds:
                status = 'FAILED_IDLE_WATCHDOG'
            if status != 'COMPLETED' or (proc.poll() is not None and group_count == 0):
                break
    except BaseException as exc:
        status = 'SPAWN_FAILED' if proc is None else 'FAILED_RUNNER_EXCEPTION'
        first_failure = {'type': type(exc).__name__, 'message': str(exc), 'traceback': traceback.format_exc()}
    finally:
        if proc is not None:
            # Also reap descendants when the leader already exited.
            try:
                live_group = proc.poll() is None or _proc_group(proc.pid)[2]
            except Exception:
                live_group = True
            if live_group:
                _terminate_group(proc)
            proc.wait()
            for name, stream in (('stdout', proc.stdout), ('stderr', proc.stderr)):
                try:
                    while True:
                        chunk = os.read(stream.fileno(), 65536)
                        if not chunk:
                            break
                        output[name].extend(chunk)
                except OSError:
                    pass
                finally:
                    stream.close()
        selector.close()
    if first_failure is not None:
        output['stderr'].extend(('\n'+first_failure['traceback']).encode())
    if status == 'COMPLETED' and proc.returncode != 0:
        status = 'FAILED_PROCESS_EXIT'
    return {'status': status, 'exit_code': proc.returncode if proc is not None else None,
            'stdout': bytes(output['stdout']), 'stderr': bytes(output['stderr']),
            'resources': resources, 'elapsed_seconds': time.monotonic()-started,
            'process_group_cpu_seconds': max_ticks/hz, 'first_failure': first_failure,
            'process_started': proc is not None, 'pid': proc.pid if proc is not None else None}


def build_env(plan: dict) -> dict[str, str]:
    env = os.environ.copy()
    env.update({name: '1' for name in THREAD_ENV})
    env.update({'JULIA_NUM_THREADS': '1', 'JULIA_DEPOT_PATH': str(Path(plan['depot_path']).resolve()),
                'JULIA_PROJECT': str(Path(plan['project_path']).resolve()),
                'JULIA_LOAD_PATH': '@:@stdlib'})
    return env


def attempt_command(plan: dict, case: dict, method: str) -> list[str]:
    return [str(Path(plan['julia_path']).resolve()), '--startup-file=no', '--history-file=no',
            '--threads=1', f"--project={Path(plan['project_path']).resolve()}",
            str(Path(plan['adapter_path']).resolve()),
            str(Path(case['descriptor']['path']).resolve()), method,
            format(case['rtol'], '.17g'), format(case['atol'], '.17g'), str(case['max_steps'])]


def assess_attempt(stdout: bytes, execution_status: str, exit_code: int,
                   case: dict, expected_method: str, expected_julia_version: str,
                   error_limit: float = ERROR_LIMIT) -> dict:
    result = None
    try:
        rows = [json.loads(line) for line in stdout.decode('utf-8').splitlines() if line.strip()]
        result = next(row for row in reversed(rows) if isinstance(row, dict) and row.get('kind') == 'result')
    except (UnicodeDecodeError, json.JSONDecodeError, StopIteration):
        return {'execution_status': execution_status, 'exit_code': exit_code,
                'scientific_status': 'OUTPUT_CONTRACT_FAILURE', 'actual_error': None,
                'reference_uncertainty_U': case['uncertainty_U'], 'error_limit': error_limit,
                'reference_provenance': case['reference_provenance']}
    answer = {'execution_status': execution_status, 'exit_code': exit_code,
              'adapter_reported_status': result.get('scientific_status'),
              'actual_error': None, 'error_plus_U': None,
              'reference_uncertainty_U': case['uncertainty_U'], 'error_limit': error_limit,
              'reference_provenance': case['reference_provenance'],
              'norm': 'maximum component absolute endpoint error',
              'case': case['case'], 'method': result.get('method')}
    if execution_status != 'COMPLETED' or exit_code != 0:
        answer['scientific_status'] = 'PROCESS_FAILURE'
        return answer
    if (result.get('case') != case['case'] or result.get('size') != 'standard' or
            result.get('method') != expected_method or expected_method not in METHODS or
            result.get('julia_version') != expected_julia_version or result.get('solver_success') is not True or
            result.get('threads') != 1 or result.get('parallel_layer') != 'serial'):
        answer['scientific_status'] = 'OUTPUT_CONTRACT_FAILURE'
        return answer
    state = result.get('final_state')
    try:
        state = _finite_vector(state, case['dimension'], 'final state')
    except ValueError:
        answer['scientific_status'] = 'OUTPUT_CONTRACT_FAILURE'
        return answer
    end = result.get('final_time')
    if (isinstance(end, bool) or not isinstance(end, (int, float)) or not math.isfinite(end) or
            abs(end-case['t_end']) > 1e-10*max(1.0, abs(case['t_end']))):
        answer['scientific_status'] = 'OUTPUT_CONTRACT_FAILURE'
        return answer
    error = max(abs(a-b) for a, b in zip(state, case['reference_state']))
    error_plus_u = error+case['uncertainty_U']
    if not math.isfinite(error_plus_u):
        answer['scientific_status'] = 'ERROR_CAP_FAILURE'
        answer['error_calculation'] = 'NONFINITE_DIFFERENCE'
        return answer
    answer['actual_error'] = error
    answer['error_plus_U'] = error_plus_u
    answer['final_time'] = float(end)
    answer['julia_version'] = result.get('julia_version')
    answer['retcode'] = result.get('retcode')
    answer['scientific_status'] = 'PASS_SCOPED' if error_plus_u <= error_limit else 'ERROR_CAP_FAILURE'
    return answer


def capture_hashes(locks: dict) -> dict:
    observed = {}
    for name, item in locks.items():
        row = {'path': str(Path(item['path']).resolve()), 'expected_sha256': item['sha256']}
        try:
            row.update(sha256=sha256_file(row['path']), bytes=Path(row['path']).stat().st_size)
            row['matches'] = row['sha256'] == item['sha256']
        except Exception as exc:
            row.update(sha256=None, matches=False, capture_error=f'{type(exc).__name__}: {exc}')
        observed[name] = row
    return observed


def _lock_snapshot(locks, repo):
    try:
        source = git_identity(repo)
    except Exception as exc:
        source = {'capture_error': f'{type(exc).__name__}: {exc}'}
    return {'source_git': source, 'files': capture_hashes(locks)}


def _locks_valid(snapshot, expected_git):
    return (snapshot['source_git'] == expected_git and
            all(row['matches'] for row in snapshot['files'].values()))


def _empty_run(status):
    return {'status': status, 'exit_code': None, 'stdout': b'', 'stderr': b'',
            'resources': [], 'elapsed_seconds': 0.0, 'process_group_cpu_seconds': 0.0,
            'process_started': False, 'first_failure': None}


def _record_run(folder, command, env, limits, cwd, timeout, budget=None, refused=None):
    started = time.monotonic()
    try:
        if budget is not None:
            timeout = min(timeout, budget.remaining())
        if refused:
            result = _empty_run(refused)
        elif timeout <= 0:
            result = _empty_run('NOT_RUN_GLOBAL_OR_CAMPAIGN_BUDGET')
        else:
            def spawned(proc):
                write_new(folder/'PROCESS.json', {'pid': proc.pid, 'pgid': proc.pid,
                    'started_utc': utc_now().isoformat(), 'argv': command})
            result = run_bounded(command, env, limits['cpu'], timeout_seconds=timeout,
                idle_timeout_seconds=limits['idle_timeout_seconds'],
                max_rss_bytes=limits['max_rss_bytes'], max_output_bytes=limits['max_output_bytes'],
                cwd=cwd, on_spawn=spawned)
    except BudgetExhausted:
        result = _empty_run('NOT_RUN_GLOBAL_OR_CAMPAIGN_BUDGET')
    except BaseException as exc:
        result = _empty_run('FAILED_RUNNER_EXCEPTION')
        result.update(elapsed_seconds=time.monotonic()-started,
                      first_failure={'type': type(exc).__name__, 'message': str(exc),
                                     'traceback': traceback.format_exc()},
                      stderr=traceback.format_exc().encode())
    write_bytes_new(folder/'stdout.raw', result['stdout'])
    write_bytes_new(folder/'stderr.raw', result['stderr'])
    with (folder/'resources.jsonl').open('x', encoding='utf-8') as stream:
        for row in result['resources']:
            stream.write(json.dumps(row, sort_keys=True, allow_nan=False)+'\n')
    write_new(folder/'PROCESS_COST.json', {key: result.get(key) for key in
        ('status', 'elapsed_seconds', 'process_group_cpu_seconds', 'process_started', 'first_failure', 'pid')})
    return result


def _attempt(folder: Path, plan: dict, locks: dict, case: dict, method: str,
             phase: str, index: int, campaign_start: float,
             timeout_seconds: float | None = None, budget=None) -> dict:
    folder.mkdir(parents=True, exist_ok=False)
    command = attempt_command(plan, case, method)
    env = build_env(plan)
    locks = dict(locks, julia={'path': plan['julia_path'], 'sha256': plan['julia_sha256']})
    lock_before = _lock_snapshot(locks, plan['source']['repo'])
    expected_git = {'commit': plan['source']['commit'], 'tree': plan['source']['tree'],
                    'status_porcelain': ''}
    start = {'case': case['case'], 'method': method, 'phase': phase, 'index': index,
             'argv': command, 'cwd': str(Path(plan['source']['repo']).resolve()),
             'cpu_affinity': [plan['limits']['cpu']], 'threads': 1,
             'environment': {key: env[key] for key in (*THREAD_ENV, 'JULIA_DEPOT_PATH',
                                                        'JULIA_PROJECT', 'JULIA_LOAD_PATH')},
             'input_hashes_before': lock_before,
             'descriptor_sha256': case['descriptor']['sha256'],
             'reference_sha256': case['reference_file']['sha256'],
             'reference_U': case['uncertainty_U'], 'error_limit': ERROR_LIMIT,
             'reference_uncertainty_interpretation': case['uncertainty_interpretation'],
             'attempt_timeout_seconds': timeout_seconds or plan['limits']['attempt_timeout_seconds'],
             'campaign_elapsed_at_start_seconds': time.monotonic()-campaign_start,
             'started_utc': dt.datetime.now(dt.timezone.utc).isoformat()}
    write_new(folder/'START.json', start)
    write_new(folder/'ARGV.json', command)
    write_new(folder/'ENVIRONMENT.json', start['environment'])
    run = _record_run(folder, command, env, plan['limits'], start['cwd'],
                      timeout_seconds or plan['limits']['attempt_timeout_seconds'], budget,
                      refused=None if _locks_valid(lock_before, expected_git) else 'NOT_RUN_INPUT_MUTATION')
    assessment = assess_attempt(run['stdout'], run['status'], run['exit_code'], case,
                                method, plan['julia_version'])
    lock_after = _lock_snapshot(locks, plan['source']['repo'])
    if (not _locks_valid(lock_before, expected_git) or not _locks_valid(lock_after, expected_git)
            or lock_before != lock_after):
        assessment['scientific_status'] = 'FAILED_INPUT_MUTATION'
    receipt = {'case': case['case'], 'method': method, 'phase': phase, 'index': index,
               'execution_status': run['status'], 'exit_code': run['exit_code'],
               'scientific_status': assessment['scientific_status'],
               'first_failure': run.get('first_failure'),
               'input_hashes_before': lock_before,
               'assessment': assessment,
               'adapter_process_wall_seconds': run['elapsed_seconds'],
               'process_group_cpu_seconds': run.get('process_group_cpu_seconds'),
               'adapter_reported_warmup_seconds': None,
               'adapter_reported_solve_seconds': None,
               'method_jit_seconds': 'UNATTRIBUTED',
               'setup_residual_seconds': 'UNATTRIBUTED',
               'raw': {'stdout_sha256': hashlib.sha256(run['stdout']).hexdigest(),
                       'stderr_sha256': hashlib.sha256(run['stderr']).hexdigest(),
                       'stdout_bytes': len(run['stdout']), 'stderr_bytes': len(run['stderr']),
                       'argv_sha256': sha256_file(folder/'ARGV.json'),
                       'environment_sha256': sha256_file(folder/'ENVIRONMENT.json'),
                       'resources_sha256': sha256_file(folder/'resources.jsonl')},
               'input_hashes_after': lock_after,
               'reference_provenance': case['reference_provenance'],
               'finished_utc': dt.datetime.now(dt.timezone.utc).isoformat()}
    try:
        parsed = next(row for row in reversed([json.loads(line) for line in run['stdout'].decode().splitlines()
                                               if line.strip()])
                      if isinstance(row, dict) and row.get('kind') == 'result')
        for source, target in (('warmup_seconds', 'adapter_reported_warmup_seconds'),
                               ('elapsed_seconds', 'adapter_reported_solve_seconds')):
            value = parsed.get(source)
            if isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) and value >= 0:
                receipt[target] = float(value)
    except (UnicodeDecodeError, json.JSONDecodeError, StopIteration):
        pass
    write_new(folder/'EXIT.json', {'execution_status': run['status'], 'exit_code': run['exit_code']})
    write_new(folder/'RECEIPT.json', receipt)
    return receipt


def _run_setup_probe(label: str, command: list[str], env: dict[str, str],
                     limits: dict, folder: Path, cpu: int, cwd: str,
                     timeout_seconds: float | None = None, *, locks=None,
                     expected_git=None, budget=None) -> dict:
    folder.mkdir(parents=True, exist_ok=False)
    write_new(folder/'ARGV.json', command)
    write_new(folder/'ENVIRONMENT.json', {k: env[k] for k in (*THREAD_ENV, 'JULIA_DEPOT_PATH',
                                                               'JULIA_PROJECT', 'JULIA_LOAD_PATH')})
    before = _lock_snapshot(locks, cwd) if locks is not None else None
    refused = 'NOT_RUN_INPUT_MUTATION' if before is not None and not _locks_valid(before, expected_git) else None
    timeout = limits['setup_timeout_seconds'] if timeout_seconds is None else timeout_seconds
    result = _record_run(folder, command, env, limits, cwd, timeout, budget, refused)
    after = _lock_snapshot(locks, cwd) if locks is not None else None
    status = result['status']
    if before is not None and (not _locks_valid(before, expected_git) or
                              not _locks_valid(after, expected_git) or before != after):
        status = 'FAILED_INPUT_MUTATION'
    row = {'label': label, 'execution_status': status, 'exit_code': result['exit_code'],
           'process_wall_seconds': result['elapsed_seconds'],
           'process_group_cpu_seconds': result.get('process_group_cpu_seconds'),
           'first_failure': result.get('first_failure'),
           'input_hashes_before': before, 'input_hashes_after': after,
           'stdout_sha256': hashlib.sha256(result['stdout']).hexdigest(),
           'stderr_sha256': hashlib.sha256(result['stderr']).hexdigest(),
           'argv_sha256': sha256_file(folder/'ARGV.json'),
           'environment_sha256': sha256_file(folder/'ENVIRONMENT.json'),
           'resources_sha256': sha256_file(folder/'resources.jsonl')}
    write_new(folder/'EXIT.json', {'execution_status': status, 'exit_code': result['exit_code']})
    write_new(folder/'RECEIPT.json', row)
    return row


def run_campaign(plan_path: Path, output: Path) -> dict:
    plan_bytes = plan_path.read_bytes()
    plan = json.loads(plan_bytes)
    cost_root = validate_global_binding(plan)
    output = output.resolve()
    if output == cost_root or not output.is_relative_to(cost_root):
        raise ValueError('Julia campaign output must be inside the existing global cost root')
    output.parent.mkdir(parents=True, exist_ok=True)
    with (cost_root/'.julia_comparison_campaign.lock').open('a+') as lock_stream:
        try:
            fcntl.flock(lock_stream, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as exc:
            raise RuntimeError('another Julia comparison campaign holds the global run lock') from exc
        output.mkdir(parents=True, exist_ok=False)
        write_bytes_new(output/'PLAN.json', plan_bytes)
        campaign_start = time.monotonic()
        started_utc = utc_now().isoformat()
        limits = plan['limits']
        requested = limits.get('campaign_budget_seconds')
        if isinstance(requested, bool) or not isinstance(requested, (int, float)) or not math.isfinite(requested) or requested <= 0:
            raise ValueError('invalid campaign budget prerequisite')
        budget = CampaignBudget(plan, output, campaign_start)
        rows, setup_records = [], []
        stopped_budget, failure = False, None
        try:
            budget.begin()
            verified = validate_plan(plan)
            locks = {key: verified[key] for key in
                     ('adapter', 'binary', 'project', 'manifest', 'build_receipt', 'julia')}
            for case in verified['cases']:
                locks[f"descriptor_{case['case']}"] = case['descriptor']
                locks[f"reference_{case['case']}"] = case['reference_file']
            env = build_env(plan)
            cwd = str(Path(plan['source']['repo']).resolve())
            expected_git = verified['source']
            version_cmd = [str(Path(plan['julia_path']).resolve()), '--version']
            version = _run_setup_probe('julia-version', version_cmd, env, limits,
                output/'setup'/'version', limits['cpu'], cwd,
                timeout_seconds=min(limits['setup_timeout_seconds'], budget.remaining()),
                locks=locks, expected_git=expected_git, budget=budget)
            setup_records.append(version)
            version_text = (output/'setup'/'version'/'stdout.raw').read_text().strip()
            if version['execution_status'] != 'COMPLETED' or version_text != f"julia version {plan['julia_version']}":
                raise RuntimeError(f"Julia version probe failed: {version['execution_status']}: {version.get('first_failure')}")
            import_expr = ('using JSON3, SciMLBase, OrdinaryDiffEqTsit5, '
                           'OrdinaryDiffEqRosenbrock, LinearAlgebra; '
                           'BLAS.set_num_threads(1); '
                           'println("IMPORT_OK ", VERSION, " BLAS_THREADS ", BLAS.get_num_threads())')
            import_cmd = [str(Path(plan['julia_path']).resolve()), '--startup-file=no', '--history-file=no',
                          '--threads=1', f"--project={Path(plan['project_path']).resolve()}", '-e', import_expr]
            imports = _run_setup_probe('pinned-import-probe', import_cmd, env, limits,
                output/'setup'/'import_probe', limits['cpu'], cwd,
                timeout_seconds=min(limits['setup_timeout_seconds'], budget.remaining()),
                locks=locks, expected_git=expected_git, budget=budget)
            setup_records.append(imports)
            import_text = (output/'setup'/'import_probe'/'stdout.raw').read_text().strip()
            if imports['execution_status'] != 'COMPLETED' or import_text != f"IMPORT_OK {plan['julia_version']} BLAS_THREADS 1":
                raise RuntimeError(f"pinned Julia import probe failed: {imports['execution_status']}: {imports.get('first_failure')}")
            stop = False
            for case in verified['cases']:
                for method in METHODS:
                    group = {'case': case['case'], 'method': method, 'attempts': []}
                    rows.append(group)  # retain the active group even if its next attempt raises
                    for phase, count in (('campaign_warmup', WARMUP_REPEATS), ('timed', TIMED_REPEATS)):
                        for index in range(1, count+1):
                            remaining = budget.remaining()
                            if remaining <= 0:
                                stopped_budget = stop = True
                                group['attempts'].append({'phase': phase, 'index': index,
                                    'execution_status': 'NOT_RUN_GLOBAL_OR_CAMPAIGN_BUDGET',
                                    'scientific_status': 'NOT_RUN', 'charged_seconds': 0.0})
                                break
                            folder = output/'attempts'/f"case{case['case']:02d}_{method}"/f'{phase}_{index:03d}'
                            receipt = _attempt(folder, plan, locks, case, method, phase, index,
                                campaign_start, timeout_seconds=min(limits['attempt_timeout_seconds'], remaining),
                                budget=budget)
                            group['attempts'].append(receipt)
                            if (receipt['scientific_status'] == 'FAILED_INPUT_MUTATION' or
                                    receipt['execution_status'] in ('FAILED_RUNNER_EXCEPTION', 'NOT_RUN_INPUT_MUTATION',
                                                                   'NOT_RUN_GLOBAL_OR_CAMPAIGN_BUDGET')):
                                stop = True
                                stopped_budget = receipt['execution_status'] == 'NOT_RUN_GLOBAL_OR_CAMPAIGN_BUDGET'
                                break
                        if stop:
                            break
                    timed = [r for r in group['attempts'] if r.get('phase') == 'timed']
                    accepted = [r for r in timed if r.get('scientific_status') == 'PASS_SCOPED']
                    group.update(timed_passes=len(accepted), timed_required=TIMED_REPEATS,
                                 status='PASS_SCOPED' if len(timed) == TIMED_REPEATS and len(accepted) == TIMED_REPEATS else 'NOT_ELIGIBLE')
                    errors = [r['assessment']['actual_error'] for r in timed
                              if isinstance(r.get('assessment'), dict) and
                              isinstance(r['assessment'].get('actual_error'), (int, float))]
                    solve = sorted(r['adapter_reported_solve_seconds'] for r in accepted
                                   if isinstance(r.get('adapter_reported_solve_seconds'), (int, float)))
                    group.update(max_actual_error=max(errors) if errors else None,
                        reference_uncertainty_U=case['uncertainty_U'], reference_provenance=case['reference_provenance'],
                        endpoint=case['t_end'], norm='maximum component absolute endpoint error',
                        median_solve_seconds=solve[2] if group['status'] == 'PASS_SCOPED' and len(solve) == TIMED_REPEATS else None)
                    if stop:
                        break
                if stop:
                    break
        except BudgetExhausted as exc:
            stopped_budget = True
            failure = {'type': type(exc).__name__, 'message': str(exc)}
        except BaseException as exc:
            failure = {'type': type(exc).__name__, 'message': str(exc), 'traceback': traceback.format_exc()}
            raise
        finally:
            elapsed = time.monotonic()-campaign_start
            # These records are written immediately after every started child,
            # before assessment/hash capture can fail. Include setup and an
            # unfinished current group rather than relying on completed groups.
            process_records = [{'path': str(path), **json.loads(path.read_text())}
                               for path in sorted(output.glob('**/PROCESS_COST.json'))]
            child_wall = sum(row['elapsed_seconds'] for row in process_records)
            setup_wall = sum(row['elapsed_seconds'] for row in process_records if '/setup/' in row['path'])
            charged = max(elapsed, child_wall)
            attempts = [a for group in rows for a in group['attempts']]
            summary = {'schema': 'vigilode.julia-comparison-summary.v1',
                'plan_sha256': hashlib.sha256(plan_bytes).hexdigest(),
                'started_utc': started_utc, 'finished_utc': utc_now().isoformat(),
                'campaign_wall_seconds': elapsed, 'charged_seconds': charged,
                'controller_exclusive_seconds': max(0.0, elapsed-child_wall),
                'observed_child_process_wall_seconds_total': child_wall,
                'setup_process_wall_seconds_total': setup_wall,
                'attempt_process_wall_seconds_total': child_wall-setup_wall,
                'attempt_process_group_cpu_seconds_total': sum(a.get('process_group_cpu_seconds') or 0.0 for a in attempts),
                'setup_process_group_cpu_seconds_total': sum(r.get('process_group_cpu_seconds') or 0.0 for r in setup_records),
                'process_records': process_records, 'setup_records': setup_records,
                'first_failure': failure or next((r['first_failure'] for r in process_records if r.get('first_failure')), None),
                'explicit_import_probe_wall_seconds': next((r['process_wall_seconds'] for r in setup_records
                                                            if r['label'] == 'pinned-import-probe'), None),
                'import_probe_is_separate_process': True,
                'per_attempt_timing': {'adapter_process_wall_includes_startup_import_and_JIT': True,
                    'adapter_reported_warmup_seconds': 'reported by adapter; compilation may be included',
                    'adapter_reported_solve_seconds': 'reported by adapter',
                    'method_jit_seconds': 'UNATTRIBUTED', 'setup_residual_seconds': 'UNATTRIBUTED'},
                'budget_seconds': requested, 'admitted_execution_budget_seconds': budget.execution_budget,
                'global_reserved_seconds': budget.reserved, 'cost_root': str(cost_root),
                'global_cap_seconds': GLOBAL_CAP_SECONDS, 'deadline_utc': GLOBAL_DEADLINE,
                'budget_exhausted': stopped_budget, 'timing_authority': 'diagnostic_only',
                'methods': list(METHODS), 'cases': CASES, 'warmup_repeats': WARMUP_REPEATS,
                'timed_repeats': TIMED_REPEATS, 'error_limit': ERROR_LIMIT,
                'runner_sha256': sha256_file(Path(__file__)),
                'reference_norm': 'maximum component absolute endpoint error', 'groups': rows,
                'speed_comparison': {'status': 'NOT_COMPUTED', 'reason': 'no same-host, same-source comparison cohort supplied',
                                     'required_per_member_condition': 'actual_error + U <= 1e-4 for every timed member',
                                     'speed_ratios': {}},
                'historical_native_diffsol_context': plan.get('historical_cohort_context', {'status': 'NOT_SUPPLIED'})}
            write_new(output/'SUMMARY.json', summary)
            try:
                budget.finish(output/'SUMMARY.json', charged)
            except BaseException as exc:
                write_new(output/'COST_EVENT_FAILURE.json', {'type': type(exc).__name__, 'message': str(exc),
                    'first_failure_preserved': failure, 'reservation_remains_authoritative': True})
                if failure is None:
                    raise
        return summary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    summary = run_campaign(args.plan, args.output)
    print(json.dumps({'summary': str(args.output.resolve()/'SUMMARY.json'),
                      'groups': len(summary['groups']),
                      'budget_exhausted': summary['budget_exhausted']}, sort_keys=True))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
