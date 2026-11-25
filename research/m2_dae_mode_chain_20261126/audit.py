from pathlib import Path
import hashlib,json,collections,mpmath as mp
P=Path(__file__).parent;mp.mp.dps=85
def m(x):return mp.mpf(float(x))
def inside(b,x):return m(b[0])<=x<=m(b[1])
def width(b):return m(b[1])-m(b[0])
q=mp.mpf(1)/8;rad=mp.mpf(2)**-12;horizon=mp.mpf(5)/8
ds=[-rad+2*rad*i/64 for i in range(65)]
def tau(case,d):return q if case in ('pencil','tie')else q-d
def second(case):return q+mp.mpf(2)**-9 if case=='near-dwell'else mp.mpf(3)/8
def state(case,mode,t,d):
 a=tau(case,d);b=second(case)
 x=t+d if mode==0 else mp.mpf(1)/4-a+2*(t-a) if mode==1 else mp.mpf(1)/4-a+2*(b-a)+mp.mpf(1)/8-(t-b)
 if case=='pencil':z=x/(1+t/8)
 elif case=='rank-loss' and mode==1:z=None if x<mp.mpf(1)/4 else mp.sqrt(x-mp.mpf(1)/4)
 else:z=x+t
 return x,z
def guard(case,mode,k,t,d):
 x,z=state(case,mode,t,d)
 if mode==0:
  if case=='tie':return t-q
  if case=='pencil':return t-(mp.mpf(1)/2 if k==0 else q)
  return x-((q+mp.mpf(2)**-14 if case=='overlap' else mp.mpf(1)/2)if k==0 else q)
 if k==1:return t-mp.mpf(1)/2
 if case=='gap-cancel':return (t-mp.mpf(3)/8)*mp.mpf(2)**-12
 if case=='gap-excursion':return rad*rad/4-(t-q)**2
 return t-second(case)
