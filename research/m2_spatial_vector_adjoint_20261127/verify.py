from pathlib import Path
from fractions import Fraction as Q
from functools import lru_cache
import gzip,json,collections,math
P=Path(__file__).parent
def read(p,m):
    if not p.exists():p=Path(str(p)+'.gz')
    s=gzip.decompress(p.read_bytes()).decode() if p.suffix=='.gz' else p.read_text()
    return [json.loads(l.split(m+' ',1)[1]) for l in s.splitlines() if l.startswith(m+' {') or l.startswith('test ') and ' ... '+m+' {' in l]
def dump(n,o):(P/n).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def requests(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def up(x):
    f=float(x);return math.nextafter(f,math.inf)if Q(f)<x else f
def dot(x,y):return sum((a*b for a,b in zip(x,y)),Q(0))
def dot_box(v,u):
    l=h=Q(0)
    for (a,b),c in zip(v,u):
        l+=min(a*c,b*c);h+=max(a*c,b*c)
    return l,h
def err(v,ref):return max(max(abs(Q(x)-a),abs(Q(x)-b))for x,(a,b)in zip(v,ref))
def encl(pair,exact):
    a,b=map(Q,pair);l,h=exact
    assert a<=l<=h<=b,(float(a),float(l),float(h),float(b))
def mv(rows,v,transpose):
    if transpose:
        out=[Q(0)]*len(v)
        for i,edges in enumerate(rows):
            for j,r in edges:
                if i!=j and r:
                    w=r*v[i];out[i]-=w;out[j]+=w
        return out
    return [sum((r*(v[j]-v[i])for j,r in edges if i!=j and r),Q(0))for i,edges in enumerate(rows)]
@lru_cache(maxsize=128)
def exp_negative(a):
    assert a>=0;k=0
    while a>1:a/=2;k+=1
    s=v=Q(1)
    for i in range(1,66):
        v*= -a/i;s+=v
        if i==64:hi=s
    return s**(2**k),hi**(2**k)
def series(rows,v,t,transpose):
    # Independent direct exact Taylor on the full represented matrix,
    # NOT the production uniformization or subcell algorithm.
    lv=mv(rows,v,transpose)
    k=next((i for i,x in enumerate(v)if x),None)
    if k is None:return [(Q(0),Q(0))for _ in v],0
    mu=lv[k]/v[k]
    if mu<=0 and all(a==mu*b for a,b in zip(lv,v)):
        l,h=exp_negative(-mu*t)
        return [(min(l*x,h*x),max(l*x,h*x))for x in v],0
    M=2*t*max(sum((r for j,r in edges if j!=i),Q(0))for i,edges in enumerate(rows))
    norm=sum(map(abs,v))if transpose else max(map(abs,v))
    s=v.copy();term=v.copy();weight=Q(1);p=0
    while True:
        if p+2>M:
            tail=norm*weight*M/(p+1)/(1-M/(p+2))
            if tail<=Q(1,10**42):break
        p+=1;assert p<=256
        weight*=M/p
        term=[a*t/p for a in mv(rows,term,transpose)]
        s=[a+b for a,b in zip(s,term)]
    return [(a-tail,a+tail)for a in s],p
grids=read(P/'portfolio.log','SPATIAL_GRID')
primal=read(P/'portfolio.log','SPATIAL_PRIMAL')
goals=read(P/'portfolio.log','SPATIAL_GOAL')
cert=read(P/'portfolio.log','SPATIAL_CERT')
num=read(P/'portfolio.log','SPATIAL_NUMERIC')
qoi=read(P/'portfolio.log','SPATIAL_QOI')
dense=read(P/'portfolio.log','SPATIAL_DENSE')
assert [len(x)for x in [grids,primal,goals,cert,num,qoi,dense]]==[6,24,18,72,72,72,6]
checks=collections.Counter();data={};accuracy=[];ref={};weights={};goals_exact={};balance=[];primals={}
for g in grids:
    n=g['n'];case=g['case'];rows=[[(j,Q(a))for j,a in r]for r in g['rows']]
    assert len(rows)==n and all(len(r)==6 for r in rows)
    assert all(0<=j<n and r>=0 for edges in rows for j,r in edges)
    assert g['basis_comparison_upper']<=1e-12
    # Independently check Cartesian row geometry and exact dyadic rates.
    if case.startswith('box'):
        w=g['width'];ny=4 if case!='box2_medium'else 8;nz=n//(w*ny)
        for i,edges in enumerate(rows):
            x=i%w;y=i//w%ny;z=i//(w*ny)
            index=lambda x,y,z:(z*ny+y)*w+x
            expected=[(index((x-1)%w,y,z),Q(1,8)),(index((x+1)%w,y,z),Q(1,8)),(index(x,(y-1)%ny,z),Q(1,4)),(index(x,(y+1)%ny,z),Q(1,4)),(index(x,y,(z-1)%nz),Q(1,16)if nz>1 else Q(0)),(index(x,y,(z+1)%nz),Q(1,16)if nz>1 else Q(0))]
            assert edges==expected;checks['cartesian_row_edges']+=6
    else:
        w=g['width'];nt=n//w
        for i,r in enumerate(rows):
            x=i%w;y=i//w
            assert [j for j,a in r]==[y*w+(x-1)%w,y*w+(x+1)%w,max(0,y-1)*w+x,min(nt-1,y+1)*w+x,i,i]
            assert r[0][1]==r[1][1] and r[4][1]==r[5][1]==0
            if y==0:assert r[2][1]==0
            if y==nt-1:assert r[3][1]==0
            checks['sphere_row_geometry']+=6
    y=list(map(Q,g['initial']));T=Q(g['duration']);assert T==Q(1,8)
    pf,p=series(rows,y,T,False);data[case]=(rows,y,T,pf)
    b=mv(rows,list(map(Q,g['mass'])),True);balance.append({'case':case,'represented_mass_residual_l1':up(sum(map(abs,b)))})
    for r in [r for r in primal if r['case']==case]:
        e=err(r['run']['values'],pf);assert e<=Q(1,10**6)
        checks['primal_endpoint_corners']+=2*n
        accuracy.append({'case':case,'candidate':r['candidate'],'error_upper':up(e),'reference_order':p})
        primals[case,r['candidate']]=list(map(Q,r['run']['values']))
for r in goals:
    key=r['case'],r['goal'];rows,y,T,pf=data[key[0]]
    c=list(map(Q,r['weights']));weights[key]=c
    ref[key],p=series(rows,c,T,True);goals_exact[key]=dot_box(ref[key],y)
    # Independent primal/dual Taylor interval overlap, not a substitute
    # for exact enclosure verification below.
    l,h=dot_box(pf,c);a,b=goals_exact[key];assert l<=b and a<=h
    checks['primal_dual_overlap']+=1
for r in cert:
    key=r['case'],r['goal'];n=len(ref[key]);assert requests(r['allocation'])==r['allocation']['requested_bytes']==0
    for a,b in zip(r['enclosure'],ref[key]):encl(a,b);checks['gradient_corners']+=2
    w=r['work'];encl(w['exact_goal'],goals_exact[key]);checks['exact_goal_corners']+=2
    l,h=goals_exact[key];j=dot(weights[key],primals[key[0],'Dopri'])
    encl(w['signed_goal_error'],(l-j,h-j));checks['cert_error_corners']+=2
    assert r['qualified']==(w['width_upper']<=1e-6)
    if r['candidate']!='ModeResidual':assert r['qualified']
    assert max(Q(b)-Q(a)for a,b in r['enclosure'])<=Q(w['width_upper'])
for r in num:
    key=r['case'],r['goal'];e=err(r['run']['values'],ref[key]);assert e<=Q(r['error_upper'])<=Q(1,10**6)
    checks['native_adjoint_corners']+=2*len(ref[key])
for r in qoi:
    key=r['case'],r['goal'];l,h=goals_exact[key];j=dot(weights[key],primals[key[0],r['candidate']])
    encl(r['error'],(l-j,h-j));checks['native_goal_error_corners']+=2
    assert max(map(abs,r['error']))<=1e-6
for r in dense:
    key=r['case'],r['goal'];c=r['certificate'];v=c['candidate'];x=ref[key]
    e=sum((max(abs(Q(a)-l),abs(Q(a)-h))**2 for a,(l,h)in zip(v,x)),Q(0))
    assert e<=Q(c['error_upper'])**2;checks['dense_l2_bound']+=1
assert read(P/'portfolio.log','SPATIAL_EDGE')==[{'duplicate_neighbors':True,'polar_two_rings':True,'zero_time':True,'zero_diffusion':True,'transactional_refusal':True}]
dump('PRIMAL_ACCURACY.json',accuracy)
dump('INDEPENDENT_CHECKS.json',{'assertions':dict(checks),'cert_rows':len(cert),'qualified_cert':sum(r['qualified']for r in cert),'native_adjoint_rows':len(num),'native_primal_rows':len(primal),'goal_error_rows':len(qoi),'max_primal_error':max(r['error_upper']for r in accuracy),'max_native_adjoint_error':max(r['error_upper']for r in num),'max_goal_error_upper':max(max(map(abs,r['error']))for r in qoi),'represented_mass_residual':balance,'scope':'Independent exact Fraction full-matrix sparse Taylor (or exact eigen identity), rational geometric truncation tail below1e-42; represented semidiscrete generator, not continuum PDE discretization.'})
dump('RESOURCE_COMPARE.json',{'certificates':[{k:v for k,v in r.items()if k!='enclosure'}for r in cert],'native_adjoint':[{**{k:v for k,v in r.items()if k!='run'},'run':{k:v for k,v in r['run'].items()if k!='values'}}for r in num],'primal':[{**{k:v for k,v in r.items()if k!='run'},'run':{k:v for k,v in r['run'].items()if k!='values'}}for r in primal],'goal_errors':qoi,'dense':dense})
print('EXACT',dict(checks),'total',sum(checks.values()),'qualified',sum(r['qualified']for r in cert),'max_primal',max(r['error_upper']for r in accuracy),'max_adjoint',max(r['error_upper']for r in num),'max_goal',max(max(map(abs,r['error']))for r in qoi),'mass_residual',balance)
