#!/usr/bin/env python3
"""Exact rational enclosure oracle; no floating transcendental library is used.

Checks the actual represented symmetric matrix by exact rational Q^T Q=I and
A=Q diag(lambda) Q^T. Positive exp Taylor series plus a proved geometric tail
(or alternating phi series near zero) give exact rational phi intervals.
The sum of squared *upper* component distances must fit the reported bound².
"""
import argparse
from fractions import Fraction as F
from functools import lru_cache
import json
from math import factorial
from pathlib import Path
import struct

EPS = F(1, 1 << 1500)
def number(s):
    return F.from_float(struct.unpack('>d', bytes.fromhex(s))[0])
def add(x,y): return x[0]+y[0], x[1]+y[1]
def scale(x,a): return (x[0]*a,x[1]*a) if a>=0 else (x[1]*a,x[0]*a)
@lru_cache(None)
def phis(z):
    assert z <= 0
    if z == 0: return tuple((F(1,factorial(k)),)*2 for k in range(5))
    if -z <= 1:
        out=[]
        for k in range(5):
            term=F(1,factorial(k)); total=term
            for j in range(1,10000):
                term *= z/F(j+k); total += term
                following=term*z/F(j+k+1)
                if abs(following) < EPS:
                    out.append((min(total,total+following),max(total,total+following)));break
            else: raise AssertionError('near-zero series cap')
        return tuple(out)
    x=-z; total=F(1); term=F(1)
    for j in range(1,10000):
        term*=x/j; total+=term; ratio=x/(j+1)
        if ratio<1:
            tail=term*ratio/(1-ratio)
            if tail<EPS:
                exponential=(1/(total+tail),1/total);break
    else: raise AssertionError('exp series cap')
    out=[exponential]
    for k in range(1,5):
        previous=add(out[-1],(-F(1,factorial(k-1)),)*2)
        out.append(scale(previous,1/z))
    return tuple(out)

def check(row):
    eigenvalues=list(map(number,row['eigenvalues'])); n=len(eigenvalues)
    q=[list(map(number,v)) for v in row['q']]
    matrix=[list(map(number,v)) for v in row['matrix']]
    for i in range(n):
        for j in range(n):
            assert sum(q[t][i]*q[t][j] for t in range(n)) == (1 if i==j else 0)
            assert sum(q[i][t]*eigenvalues[t]*q[j][t] for t in range(n)) == matrix[i][j]
    vectors=[list(map(number,v)) for v in row['vectors']]
    scales=list(map(number,row['scales']))
    if row['same_vector']:
        vectors=[[s*x for x in vectors[0]] for s in scales]
    h=number(row['h']); target=[(F(0),F(0)) for _ in range(n)]
    for a,eig in enumerate(eigenvalues):
        phi=phis(h*eig)
        for k in range(5):
            projection=sum(q[j][a]*vectors[k][j] for j in range(n))
            for i in range(n): target[i]=add(target[i],scale(phi[k],q[i][a]*projection))
    fused=list(map(number,row['fused'])); bound=number(row['bound'])
    squared=sum(max(abs(x-lo),abs(x-hi))**2 for x,(lo,hi) in zip(fused,target))
    assert squared <= bound**2, (row['name'],row['same_vector'],'enclosure violated')
    return {'name':row['name'],'same_vector':row['same_vector'], 'contained':True,
            'degree':row['degree'],'converged':row['converged'],
            'squared_error_over_bound_squared_upper':float(squared/bound**2) if bound else 0.0}

def main():
    p=argparse.ArgumentParser();p.add_argument('runs');p.add_argument('--output');a=p.parse_args()
    doc=json.loads(Path(a.runs).read_text()); rows=[check(r) for r in doc['rows']]
    output={'schema':'vigilode-leja-fraction-check-v1','method':'exact rational spectral identity and phi Taylor enclosures','precision_tail_bits':1500,'cases':len(rows),'passed':True,'rows':rows}
    text=json.dumps(output,indent=2)+'\n'
    if a.output: Path(a.output).write_text(text)
    else: print(text,end='')
if __name__=='__main__':main()
