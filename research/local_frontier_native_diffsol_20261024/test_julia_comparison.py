"""Synthetic review controls. No Julia executable or scientific solve is run."""
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

spec = importlib.util.spec_from_file_location('julia_comparison_under_test', Path(__file__).with_name('julia_comparison.py'))
jc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(jc)


class WrapperControls(unittest.TestCase):
    def setUp(self):
        evidence = os.environ.get('VIGILODE_TEST_EVIDENCE')
        if evidence:
            self.root = Path(evidence)/self._testMethodName
            self.root.mkdir(parents=True)
        else:
            self.temp = tempfile.TemporaryDirectory()
            self.addCleanup(self.temp.cleanup)
            self.root = Path(self.temp.name)
        self.cost = self.root/'global_costs'
        self.cost.mkdir()
        (self.cost/'COST_EVENTS.jsonl').write_text('')
        self.addCleanup(patch.stopall)
        patch.object(jc, 'GLOBAL_COST_ROOT', self.cost).start()
        self.git = {'commit': 'frozen', 'tree': 'tree', 'status_porcelain': ''}
        patch.object(jc, 'git_identity', return_value=self.git).start()
        self.files = {}
        for name in ('adapter', 'julia', 'binary', 'project', 'manifest', 'build_receipt', 'descriptor', 'reference'):
            path = self.root/name
            path.write_text(name+' immutable bytes\n')
            self.files[name] = {'path': str(path), 'sha256': jc.sha256_file(path)}
        self.plan = {'source': {'repo': str(self.root), 'commit': 'frozen', 'tree': 'tree'},
            'julia_path': self.files['julia']['path'], 'julia_sha256': self.files['julia']['sha256'],
            'adapter_path': self.files['adapter']['path'], 'project_path': self.files['project']['path'],
            'depot_path': str(self.root), 'julia_version': '1.12.7',
            'cost_root': str(self.cost), 'new_cap_seconds': jc.GLOBAL_CAP_SECONDS,
            'deadline_utc': jc.GLOBAL_DEADLINE,
            'limits': {'cpu': min(os.sched_getaffinity(0)), 'campaign_budget_seconds': 10,
                'setup_timeout_seconds': 2, 'attempt_timeout_seconds': 2,
                'idle_timeout_seconds': 1, 'max_rss_bytes': 1000000000, 'max_output_bytes': 10000}}
        self.case = {'case': 1, 'size': 'standard', 'dimension': 2, 't_end': 1.0,
            'reference_state': [1.0, 2.0], 'uncertainty_U': 1e-6,
            'reference_provenance': 'synthetic fixture', 'uncertainty_interpretation': 'synthetic only',
            'descriptor': self.files['descriptor'], 'reference_file': self.files['reference'],
            'rtol': 1e-6, 'atol': 1e-9, 'max_steps': 200000}
        self.verified = {name: self.files[name] for name in
            ('adapter', 'julia', 'binary', 'project', 'manifest', 'build_receipt')}
        self.verified.update(source=self.git, cases=[self.case])
        self.plan_path = self.root/'plan.json'
        self.plan_path.write_text(json.dumps(self.plan))

    def result(self, command, *args, **kwargs):
        if '--version' in command:
            stdout = b'julia version 1.12.7\n'
        elif '-e' in command:
            stdout = b'IMPORT_OK 1.12.7 BLAS_THREADS 1\n'
        else:
            stdout = (json.dumps({'kind': 'result', 'case': 1, 'size': 'standard',
                'method': command[-4], 'julia_version': '1.12.7', 'threads': 1,
                'parallel_layer': 'serial', 'solver_success': True, 'final_time': 1.0,
                'final_state': [1.0, 2.0], 'retcode': 'Success',
                'warmup_seconds': .002, 'elapsed_seconds': .003})+'\n').encode()
        return {'status': 'COMPLETED', 'exit_code': 0, 'stdout': stdout, 'stderr': b'',
                'resources': [], 'elapsed_seconds': .01, 'process_group_cpu_seconds': .005,
                'first_failure': None, 'process_started': False}

    def campaign(self, name='campaign', runner=None):
        with patch.object(jc, 'validate_plan', return_value=self.verified), \
             patch.object(jc, 'run_bounded', side_effect=runner or self.result):
            return jc.run_campaign(self.plan_path, self.cost/name)

    def test_selected_analytic_vector_is_bound_not_just_a_separate_copy(self):
        descriptor = {'kind': 'model_contract', 'case_id': 10, 'size': 'standard',
            'initial_state': [1.0, 0.0], 't_end': 1.0, 'exact_final': [1.0, 0.0],
            'model_contract': {'case': 10, 'dimension_standard': 2,
                               'reference_kind': 'analytic-semidiscrete-endpoint'}}
        reference = {'selected': [3.0, 0.0], 'descriptor_copy': [1.0, 0.0],
                     'U': 1e-6, 'provenance': 'synthetic analytic reference'}
        d, r = self.root/'analytic_descriptor.json', self.root/'analytic_reference.json'
        d.write_text(json.dumps(descriptor));r.write_text(json.dumps(reference))
        plan = {'case': 10, 'size': 'standard', 'descriptor_path': str(d),
                'descriptor_sha256': jc.sha256_file(d), 'rtol': 1e-6, 'atol': 1e-9,
                'max_steps': 1000, 'reference': {'path': str(r), 'sha256': jc.sha256_file(r),
                    'state_path': 'selected', 'uncertainty_path': 'U',
                    'provenance_path': 'provenance', 'uncertainty_interpretation': 'synthetic',
                    'descriptor_binding': {'kind': 'descriptor_exact_final', 'path': 'descriptor_copy'}}}
        with self.assertRaisesRegex(ValueError, 'selected reference state conflicts'):
            jc.resolve_case(plan)
        reference['selected'] = [1.0+5e-7, 0.0]
        r.write_text(json.dumps(reference));plan['reference']['sha256'] = jc.sha256_file(r)
        self.assertEqual(jc.resolve_case(plan)['reference_state'], reference['selected'])
        reference['U'] = 1.1e-4
        r.write_text(json.dumps(reference));plan['reference']['sha256'] = jc.sha256_file(r)
        with self.assertRaisesRegex(ValueError, 'uncertainty'):
            jc.resolve_case(plan)

    def test_input_mutation_keeps_raw_exit_terminal_receipt_and_current_group(self):
        def mutate(command, *args, **kwargs):
            row = self.result(command)
            if '--version' not in command and '-e' not in command:
                Path(self.files['adapter']['path']).write_text('changed during attempt')
            return row
        summary = self.campaign(runner=mutate)
        attempt = summary['groups'][0]['attempts'][0]
        self.assertEqual(attempt['scientific_status'], 'FAILED_INPUT_MUTATION')
        folder = self.cost/'campaign/attempts/case01_julia-tsit5/campaign_warmup_001'
        self.assertTrue(all((folder/name).exists() for name in ('stdout.raw','stderr.raw','EXIT.json','RECEIPT.json','PROCESS_COST.json')))
        self.assertFalse(attempt['input_hashes_after']['files']['adapter']['matches'])
        self.assertAlmostEqual(summary['observed_child_process_wall_seconds_total'], .03)
        self.assertGreaterEqual(summary['charged_seconds'], .03)

    def test_deleted_input_is_an_observation_not_a_receipt_aborting_exception(self):
        path = Path(self.files['adapter']['path']);path.unlink()
        observed = jc.capture_hashes({'adapter': self.files['adapter']})['adapter']
        self.assertFalse(observed['matches']);self.assertIsNone(observed['sha256'])
        self.assertIn('capture_error', observed)

    def test_julia_binary_mutation_is_in_before_after_attempt_locks(self):
        def mutate(command, *args, **kwargs):
            row = self.result(command)
            if '--version' not in command and '-e' not in command:
                Path(self.files['julia']['path']).write_text('mutated Julia binary')
            return row
        summary = self.campaign(runner=mutate)
        attempt = summary['groups'][0]['attempts'][0]
        self.assertTrue(attempt['input_hashes_before']['files']['julia']['matches'])
        self.assertFalse(attempt['input_hashes_after']['files']['julia']['matches'])
        self.assertEqual(attempt['scientific_status'], 'FAILED_INPUT_MUTATION')

    def test_prelaunch_julia_mutation_prevents_process_start_and_keeps_receipt(self):
        Path(self.files['julia']['path']).write_text('mutation before launch')
        with patch.object(jc, 'run_bounded') as runner:
            receipt = jc._attempt(self.root/'prelaunch', self.plan, {}, self.case,
                                  'julia-tsit5', 'timed', 1, time.monotonic())
            runner.assert_not_called()
        self.assertEqual(receipt['scientific_status'], 'FAILED_INPUT_MUTATION')
        self.assertEqual(receipt['adapter_process_wall_seconds'], 0)
        self.assertTrue((self.root/'prelaunch/RECEIPT.json').exists())

    def test_setup_crash_is_terminal_and_globally_charged(self):
        def crash(*args, **kwargs):
            raise RuntimeError('first setup collector crash')
        with self.assertRaisesRegex(RuntimeError, 'first setup collector crash'):
            self.campaign(runner=crash)
        summary = json.loads((self.cost/'campaign/SUMMARY.json').read_text())
        self.assertEqual(len(summary['setup_records']), 1)
        self.assertGreater(summary['setup_process_wall_seconds_total'], 0)
        self.assertIn('first setup collector crash', summary['first_failure']['message'])
        self.assertTrue((self.cost/'campaign/setup/version/RECEIPT.json').exists())
        self.assertGreater(jc._cost_module().external_costs(self.cost), 0)

    def test_current_group_crash_retains_setup_and_attempt_costs(self):
        def crash(command, *args, **kwargs):
            if '--version' in command or '-e' in command:
                return self.result(command)
            raise RuntimeError('current-group crash')
        summary = self.campaign(runner=crash)
        self.assertEqual(len(summary['groups']), 1)
        self.assertEqual(len(summary['groups'][0]['attempts']), 1)
        self.assertEqual(len(summary['process_records']), 3)
        self.assertGreaterEqual(summary['setup_process_wall_seconds_total'], .02)
        self.assertIn('current-group crash', summary['groups'][0]['attempts'][0]['first_failure']['message'])

    def test_fresh_invocation_cannot_bypass_an_unfinished_global_reservation(self):
        row = {'schema': 'vigilode.local-continuation-cost-event.v1', 'category': 'julia',
            'event_id': 'previous-start', 'attempt_id': 'previous-campaign', 'phase': 'STARTED',
            'charge_upper_seconds': jc.GLOBAL_CAP_SECONDS-30, 'prev_event_sha256': '0'*64}
        (self.cost/'COST_EVENTS.jsonl').write_text(json.dumps(row)+'\n')
        before = (self.cost/'COST_EVENTS.jsonl').read_bytes()
        with patch.object(jc, 'run_bounded') as runner:
            for name in ('fresh1','fresh2'):
                summary = jc.run_campaign(self.plan_path,self.cost/name)
                self.assertTrue(summary['budget_exhausted'])
                self.assertEqual(summary['process_records'], [])
            runner.assert_not_called()
        self.assertEqual((self.cost/'COST_EVENTS.jsonl').read_bytes(),before)

    def test_absolute_deadline_and_global_root_cannot_be_replaced(self):
        end = jc.dt.datetime.fromisoformat(jc.GLOBAL_DEADLINE.replace('Z','+00:00'))
        with patch.object(jc, 'utc_now', return_value=end), patch.object(jc, 'run_bounded') as runner:
            summary = jc.run_campaign(self.plan_path,self.cost/'expired')
            self.assertTrue(summary['budget_exhausted']);runner.assert_not_called()
        bad = dict(self.plan,cost_root=str(self.root))
        with self.assertRaisesRegex(ValueError,'existing global cost root'):
            jc.validate_global_binding(bad)

    def test_success_uses_existing_chain_schema_and_never_reports_speed_ratios(self):
        summary = self.campaign()
        self.assertEqual(len(summary['groups']),2)
        self.assertTrue(all(g['timed_passes']==5 for g in summary['groups']))
        self.assertEqual(summary['speed_comparison']['speed_ratios'],{})
        events = [json.loads(line) for line in (self.cost/'COST_EVENTS.jsonl').read_text().splitlines()]
        self.assertEqual([row['phase'] for row in events],['STARTED','TERMINAL'])
        self.assertEqual(events[1]['prev_event_sha256'],jc._canonical_sha(events[0]))
        self.assertEqual(jc._cost_module().external_costs(self.cost),summary['charged_seconds'])

    def test_spawn_observer_exception_kills_and_reaps_started_process(self):
        def fail(proc):
            raise RuntimeError('observer failed after spawn')
        run = jc.run_bounded([sys.executable,'-c','import time;time.sleep(10)'],os.environ.copy(),
            min(os.sched_getaffinity(0)),timeout_seconds=2,idle_timeout_seconds=1,
            max_rss_bytes=1000000000,max_output_bytes=10000,on_spawn=fail)
        self.assertEqual(run['status'],'FAILED_RUNNER_EXCEPTION')
        self.assertIn('observer failed after spawn',run['first_failure']['message'])
        with self.assertRaises(ProcessLookupError):
            os.killpg(run['pid'],0)

    def test_finite_state_difference_overflow_is_a_serializable_accuracy_failure(self):
        case = dict(self.case, reference_state=[-1e308,2.0])
        raw = json.loads(self.result(['julia-tsit5','1e-6','1e-9','100'])['stdout'])
        raw['final_state'] = [1e308,2.0]
        assessment = jc.assess_attempt(json.dumps(raw).encode(),'COMPLETED',0,case,'julia-tsit5','1.12.7')
        self.assertEqual(assessment['scientific_status'],'ERROR_CAP_FAILURE')
        json.dumps(assessment,allow_nan=False)


if __name__ == '__main__':
    unittest.main()
