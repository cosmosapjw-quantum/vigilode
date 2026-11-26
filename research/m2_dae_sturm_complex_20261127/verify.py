from pathlib import Path
from fractions import Fraction as Q
from itertools import product
import json,gzip,collections
P=Path(__file__).parent
PRIOR=P.parent/'m2_dae_vector_ls_20261127'
def read(p,m):
    if not p.exists():p=Path(str(p)+'.gz')
    t=gzip.decompress(p.read_bytes()).decode() if p.suffix=='.gz' else p.read_text()
    return [json.loads(l.split(m+' ',1)[1])for l in t.splitlines()if l.startswith(m+' {')or l.startswith('test ')and ' ... '+m+' {'in l]
def dump(name,o):(P/name).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def n(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def f(x):return Q(x)
def root_range(n,x,center):
    target=x+Q(1,2**n)
    if center**n==target:return center,center
    a,b=Q(7,16),Q(19,32)
    assert a**n<=target<=b**n
    for _ in range(150):
        m=(a+b)/2
        if m**n==target:return m,m
        if m**n<target:a=m
        else:b=m
    return a,b
points=read(P/'sturm.log','STURM_POINT')
assert len(points)==27
root_checks=0;accepted=collections.Counter();refused=[]
for r in points:
    if not r['qualified']:
        refused.append(r);continue
    degree=r['degree'];v=r['report'];x=f(r['x'][0]);a,b=map(f,v['root_box'][0]);center=f(v['z'][0]);e=f(v['state_error_upper']);de=f(v['derivative_error_upper']);target=x+Q(1,2**degree)
    assert a**degree<=target<=b**degree
    assert max(Q(0),center-e)**degree<=target<=(center+e)**degree
    l,h=root_range(degree,x,center)
    zx=[1/(degree*z**(degree-1))for z in (l,h)]
    xd=[(1+z)/8 for z in(l,h)]
    for k,vals in [('xdot',xd),('zt',[Q(0)]),('zdot',[u*v for u,v in product(xd,zx)])]:
        for vv in vals:assert abs(f(v[k][0])-vv)<=de,(degree,r['route'],k,float(abs(f(v[k][0])-vv)),float(de))
    for k,vals in [('zx_columns',zx),('reduced_jacobian',[z/8 for z in zx])]:
        for vv in vals:assert abs(f(v[k][0][0])-vv)<=de,(degree,r['route'],k)
    root_checks+=2+len(xd)+1+len(xd)*len(zx)+len(zx)*2
    accepted[r['route']]+=1
    if r['route']!='legacy':assert n(r['allocation'])==r['allocation']['requested_bytes']==0
edges=read(P/'sturm.log','STURM_EDGE');assert len(edges)==14
for r in edges:assert r['accepted']==(r['case'] in ('one_interior','quartic_single'))
recon=read(P/'sturm.log','STURM_RECON');assert len(recon)==45
for r in recon:
    if r['qualified']:
        assert r['whole_x']<=.001 and r['whole_z']<=.001
        if r['candidate']!='legacy':assert n(r['allocation'])==r['allocation']['requested_bytes']==0
epochs=read(P/'sturm.log','STURM_EPOCH');assert len(epochs)==12
# Exact rational complex Taylor with geometric tail. Independent arithmetic
# and unscaled series, not the Rust interval recurrence.
def cmul(a,b):return (a[0]*b[0]-a[1]*b[1],a[0]*b[1]+a[1]*b[0])
def cexp(z):
    R=abs(z[0])+abs(z[1]);K=100 if R>4 else 64
    t=(Q(1),Q(0));s=t
    for k in range(1,K+1):
        q=cmul(t,z);t=(q[0]/k,q[1]/k);s=(s[0]+t[0],s[1]+t[1])
    tail=(abs(t[0])+abs(t[1]))*R/(K+1)/(1-R/(K+2))
    return [(s[0]-tail,s[0]+tail),(s[1]-tail,s[1]+tail)]
def exact_poly(t):return (1+t+t*t,Q(1,2)-t)
def exact_response(freq,forced,t0,t1,x0,x1):
    g=cexp((-Q(1,4)*(t1-t0),freq*(t1-t0)))
    a,b=exact_poly(t0) if forced else (Q(0),Q(0));c,d=exact_poly(t1) if forced else (Q(0),Q(0))
    out=[cmul((u,v),(x0-a,x1-b))for u,v in product(*g)]
    return [(min(x[i]for x in out)+p,max(x[i]for x in out)+p)for i,p in enumerate((c,d))]
cb=read(P/'complex.log','COMPLEX_BOUND');assert len(cb)==24
checks=0
for r in cb:
    assert n(r['allocation'])==r['allocation']['requested_bytes']==0
    actual=exact_response(f(r['freq']),r['forced'],Q(0),Q(1),Q(1),Q(1,2))
    right=(Q(3),-Q(1,2))if r['forced']else(Q(1),Q(1,2))
    e=f(r['bounds']['candidate_error'])
    for vals,x in zip(actual,right):
        for v in vals:assert abs(v-x)<=e,(r['freq'],r['route'],r['forced'],float(abs(v-x)),float(e));checks+=1
native=read(P/'complex.log','COMPLEX_NATIVE');native_checks=0
for r in native:
    a,b=map(f,r['times']);ein=f(r['incoming'])
    power=r['power'][:2]
    left=[(f(row[0][0])-ein,f(row[0][1])+ein)for row in power]
    right=[(sum((f(v[0])for v in row),Q(0)),sum((f(v[1])for v in row),Q(0)))for row in power]
    e=f(r['bounds']['candidate_error'])
    for x0,x1 in product(*left):
        actual=exact_response(f(r['freq']),r['forced'],a,b,x0,x1)
        for vals,rr in zip(actual,right):
            for v,w in product(vals,rr):
                assert abs(v-w)<=e,(r['freq'],r['solver'],r['forced'],float(abs(v-w)),float(e))
                native_checks+=1
    assert r['outgoing']<=r['base_outgoing']
ce=read(P/'complex.log','COMPLEX_EPOCH');assert len(ce)==24 and all(r['qualified'] for r in ce)
replay={}
for m,k,count in [('GUARD','native',520),('VECTOR','vector',240),('COMPONENT','components',360)]:
    a,b=read(PRIOR/(k+'.log'),m),read(P/(k+'.log'),m)
    assert len(a)==len(b)==count
    logical=sum({k:v for k,v in x.items()if k!='allocation'}=={k:v for k,v in y.items()if k!='allocation'}for x,y in zip(a,b))
    exact=sum(x==y for x,y in zip(a,b));requests=sum(n(x['allocation'])==n(y['allocation'])for x,y in zip(a,b))
    assert logical==requests==count
    if m!='COMPONENT':assert exact==count
    replay[m]={'count':count,'logical':logical,'exact':exact,'same_heap_requests':requests,'changed':[{'operation':x.get('operation'),'before':x['allocation'],'after':y['allocation']}for x,y in zip(a,b)if x['allocation']!=y['allocation']]}
dump('INDEPENDENT_CHECKS.json',{'root_assertions':root_checks,'point_acceptance':dict(accepted),'refused_points':refused,'root_edge_cases':edges,'complex_manufactured_assertions':checks,'complex_native_assertions':native_checks,'complex_native_rows':len(native),'scope':'Exact Fraction monotone power roots/derivative intervals, explicit edge admissions, exact complex Taylor with geometric tail and manufactured particular solution plus uncertainty corners; not general proof or holdout'})
dump('REPLAY_COMPARE.json',replay)
dump('RESOURCE_COMPARE.json',{'sturm_epochs':epochs,'complex_epochs':ce,'sturm_points':len(points),'sturm_reconstruction':len(recon),'complex_bound_rows':len(cb),'complex_native_rows':len(native)})
print('EXACT',root_checks,checks,native_checks,'native complex rows',len(native),'point acceptance',dict(accepted))
print('REPLAY',[(k,v['logical'],v['exact'],v['same_heap_requests'])for k,v in replay.items()])
for op in sorted({x['operation']for x in replay['COMPONENT']['changed']}):
    r=[x for x in replay['COMPONENT']['changed']if x['operation']==op];print('BYTE_DELTA',op,len(r),min(x['after']['requested_bytes']-x['before']['requested_bytes']for x in r),max(x['after']['requested_bytes']-x['before']['requested_bytes']for x in r))
