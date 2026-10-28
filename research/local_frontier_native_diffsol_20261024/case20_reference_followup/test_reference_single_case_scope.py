"""Synthetic single-case and approval controls; never launches a Julia solve."""
import copy
import hashlib
import json
from pathlib import Path
import signal
import time
import unittest
from unittest.mock import patch

from test_reference_launch_controller import Fixture, c, dump, event, temporary


class SingleCaseScopeTests(unittest.TestCase):
    def tearDown(self):
        signal.setitimer(signal.ITIMER_REAL, 0)
        c.release_execution_lock()

    def test_each_independent_case_accepts_one_predeclared_task(self):
        for case in (8, 9, 11, 20):
            with self.subTest(case=case), temporary() as t:
                root = Path(t); fixture = Fixture(root)
                fixture.select_case(case)
                with fixture.mocks():
                    locks = c.verify_locks(fixture.plan, root, time.monotonic()+900)
                self.assertIn(f'descriptor_{case}', locks['locks'])

    def test_unapproved_single_case_change_refused_before_reservation(self):
        with temporary() as t:
            root = Path(t); fixture = Fixture(root)
            fixture.approve()
            fixture.select_case(9)
            with fixture.mocks(approve=False), patch.object(c, 'run_child') as child:
                with self.assertRaisesRegex(ValueError, 'approval'):
                    c.launch(fixture.path)
            child.assert_not_called()
            self.assertEqual((root/'COST_EVENTS.jsonl').read_bytes(), b'')
            self.assertFalse((root/'output').exists())

    def test_case20_requires_fresh_approval_and_matching_selection(self):
        for mutation in ('approval','sequence','script'):
            with self.subTest(mutation=mutation),temporary() as t:
                root=Path(t); fixture=Fixture(root); fixture.approve(); fixture.select_case(20)
                if mutation=='sequence':
                    fixture.plan['execution_policy']['sequence']=[8]
                elif mutation=='script':
                    fixture.plan['tasks'][0]['julia_script_path']=fixture.task_catalog[9]['julia_script_path']
                    fixture.plan['tasks'][0]['julia_script_sha256']=fixture.task_catalog[9]['julia_script_sha256']
                    fixture.approve()
                with fixture.mocks(approve=False),patch.object(c,'run_child') as child:
                    if mutation=='script':
                        result=c.launch(fixture.path)
                        self.assertEqual(result['status'],'FAILED_CONTROLLER')
                        self.assertIn('case-specific',result['error'])
                    else:
                        with self.assertRaisesRegex(ValueError,'approval|sequence'):
                            c.launch(fixture.path)
                child.assert_not_called()

    def test_first_failure_does_not_block_separately_approved_later_cases(self):
        with temporary() as t:
            root = Path(t); fixture = Fixture(root); calls = []
            def child(argv, *args, **kwargs):
                case = int(Path(argv[-3]).stem.split('_')[-1])
                calls.append(case)
                self.assertEqual(args[3], 900)
                output = Path(argv[-2]); output.mkdir()
                dump(output/'EXTERNAL_REFERENCE.json', {'synthetic_case':case})
                Path(args[8]).write_bytes(b'synthetic raw retained\n')
                return {'status':'FAILED_CHILD_EXIT' if case == 8 else 'COMPLETED',
                        'exit_code':7 if case == 8 else 0, 'elapsed_seconds':0.01}
            first_prefix = b''
            with patch.object(c, 'run_child', side_effect=child):
                for case in (8, 9, 11, 20):
                    fixture.select_case(case)
                    fixture.path = root/f'plan_case{case}.json'
                    fixture.approval_path = root/f'approval_case{case}.json'
                    fixture.ledger_path = root/f'ledger_case{case}.json'
                    fixture.plan['output_root'] = str(root/f'output_case{case}')
                    fixture.refresh_ledger()
                    with fixture.mocks():
                        result = c.launch(fixture.path)
                    self.assertEqual(result['status'], 'STOPPED_ON_FIRST_FAILURE' if case == 8 else 'COMPLETED_ALL_CHILDREN')
                    self.assertEqual([row['case'] for row in result['children']], [case])
                    start = json.loads((root/f'output_case{case}'/'CAMPAIGN_START.json').read_text())
                    self.assertEqual(start['selected_case'], case)
                    self.assertEqual(start['reserved_seconds'], 900)
                    self.assertEqual(start['plan_sha256'], hashlib.sha256(fixture.path.read_bytes()).hexdigest())
                    self.assertEqual(start['plan_core_sha256'], c.plan_core_sha(fixture.plan))
                    self.assertEqual(start['prior_charged_plus_reserved_seconds'],
                                     sum(row['charged_seconds'] for row in c.validate_event_chain(root/'COST_EVENTS.jsonl')[0][:-2] if row['phase'] == 'TERMINAL'))
                    raw = (root/'COST_EVENTS.jsonl').read_bytes()
                    self.assertTrue(raw.startswith(first_prefix))
                    first_prefix = raw
            self.assertEqual(calls, [8, 9, 11, 20])
            rows, _ = c.validate_event_chain(root/'COST_EVENTS.jsonl')
            self.assertEqual([row['phase'] for row in rows], ['STARTED', 'TERMINAL']*4)
            self.assertEqual([row['charge_upper_seconds'] for row in rows if row['phase'] == 'STARTED'], [900]*4)
            self.assertGreater(c.cumulative_cost(root), 0)
            for case in (8, 9, 11, 20):
                frozen = json.loads((root/f'plan_case{case}.json').read_text())
                approval = json.loads(Path(frozen['approval_receipt']['path']).read_text())
                self.assertEqual(approval['approved_plan_core_sha256'], c.plan_core_sha(frozen))
                self.assertEqual(c.sha_file(frozen['approval_receipt']['path']), frozen['approval_receipt']['sha256'])
            self.assertEqual((root/'output_case8'/'01_case8'/'stdout.raw').read_bytes(), b'synthetic raw retained\n')
            self.assertEqual(json.loads((root/'output_case8'/'01_case8'/'TERMINAL.json').read_text())['process']['exit_code'], 7)

    def test_multi_case_empty_duplicate_and_unapproved_case_refuse_before_reservation(self):
        for cases in ([], [8,9,11], [8,20], [8,8], [1], [21], [True]):
            with self.subTest(cases=cases), temporary() as t:
                root = Path(t); fixture = Fixture(root)
                fixture.approve()
                fixture.plan['tasks'] = [dict(fixture.task_catalog[8], case=case) for case in cases]
                fixture.plan['execution_policy']['sequence'] = cases
                with fixture.mocks(approve=False), patch.object(c, 'run_child') as child:
                    with self.assertRaisesRegex(ValueError, 'predeclared|must be case'):
                        c.launch(fixture.path)
                child.assert_not_called()
                self.assertEqual((root/'COST_EVENTS.jsonl').read_bytes(), b'')
                self.assertFalse((root/'output').exists())

    def test_approval_binds_all_plan_fields_except_approval_reference(self):
        with temporary() as t:
            root = Path(t); fixture = Fixture(root); fixture.approve()
            original = copy.deepcopy(fixture.plan)
            mutations = {
                'controller_sha256':'1'*64, 'cost_root':str(root/'other'),
                'new_48h_cap_seconds':172801, 'hard_deadline_utc':'2099-01-01T00:00:00Z',
                'execution_lock_path':str(root/'other.lock'), 'expected_cost_events_sha256':'1'*64,
                'source':dict(original['source'], head='other'),
                'project_sha256':'1'*64, 'manifest_sha256':'1'*64, 'julia_executable_sha256':'1'*64,
                'campaign_reservation_seconds':899, 'overall_hard_timeout_seconds':899,
                'child_hard_timeout_seconds':901, 'cpu':1, 'thread_count':2,
                'output_root':str(root/'other-output'), 'isolated_depot_path':str(root/'other-depot'),
                'tasks':[dict(original['tasks'][0], descriptor_sha256='1'*64)],
                'support_path':[dict(original['tasks'][0], sparse_support_path=str(root/'other'))],
                'support_sha':[dict(original['tasks'][0], sparse_support_sha256='1'*64)],
            }
            for key, value in mutations.items():
                fixture.plan = dict(copy.deepcopy(original), **{'tasks' if key in ('support_path','support_sha') else key:value})
                with self.subTest(field=key), fixture.mocks(approve=False), patch.object(c, 'run_child') as child:
                    with self.assertRaisesRegex(ValueError, 'approval'):
                        c.launch(fixture.path)
                child.assert_not_called()
            self.assertEqual((root/'COST_EVENTS.jsonl').read_bytes(), b'')
            self.assertFalse((root/c.EXECUTION_LOCK_NAME).exists())

    def test_approval_receipt_content_and_bytes_are_checked_before_execution_lock(self):
        for field, value in (('schema','wrong'), ('decision','HOLD'), ('selected_case',9),
                             ('approved_plan_core_sha256','0'*64)):
            with self.subTest(field=field), temporary() as t:
                root = Path(t); fixture = Fixture(root); fixture.approve()
                approval = json.loads((root/'approval.json').read_text())
                approval[field] = value
                fixture.plan['approval_receipt'] = dump(root/'approval.json', approval)
                with fixture.mocks(approve=False):
                    with self.assertRaisesRegex(ValueError, 'approval'):
                        c.launch(fixture.path)
                self.assertEqual((root/'COST_EVENTS.jsonl').read_bytes(), b'')
                self.assertFalse((root/c.EXECUTION_LOCK_NAME).exists())
        with temporary() as t:
            root = Path(t); fixture = Fixture(root); fixture.approve()
            (root/'approval.json').write_text('{}')
            with fixture.mocks(approve=False):
                with self.assertRaisesRegex(ValueError, 'SHA mismatch'):
                    c.launch(fixture.path)
            self.assertFalse((root/c.EXECUTION_LOCK_NAME).exists())

    def test_changed_cost_snapshot_requires_fresh_snapshot_and_approval(self):
        with temporary() as t:
            root = Path(t); fixture = Fixture(root); fixture.approve()
            c.append_event(root/'COST_EVENTS.jsonl', root/'COST_EVENTS.lock', event(root, 'prior', value=7))
            prefix = (root/'COST_EVENTS.jsonl').read_bytes()
            with fixture.mocks(approve=False):
                with self.assertRaisesRegex(RuntimeError, 'COST_EVENTS changed'):
                    c.launch(fixture.path)
            self.assertEqual((root/'COST_EVENTS.jsonl').read_bytes(), prefix)
            fixture.refresh_ledger()
            with fixture.mocks(approve=False):
                with self.assertRaisesRegex(ValueError, 'approval'):
                    c.launch(fixture.path)
            fixture.approve()
            with fixture.mocks(approve=False), patch.object(c, 'verify_locks', side_effect=RuntimeError('synthetic preflight stop')):
                result = c.launch(fixture.path)
            self.assertEqual(result['status'], 'FAILED_CONTROLLER')
            self.assertTrue((root/'COST_EVENTS.jsonl').read_bytes().startswith(prefix))
            self.assertEqual(json.loads((root/'output'/'CAMPAIGN_START.json').read_text())['prior_charged_plus_reserved_seconds'], 7)


if __name__ == '__main__':
    unittest.main(verbosity=2)
