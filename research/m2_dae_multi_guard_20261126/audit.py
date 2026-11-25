from pathlib import Path
import hashlib,json,collections,mpmath as mp
P=Path(__file__).parent;mp.mp.dps=85
def m(x):return mp.mpf(float(x))
def inside(b,x):return m(b[0])<=x<=m(b[1])
def width(b):return m(b[1])-m(b[0])
q=mp.mpf(1)/8;d=mp.mpf(2)**-14
horizon=mp.mpf(3)/2
# independent, explicit scalar guard family; not using Rust source coefficients
def kinds(case):
 return {
 'separated':[('x',mp.mpf(1)/2,1),('x',q,1),('x',mp.mpf(3),1)],
 'overlap-fixed':[('x',q+d,1),('x',q,1)],
 'overlap-swaps':[('x',q,1),('t',q,1)],
 'exact-tie':[('t',q,1),('t',q,2)],
 'absent':[('x',mp.mpf(3),1),('t',mp.mpf(2),1)],
 'constraint-cancel':[('z',2*q+d,1),('xt',2*q,1)],
 'slab-budget':[('x',q+d,1),('x',q,1)],
 'pencil':[('pencil',q+d,1),('pencil',q,1)],
 }[case]
def root(kind,delta):
 k,c,w=kind
 if k=='x':return c-delta
 if k=='t':return c
 if k in ('z','xt'):return (c-delta)/2
 if k=='pencil':
  b=1-c/8
  return 2*(c-delta)/(b+mp.sqrt(b*b+2*(c-delta)))
 raise ValueError(k)
def exact(kind,t,delta):
 k,c,w=kind
 if k=='x':return w*(delta+t-c),mp.mpf(w),mp.mpf(w),mp.mpf(0)
 if k=='t':return w*(t-c),mp.mpf(w),mp.mpf(0),mp.mpf(w)
 if k in ('z','xt'):return w*(delta+2*t-c),mp.mpf(2*w),mp.mpf(w),mp.mpf(w)
 if k=='pencil':
  x=delta+t+t*t/2;den=1+t/8;z=x/den;gx=1/den;gt=-x/(8*den*den)
  return w*(z-c),w*(gx*(1+t)+gt),w*gx,w*gt
 raise ValueError(k)
