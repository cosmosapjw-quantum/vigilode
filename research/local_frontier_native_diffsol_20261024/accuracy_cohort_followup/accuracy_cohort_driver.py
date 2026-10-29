#!/usr/bin/env python3
"""Source-locked, serial accuracy cohort executor; requires independent approval.

The V3 selection is planning input. A materialized plan and a separate hash-bound
approval are required before any benchmark child is launched.
"""
from __future__ import annotations

import time
PROCESS_STARTED = time.monotonic()

import argparse
import ctypes
import importlib.util
import datetime as dt
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import signal
import subprocess
import uuid

# Source copies are evidence; moving one must never move the canonical cost root.
ROOT = Path('/tmp/vigilode-local-run-20261024.geLVCp9Q/continuation_20261025T080718Z')
COST_HELPER_PATH = ROOT/'reference_launch_controller.py'
COMPARE_PATH = Path('/tmp/vigilode-independent-report.Iavhh9Az/checkout/research/local_frontier_native_diffsol_20261024/compare.py')
COMPARE_SHA256 = 'd74c2732d70871349574c3c84fdbf5637e459fe59a50df66a4f0179c85065396'
COST_HELPER_LOADED_SHA256 = hashlib.sha256(COST_HELPER_PATH.read_bytes()).hexdigest()
DRIVER_LOADED_SHA256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
cost = None
comparison = None
SELECTION = ROOT / 'ACCURACY_COHORT_SELECTION_V3.json'
DEADLINE = '2026-10-27T08:07:00Z'
GLOBAL_CAP = 172800.0
COHORT_CAP = 16200.0
ATTEMPT_SECONDS = 180.0
CONTROLLER_RESERVE = 3240.0
ATTEMPT_RESERVE = ATTEMPT_SECONDS
MAX_RSS_BYTES = 12*1024**3
FINALIZE_RESERVE = 20.0
SCHEMA = 'vigilode.accuracy-cohort-plan.v3'
APPROVAL_SCHEMA = 'vigilode.accuracy-cohort-approval.v3'
STOP_SIGNAL = None
ACTIVE_PROC = None


def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()


def new_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as f:
        json.dump(value, f, indent=2, sort_keys=True, allow_nan=False)
        f.write('\n')
        f.flush()
        os.fsync(f.fileno())


def plan_core_sha(plan):
    return hashlib.sha256(json.dumps({k: v for k, v in plan.items() if k != 'approval_receipt'},
        sort_keys=True, separators=(',', ':'), allow_nan=False).encode()).hexdigest()


def git_identity(repo):
    def get(*args):
        return subprocess.check_output(['git', '-C', str(repo), *args], text=True,
                                       timeout=10).strip()
    return {'head': get('rev-parse', 'HEAD'),
            'tree': get('rev-parse', 'HEAD^{tree}'),
            'status': get('status', '--porcelain=v1', '--untracked-files=all')}


def finite_number(value):
    if type(value) not in (int, float):
        return False
    try:
        return math.isfinite(value)
    except (OverflowError, ValueError):
        return False


def verify_executor(plan):
    expected = plan.get('executor', {})
    if (Path(expected.get('driver_path', '')).resolve() != Path(__file__).resolve() or
            expected.get('driver_sha256') != DRIVER_LOADED_SHA256 or
            sha(__file__) != DRIVER_LOADED_SHA256 or
            Path(expected.get('cost_helper_path', '')).resolve() != COST_HELPER_PATH.resolve() or
            (cost is not None and Path(cost.__file__).resolve() != COST_HELPER_PATH.resolve()) or
            expected.get('cost_helper_sha256') != COST_HELPER_LOADED_SHA256 or
            sha(COST_HELPER_PATH) != COST_HELPER_LOADED_SHA256):
        raise ValueError('executor driver or imported cost-helper identity mismatch')
    if (Path(expected.get('compare_path', '')).resolve() != COMPARE_PATH.resolve() or
            expected.get('compare_sha256') != COMPARE_SHA256 or sha(COMPARE_PATH) != COMPARE_SHA256):
        raise ValueError('executor comparison tool path or byte identity mismatch')


def source_module(name, path):
    # Compile verified source bytes directly; do not write caches into frozen evidence.
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    exec(compile(Path(path).read_bytes(), str(path), 'exec'), module.__dict__)
    return module


def load_cost_helper(plan):
    global cost
    verify_executor(plan)
    if cost is None:
        cost = source_module('accuracy_reference_cost_helper', COST_HELPER_PATH)
    verify_executor(plan)
    return cost


def load_comparison():
    global comparison
    if sha(COMPARE_PATH) != COMPARE_SHA256:
        raise ValueError('comparison tool byte identity mismatch')
    if comparison is None:
        comparison = source_module('accuracy_locked_comparison', COMPARE_PATH)
    return comparison


def admitted_reference(binding):
    if sha(binding['path']) != binding['sha256']:
        raise ValueError('reference file SHA mismatch')
    reference = json.loads(Path(binding['path']).read_text())
    dimension = reference.get('model_descriptor', {}).get('model_contract', {}).get('dimension')
    state, uncertainty = reference.get('reference_state'), reference.get('uncertainty')
    if (reference.get('model_descriptor_sha256') != binding['descriptor_sha256'] or
            reference.get('norm') != 'max_component_absolute' or
            type(dimension) is not int or dimension <= 0 or not isinstance(state, list) or
            len(state) != dimension or not all(finite_number(x) for x in state) or
            not finite_number(uncertainty) or not 0 <= uncertainty <= 1e-5):
        raise ValueError('reference full-vector or uncertainty contract mismatch')
    return reference


