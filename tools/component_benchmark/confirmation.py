#!/usr/bin/env python3
"""Frozen, barriered P3 paired timing on fixed guest CPU lanes.

These timings remain diagnostic only. A missing resource or contention signal
holds a round; it is never discarded from the frozen 17-pair design.
"""
from __future__ import annotations

import argparse
import concurrent.futures
import json
import math
import os
from pathlib import Path
import platform
import random
import sys
import time

if __package__ in (None, ''):
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from tools.component_benchmark import hpc
from tools.component_benchmark.analysis import paired_log_cost
from tools.component_benchmark.budget import PHASE_SECONDS, locked_ledger
from tools.component_benchmark.protocol import digest, file_digest, strict_load, validate_case, validate_result
from tools.component_benchmark.runner import atomic_json, executable_binding, source_binding

MIB = 1 << 20
ARM_TIMEOUT = 600
ROUNDS = 17


def _arm(contrast, label, solvers):
    sid = contrast[label]
    if sid not in solvers:
        raise ValueError('contrast solver missing: '+sid)
    cid = contrast.get(label+'_config_id', 'default')
    explicit = contrast.get(label+'_solver_config')
    if explicit is not None:
        if not isinstance(explicit,dict) or not explicit or cid == 'default':
            raise ValueError('explicit solver_config needs nondefault frozen config ID')
        extra = explicit
    elif cid == 'default':
        extra = None
    elif isinstance(cid,str) and cid.startswith('h_divisor_') and cid.removeprefix('h_divisor_') in ('1000','2000','4000','8000'):
        extra = {'h_divisor':int(cid.removeprefix('h_divisor_'))}
    else:
        raise ValueError('unknown frozen solver configuration: '+str(cid))
    return {'solver_id':sid,'config_id':cid,'solver_spec':solvers[sid],
            'solver_config':extra}


def _rss(peak, arm):
    keys = (arm['solver_id']+'#'+arm['config_id'],arm['solver_id'])
    for key in keys:
        value = peak.get(key)
        if type(value) is int and value > 0:
            return value
    return None


def freeze(config, resources, source, bindings):
    contrasts = config.get('global_contrasts')
    cases = config.get('cases')
    solvers = config.get('solvers')
    if not isinstance(contrasts,list) or not 1 <= len(contrasts) <= 16:
        raise ValueError('one to sixteen frozen global contrasts required')
    if not isinstance(cases,list) or not isinstance(solvers,dict):
        raise ValueError('cases and solvers required')
    by_case = {case['case_id']:case for case in cases}
    if len(by_case) != len(cases):
        raise ValueError('duplicate case_id')
    for case in cases:
        errors = validate_case(case)
        if errors:
            raise ValueError(str(errors))
    peak = config.get('screened_peak_rss_bytes', {})
    if not isinstance(peak,dict):
        raise ValueError('screened_peak_rss_bytes must be a map')
    prepared = []
    missing = []
    for index, row in enumerate(contrasts):
        case = by_case.get(row.get('case_id'))
        if case is None:
            raise ValueError('contrast case missing')
        baseline = _arm(row,'baseline',solvers)
        candidate = _arm(row,'candidate',solvers)
        if (baseline['solver_id'],baseline['config_id']) == (candidate['solver_id'],candidate['config_id']):
            raise ValueError('contrast arms identical')
        sizes = (_rss(peak,baseline),_rss(peak,candidate))
        if None in sizes:
            missing.append(index)
            memory = None
        else:
            memory = math.ceil((2*max(sizes)+512*MIB)/MIB)*MIB
        prepared.append({'contrast_index':index,'contrast':row,'case':case,
                         'baseline':baseline,'candidate':candidate,
                         'screened_peak_rss_bytes':list(sizes),
                         'memory_reservation_bytes':memory})
    # Contrasts are assigned fixed guest CPUs in frozen order, even when
    # memory forces multiple sequential cohorts.
    cpus = list(resources.worker_cpus)
    if len(cpus) < len(prepared):
        return {'status':'HOLD_RESOURCES','reason':'INSUFFICIENT_DISTINCT_GUEST_CPUS',
                'contrasts':prepared,'cohorts':[]}
    for i,item in enumerate(prepared):
        item['guest_cpu'] = cpus[i]
    if missing:
        return {'status':'HOLD_MISSING_SCREENED_RSS','missing_contrasts':missing,
                'contrasts':prepared,'cohorts':[]}
    cohorts = []
    current = []
    used = 0
    for item in prepared:
        memory = item['memory_reservation_bytes']
        if memory > resources.memory_budget_bytes:
            return {'status':'HOLD_RESOURCES','reason':'CONTRAST_EXCEEDS_MEMORY_BUDGET',
                    'contrasts':prepared,'cohorts':[]}
        if current and used+memory > resources.memory_budget_bytes:
            cohorts.append(current)
            current, used = [], 0
        current.append(item['contrast_index'])
        used += memory
    if current:
        cohorts.append(current)
    required = ROUNDS*len(cohorts)*2*ARM_TIMEOUT
    if required+30 > PHASE_SECONDS['P3']:
        return {'status':'HOLD_PHASE_CAP','reason':'P3_WORST_CASE_TIMEOUT_RESERVATION',
                'contrasts':prepared,'cohorts':cohorts,
                'estimated_arm_timeout_seconds':required}
    seed = config.get('random_seed')
    if type(seed) is not int:
        raise ValueError('frozen integer random_seed required')
    orders = {}
    for round_index in range(ROUNDS):
        for item in prepared:
            # Independent seeds by round and contrast, derived from the one
            # frozen seed before any process result is observed.
            local_seed = int(digest([seed,round_index,item['contrast_index']])[:16],16)
            local = random.Random(local_seed)
            orders[f"{round_index}:{item['contrast_index']}"] = (
                ['baseline','candidate'] if local.randrange(2)==0 else ['candidate','baseline'])
    return {'status':'READY','contrasts':prepared,'cohorts':cohorts,'orders':orders,
            'global_multiplicity':len(prepared),'rounds':ROUNDS,
            'estimated_arm_timeout_seconds':required,
            'source_binding':source,'source_binding_sha256':digest(source),
            'executable_binding':bindings,'host':resources.host,
            'worker_cpuset':list(resources.worker_cpus),
            'memory_budget_bytes':resources.memory_budget_bytes,
            'timing_authority':'diagnostic_only','production_admission':'HOLD',
            'assumptions':['independent rounds','stationary host load','guest affinity is not physical core exclusivity']}


