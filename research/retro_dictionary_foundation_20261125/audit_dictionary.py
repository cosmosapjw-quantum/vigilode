"""Independent exact-rational and high-precision checks of the actual Rust receipts."""
import hashlib,json,struct,math
from pathlib import Path
from fractions import Fraction as F
import mpmath as mp
root=Path(__file__).resolve().parent
repo=root.parent.parent
src=repo/'crates/rodas5p-core/data/retro_dictionary_v1'
mp.mp.dps=85
def f(h):return struct.unpack('>d',bytes.fromhex(h))[0]
def m(h):return mp.mpf(f(h))
def r(h):return F.from_float(f(h))
def records(mark):
    return [json.JSONDecoder().raw_decode(l.split(mark,1)[1])[0] for l in (root/'core_regression.log').read_text().splitlines() if mark+'{' in l]
counts={};ratios={}
def check(name,v,interval):
    lo,hi=map(m,interval);assert lo<=v<=hi,(name,str(v),str(lo),str(hi))
    counts[name]=counts.get(name,0)+1
    if hi>lo:ratios[name]=max(ratios.get(name,0),float(abs(v-(hi+lo)/2)/((hi-lo)/2)))
def phi(j,x):
    return mp.exp(-x) if j==0 else mp.hyp1f1(1,j+1,-x)/mp.factorial(j)
real=records('REAL ')
for row in real:
    x=m(row['x'])
    for j in range(9):
        v=phi(j,x)
        for k in ['phi','direct_phi','hybrid_phi']:check(k,v,row[k][j])
    for p in range(8):
        for k in range(p+1):
            j=p*(p+1)//2+k
            v=mp.hyp1f1(p-k+1,p+2,-x)/(p+1)
            for mode in ['moment','direct_moment','hybrid_moment']:check(mode,v,row[mode][j])
            if x>=64:check('scaled_fast',v,row['scaled_fast'][j])
lag=json.loads((src/'LAGRANGE_TABLE.json').read_bytes())['records']
for row in records('GREEN '):
    n=row['m'];a=m(row['a']);theta=m(row['theta']);l=row['l']
    # Integrate original monomial Lagrange basis with an independent
    # hypergeometric formula, not the Bernstein restriction/lift code.
    for c,iv in zip(lag[str(n)]['coefficients_ascending'],row['weights']):
        v=mp.mpf(0)
        for k,b in enumerate(c):
            v+=mp.mpf(b)*theta**k*mp.beta(l+1,k+1)*mp.hyp1f1(l+1,l+k+2,-a*theta)
        v*=theta**(l+1)/mp.factorial(l)
        check('green_weight',v,iv)
ls=json.loads((src/'UNIVERSAL_LEGENDRE.json').read_bytes())['linear_real_endpoint_rational']
for row in records('LS '):
    p=row['p'];z=r(row['z']);c=ls[str(p)]
    ev=lambda a:sum(F(v)*z**i for i,v in enumerate(a))
    v=ev(c['num_ascending'])/ev(c['den_ascending'])
    assert r(row['value'][0])<=v<=r(row['value'][1])
    counts['ls_exact_rational']=counts.get('ls_exact_rational',0)+1
# Orthogonal projection of exact Legendre polynomial products independently.
import sympy as sy
x=sy.symbols('x');L=[sy.legendre(i,x) for i in range(17)]
for row in records('PRODUCT '):
    i,j=row['i'],row['j'];a=sy.Poly(L[i]*L[j],x)
    for k,iv in enumerate(row['values']):
        if k>i+j or k<abs(i-j) or (i+j-k)%2:v=F(0)
        else:
            b=sy.Poly(a.as_expr()*L[k],x)
            v=F(str(sum(c*sy.Rational(2,e[0]+1) for e,c in b.terms() if e[0]%2==0)*(2*k+1)/2))
        assert r(iv[0])<=v<=r(iv[1])
        counts['legendre_exact_rational']=counts.get('legendre_exact_rational',0)+1
max_complex=0.
for row in records('COMPLEX '):
    z=mp.mpc(m(row['re']),m(row['im']))
    for j,c in enumerate(row['values']):
        ref=mp.exp(z) if j==0 else mp.hyp1f1(1,j+1,z)/mp.factorial(j)
        error=float(abs(mp.mpc(m(c[0]),m(c[1]))-ref))
        max_complex=max(max_complex,error);counts['complex_numerical']=counts.get('complex_numerical',0)+1
assert max_complex<5e-14
result={'real_points':len(real),'counts':counts,'max_center_error_over_radius':ratios,'underbounds':0,'complex_max_absolute_error':max_complex,'mpmath':mp.__version__,'sympy':sy.__version__}
(root/'DICTIONARY_AUDIT.json').write_text(json.dumps(result,indent=2,sort_keys=True)+'\n');print(json.dumps(result,indent=2))