fields=['solver','method','root_route','derivative','value']
cases=['separated','overlap-fixed','overlap-swaps','exact-tie','absent','constraint-cancel','slab-budget','pencil']
rows=[json.loads(l.split('CELL ',1)[1])for l in (P/'training.log').read_text().splitlines()if 'CELL {'in l]
cats=json.loads((P/'CANDIDATES.json').read_text())['routes']
assert len(rows)==len({(r['case'],*(r[f]for f in fields))for r in rows})==960
assert {(r['case'],*(r[f]for f in fields))for r in rows}=={(k,*(v[f]for f in fields))for k in cases for v in cats}
out=[];counts=collections.Counter();reasons=collections.Counter()
for r in rows:
 case=r['case'];ks=kinds(case);rad=m(r['rad']);ds=[-rad+2*rad*i/64 for i in range(65)]
 roots=[[root(k,delta)for k in ks]for delta in ds]
 v={k:r[k]for k in ['case',*fields]};v['qualified']=False
 if 'error'in r:
  counts['api_errors']+=1;v['refusal']=r['error'];reasons[v['refusal']]+=1;out.append(v);continue
 gs=r['guards'];w=r['work'];n=len(ks)
 assert len(gs)<=n and w['native_runs']<=n and w['attempts']<=100000
 assert m(r['safe'])<=min(horizon,min(min(a)for a in roots)),(v,r['safe'])
 counts['safe_prefix_checks']+=65
 if r['method']!='Independent':
  assert w['native_runs']==1 and w['physical_certificates']<=r['cap']
  assert w['shared_projection']['rhs_count']>=0
 else:
  assert w['attempts']<=r['cap']*n
  assert w['physical_certificates']==sum(g['work']['certified_slabs']for g in gs)
 assert w['guard_evals']==sum(g['work']['guard_evals']for g in gs)
 assert w['comparison_attempts']==len(r['comparisons'])
 assert w['difference_cells']==sum(len(c['cells'])for c in r['comparisons'])
 for i,g in enumerate(gs):
  assert g['index']==i
  if g['root']:
   assert g['initial_negative'] and width(g['root'])<=m(1e-3)
   for a in roots:
    assert inside(g['root'],a[i]),(v,g['root'],mp.nstr(a[i],30))
    counts['individual_root_checks']+=1
  if g['initial_negative']:
   assert m(g['safe'])<=min(horizon,min(a[i]for a in roots))
  pp=g['pending']
  if pp and pp['candidate']:
   for a in roots:
    assert inside(pp['candidate'],a[i]),(v,pp['candidate'],mp.nstr(a[i],30))
    counts['pending_candidate_checks']+=1
  for s in g['slabs']:
   a,b=map(m,s['times'])
   for j in range(9):
    t=a+(b-a)*j/8
    for delta in [-rad,rad]:
     _,vel,gx,gt=exact(ks[i],t,delta)
     assert inside(s['velocity'],vel),(v,s['velocity'],vel)
     assert inside(s['reduced_gradient'][0],gx),(v,s['reduced_gradient'],gx)
     assert inside(s['reduced_clock'],gt),(v,s['reduced_clock'],gt)
     counts['velocity_checks']+=1;counts['gradient_clock_checks']+=2
   for p in s['evaluations']:
    for delta in [-rad,rad]:
     val,*_=exact(ks[i],m(p['time']),delta)
     assert inside(p['value'],val),(v,p,mp.nstr(val,40))
     counts['guard_point_checks']+=1
 for c in r['comparisons']:
  j,k=c['earlier'],c['later'];assert j!=k and gs[k]['root']==c['compared_root']
  for cell in c['cells']:
   a,b=map(m,cell['times'])
   for n0 in range(9):
    t=a+(b-a)*n0/8
    for delta in [-rad,rad]:
     vj,*_=exact(ks[j],t,delta);vk,*_=exact(ks[k],t,delta)
     diff=vj-vk
     # difference can be exactly constant; evaluate analytic formula
     # without subtracting independently rounded values at 85 digits.
     if case in ('overlap-fixed','pencil','slab-budget','constraint-cancel'):
      diff=d if j==1 else -d
     if case=='exact-tie':
      diff=(ks[j][2]-ks[k][2])*(t-q)
     if case=='overlap-swaps':diff=delta if j==0 else -delta
     assert inside(cell['enclosure'],diff),(v,cell,mp.nstr(diff,40))
     counts['difference_checks']+=1
  if c['positive']:
   assert c['cells'] and c['cells'][0]['times'][0]==c['compared_root'][0] and c['cells'][-1]['times'][1]==c['compared_root'][1]
   assert all(x['enclosure'][0]>0 for x in c['cells'])
   assert all(a['times'][1]==b['times'][0]for a,b in zip(c['cells'],c['cells'][1:]))
   for a in roots:
    assert a[j]<a[k]
    counts['positive_order_checks']+=1
 expected=1 if case in ('separated','overlap-fixed','constraint-cancel','pencil')else None
 v.update({'attempts':w['attempts'],'native_runs':w['native_runs'],'rhs_proxy':w['driver_rhs']+w['shared_projection']['rhs_count']+w['replay_projection_rhs']+w['independent_scan_projection_rhs']+w['guard_prepared_rhs']+w['difference_prepared_rhs'],'driver_rhs':w['driver_rhs'],'certificates':w['physical_certificates'],'guard_evals':w['guard_evals'],'comparisons':w['comparison_attempts'],'difference_cells':w['difference_cells'],'dictionary_uses':w['dictionary_uses'],'warp_predictions':w['shared_projection']['warp_predictions']+r.get('projection',{}).get('warp_predictions',0),'winner':r['winner'],'safe':r['safe']})
 if r['winner'] is not None:
  j=r['winner'];assert expected==j and r['reason'] is None and len(gs)==len(ks)
  b=r['root'];assert gs[j]['root']==b and m(r['safe'])<=m(b[0])
  assert m(b[1])<=m(r['numerical_end'])<=horizon
  for a in roots:
   assert inside(b,a[j]) and all(a[j]<a[k]for k in range(len(a))if k!=j)
   counts['winner_order_checks']+=1
  v['qualified']=True;v['width']=float(width(b));counts['winner_qualified']+=1
  if any(c['positive']and c['earlier']==j for c in r['comparisons']):counts['correlation_rescues']+=1
 elif case=='absent' and r['reason'] is None:
  assert len(gs)==len(ks) and r['safe']==1.5 and all(min(a)>horizon for a in roots)
  v['qualified']=True;v['width']=0.;counts['absence_qualified']+=1
 else:
  assert r['root'] is None
  if case in ('overlap-swaps','exact-tie','slab-budget') and len(gs)==len(ks):
   assert r['reason'] is not None
   counts['negative_control_refusals']+=1
  v['refusal']=r['reason'] or 'unexpected no event';reasons[v['refusal']]+=1
 out.append(v)
summary={'cells':len(rows),**dict(counts),'qualified':sum(r['qualified']for r in out),'refusals':dict(reasons),'training_sha256':hashlib.sha256((P/'training.log').read_bytes()).hexdigest(),'rows':out}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,sort_keys=True,allow_nan=False)+'\n')
print({k:v for k,v in summary.items()if k!='rows'})