def _lease_deadline(ledger, lease_id, required_seconds):
    with locked_ledger(ledger) as data:
        found = [x for x in data['leases'] if x['id']==lease_id]
        if len(found)!=1:
            raise RuntimeError('P3_LEASE_IDENTITY_MISMATCH')
        row = found[0]
        if row['phase']!='P3' or row['host']!=platform.node() or 'end' in row:
            raise RuntimeError('P3_LEASE_INACTIVE')
        remaining = row['deadline']-time.time()
        if remaining < required_seconds+30:
            raise RuntimeError('P3_LEASE_REMAINING_INSUFFICIENT')
        return row['start_monotonic']+row['reserved_seconds']-30


def _receipt_valid(path, identity):
    row = strict_load(path.read_text())
    if row.get('identity') != identity:
        return False
    folder = path.parent
    for name, expected in row.get('raw_sha256',{}).items():
        if not (folder/name).is_file() or file_digest(folder/name)!=expected:
            return False
    if not {'request.json','stdout','stderr'} <= set(row.get('raw_sha256',{})):
        return False
    return row.get('receipt_sha256') == digest({k:v for k,v in row.items() if k!='receipt_sha256'})


def _run_arm(job, allocation, hard_stop):
    folder = Path(job['folder'])
    folder.mkdir(parents=True,exist_ok=False)
    request = {'case':job['case'],'solver_id':job['arm']['solver_id']}
    if job['arm']['solver_config'] is not None:
        request['solver_config'] = job['arm']['solver_config']
    atomic_json(folder/'request.json',request)
    spec = job['arm']['solver_spec']
    argv = spec['command']+['--request',str((folder/'request.json').resolve())]
    env = hpc.thread_environment({**os.environ,**spec.get('env',{})},allocation)
    remaining = min(ARM_TIMEOUT,hard_stop-time.monotonic())
    raw = None
    if remaining > 0:
        try:
            raw = hpc.run_process(argv,allocation=allocation,tracker=hpc.ProcessTracker(),
                                  env=env,cwd=spec.get('cwd'),timeout=remaining,
                                  metrics_path=folder/'gnu_time.txt')
        except (OSError,RuntimeError) as exc:
            raw = {'command':argv,'stdout':b'','stderr':str(exc).encode(),
                   'exit_code':None,'timed_out':False,'elapsed_seconds':None,
                   'affinity_cpus':list(allocation.cpus)}
    else:
        raw = {'command':argv,'stdout':b'','stderr':b'host deadline',
               'exit_code':None,'timed_out':True,'elapsed_seconds':None,
               'affinity_cpus':list(allocation.cpus)}
    (folder/'stdout').write_bytes(raw['stdout'])
    (folder/'stderr').write_bytes(raw['stderr'])
    status = 'ENVIRONMENT_FAILURE'
    result = None
    validation = []
    if raw['exit_code']==0 and not raw['timed_out']:
        try:
            result = strict_load(raw['stdout'].decode())
            validation = validate_result(result,request['case'],'timing')
            status = result['status'] if not validation else 'PROTOCOL_FAILURE'
        except (UnicodeError,ValueError,TypeError,KeyError) as exc:
            status = 'PROTOCOL_FAILURE'
            validation = [str(exc)]
    receipt = {'identity':job['identity'],'request':request,'command':raw['command'],
               'exit_code':raw['exit_code'],'timed_out':raw['timed_out'],
               'elapsed_seconds':raw['elapsed_seconds'],'affinity_cpus':raw['affinity_cpus'],
               'observed_thread_environment':{k:env.get(k) for k in hpc.THREAD_KEYS},
               'peak_rss_bytes':raw.get('peak_rss_bytes'),
               'affinity_verified':raw.get('affinity_verified'),
               'affinity_evidence':raw.get('affinity_evidence'),
               'affinity_escape':raw.get('affinity_escape'),
               'cgroup_oom_events_delta':raw.get('cgroup_oom_events_delta'),
               'assigned_cpu_steal_fraction':raw.get('assigned_cpu_steal_fraction'),
               'assigned_cpu_sample_source':raw.get('assigned_cpu_sample_source'),
               'rss_source':raw.get('rss_source'),
               'gnu_time_report':raw.get('gnu_time_report'),
               'cgroup_oom_scope':raw.get('cgroup_oom_scope'),
               'external_background_status':'UNKNOWN_NOT_ATTRIBUTABLE',
               'status':status,'protocol_errors':validation,'result':result,
               'raw_sha256':{name:file_digest(folder/name) for name in ('request.json','stdout','stderr','gnu_time.txt')
                             if (folder/name).is_file()}}
    receipt['receipt_sha256'] = digest(receipt)
    atomic_json(folder/'receipt.json',receipt)
    return receipt


