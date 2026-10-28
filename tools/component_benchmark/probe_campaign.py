#!/usr/bin/env python3
"""Bounded, resumable P1 equation probes; no solver accuracy admission."""
from __future__ import annotations

import argparse
import concurrent.futures
import json
import os
import platform
from pathlib import Path
import subprocess
import sys
import time

if __package__ in (None, ''):
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from tools.component_benchmark.budget import locked_ledger
from tools.component_benchmark import hpc as hpc_resources
from tools.component_benchmark.parity import admit_probe, frozen_points
from tools.component_benchmark.protocol import (digest, file_digest,
                                                physical_input_digest, strict_load,
                                                validate_case)
from tools.component_benchmark.runner import (atomic_json, executable_binding,
                                              source_binding)

DEFAULT_ADAPTERS = ('native-rodas5p-fast', 'diffsol-bdf', 'scipy-radau',
                    'julia-rodas5p', 'hairer-radau5')
THREAD_KEYS = ('OMP_NUM_THREADS', 'OPENBLAS_NUM_THREADS', 'MKL_NUM_THREADS',
               'RAYON_NUM_THREADS', 'JULIA_NUM_THREADS')


def unique_cases(config):
    result = {}
    for case in config['cases']:
        errors = validate_case(case)
        if errors:
            raise ValueError(f"{case.get('case_id')}: {errors}")
        result.setdefault(physical_input_digest(case), case)
    return result


def selected_solvers(config, adapters):
    ids = tuple(adapters or DEFAULT_ADAPTERS)
    if not ids or len(ids) != len(set(ids)):
        raise ValueError('adapter selection must contain distinct solver IDs')
    for sid in ids:
        if sid not in config['solvers']:
            raise ValueError('adapter absent from frozen config: ' + sid)
    return {sid: config['solvers'][sid] for sid in ids}


def _lease_deadline(ledger_path, lease_id, required_seconds=0):
    with locked_ledger(ledger_path) as ledger:
        matches = [x for x in ledger['leases'] if x['id'] == lease_id]
        if len(matches) != 1:
            raise RuntimeError('P1_LEASE_IDENTITY_MISMATCH')
        record = matches[0]
        if (record['phase'] != 'P1' or record['host'] != platform.node()
                or 'end' in record or time.time() >= record['deadline']):
            raise RuntimeError('P1_LEASE_INACTIVE')
        cleanup = 30
        if record['deadline'] - time.time() < required_seconds + cleanup:
            raise RuntimeError('P1_LEASE_REMAINING_INSUFFICIENT')
        return record['start_monotonic'] + record['reserved_seconds'] - cleanup


def _bound_files(spec):
    bound = executable_binding({'solvers': {'selected': spec}})['selected']
    return bound


def _receipt_valid(folder, receipt, job):
    if receipt.get('job') != job:
        return False
    for key in ('request.json', 'adapter.stdout', 'adapter.stderr',
                'oracle.stdout', 'oracle.stderr', 'adapter.time', 'oracle.time'):
        path = folder / key
        expected = receipt.get('files_sha256', {}).get(key)
        if expected is None:
            if path.exists():
                return False
        elif not path.is_file() or file_digest(path) != expected:
            return False
    if receipt.get('adapter_binding') != _bound_files(job['solver_spec']):
        return False
    if receipt.get('receipt_sha256') != digest({k:v for k,v in receipt.items()
                                                if k != 'receipt_sha256'}):
        return False
    return True


def _write_receipt(folder, receipt):
    receipt['files_sha256'] = {name: file_digest(folder / name)
                               for name in ('request.json', 'adapter.stdout',
                                            'adapter.stderr', 'oracle.stdout',
                                            'oracle.stderr', 'adapter.time', 'oracle.time') if (folder / name).is_file()}
    receipt['receipt_sha256'] = digest(receipt)
    atomic_json(folder / 'receipt.json', receipt)


