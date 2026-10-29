#!/usr/bin/env python3
"""Independent Fraction oracle for source-specific C8 resolvent and norm-one no-go.
This is an exact 3x3 reference, never a J-free production algorithm.
"""
from fractions import Fraction as F
from pathlib import Path
import json,random
p=Path(__file__).resolve().parent
def jac(k,z):
 k1,k2,k3=k;a,b,c=z
 return [[-k1,k2*c,k2*b],[k1,-k2*c-2*k3*b,-k2*b],[F(0),2*k3*b,F(0)]]
def inverse(mat):
 m=[list(row)+[F(i==j) for j in range(3)] for i,row in enumerate(mat)]
 for i in range(3):
  ip=next(k for k in range(i,3) if m[k][i])
  m[i],m[ip]=m[ip],m[i]
  q=m[i][i];m[i]=[z/q for z in m[i]]
  for j in range(3):
   if j==i:continue
   q=m[j][i];m[j]=[a-q*b for a,b in zip(m[j],m[i])]
 return [r[3:] for r in m]
def norm1(mat):return max(sum(abs(row[j]) for row in mat) for j in range(3))
def test_one(k,z,tau,cap):
 j=jac(k,z);a=[[F(i==m)-tau*j[i][m] for m in range(3)] for i in range(3)]
 inv=inverse(a);norm=norm1(inv)
 col=[j[i][i]+sum(abs(j[r][i]) for r in range(3) if r!=i) for i in range(3)]
 assert col==[F(0),F(0),2*k[1]*z[1]]
 assert sum(j[0][i]+j[1][i]+j[2][i] for i in range(3))==0
 assert all(sum(j[r][i] for r in range(3))==0 for i in range(3))
 m=sum(z);l=2*max(k[0],(k[1]+2*k[2])*cap*m)
 if tau*l<1:
  assert norm<=1/(1-tau*l), (k,z,tau,norm,l)
 return norm,tau*l
rng=random.Random(20261030)
checked=0;admitted=0;fails=[]
for i in range(256):
 k=tuple(F(rng.randint(0,64),64) for _ in range(3))
 z0=tuple(F(rng.randint(0,64),256) for _ in range(3))
 t=F(1,2**rng.randrange(3,13))
 try:
  norm,threshold=test_one(k,z0,t,F(11,10))
 except Exception as exc:
  fails.append({'case':i,'error':repr(exc)});break
 checked+=1
 if threshold<1:admitted+=1
k=(F(1,2),F(1),F(1,2));z=(F(1,2),F(1,4),F(1,4));tau=F(2)
mat=jac(k,z);res,trigger=test_one(k,z,tau,F(11,10))
assert mat[1][2]==-F(1,4)
assert res==F(19,15) and trigger>1
assert not fails, fails
report={'schema':'vigilode.c8.exact_fraction_resolvent.v1','seed':20261030,'cases':checked,'neumann_conditional_passes':admitted,'identity_assertions_at_least':checked*6+3,'counterexample':{'k':[str(x) for x in k],'z':[str(x) for x in z],'tau':'2','mu1':'1/2','inverse_l1_norm':str(res),'J23':str(mat[1][2]),'neumann_margin_nonpositive':trigger>1},'claim':'Positive mass-conserving chemistry is not Metzler, no unconditional l1 resolvent contraction; source-specific small 3x3 Fraction oracle only','result':'PASS'}
(p/'exact_resolvent.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
