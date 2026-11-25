"""Independent verification of actual binary64 receipts.
Full polynomial products in Fraction, no reuse of the Rust multiplication
table, derivative or LU. Independent 100-digit midpoint roots and exact
Hermite residual coefficients; DOP853 convergence is a numerical crosscheck.
"""
import json,struct,math,hashlib
from fractions import Fraction as F
from pathlib import Path
import mpmath as mp
import numpy as np
import scipy
from scipy.integrate import solve_ivp
root=Path(__file__).resolve().parent
repo=root.parent.parent
mp.mp.dps=100
def f(x):return F.from_float(struct.unpack(">d",bytes.fromhex(x))[0])
def mf(x):return mp.mpf(x.numerator)/x.denominator
def records(mark):
    return [json.JSONDecoder().raw_decode(l.split(mark,1)[1])[0] for l in (root/'regression.log').read_text().splitlines() if mark in l]
def model(c):
    return c['n'],f(c['h']),list(map(f,c['a'])),list(map(f,c['b'])),list(map(f,c['y'])),[(f(t['beta']),list(map(f,t['v'])),t['degree']) for t in c['terms']]
def plus(a,b,scale=F(1)):
    z=a.copy()
    for k,v in b.items():
        z[k]=z.get(k,F(0))+scale*v
    return {k:v for k,v in z.items() if v}
def product(a,b,degree):
    z={}
    for i,x in a.items():
        for j,y in b.items():
            k=tuple(v+w for v,w in zip(i,j))
            if sum(k)<=degree:z[k]=z.get(k,F(0))+x*y
    return {k:v for k,v in z.items() if v}
def scale(a,s):return {k:v*s for k,v in a.items() if v*s}
def source(a,b,terms,Y,degree,zero):
    n=len(b);rhs=[{zero:bb} for bb in b]
    for i in range(n):
        for j in range(n):rhs[i]=plus(rhs[i],Y[j],a[i*n+j])
    for beta,v,d in terms:
        p={}
        for vi,yi in zip(v,Y):p=plus(p,yi,vi)
        q={zero:F(1)}
        for _ in range(d):q=product(q,p,degree)
        for i in range(n):rhs[i]=plus(rhs[i],q,-beta*v[i])
    return rhs
def rsource(a,b,terms,y):
    n=len(y);r=[b[i]+sum(a[i*n+j]*y[j] for j in range(n)) for i in range(n)]
    for beta,v,d in terms:
        p=beta*sum(vj*yj for vj,yj in zip(v,y))**d
        r=[ri-p*vi for ri,vi in zip(r,v)]
    return r
def norm2(v):return mp.sqrt(sum(mf(x)**2 for x in v))
def mx(v):return max(map(abs,v),default=F(0))
jet=records('JET_CASE ')
assert len(jet)==1
c=jet[0];n,h,a,b,y,terms=model(c)
ex=[tuple(e) for e in c['exponents']];D=max(map(sum,ex));coeff=[list(map(f,row)) for row in c['coefficients']]
zero=(0,0,0);prev=[{zero:v} for v in y];max_formal=F(0);checked=0
for i in range(c['steps']):
    now=[{e:co[i*n+j] for e,co in zip(ex,coeff) if co[i*n+j]} for j in range(n)]
    mid=[scale(plus(u,v),F(1,2)) for u,v in zip(prev,now)]
    src=source(a,b,terms,mid,D,zero)
    for j in range(n):
        r=plus(plus(now[j],prev[j],-1),src[j],-h)
        r0=r.pop(zero,F(0));axis=tuple(int(k==i%3) for k in range(3))
        r[axis]=r.get(axis,F(0))+r0
        for e in ex[1:]:
            max_formal=max(max_formal,abs(r.get(e,F(0))));checked+=1
    prev=now
assert checked==c['steps']*n*(len(ex)-1)
assert max_formal<=F(1,10**15),float(max_formal)
# Table identity and complete exact pairs, including independent projections.
path=repo/'crates/rodas5p-integrators/src/retro_jet_table.txt'
raw=path.read_bytes()
assert hashlib.sha256(raw).hexdigest()=='aca4c9cce0624f3fc8d8184e5eaeb2f56b85c26f5f63cfabfe605158305fd786'
rows=[list(map(int,l.split())) for l in raw.decode().splitlines()]
es=[tuple(r[:3]) for r in rows];assert len(es)==len(set(es))==84
assert set(es)=={(i,j,k) for i in range(7) for j in range(7) for k in range(7) if i+j+k<=6}
pair_count=0
for i,row in enumerate(rows):
    pairs=list(zip(row[3::2],row[4::2]))
    expected={(j,k) for j in range(1,len(es)) for k in range(1,len(es)) if tuple(x+y for x,y in zip(es[j],es[k]))==es[i]}
    assert len(pairs)==len(set(pairs)) and set(pairs)==expected
    pair_count+=len(pairs)
