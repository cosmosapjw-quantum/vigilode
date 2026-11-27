from pathlib import Path
from fractions import Fraction as Q
import gzip,json,collections,math
P=Path(__file__).parent
PRIOR=P.parent/'m2_goal_budget_shifted_spatial_20261127'
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
checks=collections.Counter()
goals=read(P/'cost.log','COST_BUDGET');exhaust=read(P/'cost.log','COST_EXHAUST')
assert len(goals)==75 and len(exhaust)==5
assert len({(r['case'],r['budget'],r['candidate'])for r in goals})==75
goal_rows=[]
for r in goals+[dict(r,case='clock2',n=2,budget='exhaust',absolute=1e-30,qualified=False,weights=[0.,1.,.5,0.])for r in exhaust]:
 w=list(map(Q,r['weights']));j=dot_box(state(r['case'],r['n'],Q(1,8)),w)
 attempts=r['attempts'];assert len(attempts)<=7 and attempts
 assert all(a['multiplier']>b['multiplier']for a,b in zip(attempts,attempts[1:]))
 assert all(not a['accepted']for a in attempts[:-1]);assert r['qualified']==attempts[-1]['accepted']
 assert r['total_rhs']==sum(a['audit']['stats']['counters']['rhs_calls']for a in attempts if a['audit']['stats']is not None)
 if 'total_steps'in r:assert r['total_steps']==sum(a['audit']['stats']['attempts']for a in attempts if a['audit']['stats']is not None)
 if 'certificate_actions'in r:assert r['certificate_actions']==sum(c['work']['matrix_actions']for a in attempts for c in a['certificates']if c['work']is not None)
 for a in attempts:
  if not a['physical_qualified']:assert not a['accepted'];continue
  assert a['terminal_state']is not None
  jn=sum((Q(x)*v for x,v in zip(a['terminal_state'],w)),Q(0));ex=(j[0]-jn,j[1]-jn)
  encl([-a['state_bound'],a['state_bound']],ex,(r['case'],'state'));checks['state_goal_corners']+=2
  for cert in a['certificates']:
   if cert['work']is not None:
    encl(cert['work']['signed_error'],ex,(r['case'],cert['route']));checks['secant_corners']+=2
  if a['enclosure']is not None:
   encl(a['enclosure'],ex,(r['case'],'intersection'));checks['intersection_corners']+=2
   l,h=map(Q,a['enclosure']);assert l>=-Q(a['state_bound'])and h<=Q(a['state_bound'])
   for cert in a['certificates']:
    if cert['work']is not None:
     u,v=map(Q,cert['work']['signed_error']);assert u<=l<=h<=v
  if a['accepted']:
   assert max(map(abs,a['enclosure']))<=r['absolute']
   assert max(map(abs,ex))<=Q(r['absolute']);checks['accepted_budget']+=1
 if r['budget']!='exhaust':
  goal_rows.append(dict(case=r['case'],budget=r['budget'],candidate=r['candidate'],qualified=r['qualified'],total_rhs=r['total_rhs'],attempts=len(attempts),multiplier=attempts[-1]['multiplier'],certificate_actions=r['certificate_actions'],median_ns=r['median_ns'],allocation=r['allocation'],enclosure=attempts[-1]['enclosure']))
# Original represented sparse transpose residual recomputed in exact rational arithmetic.
grids=read(P/'early.log','EARLY_GRID');shifts=read(P/'early.log','EARLY_RUN')
assert len(grids)==5 and len(shifts)==160
assert len({(r['case'],r['shift'],r['candidate'])for r in shifts})==160
models={}
def mv(rows,v):
 out=[Q(0)]*len(v)
 for i,edges in enumerate(rows):
  for j,a in edges:
   if i!=j and a:
    x=v[i]*a;out[i]-=x;out[j]+=x
 return out
for g in grids:
 n=g['n'];rows=[[(j,Q(a))for j,a in row]for row in g['rows']]
 assert len(rows)==n and all(len(row)==6 for row in rows)and all(0<=j<n and a>=0 for row in rows for j,a in row)
 models[g['case']]=(rows,list(map(Q,g['rhs'])),n)
