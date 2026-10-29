#!/usr/bin/env python3
"""Hash-locked, single-task Julia independent-reference launch controller.

Real launches require a post-freeze approved plan. Never runs install/build/timing.
"""
from __future__ import annotations

import argparse
import datetime as dt
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import time
import uuid

SCHEMA = 'vigilode.independent-reference-launch-plan.v1'
ALLOWED_CASES = (8, 9, 11, 20)
APPROVAL_SCHEMA = 'vigilode.independent-reference-launch-approval.v1'
ZERO_SHA = '0' * 64
CONTROLLER_HARD_CAP_SECONDS = 900.0
GLOBAL_HARD_CAP_SECONDS = 172800.0
GLOBAL_COST_ROOT = Path('/tmp/vigilode-local-run-20261024.geLVCp9Q/continuation_20261025T080718Z')
GLOBAL_DEADLINE = '2026-10-27T08:07:00Z'
EXECUTION_LOCK_NAME = '.julia_comparison_campaign.lock'
EXPECTED_SCRIPTS = {8: 'independent_sparse_references.jl', 9: 'independent_julia_reference.jl', 11: 'independent_julia_reference.jl', 20: 'independent_sparse_references.jl'}
PREFLIGHT_TIMEOUT_SECONDS = 10.0
FINALIZE_RESERVE_SECONDS = 15.0
ACTIVE_PROC = None
ACTIVE_ATTEMPT_DIR = None
EXEC_LOCK_STREAM = None
THREAD_VARS = ('JULIA_NUM_THREADS', 'OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS',
               'MKL_NUM_THREADS', 'BLIS_NUM_THREADS', 'VECLIB_MAXIMUM_THREADS',
               'RAYON_NUM_THREADS')


def sha_file(path: str | Path) -> str:
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda: f.read(1 << 20), b''):
            h.update(block)
    return h.hexdigest()


def write_new(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x', encoding='utf-8') as f:
        json.dump(value, f, sort_keys=True, indent=2, allow_nan=False)
        f.write('\n')
        f.flush()
        os.fsync(f.fileno())


def utc_now():
    return dt.datetime.now(dt.timezone.utc)


def absolute_remaining():
    return (dt.datetime.fromisoformat(GLOBAL_DEADLINE.replace('Z', '+00:00'))-utc_now()).total_seconds()


def bounded_command(argv, cwd, evidence_dir, name, campaign_deadline):
    timeout = min(PREFLIGHT_TIMEOUT_SECONDS, campaign_deadline-time.monotonic()-FINALIZE_RESERVE_SECONDS,
                  absolute_remaining()-FINALIZE_RESERVE_SECONDS)
    if timeout <= 0:
        raise RuntimeError('deadline leaves no preflight budget')
    result = run_child(argv, str(cwd), dict(os.environ), 0, timeout, timeout,
                       12*1024**3, 1<<20, 0.05,
                       evidence_dir/(name+'.stdout'), evidence_dir/(name+'.stderr'),
                       evidence_dir/(name+'.resources.jsonl'), campaign_deadline=campaign_deadline)
    write_new(evidence_dir/(name+'.json'), result)
    if result['status'] != 'COMPLETED':
        raise RuntimeError(f'bounded preflight command failed: {name}: {result["status"]}')
    return (evidence_dir/(name+'.stdout')).read_text().strip()


def git_identity(repo, evidence_dir, campaign_deadline):
    def git(name, *args):
        return bounded_command(['git', '-C', str(repo), *args], repo, evidence_dir, name, campaign_deadline)
    return {'head': git('git_head', 'rev-parse', 'HEAD'),
            'tree': git('git_tree', 'rev-parse', 'HEAD^{tree}'),
            'status_porcelain': git('git_status', 'status', '--porcelain=v1', '--untracked-files=all')}


def locked_file(path: str | Path, expected_sha: str) -> dict:
    p = Path(path).resolve(strict=True)
    actual = sha_file(p)
    if not isinstance(expected_sha, str) or actual != expected_sha:
        raise ValueError(f'locked file SHA mismatch: {p}')
    return {'path': str(p), 'sha256': actual, 'bytes': p.stat().st_size}


def canonical_sha(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False).encode()).hexdigest()


def plan_core_sha(plan):
    """Bind every plan field except the approval reference, avoiding a hash cycle."""
    return canonical_sha({key: value for key, value in plan.items() if key != 'approval_receipt'})


