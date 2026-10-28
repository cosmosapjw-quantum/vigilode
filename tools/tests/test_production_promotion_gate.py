import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from production_promotion_gate import assess_wp, assess_comparison, safe_file, evaluate


def fixture():
    protocol = {'tier': 'radau5_cvode_refinement_v1',
                'agreement_threshold': 1e-10, 'refinement_threshold': 1e-10,
                'rtols': [1e-12, 1e-13], 'atol_factors': [1.0, 0.1]}
    corpus = {'reference': {'independent_protocol': protocol},
              'problems': [{'id': 'p', 'banded': False, 'reference_atol': 1e-16}], 'arms': ['vigilode-dense'],
              'rtols': [1e-6], 'grid_points': 101, 'repetitions': 1}
    manifest = 'frozen exact source bytes'
    raw = {'schema': 'vigilode-bench.wp.v1', 'corpus': copy.deepcopy(corpus),
           'provenance': {'git_commit': 'a' * 40, 'source_identity': {
               'status': 'VERIFIED_BUILD_SOURCE', 'corpus_sha256': 'b' * 64,
               'git_dirty': False, 'dirty_source_allowed': False,
               'verified_boundaries': ['before_run', 'after_run'],
               'manifest': manifest, 'manifest_sha256': hashlib.sha256(manifest.encode()).hexdigest()}},
           'references': [{'problem': 'p', 'disputed': False, 'arm': 'radau5', 'rtol': 1e-13,
                           'atol': 1e-16 * .1, 'status': {'kind': 'success'}, 'states': [[1]] * 101,
                           'independent_protocol': {**protocol, 'status': 'PASS', 'levels': [
                               {'rtol': rtol, 'atol': 1e-16 * factor,
                                'radau5_status': {'kind': 'success'}, 'cvode_status': {'kind': 'success'},
                                'radau5_grid_points': 101, 'cvode_grid_points': 101,
                                'cross_family_disagreement': 1e-12,
                                'radau5_previous_level_disagreement': 1e-12 if index else None,
                                'cvode_previous_level_disagreement': 1e-12 if index else None}
                               for index, (rtol, factor) in enumerate(zip(protocol['rtols'], protocol['atol_factors']))]}}],
           'protocol_violation_rows': 0, 'timing_authority': 'diagnostic_only',
           'rows': [{'problem': 'p', 'arm': 'vigilode-dense', 'rtol': 1e-6,
                     'status': {'kind': 'success'}, 'grid_points_completed': 101,
                     'error_wrms_own': 0.7, 'deterministic': True,
                     'repetitions': [{'status': {'kind': 'success'},
                                      'grid_points_completed': 101, 'work': {'rhs': 80}}]}]}
    return raw, corpus


