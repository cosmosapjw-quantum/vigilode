#!/usr/bin/env python3
"""C7 exact-real Robertson BE versus original RODAS first U stage, no SciPy.
All dyadics treated as exact represented binary64 reals. No full-IVP claim."""
import csv,json,random
from fractions import Fraction as F
from pathlib import Path
root=Path(__file__).resolve().parents[1];rng=random.Random(20261030)
zero=F(0)
def ff(z,k):
 a,b,c=z;k1,k2,k3=k
 return (-k1*a+k2*b*c,k1*a-k2*b*c-k3*b*b,k3*b*b)
def jj(z,k):
 a,b,c=z;k1,k2,k3=k
 return ((-k1,k2*c,k2*b),(k1,-k2*c-2*k3*b,-k2*b),(zero,2*k3*b,zero))
def record(z,k,tau):
 fz=ff(z,k); y=tuple(z[i]-tau*fz[i] for i in range(3));d=tuple(z[i]-y[i] for i in range(3))
 if min(y)<0:return None
 f0=ff(y,k);j0=jj(y,k)
 r=tuple(d[i]-tau*(sum(j0[i][j]*d[j] for j in range(3))+f0[i]) for i in range(3))
 q=(tau*k[1]*d[1]*d[2],-tau*(k[1]*d[1]*d[2]+k[2]*d[1]*d[1]),tau*k[2]*d[1]*d[1])
 assert r==q
 assert sum(r)==0
 assert tuple(d[i]-tau*fz[i] for i in range(3))==(zero,zero,zero)
 assert sum(y)==sum(z)
 a0,b0,c0=y;k1,k2,k3=k;b=z[1]
 cc=c0+tau*k3*b*b
 aa=(a0+tau*k2*b*cc)/(1+tau*k1)
 f_scalar=aa+b+cc-sum(y)
 assert (aa,cc)==(z[0],z[2]) and f_scalar==0
 assert r[0]==q[0] and r[1]==q[1] and r[2]==q[2]
 assert all(x>=0 for x in z) and all(x>=0 for x in y)
 return {"tau":str(tau),"dy":[str(x) for x in d],"max_raw_u0":float(max(map(abs,r))),"r":[str(x) for x in r],"be_residual":"0","source_mass":str(sum(y)),"source_k":[str(x) for x in k]}
rows=[];cases=0
manufactured=record((F(1,2),F(1,4),F(1,4)),(F(1,2),F(1),F(1,2)),F(1,4))
assert manufactured and manufactured['r']==['5/65536','-35/131072','25/131072']
for it in range(256):
 for _ in range(15):
  z=tuple(F(rng.randint(1,96),128) for i in range(3))
  k=tuple(F(rng.randint(1,8),8) for i in range(3))
  tau=F(1,2**rng.randint(5,9))
  r=record(z,k,tau)
  if r:break
 else:raise AssertionError('unresolved nonnegative carrier')
 rows.append(r)
 cases+=1
assert cases==256
assert any(x['max_raw_u0']>0 for x in rows)
report={"schema":"vigilode.c7.be_original_u0_fraction.v1","seed":20261030,"cases":cases,"assertions":cases*7+7,"all_exact_rational":True,"false_admission_cases":sum(x['max_raw_u0']>0 for x in rows),"first_stage_original_residual_max":max(x['max_raw_u0'] for x in rows),"manufactured":manufactured,"result":"PASS","scope":"auxiliary exact BE vs original first RODAS5P raw-U; no full-IVP"}
(root/'analysis/results_fraction.json').write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n')
with (root/'analysis/oracle_rows.csv').open('w',newline='') as f:
 w=csv.writer(f);w.writerow(['case','tau','max_raw_u0','be_residual'])
 for i,r in enumerate(rows):w.writerow([i,r['tau'],format(r['max_raw_u0'],'.17g'),r['be_residual']])
print(json.dumps(report,indent=2))