def selected_task(plan):
    tasks = plan.get('tasks')
    if not isinstance(tasks, list) or len(tasks) != 1 or not isinstance(tasks[0], dict):
        raise ValueError('exactly one predeclared reference task is required per invocation')
    task = tasks[0]
    if type(task.get('case')) is not int or task['case'] not in ALLOWED_CASES:
        raise ValueError('single reference task must be case8, case9, case11, or case20')
    if plan.get('execution_policy', {}).get('sequence') != [task['case']]:
        raise ValueError('execution policy sequence must match the single predeclared case')
    return task


def verify_plan_approval(plan):
    task = selected_task(plan)
    reference = plan.get('approval_receipt')
    if not isinstance(reference, dict):
        raise ValueError('hash-bound plan approval receipt is required')
    info = locked_file(reference['path'], reference['sha256'])
    approval = json.loads(Path(info['path']).read_text())
    if (approval.get('schema') != APPROVAL_SCHEMA or
            approval.get('decision') != 'APPROVED_SINGLE_REFERENCE_TASK' or
            approval.get('selected_case') != task['case'] or
            approval.get('approved_plan_core_sha256') != plan_core_sha(plan)):
        raise ValueError('approval does not bind this exact single-task plan core')
    return info


def event_accounting(rows):
    """Validate the existing chain schema, preserving legacy terminal-only events."""
    slots = {}
    for row in rows:
        phase, attempt = row.get('phase'), row.get('attempt_id')
        if (not isinstance(attempt, str) or not attempt or phase not in ('STARTED', 'TERMINAL') or
                row.get('category') not in ('setup', 'build', 'reference', 'julia', 'controller', 'other')):
            raise ValueError('invalid COST_EVENTS phase, attempt, or category')
        slot = slots.setdefault(attempt, {})
        if phase in slot or phase == 'STARTED' and 'TERMINAL' in slot:
            raise ValueError('duplicate or reversed COST_EVENTS phase')
        key = 'charged_seconds' if phase == 'TERMINAL' else 'charge_upper_seconds'
        value = row.get(key)
        if (type(value) not in (int, float) or not math.isfinite(value) or
                value < 0 or phase == 'STARTED' and value == 0):
            raise ValueError('invalid COST_EVENTS charge or reservation')
        if phase == 'TERMINAL':
            try:
                valid = sha_file(row['evidence_path']) == row['evidence_sha256']
            except (KeyError, OSError, TypeError):
                valid = False
            if not valid:
                raise ValueError('unbound terminal COST_EVENTS evidence')
        slot[phase] = row
    charged = sum(x['TERMINAL']['charged_seconds'] for x in slots.values() if 'TERMINAL' in x)
    reserved = sum(x['STARTED']['charge_upper_seconds'] for x in slots.values() if 'TERMINAL' not in x)
    return charged, reserved


def validate_event_chain(path):
    raw = Path(path).read_bytes()
    previous, rows, ids = ZERO_SHA, [], set()
    for index, line in enumerate(raw.splitlines(keepends=True), 1):
        if not line.endswith(b'\n'):
            raise ValueError(f'COST_EVENTS has incomplete trailing line {index}')
        row = json.loads(line)
        if (row.get('schema') != 'vigilode.local-continuation-cost-event.v1' or
                row.get('prev_event_sha256') != previous):
            raise ValueError(f'COST_EVENTS hash chain mismatch at line {index}')
        event_id = row.get('event_id')
        if not isinstance(event_id, str) or not event_id or event_id in ids:
            raise ValueError('COST_EVENTS event id absent or duplicated')
        ids.add(event_id)
        previous = canonical_sha(row)
        rows.append(row)
    event_accounting(rows)
    return rows, hashlib.sha256(raw).hexdigest()


def append_event(path, lock_path, row, *, already_locked=False, expected_head=None):
    def append_locked():
        rows, _ = validate_event_chain(path)
        head = canonical_sha(rows[-1]) if rows else ZERO_SHA
        if expected_head is not None and head != expected_head:
            raise ValueError('global cost chain changed before append')
        event = dict(row, prev_event_sha256=head)
        if (event.get('schema') != 'vigilode.local-continuation-cost-event.v1' or
                not isinstance(event.get('event_id'), str) or not event['event_id']):
            raise ValueError('invalid proposed cost event schema or id')
        # Validate the proposed phase/evidence before touching the append-only file.
        event_accounting(rows+[event])
        if any(x['event_id'] == event['event_id'] for x in rows):
            raise ValueError('duplicate cost event id')
        encoded = (json.dumps(event, sort_keys=True, separators=(',', ':'), allow_nan=False)+'\n').encode()
        with path.open('ab', buffering=0) as stream:
            if stream.write(encoded) != len(encoded):
                raise OSError('short cost event append')
            os.fsync(stream.fileno())
        return event
    if already_locked:
        return append_locked()
    with lock_path.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        return append_locked()


