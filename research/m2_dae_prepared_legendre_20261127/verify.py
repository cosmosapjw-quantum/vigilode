"""Independent receipt replay and exact rational manufactured bounds.
Works with raw logs or their losslessly compressed archive.
"""
from pathlib import Path
import json,gzip,collections,hashlib,subprocess
from fractions import Fraction as Q
from math import comb,factorial
P=Path(__file__).parent
PRIOR=P.parent/'m2_dae_reconstruct_affine_20261126'
def read(p,marker):
    if not p.exists():p=Path(str(p)+'.gz')
    s=gzip.decompress(p.read_bytes()).decode() if p.suffix=='.gz' else p.read_text()
    return [json.loads(l.split(marker+' ',1)[1]) for l in s.splitlines() if l.startswith(marker+' {') or l.startswith('test ') and ' ... '+marker+' {' in l]
def dump(n,x):(P/n).write_text(json.dumps(x,indent=2,sort_keys=True,allow_nan=False)+'\n')
def req(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def exact_bernstein(coeff,n):
    return max(abs(sum((c*Q(comb(i,j),comb(n,j)) for j,c in enumerate(coeff) if j<=i),Q(0))) for i in range(n+1))
def exact_legendre(coeff):
    n=len(coeff)-1
    return sum(abs(sum((c*Q((2*j+1)*factorial(k)**2,factorial(k-j)*factorial(k+j+1)) for k,c in enumerate(coeff) if k>=j),Q(0))) for j in range(n+1))
def mul(a,b):
    out=[Q(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):out[i+j]+=x*y
    return out
def evalp(a,u):return sum((v*u**k for k,v in enumerate(a)),Q(0))
# Manufactured represented power: x=t=u/8, z=x+2^-20 P4(2u-1).
# Source g=(1+t^3)(z-x); differential defect dx/du-h f=-h*x^4/16.
g=mul([Q(1,2**20)*i for i in [1,-20,90,-140,70]],[Q(1),Q(0),Q(0),Q(1,512)])
dx=[Q(0)]*4+[-Q(1,8**5*16)]
exact={'Bernstein':[exact_bernstein(g,7),exact_bernstein(dx,16)],'Legendre':[exact_legendre(g),exact_legendre(dx)]}
exact['Hybrid']=[min(a,b)for a,b in zip(exact['Bernstein'],exact['Legendre'])]
rows=[r for r in read(P/'spectral.log','LEGENDRE')if r['case']=='oscillatory']
assert len(rows)==5
checks=0;result=[]
for r in rows:
    k=r['route'];basis='Bernstein' if k=='Bernstein' else 'Legendre' if k.startswith('Legendre') else 'Hybrid'
    measured=[Q.from_float(r['bounds'][f])for f in ['original_g_upper','differential_defect']]
    for observed,ideal in zip(measured,exact[basis]):assert observed>=ideal,(k,observed,ideal)
    for j in range(1025):
        u=Q(j,1024)
        assert abs(evalp(g,u))<=measured[0] and abs(evalp(dx,u))<=measured[1],(k,u)
        checks+=2
    result.append({'route':k,'exact_basis_bound':[str(v)for v in exact[basis]],'measured_bound':[float(v)for v in measured]})
dump('EXACT_MANUFACTURED_CHECK.json',{'source':'exact dyadic original g and normalized differential residual, independent Fraction factorial formula and Bernstein conversion','sample_assertions':checks,'basis_bound_assertions':10,'rows':result,'scope':'original g and differential residual; preconditioned bound verified by production directed calculation, not independently recomputed here'})
out={}
for m,f,expected in [('GUARD','native',520),('VECTOR','vector',240),('COMPONENT','components',360)]:
    a,b=read(PRIOR/(f+'.log'),m),read(P/(f+'.log'),m)
    assert len(a)==len(b)==expected
    logical=sum({k:v for k,v in x.items()if k!='allocation'}=={k:v for k,v in y.items()if k!='allocation'}for x,y in zip(a,b))
    counts=sum(req(x['allocation'])==req(y['allocation'])for x,y in zip(a,b))
    exact_count=sum(x==y for x,y in zip(a,b))
    assert logical==counts==expected
    if m!='COMPONENT':assert exact_count==expected
    changes=[{'operation':x.get('operation'),'before':x['allocation'],'after':y['allocation']} for x,y in zip(a,b)if x['allocation']!=y['allocation']]
    out[m]={'count':expected,'exact':exact_count,'logical':logical,'same_heap_requests':counts,'changed_allocations':changes}
rec=read(P/'green_final.log','ROUND_MAP')
assert len(rec)==10 and all(r['audit']['legacy']['root_map_regrouped']==1 and r['audit']['legacy']['root_map_rechecks']==1 and not r['audit']['legacy'].get('root_map_expanded',0) for r in rec)
event=read(P/'regression.log','COUPLED_EVENT')
assert len(event)==4
for v in event:
    if v['derivative']=='SchurAffineGraph':assert v['root'][0]<=.0625<=v['root'][1] and v['slabs']==4
    else:assert v['root'] is None and v['slabs']==4
out['recovery']={'projection_rows':len(rec),'rows':rec,'events':event,'expanded_case_observed':False}
dump('REPLAY_AND_RECOVERY.json',out)
r=read(P/'prepared.log','PREPARED_CALLBACK')
points=read(P/'prepared.log','PREPARED_POINT')
assert len(r)==len(points)==30
assert all(req(x['allocation'])==0 for x in points)
assert all(req(a)==0 for x in r if x['solver']=='BoundedPrepared5' for a in x['allocations'])
atlas=read(P/'prepared.log','PREPARED_ATLAS')
assert len(atlas)==10 and all(x['stop']=='Horizon' and x['slabs']==56 and x['phases']==3 for x in atlas)
recon=read(P/'prepared.log','PREPARED_RECON')
assert len(recon)==90 and all(x['qualified'] and x['whole_x']<=.001 and x['whole_z']<=.001 for x in recon)
assert all(req(x['allocation'])==0 for x in recon if x['candidate']!='existing')
spec=read(P/'spectral.log','LEGENDRE')
assert len(spec)==95 and all(req(x['allocation'])==0 and x['qualified'] and x['whole_x']<=.001 and x['whole_z']<=.001 for x in spec)
ep=read(P/'spectral.log','SPECTRAL_EPOCH')
assert len(ep)==10 and all(x['qualified'] and x['max_physical_error']<=.001 for x in ep)
print('EXACT',checks,'sample assertions +10 independent basis bounds')
print('REPLAY',[(k,v['logical'],v['exact'],v['same_heap_requests'])for k,v in out.items()if k!='recovery'])
print('RECOVERY regrouped',len(rec),'expanded not exercised; both real affine graph roots')
print('POINT',len(points),'callbacks',len(r),'atlas',len(atlas),'reconstruction',len(recon),'spectral',len(spec),'epoch',len(ep))
dump('RESOURCE_COMPARE.json',{'atlas':atlas,'epochs':ep,'callbacks':r,'point_count':len(points),'reconstruction_count':len(recon),'spectral_count':len(spec)})
