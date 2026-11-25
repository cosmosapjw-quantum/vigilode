from pathlib import Path
from fractions import Fraction as Q
from itertools import product
import json,collections,hashlib,bisect
import numpy as np
from scipy.optimize import brentq
from reference import solve,evaluate
from structural import native,ev
from exact_native import audit_run,exact_cells,counts
from exact_transport import audit_transport
P=Path(__file__).parent
def load(n,prefix):return [json.loads(l.split(prefix,1)[1])for l in (P/n).read_text().splitlines()if prefix+'{' in l]
rows=load('training.log','CELL ')
fields=['solver','sync','l1_route','gap_route','order'];cases=['separated','overlap','cancel']
axes=[['Mass','Prepared5','Scalar5'],['Centered','GreenMoment'],['BoxLift','JointTransport'],['Box','AffineDual'],['Shared','Correlated']]
assert len(rows)==144 and {(r['case'],*(r[f]for f in fields))for r in rows}=={(c,*k)for c in cases for k in product(*axes)}
q=Q(1,256);b=Q(1,64);end=Q(1,32);rad=Q(1,2**20);alpha=Q(1,128);eps=Q(1,4096)
M0=[[1.,0.,0.],[0.,.5,0.],[0.,.5,1.]]
M1=[[1023/1024,0.,.25],[0.,.5,0.],[1/1024,.5,.75]]
rates=[30000000,60000000,30000000]
# Guard full coefficients: constant, time, u,v,z. Fixed declared family.
def full_guard(case,mode,k):
 if mode==0:return [-q*(2 if k==0 else 1),Q(1),Q(0),Q(0),alpha]
 if k==0:
  return [-(b+Q(1,2**28) if case=='overlap'else Q(1./40.)),Q(1),Q(0),Q(0),alpha]
 if case=='cancel':return [-1-eps*b,eps,Q(1),Q(1),1+eps*alpha]
 return [-b,Q(1),Q(0),Q(0),alpha]
def reduced(h):return [h[0]+h[4],h[1],h[2]-h[4],h[3]-h[4]]
def val(h,t,x):return float(h[0])+float(h[1])*t+sum(float(a)*v for a,v in zip(h[2:],x))
def affine_range(h,t,xb):
 lo=hi=h[0]
 for c,(a,b) in zip(h[1:], [t,*xb]):
  lo+=min(c*a,c*b);hi+=max(c*a,c*b)
 return lo,hi
def contains(box,rng):
 assert Q(box[0])<=rng[0]<=rng[1]<=Q(box[1]),(box,rng)
def gap_exact(case,ev0,g):
 target=ev0['target'];h=full_guard(case,target,g['guard']);r=reduced(h);e=g['evidence']
 t=list(map(Q,e['times']));x=[list(map(Q,v))for v in e['differential_box']];c=list(map(Q,e['center']))
 z=[1-x[0][1]-x[1][1],1-x[0][0]-x[1][0]]
 direct=affine_range(h,t,[*x,z]);contains(e['direct'],direct)
 if g['cell']is not None:
  bp=ev0['reset']['cells'][g['cell']]['tube']['attempts'][-1]
  assert e['times']==bp['times'] and e['differential_box']==bp['differential_box'] and e['incoming_l1'] is None
  true_range=affine_range(r,t,x)
 else:
  assert e['times']==[ev0['event_time'][1]]*2
  assert e['center']==ev0['transport']['center'] and e['incoming_l1']==ev0['transport']['selected_l1']
  radius=Q(e['incoming_l1'])
  assert all(a<=cc-radius<=cc+radius<=bb for (a,bb),cc in zip(x,c))
  mid=r[0]+r[1]*t[1]+sum(a*v for a,v in zip(r[2:],c))
  er=radius*max(map(abs,r[2:]));true_range=mid-er,mid+er
  counts['restart_full_l1_checks']+=1
 assert g['enclosure']==e['enclosure']
 if e['route']=='AffineDual':
  for coeff,box in zip(r[2:],e['reduced_gradient']):contains(box,[coeff,coeff])
  contains(e['reduced_clock'],[r[1],r[1]])
  mid=r[0]+r[1]*t[1]+sum(a*v for a,v in zip(r[2:],c));contains(e['center_value'],[mid,mid])
  contains(e['centered'],true_range)
 contains(e['enclosure'],true_range)
 counts['exact_gap_bounds']+=1
 if ev0['committed']:assert e['enclosure'][1]<0
# exact initial dyadic triangle of 15 points
seeds=[]
for i in range(5):
 for j in range(5-i):
  a=rad*i/4;b0=rad*j/8;seed=[float(1-a-b0),float(b0),float(a)]
  assert sum(map(Q,seed))==1 and Q(1)-Q(seed[0])+Q(seed[1])<=rad
  seeds.append(seed)
