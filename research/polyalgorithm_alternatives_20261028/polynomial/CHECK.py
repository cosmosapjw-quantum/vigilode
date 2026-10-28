#!/usr/bin/env python3
"""One prospective exact-algebra pass; no solver, benchmark or tuning execution."""
from fractions import Fraction as F
from math import comb, factorial
import json

def trim(p):
    q = list(p)
    while len(q) > 1 and q[-1] == 0:
        q.pop()
    return q

def add(*polys):
    q = [F(0)] * max(map(len, polys))
    for p in polys:
        for i, v in enumerate(p):
            q[i] += v
    return trim(q)

def scale(p, a):
    return trim([a * v for v in p])

def mul(p, q):
    out = [F(0)] * (len(p) + len(q) - 1)
    for i, a in enumerate(p):
        for j, b in enumerate(q):
            out[i+j] += a*b
    return trim(out)

def laguerre(k):
    return [F((-1)**j * comb(k,j), factorial(j)) for j in range(k+1)]

def chebyshev(k):
    p, q = [F(1)], [F(0),F(1)]
    if k == 0:
        return p
    for _ in range(1,k):
        p,q = q,add(mul([F(0),F(2)],q),scale(p,F(-1)))
    return q

def laguerre_backward(coeff):
    m=len(coeff)-1
    b1,b2=[F(0)],[F(0)]
    for k in range(m,-1,-1):
        b=add([coeff[k]],mul([F(2*k+1,k+1),F(-1,k+1)],b1),
              scale(b2,F(-(k+1),k+2)))
        b1,b2=b,b1
    return b1

def chebyshev_backward(coeff):
    m=len(coeff)-1
    b1,b2=[F(0)],[F(0)]
    for k in range(m,0,-1):
        b=add([coeff[k]],mul([F(0),F(2)],b1),scale(b2,F(-1)))
        b1,b2=b,b1
    return add([coeff[0]],mul([F(0),F(1)],b1),scale(b2,F(-1)))

checks=[]
for family,basis,backward in (
        ("laguerre",laguerre,laguerre_backward),
        ("chebyshev",chebyshev,chebyshev_backward)):
    for m in range(0,9):
        coeff=[F((-1)**k,k+1) for k in range(m+1)]
        expected=add(*[scale(basis(k),coeff[k]) for k in range(m+1)])
        assert backward(coeff)==expected, (family,m,"coefficient identity")
        for j in range(m+1):
            impulse=[F(0)]*(m+1)
            impulse[j]=F(1)
            assert backward(impulse)==basis(j), (family,m,j,"defect transfer")
        checks.append({"family":family,"degree":m,"exact_identity":True,
                       "all_impulse_positions":True})

epsilon=F(1,100_000_000)
q=F(1,2)
factor=lambda m:2*q**m/(1+q**(2*m))
assert factor(28)<=epsilon<factor(27)

# Jordan example evaluated exactly: (I-A)e2=-K e1; (A-I)^2=0.
K=F(100)
A=[[F(1),K],[F(0),F(1)]]
P=[[F(1),-K],[F(0),F(1)]]
AP=[[sum(A[i][k]*P[k][j] for k in range(2)) for j in range(2)] for i in range(2)]
assert AP==[[F(1),F(0)],[F(0),F(1)]]

receipt={
    "schema":"vigilode-polynomial-exact-check-v1",
    "scope":"finite exact rational algebra; not solver correctness/performance",
    "checks":checks,
    "chebyshev_degree_example":{
        "interval":["1","9"],"epsilon":str(epsilon),"minimal_degree":28,
        "factor_m":str(factor(28)),"factor_previous":str(factor(27))
    },
    "jordan":{"K":str(K),"linear_residual_norm":str(K),
              "declared_nilpotent_inverse_exact":True},
    "native_solver_tests":False,"benchmarks":False,"profiling":False,
    "status":"PASS"
}
print(json.dumps(receipt,indent=2,sort_keys=True))
