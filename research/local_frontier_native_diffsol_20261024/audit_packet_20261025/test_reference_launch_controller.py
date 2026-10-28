"""Synthetic controls only: no Julia executable or numerical solve is launched."""
import contextlib
import copy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import signal
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).with_name('reference_launch_controller.py')
spec = importlib.util.spec_from_file_location('reference_launch_controller', SCRIPT)
c = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = c
spec.loader.exec_module(c)


_CONTROL_INDEX = 0


@contextlib.contextmanager
def temporary():
    global _CONTROL_INDEX
    target = os.environ.get('REFERENCE_CONTROLLER_TEST_EVIDENCE')
    if target:
        _CONTROL_INDEX += 1
        parent = Path(target); parent.mkdir(parents=True, exist_ok=True)
        directory = parent/f'fixture_{_CONTROL_INDEX:03d}'
        directory.mkdir(mode=0o700)
        # Keep paths live so retained synthetic cost chains remain independently replayable.
        yield str(directory)
        return
    with tempfile.TemporaryDirectory() as directory:
        yield directory


def dump(p, value):
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(value, sort_keys=True)+'\n')
    return {'path': str(p), 'sha256': c.sha_file(p)}


def event(root, attempt='a', phase='TERMINAL', value=2, **extra):
    evidence = root/(attempt+'_'+phase+'.evidence')
    evidence.write_text('immutable evidence\n')
    return dict(schema='vigilode.local-continuation-cost-event.v1', event_id=attempt+'-'+phase,
        attempt_id=attempt, category='julia', phase=phase,
        **({'charged_seconds': value} if phase == 'TERMINAL' else {'charge_upper_seconds': value}),
        evidence_path=str(evidence), evidence_sha256=c.sha_file(evidence), **extra)


def chain(root, events, spaced=False):
    previous, lines = c.ZERO_SHA, []
    for e in events:
        row = dict(e, prev_event_sha256=previous)
        lines.append(json.dumps(row, sort_keys=True, separators=None if spaced else (',', ':'))+'\n')
        previous = c.canonical_sha(row)
    (root/'COST_EVENTS.jsonl').write_text(''.join(lines))
    (root/'COST_EVENTS.lock').touch()


