from pathlib import Path
from fractions import Fraction as Q
from itertools import product
import json,collections,hashlib
import numpy as np
from scipy.optimize import brentq
from reference import solve,evaluate
from structural import native,ev,br,deriv,add,sub
from exact_native import exact_cells,counts
from exact_phase import encl,check_native,query_check
P=Path(__file__).parent
rows=[json.loads(l.split('CELL ',1)[1])for l in (P/'affine_training.log').read_text().splitlines()if 'CELL {'in l]
fields=['solver','cert','order','method'];cases=['alternating','distinct','overlap']
axes=[['Affine5'],['Max','Integral4'],['Shared','Correlated'],['JointCentered','JointGreen','AtlasFresh','AtlasReuse']]
assert len(rows)==48 and {(r['case'],*(r[k]for k in fields))for r in rows}=={(c,*k)for c in cases for k in product(*axes)}
v=Q(1,8192);center=[1-v,v,Q(0)];end=.002;q=float(Q(1,16384));rad=2.**-26
rates=[30000000,60000000,30000000]
def edge(case,i):
 if case=='distinct':return [(1,Q(1,8192)),(2,Q(5,32768)),(1,Q(3,16384))][i]
 return [(1,Q(3,16384)),(0,Q(5,32768))][i]
def center_for(v):return [float(1-v),float(v),0.]
def period(rate,seed,method):
 sol=solve(rate,(0,end),seed if method=='Radau'else seed[:2],method,3e-13,1e-16)
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
ss=seeds(rad)
nom={}
for case in cases:
 for i in range(3 if case=='distinct'else 2):
  j,w=edge(case,i)
  for method in ['Radau','BDF']:
   k=(j,w,method)
   if k not in nom:nom[k]=period(rates[j],center_for(w),method)
refs={}
for s0 in ss:
 for method in ['Radau','BDF']:
  first=period(rates[0],s0,method)
  for case in cases:
   seq=[];i=0;a=0.;sol,t=first
   for n in range(11):
    seq.append((i,a,a+t,sol))
    if a+t>end:break
    j,w=edge(case,i);a+=t;i=j;sol,t=nom[i,w,method]
   assert len(seq)==7,(case,method,len(seq))
   refs[case,tuple(s0),method]=seq
def event_ref(case,s,i,root):
 a=refs[case,tuple(s),'Radau'][i][2];b=refs[case,tuple(s),'BDF'][i][2]
 assert root[0]-abs(a-b)-2e-14<=a<=root[1]+abs(a-b)+2e-14,(case,i,root,a)
 counts['event_reference_checks']+=1
def state(case,s,method,i,t):
 mode,a,b,sol=refs[case,tuple(s),method][i]
 assert a-2e-12<=t<=b+2e-12,(case,i,a,t,b)
 return evaluate(sol,max(0.,t-a),method)