def _barrier_jobs(jobs, plan, output, hard_stop):
    all_old = []
    pending = []
    for job in jobs:
        path = Path(job['folder'])/'receipt.json'
        if path.exists():
            if not _receipt_valid(path,job['identity']):
                raise RuntimeError('CONFIRMATION_RECEIPT_IDENTITY_MISMATCH')
            all_old.append(strict_load(path.read_text()))
        elif path.parent.exists():
            raise RuntimeError('INCOMPLETE_CONFIRMATION_RECEIPT')
        else:
            pending.append(job)
    if all_old and pending:
        raise RuntimeError('PARTIAL_BARRIER_HOLD_NO_DUPLICATE_RERUN')
    if not pending:
        return all_old
    with concurrent.futures.ThreadPoolExecutor(max_workers=len(pending)) as pool:
        futures = []
        for job in pending:
            item = plan['contrasts'][job['contrast_index']]
            allocation = hpc.Allocation((item['guest_cpu'],),
                                        item['memory_reservation_bytes'],'confirmation')
            futures.append(pool.submit(_run_arm,job,allocation,hard_stop))
        return [f.result() for f in futures]


def classify_pair(receipts):
    """Retain both observations even when one is unusable for inference."""
    if len(receipts) != 2:
        return {'status':'HOLD_INCOMPLETE_PAIR','log_cost':None}
    reasons = []
    for row in receipts:
        if row.get('status') != 'OK':
            reasons.append('ARM_'+str(row.get('status')))
        steal = row.get('assigned_cpu_steal_fraction')
        if type(steal) not in (int,float) or not math.isfinite(steal):
            reasons.append('UNKNOWN_ASSIGNED_CPU_STEAL')
        elif steal > .01:
            reasons.append('ASSIGNED_CPU_STEAL_GT_1_PERCENT')
        if row.get('affinity_escape') is True:
            reasons.append('AFFINITY_ESCAPE')
        elif row.get('affinity_escape') is not False:
            reasons.append('UNKNOWN_AFFINITY')
        if row.get('affinity_verified') is not True:
            reasons.append('UNKNOWN_AFFINITY_VERIFICATION')
        oom = row.get('cgroup_oom_events_delta')
        if not isinstance(oom,dict) or any(type(oom.get(key)) is not int for key in ('oom','oom_kill')):
            reasons.append('UNKNOWN_OOM_TELEMETRY')
        elif any(oom[key] > 0 for key in ('oom','oom_kill')):
            reasons.append('OOM_EVENT')
        peak = row.get('peak_rss_bytes')
        if type(peak) is not int or peak <= 0:
            reasons.append('UNKNOWN_PEAK_RSS')
        if row.get('unregistered_campaign_work') is True:
            reasons.append('UNREGISTERED_CAMPAIGN_WORK')
    by_arm = {row.get('identity',{}).get('arm'):row for row in receipts}
    if set(by_arm) != {'baseline','candidate'}:
        reasons.append('ARM_IDENTITY_MISMATCH')
    log_cost = None
    if not reasons:
        log_cost = paired_log_cost(by_arm['baseline'].get('result'),
                                   by_arm['candidate'].get('result'))
        if log_cost is None:
            reasons.append('TIMING_SCOPE_OR_VALUE_MISMATCH')
    if reasons:
        return {'status':'HOLD_CONTAMINATED' if any(x in reasons for x in (
                    'ASSIGNED_CPU_STEAL_GT_1_PERCENT','AFFINITY_ESCAPE',
                    'OOM_EVENT','UNREGISTERED_CAMPAIGN_WORK')) else 'HOLD_UNKNOWN',
                'reasons':sorted(set(reasons)),'log_cost':None,
                'external_background_status':'UNKNOWN_NOT_ATTRIBUTABLE'}
    return {'status':'PAIR_DIAGNOSTIC','log_cost':log_cost,
            'external_background_status':'UNKNOWN_NOT_ATTRIBUTABLE',
            'uncontended_claim':False}