class PromotionGateTest(unittest.TestCase):
    def assess(self, raw, corpus):
        return assess_wp(raw, corpus, 'b' * 64, 'a' * 40)

    def test_empirical_grid_pass_does_not_claim_continuous_accuracy_or_speed(self):
        raw, corpus = fixture()
        out = self.assess(raw, corpus)
        self.assertEqual(out['status'], 'EMPIRICAL_GRID_PASS')
        self.assertEqual(out['continuous_error_certificate'], 'NOT_INFERRED_FROM_GRID')
        self.assertTrue(out['speed_promotion'].startswith('HOLD'))

    def test_missing_duplicate_and_budget_laundered_rows_refuse(self):
        for mutation, reason in [
            (lambda r: r['rows'].clear(), 'CANDIDATE_COVERAGE_MISMATCH'),
            (lambda r: r['rows'].append(copy.deepcopy(r['rows'][0])), 'DUPLICATE_CANDIDATE_ROW'),
            (lambda r: r['corpus']['rtols'].__setitem__(0, 1e-3), 'CORPUS_CHANGED')]:
            raw, corpus = fixture(); mutation(raw)
            self.assertIn(reason, self.assess(raw, corpus)['reasons'])

    def test_later_failed_repetition_cannot_hide_behind_first_success(self):
        raw, corpus = fixture()
        raw['rows'][0]['repetitions'][0]['status'] = {'kind': 'failed'}
        self.assertIn('FAILED_OR_INCOMPLETE_REPETITION', self.assess(raw, corpus)['reasons'])

    def test_wrong_source_or_missing_independent_reference_refuses(self):
        raw, corpus = fixture()
        raw['provenance']['git_commit'] = 'c' * 40
        raw['references'][0]['independent_protocol']['status'] = 'REFERENCE_DISPUTED'
        reasons = self.assess(raw, corpus)['reasons']
        self.assertIn('BUILD_COMMIT_MISMATCH', reasons)
        self.assertIn('REFERENCE_DISPUTED_OR_UNVERIFIED', reasons)

    def test_pass_label_without_ladder_and_joint_drift_are_refused(self):
        raw, corpus = fixture()
        raw['references'][0]['independent_protocol']['levels'] = []
        self.assertIn('REFERENCE_LADDER_COVERAGE', self.assess(raw, corpus)['reasons'])
        raw, corpus = fixture()
        level = raw['references'][0]['independent_protocol']['levels'][1]
        level['radau5_previous_level_disagreement'] = 1e-4
        level['cvode_previous_level_disagreement'] = 1e-4
        self.assertIn('REFERENCE_REFINEMENT_FAILED', self.assess(raw, corpus)['reasons'])

    def test_nan_or_excess_error_is_preserved_as_failure(self):
        for error in [float('nan'), float('inf'), -0.1, 1.01]:
            raw, corpus = fixture(); raw['rows'][0]['error_wrms_own'] = error
            result = self.assess(raw, corpus)
            self.assertIn('GLOBAL_GRID_ERROR_ABOVE_ORIGINAL_UNIT', result['reasons'])
            self.assertEqual(len(result['excess_error_rows']), 1)

    def test_no_empty_campaign_vacuous_pass(self):
        raw, corpus = fixture(); corpus['arms'] = []; raw['corpus'] = copy.deepcopy(corpus)
        self.assertIn('EMPTY_CANDIDATE_SCOPE', self.assess(raw, corpus)['reasons'])

    def test_stage_certificate_cannot_be_promoted_to_ode_or_full_curve(self):
        raw = {'schema': 'vigilode.reaudit-comparison.v1', 'suite': 'actual-k',
               'status': 'PASS_CERTIFIED_STAGE_FUNCTIONAL', 'scope': 'STAGE_FUNCTIONAL_ONLY',
               'full_curve_certified': False, 'original_budget': 1e-9, 'achieved_bound': 1e-10,
               'outputs': {'center': [1]}, 'work': {'attempts': 3},
               'solver_transaction_timed': True, 'elapsed_ns': 1000}
        self.assertEqual(assess_comparison(raw)['status'], 'SCOPED_CERTIFICATE_PASS')
        raw['full_curve_certified'] = True
        self.assertIn('CERTIFICATE_SCOPE_MISMATCH', assess_comparison(raw)['reasons'])
        raw['full_curve_certified'] = False
        raw['scope'] = 'REQUESTED_ODE_ENDPOINTS_ONLY'
        self.assertIn('CERTIFICATE_SCOPE_MISMATCH', assess_comparison(raw)['reasons'])

    def test_relaxed_comparison_budget_cannot_pass_even_with_small_bound(self):
        raw = {'schema': 'vigilode.reaudit-comparison.v1', 'suite': 'homotopy',
               'status': 'PASS_CERTIFIED_ENDPOINTS', 'scope': 'REQUESTED_ODE_ENDPOINTS_ONLY',
               'full_curve_certified': False, 'original_budget': 1e-3, 'achieved_bound': 1e-6,
               'outputs': [1], 'work': {'attempts': 3},
               'solver_transaction_timed': True, 'elapsed_ns': 1000}
        self.assertIn('ORIGINAL_BUDGET_NOT_SATISFIED', assess_comparison(raw)['reasons'])

    def test_receipts_cannot_escape_output_root_or_change_after_capture(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp); child = base / 'run.json'; child.write_text('{}')
            digest = hashlib.sha256(child.read_bytes()).hexdigest()
            self.assertEqual(safe_file(base, 'run.json', digest), child)
            child.write_text('{"altered":true}')
            with self.assertRaises(ValueError): safe_file(base, 'run.json', digest)
            with self.assertRaises(ValueError): safe_file(base, '../outside.json', digest)