out=[];stops=collections.Counter();max_spread=0.;max_joint_ratio=0.;pairs={}
for r in rows:
 case=r['case'];method=r['method'];is_atlas=method.startswith('Atlas')
 v0={k:r[k]for k in ['case',*fields,'radius']}
 assert r['radius']==rad
 if is_atlas:
  run=r['atlas'];w=run['work']['phase'];es=run['epochs'];events=run['events'];ph=run['phases']
  assert w['native_attempts']<=w['charged_native_attempts']<=4096 and w['scan_slabs']<=w['charged_scan_slabs']<=512
  assert w['committed']<=10 and w['reset_checks']<=64
  assert w['committed']==sum(e['event']['committed']for e in events)
  assert w['native_runs']==len(ph) and w['cache_hits']==sum(e['phase']['cached']for e in es)
  assert run['work']['cache_inserts']<=10
  certs=[]
  for i,p in enumerate(ph):
   assert p['initial']['l1_radius']==(rad if i==0 else 0.)
   certs.append(check_native(p['slabs'],r['cert'],p['initial']['l1_radius'],p['initial']['center'],rates[p['mode']]))
  seen={}
  for i,e in enumerate(es):
   z=e['phase'];p=ph[z['phase_index']];assert e['mode']==p['mode']
   mode=0 if i==0 else events[i-1]['target'];assert e['mode']==mode
   assert z['clock']==([0.,0.]if i==0 else events[i-1]['event']['absolute'])
   if i:
    assert events[i-1]['event']['committed'] and p['initial']['center']==events[i-1]['reset_center'] and p['initial']['l1_radius']==0.
   else:assert p['initial']['center']==list(map(float,center[:2]))
   assert z['relative_safe']==p['safe'] and Q(z['absolute_safe'])<=Q(z['clock'][0])+Q(z['relative_safe'])
   key=(mode,tuple(p['initial']['center']))
   if z['cached']:
    assert i and key in seen and seen[key]==z['phase_index'];counts['cache_identity_checks']+=1
   elif i and method=='AtlasReuse':
    assert key not in seen
    if p['winner'] is not None:seen[key]=z['phase_index']
  assert run['final_mode']==(events[-1]['target']if events and events[-1]['event']['committed']else es[-1]['mode'])
  for i,e in enumerate(events):
   z=e['event'];p=ph[z['phase_index']];target,rv=edge(case,e['from'])
   assert z['epoch']==i and e['from']==es[i]['mode'] and e['target']==target
   assert z['guard']==0 and e['reset_center']==center_for(rv)[:2]
   c=es[i]['phase']['clock'];lo=Q(c[0])+Q(z['relative'][0]);hi=Q(c[1])+Q(z['relative'][1])
   encl(z['absolute'],lo,hi);assert Q(z['width_upper'])>=Q(z['absolute'][1])-Q(z['absolute'][0])
   assert z['relative']==p['guards'][z['guard']]['root']
   if i:assert z['dwell_lower']==z['relative'][0]
   else:assert z['dwell_lower'] is None
   if z['committed']:
    assert z['width_upper']<=1e-6 and z['absolute'][1]<end and len(e['reset_guards'])==2
    for k,g in enumerate(e['reset_guards']):
     threshold=Q(1,16384)-(Q(1,2**28 if case=='overlap'else 2**18)if k else 0)
     x=threshold-rv
     assert g['center']==e['reset_center'] and g['incoming_l1']==0. and g['times']==[0.,0.]
     encl(g['enclosure'],x,x);assert g['enclosure'][1]<0
     counts['exact_reset_guard_checks']+=1
   counts['clock_sum_checks']+=1
  for z in run['queries']:
   i=z['epoch'];e=es[i]['phase'];p=ph[e['phase_index']];a,b=z['horner'],z['centered']
   assert a['t']==b['t'] and a['phase']==b['phase']
   for y in (a,b):query_check(y,p,e,certs[e['phase_index']])
   assert all(x[0]<=y[0]<=y[1]<=x[1]for x,y in zip(a['enclosure'],b['enclosure']))
   counts['centered_strict_queries']+=a['enclosure']!=b['enclosure']
  for s in ss:
   for i,e in enumerate(events):event_ref(case,s,i,e['event']['absolute'])
   for z in run['queries']:
    i=z['epoch'];t=z['horner']['t'];xa=state(case,s,'Radau',i,t);xb=state(case,s,'BDF',i,t)
    spread=float(max(abs(xa-xb)));max_spread=max(max_spread,spread)
    for query in [z['horner'],z['centered']]:
     assert all(a-spread-2e-13<=x<=b+spread+2e-13 for x,(a,b)in zip(xa,query['enclosure'])),(v0,t,xa,query['enclosure'])
     counts['atlas_reference_components']+=3
  good=run['stop']=='Horizon'
  if good:
   assert w['committed']==6 and len(es)==7 and es[-1]['phase']['absolute_safe']==end
   assert [e['mode']for e in es]==[i[0]for i in refs[case,tuple(ss[0]),'Radau']]
  last=next((z for z in run['queries']if z['centered']['t']==end),None)
  if good:assert last is not None
  v0.update({'stop':run['stop'],'reason':run['reason'],'qualified':good,'committed':w['committed'],'attempts':w['native_attempts'],'rhs_proxy':w['driver_rhs']+w['projection_rhs'],'native_runs':w['native_runs'],'cache_hits':w['cache_hits'],'cache_inserts':run['work']['cache_inserts'],'last_width':None if last is None else max(b-a for a,b in last['centered']['enclosure']),'last_horner_width':None if last is None else max(b-a for a,b in last['horner']['enclosure']),'absolute_widths':[e['event']['width_upper']for e in events],'events':[e['event']['absolute']for e in events]})
  pairs.setdefault((case,r['solver'],r['cert'],r['order']),{})[method]=run
 else:
  run=r['joint'];w=run['work']['common'];es=run['epochs'];events=run['events']
  assert w['native_attempts']<=w['charged_native_attempts']<=4096 and w['charged_scan_slabs']<=512
  assert w['transitions_committed']==sum(e['committed']for e in events)
  for i,e in enumerate(es):
   assert e['mode']==refs[case,tuple(ss[0]),'Radau'][i][0]
   check_native(e['slabs'],r['cert'],e['radius'],e['center'],rates[e['mode']])
   if i:assert e['start']==events[i-1]['event_time'][1] and e['radius']==events[i-1]['transport']['selected_l1']
  for s in ss:
   for i,e in enumerate(events):
    assert e['guard']==0;event_ref(case,s,i,e['event_time'])
   for i,e in enumerate(es):
    for query in e['queries']:
     t=query['t'];a=state(case,s,'Radau',i,t);b=state(case,s,'BDF',i,t)
     spread=float(max(abs(a-b)));max_spread=max(max_spread,spread)
     err=float(max(abs(a-query['state'])));bound=query['physical_error_upper']
     assert err<=bound+spread+2e-13,(v0,t,err,bound)
     if bound>0:max_joint_ratio=max(max_joint_ratio,err/bound)
     counts['joint_reference_components']+=3
  good=run['stop']=='Horizon'
  if good:assert w['transitions_committed']==6 and len(es)==7 and es[-1]['safe']==end
  last=next((q for e in es for q in e['queries']if q['t']==end),None)
  v0.update({'stop':run['stop'],'reason':run['reason'],'qualified':good,'committed':w['transitions_committed'],'attempts':w['native_attempts'],'rhs_proxy':w['driver_rhs']+w['projection_rhs']+w['reset_rhs']+w['gap_rhs'],'native_runs':w['native_runs'],'cache_hits':0,'cache_inserts':0,'last_width':None if last is None else 2*last['physical_error_upper'],'last_horner_width':None,'absolute_widths':[e['event_time'][1]-e['event_time'][0]for e in events],'events':[e['event_time']for e in events]})
 out.append(v0);stops[case,method,run['stop']]+=1