fields=['solver','order','sync','gap_route']
cases=['separated','overlap','gap-cancel','gap-excursion','near-dwell','rank-loss','tie','pencil']
rows=[json.loads(l.split('CELL ',1)[1])for l in (P/'training.log').read_text().splitlines()if 'CELL {'in l]
cats=json.loads((P/'CANDIDATES.json').read_text())['routes']
assert len(rows)==len({(r['case'],*(r[f]for f in fields))for r in rows})==384
assert {(r['case'],*(r[f]for f in fields))for r in rows}=={(k,*(v[f]for f in fields))for k in cases for v in cats}
out=[];counts=collections.Counter();stops=collections.Counter();max_ratio=mp.mpf(0);max_err=mp.mpf(0)
positive={'separated','overlap','gap-cancel','pencil'}
for r in rows:
 case=r['case'];w=r['work'];v={k:r[k]for k in ['case',*fields]};stop=r['stop'];stops[stop]+=1
 assert w['native_attempts']<=w['charged_native_attempts']<=256
 assert w['scan_slabs']<=w['charged_scan_slabs']<=128
 assert w['transitions_committed']<=w['transitions_attempted']<=4
 assert w['reset_cells']<=w['charged_reset_cells']<=64 and w['gap_checks']<=256 and w['picard_proofs']<=8*w['reset_cells']
 assert w['transitions_committed']==sum(e['committed']for e in r['transitions'])
 assert w['reset_cells']==sum(len(e['reset']['cells'])for e in r['transitions']if e['reset'])
 assert w['gap_checks']>=sum(len(e['gap'])for e in r['transitions'])
 counts['budget_checks']+=1
 committed=0;last=None;max_bound=0.
 for i,e in enumerate(r['transitions']):
  assert e['from']==i and e['target']==i+1 and e['guard']==(1 if i==0 else 0) and case!='tie',(v,e)
  root=e['event_time'];s=m(root[1]);assert width(root)<=m(1e-3)
  for d in ds:
   t0=tau(case,d)if i==0 else second(case)
   assert inside(root,t0),(v,root,t0)
   counts['event_root_checks']+=1
  if i:
   dl=m(e['dwell_lower']);assert dl<=m(root[0])-m(last[1])
   for d in ds:
    assert dl<=second(case)-tau(case,d)
    counts['dwell_checks']+=1
   if e['committed']:assert dl>=m(.01)
  else:assert e['dwell_lower'] is None
  last=root
  prep=e['reset'];target=i+1
  if prep:
   assert prep['event_time']==root
   for ci,cell in enumerate(prep['cells']):
    assert cell['times'][0]>=root[0] and cell['times'][1]<=root[1]
    bp=cell['tube']['attempts'][-1]if cell['tube'] else None
    if bp:
     assert bp['times']==[cell['times'][0],root[1]] and bp['defect']<1.
     assert all(b[0]<a[0]and a[1]<b[1]for a,b in zip(cell['tube']['picard_image'],cell['tube']['tube']))
    for d in ds:
     t0=tau(case,d)if i==0 else second(case)
     if not inside(cell['times'],t0):continue
     x,z=state(case,target,t0,d)
     if cell['reset']:
      assert inside(cell['reset'][0]['enclosure'],x),(v,cell['reset'],x)
      counts['reset_checks']+=1
     if bp:
      for j in range(5):
       t=t0+(s-t0)*j/4;x,z=state(case,target,t,d)
       assert z is not None and inside(bp['differential_box'][0],x)and inside(bp['root_box'][0],z),(v,bp,x,z)
       counts['tube_checks']+=2
     if cell['sync']:
      x,z=state(case,target,s,d);assert inside(cell['sync']['synchronized'][0],x)
      counts['local_sync_checks']+=1
   if prep['synchronized_x']:
    for d in ds:
     x,z=state(case,target,s,d);assert inside(prep['synchronized_x'][0],x)
     counts['sync_checks']+=1
     if prep['restart_branch']:
      bp=prep['restart_branch'];assert z is not None and inside(bp['root_box'][0],z)
      assert abs(x-m(bp['initial']['x'][0]))<=m(prep['incoming_radius'])
      counts['radius_checks']+=1
   for gc in e['gap']:
    assert gc['enclosure']==gc['evidence']['enclosure']
    if e['committed']:assert gc['enclosure'][1]<0.
    for d in ds:
     t0=tau(case,d)if i==0 else second(case)
     if gc['cell'] is not None and not inside(prep['cells'][gc['cell']]['times'],t0):continue
     a=max(t0,m(gc['times'][0]));b=m(gc['times'][1])
     if a>b:continue
     for j in range(5):
      t=a+(b-a)*j/4;val=guard(case,target,gc['guard'],t,d)
      assert inside(gc['enclosure'],val),(v,gc,val)
      counts['gap_value_checks']+=1
   if e['committed']:
    assert prep['restart_branch']and prep['reason']is None and all(c['tube']and c['sync']for c in prep['cells'])
    assert len(e['gap'])==(len(prep['cells'])+1)*(2 if target==1 else 0)
    committed+=1
  if not e['committed']:assert i==len(r['transitions'])-1
 assert r['final_mode']==committed
 assert len(r['epochs'])<=committed+1
 for i,e in enumerate(r['epochs']):
  assert e['mode']==i
  if i:
   prev=r['transitions'][i-1];assert prev['committed'] and e['start']==prev['event_time'][1] and e['radius']==prev['reset']['incoming_radius']
  else:assert e['start']==0. and e['radius']==float(rad)
  if i==0:assert m(e['safe'])<=min(tau(case,d)for d in ds)
  elif i==1:assert m(e['safe'])<=second(case)
  else:assert m(e['safe'])<=horizon
  if e['safe']>e['start']:
   assert len(e['queries'])==9,(v,e['start'],e['safe'],len(e['queries']))
  for query in e['queries']:
   t=m(query['t']);assert m(e['start'])<=t<=m(e['safe'])
   bd=m(query['physical_error_upper']);max_bound=max(max_bound,float(bd))
   for d in ds:
    x,z=state(case,i,t,d);assert z is not None
    err=max(abs(x-m(query['state'][0])),abs(z-m(query['state'][1])))
    assert err<=bd,(v,query,mp.nstr(err,35),bd)
    counts['query_checks']+=1;max_err=max(max_err,err)
    if bd>0:max_ratio=max(max_ratio,err/bd)
 if case in ('gap-excursion','rank-loss','tie'):assert committed==0
 if case=='near-dwell':assert committed<=1
 if case=='gap-excursion':
  assert guard(case,1,0,q,rad)>0 and guard(case,1,0,q+rad,rad)<0
 if stop=='Horizon':
  assert case in positive and committed==2 and r['final_mode']==2 and len(r['epochs'])==3 and r['epochs'][-1]['safe']==.625
  counts['qualified']+=1
 else:assert r['reason'] is not None
 v.update({'stop':stop,'qualified':stop=='Horizon','committed':committed,'charged_attempts':w['charged_native_attempts'],'native_attempts':w['native_attempts'],'rhs_proxy':w['driver_rhs']+w['projection_rhs']+w['gap_rhs']+w['reset_rhs'],'scan_slabs':w['scan_slabs'],'reset_cells':w['reset_cells'],'gap_checks':w['gap_checks'],'max_bound':max_bound,'warp_predictions':w['warp_predictions'],'green_calls':w['green_calls'],'dictionary_uses':w['dictionary_uses']})
 out.append(v)
summary={'cells':len(rows),**dict(counts),'stops':dict(stops),'max_query_error':float(max_err),'max_error_ratio':float(max_ratio),'training_sha256':hashlib.sha256((P/'training.log').read_bytes()).hexdigest(),'rows':out}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,sort_keys=True,allow_nan=False)+'\n')
print({k:v for k,v in summary.items()if k!='rows'})