def cumulative_cost(root, rows=None):
    """Match continuation accounting: external events + native attempts + controller overhead."""
    if rows is None:
        rows, _ = validate_event_chain(root/'COST_EVENTS.jsonl')
    total = sum(event_accounting(rows))
    for start in root.glob('**/attempt_*/START.json'):
        receipt = start.parent/'RECEIPT.json'
        if receipt.exists():
            row = json.loads(receipt.read_text())
        else:
            recon, process = start.parent/'RECONCILIATION.json', start.parent/'PROCESS.json'
            if not recon.is_file() or not process.is_file():
                raise ValueError('unfinished native attempt requires reconciliation')
            row, proc = json.loads(recon.read_text()), json.loads(process.read_text())
            if (row.get('status') != 'FAILED_INTERRUPTED_RECONCILED' or
                    row.get('start_sha256') != sha_file(start) or row.get('process_sha256') != sha_file(process) or
                    row.get('task_id') != json.loads(start.read_text()).get('task_id') or
                    row.get('process_tree_checked') is not True or row.get('pgid') != proc.get('pgid') or
                    type(proc.get('pgid')) is not int or proc['pgid'] <= 1 or
                    row.get('charged_new_seconds', -1) < 960):
                raise ValueError('invalid native reconciliation')
            try:
                os.killpg(proc['pgid'], 0)
            except ProcessLookupError:
                pass
            else:
                raise ValueError('native process group still exists')
        value = row.get('charged_new_seconds')
        if type(value) not in (int, float) or not math.isfinite(value) or value < 0:
            raise ValueError('invalid native cost')
        total += value
    for start in root.glob('**/controller/invocation_*/START.json'):
        receipt = start.parent/'RECEIPT.json'
        if not receipt.exists():
            # A concurrent/interrupted controller has no closed exclusive overhead bound.
            raise ValueError('unfinished continuation controller requires reconciliation')
        value = json.loads(receipt.read_text()).get('charged_seconds')
        if type(value) not in (int, float) or not math.isfinite(value) or value < 0:
            raise ValueError('invalid controller cost')
        total += value
    return total


def proc_group(pgid: int) -> tuple[int, int, int]:
    ticks = rss = count = 0
    page = os.sysconf('SC_PAGE_SIZE')
    for stat_path in Path('/proc').glob('[0-9]*/stat'):
        try:
            raw = stat_path.read_text()
            rest = raw[raw.rfind(')') + 2:].split()
            if int(rest[2]) != pgid:
                continue
            ticks += int(rest[11]) + int(rest[12])
            rss += int(stat_path.with_name('statm').read_text().split()[1]) * page
            count += 1
        except (OSError, ValueError, IndexError):
            continue
    return ticks, rss, count


def terminate_group(proc: subprocess.Popen, grace: float = 2.0) -> None:
    for sig, delay in ((signal.SIGTERM, grace), (signal.SIGKILL, 2.0)):
        try:
            os.killpg(proc.pid, sig)
        except ProcessLookupError:
            pass
        try:
            proc.wait(timeout=delay)
            if proc.poll() is not None:
                # Descendants may outlive a reaped leader; still issue SIGKILL below.
                if sig == signal.SIGKILL:
                    return
        except subprocess.TimeoutExpired:
            pass


