"""Synthetic controls only: Python children and fixtures outside the cost root."""
import importlib.util
import copy
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock

MODULE_PATH = Path(os.environ.get('ACCURACY_DRIVER_UNDER_TEST',
                                 Path(__file__).with_name('accuracy_cohort_driver.py')))
sys.path.insert(0, '/tmp/vigilode-local-run-20261024.geLVCp9Q/continuation_20261025T080718Z')
spec = importlib.util.spec_from_file_location('accuracy_driver_under_test', MODULE_PATH)
driver = importlib.util.module_from_spec(spec)
spec.loader.exec_module(driver)
SCOPE = 'maximum component absolute endpoint error across every ensemble member; no continuous/phase-global certificate'


class AccuracyDriverSyntheticTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='vigilode-accuracy-synthetic-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.assertFalse(self.root.is_relative_to(driver.ROOT))

    def descriptor(self, case, dimension=2):
        return {'case_id':case,'size':'standard','t_start':0.0,'t_end':1.0,
                'initial_state':[0.0]*dimension, 'model_contract':{'case':case,'id':case,
                'dimension':dimension,'name':'Synthetic','variant':'fixture',
                'size':'standard','t_span':[0.0,1.0],'reference_kind':'external-reference-required'}}

    def expected(self):
        return {'case':2,'method':'dopri5','threads':1}, {'head':'h'*40}

    def header(self, ref, item=None, source=None):
        default_item,default_source=self.expected()
        item=item or default_item; source=source or default_source
        return {'kind':'header','schema_version':1,
            'model_contract':json.loads(Path(ref['path']).read_text())['model_descriptor']['model_contract'],
            'method':item['method'],'parallel_layer':'ensemble','threads':1,'ensemble_count':256,
            'instrumented':False,'rtol':1e-6,'atol':1e-9,'error_limit':1e-4,'max_steps':200000,
            'source_commit':source['head']}

    def reference(self, uncertainty=1e-12, dimension=2):
        path = self.root/'reference.json'
        descriptor = self.descriptor(2, dimension)
        value = {'model_descriptor_sha256': 'b'*64, 'norm': 'max_component_absolute',
                 'model_descriptor': descriptor,
                 'reference_state': [0.0]*dimension, 'uncertainty': uncertainty}
        path.write_text(json.dumps(value))
        return {'path': str(path), 'sha256': driver.sha(path), 'descriptor_sha256': 'b'*64}

    def row(self, ref, **overrides):
        uncertainty = json.loads(Path(ref['path']).read_text())['uncertainty']
        value = {'kind': 'result', 'schema_version':1, 'case': 2, 'ensemble_count': 256,
                 'method':'dopri5','parallel_layer':'ensemble','threads':1,'instrumented':False,
                 'size':'standard','dimension':2,'name':'Synthetic','variant':'fixture','final_time':1.0,
                 'member_successes': 256, 'solver_success': True, 'accuracy_scope': SCOPE,
                 'reference_sha256': ref['sha256'],
                 'model_descriptor_sha256': ref['descriptor_sha256'],
                 'actual_error': 1e-4, 'endpoint_reference_difference': 1e-4-uncertainty,
                 'reference_absolute_bound': uncertainty, 'maximum_error_member_index': 255,
                 'error_limit': 1e-4, 'final_state': [0.0, 0.0], 'status': 'PASS_SCOPED'}
        value.update(overrides)
        return value

    def assess_row(self, ref, **overrides):
        stdout = self.root/'stdout'
        stdout.write_text(json.dumps(self.header(ref))+'\n'+json.dumps(self.row(ref, **overrides))+'\n')
        item,source=self.expected()
        return driver.assess(stdout, item, ref, source)

    def legacy_binding(self, ref):
        # Reproduce the historical PASS fallback independently of its missing-key crash.
        return dict(ref, uncertainty=json.loads(Path(ref['path']).read_text())['uncertainty'])

    def test_real_three_field_binding_is_sufficient_and_uncertainty_counted_once(self):
        ref = self.reference()
        self.assertEqual(set(ref), {'path', 'sha256', 'descriptor_sha256'})
        self.assertEqual(self.assess_row(ref)['status'], 'PASS_SCOPED')

    def test_parent_rejects_forged_pass_above_achieved_cap(self):
        ref = self.legacy_binding(self.reference())
        self.assertEqual(self.assess_row(ref, actual_error=1.00001e-4,
                         endpoint_reference_difference=1.00001e-4-1e-12)['status'], 'ACCURACY_FAILED')

    def test_recomputed_error_bound_cannot_cross_cap_within_report_rounding_tolerance(self):
        ref = self.reference()
        self.assertEqual(self.assess_row(ref, actual_error=1e-4,
            endpoint_reference_difference=1e-4-1e-12+5e-16)['status'], 'ACCURACY_FAILED')

    def test_parent_rejects_forged_pass_with_solver_failure(self):
        ref = self.legacy_binding(self.reference())
        self.assertEqual(self.assess_row(ref, solver_success=False)['status'], 'SOLVER_FAILED')

    def test_bad_full_vector_outputs_and_member_count_are_rejected(self):
        ref = self.legacy_binding(self.reference())
        for changes in ({'final_state': []}, {'final_state': [0.0]},
                        {'final_state': [0.0, float('nan')]}, {'member_successes': 255},
                        {'member_successes': True}, {'actual_error': True},
                        {'reference_absolute_bound': 0.0}):
            with self.subTest(changes=changes):
                self.assertEqual(self.assess_row(ref, **changes)['status'], 'OUTPUT_CONTRACT_FAILURE')

    def test_reference_uncertainty_cap_cannot_be_bypassed(self):
        ref = self.legacy_binding(self.reference(uncertainty=1.00001e-5))
        self.assertEqual(self.assess_row(ref)['status'], 'OUTPUT_CONTRACT_FAILURE')

    def test_reference_mutation_and_malformed_full_vectors_fail_closed(self):
        ref = self.reference()
        Path(ref['path']).write_text('{}')
        stdout = self.root/'stdout'; stdout.write_text('{}\n')
        self.assertEqual(driver.assess(stdout, self.expected()[0], ref, self.expected()[1])['status'], 'OUTPUT_CONTRACT_FAILURE')
        for state in ([], [0.0], [0.0, float('inf')]):
            ref = self.reference()
            value = json.loads(Path(ref['path']).read_text()); value['reference_state'] = state
            Path(ref['path']).write_text(json.dumps(value)); ref['sha256'] = driver.sha(ref['path'])
            self.assertEqual(self.assess_row(self.legacy_binding(ref))['status'], 'OUTPUT_CONTRACT_FAILURE')

    def test_typed_solver_failure_is_preserved(self):
        ref = self.reference()
        self.assertEqual(self.assess_row(ref,status='SOLVER_FAILED',solver_success=False,
            member_successes=0,actual_error=None,endpoint_reference_difference=None,
            maximum_error_member_index=None,final_time=0.5)['status'],'SOLVER_FAILED')

    def test_header_endpoint_and_execution_identity_are_required(self):
        ref=self.reference(); item,source=self.expected(); stdout=self.root/'stdout'
        for kind,changes in [('missing_header',{}),('header',{'source_commit':'wrong'}),
                ('header',{'rtol':1e-5}),('header',{'max_steps':200001}),
                ('header',{'model_contract':{}}),('header',{'instrumented':True}),
                ('result',{'final_time':float('nan')}),('result',{'final_time':-123}),
                ('result',{'method':'wrong'}),('result',{'threads':99}),
                ('result',{'parallel_layer':'serial'}),('result',{'instrumented':True}),
                ('result',{'dimension':3}),('result',{'final_state':[1.0,0.0]})]:
            with self.subTest(kind=kind,changes=changes):
                header=self.header(ref); row=self.row(ref)
                (header if kind=='header' else row).update(changes)
                stdout.write_text(('' if kind=='missing_header' else json.dumps(header)+'\n')+json.dumps(row)+'\n')
                self.assertEqual(driver.assess(stdout,item,ref,source)['status'],'OUTPUT_CONTRACT_FAILURE')

    def test_numeric_validation_is_total_for_extreme_json_numbers(self):
        ref=self.reference()
        for changes in ({'final_state':[10**400,0]},{'actual_error':10**400},
                {'endpoint_reference_difference':10**400},{'final_time':10**400}):
            with self.subTest(field=next(iter(changes))):
                self.assertEqual(self.assess_row(ref,**changes)['status'],'OUTPUT_CONTRACT_FAILURE')

    def fixture(self):
        source = self.root/'source'; source.mkdir()
        binary = source/'fake_binary'; binary.write_bytes(b'synthetic-not-executable')
        lock = source/'Cargo.lock'; lock.write_bytes(b'synthetic-lock')
        source_identity = {'path': str(source), 'head': 'h'*40, 'tree': 't'*40,
             'cargo_lock_sha256': driver.sha(lock), 'binary_path': str(binary),
             'binary_sha256': driver.sha(binary)}
        refs = {}; selection_refs = {}
        for case in ('2', '3', '5'):
            directory = self.root/case; directory.mkdir()
            descriptor = directory/'describe'; descriptor.write_text(json.dumps(self.descriptor(int(case))))
            value = {'model_descriptor_sha256': driver.sha(descriptor),
                     'model_descriptor': self.descriptor(int(case)),
                     'norm': 'max_component_absolute', 'reference_state': [0.0,0.0],
                     'uncertainty': 1e-12}
            ref_path = directory/'ref.json'; ref_path.write_text(json.dumps(value))
            refs[case] = {'path': str(ref_path), 'sha256': driver.sha(ref_path),
                          'descriptor_sha256': driver.sha(descriptor)}
            selection_refs[case] = dict(refs[case], frozen_binary_describe_path=str(descriptor))
        attempts = []; groups=[]
        for case in (2,3,5):
            for method in ('dopri5','rodas5p-mf','rodas5p-dense','switching'):
                gid=f'c{case:02d}_{method}_ensemble_t1'; ref=refs[str(case)]; ids=[]
                for repeat in (None,0,1,2,3,4):
                    aid=gid+('_warmup' if repeat is None else f'_timed{repeat}'); ids.append(aid)
                    attempts.append({'attempt_id':aid,'group_id':gid,'case':case,'method':method,
                        'cpus':[0],'threads':1,'warmup':repeat is None,'timed_repeat':repeat,
                        'hard_timeout_seconds':180,'max_rss_bytes':12*1024**3,
                        'stdout_limit_bytes':4*1024**2,'stderr_limit_bytes':4*1024**2,
                        'env_overrides':{**{v:'1' for v in ('BLIS_NUM_THREADS','MKL_NUM_THREADS','OMP_NUM_THREADS',
                            'OPENBLAS_NUM_THREADS','RAYON_NUM_THREADS','VECLIB_MAXIMUM_THREADS')},
                            'VIGILODE_SOURCE_COMMIT':source_identity['head']},
                        'argv':[str(binary),'--case',str(case),'--size','standard','--method',method,
                            '--parallel-layer','ensemble','--threads','1','--rtol','1e-6','--atol','1e-9',
                            '--error-limit','1e-4','--max-steps','200000','--ensemble-count','256',
                            '--reference-json',ref['path'],'--reference-sha256',ref['sha256'],'--emit-state']})
                groups.append({'group_id':gid,'case':case,'method':method,'layer':'ensemble','threads':1,
                    'rtol':1e-6,'atol':1e-9,'error_limit':1e-4,'max_steps':200000,
                    'ensemble_count':256,'reference_sha256':ref['sha256'],'attempt_ids':ids})
        selection = {'schema': 'vigilode.local-continuation-accuracy-cohort-selection-template.v3',
                     'frozen_source_path':str(source),'binary_path':str(binary),
                     'references':selection_refs,'attempts':attempts,'groups':groups}
        selection_path = self.root/'selection.json'; selection_path.write_text(json.dumps(selection))
        (self.root/'COST_EVENTS.jsonl').write_text('')
        plan = {'schema': 'vigilode.accuracy-cohort-plan.v3', 'status': 'APPROVED_MATERIALIZED_PLAN',
                'deadline_utc': driver.DEADLINE, 'global_cap_seconds': driver.GLOBAL_CAP,
                'cohort_cap_seconds': driver.COHORT_CAP, 'cost_root': str(self.root),
                'shared_lock_path': str(self.root/'.julia_comparison_campaign.lock'),
                'cost_events_path': str(self.root/'COST_EVENTS.jsonl'),
                'selection_sha256': driver.sha(selection_path), 'source': source_identity,
                'output_root': str(self.root/'cohort'), 'references': refs,
                'executor': {'driver_path': str(MODULE_PATH.resolve()),
                   'driver_sha256': driver.sha(MODULE_PATH),
                   'cost_helper_path': str(driver.COST_HELPER_PATH.resolve()),
                   'cost_helper_sha256': driver.sha(driver.COST_HELPER_PATH),
                   'compare_path': str(driver.COMPARE_PATH), 'compare_sha256': driver.COMPARE_SHA256}}
        self.approve(plan)
        return plan, selection, selection_path

    def approve(self, plan):
        approval = {'schema': 'vigilode.accuracy-cohort-approval.v3',
            'decision': 'APPROVED_ACCURACY_COHORT_V3', 'plan_core_sha256': driver.plan_core_sha(plan),
            'source_head': plan['source']['head'], 'source_tree': plan['source']['tree'],
            'binary_sha256': plan['source']['binary_sha256'],
            'admitted_reference_sha256': {case: ref['sha256'] for case, ref in plan['references'].items()}}
        path = self.root/'approval.json'; path.write_text(json.dumps(approval))
        plan['approval_receipt'] = {'path': str(path), 'sha256': driver.sha(path)}

    def test_v3_verification_preserves_exact_binding_and_rejects_rss_widening(self):
        plan, selection, path = self.fixture()
        git = {k: plan['source'][k] for k in ('head','tree')}; git['status'] = ''
        with mock.patch.object(driver, 'ROOT', self.root), mock.patch.object(driver,'git_identity',return_value=git):
            self.assertEqual(driver.verify_plan(plan, selection_path=path), selection)
            selection['attempts'][0]['max_rss_bytes'] = 32*1024**3
            path.write_text(json.dumps(selection)); plan['selection_sha256'] = driver.sha(path); self.approve(plan)
            with self.assertRaisesRegex(ValueError, 'attempt.*contract'):
                driver.verify_plan(plan, selection_path=path)

    def test_v2_selection_is_nonlaunchable_and_executor_bytes_are_bound(self):
        plan, selection, path = self.fixture()
        git = {k: plan['source'][k] for k in ('head','tree')}; git['status'] = ''
        with mock.patch.object(driver,'ROOT',self.root), mock.patch.object(driver,'git_identity',return_value=git):
            plan['executor']['driver_sha256'] = '0'*64; self.approve(plan)
            with self.assertRaisesRegex(ValueError, 'executor'):
                driver.verify_plan(plan, selection_path=path)
            plan['executor']['driver_sha256'] = driver.sha(MODULE_PATH)
            plan['executor']['cost_helper_sha256'] = '0'*64; self.approve(plan)
            with self.assertRaisesRegex(ValueError, 'executor'):
                driver.verify_plan(plan, selection_path=path)
            plan['executor']['cost_helper_sha256'] = driver.sha(driver.COST_HELPER_PATH)
            selection['schema'] = 'vigilode.local-continuation-accuracy-cohort-selection-template.v2'
            path.write_text(json.dumps(selection)); plan['selection_sha256'] = driver.sha(path); self.approve(plan)
            with self.assertRaisesRegex(ValueError, 'V3 selection'):
                driver.verify_plan(plan, selection_path=path)

    def test_report_comparison_tool_path_and_bytes_are_locked(self):
        plan, selection, path = self.fixture()
        git = {k: plan['source'][k] for k in ('head','tree')}; git['status'] = ''
        with mock.patch.object(driver,'ROOT',self.root), mock.patch.object(driver,'git_identity',return_value=git):
            self.assertEqual(driver.verify_plan(plan,selection_path=path),selection)
            plan['executor']['compare_sha256']='0'*64; self.approve(plan)
            with self.assertRaisesRegex(ValueError,'executor comparison'):
                driver.verify_plan(plan,selection_path=path)
            plan['executor']['compare_sha256']=driver.COMPARE_SHA256
            other=self.root/'compare.py'; other.write_bytes(driver.COMPARE_PATH.read_bytes())
            plan['executor']['compare_path']=str(other); self.approve(plan)
            with self.assertRaisesRegex(ValueError,'executor comparison'):
                driver.verify_plan(plan,selection_path=path)
            other.write_text('synthetic byte mutation')
            with mock.patch.object(driver,'COMPARE_PATH',other):
                with self.assertRaisesRegex(ValueError,'executor comparison'):
                    driver.verify_plan(plan,selection_path=path)

    def test_exact_group_partition_and_full_argv_are_required(self):
        plan,selection,path=self.fixture()
        git={k:plan['source'][k] for k in ('head','tree')};git['status']=''
        mutations=[lambda s:s['groups'][0].update(attempt_ids=[]),
            lambda s:s['groups'][0].update(attempt_ids=s['groups'][0]['attempt_ids'][:1]),
            lambda s:s['groups'][1].update(group_id=s['groups'][0]['group_id']),
            lambda s:s['attempts'][1].update(attempt_id=s['attempts'][0]['attempt_id']),
            lambda s:s['attempts'][1].update(warmup=True),
            lambda s:s['attempts'][1].update(timed_repeat=4),
            lambda s:s['groups'][1].update(attempt_ids=s['groups'][0]['attempt_ids']),
            lambda s:s['attempts'][0]['argv'].__setitem__(s['attempts'][0]['argv'].index('--rtol')+1,'1e-5')]
        with mock.patch.object(driver,'ROOT',self.root),mock.patch.object(driver,'git_identity',return_value=git):
            for mutate in mutations:
                bad=copy.deepcopy(selection);mutate(bad);path.write_text(json.dumps(bad))
                plan['selection_sha256']=driver.sha(path);self.approve(plan)
                with self.subTest(mutation=mutations.index(mutate)):
                    with self.assertRaisesRegex(ValueError,'selection|attempt|group'):
                        driver.verify_plan(plan,selection_path=path)

    def synthetic_binary(self,plan,mode='success'):
        binary=Path(plan['source']['binary_path'])
        binary.write_text('#!/usr/bin/env python3\nimport json,os,sys\n'
            +'a=sys.argv[1:];arg=lambda f:a[a.index(f)+1]\n'
            +'ref=json.load(open(arg("--reference-json")));d=ref["model_descriptor"]\n'
            +'h={"kind":"header","schema_version":1,"model_contract":d["model_contract"],'
            +'"method":arg("--method"),"parallel_layer":"ensemble","threads":1,"ensemble_count":256,'
            +'"instrumented":False,"rtol":1e-6,"atol":1e-9,"error_limit":1e-4,"max_steps":200000,'
            +'"source_commit":os.environ["VIGILODE_SOURCE_COMMIT"]}\n'
            +'r={"kind":"result","schema_version":1,"case":int(arg("--case")),"name":"Synthetic",'
            +'"variant":"fixture","size":"standard","dimension":2,"method":arg("--method"),'
            +'"parallel_layer":"ensemble","threads":1,"ensemble_count":256,"instrumented":False,'
            +'"status":"PASS_SCOPED","solver_success":True,"member_successes":256,"final_time":1.0,'
            +'"final_state":[0.0,0.0],"actual_error":ref["uncertainty"],"reference_absolute_bound":ref["uncertainty"],'
            +'"endpoint_reference_difference":0.0,"maximum_error_member_index":0,"error_limit":1e-4,'
            +'"reference_sha256":arg("--reference-sha256"),"model_descriptor_sha256":ref["model_descriptor_sha256"],'
            +'"accuracy_scope":'+repr(SCOPE)+'}\n'
            +('r["final_state"]=[10**400,0]\n' if mode=='huge' else '')
            +'print(json.dumps(h));print(json.dumps(r))\n')
        binary.chmod(0o700);plan['source']['binary_sha256']=driver.sha(binary);self.approve(plan)

    def execute_fixture(self,plan,path):
        plan_path=self.root/'plan.json';plan_path.write_text(json.dumps(plan))
        git={k:plan['source'][k] for k in ('head','tree')};git['status']=''
        with mock.patch.object(driver,'ROOT',self.root),mock.patch.object(driver,'SELECTION',path), \
             mock.patch.object(driver,'git_identity',return_value=git):
            return driver.execute(plan_path)

    def test_full_approved_synthetic_cohort_succeeds_and_charges_each_child_once(self):
        plan,selection,path=self.fixture();self.synthetic_binary(plan)
        result=self.execute_fixture(plan,path)
        self.assertEqual(result['status'],'COMPLETED')
        self.assertEqual(len(result['groups']),12)
        self.assertTrue(all(g['status']=='PASS_SCOPED' for g in result['groups'].values()))
        rows,_=driver.cost.validate_event_chain(self.root/'COST_EVENTS.jsonl')
        children=[r for r in rows if r['category']=='other' and r['phase']=='TERMINAL']
        self.assertEqual(len(children),72)
        controller=[r for r in rows if r['category']=='controller' and r['phase']=='TERMINAL'][0]
        receipt=json.loads(Path(controller['evidence_path']).read_text())
        self.assertAlmostEqual(sum(r['charged_seconds'] for r in children),receipt['child_charged_seconds'])
        self.assertEqual(receipt['charged_seconds'],receipt['measured_controller_seconds']+20)

    def test_malformed_numeric_output_retains_terminal_and_closes_cost_events(self):
        plan,selection,path=self.fixture();self.synthetic_binary(plan,'huge')
        result=self.execute_fixture(plan,path)
        rows,_=driver.cost.validate_event_chain(self.root/'COST_EVENTS.jsonl')
        for item in selection['attempts']:
            folder=Path(plan['output_root'])/'runs'/item['attempt_id']
            if item['warmup']:
                terminal=json.loads((folder/'TERMINAL.json').read_text())
                self.assertEqual(terminal['assessment']['status'],'OUTPUT_CONTRACT_FAILURE')
                self.assertTrue(terminal['process_group_cleanup']['group_dead'])
                self.assertEqual({r['phase'] for r in rows if r['attempt_id']==item['attempt_id']},{'STARTED','TERMINAL'})
        self.assertTrue(all(g['status']!='PASS_SCOPED' for g in result['groups'].values()))

    def test_injected_assessment_exception_is_typed_and_child_charge_is_closed(self):
        plan,selection,path=self.fixture();self.synthetic_binary(plan)
        with mock.patch.object(driver,'assess',side_effect=RuntimeError('injected assessment failure')):
            result=self.execute_fixture(plan,path)
        folder=Path(plan['output_root'])/'runs'/selection['attempts'][0]['attempt_id']
        terminal=json.loads((folder/'TERMINAL.json').read_text())
        self.assertEqual(terminal['assessment']['status'],'CONTROLLER_ASSESSMENT_FAILURE')
        self.assertIn('injected assessment failure',terminal['assessment']['reason'])
        self.assertTrue(terminal['process_group_cleanup']['group_dead'])
        rows,_=driver.cost.validate_event_chain(self.root/'COST_EVENTS.jsonl')
        self.assertEqual([r['phase'] for r in rows],['STARTED','STARTED','TERMINAL','TERMINAL'])
        self.assertEqual(result['status'],'FAILED_CONTROLLER')

    def test_rejected_executor_is_accounted_without_cohort_or_lock_mutation(self):
        plan,selection,path=self.fixture();plan['executor']['driver_sha256']='0'*64;self.approve(plan)
        result=self.execute_fixture(plan,path)
        self.assertEqual(result['status'],'REJECTED_BEFORE_COHORT')
        self.assertFalse(Path(plan['output_root']).exists())
        self.assertFalse(Path(plan['shared_lock_path']).exists())
        self.assertEqual((self.root/'COST_EVENTS.jsonl').read_bytes(),b'')
        receipts=list((self.root/'accuracy_rejections/controller').glob('invocation_*/RECEIPT.json'))
        self.assertEqual(len(receipts),1)
        receipt=json.loads(receipts[0].read_text())
        self.assertGreater(receipt['charged_seconds'],0)
        self.assertEqual(receipt['charged_seconds'],receipt['measured_controller_seconds']+20)

    def test_invalid_approval_and_lock_rejections_receive_canonical_charges(self):
        plan,selection,path=self.fixture();plan['approval_receipt']['sha256']='0'*64
        result=self.execute_fixture(plan,path)
        self.assertEqual(result['status'],'REJECTED_BEFORE_COHORT')
        self.approve(plan)
        import fcntl
        with Path(plan['shared_lock_path']).open('a+') as held:
            fcntl.flock(held,fcntl.LOCK_EX|fcntl.LOCK_NB)
            result=self.execute_fixture(plan,path)
        self.assertEqual(result['status'],'REJECTED_BEFORE_COHORT')
        receipts=list((self.root/'accuracy_rejections/controller').glob('invocation_*/RECEIPT.json'))
        self.assertEqual(len(receipts),2)
        self.assertFalse(Path(plan['output_root']).exists())

    def test_cost_chain_and_cap_rejections_are_accounted_without_cohort_output(self):
        plan,selection,path=self.fixture()
        events=self.root/'COST_EVENTS.jsonl';events.write_text('invalid chain\n')
        result=self.execute_fixture(plan,path)
        self.assertEqual(result['status'],'REJECTED_BEFORE_COHORT')
        self.assertFalse(Path(plan['output_root']).exists())
        events.write_text('')
        driver.load_cost_helper(plan)
        evidence=self.root/'prior-cost.json';evidence.write_text('{}')
        row={'schema':'vigilode.local-continuation-cost-event.v1','event_id':'prior-cost-terminal',
            'attempt_id':'prior-cost','category':'setup','phase':'TERMINAL',
            'charged_seconds':driver.GLOBAL_CAP-1,'evidence_path':str(evidence),
            'evidence_sha256':driver.sha(evidence),'observed_utc':'synthetic'}
        driver.cost.append_event(events,self.root/'COST_EVENTS.lock',row)
        result=self.execute_fixture(plan,path)
        self.assertEqual(result['status'],'REJECTED_BEFORE_COHORT')
        self.assertFalse(Path(plan['output_root']).exists())
        receipts=list((self.root/'accuracy_rejections/controller').glob('invocation_*/RECEIPT.json'))
        self.assertEqual(len(receipts),2)
        rows,_=driver.cost.validate_event_chain(events)
        self.assertEqual(len(rows),1)

    def test_delayed_terminal_commit_is_conservatively_charged_and_overrun_recorded(self):
        plan,selection,path=self.fixture();plan_path=self.root/'plan.json';plan_path.write_text(json.dumps(plan))
        driver.load_cost_helper(plan)
        original=driver.cost.append_event
        class Clock:
            now=100.0
            def __call__(self):return self.now
        clock=Clock()
        def delayed(*args,**kwargs):
            result=original(*args,**kwargs)
            row=args[2]
            if row['category']=='controller' and row['phase']=='TERMINAL':clock.now+=25
            return result
        with mock.patch.object(driver,'ROOT',self.root),mock.patch.object(driver,'SELECTION',path), \
             mock.patch.object(driver,'verify_plan',return_value={'groups':[],'attempts':[]}), \
             mock.patch.object(driver.time,'monotonic',side_effect=clock), \
             mock.patch.object(driver.signal,'setitimer'),mock.patch.object(driver.cost,'append_event',side_effect=delayed):
            result=driver.execute(plan_path)
        self.assertEqual(result['status'],'FAILED_BUDGET')
        receipts=list((self.root/'accuracy_rejections/controller').glob('invocation_*/RECEIPT.json'))
        self.assertEqual(len(receipts),1)
        self.assertGreaterEqual(driver.cost.cumulative_cost(self.root),25)

    def test_one_failed_timed_result_prevents_group_pass(self):
        plan,selection,path=self.fixture();output=Path(plan['output_root'])
        for item in selection['attempts']:
            folder=output/'runs'/item['attempt_id'];folder.mkdir(parents=True)
            status='ACCURACY_FAILED' if item['timed_repeat']==0 else 'PASS_SCOPED'
            (folder/'TERMINAL.json').write_text(json.dumps({'assessment':{'status':status}}))
        groups=driver.summarize_groups(output,selection)
        self.assertEqual(len(groups),12)
        self.assertTrue(all(g['status']=='INCOMPLETE_OR_FAILED' for g in groups.values()))

    def attempt(self, code):
        source = self.root/'child-source'; source.mkdir()
        lock = source/'Cargo.lock'; lock.write_bytes(b'synthetic-lock')
        ref = self.reference()
        plan = {'selection_sha256': 's'*64, 'source': {'path': str(source),
            'binary_path': sys.executable, 'binary_sha256': driver.sha(sys.executable),
            'cargo_lock_sha256': driver.sha(lock), 'head': 'synthetic'},
            'executor': {'driver_path': str(MODULE_PATH.resolve()), 'driver_sha256': driver.sha(MODULE_PATH),
                'cost_helper_path': str(driver.COST_HELPER_PATH.resolve()),
                'cost_helper_sha256': driver.sha(driver.COST_HELPER_PATH),
                   'compare_path': str(driver.COMPARE_PATH), 'compare_sha256': driver.COMPARE_SHA256}}
        item = {'attempt_id':'synthetic','case':2,'argv':[sys.executable,'-c',code],
                'env_overrides': {'OMP_NUM_THREADS':'1'}, 'hard_timeout_seconds':180,
                'max_rss_bytes':12*1024**3,'stdout_limit_bytes':1<<20,'stderr_limit_bytes':1<<20}
        return item, plan, ref, self.root/'runs'/'synthetic'

    def assert_dead(self, terminal):
        proof = terminal['process_group_cleanup']
        self.assertTrue(proof['group_dead'])
        self.assertTrue(proof['leader_reaped'])
        self.assertEqual(proof['remaining_pids'], [])
        with self.assertRaises(ProcessLookupError):
            os.killpg(terminal['pid'], 0)

    def test_real_child_nonzero_exit_never_becomes_scoped_pass(self):
        ref = self.reference(); row = self.row(ref)
        item, plan, ref, folder = self.attempt('import json; print('+repr(json.dumps(row))+'); raise SystemExit(7)')
        # attempt() preserves identical reference file bytes.
        terminal = driver.execute_attempt(item, plan, ref, folder)
        self.assertEqual(terminal['exit_code'], 7)
        self.assertEqual(terminal['assessment']['status'], 'NONZERO_EXIT')
        self.assertIn('PASS_SCOPED',(folder/'stdout.bin').read_text())
        self.assert_dead(terminal)

    def test_real_wrapper_descendant_rss_limit_kills_and_reaps_group(self):
        item, plan, ref, folder = self.attempt('import time; a=bytearray(48*1024*1024); print("allocated",flush=True); time.sleep(30)')
        item['max_rss_bytes'] = 32*1024*1024
        terminal = driver.execute_attempt(item, plan, ref, folder)
        self.assertEqual(terminal['assessment']['status'], 'MEMORY_LIMIT')
        self.assertGreater(terminal['maximum_process_group_rss_bytes'], item['max_rss_bytes'])
        self.assertGreaterEqual(terminal['maximum_process_group_process_count'], 2)
        self.assert_dead(terminal)

    def test_real_timeout_preserves_raw_output_and_dead_group(self):
        item, plan, ref, folder = self.attempt('import time; print("before timeout",flush=True); time.sleep(30)')
        with mock.patch.object(driver,'ATTEMPT_SECONDS',0.3):
            terminal = driver.execute_attempt(item, plan, ref, folder)
        self.assertEqual(terminal['assessment']['status'], 'TIMEOUT')
        self.assertIn('before timeout',(folder/'stdout.bin').read_text())
        self.assert_dead(terminal)

    def test_real_sigterm_retains_lock_until_dead_group_proof(self):
        # Run the real execute/shared-lock/accounting path with a Python synthetic child.
        plan, selection, selection_path = self.fixture()
        item = selection['attempts'][0]
        item['argv'] = [sys.executable, '-c', 'import time; print("ready",flush=True); time.sleep(30)']
        plan_path = self.root/'plan.json'; plan_path.write_text(json.dumps(plan))
        folder = Path(plan['output_root'])/'runs'/item['attempt_id']
        script = self.root/'outer.py'
        script.write_text('import importlib.util,json,sys\nfrom pathlib import Path\n'
            +'s=importlib.util.spec_from_file_location("d",'+repr(str(MODULE_PATH))+');d=importlib.util.module_from_spec(s);s.loader.exec_module(d)\n'
            +'d.ROOT=Path('+repr(str(self.root))+')\n'
            +'d.verify_plan=lambda plan: '+repr(selection)+'\n'
            +'d.verify_selection=lambda plan,**kwargs: '+repr(selection)+'\n'
            +'result=d.execute(Path('+repr(str(plan_path))+'))\n'
            +'print(json.dumps(result));raise SystemExit(0 if result["status"]=="FAILED_INTERRUPTED" else 1)\n')
        proc = subprocess.Popen([sys.executable,str(script)],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        self.addCleanup(lambda: proc.poll() is None and proc.kill())
        limit = time.monotonic()+10
        while not (folder/'PROCESS.json').exists() and proc.poll() is None and time.monotonic()<limit:
            time.sleep(0.02)
        self.assertTrue((folder/'PROCESS.json').exists())
        import fcntl
        with Path(plan['shared_lock_path']).open('a+') as competing:
            with self.assertRaises(BlockingIOError):
                fcntl.flock(competing,fcntl.LOCK_EX|fcntl.LOCK_NB)
            os.kill(proc.pid,signal.SIGTERM)
            out,err = proc.communicate(timeout=10)
            self.assertEqual(proc.returncode,0,(out,err))
            fcntl.flock(competing,fcntl.LOCK_EX|fcntl.LOCK_NB)
        terminal = json.loads((folder/'TERMINAL.json').read_text())
        self.assertEqual(terminal['assessment']['status'],'INTERRUPTED')
        self.assert_dead(terminal)
        rows,_ = driver.cost.validate_event_chain(self.root/'COST_EVENTS.jsonl')
        self.assertEqual([r['phase'] for r in rows],['STARTED','STARTED','TERMINAL','TERMINAL'])
        receipt = json.loads(Path(rows[-1]['evidence_path']).read_text())
        self.assertTrue(receipt['all_spawned_groups_resolved'])
        self.assertAlmostEqual(sum(r.get('charged_seconds',0) for r in rows),receipt['outer_elapsed_seconds']+receipt['terminal_accounting_allowance_seconds'])

    def test_prior_unresolved_group_or_missing_proof_blocks_resume(self):
        folder = self.root/'attempt'; folder.mkdir()
        driver.new_json(folder/'START.json', {'attempt_id':'synthetic'})
        with self.assertRaisesRegex(ValueError,'unfinished'):
            driver.read_terminal(folder)
        driver.new_json(folder/'TERMINAL.json', {'assessment':{'status':'PASS_SCOPED'},
             'process_group_cleanup':{'group_dead':False,'leader_reaped':True,'remaining_pids':[123]}})
        with self.assertRaisesRegex(ValueError,'unresolved'):
            driver.read_terminal(folder)

    def test_preflight_failure_and_resume_only_invocations_are_charged(self):
        plan, selection, path = self.fixture()
        plan_path = self.root/'plan.json'; plan_path.write_text(json.dumps(plan))
        git = {k:plan['source'][k] for k in ('head','tree')}; git['status']=''
        with mock.patch.object(driver,'ROOT',self.root), mock.patch.object(driver,'SELECTION',path), \
             mock.patch.object(driver,'git_identity',side_effect=ValueError('synthetic preflight failure')):
            result = driver.execute(plan_path)
        self.assertEqual(result['status'],'FAILED_CONTROLLER')
        receipts = list((Path(plan['output_root'])/'accuracy_invocations').glob('*/RECEIPT.json'))
        self.assertEqual(len(receipts),1)
        receipt = json.loads(receipts[0].read_text())
        self.assertEqual(receipt['child_charged_seconds'],0)
        self.assertGreater(receipt['charged_seconds'],0)
        rows,_ = driver.cost.validate_event_chain(self.root/'COST_EVENTS.jsonl')
        self.assertEqual([r['phase'] for r in rows],['STARTED','TERMINAL'])
        self.assertEqual(rows[0]['charge_upper_seconds'],3240)
        self.assertEqual(rows[-1]['charged_seconds'],receipt['charged_seconds'])
        self.assertEqual(driver.cost.cumulative_cost(self.root,rows),receipt['charged_seconds'])

    def test_resume_without_children_still_charges_new_invocation_overhead(self):
        plan, selection, path = self.fixture()
        plan_path = self.root/'plan.json'; plan_path.write_text(json.dumps(plan))
        with mock.patch.object(driver,'ROOT',self.root), mock.patch.object(driver,'SELECTION',path), \
             mock.patch.object(driver,'verify_plan',return_value={'attempts':[],'groups':[]}), \
             mock.patch.object(driver,'execute_attempt') as run:
            first = driver.execute(plan_path)
            second = driver.execute(plan_path)
        run.assert_not_called()
        self.assertEqual(first['status'],'COMPLETED'); self.assertEqual(second['status'],'COMPLETED')
        rows,_ = driver.cost.validate_event_chain(self.root/'COST_EVENTS.jsonl')
        self.assertEqual([r['phase'] for r in rows],['STARTED','TERMINAL','STARTED','TERMINAL'])
        receipts=[json.loads(Path(r['evidence_path']).read_text()) for r in rows if r['phase']=='TERMINAL']
        self.assertTrue(all(r['charged_seconds']>0 and r['child_charged_seconds']==0 for r in receipts))
        self.assertAlmostEqual(driver.cost.cumulative_cost(self.root,rows),sum(r['charged_seconds'] for r in receipts))

    def test_entire_outer_elapsed_enforces_controller_inclusive_cohort_cap(self):
        plan, selection, path = self.fixture()
        plan_path = self.root/'plan.json'; plan_path.write_text(json.dumps(plan))
        class Clock:
            elapsed=100.0
            def __call__(self):
                return self.elapsed
        clock=Clock()
        def preflight(plan):
            clock.elapsed += driver.COHORT_CAP+1
            return selection
        with mock.patch.object(driver,'ROOT',self.root), mock.patch.object(driver,'SELECTION',path), \
             mock.patch.object(driver,'verify_plan',side_effect=preflight), \
             mock.patch.object(driver.time,'monotonic',side_effect=clock), \
             mock.patch.object(driver.signal,'setitimer'), \
             mock.patch.object(driver,'execute_attempt') as run:
            result=driver.execute(plan_path)
        run.assert_not_called()
        self.assertEqual(result['status'],'FAILED_BUDGET')
        rows,_=driver.cost.validate_event_chain(self.root/'COST_EVENTS.jsonl')
        self.assertEqual(rows[-1]['charged_seconds'],driver.COHORT_CAP+1+driver.FINALIZE_RESERVE)

    def test_live_controller_budget_stops_before_any_child_and_preserves_charge(self):
        plan, selection, path = self.fixture()
        plan_path=self.root/'plan.json'; plan_path.write_text(json.dumps(plan))
        with mock.patch.object(driver,'ROOT',self.root), mock.patch.object(driver,'SELECTION',path), \
             mock.patch.object(driver,'verify_plan',return_value=selection), \
             mock.patch.object(driver,'CONTROLLER_RESERVE',0.000001), \
             mock.patch.object(driver,'execute_attempt') as run:
            result=driver.execute(plan_path)
        run.assert_not_called()
        self.assertEqual(result['status'],'FAILED_BUDGET')
        rows,_=driver.cost.validate_event_chain(self.root/'COST_EVENTS.jsonl')
        self.assertEqual([r['phase'] for r in rows],['STARTED','TERMINAL'])
        self.assertGreater(rows[-1]['charged_seconds'],rows[0]['charge_upper_seconds'])


if __name__ == '__main__':
    unittest.main()