for key,d in pairs.items():
 a,b=d['AtlasFresh'],d['AtlasReuse'];assert a['stop']==b['stop'] and a['final_mode']==b['final_mode']
 def normalize(events):return [{**e,'event':{k:v for k,v in e['event'].items()if k!='phase_index'}}for e in events]
 assert normalize(a['events'])==normalize(b['events'])
 assert a['queries']==b['queries']
 assert b['work']['phase']['native_attempts']<=a['work']['phase']['native_attempts']
 counts['fresh_reuse_equivalence']+=1
s={'counts':dict(counts),'stops':[{'case':k[0],'method':k[1],'stop':k[2],'n':n}for k,n in sorted(stops.items())],'reference_periods':[{'mode':i,'center_v':float(v),'solver':m,'period':x[1]}for (i,v,m),x in nom.items()],'initial_samples':len(ss),'max_reference_spread':max_spread,'max_joint_ratio':max_joint_ratio,'raw_sha256':hashlib.sha256((P/'affine_training.log').read_bytes()).hexdigest(),'rows':out}
(P/'AFFINE_AUDIT.json').write_text(json.dumps(s,indent=2,allow_nan=False)+'\n')
print({k:v for k,v in s.items()if k!='rows'})
for c in cases:print(c,[(m,sum(r['qualified']for r in out if r['case']==c and r['method']==m))for m in axes[-1]])
