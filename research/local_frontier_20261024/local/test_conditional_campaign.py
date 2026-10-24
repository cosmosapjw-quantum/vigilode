#!/usr/bin/env python3
"""Campaign-integrity tests with synthetic executables; never solver evidence."""
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import conditional_campaign as cc


class ConditionalCampaignTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.base = Path(self.temp.name)
        self.repo = self.base / 'repo'
        self.repo.mkdir()
        (self.repo / 'Cargo.lock').write_text('locked dependency fixture\n')
        self.source = {'commit': 'a' * 40, 'tree': 'b' * 40, 'status_porcelain': '', 'clean': True, 'exploratory_dirty_run': False}

    def tearDown(self):
        self.temp.cleanup()

    def fixture(self, name='conditional_chart_sweep'):
        binary = self.base / name
        binary.write_text('#!/usr/bin/env python3\nimport json\nprint(json.dumps({"kind":"header","fixture":True}))\nprint(json.dumps({"kind":"result","case":"negative-control","method":"candidate","scientific_status":"REFUSED","accepted":False,"negative_control":True,"actual_error_upper":1.0,"elapsed_seconds":0.001,"fallback_seconds":0.0002}))\n')
        binary.chmod(0o755)
        receipt = self.base / (name + '-BUILD_RECEIPT.json')
        receipt.write_text(json.dumps({'exit_code': 0, 'cargo_exit_code': 0, 'source': self.source, 'source_after': self.source, 'source_changed_during_build': False,
            'binary': str(binary), 'binary_sha256': cc.digest(binary), 'cargo_lock_sha256': cc.digest(self.repo / 'Cargo.lock'),
            'command': ['cargo', 'build', '--locked', '-p', cc.REGISTRY[name]['package'], '--example', name, '--profile', 'measurement']}))
        return {'example': name, 'binary': str(binary), 'build_receipt': str(receipt), 'args': []}

    def test_template_separates_every_registered_example(self):
        data = cc.template(self.base / 'target', self.base / 'receipts')
        self.assertEqual(len(data['experiments']), len(cc.REGISTRY))
        self.assertEqual(len({e['example'] for e in data['experiments']}), len(cc.REGISTRY))
        self.assertTrue(all(e['binary'].endswith(e['example']) for e in data['experiments']))

    def test_valid_build_is_bound_to_package_source_lock_and_binary(self):
        with patch.object(cc, 'REPO', self.repo):
            e = self.fixture()
            validated = cc.validate_entry(e, self.source)
            self.assertEqual(validated['binary_sha256'], cc.digest(e['binary']))
            self.assertEqual(validated['build_receipt_sha256'], cc.digest(e['build_receipt']))

    def test_wrong_package_is_rejected_even_with_valid_binary_hash(self):
        with patch.object(cc, 'REPO', self.repo):
            e = self.fixture(); p = Path(e['build_receipt']); r = json.loads(p.read_text())
            r['command'][4] = 'wrong-package'; p.write_text(json.dumps(r))
            with self.assertRaisesRegex(ValueError, 'registered package'):
                cc.validate_entry(e, self.source)

    def test_changed_binary_and_changed_lock_are_rejected(self):
        with patch.object(cc, 'REPO', self.repo):
            e = self.fixture(); Path(e['binary']).write_text('changed')
            with self.assertRaisesRegex(ValueError, 'build receipt'):
                cc.validate_entry(e, self.source)
            e = self.fixture(); (self.repo / 'Cargo.lock').write_text('different lock')
            with self.assertRaisesRegex(ValueError, 'Cargo.lock'):
                cc.validate_entry(e, self.source)

    def test_source_changed_build_cannot_be_used(self):
        with patch.object(cc, 'REPO', self.repo):
            e = self.fixture(); p = Path(e['build_receipt']); r = json.loads(p.read_text())
            r['source_after'] = {**self.source, 'clean': False}; p.write_text(json.dumps(r))
            with self.assertRaisesRegex(ValueError, 'source-after'):
                cc.validate_entry(e, self.source)

    def test_nonobject_or_missing_header_is_not_accepted(self):
        p = self.base / 'raw.jsonl'; p.write_text('[1,2]\n{"kind":"result"}\n')
        _, errors = cc.parse_rows(p, 'conditional_chart_sweep')
        self.assertEqual(len(errors), 2)

    def test_internal_iterations_do_not_become_independent_processes(self):
        receipts = [{'task': {'example': 'invariant_module_bench', 'process_repeat': repeat}, 'execution_status': 'COMPLETED',
                     'conditional_rows': [{'kind': 'measurement', 'mode': 'candidate', 'dimension': 16,
                                           'seconds': seconds, 'accepted': False, 'negative_control': True,
                                           'error_l2_upper': 2.0, 'fallback_seconds': 0.1}
                                          for seconds in [0.1, 0.2, 0.9]]} for repeat in [0, 1, 2]]
        summary = cc.summarize_rows(receipts)
        group = summary['groups'][0]
        self.assertEqual(group['raw_observation_count'], 9)
        self.assertEqual(group['independent_process_count'], 3)
        self.assertEqual(group['per_process_median_timings'][0]['median_timings']['seconds'], 0.2)
        self.assertEqual(summary['raw_status_counts']['REFUSED'], 9)
        self.assertFalse(summary['whole_ode_ranking_eligible'])
        self.assertTrue(all(o['negative_control'] for o in group['observations']))

    def test_three_process_campaign_resume_and_tamper_detection(self):
        with patch.object(cc, 'REPO', self.repo), patch.object(cc, 'source_identity', return_value=self.source):
            e = self.fixture()
            manifest = self.base / 'manifest.json'; manifest.write_text(json.dumps({'experiments': [e]}))
            output = self.base / 'output'
            args = ['--manifest', str(manifest), '--expected-commit', self.source['commit'], '--expected-tree', self.source['tree'], '--output', str(output), '--repeats', '3', '--budget', '10', '--timeout', '2']
            self.assertEqual(cc.main(args), 0)
            receipts = sorted(output.glob('runs/*/RECEIPT.json'))
            self.assertEqual(len(receipts), 3)
            raw_hashes = [cc.digest(p.parent / 'stdout.jsonl') for p in receipts]
            self.assertEqual(cc.main(args), 0)
            self.assertEqual(raw_hashes, [cc.digest(p.parent / 'stdout.jsonl') for p in receipts])
            r = json.loads(receipts[0].read_text()); r['conditional_rows'][1]['accepted'] = True; receipts[0].write_text(json.dumps(r))
            with self.assertRaisesRegex(ValueError, 'conditional rows'):
                cc.main(args)

    def test_host_change_blocks_resume_even_with_same_cpu_ids(self):
        with patch.object(cc, 'REPO', self.repo), patch.object(cc, 'source_identity', return_value=self.source):
            e = self.fixture()
            manifest = self.base / 'manifest.json'; manifest.write_text(json.dumps({'experiments': [e]}))
            args = ['--manifest', str(manifest), '--expected-commit', self.source['commit'], '--expected-tree', self.source['tree'], '--output', str(self.base / 'host-output'), '--repeats', '1', '--budget', '10']
            with patch.object(cc.platform, 'node', return_value='host-A'):
                self.assertEqual(cc.main(args), 0)
            with patch.object(cc.platform, 'node', return_value='host-B'):
                with self.assertRaisesRegex(ValueError, 'immutable campaign'):
                    cc.main(args)

    def test_nested_method_times_guards_and_distinct_config_are_preserved(self):
        result = {'kind': 'result', 'case_id': 'same-case', 'config': {'p': 0.8},
                  'raw': {'status': 'REFUSED', 'error_to_reference': 0.2},
                  'conditional': {'status': 'PASS_SCOPED', 'error_to_reference': 1e-9},
                  'seconds_median': {'raw': 0.2, 'conditional': 0.1}}
        alternative = {**result, 'config': {'p': 0.9}}
        guard = {'kind': 'guard', 'case_id': 'bad-branch', 'expected_refusal': True, 'guard_pass': True}
        negative = {'kind': 'negative_control', 'fixture': 'perturbed-candidate', 'bad_candidate_refused': True, 'fallback_seconds': 0.03}
        receipt = {'task': {'example': 'conditional_continuation_sweep', 'process_repeat': 0}, 'execution_status': 'COMPLETED', 'conditional_rows': [result, alternative, guard, negative]}
        summary = cc.summarize_rows([receipt])
        self.assertEqual(len(summary['groups']), 4)
        bundle = next(g for g in summary['groups'] if g['identity']['case'] == 'same-case')
        observation = bundle['observations'][0]
        self.assertEqual(observation['timings']['seconds_median.conditional'], 0.1)
        self.assertEqual(observation['error_and_residual_fields']['conditional.error_to_reference'], 1e-9)
        self.assertTrue(next(g for g in summary['groups'] if g['identity']['case'] == 'bad-branch')['observations'][0]['negative_control'])
        self.assertEqual(summary['raw_status_counts']['raw.status:REFUSED'], 2)
        self.assertTrue(next(g for g in summary['groups'] if g['identity']['case'] == 'perturbed-candidate')['observations'][0]['negative_control'])

    def test_output_inside_source_is_rejected_before_mutation(self):
        with patch.object(cc, 'REPO', self.repo):
            with self.assertRaisesRegex(ValueError, 'outside the clean'):
                cc.main(['--manifest', 'unused.json', '--expected-commit', self.source['commit'], '--expected-tree', self.source['tree'], '--output', str(self.repo / 'outputs')])
            self.assertFalse((self.repo / 'outputs').exists())

    def test_failed_child_output_is_retained(self):
        with patch.object(cc, 'REPO', self.repo), patch.object(cc, 'source_identity', return_value=self.source):
            e = self.fixture(); p = Path(e['binary']); p.write_text('#!/usr/bin/env python3\nimport sys\nprint("intentional fixture failure",file=sys.stderr)\nraise SystemExit(7)\n'); p.chmod(0o755)
            rpath = Path(e['build_receipt']); r = json.loads(rpath.read_text()); r['binary_sha256'] = cc.digest(p); rpath.write_text(json.dumps(r))
            manifest = self.base / 'failure.json'; manifest.write_text(json.dumps({'experiments': [e]}))
            output = self.base / 'failed-output'
            self.assertEqual(cc.main(['--manifest', str(manifest), '--expected-commit', self.source['commit'], '--expected-tree', self.source['tree'], '--output', str(output), '--repeats', '1', '--budget', '10']), 2)
            receipt = json.loads(next(output.glob('runs/*/RECEIPT.json')).read_text())
            self.assertEqual(receipt['exit_code'], 7)
            self.assertEqual(receipt['execution_status'], 'PROCESS_FAILURE')
            self.assertIn('intentional fixture failure', next(output.glob('runs/*/stderr.log')).read_text())


if __name__ == '__main__':
    unittest.main()
