"""Independent exact checks of newly adopted algebraic contracts (standard library)."""
from fractions import Fraction as F
from decimal import Decimal, localcontext
import json
from pathlib import Path

def mm(a,b):
    return [[sum(x*y for x,y in zip(row,col)) for col in zip(*b)] for row in a]
def add(a,b):return [[x+y for x,y in zip(r,s)] for r,s in zip(a,b)]
def identity(n):return [[F(i==j) for j in range(n)] for i in range(n)]

checks=[]
for n in [2,5]:
    l=[[F((i+1)*(j+2),7) if i>j else F(0) for j in range(n)] for i in range(n)]
    p=identity(n); total=identity(n)
    for _ in range(1,n):p=mm(p,l);total=add(total,p)
    iml=[[F(i==j)-l[i][j] for j in range(n)] for i in range(n)]
    assert mm(iml,total)==identity(n)
    checks.append({'id':f'nilpotent-{n}','passed':True})
    exact=[]
    for i in range(n):exact.append(F(i+1)+sum(F(j+1,10)*x*x for j,x in enumerate(exact)))
    old=[F(-3) for _ in range(n)]
    for k in range(n):
        old=[F(i+1)+sum(F(j+1,10)*old[j]*old[j] for j in range(i)) for i in range(n)]
        assert old[:k+1]==exact[:k+1]
    checks.append({'id':f'nonlinear-front-{n}','passed':True,'global_contraction_assumed':False})

for a,c,u in [(F(3),F(1,7),F(2)),(F(2),F(-1,10),F(1)),(F(1,2),F(2),F(3,5))]:
    b=c*u*u+a*u
    dr=a+2*c*u
    assert dr>0 and dr*dr==a*a+4*c*b
    for du in [F(-1,100),F(1,7)]:
        v=u+du
        assert a+2*c*v>=0
        assert (v-u)*(a+2*c*v+dr)==2*(c*v*v+a*v-b)
    checks.append({'id':f'quadratic-{len(checks)}','passed':True})

for d,w,s in [(F(0),F(3),F(2,5)),(F(7),F(-2),F(3)),(F(2),F(0),F(0))]:
    a=1+s*d;v=s*w;den=a*a+v*v
    z=[[a/den,-v/den],[v/den,a/den]]
    assert mm([[a,v],[-v,a]],z)==identity(2)
    checks.append({'id':f'rotation-{len(checks)}','passed':True})

with localcontext() as ctx:
    ctx.prec=90
    references={str(d):str((Decimal(-d).exp())*Decimal('1e308')) for d in [744,1000,1500]}
out={'schema':'vigilode.independent-algebra-check.v1','checks':checks,'passed':all(x['passed'] for x in checks),'decimal_damped_large_state_references':references,'scope':'Exact rational identities and high-precision extreme-input references; no solver trajectory timing.'}
path=Path(__file__).resolve().parents[1]/'results/independent_review_checks.json'
path.write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out,indent=2))
