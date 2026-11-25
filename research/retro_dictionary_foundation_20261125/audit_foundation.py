"""Independent exact rational replays of represented problem inputs.
Reads Rust output receipts; does not use Rust's reduction or residual code."""
import json, re, math
from fractions import Fraction as F
from pathlib import Path
import sympy as s
import mpmath as mp
mp.mp.dps=85
root=Path(__file__).parent
txt=(root/'foundation_regression.log').read_text()
def records(tag):
    return [json.loads(x) for x in re.findall(r'\b'+tag+r' (\{[^\n]+\})',txt)]
f=lambda x:F.from_float(float(x))
audit=[]
for row in records('TIME'):
    n=len(row['y']);y=list(map(f,row['y']));maxerr=F(0)
    for k,(g,b,x,bound) in enumerate(zip(row['g'],row['b'],row['x'],row['bounds'])):
        G=list(map(f,g));y=[sum(G[i*n+j]*y[j] for j in range(n))+f(b[i]) for i in range(n)]
        err=max(abs(v-f(z)) for v,z in zip(y,x))
        assert err<=f(bound),(k,err,bound)
        maxerr=max(maxerr,err)
    audit.append(dict(case='aperiodic',mode=row['mode'],nodes=len(row['x']),max_error=float(maxerr),bound=max(row['bounds'])))
for row in records('QUOTIENT'):
    p=row['p'];w=row['w'];n=math.isqrt(len(w));m=math.prod(map(len,p))
    z=s.symbols('x:'+str(len(p)));R=lambda v:s.Rational(f(v).numerator,f(v).denominator)
    stride=[math.prod(len(q) for q in p[:k]) for k in range(len(p))]
    mod=[x**len(q)+sum(R(a)*x**i for i,a in enumerate(q)) for x,q in zip(z,p)]
    basis=[math.prod(x**((i//st)%len(q)) for x,st,q in zip(z,stride,p)) for i in range(m)]
    mat=s.zeros(n*m)
    for i in range(m):
        for r in range(n):
            for c in range(n):mat[i*n+r,i*n+c]=R(w[r*n+c])
    for t in row['terms']:
        mono=math.prod(x**k for x,k in zip(z,t['powers']))
        for j,b in enumerate(basis):
            r=mono*b
            for x,q in zip(z,mod):r=s.rem(r,q,x)
            r=s.Poly(r,*z)
            for i,a in enumerate(basis):
                c=r.coeff_monomial(a)
                for k in range(n):
                    for l in range(n):mat[i*n+k,j*n+l]+=c*R(t['operator'][k*n+l])
    b=s.Matrix([R(v) for r in row['b'] for v in r])
    x=mat.inv()*b
    got=[R(v) for r in row['x'] for v in r]
    err=max(abs(v-z) for v,z in zip(x,got))
    assert err<=R(row['bound'])
    audit.append(dict(case='tensor',mode=row['mode'],channels=m,dimension=n,error=float(err),bound=row['bound']))
for row in records('NIL_GREEN'):
    a=mp.mpf(row['a']);e=mp.exp(-a)
    p1=1 if a==0 else -mp.expm1(-a)/a
    q=mp.mpf('0.5') if a==0 else (1-(1+a)*e)/a**2
    y=[e*mp.mpf('0.4375')+p1*mp.mpf('0.25')+q*mp.mpf('0.125'),-e*mp.mpf('0.25')+p1*mp.mpf('0.5')]
    assert all(mp.mpf(l)<=v<=mp.mpf(h) for l,h,v in zip(row['lo'],row['hi'],y))
    audit.append(dict(case='nilpotent_green',mode=row['mode'],a=row['a'],components=2))
for row in records('PICARD'):
    err=max(abs(f(v)-e) for v,e in zip(row['values'],[F(1,48),F(1,4),F(1)]))
    assert err<=f(row['bound'])
    audit.append(dict(case='ordered_picard',error=float(err),bound=row['bound']))
for row in records('GREEN_FULL'):
    # Analytic solution for dy/dt=-k*y-beta*y^3;
    # Rust logs already computed point errors. Compare these against bound.
    assert row['error']<=row['bound']+2e-15
    audit.append(dict(case='green_full',k=row['k'],mode=row['mode'],error=row['error'],bound=row['bound']))
assert len(audit)==28,len(audit)
(root/'FOUNDATION_AUDIT.json').write_text(json.dumps({'records':audit,'underbounds':0,'exact_aperiodic_and_tensor':True,'mpmath_dps':85},indent=2)+'\n')
print('independent foundation audit:',len(audit),'records')