class FrozenCampaignRegressionTest(unittest.TestCase):
    """Synthetic complete receipt chain, never solver or speed evidence."""

    def setUp(self):
        from run_ncp_reaudit import canonical, planned_case_ids, steps
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.raw_wp, self.corpus = fixture()
        self.corpus_sha = hashlib.sha256(canonical(self.corpus)).hexdigest()
        self.raw_wp['provenance']['source_identity']['corpus_sha256'] = self.corpus_sha
        self.source = {'commit': 'a' * 40, 'tree': 'b' * 40, 'closure_sha256': 'c' * 64,
                       'closure_policy_sha256': 'd' * 64, 'clean': True}
        self.config = {'comparison_cases': [{'suite': 'polynomial', 'case': 'p4',
                                            'arms': ['router'], 'original_budget': 1e-7}],
                       'threads': [1], 'warmups': 0, 'repetitions': 1,
                       'memory_limit_bytes': 1024, 'corpus': 'corpus.json',
                       'locked_inputs': {'corpus.json': self.corpus_sha}}
        identity = {'source': self.source, 'config_sha256': hashlib.sha256(canonical(self.config)).hexdigest()}
        self.execution = hashlib.sha256(canonical(identity)).hexdigest()
        self.plan = {'identity': identity, 'config': self.config, 'steps': [list(x) for x in steps(self.config)]}
        self.write_json('PLAN.json', self.plan)
        self.bin_paths = {'comparison_build': 'targets/native/release/examples/reaudit_comparison',
                          'bench_build': 'targets/bench/release/vigilode-bench'}
        self.libraries = {'libraries': {'/fixture/libc.so': 'f' * 64}, 'scope': 'synthetic fixture'}
        self.bindings = {}
        for name, rel in self.bin_paths.items():
            self.write_bytes(rel, name.encode())
            self.bindings[name] = {'path': rel, 'sha256': self.sha(rel), 'runtime_libraries': self.libraries}
        validation = []
        for name, argv, _ in steps(self.config):
            binding = self.bindings.get(name)
            extra = {binding['path']: binding['sha256']} if binding else {}
            libraries = {binding['path']: self.libraries} if binding else {}
            validation.append(self.receipt(name, argv, extra=extra, libraries=libraries))
        self.comparison_id = planned_case_ids(self.config)[0]
        raw = {'schema': 'vigilode.reaudit-comparison.v1', 'suite': 'polynomial', 'case': 'p4',
               'arm': 'router', 'threads': 1, 'status': 'PASS_CERTIFIED_ENDPOINTS',
               'scope': 'REQUESTED_ODE_ENDPOINTS_ONLY', 'full_curve_certified': False,
               'original_budget': 1e-7, 'achieved_bound': 1e-8, 'input_sha256': 'e' * 64,
               'outputs': [1.0], 'work': {'attempts': 1}, 'solver_transaction_timed': True,
               'elapsed_ns': 1000}
        raw_path = 'runs/' + self.comparison_id + '/raw.json'
        self.write_json(raw_path, raw)
        comparison_argv = [str(self.base / self.bin_paths['comparison_build']), '--suite', 'polynomial',
                           '--case', 'p4', '--arm', 'router', '--threads', '1']
        case = self.receipt(self.comparison_id, comparison_argv,
                            extra={raw_path: self.sha(raw_path)}, binding=self.bindings['comparison_build'])
        case.update({'kind': 'comparison', 'status': 'EXECUTED', 'issues': [],
                     'raw_path': raw_path, 'raw_sha256': self.sha(raw_path), 'process_elapsed_ns': 2000})
        self.raw_wp['provenance']['source_identity']['binary_sha256'] = self.bindings['bench_build']['sha256']
        self.write_json('WORK_PRECISION.json', self.raw_wp)
        wp_argv = [str(self.base / self.bin_paths['bench_build']), 'wp', '--corpus', '/fixture/repo/corpus.json',
                   '--out', str(self.base / 'WORK_PRECISION.json'), '--require-independent-reference',
                   '--source-root', '/fixture/repo']
        wp = self.receipt('work_precision', wp_argv,
                          extra={'WORK_PRECISION.json': self.sha('WORK_PRECISION.json')},
                          binding=self.bindings['bench_build'])
        wp.update({'kind': 'work_precision', 'status': 'EXECUTED', 'issues': [],
                   'raw_path': 'WORK_PRECISION.json', 'raw_sha256': self.sha('WORK_PRECISION.json')})
        self.summary = {'schema': 'vigilode.ncp-receipt.v1', 'source': self.source,
                        'execution_identity_sha256': self.execution,
                        'plan_path': 'PLAN.json', 'plan_sha256': self.sha('PLAN.json'),
                        'validation': {'all_passed': True, 'required_steps': [x[0] for x in steps(self.config)],
                                       'receipts': validation},
                        'benchmarks': {'required_cases': planned_case_ids(self.config), 'cases': [case, wp]}}

    def write_bytes(self, rel, data):
        path = self.base / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)

    def write_json(self, rel, value):
        self.write_bytes(rel, json.dumps(value, sort_keys=True).encode())

    def sha(self, rel):
        return hashlib.sha256((self.base / rel).read_bytes()).hexdigest()

    def receipt(self, name, argv, extra=None, libraries=None, binding=None):
        stdout, stderr = f'runs/{name}/stdout.log', f'runs/{name}/stderr.log'
        self.write_bytes(stdout, b'fixture stdout')
        self.write_bytes(stderr, b'')
        receipt = {'id': name, 'argv': argv, 'source': self.source, 'source_after': self.source,
                   'execution_identity_sha256': self.execution, 'status': 'PASS',
                   'process': {'exit_code': 0, 'timed_out': False, 'rss_status': 'OBSERVED',
                               'peak_rss_bytes': 10, 'affinity_verified': True, 'affinity_escape': False},
                   'artifacts': {stdout: self.sha(stdout), stderr: self.sha(stderr), **(extra or {})},
                   'runtime_libraries': libraries or {}, 'executable_binding': binding,
                   'executable_unchanged_after': True}
        rel = f'runs/{name}/RECEIPT.json'
        self.write_json(rel, receipt)
        return {'id': name, 'receipt_path': rel, 'receipt_sha256': self.sha(rel)}

    def assess(self):
        return evaluate(self.summary, self.base, self.corpus, self.corpus_sha, 'a' * 40, self.config)

    def test_complete_synthetic_chain_is_scoped_and_never_broad_production(self):
        result = self.assess()
        self.assertEqual(result['receipt_integrity'], 'PASS', result)
        self.assertEqual([x['status'] for x in result['cases']], ['SCOPED_CERTIFICATE_PASS', 'EMPIRICAL_GRID_PASS'])
        self.assertEqual(result['broad_production'], 'HOLD')

    def test_self_consistent_corpus_replacement_cannot_override_frozen_config_lock(self):
        self.corpus['id'] = 'rebound-corpus'
        self.raw_wp['corpus'] = copy.deepcopy(self.corpus)
        from run_ncp_reaudit import canonical
        self.corpus_sha = hashlib.sha256(canonical(self.corpus)).hexdigest()
        self.raw_wp['provenance']['source_identity']['corpus_sha256'] = self.corpus_sha
        self.write_json('WORK_PRECISION.json', self.raw_wp)
        row = self.summary['benchmarks']['cases'][1]
        row['raw_sha256'] = self.sha('WORK_PRECISION.json')
        path = row['receipt_path']
        receipt = json.loads((self.base / path).read_text())
        receipt['artifacts']['WORK_PRECISION.json'] = row['raw_sha256']
        self.write_json(path, receipt)
        row['receipt_sha256'] = self.sha(path)
        result = self.assess()
        self.assertEqual(result['receipt_integrity'], 'HOLD')
        self.assertTrue(any('supplied corpus hash differs from frozen config lock' in r for r in result['reasons']))

    def test_valid_other_build_binary_cannot_be_rebound_to_comparison_command(self):
        row = self.summary['benchmarks']['cases'][0]
        path = row['receipt_path']
        receipt = json.loads((self.base / path).read_text())
        receipt['executable_binding'] = self.bindings['bench_build']
        self.write_json(path, receipt)
        row['receipt_sha256'] = self.sha(path)
        result = self.assess()
        self.assertEqual(result['receipt_integrity'], 'HOLD')
        self.assertIn('INVALID_RAW_RECEIPT:' + self.comparison_id, result['reasons'])
        self.assertIn('executable binding differs', result['cases'][0]['detail'])


if __name__ == '__main__':
    unittest.main()
