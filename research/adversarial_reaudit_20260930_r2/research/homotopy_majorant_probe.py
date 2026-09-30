#!/usr/bin/env python3
"""Constructive quadratic stage enclosures for homotopy candidate output error.
Exact Fraction fixture proves case inequalities without a floating safety fudge.
Latest RODAS tableau is separately tested numerically; not directed-rounding cert.
"""
from pathlib import Path
from fractions import Fraction as F
import json,math,hashlib,platform,argparse
import numpy as np
OUT=Path(__file__).resolve().parent
parser=argparse.ArgumentParser()
parser.add_argument('--tableau',type=Path,default=None)
args=parser.parse_args()
if args.tableau is not None:
    SRC=args.tableau
else:
    candidates=[parent/rel for parent in [OUT,*OUT.parents] for rel in ['fixtures/rodas5p_coefficients_snapshot.json','vigilode/fixtures/rodas5p_coefficients_snapshot.json']]
    SRC=next((p for p in candidates if p.is_file()),None)
    if SRC is None:raise FileNotFoundError('Pass --tableau /path/to/pinned/rodas5p_coefficients_snapshot.json')
raw=json.loads(SRC.read_text())
def num(x):return float(F(x))
gamma=num(raw['gamma']);A=np.array([[num(x) for x in row] for row in raw['A']]);C=np.array([[num(x) for x in row] for row in raw['C']]);s=len(A)
G=np.linalg.inv(np.eye(s)/gamma-C);alpha=np.tril(A@G,-1);L=np.tril(alpha+G,-1)
b=G.T@np.array([num(x) for x in raw['b_code']])
class System:
    def __init__(self,J,y,h,q):
        self.J=np.array(J);self.y=np.array(y);self.h=h;self.q=q;self.W=np.eye(len(y))-h*gamma*self.J
        self.rhs=np.tile(h*(self.J@self.y-q*self.y*self.y),(s,1));self.batches=0
    def solve(self,r):self.batches+=1;return np.linalg.solve(self.W,np.asarray(r).T).T
    def coupling(self,k):return self.h*(L@k)@self.J.T
    def N(self,k):return self.q*(alpha@k)**2
    def residual(self,k):return k@self.W.T-self.coupling(k)-self.rhs-self.h*self.N(k)
    def P1(self,r):
        t0=self.solve(r);t1=self.solve(self.coupling(t0));return t0+t1,t1
    def exact(self):
        k=np.zeros_like(self.rhs)
        for i in range(s):k[i]=np.linalg.solve(self.W,self.rhs[i]+self.h*self.J@(L[i]@k)+self.h*self.q*(alpha[i]@k)**2)
        return k
    def candidates(self):
        k=self.solve(self.rhs);k+=self.solve(self.coupling(k)+self.h*self.N(k))
        for _ in range(2):corr,t1=self.P1(self.residual(k));k-=corr
        q1=k.copy();q1_batches=self.batches
        k-=self.solve(self.coupling(t1));self.solve(self.residual(k))
        return [(q1,q1_batches,'q1'),(k.copy(),self.batches,'q2')]
    def majorant(self,k):
        # Infinity vector norm. This implementation is numerical, not interval.
        r=self.residual(k);winv=float(np.linalg.norm(np.linalg.inv(self.W),np.inf));jnorm=float(np.linalg.norm(self.J,np.inf))
        E=np.zeros(s);radius=np.zeros(s);ell=np.zeros(s)
        for i in range(s):
            radius[i]=np.abs(alpha[i,:i])@E[:i];dhat=float(np.linalg.norm(alpha[i]@k,np.inf))
            ell[i]=abs(self.q)*(2*dhat+radius[i])
            E[i]=winv*(float(np.linalg.norm(r[i],np.inf))+self.h*jnorm*(np.abs(L[i,:i])@E[:i])+self.h*ell[i]*radius[i])
        return E,radius,ell
    def componentwise_majorant(self,k):
        # Componentwise absolute operator keeps spatial triangular structure.
        r=self.residual(k);invabs=np.abs(np.linalg.inv(self.W));E=np.zeros_like(k)
        for i in range(s):
            rad=np.abs(alpha[i,:i])@E[:i]
            linear=np.abs(self.J)@(np.abs(L[i,:i])@E[:i])
            remainder=abs(self.q)*(2*np.abs(alpha[i]@k)*rad+rad**2)
            E[i]=invabs@(np.abs(r[i])+self.h*(linear+remainder))
        return E,np.abs(b)@E

def dot(x,y):return sum((a*b for a,b in zip(x,y)),F(0))
def matvec(m,x):return [dot(row,x) for row in m]
def add(x,y):return [a+b for a,b in zip(x,y)]
def sub(x,y):return [a-b for a,b in zip(x,y)]
def scale(a,x):return [a*b for b in x]

