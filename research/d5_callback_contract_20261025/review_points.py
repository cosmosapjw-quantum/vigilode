#!/usr/bin/env python3
"""Independent exact-rational review of the one saved D5 Rust corpus."""
import hashlib,json,math,struct,sys
from collections import Counter
from fractions import Fraction as F
from pathlib import Path

root=Path(__file__).resolve().parent
source=root/'evidence/points.jsonl'
rows=[json.loads(line) for line in source.read_text().splitlines()]
counts=Counter(r['kind'] for r in rows)
assert counts=={'point':729,'fd':49,'finite_wrong_j':1,'input_refusal':7,'fd_refusal':7,'summary':1}, counts
assert rows[-1]['kind']=='summary'
comparisons=0
def eq(a,b):
 global comparisons
 comparisons+=1
 assert a==b,(a,b)
def q(value):
 assert math.isfinite(value)
 return F.from_float(value)
def bits_value(bits):return struct.unpack('>d',int(bits).to_bytes(8,'big'))[0]
def reference(t,z):
 # Independent expanded polynomial and stoichiometric columns, exact rationals.
 t,x,y=q(t),q(z[0]),q(z[1])
 transfer=3*x
 backward=2*y*y+t*t*y*y/2
 f=[backward-transfer,transfer-backward-y]
 coupling=4*y+t*t*y
 j=[[-F(3),coupling],[F(3),-1-coupling]]
 v=[-3-coupling,4+coupling]
 return f,j,v

mutation={name:{'detected_rows':0,'first':None,'maximum_entry_error':0.0} for name in ['transpose','off_diagonal_sign','finite_J_plus_I']}
point_seen=set();max_point_error=0.0;points=[]
for number,r in enumerate((r for r in rows if r['kind']=='point'),1):
 key=tuple(r['i']);assert key not in point_seen;point_seen.add(key)
 eq([r['t'],*r['y']],[i/8 for i in key])
 f,j,v=reference(r['t'],r['y'])
 for got,want in zip(r['rhs'],f):eq(q(got),want)
 for grow,wrow in zip(r['jac'],j):
  for got,want in zip(grow,wrow):eq(q(got),want)
 for got,want in zip(r['jv'],v):eq(q(got),want)
 eq(q(r['mass_residual']),F(0))
 eq([r['rhs_count'],r['jac_count'],r['pairs']],[number]*3)
 mats={'transpose':[[q(r['jac'][c][a]) for c in range(2)] for a in range(2)],
       'off_diagonal_sign':[[q(r['jac'][a][c])*(-1 if a!=c else 1) for c in range(2)] for a in range(2)],
       'finite_J_plus_I':[[q(r['jac'][a][c])+(1 if a==c else 0) for c in range(2)] for a in range(2)]}
 errors={}
 for name,mat in mats.items():
  error=max(abs(mat[a][c]-j[a][c]) for a in range(2) for c in range(2));errors[name]=float(error)
  m=mutation[name];m['maximum_entry_error']=max(m['maximum_entry_error'],float(error))
  if error:
   m['detected_rows']+=1
   if m['first'] is None:m['first']={'grid_index':list(key),'exact_error':str(error)}
 points.append({'i':list(key),'rhs_exact_error':0.0,'jacobian_exact_error':0.0,'mutant_errors':errors})
eq(len(point_seen),729)
assert all(m['detected_rows']>0 for m in mutation.values())

fd=[];seen=set();max_fd_error=0.0
gate=64*sys.float_info.epsilon
for number,r in enumerate((r for r in rows if r['kind']=='fd'),1):
 key=(r['it'],r['power']);assert key not in seen;seen.add(key)
 eq(r['t'],r['it']/8);eq(r['eps'],2.0**(-r['power']));eq(r['y'],[0.25,0.75])
 t,tm,tp,den,y=q(r['t']),q(r['minus']),q(r['plus']),q(r['denominator']),q(r['y'][1])
 eq(tm,t-q(r['eps']));eq(tp,t+q(r['eps']));eq(den,tp-tm);eq(tp+tm,2*t)
 exact=(tp+tm)*y*y/2;derivative=t*y*y;eq(exact,derivative)
 errors=[]
 for got,want in zip(r['estimate'],[exact,-exact]):
  error=abs(q(got)-want);errors.append(float(error));comparisons+=1
  assert float(error)<=gate*max(1.0,abs(float(want)))
 max_fd_error=max(max_fd_error,*errors);eq(r['rhs_count'],2*number)
 fd.append({'t':r['t'],'power':r['power'],'eps':r['eps'],'max_absolute_error':max(errors),'gate':gate*max(1.0,abs(float(exact)))})
eq(len(seen),49)

wrong=next(r for r in rows if r['kind']=='finite_wrong_j')
f,j,_=reference(wrong['t'],wrong['y']);eq(wrong['admitted'],True)
for a in range(2):
 eq(q(wrong['rhs'][a]),f[a])
 for c in range(2):eq(q(wrong['jac'][a][c])-j[a][c],F(int(a==c)))

for r in (r for r in rows if r['kind']=='input_refusal'):
 t=bits_value(r['t_bits']);z=[bits_value(b) for b in r['y_bits']]
 assert not (math.isfinite(t) and 0<=t<=1 and all(math.isfinite(v) and 0<=v<=1 for v in z))
 eq(r['fault'],'OutsideDomain');eq([r['rhs_count'],r['jac_count'],r['pairs']],[0,0,0])
for r in (r for r in rows if r['kind']=='fd_refusal'):
 eq(r['fault'],'OutsideDomain' if r['name'] in ['left','right'] else 'InvalidDifferenceStep')
 eq(r['rhs_count'],0);eq(r['probe_count'],int(r['name'] in ['left','right','collapse']))

summary=rows[-1]
expected={'rows_before_summary':793,'points':729,'stencils':49,'wrong_finite_pairs':1,'input_refusals':7,'fd_refusals':7,'rhs_entries':828,'jac_entries':730,'forbidden_callback_entries':0,'witness_calls':3168,'new_ode_trajectories':0}
for key,value in expected.items():eq(summary[key],value)
result={'schema':'vigilode.d5.exact_oracle.v1','status':'PASS_SCOPED','input':{'path':'evidence/points.jsonl','bytes':source.stat().st_size,'sha256':hashlib.sha256(source.read_bytes()).hexdigest()},'row_counts':dict(counts),'comparison_assertions':comparisons,'point_value_comparisons':729*9,'point_exact_rational_mismatches':0,'max_point_error':max_point_error,'fd_max_absolute_error':max_fd_error,'fd_tolerance_rule':'64*binary64 epsilon*max(1,abs(exact secant))','fd_min_gate':gate,'observed':expected,'mutations':mutation,'mutation_method':'independent interpretations of saved true-J rows, not compiled source mutants','wrong_finite_J_control':'one actual full-write J+I callback pair admitted; exact derivative oracle rejects it','new_ode_trajectories':0,'old_science_replays':0,'production':'HOLD','global_completion':None}
out=root/'REVIEW.json'
if out.exists():raise RuntimeError('saved review already exists; do not repeat without a concrete new obligation')
out.write_text(json.dumps(result,indent=2)+'\n')
(root/'figures/PLOT_DATA.json').write_text(json.dumps({'points':points,'fd':fd,'mutations':mutation,'summary':expected},indent=2)+'\n')
print(json.dumps(result,indent=2))
