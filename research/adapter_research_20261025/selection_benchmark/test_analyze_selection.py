import copy
import json
import math
import unittest
from analyze_selection import analyze, unique_object, reject_constant


def fixture():
    identity = {key: 'a'*64 for key in (
        'problem_sha256', 'parameters_sha256', 'start_state_sha256',
        'physical_metric_sha256', 'reference_sha256', 'host_sha256',
        'source_tree_sha256', 'build_config_sha256', 'output_workload_sha256',
        'timing_scope_sha256')}
    identity.update(t_start_hex=0.0.hex(), t_end_hex=1.0.hex(),
                    error_ceiling_hex=(1e-4).hex(), reference_kind='analytic', environment_epoch=0)
    row = dict(window_id='window0', paired_block_id='repeat0', candidate_id='selected',
               selected=True, identity=identity, status='completed', final_time_hex=1.0.hex(),
               raw_verified=True, receipt_sha256='b'*64, binary_sha256='c'*64,
               reference_admitted=True, physical_error=1e-6, reference_uncertainty=0.0,
               selection_reason='measured_cost', cost=dict(solve_seconds=1.0,setup_seconds=0.2,
               probe_seconds=0.3,transition_seconds=0.5,total_seconds=2.0,disjoint_components=True))
    other = copy.deepcopy(row)
    other.update(candidate_id='alternative', selected=False)
    other['cost'].update(solve_seconds=0.4,setup_seconds=0.1,probe_seconds=0.0,transition_seconds=0.0,total_seconds=0.5)
    return dict(schema_version=1,data_kind='synthetic_contract_test',rows=[row,other])