def verify_envelope(plan):
    # No subprocess runs before a hash-bound approved envelope and cost reservation.
    if (plan.get('schema') != SCHEMA or plan.get('status') != 'APPROVED_MATERIALIZED_PLAN' or
            plan.get('deadline_utc') != DEADLINE or plan.get('global_cap_seconds') != GLOBAL_CAP or
            plan.get('cohort_cap_seconds') != COHORT_CAP):
        raise ValueError('plan schema or hard caps mismatch')
    if Path(plan.get('cost_root', '')).resolve() != ROOT.resolve():
        raise ValueError('cost root mismatch')
    if Path(plan.get('shared_lock_path', '')).resolve() != (ROOT/'.julia_comparison_campaign.lock').resolve():
        raise ValueError('shared execution lock mismatch')
    if Path(plan.get('cost_events_path', '')).resolve() != (ROOT/'COST_EVENTS.jsonl').resolve():
        raise ValueError('cost events path mismatch')
    if Path(plan.get('output_root', '')).resolve().parent != ROOT.resolve():
        raise ValueError('output must be exclusive continuation child')
    source, refs = plan.get('source', {}), plan.get('references', {})
    approval_ref = plan.get('approval_receipt', {})
    if sha(approval_ref['path']) != approval_ref['sha256']:
        raise ValueError('approval receipt SHA mismatch')
    approval = json.loads(Path(approval_ref['path']).read_text())
    if (approval.get('schema') != APPROVAL_SCHEMA or
            approval.get('decision') != 'APPROVED_ACCURACY_COHORT_V3' or
            approval.get('plan_core_sha256') != plan_core_sha(plan) or
            approval.get('source_head') != source.get('head') or
            approval.get('source_tree') != source.get('tree') or
            approval.get('binary_sha256') != source.get('binary_sha256') or
            approval.get('admitted_reference_sha256') !=
            {case: refs[case]['sha256'] for case in ('2', '3', '5')}):
        raise ValueError('independent approval does not bind plan')


