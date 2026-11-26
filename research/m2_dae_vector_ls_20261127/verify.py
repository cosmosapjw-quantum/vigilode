from pathlib import Path
from fractions import Fraction as Q
from itertools import product
import json,gzip,collections
P=Path(__file__).parent
PRIOR=P.parent/'m2_dae_affine_picard_20261127'
def read(p,m):
    if not p.exists():p=Path(str(p)+'.gz')
    t=gzip.decompress(p.read_bytes()).decode() if p.suffix=='.gz' else p.read_text()
    return [json.loads(l.split(m+' ',1)[1])for l in t.splitlines()if l.startswith(m+' {')or l.startswith('test ')and ' ... '+m+' {'in l]
def dump(name,o):(P/name).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def n(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def f(x):return Q(x)
def exp_negative(a):
    assert a>=0
    k=0
    while a>1:a/=2;k+=1
    term=Q(1);s=Q(1)
    for j in range(1,34):
        term*= -a/j;s+=term
        if j==32:hi=s
    return s**(2**k),hi**(2**k)
def solve(a,b):
    n=len(b);a=[row.copy()+[v] for row,v in zip(a,b)]
    for k in range(n):
        i=next(i for i in range(k,n)if a[i][k]);a[k],a[i]=a[i],a[k]
        v=a[k][k];a[k]=[x/v for x in a[k]]
        for j in range(n):
            if j!=k:
                v=a[j][k];a[j]=[x-v*y for x,y in zip(a[j],a[k])]
    return [row[-1]for row in a]
points=read(P/'vector_new.log','VECTOR_POINT');assert len(points)==27
root_checks=0
for r in points:
    v=r['report'];x=f(r['x'][0]);e=f(v['state_error_upper']);de=f(v['derivative_error_upper'])
    for j in range(2):
        z=(j+1)*x;l,h=map(f,v['root_box'][j]);assert l<=z<=h
        assert abs(f(v['z'][j])-z)<=e
        assert abs(f(v['zx_columns'][0][j])-(j+1))<=de
        assert abs(f(v['zt'][j]))<=de
        assert abs(f(v['zdot'][j])-(j+1))<=de
        root_checks+=5
    assert abs(f(v['xdot'][0])-1)<=de
    assert abs(f(v['reduced_jacobian'][0][0]))<=de
    root_checks+=2
    if r['route']!='legacy':assert n(r['allocation'])==r['allocation']['requested_bytes']==0
recon=read(P/'vector_new.log','VECTOR_RECON');assert len(recon)==45
assert all(r['qualified'] and r['whole_x']<=.001 and r['whole_z']<=.001 for r in recon)
assert all(n(r['allocation'])==r['allocation']['requested_bytes']==0 for r in recon if r['candidate']!='legacy')
epochs=read(P/'vector_new.log','VECTOR_EPOCH');assert len(epochs)==12 and all(r['qualified'] and r['max_physical_error']<=.001 for r in epochs)
ls=read(P/'ls.log','LS_BOUND');assert len(ls)==72
table=json.loads((P.parent.parent/'crates/rodas5p-core/data/retro_dictionary_v1/UNIVERSAL_LEGENDRE.json').read_text())['records']
ls_checks=0;exact_response={}
for r in ls:
    z=f(r['rate']);p=r['order'];b=r['bounds'];L=table[str(p)]
    a=[[Q(L['G'][i][j])-z*(Q(L['C'][i][j])+Q(L['C'][j][i]))+z*z*Q(L['M'][i][j])for j in range(p)]for i in range(p)]
    rhs=[z*Q(L['d'][i])-z*z*Q(L['v'][i])for i in range(p)]
    c=solve(a,rhs);er=1+sum((Q(v)*w for v,w in zip(L['ell'],c)),Q(0))
    lo,hi=map(f,b['response']);e=f(b['candidate_error'])
    if r['route']=='RationalResponse':
        assert lo<=er<=hi,(r['rate'],p,r['route'])
    else:
        ce=f(b['coefficient_error'])*sum(abs(Q(v))for v in L['ell'])
        assert lo-ce<=er<=hi+ce,(r['rate'],p,r['route'])
    ls_checks+=1
    for v in exp_negative(-z):
        assert abs(v-1)<=e,(r['rate'],p,r['route'],float(abs(v-1)),float(e))
        ls_checks+=1
    if r['route'] in ('BoundedNormal','RationalResponse'):assert n(r['allocation'])==r['allocation']['requested_bytes']==0
    exact_response[(float(z),p)]=str(er)
native=read(P/'ls.log','LS_NATIVE');native_checks=0
for r in native:
    a,b=map(f,r['times']);z=f(r['rate'])*(b-a);ein=f(r['incoming'])
    row=r['power'][0];left=[f(row[0][0])-ein,f(row[0][1])+ein]
    right=[sum((f(v[i])for v in row),Q(0))for i in range(2)]
    e=f(r['bounds']['candidate_error'])
    for x,g,y in product(left,exp_negative(-z),right):
        assert abs(x*g-y)<=e,(r['rate'],r['solver'],float(abs(x*g-y)),float(e))
        native_checks+=1
    assert r['outgoing']<=r['base_outgoing']
# Compare prior untouched original routes, retaining allocation byte changes.
replay={}
for m,k,count in [('GUARD','native',520),('VECTOR','vector',240),('COMPONENT','components',360)]:
    a,b=read(PRIOR/(k+'.log'),m),read(P/(k+'.log'),m)
    assert len(a)==len(b)==count
    logical=sum({k:v for k,v in x.items()if k!='allocation'}=={k:v for k,v in y.items()if k!='allocation'}for x,y in zip(a,b))
    exact=sum(x==y for x,y in zip(a,b));requests=sum(n(x['allocation'])==n(y['allocation'])for x,y in zip(a,b))
    assert logical==requests==count
    if m!='COMPONENT':assert exact==count
    replay[m]={'count':count,'logical':logical,'exact':exact,'same_heap_requests':requests,'changed':[{'operation':x.get('operation'),'before':x['allocation'],'after':y['allocation']}for x,y in zip(a,b)if x['allocation']!=y['allocation']]}
dump('INDEPENDENT_CHECKS.json',{'vector_exact_assertions':root_checks,'normal_response_and_ivp_assertions':ls_checks,'native_ls_endpoint_assertions':native_checks,'native_ls_rows':len(native),'exact_ls_responses':[{'rate':z,'order':p,'exact':v}for(z,p),v in sorted(exact_response.items())],'scope':'Exact Fraction graph, derivative comparisons, rational Gram solve and alternating scaled exponential brackets; native endpoint candidate includes uncertainty corners. Not a proof of general solver or all branches.'})
dump('REPLAY_COMPARE.json',replay)
dump('RESOURCE_COMPARE.json',{'vector_epochs':epochs,'ls_epochs':read(P/'ls.log','LS_EPOCH'),'vector_point_rows':len(points),'reconstruction_rows':len(recon),'ls_bound_rows':len(ls),'ls_native_rows':len(native)})
print('EXACT',root_checks,ls_checks,native_checks,'native LS rows',len(native))
print('REPLAY',[(k,v['count'],v['logical'],v['exact'],v['same_heap_requests'])for k,v in replay.items()])
for op in sorted({x['operation']for x in replay['COMPONENT']['changed']}):
    r=[x for x in replay['COMPONENT']['changed']if x['operation']==op];print('BYTE_DELTA',op,len(r),min(x['after']['requested_bytes']-x['before']['requested_bytes']for x in r),max(x['after']['requested_bytes']-x['before']['requested_bytes']for x in r))
