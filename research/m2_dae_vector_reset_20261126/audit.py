from pathlib import Path
import hashlib,json,collections,mpmath as mp
P=Path(__file__).parent;mp.mp.dps=85
def m(x):return mp.mpf(float(x))
def inside(b,x):return m(b[0])<=x<=m(b[1])
def width(b):return m(b[1])-m(b[0])
def vec_inside(bs,vs):return len(bs)==len(vs) and all(inside(b,v)for b,v in zip(bs,vs))
def reset(case,q,tau,delta):
 if case in ['nonlinear','negative-branch']:return [mp.mpf('.75')+tau,1+q*q]
 if case=='cancel':return [1+q-tau+(tau-q)**2,1-q+tau+(tau-q)**2]
 if case=='exact-pencil':return [1+(q+delta)**2+(2*q+delta)**2/4,1-q]
 if case=='rank-loss':return [mp.mpf('.5')+tau,mp.mpf(1)]
 if case=='excursion':return [mp.mpf(1),mp.mpf(1)]
 raise ValueError(case)
def exact(case,q,tau,delta,t):
 a,b=reset(case,q,tau,delta);d=t-tau
 if case in ['nonlinear','cancel']:
  x=a/(1+a*d);y=b+mp.log1p(a*d);z=x
  f=[-x*x,x];j=[[-2*x,0],[1,0]];ft=[0,0]
 elif case=='negative-branch':
  x=a*mp.exp(-d);z=-mp.sqrt(x);y=b+2*mp.sqrt(a)*mp.expm1(-d/2)
  f=[-x,z];j=[[-1,0],[-1/(2*mp.sqrt(x)),0]];ft=[0,0]
 elif case=='exact-pencil':
  x=a*mp.exp(-d);y=b-a*mp.expm1(-d);z=x/(1+t/8)
  f=[-x,x];j=[[-1,0],[1,0]];ft=[0,0]
 elif case=='rank-loss':
  x=a*mp.exp(-d);y=b;u=x-(1-mp.mpf(2)**-11);z=None if u<0 else mp.sqrt(u)
  f=[-x,0];j=[[-1,0],[0,0]];ft=[0,0]
 elif case=='excursion':
  x=a*mp.exp(-d);y=b+32*((tau-q)**2-(t-q)**2);z=y
  f=[-x,-64*(t-q)];j=[[-1,0],[0,0]];ft=[0,-64]
 return [x,y],z,f,j,ft
def work(p):return p['work']['rhs_count']+p['initial']['work']['rhs_count']
fields=['sync_route','subdivisions','branch_route','post_route']
cases=['nonlinear','cancel','negative-branch','exact-pencil','rank-loss','excursion']
rows=[json.loads(l.split('CELL ',1)[1])for l in (P/'training.log').read_text().splitlines()if 'CELL {' in l]
cats=json.loads((P/'CANDIDATES.json').read_text())['routes']
assert len(rows)==len({(r['case'],*(r[f]for f in fields))for r in rows})==576
assert {(r['case'],*(r[f]for f in fields))for r in rows}=={(k,*(v[f]for f in fields))for k in cases for v in cats}
out=[];counts=collections.Counter();reasons=collections.Counter();max_ratio=mp.mpf(0);max_error=mp.mpf(0)
def check_branch(bp,x,z,f,j,ft,v):
 assert z is not None
 assert vec_inside(bp['differential_box'],x)and inside(bp['root_box'][0],z),(v,bp,x,z)
 assert vec_inside(bp['field'],f)
 for row,ref in zip(bp['reduced_jacobian'],j):
  assert vec_inside(row,ref),(v,row,ref)
 assert vec_inside(bp['reduced_clock'],ft)
 counts['branch_state_checks']+=1
 counts['field_component_checks']+=len(f)
 counts['jacobian_clock_checks']+=sum(map(len,j))+len(ft)
