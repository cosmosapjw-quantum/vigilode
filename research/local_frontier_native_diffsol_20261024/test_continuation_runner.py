"""Mechanical negative controls for the continuation watchdog and identity lock."""
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock

import continuation_runner as runner


def task(command):
    return {'id': 'task001', 'command': command}


class RunnerTests(unittest.TestCase):
    def make_plan(self, root):
        repo = root/'repo'; repo.mkdir()
        subprocess.run(['git', '-C', str(repo), 'init', '-q'], check=True)
        (repo/'f').write_text('frozen\n')
        subprocess.run(['git', '-C', str(repo), 'add', 'f'], check=True)
        subprocess.run(['git', '-C', str(repo), '-c', 'user.name=Test',
                        '-c', 'user.email=t@example.invalid', 'commit', '-qm', 'freeze'], check=True)
        identity = runner.git_identity(repo)
        binary = root/'binary'
        binary.write_text('#!/usr/bin/env python3\nimport json\nprint(json.dumps({"kind":"result"}))\n')
        binary.chmod(0o755)
        receipt = root/'old-receipt'; receipt.write_text('{"task":{"case":1}}')
        (root/'COST_EVENTS.jsonl').write_text('')
        plan = {'schema': 'vigilode.local-continuation-runner.v1',
                'deadline_utc': runner.DEADLINE, 'new_cap_seconds': runner.NEW_CAP_SECONDS,
                'cost_root': str(root), 'source': {'frozen': {'path': str(repo), **identity}},
                'tasks': [{'id': 'task001', 'command': [str(binary)], 'binary': str(binary),
                           'original_task': {'case': 1}, 'binary_sha256': runner.digest(binary),
                           'original_receipt': str(receipt),
                           'original_receipt_sha256': runner.digest(receipt)}]}
        plan_path = root/'PLAN.json'; plan_path.write_text(json.dumps(plan))
        return plan, plan_path, repo, receipt

    def test_diagnostic_timeout_without_physical_progress(self):
        with tempfile.TemporaryDirectory() as directory:
            with mock.patch.object(runner, 'DIAGNOSTIC_SECONDS', 0.15), \
                 mock.patch.object(runner, 'MAX_SECONDS', 0.45), \
                 mock.patch.object(runner, 'IDLE_SECONDS', 2):
                result = runner.run_one(task([sys.executable, '-c', 'import time; time.sleep(5)']),
                                        Path(directory)/'attempt', poll_seconds=0.025)
            self.assertEqual(result['status'], 'FAILED_TIMEOUT_DIAGNOSTIC')
            self.assertEqual(result['physical_progress']['status'], 'UNKNOWN')
            self.assertFalse(result['diagnostic_extended'])
            self.assertGreater(result['charged_new_seconds'], 0)

    def test_absolute_deadline_kills_child(self):
        with tempfile.TemporaryDirectory() as directory:
            deadline = dt.datetime.now(dt.timezone.utc)+dt.timedelta(seconds=0.15)
            with mock.patch.object(runner, 'deadline_time', return_value=deadline), \
                 mock.patch.object(runner, 'DIAGNOSTIC_SECONDS', 3), \
                 mock.patch.object(runner, 'MAX_SECONDS', 5):
                result = runner.run_one(task([sys.executable, '-c', 'import time; time.sleep(5)']),
                                        Path(directory)/'attempt', poll_seconds=0.025)
            self.assertEqual(result['status'], 'FAILED_DEADLINE')

    def test_process_group_descendant_is_killed(self):
        with tempfile.TemporaryDirectory() as directory:
            code = ('import subprocess,sys,time,json; '
                    'p=subprocess.Popen([sys.executable,"-c","import time;time.sleep(30)"]); '
                    'print(json.dumps({"child_pid":p.pid}),flush=True); time.sleep(30)')
            with mock.patch.object(runner, 'DIAGNOSTIC_SECONDS', 1.0), \
                 mock.patch.object(runner, 'MAX_SECONDS', 1.3), \
                 mock.patch.object(runner, 'IDLE_SECONDS', 2):
                root = Path(directory)/'attempt'
                result = runner.run_one(task([sys.executable, '-c', code]), root, poll_seconds=0.025)
            self.assertEqual(result['status'], 'FAILED_TIMEOUT_DIAGNOSTIC')
            child = json.loads((root/'stdout.jsonl').read_text().splitlines()[0])['child_pid']
            for _ in range(20):
                stat = Path('/proc')/str(child)/'stat'
                if not stat.exists() or stat.read_text().split(') ', 1)[1][0] == 'Z':
                    break
                time.sleep(0.025)
            else:
                self.fail('descendant remained live after process-group kill')

    def test_source_and_original_receipt_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            plan, _, repo, receipt = self.make_plan(root)
            runner.validate_plan(plan)
            receipt.write_text('changed')
            with self.assertRaisesRegex(ValueError, 'original receipt identity mismatch'):
                runner.validate_plan(plan)
            receipt.write_text('{"task":{"case":1}}')
            (repo/'f').write_text('dirty\n')
            with self.assertRaisesRegex(ValueError, 'source identity mismatch'):
                runner.validate_plan(plan)

    def test_extension_requires_observed_increasing_steps(self):
        progress = {'status': 'OBSERVED', 'accepted_steps': 50, 'target_steps': 100}
        self.assertFalse(runner.projected_extension(progress, [], 300))
        samples = [{'elapsed_seconds': 100, 'physical_progress':
                    {'status': 'OBSERVED', 'accepted_steps': 20, 'target_steps': 100}},
                   {'elapsed_seconds': 290, 'physical_progress': progress}]
        self.assertTrue(runner.projected_extension(progress, samples, 300))
        self.assertFalse(runner.projected_extension({'status': 'UNKNOWN'}, samples, 300))

    def test_reference_mutation_is_terminal_and_hashes_retained(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root/'fake_solver'
            binary.write_text('#!/usr/bin/env python3\n'
                              'import json,sys\n'
                              'if "--describe" in sys.argv:\n'
                              ' print(json.dumps({"case":1}))\n'
                              'else:\n'
                              ' p=sys.argv[sys.argv.index("--reference-json")+1]\n'
                              ' open(p,"w").write("{}")\n'
                              ' print(json.dumps({"kind":"result"}))\n')
            binary.chmod(0o755)
            descriptor = subprocess.check_output([str(binary), '--describe', '--case', '1',
                                                  '--size', 'standard'])
            reference = root/'reference.json'
            reference.write_text(json.dumps({'schema': 'vigilode.local-bottleneck.external-reference.v1',
                                             'model_descriptor': {'case': 1},
                                             'model_descriptor_sha256': hashlib.sha256(descriptor).hexdigest()}))
            row = task([str(binary)])
            row.update(binary=str(binary), case=1, size='standard', reference_json=str(reference),
                       reference_sha256=runner.digest(reference))
            result = runner.run_one(row, root/'attempt', poll_seconds=0.025)
            self.assertEqual(result['status'], 'FAILED_REFERENCE_CHANGED')
            self.assertNotEqual(result['reference_sha256_before'], result['reference_sha256_after'])
            self.assertEqual(result['descriptor_sha256_before'], hashlib.sha256(descriptor).hexdigest())
            self.assertEqual(result['descriptor_sha256_after'], hashlib.sha256(descriptor).hexdigest())

    def test_unfinished_attempt_blocks_relaunch_before_output_mutation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            _, plan_path, _, _ = self.make_plan(root)
            attempt = root/'campaign'/'runs'/'task001'/'attempt_001'
            attempt.mkdir(parents=True)
            (attempt/'START.json').write_text(json.dumps({'task_id': 'task001'}))
            with self.assertRaisesRegex(ValueError, 'unfinished attempt'):
                runner.run_plan(plan_path, root/'campaign', retry_failed=True)
            self.assertFalse((root/'campaign'/'CAMPAIGN_IDENTITY.json').exists())
            self.assertFalse((root/'campaign'/'runs'/'task001'/'attempt_002').exists())

    def test_failed_attempt_needs_exact_retry_authorization(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            _, plan_path, _, _ = self.make_plan(root)
            attempt = root/'campaign'/'runs'/'task001'/'attempt_001'
            attempt.mkdir(parents=True)
            (attempt/'START.json').write_text(json.dumps({'task_id': 'task001'}))
            (attempt/'RECEIPT.json').write_text(json.dumps({'status': 'FAILED_TIMEOUT_DIAGNOSTIC',
                                                          'charged_new_seconds': 0.1}))
            with self.assertRaisesRegex(ValueError, 'missing hash-bound retry authorization'):
                runner.run_plan(plan_path, root/'campaign', retry_failed=True)
            self.assertFalse((root/'campaign'/'runs'/'task001'/'attempt_002').exists())

    def test_separate_retry_manifest_preserves_campaign_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            plan, plan_path, _, _ = self.make_plan(root)
            campaign = root/'campaign'
            runner.run_plan(plan_path, campaign)
            identity_before = (campaign/'CAMPAIGN_IDENTITY.json').read_bytes()
            first = campaign/'runs'/'task001'/'attempt_001'
            receipt = first/'RECEIPT.json'
            original = json.loads(receipt.read_text())
            original['status'] = 'FAILED_TIMEOUT_DIAGNOSTIC'
            receipt.write_text(json.dumps(original))
            auth = {'decision': 'APPROVED_TARGETED_RETRY', 'task_id': 'task001',
                    'task_sha256': runner.canonical_sha(plan['tasks'][0]),
                    'source_sha256': runner.canonical_sha(plan['source']),
                    'reference_sha256': None, 'prior_attempt_sha256': runner.digest(receipt),
                    'prior_status': 'FAILED_TIMEOUT_DIAGNOSTIC',
                    'next_attempt_number': 2, 'max_attempts': 2}
            auth_path = root/'RETRY_APPROVAL_001.json'
            auth_path.write_text(json.dumps(auth))
            manifest_path = root/'RETRY_MANIFEST_001.json'
            manifest_path.write_text(json.dumps({
                'schema': 'vigilode.local-continuation-retry-manifest.v1',
                'plan_sha256': runner.digest(plan_path),
                'authorizations': {'task001': {'path': str(auth_path),
                                                'sha256': runner.digest(auth_path)}}}))
            runner.run_plan(plan_path, campaign, retry_failed=True,
                            retry_authorizations=manifest_path)
            self.assertEqual((campaign/'CAMPAIGN_IDENTITY.json').read_bytes(), identity_before)
            second = campaign/'runs'/'task001'/'attempt_002'
            self.assertEqual(json.loads((second/'RECEIPT.json').read_text())['status'], 'COMPLETED')
            self.assertEqual((second/'RETRY_AUTHORIZATION.json').read_bytes(), auth_path.read_bytes())
            self.assertEqual((second/'RETRY_MANIFEST.json').read_bytes(), manifest_path.read_bytes())
            self.assertEqual(json.loads((second/'START.json').read_text())['plan_sha256'], runner.digest(plan_path))
            self.assertEqual(len(list((campaign/'runs'/'task001').glob('attempt_*'))), 2)

    def test_retry_manifest_rejects_stale_plan_and_changed_authorization(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            plan, plan_path, _, _ = self.make_plan(root)
            auth_path = root/'auth.json'; auth_path.write_text('{}')
            manifest_path = root/'manifest.json'
            manifest_path.write_text(json.dumps({
                'schema': 'vigilode.local-continuation-retry-manifest.v1',
                'plan_sha256': '0'*64,
                'authorizations': {'task001': {'path': str(auth_path),
                                                'sha256': runner.digest(auth_path)}}}))
            with self.assertRaisesRegex(ValueError, 'retry manifest plan identity mismatch'):
                runner.run_plan(plan_path, root/'campaign', retry_failed=True,
                                retry_authorizations=manifest_path)
            self.assertFalse((root/'campaign').exists())
            plan['retry_authorizations'] = {}
            with self.assertRaisesRegex(ValueError, 'separate immutable artifact'):
                runner.validate_plan(plan)

    def test_same_plan_cannot_relaunch_under_different_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            _, plan_path, _, _ = self.make_plan(root)
            first = root/'campaign_a'
            runner.run_plan(plan_path, first)
            self.assertTrue((first/'runs'/'task001'/'attempt_001'/'RECEIPT.json').exists())
            with self.assertRaisesRegex(ValueError, 'campaign output identity mismatch'):
                runner.run_plan(plan_path, root/'campaign_b')
            self.assertFalse((root/'campaign_b').exists())
            binding = json.loads((root/'CAMPAIGN_OUTPUT_IDENTITY.json').read_text())
            self.assertEqual(binding['output'], str(first.resolve()))

    def test_external_cost_reserve_prevents_new_attempt(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            _, plan_path, _, _ = self.make_plan(root)
            event = {'schema': 'vigilode.local-continuation-cost-event.v1',
                     'prev_event_sha256': '0'*64, 'event_id': 'build-start',
                     'attempt_id': 'build-1', 'phase': 'STARTED', 'category': 'build',
                     'charge_upper_seconds': runner.NEW_CAP_SECONDS-100}
            (root/'COST_EVENTS.jsonl').write_text(json.dumps(event)+'\n')
            self.assertEqual(runner.external_costs(root), runner.NEW_CAP_SECONDS-100)
            runner.run_plan(plan_path, root/'campaign')
            self.assertFalse((root/'campaign'/'runs').exists())


if __name__ == '__main__':
    unittest.main()