def run_child(argv, cwd, env, cpu, timeout_s, idle_s, rss_limit, output_limit,
              sample_s, stdout_path, stderr_path, resources_path, *, campaign_deadline=None):
    global ACTIVE_PROC, ACTIVE_ATTEMPT_DIR
    started, start_utc = time.monotonic(), utc_now().isoformat()
    proc, status, error, actual_affinity = None, 'COMPLETED', None, None
    resources, counts = [], {'stdout': 0, 'stderr': 0}
    selector = selectors.DefaultSelector()
    ACTIVE_ATTEMPT_DIR = stdout_path.parent
    with stdout_path.open('xb', buffering=0) as out, stderr_path.open('xb', buffering=0) as err, resources_path.open('x') as res:
        streams = {'stdout': out, 'stderr': err}
        try:
            if cpu not in os.sched_getaffinity(0):
                raise ValueError('requested CPU unavailable')
            # Last potentially blocking setup precedes this deadline check and spawn.
            remaining = min(timeout_s, absolute_remaining()-FINALIZE_RESERVE_SECONDS)
            if campaign_deadline is not None:
                remaining = min(remaining, campaign_deadline-time.monotonic()-FINALIZE_RESERVE_SECONDS)
            if remaining <= 0:
                raise RuntimeError('deadline reached directly before spawn')
            child_deadline = time.monotonic()+remaining
            proc = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True,
                                    preexec_fn=lambda: os.sched_setaffinity(0, {cpu}), bufsize=0)
            ACTIVE_PROC = proc
            write_new(stdout_path.parent/(stdout_path.name+'.PROCESS.json'),
                      {'pid': proc.pid, 'pgid': proc.pid, 'argv': argv, 'started_utc': start_utc})
            actual_affinity = sorted(os.sched_getaffinity(proc.pid))
            if actual_affinity != [cpu]:
                raise ValueError('child CPU affinity mismatch')
            for kind in streams:
                selector.register(getattr(proc, kind), selectors.EVENT_READ, kind)
            ticks, _, _ = proc_group(proc.pid)
            last_progress = last_sample = time.monotonic()
            while True:
                now = time.monotonic()
                wait = min(sample_s, max(0.001, child_deadline-now), max(0.001, idle_s-(now-last_progress)))
                for key, _ in selector.select(wait):
                    chunk = os.read(key.fileobj.fileno(), 65536)
                    if not chunk:
                        selector.unregister(key.fileobj)
                        continue
                    streams[key.data].write(chunk)
                    counts[key.data] += len(chunk)
                    last_progress = time.monotonic()
                now = time.monotonic()
                cpu_ticks, rss, count = proc_group(proc.pid)
                if cpu_ticks > ticks:
                    ticks, last_progress = cpu_ticks, now
                if now-last_sample >= sample_s or proc.poll() is not None:
                    sample = {'elapsed_seconds': now-started, 'process_group_cpu_seconds': cpu_ticks/os.sysconf('SC_CLK_TCK'),
                              'process_group_rss_bytes_sum': rss, 'process_group_process_count': count,
                              'stdout_bytes': counts['stdout'], 'stderr_bytes': counts['stderr']}
                    resources.append(sample)
                    res.write(json.dumps(sample, sort_keys=True)+'\n'); res.flush()
                    last_sample = now
                if sum(counts.values()) > output_limit:
                    status = 'FAILED_OUTPUT_LIMIT'
                elif rss > rss_limit:
                    status = 'FAILED_RSS_LIMIT'
                elif now >= child_deadline or absolute_remaining() <= FINALIZE_RESERVE_SECONDS:
                    status = 'FAILED_HARD_DEADLINE'
                elif now-last_progress >= idle_s:
                    status = 'FAILED_IDLE_WATCHDOG'
                if status != 'COMPLETED' or proc.poll() is not None and count == 0 and not selector.get_map():
                    break
        except BaseException as exc:
            status = 'FAILED_OBSERVER' if proc is not None else 'FAILED_PREFLIGHT_OR_SPAWN'
            error = f'{type(exc).__name__}: {exc}'
        finally:
            # Cleanup failures remain observations; they cannot bypass reaping or terminal receipts.
            try:
                if proc is not None:
                    terminate_group(proc, grace=0.1)
                    proc.wait(timeout=2)
                    end = time.monotonic()+0.5
                    while selector.get_map() and time.monotonic() < end:
                        for key, _ in selector.select(0.01):
                            chunk = os.read(key.fileobj.fileno(), 65536)
                            if not chunk:
                                selector.unregister(key.fileobj)
                            else:
                                streams[key.data].write(chunk); counts[key.data] += len(chunk)
            except BaseException as exc:
                error = error or f'{type(exc).__name__}: {exc}'
                status = 'FAILED_OBSERVER'
            finally:
                try:
                    if proc is not None:
                        terminate_group(proc, grace=0.0)
                        proc.wait(timeout=2)
                finally:
                    selector.close()
                    if proc is not None:
                        for pipe in (proc.stdout, proc.stderr):
                            if pipe is not None:
                                pipe.close()
                    ACTIVE_PROC = ACTIVE_ATTEMPT_DIR = None
            for stream in (out, err, res):
                try:
                    stream.flush(); os.fsync(stream.fileno())
                except OSError as exc:
                    status, error = 'FAILED_OBSERVER', error or f'{type(exc).__name__}: {exc}'
    code = proc.returncode if proc is not None else None
    if status == 'COMPLETED' and code != 0:
        status = 'FAILED_CHILD_EXIT'
    return {'status': status, 'exit_code': code, 'error': error, 'actual_cpu_affinity': actual_affinity,
            'elapsed_seconds': time.monotonic()-started, 'started_utc': start_utc, 'finished_utc': utc_now().isoformat(),
            'stdout_bytes': counts['stdout'], 'stderr_bytes': counts['stderr'],
            'stdout_sha256': sha_file(stdout_path), 'stderr_sha256': sha_file(stderr_path),
            'resources_sha256': sha_file(resources_path), 'resources': resources}