for r in shifts:
 if not r['qualified']:continue
 rows,b,n=models[r['case']];x=list(map(Q,r['values']));z=Q(r['shift']);assert len(x)==n
 res=[a-z*v+w for a,v,w in zip(b,x,mv(rows,x))];rho=sum(map(abs,res))
 report=r['report'];assert rho<=Q(report['residual_l1']) and rho/z<=Q(report['error_l1'])<=Q(1,10**8)
 checks['shifted_exact_residual_components']+=n
 assert abs(sum(x)-sum(b)/z)<=Q(report['error_l1']);checks['shift_mass_identity']+=1
 if r['candidate']!='Fourier':
  assert req(r['prepare'])==0
  assert r['counters']['linear_iterations']==report['iterations']
  if r['candidate'].startswith('Early'):
   assert report.get('screen_checks',0)==r['counters']['diagnostic_matvecs']
   assert report.get('prefix_solves',0)==report['iterations']
   assert report.get('directed_checks',0)>=1
  if r['candidate']=='FourierTranspose':
   assert report['iterations']==0 and report['directed_checks']==1 and r['counters']['direct_solve_calls']==1

for case in models:
 for z in (4.,32.,256.,4096.):
  a=[r for r in shifts if r['case']==case and r['shift']==z and r['qualified']]
  f=next(r for r in a if r['candidate']=='Fourier')
  for r in a:
   diff=sum(abs(Q(x)-Q(y))for x,y in zip(r['values'],f['values']))
   assert diff<=Q(r['report']['error_l1'])+Q(f['report']['error_l1']);checks['shift_pair_consistency']+=1
# Independent direct Taylor with fixed-point integer outward arithmetic.
# Recenter after each bounded subcell, carry ONE exact l1 error radius via
# contractive exact semigroup, never amplify accumulated rounding by exp(M).
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
duals=read(P/'ivp.log','EARLY_DUAL');ivp=read(P/'ivp.log','EARLY_IVP')
assert len(duals)==6 and len(ivp)==51
native_grids=read(P/'ivp.log','EARLY_NATIVE_GRID')
assert len(native_grids)==6
for g in native_grids:
 n=g['n'];rows=[[(j,Q(a))for j,a in row]for row in g['rows']]
 assert len(rows)==n and all(len(row)==6 for row in rows)and all(0<=j<n and a>=0 for row in rows for j,a in row)
 if g['case']in models:assert rows==models[g['case']][0]
 else:models[g['case']]=(rows,[],n)

native=[]
for g in duals:
 case=g['case'];rows,_,n=models[case];ref,cells,p=direct_taylor(rows,list(map(Q,g['weights'])),Q(g['duration']))
 for x,b in zip(g['enclosure'],ref):encl(x,b,(case,'Poisson'));checks['dual_reference_corners']+=2
 for r in [r for r in ivp if r['case']==case]:
  if not r['qualified']:continue
  e=error(r['run']['values'],ref);assert e<=Q(r['error_upper'])<=Q(1,10**6)
  checks['native_adjoint_corners']+=2*n
  if 'work'in r:
   assert r['work']['returned']<=r['work']['solves']==r['run']['linear_solves']
   assert r['work']['preparations']==r['run']['accepted']+r['run']['rejected']
   if r['candidate']=='FourierTranspose':
    assert r['run']['linear_iterations']==0 and r['work']['directed_checks']==r['work']['returned']
   if r['candidate'].startswith('Early'):
    assert r['run']['counters']['diagnostic_matvecs']>=r['work'].get('screen_checks',0)
    assert r['run']['linear_iterations']>=r['work'].get('prefix_solves',0)

  native.append(dict(case=case,candidate=r['candidate'],error_upper=up(e),reference_cells=cells,reference_order=p))