def verify_selection(plan, *, selection_path=None):
    selection_path = SELECTION if selection_path is None else selection_path
    if sha(selection_path) != plan.get('selection_sha256'):
        raise ValueError('V3 selection byte identity mismatch')
    selection = json.loads(Path(selection_path).read_text())
    if (selection.get('schema') != 'vigilode.local-continuation-accuracy-cohort-selection-template.v3' or
            not isinstance(selection.get('groups'), list) or len(selection['groups']) != 12 or
            not isinstance(selection.get('attempts'), list) or len(selection['attempts']) != 72):
        raise ValueError('V3 selection contract mismatch')
    source = plan.get('source', {})
    if not all(source.get(x) for x in ('path', 'head', 'tree', 'cargo_lock_sha256', 'binary_path', 'binary_sha256')):
        raise ValueError('unbound source identity')
    if source['path'] != selection['frozen_source_path'] or source['binary_path'] != selection['binary_path']:
        raise ValueError('source path differs from selection')
    refs = plan.get('references', {})
    if set(refs) != {'2', '3', '5'} or set(selection.get('references', {})) != set(refs):
        raise ValueError('missing or extra reference')
    for case, expected in selection['references'].items():
        got = refs[case]
        if got != {k: expected[k] for k in ('path', 'sha256', 'descriptor_sha256')}:
            raise ValueError('reference binding differs from selection')
        admitted_reference(got)
        if sha(expected['frozen_binary_describe_path']) != got['descriptor_sha256']:
            raise ValueError('saved frozen binary descriptor mismatch')
    methods = ('dopri5', 'rodas5p-mf', 'rodas5p-dense', 'switching')
    expected_groups = [(case, method, f'c{case:02d}_{method}_ensemble_t1')
                       for case in (2, 3, 5) for method in methods]
    if [g.get('group_id') for g in selection['groups']] != [x[2] for x in expected_groups]:
        raise ValueError('selection group identity or uniqueness mismatch')
    expected_ids = []
    thread_vars = ('BLIS_NUM_THREADS', 'MKL_NUM_THREADS', 'OMP_NUM_THREADS',
                   'OPENBLAS_NUM_THREADS', 'RAYON_NUM_THREADS', 'VECLIB_MAXIMUM_THREADS')
    for group, (case, method, gid) in zip(selection['groups'], expected_groups):
        ids = [gid+'_warmup', *[gid+f'_timed{i}' for i in range(5)]]
        metadata = {'case': case, 'method': method, 'layer': 'ensemble', 'threads': 1,
                    'rtol': 1e-6, 'atol': 1e-9, 'error_limit': 1e-4, 'max_steps': 200000,
                    'ensemble_count': 256, 'reference_sha256': refs[str(case)]['sha256']}
        if group.get('attempt_ids') != ids or any(group.get(k) != v for k, v in metadata.items()):
            raise ValueError('selection group membership or metadata mismatch')
        expected_ids.extend(ids)
    attempts = selection['attempts']
    if [a.get('attempt_id') for a in attempts] != expected_ids or len(set(expected_ids)) != 72:
        raise ValueError('selection attempt coverage or uniqueness mismatch')
    for index, item in enumerate(attempts):
        case, method, gid = expected_groups[index//6]
        repeat = None if index % 6 == 0 else index % 6-1
        ref = refs[str(case)]
        argv = [source['binary_path'], '--case', str(case), '--size', 'standard', '--method', method,
                '--parallel-layer', 'ensemble', '--threads', '1', '--rtol', '1e-6', '--atol', '1e-9',
                '--error-limit', '1e-4', '--max-steps', '200000', '--ensemble-count', '256',
                '--reference-json', ref['path'], '--reference-sha256', ref['sha256'], '--emit-state']
        if (item.get('group_id') != gid or type(item.get('case')) is not int or item['case'] != case or
                item.get('method') != method or type(item.get('threads')) is not int or item['threads'] != 1 or
                item.get('cpus') != [0] or item.get('warmup') is not (repeat is None) or
                item.get('timed_repeat') != repeat or
                (repeat is not None and type(item.get('timed_repeat')) is not int) or
                item.get('max_rss_bytes') != MAX_RSS_BYTES or item.get('hard_timeout_seconds') != ATTEMPT_SECONDS or
                item.get('stdout_limit_bytes') != 4*1024**2 or item.get('stderr_limit_bytes') != 4*1024**2 or
                any(item.get('env_overrides', {}).get(v) != '1' for v in thread_vars) or
                item.get('env_overrides', {}).get('VIGILODE_SOURCE_COMMIT') != source['head'] or item.get('argv') != argv):
            raise ValueError('attempt command or resource contract mismatch')
    return selection


def verify_plan(plan, *, selection_path=None):
    verify_executor(plan)
    verify_envelope(plan)
    selection = verify_selection(plan, selection_path=selection_path)
    source = plan['source']
    if git_identity(source['path']) != {'head': source['head'], 'tree': source['tree'], 'status': ''}:
        raise ValueError('source checkout identity mismatch')
    if sha(Path(source['path'])/'Cargo.lock') != source['cargo_lock_sha256'] or sha(source['binary_path']) != source['binary_sha256']:
        raise ValueError('Cargo.lock or binary identity mismatch')
    if 0 not in os.sched_getaffinity(0):
        raise ValueError('CPU0 unavailable')
    return selection


def remaining_deadline():
    return (dt.datetime.fromisoformat(DEADLINE.replace('Z', '+00:00'))-
            dt.datetime.now(dt.timezone.utc)).total_seconds()


def read_terminal(folder):
    terminal = folder/'TERMINAL.json'
    if terminal.exists():
        record = json.loads(terminal.read_text())
        proof = record.get('process_group_cleanup', {})
        if proof.get('group_dead') is not True or proof.get('leader_reaped') is not True:
            raise ValueError(f'unresolved prior process group requires reconciliation: {folder}')
        pgid = record.get('pid')
        if pgid is not None and process_group(pgid)[1]:
            raise ValueError(f'unresolved prior process group remains: {folder}')
        return record
    if folder.exists():
        raise ValueError(f'unfinished attempt requires independent reconciliation: {folder}')
    return None


def assess(stdout, item, reference, source):
    try:
        admitted = admitted_reference(reference)
        descriptor = admitted['model_descriptor']
        endpoint = descriptor['t_end']
        if (descriptor.get('case_id') != item['case'] or descriptor.get('size') != 'standard' or
                not finite_number(endpoint) or not finite_number(descriptor['t_start']) or
                descriptor['model_contract']['t_span'] != [descriptor['t_start'], endpoint]):
            raise ValueError('reference descriptor endpoint mismatch')
    except (ValueError, OSError, KeyError, TypeError, AttributeError, OverflowError):
        return {'status': 'OUTPUT_CONTRACT_FAILURE', 'reason': 'SHA-bound reference contract'}
    records = []
    for line in Path(stdout).read_text(errors='replace').splitlines():
        try:
            records.append(json.loads(line))
        except ValueError:
            continue
    headers = [r for r in records if isinstance(r, dict) and r.get('kind') == 'header']
    results = [r for r in records if isinstance(r, dict) and r.get('kind') == 'result']
    failure = {'status': 'OUTPUT_CONTRACT_FAILURE', 'reason': 'header/result identity or endpoint contract'}
    if len(headers) != 1 or len(results) != 1:
        return failure
    header, r = headers[0], results[0]
    common = {'schema_version': 1, 'method': item['method'], 'parallel_layer': 'ensemble',
              'threads': 1, 'ensemble_count': 256, 'instrumented': False, 'error_limit': 1e-4}
    header_expected = dict(common, model_contract=descriptor['model_contract'],
                           source_commit=source['head'], rtol=1e-6, atol=1e-9, max_steps=200000)
    result_expected = dict(common, case=item['case'], size='standard',
        dimension=len(admitted['reference_state']), name=descriptor['model_contract']['name'],
        variant=descriptor['model_contract']['variant'], reference_sha256=reference['sha256'],
        model_descriptor_sha256=reference['descriptor_sha256'])
    if (any(type(header.get(k)) is not int for k in ('schema_version', 'threads', 'ensemble_count', 'max_steps')) or
            any(type(r.get(k)) is not int for k in ('schema_version', 'case', 'dimension', 'threads', 'ensemble_count')) or
            any(header.get(k) != v for k, v in header_expected.items()) or
            any(r.get(k) != v for k, v in result_expected.items()) or
            header.get('instrumented') is not False or r.get('instrumented') is not False or
            not finite_number(r.get('final_time'))):
        return failure
    successes = r.get('member_successes')
    if type(successes) is not int or not 0 <= successes <= 256:
        return failure
    if r.get('status') == 'SOLVER_FAILED' and r.get('solver_success') is False:
        return {'status': 'SOLVER_FAILED', 'member_successes': successes, 'reported_status': 'SOLVER_FAILED'}
    e, u, difference = r.get('actual_error'), r.get('reference_absolute_bound'), r.get('endpoint_reference_difference')
    state = r.get('final_state')
    if (successes != 256 or r.get('accuracy_scope') !=
            'maximum component absolute endpoint error across every ensemble member; no continuous/phase-global certificate' or
            not all(finite_number(x) and x >= 0 for x in (e, u, difference)) or
            not finite_number(difference+u) or abs(e-(difference+u)) > 8*math.ulp(max(1.0, e)) or
            u != admitted['uncertainty'] or type(r.get('maximum_error_member_index')) is not int or
            not 0 <= r['maximum_error_member_index'] < 256 or not isinstance(state, list) or
            len(state) != len(admitted['reference_state']) or not all(finite_number(x) for x in state)):
        return failure
    # Both paths subtract the same parsed f64 endpoints. No extra state tolerance is needed.
    first_difference = max(abs(a-b) for a, b in zip(state, admitted['reference_state']))
    if not finite_number(first_difference) or first_difference > difference:
        return {'status': 'OUTPUT_CONTRACT_FAILURE', 'reason': 'first-member difference exceeds all-member witness'}
    # Invoke the hash-locked report comparison's unchanged endpoint/finite-state policy.
    first = load_comparison().assess(r, descriptor, admitted['reference_state'], u, 1e-4, 'COMPLETED')
    if first['status'] not in ('PASS_SCOPED', 'ERROR_CAP_FAILURE', 'SOLVER_FAILURE'):
        return failure
    if r.get('solver_success') is not True:
        status = 'SOLVER_FAILED'
    elif max(e, difference+u) > 1e-4:
        status = 'ACCURACY_FAILED'
    elif r.get('status') == 'PASS_SCOPED' and first['status'] == 'PASS_SCOPED':
        status = 'PASS_SCOPED'
    elif r.get('status') in ('SOLVER_FAILED', 'ACCURACY_FAILED'):
        status = r['status']
    else:
        status = 'OUTPUT_CONTRACT_FAILURE'
    return {'status': status, 'actual_error': e, 'reference_absolute_bound': u,
            'endpoint_reference_difference': difference, 'first_member_difference': first_difference,
            'reported_status': r.get('status')}


def stop_signal(signum, _frame):
    # Keep the shared execution lock held. Normal control flow cleans up and records proof.
    global STOP_SIGNAL
    STOP_SIGNAL = signum


def process_group(pgid):
    rss, members = 0, []
    page = os.sysconf('SC_PAGE_SIZE')
    for stat_path in Path('/proc').glob('[0-9]*/stat'):
        try:
            raw = stat_path.read_text()
            rest = raw[raw.rfind(')')+2:].split()
            if int(rest[2]) != pgid:
                continue
            members.append(int(stat_path.parent.name))
            rss += int(stat_path.with_name('statm').read_text().split()[1])*page
        except FileNotFoundError:
            continue  # Process exited during this observation.
        except ProcessLookupError:
            continue
    return rss, members


def set_subreaper(value=None):
    # /usr/bin/time cannot reap its child after group SIGKILL; adopt and reap that child.
    libc = ctypes.CDLL(None, use_errno=True)
    previous = ctypes.c_int()
    if libc.prctl(37, ctypes.byref(previous), 0, 0, 0) != 0:
        raise OSError(ctypes.get_errno(), 'PR_GET_CHILD_SUBREAPER failed')
    if value is not None and libc.prctl(36, int(value), 0, 0, 0) != 0:
        raise OSError(ctypes.get_errno(), 'PR_SET_CHILD_SUBREAPER failed')
    return previous.value


def kill_group(proc):
    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    proc.wait(timeout=5)
    end = time.monotonic()+5
    while True:
        while True:
            try:
                child, _ = os.waitpid(-proc.pid, os.WNOHANG)
            except ChildProcessError:
                break
            if child == 0:
                break
        _, members = process_group(proc.pid)
        if not members or time.monotonic() >= end:
            break
        time.sleep(0.01)
    return {'pgid': proc.pid, 'leader_reaped': proc.returncode is not None,
            'group_dead': not members, 'remaining_pids': members,
            'observed_utc': dt.datetime.now(dt.timezone.utc).isoformat()}


def execute_attempt(item, plan, reference, folder, *, campaign_deadline=None):
    global ACTIVE_PROC
    start = time.monotonic()
    source = plan['source']
    verify_executor(plan)
    if sha(source['binary_path']) != source['binary_sha256'] or \
            sha(Path(source['path'])/'Cargo.lock') != source['cargo_lock_sha256'] or \
            sha(reference['path']) != reference['sha256']:
        raise ValueError('pre-attempt immutable input mutation')
    folder.mkdir(parents=True, exist_ok=False)
    new_json(folder/'START.json', {'schema': 'vigilode.accuracy-attempt-start.v3',
        'attempt_id': item['attempt_id'], 'selection_sha256': plan['selection_sha256'],
        'source': source, 'executor': plan['executor'], 'argv': item['argv'], 'reference': reference,
        'started_utc': dt.datetime.now(dt.timezone.utc).isoformat()})
    env = {**os.environ, **item['env_overrides'], 'VIGILODE_SOURCE_COMMIT': source['head']}
    stdout, stderr, resource = (folder/n for n in ('stdout.bin', 'stderr.bin', 'resource.txt'))
    status, exit_code, pid, proc = 'SPAWN_FAILED', None, None, None
    previous_subreaper = None
    maximum_rss = maximum_count = 0
    proof = {'group_dead': True, 'leader_reaped': True, 'remaining_pids': [], 'no_child_spawned': True}
    try:
        with stdout.open('xb') as out, stderr.open('xb') as err, \
                (folder/'PROCESS_GROUP_SAMPLES.jsonl').open('x') as samples:
            cmd = ['/usr/bin/time', '-v', '-o', str(resource), '--', *item['argv']]
            def pin():
                os.sched_setaffinity(0, {0})
                if os.sched_getaffinity(0) != {0}:
                    raise RuntimeError('CPU0 affinity failed')
            if STOP_SIGNAL is not None:
                status = 'INTERRUPTED'
            elif remaining_deadline() <= FINALIZE_RESERVE:
                status = 'TIMEOUT'
            else:
                previous_subreaper = set_subreaper(True)
                proc = subprocess.Popen(cmd, cwd=source['path'], env=env, stdout=out,
                    stderr=err, start_new_session=True, preexec_fn=pin)
                ACTIVE_PROC = proc
                pid = proc.pid
                new_json(folder/'PROCESS.json', {'pid': pid, 'pgid': pid, 'cpu0_requested': True,
                    'actual_affinity': sorted(os.sched_getaffinity(pid)), 'subreaper_enabled': True})
                while True:
                    exit_code = proc.poll()
                    rss, members = process_group(pid)
                    maximum_rss, maximum_count = max(maximum_rss, rss), max(maximum_count, len(members))
                    samples.write(json.dumps({'elapsed_seconds': time.monotonic()-start,
                        'process_group_rss_bytes_sum': rss, 'process_group_process_count': len(members)})+'\n')
                    samples.flush()
                    if STOP_SIGNAL is not None:
                        status = 'FAILED_BUDGET' if STOP_SIGNAL == signal.SIGALRM else 'INTERRUPTED'
                    elif (stdout.stat().st_size > item['stdout_limit_bytes'] or
                            stderr.stat().st_size > item['stderr_limit_bytes']):
                        status = 'OUTPUT_LIMIT'
                    elif rss > item['max_rss_bytes']:
                        status = 'MEMORY_LIMIT'
                    elif time.monotonic()-start >= ATTEMPT_SECONDS or remaining_deadline() <= FINALIZE_RESERVE:
                        status = 'TIMEOUT'
                    elif campaign_deadline is not None and time.monotonic() >= campaign_deadline:
                        status = 'FAILED_BUDGET'
                    elif exit_code is not None:
                        status = 'COMPLETED' if exit_code == 0 else 'NONZERO_EXIT'
                    if status != 'SPAWN_FAILED':
                        break
                    time.sleep(0.1)
    except Exception as error:
        status = 'SPAWN_OR_CONTROLLER_FAILURE'
        stdout.touch(exist_ok=True)
        with stderr.open('ab') as err:
            err.write((type(error).__name__+': '+str(error)+'\n').encode())
    finally:
        if proc is not None:
            try:
                proof = kill_group(proc)
            except Exception as error:
                proof = {'group_dead': False, 'leader_reaped': proc.returncode is not None,
                         'remaining_pids': [], 'error': type(error).__name__+': '+str(error)}
            exit_code = proc.returncode
            if proof['group_dead']:
                ACTIVE_PROC = None
        if previous_subreaper is not None and proof['group_dead']:
            set_subreaper(previous_subreaper)
        stdout.touch(exist_ok=True); stderr.touch(exist_ok=True); resource.touch(exist_ok=True)
    elapsed = time.monotonic()-start
    try:
        verify_executor(plan)
        mutation = (sha(source['binary_path']) != source['binary_sha256'] or
                    sha(Path(source['path'])/'Cargo.lock') != source['cargo_lock_sha256'] or
                    sha(reference['path']) != reference['sha256'])
    except (ValueError, OSError):
        mutation = True
    try:
        assessment = ({'status': 'UNRESOLVED_PROCESS_GROUP'} if not proof['group_dead'] else
                      {'status': 'IMMUTABLE_INPUT_MUTATED'} if mutation else
                      assess(stdout, item, reference, source) if status == 'COMPLETED' and exit_code == 0 else {'status': status})
    except Exception as error:
        assessment = {'status': 'CONTROLLER_ASSESSMENT_FAILURE',
                      'reason': type(error).__name__+': '+str(error)}
    elapsed = time.monotonic()-start
    terminal = {'schema': 'vigilode.accuracy-attempt-terminal.v3', 'attempt_id': item['attempt_id'],
                'execution_status': status, 'exit_code': exit_code, 'pid': pid,
                'elapsed_seconds': elapsed, 'assessment': assessment, 'process_group_cleanup': proof,
                'maximum_process_group_rss_bytes': maximum_rss,
                'maximum_process_group_process_count': maximum_count,
                'stdout_sha256': sha(stdout), 'stderr_sha256': sha(stderr),
                'resource_sha256': sha(resource), 'stop_signal': STOP_SIGNAL,
                'finished_utc': dt.datetime.now(dt.timezone.utc).isoformat()}
    new_json(folder/'TERMINAL.json', terminal)
    return terminal


def event_for(invocation_id, phase, evidence, seconds, category):
    return {'schema': 'vigilode.local-continuation-cost-event.v1',
            'event_id': invocation_id+'-'+phase.lower(), 'attempt_id': invocation_id,
            'category': category, 'phase': phase,
            'charge_upper_seconds' if phase == 'STARTED' else 'charged_seconds': seconds,
            'evidence_path': str(evidence), 'evidence_sha256': sha(evidence),
            'observed_utc': dt.datetime.now(dt.timezone.utc).isoformat()}


def prior_cohort_charge(output, rows):
    # No old invocation or child may be silently resumed without terminal accounting.
    for start in (output/'accuracy_invocations').glob('*/START.json'):
        receipt = start.parent/'RECEIPT.json'
        if not receipt.is_file():
            raise ValueError('unfinished accuracy invocation requires reconciliation')
        if json.loads(receipt.read_text()).get('all_spawned_groups_resolved') is not True:
            raise ValueError('unresolved prior invocation process group requires reconciliation')
    for folder in (output/'runs').glob('*'):
        read_terminal(folder)
    relevant = [r for r in rows if Path(r.get('evidence_path', '')).resolve().is_relative_to(output.resolve())]
    charged, reserved = cost.event_accounting(relevant)
    if reserved:
        raise ValueError('unfinished accuracy cost reservation requires reconciliation')
    for receipt in (ROOT/'accuracy_rejections/controller').glob('invocation_*/RECEIPT.json'):
        charged += json.loads(receipt.read_text())['charged_seconds']
    return charged


def record_rejection(plan_path, started, reason, *, already_charged=0.0, status='REJECTED_BEFORE_COHORT'):
    # This stdlib-only path is independent of any rejected helper/envelope. Existing
    # cumulative_cost discovers controller/invocation_*/START.json and charges its receipt.
    invocation = ROOT/'accuracy_rejections/controller'/('invocation_'+uuid.uuid4().hex)
    new_json(invocation/'START.json', {'schema': 'vigilode.accuracy-rejection-start.v1',
        'plan_path': str(Path(plan_path).resolve()), 'reason': reason,
        'accounting_policy': 'canonical rejection record; no cohort or attempt reservation'})
    measured = max(0.0, time.monotonic()-started-already_charged)
    charge = measured+FINALIZE_RESERVE
    receipt = {'schema': 'vigilode.accuracy-rejection-receipt.v1', 'status': status,
        'reason': reason, 'measured_controller_seconds': measured, 'charged_seconds': charge,
        'terminal_accounting_allowance_seconds': FINALIZE_RESERVE,
        'already_charged_seconds': already_charged,
        'charge_basis': 'measured uncharged invocation time plus conservative terminal commit allowance',
        'all_spawned_groups_resolved': ACTIVE_PROC is None}
    new_json(invocation/'RECEIPT.json', receipt)
    return {'schema': 'vigilode.accuracy-cohort-summary.v3', 'status': status,
            'error': reason, 'groups': {}, 'rejection_receipt_path': str(invocation/'RECEIPT.json'),
            'timing_authority': 'diagnostic_only', 'old_source_speed_ratio': 'NOT_EVALUABLE'}


def append_cost_event(event):
    with (ROOT/'COST_EVENTS.lock').open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        cost.append_event(ROOT/'COST_EVENTS.jsonl', ROOT/'COST_EVENTS.lock', event, already_locked=True)


def execute(plan_path, *, started=None):
    global STOP_SIGNAL
    started = time.monotonic() if started is None else started
    STOP_SIGNAL = None
    state = {'admitted': False, 'child_posted': 0.0, 'controller_posted': 0.0}
    previous_handlers = {sig: signal.signal(sig, stop_signal) for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGALRM)}
    try:
        try:
            plan = json.loads(Path(plan_path).read_text())
            # Pure preflight precedes shared locks, helper execution, cohort mutation and reservations.
            verify_executor(plan)
            verify_envelope(plan)
            verify_selection(plan)
            load_cost_helper(plan)
            with (ROOT/'.julia_comparison_campaign.lock').open('a+') as exclusive:
                fcntl.flock(exclusive, fcntl.LOCK_EX | fcntl.LOCK_NB)
                result = execute_locked(plan_path, started, state)
                json.dumps(result, sort_keys=True, allow_nan=False)
                observed = time.monotonic()-started
                posted = state['child_posted']+state['controller_posted']
                if observed > posted:
                    adjustment = record_rejection(plan_path, started, 'terminal commit allowance exceeded',
                        already_charged=posted, status='FAILED_BUDGET')
                    result['status'] = 'FAILED_BUDGET'
                    result['terminal_adjustment_receipt_path'] = adjustment['rejection_receipt_path']
                return result
        except Exception as error:
            return record_rejection(plan_path, started, type(error).__name__+': '+str(error),
                already_charged=state['child_posted']+state['controller_posted'],
                status='FAILED_CONTROLLER' if state['admitted'] else 'REJECTED_BEFORE_COHORT')
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        for sig, handler in previous_handlers.items():
            signal.signal(sig, handler)


