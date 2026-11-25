from pathlib import Path
import hashlib,json,collections,mpmath as mp
P=Path(__file__).parent;mp.mp.dps=85
def m(x):return mp.mpf(float(x))
def inside(b,x):return m(b[0])<=x<=m(b[1])
def width(b):return m(b[1])-m(b[0])
def reset(case,q,t):
 if case in ['cancel-positive','boundary']:return 1+q-t+(t-q)**2
 if case=='rank-loss':return mp.mpf('.5')+t
 return mp.mpf('.5')+q*q+(q+t)**2/4
def algebraic(case,t,x):
 if case=='time-pencil':return [x/(1+t/8)]
 if case=='rank-loss':
  v=x-(1-mp.mpf(2)**-11)
  return None if v<0 else [mp.sqrt(v)]
 z=mp.sqrt(x)
 if case=='state-negative':return [-z]
 if case=='coupled':return [z,(1-z)/2]
 return [z]
fields=['pre_route','envelope','derivative','branch_route','post_route']
cases=['cancel-positive','state-negative','time-pencil','rank-loss','boundary','coupled']
rows=[json.loads(l.split('CELL ',1)[1])for l in (P/'training.log').read_text().splitlines()if 'CELL {' in l]
cats=json.loads((P/'CANDIDATES.json').read_text())['routes']
assert len(rows)==len({(r['case'],*(r[f]for f in fields))for r in rows})==864
assert {(r['case'],*(r[f]for f in fields))for r in rows}=={(k,*(v[f]for f in fields))for k in cases for v in cats}
out=[];counts=collections.Counter();reasons=collections.Counter();pre={};max_ratio=mp.mpf(0);max_error=mp.mpf(0)
for r in rows:
 case=r['case'];q=m(r['threshold']);rad=m(r['rad']);lo=q-rad;hi=q+rad
 v={k:r[k]for k in ['case',*fields]};v['qualified']=False
 if 'pre_error' in r:
  v['refusal']=r['pre_error'];reasons[v['refusal']]+=1;out.append(v);continue
 pr=r['pre'];key=(case,r['pre_route'])
 if key in pre:assert pr==pre[key]
 pre[key]=pr
 if not pr['root']:
  v['refusal']=pr['reason'];reasons[v['refusal']]+=1;out.append(v);continue
 b=pr['root'];assert inside(b,lo)and inside(b,hi)and width(b)<=m(1e-3) and m(pr['safe'])<=lo
 s=m(b[1])
 counts['root_checks']+=1
 if 'reset_error' in r:v['refusal']=r['reset_error'];reasons[v['refusal']]+=1;out.append(v);continue
 rr=r['reset'];counts['reset_receipts']+=1
 assert rr['event_time']==b
 assert rr['slabs'][0]['times'][0]==b[0] and rr['slabs'][-1]['times'][1]==b[1]
 assert all(a['times'][1]==z['times'][0]for a,z in zip(rr['slabs'],rr['slabs'][1:]))
 v.update({'sync_width':float(width(rr['sync_x'])),'incoming_radius':rr['incoming_radius'],'reset_width':float(width(rr['reset_x'])),'green_calls':sum(x['green_calls']for x in rr['slabs']),'slabs':len(rr['slabs']),'branch_proved':rr['branch'] is not None,'committed':rr['committed']})
 center=m(rr['sync_x'][0])/2+m(rr['sync_x'][1])/2
 # rounded center checked against actual post initial below, not high-precision midpoint
 for se in rr['slabs']:
  l=max(lo,m(se['times'][0]));h=min(hi,m(se['times'][1]))
  if l>h:continue
  for i in range(17):
   tau=l+(h-l)*i/16;r0=reset(case,q,tau);x=r0*mp.exp(-(s-tau))
   assert inside(se['reset_x'],r0),(v,se,str(r0))
   assert inside(se['synchronized_x'],x),(v,se,str(x))
   counts['slab_reset_sync_checks']+=2
 for i in range(65):
  tau=lo+(hi-lo)*i/64;r0=reset(case,q,tau);sync=r0*mp.exp(-(s-tau))
  assert inside(rr['reset_x'],r0) and inside(rr['sync_x'],sync),(v,rr,str(sync))
  counts['global_reset_sync_checks']+=2
  for j in range(5):
   t=tau+(s-tau)*j/4;x=r0*mp.exp(-(t-tau))
   assert inside(rr['transition_x'],x),(v,rr,str(x))
   counts['transition_x_checks']+=1
   if rr['branch']:
    bp=rr['branch'];z=algebraic(case,t,x)
    assert z is not None
    assert bp['times']==b and bp['differential_box']==rr['transition_x'] and bp['defect']<1.
    for k,a in enumerate(z):
     assert inside(bp['root_box'][k],a) and inside(bp['branch'][k],a),(v,bp,str(a))
     counts['branch_component_checks']+=1
 if case=='rank-loss':
  x=reset(case,q,lo)*mp.exp(-(s-lo))
  assert x<1-mp.mpf(2)**-11
  assert rr['branch'] is None and not rr['committed']
  counts['rank_loss_refusals']+=1
 if rr['branch']:counts['branch_proved']+=1
 proxy=pr['driver_rhs']+pr['projection']['rhs_count']+pr['work']['projection']['rhs_count']+sum(se['prepared_work']['rhs_count']for se in pr['guard_slabs'])+sum(se['prepared_work']['rhs_count']for se in rr['slabs'])
 if rr['branch']:proxy+=rr['branch']['work']['rhs_count']+rr['branch']['initial']['work']['rhs_count']
 v['attempts']=pr['attempts']
 po=r.get('post')
 if po:
  counts['post_receipts']+=1
  proxy+=po['driver_rhs']+po['work']['rhs_count']+po['initial']['work']['rhs_count']
  v['attempts']+=po['attempts'];v['bound']=po['bound']
  assert po['first_incoming']==rr['incoming_radius']
  c0=m(po['initial']['x'][0]);err=m(rr['incoming_radius'])
  for i in range(33):
   tau=lo+(hi-lo)*i/32;sync=reset(case,q,tau)*mp.exp(-(s-tau))
   assert abs(sync-c0)<=err
   counts['incoming_checks']+=1
  for query in po['queries']:
   t=m(query['t'])
   assert t>=s
   for i in range(33):
    tau=lo+(hi-lo)*i/32;x=reset(case,q,tau)*mp.exp(-(t-tau));z=algebraic(case,t,x)
    assert z is not None
    state=[x,*z]
    observed=max(abs(a-m(b))for a,b in zip(state,query['state']))
    max_error=max(max_error,observed);counts['post_samples']+=1
    if po['global']:
     bound=m(query['physical_error_upper']);assert observed<=bound,(v,query,str(observed))
     if bound>0:max_ratio=max(max_ratio,observed/bound)
 v['rhs_proxy']=proxy
 if rr['committed']:
  assert rr['reason'] is None and po and po['global'] and po['branch'] and po['pointwise']
  counts['committed']+=1
  if width(rr['sync_x'])<=m(r['cap']):
   v['qualified']=True;counts['qualified']+=1
  else:v['refusal']='sync-width-cap';reasons[v['refusal']]+=1
 else:
  assert rr['reason'] is not None
  v['refusal']=rr['reason'];reasons[v['refusal']]+=1
 out.append(v)
summary={'cells':len(rows),'distinct_pre_receipts':len(pre),**dict(counts),'max_post_sample_error':float(max_error),'max_certified_error_ratio':float(max_ratio),'refusals':dict(reasons),'training_sha256':hashlib.sha256((P/'training.log').read_bytes()).hexdigest(),'rows':out}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,sort_keys=True,allow_nan=False)+'\n')
print({k:v for k,v in summary.items()if k!='rows'})