# Exact original .21 replay (match .22 isolated cold test context)
replay={}
for file,markers in [('secant_replay.log',['SECANT_PRIMAL','SECANT_GOAL','SECANT_SIGN','SECANT_GRADIENT','SECANT_NUMERIC','SECANT_QUERY','SECANT_EDGE']),('spatial_replay.log',['SPATIAL_GRID','SPATIAL_PRIMAL','SPATIAL_GOAL','SPATIAL_CERT','SPATIAL_NUMERIC','SPATIAL_QOI','SPATIAL_DENSE','SPATIAL_EDGE','GATHER_GRID','GATHER_KERNEL','GATHER_IVP']),('stiff_replay.log',['STIFF_GRID','STIFF_PRIMAL','STIFF_CERT','STIFF_NUMERIC','STIFF_QOI'])]:
 for marker in markers:
  a,b=read(PRIOR/file,marker),read(P/file,marker)
  assert len(a)==len(b)>0,(marker,len(a),len(b))
  key=lambda r:{k:v for k,v in r.items()if k not in ('median_ns','samples_ns')}
  assert all(key(x)==key(y)for x,y in zip(a,b)),marker
  replay[marker]={'count':len(a),'logical':len(a),'exact':sum(x==y for x,y in zip(a,b))}
# Previous .22 overlap: compare all non-allocation and non-timing fields,
# separately retain resource changes (new route fields and cold context).
overlap={}
for file,m0,m1,seq,fields in [
 ('goal.log','GOAL_BUDGET','COST_BUDGET',goals,('case','budget','candidate')),
 ('shift.log','SHIFT_RUN','EARLY_RUN',shifts,('case','shift','candidate')),
 ('ivp.log','SHIFT_IVP','EARLY_IVP',ivp,('case','candidate'))]:
 a=read(PRIOR/file,m0);index={tuple(r[f]for f in fields):r for r in seq}
 changes=[]
 for x in a:
  k=tuple(x[f]for f in fields);y=index[k]
  ignore=('allocation','setup','prepare','median_ns','samples_ns','timing_repetitions')
  key=lambda r:{k:v for k,v in r.items()if k not in ignore}
  assert key(x)==key(y),(file,k,[z for z in key(x)if key(x)[z]!=key(y).get(z)])
  if any(x.get(q)!=y.get(q)for q in ('allocation','setup','prepare')):
   changes.append(dict(key=list(k),old={q:x.get(q)for q in ('allocation','setup','prepare')},new={q:y.get(q)for q in ('allocation','setup','prepare')}))
 overlap[m0]=dict(count=len(a),logical=len(a),resource_changes=changes)
dump('REPLAY_COMPARE.json',replay)
dump('PREVIOUS_OVERLAP.json',overlap)
dump('INDEPENDENT_CHECKS.json',dict(assertions=dict(checks),goal_qualified=sum(r['qualified']for r in goals),shift_qualified=sum(r['qualified']for r in shifts),shift_warm_zero=sum(r['qualified']and req(r['allocation'])==0 for r in shifts),ivp_qualified=sum(r['qualified']for r in ivp),native_accuracy=native,reference='Exact Fraction original shifted residual and goal rational/log; independent 256-bit fixed-point direct Taylor with exact geometric tail and recentered l1 radius.'))
dump('RESOURCE_COMPARE.json',dict(goal=goal_rows,shift=[{k:v for k,v in r.items()if k!='values'}for r in shifts],ivp=[{**{k:v for k,v in r.items()if k!='run'},'run':{k:v for k,v in r.get('run',{}).items()if k!='values'}}for r in ivp]))
print('EXACT',dict(checks),'total',sum(checks.values()),'goal',sum(r['qualified']for r in goals),'shift',sum(r['qualified']for r in shifts),'zeroheap',sum(r['qualified']and req(r['allocation'])==0 for r in shifts),'ivp',sum(r['qualified']for r in ivp),'max native',max(r['error_upper']for r in native))
print('REPLAY',sum(r['count']for r in replay.values()),sum(r['exact']for r in replay.values()),'OVERLAP',[(k,v['count'],len(v['resource_changes']))for k,v in overlap.items()])
