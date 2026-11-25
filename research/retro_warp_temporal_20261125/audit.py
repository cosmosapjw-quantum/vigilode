"""Independent exact-binary recurrence audit; exact analytic shear flow reference.
Also tests an aperiodic frozen-period correction in NumPy, not the Rust solver.
"""
import json,struct,math
from fractions import Fraction as F
from pathlib import Path
import mpmath as mp
import numpy as np
from scipy.linalg import solve
root=Path(__file__).resolve().parent
mp.mp.dps=100
def f(h):return F.from_float(struct.unpack(">d",bytes.fromhex(h))[0])
def mf(x):return mp.mpf(x.numerator)/x.denominator
def mv(a,x):
    n=len(x)
    return [sum(a[i*n+j]*x[j] for j in range(n)) for i in range(n)]
def inf(x):return max(map(abs,x))
def records(name,marker):
    return [json.JSONDecoder().raw_decode(line.split(marker,1)[1])[0]
            for line in (root/name).read_text().splitlines() if marker in line]
cases=records('regression.log','EXACT_CASE ')
assert len(cases)==4
audit=[]
for case in cases:
    pattern=[list(map(f,row)) for row in case['pattern']]
    y=list(map(f,case['y']))
    b=[list(map(f,row)) for row in case['b']]
    x=[list(map(f,row)) for row in case['x']]
    force=[list(map(f,row)) for row in case['forcing']]
    c=case['certificate'];exact=y[:];previous=y[:];continuous=list(map(mf,y))
    count=0;maxerr=F(0);maxres=F(0);maxratio=F(0)
    rep_error=mp.mpf(0);continuous_error=mp.mpf(0)
    h=mp.mpf(1)/16;z=4*h
    a=mp.exp(-z);p1=-mp.expm1(-z)/z;j1=(1-(1+z)*a)/z**2
    for i,(bb,xx,ff) in enumerate(zip(b,x,force)):
        g=pattern[i%2]
        exact=[v+w for v,w in zip(mv(g,exact),bb)]
        r=inf([v+w-xxj for v,w,xxj in zip(mv(g,previous),bb,xx)])
        e=inf([v-w for v,w in zip(exact,xx)])
        bound=F.from_float(c['node_error_upper'][i])
        assert r<=F.from_float(c['residual_upper'])
        assert e<=bound,(case['mode'],i,e,bound)
        directed=case['directed_certificate']
        assert e<=F.from_float(directed['node_error_upper'][i])
        assert r<=F.from_float(directed['residual_upper'])
        if bound:maxratio=max(maxratio,e/bound)
        previous=xx;maxerr=max(maxerr,e);maxres=max(maxres,r);count+=1
        # Exact continuous-flow oracle, independent of represented G and b.
        S=[0,2,0,0] if i%2==0 else [0,0,-3,0]
        sy=mv(S,continuous);ff=list(map(mf,ff));sf=mv(S,ff)
        continuous=[a*(v+h*w)+h*p1*u+h*h*j1*s for v,w,u,s in zip(continuous,sy,ff,sf)]
        rep_error=max(rep_error,inf([mf(v)-w for v,w in zip(exact,continuous)]))
        continuous_error=max(continuous_error,inf([mf(v)-w for v,w in zip(xx,continuous)]))
    assert all(abs(a-b)<=F.from_float(c['endpoint_component_upper'][i]) for i,(a,b) in enumerate(zip(previous,exact)))
    assert c['accepted'] and maxerr<=F.from_float(c['error_upper'])
    audit.append({'mode':case['mode'],'nodes':count,'components':len(y)*count,
                  'max_exact_represented_error':float(maxerr),'max_exact_residual':float(maxres),
                  'max_error_over_node_bound':float(maxratio),'representation_error_vs_continuous':float(rep_error),
                  'candidate_error_vs_continuous':float(continuous_error),
                  'error_upper':c['error_upper'],'directed_error_upper':case['directed_certificate']['error_upper']})
scale_audit=[]
for case in records('regression.log','SCALE_CASE '):
    g=list(map(f,case['pattern'][0]));y=list(map(f,case['y']))
    exact=y[:];prev=y[:];err=F(0);res=F(0)
    for i,(b,x) in enumerate(zip(case['b'],case['x'])):
        b=list(map(f,b));x=list(map(f,x))
        exact=[a+v for a,v in zip(mv(g,exact),b)]
        e=inf([a-v for a,v in zip(exact,x)])
        r=inf([a+v-w for a,v,w in zip(mv(g,prev),b,x)])
        for key in ('certificate','directed_certificate'):
            c=case[key]
            assert e<=F.from_float(c['node_error_upper'][i]),(key,case['scale'],i,e)
            assert r<=F.from_float(c['residual_upper'])
        err=max(err,e);res=max(res,r);prev=x
    for key in ('certificate','directed_certificate'):
        c=case[key]
        assert all(abs(a-b)<=F.from_float(c['endpoint_component_upper'][i]) for i,(a,b) in enumerate(zip(prev,exact)))
    scale_audit.append({'scale_bits':case['scale'],'nodes':len(case['b']),
        'max_error':float(err),'max_residual':float(res),'fast_upper':case['certificate']['error_upper'],
        'directed_upper':case['directed_certificate']['error_upper'],'accepted':case['certificate']['accepted']})
