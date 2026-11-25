"""Transcribed archived Round01 generator table sections; offline intake only.
No online generation. Accept exact prior bytes by the SOURCE_INDEX.tsv hashes."""
import json,math,itertools
from pathlib import Path
import numpy as np
from numpy.polynomial.legendre import leggauss
import sympy as sy
from sympy.physics.wigner import wigner_3j
import mpmath as mp
R=Path(__file__).resolve().parent/'restored'
R.mkdir(exist_ok=True)
def dump(n,obj): (R/n).write_text(json.dumps(obj,indent=2,sort_keys=True,allow_nan=False)+'\n')
z,t=sy.symbols('z t');tables={};rtables={}
for p in range(1,7):
    L=[sy.Poly(sy.legendre(j,2*t-1),t) for j in range(p)]
    I=[sy.Poly(sy.integrate(a.as_expr(),t)-sy.integrate(a.as_expr(),t).subs(t,0),t) for a in L]
    integ=lambda a:sy.integrate(a.as_expr(),(t,0,1))
    G=sy.diag(*[sy.Rational(1,2*j+1) for j in range(p)])
    M=sy.Matrix(p,p,lambda i,j:integ(I[i]*I[j])); C=sy.Matrix(p,p,lambda i,j:integ(L[i]*I[j]))
    l=sy.Matrix([a.eval(1) for a in I]);v=sy.Matrix([integ(a) for a in I]);d=sy.Matrix([integ(a) for a in L])
    A=G-z*(C+C.T)+z*z*M; rhs=z*d-z*z*v
    den=A.det(method='domain-ge');num=(A+rhs*l.T).det(method='domain-ge');RR=sy.cancel(num/den);nn,dd=sy.fraction(RR)
    assert sy.limit(RR,z,-sy.oo)==sy.Rational((-1)**p,p+1)
    series=sy.series(RR-sy.exp(z),z,0,2*p+3).removeO().expand();lead=min((k for k in range(2*p+3) if series.coeff(z,k)!=0),default=None)
    def serialize(A):return [[str(a) for a in row] for row in A.tolist()]
    def asc(poly,var):return [str(a) for a in sy.Poly(poly,var).all_coeffs()[::-1]]
    q=3*p+1;xn,wn=leggauss(q);xs=(xn+1)/2;weights=wn/2
    tables[str(p)]={'L':[asc(a.as_expr(),t) for a in L],'I':[asc(a.as_expr(),t) for a in I],'G':serialize(G),'M':serialize(M),'C':serialize(C),'ell':list(map(str,l)),'d':list(map(str,d)),'v':list(map(str,v)),'gauss_nodes':xs.tolist(),'gauss_weights':weights.tolist(),'q':q}
    rtables[str(p)]={'num_ascending':asc(nn,z),'den_ascending':asc(dd,z),'first_nonzero_error_power':lead,'leading_error_coefficient':str(series.coeff(z,lead)) if lead is not None else None,'stiff_limit':str(sy.Rational((-1)**p,p+1))}
    assert all(abs(sum(weights*xs**j)-1/(j+1))<5e-14 for j in range(2*q))
    print('LS',p,flush=True)
dump('UNIVERSAL_LEGENDRE.json',{'basis':'L_j=P_j(2 theta-1), I_j=int_0^theta L_j','degree_range':[1,6],'records':tables,'linear_real_endpoint_rational':rtables})
mp.mp.dps=75;degree=20;max_j=8
edges=[mp.mpf(0),mp.mpf(1)]
for k in range(14):
    a=mp.mpf(2)**k;edges.extend(a*(1+mp.mpf(i)/4) for i in range(1,5))
cells=[]
for xa,xb in zip(edges[:-1],edges[1:]):
    c=-(xa+xb)/2;r=(xb-xa)/2;a=xa;coeff=[];tails=[]
    for j in range(max_j+1):
        if j==0:
            co=[mp.exp(c)*r**k/mp.factorial(k) for k in range(degree+1)]
            tail=mp.exp(-a)*r**(degree+1)/mp.factorial(degree+1)
        else:
            co=[r**k*mp.hyp1f1(k+1,k+j+1,c)/mp.factorial(k+j) for k in range(degree+1)]
            tail=r**(degree+1)/mp.factorial(degree+1+j)
            if a>0:tail=min(tail,r**(degree+1)/(a**(degree+2)*mp.factorial(j-1)))
        coeff.append(list(map(float,co)));tails.append(float(tail))
    cells.append({'lo':float(-xb),'hi':float(-xa),'center':float(c),'radius':float(r),'coefficients':coeff,'analytic_truncation_upper_approx':tails})
dump('PHI_TABLE.json',{'z_domain':[-16384,0],'j_range':[0,max_j],'degree':degree,'generator_precision_digits':75,'cells':cells,'notes':'analytic truncation bounds approximate stored values, not interval rounding certificate'})
x=sy.symbols('x');tables={}
for m in [3,5,7]:
    nodes=[sy.Rational(i,m-1) for i in range(m)]
    L=[sy.Poly(sy.prod((x-a)/(b-a) for a in nodes if a!=b),x) for b in nodes]
    C=[[str(l.nth(i)) for i in range(m)] for l in L]
    assert all(l.eval(b)==int(i==j) for i,l in enumerate(L) for j,b in enumerate(nodes))
    tables[str(m)]={'nodes':list(map(str,nodes)),'coefficients_ascending':C}
(R/'LAGRANGE_TABLE.json').write_text(json.dumps({'schema':'green-collocation.v1','records':tables},indent=2)+'\n')
entries=[];pmax=8
poly=[sy.legendre(i,z) for i in range(2*pmax+1)]
for i in range(pmax+1):
    for j in range(pmax+1):
        terms=[]
        for k in range(abs(i-j),i+j+1,2):
            v=sy.simplify((2*k+1)*wigner_3j(i,j,k,0,0,0)**2)
            if v:terms.append((k,str(v)));entries.append([i,j,k,str(v)])
        assert sy.expand(poly[i]*poly[j]-sum(sy.Rational(v)*poly[k] for k,v in terms))==0
(R/'LEGENDRE_PRODUCT_TABLE.json').write_text(json.dumps({'schema':'shifted-legendre-products.v1','input_max_degree':8,'output_max_degree':16,'entries':[{'i':i,'j':j,'k':k,'value':v} for i,j,k,v in entries]},indent=2)+'\n')
print('round1 restored',flush=True)
