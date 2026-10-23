#!/usr/bin/env python3
"""Independent exact-rational fixtures; no timings or whole-ODE claims."""
import json
from fractions import Fraction as F
from pathlib import Path

def eye(n): return [[F(i == j) for j in range(n)] for i in range(n)]
def add(a,b): return [[x+y for x,y in zip(r,s)] for r,s in zip(a,b)]
def scale(a,c): return [[c*x for x in r] for r in a]
def mul(a,b): return [[sum((a[i][k]*b[k][j] for k in range(len(b))),F(0)) for j in range(len(b[0]))] for i in range(len(a))]
def mv(a,x): return [sum((v*w for v,w in zip(r,x)),F(0)) for r in a]
def power(a,k):
    out=eye(len(a))
    for _ in range(k): out=mul(out,a)
    return out

def norm(a): return max(sum(abs(x) for x in r) for r in a)
def poly(a,q):
    out=scale(eye(len(a)),q[-1])
    for c in q[-2::-1]: out=add(mul(a,out),scale(eye(len(a)),c))
    return out

def quotient(q):
    den=sum(q); p=[F(0)]*(len(q)-1); p[-1]=q[-1]
    for k in range(len(q)-2,0,-1): p[k-1]=q[k]+p[k]
    return [c/den for c in p]

checks=[]
b=[[F(-20),F(40)],[F(1,10000),F(-20)]]; ident=eye(2); m=add(ident,scale(b,-1))
q=[F(400),F(40),F(1)]; p=poly(b,quotient(q)); defect=scale(poly(b,q),1/sum(q))
assert add(mul(m,p),defect)==ident
assert norm(b)==60 and norm(defect)==F(1,110250)
x=mv(p,[F(1),F(1)]); records=[]
for k in range(4):
    residual=[v-w for v,w in zip([F(1),F(1)],mv(m,x))]
    assert residual==mv(power(defect,k+1),[F(1),F(1)])
    records.append([str(v) for v in residual]); dx=mv(p,residual); x=[a+c for a,c in zip(x,dx)]
checks.append({'id':'Q-RATIONAL','result':'pass','operator_norm':str(norm(b)),'eta':str(norm(defect)),'residuals':records})
# Several unrelated exact matrices/polynomials check the Bezout evaluation identity.
a=[[F(2),F(-7)],[F(3),F(4)]]
for degree in range(1,7):
    qq=[F(i+1) for i in range(degree+1)]; pp=poly(a,quotient(qq))
    assert add(mul(add(eye(2),scale(a,-1)),pp),scale(poly(a,qq),1/sum(qq)))==eye(2)
checks.append({'id':'Q-DEGREES-1-6','result':'pass'})
# Shifted nilpotent evaluated in a local chart, no expanded-polynomial cancellation.
n=[[F(0),F(100),F(0)],[F(0),F(0),F(-20)],[F(0),F(0),F(0)]]; aa=F(-10)
bb=add(scale(eye(3),aa),n); inv=scale(eye(3),1/(1-aa))
for j in range(1,3): inv=add(inv,scale(power(n,j),1/(1-aa)**(j+1)))
assert mul(add(eye(3),scale(bb,-1)),inv)==eye(3)
checks.append({'id':'SHIFTED-NILPOTENT','result':'pass','nilpotence_degree':3,'operator_norm':str(norm(bb))})
# CRT idempotents for a nonnormal two-eigenvalue representation.
a=[[F(-2),F(10)],[F(0),F(3)]]; e1=scale(add(scale(eye(2),3),scale(a,-1)),F(1,5)); e2=add(eye(2),scale(e1,-1))
assert mul(e1,e1)==e1 and mul(e2,e2)==e2 and mul(e1,e2)==scale(eye(2),0)
assert add(e1,e2)==eye(2)
checks.append({'id':'CRT-PROJECTORS','result':'pass','projector_norms':[str(norm(e1)),str(norm(e2))]})
# Reusing P from theta0: defect formula Etheta=E0-DeltaM P is exact.
m0=[[F(2),F(0)],[F(0),F(3)]]; pp=[[F(1,2),F(0)],[F(0),F(1,3)]]; delta=[[F(0),F(1,4)],[F(-1,5),F(0)]]
e0=add(eye(2),scale(mul(m0,pp),-1)); etheta=add(eye(2),scale(mul(add(m0,delta),pp),-1))
assert etheta==add(e0,scale(mul(delta,pp),-1))
assert norm(etheta)<=norm(e0)+norm(mul(delta,pp))<1
checks.append({'id':'PARAMETER-INVERSE-REUSE','result':'pass','uniform_eta':str(norm(etheta))})
# Nonlinear triangular update reaches the exact causal prefix without contraction.
def target(z): return [F(1),F(2)+z[0]**2,F(3)+2*z[0]+z[1]**2,F(4)+z[0]*z[2]]
zstar=[F(1),F(3),F(14),F(18)]
z=[F(1),F(-7),F(8),F(-9)]
for k in range(1,4):
    z=target(z); assert z[:k+1]==zstar[:k+1]
assert z==zstar
checks.append({'id':'CAUSAL-NONLINEAR-PREFIX','result':'pass','stages':4,'updates_after_exact_seed':3})
out={'schema_version':1,'arithmetic':'Python fractions.Fraction exact rational','status':'pass','checks':checks,'scope':'Finite algebraic fixtures independently check identities; general statements use proofs in THEORY_REASSESSMENT.md. No binary64/whole-ODE or speed evidence.'}
path=Path(__file__).resolve().parents[1]/'analysis'/'THEORY_EXACT_CHECKS.json'; path.write_text(json.dumps(out,indent=2)+'\n'); print(json.dumps({'status':'pass','checks':len(checks),'path':str(path)}))
