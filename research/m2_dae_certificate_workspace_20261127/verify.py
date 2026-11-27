from pathlib import Path
from fractions import Fraction as Q
from itertools import product
from functools import lru_cache
import json,gzip,collections
P=Path(__file__).parent
PRIOR=P.parent/'m2_dae_pivot_aperiodic_20261127'
def read(p,m):
    if not p.exists():p=Path(str(p)+'.gz')
    t=gzip.decompress(p.read_bytes()).decode() if p.suffix=='.gz' else p.read_text()
    return [json.loads(l.split(m+' ',1)[1])for l in t.splitlines()if l.startswith(m+' {')or l.startswith('test ')and ' ... '+m+' {'in l]
def dump(name,o):(P/name).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def n(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def f(x):return Q(x)
@lru_cache(maxsize=4096)
def exp_negative(a):
    assert a>=0
    k=0
    while a>1:a/=2;k+=1
    term=Q(1);s=Q(1)
    for j in range(1,42):
        term*= -a/j;s+=term
        if j==40:hi=s
    return s**(2**k),hi**(2**k)
def exp_bounds(a):
    if a<=0:return exp_negative(-a)
    l,h=exp_negative(a);return 1/h,1/l
def root_upper(x):
    assert x>0
    a,b=Q(0),Q(2)
    assert b**4>=x
    for _ in range(90):
        m=(a+b)/2
        if m**4<x:a=m
        else:b=m
    return b
def graph_checks(case,c):
    x=[tuple(map(f,v))for v in c['x_range']]
    z=[tuple(map(f,v))for v in c['branch_root_box']]
    q=f(c['branch_defect']);assert 0<=q<1
    assert all(f(a)<=u<=v<=f(b)for (u,v),(a,b)in zip(z,c['self_map_box']))
    lip=f(c['zx_norm_upper']);mu=f(c['reduced_mu_upper'])
    checks=2
    if case=='stable':
        assert z[0][0]<=sum(v[0]for v in x) and sum(v[1]for v in x)<=z[0][1]
        assert lip>=2 and mu>=-Q(1,4);checks+=4
    elif case=='stiff':
        assert z[0][0]<=x[0][0] and x[0][1]<=z[0][1]
        assert lip>=1 and mu>=-64;checks+=4
    elif case=='nonlinear':
        target=[v+Q(1,16)for v in x[0]]
        assert z[0][0]>0 and z[0][0]**4<=target[0] and z[0][1]**4>=target[1]
        # Lower rational bound on maximal exact derivative on x-tube.
        derivative=1/(4*root_upper(target[0])**3)
        assert lip>=derivative and mu>=derivative/8;checks+=5
    else:
        nz=int(case[6:])
        for j,(a,b)in enumerate(z,1):
            assert a<=j*x[0][0] and j*x[0][1]<=b;checks+=2
        assert lip>=nz and mu>=Q(1,8);checks+=2
    assert c['global_qualified'] and f(c['whole_x_error'])<f(c['tube_radius'])
    assert f(c['whole_z_error'])>=f(c['algebraic_defect'])+lip*f(c['whole_x_error'])
    a,b=map(f,c['times']);h=b-a
    residual=f(c['normalized_reduced_defect']);ein=f(c['incoming'])
    fz=Q(1,8)if case=='nonlinear'or case.startswith('vector')else Q(0)
    assert residual>=f(c['differential_defect'])+h*fz*f(c['algebraic_defect'])
    if c['green_used']:
        t=-mu*h;assert t>0
        l,u=exp_negative(t)
        # Independent rational upper on exact Green expression.
        upper=u*ein+(1-l)/t*residual
        assert upper<=f(c['outgoing']), (case,float(upper),c['outgoing'])
    else:
        _,u=exp_bounds(max(Q(0),mu*h))
        assert u*(ein+residual)<=f(c['outgoing']), (case,float(u*(ein+residual)),c['outgoing'])
    return checks+4
def source_g(case,t,y):
    if case=='stable':return [y[2]-y[0]-y[1]]
    if case=='stiff':return [y[1]-y[0]]
    if case=='nonlinear':return [y[1]**4-y[0]-Q(1,16)]
    nz=int(case[6:]);v=[]
    for i in range(nz):
        j=(i+1)%nz;scale=[Q(3),Q(1,8),Q(8),Q(1,2)][i%4]
        r=y[j+1]-(j+1)*y[0]
        other=y[j+2]-(j+2)*y[0] if j+1<nz else Q(0)
        v.append(scale*(r+other/4+r*r/256))
    return v
def residual_checks(case,c,power):
    a,b=map(f,c['times']);checks=0
    for u in [Q(0),Q(1,2),Q(1)]:
        y=[sum(((f(lo)+f(hi))/2*u**k for k,(lo,hi)in enumerate(row)),Q(0))for row in power]
        assert max(map(abs,source_g(case,a+(b-a)*u,y)))<=f(c['original_g_upper'])
        checks+=1
    return checks
def endpoint_checks(case,c,power):
    if case=='nonlinear':return 0
    a,b=map(f,c['times']);h=b-a;ein=f(c['incoming']);nx=len(c['x_range'])
    left=[(f(row[0][0])-ein,f(row[0][1])+ein)for row in power[:nx]]
    right=[(sum((f(v[0])for v in row),Q(0)),sum((f(v[1])for v in row),Q(0)))for row in power[:nx]]
    err=f(c['outgoing']);cnt=0
    for x in product(*left):
        if case=='stable':
            poly=lambda t:t*t/4-t+2
            r=[tuple(poly(b)+(x[0]-poly(a))*g for g in exp_negative(h/2)),tuple(x[1]*g for g in exp_negative(h/4))]
        elif case=='stiff':r=[tuple(x[0]*g for g in exp_negative(64*h))]
        else:r=[tuple((1+x[0])*g-1 for g in exp_bounds(h/8))]
        for rr,pp in zip(r,right):
            for v,w in product(rr,pp):
                assert abs(v-w)<=err,(case,float(abs(v-w)),float(err))
                cnt+=1
    return cnt
points=read(P/'portfolio.log','CERT_POINT');assert len(points)==75
native=read(P/'portfolio.log','CERT_NATIVE')
epochs=read(P/'portfolio.log','CERT_EPOCH');assert len(epochs)==45
assert all(r['qualified']for r in epochs)
assert sum(r['slabs']for r in epochs if r['candidate']!='legacy')==len(native)
checks=collections.Counter();packed=0;refined=0;exact=collections.Counter()
by={}
for r in points:
    case=r['case'];c=r['certificate']
    checks['point_graph_and_transport']+=graph_checks(case,c)
    checks['point_sampled_source_residual']+=residual_checks(case,c,r['power'])
    checks['point_exact_endpoint']+=endpoint_checks(case,c,r['power'])
    by.setdefault((case,r['position']),{})[r['candidate']]=c
    if r['candidate']!='legacy':
        assert n(r['allocation'])==r['allocation']['requested_bytes']==0
        assert n(r['owned_allocation'])==6
        packed+=r['work']['packed_green_calls']
        refined+=r['work']['refinement']['returned']
for vs in by.values():
    for name in ('Pivot','Equilibrated','Refined','PackedGreen'):
        exact[name]+=vs[name]==vs['legacy']
for r in native:
    case=r['case'];c=r['certificate']
    checks['native_graph_and_transport']+=graph_checks(case,c)
    checks['native_sampled_source_residual']+=residual_checks(case,c,r['power'])
    checks['native_exact_endpoint']+=endpoint_checks(case,c,r['power'])
assert packed>0
edges=read(P/'portfolio.log','CERT_EDGE');assert len(edges)==5 and all(r['transactional']and r['unresolved']for r in edges)
profiles=read(P/'profile.log','DRIVER_PROFILE');assert len(profiles)==5
replay={}
for m,k,count in [('GUARD','native',520),('VECTOR','vector',240),('COMPONENT','components',360)]:
    a,b=read(PRIOR/(k+'.log'),m),read(P/(k+'.log'),m)
    assert len(a)==len(b)==count
    logical=sum({k:v for k,v in x.items()if k!='allocation'}=={k:v for k,v in y.items()if k!='allocation'}for x,y in zip(a,b))
    exact_count=sum(x==y for x,y in zip(a,b));requests=sum(n(x['allocation'])==n(y['allocation'])for x,y in zip(a,b))
    assert logical==requests==count
    if m!='COMPONENT':assert exact_count==count
    replay[m]={'count':count,'logical':logical,'exact':exact_count,'same_heap_requests':requests,'changed':[{'operation':x.get('operation'),'before':x['allocation'],'after':y['allocation']}for x,y in zip(a,b)if x['allocation']!=y['allocation']]}
dump('INDEPENDENT_CHECKS.json',{'assertions':dict(checks),'point_rows':len(points),'native_rows':len(native),'paired_complete_report_matches':dict(exact),'packed_green_calls':packed,'refinement_sweeps':refined,'transactional_cases':len(edges),'scope':'Exact Fraction source graph, derivative and transport inequalities, selected original residual samples, and analytic endpoint uncertainty corners for stable, stiff and vector fixtures. Nonlinear IVP endpoint not independently checked; finite cases only.'})
dump('REPLAY_COMPARE.json',replay)
dump('RESOURCE_COMPARE.json',{'profiles':profiles,'epochs':epochs,'storage':read(P/'portfolio.log','CERT_STORAGE'),'point_rows':len(points),'native_rows':len(native)})
print('EXACT',dict(checks),'native rows',len(native),'paired report identity',dict(exact),'packed',packed,'refinement',refined)
print('REPLAY',[(k,v['logical'],v['exact'],v['same_heap_requests'])for k,v in replay.items()])
for op in sorted({x['operation']for x in replay['COMPONENT']['changed']}):
    r=[x for x in replay['COMPONENT']['changed']if x['operation']==op];print('BYTE_DELTA',op,len(r),min(x['after']['requested_bytes']-x['before']['requested_bytes']for x in r),max(x['after']['requested_bytes']-x['before']['requested_bytes']for x in r))