refs={}
# Same physical winner equations in the three families: cancel guard
# reduces exactly to eps*(t+z/128-b). Independent numerical reference
# integrates in physical time, resetting at each independently located root.
for seed in seeds:
 for method in ['Radau','BDF']:
  y=seed if method=='Radau'else seed[:2];start=0.;segments=[];roots=[];resets=[]
  for i,rate in enumerate(rates):
   seg=solve(rate,(start,float(end)),y,method,3e-13,1e-16);segments.append(seg)
   if i<2:
    threshold=float(q if i==0 else b)
    tau=brentq(lambda t:t+alpha*evaluate(seg,t,method)[2]-threshold,start,float(end),xtol=5e-324,rtol=4*np.finfo(float).eps)
    old=evaluate(seg,tau,method);rst=np.asarray(M0 if i==0 else M1)@old
    assert min(rst)>=-1e-15 and abs(sum(rst)-1)<2e-14
    roots.append(tau);resets.append(rst);start=tau;y=rst if method=='Radau'else rst[:2]
  refs[tuple(seed),method]=(segments,roots,resets)
def band(a,b):return float(max(abs(a-b)))+2e-13
def inside(x,b,tol):return b[0]-tol<=x<=b[1]+tol
def boxcheck(x,b,tol):return len(x)==len(b)and all(inside(v,w,tol)for v,w in zip(x,b))
max_ratio=0.;max_spread=0.;max_error=0.
def compare(qs,mode,refa,refb):
 global max_ratio,max_spread,max_error
 if not qs:return
 tt=np.array([x['t']for x in qs]);yy=np.array([x['state']for x in qs]);bd=np.array([x['physical_error_upper']for x in qs])
 assert np.all(np.isfinite(bd))and np.all(bd>=0)
 a=evaluate(refa[0][mode],tt,'Radau').T;b=evaluate(refb[0][mode],tt,'BDF').T
 err=np.max(abs(a-yy),axis=1);spread=np.max(abs(a-b),axis=1)
 assert np.all(err<=bd+spread+2e-13),(mode,float(max(err-bd-spread)))
 max_ratio=max(max_ratio,float(max(err/np.maximum(bd,1e-300))))
 max_spread=max(max_spread,float(max(spread)));max_error=max(max_error,float(max(err)))
 counts['numeric_queries']+=len(qs)
def audit_epoch(e,rate):
 prev=Q(e['radius']);last=None;ends=[];polys=[]
 for s in e['slabs']:
  c=s['certificate'];assert c and c['global_qualified'] and c['conservation']
  pp=native(s)[:2];counts['exact_coeff_checks']+=5*len(s['native']['y0'])
  a,b=map(Q,s['times']);assert b>a and Q(c['incoming'])==prev
  if last is None:assert a==Q(e['start']) and [ev(x,Q(0))for x in pp]==list(map(Q,e['center']))
  else:assert a==last[0]and [ev(x,Q(0))for x in pp]==last[1]
  prev=exact_cells(s,pp,rate,'Integral4');last=(b,[ev(x,Q(1))for x in pp]);ends.append(b);polys.append(pp)
 for x in e['queries']:
  t=Q(x['t']);i=bisect.bisect_left(ends,t);s=e['slabs'][i];a,b=map(Q,s['times']);theta=(t-a)/(b-a)
  assert 0<=theta<=1
  u,v=[ev(p,theta)for p in polys[i]];z=1-u-v
  corr=max(abs(a-Q(b))for a,b in zip([u,v,z],x['state']))
  assert Q(x['physical_error_upper'])>=Q(s['certificate']['whole_x_error'])+corr
  counts['exact_query_checks']+=1
 if e['terminal']:
  assert e['terminal']['slabs']==e['slabs']
  audit_run(e['terminal'],rate,'Integral4',e['radius'])
