from pathlib import Path
from fractions import Fraction as Q
from functools import lru_cache
import gzip,json,collections
P=Path(__file__).parent
PRIOR=P.parent/'m2_dae_certificate_workspace_20261127'
def read(p,m):
    if not p.exists():p=Path(str(p)+'.gz')
    s=gzip.decompress(p.read_bytes()).decode() if p.suffix=='.gz' else p.read_text()
    return [json.loads(l.split(m+' ',1)[1])for l in s.splitlines()if l.startswith(m+' {')or l.startswith('test ')and ' ... '+m+' {'in l]
def dump(name,o):(P/name).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def f(x):return Q(x)
def n(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
@lru_cache(maxsize=4096)
def exp_negative(a):
    assert a>=0
    k=0
    while a>1:a/=2;k+=1
    t=Q(1);s=t
    for i in range(1,42):
        t*= -a/i;s+=t
        if i==40:hi=s
    return s**(2**k),hi**(2**k)
def exp_bounds(a):
    if a<=0:return exp_negative(-a)
    l,h=exp_negative(a);return 1/h,1/l
def log_bounds(r):
    assert r>0
    w=(r-1)/(r+1);a=abs(w);s=Q(0);term=a
    for k in range(42):
        s+=2*term/(2*k+1);term*=a*a
    tail=2*term/(85*(1-a*a))
    return (s,s+tail) if w>=0 else (-s-tail,-s)
def quartic_time(z):
    p=lambda z:z**3/3-z*z/2+z
    l,h=log_bounds((1+z)/Q(3,2))
    d=p(z)-p(Q(1,2))
    return 32*(d-h),32*(d-l)
@lru_cache(maxsize=256)
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
def graph(case,x):
    if case in ('stable','mass'):return [sum(x)]
    if case=='stiff':return x
    if case=='conserved':return [1-sum(x)]
    if case=='nonlinear':return None
    return [j*x[0]for j in range(1,int(case[6:])+1)]
def g(case,y,nx):
    x=y[:nx];z=y[nx:]
    if case=='nonlinear':return [z[0]**4-x[0]-Q(1,16)]
    if case.startswith('vector'):
        out=[];nz=len(z)
        for i in range(nz):
            j=(i+1)%nz;scale=[Q(3),Q(1,8),Q(8),Q(1,2)][i%4]
            r=z[j]-(j+1)*x[0]
            other=z[j+1]-(j+2)*x[0] if j+1<nz else Q(0)
            out.append(scale*(r+other/4+r*r/256))
        return out
    return [a-b for a,b in zip(z,graph(case,x))]
def analytic(case,t):
    if case in ('stable','mass'):
        p=lambda t:t*t/4-t+2
        u=[p(t)+(Q(1,4)-2)*a for a in exp_negative(t/2)]
        v=[-a/8 for a in exp_negative(t/4)]
        x=[(min(u),max(u)),(min(v),max(v))]
        return x+[(sum(a[0]for a in x),sum(a[1]for a in x))]
    if case=='stiff':
        a=exp_negative(64*t);return [a,a]
    if case=='nonlinear':
        l,h=quartic_ivp(t)
        return [(l**4-Q(1,16),h**4-Q(1,16)),(l,h)]
    if case.startswith('vector'):
        l,h=exp_bounds(t/8);l-=1;h-=1
        return [(l,h)]+[(j*l,j*h)for j in range(1,int(case[6:])+1)]
    return None
points=read(P/'portfolio.log','QUERY_POINT');epochs=read(P/'portfolio.log','QUERY_EPOCH')
refs=read(P/'portfolio.log','QUERY_REFERENCE')
assert len(points)==420 and len(epochs)==40 and len(refs)==7
assert all(r['qualified'] for r in points+epochs)
assert all(n(r['allocation'])==r['allocation']['requested_bytes']==0 for r in points if r['candidate'] in ('fixed_rows','stream_pairs','cursor_pairs'))
groups={}
for r in points:
    groups.setdefault((r['case'],r['index']),{})[r['candidate']]=r
assert len(groups)==84
for vs in groups.values():
    assert set(vs)=={'legacy','fresh_fixed','fixed_rows','stream_pairs','cursor_pairs'}
    assert all(r['query']==vs['legacy']['query'] for r in vs.values())
    assert all(r['work']['projection_calls']==r['work']['dense_into']==1 for r in vs.values() if r['work']is not None)
checks=collections.Counter()
for r in refs:
    case=r['case'];assert len(r['queries'])==12
    for q in r['queries']:
        v=q['projection'];nx=len(v['x']);x=list(map(f,v['x']));y=list(map(f,q['state']));z=y[nx:];t=f(q['t'])
        assert q['state'][:nx]==v['x']==q['raw'][:nx] and q['state'][nx:]==v['z']
        checks['state_identity']+=1
        assert max(map(abs,g(case,y,nx)))<=f(q['projected_constraint_upper']);checks['source_residual']+=1
        if q['raw_constraint_upper']is not None:
            assert max(map(abs,g(case,list(map(f,q['raw'])),nx)))<=f(q['raw_constraint_upper']);checks['raw_mass_residual']+=1
        exact=graph(case,x)
        if exact is None:
            a,b=map(f,v['root_box'][0]);target=x[0]+Q(1,16);e=f(v['state_error_upper'])
            assert a>0 and a**4<=target<=b**4
            assert max(Q(0),z[0]-e)**4<=target<=(z[0]+e)**4
            checks['projection_root_and_error']+=2
        else:
            for i,e in enumerate(exact):
                a,b=map(f,v['root_box'][i]);assert a<=e<=b
                assert abs(z[i]-e)<=f(v['state_error_upper']);checks['projection_root_and_error']+=2
        s=next(s for s in r['slabs']if f(s['times'][1])>=t);c=s['certificate']
        assert c['global_qualified'] and f(s['times'][0])<=t<=f(s['times'][1])
        u=(t-f(s['times'][0]))/(f(s['times'][1])-f(s['times'][0]))
        p=[tuple(sum((f(v[k])*u**j for j,v in enumerate(row)),Q(0))for k in range(2))for row in s['power'][:nx]]
        err=f(q['physical_error_upper']);ideal=f(c['whole_x_error'])
        if c.get('conservation') is not None:
            p.append((1-p[0][1]-p[1][1],1-p[0][0]-p[1][0]))
            correction=max(abs(v-w)for row,w in zip(p,y)for v in row)
            assert ideal+correction<=err
        else:
            ex=ideal+max(abs(v-w)for row,w in zip(p,x)for v in row)
            assert max(ex,f(v['state_error_upper'])+f(c['zx_norm_upper'])*ex)<=err
        checks['physical_query_bound']+=1
        actual=analytic(case,t)
        if actual is not None:
            for interval,w in zip(actual,y):
                for a in interval:
                    assert abs(a-w)<=err,(case,q['t'],float(abs(a-w)),float(err))
                    checks['analytic_ivp_corner']+=1
        if case=='nonlinear':checks['nonlinear_ivp_queries']+=1
edges=read(P/'portfolio.log','QUERY_EDGE');assert len(edges)==7 and all(r['transactional']for r in edges)
chain=read(P/'portfolio.log','QUERY_CHAIN');assert len(chain)==1 and chain[0]['committed']>0
u=read(P/'portfolio.log','QUERY_UNRESOLVED');assert u==[{'physical_error':None,'qualified':True}]
replay={}
for m,k,count in [('GUARD','native',520),('VECTOR','vector',240),('COMPONENT','components',360)]:
    a,b=read(PRIOR/(k+'.log'),m),read(P/(k+'.log'),m)
    assert len(a)==len(b)==count
    logical=sum({k:v for k,v in x.items()if k!='allocation'}=={k:v for k,v in y.items()if k!='allocation'}for x,y in zip(a,b))
    exact=sum(x==y for x,y in zip(a,b));requests=sum(n(x['allocation'])==n(y['allocation'])for x,y in zip(a,b))
    assert logical==requests==count
    if m!='COMPONENT':assert exact==count
    replay[m]={'count':count,'logical':logical,'exact':exact,'same_heap_requests':requests,'changed':[{'operation':x.get('operation'),'before':x['allocation'],'after':y['allocation']}for x,y in zip(a,b)if x['allocation']!=y['allocation']]}
dump('INDEPENDENT_CHECKS.json',{'assertions':dict(checks),'point_rows':len(points),'exact_query_groups':len(groups),'epoch_rows':len(epochs),'chain':chain[0],'unresolved':u[0],'edges':edges,'scope':'Exact Fraction graph/residual and interval-polynomial query radius checks; analytic IVP corners stable, stiff, vector and mass; quartic nonlinear implicit flow via rational log bounds and monotonic bisection. Conserved chain uses source graph and original retained L1 certificate, not independent Robertson IVP.'})
dump('REPLAY_COMPARE.json',replay)
dump('RESOURCE_COMPARE.json',{'epochs':epochs,'storage':read(P/'portfolio.log','QUERY_STORAGE'),'point_rows':len(points)})
print('EXACT',dict(checks),'groups',len(groups),'chain',chain)
print('REPLAY',[(k,v['logical'],v['exact'],v['same_heap_requests'])for k,v in replay.items()])
for op in sorted({x['operation']for x in replay['COMPONENT']['changed']}):
    r=[x for x in replay['COMPONENT']['changed']if x['operation']==op];print('BYTE_DELTA',op,len(r),min(x['after']['requested_bytes']-x['before']['requested_bytes']for x in r),max(x['after']['requested_bytes']-x['before']['requested_bytes']for x in r))
