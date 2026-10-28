#!/usr/bin/env python3
"""Small exact checks of new algebra-to-solver transfer, not old archive replay."""
from fractions import Fraction as F
import json, math, platform
from pathlib import Path

def padd(a,b):
    c=[F(0)]*max(len(a),len(b))
    for i,x in enumerate(a): c[i]+=x
    for i,x in enumerate(b): c[i]+=x
    return c

def pmul(a,b):
    c=[F(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b): c[i+j]+=x*y
    return c

def peval(c,x):
    v=F(0)
    for a in c[::-1]:v=v*x+a
    return v

def resolvent(q,s):
    r=1/s; qr=peval(q,r)
    assert qr
    g=[F(0)]*(len(q)-1); g[-1]=q[-1]
    for j in range(len(g)-2,-1,-1): g[j]=q[j+1]+r*g[j+1]
    return [x/(s*qr) for x in g], qr

def mm(a,b):
    return [[sum(a[i][k]*b[k][j] for k in range(len(b))) for j in range(len(b[0]))] for i in range(len(a))]
def madd(a,b): return [[x+y for x,y in zip(r,s)] for r,s in zip(a,b)]
def mscale(a,t):return [[t*x for x in r] for r in a]
def mid(n):return [[F(i==j) for j in range(n)] for i in range(n)]
def mpoly(c,a):
    r=[[F(0)]*len(a) for _ in a]; eye=mid(len(a))
    for x in c[::-1]:r=madd(mm(r,a),mscale(eye,x))
    return r

checks=[]
for name,q,a in [
    ('rotation',[F(13),F(4),F(1)],[[-F(2),-F(3)],[F(3),-F(2)]]),
    ('confluent_nonsemisimple',[F(4),F(4),F(1)],[[-F(2),F(5)],[F(0),-F(2)]]),
    ('mixed_primary',[F(12),F(16),F(7),F(1)],[[-F(2),F(5),F(0)],[F(0),-F(2),F(0)],[F(0),F(0),-F(3)]])]:
    assert mpoly(q,a)==[[F(0)]*len(a) for _ in a]
    for s in map(F,['1/100','1/4','10']):
        p,qr=resolvent(q,s)
        lhs=padd(pmul([F(1),-s],p),[-F(1)])
        rhs=[-x/qr for x in q]
        assert lhs==rhs
        assert mm(madd(mid(len(a)),mscale(a,-s)),mpoly(p,a))==mid(len(a))
        checks.append({'type':'exact_quotient_resolvent','name':name,'s':str(s),'degree':len(q)-1,'pass':True})

# Strictly causal nonlinear target z_i = b_i + sum_j<i c_ij z_j^2.
# Correct after i+1 synchronous rounds, with arbitrary initial values.
s=8; b=[F(i+1,50) for i in range(s)]; c=[[F((i+j)%3+1,100) if j<i else F(0) for j in range(s)] for i in range(s)]
def stage(i,z):return b[i]+sum(c[i][j]*z[j]**2 for j in range(i))
zseq=[]
for i in range(s):zseq.append(stage(i,zseq))
z=[F(-1,7)]*s
fronts=[]
for k in range(s):
    z=[stage(i,z) for i in range(s)]
    assert z[:k+1]==zseq[:k+1]
    fronts.append(k+1)
assert z==zseq
checks.append({'type':'exact_causal_nonlinear_front','stages':s,'fronts_after_round':fronts,'pass':True,'full_round_task_count':s*s,'triangular_front_skipping_task_count':s*(s+1)//2,'sequential_task_count':s})
# Stable branch examples whose discriminants are exact squares.
for a,b,c in [(F(2),F(3),F(1)),(F(5),F(2),F(-3)),(F(3),F(2),F(0)),(F(2),F(0),F(7))]:
    delta=a*a+4*c*b
    num=math.isqrt(delta.numerator); den=math.isqrt(delta.denominator)
    assert num*num==delta.numerator and den*den==delta.denominator
    sd=F(num,den); u=2*b/(a+sd)
    assert c*u*u+a*u-b==0 and a+2*c*u==sd
    for err in [F(1,100),-F(1,100)]:
        uh=u+err; dh=a+2*c*uh
        if dh>=0:
            residual=c*uh*uh+a*uh-b
            assert abs(err)==2*abs(residual)/(dh+sd)
            assert abs(err)<=2*abs(residual)/sd
    checks.append({'type':'exact_quadratic_branch','a':str(a),'b':str(b),'c':str(c),'root':str(u),'pass':True})
result={'schema':'vigilode.algebraic-transfer-check.v1','scope':'exact rational identities and causal front; no wall-time benchmark','python':platform.python_version(),'checks':checks,'all_pass':all(x['pass'] for x in checks)}
print(json.dumps(result,indent=2))
