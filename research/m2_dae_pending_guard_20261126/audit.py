from pathlib import Path
import hashlib,json,collections,itertools,mpmath as mp
P=Path(__file__).parent;mp.mp.dps=85
def m(x):return mp.mpf(float(x))
def width(b):return m(b[1])-m(b[0])
def inside(b,x):return m(b[0])<=x<=m(b[1])
def roots(case,rad):
 if case=='variable':
  f=lambda d:2*(mp.mpf(3)/16-d)/(1+mp.sqrt(1+16*(mp.mpf(3)/16-d)))
  return f(rad),f(-rad)
 if case=='physical-floor':return (mp.mpf(1)/8-rad/2,mp.mpf(1)/8+rad/2)
 if case=='checkpoint-cap':return (mp.mpf(3)/4,mp.mpf(3)/4)
 return (mp.mpf(1)/8,mp.mpf(1)/8)
def exact(case,t,d):
 if case in ['boundary','physical-floor']:return 2*t+d-mp.mpf(1)/4,mp.mpf(2),[t+d]
 if case=='variable':return t+d+4*t*t-mp.mpf(3)/16,1+8*t,[t+d]
 if case=='checkpoint-cap':return t-mp.mpf(3)/4,mp.mpf(1),[t]
 if case=='nonlinear-z':return t*t+t-mp.mpf(9)/64,2*t+1,[t]
 if case=='coupled':return t-mp.mpf(1)/8,mp.mpf(1),[8*t+32*t*t+d,t+d]
 if case=='pencil':return t-mp.mpf(1)/8,mp.mpf(1),[t+t*t/2]
 raise ValueError(case)