for r in rows:
 case=r['case'];q=m(r['q']);rad=m(r['rad']);lo=q if r['clock']else q-rad;hi=q if r['clock']else q+rad
 v={k:r[k]for k in ['case',*fields]};v['qualified']=False
 root=r['root'];assert inside(root,lo)and inside(root,hi)and width(root)<=m(1e-3)and m(r['safe'])<=lo
 counts['root_checks']+=1
 s=m(root[1]);cells=r['cells'];assert cells and cells[0]['times'][0]==root[0]
 assert all(a['times'][1]==b['times'][0]for a,b in zip(cells,cells[1:]))
 proxy=r['pre_rhs']+r['pre_work']['rhs_count'];picard=0;green=0
 for cell in cells:
  a,b=map(m,cell['times']);l=max(a,lo);h=min(b,hi)
  proxy+=sum(c['prepared_work']['rhs_count']for c in cell['reset'])
  tube=cell['tube'];sync=cell['sync']
  if tube:
   counts['tube_cells']+=1;picard+=len(tube['attempts']);proxy+=sum(work(p)for p in tube['attempts'])
   bp=tube['attempts'][-1]
   assert bp['times']==[cell['times'][0],root[1]]
   assert bp['defect']<1
   assert all(b[0]<a[0]and a[1]<b[1]for a,b in zip(tube['picard_image'],tube['tube']))
  if sync:green+=sync['green_calls']
  if l>h:continue
  for i in range(9):
   tau=l+(h-l)*i/8
   ds=[-rad+2*rad*i/8]if r['clock']else[q-tau]
   for delta in ds:
    R=reset(case,q,tau,delta)
    if len(cell['reset'])==2:
     assert vec_inside([x['enclosure']for x in cell['reset']],R),(v,cell,R)
     counts['local_reset_checks']+=2
    if tube:
     for j in range(5):
      t=tau+(s-tau)*j/4;x,z,f,jac,ft=exact(case,q,tau,delta,t)
      assert vec_inside(tube['tube'],x),(v,cell,x)
      counts['tube_state_checks']+=2
      check_branch(bp,x,z,f,jac,ft,v)
    if sync:
     x,*_=exact(case,q,tau,delta,s)
     assert vec_inside(sync['synchronized'],x),(v,cell,x)
     counts['local_sync_checks']+=2
 if case=='rank-loss':
  x,z,*_=exact(case,q,lo,rad,s);assert z is None
  assert not r['committed'] and r['restart_branch'] is None
  counts['rank_refusals']+=1
 if r['sync']:
  assert cells[-1]['times'][1]==root[1] and all(c['sync']for c in cells)
  counts['sync_receipts']+=1
  v['width']=float(max(map(width,r['sync'])));v['radius']=r['radius']
  for i in range(65):
   tau=lo+(hi-lo)*i/64;delta=-rad+2*rad*i/64 if r['clock']else q-tau
   x,z,f,j,ft=exact(case,q,tau,delta,s)
   assert vec_inside(r['sync'],x),(v,r['sync'],x)
   counts['global_sync_checks']+=2
   if r['restart_branch']:
    bp=r['restart_branch'];check_branch(bp,x,z,f,j,ft,v)
    for a,b in zip(x,bp['initial']['x']):
     assert abs(a-m(b))<=m(r['radius'])
     counts['incoming_component_checks']+=1
  if r['restart_branch']:
   proxy+=work(r['restart_branch']);counts['restart_proofs']+=1
 po=r.get('post');v['attempts']=r['pre_attempts']
 if po:
  counts['post_receipts']+=1
  assert po['first_incoming']==r['radius']
  proxy+=po['rhs']+po['work']['rhs_count']+po['initial']['work']['rhs_count']
  v['attempts']+=po['attempts'];v['bound']=po['bound'];v['warp_predictions']=po['work']['warp_predictions']
  for query in po['queries']:
   t=m(query['t'])
   for i in range(17):
    tau=lo+(hi-lo)*i/16;delta=-rad+2*rad*i/16 if r['clock']else q-tau
    x,z,*_=exact(case,q,tau,delta,t);assert z is not None
    observed=max(abs(a-m(b))for a,b in zip([*x,z],query['state']))
    max_error=max(max_error,observed);counts['post_samples']+=1
    if po['global']:
     bd=m(query['physical_error_upper']);assert observed<=bd,(v,query,observed)
     if bd>0:max_ratio=max(max_ratio,observed/bd)
 v.update({'rhs_proxy':proxy,'picard_attempts':picard,'green_calls':green,'cells':len(cells),'committed':r['committed']})
 if r['committed']:
  assert r['reason'] is None and po and po['global']and po['branch']and po['pointwise']
  counts['committed']+=1
  if max(map(width,r['sync']))<=m(r['cap']):
   v['qualified']=True;counts['qualified']+=1
  else:v['refusal']='sync-width-cap';reasons[v['refusal']]+=1
 else:
  assert r['reason'] is not None
  v['refusal']=r['reason'];reasons[v['refusal']]+=1
 out.append(v)
summary={'cells':len(rows),**dict(counts),'max_post_sample_error':float(max_error),'max_certified_error_ratio':float(max_ratio),'refusals':dict(reasons),'training_sha256':hashlib.sha256((P/'training.log').read_bytes()).hexdigest(),'rows':out}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,sort_keys=True,allow_nan=False)+'\n')
print({k:v for k,v in summary.items()if k!='rows'})
