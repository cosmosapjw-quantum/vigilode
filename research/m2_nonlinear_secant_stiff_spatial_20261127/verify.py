from pathlib import Path
from fractions import Fraction as Q
import gzip,json,collections,math
P=Path(__file__).parent
PRIOR=P.parent/'m2_spatial_vector_adjoint_20261127'
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
def grad(case,n,t,w):
 if case=='quadratic':return [pt(1+t+t*t/16),pt(1)]
 c=[w[i]+w[n+i]for i in range(n)];out=[]
 for j in range(n//2):
  d=1+Q(j+1)*t*t/8
  out.extend([pt(c[2*j]/d**2+c[2*j+1]*t*t/(2*d)),pt(c[2*j+1])])
 return out
checks=collections.Counter();models={};goal_ref={}
for r in read(P/'secant.log','SECANT_PRIMAL'):
 case,n=r['case'],r['n'];w=list(map(Q,r['weights']));exact=state(case,n,Q(1,8));v=list(map(Q,r['terminal']))
 assert len(exact)==len(v)==2*n
 j=dot_box(exact,w);numeric=sum((a*b for a,b in zip(v,w)),Q(0))
 goal_ref[case]=(j[0]-numeric,j[1]-numeric);models[case]=(n,w)
 for x,(a,b)in zip(v,exact):assert max(abs(x-a),abs(x-b))<=Q(r['error']);checks['terminal_state_corners']+=2
assert len(models)==5
for case in ('clock2','clock4','clock8','mixed_inverse'):
 for j in range(models[case][0]//2):
  a=Q(j+1);t=Q(1,16);s=Q(1,8);xt=Q(1,4)/(1+a*t*t/8);xs=Q(1,4)/(1+a*s*s/8)
  assert 2*a*s*t*(xt-xs)!=0;checks['noncommuting_blocks']+=1
goal=read(P/'secant.log','SECANT_GOAL');sign=read(P/'secant.log','SECANT_SIGN')
assert len(goal)==35 and len(sign)==32
assert len({(r['case'],r['candidate'])for r in goal})==35
for r in goal:
 case,k=r['case'],r['candidate']
 if 'work'not in r:
  assert case=='mixed_inverse'and k in ('Diagonal4','Scaled4','Scaled16')and not r['qualified'];continue
 w=r['work'];assert req(r['allocation'])==r['allocation']['requested_bytes']==0
 encl(w['signed_error'],goal_ref[case],(case,k));checks['goal_error_corners']+=2
 a,b=map(Q,w['signed_error']);assert b-a<=Q(w['width_upper'])
 offsets=[w[k]for k in ('terminal_offset','seam_offset','initial_offset','integrated')]
 encl(w['signed_error'],(sum(Q(v[0])for v in offsets),sum(Q(v[1])for v in offsets)),(case,k,'decomposition'));checks['goal_decomposition_corners']+=2
 assert r['qualified']==(w['width_upper']<=1e-6)
 assert w['scale']==(65536 if k.startswith('Scaled')else 1)
 assert w['cells']==w['slabs']*(1 if k=='WholeHorner'else 16 if k=='Scaled16'else 4)
 if k=='Shifted4':assert w['kernel']['max_residual_upper']>0 and max(map(abs,w['seam_offset']))>0
 if k in ('Diagonal4','Scaled4','Scaled16'):assert w['kernel']['guard_solves']==0 and w['kernel']['direct_divisions']>0
 else:assert w['kernel']['guard_solves']>0
 if k in ('Sub4Bernstein','Diagonal4','Scaled4','Scaled16','Shifted4'):assert w['kernel']['bernstein_ranges']>0
for r in sign:
 a,b=goal_ref[r['case']];encl(r['error'],(-b,-a),(r['case'],r['candidate'],'negative'));checks['negative_goal_corners']+=2
grad_rows=read(P/'secant.log','SECANT_GRADIENT');numeric=read(P/'secant.log','SECANT_NUMERIC');assert len(grad_rows)==15 and len(numeric)==10
for r in grad_rows:
 n,w=models[r['case']];exact=grad(r['case'],n,Q(1,8),w)
 assert len(exact)==len(r['enclosure'])and req(r['allocation'])==0
 for a,b in zip(r['enclosure'],exact):encl(a,b,(r['case'],r['candidate'],'gradient'));checks['gradient_corners']+=2
numeric_errors=[]
for r in numeric:
 n,w=models[r['case']];exact=grad(r['case'],n,Q(1,8),w);v=r['proposal']['gradient'];e=error(v,exact);assert e<=Q(1,10**6)
 checks['numeric_gradient_corners']+=2*n;numeric_errors.append({'case':r['case'],'candidate':r['candidate'],'error_upper':up(e)})
for case in models:
 a,b=[r for r in numeric if r['case']==case];assert a['proposal']==b['proposal']
for r in read(P/'secant.log','SECANT_QUERY'):
 n,_=models[r['case']];ex=state(r['case'],n,Q(r['t']))
 for x,(a,b)in zip(r['state'],ex):
  assert max(abs(Q(x)-a),abs(Q(x)-b))<=Q(r['error']),(r['case'],r['t'])
  checks['primal_query_corners']+=2
assert read(P/'secant.log','SECANT_EDGE')==[{'nonzero_incoming_refused':True,'unchanged_output':True}]
# Independent fixed-point integer direct Taylor, not uniformization:
# each multiplication/division is rounded outward at a fixed 256-bit
# dyadic grid; full-horizon tail is exact rational geometric majorant.
S=1<<256
def floor(x):return x.numerator//x.denominator
def ceil(x):return -floor(-x)
def fixed(x):return floor(Q(x)*S),ceil(Q(x)*S)
def scale(a,num,den):
 lo,hi=a
 if num<0:lo,hi=-hi,-lo;num=-num
 return lo*num//den,-((-hi*num)//den)
def fixed_mv(edges,v,trans):
 n=len(v);lo=[0]*n;hi=[0]*n
 if trans:
  for i,j,a,b in edges:
   l,h=scale(v[i],a,b);lo[i]-=h;hi[i]-=l;lo[j]+=l;hi[j]+=h
 else:
  for i,j,a,b in edges:
   l,h=scale((v[j][0]-v[i][1],v[j][1]-v[i][0]),a,b);lo[i]+=l;hi[i]+=h
 return list(zip(lo,hi))
def direct_taylor(rows,initial,T,trans):
 n=len(rows);v=[fixed(x)for x in initial]
 edges=[(i,j,a.numerator,a.denominator)for i,row in enumerate(rows)for j,a in row if j!=i and a]
 M=2*T*max(sum((a for j,a in row if j!=i),Q(0))for i,row in enumerate(rows))
 term_weight=Q(1);p=0
 while True:
  if p+2>M:
   f=term_weight*M/(p+1)/(1-M/(p+2))
   if f<=Q(1,1<<180):break
  p+=1;assert p<=512;term_weight*=M/p
 norm=sum(max(abs(l),abs(h))for l,h in v)if trans else max(max(abs(l),abs(h))for l,h in v)
 tail=ceil(norm*f);term=v.copy();res=v.copy()
 for k in range(1,p+1):
  h=T/k;term=[scale(x,h.numerator,h.denominator)for x in fixed_mv(edges,term,trans)]
  res=[(a+c,b+d)for (a,b),(c,d)in zip(res,term)]
 box=[(Q(a-tail,S),Q(b+tail,S))for a,b in res]
 assert max(b-a for a,b in box)<=Q(1,10**25),(p,float(M),max(float(b-a)for a,b in box))
 return box,p
grids=read(P/'stiff.log','STIFF_GRID');primals=read(P/'stiff.log','STIFF_PRIMAL');certs=read(P/'stiff.log','STIFF_CERT');nums=read(P/'stiff.log','STIFF_NUMERIC');qois=read(P/'stiff.log','STIFF_QOI')
assert [len(x)for x in (grids,primals,certs,nums,qois)]==[4,14,12,16,14]
refs={};primal_err=[];native_err=[];goal_errors=[]
for g in grids:
 case,n=g['case'],g['n'];rows=[[(j,Q(a))for j,a in row]for row in g['rows']]
 assert len(rows)==n and all(len(r)==6 for r in rows)
 assert all(0<=j<n and r>=0 for row in rows for j,r in row)
 y=list(map(Q,g['initial']));c=list(map(Q,g['goal']));T=Q(g['duration']);assert T==Q(1,8)
 # Independently verify row topology and Cartesian rates.
 w,ny,nz=(16,8,1)if case=='stiff2'else(8,8,4)if case=='stiff3'else(32,16,1)if case=='sphere512'else(16,8,1)
 for i,row in enumerate(rows):
  x=i%w;r=i//w;yy=r%ny;zz=r//ny
  idx=lambda a,b,c:(c*ny+b)*w+a
  if case.startswith('stiff'):
   assert row==[(idx((x-1)%w,yy,zz),Q(32)),(idx((x+1)%w,yy,zz),Q(32)),(idx(x,(yy-1)%ny,zz),Q(16)),(idx(x,(yy+1)%ny,zz),Q(16)),(idx(x,yy,(zz-1)%nz),Q(8)if nz>1 else Q(0)),(idx(x,yy,(zz+1)%nz),Q(8)if nz>1 else Q(0))]
  else:
   assert [j for j,a in row]==[yy*w+(x-1)%w,yy*w+(x+1)%w,max(0,yy-1)*w+x,min(ny-1,yy+1)*w+x,i,i]
   assert row[0][1]==row[1][1]and row[4][1]==row[5][1]==0
   if yy==0:assert row[2][1]==0
   if yy==ny-1:assert row[3][1]==0
  checks['stiff_row_edges']+=6
 pf,pp=direct_taylor(rows,y,T,False);af,ap=direct_taylor(rows,c,T,True)
 exact_goal=dot_box(af,y);pgoal=dot_box(pf,c);assert pgoal[0]<=exact_goal[1]and exact_goal[0]<=pgoal[1];checks['stiff_primal_dual_overlap']+=1
 refs[case]=(pf,af,exact_goal,c,y,pp,ap)
 for r in [r for r in primals if r['case']==case]:
  e=error(r['run']['values'],pf);assert e<=Q(1,10**6);checks['stiff_primal_corners']+=2*n
  primal_err.append({'case':case,'candidate':r['candidate'],'error_upper':up(e),'reference_order':pp})
 for r in [r for r in nums if r['case']==case]:
  e=error(r['run']['values'],af);assert e<=Q(r['error_upper'])<=Q(1,10**6);checks['stiff_adjoint_corners']+=2*n
  native_err.append({'case':case,'candidate':r['candidate'],'error_upper':up(e),'reference_order':ap})
 for r in [r for r in certs if r['case']==case]:
  assert req(r['allocation'])==0 and r['work']['width_upper']<=1e-6
  for x,b in zip(r['enclosure'],af):encl(x,b,(case,r['candidate']));checks['stiff_gradient_corners']+=2
  encl(r['work']['exact_goal'],exact_goal,(case,r['candidate'],'goal'));checks['stiff_goal_corners']+=2
  q=next(x for x in primals if x['case']==case and x['candidate']=='Dopri');j=sum((a*Q(b)for a,b in zip(c,q['run']['values'])),Q(0))
  encl(r['work']['signed_goal_error'],(exact_goal[0]-j,exact_goal[1]-j),(case,r['candidate'],'goal_error'));checks['stiff_cert_error_corners']+=2
 for r in [r for r in qois if r['case']==case]:
  q=next(x for x in primals if x['case']==case and x['candidate']==r['candidate']);j=sum((a*Q(b)for a,b in zip(c,q['run']['values'])),Q(0))
  encl(r['error'],(exact_goal[0]-j,exact_goal[1]-j),(case,r['candidate'],'native_goal_error'));checks['stiff_native_goal_corners']+=2
  goal_errors.append({'case':case,'candidate':r['candidate'],'upper':max(map(abs,r['error']))})
# Prior spatial campaign exact output replay, timing ignored only.
replay={}
for m,count,ignore in [('SPATIAL_GRID',6,()),('SPATIAL_PRIMAL',24,('median_ns','samples_ns')),('SPATIAL_GOAL',18,()),('SPATIAL_CERT',72,('median_ns','samples_ns')),('SPATIAL_NUMERIC',72,('median_ns','samples_ns')),('SPATIAL_QOI',72,()),('SPATIAL_DENSE',6,()),('SPATIAL_EDGE',1,()),('GATHER_GRID',4,()),('GATHER_KERNEL',8,('median_ns','samples_ns')),('GATHER_IVP',4,('median_ns','samples_ns'))]:
 a,b=read(PRIOR/'final_portfolio.log',m),read(P/'spatial_replay.log',m);assert len(a)==len(b)==count,(m,len(a),len(b))
 key=lambda r:{k:v for k,v in r.items()if k not in ignore}
 assert all(key(x)==key(y)for x,y in zip(a,b)),m
 replay[m]={'count':count,'logical':count,'exact':sum(x==y for x,y in zip(a,b))}
dump('REPLAY_COMPARE.json',replay)
dump('INDEPENDENT_CHECKS.json',{'assertions':dict(checks),'secant_success':sum('work'in r for r in goal),'secant_qualified':sum(r['qualified']for r in goal),'numeric_gradient_errors':numeric_errors,'stiff_primal_errors':primal_err,'stiff_adjoint_errors':native_err,'stiff_goal_errors':goal_errors,'reference':'exact Fraction rational/log state and sensitivity; independent direct Taylor fixed-point 256-bit outward integer arithmetic, exact rational tail <=2^-180 times input norm; original represented spatial generator'})
dump('RESOURCE_COMPARE.json',{'secant':goal,'gradients':grad_rows,'stiff_cert':[ {k:v for k,v in r.items()if k!='enclosure'}for r in certs],'stiff_primal':[{**{k:v for k,v in r.items()if k!='run'},'run':{k:v for k,v in r['run'].items()if k!='values'}}for r in primals],'stiff_adjoint':[{**{k:v for k,v in r.items()if k!='run'},'run':{k:v for k,v in r['run'].items()if k!='values'}}for r in nums]})
print('EXACT',dict(checks),'total',sum(checks.values()),'secant',len(sign),sum(r['qualified']for r in goal),'numeric grad',max(r['error_upper']for r in numeric_errors),'stiff primal',max(r['error_upper']for r in primal_err),'stiff adjoint',max(r['error_upper']for r in native_err),'stiff goal',max(r['upper']for r in goal_errors))
print('REPLAY',sum(r['count']for r in replay.values()),sum(r['exact']for r in replay.values()))