class SelectionAnalysis(unittest.TestCase):
    def result(self, p): return analyze(p)['windows'][0]
    def test_all_costs_charged_and_synthetic_label(self):
        result=analyze(fixture()); w=result['windows'][0]
        self.assertEqual(w['observed_cost_ratio'],4.0)
        self.assertEqual(w['observed_excess_seconds'],1.5)
        self.assertEqual(result['data_kind'],'synthetic_contract_test')
        self.assertFalse(result['performance_claim_authorized'])
    def test_fast_inaccurate_never_best(self):
        p=fixture();p['rows'][1]['physical_error']=1
        w=self.result(p);self.assertEqual(w['status'],'NO_QUALIFIED_COUNTERFACTUAL')
        self.assertEqual(w['assessment'][1]['excluded'],'ACCURACY_FAILED')
    def test_failed_selected_never_gets_speedup(self):
        p=fixture();p['rows'][0]['status']='timeout'
        w=self.result(p);self.assertEqual(w['status'],'SELECTED_NOT_QUALIFIED')
        self.assertNotIn('observed_cost_ratio',w)
    def test_every_identity_difference_excludes(self):
        for field in ('start_state_sha256','source_tree_sha256','host_sha256','physical_metric_sha256',
                      'output_workload_sha256','reference_sha256','build_config_sha256','timing_scope_sha256'):
            with self.subTest(field=field):
                p=fixture();p['rows'][1]['identity'][field]='d'*64
                w=self.result(p);self.assertEqual(w['assessment'][1]['excluded'],'UNMATCHED_COMPARISON')
                self.assertEqual(w['assessment'][1]['different_fields'],[field])
    def test_same_interval_required(self):
        p=fixture();p['rows'][1]['identity']['t_end_hex']=2.0.hex()
        self.assertEqual(self.result(p)['assessment'][1]['excluded'],'UNMATCHED_COMPARISON')
    def test_early_exit_rejected(self):
        p=fixture();p['rows'][1]['final_time_hex']=(1-1e-12).hex()
        self.assertEqual(self.result(p)['assessment'][1]['excluded'],'INCOMPLETE_HORIZON')
    def test_bad_error_values(self):
        for value in (True,math.nan,math.inf,-1,10**10000):
            p=fixture();p['rows'][1]['physical_error']=value
            self.assertEqual(self.result(p)['assessment'][1]['excluded'],'INVALID_ACCURACY_VALUE')
    def test_bad_cost_values(self):
        for value in (True,math.nan,math.inf,-1,10**10000):
            p=fixture();p['rows'][1]['cost']['solve_seconds']=value
            self.assertEqual(self.result(p)['assessment'][1]['excluded'],'INVALID_COST')
    def test_cost_mismatch(self):
        p=fixture();p['rows'][0]['cost']['total_seconds']=1
        self.assertEqual(self.result(p)['cause'],'COST_ACCOUNTING_MISMATCH')
    def test_unknown_overlapping_cost(self):
        p=fixture();p['rows'][0]['cost']['disjoint_components']=False
        self.assertEqual(self.result(p)['cause'],'UNKNOWN_COST_SCOPE')
    def test_missing_reference_not_failure(self):
        p=fixture();p['rows'][1]['reference_admitted']=False
        self.assertEqual(self.result(p)['assessment'][1]['excluded'],'REFERENCE_UNADMITTED')
    def test_reference_uncertainty_charged(self):
        p=fixture();p['rows'][1]['reference_uncertainty']=1e-4
        self.assertEqual(self.result(p)['assessment'][1]['excluded'],'ACCURACY_FAILED')
    def test_exact_binary_boundary_admitted(self):
        p=fixture();p['rows'][1]['physical_error']=1e-4
        self.assertEqual(self.result(p)['status'],'COMPARABLE')
    def test_duplicate_candidate_invalidates_group(self):
        p=fixture();p['rows'].append(copy.deepcopy(p['rows'][1]))
        self.assertEqual(self.result(p)['status'],'INVALID_SELECTION_GROUP')
    def test_two_selected_invalidates_group(self):
        p=fixture();p['rows'][1]['selected']=True
        self.assertEqual(self.result(p)['status'],'INVALID_SELECTION_GROUP')
    def test_different_repeat_never_paired(self):
        p=fixture();p['rows'][1]['paired_block_id']='repeat1'
        w=analyze(p)['windows'];self.assertEqual(len(w),2)
        self.assertEqual(w[0]['status'],'NO_QUALIFIED_COUNTERFACTUAL')
        self.assertEqual(w[1]['status'],'INVALID_SELECTION_GROUP')
    def test_unverified_raw_excluded(self):
        p=fixture();p['rows'][1]['raw_verified']=False
        self.assertEqual(self.result(p)['assessment'][1]['excluded'],'UNVERIFIED_RECEIPT')
    def test_invalid_hex_identity(self):
        p=fixture();p['rows'][0]['identity']['t_end_hex']='nan'
        self.assertEqual(self.result(p)['cause'],'INVALID_IDENTITY_NUMBER')
    def test_realistic_incomplete_statuses(self):
        for status in ('timeout','interrupted','not_run','reference_missing','solver_failed'):
            p=fixture();p['rows'][1]['status']=status
            self.assertEqual(self.result(p)['assessment'][1]['excluded'],status.upper())
    def test_json_parser_guards(self):
        for data in ('{"a":1,"a":2}','{"a":NaN}','{"a":Infinity}'):
            with self.assertRaises(ValueError):
                json.loads(data, object_pairs_hook=unique_object,parse_constant=reject_constant)
    def test_zero_cost_rejected(self):
        p=fixture();p['rows'][1]['cost'].update({key:0 for key in p['rows'][1]['cost'] if key.endswith('seconds')})
        self.assertEqual(self.result(p)['assessment'][1]['excluded'],'INVALID_COST')
    def test_invalid_packet(self):
        for p in ({},dict(schema_version=True,rows=[]),dict(schema_version=1,data_kind='fake',rows=[])):
            with self.assertRaises(ValueError): analyze(p)

if __name__=='__main__': unittest.main()
