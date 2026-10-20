#!/usr/bin/env python3
from pathlib import Path
import csv,json,math,hashlib
P=Path(__file__).resolve().parent
b=list(csv.DictReader((P/'vode_bdf_targeted.csv').open()))
a=json.loads((P/'a07_vode_exact.json').read_text())
assert len(b)==20, len(b)
endpoint=[r for r in b if r.get('grid','')!='uniform101']
assert len(endpoint)==18,len(endpoint)
assert sum(r['status']=='solver_failure' for r in endpoint)==3
assert len([r for r in b if r.get('grid')=='uniform101'])==2
for row in b:
 if row['status']=='ok':
  assert float(row['t_final']) > 0 and math.isfinite(float(row['endpoint_WRMS_at_requested_tol']))
assert math.isclose(a['vode']['rms_scaled_max_grid'],128.50035066103732,rel_tol=1e-12)
assert a['vode']['output_count']==101
assert len(set(x['output_hash'] for x in a['repetitions']))==1
assert a['vode']['reference_discrepancy_scaled_max_grid']<0.01
print(json.dumps({'status':'PASS_SCOPED','vode_cases':len(endpoint),'vode_failed':3,'a07_accuracy':a['vode']['rms_scaled_max_grid'],'A07_reference_discrepancy':a['vode']['reference_discrepancy_scaled_max_grid']}))
