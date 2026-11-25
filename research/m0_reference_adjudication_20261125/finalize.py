"""Compute the final reference disposition and empirical uncertainty proxy."""
from pathlib import Path
import json,hashlib
D=Path(__file__).resolve().parent
s=json.loads((D/'SUMMARY.json').read_bytes())
new=json.loads((D/'ADJUDICATED_REFERENCES.json').read_bytes())
assert len(s['cases'])==len(new['references'])==s['new_qualified']==14
assert s['historical_disputed']==12 and s['original_replay_exact']==14
assert s['api_failures']==12
for c in s['cases']:
 p=next(q for q in c['protocols'] if q['protocol']==c['selected_protocol'])
 assert p['status']=='PASS_EMPIRICAL'
 rho=max(p['finest_discrepancy'],*(v for f in p['successive'] for v in f[-1]))
 corpus=json.loads((D/'intake'/f"{c['cohort']}.corpus.json").read_bytes())
 c['empirical_finest_spread_proxy']=rho
 c['minimum_planned_error_level']=min(corpus['error_levels'])
 c['spread_over_minimum_target']=rho/c['minimum_planned_error_level']
 c['empirical_spread_budget_pass']=rho<=0.1*c['minimum_planned_error_level']
 assert c['empirical_spread_budget_pass']
 # This is a diagnostic proxy, not an upper bound on true error.
 c['reference_error_bound']='UNPROVED'
s['max_selected_all_level_discrepancy']=max(c['selected_max_discrepancy'] for c in s['cases'])
s['max_finest_cross_discrepancy']=max(c['finest_discrepancy'] for c in s['cases'])
s['max_empirical_spread_proxy']=max(c['empirical_finest_spread_proxy'] for c in s['cases'])
s['max_empirical_spread_over_minimum_target']=max(c['spread_over_minimum_target'] for c in s['cases'])
s['discretionary_wolfram_stop']=json.loads((D/'receipts/WOLFRAM_STOP.json').read_bytes())
s['status']='M0_CLOSED_SCOPED_EMPIRICAL'
s['next']='M1-EVENT-TRANSACTION-01'
s['next_id_status']='selected bounded subnode under existing M1 plan'
s['source_binding']={'base_commit':'4a87f9b56f734ef10bd888d0787b723a53f05df1','base_tree':'5e0734e8c33782d00258dc531c20fd189c8b9814','bridge_sha256':hashlib.sha256((D/'bridge/model_bridge.so').read_bytes()).hexdigest(),'reference_sha256':hashlib.sha256((D/'ADJUDICATED_REFERENCES.json').read_bytes()).hexdigest()}
(D/'REFERENCE_ADJUDICATION.json').write_text(json.dumps(s,indent=2,sort_keys=True,allow_nan=False)+'\n')
print('FINAL',s['status'],s['new_qualified'],s['new_unresolved'],s['max_selected_all_level_discrepancy'],s['max_finest_cross_discrepancy'],s['max_empirical_spread_proxy'],s['max_empirical_spread_over_minimum_target'],s['raw_execution_rows'],s['raw_ok_flags'])
