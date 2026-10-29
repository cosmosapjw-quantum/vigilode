"""One-shot NCP P3 launcher. Run only after an independent config freeze.

The review receipt schema is deliberately small and exact:
{"status":"APPROVED_P3_CONFIG", "config_sha256":"...",
 "source_binding_sha256":"...", "exec_sha":"..."}.
An interrupted run is retained under the same output ID and is not resumed.
"""
from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import platform
import signal
import subprocess
import sys
import threading
import time

ROOT = Path('/root/VIGILODE_COMPONENT_20261018T134235Z/repo')
RUN_NAME = 'ncp_confirmation_01'
P2_NAME = 'ncp_screening_01'
P3_SECONDS = 24*3600-60
POLL_SECONDS = 15
CLEANUP_SECONDS = 30
OWNED_CLEANUP_SECONDS = 2


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_json(path, data):
    path = Path(path)
    tmp = path.with_name(path.name+'.tmp-'+str(os.getpid()))
    tmp.write_text(json.dumps(data,indent=2,sort_keys=True,allow_nan=False)+'\n')
    os.replace(tmp,path)


def read_ledger(path):
    with Path(path).open('r') as handle:
        fcntl.flock(handle,fcntl.LOCK_SH)
        try:
            return json.load(handle)
        finally:
            fcntl.flock(handle,fcntl.LOCK_UN)


def active_p2(ledger, context, host):
    from component_benchmark.budget import PHASE_SECONDS, usage
    data=read_ledger(ledger)
    active=[row for row in data['leases'] if 'end' not in row]
    if len(active)!=1 or active[0]['phase']!='P2' or active[0]['host']!=host or active[0]['id']!=context.get('lease_id'):
        raise RuntimeError('P2_ACTIVE_LEASE_IDENTITY_MISMATCH')
    row=active[0]
    if time.time()>=row['deadline']-CLEANUP_SECONDS:
        raise RuntimeError('P2_LEASE_CLEANUP_RESERVE_REACHED')
    if usage(data)['P2']>PHASE_SECONDS['P2'] or any(
            old['phase']=='P2' and old.get('reservation_overrun') for old in data['leases']):
        raise RuntimeError('P2_PHASE_CAP_OR_RESERVATION_OVERRUN')
    return row


def wait_p2(marker, seconds, lease_deadline_unix):
    deadline = time.monotonic()+seconds
    while not marker.is_file():
        remaining_lease=lease_deadline_unix-time.time()-CLEANUP_SECONDS
        if remaining_lease<=0:
            raise RuntimeError('P2_LEASE_CLEANUP_RESERVE_REACHED')
        if time.monotonic() >= deadline:
            raise RuntimeError('P2_QUEUES_FINISHED_MISSING')
        time.sleep(min(POLL_SECONDS,max(0,deadline-time.monotonic()),remaining_lease))
    if time.time()>=lease_deadline_unix-CLEANUP_SECONDS:
        raise RuntimeError('P2_LEASE_CLEANUP_RESERVE_REACHED')
    data=json.loads(marker.read_text())
    if data.get('lease_remains_active') is not True:
        raise RuntimeError('P2_FINISH_MARKER_INVALID')
    return data


def process_identity(pid):
    """Read Linux PID identity; starttime prevents killing a reused PID/group."""
    try:
        stat=Path(f'/proc/{pid}/stat').read_text()
        fields=stat[stat.rfind(')')+2:].split()
        return {'pid':pid,'state':fields[0],'ppid':int(fields[1]),
                'pgid':int(fields[2]),'sid':int(fields[3]),
                'starttime':int(fields[19])}
    except (OSError,ValueError,IndexError):
        return None


def descendant_snapshot(controller):
    """Find only processes whose current PPid chain reaches this controller."""
    rows={}
    try:
        names=os.listdir('/proc')
    except OSError:
        return {}
    for name in names:
        if name.isdecimal():
            row=process_identity(int(name))
            if row is not None:
                rows[row['pid']]=row
    if rows.get(controller['pid'],{}).get('starttime')!=controller['starttime']:
        return {}
    owned={controller['pid']:controller}
    changed=True
    while changed:
        changed=False
        for pid,row in rows.items():
            if pid not in owned and row['ppid'] in owned:
                owned[pid]=row
                changed=True
    owned.pop(controller['pid'],None)
    return owned


