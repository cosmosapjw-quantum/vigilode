"""Independent 85-digit physical state/root oracle and exact residual coefficients."""
import json,sys,functools,math
from pathlib import Path
from fractions import Fraction as F
import mpmath as mp
mp.mp.dps=85
P=Path(__file__).resolve().parent
def f(x):return F.from_float(float(x))
def m(x):return mp.mpf(float(x))
def mf(x):return mp.mpf(x.numerator)/x.denominator
def read(file,tag):return [json.loads(l.split(tag,1)[1]) for l in file.read_text().splitlines() if tag+'{' in l]
def effective(g):
 t=g['terms']
 if len(t)==1 and t[0]['coefficient']!=0 and t[0]['factors'] and all(a==t[0]['factors'][0] for a in t[0]['factors']):
  return [{'coefficient':1.,'factors':t[0]['factors'][:1]}],len(t[0]['factors']),math.copysign(1,t[0]['coefficient'])
 return t,None,1
def event(g,t,y,dy):
 val=mp.mpf(0);der=mp.mpf(0)
 for term in g:
  c=m(term['coefficient']);vs=[];ds=[]
  for a in term['factors']:
   vs.append(m(a['constant'])+m(a['time'])*t+sum(m(w)*y[i] for i,w in a['state']))
   ds.append(m(a['time'])+sum(m(w)*dy[i] for i,w in a['state']))
  val+=c*mp.fprod(vs)
  der+=c*sum(v*mp.fprod(vs[:i]+vs[i+1:]) for i,v in enumerate(ds))
 return val,der
def make_oracle(src):
 n=len(src['y']);A=mp.matrix(n+1);y0=mp.matrix([m(x) for x in src['y']]+[1])
 for i in range(n):
  for j in range(n):A[i,j]=m(src['a'][i*n+j])
  A[i,n]=m(src['b'][i])
 @functools.lru_cache(None)
 def state(t):
  # Independent high precision homogeneous augmented matrix exponential.
  if n==1:
   lam=-A[0,0];ex=mp.exp(-lam*t);v=ex*y0[0]+(A[0,1]*(-mp.expm1(-lam*t))/lam if lam else A[0,1]*t)
   return [v],[A[0,0]*v+A[0,1]]
  v=mp.expm(A*t)*y0;dy=A*v
  return list(v)[:n],list(dy)[:n]
 return state
def coeffs(src,seg):
 n=len(src['y']);a,b=map(f,seg['times']);h=b-a
 H=src['H'];u=seg['u'];p=[]
 for i in range(n):
  d=[sum(f(H[j][k])*f(u[k*n+i]) for k in range(8)) for j in range(3)]
  y0,y1=f(seg['y0'][i]),f(seg['y1'][i])
  p.append([y0,y1-y0+d[0],d[1]-d[0],d[2]-d[1],-d[2]])
 beta=[F(0)]*5
 for i in range(n):
  r=[(j+1)*p[i][j+1] if j<4 else F(0) for j in range(5)]
  for j in range(5):
   r[j]-=h*(sum(f(src['a'][i*n+k])*p[k][j] for k in range(n))+(f(src['b'][i]) if j==0 else 0))
  for k in range(5):beta[k]=max(beta[k],abs(sum(r[j]*F(math.comb(k,j),math.comb(4,j)) for j in range(k+1))))
 return a,h,p,beta
def audit_source(src):
 state=make_oracle(src);g,mult,orientation=effective(src['g'])
 root=None;direction=None
 if src['expected']=='Unique':
  lo,hi=mp.mpf(0),m(src['end']);left=event(g,lo,*state(lo))[0];right=event(g,hi,*state(hi))[0]
  assert left*right<0,src['name']
  for _ in range(190):
   mid=(lo+hi)/2;v=event(g,mid,*state(mid))[0]
   if v*left>0:lo=mid
   else:hi=mid
  root=(lo+hi)/2
  direction='Touch' if mult is not None and mult%2==0 else 'Up' if event(g,root,*state(root))[1]*orientation>0 else 'Down'
 packets=[]
 for seg in src['packets']:
  a,h,p,beta=coeffs(src,seg);samples=[]
  for q in map(F,[0,.25,.5,.75,1]):
   t=mf(a+h*q);v,dy=state(t)
   u=[mf(sum(c*q**j for j,c in enumerate(row))) for row in p]
   gv,dg=event(g,t,v,dy);gu,_=event(g,t,u,[mp.mpf(0)]*len(u))
   samples.append({'q':q,'t':t,'y':v,'error':max(abs(a-b) for a,b in zip(u,v)),'g':gv,'dg':dg,'gap':abs(gv-gu)})
  packets.append((seg,beta,samples))
 return root,direction,packets
