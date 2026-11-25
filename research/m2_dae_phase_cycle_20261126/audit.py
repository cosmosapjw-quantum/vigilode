from pathlib import Path
from fractions import Fraction as Q
from itertools import product
import json,collections,hashlib
import numpy as np
from scipy.optimize import brentq
from reference import solve,evaluate
from structural import native,ev,br,deriv,add,sub
from exact_native import exact_cells,counts
P=Path(__file__).parent
rows=[json.loads(l.split('CELL ',1)[1])for l in (P/'training.log').read_text().splitlines()if 'CELL {'in l]
fields=['solver','cert','order','method'];cases=['wide','narrow','overlap']
axes=[['Mass','Prepared5','Scalar5'],['Max','Integral4'],['Shared','Correlated'],['JointCentered','JointGreen','PhaseFresh','PhaseReuse']]
assert len(rows)==144 and {(r['case'],*(r[k]for k in fields))for r in rows}=={(c,*k)for c in cases for k in product(*axes)}
v=Q(1,8192);center=[1-v,v,Q(0)];end=.001;q=float(Q(1,16384))
def period(seed,method):
 sol=solve(3e7,(0,end),seed if method=='Radau'else seed[:2],method,3e-13,1e-16)
 t=brentq(lambda t:evaluate(sol,t,method)[1]-q,0,end,xtol=5e-324,rtol=4*np.finfo(float).eps)
 return sol,t
def seeds(rad):
 rad=Q(rad);out=[]
 for i in range(-4,5):
  for j in range(-4,5):
   if abs(i)+abs(j)>4 or i+j>0:continue
   du=rad*i/4;dv=rad*j/4
   s=[float(center[0]+du),float(center[1]+dv),float(-du-dv)]
   assert sum(map(Q,s))==1 and min(s)>=0 and abs(Q(s[0])-center[0])+abs(Q(s[1])-center[1])<=rad
   out.append(s)
 return out
nom={m:period(list(map(float,center)),m)for m in ['Radau','BDF']}
refs={}
for rad in [2.**-26,2.**-30]:
 for s in seeds(rad):
  refs[rad,tuple(s)]={m:period(s,m)for m in ['Radau','BDF']}
def encl(box,a,b):
 assert Q(box[0])<=a<=b<=Q(box[1]),(box,a,b)
def check_native(slabs,cert,rad,c0):
 prev=Q(rad);last=None;out={}
 for s in slabs:
  a,b=map(Q,s['times']);assert a<b
  pp=native(s)[:2];counts['coefficient_checks']+=5*len(s['native']['y0'])
  c=s['certificate']
  if not c or not c['global_qualified'] or not c.get('conservation'):
   counts['unqualified_slabs']+=1;break
  assert Q(c['incoming'])==prev
  if last is not None:assert a==last[0]and [ev(x,Q(0))for x in pp]==last[1]
  else:assert [ev(x,Q(0))for x in pp]==list(map(Q,c0))
  prev=exact_cells(s,pp,30000000,cert)
  last=(b,[ev(x,Q(1))for x in pp])
  out[tuple(s['times'])]=(pp,c)
 return out
def state(ref,m,i,t):
 if i==0:return evaluate(ref[m][0],t,m)
 a=ref[m][1]+(i-1)*nom[m][1]
 assert t>=a-2e-12,(i,t,a)
 return evaluate(nom[m][0],max(0.,t-a),m)
def true_event(ref,m,i):return ref[m][1]+i*nom[m][1]
def check_event(ref,i,root):
 a=true_event(ref,'Radau',i);b=true_event(ref,'BDF',i);tol=abs(a-b)+2e-14
 assert root[0]-tol<=a<=root[1]+tol,(i,root,a)
 counts['event_reference_checks']+=1
