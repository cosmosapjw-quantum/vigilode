from pathlib import Path
from fractions import Fraction as Q
from functools import lru_cache
import gzip,json,collections,math
P=Path(__file__).parent
PRIOR=P.parent/'m2_dae_native_stage_adjoint_20261127'
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
def state(case,t):
    z=Q(1,2)+t/16;x=z*z-Q(1,4)
    if case=='quadratic':return [pt(x),pt(z)]
    if case=='quartic':
        l,h=quartic_ivp(t);return [(l**4-Q(1,16),h**4-Q(1,16)),(l,h)]
    if case=='logistic':return [pt(1/(4+t))]*2
    if case=='affine':return [exp_negative(64*t)]*2
    if case=='coupled':
        y=Q(1,16)+t*t/32+t**3/768
        return [pt(x),pt(y),pt(z),pt(y)]
    if case.startswith('pure_linear'):
        l,h=exp_negative(2*t);return [(l/2+t/2-Q(1,4),h/2+t/2-Q(1,4))]*int(case[11:])
    if case=='pure_logistic8':return [pt(1/(4+t))]*8
    raise ValueError(case)
def grad(case,t):
    if case=='quadratic':return [pt(Q(1))]
    if case=='quartic':
        l,h=quartic_ivp(t)
        g=lambda z:(1+z)/(6*z**3)
        return [(g(h),g(l))]
    if case=='logistic':return [pt(16/(4+t)**2)]
    if case=='coupled':return [pt(1+t+t*t/16),pt(Q(1))]
    if case=='affine':return [exp_negative(64*t)]
    raise ValueError(case)
adj=read(P/'adjoint.log','NL_ADJOINT');num=read(P/'adjoint.log','NL_NUMERIC');primal=read(P/'adjoint.log','NL_PRIMAL')
assert len(adj)==20 and len(num)==10 and len(primal)==5
assert len({(r['case'],r['candidate'])for r in adj})==20
checks=collections.Counter();worst={}
for r in adj:
    if not r['qualified']:
        assert r['case']=='coupled' and r['candidate']=='ScalarExp16';continue
    assert n(r['allocation'])==r['allocation']['requested_bytes']==0
    assert r['work']['width_upper']<=.1
    exact=grad(r['case'],Q(1,8));assert len(exact)==len(r['enclosure'])
    for (l,h),(a,b) in zip(r['enclosure'],exact):
        assert Q(l)<=a<=b<=Q(h),(r['case'],r['candidate'],float(a),l,h)
        checks['adjoint_enclosure_corners']+=2
for r in num:
    case=r['case'];ex=grad(case,Q(1,8));v=r['proposal']['gradient'];assert len(v)==len(ex)
    err=max(max(abs(Q(x)-a),abs(Q(x)-b))for x,(a,b) in zip(v,ex))
    assert err<=Q(1,10**6),(case,float(err))
    checks['adjoint_numeric_corners']+=2*len(ex);worst[(case,r['candidate'])]=up(err)
for r in primal:
    assert r['qualified']
    for q in r['queries']:
        ex=state(r['case'],Q(q['t']));assert len(ex)==len(q['state'])
        for x,(a,b) in zip(q['state'],ex):
            assert max(abs(Q(x)-a),abs(Q(x)-b))<=Q(q['error']),(r['case'],q['t'])
            checks['primal_physical_corners']+=2
dopri=read(P/'dopri.log','DOPRI_KERNEL');assert len(dopri)==16
accuracy=[]
for r in dopri:
    case=r['case'];d=r['n'];e=Q(0)
    assert n(r['fixed_allocation'])==0
    w=r['fixed_work'];assert w['linear_products']==26*d
    assert w['state_writes']==(26 if r['candidate']=='Rows'else 6)*d
    assert w['error_writes']==(7 if r['candidate']=='Rows'else 1)*d
    for t,y in zip(r['times'],r['values']):
        ex=state(case,Q(t))[:d];assert len(ex)==len(y)==d
        for x,(a,b) in zip(y,ex):
            e=max(e,abs(Q(x)-a),abs(Q(x)-b));checks['dopri_sample_corners']+=2
    assert e<=Q(1,10**6)
    accuracy.append({'case':case,'candidate':r['candidate'],'error_upper':up(e),'rhs':r['rhs'],'steps':r['steps']})
for case in {r['case']for r in dopri}:
    a,b=sorted([r for r in dopri if r['case']==case],key=lambda r:r['candidate'])
    for k in ('times','values','rhs','attempts','steps','rejected','fsal','allocation'):assert a[k]==b[k],(case,k)
assert read(P/'dopri.log','DOPRI_EDGE')==[{'tiny_step_identity':True}]
split=read(P/'split.log','SPLIT_STAGE');assert len(split)==30
old=read(PRIOR/'portfolio.log','NATIVE_STAGE')
lookup={(r['case'],r['candidate']):r for r in old}
split_results=[]
for r in split:
    a=lookup[(r['case'],r['candidate'])]
    for k in ('point_calls','factorizations','rhs','linear_solves','attempts','accepted','max_physical_error','work'):assert r[k]==a[k],(r['case'],r['candidate'],k)
    diff=a['allocation']['requested_bytes']-r['allocation']['requested_bytes']
    if r['candidate'] in ('Triangular','Reciprocal'):assert diff>0,(r['case'],diff)
    split_results.append({'case':r['case'],'candidate':r['candidate'],'before':a['allocation'],'after':r['allocation'],'saved_bytes':diff})
# Same fixed-capacity layout change across dimensions; no assumed 30KiB minimum.
assert len({r['saved_bytes'] for r in split_results if r['candidate'] in ('Triangular','Reciprocal')})==1
dump('ODE_ACCURACY.json',accuracy)
dump('INDEPENDENT_CHECKS.json',{'assertions':dict(checks),'numeric_errors':[{'case':k[0],'candidate':k[1],'error_upper':v}for k,v in worst.items()],'qualified_adjoint':sum(r['qualified']for r in adj),'dopri_rows':len(dopri),'split_rows':len(split),'scope':'Independent exact Fraction rational/log/exponential enclosures on actual primal queries, continuous gradients and numerical DOPRI samples; bitwise adaptive replay asserted in Rust. Not independent blinded validation.'})
dump('RESOURCE_COMPARE.json',{'adjoints':adj,'numeric':num,'dopri':[{k:v for k,v in r.items()if k not in ('values','times')}for r in dopri],'split':split_results})
print('EXACT',dict(checks),'total',sum(checks.values()),'qualified',sum(r['qualified']for r in adj),'numeric worst',max(worst.values()),'ODE worst',max(r['error_upper']for r in accuracy),'split saved',min(r['saved_bytes']for r in split_results if r['candidate']=='Triangular'))
