#!/usr/bin/env python3
"""Exact dyadic manufactured BE root, false RODAS-U0 admission gap is cubic in tau.
Scientific source: Robertson quadratic mass action; no arbitrary native solver scaling claim."""
from fractions import Fraction as F
import json
from pathlib import Path
def ff(z,k):
 a,b,c=z;k1,k2,k3=k
 return (-k1*a+k2*b*c,k1*a-k2*b*c-k3*b*b,k3*b*b)
def jj(z,k):
 a,b,c=z;k1,k2,k3=k
 return ((-k1,k2*c,k2*b),(k1,-k2*c-2*k3*b,-k2*b),(F(0),2*k3*b,F(0)))
r=Path(__file__).resolve().parent
z=(F(1,2),F(1,4),F(1,4));k=(F(1,2),F(1),F(1,2));f=ff(z,k)
rows=[]
for power in range(2,12):
 tau=F(1,2**power)
 y=tuple(z[i]-tau*f[i] for i in range(3))
 d=tuple(z[i]-y[i] for i in range(3))
 assert min(y)>=0
 j=jj(y,k);fy=ff(y,k)
 rr=tuple(d[i]-tau*(fy[i]+sum(j[i][l]*d[l] for l in range(3))) for i in range(3))
 predicted=(k[1]*f[1]*f[2],-k[1]*f[1]*f[2]-k[2]*f[1]**2,k[2]*f[1]**2)
 assert all(rr[i]==tau**3*predicted[i] for i in range(3))
 assert sum(rr)==0
 rows.append({'tau':str(tau),'raw_u0_max':float(max(map(abs,rr))),'exact':list(map(str,rr))})
assert all(abs(rows[i]['raw_u0_max']/rows[i+1]['raw_u0_max']-8)<1e-12 for i in range(len(rows)-1))
result={'schema':'vigilode.n01.c7.manufactured_cubic_remainder.v1','cases':len(rows),'status':'PASS','fixed_z':[str(x) for x in z],'fixed_k':[str(x) for x in k],'exact_exponent':3,'adjacent_doubling_ratio':8,'rows':rows}
(r/'tau_scaling.json').write_text(json.dumps(result,indent=2)+'\n')
try:
 import matplotlib
 matplotlib.use('Agg')
 import matplotlib.pyplot as plt
 import math
 x=[float(F(v['tau'])) for v in rows];y=[v['raw_u0_max'] for v in rows]
 plt.figure(figsize=(8.2,4.7))
 plt.loglog(x,y,'o',label='Exact manufactured BE root in RODAS U0 equation')
 plt.loglog(x,[y[0]*(xi/x[0])**3 for xi in x],linestyle='--',label='Exact cubic scaling')
 plt.xlabel('tau = h gamma (dimensionless in this benchmark)')
 plt.ylabel('Original raw-U0 residual infinity norm')
 plt.title('A BE-correct root is not an exact RODAS5P raw-U0 stage')
 plt.legend()
 plt.grid(True,which='both',alpha=0.25)
 plt.tight_layout()
 plt.savefig(r/'be_raw_u0_cubic_scaling.png',dpi=160)
 plt.close()
 print('PLOT_SAVED',r/'be_raw_u0_cubic_scaling.png')
except ImportError:
 print('MPL_UNAVAILABLE')
print('EXACT_CUBIC_PASS',len(rows),'ratio=8')