def campaign_process_census(plan):
    """Observe matching solver commands at barriers, when own arms are absent."""
    tokens = set()
    for item in plan['contrasts']:
        for arm in (item['baseline'],item['candidate']):
            command = arm['solver_spec']['command']
            scripts = [x for x in command[1:] if x.endswith(('.py','.jl'))]
            tokens.add(str(Path(scripts[-1] if scripts else command[0]).resolve()))
    matches = []
    try:
        processes = Path('/proc').iterdir()
        for process in processes:
            if not process.name.isdigit() or int(process.name)==os.getpid():
                continue
            try:
                args = (process/'cmdline').read_bytes().split(b'\0')
                paths = {str(Path(x.decode(errors='replace')).resolve())
                         for x in args if x.startswith(b'/')}
            except OSError:
                continue
            if tokens & paths:
                matches.append(int(process.name))
        return {'status':'OBSERVED','matching_pids':sorted(matches),
                'unregistered_campaign_work':bool(matches)}
    except OSError:
        return {'status':'UNKNOWN','matching_pids':[],
                'unregistered_campaign_work':None}


def execute(plan, output, ledger, lease_id, root=None):
    if plan.get('status')!='READY':
        raise RuntimeError('P3_RESOURCE_PLAN_HOLD')
    root = Path(root or Path(__file__).resolve().parents[2]).resolve()
    if source_binding(root)!=plan['source_binding']:
        raise RuntimeError('P3_SOURCE_BINDING_MISMATCH')
    solvers = {arm['solver_id']:arm['solver_spec']
               for item in plan['contrasts'] for arm in (item['baseline'],item['candidate'])}
    if executable_binding({'solvers':solvers})!=plan['executable_binding']:
        raise RuntimeError('P3_EXECUTABLE_BINDING_MISMATCH')
    output = Path(output)
    pending_barriers = 0
    for round_index in range(ROUNDS):
        for cohort_index, cohort in enumerate(plan['cohorts']):
            for phase in (0,1):
                exists = [(output/'jobs'/f"r{round_index:02d}-c{cohort_index:02d}-p{phase}-{contrast_index:02d}"/'receipt.json').exists()
                          for contrast_index in cohort]
                if any(exists) and not all(exists):
                    raise RuntimeError('PARTIAL_BARRIER_HOLD_NO_DUPLICATE_RERUN')
                pending_barriers += not all(exists)
    hard_stop = _lease_deadline(ledger,lease_id,pending_barriers*ARM_TIMEOUT)
    run_summary = []
    barrier_context = {}
    for round_index in range(ROUNDS):
        for cohort_index,cohort in enumerate(plan['cohorts']):
            for phase in (0,1):
                jobs = []
                for contrast_index in cohort:
                    item = plan['contrasts'][contrast_index]
                    arm_name = plan['orders'][f'{round_index}:{contrast_index}'][phase]
                    arm = item[arm_name]
                    identity = {'plan_sha256':digest(plan),'round':round_index,
                                'cohort':cohort_index,'phase':phase,
                                'contrast_index':contrast_index,'arm':arm_name,
                                'case_sha256':digest(item['case']),
                                'solver_spec_sha256':digest(arm['solver_spec'])}
                    folder = output/'jobs'/f"r{round_index:02d}-c{cohort_index:02d}-p{phase}-{contrast_index:02d}"
                    jobs.append({'identity':identity,'folder':str(folder),'case':item['case'],
                                 'arm':arm,'contrast_index':contrast_index})
                # Every first arm finishes before any second arm is launched.
                before = campaign_process_census(plan)
                completed = _barrier_jobs(jobs,plan,output,hard_stop)
                after = campaign_process_census(plan)
                barrier_context[f'{round_index}:{cohort_index}:{phase}'] = {
                    'before':before,'after':after,
                    'unregistered_campaign_work':bool(before['matching_pids'] or after['matching_pids']),
                    'coverage':'BARRIER_SNAPSHOTS_ONLY'}
                run_summary.extend(completed)
            # Both phases complete before the next cohort or round begins.
    if source_binding(root)!=plan['source_binding']:
        raise RuntimeError('P3_SOURCE_DRIFT_AFTER_RUN')
    pairs = {}
    for row in run_summary:
        ident = row['identity']
        pairs.setdefault((ident['round'],ident['contrast_index']),[]).append(row)
    decisions = {}
    for round_index in range(ROUNDS):
        for contrast_index in range(plan['global_multiplicity']):
            cohort_index = next(i for i,cohort in enumerate(plan['cohorts']) if contrast_index in cohort)
            rows = pairs.get((round_index,contrast_index),[])
            if any(barrier_context[f'{round_index}:{cohort_index}:{phase}']['unregistered_campaign_work']
                   for phase in (0,1)):
                rows = [{**row,'unregistered_campaign_work':True} for row in rows]
            decisions[f'{round_index}:{contrast_index}'] = classify_pair(rows)
    atomic_json(output/'paired_diagnostics.json',{
        'status':'P3_DIAGNOSTIC_ONLY','global_multiplicity':plan['global_multiplicity'],
        'rounds':ROUNDS,'pairs':decisions,'barrier_context':barrier_context,
        'external_background_status':'UNKNOWN_NOT_ATTRIBUTABLE',
        'statistical_authority':'HOLD','production_admission':'HOLD'})
    return run_summary


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--config',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--ledger',type=Path)
    parser.add_argument('--lease-id')
    parser.add_argument('--mode',choices=('plan','run'),default='plan')
    args = parser.parse_args()
    config = strict_load(args.config.read_text())
    root = Path(__file__).resolve().parents[2]
    selected = {row[arm]:config['solvers'][row[arm]]
                for row in config['global_contrasts'] for arm in ('baseline','candidate')}
    path = args.output/'confirmation_plan.json'
    if args.mode=='plan':
        resources = hpc.detect_resources()
        source = source_binding(root)
        bindings = executable_binding({'solvers':selected})
        plan = freeze(config,resources,source,bindings)
        plan['config_sha256'] = file_digest(args.config)
        args.output.mkdir(parents=True,exist_ok=True)
        if path.exists() and strict_load(path.read_text())!=plan:
            raise RuntimeError('P3_FROZEN_PLAN_IDENTITY_MISMATCH')
        if not path.exists():
            atomic_json(path,plan)
        print(json.dumps({'status':plan['status'],'cohorts':len(plan['cohorts']),
                          'global_multiplicity':len(config['global_contrasts'])}))
        return
    if not args.ledger or not args.lease_id:
        parser.error('run requires --ledger and --lease-id')
    if not path.is_file():
        raise FileNotFoundError('P3_FROZEN_PLAN_REQUIRED')
    plan = strict_load(path.read_text())
    if plan.get('config_sha256')!=file_digest(args.config):
        raise RuntimeError('P3_FROZEN_CONFIG_IDENTITY_MISMATCH')
    resources = hpc.detect_resources()
    if resources.host!=plan.get('host') or not set(plan.get('worker_cpuset',[])) <= set(resources.worker_cpus):
        raise RuntimeError('P3_GUEST_CPU_IDENTITY_MISMATCH')
    for cohort in plan.get('cohorts',[]):
        if sum(plan['contrasts'][index]['memory_reservation_bytes'] for index in cohort) > resources.memory_budget_bytes:
            raise RuntimeError('P3_MEMORY_CAPACITY_REGRESSED')
    receipts = execute(plan,args.output,args.ledger,args.lease_id,root)
    print(json.dumps({'status':'P3_DIAGNOSTIC_ONLY','receipts':len(receipts)}))


if __name__=='__main__':
    main()
