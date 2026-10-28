#!/usr/bin/env python3
"""Source-locked, serial accuracy cohort executor; requires independent approval.

The V2 selection is planning input. A materialized plan and a separate hash-bound
approval are required before any benchmark child is launched.
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
import signal
import subprocess
import time

import reference_launch_controller as cost

ROOT = Path(__file__).resolve().parent
SELECTION = ROOT / 'ACCURACY_COHORT_SELECTION_TEMPLATE_V2.json'
DEADLINE = '2026-10-27T08:07:00Z'
GLOBAL_CAP = 172800.0
COHORT_CAP = 16200.0
ATTEMPT_SECONDS = 180.0
ATTEMPT_CONTROLLER_RESERVE = 45.0
ATTEMPT_RESERVE = ATTEMPT_SECONDS + ATTEMPT_CONTROLLER_RESERVE
FINALIZE_RESERVE = 20.0
SCHEMA = 'vigilode.accuracy-cohort-plan.v1'
APPROVAL_SCHEMA = 'vigilode.accuracy-cohort-approval.v1'


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
    return cost.canonical_sha({k: v for k, v in plan.items() if k != 'approval_receipt'})


def git_identity(repo):
    def get(*args):
        return subprocess.check_output(['git', '-C', str(repo), *args], text=True,
                                       timeout=10).strip()
    return {'head': get('rev-parse', 'HEAD'),
            'tree': get('rev-parse', 'HEAD^{tree}'),
            'status': get('status', '--porcelain=v1', '--untracked-files=all')}


def verify_plan(plan, *, selection_path=SELECTION):
    if plan.get('schema') != SCHEMA or plan.get('status') != 'APPROVED_MATERIALIZED_PLAN' or \
            plan.get('deadline_utc') != DEADLINE or \
            plan.get('global_cap_seconds') != GLOBAL_CAP or \
            plan.get('cohort_cap_seconds') != COHORT_CAP:
        raise ValueError('plan schema or hard caps mismatch')
    if Path(plan.get('cost_root', '')).resolve() != ROOT.resolve():
        raise ValueError('cost root mismatch')
    if Path(plan.get('shared_lock_path', '')).resolve() != (ROOT/'.julia_comparison_campaign.lock').resolve():
        raise ValueError('shared execution lock mismatch')
    if Path(plan.get('cost_events_path', '')).resolve() != (ROOT/'COST_EVENTS.jsonl').resolve():
        raise ValueError('cost events path mismatch')
    if sha(selection_path) != plan.get('selection_sha256'):
        raise ValueError('V2 selection byte identity mismatch')
    selection = json.loads(Path(selection_path).read_text())
    if (selection.get('schema') != 'vigilode.local-continuation-accuracy-cohort-selection-template.v2'
            or len(selection.get('groups', [])) != 12 or len(selection.get('attempts', [])) != 72):
        raise ValueError('V2 selection contract mismatch')
    source = plan.get('source', {})
    if not all(source.get(x) for x in ('path', 'head', 'tree', 'cargo_lock_sha256', 'binary_path', 'binary_sha256')):
        raise ValueError('unbound source identity')
    if source['path'] != selection['frozen_source_path'] or \
            source['binary_path'] != selection['binary_path']:
        raise ValueError('source path differs from selection')
    if git_identity(source['path']) != {'head': source['head'], 'tree': source['tree'], 'status': ''}:
        raise ValueError('source checkout identity mismatch')
    if sha(Path(source['path'])/'Cargo.lock') != source['cargo_lock_sha256'] or \
            sha(source['binary_path']) != source['binary_sha256']:
        raise ValueError('Cargo.lock or binary identity mismatch')
    if Path(plan.get('output_root', '')).resolve().parent != ROOT.resolve():
        raise ValueError('output must be exclusive continuation child')
    if plan['output_root'] in (str(ROOT), str(selection_path)):
        raise ValueError('output root collision')
    refs = plan.get('references', {})
    if set(refs) != {'2', '3', '5'}:
        raise ValueError('missing or extra reference')
    for case, expected in selection['references'].items():
        got = refs[case]
        if got != {'path': expected['path'], 'sha256': expected['sha256'],
                   'descriptor_sha256': expected['descriptor_sha256']}:
            raise ValueError('reference binding differs from selection')
        if sha(got['path']) != got['sha256']:
            raise ValueError('reference file SHA mismatch')
        reference = json.loads(Path(got['path']).read_text())
        if (reference.get('model_descriptor_sha256') != got['descriptor_sha256'] or
                reference.get('norm') != 'max_component_absolute' or
                len(reference.get('reference_state', [])) != reference['model_descriptor']['model_contract']['dimension'] or
                type(reference.get('uncertainty')) not in (int, float) or
                not math.isfinite(reference['uncertainty']) or reference['uncertainty'] < 0):
            raise ValueError('reference full-vector or uncertainty contract mismatch')
        if sha(expected['frozen_binary_describe_path']) != got['descriptor_sha256']:
            raise ValueError('saved frozen binary descriptor mismatch')
    if 0 not in os.sched_getaffinity(0):
        raise ValueError('CPU0 unavailable')
    approval_ref = plan.get('approval_receipt', {})
    if sha(approval_ref['path']) != approval_ref['sha256']:
        raise ValueError('approval receipt SHA mismatch')
    approval = json.loads(Path(approval_ref['path']).read_text())
    if (approval.get('schema') != APPROVAL_SCHEMA or
            approval.get('decision') != 'APPROVED_ACCURACY_COHORT_V2' or
            approval.get('plan_core_sha256') != plan_core_sha(plan) or
            approval.get('source_head') != source['head'] or
            approval.get('source_tree') != source['tree'] or
            approval.get('binary_sha256') != source['binary_sha256'] or
            approval.get('admitted_reference_sha256') !=
            {case: refs[case]['sha256'] for case in ('2', '3', '5')}):
        raise ValueError('independent approval does not bind plan')
    for item in selection['attempts']:
        argv = item['argv']
        if (item['threads'] != 1 or item['cpus'] != [0] or
                argv[0] != source['binary_path'] or
                argv[argv.index('--threads')+1] != '1' or
                argv[argv.index('--reference-json')+1] != refs[str(item['case'])]['path'] or
                argv[argv.index('--reference-sha256')+1] != refs[str(item['case'])]['sha256'] or
                argv[argv.index('--error-limit')+1] != '1e-4' or '--emit-state' not in argv):
            raise ValueError('attempt command contract mismatch')
    return selection


def remaining_deadline():
    return (dt.datetime.fromisoformat(DEADLINE.replace('Z', '+00:00'))-
            dt.datetime.now(dt.timezone.utc)).total_seconds()


def read_terminal(folder):
    terminal = folder/'TERMINAL.json'
    if terminal.exists():
        return json.loads(terminal.read_text())
    if folder.exists():
        raise ValueError(f'unfinished attempt requires independent reconciliation: {folder}')
    return None


def assess(stdout, case, reference):
    records = []
    for line in Path(stdout).read_text(errors='replace').splitlines():
        try:
            records.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    results = [r for r in records if isinstance(r, dict) and r.get('kind') == 'result']
    if len(results) != 1:
        return {'status': 'OUTPUT_CONTRACT_FAILURE', 'reason': 'exactly one result required'}
    r = results[0]
    if (r.get('case') == case and r.get('reference_sha256') == reference['sha256'] and
            r.get('model_descriptor_sha256') == reference['descriptor_sha256'] and
            r.get('ensemble_count') == 256 and r.get('status') == 'SOLVER_FAILED' and
            r.get('solver_success') is False):
        return {'status': 'SOLVER_FAILED', 'member_successes': r.get('member_successes'),
                'reported_status': 'SOLVER_FAILED'}
    e, u, difference = (r.get('actual_error'), r.get('reference_absolute_bound'),
                        r.get('endpoint_reference_difference'))
    if (r.get('case') != case or r.get('ensemble_count') != 256 or
            r.get('member_successes') != 256 or r.get('accuracy_scope') !=
            'maximum component absolute endpoint error across every ensemble member; no continuous/phase-global certificate' or
            r.get('reference_sha256') != reference['sha256'] or
            r.get('model_descriptor_sha256') != reference['descriptor_sha256'] or
            type(e) not in (int, float) or type(u) not in (int, float) or
            type(difference) not in (int, float) or
            not math.isfinite(e) or not math.isfinite(u) or
            not math.isfinite(difference) or e < 0 or u < 0 or difference < 0 or
            abs(e-(difference+u)) > 8*math.ulp(max(1.0, e)) or
            u != reference['uncertainty'] or
            type(r.get('maximum_error_member_index')) is not int or
            not 0 <= r['maximum_error_member_index'] < 256 or
            r.get('error_limit') != 1e-4 or not isinstance(r.get('final_state'), list)):
        return {'status': 'OUTPUT_CONTRACT_FAILURE', 'reason': 'result identity or all-member error contract'}
    passed = r.get('status') == 'PASS_SCOPED' and r.get('solver_success') is True and e <= 1e-4
    return {'status': 'PASS_SCOPED' if passed else r.get('status', 'ACCURACY_OR_SOLVER_FAILED'),
            'actual_error': e, 'reference_absolute_bound': u,
            'endpoint_reference_difference': difference,
            'reported_status': r.get('status')}


def kill_group(proc):
    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    proc.wait()


def child_rss_bytes(pid):
    try:
        for line in (Path('/proc')/str(pid)/'status').read_text().splitlines():
            if line.startswith('VmRSS:'):
                return int(line.split()[1])*1024
    except (OSError, ValueError, IndexError):
        return 0
    return 0


def execute_attempt(item, plan, reference, folder):
    start = time.monotonic()
    source = plan['source']
    if sha(source['binary_path']) != source['binary_sha256'] or \
            sha(Path(source['path'])/'Cargo.lock') != source['cargo_lock_sha256'] or \
            sha(reference['path']) != reference['sha256']:
        raise ValueError('pre-attempt immutable input mutation')
    folder.mkdir(parents=True, exist_ok=False)
    start_record = {'schema': 'vigilode.accuracy-attempt-start.v1', 'attempt_id': item['attempt_id'],
                    'selection_sha256': plan['selection_sha256'], 'source': source,
                    'argv': item['argv'], 'reference': reference,
                    'started_utc': dt.datetime.now(dt.timezone.utc).isoformat()}
    new_json(folder/'START.json', start_record)
    env = {**os.environ, **item['env_overrides'], 'VIGILODE_SOURCE_COMMIT': source['head']}
    stdout, stderr, resource = (folder/n for n in ('stdout.bin', 'stderr.bin', 'resource.txt'))
    status = 'SPAWN_FAILED'; exit_code = None; pid = None
    try:
        with stdout.open('xb') as out, stderr.open('xb') as err:
            # /usr/bin/time writes a distinct resource record, even on child failure.
            cmd = ['/usr/bin/time', '-v', '-o', str(resource), '--', *item['argv']]
            def pin():
                os.sched_setaffinity(0, {0})
                if os.sched_getaffinity(0) != {0}:
                    raise RuntimeError('CPU0 affinity failed')
            proc = subprocess.Popen(cmd, cwd=source['path'], env=env, stdout=out,
                                    stderr=err, start_new_session=True, preexec_fn=pin)
            pid = proc.pid
            new_json(folder/'PROCESS.json', {'pid': pid, 'pgid': pid, 'cpu0_requested': True})
            while True:
                exit_code = proc.poll()
                if exit_code is not None:
                    status = 'COMPLETED' if exit_code == 0 else 'NONZERO_EXIT'
                    break
                if (stdout.stat().st_size > item['stdout_limit_bytes'] or
                        stderr.stat().st_size > item['stderr_limit_bytes']):
                    status = 'OUTPUT_LIMIT'
                elif child_rss_bytes(pid) > item['max_rss_bytes']:
                    status = 'MEMORY_LIMIT'
                elif time.monotonic()-start >= ATTEMPT_SECONDS or remaining_deadline() <= FINALIZE_RESERVE:
                    status = 'TIMEOUT'
                if status != 'SPAWN_FAILED':
                    kill_group(proc)
                    exit_code = proc.returncode
                    break
                time.sleep(0.1)
    except Exception as error:
        if 'proc' in locals() and proc.poll() is None:
            kill_group(proc)
        status = 'SPAWN_OR_CONTROLLER_FAILURE'
        exit_code = proc.returncode if 'proc' in locals() else None
        stdout.touch(exist_ok=True)
        with stderr.open('ab') as err:
            err.write((type(error).__name__+': '+str(error)+'\n').encode())
        resource.touch(exist_ok=True)
    except BaseException:
        if 'proc' in locals() and proc.poll() is None:
            kill_group(proc)
        raise
    elapsed = time.monotonic()-start
    mutation = (sha(source['binary_path']) != source['binary_sha256'] or
                sha(Path(source['path'])/'Cargo.lock') != source['cargo_lock_sha256'] or
                sha(reference['path']) != reference['sha256'])
    assessment = ({'status': 'IMMUTABLE_INPUT_MUTATED'} if mutation else
                  assess(stdout, item['case'], reference) if status == 'COMPLETED' else {'status': status})
    terminal = {'schema': 'vigilode.accuracy-attempt-terminal.v1', 'attempt_id': item['attempt_id'],
                'execution_status': status, 'exit_code': exit_code, 'pid': pid,
                'elapsed_seconds': elapsed, 'assessment': assessment,
                'stdout_sha256': sha(stdout), 'stderr_sha256': sha(stderr),
                'resource_sha256': sha(resource) if resource.exists() else None,
                'finished_utc': dt.datetime.now(dt.timezone.utc).isoformat()}
    new_json(folder/'TERMINAL.json', terminal)
    return terminal


def execute(plan_path):
    plan = json.loads(Path(plan_path).read_text())
    lock_path = ROOT/'.julia_comparison_campaign.lock'
    with lock_path.open('a+') as exclusive:
        fcntl.flock(exclusive, fcntl.LOCK_EX)
        selection = verify_plan(plan)
        output = Path(plan['output_root'])
        if not output.exists():
            output.mkdir(mode=0o700)
            new_json(output/'IDENTITY.json', {'plan_sha256': sha(plan_path),
                                             'selection_sha256': plan['selection_sha256']})
        elif json.loads((output/'IDENTITY.json').read_text()) != \
                {'plan_sha256': sha(plan_path), 'selection_sha256': plan['selection_sha256']}:
            raise ValueError('resume output identity mismatch')
        groups = {g['group_id']: g for g in selection['groups']}
        attempts = selection['attempts']
        stop_reason = None
        for item in attempts:
            folder = output/'runs'/item['attempt_id']
            prior = read_terminal(folder)
            if prior:
                rows, _ = cost.validate_event_chain(ROOT/'COST_EVENTS.jsonl')
                phases = {r['phase'] for r in rows if r['attempt_id'] == item['attempt_id']}
                if phases != {'STARTED', 'TERMINAL'}:
                    raise ValueError('completed attempt lacks terminal global cost event')
                continue
            if not item['warmup']:
                warm = output/'runs'/f"{item['group_id']}_warmup"
                result = read_terminal(warm)
                if result is None:
                    raise ValueError('timed attempt lacks warmup')
                if result['assessment']['status'] != 'PASS_SCOPED':
                    continue  # Typed NOT_RUN_DEPENDENCY from summary; no fabricated launch.
            with (ROOT/'COST_EVENTS.lock').open('a+') as cost_lock:
                fcntl.flock(cost_lock, fcntl.LOCK_EX)
                if (sha(plan['source']['binary_path']) != plan['source']['binary_sha256'] or
                        sha(Path(plan['source']['path'])/'Cargo.lock') != plan['source']['cargo_lock_sha256'] or
                        sha(plan['references'][str(item['case'])]['path']) !=
                        plan['references'][str(item['case'])]['sha256']):
                    raise ValueError('pre-reservation immutable input mutation')
                rows, _ = cost.validate_event_chain(ROOT/'COST_EVENTS.jsonl')
                charged = cost.cumulative_cost(ROOT, rows)
                completed = sum(json.loads(x.read_text())['elapsed_seconds']+ATTEMPT_CONTROLLER_RESERVE
                                for x in (output/'runs').glob('*/TERMINAL.json')) if (output/'runs').exists() else 0
                if (charged+ATTEMPT_RESERVE+FINALIZE_RESERVE > GLOBAL_CAP or
                        completed+ATTEMPT_RESERVE > COHORT_CAP or
                        remaining_deadline() <= ATTEMPT_RESERVE+FINALIZE_RESERVE):
                    stop_reason = 'GLOBAL_OR_COHORT_COST_OR_DEADLINE_GATE'
                    break
                start_event = {'schema': 'vigilode.local-continuation-cost-event.v1',
                               'event_id': item['attempt_id']+'-started', 'attempt_id': item['attempt_id'],
                               'category': 'other', 'phase': 'STARTED',
                               'charge_upper_seconds': ATTEMPT_RESERVE,
                               'evidence_path': str(output/'IDENTITY.json'),
                               'evidence_sha256': sha(output/'IDENTITY.json'),
                               'observed_utc': dt.datetime.now(dt.timezone.utc).isoformat()}
                cost.append_event(ROOT/'COST_EVENTS.jsonl', ROOT/'COST_EVENTS.lock',
                                  start_event, already_locked=True)
            terminal = execute_attempt(item, plan, plan['references'][str(item['case'])], folder)
            with (ROOT/'COST_EVENTS.lock').open('a+') as cost_lock:
                fcntl.flock(cost_lock, fcntl.LOCK_EX)
                event = {'schema': 'vigilode.local-continuation-cost-event.v1',
                         'event_id': item['attempt_id']+'-terminal', 'attempt_id': item['attempt_id'],
                         'category': 'other', 'phase': 'TERMINAL',
                         'charged_seconds': terminal['elapsed_seconds']+ATTEMPT_CONTROLLER_RESERVE,
                         'evidence_path': str(folder/'TERMINAL.json'),
                         'evidence_sha256': sha(folder/'TERMINAL.json'),
                         'observed_utc': dt.datetime.now(dt.timezone.utc).isoformat()}
                cost.append_event(ROOT/'COST_EVENTS.jsonl', ROOT/'COST_EVENTS.lock', event,
                                  already_locked=True)
        summary = {'schema': 'vigilode.accuracy-cohort-summary.v1',
                   'plan_sha256': sha(plan_path), 'timing_authority': 'diagnostic_only',
                   'old_source_speed_ratio': 'NOT_EVALUABLE',
                   'stop_reason': stop_reason, 'groups': {}}
        for group_id, group in groups.items():
            receipts = [read_terminal(output/'runs'/aid) for aid in group['attempt_ids']]
            warmup_failed = receipts[0] is not None and receipts[0]['assessment']['status'] != 'PASS_SCOPED'
            summary['groups'][group_id] = {
                'status': ('PASS_SCOPED' if all(r and r['assessment']['status']=='PASS_SCOPED' for r in receipts)
                           else 'INCOMPLETE_OR_FAILED'),
                'attempts': [{'id': aid, 'status': r['assessment']['status'] if r else
                              'NOT_RUN_DEPENDENCY' if warmup_failed and i > 0 else 'NOT_RUN_BUDGET_OR_INTERRUPTED'}
                             for i, (aid, r) in enumerate(zip(group['attempt_ids'], receipts))]}
        snapshots = sorted(output.glob('SUMMARY_*.json'))
        if not snapshots or json.loads(snapshots[-1].read_text()) != summary:
            new_json(output/f'SUMMARY_{len(snapshots)+1:04}.json', summary)
        return summary


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--plan', required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(execute(args.plan), sort_keys=True))
