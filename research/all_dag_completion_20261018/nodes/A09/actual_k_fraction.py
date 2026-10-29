#!/usr/bin/env python3
"""Exact Fraction full represented-beta linear stage target oracle."""
import json,sys
from fractions import Fraction as F
x=json.load(open(sys.argv[1])); assert x['source']['chi']==0
beta=[[F(v) for v in row] for row in x['beta']];J=[[F(v) for v in row] for row in x['frozen_jacobian']]
y=[F(v) for z in x['source']['initial'] for v in z];h=F(x['h']);b=list(map(F,x['b']));s=len(beta);n=len(y);d=s*n
# This exact target uses beta directly. No alpha masking or triangular solve.
f0=[sum(a*z for a,z in zip(row,y)) for row in J]
A=[[F(int(i==j))-h*beta[i//n][j//n]*J[i%n][j%n] for j in range(d)] for i in range(d)]
rhs=[h*f0[i%n] for i in range(d)]
aug=[row+[v] for row,v in zip(A,rhs)]
for c in range(d):
 p=next(i for i in range(c,d) if aug[i][c]);aug[c],aug[p]=aug[p],aug[c]
 v=aug[c][c];aug[c]=[z/v for z in aug[c]]
 for i in range(d):
  if i!=c:
   v=aug[i][c];aug[i]=[a-v*b for a,b in zip(aug[i],aug[c])]
root=[row[-1] for row in aug]
assert all(sum(a*z for a,z in zip(row,root))==v for row,v in zip(A,rhs))
checks=0;errors=[]
for i in range(s):
 for a in range(n):
  error=abs(F(x['candidate'][i][a])-root[i*n+a]);assert error<=F(x['stage_radii'][i]);checks+=1;errors.append(error)
for a in range(n):
 exact=y[a]+sum(b[i]*root[i*n+a] for i in range(s))
 assert abs(F(x['output'][a])-exact)<=F(x['output_radii'][a]);checks+=1
print(json.dumps({'status':'PASS','evidence_kind':'exact_fraction_full_represented_beta_system',
 'dimension':d,'containments':checks,'max_stage_distance_float':float(max(errors)),
 'noncausal_beta_entries_retained':sum(beta[i][j]!=0 for i in range(s) for j in range(i+1,s)),
 'scope':'exact linear represented K target; not continuous ODE truncation'},indent=2))