class Fixture:
    def __init__(self, root):
        self.root = root
        self.approval_path = root/'approval.json'
        self.ledger_path = root/'ledger.json'
        chain(root, [])
        self.plan = json.loads(SCRIPT.with_name('INDEPENDENT_REFERENCE_LAUNCH_PLAN.template.json').read_text())
        p = self.plan
        p.update(status='APPROVED', launch_enabled=True, cost_root=str(root),
            execution_lock_path=str(root/c.EXECUTION_LOCK_NAME), output_root=str(root/'output'),
            new_48h_cap_seconds=172800.0, hard_deadline_utc=c.GLOBAL_DEADLINE,
            source={'repo': str(root), 'head':'head', 'tree':'tree'}, blocked_process_tokens=[])
        for name in ('controller','descriptor_manifest','project','manifest','julia_executable','terminal_matrix'):
            path = (root/'julia-1.12.7'/'bin'/'julia' if name=='julia_executable' else
                    root/'environment'/('Project.toml' if name=='project' else 'Manifest.toml') if name in ('project','manifest') else root/(name+'.json'))
            dump(path, {'schema':'vigilode.local-continuation-terminal-matrix.v1'} if name=='terminal_matrix' else {'synthetic':name})
            p[name+'_path'], p[name+'_sha256'] = str(path), c.sha_file(path)
        p['isolated_depot_path'] = str(root/'depot'); (root/'depot').mkdir()
        self.task_catalog = {}
        for case in (8,9,11):
            script = root/c.EXPECTED_SCRIPTS[case]; script.write_text('# synthetic '+script.name)
            desc = root/f'descriptor_{case}.json'; dump(desc, {'case':case})
            self.task_catalog[case] = {'case':case, 'mode':'solve', 'julia_script_path':str(script),
                'julia_script_sha256':c.sha_file(script), 'descriptor_path':str(desc),
                'descriptor_sha256':c.sha_file(desc), 'required_outputs':['EXTERNAL_REFERENCE.json']}
        self.select_case(8)
        participants=[]
        for name in ('julia_comparison.py','continuation_runner.py'):
            q=root/name; q.write_text('# synthetic reviewed participation')
            participants.append({'path':str(q), 'sha256':c.sha_file(q)})
        p['shared_execution_lock_participants'] = participants
        p['shared_execution_lock_receipt'] = dump(root/'shared_review.json', {
            'schema':'vigilode.shared-execution-lock-review.v1','decision':'PASS_SHARED_EXECUTION_LOCK',
            'execution_lock_path':str(root/c.EXECUTION_LOCK_NAME), 'participants':participants})
        self.refresh_ledger()
        self.path=root/'plan.json'

    def select_case(self, case):
        self.plan['tasks'] = [copy.deepcopy(self.task_catalog[case])]
        self.plan['execution_policy']['sequence'] = [case]

    def approve(self):
        self.plan['approval_receipt'] = dump(self.approval_path, {
            'schema':c.APPROVAL_SCHEMA, 'decision':'APPROVED_SINGLE_REFERENCE_TASK',
            'selected_case':self.plan['tasks'][0]['case'],
            'approved_plan_core_sha256':c.plan_core_sha(self.plan)})

    def refresh_ledger(self):
        p=self.plan; rows, digest=c.validate_event_chain(self.root/'COST_EVENTS.jsonl')
        p['expected_cost_events_sha256']=digest
        info=dump(self.ledger_path, {'events_sha256_at_snapshot':digest,'cost_event_count':len(rows),
            'hard_deadline_utc':c.GLOBAL_DEADLINE,'new_48h_cap_seconds':172800.0})
        p['cost_ledger_path'],p['cost_ledger_sha256']=info['path'],info['sha256']

    @contextlib.contextmanager
    def mocks(self, *, approve=True):
        if approve:
            self.approve()
        dump(self.path,self.plan)
        with patch.object(c,'GLOBAL_COST_ROOT',self.root), patch.object(c,'git_identity',return_value={
                'head':'head','tree':'tree','status_porcelain':''}), patch.object(c,'bounded_command',return_value='julia version 1.12.7'):
            yield