assert pair_count==757
cases=records('COUPLED ')+records('THETA_CASE ')
assert len(cases)==7
audit=[]
for c in cases:
    n,h,a,b,y,terms=model(c)
    theta=f(c['theta']) if 'theta' in c else F(1,2)
    vals=list(map(f,c['x']));cert=c['certificate'];cont=c['continuous']
    prev=y[:];exact=list(map(mf,y))
    ah=list(map(mf,a));bh=list(map(mf,b));th=[(mf(beta),list(map(mf,v)),d) for beta,v,d in terms]
    hm=mf(h)
    def fs(Y):
        R=[bh[i]+sum(ah[i*n+j]*Y[j] for j in range(n)) for i in range(n)]
        for beta,v,d in th:
            g=beta*sum(vi*yi for vi,yi in zip(v,Y))**d
            R=[ri-g*vi for ri,vi in zip(R,v)]
        return R
    max_e=mp.mpf(0);max_r=mp.mpf(0);max_c_ratio=mp.mpf(0);max_d_ratio=mp.mpf(0)
    for i in range(c['steps']):
        now=vals[i*n:(i+1)*n]
        mid=[(1-theta)*u+theta*v for u,v in zip(prev,now)]
        res=[v-u-h*fv for u,v,fv in zip(prev,now,rsource(a,b,terms,mid))]
        rn=norm2(res);assert rn<=mp.mpf(cert['residual_upper'])
        def eq(*z):
            fm=fs([(1-mf(theta))*u+mf(theta)*v for u,v in zip(exact,z)])
            return [v-u-hm*fv for u,v,fv in zip(exact,z,fm)]
        exact=list(mp.findroot(eq,tuple(exact),tol=mp.mpf('1e-88'),maxsteps=30))
        en=mp.sqrt(sum((mf(v)-u)**2 for v,u in zip(now,exact)))
        assert en<=mp.mpf(cert['node_upper'][i]),(c['name'],i,en,cert['node_upper'][i])
        max_e=max(max_e,en);max_r=max(max_r,rn)
        if cert['node_upper'][i]>0:max_d_ratio=max(max_d_ratio,en/mp.mpf(cert['node_upper'][i]))
        f0=rsource(a,b,terms,prev);f1=rsource(a,b,terms,now)
        u=[]
        for aa,bb,p,q in zip(prev,now,f0,f1):
            delta=bb-aa
            u.append(dict(enumerate([aa,h*p,3*delta-h*(2*p+q),-2*delta+h*(p+q)])))
        U=[{(k,):v for k,v in p.items()} for p in u]
        src=source(a,b,terms,U,21,(0,))
        R=[plus({(k-1,):k*v for (k,),v in p.items() if k},s,-h) for p,s in zip(U,src)]
        # Independent exact rational coefficients, high-precision norm sum.
        integ=sum(norm2([p.get((k,),F(0)) for p in R])/(k+1) for k in range(22))
        assert integ<=mp.mpf(cont['local_defect_integrals'][i]),(c['name'],i,integ,cont['local_defect_integrals'][i])
        if cont['local_defect_integrals'][i]>0:max_c_ratio=max(max_c_ratio,integ/mp.mpf(cont['local_defect_integrals'][i]))
        prev=now
    # Two independently requested DOP853 accuracies; not an interval oracle.
    aa=np.array(list(map(float,a))).reshape(n,n);bb=np.array(list(map(float,b)))
    tterms=[(float(beta),np.array(list(map(float,v))),d) for beta,v,d in terms]
    def npf(t,y):
        f=aa@y+bb
        for beta,v,d in tterms:f-=beta*v*(v@y)**d
        return f
    times=float(h)*np.arange(1,c['steps']+1)
    refs=[]
    if n==1:
        k=-mf(a[0]);beta=mf(terms[0][0]);y0=mf(y[0])
        ref=np.array([[float(y0*mp.exp(-k*hm*(i+1))/mp.sqrt(1+beta*y0**2/k*(-mp.expm1(-2*k*hm*(i+1)))))] for i in range(c['steps'])])
        refs=[ref,ref]
    else:
        for rt in [1e-12,3e-14]:
            sol=solve_ivp(npf,(0,float(times[-1])),list(map(float,y)),t_eval=times,method='DOP853',rtol=rt,atol=rt/10)
            assert sol.success;refs.append(sol.y.T)
    refdiff=float(np.max(np.linalg.norm(refs[0]-refs[1],axis=1)))
    err=np.linalg.norm(np.array(list(map(float,vals))).reshape(-1,n)-refs[-1],axis=1)
    assert all(e<=b+refdiff+1e-14 for e,b in zip(err,cont['node_upper']))
    audit.append(dict(arm=c['name'],theta=float(theta),nodes=c['steps'],max_discrete_error=float(max_e),max_residual=float(max_r),
                      discrete_bound=cert['error_upper'],max_discrete_error_over_bound=float(max_d_ratio),
                      continuous_defect_integral_ratio=float(max_c_ratio),continuous_bound=cont['error_upper'],
                      max_continuous_observed_error=float(max(err)),reference_difference=refdiff))
out=dict(schema='retro-nonlinear-independent-audit-v1',formal_coefficient_count=checked,max_formal_coefficient_residual=float(max_formal),
         table_channels=len(es),table_pairs=pair_count,endpoint_nodes=sum(r['nodes'] for r in audit),underbounds=0,
         cases=audit,versions=dict(mpmath=mp.__version__,numpy=np.__version__,scipy=scipy.__version__))
(root/'AUDIT.json').write_text(json.dumps(out,indent=2,sort_keys=True)+'\n')
print(json.dumps(out,indent=2))