def execute_locked(plan_path, started, state):
    global ACTIVE_PROC
    plan = json.loads(Path(plan_path).read_text())
    verify_envelope(plan)
    output = Path(plan['output_root'])
    identity = {'plan_sha256': sha(plan_path), 'selection_sha256': plan['selection_sha256']}
    if output.exists() and json.loads((output/'IDENTITY.json').read_text()) != identity:
        raise ValueError('resume output identity mismatch')
    with (ROOT/'COST_EVENTS.lock').open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        rows, _ = cost.validate_event_chain(ROOT/'COST_EVENTS.jsonl')
        global_before = cost.cumulative_cost(ROOT, rows)
        cohort_before = prior_cohort_charge(output, rows)
        if (global_before+CONTROLLER_RESERVE+FINALIZE_RESERVE > GLOBAL_CAP or
                cohort_before+CONTROLLER_RESERVE+FINALIZE_RESERVE > COHORT_CAP or
                remaining_deadline() <= CONTROLLER_RESERVE+FINALIZE_RESERVE):
            raise ValueError('no controller reservation within global/cohort/deadline cap')
        if not output.exists():
            output.mkdir(mode=0o700)
            new_json(output/'IDENTITY.json', identity)
        invocation_id = 'accuracy-controller-'+uuid.uuid4().hex
        invocation = output/'accuracy_invocations'/invocation_id
        new_json(invocation/'START.json', {'invocation_id': invocation_id,
            'plan_path': str(Path(plan_path).resolve()), 'identity': identity,
            'controller_reservation_seconds': CONTROLLER_RESERVE,
            'prior_cohort_charged_seconds': cohort_before, 'prior_global_charged_seconds': global_before,
            'includes_preflight_resume_and_summary': True,
            'started_utc': dt.datetime.now(dt.timezone.utc).isoformat()})
        cost.append_event(ROOT/'COST_EVENTS.jsonl', ROOT/'COST_EVENTS.lock',
            event_for(invocation_id, 'STARTED', invocation/'START.json', CONTROLLER_RESERVE, 'controller'),
            already_locked=True)
    state['admitted'] = True
    child_charged = 0.0
    selection, error, stop_reason = None, None, None
    status = 'COMPLETED'
    campaign_deadline = started+min(COHORT_CAP-cohort_before, GLOBAL_CAP-global_before,
                                    remaining_deadline())-FINALIZE_RESERVE
    signal.setitimer(signal.ITIMER_REAL, max(0.001, campaign_deadline-time.monotonic()))
    try:
        selection = verify_plan(plan)
        for item in selection['attempts']:
            elapsed = time.monotonic()-started
            live_overhead = max(0.0, elapsed-child_charged)
            if STOP_SIGNAL is not None:
                status = 'FAILED_BUDGET' if STOP_SIGNAL == signal.SIGALRM else 'FAILED_INTERRUPTED'
                stop_reason = 'CONTROLLER_SIGNAL'; break
            if live_overhead+FINALIZE_RESERVE > CONTROLLER_RESERVE or time.monotonic() >= campaign_deadline:
                status = 'FAILED_BUDGET'; stop_reason = 'CONTROLLER_OR_COHORT_LIVE_CAP'; break
            folder = output/'runs'/item['attempt_id']
            prior = read_terminal(folder)
            if prior:
                rows, _ = cost.validate_event_chain(ROOT/'COST_EVENTS.jsonl')
                if {r['phase'] for r in rows if r['attempt_id'] == item['attempt_id']} != {'STARTED', 'TERMINAL'}:
                    raise ValueError('completed attempt lacks terminal global cost event')
                continue
            if not item['warmup']:
                warm = read_terminal(output/'runs'/f"{item['group_id']}_warmup")
                if warm is None:
                    raise ValueError('timed attempt lacks warmup')
                if warm['assessment']['status'] != 'PASS_SCOPED':
                    continue
            with (ROOT/'COST_EVENTS.lock').open('a+') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                verify_executor(plan)
                if (sha(plan['source']['binary_path']) != plan['source']['binary_sha256'] or
                        sha(Path(plan['source']['path'])/'Cargo.lock') != plan['source']['cargo_lock_sha256'] or
                        sha(plan['references'][str(item['case'])]['path']) != plan['references'][str(item['case'])]['sha256']):
                    raise ValueError('pre-reservation immutable input mutation')
                rows, _ = cost.validate_event_chain(ROOT/'COST_EVENTS.jsonl')
                elapsed = time.monotonic()-started
                live_overhead = max(0.0, elapsed-child_charged)
                # Replace this invocation's open overhead reservation by observed overhead.
                charged = cost.cumulative_cost(ROOT, rows)-CONTROLLER_RESERVE+live_overhead
                if (charged+ATTEMPT_RESERVE+FINALIZE_RESERVE > GLOBAL_CAP or
                        cohort_before+elapsed+ATTEMPT_RESERVE+FINALIZE_RESERVE > COHORT_CAP or
                        time.monotonic()+ATTEMPT_RESERVE >= campaign_deadline):
                    status = 'FAILED_BUDGET'; stop_reason = 'GLOBAL_OR_COHORT_COST_OR_DEADLINE_GATE'; break
                cost.append_event(ROOT/'COST_EVENTS.jsonl', ROOT/'COST_EVENTS.lock',
                    event_for(item['attempt_id'], 'STARTED', output/'IDENTITY.json', ATTEMPT_RESERVE, 'other'),
                    already_locked=True)
            terminal = execute_attempt(item, plan, plan['references'][str(item['case'])], folder,
                                       campaign_deadline=campaign_deadline)
            child_charged += terminal['elapsed_seconds']
            append_cost_event(event_for(item['attempt_id'], 'TERMINAL', folder/'TERMINAL.json', terminal['elapsed_seconds'], 'other'))
            state['child_posted'] += terminal['elapsed_seconds']
            if terminal['assessment']['status'] == 'CONTROLLER_ASSESSMENT_FAILURE':
                status = 'FAILED_CONTROLLER'; stop_reason = 'CONTROLLER_ASSESSMENT_FAILURE'; break
            if (not terminal['process_group_cleanup']['group_dead'] or
                    terminal['execution_status'] in ('INTERRUPTED', 'FAILED_BUDGET')):
                status = 'FAILED_BUDGET' if terminal['execution_status'] == 'FAILED_BUDGET' else 'FAILED_INTERRUPTED'
                stop_reason = terminal['assessment']['status']; break
    except BaseException as exc:
        error = type(exc).__name__+': '+str(exc)
        status = 'FAILED_CONTROLLER'
    finally:
        # Signal handlers only set flags, so cleanup and terminal accounting retain the lock.
        if ACTIVE_PROC is not None:
            try:
                proof = kill_group(ACTIVE_PROC)
                if proof['group_dead']:
                    ACTIVE_PROC = None
                else:
                    status = 'FAILED_UNRESOLVED_PROCESS_GROUP'
            except Exception as exc:
                status = 'FAILED_UNRESOLVED_PROCESS_GROUP'
                error = error or type(exc).__name__+': '+str(exc)
        elapsed = time.monotonic()-started
        measured_controller = max(0.0, elapsed-child_charged)
        controller_charge = measured_controller+FINALIZE_RESERVE
        if (controller_charge > CONTROLLER_RESERVE or cohort_before+elapsed+FINALIZE_RESERVE > COHORT_CAP or
                global_before+elapsed+FINALIZE_RESERVE > GLOBAL_CAP or remaining_deadline() <= 0):
            status = 'FAILED_BUDGET'
        if STOP_SIGNAL is not None and status != 'FAILED_UNRESOLVED_PROCESS_GROUP':
            status = 'FAILED_BUDGET' if STOP_SIGNAL == signal.SIGALRM else 'FAILED_INTERRUPTED'
        summary = {'schema': 'vigilode.accuracy-cohort-summary.v3', 'status': status,
            'plan_sha256': sha(plan_path), 'timing_authority': 'diagnostic_only',
            'old_source_speed_ratio': 'NOT_EVALUABLE', 'stop_reason': stop_reason,
            'error': error, 'groups': {}, 'stop_signal': STOP_SIGNAL}
        if selection is not None:
            summary['groups'] = summarize_groups(output, selection)
        new_json(invocation/'SUMMARY.json', summary)
        elapsed = time.monotonic()-started
        measured_controller = max(0.0, elapsed-child_charged)
        controller_charge = measured_controller+FINALIZE_RESERVE
        if STOP_SIGNAL is not None and status != 'FAILED_UNRESOLVED_PROCESS_GROUP':
            status = 'FAILED_BUDGET' if STOP_SIGNAL == signal.SIGALRM else 'FAILED_INTERRUPTED'
        if (controller_charge > CONTROLLER_RESERVE or cohort_before+elapsed+FINALIZE_RESERVE > COHORT_CAP or
                global_before+elapsed+FINALIZE_RESERVE > GLOBAL_CAP or remaining_deadline() <= 0):
            status = 'FAILED_BUDGET'
        if summary['status'] != status:
            summary['status'] = status
            new_json(invocation/'FINAL_SUMMARY.json', summary)
            elapsed = time.monotonic()-started
            measured_controller = max(0.0, elapsed-child_charged)
            controller_charge = measured_controller+FINALIZE_RESERVE
        receipt = {'schema': 'vigilode.accuracy-controller-receipt.v3', 'status': status,
            'invocation_id': invocation_id, 'outer_elapsed_seconds': elapsed,
            'child_charged_seconds': child_charged, 'charged_seconds': controller_charge,
            'measured_controller_seconds': measured_controller,
            'terminal_accounting_allowance_seconds': FINALIZE_RESERVE,
            'charge_basis': 'measured invocation remainder plus charged conservative terminal commit allowance',
            'includes_preflight_resume_and_summary': True,
            'terminal_accounting_reserve_seconds': FINALIZE_RESERVE,
            'all_spawned_groups_resolved': ACTIVE_PROC is None,
            'finished_utc': dt.datetime.now(dt.timezone.utc).isoformat()}
        new_json(invocation/'RECEIPT.json', receipt)
        append_cost_event(event_for(invocation_id, 'TERMINAL', invocation/'RECEIPT.json', controller_charge, 'controller'))
        state['controller_posted'] = controller_charge
        summary['controller_receipt_path'] = str(invocation/'RECEIPT.json')
    return summary


def summarize_groups(output, selection):
    groups = {}
    for group in selection['groups']:
        receipts = [json.loads((output/'runs'/aid/'TERMINAL.json').read_text())
                    if (output/'runs'/aid/'TERMINAL.json').exists() else None
                    for aid in group['attempt_ids']]
        warmup_failed = receipts[0] is not None and receipts[0]['assessment']['status'] != 'PASS_SCOPED'
        groups[group['group_id']] = {
            'status': ('PASS_SCOPED' if all(r and r['assessment']['status']=='PASS_SCOPED' for r in receipts)
                       else 'INCOMPLETE_OR_FAILED'),
            'attempts': [{'id': aid, 'status': r['assessment']['status'] if r else
                          'NOT_RUN_DEPENDENCY' if warmup_failed and i > 0 else 'NOT_RUN_BUDGET_OR_INTERRUPTED'}
                         for i, (aid, r) in enumerate(zip(group['attempt_ids'], receipts))]}
    return groups


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--plan', required=True, type=Path)
    args = parser.parse_args()
    result = execute(args.plan, started=PROCESS_STARTED)
    print(json.dumps(result, sort_keys=True))
    return 0 if result['status'] == 'COMPLETED' else 1


if __name__ == '__main__':
    raise SystemExit(main())