class ControllerSyntheticTests(unittest.TestCase):
    def tearDown(self):
        signal.setitimer(signal.ITIMER_REAL,0)
        c.release_execution_lock()

    def test_canonical_chain_accepts_spaced_historical_json(self):
        with temporary() as t:
            r=Path(t); chain(r,[event(r,'a','STARTED',9),event(r,'a'),event(r,'legacy'),event(r,'pending','STARTED',8)],spaced=True)
            rows,digest=c.validate_event_chain(r/'COST_EVENTS.jsonl')
            self.assertEqual(c.event_accounting(rows),(4.0,8.0))
            self.assertEqual(digest,c.sha_file(r/'COST_EVENTS.jsonl'))
            raw=(r/'COST_EVENTS.jsonl').read_text().replace('"charged_seconds": 2','"charged_seconds": 3',1)
            (r/'COST_EVENTS.jsonl').write_text(raw)
            with self.assertRaisesRegex(ValueError,'hash chain'):
                c.validate_event_chain(r/'COST_EVENTS.jsonl')

    def test_phase_category_evidence_and_amount_negatives(self):
        with temporary() as t:
            r=Path(t); good=event(r)
            bad=[dict(good,category='unknown'),dict(good,phase='UNKNOWN'),dict(good,charged_seconds=True),
                 dict(good,evidence_sha256='0'*64),dict(good,charged_seconds=float('inf'))]
            for row in bad:
                with self.subTest(row=row),self.assertRaises(ValueError):
                    c.event_accounting([row])
            for rows in ([good,good], [good,event(r,'a','STARTED',9)]):
                with self.assertRaisesRegex(ValueError,'phase'):
                    c.event_accounting(rows)

    def test_atomic_append_checks_prior_head_and_preserves_first_terminal(self):
        with temporary() as t:
            r=Path(t); chain(r,[])
            row=event(r,'a','STARTED',9)
            c.append_event(r/'COST_EVENTS.jsonl',r/'COST_EVENTS.lock',row,expected_head=c.ZERO_SHA)
            first=(r/'COST_EVENTS.jsonl').read_bytes()
            with self.assertRaisesRegex(ValueError,'chain changed'):
                c.append_event(r/'COST_EVENTS.jsonl',r/'COST_EVENTS.lock',event(r),expected_head=c.ZERO_SHA)
            self.assertEqual(first,(r/'COST_EVENTS.jsonl').read_bytes())
            terminal=event(r)
            c.append_event(r/'COST_EVENTS.jsonl',r/'COST_EVENTS.lock',terminal)
            first=(r/'COST_EVENTS.jsonl').read_bytes()
            with self.assertRaises(ValueError):
                c.append_event(r/'COST_EVENTS.jsonl',r/'COST_EVENTS.lock',terminal)
            self.assertEqual(first,(r/'COST_EVENTS.jsonl').read_bytes())

    def test_native_and_controller_costs_count_and_unfinished_refuses(self):
        with temporary() as t:
            r=Path(t); chain(r,[event(r,value=5)])
            start=r/'native'/'attempt_001'/'START.json'; dump(start,{})
            with self.assertRaisesRegex(ValueError,'unfinished native'):
                c.cumulative_cost(r)
            dump(start.parent/'RECEIPT.json',{'charged_new_seconds':7})
            ctrl=r/'native'/'controller'/'invocation_001'/'START.json'; dump(ctrl,{})
            with self.assertRaisesRegex(ValueError,'unfinished continuation controller'):
                c.cumulative_cost(r)
            dump(ctrl.parent/'RECEIPT.json',{'charged_seconds':11})
            self.assertEqual(c.cumulative_cost(r),23)

    def test_fixed_global_cap_root_and_deadline_reject_restart_bypass(self):
        with temporary() as t:
            r=Path(t); f=Fixture(r)
            with patch.object(c,'GLOBAL_COST_ROOT',r):
                for key,value in [('cost_root',str(r/'replacement')),('new_48h_cap_seconds',172801),
                                   ('hard_deadline_utc','2099-01-01T00:00:00Z'),('execution_lock_path',str(r/'other.lock'))]:
                    bad=dict(f.plan,**{key:value})
                    with self.subTest(key=key),self.assertRaisesRegex(ValueError,'fixed global'):
                        c.validate_global_binding(bad)
                chain(r,[event(r,value=171850)])
                dump(r/'native'/'attempt_001'/'START.json',{})
                dump(r/'native'/'attempt_001'/'RECEIPT.json',{'charged_new_seconds':100})
                f.refresh_ledger()
                with self.assertRaisesRegex(ValueError,'cumulative cap'):
                    c.current_cost_gate(f.plan)

    def test_case_specific_script_and_hash(self):
        with temporary() as t:
            f=Fixture(Path(t))
            for task in f.task_catalog.values():
                self.assertEqual(Path(c.task_script(task)['path']).name,c.EXPECTED_SCRIPTS[task['case']])
            bad=dict(f.task_catalog[9],julia_script_path=f.task_catalog[8]['julia_script_path'])
            with self.assertRaisesRegex(ValueError,'case-specific'):
                c.task_script(bad)
            with self.assertRaisesRegex(ValueError,'SHA mismatch'):
                c.task_script(dict(f.plan['tasks'][0],julia_script_sha256='0'*64))

    def test_direct_versioned_executable_required(self):
        with temporary() as t:
            r=Path(t); f=Fixture(r)
            with f.mocks():
                c.verify_locks(f.plan,r, time.monotonic()+900)
                launcher=r/'julialauncher'; launcher.write_text('launcher')
                f.plan.update(julia_executable_path=str(launcher),julia_executable_sha256=c.sha_file(launcher))
                f.approve()
                with self.assertRaisesRegex(ValueError,'versioned Julia'):
                    c.verify_locks(f.plan,r,time.monotonic()+900)

    def test_shared_lock_review_required_for_both_participants(self):
        with temporary() as t:
            r=Path(t); f=Fixture(r)
            with f.mocks():
                self.assertEqual(len(c.verify_shared_lock(f.plan)),2)
                f.plan['shared_execution_lock_participants']=f.plan['shared_execution_lock_participants'][:1]
                with self.assertRaisesRegex(ValueError,'HOLD'):
                    c.verify_shared_lock(f.plan)

    def test_shared_execution_lock_blocks_competing_campaign_before_reservation(self):
        with temporary() as t:
            r=Path(t); f=Fixture(r)
            with (r/c.EXECUTION_LOCK_NAME).open('a+') as held, f.mocks():
                fcntl.flock(held,fcntl.LOCK_EX|fcntl.LOCK_NB)
                with self.assertRaisesRegex(RuntimeError,'shared execution lock'):
                    c.launch(f.path)
            self.assertEqual((r/'COST_EVENTS.jsonl').read_bytes(),b'')
            self.assertFalse((r/'output').exists())

    def test_setup_crash_is_terminal_and_charged(self):
        with temporary() as t:
            r=Path(t); f=Fixture(r)
            with f.mocks(),patch.object(c,'verify_locks',side_effect=RuntimeError('synthetic setup crash')):
                result=c.launch(f.path)
            self.assertEqual(result['status'],'FAILED_CONTROLLER')
            self.assertIn('setup crash',result['error'])
            rows,_=c.validate_event_chain(r/'COST_EVENTS.jsonl')
            self.assertEqual([x['phase'] for x in rows],['STARTED','TERMINAL'])
            self.assertGreater(rows[-1]['charged_seconds'],0)
            self.assertTrue(json.loads((r/'output'/'CAMPAIGN_START.json').read_text())['includes_preflight'])

    def test_actual_executable_mutation_during_version_probe_is_charged(self):
        with temporary() as t:
            r=Path(t); f=Fixture(r)
            def version(*args):
                Path(f.plan['julia_executable_path']).write_text('mutated binary')
                return 'julia version 1.12.7'
            with f.mocks(),patch.object(c,'bounded_command',side_effect=version),patch.object(c,'run_child') as spawn:
                result=c.launch(f.path)
            self.assertIn('SHA mismatch',result['error'])
            spawn.assert_not_called()
            self.assertEqual(c.validate_event_chain(r/'COST_EVENTS.jsonl')[0][-1]['phase'],'TERMINAL')

    def test_single_predeclared_script_and_shared_lock_used_during_child(self):
        with temporary() as t:
            r=Path(t); f=Fixture(r); scripts=[]
            def child(argv,*args,**kwargs):
                scripts.append(Path(argv[-4]).name)
                with (r/c.EXECUTION_LOCK_NAME).open('a+') as competing:
                    with self.assertRaises(BlockingIOError):
                        fcntl.flock(competing,fcntl.LOCK_EX|fcntl.LOCK_NB)
                output=Path(argv[-2]); output.mkdir()
                dump(output/'EXTERNAL_REFERENCE.json',{'synthetic':True})
                return {'status':'COMPLETED','exit_code':0,'elapsed_seconds':0.01}
            with f.mocks(),patch.object(c,'run_child',side_effect=child):
                result=c.launch(f.path)
            self.assertEqual(scripts,['independent_sparse_references.jl'])
            self.assertEqual(result['status'],'COMPLETED_ALL_CHILDREN')
            self.assertEqual(len(result['children']),1)

    def test_actual_executable_version_mismatch_is_charged(self):
        with temporary() as t:
            r=Path(t); f=Fixture(r)
            with f.mocks(),patch.object(c,'bounded_command',return_value='julia version 0.0.0'),patch.object(c,'run_child') as spawn:
                result=c.launch(f.path)
            self.assertIn('executable version mismatch',result['error'])
            spawn.assert_not_called()
            self.assertEqual(c.validate_event_chain(r/'COST_EVENTS.jsonl')[0][-1]['phase'],'TERMINAL')

    def test_input_mutation_after_child_gets_terminal_and_stops_wave(self):
        self._mutation_control('descriptor_path')
        self._mutation_control('julia_executable_path')

    def _mutation_control(self, field):
        with temporary() as t:
            r=Path(t); f=Fixture(r)
            calls=[]
            def child(*args,**kwargs):
                calls.append(args[0]); Path(f.plan['tasks'][0]['descriptor_path'] if field=='descriptor_path' else f.plan[field]).write_text('mutated')
                return {'status':'COMPLETED','exit_code':0,'elapsed_seconds':0.01}
            with f.mocks(),patch.object(c,'run_child',side_effect=child):
                result=c.launch(f.path)
            self.assertEqual(len(calls),1)
            terminal=json.loads((r/'output'/'01_case8'/'TERMINAL.json').read_text())
            self.assertEqual(terminal['controller_gate'],'FAILED_INPUT_MUTATION')
            self.assertIn('observation_error',terminal['locks_after']['locks']['descriptor_8' if field=='descriptor_path' else 'julia_executable'])
            self.assertEqual(result['status'],'STOPPED_ON_FIRST_FAILURE')
            self.assertEqual(c.validate_event_chain(r/'COST_EVENTS.jsonl')[0][-1]['phase'],'TERMINAL')

    def test_observer_exception_and_after_verification_exception_get_terminal(self):
        with temporary() as t:
            r=Path(t); f=Fixture(r); original=c.verify_locks
            def after(*args,**kwargs):
                if kwargs.get('observational'): raise OSError('synthetic after-observer failure')
                return original(*args,**kwargs)
            with f.mocks(),patch.object(c,'verify_locks',side_effect=after),patch.object(c,'run_child',side_effect=OSError('synthetic capture failure')):
                result=c.launch(f.path)
            terminal=json.loads((r/'output'/'01_case8'/'TERMINAL.json').read_text())
            self.assertEqual(terminal['process']['status'],'FAILED_OBSERVER')
            self.assertEqual(terminal['controller_gate'],'FAILED_INPUT_MUTATION')
            self.assertEqual(result['status'],'STOPPED_ON_FIRST_FAILURE')

    def test_existing_output_preserves_all_prior_bytes(self):
        with temporary() as t:
            r=Path(t); f=Fixture(r); old=r/'output'/'FAILED.json'; dump(old,{'first':'failure'}); before=old.read_bytes()
            with f.mocks(),self.assertRaises(RuntimeError): c.launch(f.path)
            self.assertEqual(before,old.read_bytes())
            self.assertEqual((r/'COST_EVENTS.jsonl').read_bytes(),b'')

    def test_observer_failure_kills_group_reaps_and_retains_raw(self):
        with temporary() as t:
            r=Path(t); marker=r/'child_ready'
            code='import subprocess,sys,time,pathlib; subprocess.Popen([sys.executable,"-c","import time; time.sleep(5)"]); print("started",flush=True); pathlib.Path(sys.argv[1]).write_text("ready"); time.sleep(5)'
            def failed_observer(_):
                until=time.monotonic()+2
                while not marker.exists() and time.monotonic()<until: time.sleep(.01)
                if not marker.exists(): raise RuntimeError('synthetic child startup failed')
                raise RuntimeError('synthetic observer failure')
            with patch.object(c,'proc_group',side_effect=failed_observer):
                result=c.run_child([sys.executable,'-c',code,str(marker)],t,dict(os.environ),0,3,3,1<<30,1<<20,.01,r/'out',r/'err',r/'res')
            self.assertEqual(result['status'],'FAILED_OBSERVER')
            self.assertIsNotNone(result['exit_code'])
            self.assertIn(b'started',(r/'out').read_bytes())
            pgid=json.loads((r/'out.PROCESS.json').read_text())['pgid']
            # A killed grandchild can briefly remain as init-owned zombie; no live descendant is allowed.
            self.assertEqual(c.proc_group(pgid)[1],0)
            self.assertIsNone(c.ACTIVE_PROC)

    def test_idle_and_hard_deadline_group_kill(self):
        for hard in (False,True):
            with self.subTest(hard=hard),temporary() as t:
                r=Path(t); code='import signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); print("partial",flush=True); time.sleep(5)'
                result=c.run_child([sys.executable,'-c',code],t,dict(os.environ),0,.15 if hard else 1,1 if hard else .15,
                                   1<<30,1<<20,.01,r/'out',r/'err',r/'res')
                self.assertEqual(result['status'],'FAILED_HARD_DEADLINE' if hard else 'FAILED_IDLE_WATCHDOG')
                self.assertEqual(result['exit_code'],-9)
                self.assertEqual(result['actual_cpu_affinity'],[0])

    def test_absolute_deadline_rechecked_at_spawn_after_slow_setup(self):
        with temporary() as t:
            r=Path(t)
            with patch.object(c,'absolute_remaining',return_value=0),patch.object(c.subprocess,'Popen') as spawn:
                result=c.run_child([sys.executable],t,dict(os.environ),0,900,120,1<<30,1<<20,.01,r/'out',r/'err',r/'res')
            spawn.assert_not_called()
            self.assertEqual(result['status'],'FAILED_PREFLIGHT_OR_SPAWN')
            self.assertIn('directly before spawn',result['error'])

    def test_preflight_command_timeout_is_bounded_and_raw_retained(self):
        with temporary() as t:
            r=Path(t); before=time.monotonic()
            with patch.object(c,'PREFLIGHT_TIMEOUT_SECONDS',.15),self.assertRaisesRegex(RuntimeError,'bounded preflight'):
                c.bounded_command([sys.executable,'-c','import time; print("setup",flush=True); time.sleep(5)'],t,r,'command',time.monotonic()+900)
            self.assertLess(time.monotonic()-before,2)
            self.assertIn(b'setup',(r/'command.stdout').read_bytes())
            self.assertEqual(json.loads((r/'command.json').read_text())['status'],'FAILED_HARD_DEADLINE')

    def test_earliest_global_timer_arms_before_preflight(self):
        with temporary() as t:
            r=Path(t); f=Fixture(r); armed=[]
            with f.mocks(),patch.object(c,'absolute_remaining',return_value=30),patch.object(c.signal,'setitimer',side_effect=lambda *x:armed.append(x)),patch.object(c,'verify_locks',side_effect=RuntimeError('stop before commands')):
                c.launch(f.path)
            self.assertGreater(armed[0][1],0)
            self.assertLessEqual(armed[0][1],30)
            self.assertEqual(armed[-1][1],0)

    def test_nonlaunchable_template_rejected_without_side_effects(self):
        with temporary() as t:
            r=Path(t); p=r/'plan.json'; dump(p,{'schema':c.SCHEMA,'status':'TEMPLATE','launch_enabled':False})
            with self.assertRaisesRegex(ValueError,'nonlaunchable'): c.launch(p)
            self.assertEqual(list(r.iterdir()),[p])


if __name__=='__main__':
    unittest.main(verbosity=2)