def _one(job, output, hard_stop, oracle_stop, allocation=None, tracker=None):
    folder = output / 'jobs' / job['job_id']
    folder.mkdir(parents=True, exist_ok=False)
    request = job['request']
    atomic_json(folder / 'request.json', request)
    argv = job['solver_spec']['command'] + ['--probe', '--request', str((folder / 'request.json').resolve())]
    env = dict(os.environ)
    env.update(job['solver_spec'].get('env', {}))
    if allocation is not None:
        env = hpc_resources.thread_environment(env, allocation)
    receipt = {'job': job, 'adapter_binding': job['adapter_binding'],
               'source_binding_sha256': job['source_binding_sha256'],
               'argv': argv, 'environment': {key:env.get(key) for key in THREAD_KEYS},
               'configured_environment': job['solver_spec'].get('env', {}),
               'started_unix': time.time(), 'adapter_exit_code': None,
               'oracle_exit_code': None, 'adapter_status': None,
               'parity': {'status':'HOLD','reason':'NOT_RUN'}}
    remaining = min(60, hard_stop - time.monotonic())
    if remaining <= 0:
        receipt['adapter_status'] = 'BUDGET_TIMEOUT_HOLD'
        _write_receipt(folder, receipt)
        return receipt
    try:
        if allocation is None:
            proc = subprocess.run(argv, cwd=job['solver_spec'].get('cwd'), env=env,
                                  capture_output=True, timeout=remaining, check=False)
            stdout, stderr, code = proc.stdout, proc.stderr, proc.returncode
        else:
            proc = hpc_resources.run_process(argv, allocation=allocation, tracker=tracker,
                env=env, cwd=job['solver_spec'].get('cwd'), timeout=remaining,
                metrics_path=folder/'adapter.time')
            stdout, stderr, code = proc['stdout'], proc['stderr'], proc['exit_code']
            receipt['adapter_hpc'] = {key:proc[key] for key in ('command','affinity_cpus','peak_rss_bytes',
                'rss_source','rss_samples','affinity_escape','affinity_verified','cgroup_oom_events_delta',
                'assigned_cpu_steal_fraction','assigned_cpu_busy_fraction')}
            if proc['timed_out']:
                receipt['adapter_status'] = 'ENVIRONMENT_FAILURE:TIMEOUT'
                code = None
    except subprocess.TimeoutExpired as exc:
        stdout, stderr, code = exc.stdout or b'', exc.stderr or b'', None
        receipt['adapter_status'] = 'ENVIRONMENT_FAILURE:TIMEOUT'
    except (OSError, RuntimeError, ValueError) as exc:
        stdout, stderr, code = b'', str(exc).encode(), None
        receipt['adapter_status'] = 'ENVIRONMENT_FAILURE:SPAWN'
    (folder / 'adapter.stdout').write_bytes(stdout)
    (folder / 'adapter.stderr').write_bytes(stderr)
    receipt['adapter_exit_code'] = code
    if code == 0:
        try:
            adapter = strict_load(stdout.decode())
            if not isinstance(adapter, dict):
                raise ValueError('probe output not an object')
            receipt['adapter_status'] = adapter.get('status', 'PROTOCOL_FAILURE')
        except (UnicodeError, ValueError, TypeError) as exc:
            receipt['adapter_status'] = 'PROTOCOL_FAILURE'
            receipt['parse_error'] = str(exc)
    elif receipt['adapter_status'] is None:
        receipt['adapter_status'] = 'ENVIRONMENT_FAILURE:EXIT'
    if receipt['adapter_status'] not in ('OK', 'UNSUPPORTED_CAPABILITY'):
        receipt['parity'] = {'status':'HOLD','reason':'ADAPTER_'+receipt['adapter_status']}
    else:
        remaining = min(hard_stop, oracle_stop) - time.monotonic()
        if remaining <= 0:
            receipt['parity'] = {'status':'HOLD','reason':'ORACLE_BUDGET_TIMEOUT'}
        else:
            oracle_command = [sys.executable, str(Path(__file__).resolve()),
                              '--oracle-request', str((folder/'request.json').resolve()),
                              '--oracle-output', str((folder/'adapter.stdout').resolve()),
                              '--case-sha256', job['case_sha256'],
                              '--source-sha256', job['source_binding_sha256'],
                              '--observed-source-sha256', job['source_binding_sha256']]
            try:
                if allocation is None:
                    oracle = subprocess.run(oracle_command, capture_output=True,
                                            timeout=remaining, check=False)
                    oracle_stdout, oracle_stderr = oracle.stdout, oracle.stderr
                    oracle_code = oracle.returncode
                else:
                    oracle = hpc_resources.run_process(oracle_command,allocation=allocation,
                        tracker=tracker,env=env,timeout=remaining,metrics_path=folder/'oracle.time')
                    oracle_stdout, oracle_stderr = oracle['stdout'], oracle['stderr']
                    oracle_code = None if oracle['timed_out'] else oracle['exit_code']
                    receipt['oracle_hpc'] = {key:oracle[key] for key in ('command','affinity_cpus','peak_rss_bytes',
                        'rss_source','rss_samples','affinity_escape','affinity_verified','cgroup_oom_events_delta',
                        'assigned_cpu_steal_fraction','assigned_cpu_busy_fraction')}
                receipt['oracle_exit_code'] = oracle_code
                if oracle_code == 0:
                    decision = strict_load(oracle_stdout.decode())
                    if not isinstance(decision, dict) or decision.get('status') not in ('PASS_DIAGNOSTIC','HOLD'):
                        raise ValueError('invalid oracle decision')
                    receipt['parity'] = decision
                elif allocation is not None and oracle['timed_out']:
                    receipt['parity'] = {'status':'HOLD','reason':'ORACLE_BUDGET_TIMEOUT'}
                else:
                    receipt['parity'] = {'status':'HOLD','reason':'ORACLE_PROCESS_FAILURE'}
            except subprocess.TimeoutExpired as exc:
                oracle_stdout, oracle_stderr = exc.stdout or b'', exc.stderr or b''
                receipt['parity'] = {'status':'HOLD','reason':'ORACLE_BUDGET_TIMEOUT'}
            except (OSError, RuntimeError, ValueError) as exc:
                oracle_stdout, oracle_stderr = b'', str(exc).encode()
                receipt['parity'] = {'status':'HOLD','reason':'ORACLE_SPAWN_FAILURE'}
            except (UnicodeError, ValueError, TypeError) as exc:
                oracle_stdout = oracle_stdout if 'oracle_stdout' in locals() else b''
                oracle_stderr = str(exc).encode()
                receipt['parity'] = {'status':'HOLD','reason':'ORACLE_PROTOCOL_FAILURE'}
            (folder/'oracle.stdout').write_bytes(oracle_stdout)
            (folder/'oracle.stderr').write_bytes(oracle_stderr)
    receipt['ended_unix'] = time.time()
    _write_receipt(folder, receipt)
    return receipt