def process_conflicts(blocked_tokens: list[str]) -> list[dict]:
    conflicts = []
    own = os.getpid()
    for p in Path('/proc').glob('[0-9]*'):
        try:
            pid = int(p.name)
            if pid == own:
                continue
            cmd = (p/'cmdline').read_bytes().replace(b'\0', b' ').decode(errors='replace')
            if any(token and token in cmd for token in blocked_tokens):
                conflicts.append({'pid': pid, 'cmdline': cmd[:800]})
        except (OSError, ValueError):
            continue
    return conflicts


def validate_global_binding(plan):
    root = Path(plan['cost_root']).resolve()
    if (root != GLOBAL_COST_ROOT.resolve() or plan['new_48h_cap_seconds'] != GLOBAL_HARD_CAP_SECONDS or
            plan['hard_deadline_utc'] != GLOBAL_DEADLINE or
            Path(plan['execution_lock_path']).resolve() != root/EXECUTION_LOCK_NAME):
        raise ValueError('plan must bind fixed global cost root, 172800-second cap, deadline and shared execution lock')
    return root


def current_cost_gate(plan):
    root = validate_global_binding(plan)
    rows, event_sha = validate_event_chain(root/'COST_EVENTS.jsonl')
    if event_sha != plan['expected_cost_events_sha256']:
        raise ValueError('COST_EVENTS changed since plan review; refresh ledger/plan')
    ledger_info = locked_file(plan['cost_ledger_path'], plan['cost_ledger_sha256'])
    ledger = json.loads(Path(ledger_info['path']).read_text())
    total = cumulative_cost(root, rows)
    if (ledger.get('events_sha256_at_snapshot') != event_sha or ledger.get('cost_event_count') != len(rows) or
            ledger.get('hard_deadline_utc') != GLOBAL_DEADLINE or
            ledger.get('new_48h_cap_seconds') != GLOBAL_HARD_CAP_SECONDS):
        raise ValueError('COST ledger does not match current events/deadline/cap')
    if total+plan['campaign_reservation_seconds'] > GLOBAL_HARD_CAP_SECONDS:
        raise ValueError('48-hour cumulative cap cannot admit full campaign reservation')
    if absolute_remaining() <= FINALIZE_RESERVE_SECONDS:
        raise ValueError('hard absolute UTC deadline leaves no safe launch budget')
    return rows, event_sha, total


def task_script(task):
    p = Path(task['julia_script_path']).resolve(strict=True)
    if EXPECTED_SCRIPTS.get(task['case']) != p.name or task.get('mode') != 'solve':
        raise ValueError('case-specific Julia script or solve mode mismatch')
    return locked_file(p, task['julia_script_sha256'])


def task_descriptor(task):
    info = locked_file(task['descriptor_path'], task['descriptor_sha256'])
    descriptor = json.loads(Path(info['path']).read_text())
    if (type(descriptor.get('case_id')) is not int or
            descriptor['case_id'] != task['case'] or descriptor.get('size') != 'standard'):
        raise ValueError('descriptor case/size must match the single selected standard task')
    return info


def task_sparse_support(task):
    """The unconditional include used by case8/case20 must resolve to this sibling."""
    script = Path(task_script(task)['path'])
    sibling = script.with_name('sparse_jacobian_support.jl')
    declared = Path(task['sparse_support_path'])
    if sibling.is_symlink() or declared.is_symlink() or declared.absolute() != sibling:
        raise ValueError('sparse support must be the actual script sibling path')
    if declared.resolve(strict=True) != sibling:
        raise ValueError('sparse support sibling path mismatch')
    return locked_file(sibling, task['sparse_support_sha256'])


def verify_shared_lock(plan):
    info = locked_file(plan['shared_execution_lock_receipt']['path'], plan['shared_execution_lock_receipt']['sha256'])
    review = json.loads(Path(info['path']).read_text())
    participants = plan['shared_execution_lock_participants']
    if (review.get('schema') != 'vigilode.shared-execution-lock-review.v1' or
            review.get('decision') != 'PASS_SHARED_EXECUTION_LOCK' or
            review.get('execution_lock_path') != str(GLOBAL_COST_ROOT/EXECUTION_LOCK_NAME) or
            review.get('participants') != participants or
            {Path(x['path']).name for x in participants} != {'julia_comparison.py', 'continuation_runner.py'}):
        raise ValueError('HOLD: shared lock participation requires independent hash-bound review')
    return [locked_file(x['path'], x['sha256']) for x in participants]


