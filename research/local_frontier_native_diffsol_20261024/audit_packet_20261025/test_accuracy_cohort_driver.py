"""Synthetic contract tests; never launch the VigilODE solver."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

import accuracy_cohort_driver as driver


class AccuracyDriverSyntheticTests(unittest.TestCase):
    def test_v2_selection_cpu0_and_bounded(self):
        selection = json.loads(driver.SELECTION.read_text())
        self.assertEqual(len(selection['groups']), 12)
        self.assertEqual(len(selection['attempts']), 72)
        self.assertEqual(selection['optional_two_thread_arm']['status'],
                         'NOT_RUN_PRUNED_SMALL_COHORT')
        for a in selection['attempts']:
            self.assertEqual(a['cpus'], [0])
            self.assertEqual(a['threads'], 1)
            self.assertEqual(a['hard_timeout_seconds'], 180)
            self.assertIn('--emit-state', a['argv'])
            self.assertEqual(a['argv'][a['argv'].index('--error-limit')+1], '1e-4')

    def test_all_member_cap_counts_uncertainty_once(self):
        with tempfile.TemporaryDirectory() as tmp:
            stdout = Path(tmp)/'stdout'
            ref = {'sha256': 'a'*64, 'descriptor_sha256': 'b'*64, 'uncertainty': 1e-12}
            row = {'kind': 'result', 'case': 2, 'ensemble_count': 256,
                   'member_successes': 256, 'solver_success': True,
                   'accuracy_scope': 'maximum component absolute endpoint error across every ensemble member; no continuous/phase-global certificate',
                   'reference_sha256': ref['sha256'],
                   'model_descriptor_sha256': ref['descriptor_sha256'],
                   'actual_error': 1e-4, 'endpoint_reference_difference': 1e-4-1e-12,
                   'reference_absolute_bound': 1e-12,
                   'maximum_error_member_index': 255, 'error_limit': 1e-4,
                   'final_state': [0.0], 'status': 'PASS_SCOPED'}
            stdout.write_text(json.dumps(row)+'\n')
            self.assertEqual(driver.assess(stdout, 2, ref)['status'], 'PASS_SCOPED')
            row['member_successes'] = 255
            stdout.write_text(json.dumps(row)+'\n')
            self.assertEqual(driver.assess(stdout, 2, ref)['status'], 'OUTPUT_CONTRACT_FAILURE')

    def test_typed_solver_failure_retained(self):
        with tempfile.TemporaryDirectory() as tmp:
            stdout = Path(tmp)/'stdout'
            ref = {'sha256': 'a'*64, 'descriptor_sha256': 'b'*64, 'uncertainty': 1e-12}
            stdout.write_text(json.dumps({'kind':'result','case':5,'ensemble_count':256,
                'reference_sha256':ref['sha256'],'model_descriptor_sha256':ref['descriptor_sha256'],
                'status':'SOLVER_FAILED','solver_success':False,'member_successes':0})+'\n')
            self.assertEqual(driver.assess(stdout, 5, ref)['status'], 'SOLVER_FAILED')

    def test_mutation_prevents_attempt_and_incomplete_resume_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            binary = root/'fake-binary'; binary.write_bytes(b'original')
            lock = root/'Cargo.lock'; lock.write_bytes(b'original-lock')
            reference = root/'reference.json'; reference.write_bytes(b'original-reference')
            source = {'path':str(root),'binary_path':str(binary),
                      'binary_sha256':driver.sha(binary),'cargo_lock_sha256':driver.sha(lock)}
            reference_binding = {'path':str(reference),'sha256':driver.sha(reference)}
            reference.write_bytes(b'mutated-reference')
            folder = root/'runs'/'attempt'
            with self.assertRaisesRegex(ValueError, 'immutable input mutation'):
                driver.execute_attempt({'attempt_id':'attempt'}, {'source':source},
                                       reference_binding, folder)
            self.assertFalse(folder.exists())
            folder.mkdir(parents=True)
            driver.new_json(folder/'START.json', {'attempt_id':'attempt'})
            with self.assertRaisesRegex(ValueError, 'unfinished attempt'):
                driver.read_terminal(folder)
            terminal = {'attempt_id':'attempt','assessment':{'status':'SOLVER_FAILED'},
                        'execution_status':'COMPLETED'}
            driver.new_json(folder/'TERMINAL.json', terminal)
            self.assertEqual(driver.read_terminal(folder), terminal)

    def test_approval_core_hash_binds_plan(self):
        plan = {'schema':driver.SCHEMA,'source':{'head':'a'},
                'approval_receipt':{'path':'later','sha256':'0'*64}}
        before = driver.plan_core_sha(plan)
        plan['approval_receipt']['sha256'] = '1'*64
        self.assertEqual(driver.plan_core_sha(plan), before)
        plan['source']['head'] = 'b'
        self.assertNotEqual(driver.plan_core_sha(plan), before)


if __name__ == '__main__':
    unittest.main()
