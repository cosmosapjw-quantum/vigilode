"""Synthetic v2 controls; no scientific solver or build is run."""
import copy
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import signal
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import continuation_runner as runner


class V2Tests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.addCleanup(mock.patch.stopall)
        mock.patch.object(runner, 'COST_ROOT', self.root).start()
        self.repo = self.root/'repo'
        self.repo.mkdir()
        subprocess.run(['git', '-C', str(self.repo), 'init', '-q'], check=True)
        (self.repo/'f').write_text('frozen\n')
        subprocess.run(['git', '-C', str(self.repo), 'add', '.'], check=True)
        subprocess.run(['git', '-C', str(self.repo), '-c', 'user.name=Test', '-c',
                        'user.email=test@example.invalid', 'commit', '-qm', 'freeze'], check=True)
        self.binary = self.root/'synthetic'
        self.binary.write_text('#!/usr/bin/env python3\nimport json\nprint(json.dumps({"kind":"result"}))\n')
        self.binary.chmod(0o755)
        (self.root/'COST_EVENTS.jsonl').write_text('')
        original = {'id': 'original3', 'case': 3, 'cpus': [0], 'threads': 1}
        self.original_receipt = self.root/'ORIGINAL.json'
        self.original_receipt.write_text(json.dumps({'task': original, 'execution_status': 'TIMEOUT',
                                                    'command': ['/original/binary', '--case', '3']}))
        selection = {'case': 3, 'wave': 1, 'depends_on_case': None,
                     'original_receipt': str(self.original_receipt),
                     'original_receipt_sha256': runner.digest(self.original_receipt),
                     'command': ['TO_FILL_FROZEN_BINARY_ABSOLUTE_PATH', '--case', '3']}
        self.catalog = self.root/'CATALOG.json'
        self.catalog.write_text(json.dumps({'schema': 'vigilode.timeout-diagnostic-runner-template.v2',
                                           'selection': [selection]}))
        mock.patch.object(runner, 'DIAGNOSTIC_CATALOG_PATH', self.catalog, create=True).start()
        mock.patch.object(runner, 'DIAGNOSTIC_CATALOG_SHA256', runner.digest(self.catalog), create=True).start()
        source = {'frozen': {'path': str(self.repo), **runner.git_identity(self.repo)}}
        self.build = self.root/'BUILD.json'
        self.build.write_text(json.dumps({'schema': 'vigilode.timeout-diagnostic-build.v1',
                                         'status': 'COMPLETED', 'exit_code': 0,
                                         'source': source, 'binary': str(self.binary),
                                         'binary_sha256': runner.digest(self.binary),
                                         'command': ['synthetic-build-not-executed']}))
        self.output = self.root/'diagnostic3'
        self.plan = {'schema': 'vigilode.local-continuation-runner.v2',
                     'deadline_utc': runner.DEADLINE, 'new_cap_seconds': runner.NEW_CAP_SECONDS,
                     'cost_root': str(self.root), 'output': str(self.output), 'source': source,
                     'runner': {'path': str(Path(runner.__file__).resolve()),
                                'sha256': runner.digest(runner.__file__)},
                     'catalog': {'path': str(self.catalog), 'sha256': runner.digest(self.catalog)},
                     'build_receipt': {'path': str(self.build), 'sha256': runner.digest(self.build)},
                     'watchdog': {'poll_seconds': 5, 'idle_seconds': 120, 'hard_seconds': 300,
                                  'maximum_samples': 180, 'rss_limit_bytes': 12*1024**3,
                                  'reservation_seconds': 960, 'extension_enabled': False},
                     'tasks': [{'id': 'diagnostic3', 'case': 3, 'cpus': [0], 'threads': 1,
                                'command': [str(self.binary), '--case', '3'], 'binary': str(self.binary),
                                'binary_sha256': runner.digest(self.binary), 'original_task': original,
                                'original_task_sha256': runner.canonical_sha(original),
                                'original_receipt': str(self.original_receipt),
                                'original_receipt_sha256': runner.digest(self.original_receipt),
                                'physical_progress_target': None, 'wave': 1,
                                'depends_on_case': None}], 'predecessor': None}
        self.path = self.root/'PLAN.json'
        self.approval = self.root/'APPROVAL.json'
        self.approve()
        def retain():
            evidence = os.environ.get('TIMEOUT_RUNNER_TEST_EVIDENCE')
            if evidence:
                shutil.copytree(self.root, Path(evidence)/self._testMethodName, symlinks=True)
        self.addCleanup(retain)

    def approve(self):
        core = {key: value for key, value in self.plan.items() if key != 'approval'}
        task = self.plan['tasks'][0]
        approval = {'schema': 'vigilode.timeout-diagnostic-approval.v1',
                    'decision': 'APPROVED_ONE_DIAGNOSTIC', 'approver': 'independent-reviewer',
                    'implementer': 'synthetic-implementer', 'independent_review': True,
                    'plan_core_sha256': runner.canonical_sha(core),
                    'original_task_sha256': task['original_task_sha256'],
                    'original_receipt_sha256': task['original_receipt_sha256'],
                    **{key: self.plan[key] for key in ('source', 'runner', 'catalog', 'build_receipt',
                                                     'output', 'deadline_utc', 'new_cap_seconds')},
                    'binary': task['binary'], 'binary_sha256': task['binary_sha256']}
        self.approval.write_text(json.dumps(approval))
        self.plan['approval'] = {'path': str(self.approval), 'sha256': runner.digest(self.approval)}
        self.path.write_text(json.dumps(self.plan))

    def run_synthetic(self, **kwargs):
        real = runner.run_diagnostic_one
        def fast(*args, **options):
            return real(*args, **options, poll_seconds=.01)
        with mock.patch.object(runner, 'run_diagnostic_one', side_effect=fast):
            return runner.run_plan(self.path, self.output, **kwargs)

    def test_v2_accepts_exact_frozen_single_task(self):
        runner.validate_plan(self.plan)

    def test_v2_schema_is_opt_in_legacy_unchanged(self):
        for schema in ('vigilode.local-continuation-runner.v3', None):
            plan = dict(self.plan, schema=schema)
            with self.assertRaisesRegex(ValueError, 'schema'):
                runner.validate_plan(plan)

    def test_catalog_selection_and_command_cannot_drift_even_with_new_approval(self):
        for field, value in [('case', 8), ('wave', 2), ('depends_on_case', 14),
                             ('original_receipt', str(self.root/'other')),
                             ('command', [str(self.binary), '--case', '8'])]:
            with self.subTest(field=field):
                saved = copy.deepcopy(self.plan)
                self.plan['tasks'][0][field] = value
                self.approve()
                with self.assertRaises((ValueError, FileNotFoundError)):
                    runner.validate_plan(self.plan)
                self.plan = saved

    def test_mutated_plan_core_and_artifacts_rejected(self):
        for field in ('runner', 'catalog', 'build_receipt', 'approval'):
            with self.subTest(field=field):
                saved = copy.deepcopy(self.plan)
                self.plan[field]['sha256'] = '0'*64
                with self.assertRaises(ValueError):
                    runner.validate_plan(self.plan)
                self.plan = saved
        self.plan['tasks'][0]['id'] = 'alias'
        with self.assertRaisesRegex(ValueError, 'approval'):
            runner.validate_plan(self.plan)

    def test_cardinality_extension_and_watchdog_cannot_widen(self):
        changes = [('tasks', []), ('tasks', self.plan['tasks']*2),
                   ('watchdog', dict(self.plan['watchdog'], hard_seconds=900)),
                   ('watchdog', dict(self.plan['watchdog'], extension_enabled=True))]
        for key, value in changes:
            with self.subTest(key=key), self.assertRaises(ValueError):
                runner.validate_plan(dict(self.plan, **{key: value}))
        self.plan['tasks'][0]['physical_progress_target'] = 100
        self.approve()
        with self.assertRaisesRegex(ValueError, 'progress|extension'):
            runner.validate_plan(self.plan)

    def test_build_runner_source_receipt_and_original_task_mutations(self):
        for path in (self.build, self.original_receipt, self.repo/'f', self.binary):
            with self.subTest(path=path):
                before = path.read_bytes()
                path.write_bytes(before+b'\nchanged')
                with self.assertRaises((ValueError, json.JSONDecodeError)):
                    runner.validate_plan(self.plan)
                path.write_bytes(before)
        self.plan['tasks'][0]['original_task_sha256'] = '0'*64
        self.approve()
        with self.assertRaisesRegex(ValueError, 'original task'):
            runner.validate_plan(self.plan)

    def test_independent_approval_required(self):
        row = json.loads(self.approval.read_text())
        for update in ({'approver': ''}, {'independent_review': False},
                       {'approver': row['implementer']}, {'decision': 'HOLD'},
                       {'original_receipt_sha256': '0'*64}, {'output': str(self.root/'swap')}):
            with self.subTest(update=update):
                self.approval.write_text(json.dumps(dict(row, **update)))
                self.plan['approval']['sha256'] = runner.digest(self.approval)
                with self.assertRaisesRegex(ValueError, 'approval'):
                    runner.validate_plan(self.plan)

    def test_binding_before_spawn_and_replay_alias_swap_refused(self):
        real = runner.spawn_pinned
        def observe(*args, **kwargs):
            binding = self.root/'DIAGNOSTIC_OUTPUT_BINDINGS'/(
                self.plan['tasks'][0]['original_task_sha256']+'.json')
            self.assertTrue(binding.is_file())
            self.assertEqual(json.loads(binding.read_text())['output'], str(self.output))
            self.assertTrue(next(self.output.glob('runs/*/attempt_*/START.json')).is_file())
            return real(*args, **kwargs)
        with mock.patch.object(runner, 'spawn_pinned', side_effect=observe):
            rows = self.run_synthetic()
        self.assertEqual(rows[0]['status'], 'COMPLETED')
        before = {p: p.read_bytes() for p in self.output.rglob('*') if p.is_file()}
        for output in (self.output, self.root/'another'):
            self.plan['output'] = str(output)
            self.plan['tasks'][0]['id'] = 'alias'
            self.approve()
            with self.assertRaisesRegex(ValueError, 'binding|replay'):
                runner.run_plan(self.path, output)
        for path, content in before.items():
            self.assertEqual(path.read_bytes(), content)

    def test_output_alias_and_existing_output_refused(self):
        alias = self.root/'alias'
        alias.symlink_to(self.root, target_is_directory=True)
        self.plan['output'] = str(alias/'campaign')
        self.approve()
        with self.assertRaisesRegex(ValueError, 'canonical|alias'):
            runner.run_plan(self.path, alias/'campaign')
        self.plan['output'] = str(self.output)
        self.approve()
        self.output.mkdir()
        with self.assertRaisesRegex(ValueError, 'output'):
            runner.run_plan(self.path, self.output)

    def test_reservation_recursive_cost_and_unfinished_attempt(self):
        historical = self.root/'prior'/'runs'/'task'/'attempt_001'
        historical.mkdir(parents=True)
        (historical/'START.json').write_text('{}')
        (historical/'RECEIPT.json').write_text(json.dumps({
            'charged_new_seconds': runner.NEW_CAP_SECONDS-959, 'status': 'COMPLETED'}))
        self.assertEqual(self.run_synthetic(), [])
        self.assertFalse(self.output.exists())
        (historical/'RECEIPT.json').unlink()
        with self.assertRaisesRegex(ValueError, 'unfinished attempt'):
            self.run_synthetic()

    def test_legacy_output_binding_does_not_reset_or_block_v2_costs(self):
        legacy = self.root/'CAMPAIGN_OUTPUT_IDENTITY.json'
        legacy.write_text('{"legacy":"unchanged"}')
        frozen = legacy.read_bytes()
        self.assertEqual(len(self.run_synthetic()), 1)
        self.assertEqual(legacy.read_bytes(), frozen)
        self.assertGreater(runner.cumulative_new_charge(self.root), 0)

    def test_retry_and_max_attempt_override_refused(self):
        with self.assertRaisesRegex(ValueError, 'retry'):
            runner.run_plan(self.path, self.output, retry_failed=True)
        with self.assertRaisesRegex(ValueError, 'one new attempt'):
            runner.run_plan(self.path, self.output, max_new_attempts=2)

    def test_wave2_requires_hash_bound_reviewed_predecessor(self):
        catalog = json.loads(self.catalog.read_text())
        catalog['selection'][0].update(wave=2, depends_on_case=14)
        self.catalog.write_text(json.dumps(catalog))
        mock.patch.object(runner, 'DIAGNOSTIC_CATALOG_SHA256', runner.digest(self.catalog)).start()
        self.plan['catalog']['sha256'] = runner.digest(self.catalog)
        self.plan['tasks'][0].update(wave=2, depends_on_case=14)
        self.approve()
        with self.assertRaisesRegex(ValueError, 'predecessor'):
            runner.validate_plan(self.plan)

    def prepare_predecessor(self):
        catalog = json.loads(self.catalog.read_text())
        catalog['selection'][0].update(wave=2, depends_on_case=14)
        self.catalog.write_text(json.dumps(catalog))
        mock.patch.object(runner, 'DIAGNOSTIC_CATALOG_SHA256', runner.digest(self.catalog)).start()
        self.plan['catalog']['sha256'] = runner.digest(self.catalog)
        self.plan['tasks'][0].update(wave=2, depends_on_case=14)
        original_sha = 'a'*64
        binding_path = self.root/'DIAGNOSTIC_OUTPUT_BINDINGS'/(original_sha+'.json')
        binding_path.parent.mkdir()
        binding_path.write_text(json.dumps({'original_task_sha256': original_sha,
            'case': 14, 'wave': 1, 'output': str(self.root/'prior'), 'plan_core_sha256': 'b'*64}))
        receipt_path = self.root/'prior'/'runs'/'prior'/'attempt_001'/'RECEIPT.json'
        receipt_path.parent.mkdir(parents=True)
        receipt_path.write_text(json.dumps({'original_task_sha256': original_sha,
            'plan_core_sha256': 'b'*64, 'status': 'FAILED_TIMEOUT_DIAGNOSTIC',
            'cleanup': {'all_reaped': True}}))
        review = {'schema': 'vigilode.timeout-diagnostic-predecessor-review.v1',
            'decision': 'APPROVED_NEXT_DIAGNOSTIC', 'independent_review': True,
            'binding_sha256': runner.digest(binding_path), 'receipt_sha256': runner.digest(receipt_path),
            'next_original_task_sha256': self.plan['tasks'][0]['original_task_sha256'],
            'source': self.plan['source'], 'binary_sha256': self.plan['tasks'][0]['binary_sha256'],
            'approver': 'fresh-reviewer', 'implementer': 'implementer'}
        review_path = self.root/'PREDECESSOR_REVIEW.json'
        review_path.write_text(json.dumps(review))
        self.plan['predecessor'] = {key: {'path': str(path), 'sha256': runner.digest(path)}
            for key, path in [('binding', binding_path), ('receipt', receipt_path), ('review', review_path)]}
        self.approve()

    def test_wave2_accepts_exact_review_and_rejects_stale_or_self_review(self):
        self.prepare_predecessor()
        runner.validate_plan(self.plan)
        for key in ('binding', 'receipt', 'review'):
            with self.subTest(key=key):
                original = self.plan['predecessor'][key]['sha256']
                self.plan['predecessor'][key]['sha256'] = '0'*64
                self.approve()
                with self.assertRaisesRegex(ValueError, 'predecessor'):
                    runner.validate_plan(self.plan)
                self.plan['predecessor'][key]['sha256'] = original
        review_path = Path(self.plan['predecessor']['review']['path'])
        row = json.loads(review_path.read_text())
        row['approver'] = row['implementer']
        review_path.write_text(json.dumps(row))
        self.plan['predecessor']['review']['sha256'] = runner.digest(review_path)
        self.approve()
        with self.assertRaisesRegex(ValueError, 'predecessor'):
            runner.validate_plan(self.plan)

    def direct(self, command, **kwargs):
        row = dict(self.plan['tasks'][0], command=command)
        return runner.run_diagnostic_one(row, self.root/'attempt', plan_sha256='p'*64,
                                        plan_core_sha256='c'*64, **kwargs)

    def test_group_rss_cpu_and_each_member_resources_are_aggregated(self):
        procroot = self.root/'proc'
        for pid, ticks, pages in [(101, 7, 3), (102, 11, 4), (103, 100, 900)]:
            path = procroot/str(pid)
            path.mkdir(parents=True)
            fields = ['0']*22
            fields[0], fields[2] = 'R', '101' if pid != 103 else '103'
            fields[11], fields[12], fields[21] = str(ticks), '2', str(pages)
            (path/'stat').write_text(f'{pid} (synthetic child) '+ ' '.join(fields))
        resources = lambda pid: {'pid': pid, 'cpu_affinity': [0], 'os_thread_count': 1}
        with mock.patch.object(runner, 'process_resources', side_effect=resources):
            group = runner.process_group_snapshot(101, proc_root=procroot)
        self.assertEqual(group['cpu_ticks'], 22)
        self.assertEqual(group['rss_bytes'], 7*os.sysconf('SC_PAGE_SIZE'))
        self.assertIsNone(runner.group_resource_violation(group))
        group['rss_bytes'] = 12*1024**3+1
        self.assertEqual(runner.group_resource_violation(group), 'FAILED_RSS_LIMIT')
        group['rss_bytes'] = 0
        for changes in ({'cpu_affinity': [1]}, {'os_thread_count': 2}):
            group['members'][1].update(changes)
            self.assertEqual(runner.group_resource_violation(group), 'FAILED_PROCESS_RESOURCES')

    def test_virtual_hard_stop_no_extension_and_five_second_samples(self):
        elapsed = [0.0]
        def sleep(seconds):
            elapsed[0] += seconds
            with (self.root/'attempt'/'stdout.jsonl').open('ab') as stream:
                stream.write(b'{"kind":"progress","accepted_steps":99,"target_steps":100}\n')
        row = self.direct([sys.executable, '-c', 'import time;time.sleep(30)'],
                          monotonic=lambda: elapsed[0], sleeper=sleep)
        self.assertEqual(row['status'], 'FAILED_TIMEOUT_DIAGNOSTIC')
        self.assertEqual(elapsed[0], 300)
        self.assertFalse(row['diagnostic_extended'])
        self.assertEqual(row['physical_progress']['status'], 'UNKNOWN')
        self.assertEqual([x['elapsed_seconds'] for x in row['watchdog_samples']], list(range(0, 300, 5)))
        self.assertTrue(row['cleanup']['all_reaped'])

    def test_virtual_idle_stop_and_sample_cap(self):
        elapsed = [0.0]
        with mock.patch.object(runner, 'process_group_snapshot', return_value={
                'pgid': 1, 'members': [], 'cpu_ticks': 0, 'rss_bytes': 0}):
            row = self.direct([sys.executable, '-c', 'import time;time.sleep(30)'],
                              monotonic=lambda: elapsed[0],
                              sleeper=lambda seconds: elapsed.__setitem__(0, elapsed[0]+seconds))
        self.assertEqual(row['status'], 'FAILED_IDLE')
        self.assertEqual(elapsed[0], 120)
        self.assertLessEqual(len(row['watchdog_samples']), 180)

    def test_resource_failure_retains_terminal_snapshot_and_cleanup(self):
        real = runner.process_group_snapshot
        count = [0]
        def snapshot(pgid):
            result = real(pgid)
            count[0] += 1
            if count[0] == 1:
                result['rss_bytes'] = 12*1024**3+1
            return result
        with mock.patch.object(runner, 'process_group_snapshot', side_effect=snapshot):
            row = self.direct([sys.executable, '-c', 'import time;time.sleep(30)'])
        self.assertEqual(row['status'], 'FAILED_RSS_LIMIT')
        self.assertGreater(row['terminal_process_group']['rss_bytes'], 12*1024**3)
        self.assertTrue(row['cleanup']['all_reaped'])

    def test_descendant_is_killed_and_reaped_after_successful_parent_exit(self):
        code = ('import subprocess,sys,json; '
                'p=subprocess.Popen([sys.executable,"-c","import time;time.sleep(30)"]); '
                'print(json.dumps({"kind":"result","child_pid":p.pid}),flush=True)')
        row = self.direct([sys.executable, '-c', code], poll_seconds=.01)
        self.assertEqual(row['status'], 'COMPLETED')
        child = json.loads((self.root/'attempt'/'stdout.jsonl').read_text())['child_pid']
        self.assertFalse((Path('/proc')/str(child)).exists())
        self.assertTrue(row['cleanup']['all_reaped'])
        self.assertIn(child, [item['pid'] for item in row['cleanup']['reaped_descendants']])

    def test_controller_termination_cleans_group_retains_receipt_and_restores_handlers(self):
        old = signal.getsignal(signal.SIGTERM)
        def terminate(seconds):
            os.kill(os.getpid(), signal.SIGTERM)
        with self.assertRaises(SystemExit):
            self.direct([sys.executable, '-c', 'import time;time.sleep(30)'], sleeper=terminate)
        row = json.loads((self.root/'attempt'/'RECEIPT.json').read_text())
        self.assertEqual(row['status'], 'FAILED_INTERRUPTED')
        self.assertTrue(row['cleanup']['all_reaped'])
        self.assertEqual(signal.getsignal(signal.SIGTERM), old)

    def test_observation_exception_still_kills_child(self):
        real = runner.process_group_snapshot
        count = [0]
        def snapshot(pgid):
            count[0] += 1
            if count[0] <= 2:
                raise OSError('synthetic observation failure')
            return real(pgid)
        with mock.patch.object(runner, 'process_group_snapshot', side_effect=snapshot):
            row = self.direct([sys.executable, '-c', 'import time;time.sleep(30)'])
        self.assertEqual(row['status'], 'FAILED_RUNNER')
        self.assertTrue(row['cleanup']['all_reaped'])
        self.assertTrue(row['cleanup']['errors'])

    def test_post_execution_identity_mutation_remains_failed_and_charged(self):
        calls = [0]
        def identity():
            calls[0] += 1
            if calls[0] == 2:
                raise ValueError('synthetic frozen identity changed')
        row = self.direct([sys.executable, '-c', 'print("{\\"kind\\":\\"result\\"}")'],
                          poll_seconds=.01, identity_check=identity)
        self.assertEqual(row['status'], 'FAILED_IDENTITY_CHANGED')
        self.assertTrue(row['cleanup']['all_reaped'])
        self.assertGreater(row['charged_new_seconds'], 0)

    def test_v2_mutex_covers_validation_binding_spawn_and_cleanup(self):
        import fcntl
        phases = []
        def prove(phase):
            with (self.root/'.julia_comparison_campaign.lock').open('a+') as lock:
                with self.assertRaises(BlockingIOError):
                    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            phases.append(phase)
        validate, durable, cleanup = runner.validate_plan, runner.durable_new, runner.cleanup_diagnostic_group
        def v(plan):
            prove('validation'); return validate(plan)
        def d(path, value):
            prove('durable write'); return durable(path, value)
        def c(proc):
            prove('cleanup'); return cleanup(proc)
        with mock.patch.object(runner, 'validate_plan', side_effect=v), \
             mock.patch.object(runner, 'durable_new', side_effect=d), \
             mock.patch.object(runner, 'cleanup_diagnostic_group', side_effect=c):
            self.run_synthetic()
        self.assertTrue({'validation', 'durable write', 'cleanup'}.issubset(phases))

    def failed_cleanup_fixture(self, cleanup):
        attempt = self.root/'prior'/'runs'/'prior'/'attempt_001'
        attempt.mkdir(parents=True, exist_ok=True)
        now = runner.utc_now()
        start = {'schema': 'vigilode.timeout-diagnostic-start.v1', 'task_id': 'prior',
                 'original_task_sha256': 'd'*64, 'plan_core_sha256': 'e'*64,
                 'started_utc': (now-dt.timedelta(seconds=1000)).isoformat()}
        process = {'pid': 987654321, 'pgid': 987654321}
        row = {'schema': 'vigilode.timeout-diagnostic-receipt.v1', 'task_id': 'prior',
               'original_task_sha256': 'd'*64, 'plan_core_sha256': 'e'*64,
               'status': 'FAILED_CLEANUP', 'charged_new_seconds': 7.0,
               'started_utc': start['started_utc'],
               'finished_utc': (now-dt.timedelta(seconds=990)).isoformat(), 'cleanup': cleanup}
        for name, value in [('START.json', start), ('PROCESS.json', process), ('RECEIPT.json', row)]:
            (attempt/name).write_text(json.dumps(value))
        binding = self.root/'DIAGNOSTIC_OUTPUT_BINDINGS'/('d'*64+'.json')
        binding.parent.mkdir(exist_ok=True)
        binding.write_text(json.dumps({'original_task_sha256': 'd'*64, 'output': str(self.root/'prior')}))
        return attempt

    def test_repair_unresolved_cleanup_blocks_other_task_before_binding_or_spawn(self):
        for cleanup in ({'all_reaped': False, 'after': {'members': [{'pid': 12345}]}},
                        {'all_reaped': False, 'after': None, 'errors': ['snapshot unavailable']},
                        None, {'all_reaped': True, 'after': None}):
            with self.subTest(cleanup=cleanup):
                attempt = self.failed_cleanup_fixture(cleanup)
                original = (attempt/'RECEIPT.json').read_bytes()
                with mock.patch.object(runner, 'spawn_pinned') as spawn:
                    with self.assertRaisesRegex(ValueError, 'unresolved.*cleanup'):
                        runner.run_plan(self.path, self.output)
                    spawn.assert_not_called()
                self.assertFalse(self.output.exists())
                self.assertFalse((self.root/'DIAGNOSTIC_OUTPUT_BINDINGS'/(
                    self.plan['tasks'][0]['original_task_sha256']+'.json')).exists())
                self.assertEqual((attempt/'RECEIPT.json').read_bytes(), original)

    def test_repair_exact_cleanup_reconciliation_requires_absence_and_full_cost(self):
        attempt = self.failed_cleanup_fixture({'all_reaped': False, 'after': None})
        original = {path: path.read_bytes() for path in attempt.iterdir()}
        start = json.loads((attempt/'START.json').read_text())
        row = {'schema': 'vigilode.timeout-diagnostic-cleanup-reconciliation.v1',
               'status': 'FAILED_CLEANUP_RECONCILED', 'task_id': 'prior',
               'original_task_sha256': start['original_task_sha256'],
               'start_sha256': runner.digest(attempt/'START.json'),
               'process_sha256': runner.digest(attempt/'PROCESS.json'),
               'receipt_sha256': runner.digest(attempt/'RECEIPT.json'),
               'pgid': 987654321, 'process_tree_checked': True,
               'process_group_absent': True,
               'reconciled_utc': runner.utc_now().isoformat(), 'charged_new_seconds': 1100.0}
        path = attempt/'CLEANUP_RECONCILIATION.json'
        path.write_text(json.dumps(row))
        with mock.patch.object(runner.os, 'killpg', side_effect=ProcessLookupError), \
             mock.patch.object(runner, 'process_group_snapshot', return_value={
                 'pgid': row['pgid'], 'members': [], 'cpu_ticks': 0, 'rss_bytes': 0}):
            self.assertEqual(runner.cumulative_new_charge(self.root), 1100)
            for update in ({'receipt_sha256': '0'*64}, {'charged_new_seconds': 960},
                           {'process_group_absent': False}, {'pgid': 987654322}):
                with self.subTest(update=update):
                    path.write_text(json.dumps(dict(row, **update)))
                    with self.assertRaisesRegex(ValueError, 'reconciliation'):
                        runner.cumulative_new_charge(self.root)
            path.write_text(json.dumps(row))
        with mock.patch.object(runner.os, 'killpg', return_value=None):
            with self.assertRaisesRegex(ValueError, 'process group still exists'):
                runner.cumulative_new_charge(self.root)
        real_kill = runner.os.killpg
        def kill(pgid, signum):
            if pgid == row['pgid'] and signum == 0:
                raise ProcessLookupError
            return real_kill(pgid, signum)
        with mock.patch.object(runner.os, 'killpg', side_effect=kill):
            self.assertEqual(self.run_synthetic()[0]['status'], 'COMPLETED')
            self.assertGreater(runner.cumulative_new_charge(self.root), 1100)
        for old_path, data in original.items():
            self.assertEqual(old_path.read_bytes(), data)
        self.assertTrue((self.root/'DIAGNOSTIC_OUTPUT_BINDINGS'/('d'*64+'.json')).exists())

    def test_repair_plan_replacement_after_locked_parse_refused_before_binding(self):
        validate = runner.validate_plan
        calls = [0]
        def replace(plan):
            validate(plan)
            calls[0] += 1
            if calls[0] == 1:
                replacement = self.root/'REPLACEMENT.json'
                replacement.write_text(json.dumps(dict(self.plan, replacement_marker=True)))
                replacement.replace(self.path)
        with mock.patch.object(runner, 'validate_plan', side_effect=replace), \
             mock.patch.object(runner, 'spawn_pinned', side_effect=AssertionError('unexpected spawn')) as spawn:
            with self.assertRaisesRegex(ValueError, 'execution plan changed'):
                runner.run_plan(self.path, self.output)
            spawn.assert_not_called()
        self.assertFalse((self.root/'DIAGNOSTIC_OUTPUT_BINDINGS').exists())
        self.assertFalse(self.output.exists())

    def test_repair_bound_json_parses_only_the_bytes_it_verified(self):
        read = Path.read_text
        predecessor = self.root/'PREDECESSOR.json'
        predecessor.write_text('{"predecessor":"verified"}')
        for path in (self.catalog, self.approval, self.build, self.original_receipt, predecessor):
            expected = json.loads(path.read_bytes())
            def replacement(item, *args, **kwargs):
                if item == path:
                    return '{"marker":"unverified replacement"}'
                return read(item, *args, **kwargs)
            with self.subTest(path=path), mock.patch.object(Path, 'read_text', autospec=True,
                                                           side_effect=replacement):
                self.assertEqual(runner.bound_json({'path': str(path), 'sha256': runner.digest(path)},
                                                   'synthetic artifact'), expected)
        def original_replacement(item, *args, **kwargs):
            if item == self.original_receipt:
                return '{"task":{"marker":"unverified replacement"}}'
            return read(item, *args, **kwargs)
        with mock.patch.object(Path, 'read_text', autospec=True, side_effect=original_replacement):
            runner.validate_plan(self.plan)


if __name__ == '__main__':
    unittest.main()