def rational_fixture(h,nonlinear):
    n=8;gam=F(1,4);j=F(-1);q=F(1,4) if nonlinear else F(0);W=1-h*gam*j
    al=[[F(0) for _ in range(n)] for _ in range(n)];lo=[[F(0) for _ in range(n)] for _ in range(n)]
    for i in range(1,n):
        al[i][i-1]=F(1,2);lo[i][i-1]=F(1,5)
        if i>1:al[i][i-2]=F(-1,10);lo[i][i-2]=F(-1,7)
    weights=[F(1 if i%2==0 else -1,8) for i in range(n)];rhs=[h*(j-q)]*n
    def solve(r):return scale(1/W,r)
    def coup(k):return scale(h*j,matvec(lo,k))
    def rem(k):return [h*q*x*x for x in matvec(al,k)]
    def res(k):return sub(sub(sub(scale(W,k),coup(k)),rhs),rem(k))
    exact=[F(0)]*n
    for i in range(n):exact[i]=(rhs[i]+h*j*dot(lo[i],exact)+h*q*dot(al[i],exact)**2)/W
    k=solve(rhs);k=add(k,solve(add(coup(k),rem(k))))
    for _ in range(2):t0=solve(res(k));t1=solve(coup(t0));k=sub(k,add(t0,t1))
    candidates=[('q1',k.copy(),6)]
    candidates.append(('q2',sub(k,solve(coup(t1))),8))
    records=[]
    for name,k,depth in candidates:
        r=res(k);E=[F(0)]*n;rad=[F(0)]*n
        for i in range(n):
            rad[i]=dot([abs(x) for x in al[i]],E);d=abs(dot(al[i],k))
            E[i]=(abs(r[i])+h*abs(j)*dot([abs(x) for x in lo[i]],E)+h*abs(q)*(2*d*rad[i]+rad[i]**2))/abs(W)
        endpoint=abs(dot(weights,sub(k,exact)));bound=dot([abs(x) for x in weights],E)
        stageok=all(abs(k[i]-exact[i])<=E[i] for i in range(n));enclosed=all(abs(dot(al[i],sub(k,exact)))<=rad[i] for i in range(n))
        records.append(dict(family='nonlinear' if nonlinear else 'affine',h=str(h),method=name,W_batches=depth,
            actual_output_error=float(endpoint),output_bound=float(bound),stage_inequalities_exact=stageok,
            state_enclosures_exact=enclosed,output_inequality_exact=endpoint<=bound,
            certificate_accept_1e_8=bound<=F(1,10**8),actual_pass_1e_8=endpoint<=F(1,10**8),
            exact_bound_sha256=hashlib.sha256(str(bound).encode()).hexdigest(),exact_bound_numerator_bits=bound.numerator.bit_length()))
    return records

def run():
    rational=[]
    for h in [F(1,10),F(1),F(3)]:
        for nonlinear in [False,True]:rational+=rational_fixture(h,nonlinear)
    rows=[]
    for family,J,y,q in [('affine',[[-1.]],[1.],0.),('nonnormal',[[-1.,100.],[0.,-1.]],[0.,1.],0.),('quadratic',[[-1.]],[1.],.25)]:
        for h in [.01,.1,1.,10.]:
            sys=System(J,y,h,q);ref=sys.exact()
            for k,depth,method in sys.candidates():
                E,rad,ell=sys.majorant(k);Ec,Bc=sys.componentwise_majorant(k);actual=float(np.linalg.norm(b@(k-ref),np.inf));bound=float(np.abs(b)@E)
                roundtol=1e-12*max(1.,float(np.linalg.norm(ref,np.inf)))
                rows.append(dict(family=family,h=h,method=method,W_batches=depth,W_vector_solves=depth*s,
                    output_error=actual,output_bound_nondirected=bound,componentwise_bound_nondirected=float(max(Bc)),componentwise_pass_1e_8=bool(max(Bc)<=1e-8),componentwise_numerical_check=bool(np.all(np.abs(k-ref)<=Ec+roundtol)),majorant_over_error=bound/actual if actual else None,
                    stage_majorant_numerical_check=bool(np.all(np.max(np.abs(k-ref),axis=1)<=E+roundtol)),
                    output_majorant_numerical_check=actual<=bound+roundtol,
                    arithmetic_check_tolerance=roundtol,pass_bound_1e_8=bound<=1e-8,actual_pass_1e_8=actual<=1e-8,
                    max_enclosure_radius=float(max(rad)),max_remainder_lipschitz=float(max(ell))))
    checks=dict(exact_rational_stage_bounds=all(r['stage_inequalities_exact'] for r in rational),
        exact_rational_enclosures=all(r['state_enclosures_exact'] for r in rational),
        exact_rational_output_bounds=all(r['output_inequality_exact'] for r in rational),
        exact_rational_no_false_accept=all(not r['certificate_accept_1e_8'] or r['actual_pass_1e_8'] for r in rational),
        actual_tableau_numerical_inequalities=all(r['stage_majorant_numerical_check'] and r['output_majorant_numerical_check'] and r['componentwise_numerical_check'] for r in rows))
    result=dict(schema='vigilode.reaudit.homotopy_majorant.v1',source_commit='7708ef90554fc3986478d4602de6a01c7266b14f',
        tableau_sha256=hashlib.sha256(SRC.read_bytes()).hexdigest(),runtime={'python':platform.python_version(),'numpy':np.__version__},
        claim_ceiling='derived quadratic enclosure theorem; exact rational toy implementation verified; actual tableau only numerically checked; no production certificate or wall speedup',
        metric='stage/output infinity norm; exact rational toy distinct from source tableau; production WRMS conversion pending',
        checks=checks,exact_rational_rows=rational,actual_tableau_rows=rows,
        work_scope='W candidate batches only, scalar majorant O(s^2), inverse norm witness/residual bound construction and directed rounding cost not benchmarked')
    (OUT/'homotopy_majorant_results.json').write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
    print(json.dumps(result,indent=2));assert all(checks.values())
if __name__=='__main__':run()
