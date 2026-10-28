"""Deterministic P3 scheduling and negative resource/telemetry controls."""
from __future__ import annotations

import copy
import json
import os
from pathlib import Path
import platform
import shutil
import sys
import tempfile
import threading
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT))
from tools.component_benchmark.budget import close, reserve
from tools.component_benchmark.cases import build_cases
from tools.component_benchmark import confirmation as co


def fixture(count=2, rss=100*(1<<20), budget=2*(1<<30)):
    case = next(c for c in build_cases() if c['case_id']=='ode-scalar-decay')
    solvers = {'base':{'command':['/bin/true'],'cwd':str(ROOT),'env':{}},
               'candidate':{'command':['/bin/true'],'cwd':str(ROOT),'env':{}}}
    config = {'cases':[case], 'solvers':solvers, 'random_seed':20261018,
              'global_contrasts':[{'case_id':case['case_id'],'baseline':'base',
                                   'candidate':'candidate'} for _ in range(count)],
              'screened_peak_rss_bytes':{'base':rss,'candidate':rss}}
    resources = SimpleNamespace(worker_cpus=tuple(range(8,24)),memory_budget_bytes=budget,
                                host=platform.node())
    return config,resources


class ConfirmationTests(unittest.TestCase):
    def test_freeze_rejects_parallel_solver_thread_options(self):
        config,resources=fixture(count=1)
        config['solvers']['base']['threads']=2
        with self.assertRaisesRegex(ValueError,'P3_SINGLE_THREAD_LANE_REQUIRED'):
            co.freeze(config,resources,{},{} )
        del config['solvers']['base']['threads']
        config['global_contrasts'][0]['baseline_config_id']='parallel'
        config['global_contrasts'][0]['baseline_solver_config']={'threads':4}
        with self.assertRaisesRegex(ValueError,'P3_SINGLE_THREAD_LANE_REQUIRED'):
            co.freeze(config,resources,{},{} )
        del config['global_contrasts'][0]['baseline_solver_config']
        del config['global_contrasts'][0]['baseline_config_id']
        config['solvers']['base']['env']={'RAYON_NUM_THREADS':'4'}
        with self.assertRaisesRegex(ValueError,'P3_SINGLE_THREAD_LANE_REQUIRED'):
            co.freeze(config,resources,{},{} )

    def test_resource_regression_before_next_pending_barrier_persists_hold(self):
        config,resources=fixture(count=1)
        plan=co.freeze(config,resources,{},{} )
        reduced=SimpleNamespace(**{**resources.__dict__, 'memory_budget_bytes':1})
        with tempfile.TemporaryDirectory() as temp:
            output=Path(temp)/'out'
            launched=[]
            def fake_barrier(jobs,plan,output,hard_stop):
                launched.append(jobs[0]['identity']['phase'])
                folder=Path(jobs[0]['folder'])
                folder.mkdir(parents=True)
                (folder/'receipt.json').write_text('{}')
                return []
            with patch.object(co,'ROUNDS',1), \
                 patch.object(co,'source_binding',return_value={}), \
                 patch.object(co,'executable_binding',return_value={}), \
                 patch.object(co,'_lease_deadline',return_value=time.monotonic()+100), \
                 patch.object(co.hpc,'detect_resources',side_effect=[resources,reduced]), \
                 patch.object(co,'campaign_process_census',return_value={'status':'OBSERVED','matching_pids':[]}), \
                 patch.object(co,'_barrier_jobs',side_effect=fake_barrier):
                with self.assertRaisesRegex(RuntimeError,'P3_MEMORY_CAPACITY_REGRESSED'):
                    co.execute(plan,output,Path(temp)/'ledger','fixture',ROOT)
            self.assertEqual(launched,[0])
            hold=json.loads((output/'resource_hold.json').read_text())
            self.assertEqual(hold['status'],'HOLD_RESOURCES')
            self.assertEqual(hold['phase'],1)
            self.assertEqual(len(hold['completed_receipts']),1)

    def test_resume_with_completed_barrier_holds_without_recapturing_census(self):
        config,resources=fixture(count=1)
        plan=co.freeze(config,resources,{},{} )
        with tempfile.TemporaryDirectory() as temp:
            output=Path(temp)/'out'
            prior=output/'jobs'/'r00-c00-p0-00'/'receipt.json'
            prior.parent.mkdir(parents=True)
            prior.write_text('{"frozen":"original"}\n')
            with patch.object(co,'ROUNDS',1), \
                 patch.object(co,'source_binding',return_value={}), \
                 patch.object(co,'executable_binding',return_value={}), \
                 patch.object(co,'campaign_process_census',side_effect=AssertionError('recaptured')), \
                 patch.object(co,'_barrier_jobs',side_effect=AssertionError('relaunched')):
                with self.assertRaisesRegex(RuntimeError,'P3_RESUME_HOLD_HISTORICAL_BARRIER_CENSUS_UNAVAILABLE'):
                    co.execute(plan,output,Path(temp)/'ledger','fixture',ROOT)
            self.assertEqual(prior.read_text(),'{"frozen":"original"}\n')
            hold=json.loads((output/'resume_hold.json').read_text())
            self.assertEqual(hold['status'],'HOLD_RESUME')
            self.assertEqual(len(hold['completed_receipts']),1)

    def test_missing_rss_holds_and_cohort_assignment_is_frozen(self):
        config, resources = fixture(count=3,budget=900*(1<<20))
        missing = copy.deepcopy(config)
        del missing['screened_peak_rss_bytes']['candidate']
        self.assertEqual(co.freeze(missing,resources,{},{} )['status'],
                         'HOLD_MISSING_SCREENED_RSS')
        plan = co.freeze(config,resources,{},{} )
        self.assertEqual(plan['status'],'READY')
        self.assertEqual(plan['cohorts'],[[0],[1],[2]])
        self.assertEqual([x['guest_cpu'] for x in plan['contrasts']],[8,9,10])
        self.assertEqual(plan['global_multiplicity'],3)
        self.assertEqual(plan['estimated_arm_timeout_seconds'],17*3*2*600)
        self.assertEqual(len(plan['orders']),17*3)

    def test_worst_case_phase_cap_blocks_overpacked_plan(self):
        config,resources = fixture(count=16,budget=900*(1<<20))
        plan = co.freeze(config,resources,{},{} )
        self.assertEqual(plan['status'],'HOLD_PHASE_CAP')
        self.assertGreater(plan['estimated_arm_timeout_seconds'],24*3600)

    def test_barrier_prevents_second_arm_until_all_first_arms_finish(self):
        config,resources = fixture()
        plan = co.freeze(config,resources,{},{} )
        events = []
        lock = threading.Lock()
        def fake_arm(job,allocation,hard_stop):
            with lock: events.append(('start',job['identity']['phase'],job['contrast_index']))
            if job['identity']['phase']==0 and job['contrast_index']==0:
                time.sleep(.03)
            with lock: events.append(('end',job['identity']['phase'],job['contrast_index']))
            return {'identity':job['identity']}
        with tempfile.TemporaryDirectory() as temp, patch.object(co,'_run_arm',side_effect=fake_arm):
            for phase in (0,1):
                jobs = []
                for i in range(2):
                    jobs.append({'identity':{'phase':phase,'arm':'baseline' if phase==0 else 'candidate'},
                                 'folder':str(Path(temp)/f'{phase}-{i}'),
                                 'case':plan['contrasts'][i]['case'],
                                 'arm':plan['contrasts'][i]['baseline'],
                                 'contrast_index':i})
                co._barrier_jobs(jobs,plan,Path(temp),time.monotonic()+10)
        last_first = max(i for i,event in enumerate(events) if event[0]=='end' and event[1]==0)
        first_second = min(i for i,event in enumerate(events) if event[0]=='start' and event[1]==1)
        self.assertLess(last_first,first_second)

    def test_contamination_and_missing_pairs_hold(self):
        self.assertEqual(co.classify_pair([])['status'],'HOLD_INCOMPLETE_PAIR')
        base = {'identity':{'arm':'baseline'},'status':'OK',
                'assigned_cpu_steal_fraction':0.0,'affinity_escape':False,
                'affinity_verified':True,
                'cgroup_oom_events_delta':{'oom':0,'oom_kill':0},'peak_rss_bytes':1,
                'unregistered_campaign_work':False,
                'result':{'measurement_mode':'timing',
                          'timing_scope':co.paired_log_cost.__globals__['TIMING_SCOPE'],
                          'timing':{'timed_seconds':2.0,'warmup_seconds':1.0}}}
        candidate = copy.deepcopy(base)
        candidate['identity']['arm']='candidate'
        candidate['result']['timing']['timed_seconds']=1.0
        self.assertEqual(co.classify_pair([base,candidate])['status'],'PAIR_DIAGNOSTIC')
        unknown_rss=copy.deepcopy(candidate)
        unknown_rss['peak_rss_bytes']=None
        unknown_rss['rss_status']='UNAVAILABLE'
        unknown=co.classify_pair([base,unknown_rss])
        self.assertEqual(unknown['status'],'HOLD_UNKNOWN')
        self.assertIn('UNKNOWN_PEAK_RSS',unknown['reasons'])
        self.assertIsNone(unknown['log_cost'])
        candidate['assigned_cpu_steal_fraction']=.0101
        self.assertEqual(co.classify_pair([base,candidate])['status'],'HOLD_CONTAMINATED')
        candidate['assigned_cpu_steal_fraction']=None
        self.assertEqual(co.classify_pair([base,candidate])['status'],'HOLD_UNKNOWN')
        candidate['assigned_cpu_steal_fraction']=0
        candidate['affinity_escape']=True
        self.assertEqual(co.classify_pair([base,candidate])['status'],'HOLD_CONTAMINATED')
        candidate['affinity_escape']=False
        candidate['cgroup_oom_events_delta']={'oom':1,'oom_kill':0}
        self.assertEqual(co.classify_pair([base,candidate])['status'],'HOLD_CONTAMINATED')
        candidate['cgroup_oom_events_delta']={'oom':0,'oom_kill':0}
        candidate['unregistered_campaign_work']=True
        self.assertEqual(co.classify_pair([base,candidate])['status'],'HOLD_CONTAMINATED')
        candidate['unregistered_campaign_work']=False
        candidate['affinity_verified']=None
        self.assertEqual(co.classify_pair([base,candidate])['status'],'HOLD_UNKNOWN')
        candidate['affinity_verified']=True
        candidate['cgroup_oom_events_delta']=None
        decision=co.classify_pair([base,candidate])
        self.assertEqual(decision['status'],'HOLD_UNKNOWN')
        self.assertIn('UNKNOWN_OOM_TELEMETRY',decision['reasons'])
        candidate['cgroup_oom_events_delta']={'oom':0,'oom_kill':0}
        candidate['unregistered_campaign_work']=None
        decision=co.classify_pair([base,candidate])
        self.assertEqual(decision['status'],'HOLD_UNKNOWN')
        self.assertIn('UNKNOWN_CAMPAIGN_PROCESS_CENSUS',decision['reasons'])

    def test_unknown_census_cannot_become_clean_empty_census(self):
        observed={'status':'OBSERVED','matching_pids':[],'unregistered_campaign_work':False}
        unknown={'status':'UNKNOWN','matching_pids':None,'unregistered_campaign_work':None}
        self.assertIsNone(co._barrier_census_status(unknown,observed))
        self.assertIsNone(co._barrier_census_status(observed,unknown))
        self.assertIs(co._barrier_census_status(observed,observed),False)

    def test_lease_requires_full_timeout_reservation(self):
        with tempfile.TemporaryDirectory() as temp:
            ledger = Path(temp)/'ledger.json'
            lease = reserve(ledger,'P3',120,platform.node())
            try:
                with self.assertRaisesRegex(RuntimeError,'INSUFFICIENT'):
                    co._lease_deadline(ledger,lease,17*2*600)
            finally:
                close(ledger,lease)

    def test_real_hpc_process_fields_flow_into_arm_receipt(self):
        if not shutil.which('taskset'):
            self.skipTest('taskset unavailable')
        case = next(c for c in build_cases() if c['case_id']=='ode-scalar-decay')
        with tempfile.TemporaryDirectory() as temp:
            base=Path(temp)
            result={'status':'OK','measurement_mode':'timing',
                    'timing_scope':'warmed_solver_setup_integration_output_certificate',
                    'times':case['output_times'],'states':[[0.0] for _ in case['output_times']],
                    'counters':{'rhs_evaluations':1},
                    'timing':{'timed_seconds':.001,'warmup_seconds':.001},
                    'allocations':None}
            data=base/'result.json';data.write_text(json.dumps(result))
            script=base/'adapter.py'
            script.write_text('import pathlib,sys\nprint(pathlib.Path(sys.argv[1]).read_text())\n')
            arm={'solver_id':'fake','config_id':'default','solver_config':None,
                 'solver_spec':{'command':[sys.executable,str(script),str(data)],
                                'cwd':str(ROOT),'env':{}}}
            job={'identity':{'arm':'baseline'},'folder':str(base/'job'),
                 'case':case,'arm':arm}
            cpu=min(os.sched_getaffinity(0))
            receipt=co._run_arm(job,co.hpc.Allocation((cpu,),1<<20,'confirmation'),
                                time.monotonic()+10)
            self.assertEqual(receipt['status'],'OK',receipt['protocol_errors'])
            self.assertIs(receipt['affinity_verified'],True)
            self.assertIs(receipt['affinity_escape'],False)
            if receipt['peak_rss_bytes'] is None:
                self.assertEqual(receipt['rss_status'],'UNAVAILABLE')
                self.assertIn('unavailable',receipt['rss_source'])
                self.assertIn('UNKNOWN_PEAK_RSS',co.classify_pair([receipt,receipt])['reasons'])
            else:
                self.assertIsInstance(receipt['peak_rss_bytes'],int)
                self.assertGreater(receipt['peak_rss_bytes'],0)
                self.assertEqual(receipt['rss_status'],'OBSERVED')
            events=co.hpc._memory_events()
            if events is None:
                self.assertIsNone(receipt['cgroup_oom_events_delta'])
            else:
                self.assertEqual(set(receipt['cgroup_oom_events_delta']),{'oom','oom_kill'})
            self.assertTrue((base/'job'/'stdout').is_file())
            self.assertTrue((base/'job'/'receipt.json').is_file())

    def test_run_loads_frozen_plan_when_available_memory_changes(self):
        config, resources = fixture(count=1,budget=2*(1<<30))
        with tempfile.TemporaryDirectory() as temp:
            base=Path(temp)
            config_path=base/'config.json';config_path.write_text(json.dumps(config))
            out=base/'out'
            plan_args=['confirmation.py','--config',str(config_path),
                       '--output',str(out),'--mode','plan']
            run_args=['confirmation.py','--config',str(config_path),
                      '--output',str(out),'--mode','run',
                      '--ledger',str(base/'ledger.json'),'--lease-id','fixture']
            larger=SimpleNamespace(**{**resources.__dict__,
                                       'memory_budget_bytes':3*(1<<30)})
            with patch.object(co.hpc,'detect_resources',side_effect=[resources,larger]),\
                 patch.object(co,'source_binding',return_value={'frozen':'source'}),\
                 patch.object(co,'execute',return_value=[]) as called,\
                 patch.object(sys,'argv',plan_args):
                co.main()
                sys.argv=run_args
                co.main()
                called.assert_called_once()


if __name__=='__main__':
    unittest.main()
