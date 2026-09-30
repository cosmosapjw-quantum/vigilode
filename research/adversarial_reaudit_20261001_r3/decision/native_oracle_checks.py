#!/usr/bin/env python3
from fractions import Fraction as F
from pathlib import Path
import json,struct
root=Path(__file__).resolve().parent
rows=[json.loads(s) for s in (root/'time_native_independent.jsonl').read_text().splitlines()]
time=[]
for r in rows:
 if r.get('case')=='documented_step_clock_mismatch':
  exact=(F(r['tf'])-F(r['origin']))*F(r['velocity']); actual=F(r['values'][-1][0])
  assert exact==1 and abs(actual-exact)>F(1,10) and r['success']
  time.append({'method':r['method'],'origin':r['origin'],'exact_final':str(exact),'actual':float(actual),'error':float(abs(actual-exact))})
 elif r.get('case') in ['single_point_uniform','single_point_explicit']:
  assert len(r['times'])==1 and r['times'][0]==r['tf'] and r['values'][0][0]==0 and r['success']
rows=[json.loads(s) for s in (root/'arithmetic_native_independent.jsonl').read_text().splitlines()]
arith=[]
for r in rows:
 if r.get('id') in ['nilpotent_partial_underflow','nilpotent_negative_h']:
  x={k:F(v) for k,v in r['inputs'].items()};exact=x['b0']+x['h']**3*x['off']*x['b2']/6
  actual=F(r['fused']['value'][0]); rel=abs((actual-exact)/exact)
  assert rel>F(99,100) and r['fused']['converged'] and r['fused']['error_estimate']==0
  arith.append({'id':r['id'],'exact_rounded_component0':float(exact),'actual_component0':float(actual),'relative_error':float(rel),'reported_converged':True,'reported_error':0})
raw=json.loads((root.parent/'homotopy/native_tableau_bits.json').read_text())
def decode_mat(key):return [[F(struct.unpack('>d',bytes.fromhex(x))[0]) for x in row] for row in raw[key]]
def mul(a,b):return [[sum(a[i][k]*b[k][j] for k in range(8)) for j in range(8)] for i in range(8)]
coeff=[]
for key in ['alpha','l']:
 a=decode_mat(key);a2=mul(a,a);a4=mul(a2,a2);a8=mul(a4,a4)
 ap=[[a[i][j] if j<i else F(0) for j in range(8)] for i in range(8)];ap2=mul(ap,ap);ap4=mul(ap2,ap2);ap8=mul(ap4,ap4)
 assert any(x!=0 for row in a8 for x in row) and all(x==0 for row in ap8 for x in row)
 coeff.append({'matrix':key,'native_eighth_power_nonzero':True,'maximum_abs_eighth_power':float(max(abs(x) for row in a8 for x in row)),'projected_eighth_power_zero':True})
result={'status':'CONFIRMED_WITHIN_SCOPE','independent_native_time_exact_oracle':time,'independent_native_phi_exact_oracle':arith,'independent_coefficient_structure_exact_oracle':coeff,'claims_excluded':['Native full-block macroscopic solver defect from coefficient leakage','General ODE failure from extreme phi fixture']}
(root/'native_oracle_checks.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
