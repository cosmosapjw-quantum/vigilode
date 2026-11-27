from pathlib import Path
from fractions import Fraction as Q
from functools import lru_cache
import gzip,json,collections,math
P=Path(__file__).parent
PRIOR=P.parent/'m2_dae_nonlinear_adjoint_dopri_20261127'
def read(p,m):
    if not p.exists():p=Path(str(p)+'.gz')
    s=gzip.decompress(p.read_bytes()).decode() if p.suffix=='.gz' else p.read_text()
    return [json.loads(l.split(m+' ',1)[1])for l in s.splitlines()if l.startswith(m+' {')or l.startswith('test ')and ' ... '+m+' {'in l]
def dump(name,o):(P/name).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def n(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def up(x):
    y=float(x);return math.nextafter(y,math.inf)if Q(y)<x else y
@lru_cache(maxsize=4096)
def exp_negative(a):
    assert a>=0;k=0
    while a>1:a/=2;k+=1
    v=s=Q(1)
    for j in range(1,46):
        v*= -a/j;s+=v
        if j==44:hi=s
    return s**(2**k),hi**(2**k)
def log_bounds(r):
    assert r>0
    w=(r-1)/(r+1);a=abs(w);s=Q(0);term=a
    for k in range(42):
        s+=2*term/(2*k+1);term*=a*a
    tail=2*term/(85*(1-a*a))
    return (s,s+tail) if w>=0 else (-s-tail,-s)
def quartic_time(z):
    p=lambda z:z**3/3-z*z/2+z
    l,h=log_bounds((1+z)/Q(3,2));d=p(z)-p(Q(1,2))
    return 32*(d-h),32*(d-l)
@lru_cache(maxsize=4096)
def quartic_ivp(t):
    if t==0:return Q(1,2),Q(1,2)
    a,b=Q(1,2),Q(19,32)
    assert quartic_time(a)[1]<=t<=quartic_time(b)[0]
    for _ in range(85):
        m=(a+b)/2;l,h=quartic_time(m)
        if t<l:b=m
        elif t>h:a=m
        else:break
    return a,b
def pt(x):return (x,x)
def exp_positive(t):
    l,h=exp_negative(t);return 1/h,1/l
def state(case,t):
    z=Q(1,2)+t/16;x=z*z-Q(1,4)
    if case=='quadratic':return [pt(x),pt(z)]
    if case=='quartic':
        l,h=quartic_ivp(t);return [(l**4-Q(1,16),h**4-Q(1,16)),(l,h)]
    if case=='logistic':return [pt(1/(4+t))]*2
    if case=='affine':return [exp_negative(64*t)]*2
    if case=='clock':
        l,h=exp_positive(t);return [(l-1-t,h-1-t)]*2
    if case=='equilibrium':return [pt(Q(0))]*2
    if case=='shifted':return [pt(z*z+z-Q(3,4)),pt(z)]
    if case=='coupled':
        y=Q(1,16)+t*t/32+t**3/768
        return [pt(x),pt(y),pt(z),pt(y)]
    if case.startswith('linear'):
        l,h=exp_negative(2*t);return [(l/2+t/2-Q(1,4),h/2+t/2-Q(1,4))]*int(case[6:])
    if case=='logistic31':return [pt(1/(4+t))]*31
    raise ValueError(case)
def grad(case,t):
    if case=='quadratic':return pt(Q(1))
    if case=='quartic':
        l,h=quartic_ivp(t);g=lambda z:(1+z)/(6*z**3);return g(h),g(l)
    if case=='logistic':return pt(16/(4+t)**2)
    if case=='affine':return exp_negative(64*t)
    if case=='clock':return exp_positive(t)
    if case=='equilibrium':return exp_negative(2*t)
    if case=='shifted':return pt(Q(1,2))
    raise ValueError(case)
rows=read(P/'scalar.log','SCALAR_INTEGRAL');sign=read(P/'scalar.log','SCALAR_SIGN');primal=read(P/'scalar.log','SCALAR_PRIMAL')
assert len(rows)==49 and len(primal)==7
checks=collections.Counter();groups=collections.defaultdict(dict)
for r in rows:
    case,k=r['case'],r['candidate'];assert k not in groups[case];groups[case][k]=r
    if not r['qualified']:
        assert (k=='FlowRatio'and case in ('clock','equilibrium'))or(k=='QuadraticSeed16'and case!='shifted'),r
        continue
    w=r['work'];assert n(r['allocation'])==r['allocation']['requested_bytes']==0
    assert w['width_upper']<=.1
    l,h=map(Q,r['enclosure']);a,b=grad(case,Q(1,8));assert l<=a<=b<=h,(case,k,float(a),float(l),float(h))
    checks['gradient_corners']+=2
    assert w['exponentials']==(16*r['slabs'] if k=='Repeated16' else 0 if k=='FlowRatio'else 1)
    if k=='FlowRatio':assert w['source_rows']==12 and w['cells']==0
    if k=='Adaptive':assert r['slabs']<=w['cells']<=64*r['slabs'] and w['split_evaluations']==2*w['cells']-r['slabs']
    if k=='QuadraticSeed16':assert w['quadratic_seed_uses']==w['cells']+1
for case,g in groups.items():
    assert len(g)==7
    assert g['Repeated16']['work']['source_rows']==g['IntegralGuard16']['work']['source_rows']
    assert g['Repeated16']['work']['cells']==g['IntegralGuard16']['work']['cells']
    assert g['Direct16']['work']['root_passes']==g['IntegralGuard16']['work']['root_passes']
    assert g['DirectBare16']['work']['root_passes']==0
assert len(sign)==sum(r['qualified']for r in rows)==41
for r in sign:
    l,h=map(Q,r['enclosure']);a,b=grad(r['case'],Q(1,8));assert l<=-b<=-a<=h
    checks['negative_gradient_corners']+=2
for r in primal:
    assert r['qualified']
    for q in r['queries']:
        exact=state(r['case'],Q(q['t']));assert len(exact)==len(q['state'])
        for x,(a,b) in zip(q['state'],exact):
            assert max(abs(Q(x)-a),abs(Q(x)-b))<=Q(q['error']),(r['case'],q['t'])
            checks['primal_corners']+=2
assert read(P/'scalar.log','SCALAR_EDGE')==[{'nonzero_incoming_refused':True,'unchanged_output':True}]
block=read(P/'blocked.log','BLOCK_KERNEL');values=read(P/'blocked.log','BLOCK_VALUES')
assert len(block)==78 and len(values)==13
accuracy=[]
for r in values:
    d=r['n'];e=Q(0);case=r['case'];assert len(r['times'])==len(r['values'])==17
    for t,y in zip(r['times'],r['values']):
        a=state(case,Q(t))[:d];assert len(a)==len(y)==d
        for x,(l,h) in zip(y,a):
            e=max(e,abs(Q(x)-l),abs(Q(x)-h));checks['ode_baseline_sample_corners']+=2
    assert e<=Q(1,10**6);accuracy.append({'case':case,'error_upper':up(e),'n':d})
for case in {r['case']for r in block}:
    g=[r for r in block if r['case']==case];assert len(g)==6
    b=next(r for r in g if r['candidate']=='Rows')
    for r in g:
        w=r['fixed_work'];d=r['n'];assert r['qualified']and r['identity']
        assert n(r['fixed_allocation'])==0 and w['linear_products']==26*d
        assert w['state_writes']==(26 if r['candidate']=='Rows'else 6)*d and w['error_writes']==(7 if r['candidate']=='Rows'else 1)*d
        for k in ('n','rhs','attempts','steps','rejected','fsal','allocation'):assert r[k]==b[k],(case,k)
        assert len(r['samples_ns'])==len(r['fixed_samples_ns'])==7
assert read(P/'blocked.log','BLOCK_EDGE')==[{'tiny_step_identity':True,'odd_tail':31}]
dump('ODE_ACCURACY.json',accuracy)
dump('INDEPENDENT_CHECKS.json',{'assertions':dict(checks),'qualified_scalar':sum(r['qualified']for r in rows),'block_rows':len(block),'scope':'Exact Fraction rational/log/exponential enclosures on actual primal queries, scalar positive/negative gradients and shared bit-identical DOPRI baseline samples. Each baseline sample checked once; all other candidates bit-identical in Rust. Prospective fixtures, not blinded holdout.'})
dump('RESOURCE_COMPARE.json',{'scalar':rows,'blocked':block})
print('EXACT',dict(checks),'total',sum(checks.values()),'scalar',len(sign),'ODE max',max(r['error_upper']for r in accuracy))
