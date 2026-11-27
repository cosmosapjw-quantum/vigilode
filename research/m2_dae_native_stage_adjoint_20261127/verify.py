from pathlib import Path
from fractions import Fraction as Q
from functools import lru_cache
import gzip,json,collections,math
P=Path(__file__).parent
PRIOR=P.parent/'m2_dae_engine_adjoint_20261127'
SLAB=P.parent/'m2_dae_slab_arena_20261127'
ARENA=P.parent/'m2_dae_retained_arena_20261127'
QUERY=P.parent/'m2_dae_query_workspace_20261127'
def read(p,m):
    if not p.exists():p=Path(str(p)+'.gz')
    s=gzip.decompress(p.read_bytes()).decode() if p.suffix=='.gz' else p.read_text()
    return [json.loads(l.split(m+' ',1)[1])for l in s.splitlines()if l.startswith(m+' {')or l.startswith('test ')and ' ... '+m+' {'in l]
def dump(name,o):(P/name).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def n(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def up(x):
    y=float(x);return math.nextafter(y,math.inf)if Q(y)<x else y
def dot(a,b):return sum((x*y for x,y in zip(a,b)),Q(0))
def mv(a,v):return [dot(r,v)for r in a]
def trans(a):return [list(x)for x in zip(*a)]
def tri(a,b):
    n=len(b);lo=all(a[i][j]==0 for i in range(n)for j in range(i+1,n))
    assert lo or all(a[i][j]==0 for i in range(n)for j in range(i))
    x=[Q(0)]*n
    order=list(range(n))if lo else list(range(n-1,-1,-1))
    for k,i in enumerate(order):x[i]=(b[i]-sum((a[i][j]*x[j]for j in order[:k]),Q(0)))/a[i][i]
    return x
@lru_cache(maxsize=4096)
def exp_negative(a):
    assert a>=0;k=0
    while a>1:a/=2;k+=1
    v=s=Q(1)
    for j in range(1,46):
        v*= -a/j;s+=v
        if j==44:hi=s
    return s**(2**k),hi**(2**k)
def exact_state(case,t):
    if case=='stiff':return [exp_negative(64*t)]
    if case=='mixed_rates':
        a,b=exp_negative(32*t);c,d=exp_negative(t)
        return [(a/4,b/4),((c-b)/992,(d-a)/992)]
    if case.startswith('diag'):return [(a/64,b/64)for a,b in [exp_negative(i*t)for i in range(1,int(case[-1])+1)]]
    n,r,k=(4,Q(1),Q(1,2**18))if case=='chain_weak'else(8,Q(32),Q(8))
    a,b=exp_negative(r*t)
    return [(a*p/64,b*p/64)for i in range(n) for p in [sum(((k*t)**j/math.factorial(j)for j in range(i+1)),Q(0))]]
models=read(P/'portfolio.log','NATIVE_MODEL')
table,=read(P/'portfolio.log','NATIVE_TABLEAU')
gamma=Q(table['gamma']);aa=[list(map(Q,table['a'][i*8:(i+1)*8]))for i in range(8)]
cc=[list(map(Q,table['c'][i*8:(i+1)*8]))for i in range(8)];bb=list(map(Q,table['b']))
assert len(models)==6 and all(len(x)==8 for x in aa+cc)
assert all(aa[i][j]==cc[i][j]==0 for i in range(8)for j in range(i,8))
M={}
for r in models:
    case=r['case'];d=r['n'];a=[list(map(Q,r['matrix'][i*d:(i+1)*d]))for i in range(d)];c=list(map(Q,r['goal']))
    if case=='stiff':assert a==[[-64]]and c==[1]
    elif case=='mixed_rates':assert a==[[-32,0],[Q(1,8),-1]]and c==[0,1]
    elif case.startswith('diag'):assert a==[[-i-1 if i==j else 0 for j in range(d)]for i in range(d)]and c==[1]*d
    else:
        rate,k=(1,Q(1,2**18))if case=='chain_weak'else(32,Q(8))
        assert a==[[-rate if i==j else k if j==i-1 else 0 for j in range(d)]for i in range(d)]
        assert c==([1]+[0]*(d-1)if case=='chain_weak'else[1]*d)
    assert n(r['construction'])==0
    M[case]=(a,c)
rows=read(P/'portfolio.log','NATIVE_STAGE');assert len(rows)==36
assert len({(r['case'],r['candidate'])for r in rows})==36
good=[r for r in rows if r['qualified']]
assert sum(r['candidate']=='Legacy'for r in good)==6
lookup={(r['case'],r['candidate']):r for r in good}
for r in good:
    assert r['max_physical_error']<=.001
    if r['candidate'] not in ('Legacy','ReuseLu'):
        assert r['factorizations']==0 and r['work']['source_jacobian_skips']==r['logical_jacobian_builds']
    if r['candidate'] in ('GoalUniform','GoalAdjoint'):
        w=r['work'];assert r['accepted']<=w['completed_certified_attempts']<=r['attempts']
        assert w['max_stage_error_upper']<=1e-9 and w['max_goal_error_upper']<=1e-10
        if r['candidate']=='GoalUniform':assert w['max_goal_uniform_upper']<=1e-10
checks=collections.Counter();packet_ratios=[];perturbed=None
packets=read(P/'portfolio.log','NATIVE_PACKET');assert len(packets)==len(good)+1
for r in packets:
    case=r['case'];A,c=M[case];d=len(c);h=Q(r['h']);y=list(map(Q,r['y0']));u=[list(map(Q,v))for v in r['stages']]
    W=[[Q(i==j)/(h*gamma)-A[i][j]for j in range(d)]for i in range(d)]
    def T(i,j):return [[aa[i][j]*A[k][l]+(cc[i][j]/h if k==l else 0)for l in range(d)]for k in range(d)]
    f=mv(A,y);ue=[];res=[]
    for i in range(8):
        rhs=f.copy();rh=f.copy()
        for j in range(i):
            v=mv(T(i,j),ue[j]);w=mv(T(i,j),u[j])
            rhs=[x+y for x,y in zip(rhs,v)];rh=[x+y for x,y in zip(rh,w)]
        ue.append(tri(W,rhs));res.append([a-b for a,b in zip(rh,mv(W,u[i]))])
    e=[[a-b for a,b in zip(v,w)]for v,w in zip(ue,u)]
    # independently solve exact transpose block and prove dual equality,
    # not merely a comparison of two floating-point implementations.
    lam=[None]*8
    for i in range(7,-1,-1):
        rhs=[bb[i]*v for v in c]
        for j in range(i+1,8):rhs=[x+y for x,y in zip(rhs,mv(trans(T(j,i)),lam[j]))]
        lam[i]=tri(trans(W),rhs)
    delta=sum((bb[i]*dot(c,e[i])for i in range(8)),Q(0))
    dual=sum((dot(lam[i],res[i])for i in range(8)),Q(0));assert delta==dual;checks['exact_dual_identity']+=1
    b=r['receipt'];assert n(r['allocation'])==0
    m=max(abs(x)for row in e for x in row)
    assert m<=Q(b['max_stage_error_upper']);checks['exact_stage_components']+=8*d
    l,u0=map(Q,b['signed_goal_enclosure']);assert l<=delta<=u0 and abs(delta)<=Q(b['endpoint_goal_error_upper']);checks['exact_goal']+=2
    uniform=max(sum(map(abs,row))for row in lam)*sum(max(map(abs,row))for row in res)
    assert uniform<=Q(b['uniform_goal_upper']);checks['exact_uniform_majorant']+=1
    packet_ratios.append({'case':case,'candidate':r['candidate'],'actual_stage_error':up(m),'actual_goal_error':up(abs(delta)),'directed_stage_error':b['max_stage_error_upper'],'directed_goal_error':b['endpoint_goal_error_upper'],'uniform_goal_upper':b['uniform_goal_upper']})
    if r['candidate']=='perturbed':perturbed=packet_ratios[-1]
assert perturbed and perturbed['directed_stage_error']>1e-10 and perturbed['directed_goal_error']<perturbed['uniform_goal_upper']
states=read(P/'portfolio.log','NATIVE_STATE');assert len(states)==len(good)
worst=collections.defaultdict(float)
for r in states:
    key=(r['case'],r['candidate']);assert key in lookup
    for q in r['queries']:
        a=exact_state(r['case'],Q(q['t']));a=a+[(sum(v[0]for v in a),sum(v[1]for v in a))]
        err=Q(q['error']);y=list(map(Q,q['state']))
        assert len(y)==len(a)
        for x,(l,h)in zip(y,a):
            e=max(abs(x-l),abs(x-h));assert e<=err,(key,q['t'],float(e),float(err));checks['analytic_physical_corners']+=2
            if err:worst[key]=max(worst[key],float(e/err))
ode=read(P/'portfolio.log','NATIVE_ODE');assert len(ode)==18
accuracy=[]
for r in ode:
    e=Q(0);assert len(r['times'])==len(r['values'])==17
    for t,y in zip(r['times'],r['values']):
        a=exact_state(r['case'],Q(t));assert len(y)==len(a)
        for v,(l,h)in zip(y,a):
            e=max(e,abs(Q(v)-l),abs(Q(v)-h));checks['sampled_ode_corners']+=2
    accuracy.append({'case':r['case'],'candidate':r['candidate'],'error_upper':up(e),'rhs':r['rhs'],'steps':r['steps']})
assert read(P/'portfolio.log','NATIVE_REFUSAL')==[{'clock':True,'nonfinite':True,'nonlinear':True,'wfiltered':True}]
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
    replay[m]=compare(SLAB/'portfolio.log',P/'slab_replay.log',m,count,('allocation','heap','restore_allocation','workspace_inline_bytes'))
replay['ENGINE_EPOCH']=compare(PRIOR/'engine.log',P/'engine_replay.log','ENGINE_EPOCH',28,('allocation','heap','setup','callbacks'))
for m,count in [('ADJOINT_MODEL',4),('ADJOINT_TAYLOR',12),('ADJOINT_ODE',16),('ADJOINT_FD',4),('ADJOINT_REFUSAL',1)]:
    replay[m]=compare(PRIOR/'adjoint.log',P/'adjoint_replay.log',m,count,('allocation','heap','median_ns','samples_ns','construction'))
replay['ODE_COMPARE']=compare(PRIOR/'ode.log',P/'ode_replay.log','ODE_COMPARE',15,('allocation',))
dump('INDEPENDENT_CHECKS.json',{'assertions':dict(checks),'native_candidates':len(rows),'qualified':len(good),'zeroheap_replays':len(packets),'packet_bounds':packet_ratios,'perturbed':perturbed,'analytic_max_ratio':[{'case':k[0],'candidate':k[1],'ratio':v}for k,v in worst.items()],'scope':'Exact Fraction original represented transformed tableau, forward triangular and transpose-block identity, independent analytic IVP and samples. Stage goal is not adaptive solver derivative, LTE or global bound.'})
dump('RESOURCE_COMPARE.json',{'stages':rows,'forward':ode})
dump('REPLAY_COMPARE.json',replay)
print('EXACT',dict(checks),'total',sum(checks.values()),'qualified',len(good),'packets',len(packets),'perturbed',perturbed)
print('REPLAY',sum(r['count']for r in replay.values()),[(k,v['count'],v['logical'],v['exact'])for k,v in replay.items()])