def query_check(a,p,e,certs):
 span=list(map(Q,a['phase']));clock=list(map(Q,e['clock']));t=Q(a['t'])
 assert Q(e['clock'][1])<=t<=Q(e['absolute_safe'])
 encl(a['phase'],t-clock[1],t-clock[0]);assert span[0]>=0 and span[1]<=Q(e['relative_safe'])
 assert a['cells']
 covered=span[0];hull=[[float('inf'),-float('inf')]for _ in range(3)]
 for cell in a['cells']:
  key=tuple(cell['native_times']);ps,c=certs[key]
  l,h=map(Q,key);x,y=map(Q,cell['phase'])
  assert l<=x<=y<=h and x<=covered
  assert x==max(l,span[0])and y==min(h,span[1])
  covered=max(covered,y)
  u0,u1=(x-l)/(h-l),(y-l)/(h-l)
  pol=[*ps,sub([Q(1)],add(*ps))]
  err=Q(cell['error_l1']);assert err==Q(c['whole_x_error'])
  for i,pp in enumerate(pol):
   b=cell['represented'][i];d=br(deriv(pp),u0,u1-u0)
   if min(d)>=0 or max(d)<=0:
    v=ev(pp,u0),ev(pp,u1);encl(b,min(v),max(v));counts['exact_monotone_ranges']+=1
   else:
    bb=[w for j in range(16)for w in br(pp,u0+(u1-u0)*Q(j,16),(u1-u0)/16)]
    if Q(b[0])<=min(bb)<=max(bb)<=Q(b[1]):counts['exact_bernstein_ranges']+=1
    else:counts['sample_only_ranges']+=1
   for j in range(5):
    z=ev(pp,u0+(u1-u0)*Q(j,4));encl(b,z,z);counts['represented_samples']+=1
   # Directed physical error inflation followed by valid invariant clipping.
   expect=max(Q(0),Q(b[0])-err),min(Q(1),Q(b[1])+err)
   encl(cell['enclosure'][i],*expect)
   hull[i][0]=min(hull[i][0],cell['enclosure'][i][0]);hull[i][1]=max(hull[i][1],cell['enclosure'][i][1])
   counts['inflation_checks']+=1
 assert covered==span[1] and hull==a['enclosure']
 counts['phase_queries']+=1