def freeze_verified_controller(controller):
    current=process_identity(controller['pid'])
    if current is None or current['starttime']!=controller['starttime'] or current['pgid']!=controller['pgid']:
        return False
    try:
        os.killpg(controller['pgid'],signal.SIGSTOP)
        return True
    except ProcessLookupError:
        return False


def verified_owned_cleanup(controller, observed, proc, controller_frozen=False):
    """Freeze parent, verify live owned members, kill only their process groups."""
    report={'controller_identity':controller,'observed_descendants':list(observed.values()),
            'verified_groups':[],'signals':[],'unverified_groups':[],
            'cleanup_limit_seconds':OWNED_CLEANUP_SECONDS}
    began=time.monotonic()
    current=process_identity(controller['pid'])
    controller_live=(current is not None and current['starttime']==controller['starttime']
                     and current['pgid']==controller['pgid'])
    if controller_frozen:
        report['signals'].append({'pgid':controller['pgid'],'signal':'SIGSTOP','target':'controller'})
    elif controller_live:
        controller_live=freeze_verified_controller(controller)
        if controller_live:
            report['signals'].append({'pgid':controller['pgid'],'signal':'SIGSTOP','target':'controller'})
    if controller_live:
        observed.update(descendant_snapshot(controller))
    groups={row['pgid'] for row in observed.values() if row['pgid']!=controller['pgid']}
    for pgid in sorted(groups):
        if time.monotonic()-began>=OWNED_CLEANUP_SECONDS:
            report['cleanup_time_exceeded']=True
            report['unverified_groups'].extend(sorted(groups-{entry['pgid'] for entry in report['verified_groups']}))
            break
        live=[]
        for row in observed.values():
            if row['pgid']!=pgid:
                continue
            now=process_identity(row['pid'])
            if now is not None and now['starttime']==row['starttime'] and now['pgid']==pgid and now['sid']==row['sid'] and now['state']!='Z':
                live.append(now)
        if not live:
            report['unverified_groups'].append(pgid)
            continue
        report['verified_groups'].append({'pgid':pgid,'members':live})
        try:
            os.killpg(pgid,signal.SIGKILL)
            report['signals'].append({'pgid':pgid,'signal':'SIGKILL','target':'verified_descendant_group'})
        except ProcessLookupError:
            report['unverified_groups'].append(pgid)
    current=process_identity(controller['pid'])
    if current is not None and current['starttime']==controller['starttime'] and current['pgid']==controller['pgid']:
        try:
            os.killpg(controller['pgid'],signal.SIGKILL)
            report['signals'].append({'pgid':controller['pgid'],'signal':'SIGKILL','target':'controller'})
        except ProcessLookupError:
            pass
    try:
        proc.wait(timeout=max(0.01,OWNED_CLEANUP_SECONDS-(time.monotonic()-began)))
    except subprocess.TimeoutExpired:
        report['controller_reap']='TIMEOUT'
    else:
        report['controller_reap']='REAPED'
    report['elapsed_seconds']=time.monotonic()-began
    return report


