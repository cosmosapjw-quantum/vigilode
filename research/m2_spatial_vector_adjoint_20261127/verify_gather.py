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
out={}
for m,count,ignore in [('SPATIAL_GRID',6,()),('SPATIAL_PRIMAL',24,('median_ns','samples_ns')),('SPATIAL_GOAL',18,()),('SPATIAL_CERT',72,('median_ns','samples_ns')),('SPATIAL_NUMERIC',72,('median_ns','samples_ns')),('SPATIAL_QOI',72,()),('SPATIAL_DENSE',6,()),('SPATIAL_EDGE',1,())]:
 a,b=read(P/'portfolio.log',m),read(P/'final_portfolio.log',m)
 assert len(a)==len(b)==count,(m,len(a),len(b))
 k=lambda x:{i:j for i,j in x.items()if i not in ignore}
 assert all(k(x)==k(y)for x,y in zip(a,b)),m
 out[m]={'count':count,'exact':sum(x==y for x,y in zip(a,b)),'logical':count}
g=read(P/'final_portfolio.log','GATHER_GRID');kernels=read(P/'final_portfolio.log','GATHER_KERNEL');ivps=read(P/'final_portfolio.log','GATHER_IVP')
assert len(g)==4 and len(kernels)==8 and len(ivps)==4
checks=collections.Counter();models={}
for r in g:
 c=list(map(Q,r['weights']));rows=[[(j,Q(a))for j,a in e]for e in r['rows']]
 models[r['case']]=(rows,c);expected=mv(rows,c,True)
 for v in [v for v in kernels if v['case']==r['case']]:
  assert requests(v['allocation'])==0 and len(v['values'])==len(expected)
  e=max(abs(Q(x)-y)for x,y in zip(v['values'],expected));assert e<=Q(v['error_upper'])<=Q(1,10**10)
  checks['gather_direct_components']+=len(expected)
for r in ivps:
 rows,c=models[r['case']];ref,p=series(rows,c,Q(1,8),True)
 e=err(r['run']['values'],ref);assert e<=Q(r['error_upper'])<=Q(1,10**6)
 checks['gather_ivp_corners']+=2*len(c)
dump('GATHER_CHECKS.json',{'assertions':dict(checks),'logical_replay':sum(r['logical']for r in out.values()),'exact_replay':sum(r['exact']for r in out.values()),'replay':out,'scope':'Independent represented exact Fraction transpose and full Taylor endpoint; initial spatial campaign logical replay on final source, timing excluded.'})
dump('GATHER_COMPARE.json',{'kernel':[{k:v for k,v in r.items()if k!='values'}for r in kernels],'ivp':[{**{k:v for k,v in r.items()if k!='run'},'run':{k:v for k,v in r['run'].items()if k!='values'}}for r in ivps]})
print('GATHER_EXACT',dict(checks),sum(checks.values()),'REPLAY',sum(r['logical']for r in out.values()),sum(r['exact']for r in out.values()))