out=[];stops=collections.Counter();max_spread=0.;max_joint_ratio=0.;pairs={}
for r in rows:
 rad=r['radius'];method=r['method'];phase=method.startswith('Phase')
 v0={k:r[k]for k in ['case',*fields,'radius']}
 if phase:
  run=r['phase'];w=run['work'];es=run['epochs'];events=run['events'];ph=run['phases']
  assert w['native_attempts']<=w['charged_native_attempts']<=4096
  assert w['scan_slabs']<=w['charged_scan_slabs']<=512 and w['committed']<=3
  assert w['committed']==sum(e['committed']for e in events)
  assert w['native_runs']==len(ph) and w['cache_hits']==sum(e['cached']for e in es)
  assert all(g['enclosure'][1]<0 for g in run['reset_guards'])
  assert run['reset_center']==list(map(float,center[:2]))
  certs=[]
  for i,p in enumerate(ph):
   assert p['initial']['l1_radius']==(rad if i==0 else 0.)
   assert p['initial']['center']==list(map(float,center[:2]))
   certs.append(check_native(p['slabs'],r['cert'],p['initial']['l1_radius'],p['initial']['center']))
  for i,e in enumerate(es):
   assert e['clock']==([0.,0.]if i==0 else events[i-1]['absolute'])
   assert e['relative_safe']==ph[e['phase_index']]['safe']
   assert Q(e['absolute_safe'])<=Q(e['clock'][0])+Q(e['relative_safe'])
   if i:assert events[i-1]['committed']
   if e['cached']:assert e['phase_index']==1
  for i,e in enumerate(events):
   assert e['epoch']==i and e['guard']==0 and e['relative'][0]>0
   c=es[i]['clock'];lo=Q(c[0])+Q(e['relative'][0]);hi=Q(c[1])+Q(e['relative'][1])
   encl(e['absolute'],lo,hi);assert Q(e['width_upper'])>=Q(e['absolute'][1])-Q(e['absolute'][0])
   assert e['relative']==ph[e['phase_index']]['guards'][e['guard']]['root']
   if i:assert e['dwell_lower']==e['relative'][0]
   else:assert e['dwell_lower']is None
   if e['committed']:assert e['width_upper']<=1e-6 and e['absolute'][1]<end
   counts['clock_sum_checks']+=1
  for z in run['queries']:
   i=z['epoch'];e=es[i];p=ph[e['phase_index']];a,b=z['horner'],z['centered']
   assert a['t']==b['t'] and a['phase']==b['phase']
   for y in (a,b):query_check(y,p,e,certs[e['phase_index']])
   assert all(x[0]<=y[0]<=y[1]<=x[1]for x,y in zip(a['enclosure'],b['enclosure']))
   if a['enclosure']!=b['enclosure']:counts['centered_strict_queries']+=1
  for s in seeds(rad):
   ref=refs[rad,tuple(s)]
   for i,e in enumerate(events):check_event(ref,i,e['absolute'])
   for z in run['queries']:
    i=z['epoch'];t=z['horner']['t'];xa=state(ref,'Radau',i,t);xb=state(ref,'BDF',i,t)
    spread=float(max(abs(xa-xb)));max_spread=max(max_spread,spread)
    assert t<=true_event(ref,'Radau',i)+2e-12
    for query in [z['horner'],z['centered']]:
     assert all(a-spread-2e-13<=x<=b+spread+2e-13 for x,(a,b) in zip(xa,query['enclosure'])),(v0,t,xa,query['enclosure'])
     counts['phase_reference_components']+=3
  good=run['stop']=='Horizon'
  if good:
   assert w['committed']==2 and len(es)==3 and es[-1]['absolute_safe']==end
   assert len([z for z in run['queries']if z['horner']['t']==end])==1
  last=next((z for z in run['queries']if z['centered']['t']==end),None)
  v0.update({'stop':run['stop'],'reason':run['reason'],'qualified':good,'committed':w['committed'],'attempts':w['native_attempts'],'rhs_proxy':w['driver_rhs']+w['projection_rhs'],'native_runs':w['native_runs'],'cache_hits':w['cache_hits'],'last_width':None if last is None else max(b-a for a,b in last['centered']['enclosure']),'last_horner_width':None if last is None else max(b-a for a,b in last['horner']['enclosure']),'absolute_widths':[e['width_upper']for e in events],'max_relative_initial_after_reset':max((p['initial']['l1_radius']for p in ph[1:]),default=None),'events':[e['absolute']for e in events]})
  key=(r['case'],r['solver'],r['cert'],r['order'])
  pairs.setdefault(key,{})[method]=run
 else:
  run=r['joint'];w=run['work']['common'];es=run['epochs'];events=run['events']
  assert w['native_attempts']<=w['charged_native_attempts']<=4096 and w['charged_scan_slabs']<=512
  assert w['transitions_committed']==sum(e['committed']for e in events)
  for i,e in enumerate(es):
   check_native(e['slabs'],r['cert'],e['radius'],e['center'])
   if i:assert e['start']==events[i-1]['event_time'][1] and e['radius']==events[i-1]['transport']['selected_l1']
  for s in seeds(rad):
   ref=refs[rad,tuple(s)]
   for i,e in enumerate(events):
    assert e['guard']==0;check_event(ref,i,e['event_time'])
   for i,e in enumerate(es):
    for query in e['queries']:
     t=query['t'];a=state(ref,'Radau',i,t);b=state(ref,'BDF',i,t)
     spread=float(max(abs(a-b)));max_spread=max(max_spread,spread)
     err=float(max(abs(a-query['state'])));bound=query['physical_error_upper']
     assert err<=bound+spread+2e-13,(v0,t,err,bound)
     if bound>0:max_joint_ratio=max(max_joint_ratio,err/bound)
     counts['joint_reference_components']+=3
  good=run['stop']=='Horizon'
  if good:assert w['transitions_committed']==2 and len(es)==3 and es[-1]['safe']==end
  last=next((q for e in es for q in e['queries']if q['t']==end),None)
  v0.update({'stop':run['stop'],'reason':run['reason'],'qualified':good,'committed':w['transitions_committed'],'attempts':w['native_attempts'],'rhs_proxy':w['driver_rhs']+w['projection_rhs']+w['reset_rhs']+w['gap_rhs'],'native_runs':w['native_runs'],'cache_hits':0,'last_width':None if last is None else 2*last['physical_error_upper'],'last_horner_width':None,'absolute_widths':[e['event_time'][1]-e['event_time'][0]for e in events],'max_relative_initial_after_reset':max((e['radius']for e in es[1:]),default=None),'events':[e['event_time']for e in events]})
 out.append(v0);stops[method,run['stop']]+=1
for key,d in pairs.items():
 a,b=d['PhaseFresh'],d['PhaseReuse'];assert a['stop']==b['stop']
 assert [{k:v for k,v in e.items()if k!='phase_index'}for e in a['events']]==[{k:v for k,v in e.items()if k!='phase_index'}for e in b['events']]
 assert a['queries']==b['queries']
 assert b['work']['native_attempts']<=a['work']['native_attempts']
 counts['fresh_reuse_equivalence']+=1
s={'counts':dict(counts),'stops':[{'method':k[0],'stop':k[1],'n':n}for k,n in sorted(stops.items())],'reference_periods':{m:x[1]for m,x in nom.items()},'initial_samples':{str(rad):len(seeds(rad))for rad in [2.**-26,2.**-30]},'max_reference_spread':max_spread,'max_joint_ratio':max_joint_ratio,'raw_sha256':hashlib.sha256((P/'training.log').read_bytes()).hexdigest(),'rows':out}
(P/'AUDIT.json').write_text(json.dumps(s,indent=2,allow_nan=False)+'\n')
print({k:v for k,v in s.items()if k!='rows'})
for c in cases:print(c,[(m,sum(r['qualified']for r in out if r['case']==c and r['method']==m))for m in axes[-1]])
