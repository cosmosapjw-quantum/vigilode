#!/usr/bin/env python3
"""Receipt-preserving, source-locked selective continuation executor."""
import argparse
import datetime as dt
import fcntl
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time
import re
import math
import ctypes

DIAGNOSTIC_CATALOG_PATH = Path(
    '/tmp/vigilode-local-run-20261024.geLVCp9Q/continuation_20261025T080718Z/'
    'TIMEOUT_DIAGNOSTIC_RUNNER_TEMPLATE_V2_20261025.json')
DIAGNOSTIC_CATALOG_SHA256 = 'e1954a68f0558df75219394e0d4cfd27f7b563902ddffc1c03c5408ef419b4ea'
DIAGNOSTIC_WATCHDOG = {'poll_seconds': 5, 'idle_seconds': 120, 'hard_seconds': 300,
                       'maximum_samples': 180, 'rss_limit_bytes': 12*1024**3,
                       'reservation_seconds': 960, 'extension_enabled': False}

DEADLINE = '2026-10-27T08:07:00Z'
NEW_CAP_SECONDS = 172800.0
COST_ROOT = Path('/tmp/vigilode-local-run-20261024.geLVCp9Q/continuation_20261025T080718Z')
POLL_SECONDS = 5.0
IDLE_SECONDS = 120.0
DIAGNOSTIC_SECONDS = 300.0
MAX_SECONDS = 900.0
MAX_SAMPLES = 180
ATTEMPT_RESERVE_SECONDS = MAX_SECONDS + 60.0
THREAD_ENVIRONMENT = {key: '1' for key in (
    'RAYON_NUM_THREADS', 'OMP_NUM_THREADS', 'OPENBLAS_NUM_THREADS',
    'MKL_NUM_THREADS', 'VECLIB_MAXIMUM_THREADS', 'NUMEXPR_NUM_THREADS',
    'BLIS_NUM_THREADS', 'JULIA_NUM_THREADS')}


def utc_now():
    return dt.datetime.now(dt.timezone.utc)


def deadline_time():
    return dt.datetime.fromisoformat(DEADLINE.replace('Z', '+00:00'))


def digest(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()


def digest_or_none(path):
    try:
        return digest(path)
    except OSError:
        return None


def write_new(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')


def canonical_sha(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':'),
                                     allow_nan=False).encode()).hexdigest()


def json_snapshot(path):
    """Hash and parse the same immutable read, never reopen between the two."""
    raw = Path(path).read_bytes()
    return json.loads(raw), hashlib.sha256(raw).hexdigest()


def git_identity(repo):
    def run(*args):
        return subprocess.check_output(['git', '-C', str(repo), *args], text=True).strip()
    return {'head': run('rev-parse', 'HEAD'), 'tree': run('rev-parse', 'HEAD^{tree}'),
            'status_porcelain': run('status', '--porcelain=v1')}


def validate_resources(task, *, check_host=True):
    original = task.get('original_task', {})
    cpus = task.get('cpus', original.get('cpus'))
    threads = task.get('threads', original.get('threads'))
    if not isinstance(cpus, list) or len(cpus) != 1 or type(cpus[0]) is not int or cpus[0] != 0:
        raise ValueError('CPU binding must be exactly CPU0')
    if type(threads) is not int or threads != 1:
        raise ValueError('configured thread count must be one')
    if original and (original.get('cpus') != [0] or
                     any(type(cpu) is not int for cpu in original.get('cpus', [])) or
                     type(original.get('threads')) is not int or original['threads'] != 1):
        raise ValueError('original CPU/thread binding mismatch')
    environment = task.get('environment', {})
    if not isinstance(environment, dict) or any(
            key in environment and environment[key] != value
            for key, value in THREAD_ENVIRONMENT.items()):
        raise ValueError('thread environment conflicts with one configured thread')
    command = task.get('command', [])
    for index, part in enumerate(command):
        if part in ('--threads', '--thread-count', '-t'):
            if index + 1 >= len(command) or command[index + 1] != '1':
                raise ValueError('command thread count must be one')
        elif any(part.startswith(flag + '=') for flag in ('--threads', '--thread-count', '-t')):
            if part.split('=', 1)[1] != '1':
                raise ValueError('command thread count must be one')
    if check_host:
        if not all(callable(getattr(os, name, None)) for name in ('sched_setaffinity', 'sched_getaffinity')):
            raise ValueError('CPU affinity APIs unavailable')
        try:
            available = os.sched_getaffinity(0)
        except OSError as error:
            raise ValueError('CPU affinity permission unavailable') from error
        if 0 not in available:
            raise ValueError('CPU0 is unavailable in controller affinity')


def spawn_pinned(task, command, *, unblock_child_signals=False, **kwargs):
    """Pin in the child, then retain its kernel-observed pre-exec resources."""
    read_fd, write_fd = os.pipe()

    def pin_child():
        if unblock_child_signals:
            signal.pthread_sigmask(signal.SIG_UNBLOCK, (signal.SIGINT, signal.SIGTERM, signal.SIGHUP))
        os.sched_setaffinity(0, {0})
        affinity = sorted(os.sched_getaffinity(0))
        thread_count = len(list(Path('/proc/self/task').iterdir()))
        if affinity != [0] or thread_count != 1:
            raise ValueError('child CPU/thread affinity validation failed')
        audit = {'pid': os.getpid(), 'cpu_affinity': affinity,
                 'os_thread_count': thread_count, 'observed_in_child_before_exec': True}
        os.write(write_fd, json.dumps(audit, sort_keys=True).encode())
        os.close(write_fd)

    proc = None
    try:
        try:
            proc = subprocess.Popen(command, start_new_session=True, cwd=task.get('cwd'),
                                    env={**os.environ, **task.get('environment', {}), **THREAD_ENVIRONMENT},
                                    preexec_fn=pin_child, pass_fds=(write_fd,), **kwargs)
        finally:
            os.close(write_fd)
        audit = json.loads(os.read(read_fd, 4096))
        if (audit.get('pid') != proc.pid or audit.get('cpu_affinity') != [0] or
                audit.get('os_thread_count') != 1 or audit.get('observed_in_child_before_exec') is not True):
            raise ValueError('invalid child affinity evidence')
        return proc, audit
    except Exception:
        if proc is not None:
            kill_reap_group(proc)
        raise
    finally:
        os.close(read_fd)


def process_resources(pid):
    try:
        fields = dict(line.split(':', 1) for line in (Path('/proc')/str(pid)/'status').read_text().splitlines()
                      if ':' in line)
        cpus = []
        for item in fields['Cpus_allowed_list'].strip().split(','):
            bounds = [int(value) for value in item.split('-')]
            cpus.extend(range(bounds[0], bounds[-1] + 1))
        return {'pid': pid, 'cpu_affinity': cpus, 'os_thread_count': int(fields['Threads']),
                'observed_after_exec': True}
    except (OSError, KeyError, ValueError):
        return None


def validate_cost_root(plan):
    if not plan.get('cost_root'):
        raise ValueError('missing new-cost root')
    root = Path(plan['cost_root']).resolve()
    if root != COST_ROOT.resolve():
        raise ValueError('canonical new-cost root identity mismatch')
    if not root.is_dir():
        raise ValueError('missing new-cost root')
    return root


