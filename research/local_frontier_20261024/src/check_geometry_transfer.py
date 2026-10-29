#!/usr/bin/env python3
"""Exact-rational checks of the solver transfer; no manuscript revalidation."""
from fractions import Fraction as Q
import json
from pathlib import Path

def mat(a): return [[Q(v) for v in row] for row in a]
def mm(a,b): return [[sum(x*y for x,y in zip(row,col)) for col in zip(*b)] for row in a]
def add(a,b): return [[x+y for x,y in zip(ar,br)] for ar,br in zip(a,b)]
def neg(a): return [[-x for x in row] for row in a]
def scale(s,a): return [[s*x for x in row] for row in a]
def eye(n): return [[Q(i==j) for j in range(n)] for i in range(n)]
def inverse(a):
    n=len(a);b=[row[:]+i for row,i in zip(a,eye(n))]
    for k in range(n):
        p=next(i for i in range(k,n) if b[i][k]);b[k],b[p]=b[p],b[k]
        d=b[k][k];b[k]=[v/d for v in b[k]]
        for i in range(n):
            if i!=k:
                d=b[i][k];b[i]=[x-d*y for x,y in zip(b[i],b[k])]
    return [row[n:] for row in b]

B=mat([[-2,1,0,0],[0,-2,0,0],[0,0,-2,1],[0,0,0,-2]])
V=mat([[1,0],[0,1],[1,0],[0,1]])
W=mat([[1,0,0,0],[0,1,0,0]])
H=mat([[-2,1],[0,-2]])
b=mat([[1],[2],[1],[2]])
assert mm(W,V)==eye(2)
assert mm(B,V)==mm(V,H)
checks=[{'id':'exact_module','result':'pass'}]
for alpha in [Q(0),Q(1,4),Q(1),Q(100)]:
    reduced=mm(V,mm(inverse(add(eye(2),scale(-alpha,H))),mm(W,b)))
    full=mm(inverse(add(eye(4),scale(-alpha,B))),b)
    assert reduced==full
checks.append({'id':'four_shift_exact_resolvent','result':'pass'})
# Defect identity permits a nonzero reduced-solve defect and leakage.
Bp=[row[:] for row in B];Bp[2][2]-=Q(1,100)
z=mat([[Q(1,3)],[Q(2,3)]]);alpha=Q(1)
E=add(mm(Bp,V),neg(mm(V,H)))
delta=add(mm(W,b),neg(mm(add(eye(2),scale(-alpha,H)),z)))
r=add(b,neg(mm(add(eye(4),scale(-alpha,Bp)),mm(V,z))))
rhs=add(add(add(b,neg(mm(V,mm(W,b)))),mm(V,delta)),scale(alpha,mm(E,z)))
assert r==rhs
checks.append({'id':'original_residual_decomposition_with_leakage','result':'pass'})
# Keep the length-two local algebra at the repeated root.
exact=mm(inverse(add(eye(2),neg(H))),mat([[1],[2]]))
collapsed=scale(Q(1,3),mat([[1],[2]]))
assert exact[0][0]-collapsed[0][0]==Q(2,9)
checks.append({'id':'jordan_multiplicity_required','result':'pass','collapsed_error':'2/9'})
S=mat([[2,1],[0,1]]);Si=inverse(S)
Vp=mm(V,S);Wp=mm(Si,W);Hp=mm(Si,mm(H,S))
assert mm(Wp,Vp)==eye(2) and mm(B,Vp)==mm(Vp,Hp)
assert mm(Vp,mm(inverse(add(eye(2),neg(Hp))),mm(Wp,b)))==mm(V,mm(inverse(add(eye(2),neg(H))),mm(W,b)))
checks.append({'id':'regular_chart_similarity','result':'pass'})
# The fold derivative vanishes at the wall; a fixed positive inverse cannot
# have contraction q<1 on a ball containing x=0 for F=x^2-lambda.
assert Q(1)-Q(2)*Q(1)*Q(0)==1
checks.append({'id':'fold_wall_local_guard','result':'pass','scope':'specific scalar fold, no general no-go'})
result={'schema_version':1,'method':'Python fractions exact rational arithmetic','checks':checks,
        'scope':'New transfer identities only; historical manuscript certificates reused, not rerun.'}
dest=Path(__file__).resolve().parents[1]/'analysis/GEOMETRY_EXACT_CHECKS.json'
dest.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