def run_logged_command(logs, label, argv, root, env, hard_stop, source_sha, config_sha, driver_sha):
    remaining=hard_stop-time.monotonic()
    if remaining<=0:
        raise RuntimeError('P3_LEASE_CLEANUP_RESERVE_REACHED')
    start=time.time()
    stdout=logs/(label+'.stdout');stderr=logs/(label+'.stderr')
    timed_out=False
    cleanup=None
    with stdout.open('wb') as out,stderr.open('wb') as err:
        proc=subprocess.Popen(argv,cwd=root,env=env,stdout=out,stderr=err,
                              start_new_session=True)
        controller=process_identity(proc.pid)
        if controller is None and proc.poll() is None:
            raise RuntimeError('P3_CONTROLLER_PROCESS_IDENTITY_UNAVAILABLE')
        if controller is not None and controller['pgid']!=proc.pid:
            raise RuntimeError('P3_CONTROLLER_PROCESS_GROUP_MISMATCH')
        observed={}
        observed_lock=threading.Lock()
        stop_monitor=threading.Event()
        def monitor_descendants():
            while not stop_monitor.wait(0.05):
                found=descendant_snapshot(controller)
                with observed_lock:
                    observed.update(found)
        monitor=threading.Thread(target=monitor_descendants,daemon=True) if controller is not None else None
        if monitor is not None:
            monitor.start()
        try:
            proc.wait(timeout=remaining)
        except subprocess.TimeoutExpired:
            timed_out=True
            frozen=freeze_verified_controller(controller)
            stop_monitor.set()
            if monitor is not None:
                monitor.join(timeout=0.2)
            with observed_lock:
                owned=dict(observed)
            cleanup=verified_owned_cleanup(controller,owned,proc,controller_frozen=frozen)
        finally:
            stop_monitor.set()
            if monitor is not None:
                monitor.join(timeout=0.2)
    if cleanup is not None:
        write_json(logs/(label+'.cleanup.json'),cleanup)
    write_json(logs/(label+'.json'),{'argv':argv,'cwd':str(root),'started_unix':start,
        'ended_unix':time.time(),'exit_code':proc.returncode,
        'timeout_seconds':remaining,'timed_out':timed_out,
        'owned_cleanup_sha256':sha(logs/(label+'.cleanup.json')) if cleanup is not None else None,
        'raw_status':'ENVIRONMENT_FAILURE:HOST_LEASE_TIMEOUT' if timed_out else
            ('OK' if proc.returncode==0 else 'ENVIRONMENT_FAILURE:EXIT'),
        'stdout_sha256':sha(stdout),'stderr_sha256':sha(stderr),
        'source_binding_sha256':source_sha,'config_sha256':config_sha,
        'driver_sha256':driver_sha})
    print(label,proc.returncode,flush=True)
    if timed_out:
        raise RuntimeError(label+'_HOST_LEASE_TIMEOUT')
    if proc.returncode:
        raise RuntimeError(label+'_EXIT_'+str(proc.returncode))


def campaign_process_census(root):
    """Snapshot own work only; unrelated host work remains unattributed."""
    root=Path(root).resolve()
    active=[]
    unknown=[]
    try:
        entries=list(Path('/proc').iterdir())
    except OSError as exc:
        return {'status':'UNKNOWN','reason':str(exc),'active_campaign_jobs':None,
                'coverage':'PROCESS_SNAPSHOT_ONLY'}
    for entry in entries:
        if not entry.name.isdecimal() or int(entry.name)==os.getpid():
            continue
        try:
            raw=(entry/'cmdline').read_bytes()
            if not raw:
                continue
            argv=[item.decode(errors='replace') for item in raw.split(b'\0') if item]
            cwd=(entry/'cwd').resolve()
        except FileNotFoundError:
            continue  # A disappearing process cannot establish an active job.
        except (OSError,RuntimeError) as exc:
            return {'status':'UNKNOWN','reason':str(exc),'active_campaign_jobs':None,
                    'coverage':'PROCESS_SNAPSHOT_ONLY'}
        joined=' '.join(argv)
        own_cwd=cwd==root or root in cwd.parents
        own_path=str(root) in joined or str(root.parent) in joined
        if not (own_cwd or own_path):
            continue
        request_from_prior=any(token in joined for token in
            ('/ncp_screening_01/','/ncp_execution_01/references/',
             '/ncp_execution_01/probes/'))
        own_driver=any(token in joined for token in
            ('run_ncp_p1.py','run_ncp_p2.py','prepare_p2.py'))
        numerical_or_build=any(token in joined for token in
            ('component_benchmark.py','probe_campaign.py','component_contracts.py',
             'campaign.py','configured_adapter.py','scipy_adapter.py',
             'julia_adapter.jl','hairer_adapter.py','cargo build','cargo test',
             'rustc ','gfortran ','gcc ','cc1 '))
        if request_from_prior or own_driver or numerical_or_build:
            active.append({'pid':int(entry.name),'cwd':str(cwd),'argv':argv})
        else:
            unknown.append({'pid':int(entry.name),'cwd':str(cwd),'argv':argv})
    return {'status':'OBSERVED','unix':time.time(),
            'active_campaign_jobs':sorted(active,key=lambda x:x['pid']),
            'other_own_processes':sorted(unknown,key=lambda x:x['pid']),
            'coverage':'PROCESS_SNAPSHOT_ONLY',
            'external_background_status':'UNKNOWN_NOT_ATTRIBUTABLE'}


