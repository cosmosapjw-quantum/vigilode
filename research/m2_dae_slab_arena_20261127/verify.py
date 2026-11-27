from pathlib import Path
from fractions import Fraction as Q
from functools import lru_cache
import gzip,json,collections
P=Path(__file__).parent
PRIOR=P.parent/'m2_dae_retained_arena_20261127'
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
    for j in range(1,42):
        term*= -a/j;s+=term
        if j==40:hi=s
    return s**(2**k),hi**(2**k)
profiles=read(P/'profile.log','SLAB_PROFILE');assert len(profiles)==7
rows=read(P/'portfolio.log','SLAB_EPOCH');assert len(rows)==70
groups={}
for r in rows:groups.setdefault((r['case'],r['grid']),{})[r['candidate']]=r
assert len(groups)==14
dims={'stable':(2,1),'stiff':(1,1),'nonlinear':(1,1),'vector4':(1,4),'vector8':(1,8),'mixed_rates':(2,1),'quadratic_flow':(1,1)}
view_count=0
for (case,grid),g in groups.items():
    assert len(g)==5
    b=g['legacy'];nx,nz=dims[case]
    payload=8*(12*nx+17*nz+nz*nz)+1
    for name,r in g.items():
        assert r['qualified'] and all(r[v]==b[v]for v in ('slabs','accepted','queries','max_physical_error','audit'))
        assert r['heap']['peak_delta']>=max(0,r['heap']['end_delta'])
        if name!='legacy':
            assert n(r['view_allocation'])==0
            view_count+=r['slabs']
        if name.startswith('flat'):assert r['slab_numeric_bytes']==payload*r['slabs'],(case,r['slab_numeric_bytes'],payload*r['slabs'])
    assert g['flat_rows']['slab_numeric_bytes']<b['slab_numeric_bytes']
    assert g['flat_rows']['slab_numeric_bytes']==g['flat_component']['slab_numeric_bytes']
for r in profiles:
    b=groups[(r['case'],'dense')]['legacy'];assert r['slabs']==b['slabs'] and r['max_physical_error']==b['max_physical_error'] and r['audit']==b['audit']
warm=read(P/'portfolio.log','SLAB_WARM');assert len(warm)==7
for r in warm:
    assert r['transactional'] and all(n(a)==a['requested_bytes']==0 for a in r['allocation'])
assert sum(r['slabs']for r in warm)==sum(r['slabs']for r in profiles)
edges=read(P/'portfolio.log','SLAB_EDGE');assert len(edges)==2 and all(not r['qualified']and r['identity']for r in edges)
chain=read(P/'portfolio.log','SLAB_CHAIN');assert len(chain)==1 and chain[0]['committed']==1
analytic=read(P/'portfolio.log','SLAB_ANALYTIC');assert len(analytic)==4
checks=collections.Counter();worst=collections.defaultdict(float)
for r in analytic:
    for q in r['queries']:
        t=Q(q['t']);y=list(map(Q,q['state']));err=Q(q['physical_error_upper'])
        if r['case']=='quadratic_flow':
            z=Q(1,2)+t/16;xx=z*z-Q(1,4);exact=[(xx,xx),(z,z)]
        else:
            a,b=exp_negative(32*t);c,d=exp_negative(t)
            xx=(a/4,b/4);yy=((c-b)/992,(d-a)/992)
            exact=[xx,yy,(xx[0]+yy[0],xx[1]+yy[1])]
        assert len(exact)==len(y)
        for (lo,hi),v in zip(exact,y):
            e=max(abs(lo-v),abs(hi-v))
            assert e<=err,(r['case'],float(t),float(e),float(err))
            checks[r['case']]+=2
            if err:worst[r['case']]=max(worst[r['case']],float(e/err))
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
    replay[m]=compare(PRIOR/'portfolio.log',P/'arena_replay.log',m,count,('allocation','heap','restore_allocation'))
dump('INDEPENDENT_CHECKS.json',{'analytic_assertions':dict(checks),'analytic_max_error_ratio':dict(worst),'profile_rows':len(profiles),'epoch_rows':len(rows),'warm_slabs':sum(r['slabs']for r in warm),'zeroheap_slab_views':view_count,'warm':warm,'edges':edges,'chain':chain,'scope':'Exact Fraction polynomial flow and alternating rational exponential interval comparison on two prospective manufactured fixtures; complete Rust restoration identity and flat payload count, not independent blinded holdout.'})
dump('RESOURCE_COMPARE.json',{'profiles':profiles,'epochs':rows,'warm':warm})
dump('REPLAY_COMPARE.json',replay)
print('EXACT',dict(checks),'ratio',dict(worst),'warm slabs',sum(r['slabs']for r in warm),'zeroheap views',view_count,'chain',chain)
print('REPLAY',[(k,v['count'],v['logical'],v['exact'])for k,v in replay.items()])