def verify_locks(plan, evidence_dir, campaign_deadline, *, observational=False):
    def observe(fn):
        if not observational:
            return fn()
        try:
            return fn()
        except BaseException as exc:
            return {'observation_error': f'{type(exc).__name__}: {exc}'}
    source = observe(lambda: git_identity(plan['source']['repo'], evidence_dir, campaign_deadline))
    expected = {'head': plan['source']['head'], 'tree': plan['source']['tree'], 'status_porcelain': ''}
    if source != expected and not observational:
        raise ValueError('source HEAD/tree is not locked clean checkout')
    fields = ('controller', 'descriptor_manifest', 'project', 'manifest', 'julia_executable', 'terminal_matrix')
    locks = {k: observe(lambda k=k: locked_file(plan[k+'_path'], plan[k+'_sha256'])) for k in fields}
    locks['approval'] = observe(lambda: verify_plan_approval(plan))
    for task in plan['tasks']:
        locks[f"script_{task['case']}"] = observe(lambda task=task: task_script(task))
        locks[f"descriptor_{task['case']}"] = observe(lambda task=task: task_descriptor(task))
        if task['case'] in (8, 20):
            locks[f"sparse_support_{task['case']}"] = observe(lambda task=task: task_sparse_support(task))
    locks['shared_lock_participants'] = observe(lambda: verify_shared_lock(plan))
    if not observational:
        selected_task(plan)
        if Path(locks['project']['path']).parent != Path(locks['manifest']['path']).parent:
            raise ValueError('Project/Manifest environment mismatch')
        if not Path(plan['isolated_depot_path']).is_dir():
            raise ValueError('isolated depot missing')
        exe = Path(locks['julia_executable']['path'])
        if exe.name != 'julia' or exe.parent.name != 'bin' or not exe.parent.parent.name.startswith('julia-'):
            raise ValueError('actual versioned Julia executable required, not julialauncher')
    return {'source': source, 'locks': locks}


