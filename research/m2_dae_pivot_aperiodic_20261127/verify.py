from pathlib import Path
from fractions import Fraction as Q
from itertools import product
import json,gzip,collections
P=Path(__file__).parent
PRIOR=P.parent/'m2_dae_sturm_complex_20261127'
def read(p,m):
    if not p.exists():p=Path(str(p)+'.gz')
    t=gzip.decompress(p.read_bytes()).decode() if p.suffix=='.gz' else p.read_text()
    return [json.loads(l.split(m+' ',1)[1])for l in t.splitlines()if l.startswith(m+' {')or l.startswith('test ')and ' ... '+m+' {'in l]
def dump(name,o):(P/name).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def n(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def f(x):return Q(x)
points=read(P/'pivot.log','PIVOT_POINT');assert len(points)==90
root_checks=0;accepted=collections.Counter();refused=collections.Counter()
for r in points:
    if not r['qualified']:
        refused[r['route']]+=1;continue
    v=r['report'];x=f(r['x'][0]);e=f(v['state_error_upper']);de=f(v['derivative_error_upper'])
    xd=(1+x)/8
    for j in range(r['n']):
        z=(j+1)*x;l,h=map(f,v['root_box'][j]);assert l<=z<=h,(r['n'],r['route'],j)
        assert abs(f(v['z'][j])-z)<=e
        assert abs(f(v['zx_columns'][0][j])-(j+1))<=de
        assert abs(f(v['zt'][j]))<=de
        assert abs(f(v['zdot'][j])-(j+1)*xd)<=de
        root_checks+=5
    assert abs(f(v['xdot'][0])-xd)<=de
    assert abs(f(v['reduced_jacobian'][0][0])-Q(1,8))<=de
    root_checks+=2;accepted[r['route']]+=1
    if r['route'] in ('pivot','equilibrated'):assert n(r['allocation'])==r['allocation']['requested_bytes']==0
assert accepted=={'reuse':18,'pivot':18,'equilibrated':18}
assert refused=={'prepared':18,'dominant':18}
recon=read(P/'pivot.log','PIVOT_RECON');assert len(recon)==90
assert all(r['qualified'] and r['whole_x']<=.001 and r['whole_z']<=.001 for r in recon)
assert all(n(r['allocation'])==r['allocation']['requested_bytes']==0 for r in recon if r['candidate']!='reuse')
epochs=read(P/'pivot.log','PIVOT_EPOCH');assert len(epochs)==30
assert sum(r['qualified'] for r in epochs)==18
edges=read(P/'pivot.log','PIVOT_EDGE');assert len(edges)==2 and not any(r['accepted'] for r in edges)
def exp_negative(a):
    assert 0<=a<=1
    t=Q(1);s=t
    for k in range(1,34):
        t*= -a/k;s+=t
        if k==32:hi=s
    return s,hi
def closed(case,a,b,x,y):
    h=b-a
    if case=='noncommuting':
        v=x+3*(y-a*a)*h+2*(b**3-a**3);w=y+b*b-a*a
        return [(v,v),(w,w)]
    p=lambda t:t*t/4-t+2
    v=[p(b)+(x-p(a))*z for z in exp_negative(h/2)]
    w=[y*z for z in exp_negative(h/4)]
    return [(min(v),max(v)),(min(w),max(w))]
def endpoint_checks(r):
    a,b=map(f,r['times']);ein=f(r['incoming']);rows=r['power'][:2]
    left=[(f(row[0][0])-ein,f(row[0][1])+ein) for row in rows]
    right=[(sum((f(v[0])for v in row),Q(0)),sum((f(v[1])for v in row),Q(0)))for row in rows]
    e=f(r['bounds']['candidate_error']);count=0;worst=0.
    for x,y in product(*left):
        exact=closed(r['case'],a,b,x,y)
        for v,w in zip(exact,right):
            for c,d in product(v,w):
                err=abs(c-d)
                assert err<=e,(r['case'],r.get('route',r.get('solver')),float(err),float(e))
                count+=1
                if e:worst=max(worst,float(err/e))
    return count,worst
bounds=read(P/'aperiodic.log','APERIODIC_BOUND');assert len(bounds)==96
manufactured=0;worst=0.
for r in bounds:
    if r['route'] in ('CenteredStream','IntervalStream'):assert n(r['allocation'])==r['allocation']['requested_bytes']==0
    if r['route']=='OrderedScan':assert r['bounds']['threads']==2
    if r['route'] in ('Sequential','OrderedScan'):assert r['bounds']['discrete_error'] is not None
    c,w=endpoint_checks(r);manufactured+=c;worst=max(worst,w)
native=read(P/'aperiodic.log','APERIODIC_NATIVE')
native_checks=0
for r in native:
    assert r['outgoing']<=r['base_outgoing']
    c,w=endpoint_checks(r);native_checks+=c;worst=max(worst,w)
ae=read(P/'aperiodic.log','APERIODIC_EPOCH');assert len(ae)==10 and all(r['qualified'] and r['max_physical_error']<=.001 for r in ae)
assert sum(r['receipts'] for r in ae)==len(native)
replay={}
for m,k,count in [('GUARD','native',520),('VECTOR','vector',240),('COMPONENT','components',360)]:
    a,b=read(PRIOR/(k+'.log'),m),read(P/(k+'.log'),m)
    assert len(a)==len(b)==count
    logical=sum({k:v for k,v in x.items()if k!='allocation'}=={k:v for k,v in y.items()if k!='allocation'}for x,y in zip(a,b))
    exact=sum(x==y for x,y in zip(a,b));requests=sum(n(x['allocation'])==n(y['allocation'])for x,y in zip(a,b))
    assert logical==requests==count
    if m!='COMPONENT':assert exact==count
    replay[m]={'count':count,'logical':logical,'exact':exact,'same_heap_requests':requests,'changed':[{'operation':x.get('operation'),'before':x['allocation'],'after':y['allocation']}for x,y in zip(a,b)if x['allocation']!=y['allocation']]}
dump('INDEPENDENT_CHECKS.json',{'vector_assertions':root_checks,'point_acceptance':dict(accepted),'point_refusal':dict(refused),'singular_refusals':edges,'aperiodic_manufactured_assertions':manufactured,'aperiodic_native_assertions':native_checks,'native_rows':len(native),'worst_ratio':worst,'scope':'Exact Fraction graph and derivative comparison, exact noncommuting affine flow and alternating rational exponential brackets for stable linear source, incoming/right coefficient corners. No generic source/branch theorem or independent holdout.'})
dump('REPLAY_COMPARE.json',replay)
dump('RESOURCE_COMPARE.json',{'vector_epochs':epochs,'aperiodic_epochs':ae,'point_rows':len(points),'reconstruction_rows':len(recon),'aperiodic_bound_rows':len(bounds),'aperiodic_native_rows':len(native)})
print('EXACT',root_checks,manufactured,native_checks,'native rows',len(native),'worst bound ratio',worst,'accepted',dict(accepted),'refused',dict(refused))
print('REPLAY',[(k,v['logical'],v['exact'],v['same_heap_requests'])for k,v in replay.items()])
for op in sorted({x['operation']for x in replay['COMPONENT']['changed']}):
    r=[x for x in replay['COMPONENT']['changed']if x['operation']==op];print('BYTE_DELTA',op,len(r),min(x['after']['requested_bytes']-x['before']['requested_bytes']for x in r),max(x['after']['requested_bytes']-x['before']['requested_bytes']for x in r))
