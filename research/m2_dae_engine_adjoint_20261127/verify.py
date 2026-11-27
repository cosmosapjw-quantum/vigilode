from pathlib import Path
from fractions import Fraction as Q
from functools import lru_cache
import gzip,json,collections,math
P=Path(__file__).parent
PRIOR=P.parent/'m2_dae_slab_arena_20261127'
ARENA=P.parent/'m2_dae_retained_arena_20261127'
QUERY=P.parent/'m2_dae_query_workspace_20261127'
def read(p,m):
    if not p.exists():p=Path(str(p)+'.gz')
    s=gzip.decompress(p.read_bytes()).decode() if p.suffix=='.gz' else p.read_text()
    return [json.loads(l.split(m+' ',1)[1])for l in s.splitlines()if l.startswith(m+' {')or l.startswith('test ')and ' ... '+m+' {'in l]
def dump(name,o):(P/name).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def n(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
@lru_cache(maxsize=4096)
def exp_negative(a):
    assert a>=0
    k=0
    while a>1:a/=2;k+=1
    term=Q(1);s=term
    for j in range(1,46):
        term*= -a/j;s+=term
        if j==44:hi=s
    return s**(2**k),hi**(2**k)
def exact_gradient(case,t):
    if case=='stiff':return [exp_negative(64*t)]
    if case=='mixed_rates':
        a,b=exp_negative(32*t);c,d=exp_negative(t)
        return [((c-b)/248,(d-a)/248),(c,d)]
    return [exp_negative(i*t)for i in range(1,int(case[-1])+1)]
def exact_state(case,t):
    if case=='stiff':return [exp_negative(64*t)]
    if case=='mixed_rates':
        a,b=exp_negative(32*t);c,d=exp_negative(t)
        return [(a/4,b/4),((c-b)/992,(d-a)/992)]
    if case=='quadratic_flow':
        z=Q(1,2)+t/16;x=z*z-Q(1,4);return [(x,x)]
    return [(a/64,b/64)for a,b in exact_gradient(case,t)]
def upper_float(x):
    f=float(x);return math.nextafter(f,math.inf)if Q(f)<x else f
eng=read(P/'engine.log','ENGINE_EPOCH');assert len(eng)==28
groups={}
for r in eng:groups.setdefault(r['case'],{})[r['candidate']]=r
assert len(groups)==7
for case,g in groups.items():
    b=g['Legacy']
    assert len(g)==4
    for r in g.values():
        assert r['qualified'] and r['slabs']==b['slabs'] and r['max_physical_error']==b['max_physical_error']
        assert all(n(a)==0 for a in r['callbacks'])
        assert {k:v for k,v in r['audit'].items()if k!='initialization'}=={k:v for k,v in b['audit'].items()if k!='initialization'}
    assert n(g['Shared']['setup'])<n(b['setup'])
models=read(P/'adjoint.log','ADJOINT_MODEL');tay=read(P/'adjoint.log','ADJOINT_TAYLOR');sol=read(P/'adjoint.log','ADJOINT_ODE');fd=read(P/'adjoint.log','ADJOINT_FD')
assert (len(models),len(tay),len(sol),len(fd))==(4,12,16,4)
lookup={r['case']:r for r in models};checks=collections.Counter();maxerr={}
for r in models:
    case=r['case'];d=r['dimension'];a=r['matrix'];c=r['terminal'];t=Q(r['duration'])
    assert n(r['construction'])==0
    if case=='stiff':assert a==[-64]and c==[1]
    elif case=='mixed_rates':assert a==[-32,0,.125,-1]and c==[0,1]
    else:assert a==[-(i+1)if i==j else 0 for i in range(d)for j in range(d)]and c==[1]*d
    for (l,h),(a,b) in zip(r['reference'],exact_gradient(case,t)):
        assert Q(l)<=a<=b<=Q(h);checks['reference_gradient']+=2
for r in tay:
    case=r['case'];t=Q(lookup[case]['duration'])
    assert n(r['allocation'])==r['allocation']['requested_bytes']==0
    for (l,h),(a,b) in zip(r['enclosure'],exact_gradient(case,t)):
        assert Q(l)<=a<=b<=Q(h),(case,r['candidate'],float(a),l,h)
        checks['taylor_gradient']+=2
    assert r['qualified'] and r['work']['width_upper']<=1e-8
    if r['candidate']=='Budgeted':assert r['work']['truncation_upper']<=1e-9
for r in sol:
    case=r['case'];e=Q(0)
    for x,(a,b) in zip(r['gradient']['values'],exact_gradient(case,Q(lookup[case]['duration']))):
        e=max(e,abs(Q(x)-a),abs(Q(x)-b));checks['ode_gradient']+=2
    assert e<=Q(r['error_upper'])<=Q(1e-8)
    maxerr[case,r['candidate']]=upper_float(e)
for r in fd:
    case=r['case']
    for x,(a,b) in zip(r['gradient'],exact_gradient(case,Q(lookup[case]['duration']))):
        assert max(abs(Q(x)-a),abs(Q(x)-b))<=Q(2e-6);checks['finite_difference']+=2
assert read(P/'adjoint.log','ADJOINT_REFUSAL')==[{'nonlinear':True,'nonfinite':True}]
ode=read(P/'ode.log','ODE_COMPARE');assert len(ode)==15
accuracy=[]
for r in ode:
    case=r['case'];e=Q(0)
    assert len(r['times'])==len(r['values'])==17
    for t,y in zip(r['times'],r['values']):
        exact=exact_state(case,Q(t));assert len(exact)==len(y)
        for x,(a,b) in zip(y,exact):
            e=max(e,abs(Q(x)-a),abs(Q(x)-b));checks['forward_sample']+=2
    accuracy.append({'case':case,'candidate':r['candidate'],'error_upper':upper_float(e),'samples':len(r['times']),'rhs':r['rhs'],'steps':r['steps'],'scope':'independent sampled differential state error, not slab or full DAE certificate'})
dump('ODE_ACCURACY.json',accuracy)
replay={}
def compare(old,new,m,count,ignore=()):
    a,b=read(old,m),read(new,m);assert len(a)==len(b)==count,(m,len(a),len(b))
    logical=sum({k:v for k,v in x.items()if k not in ignore}=={k:v for k,v in y.items()if k not in ignore}for x,y in zip(a,b))
    exact=sum(x==y for x,y in zip(a,b));assert logical==count,(m,logical,count)
    return {'count':count,'logical':logical,'exact':exact,'changed':[{'case':x.get('case'),'candidate':x.get('candidate'),'before':{k:x.get(k)for k in ignore},'after':{k:y.get(k)for k in ignore}}for x,y in zip(a,b)if x!=y]}
for m,k,count in [('GUARD','native',520),('VECTOR','vector',240),('COMPONENT','components',360)]:
    replay[m]=compare(PRIOR/(k+'.log'),P/(k+'.log'),m,count,('allocation',))
for m,count in [('QUERY_POINT',420),('QUERY_EPOCH',40),('QUERY_EDGE',7),('QUERY_CHAIN',1),('QUERY_UNRESOLVED',1)]:
    replay[m]=compare(QUERY/'portfolio.log',P/'query_replay.log',m,count,('allocation',))
for m,count in [('ARENA_EPOCH',70),('ARENA_REPLAY',14),('NATIVE_VIEW',21),('SCALAR_SCRATCH',7),('ARENA_CHAIN',1),('ARENA_UNRESOLVED',1)]:
    replay[m]=compare(ARENA/'portfolio.log',P/'arena_replay.log',m,count,('allocation','heap','restore_allocation'))
for m,count in [('SLAB_EPOCH',70),('SLAB_WARM',7),('SLAB_EDGE',2),('SLAB_CHAIN',1),('SLAB_ANALYTIC',4)]:
    replay[m]=compare(PRIOR/'portfolio.log',P/'slab_replay.log',m,count,('allocation','heap','restore_allocation','workspace_inline_bytes'))
dump('INDEPENDENT_CHECKS.json',{'assertions':dict(checks),'engine_rows':len(eng),'initial_audit_equal':sum(r['initial_audit_equal']for r in eng),'adjoint_models':len(models),'taylor_rows':len(tay),'ode_gradient_rows':len(sol),'forward_ode_rows':len(ode),'independent_gradient_errors':[{'case':k[0],'candidate':k[1],'error_upper':v}for k,v in maxerr.items()],'scope':'Exact Fraction alternating exponential enclosures; constant source affine Schur gradient, numerical comparator and original physical-source finite differences. Not discrete solver derivative or general nonlinear adjoint.'})
dump('RESOURCE_COMPARE.json',{'engine':eng,'adjoint_model':models,'taylor':tay,'adjoint_ode':sol,'forward_ode':ode})
dump('REPLAY_COMPARE.json',replay)
print('EXACT',dict(checks),'total',sum(checks.values()),'initial audit equal',sum(r['initial_audit_equal']for r in eng))
print('REPLAY',[(k,v['count'],v['logical'],v['exact'])for k,v in replay.items()])