def launch(plan_path):
    global EXEC_LOCK_STREAM
    started = time.monotonic()
    raw = plan_path.read_bytes()
    plan = json.loads(raw)
    if plan.get('schema') != SCHEMA:
        raise ValueError('plan schema mismatch')
    if plan.get('status') != 'APPROVED' or plan.get('launch_enabled') is not True:
        raise ValueError('plan remains nonlaunchable: explicit post-freeze approval required')
    task = selected_task(plan)
    approval = verify_plan_approval(plan)
    root = validate_global_binding(plan)
    if (plan.get('cpu') != 0 or plan.get('thread_count') != 1 or plan.get('poll_seconds') != 5 or
            plan.get('idle_timeout_seconds') != 120 or plan.get('child_hard_timeout_seconds') != 900 or
            plan.get('overall_hard_timeout_seconds') != CONTROLLER_HARD_CAP_SECONDS or
            plan.get('campaign_reservation_seconds') != CONTROLLER_HARD_CAP_SECONDS or
            plan.get('finalize_reserve_seconds') != FINALIZE_RESERVE_SECONDS or
            plan.get('rss_limit_bytes') != 12*1024**3):
        raise ValueError('approved plan changes frozen execution limits')
    deadline_mono = started+min(CONTROLLER_HARD_CAP_SECONDS, absolute_remaining())
    if deadline_mono-time.monotonic() <= FINALIZE_RESERVE_SECONDS:
        raise ValueError('absolute deadline has passed')
    output = Path(plan['output_root']).resolve()
    if not output.is_relative_to(root) or output == root:
        raise ValueError('output must be fresh directory under global root')
    lock = (root/EXECUTION_LOCK_NAME).open('a+')
    try:
        fcntl.flock(lock, fcntl.LOCK_EX|fcntl.LOCK_NB)
    except BlockingIOError:
        lock.close()
        raise RuntimeError('shared execution lock is held by another campaign')
    EXEC_LOCK_STREAM = lock
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline_mono-time.monotonic()-FINALIZE_RESERVE_SECONDS))
    campaign_id = 'independent-julia-reference-'+uuid.uuid4().hex
    reserved, results, failure, summary = False, [], None, None
    try:
        with (root/'COST_EVENTS.lock').open('a+') as cost_lock:
            fcntl.flock(cost_lock, fcntl.LOCK_EX)
            rows, event_sha, total = current_cost_gate(plan)
            output.mkdir(mode=0o700, exist_ok=False)
            start_path = output/'CAMPAIGN_START.json'
            write_new(start_path, {'campaign_id': campaign_id, 'plan_path': str(plan_path.resolve()),
                'plan_sha256': hashlib.sha256(raw).hexdigest(), 'plan_core_sha256': plan_core_sha(plan),
                'selected_case': task['case'], 'approval': approval, 'started_utc': utc_now().isoformat(),
                'prior_charged_plus_reserved_seconds': total, 'cost_events_sha256_before': event_sha,
                'reserved_seconds': plan['campaign_reservation_seconds'], 'includes_preflight': True})
            append_event(root/'COST_EVENTS.jsonl', root/'COST_EVENTS.lock', {
                'schema':'vigilode.local-continuation-cost-event.v1', 'event_id':campaign_id+'-started',
                'attempt_id':campaign_id, 'category':'julia', 'phase':'STARTED',
                'charge_upper_seconds':plan['campaign_reservation_seconds'],
                'evidence_path':str(start_path), 'evidence_sha256':sha_file(start_path), 'observed_utc':utc_now().isoformat()},
                already_locked=True, expected_head=canonical_sha(rows[-1]) if rows else ZERO_SHA)
            reserved = True
        # Every subprocess below is covered by the already durable reservation.
        preflight = output/'preflight'; preflight.mkdir()
        verify_plan_approval(plan)
        locks = verify_locks(plan, preflight, deadline_mono)
        if json.loads(Path(plan['terminal_matrix_path']).read_text()).get('schema') != 'vigilode.local-continuation-terminal-matrix.v1':
            raise ValueError('terminal matrix schema mismatch')
        version = bounded_command([locks['locks']['julia_executable']['path'], '--version'], plan['source']['repo'],
                                  preflight, 'julia_version', deadline_mono)
        if version != 'julia version '+plan['julia_version']:
            raise ValueError('actual Julia executable version mismatch')
        if locked_file(plan['julia_executable_path'], plan['julia_executable_sha256']) != locks['locks']['julia_executable']:
            raise ValueError('Julia executable mutated during version probe')
        conflicts = process_conflicts(plan['blocked_process_tokens'])
        if conflicts:
            raise ValueError(f'conflicting process detected: {conflicts}')
        env = os.environ.copy()
        env.update({k:'1' for k in THREAD_VARS})
        env.update({'JULIA_DEPOT_PATH':str(Path(plan['isolated_depot_path']).resolve()),
                    'JULIA_PROJECT':str(Path(plan['project_path']).resolve()), 'JULIA_LOAD_PATH':'@:@stdlib'})
        for ordinal, task in enumerate(plan['tasks'], 1):
            folder = output/f"{ordinal:02d}_case{task['case']}"; folder.mkdir()
            before_dir = folder/'before'; before_dir.mkdir()
            before = verify_locks(plan, before_dir, deadline_mono)
            with (root/'COST_EVENTS.lock').open('a+') as cost_lock:
                fcntl.flock(cost_lock, fcntl.LOCK_EX)
                if cumulative_cost(root) > GLOBAL_HARD_CAP_SECONDS:
                    raise ValueError('global cumulative cap exceeded before child')
            argv = [before['locks']['julia_executable']['path'], '--startup-file=no', '--history-file=no', '--threads=1',
                    '--project='+str(Path(plan['project_path']).resolve()), before['locks'][f"script_{task['case']}"]['path'],
                    str(Path(task['descriptor_path']).resolve()), str(folder/'reference_output'), 'solve']
            write_new(folder/'START.json', {'case':task['case'], 'argv':argv, 'locks_before':before,
                       'started_utc':utc_now().isoformat(), 'execution_lock_path':str(root/EXECUTION_LOCK_NAME)})
            # run_child checks the absolute deadline again immediately before Popen.
            process_started = time.monotonic()
            try:
                status = run_child(argv, plan['source']['repo'], env, plan['cpu'], plan['child_hard_timeout_seconds'],
                                   plan['idle_timeout_seconds'], plan['rss_limit_bytes'], plan['output_limit_bytes'],
                                   plan['poll_seconds'], folder/'stdout.raw', folder/'stderr.raw', folder/'resources.jsonl',
                                   campaign_deadline=deadline_mono)
            except BaseException as exc:
                # run_child owns cleanup; retain a terminal observation even if capture finalization fails.
                if ACTIVE_PROC is not None:
                    terminate_group(ACTIVE_PROC, grace=0.0)
                status = {'status':'FAILED_OBSERVER', 'exit_code':None,
                          'elapsed_seconds':time.monotonic()-process_started, 'error':f'{type(exc).__name__}: {exc}'}
            after_dir = folder/'after'; after_dir.mkdir()
            try:
                after = verify_locks(plan, after_dir, deadline_mono, observational=True)
            except BaseException as exc:
                after = {'observation_error':f'{type(exc).__name__}: {exc}'}
            mutation = before != after
            outputs = []
            for p in sorted((folder/'reference_output').glob('*')):
                if p.is_file():
                    outputs.append({'path':str(p), 'bytes':p.stat().st_size, 'sha256':sha_file(p)})
            gate = 'PASS_PROCESS' if status['status']=='COMPLETED' and status['exit_code']==0 else 'FAILED_PROCESS'
            if not set(task['required_outputs']).issubset({Path(x['path']).name for x in outputs}):
                gate = 'FAILED_REQUIRED_OUTPUTS'
            if mutation:
                gate = 'FAILED_INPUT_MUTATION'
            receipt = {'case':task['case'], 'controller_gate':gate, 'process':status, 'outputs':outputs,
                       'locks_after':after, 'source_or_input_mutation':mutation,
                       'scientific_status':'NOT_ASSESSED_BY_CONTROLLER', 'finished_utc':utc_now().isoformat()}
            write_new(folder/'TERMINAL.json', receipt)
            results.append(receipt)
            if gate != 'PASS_PROCESS':
                break
    except BaseException as exc:
        failure = f'{type(exc).__name__}: {exc}'
    finally:
        # A reservation remains open if terminal evidence/accounting itself cannot be completed.
        try:
            if reserved:
                summary = {'campaign_id':campaign_id, 'status': 'FAILED_CONTROLLER' if failure else
                    ('COMPLETED_ALL_CHILDREN' if len(results)==len(plan['tasks']) and all(x['controller_gate']=='PASS_PROCESS' for x in results)
                     else 'STOPPED_ON_FIRST_FAILURE'), 'error':failure, 'children':results,
                    'campaign_wall_seconds':time.monotonic()-started, 'includes_preflight_and_controller_overhead':True,
                    'finished_utc':utc_now().isoformat(), 'claim_ceiling':'process/artifact collection only'}
                summary_path = output/'CAMPAIGN_SUMMARY.json'; write_new(summary_path, summary)
                append_event(root/'COST_EVENTS.jsonl', root/'COST_EVENTS.lock', {
                    'schema':'vigilode.local-continuation-cost-event.v1', 'event_id':campaign_id+'-terminal',
                    'attempt_id':campaign_id, 'category':'julia', 'phase':'TERMINAL',
                    'charged_seconds':time.monotonic()-started, 'charge_basis':'whole_serial_controller_wall_including_preflight',
                    'exact_duration_known':True, 'evidence_path':str(summary_path), 'evidence_sha256':sha_file(summary_path),
                    'observed_utc':utc_now().isoformat()})
        except BaseException as exc:
            if reserved:
                write_new(output/'COST_TERMINAL_FAILURE.json', {'error':f'{type(exc).__name__}: {exc}', 'prior_error':failure})
            failure = failure or f'{type(exc).__name__}: {exc}'
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            release_execution_lock()
    if failure and summary is None:
        raise RuntimeError(failure)
    if failure and summary is not None and summary['error'] is None:
        raise RuntimeError(failure)
    return summary


def hard_cap_handler(_signum, _frame):
    proc = ACTIVE_PROC
    if proc is not None:
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    raise TimeoutError('earliest controller/global cleanup deadline fired; finalization reserve remains')


def release_execution_lock():
    global EXEC_LOCK_STREAM
    if EXEC_LOCK_STREAM is not None:
        try:
            fcntl.flock(EXEC_LOCK_STREAM, fcntl.LOCK_UN)
        finally:
            EXEC_LOCK_STREAM.close()
            EXEC_LOCK_STREAM = None


def main():
    signal.signal(signal.SIGALRM, hard_cap_handler)
    parser = argparse.ArgumentParser()
    parser.add_argument('--plan', type=Path, required=True)
    args = parser.parse_args()
    try:
        result = launch(args.plan)
    except Exception as exc:
        print(f'BLOCKED: {type(exc).__name__}: {exc}', file=sys.stderr)
        return 2
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        release_execution_lock()
    print(json.dumps({'status':result['status'], 'campaign_wall_seconds':result['campaign_wall_seconds']}))
    return 0 if result['status']=='COMPLETED_ALL_CHILDREN' else 1


if __name__ == '__main__':
    raise SystemExit(main())