def inspect(output):
    output=Path(output)
    result={'output':str(output),'confirmation_plan':(output/'confirmation_plan.json').is_file(),
            'paired_diagnostics':(output/'paired_diagnostics.json').is_file(),
            'finish_marker':(output/'P3_QUEUES_FINISHED.json').is_file(),
            'driver_failure':(output/'driver_failure.json').is_file(),
            'holds':[p.name for p in output.glob('*hold.json')],
            'receipts':{},'pairs':{}}
    for file in (output/'jobs').glob('*/receipt.json'):
        try:
            row=json.loads(file.read_text())
            status=row.get('status','UNKNOWN')
        except (OSError,ValueError):
            status='INVALID_RECEIPT'
        result['receipts'][status]=result['receipts'].get(status,0)+1
    diagnostics=output/'paired_diagnostics.json'
    if diagnostics.is_file():
        data=json.loads(diagnostics.read_text())
        for row in data.get('pairs',{}).values():
            status=row.get('status','UNKNOWN')
            result['pairs'][status]=result['pairs'].get(status,0)+1
        result['barrier_coverage']='BARRIER_SNAPSHOTS_ONLY'
        result['statistical_authority']=data.get('statistical_authority')
    return result


def run(args):
    root=args.root.resolve()
    args.config=args.config.resolve()
    args.review=args.review.resolve()
    campaign=root/'research/component_benchmark_20261018'
    p2=campaign/P2_NAME
    output=campaign/RUN_NAME
    ledger=campaign/'NCP_LEASE_LEDGER.json'
    if output.exists():
        raise RuntimeError('P3_OUTPUT_ALREADY_EXISTS_NO_NEW_RUN_ID')
    sys.path.insert(0,str(root/'tools'))
    from component_benchmark.budget import close, reserve, PHASE_SECONDS, usage
    from component_benchmark.runner import source_binding
    from component_benchmark.protocol import digest
    p2_context=json.loads((p2/'context.json').read_text())
    if p2_context.get('ledger')!=str(ledger):
        raise RuntimeError('P2_CANONICAL_LEDGER_MISMATCH')
    active=active_p2(ledger,p2_context,platform.node())
    wait_p2(p2/'P2_QUEUES_FINISHED.json',args.wait_seconds,active['deadline'])
    active=active_p2(ledger,p2_context,platform.node())
    if not args.config.is_file() or not args.review.is_file():
        raise RuntimeError('P3_FROZEN_CONFIG_OR_REVIEW_MISSING')
    config_sha=sha(args.config)
    review=json.loads(args.review.read_text())
    source_sha=digest(source_binding(root))
    exec_sha=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
    if review.get('status')!='APPROVED_P3_CONFIG' or any(review.get(key)!=value for key,value in
            (('config_sha256',config_sha),('source_binding_sha256',source_sha),('exec_sha',exec_sha))):
        raise RuntimeError('P3_ASTRA_FROZEN_CONFIG_IDENTITY_MISMATCH')
    settle_deadline=time.monotonic()+min(args.wait_seconds,120,
        max(0,active['deadline']-time.time()-CLEANUP_SECONDS))
    while True:
        active=active_p2(ledger,p2_context,platform.node())
        before=campaign_process_census(root)
        if before.get('status')!='OBSERVED':
            raise RuntimeError('P3_PRIOR_CAMPAIGN_CENSUS_UNKNOWN')
        if not before['active_campaign_jobs']:
            break
        if time.monotonic()>=settle_deadline:
            raise RuntimeError('P3_PRIOR_CAMPAIGN_JOBS_ACTIVE')
        time.sleep(min(POLL_SECONDS,settle_deadline-time.monotonic(),
                       active['deadline']-time.time()-CLEANUP_SECONDS))
    active=active_p2(ledger,p2_context,platform.node())
    # The folder is allocated once. Any later failure keeps raw evidence and
    # cannot be resumed by invoking this driver again under a new run ID.
    output.mkdir(exist_ok=False)
    logs=output/'commands';logs.mkdir()
    write_json(output/'preflight_process_census.json',before)
    python=str(root.parent/'env/python/bin/python')
    env=dict(os.environ)
    env['PYTHONPATH']=str(root/'tools')
    env['PATH']=str(root.parent/'env/python/bin')+':'+env.get('PATH','')
    for key in ('OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','MKL_NUM_THREADS',
                'BLIS_NUM_THREADS','RAYON_NUM_THREADS','JULIA_NUM_THREADS'):
        env[key]='1'
    try:
        closed=close(ledger,active['id'])
        charged=read_ledger(ledger)
        if closed.get('reservation_overrun') or usage(charged)['P2']>PHASE_SECONDS['P2']:
            raise RuntimeError('P2_PHASE_CAP_OR_RESERVATION_OVERRUN')
        lease=reserve(ledger,'P3',P3_SECONDS,platform.node(),persistent=True)
    except Exception as exc:
        write_json(output/'driver_failure.json',{'time':time.time(),'type':type(exc).__name__,
            'reason':str(exc),'stage':'P2_TO_P3_LEASE_TRANSITION',
            'lease_state':'CHECK_CANONICAL_LEDGER'})
        write_json(output/'postflight_process_census.json',campaign_process_census(root))
        raise
    driver_sha=sha(__file__)
    p3_ledger=read_ledger(ledger)
    p3_row=next(row for row in p3_ledger['leases'] if row['id']==lease)
    hard_stop=p3_row['start_monotonic']+p3_row['reserved_seconds']-CLEANUP_SECONDS
    write_json(output/'context.json',{'ledger':str(ledger),'lease_id':lease,'prior_p2_lease_id':active['id'],
        'exec_sha':exec_sha,'source_binding_sha256':source_sha,'config_sha256':config_sha,
        'review_sha256':sha(args.review),'driver_sha256':driver_sha,'host':platform.node(),
        'barrier_coverage':'BARRIER_SNAPSHOTS_ONLY',
        'external_background_status':'UNKNOWN_NOT_ATTRIBUTABLE',
        'lease_remains_active':True})
    def command(label,argv):
        if sha(__file__)!=driver_sha or sha(args.config)!=config_sha or digest(source_binding(root))!=source_sha:
            raise RuntimeError('P3_DRIVER_CONFIG_OR_SOURCE_DRIFT')
        run_logged_command(logs,label,argv,root,env,hard_stop,source_sha,config_sha,driver_sha)
    try:
        command('confirmation_plan',[python,str(root/'tools/component_benchmark/confirmation.py'),
            '--config',str(args.config),'--output',str(output),'--mode','plan'])
        plan=json.loads((output/'confirmation_plan.json').read_text())
        if plan.get('status')!='READY' or plan.get('rounds')!=17 or not 1<=plan.get('global_multiplicity',0)<=16:
            raise RuntimeError('P3_FROZEN_RESOURCE_PLAN_HOLD')
        if plan.get('config_sha256')!=config_sha or plan.get('estimated_arm_timeout_seconds',10**9)+30>24*3600:
            raise RuntimeError('P3_PLAN_IDENTITY_OR_PHASE_CAP_MISMATCH')
        command('confirmation',[python,str(root/'tools/component_benchmark/confirmation.py'),
            '--config',str(args.config),'--output',str(output),'--ledger',str(ledger),
            '--lease-id',lease,'--mode','run'])
        diagnostics=output/'paired_diagnostics.json'
        if not diagnostics.is_file():
            raise RuntimeError('P3_PAIRED_DIAGNOSTICS_MISSING')
    except Exception as exc:
        write_json(output/'driver_failure.json',{'time':time.time(),'type':type(exc).__name__,
            'reason':str(exc),'completed_receipts':sum(1 for _ in (output/'jobs').glob('*/receipt.json')),
            'lease_remains_active':True})
        raise
    finally:
        write_json(output/'postflight_process_census.json',campaign_process_census(root))
    write_json(output/'P3_QUEUES_FINISHED.json',{'time':time.time(),'lease_remains_active':True,
        'paired_diagnostics_sha256':sha(output/'paired_diagnostics.json'),
        'postflight_census_sha256':sha(output/'postflight_process_census.json'),
        'barrier_coverage':'BARRIER_SNAPSHOTS_ONLY',
        'statistical_authority':'HOLD','production_admission':'HOLD'})


def main():
    parser=argparse.ArgumentParser()
    sub=parser.add_subparsers(dest='mode',required=True)
    launch=sub.add_parser('run')
    launch.add_argument('--root',type=Path,default=ROOT)
    launch.add_argument('--config',type=Path,required=True)
    launch.add_argument('--review',type=Path,required=True)
    launch.add_argument('--wait-seconds',type=int,default=7200)
    view=sub.add_parser('inspect')
    view.add_argument('--output',type=Path,default=ROOT/'research/component_benchmark_20261018'/RUN_NAME)
    args=parser.parse_args()
    if args.mode=='inspect':
        print(json.dumps(inspect(args.output),sort_keys=True))
    else:
        if args.wait_seconds<0:
            parser.error('--wait-seconds must be nonnegative')
        run(args)


if __name__=='__main__':
    main()
