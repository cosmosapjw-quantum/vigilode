from pathlib import Path
from fractions import Fraction as Q
import gzip,json,collections,struct
P=Path(__file__).parent
PRIOR=P.parent/'m2_dae_query_workspace_20261127'
def read(p,m):
    if not p.exists():p=Path(str(p)+'.gz')
    s=gzip.decompress(p.read_bytes()).decode() if p.suffix=='.gz' else p.read_text()
    return [json.loads(l.split(m+' ',1)[1])for l in s.splitlines()if l.startswith(m+' {')or l.startswith('test ')and ' ... '+m+' {'in l]
def dump(name,o):(P/name).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def n(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def f(x):return Q(x)
rows=read(P/'portfolio.log','ARENA_EPOCH')
assert len(rows)==70
groups={}
for r in rows:
    groups.setdefault((r['case'],r['grid']),{})[r['candidate']]=r
assert len(groups)==10
for k,g in groups.items():
    assert len(g)==7
    b=g['legacy']
    for name,r in g.items():
        assert r['qualified'] and all(r[v]==b[v]for v in ('slabs','accepted','queries','max_physical_error','audit'))
        assert r['heap']['peak_delta']>=max(0,r['heap']['end_delta'])
        if name.endswith(('rows','component')):assert r['native_bytes']==0
        elif name!='legacy':assert r['native_bytes']==b['native_bytes']
    assert g['packed_owned']['query_bytes']==g['packed_rows']['query_bytes']==g['packed_component']['query_bytes']
    assert g['shared_owned']['query_bytes']==g['shared_rows']['query_bytes']==g['shared_component']['query_bytes']
    assert g['shared_rows']['query_bytes']<g['packed_rows']['query_bytes']
replays=read(P/'portfolio.log','ARENA_REPLAY')
assert len(replays)==14
rg={}
append_reads=0
for r in replays:
    nx,nz,nr=(r[k]for k in ('nx','nz','nr'))
    packed=nr+3*nx+9*nz+nx*nz+nx*nx
    stride=packed if r['layout']=='Packed'else packed-nx-3*nz-(nr if nr==nx else 0)
    assert r['stride']==stride
    assert n(r['append_allocation'])==n(r['read_allocation'])==0
    assert r['append_allocation']['requested_bytes']==r['read_allocation']['requested_bytes']==0
    append_reads+=r['count']
    rg.setdefault(r['case'],{})[r['layout']]=r
for case,g in rg.items():
    a,b=g['Packed'],g['Shared']
    assert a['count']==b['count']
    assert a['capacity_bytes']-b['capacity_bytes']==8*a['count']*(a['stride']-b['stride'])-16*a['nz']
    assert b['capacity_bytes']<a['capacity_bytes']
# Orthogonal exact arithmetic: the coefficients of each represented native
# quartic are computed from original endpoint/stage/coefficient bits,
# not from the directed Rust implementation or its polynomial values.
views=read(P/'portfolio.log','NATIVE_VIEW');assert len(views)==21
checks=0;sample_checks=0;basis=collections.Counter()
for r in views:
    p=r['packet'];power=r['power'];basis[p['basis']]+=1
    a=list(map(f,p['y0']));b=list(map(f,p['y1']));h=list(map(f,p['coefficients']));u=[list(map(f,row))for row in p['stages']]
    assert len(h)==24 and len(u)==8 and len(a)==len(b)==len(power) and all(len(row)==len(a)for row in u)
    assert n(r['allocation'])==0
    for i in range(len(a)):
        d=[sum((h[8*k+j]*u[j][i]for j in range(8)),Q(0))for k in range(3)]
        exact=[a[i],b[i]-a[i]+d[0],d[1]-d[0],d[2]-d[1],-d[2]]
        for c,interval in zip(exact,power[i]):
            l,r=map(f,interval);assert l<=c<=r;checks+=1
        for theta in [Q(0),Q(1,4),Q(1,2),Q(3,4),Q(1)]:
            value=sum((c*theta**k for k,c in enumerate(exact)),Q(0))
            # Independent native D/H representation evaluated directly.
            direct=(1-theta)*a[i]+theta*b[i]+theta*(1-theta)*(d[0]+theta*d[1]+theta*theta*d[2])
            assert value==direct;sample_checks+=1
sc=read(P/'portfolio.log','SCALAR_SCRATCH');assert len(sc)==7
for r in sc:
    if r['scalar']:assert n(r['allocation'])==0
    assert r['workspace_bytes']<r['legacy_workspace_bytes']
chain=read(P/'portfolio.log','ARENA_CHAIN');assert len(chain)==1 and chain[0]['committed']==1
un=read(P/'portfolio.log','ARENA_UNRESOLVED');assert un==[{'physical_error':None,'qualified':False}]
lc=read(P/'portfolio.log','LIVE_CONTROL');assert len(lc)==1 and lc[0]['peak']>=4096 and lc[0]['end']>=4096
replay={}
for m,k,count in [('GUARD','native',520),('VECTOR','vector',240),('COMPONENT','components',360)]:
    a,b=read(PRIOR/(k+'.log'),m),read(P/(k+'.log'),m)
    assert len(a)==len(b)==count
    logical=sum({k:v for k,v in x.items()if k!='allocation'}=={k:v for k,v in y.items()if k!='allocation'}for x,y in zip(a,b))
    exact=sum(x==y for x,y in zip(a,b));requests=sum(n(x['allocation'])==n(y['allocation'])for x,y in zip(a,b))
    assert logical==requests==count
    if m!='COMPONENT':assert exact==count
    replay[m]={'count':count,'logical':logical,'exact':exact,'same_heap_requests':requests,'changed':[{'operation':x.get('operation'),'before':x['allocation'],'after':y['allocation']}for x,y in zip(a,b)if x['allocation']!=y['allocation']]}
qr={}
for m,count in [('QUERY_POINT',420),('QUERY_EPOCH',40),('QUERY_EDGE',7),('QUERY_CHAIN',1),('QUERY_UNRESOLVED',1)]:
    a,b=read(PRIOR/'portfolio.log',m),read(P/'query_replay.log',m)
    assert len(a)==len(b)==count
    logical=sum({k:v for k,v in x.items()if k!='allocation'}=={k:v for k,v in y.items()if k!='allocation'}for x,y in zip(a,b))
    assert logical==count
    qr[m]={'count':count,'logical':logical,'exact':sum(x==y for x,y in zip(a,b)),'changed':[{'case':x.get('case'),'candidate':x.get('candidate'),'before':x.get('allocation'),'after':y.get('allocation')}for x,y in zip(a,b)if x.get('allocation')!=y.get('allocation')]}
dump('INDEPENDENT_CHECKS.json',{'exact_native_coefficient_checks':checks,'exact_native_polynomial_sample_checks':sample_checks,'native_view_rows':len(views),'basis':dict(basis),'arena_replay_rows':len(replays),'zeroheap_appends':append_reads,'zeroheap_reads':append_reads,'full_epoch_rows':len(rows),'scalar_scratch_rows':sc,'chain':chain,'unresolved':un,'scope':'Independent Fraction coefficients and exact alternate D/H form, storage-stride identities, full Rust legacy materialization comparisons. No new physical theorem or independent holdout.'})
dump('REPLAY_COMPARE.json',replay);dump('QUERY_REPLAY_COMPARE.json',qr)
dump('RESOURCE_COMPARE.json',{'epochs':rows,'arenas':replays,'scalar_scratch':sc})
print('EXACT',checks,sample_checks,'native basis',dict(basis),'arena append/read',append_reads,'chain',chain)
print('REPLAY',[(k,v['logical'],v['exact'],v['same_heap_requests'])for k,v in replay.items()])
print('QUERY_REPLAY',[(k,v['logical'],v['exact'],len(v['changed']))for k,v in qr.items()])
