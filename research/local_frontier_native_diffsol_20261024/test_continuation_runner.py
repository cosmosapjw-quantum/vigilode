"""Mechanical negative controls for the continuation watchdog and identity lock."""
import contextlib
import copy
import datetime as dt
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import shutil
import tempfile
import time
import unittest
from unittest import mock

import continuation_runner as runner


def task(command):
    return {'id': 'task001', 'command': command, 'cpus': [0], 'threads': 1}


_CONTROL_INDEX = 0


@contextlib.contextmanager
def temporary():
    global _CONTROL_INDEX
    with tempfile.TemporaryDirectory() as directory:
        try:
            # Synthetic fixtures replace only the fixed campaign root locally.
            with mock.patch.object(runner, 'COST_ROOT', Path(directory)):
                yield directory
        finally:
            target = os.environ.get('CONTINUATION_TEST_EVIDENCE')
            if target:
                _CONTROL_INDEX += 1
                parent = Path(target); parent.mkdir(parents=True, exist_ok=True)
                shutil.copytree(directory, parent/f'fixture_{_CONTROL_INDEX:03d}')


class RunnerTests(unittest.TestCase):
    def setUp(self):
        # Real contract remains 5s. Tiny synthetic children need no five-second wait.
        self.real_run_one = runner.run_one
        def synthetic_poll(*args, **kwargs):
            kwargs.setdefault('poll_seconds', 0.025)
            return self.real_run_one(*args, **kwargs)
        self.poll_patch = mock.patch.object(runner, 'run_one', side_effect=synthetic_poll)
        self.poll_patch.start()
        self.addCleanup(self.poll_patch.stop)

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
        receipt = root/'old-receipt'; receipt.write_text(json.dumps({'task':{'case':1,'cpus':[0],'threads':1}}))
        (root/'COST_EVENTS.jsonl').write_text('')
        plan = {'schema': 'vigilode.local-continuation-runner.v1',
                'deadline_utc': runner.DEADLINE, 'new_cap_seconds': runner.NEW_CAP_SECONDS,
                'cost_root': str(root), 'source': {'frozen': {'path': str(repo), **identity}},
                'tasks': [{'id': 'task001', 'command': [str(binary)], 'binary': str(binary),
                           'original_task': {'case': 1, 'cpus': [0], 'threads': 1}, 'binary_sha256': runner.digest(binary),
                           'original_receipt': str(receipt),
                           'original_receipt_sha256': runner.digest(receipt)}]}
        plan_path = root/'PLAN.json'; plan_path.write_text(json.dumps(plan))
        return plan, plan_path, repo, receipt

    def test_default_cost_root_is_original_cumulative_campaign(self):
        self.assertEqual(runner.COST_ROOT, Path(
            '/tmp/vigilode-local-run-20261024.geLVCp9Q/continuation_20261025T080718Z'))

    def test_alternate_cost_root_refused_before_lock_output_or_child(self):
        with temporary() as directory:
            root = Path(directory)
            plan, path, _, _ = self.make_plan(root)
            alternate = root/'alternate'
            alternate.mkdir()
            external = alternate/'COST_EVENTS.jsonl'
            external.write_text('')  # A fresh budget must never be accepted.
            plan['cost_root'] = str(alternate)
            path.write_text(json.dumps(plan))
            frozen = {p: p.read_bytes() for p in root.rglob('*') if p.is_file()}
            paths_before = set(root.rglob('*'))
            with mock.patch.object(runner.subprocess, 'Popen') as child:
                with self.assertRaisesRegex(ValueError, 'canonical new-cost root'):
                    runner.validate_plan(plan)
                with self.assertRaisesRegex(ValueError, 'canonical new-cost root'):
                    runner.run_plan(path, alternate/'campaign')
                child.assert_not_called()
            self.assertEqual(set(root.rglob('*')), paths_before)
            for frozen_path, raw in frozen.items():
                self.assertEqual(frozen_path.read_bytes(), raw)
            self.assertFalse((root/'.julia_comparison_campaign.lock').exists())
            self.assertFalse((alternate/'.julia_comparison_campaign.lock').exists())
            self.assertFalse((alternate/'CAMPAIGN_OUTPUT_IDENTITY.json').exists())

    def test_diagnostic_timeout_without_physical_progress(self):
        with temporary() as directory:
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
        with temporary() as directory:
            deadline = dt.datetime.now(dt.timezone.utc)+dt.timedelta(seconds=0.15)
            with mock.patch.object(runner, 'deadline_time', return_value=deadline), \
                 mock.patch.object(runner, 'DIAGNOSTIC_SECONDS', 3), \
                 mock.patch.object(runner, 'MAX_SECONDS', 5):
                result = runner.run_one(task([sys.executable, '-c', 'import time; time.sleep(5)']),
                                        Path(directory)/'attempt', poll_seconds=0.025)
            self.assertEqual(result['status'], 'FAILED_DEADLINE')

    def test_process_group_descendant_is_killed(self):
        with temporary() as directory:
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
        with temporary() as directory:
            root = Path(directory)
            plan, _, repo, receipt = self.make_plan(root)
            runner.validate_plan(plan)
            receipt.write_text('changed')
            with self.assertRaisesRegex(ValueError, 'original receipt identity mismatch'):
                runner.validate_plan(plan)
            receipt.write_text(json.dumps({'task':{'case':1,'cpus':[0],'threads':1}}))
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
        with temporary() as directory:
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
        with temporary() as directory:
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
        with temporary() as directory:
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
        with temporary() as directory:
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
        with temporary() as directory:
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
        with temporary() as directory:
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
        with temporary() as directory:
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


    def test_wrong_or_missing_cpu_refuses_before_spawn(self):
        for binding in ([1], [0, 1], [], None, [False]):
            with self.subTest(cpus=binding), temporary() as directory:
                row = task([sys.executable, '-c', 'print("unexpected child")'])
                row['cpus'] = binding
                folder = Path(directory)/'attempt'
                with mock.patch.object(runner.subprocess, 'Popen') as spawn:
                    with self.assertRaisesRegex(ValueError, 'CPU'):
                        runner.run_one(row, folder)
                spawn.assert_not_called()
                self.assertFalse(folder.exists())

    def test_no_affinity_api_or_cpu_permission_refuses(self):
        with temporary() as directory:
            row = task([sys.executable, '-c', 'print("unexpected child")'])
            for api in ('sched_setaffinity', 'sched_getaffinity'):
                with self.subTest(api=api), mock.patch.object(runner.os, api, None), \
                     mock.patch.object(runner.subprocess, 'Popen') as spawn:
                    with self.assertRaisesRegex(ValueError, 'affinity'):
                        runner.run_one(row, Path(directory)/api)
                    spawn.assert_not_called()
            with mock.patch.object(runner.os, 'sched_getaffinity', return_value={1}), \
                 mock.patch.object(runner.subprocess, 'Popen') as spawn:
                with self.assertRaisesRegex(ValueError, 'CPU0'):
                    runner.run_one(row, Path(directory)/'disallowed')
                spawn.assert_not_called()

    def test_preexec_affinity_failure_never_executes_binary(self):
        with temporary() as directory:
            root = Path(directory); marker=root/'unexpected'
            row=task([sys.executable,'-c',f'open({str(marker)!r},"w").write("executed")'])
            with mock.patch.object(runner.os, 'sched_setaffinity', side_effect=PermissionError('synthetic affinity denial')):
                result=runner.run_one(row,root/'attempt')
            self.assertEqual(result['status'],'FAILED_SPAWN')
            self.assertFalse(marker.exists())
            self.assertIsNone(result['observed_affinity_before_exec'])

    def test_child_observed_cpu_mismatch_never_executes_binary(self):
        with temporary() as directory:
            root = Path(directory)
            marker = root/'unexpected'
            row = task([sys.executable, '-c', f'open({str(marker)!r},"w").write("executed")'])
            # CPU0 remains available to the parent; the child's readback is wrong.
            with mock.patch.object(runner.os, 'sched_getaffinity', return_value={0, 1}):
                result = runner.run_one(row, root/'attempt')
            self.assertEqual(result['status'], 'FAILED_SPAWN')
            self.assertFalse(marker.exists())
            self.assertIsNone(result['observed_affinity_before_exec'])

    def test_observed_child_affinity_threads_and_argv_preserved(self):
        with temporary() as directory:
            root=Path(directory)
            code=('import os,json,sys; print(json.dumps({"kind":"result",'
                  '"affinity":sorted(os.sched_getaffinity(0)),"argv":sys.argv,'
                  '"thread_env":{k:os.environ.get(k) for k in ("RAYON_NUM_THREADS","OMP_NUM_THREADS")}}))')
            command=[sys.executable,'-c',code,'literal argument']
            result=runner.run_one(task(command),root/'attempt')
            self.assertEqual(result['status'],'COMPLETED')
            row=json.loads((root/'attempt'/'stdout.jsonl').read_text())
            self.assertEqual(row['affinity'],[0])
            self.assertEqual(row['argv'],['-c','literal argument'])
            self.assertEqual(row['thread_env'],{'RAYON_NUM_THREADS':'1','OMP_NUM_THREADS':'1'})
            start=json.loads((root/'attempt'/'START.json').read_text())
            self.assertEqual(start['command'],command)
            audit=result['observed_affinity_before_exec']
            self.assertEqual(audit['cpu_affinity'],[0])
            self.assertEqual(audit['os_thread_count'],1)
            self.assertTrue(audit['observed_in_child_before_exec'])
            self.assertEqual(result['configured_threads'],1)
            process=json.loads((root/'attempt'/'PROCESS.json').read_text())
            self.assertEqual(process['observed_affinity_before_exec'],audit)
            self.assertGreaterEqual(result['observed_process_resources']['os_thread_count'],1)

    def test_thread_or_original_cpu_conflicts_refuse(self):
        for update in ({'threads':2}, {'environment':{'RAYON_NUM_THREADS':'2'}},
                       {'original_task':{'cpus':[1],'threads':1}},
                       {'command':[sys.executable,'--threads','2']}):
            with self.subTest(update=update),temporary() as directory:
                row=task([sys.executable,'-c','print(1)']); row.update(update)
                with mock.patch.object(runner.subprocess,'Popen') as spawn:
                    with self.assertRaisesRegex(ValueError,'CPU|thread'):
                        runner.run_one(row,Path(directory)/'attempt')
                    spawn.assert_not_called()

    def test_shared_mutex_contends_with_other_controller(self):
        with temporary() as directory:
            root=Path(directory); _,plan_path,_,_=self.make_plan(root)
            lock=root/'.julia_comparison_campaign.lock'
            with lock.open('a+') as other:
                fcntl.flock(other,fcntl.LOCK_EX|fcntl.LOCK_NB)
                with self.assertRaisesRegex(RuntimeError,'execution lock'):
                    runner.run_plan(plan_path,root/'campaign')
            self.assertFalse((root/'campaign').exists())
            self.assertFalse((root/'CAMPAIGN_OUTPUT_IDENTITY.json').exists())

    def test_shared_mutex_held_during_preflight_and_child(self):
        with temporary() as directory:
            root=Path(directory); _,plan_path,_,_=self.make_plan(root); phases=[]
            def prove(phase):
                with (root/'.julia_comparison_campaign.lock').open('a+') as other:
                    with self.assertRaises(BlockingIOError):
                        fcntl.flock(other,fcntl.LOCK_EX|fcntl.LOCK_NB)
                phases.append(phase)
            original=runner.validate_plan
            def validate(plan): prove('preflight'); return original(plan)
            def child(*args,**kwargs):
                prove('child'); kwargs['poll_seconds']=.025
                return self.real_run_one(*args,**kwargs)
            original_write = runner.write_new
            def write(path, value):
                if Path(path).name == 'RECEIPT.json':
                    prove('terminal receipt')
                if Path(path).name.startswith('EXECUTION_'):
                    prove('execution summary')
                return original_write(path, value)
            with mock.patch.object(runner,'validate_plan',side_effect=validate), \
                 mock.patch.object(runner,'run_one',side_effect=child), \
                 mock.patch.object(runner,'write_new',side_effect=write):
                runner.run_plan(plan_path,root/'campaign')
            self.assertIn('preflight',phases);self.assertIn('child',phases)
            self.assertIn('terminal receipt', phases)
            self.assertIn('execution summary', phases)
            with (root/'.julia_comparison_campaign.lock').open('a+') as other:
                fcntl.flock(other,fcntl.LOCK_EX|fcntl.LOCK_NB)

    def test_one_new_attempt_per_invocation_preserves_plan_output_and_cost(self):
        with temporary() as directory:
            root=Path(directory); plan,path,_,_=self.make_plan(root)
            plan['tasks']=[dict(copy.deepcopy(plan['tasks'][0]),id=f'task{i:03d}') for i in range(1,10)]
            path.write_text(json.dumps(plan)); plan_bytes=path.read_bytes(); campaign=root/'campaign'
            old_raw=(root/'old-receipt').read_bytes(); first_bytes=None; previous=0
            binding=None
            for i in range(1,10):
                results=runner.run_plan(path,campaign)
                self.assertEqual(len(results),1)
                self.assertEqual(results[0]['task_id'],f'task{i:03d}')
                self.assertEqual(len(list(campaign.glob('runs/*/attempt_*/START.json'))),i)
                current=runner.cumulative_new_charge(root); self.assertGreater(current,previous); previous=current
                self.assertEqual(path.read_bytes(),plan_bytes)
                self.assertEqual((root/'old-receipt').read_bytes(),old_raw)
                if i==1:
                    first_bytes=(campaign/'runs'/'task001'/'attempt_001'/'RECEIPT.json').read_bytes()
                    binding=(root/'CAMPAIGN_OUTPUT_IDENTITY.json').read_bytes()
                self.assertEqual((campaign/'runs'/'task001'/'attempt_001'/'RECEIPT.json').read_bytes(),first_bytes)
                self.assertEqual((root/'CAMPAIGN_OUTPUT_IDENTITY.json').read_bytes(),binding)
            self.assertEqual(runner.run_plan(path,campaign),[])
            self.assertEqual(len(list(campaign.glob('runs/*/attempt_*/START.json'))),9)
            self.assertEqual(len(list(campaign.glob('controller/invocation_*/RECEIPT.json'))),10)

    def test_first_failure_and_external_cost_continuity_across_invocations(self):
        with temporary() as directory:
            root = Path(directory)
            plan, path, _, _ = self.make_plan(root)
            binary = Path(plan['tasks'][0]['binary'])
            binary.write_text('#!/usr/bin/env python3\nimport json,sys\n'
                              'print(json.dumps({"kind":"result"}))\n'
                              'sys.exit(7 if "--fail" in sys.argv else 0)\n')
            template = dict(plan['tasks'][0], binary_sha256=runner.digest(binary))
            plan['tasks'] = [dict(copy.deepcopy(template), id=f'task{i:03d}') for i in (1, 2)]
            plan['tasks'][0]['command'].append('--fail')
            path.write_text(json.dumps(plan))
            event = {'schema': 'vigilode.local-continuation-cost-event.v1',
                     'event_id': 'prior-overhead-start', 'attempt_id': 'prior-overhead',
                     'category': 'controller', 'phase': 'STARTED',
                     'prev_event_sha256': '0'*64, 'charge_upper_seconds': 12.0}
            external = root/'COST_EVENTS.jsonl'
            external.write_text(json.dumps(event)+'\n')
            frozen = {p: p.read_bytes() for p in (path, external, root/'old-receipt')}
            campaign = root/'campaign'
            first = runner.run_plan(path, campaign)
            self.assertEqual([row['status'] for row in first], ['FAILED_NONZERO'])
            failed = campaign/'runs'/'task001'/'attempt_001'
            frozen.update({p: p.read_bytes() for p in failed.iterdir() if p.is_file()})
            charge = runner.cumulative_new_charge(root)
            self.assertGreater(charge, 12.0)
            self.assertEqual([row['task_id'] for row in runner.run_plan(path, campaign)], ['task002'])
            self.assertGreater(runner.cumulative_new_charge(root), charge)
            self.assertEqual(runner.run_plan(path, campaign), [])
            with self.assertRaisesRegex(ValueError, 'missing hash-bound retry authorization'):
                runner.run_plan(path, campaign, retry_failed=True)
            self.assertFalse((failed.parent/'attempt_002').exists())
            self.assertEqual(len(list(campaign.glob('runs/*/attempt_*/START.json'))), 2)
            for frozen_path, raw in frozen.items():
                self.assertEqual(frozen_path.read_bytes(), raw)

    def test_new_attempt_gate_cannot_be_widened(self):
        with temporary() as directory:
            root=Path(directory); _,path,_,_=self.make_plan(root)
            for limit in (0,2,9,True,None):
                with self.subTest(limit=limit),self.assertRaisesRegex(ValueError,'one new attempt'):
                    runner.run_plan(path,root/'campaign',max_new_attempts=limit)
            self.assertFalse((root/'campaign').exists())

    def test_mutation_detected_after_attempt_and_history_preserved(self):
        with temporary() as directory:
            root=Path(directory); plan,path,_,_=self.make_plan(root); campaign=root/'campaign'
            original=path.read_bytes()
            def child(*args,**kwargs):
                kwargs['poll_seconds']=.025
                row=self.real_run_one(*args,**kwargs)
                path.write_text(json.dumps(dict(plan,changed_after_child=True)))
                return row
            with mock.patch.object(runner,'run_one',side_effect=child):
                with self.assertRaisesRegex(ValueError,'execution plan changed'):
                    runner.run_plan(path,campaign)
            first=campaign/'runs'/'task001'/'attempt_001'/'RECEIPT.json'; raw=first.read_bytes()
            charged=runner.cumulative_new_charge(root); self.assertGreater(charged,0)
            with self.assertRaisesRegex(ValueError,'campaign output identity mismatch'):
                runner.run_plan(path,campaign)
            self.assertEqual(first.read_bytes(),raw)
            path.write_bytes(original)
            self.assertEqual(runner.run_plan(path,campaign),[])
            self.assertGreaterEqual(runner.cumulative_new_charge(root),charged)

    def test_new_external_reservation_stops_next_invocation_without_reset(self):
        with temporary() as directory:
            root=Path(directory); plan,path,_,_=self.make_plan(root); campaign=root/'campaign'
            plan['tasks']=[dict(copy.deepcopy(plan['tasks'][0]),id=f'task{i:03d}') for i in (1,2)]
            path.write_text(json.dumps(plan));runner.run_plan(path,campaign)
            first=campaign/'runs'/'task001'/'attempt_001'/'RECEIPT.json'; raw=first.read_bytes()
            e={'schema':'vigilode.local-continuation-cost-event.v1','event_id':'other-controller-start',
               'attempt_id':'other-controller','category':'controller','phase':'STARTED',
               'prev_event_sha256':'0'*64,'charge_upper_seconds':runner.NEW_CAP_SECONDS-100}
            (root/'COST_EVENTS.jsonl').write_text(json.dumps(e)+'\n')
            self.assertEqual(runner.run_plan(path,campaign),[])
            self.assertFalse((campaign/'runs'/'task002').exists())
            self.assertEqual(first.read_bytes(),raw)
            self.assertGreater(runner.cumulative_new_charge(root),runner.NEW_CAP_SECONDS-100)


if __name__ == '__main__':
    unittest.main()