out=[];stops=collections.Counter()
for row in rows:
 case=row['case'];w=row['work']['common'];wp=row['work']['charged_picard_attempts'];es=row['epochs'];events=row['events']
 assert w['native_attempts']<=w['charged_native_attempts']<=4096 and w['scan_slabs']<=w['charged_scan_slabs']<=512
 assert w['transitions_committed']<=w['transitions_attempted']<=4 and w['reset_cells']<=w['charged_reset_cells']<=64
 assert w['gap_checks']<=256 and w['picard_proofs']<=wp<=512
 assert w['transitions_committed']==sum(e['committed']for e in events)
 assert w['reset_cells']==sum(len(e['reset']['cells'])for e in events if e['reset'])
 assert row['final_mode']==w['transitions_committed'] and len(es)<=row['final_mode']+1
 counts['budget_checks']+=1
 for i,e in enumerate(es):
  assert e['mode']==i
  if i:
   prev=events[i-1];assert prev['committed'] and e['start']==prev['event_time'][1] and e['radius']==prev['transport']['selected_l1'] and e['center']==prev['transport']['center']
  else:assert e['start']==0 and e['radius']==float(rad) and e['center']==[1.,0.]
  audit_epoch(e,rates[i])
  if e['safe']>e['start']:assert len(e['queries'])==9
 for i,e in enumerate(events):
  assert e['from']==i and e['target']==i+1 and e['guard']==1
  root=e['event_time'];assert root[0]<root[1] and root[1]-root[0]<=1e-6
  if i:assert Q(e['dwell_lower'])<=Q(root[0])-Q(events[i-1]['event_time'][1])
  if e['committed']:
   assert e['reset'] and e['transport'] and e['reason']is None
   assert i==0 or e['dwell_lower']>=1e-4
   if i==0:assert len(e['gap'])==(len(e['reset']['cells'])+1)*2
   else:assert not e['gap']
  if e['transport'] and e['reset'] and e['reset']['reason'] is None:
   t=e['transport']
   fake={'transport':t,'preparation':e['reset'],'incoming_l1':t['selected_l1'],'l1_route':row['l1_route'],'event':root,'pre_slabs':es[i]['slabs']}
   audit_transport(fake,M0 if i==0 else M1)
  for g in e['gap']:gap_exact(case,e,g)
  if not e['committed']:assert i==len(events)-1
 for seed in seeds:
  a=refs[tuple(seed),'Radau'];b0=refs[tuple(seed),'BDF']
  for i,e in enumerate(es):
   if i<2:assert e['safe']<=a[1][i]+abs(a[1][i]-b0[1][i])+2e-14
   counts['safe_checks']+=1
   compare(e['queries'],i,a,b0)
   if e['terminal']:compare(e['terminal']['accepted']+e['terminal']['queries'],i,a,b0)
  for i,e in enumerate(events):
   tau=a[1][i];tb=b0[1][i];tol=abs(tau-tb)+2e-14
   assert inside(tau,e['event_time'],tol);counts['event_checks']+=1
   if i:assert e['dwell_lower']<=tau-a[1][i-1]+2*tol;counts['dwell_checks']+=1
   if not e['reset'] or e['reset']['reason'] is not None or e['transport'] is None:continue
   s=e['event_time'][1];rst=a[2][i];cover=False
   for cell in e['reset']['cells']:
    if not inside(tau,cell['times'],tol):continue
    cover=True
    assert boxcheck(rst[:2],[v['enclosure']for v in cell['reset']],band(rst,b0[2][i]))
    counts['reset_component_checks']+=2
    for j in range(5):
     t=tau+(s-tau)*j/4;xa=evaluate(a[0][i+1],t,'Radau');xb=evaluate(b0[0][i+1],t,'BDF');tolerance=band(xa,xb)
     assert boxcheck(xa[:2],cell['tube']['tube'],tolerance) and inside(xa[2],cell['tube']['attempts'][-1]['root_box'][0],tolerance)
     counts['tube_components']+=3
    xa=evaluate(a[0][i+1],s,'Radau');xb=evaluate(b0[0][i+1],s,'BDF')
    assert boxcheck(xa[:2],cell['sync']['synchronized'],band(xa,xb));counts['sync_components']+=2
   assert cover
   xa=evaluate(a[0][i+1],s,'Radau');xb=evaluate(b0[0][i+1],s,'BDF')
   assert sum(abs(xa[:2]-e['transport']['center']))<=e['transport']['selected_l1']+2*band(xa,xb)
   counts['carry_checks']+=1
   for g in e['gap']:
    if g['cell']is not None and not inside(tau,e['reset']['cells'][g['cell']]['times'],tol):continue
    left=max(tau,g['evidence']['times'][0]);right=g['evidence']['times'][1]
    if left>right:continue
    for j in range(5):
     t=left+(right-left)*j/4;xa=evaluate(a[0][i+1],t,'Radau');xb=evaluate(b0[0][i+1],t,'BDF');h=full_guard(case,i+1,g['guard'])
     ha,hb=val(h,t,xa),val(h,t,xb)
     assert inside(ha,g['enclosure'],abs(ha-hb)+2e-13);counts['numeric_gap_checks']+=1
 if row['stop']=='Horizon':
  assert len(events)==2 and w['transitions_committed']==2 and len(es)==3 and es[-1]['safe']==float(end)
  counts['qualified']+=1
 else:assert row['reason']is not None
 stops[row['stop']]+=1
 v={k:row[k]for k in ['case',*fields,'stop','reason']};v.update({'qualified':row['stop']=='Horizon','committed':w['transitions_committed'],'attempts':w['native_attempts'],'rhs_proxy':w['driver_rhs']+w['projection_rhs']+w['reset_rhs']+w['gap_rhs'],'picard_proofs':w['picard_proofs'],'charged_picard':wp,'gap_checks':w['gap_checks'],'green_calls':w['green_calls'],'joint_cells':row['work']['joint_cells'],'bound':es[-1]['terminal']['bound']if es and es[-1]['terminal']else None,'incoming_l1':es[-1]['radius']if es else None,'event_widths':[e['event_time'][1]-e['event_time'][0]for e in events]})
 out.append(v)
summary={'counts':dict(counts),'stops':dict(stops),'max_reference_ratio':max_ratio,'max_reference_spread':max_spread,'max_reference_error':max_error,'initial_samples':len(seeds),'rows':out,'raw_sha256':hashlib.sha256((P/'training.log').read_bytes()).hexdigest()}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,allow_nan=False)+'\n')
print({k:v for k,v in summary.items()if k!='rows'})
for c in cases:
 print(c,collections.Counter((r['order'],r['gap_route'],r['stop'])for r in out if r['case']==c))