assert len(scale_audit)==4
# New theory: nonperiodic actual maps, frozen ordered period as preconditioner.
# Independent direct block solve reference and actual two-sector convolution
# using explicit H coefficients (not the Rust dyadic algorithm).
rng=np.random.default_rng(20261125)
n=3;m=32
G0=np.array([[[.55,.14,0],[0,.5,.08],[0,0,.6]],
             [[.5,0,0],[-.13,.6,0],[.07,0,.55]]])
P=G0[1]@G0[0];length=m*2
H=[np.eye(n)]
for j in range(1,m):H.append(P@H[-1])
H=np.array(H)
def sector_solve(r,alpha=1.):
    q=np.stack([G0[1]@r[2*j]+r[2*j+1] for j in range(m)])
    residues=[]
    for sigma in [1,-1]:
        eta=alpha**(1/m)*np.exp((0 if sigma==1 else 1j*np.pi/m))
        w=eta**np.arange(m)
        hk=np.fft.fft(H*w[:,None,None],axis=0)
        qk=np.fft.fft(q*w[:,None],axis=0)
        residues.append(np.fft.ifft(np.einsum('kij,kj->ki',hk,qk),axis=0)/w[:,None])
    ends=(residues[0]+residues[1]).real/2
    out=np.empty_like(r);prev=np.zeros(n)
    for j in range(m):
        out[2*j]=G0[0]@prev+r[2*j];out[2*j+1]=ends[j];prev=ends[j]
    return out
def operator(G):
    a=np.eye(length*n)
    for j in range(1,length):a[j*n:(j+1)*n,(j-1)*n:j*n]=-G[j]
    return a
frozen=np.tile(G0,(m,1,1))
B0=operator(frozen)
E=rng.normal(size=(length,n,n))/n
rhs=rng.normal(size=(length,n))
ap=[]
for amp in [.01,.08,.25,1.0,2.0]:
    actual=frozen+amp*E
    B=operator(actual);D=B0-B
    K=solve(B0,D)
    exact=solve(B,rhs.ravel()).reshape(length,n)
    # rigorous-in-exact-arithmetic nonnegative envelope for infinity norm
    carry=np.zeros(n);q=0.
    for i in range(1,length):
        carry=np.abs(frozen[i])@carry+np.abs(actual[i]-frozen[i])@np.ones(n)
        q=max(q,np.max(carry))
    op=np.linalg.norm(K,np.inf)
    assert op<=q+1e-14
    # Original residual recomputed at every iteration.
    x=sector_solve(rhs);history=[];errs=[]
    for j in range(33):
        rr=rhs.ravel()-B@x.ravel()
        er=np.linalg.norm((x-exact).ravel(),np.inf)
        history.append(float(np.linalg.norm(rr,np.inf)));errs.append(float(er))
        if history[-1]<1e-11:break
        x=x+sector_solve(rr.reshape(length,n))
    # T is strictly lower triangular in time even when the norm exceeds 1.
    assert np.count_nonzero(np.triu(K))==0
    ap.append(dict(amplitude=amp,q_upper=q,actual_iteration_inf_norm=op,
                   iterations=len(history)-1,residuals=history,errors=errs,
                   achieved=history[-1]<1e-11))
# Finite geometric polynomial product (integer exact check); independent CAS
# analogue including singular return factor. No inversion of closure.
G=np.array([[1,2],[0,1]],dtype=object)
for m0 in [2,4,8,16]:
    coeff=[np.eye(2,dtype=object)]
    for j in range(1,m0):coeff.append(G@coeff[-1])
    for v in [-2,-1,0,1,2]:
        lhs=sum((v**j)*c for j,c in enumerate(coeff))
        prod=np.eye(2,dtype=object);p=G.copy();j=1
        while j<m0:
            prod=prod@(np.eye(2,dtype=object)+(v**j)*p)
            p=p@p;j*=2
        assert np.array_equal(lhs,prod)
        assert np.array_equal((np.eye(2,dtype=object)-v*G)@prod,np.eye(2,dtype=object)-(v**m0)*p)
result={'schema':'retro-temporal-independent-audit-v1','exact_cases':audit,
        'nodes_checked':sum(a['nodes'] for a in audit),'components_checked':sum(a['components'] for a in audit),
        'scale_cases':scale_audit,'scale_nodes_checked':sum(r['nodes'] for r in scale_audit),
        'scale_components_checked':2*sum(r['nodes'] for r in scale_audit),
        'underbounds':0,'aperiodic_reference':ap,'integer_polynomial_identity_cases':20,
        'versions':{'mpmath':mp.__version__,'numpy':np.__version__}}
(root/'AUDIT.json').write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')
print(json.dumps({k:v for k,v in result.items() if k!='aperiodic_reference'},indent=2))
for r in ap:print('APERIODIC',r['amplitude'],'q',r['q_upper'],'actual norm',r['actual_iteration_inf_norm'],'updates',r['iterations'],'residual',r['residuals'][-1],'achieved',r['achieved'])
