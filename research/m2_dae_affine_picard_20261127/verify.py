from pathlib import Path
from fractions import Fraction as Q
from itertools import product
import json,gzip,collections,subprocess
P=Path(__file__).parent
PRIOR=P.parent/'m2_dae_prepared_legendre_20261127'
def read(p,m):
    if not p.exists():p=Path(str(p)+'.gz')
    t=gzip.decompress(p.read_bytes()).decode() if p.suffix=='.gz' else p.read_text()
    return [json.loads(l.split(m+' ',1)[1])for l in t.splitlines()if l.startswith(m+' {')or l.startswith('test ')and ' ... '+m+' {'in l]
def dump(name,o):(P/name).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def n(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def f(x):return Q.from_float(x)
points=read(P/'affine.log','AFFINE_POINT');assert len(points)==120
root_checks=0;groups={}
for r in points:
    v=r['report'];x=r['x'];exact=1-sum(map(f,x),Q(0));lo,hi=map(f,v['root_box'][0]);assert lo<=exact<=hi,(r['route'],x,exact,lo,hi)
    assert abs(f(v['z'][0])-exact)<=f(v['state_error_upper'])
    root_checks+=2
    groups.setdefault((r['solver'],r['cert'],r['position']),{})[r['route']]=v
assert len(groups)==24
same_fields=['z','root_box','xdot','zdot','zt','zx_columns','reduced_jacobian','state_error_upper','derivative_error_upper','tangent_residual_upper','residual_upper','condition_upper','inverse_norm_upper']
matches=collections.Counter()
for k,vs in groups.items():
    a=vs['AffineExact'];b=vs['BoundedAffineScalar']
    for field in same_fields:matches[field]+=a[field]==b[field]
    assert a['root_box']==b['root_box'] and a['z']==b['z'],k
cores=read(P/'affine.log','AFFINE_CORE')
assert len(cores)==24 and all(n(r['allocation'])==r['allocation']['requested_bytes']==0 and r['work']['legacy']['affine_dyadic_success']==1 for r in cores)
recon=read(P/'affine.log','AFFINE_RECON');assert len(recon)==72
assert all(r['qualified'] and r['whole_x']<=.001 and r['whole_z']<=.001 for r in recon)
assert all(n(r['allocation'])==r['allocation']['requested_bytes']==0 for r in recon if r['candidate']!='existing')
callbacks=read(P/'affine.log','AFFINE_CALLBACK');assert len(callbacks)==24
assert all(n(a)==a['requested_bytes']==0 for r in callbacks if r['solver']=='BoundedAffine5' for a in r['allocations'])
atlas=read(P/'affine.log','AFFINE_ATLAS');assert len(atlas)==8 and all(r['stop']=='Horizon' and r['phases']==3 and r['slabs']==56 for r in atlas)
# Independent exact closed form for noncommuting forcing and rational
# alternating exponential brackets for stable diagonal affine source.
def exp_neg(a):
    assert 0<=a<=1
    s=Q(1);term=Q(1)
    for k in range(1,34):
        term*= -a/k;s+=term
        if k==32:hi=s
    return s,hi
def closed(case,t0,t1,x0,x1):
    h=t1-t0
    if case=='noncommuting':
        a=x0+3*(x1-t0*t0)*h+2*(t1**3-t0**3);b=x1+t1*t1-t0*t0
        return [(a,a),(b,b)]
    p=lambda t: t*t/4-t+2
    a=[p(t1)+(x0-p(t0))*v for v in exp_neg(h/2)]
    b=[x1*v for v in exp_neg(h/4)]
    return [(min(a),max(a)),(min(b),max(b))]
picard=read(P/'picard.log','PICARD_BOUND')
assert len(picard)==70
exact_endpoint_checks=0;by_case=collections.Counter();worst=0.
for r in picard:
    b=r['bounds'];assert r['qualified'] and r['outgoing']<=r['original_outgoing']
    if r['route']!='OrderedWords':assert n(r['allocation'])==r['allocation']['requested_bytes']==0
    if r['solver']=='manufactured':
        if r['route']=='Adaptive7':
            assert b['candidate_error']==b['tail_upper']==r['outgoing']==0 and b['iterations']==3
        continue
    t0,t1=map(f,r['times']);ein=f(r['incoming'])
    p=r['power'][:2]
    left=[(f(row[0][0])-ein,f(row[0][1])+ein)for row in p]
    right=[(sum((f(v[0])for v in row),Q(0)),sum((f(v[1])for v in row),Q(0)))for row in p]
    upper=f(b['candidate_error'])
    for x0,x1 in product(*left):
        result=closed(r['case'],t0,t1,x0,x1)
        for a,z in zip(result,right):
            for v,w in product(a,z):
                err=abs(v-w);assert err<=upper,(r['case'],r['route'],r['solver'],r['position'],float(err),float(upper))
                exact_endpoint_checks+=1;by_case[r['case']]+=1
                if upper:worst=max(worst,float(err/upper))
epochs=read(P/'picard.log','PICARD_EPOCH');assert len(epochs)==8 and all(r['qualified'] and r['max_physical_error']<=.001 for r in epochs)
assert all(r['ordered_words']>0 for r in epochs if r['solver']=='Reduced5OrderedPicard')
assert all(r['receipts']==r['slabs'] for r in epochs if r['solver']!='Reduced5')
# Historical receipts: compare complete logical fields and heap request
# counts, retaining new struct storage-size costs separately.
replay={}
for m,k,count in [('GUARD','native',520),('VECTOR','vector',240),('COMPONENT','components',360)]:
    a,b=read(PRIOR/(k+'.log'),m),read(P/(k+'.log'),m)
    assert len(a)==len(b)==count
    logical=sum({k:v for k,v in x.items()if k!='allocation'}=={k:v for k,v in y.items()if k!='allocation'}for x,y in zip(a,b))
    exact=sum(x==y for x,y in zip(a,b));requests=sum(n(x['allocation'])==n(y['allocation'])for x,y in zip(a,b))
    assert logical==requests==count
    if m!='COMPONENT':assert exact==count
    replay[m]={'count':count,'logical':logical,'exact':exact,'same_heap_requests':requests,'changed':[{'operation':x.get('operation'),'before':x['allocation'],'after':y['allocation']}for x,y in zip(a,b)if x['allocation']!=y['allocation']]}
dump('INDEPENDENT_CHECKS.json',{'affine_exact_rational_assertions':root_checks,'affine_point_rows':len(points),'paired_dyadic_exact_fields':dict(matches),'picard_endpoint_assertions':exact_endpoint_checks,'by_case':dict(by_case),'worst_ratio_to_candidate_bound':worst,'scope':'Exact Fraction closed-form noncommuting and alternating Taylor bounds of stable affine diagonal source, corners of incoming-expanded left and outward right represented coefficient boxes. No independent DAE slab branch theorem or full solver verification.'})
dump('REPLAY_COMPARE.json',replay)
dump('RESOURCE_COMPARE.json',{'atlas':atlas,'callbacks':callbacks,'epochs':epochs,'affine_point_rows':len(points),'affine_core_rows':len(cores),'reconstruction_rows':len(recon),'picard_rows':len(picard)})
print('AFFINE',root_checks,'exact rational assertions, paired fields',dict(matches))
print('PICARD',exact_endpoint_checks,'endpoint comparisons',dict(by_case),'worst ratio',worst)
print('REPLAY',[(k,v['count'],v['logical'],v['exact'],v['same_heap_requests'])for k,v in replay.items()])
for op in sorted({x['operation']for x in replay['COMPONENT']['changed']}):
    r=[x for x in replay['COMPONENT']['changed']if x['operation']==op];print('BYTE_DELTA',op,len(r),min(x['after']['requested_bytes']-x['before']['requested_bytes']for x in r),max(x['after']['requested_bytes']-x['before']['requested_bytes']for x in r))