def run(config_path, output, ledger, lease_id, adapters=None, concurrency=4, root=None, hpc=False):
    if not lease_id or not ledger:
        raise ValueError('shared P1 ledger and active lease ID required')
    if not hpc and not 1 <= concurrency <= 8:
        raise ValueError('probe concurrency must be 1..8')
    if hpc and not 1 <= concurrency <= 56:
        raise ValueError('HPC probe concurrency must be 1..56')
    root = Path(root or Path(__file__).resolve().parents[2]).resolve()
    output = Path(output).resolve()
    config = strict_load(Path(config_path).read_text())
    if not isinstance(config, dict) or not isinstance(config.get('cases'), list) or not isinstance(config.get('solvers'), dict):
        raise ValueError('screening config requires cases and solvers')
    physical = unique_cases(config)
    solvers = selected_solvers(config, adapters)
    source = source_binding(root)
    source_hash = digest(source)
    hard_stop = _lease_deadline(ledger, lease_id)
    if hard_stop <= time.monotonic():
        raise RuntimeError('P1_LEASE_CLEANUP_RESERVE_REACHED')
    oracle_stop = min(hard_stop, time.monotonic() + 1200)
    bindings = {sid:_bound_files(spec) for sid,spec in solvers.items()}
    jobs = []
    for physical_sha, case in physical.items():
        for sid, spec in solvers.items():
            request = {'case':case,'solver_id':sid,'probes':frozen_points(case)}
            identity = {'request':request,'solver_spec':spec,'source_binding_sha256':source_hash,
                        'adapter_binding':bindings[sid], 'physical_input_sha256':physical_sha}
            jobs.append({'job_id':digest(identity),'request':request,'solver_spec':spec,
                         'solver_id':sid,'physical_input_sha256':physical_sha,
                         'case_sha256':digest(case),'source_binding_sha256':source_hash,
                         'adapter_binding':bindings[sid]})
    output.mkdir(parents=True, exist_ok=True)
    plan = {'phase':'P1','kind':'equation_probe','config_sha256':file_digest(config_path),
            'source_binding_sha256':source_hash,'lease_id':lease_id,
            'selected_adapters':list(solvers),'jobs':jobs,
            'scientific_admission':'HOLD'}
    plan_path = output/'plan.json'
    if plan_path.exists() and strict_load(plan_path.read_text()) != plan:
        raise RuntimeError('PROBE_PLAN_IDENTITY_MISMATCH')
    if not plan_path.exists():
        atomic_json(plan_path, plan)
    pending, receipts = [], []
    for job in jobs:
        folder = output/'jobs'/job['job_id']
        path = folder/'receipt.json'
        if path.exists():
            receipt = strict_load(path.read_text())
            if not _receipt_valid(folder, receipt, job):
                raise RuntimeError('PROBE_RECEIPT_IDENTITY_MISMATCH')
            receipts.append(receipt)
        elif folder.exists():
            raise RuntimeError('INCOMPLETE_PROBE_JOB_REQUIRES_RECOVERY')
        else:
            pending.append(job)
    if hpc and pending:
        resources = hpc_resources.detect_resources()
        cap = None if concurrency == 4 else concurrency
        pool = hpc_resources.ResourcePool(resources,max_workers=cap,telemetry_path=output/'hpc_telemetry.jsonl')
        demand = lambda job: hpc_resources.demand_for(job,job['solver_spec'],'probe')
        waves = hpc_resources.estimate_waves(pending,resources,demand=demand,max_workers=pool.max_workers)
        hard_stop = _lease_deadline(ledger,lease_id,required_seconds=waves*1260)
        oracle_stop = min(hard_stop,time.monotonic()+1200)
        receipts.extend(pool.run(pending,lambda job,allocation,tracker:
            _one(job,output,hard_stop,oracle_stop,allocation,tracker),demand=demand))
    elif pending:
        with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as pool:
            futures = [pool.submit(_one, job, output, hard_stop, oracle_stop)
                       for job in pending]
            for future in concurrent.futures.as_completed(futures):
                receipts.append(future.result())
    if source_binding(root) != source or any(_bound_files(spec) != bindings[sid]
                                             for sid,spec in solvers.items()):
        raise RuntimeError('PROBE_SOURCE_OR_EXECUTABLE_DRIFT')
    mapping = {}
    for receipt in receipts:
        job = receipt['job']
        mapping.setdefault(job['physical_input_sha256'], {})[job['solver_id']] = {
            'adapter_status':receipt['adapter_status'],
            'parity':receipt['parity'], 'job_id':job['job_id']}
    atomic_json(output/'mapping.json', {'status':'PROBE_DIAGNOSTIC_ONLY',
                'source_binding_sha256':source_hash,'physical_inputs':mapping})
    return mapping


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--config', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--ledger', type=Path)
    parser.add_argument('--lease-id')
    parser.add_argument('--concurrency', type=int, default=4)
    parser.add_argument('--hpc', action='store_true')
    parser.add_argument('--adapters', nargs='+')
    parser.add_argument('--root', type=Path)
    parser.add_argument('--oracle-request', type=Path, help=argparse.SUPPRESS)
    parser.add_argument('--oracle-output', type=Path, help=argparse.SUPPRESS)
    parser.add_argument('--case-sha256', help=argparse.SUPPRESS)
    parser.add_argument('--source-sha256', help=argparse.SUPPRESS)
    parser.add_argument('--observed-source-sha256', help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.oracle_request:
        request = strict_load(args.oracle_request.read_text())
        observed = strict_load(args.oracle_output.read_text())
        print(json.dumps(admit_probe(args.case_sha256,args.source_sha256,
                                     args.observed_source_sha256,request,observed),
                         allow_nan=False,separators=(',',':')))
    else:
        if not all((args.config,args.output,args.ledger,args.lease_id)):
            parser.error('--config --output --ledger --lease-id required')
        mapping = run(args.config,args.output,args.ledger,args.lease_id,
                      args.adapters,args.concurrency,args.root,args.hpc)
        print(json.dumps({'status':'PROBE_DIAGNOSTIC_ONLY',
                          'physical_inputs':len(mapping),
                          'jobs':sum(len(x) for x in mapping.values())}))


if __name__ == '__main__':
    main()
