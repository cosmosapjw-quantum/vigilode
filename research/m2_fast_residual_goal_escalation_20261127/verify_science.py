from pathlib import Path
from fractions import Fraction as Q
import gzip,json,collections,math
P=Path(__file__).parent
PRIOR=P.parent/'m2_cost_aware_early_krylov_20261127'
def read(p,m):
 if not p.exists():p=Path(str(p)+'.gz')
 s=gzip.decompress(p.read_bytes()).decode()if p.suffix=='.gz'else p.read_text()
 return [json.loads(l.split(m+' ',1)[1])for l in s.splitlines()if l.startswith(m+' {')or l.startswith('test ')and ' ... '+m+' {'in l]
def dump(n,o):(P/n).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def req(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def up(x):
 v=float(x);return math.nextafter(v,math.inf)if Q(v)<x else v
def pt(x):return (Q(x),Q(x))
def dot_box(box,v):
 lo=hi=Q(0)
 for (a,b),c in zip(box,v):lo+=min(a*c,b*c);hi+=max(a*c,b*c)
 return lo,hi
def error(v,box):return max(max(abs(Q(x)-a),abs(Q(x)-b))for x,(a,b)in zip(v,box))
def encl(pair,box,label):
 a,b=map(Q,pair);l,h=box
 assert a<=l<=h<=b,(label,float(a),float(l),float(h),float(b))
def log1p(a):
 assert 0<=a<1
 s=Q(0);v=a
 for k in range(1,81):
  s+=v/k;v*=-a
  if k==79:hi=s
 return s,hi
def state(case,n,t):
 if case=='quadratic':
  z=Q(1,2)+t/16;x=z*z-Q(1,4);y=Q(1,16)+t*t/32+t**3/768
  return list(map(pt,[x,y,z,y]))
 s=[]
 for j in range(n//2):
  a=Q(j+1);v=a*t*t/8;d=1+v
  l,h=log1p(v);s.extend([pt(Q(1,4)/d),(Q(1,16)+l/a,Q(1,16)+h/a)])
 return s+s
def mv(rows,v):
 out=[Q(0)]*len(v)
 for i,edges in enumerate(rows):
  for j,a in edges:
   if i!=j and a:
    x=v[i]*a;out[i]-=x;out[j]+=x
 return out
S=1<<256
def floor(x):return x.numerator//x.denominator
def ceil(x):return -floor(-x)
def fixed(x):return floor(Q(x)*S),ceil(Q(x)*S)
def scale(a,num,den):
 lo,hi=a
 if num<0:lo,hi=-hi,-lo;num=-num
 return lo*num//den,-((-hi*num)//den)
def fixed_mv(edges,v):
 n=len(v);lo=[0]*n;hi=[0]*n
 for i,j,a,b in edges:
  l,h=scale(v[i],a,b);lo[i]-=h;hi[i]-=l;lo[j]+=l;hi[j]+=h
 return list(zip(lo,hi))
def direct_taylor(rows,initial,T):
 n=len(rows);v=[fixed(x)for x in initial];radius=0
 edges=[(i,j,a.numerator,a.denominator)for i,row in enumerate(rows)for j,a in row if j!=i and a]
 M=2*T*max(sum((a for j,a in row if j!=i),Q(0))for i,row in enumerate(rows))
 cells=1
 while M/cells>4:cells*=2
 h=T/cells;m=M/cells
 term_weight=Q(1);p=0
 while True:
  if p+2>m:
   f=term_weight*m/(p+1)/(1-m/(p+2))
   if f<=Q(1,1<<180):break
  p+=1;assert p<=256;term_weight*=m/p
 for _ in range(cells):
  mid=[(l+r)//2 for l,r in v]
  radius+=sum(max(abs(c-l),abs(c-r))for c,(l,r)in zip(mid,v))
  v=[(c,c)for c in mid];norm=sum(map(abs,mid));tail=ceil(norm*f)
  term=v.copy();res=v.copy()
  for k in range(1,p+1):
   w=h/k;term=[scale(x,w.numerator,w.denominator)for x in fixed_mv(edges,term)]
   res=[(a+c,b+d)for (a,b),(c,d)in zip(res,term)]
  radius+=tail;v=res
 mid=[(l+r)//2 for l,r in v];radius+=sum(max(abs(c-l),abs(c-r))for c,(l,r)in zip(mid,v))
 box=[(Q(c-radius,S),Q(c+radius,S))for c in mid]
 assert 2*Q(radius,S)<=Q(1,10**25)
 return box,cells,p
checks=collections.Counter()
grids=read(P/'residual.log','RES_GRID');inputs=read(P/'residual.log','RES_INPUT');rows=read(P/'residual.log','RES_CERT')
assert len(grids)==8 and len(inputs)==40 and len(rows)==200
models={}
for g in grids:
 n=g['n'];r=[[(j,Q(a))for j,a in e]for e in g['rows']]
 assert len(r)==n and all(len(e)==6 for e in r)and all(0<=j<n and a>=0 for e in r for j,a in e)
 inc=[0]*n
 for i,e in enumerate(r):
  for j,a in e:
   if i!=j and a:inc[j]+=1
 assert max(inc)<=6
 models[g['case']]=(r,n)
truth={};res_ref={};resources=[]
for a in inputs:
 key=a['case'],a['scenario'];r,n=models[key[0]];b=list(map(Q,a['b']));x=list(map(Q,a['x']));z=Q(a['shift'])
 assert len(b)==len(x)==n
 rho=sum(abs(bi-z*xi+vi)for bi,xi,vi in zip(b,x,mv(r,x)))
 res_ref[key]=(rho,z,sum(map(abs,b)))
 g=[v for v in rows if(v['case'],v['scenario'])==key];assert len(g)==5
 d={v['candidate']:v for v in g}
 legacy=d['Legacy']['receipt'];assert d['PackedInterval']['receipt']==legacy
 # Same mathematical source, directed legacy rhs upper is the policy
 # reference. Directed rounding of the threshold may enlarge it slightly;
 # fixtures are not within an ulp of this boundary.
 threshold=Q(1e-10)*Q(legacy['rhs_l1'])+Q(1e-13)
 eligible=rho<=threshold;truth['|'.join(key)]=eligible
 for v in g:
  c=v['receipt'];assert req(v['allocation'])==v['allocation']['requested_bytes']==req(v['prepare'])==0
  assert Q(c['rhs_l1'])>=sum(map(abs,b)) and c['rhs_l1']==legacy['rhs_l1']
  assert rho<=Q(c['residual_l1']) and rho/z<=Q(c['error_l1']),(key,v['candidate'],float(rho),c)
  if c['admitted']:assert eligible,(key,v['candidate'],'false acceptance')
  checks['exact_residual_components']+=n;checks['residual_bound']+=2
  if v['candidate'] in ('FusedEdges','FusedDiagonal'):
   assert c['fast_checks']==1 and c['interval_checks']==0
   k=n+sum(sum(i!=j and a!=0 for j,a in e)for i,e in enumerate(r))*(2 if v['candidate']=='FusedEdges'else 1)
   assert c['fmas']==k
  if v['candidate']=='Hybrid':
   if c['fallbacks']:
    assert c['residual_l1']==legacy['residual_l1']and c['interval_checks']==1
   else:assert c==d['FusedDiagonal']['receipt']
  resources.append(dict(case=key[0],scenario=key[1],candidate=v['candidate'],eligible=eligible,admitted=c['admitted'],bound=c['residual_l1'],median_ns=v['median_ns'],receipt=c,setup=v['setup'],allocation=v['allocation']))
assert read(P/'residual.log','RES_EDGE')==[{'overflow_refused':True,'nonfinite_refused':True,'invalidated_prepare':True}]
dump('RESIDUAL_TRUTH.json',truth)
# Exact shifted residual for real triggered candidate and original source.
fixeds=read(P/'trigger.log','TRIGGER_FIXED');assert len(fixeds)==20
for v in fixeds:
 if not v['qualified']:continue
 r,n=models[v['case']];b=list(map(Q,v['b']));x=list(map(Q,v['values']));z=Q(v['shift'])
 rho=sum(abs(bi-z*xi+vi)for bi,xi,vi in zip(b,x,mv(r,x)));a=v['report']
 assert rho<=Q(a['residual_l1']) and rho/z<=Q(a['error_l1'])<=Q(1,10**8)
 assert a['iterations']==v['counters']['linear_iterations']
 assert a.get('prefix_solves',0)<=a['iterations']
 assert a.get('screen_checks',0)==v['counters']['diagnostic_matvecs']
 checks['trigger_residual_components']+=n
# Independent integer directed Taylor, original represented operator.
duals=read(P/'native.log','FAST_DUAL');ivps=read(P/'native.log','FAST_IVP');triggers=read(P/'native.log','TRIGGER_IVP')
assert len(duals)==7 and len(ivps)==49 and len(triggers)==10
native=[]
for g in duals:
 case=g['case'];r,n=models[case];assert r==[[(j,Q(a))for j,a in e]for e in g['rows']]
 ref,cells,p=direct_taylor(r,list(map(Q,g['weights'])),Q(g['duration']))
 for a,b in zip(g['enclosure'],ref):encl(a,b,(case,'Poisson'));checks['dual_reference_corners']+=2
 same=[v for v in ivps if v['case']==case and v['candidate'] in ('Legacy','PackedInterval','FusedEdges','FusedDiagonal','Hybrid') and v['qualified']]
 base=next(v for v in same if v['candidate']=='Legacy')
 for v in same:
  assert v['run']['values']==base['run']['values']
  for k in ('accepted','rejected','rhs','linear_solves','linear_iterations','counters'):assert v['run'][k]==base['run'][k],(case,v['candidate'],k)
  w=v['work'];pr=w['residual_profile'];assert w['solves']==w['returned']==v['run']['linear_solves']
  assert w['preparations']==v['run']['accepted']+v['run']['rejected']
  assert pr['fast_checks']+pr['interval_checks']==w['returned']+pr['fallbacks']
  assert len(v['profiles'])==3
  for a in v['profiles']:
   assert all(a[k]==pr[k]for k in ('fast_checks','interval_checks','fallbacks','fmas','max_rounding'))
 for v in [v for v in ivps+triggers if v['case']==case]:
  if not v['qualified']:continue
  e=error(v['run']['values'],ref);assert e<=Q(v['error_upper'])<=Q(1,10**6)
  checks['native_adjoint_corners']+=2*n
  if v in triggers:
   w=v['work'];assert w['solves']==v['run']['linear_solves'];assert w.get('prefix_solves',0)<=v['run']['linear_iterations']
  native.append(dict(case=case,candidate=v['candidate'],error_upper=up(e),reference_cells=cells,reference_order=p))
# Independent analytic DAE source, original exact represented initial state.
def exp_pos(t):
 term=s=Q(1)
 for k in range(1,151):term*=t/k;s+=term
 tail=term*t/151/(1-t/152)
 return s,s+tail
def exact_state(case,n,t):
 if case=='growth2':
  l,h=exp_pos(8*t);l/=1024;h/=1024;st=[(l,h),((l-Q(1,1024))/8,(h-Q(1,1024))/8)]
  return st+st
 if case=='coupled_clock':
  x=Q(1,4)/(1+t*t/8);l,h=log1p(t*t/8);y=(Q(1,16)+l,Q(1,16)+h);st=[(x+y[0],x+y[1]),y]
  return st+st
 return state(case,n,t)
goals=read(P/'goal.log','ESC_GOAL');assert len(goals)==54
goal_resource=[];esc=collections.Counter()
for v in goals:
 t=Q(v['end']);w=list(map(Q,v['weights']));exact=exact_state(v['case'],v['n'],t);j=dot_box(exact,w)
 a=v['attempts'];assert len(a)<=2 and a
 assert v['qualified']==a[-1]['accepted']
 assert all(not r['accepted']for r in a[:-1])
 if len(a)>1:assert a[0]['multiplier']>1 and a[1]['multiplier']==1;esc[v['candidate']]+=1
 if v['total_rhs'] is not None:
  assert v['total_rhs']==sum(r['audit']['stats']['counters']['rhs_calls']for r in a if r['audit']['stats']is not None)
 assert v['certificate_actions']==sum(c['work']['matrix_actions']for r in a for c in r['certificates']if c['work']is not None)
 for r in a:
  if not r['physical_qualified']:assert not r['accepted'];continue
  jn=sum(Q(x)*c for x,c in zip(r['terminal_state'],w));ex=(j[0]-jn,j[1]-jn)
  encl([-r['state_bound'],r['state_bound']],ex,(v['case'],v['end'],'state'));checks['goal_state_corners']+=2
  for c in r['certificates']:
   if c['work']is not None:encl(c['work']['signed_error'],ex,(v['case'],v['end'],c['route']));checks['goal_secant_corners']+=2
  if r['enclosure']is not None:
   encl(r['enclosure'],ex,(v['case'],'intersection'));checks['goal_intersection_corners']+=2
   l,h=map(Q,r['enclosure']);assert l>=-Q(r['state_bound']) and h<=Q(r['state_bound'])
  if r['accepted']:assert max(map(abs,ex))<=Q(v['absolute']) and max(map(abs,r['enclosure']))<=v['absolute'];checks['accepted_goal_budget']+=1
 goal_resource.append(dict(case=v['case'],end=v['end'],budget=v['budget'],candidate=v['candidate'],qualified=v['qualified'],attempts=[r['multiplier']for r in a],rhs=v['total_rhs'],certificate_actions=v['certificate_actions'],median_ns=v['median_ns'],enclosure=a[-1]['enclosure']))
print("SCIENTIFIC",dict(checks),"goal",sum(v['qualified']for v in goals),"escalations",dict(esc),"max native",max(v['error_upper']for v in native))