def inside(x,b):return m(b[0])<=x<=m(b[1])
def run(name):
 file=P/(name+'.log');sources=read(file,'SOURCE ');rows=read(file,'CELL ')
 assert len(sources)==(9 if name in ['training','precision'] else 4)
 expected={'training':63,'holdout':4,'precision':126}
 if name=='precision_holdout':
  sel=json.loads((P/'PRECISION_SELECTION.json').read_text())
  expected[name]=sum(v['selected'] is not None for g in sel['groups'].values() for v in g.values())
 assert len(rows)==expected[name],(name,len(rows),expected[name])
 assert len({(r['case'],r['candidate'],r.get('time_tol',1e-5)) for r in rows})==len(rows)
 oracles={s['name']:audit_source(s) for s in sources}
 sums=[]
 for r in rows:
  root,direction,packets=oracles[r['case']];v=r['report'];summary={k:r[k] for k in ['case','group','candidate','expected','median_ns']}
  summary['time_tol']=r.get('time_tol',1e-5);summary['audit_pass']=False;summary['status']='Refused' if v is None else v['status'];summary['reason']=r.get('reason') if v is None else v['reason'];summary['root_oracle']=None if root is None else mp.nstr(root,70)
  if v is None:sums.append(summary);continue
  worst=0.;checked=0;no_root=0
  assert v['work']['slabs']==len(v['proofs'])
  for idx,(pr,(seg,beta,samples)) in enumerate(zip(v['proofs'],packets)):
   assert pr['times']==seg['times'];assert all(f(g)>=b for g,b in zip(pr['defect_bernstein'],beta)),(r['case'],idx,'residual')
   if idx==0:assert pr['incoming']==0.
   else:assert pr['incoming']==v['proofs'][idx-1]['outgoing']
   for q in samples:
    radius=pr['incoming'] if q['q']==0 else pr['outgoing'] if q['q']==1 else pr['whole_radius']
    # Even endpoint also lies inside whole tube.
    assert q['error']<=m(radius)+mp.mpf('1e-75'),(r['case'],r['candidate'],idx,q['q'],'state error',str(q['error']),radius)
    if radius>0:worst=max(worst,float(q['error']/m(radius)))
    assert q['error']<=m(pr['whole_radius'])+mp.mpf('1e-75')
    assert all(inside(x,b) for x,b in zip(q['y'],pr['state_range']))
    assert inside(q['g'],pr['event_range']) and inside(q['dg'],pr['derivative']), (r['case'],r['candidate'],idx,'event/derivative',q)
    assert q['gap']<=m(pr['event_gap_upper'])+mp.mpf('1e-75')
    checked+=1
   if pr['disposition']=='NoRoot':
    no_root+=1
    if root is not None:assert not m(pr['times'][0])<=root<=m(pr['times'][1]),'false no-root'
  assert len(v['proofs'])<=len(packets)
  if v['status']=='Unique':
   assert root is not None and inside(root,v['bracket']) and v['kind']==direction
   assert no_root==len(v['proofs'])-1 and v['proofs'][-1]['disposition']=='Unique'
   assert m(v['width'])>=m(v['bracket'][1])-m(v['bracket'][0])
  if v['status']=='NoRoot':
   assert root is None and no_root==len(packets)
  qualified=v['qualified'] and v['status']==r['expected']
  if qualified and root is not None:assert v['width']<=summary['time_tol']
  summary.update(audit_pass=qualified,proofs=len(v['proofs']),samples=checked,max_state_error_over_bound=worst,width=v['width'],work=v['work'],selected_actual_route=v['route'])
  sums.append(summary)
 d={'cases':len(rows),'qualified':sum(r['audit_pass'] for r in sums),'refused':sum(r['status']=='Refused' for r in sums),'unresolved':sum(r['status']=='Unresolved' for r in sums),'samples_checked':sum(r.get('samples',0) for r in sums),'slab_proofs':sum(r.get('proofs',0) for r in sums),'max_ratio':max(r.get('max_state_error_over_bound',0) for r in sums),'rows':sums,'precision_dps':mp.mp.dps}
 (P/(name.upper()+'_AUDIT.json')).write_text(json.dumps(d,indent=2,sort_keys=True,allow_nan=False)+'\n')
 print(name,{k:v for k,v in d.items() if k!='rows'})
 for r in sums:
  if not r['audit_pass']:print('NOT QUALIFIED',r['case'],r['candidate'],r['status'],r['reason'])
if __name__=='__main__':run(sys.argv[1])