fields=['solver','root_route','derivative','value','mode']
cases=['boundary','variable','physical-floor','checkpoint-cap','nonlinear-z','coupled','pencil']
rows=[json.loads(l.split('CELL ',1)[1]) for l in (P/'training.log').read_text().splitlines() if 'CELL {' in l]
keys={(r['case'],*(r[f] for f in fields))for r in rows}
cats=json.loads((P/'CANDIDATES.json').read_text())['routes']
assert len(rows)==len(keys)==1050
assert keys=={(k,*(v[f]for f in fields))for k in cases for v in cats}
out=[];counts=collections.Counter();failures=collections.Counter();max_width=mp.mpf(0)
for r in rows:
 case=r['case'];rad=m(r['rad']);lo,hi=roots(case,rad)
 v={k:r[k]for k in ['case',*fields]};v['qualified']=False
 v['true_root_width']=float(hi-lo)
 if 'error' in r:
  v['refusal']=r['error'];failures[v['refusal']]+=1;out.append(v);continue
 counts['first_receipts']+=1
 assert m(r['safe'])<=lo,(v,r['safe'],str(lo))
 rrs=[r['first']]+([r['retry']]if r.get('retry') else [])
 if r.get('retry'):counts['retry_receipts']+=1
 assert r['attempts_used']==sum(a['attempts']for a in rrs)<=100000
 proxy=0;guards=0;hints=0;refinements=0;pending_records=0
 cp=r.get('checkpoint')
 if cp:
  counts['checkpoint_receipts']+=1
  assert m(cp['time'])==m(r['first']['safe'])<=lo
  assert cp['remaining_attempts']==100000-r['first']['attempts']
  assert cp['retry_h0']==r['h0']/2
  for d in [-rad,mp.mpf(0),rad]:
   _,_,x=exact(case,m(cp['time']),d)
   for a,b in zip(x,cp['differential']):
    assert abs(a-m(b))<=m(cp['radius']),(v,cp,str(a))
    counts['checkpoint_component_checks']+=1
  proxy+=cp['projection']['rhs_count']
  if r.get('retry') and r['retry']['pending']['records']:
   # the first pending record may occur after the restart; check carried
   # radius via the checkpoint witness and original-family guard probes.
   assert r['retry']['pending']['records'][0]['times'][0]>=cp['time']
 for rr in rrs:
  assert rr['work']['inspected']==rr['accepted_steps'],(v,rr['work'],rr['accepted_steps'])
  proxy+=rr['driver']['rhs_calls']+rr['projection']['rhs_count']+rr['work']['projection']['rhs_count']
  guards+=rr['work']['guard_evals'];hints+=rr['work']['hint_calls']
  for s in rr['guard_slabs']:
   proxy+=s['prepared_work']['rhs_count']
   a,b=map(m,s['times'])
   for i in range(9):
    t=a+(b-a)*i/8
    for d in [-rad,rad]:
     _,vel,_=exact(case,t,d)
     assert inside(s['velocity'],vel),(v,s['velocity'],str(vel))
     counts['velocity_checks']+=1
   for q in s['evaluations']:
    t=m(q['time'])
    for d in [-rad,rad]:
     h,_,_=exact(case,t,d)
     assert inside(q['value'],h),(v,q,str(h))
     counts['guard_point_checks']+=1
  pp=rr['pending']
  if pp:
   records=pp['records'];pending_records+=len(records);refinements+=pp['refinements']
   assert len(records)<=pp['policy']['max_slabs']
   prev=None
   for s in records:
    assert m(s['safe_before'])<=lo
    if prev is not None and prev['safe_before']==s['safe_before']:
     assert prev['times'][1]==s['times'][0] and prev['outgoing_radius']==s['incoming_radius'],(v,prev,s)
    else:assert s['left'][1]<0.
    for t,gb in [(s['times'][0],s['left']),(s['times'][1],s['right'])]+([s['probe']]if s['probe'] else []):
     for d in [-rad,rad]:
      h,_,_=exact(case,m(t),d)
      assert inside(gb,h),(v,s,str(h))
      counts['pending_guard_checks']+=1
    prev=s
   if pp['candidate'] is not None:
    assert inside(pp['candidate'],lo) and inside(pp['candidate'],hi),(v,pp['candidate'],str(lo),str(hi))
    counts['candidate_checks']+=1
  if rr['root'] is not None:
   b=rr['root'];assert inside(b,lo) and inside(b,hi)
   assert width(b)<=m(1e-6)
   assert rr['reason'] is None and m(rr['safe'])<=m(b[0]) and m(b[1])<=m(rr['numerical_end'])
   assert all(s['velocity'][0]>0. for s in rr['guard_slabs'])
   counts['root_receipt_checks']+=1
 v.update({'attempts_used':r['attempts_used'],'rhs_proxy':proxy,'guard_evals':guards,'hint_calls':hints,'refinements':refinements,'pending_records':pending_records,'retry':bool(r.get('retry')),'first_resolution':(r['first']['pending']or {}).get('resolution'),'final_resolution':(rrs[-1]['pending']or {}).get('resolution')})
 if r['root'] is None:
  assert case=='physical-floor' or rrs[-1]['reason'] is not None or r.get('retry_error'),(v,r)
  v['refusal']=r.get('retry_error') or rrs[-1]['reason'] or 'unresolved';failures[v['refusal']]+=1
 else:
  assert case!='physical-floor'
  b=r['root'];assert inside(b,lo) and inside(b,hi) and width(b)<=m(1e-6)
  v['qualified']=True;v['root_width']=float(width(b));max_width=max(max_width,width(b))
  counts['qualified']+=1
  if r.get('retry'):counts['checkpoint_recovered']+=1
  if pending_records:counts['qualified_pending']+=1
 out.append(v)
summary={'cells':len(rows),**dict(counts),'structural_or_api_errors':sum('error' in r for r in rows),'returned_unqualified':sum(r.get('root') is None and 'error' not in r for r in rows),'max_qualified_width':float(max_width),'refusals':dict(failures),'training_sha256':hashlib.sha256((P/'training.log').read_bytes()).hexdigest(),'rows':out}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,sort_keys=True,allow_nan=False)+'\n')
print({k:v for k,v in summary.items()if k!='rows'})