def validate_plan(plan):
    if plan.get('schema') == 'vigilode.local-continuation-runner.v2':
        return validate_diagnostic_plan(plan)
    if plan.get('schema') != 'vigilode.local-continuation-runner.v1':
        raise ValueError('plan schema mismatch')
    if plan.get('deadline_utc') != DEADLINE or plan.get('new_cap_seconds') != NEW_CAP_SECONDS:
        raise ValueError('deadline or new cap mismatch')
    if not isinstance(plan.get('source'), dict) or not isinstance(plan.get('tasks'), list):
        raise ValueError('missing source or tasks')
    if 'retry_authorizations' in plan:
        raise ValueError('retry authorizations must be a separate immutable artifact')
    cost_root = validate_cost_root(plan)
    if not (cost_root/'COST_EVENTS.jsonl').is_file():
        raise ValueError('missing append-only external cost events')
    if not plan['tasks'] or len({task.get('id') for task in plan['tasks']}) != len(plan['tasks']):
        raise ValueError('empty or duplicate task ids')
    for label, item in plan['source'].items():
        if not isinstance(item, dict) or not all(item.get(key) for key in ('path', 'head', 'tree')):
            raise ValueError(f'bad source {label}')
        actual = git_identity(item['path'])
        if actual != {'head': item['head'], 'tree': item['tree'], 'status_porcelain': ''}:
            raise ValueError(f'source identity mismatch: {label}')
    for task in plan['tasks']:
        validate_resources(task)
        if not isinstance(task.get('id'), str) or not task['id'].isalnum():
            raise ValueError('bad task id')
        if not isinstance(task.get('command'), list) or not task['command'] or not all(isinstance(part, str) for part in task['command']):
            raise ValueError('bad command')
        if not task.get('binary') or digest(task['binary']) != task.get('binary_sha256'):
            raise ValueError('binary identity mismatch')
        if Path(task['command'][0]).resolve() != Path(task['binary']).resolve():
            raise ValueError('command binary mismatch')
        if not task.get('original_receipt') or digest(task['original_receipt']) != task.get('original_receipt_sha256'):
            raise ValueError('original receipt identity mismatch')
        original = json.loads(Path(task['original_receipt']).read_text())
        if original.get('task') != task.get('original_task'):
            raise ValueError('original task identity mismatch')
        if task.get('diagnostic_seconds', DIAGNOSTIC_SECONDS) != DIAGNOSTIC_SECONDS or task.get('max_seconds', MAX_SECONDS) != MAX_SECONDS:
            raise ValueError('timeout contract mismatch')
        target = task.get('physical_progress_target')
        if target is not None and (type(target) is not int or target <= 0):
            raise ValueError('bad progress target')
        if task.get('reference_json'):
            if not all(key in task for key in ('case', 'size', 'reference_sha256')):
                raise ValueError('reference binding incomplete')
            if task['size'] not in ('smoke', 'standard') or type(task['case']) is not int:
                raise ValueError('bad descriptor command')
            if not re.fullmatch('[0-9a-f]{64}', task['reference_sha256']):
                raise ValueError('bad reference digest')
            if digest(task['reference_json']) != task['reference_sha256']:
                raise ValueError('reference identity mismatch')
            if '--reference-json' in task['command'] or '--reference-sha256' in task['command']:
                raise ValueError('runner owns reference arguments')


def load_retry_manifest(path, plan_sha256):
    if path is None:
        return None, None
    path = Path(path)
    raw = path.read_bytes()
    manifest = json.loads(raw)
    if (manifest.get('schema') != 'vigilode.local-continuation-retry-manifest.v1'
            or manifest.get('plan_sha256') != plan_sha256
            or not isinstance(manifest.get('authorizations'), dict)):
        raise ValueError('retry manifest plan identity mismatch')
    return manifest, hashlib.sha256(raw).hexdigest()


def process_group_cpu_ticks(pgid):
    total = 0
    for path in Path('/proc').glob('[0-9]*/stat'):
        try:
            line = path.read_text()
            fields = line[line.rfind(')') + 2:].split()
            if int(fields[2]) == pgid:
                total += int(fields[11]) + int(fields[12])
        except (OSError, ValueError, IndexError):
            continue
    return total


def progress_snapshot(stdout_path, target):
    if target is None:
        return {'status': 'UNKNOWN', 'reason': 'no physical progress target'}
    latest = None
    try:
        with Path(stdout_path).open() as stream:
            for line in stream:
                try:
                    row = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if (isinstance(row, dict) and row.get('kind') == 'progress'
                        and type(row.get('accepted_steps')) is int and row['accepted_steps'] >= 0
                        and row.get('target_steps') == target):
                    latest = row['accepted_steps']
    except FileNotFoundError:
        pass
    return ({'status': 'OBSERVED', 'accepted_steps': latest, 'target_steps': target}
            if latest is not None else {'status': 'UNKNOWN', 'reason': 'no accepted-step telemetry'})


def projected_extension(progress, samples, elapsed):
    if progress['status'] != 'OBSERVED':
        return False
    accepted, target = progress['accepted_steps'], progress['target_steps']
    observed = [(row['elapsed_seconds'], row['physical_progress']['accepted_steps'])
                for row in samples if row['physical_progress']['status'] == 'OBSERVED'
                and row['physical_progress']['target_steps'] == target]
    if len(observed) < 2 or not 0 < accepted <= target:
        return False
    first = next(((when, steps) for when, steps in observed
                  if steps < accepted), None)
    if first is None or elapsed <= first[0]:
        return False
    rate = (accepted-first[1])/(elapsed-first[0])
    return rate > 0 and elapsed+(target-accepted)/rate <= MAX_SECONDS


def kill_reap_group(proc):
    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    proc.wait()
    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass


def descriptor_command(task):
    command = [task['binary'], '--describe', '--case', str(task['case']), '--size', task['size']]
    if task.get('end') is not None:
        command.extend(['--end', str(task['end'])])
    return command


def descriptor_probe(task, folder, label, clock=utc_now):
    """Return SHA of exact stdout bytes; retain every probe and its cost."""
    if clock() >= deadline_time():
        raise TimeoutError('hard deadline before descriptor probe')
    command = descriptor_command(task)
    started = time.monotonic()
    proc, affinity = spawn_pinned(task, command, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        output, error = proc.communicate(timeout=min(10.0, max(0.001, (deadline_time()-clock()).total_seconds())))
    except subprocess.TimeoutExpired:
        kill_reap_group(proc)
        output, error = proc.communicate()
        status = 'FAILED_DESCRIPTOR_PROBE'
    else:
        status = 'COMPLETED' if proc.returncode == 0 else 'FAILED_DESCRIPTOR_PROBE'
        kill_reap_group(proc)
    (folder/f'{label}.stdout').write_bytes(output)
    (folder/f'{label}.stderr').write_bytes(error)
    receipt = {'command': command, 'status': status, 'exit_code': proc.returncode,
               'observed_affinity_before_exec': affinity,
               'stdout_sha256': hashlib.sha256(output).hexdigest(),
               'stderr_sha256': hashlib.sha256(error).hexdigest(),
               'elapsed_process_seconds': time.monotonic()-started}
    write_new(folder/f'{label}.json', receipt)
    return receipt


def run_one(task, folder, *, clock=utc_now, monotonic=time.monotonic,
            sleeper=time.sleep, poll_seconds=POLL_SECONDS, plan_sha256=None,
            retry_evidence=None):
    validate_resources(task)
    folder = Path(folder)
    folder.mkdir(parents=True, exist_ok=False)
    if retry_evidence is not None:
        manifest_bytes, authorization_bytes, manifest_sha, authorization_sha = retry_evidence
        if (hashlib.sha256(manifest_bytes).hexdigest() != manifest_sha or
                hashlib.sha256(authorization_bytes).hexdigest() != authorization_sha):
            raise ValueError('retry evidence changed before attempt')
        (folder/'RETRY_MANIFEST.json').write_bytes(manifest_bytes)
        (folder/'RETRY_AUTHORIZATION.json').write_bytes(authorization_bytes)
    started = clock()
    command = list(task['command'])
    if task.get('reference_json'):
        command.extend(['--reference-json', task['reference_json'],
                        '--reference-sha256', task['reference_sha256']])
    write_new(folder/'START.json', {'task_id': task['id'], 'started_utc': started.isoformat(),
              'command': command, 'diagnostic_seconds': DIAGNOSTIC_SECONDS,
              'max_seconds': MAX_SECONDS, 'deadline_utc': DEADLINE,
              'configured_cpus': [0], 'configured_threads': 1,
              'thread_environment': THREAD_ENVIRONMENT,
              'plan_sha256': plan_sha256,
              'retry_manifest_sha256': retry_evidence[2] if retry_evidence else None,
              'retry_authorization_sha256': retry_evidence[3] if retry_evidence else None})
    start = monotonic()
    status, exit_code, samples, extended, spawn_error = None, None, [], False, None
    affinity, observed_resources = None, None
    reference_before = None
    if task.get('reference_json'):
        try:
            reference_before = digest(task['reference_json'])
        except OSError as error:
            status, spawn_error = 'FAILED_REFERENCE_BINDING', repr(error)
    descriptor_before = None
    descriptor_after = None
    if task.get('reference_json') and status is None:
        try:
            descriptor_before = descriptor_probe(task, folder, 'DESCRIPTOR_BEFORE', clock)
            reference = json.loads(Path(task['reference_json']).read_text())
            if (descriptor_before['status'] != 'COMPLETED' or
                    reference.get('schema') != 'vigilode.local-bottleneck.external-reference.v1' or
                    reference.get('model_descriptor_sha256') != descriptor_before['stdout_sha256'] or
                    reference.get('model_descriptor') != json.loads((folder/'DESCRIPTOR_BEFORE.stdout').read_text()) or
                    reference_before != task['reference_sha256']):
                status = 'FAILED_REFERENCE_BINDING'
        except (OSError, ValueError, json.JSONDecodeError, subprocess.SubprocessError, TimeoutError) as error:
            status, spawn_error = 'FAILED_REFERENCE_BINDING', repr(error)
    with (folder/'stdout.jsonl').open('xb') as stdout, (folder/'stderr.log').open('xb') as stderr:
        try:
            if status is not None or clock() >= deadline_time():
                if status is None:
                    status = 'FAILED_DEADLINE'
                proc = None
            else:
                proc, affinity = spawn_pinned(task, command, stdout=stdout, stderr=stderr)
        except (OSError, ValueError, subprocess.SubprocessError) as error:
            status, spawn_error = 'FAILED_SPAWN', repr(error)
            proc = None
        if proc is not None:
            try:
                write_new(folder/'PROCESS.json', {'pid': proc.pid, 'pgid': proc.pid,
                          'observed_affinity_before_exec': affinity,
                          'started_utc': clock().isoformat()})
            except Exception:
                kill_reap_group(proc)
                raise
            last_ticks = process_group_cpu_ticks(proc.pid)
            observed_resources = process_resources(proc.pid) or affinity
            last_bytes = (folder/'stdout.jsonl').stat().st_size
            last_activity = start
            while True:
                now = monotonic()
                elapsed = now - start
                if clock() >= deadline_time():
                    status = 'FAILED_DEADLINE'
                    break
                code = proc.poll()
                if code is not None:
                    exit_code = code
                    status = 'COMPLETED' if code == 0 else 'FAILED_NONZERO'
                    break
                if elapsed >= MAX_SECONDS:
                    status = 'FAILED_TIMEOUT_MAX'
                    break
                if elapsed >= DIAGNOSTIC_SECONDS and not extended:
                    progress = progress_snapshot(folder/'stdout.jsonl', task.get('physical_progress_target'))
                    if projected_extension(progress, samples, elapsed):
                        extended = True
                    else:
                        status = 'FAILED_TIMEOUT_DIAGNOSTIC'
                        break
                if now - last_activity >= IDLE_SECONDS:
                    status = 'FAILED_IDLE'
                    break
                if len(samples) >= MAX_SAMPLES:
                    status = 'FAILED_SAMPLE_LIMIT'
                    break
                wait = min(poll_seconds, MAX_SECONDS-elapsed,
                           max(0, (deadline_time()-clock()).total_seconds()))
                sleeper(wait)
                ticks = process_group_cpu_ticks(proc.pid)
                observed_resources = process_resources(proc.pid) or observed_resources
                size = (folder/'stdout.jsonl').stat().st_size
                if ticks > last_ticks or size > last_bytes:
                    last_activity = monotonic()
                last_ticks, last_bytes = ticks, size
                samples.append({'elapsed_seconds': monotonic()-start, 'cpu_ticks': ticks,
                                'stdout_bytes': size,
                                'observed_process_resources': observed_resources,
                                'physical_progress': progress_snapshot(folder/'stdout.jsonl', task.get('physical_progress_target'))})
            kill_reap_group(proc)
            if exit_code is None:
                exit_code = proc.returncode
    if task.get('reference_json') and descriptor_before is not None:
        try:
            if clock() >= deadline_time():
                status = 'FAILED_DEADLINE'
            else:
                descriptor_after = descriptor_probe(task, folder, 'DESCRIPTOR_AFTER', clock)
            if (digest_or_none(task['reference_json']) != reference_before or
                    reference_before != task['reference_sha256']):
                status = 'FAILED_REFERENCE_CHANGED'
            elif (descriptor_after is not None and (descriptor_after['status'] != 'COMPLETED' or
                    descriptor_after['stdout_sha256'] != descriptor_before['stdout_sha256'])):
                status = 'FAILED_DESCRIPTOR_CHANGED'
        except (OSError, ValueError, subprocess.SubprocessError, TimeoutError) as error:
            status, spawn_error = 'FAILED_DESCRIPTOR_CHANGED', repr(error)
    elapsed = monotonic()-start
    progress = progress_snapshot(folder/'stdout.jsonl', task.get('physical_progress_target'))
    result_row = None
    if status == 'COMPLETED':
        try:
            rows = [json.loads(line) for line in (folder/'stdout.jsonl').read_text().splitlines() if line.strip()]
            results = [row for row in rows if isinstance(row, dict) and row.get('kind') == 'result']
            if len(results) != 1:
                status = 'FAILED_MALFORMED_RESULT'
            else:
                result_row = results[0]
        except (UnicodeError, json.JSONDecodeError):
            status = 'FAILED_MALFORMED_RESULT'
    try:
        source_after = {label: git_identity(item['path']) for label, item in task.get('source', {}).items()}
        source_match = all(source_after[label] == {'head': item['head'], 'tree': item['tree'], 'status_porcelain': ''}
                           for label, item in task.get('source', {}).items())
        binary_after = digest(task['binary']) if task.get('binary') else None
        original_after = digest(task['original_receipt']) if task.get('original_receipt') else None
    except (OSError, subprocess.SubprocessError) as error:
        source_after, binary_after, original_after, source_match = {}, None, None, False
        spawn_error = repr(error)
    if not source_match or (task.get('binary_sha256') and binary_after != task['binary_sha256']) or (task.get('original_receipt_sha256') and original_after != task['original_receipt_sha256']):
        status = 'FAILED_IDENTITY_CHANGED'
    receipt = {'task_id': task['id'], 'status': status, 'exit_code': exit_code,
               'spawn_error': spawn_error, 'elapsed_process_seconds': elapsed,
               'charged_new_seconds': elapsed, 'started_utc': started.isoformat(),
               'finished_utc': clock().isoformat(), 'diagnostic_extended': extended,
               'physical_progress': progress, 'watchdog_samples': samples,
               'configured_cpus': [0], 'configured_threads': 1,
               'thread_environment': THREAD_ENVIRONMENT,
               'observed_affinity_before_exec': affinity,
               'observed_process_resources': observed_resources,
               'reference_sha256_before': reference_before,
               'reference_sha256_after': digest_or_none(task['reference_json']) if task.get('reference_json') else None,
               'descriptor_sha256_before': descriptor_before['stdout_sha256'] if descriptor_before else None,
               'descriptor_sha256_after': descriptor_after['stdout_sha256'] if descriptor_after else None,
               'descriptor_probe_seconds': sum(x['elapsed_process_seconds'] for x in (descriptor_before, descriptor_after) if x),
               'source_after': source_after, 'binary_sha256_after': binary_after,
               'original_receipt_sha256_after': original_after,
               'result_scientific_status': result_row.get('scientific_status') if result_row else None,
               'result_solver_success': result_row.get('solver_success') if result_row else None,
               'stdout_sha256': digest(folder/'stdout.jsonl'), 'stderr_sha256': digest(folder/'stderr.log')}
    write_new(folder/'RECEIPT.json', receipt)
    return receipt


def external_costs(cost_root):
    events = {}
    path = Path(cost_root)/'COST_EVENTS.jsonl'
    previous = '0'*64
    event_ids = set()
    for line in path.read_text().splitlines():
        row = json.loads(line)
        if row.get('schema') != 'vigilode.local-continuation-cost-event.v1':
            raise ValueError('external cost event schema mismatch')
        if (row.get('prev_event_sha256') != previous or not row.get('event_id') or
                row['event_id'] in event_ids or row.get('category') not in
                ('setup', 'build', 'reference', 'julia', 'controller', 'other')):
            raise ValueError('external cost event chain or category mismatch')
        event_ids.add(row['event_id'])
        previous = canonical_sha(row)
        key, phase = row.get('attempt_id'), row.get('phase')
        if not isinstance(key, str) or not key or phase not in ('STARTED', 'TERMINAL'):
            raise ValueError('bad external cost event')
        slot = events.setdefault(key, {})
        if phase in slot:
            raise ValueError('duplicate external cost event phase')
        if phase == 'TERMINAL':
            value = row.get('charged_seconds')
            if (not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0
                    or not row.get('evidence_path') or digest(row['evidence_path']) != row.get('evidence_sha256')):
                raise ValueError('unbound terminal external cost')
        else:
            value = row.get('charge_upper_seconds')
            if not isinstance(value, (int, float)) or not math.isfinite(value) or value <= 0:
                raise ValueError('unbounded started external cost')
        slot[phase] = row
    return sum(slot['TERMINAL']['charged_seconds'] if 'TERMINAL' in slot
               else slot['STARTED']['charge_upper_seconds'] for slot in events.values())


def attempt_terminal(folder, task=None):
    receipt = folder/'RECEIPT.json'
    if receipt.exists():
        row, receipt_sha = json_snapshot(receipt)
        start_path = folder/'START.json'
        start_row, start_sha = json_snapshot(start_path) if start_path.exists() else ({}, None)
        if (row.get('schema') == 'vigilode.timeout-diagnostic-receipt.v1' or
                start_row.get('schema') == 'vigilode.timeout-diagnostic-start.v1' or
                'plan_core_sha256' in start_row or row.get('status') == 'FAILED_CLEANUP'):
            return diagnostic_terminal(folder, row, receipt_sha, start_row, start_sha, task)
        return row, receipt
    start = folder/'START.json'
    process = folder/'PROCESS.json'
    reconciliation = folder/'RECONCILIATION.json'
    if not reconciliation.exists() or not process.exists():
        raise ValueError(f'unfinished attempt requires process-tree reconciliation: {folder}')
    row, _ = json_snapshot(reconciliation)
    proc, process_sha = json_snapshot(process)
    start_row, start_sha = json_snapshot(start)
    if (row.get('status') != 'FAILED_INTERRUPTED_RECONCILED' or
            row.get('start_sha256') != start_sha or
            row.get('process_sha256') != process_sha or
            row.get('task_id') != start_row.get('task_id') or
            row.get('process_tree_checked') is not True or
            row.get('pgid') != proc.get('pgid') or
            not isinstance(row.get('charged_new_seconds'), (int, float)) or
            row['charged_new_seconds'] < ATTEMPT_RESERVE_SECONDS):
        raise ValueError(f'invalid interrupted-attempt reconciliation: {folder}')
    pgid = proc.get('pgid')
    if type(pgid) is not int or pgid <= 1:
        raise ValueError('invalid process group id')
    try:
        os.killpg(pgid, 0)
    except ProcessLookupError:
        pass
    else:
        raise ValueError(f'process group still exists: {pgid}')
    if task is not None and row['task_id'] != task['id']:
        raise ValueError('reconciled task mismatch')
    return row, reconciliation


def cumulative_new_charge(cost_root, active_controller=None):
    root = Path(cost_root)
    charged = external_costs(root)
    for start in root.glob('**/attempt_*/START.json'):
        row, _ = attempt_terminal(start.parent)
        value = row.get('charged_new_seconds')
        if not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0:
            raise ValueError('invalid attempt cost')
        charged += value
    for start in root.glob('**/controller/invocation_*/START.json'):
        receipt = start.parent/'RECEIPT.json'
        if receipt.exists():
            charged += json.loads(receipt.read_text())['charged_seconds']
        else:
            elapsed = max(0.0, (utc_now()-dt.datetime.fromisoformat(json.loads(start.read_text())['started_utc'])).total_seconds())
            if active_controller == start.parent:
                process_seconds = sum(json.loads(p.read_text())['charged_new_seconds']
                                      for p in (start.parent.parent.parent/'runs').glob('*/attempt_*/RECEIPT.json')
                                      if dt.datetime.fromisoformat(json.loads(p.read_text())['started_utc']) >=
                                      dt.datetime.fromisoformat(json.loads(start.read_text())['started_utc']))
                elapsed = max(0.0, elapsed-process_seconds)
            charged += elapsed
    return charged


def check_retry_authorization(plan, manifest, task, last, last_path, next_number):
    binding = (manifest or {}).get('authorizations', {}).get(task['id'])
    if not binding or digest(binding['path']) != binding['sha256']:
        raise ValueError('missing hash-bound retry authorization')
    auth = json.loads(Path(binding['path']).read_text())
    expected = {'decision': 'APPROVED_TARGETED_RETRY', 'task_id': task['id'],
                'task_sha256': canonical_sha(task), 'source_sha256': canonical_sha(plan['source']),
                'reference_sha256': task.get('reference_sha256'),
                'prior_attempt_sha256': digest(last_path), 'prior_status': last['status'],
                'next_attempt_number': next_number}
    if any(auth.get(key) != value for key, value in expected.items()):
        raise ValueError('retry authorization binding mismatch')
    limit = auth.get('max_attempts', 1)
    if type(limit) is not int or limit < 2 or next_number > limit:
        raise ValueError('retry attempt limit exceeded')
    return Path(binding['path']).read_bytes(), binding['sha256']


def run_plan(plan_path, output, retry_failed=False, retry_authorizations=None, *, max_new_attempts=1):
    if type(max_new_attempts) is not int or max_new_attempts != 1:
        raise ValueError('invocation permits exactly one new attempt')
    plan_path, output = Path(plan_path), Path(output)
    # Bind the original cumulative cost root before opening any lock or output.
    # All other preflight and campaign writes remain inside its shared lock.
    preliminary_raw = plan_path.read_bytes()
    preliminary = json.loads(preliminary_raw)
    lock_path = validate_cost_root(preliminary)/'.julia_comparison_campaign.lock'
    with lock_path.open('a+') as execution_lock:
        try:
            fcntl.flock(execution_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise RuntimeError(f'shared execution lock is held: {lock_path}') from error
        try:
            plan_raw = plan_path.read_bytes()
            plan = json.loads(plan_raw)
            if ('vigilode.local-continuation-runner.v2' in
                    (preliminary.get('schema'), plan.get('schema')) and plan_raw != preliminary_raw):
                raise ValueError('execution plan changed before execution lock')
            if Path(plan.get('cost_root', '')).resolve() != lock_path.parent:
                raise ValueError('new-cost root changed before execution lock')
            if plan.get('schema') == 'vigilode.local-continuation-runner.v2':
                if retry_failed or retry_authorizations is not None:
                    raise ValueError('v2 diagnostic retry is forbidden')
                return _run_diagnostic_locked(plan_path, output, plan_raw, lock_path)
            return _run_plan_locked(plan_path, output, plan, retry_failed, retry_authorizations, lock_path)
        finally:
            fcntl.flock(execution_lock, fcntl.LOCK_UN)


def _run_plan_locked(plan_path, output, plan, retry_failed, retry_authorizations, execution_lock_path):
    plan_path, output = Path(plan_path), Path(output)
    validate_plan(plan)
    plan_sha256 = digest(plan_path)
    manifest, manifest_sha256 = load_retry_manifest(retry_authorizations, plan_sha256)
    cost_root = Path(plan['cost_root']).resolve()
    if not output.resolve().is_relative_to(cost_root):
        raise ValueError('output outside new-cost root')
    charged = cumulative_new_charge(cost_root)  # fail before mutation on every unfinished attempt
    identity = {'plan_sha256': plan_sha256, 'source': plan['source'],
                'task_ids': [task['id'] for task in plan['tasks']]}
    output_binding = {'campaign_identity': identity, 'output': str(output.resolve())}
    output_lock = cost_root/'CAMPAIGN_OUTPUT_IDENTITY.json'
    if output_lock.exists():
        if json.loads(output_lock.read_text()) != output_binding:
            raise ValueError('campaign output identity mismatch')
    else:
        write_new(output_lock, output_binding)
    output.mkdir(parents=True, exist_ok=True)
    lock = output/'CAMPAIGN_IDENTITY.json'
    if lock.exists():
        if json.loads(lock.read_text()) != identity:
            raise ValueError('campaign identity mismatch')
    else:
        write_new(lock, identity)
    controller = output/'controller'/f'invocation_{time.time_ns()}'
    started_utc, started_monotonic = utc_now(), time.monotonic()
    write_new(controller/'START.json', {'started_utc': started_utc.isoformat(),
              'plan_sha256': plan_sha256, 'deadline_utc': DEADLINE,
              'max_new_attempts': 1, 'execution_lock_path': str(execution_lock_path),
              'retry_manifest_sha256': manifest_sha256})
    results = []
    try:
        for task in plan['tasks']:
            retry_evidence = None
            root = output/'runs'/task['id']
            attempts = sorted(root.glob('attempt_*'))
            if attempts:
                last, last_path = attempt_terminal(attempts[-1], task)
                if last['status'] == 'COMPLETED' or not retry_failed:
                    continue
                auth_bytes, auth_sha = check_retry_authorization(
                    plan, manifest, task, last, last_path, len(attempts)+1)
                manifest_bytes = Path(retry_authorizations).read_bytes()
                if hashlib.sha256(manifest_bytes).hexdigest() != manifest_sha256:
                    raise ValueError('retry manifest changed during invocation')
                retry_evidence = (manifest_bytes, auth_bytes, manifest_sha256, auth_sha)
            charged = cumulative_new_charge(cost_root, controller)
            if utc_now() >= deadline_time() or charged + ATTEMPT_RESERVE_SECONDS > NEW_CAP_SECONDS:
                break
            folder = root/f'attempt_{len(attempts)+1:03d}'
            validate_plan(plan)
            if digest(plan_path) != plan_sha256:
                raise ValueError('execution plan changed during invocation')
            if retry_evidence is not None and digest(manifest['authorizations'][task['id']]['path']) != retry_evidence[3]:
                raise ValueError('retry authorization changed during invocation')
            result = run_one({**task, 'source': plan['source']}, folder,
                             plan_sha256=plan_sha256, retry_evidence=retry_evidence)
            results.append(result)
            if digest(plan_path) != plan_sha256:
                raise ValueError('execution plan changed during invocation')
            break  # One new attempt; later invocations reuse this exact plan/output.
    finally:
        wall = time.monotonic()-started_monotonic
        process = sum(row['charged_new_seconds'] for row in results)
        write_new(controller/'RECEIPT.json', {'started_utc': started_utc.isoformat(),
                  'finished_utc': utc_now().isoformat(), 'wall_seconds': wall,
                  'process_seconds_already_charged': process,
                  'max_new_attempts': 1, 'execution_lock_path': str(execution_lock_path),
                  'charged_seconds': max(0.0, wall-process)})
    charged = cumulative_new_charge(cost_root)
    write_new(output/f'EXECUTION_{time.time_ns()}.json', {'attempts': results,
              'charged_new_seconds': charged, 'cap_seconds': NEW_CAP_SECONDS,
              'max_new_attempts': 1, 'execution_lock_path': str(execution_lock_path),
              'deadline_utc': DEADLINE, 'prior_lifetime_cost_excluded_from_new_cap': True})
    return results


def durable_new(path, value):
    """Exclusive durable creation; a crash leaves a consumed binding, never a retry."""
    path = Path(path)
    missing = []
    ancestor = path.parent
    while not ancestor.exists():
        missing.append(ancestor)
        ancestor = ancestor.parent
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())
    for directory in {path.parent, *(item.parent for item in missing)}:
        fd = os.open(directory, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)


def canonical_path(value):
    if not isinstance(value, str) or not value or str(Path(value).resolve()) != value:
        raise ValueError('noncanonical path or output alias')
    return Path(value)


def bound_json(binding, label):
    if not isinstance(binding, dict) or set(binding) != {'path', 'sha256'}:
        raise ValueError(f'{label} binding incomplete')
    path = canonical_path(binding['path'])
    value, sha256 = json_snapshot(path)
    if sha256 != binding['sha256']:
        raise ValueError(f'{label} identity mismatch')
    return value


def diagnostic_terminal(folder, receipt, receipt_sha, start, start_sha, task=None):
    """A terminal receipt cannot release admission without proved group cleanup."""
    process_path = folder/'PROCESS.json'
    proc, process_sha = json_snapshot(process_path) if process_path.exists() else ({}, None)
    cleanup = receipt.get('cleanup')
    if (receipt.get('status') != 'FAILED_CLEANUP' and isinstance(cleanup, dict) and
            cleanup.get('all_reaped') is True and cleanup.get('leader_waited') is True and
            isinstance(cleanup.get('after'), dict) and cleanup['after'].get('members') == [] and
            type(proc.get('pgid')) is int and proc['pgid'] > 1 and
            cleanup.get('pgid') == proc['pgid'] and cleanup['after'].get('pgid') == proc['pgid']):
        return receipt, folder/'RECEIPT.json'
    reconciliation = folder/'CLEANUP_RECONCILIATION.json'
    if not reconciliation.is_file():
        raise ValueError(f'unresolved diagnostic cleanup requires reconciliation: {folder}')
    row, _ = json_snapshot(reconciliation)
    pgid = proc.get('pgid')
    expected = {'schema': 'vigilode.timeout-diagnostic-cleanup-reconciliation.v1',
                'status': 'FAILED_CLEANUP_RECONCILED', 'task_id': start.get('task_id'),
                'original_task_sha256': start.get('original_task_sha256'),
                'start_sha256': start_sha, 'process_sha256': process_sha,
                'receipt_sha256': receipt_sha, 'pgid': pgid,
                'process_tree_checked': True, 'process_group_absent': True}
    if (any(row.get(key) != value for key, value in expected.items()) or
            type(pgid) is not int or pgid <= 1 or process_sha is None or
            receipt.get('started_utc') != start.get('started_utc') or
            receipt.get('task_id') != start.get('task_id') or
            receipt.get('original_task_sha256') != start.get('original_task_sha256') or
            (task is not None and row.get('task_id') != task['id'])):
        raise ValueError(f'invalid diagnostic cleanup reconciliation: {folder}')
    try:
        started = dt.datetime.fromisoformat(start['started_utc'])
        finished = dt.datetime.fromisoformat(receipt['finished_utc'])
        reconciled = dt.datetime.fromisoformat(row['reconciled_utc'])
        original_charge, charge = receipt['charged_new_seconds'], row['charged_new_seconds']
        if (any(value.tzinfo is None for value in (started, finished, reconciled)) or
                not started <= finished <= reconciled <= utc_now() or
                type(original_charge) not in (int, float) or not math.isfinite(original_charge) or
                original_charge < 0 or type(charge) not in (int, float) or not math.isfinite(charge) or
                charge < max(ATTEMPT_RESERVE_SECONDS, (reconciled-started).total_seconds(),
                             original_charge+(reconciled-finished).total_seconds())):
            raise ValueError('unreconciled process cost')
    except (KeyError, TypeError, ValueError) as error:
        raise ValueError(f'invalid diagnostic cleanup reconciliation cost/time: {folder}') from error
    try:
        os.killpg(pgid, 0)
    except ProcessLookupError:
        pass
    else:
        raise ValueError(f'process group still exists: {pgid}')
    if process_group_snapshot(pgid)['members']:
        raise ValueError(f'process group still exists in final snapshot: {pgid}')
    return row, reconciliation


def diagnostic_plan_core(plan):
    return {key: value for key, value in plan.items() if key != 'approval'}


def validate_predecessor(plan, task):
    predecessor = plan.get('predecessor')
    if task['wave'] == 1:
        if predecessor is not None:
            raise ValueError('unexpected wave1 predecessor')
        return
    if not isinstance(predecessor, dict) or set(predecessor) != {'binding', 'receipt', 'review'}:
        raise ValueError('missing independently reviewed predecessor')
    binding = bound_json(predecessor['binding'], 'predecessor binding')
    receipt = bound_json(predecessor['receipt'], 'predecessor receipt')
    review = bound_json(predecessor['review'], 'predecessor review')
    expected_binding = (COST_ROOT/'DIAGNOSTIC_OUTPUT_BINDINGS'/
                        (binding.get('original_task_sha256', '')+'.json')).resolve()
    receipt_path = Path(predecessor['receipt']['path'])
    if (Path(predecessor['binding']['path']) != expected_binding or
            binding.get('case') != task['depends_on_case'] or binding.get('wave') != 1 or
            not receipt_path.is_relative_to(Path(binding.get('output', '/missing'))/'runs') or
            receipt.get('plan_core_sha256') != binding.get('plan_core_sha256') or
            receipt.get('original_task_sha256') != binding.get('original_task_sha256') or
            receipt.get('status') not in ('COMPLETED', 'FAILED_TIMEOUT_DIAGNOSTIC', 'FAILED_IDLE') or
            receipt.get('cleanup', {}).get('all_reaped') is not True):
        raise ValueError('predecessor terminal binding mismatch')
    expected = {'schema': 'vigilode.timeout-diagnostic-predecessor-review.v1',
                'decision': 'APPROVED_NEXT_DIAGNOSTIC', 'independent_review': True,
                'binding_sha256': predecessor['binding']['sha256'],
                'receipt_sha256': predecessor['receipt']['sha256'],
                'next_original_task_sha256': task['original_task_sha256'],
                'source': plan['source'], 'binary_sha256': task['binary_sha256']}
    if (any(review.get(key) != value for key, value in expected.items()) or
            not isinstance(review.get('approver'), str) or not review['approver'] or
            not isinstance(review.get('implementer'), str) or not review['implementer'] or
            review['approver'] == review['implementer']):
        raise ValueError('predecessor independent review mismatch')


def validate_diagnostic_plan(plan):
    """Opt-in v2 contract. Approval covers every core field, including unknown ones."""
    root = validate_cost_root(plan)
    if (plan.get('deadline_utc') != DEADLINE or plan.get('new_cap_seconds') != NEW_CAP_SECONDS or
            not (root/'COST_EVENTS.jsonl').is_file()):
        raise ValueError('diagnostic deadline, cap, or cost events mismatch')
    if not isinstance(plan.get('tasks'), list) or len(plan['tasks']) != 1:
        raise ValueError('v2 requires exactly one catalog task')
    if 'retry_authorizations' in plan:
        raise ValueError('v2 diagnostic retry is forbidden')
    if plan.get('watchdog') != DIAGNOSTIC_WATCHDOG:
        raise ValueError('diagnostic watchdog contract mismatch')
    output = canonical_path(plan.get('output'))
    if output == root or not output.is_relative_to(root):
        raise ValueError('diagnostic output outside canonical cost root')
    source = plan.get('source')
    if not isinstance(source, dict) or not source:
        raise ValueError('missing frozen source')
    for label, item in source.items():
        if not isinstance(item, dict) or not all(item.get(key) for key in ('path', 'head', 'tree')):
            raise ValueError(f'bad source {label}')
        canonical_path(item['path'])
        if git_identity(item['path']) != {'head': item['head'], 'tree': item['tree'], 'status_porcelain': ''}:
            raise ValueError(f'source identity mismatch: {label}')
    catalog = bound_json(plan.get('catalog'), 'catalog')
    if (plan['catalog'] != {'path': str(DIAGNOSTIC_CATALOG_PATH.resolve()),
                           'sha256': DIAGNOSTIC_CATALOG_SHA256} or
            catalog.get('schema') != 'vigilode.timeout-diagnostic-runner-template.v2'):
        raise ValueError('frozen diagnostic catalog mismatch')
    task = plan['tasks'][0]
    validate_resources(task)
    if (not isinstance(task.get('id'), str) or not task['id'].isalnum() or
            not isinstance(task.get('command'), list) or not task['command'] or
            not all(isinstance(part, str) for part in task['command'])):
        raise ValueError('bad diagnostic task or command')
    if ('physical_progress_target' not in task or task['physical_progress_target'] is not None or
            task.get('reference_json') is not None or task.get('extension_enabled', False) is not False or
            task.get('diagnostic_seconds', 300) != 300 or task.get('max_seconds', 300) != 300):
        raise ValueError('diagnostic progress/extension/reference contract mismatch')
    original = bound_json({'path': task.get('original_receipt'),
                           'sha256': task.get('original_receipt_sha256')}, 'original receipt')
    if (original.get('task') != task.get('original_task') or
            canonical_sha(original.get('task')) != task.get('original_task_sha256') or
            original.get('execution_status') != 'TIMEOUT'):
        raise ValueError('original task identity or timeout status mismatch')
    selected = [item for item in catalog.get('selection', [])
                if item.get('original_receipt') == task['original_receipt']]
    if len(selected) != 1:
        raise ValueError('task absent from frozen catalog')
    selected = selected[0]
    if (any(task.get(key) != selected.get(key) for key in
            ('case', 'wave', 'depends_on_case', 'original_receipt_sha256')) or
            task['command'][1:] != selected['command'][1:] or
            original.get('command', [])[1:] != selected['command'][1:] or
            task.get('case') != task['original_task'].get('case')):
        raise ValueError('frozen catalog selection/command mismatch')
    binary = canonical_path(task.get('binary'))
    if task['command'][0] != str(binary) or digest(binary) != task.get('binary_sha256'):
        raise ValueError('binary identity or command mismatch')
    runner = plan.get('runner')
    if (not isinstance(runner, dict) or runner.get('path') != str(Path(__file__).resolve()) or
            runner.get('sha256') != digest(__file__)):
        raise ValueError('runner identity mismatch')
    build = bound_json(plan.get('build_receipt'), 'build receipt')
    if (build.get('schema') != 'vigilode.timeout-diagnostic-build.v1' or
            build.get('status') != 'COMPLETED' or type(build.get('exit_code')) is not int or
            build['exit_code'] != 0 or build.get('source') != source or
            build.get('binary') != str(binary) or build.get('binary_sha256') != task['binary_sha256'] or
            not isinstance(build.get('command'), list) or not build['command'] or
            not all(isinstance(part, str) for part in build['command'])):
        raise ValueError('frozen build receipt mismatch')
    validate_predecessor(plan, task)
    approval = bound_json(plan.get('approval'), 'approval')
    expected = {'schema': 'vigilode.timeout-diagnostic-approval.v1',
                'decision': 'APPROVED_ONE_DIAGNOSTIC', 'independent_review': True,
                'plan_core_sha256': canonical_sha(diagnostic_plan_core(plan)),
                'original_task_sha256': task['original_task_sha256'],
                'original_receipt_sha256': task['original_receipt_sha256'],
                'binary': task['binary'], 'binary_sha256': task['binary_sha256'],
                **{key: plan[key] for key in ('source', 'runner', 'catalog', 'build_receipt',
                                             'output', 'deadline_utc', 'new_cap_seconds')}}
    if (any(approval.get(key) != value for key, value in expected.items()) or
            not isinstance(approval.get('approver'), str) or not approval['approver'] or
            not isinstance(approval.get('implementer'), str) or not approval['implementer'] or
            approval['approver'] == approval['implementer']):
        raise ValueError('independent approval binding mismatch')


def process_group_snapshot(pgid, *, proc_root=Path('/proc')):
    members = []
    for path in proc_root.glob('[0-9]*/stat'):
        try:
            line = path.read_text()
            fields = line[line.rfind(')')+2:].split()
            if int(fields[2]) != pgid:
                continue
            pid = int(path.parent.name)
            resources = process_resources(pid)
            if resources is None:
                if path.exists():
                    raise ValueError('unobservable process-group resources')
                continue
            members.append({**resources, 'state': fields[0],
                            'cpu_ticks': int(fields[11])+int(fields[12]),
                            'rss_bytes': max(0, int(fields[21]))*os.sysconf('SC_PAGE_SIZE')})
        except (FileNotFoundError, ProcessLookupError):
            continue
    return {'pgid': pgid, 'members': members,
            'cpu_ticks': sum(row['cpu_ticks'] for row in members),
            'rss_bytes': sum(row['rss_bytes'] for row in members)}


def group_resource_violation(snapshot):
    if snapshot['rss_bytes'] > DIAGNOSTIC_WATCHDOG['rss_limit_bytes']:
        return 'FAILED_RSS_LIMIT'
    if any(row['state'] != 'Z' and (row['cpu_affinity'] != [0] or row['os_thread_count'] != 1)
           for row in snapshot['members']):
        return 'FAILED_PROCESS_RESOURCES'
    return None


def set_subreaper(value=None):
    """Linux subreaper permits reaping orphaned members of our owned group."""
    libc = ctypes.CDLL(None, use_errno=True)
    previous = ctypes.c_int()
    if libc.prctl(37, ctypes.byref(previous), 0, 0, 0) != 0:
        raise OSError(ctypes.get_errno(), 'PR_GET_CHILD_SUBREAPER failed')
    if value is not None and libc.prctl(36, int(value), 0, 0, 0) != 0:
        raise OSError(ctypes.get_errno(), 'PR_SET_CHILD_SUBREAPER failed')
    return previous.value


def cleanup_diagnostic_group(proc):
    """Kill on every exit, reap the leader and adopted descendants, retain proof."""
    errors = []
    try:
        before = process_group_snapshot(proc.pid)
    except Exception as error:
        before = None
        errors.append(repr(error))
    reaped = []
    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    proc.wait()
    end = time.monotonic()+5
    remaining = before
    while True:
        while True:
            try:
                pid, status = os.waitpid(-proc.pid, os.WNOHANG)
            except ChildProcessError:
                break
            if pid == 0:
                break
            reaped.append({'pid': pid, 'wait_status': status})
        try:
            remaining = process_group_snapshot(proc.pid)
        except Exception as error:
            errors.append(repr(error))
            remaining = None
            break
        if not remaining['members'] or time.monotonic() >= end:
            break
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        time.sleep(.01)
    return {'pgid': proc.pid, 'signal': 'SIGKILL', 'leader_waited': True,
            'before': before, 'reaped_descendants': reaped, 'after': remaining,
            'errors': errors, 'all_reaped': remaining is not None and not remaining['members']}


def run_diagnostic_one(task, folder, *, plan_sha256, plan_core_sha256,
                       poll_seconds=POLL_SECONDS, clock=utc_now, monotonic=time.monotonic,
                       sleeper=time.sleep, identity_check=None):
    validate_resources(task)
    folder = Path(folder)
    folder.mkdir(parents=True, exist_ok=False)
    started, start = clock(), monotonic()
    durable_new(folder/'START.json', {'schema': 'vigilode.timeout-diagnostic-start.v1',
                'task_id': task['id'], 'started_utc': started.isoformat(),
                'plan_sha256': plan_sha256, 'plan_core_sha256': plan_core_sha256,
                'original_task_sha256': task['original_task_sha256'], 'command': task['command'],
                'watchdog': DIAGNOSTIC_WATCHDOG, 'deadline_utc': DEADLINE,
                'configured_cpus': [0], 'configured_threads': 1,
                'thread_environment': THREAD_ENVIRONMENT, 'reserved_seconds': 960})
    samples, cleanup, proc, affinity, failure = [], None, None, None, None
    terminal_snapshot = None
    status, interrupted, previous_subreaper = None, None, None
    old_handlers = {}
    cpu_peaks = {}
    def terminate(signum, frame):
        raise SystemExit(128+signum)
    try:
        for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
            old_handlers[signum] = signal.signal(signum, terminate)
        previous_subreaper = set_subreaper(True)
        with (folder/'stdout.jsonl').open('xb') as stdout, (folder/'stderr.log').open('xb') as stderr:
            if clock() >= deadline_time():
                status = 'FAILED_DEADLINE'
            else:
                if identity_check is not None:
                    identity_check()
                # Defer termination until Popen has returned ownership to this scope.
                # The child restores its mask before exec; cleanup uses SIGKILL.
                mask = signal.pthread_sigmask(signal.SIG_BLOCK, tuple(old_handlers))
                try:
                    proc, affinity = spawn_pinned(task, task['command'], stdout=stdout, stderr=stderr,
                                                   unblock_child_signals=True)
                finally:
                    signal.pthread_sigmask(signal.SIG_SETMASK, mask)
                durable_new(folder/'PROCESS.json', {'pid': proc.pid, 'pgid': proc.pid,
                            'observed_affinity_before_exec': affinity})
                last_activity, last_ticks, last_bytes = start, 0, 0
                while True:
                    now = monotonic()
                    snapshot = process_group_snapshot(proc.pid)
                    terminal_snapshot = snapshot
                    for row in snapshot['members']:
                        cpu_peaks[row['pid']] = max(cpu_peaks.get(row['pid'], 0), row['cpu_ticks'])
                    ticks = sum(cpu_peaks.values())
                    size = (folder/'stdout.jsonl').stat().st_size
                    if ticks > last_ticks or size > last_bytes:
                        last_activity = now
                    last_ticks, last_bytes = ticks, size
                    violation = group_resource_violation(snapshot)
                    if violation:
                        status = violation
                        break
                    if clock() >= deadline_time():
                        status = 'FAILED_DEADLINE'
                        break
                    if now-start >= DIAGNOSTIC_WATCHDOG['hard_seconds']:
                        status = 'FAILED_TIMEOUT_DIAGNOSTIC'
                        break
                    code = proc.poll()
                    if code is not None:
                        status = 'COMPLETED' if code == 0 else 'FAILED_NONZERO'
                        break
                    if now-last_activity >= DIAGNOSTIC_WATCHDOG['idle_seconds']:
                        status = 'FAILED_IDLE'
                        break
                    if len(samples) >= DIAGNOSTIC_WATCHDOG['maximum_samples']:
                        status = 'FAILED_SAMPLE_LIMIT'
                        break
                    samples.append({'elapsed_seconds': now-start, 'cpu_ticks': ticks,
                                    'stdout_bytes': size, 'process_group': snapshot,
                                    'physical_progress': {'status': 'UNKNOWN', 'reason': 'target disabled'}})
                    sleeper(min(poll_seconds, DIAGNOSTIC_WATCHDOG['hard_seconds']-(now-start),
                                DIAGNOSTIC_WATCHDOG['idle_seconds']-(now-last_activity),
                                max(0, (deadline_time()-clock()).total_seconds())))
    except BaseException as error:
        failure = repr(error)
        status = 'FAILED_INTERRUPTED' if isinstance(error, (KeyboardInterrupt, SystemExit)) else 'FAILED_RUNNER'
        if not isinstance(error, Exception):
            interrupted = error
    finally:
        try:
            for signum in old_handlers:
                signal.signal(signum, signal.SIG_IGN)
            if proc is not None:
                cleanup = cleanup_diagnostic_group(proc)
                durable_new(folder/'CLEANUP.json', cleanup)
                if not cleanup['all_reaped']:
                    status = 'FAILED_CLEANUP'
        finally:
            if previous_subreaper is not None:
                set_subreaper(previous_subreaper)
            for signum, handler in old_handlers.items():
                signal.signal(signum, handler)
    if status == 'COMPLETED':
        try:
            rows = [json.loads(line) for line in (folder/'stdout.jsonl').read_text().splitlines() if line.strip()]
            if sum(isinstance(row, dict) and row.get('kind') == 'result' for row in rows) != 1:
                status = 'FAILED_MALFORMED_RESULT'
        except (ValueError, UnicodeError):
            status = 'FAILED_MALFORMED_RESULT'
    if identity_check is not None:
        try:
            identity_check()
        except Exception as error:
            status, failure = 'FAILED_IDENTITY_CHANGED', repr(error)
    elapsed = monotonic()-start
    receipt = {'schema': 'vigilode.timeout-diagnostic-receipt.v1', 'task_id': task['id'],
               'original_task_sha256': task['original_task_sha256'],
               'plan_sha256': plan_sha256, 'plan_core_sha256': plan_core_sha256,
               'status': status, 'exit_code': proc.returncode if proc else None, 'error': failure,
               'started_utc': started.isoformat(), 'finished_utc': clock().isoformat(),
               'elapsed_process_seconds': elapsed, 'charged_new_seconds': elapsed,
               'diagnostic_extended': False, 'physical_progress': {'status': 'UNKNOWN'},
               'observed_affinity_before_exec': affinity, 'watchdog_samples': samples,
               'terminal_process_group': terminal_snapshot,
               'cleanup': cleanup, 'timing_authority': 'diagnostic_only',
               'scientific_admission': 'HOLD',
               'stdout_sha256': digest_or_none(folder/'stdout.jsonl'),
               'stderr_sha256': digest_or_none(folder/'stderr.log')}
    durable_new(folder/'RECEIPT.json', receipt)
    if interrupted is not None:
        raise interrupted
    return receipt


def _run_diagnostic_locked(plan_path, output, plan_raw, execution_lock_path):
    started, start = utc_now(), time.monotonic()
    plan = json.loads(plan_raw)
    plan_sha = hashlib.sha256(plan_raw).hexdigest()
    if Path(plan_path).read_bytes() != plan_raw:
        raise ValueError('execution plan changed after locked snapshot')
    validate_plan(plan)
    output = canonical_path(str(output))
    if str(output) != plan['output']:
        raise ValueError('diagnostic output binding mismatch')
    root = Path(plan['cost_root']).resolve()
    task = plan['tasks'][0]
    bindings = root/'DIAGNOSTIC_OUTPUT_BINDINGS'
    if bindings.exists() and bindings.resolve() != bindings:
        raise ValueError('diagnostic binding directory alias')
    binding_path = bindings/(task['original_task_sha256']+'.json')
    if binding_path.exists() or binding_path.is_symlink():
        raise ValueError('diagnostic task binding already consumed: replay forbidden')
    if output.exists():
        raise ValueError('diagnostic output already exists')
    for path in bindings.glob('*.json'):
        if json.loads(path.read_text()).get('output') == str(output):
            raise ValueError('diagnostic output binding already consumed')
    charged = cumulative_new_charge(root)
    if (utc_now() >= deadline_time() or charged+960 > NEW_CAP_SECONDS or
            (deadline_time()-utc_now()).total_seconds() < 960):
        return []
    core_sha = canonical_sha(diagnostic_plan_core(plan))
    def check_identity():
        if Path(plan_path).read_bytes() != plan_raw:
            raise ValueError('execution plan changed during invocation')
        validate_plan(plan)
    check_identity()
    binding = {'schema': 'vigilode.timeout-diagnostic-output-binding.v1',
               'original_task_sha256': task['original_task_sha256'],
               'original_receipt_sha256': task['original_receipt_sha256'],
               'case': task['case'], 'wave': task['wave'], 'output': str(output),
               'plan_sha256': plan_sha, 'plan_core_sha256': core_sha,
               'source': plan['source'], 'binary_sha256': task['binary_sha256'],
               'runner': plan['runner'], 'catalog': plan['catalog'],
               'approval': plan['approval'], 'created_utc': utc_now().isoformat()}
    durable_new(binding_path, binding)
    output.mkdir(parents=True, exist_ok=False)
    durable_new(output/'PLAN.json', plan)
    controller = output/'controller'/f'invocation_{time.time_ns()}'
    durable_new(controller/'START.json', {'started_utc': started.isoformat(),
                'plan_sha256': plan_sha, 'execution_lock_path': str(execution_lock_path),
                'max_new_attempts': 1, 'reserved_seconds': 960})
    results = []
    try:
        results.append(run_diagnostic_one(task, output/'runs'/task['id']/'attempt_001',
                       plan_sha256=plan_sha, plan_core_sha256=core_sha, identity_check=check_identity))
    finally:
        process = sum(json.loads(path.read_text())['charged_new_seconds']
                      for path in output.glob('runs/*/attempt_*/RECEIPT.json'))
        wall = time.monotonic()-start
        durable_new(controller/'RECEIPT.json', {'started_utc': started.isoformat(),
                    'finished_utc': utc_now().isoformat(), 'wall_seconds': wall,
                    'process_seconds_already_charged': process,
                    'charged_seconds': max(0, wall-process), 'max_new_attempts': 1})
    durable_new(output/'EXECUTION.json', {'attempts': results,
                'charged_new_seconds': cumulative_new_charge(root), 'cap_seconds': NEW_CAP_SECONDS,
                'deadline_utc': DEADLINE, 'max_new_attempts': 1,
                'scientific_admission': 'HOLD', 'timing_authority': 'diagnostic_only'})
    return results


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--plan', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--retry-failed', action='store_true')
    parser.add_argument('--retry-authorizations', type=Path)
    args = parser.parse_args()
    run_plan(args.plan, args.output, args.retry_failed, args.retry_authorizations)


if __name__ == '__main__':
    main()
