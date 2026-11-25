import json,collections,math,hashlib,bisect
from pathlib import Path
from fractions import Fraction as Q
import numpy as np
from structural import native,br,mag,sub,add,mul,scale,deriv,ev,f_rober
from reference import solve,evaluate
P=Path(__file__).parent
def load(name,prefix):return [json.loads(l.split(prefix,1)[1])for l in (P/name).read_text().splitlines()if prefix+'{'in l]
rows=load('campaign.log','CELL ');chains=load('campaign.log','CHAIN ');ovs=load('campaign.log','OVERSHOOT ')
assert len(rows)==72 and len(chains)==30 and len(ovs)==6
assert len({(r['solver'],r['cert'],r['rtol'],r['radius'])for r in rows})==72
assert len({(r['solver'],r['cert'],r['order'])for r in chains})==30
counts=collections.Counter()
def exact_cells(s,p,rate,method):
 c=s['certificate'];r=c['conservation'];a,b=map(Q,s['times']);h=b-a
 assert r['rate']==rate and r['norm']=='reduced-l1'
 ns={'Max':1,'Integral4':4,'Integral16':16}[method]
 assert len(r['cells'])==ns and r['route']==method
 defect=[sub(deriv(x),scale(y,h))for x,y in zip(p,f_rober(*p,rate))]
 prev=Q(r['incoming_l1']);assert prev==Q(c['incoming'])
 for k,v in enumerate(r['cells']):
  l=Q(k,ns);w=Q(1,ns);assert list(map(Q,v['theta']))==[l,l+w]
  vm=max(Q(0),-min(br(p[1],l,w)));zm=max(Q(0),-min(br(sub([Q(1)],add(*p)),l,w)))
  mu=max(Q(0),2*(-Q(.04)+10000*vm),2*rate*vm+20000*zm)
  assert Q(v['v_minus'])>=vm and Q(v['z_minus'])>=zm and Q(v['mu_l1_upper'])>=mu,(v,vm,zm,mu)
  bs=[br(d,l,w)for d in defect]
  rho=w*sum(mag(bc)if ns==1 else sum(map(abs,bc),Q(0))/len(bc)for bc in bs)
  assert Q(v['residual_integral'])>=rho,(v,rho)
  assert Q(v['incoming_l1'])==prev
  arg=h*w*Q(v['mu_l1_upper']);assert arg<1
  gain=Q(v['growth_upper']);assert gain>=1/(1-arg)
  outgoing=Q(v['outgoing_l1']);assert outgoing>=gain*(prev+Q(v['residual_integral']))
  prev=outgoing;counts['exact_cell_checks']+=1
 assert prev==Q(c['outgoing'])==Q(r['outgoing_l1'])==Q(c['whole_x_error'])
 assert Q(c['whole_z_error'])>=prev+Q(c['algebraic_defect'])
 return prev
def audit_run(r,rate,method,initial):
 assert r['driver_success'] and r['pointwise']and r['branch']and r['global']
 assert r['bound']<=1e-3
 prev=Q(initial);last=None;ends=[];powers=[]
 for s in r['slabs']:
  a,b=map(Q,s['times']);assert b>a
  p=native(s)[:2];counts['exact_coeff_checks']+=5*len(s['native']['y0'])
  c=s['certificate'];assert c['global_qualified']and c['conservation'] and Q(c['incoming'])==prev
  if last is None:assert [ev(x,Q(0))for x in p]==list(map(Q,r['initial']['x']))
  else:assert a==last[0] and [ev(x,Q(0))for x in p]==last[1]
  prev=exact_cells(s,p,rate,method);last=(b,[ev(x,Q(1))for x in p]);ends.append(b);powers.append(p)
 for q in r['queries']+r['accepted']:
  t=Q(q['t']);i=bisect.bisect_left(ends,t);s=r['slabs'][i];a,b=map(Q,s['times']);theta=(t-a)/(b-a)
  assert 0<=theta<=1
  u,v=[ev(x,theta)for x in powers[i]];z=1-u-v
  corr=max(abs(a-Q(b))for a,b in zip([u,v,z],q['state']))
  assert Q(q['physical_error_upper'])>=Q(s['certificate']['whole_x_error'])+corr
  assert q['physical_error_upper']<=r['bound']
  counts['exact_query_checks']+=1
 return prev
rad=2.**-20
# Triangle within L1 ball about (1,0) intersect exact physical simplex.
# Dyadic barycentric grid: 15 points at order 4. Exact float states with
# u+v+z=1 (binary dyadic), no invalid perturbation.
seeds=[]
for i in range(5):
 for j in range(5-i):
  a=rad*i/4;b=rad*j/8;u=1-a-b;v=b;z=a
  assert Q(u)+Q(v)+Q(z)==1 and Q(1)-Q(u)+Q(v)<=Q(rad)
  seeds.append([u,v,z])
assert len(seeds)==15
# Build separate full and reduced numerical references; high accuracy but
# not interval-certified. Compare every actual query and accepted time.
ref={}
for mode in ['plain','hybrid']:
 for seed in seeds:
  for solver in ['Radau','BDF']:
   state=seed if solver=='Radau'else seed[:2]
   if mode=='plain':ref[mode,tuple(seed),solver]=[solve(3e7,(0,1),state,solver,3e-13,1e-16)]
   else:
    a=solve(3e7,(0,1/256),state,solver,3e-13,1e-16);y=a.y[:,-1].copy()
    if solver=='Radau':y[2]+=y[1]/2
    y[1]/=2
    ref[mode,tuple(seed),solver]=[a,solve(6e7,(1/256,1/32),y,solver,3e-13,1e-16)]
max_ratio=0.;max_spread=0.;max_err=0.;max_center=0.
def compare(qs,mode,seg,ss):
 global max_ratio,max_spread,max_err,max_center
 if not qs:return 0.,0.
 t=np.array([q['t']for q in qs]);y=np.array([q['state']for q in qs]);bd=np.array([q['physical_error_upper']for q in qs])
 assert np.all(np.isfinite(bd))and np.all(bd>=0)
 local=0.;center=0.
 for seed in ss:
  a=evaluate(ref[mode,tuple(seed),'Radau'][seg],t,'Radau').T
  b=evaluate(ref[mode,tuple(seed),'BDF'][seg],t,'BDF').T
  spread=np.max(np.abs(a-b),axis=1);err=np.max(np.abs(a-y),axis=1)
  assert np.all(err<=bd+spread+2e-15),(mode,seg,seed,float(max(err-bd-spread)))
  max_ratio=max(max_ratio,float(max(err/np.maximum(bd,1e-300))))
  max_spread=max(max_spread,float(max(spread)));max_err=max(max_err,float(max(err)))
  local=max(local,float(max(err)))
  if seed==[1.,0.,0.]:center=float(max(err));max_center=max(max_center,center)
  counts['reference_query_checks']+=len(qs)
 return local,center
out=[]
for r in rows:
 v={k:r[k]for k in ['solver','cert','rtol','radius']};v['qualified']=False
 if 'error'in r:
  assert r['solver']=='WarpFolded' and 'constant pencil'in r['error'];v['reason']=r['error'];counts['structural_refusals']+=1
 else:
  a=r['run'];audit_run(a,30000000,r['cert'],r['radius'])
  err,nom=compare(a['queries']+a['accepted'],'plain',0,[[1.,0.,0.]]if r['radius']==0 else seeds)
  v.update({'qualified':nom<=2*r['rtol'],'observed_error':err,'nominal_error':nom,'bound':a['bound'],'attempts':a['attempts'],'rhs_proxy':a['counters']['rhs_calls']+a['work']['rhs_count'],'proof_cells':sum(len(s['certificate']['conservation']['cells'])for s in a['slabs']),'prepared':a['work']['prepared_owners'],'dictionary':a['work']['dictionary_uses']})
  counts['native_global']+=1
 out.append(v)
hy=[]
for r in chains:
 assert r['event']==1/256 and r['winner']==1 and r['committed']and r['reason'] is None,(r['solver'],r['cert'],r.get('reason'))
 assert r['carry']>=0 and r['attempts']<=4096
 post=r['post'];assert post['initial']['x']==r['center'];audit_run(post,60000000,r['cert'],r['carry'])
 # At reset compare true independent one-sided physical states in L1,
 # not merely their infinity distance from the center.
 carry_observed=0.
 for seed in seeds:
  a=evaluate(ref['hybrid',tuple(seed),'Radau'][1],1/256,'Radau')
  b=evaluate(ref['hybrid',tuple(seed),'BDF'][1],1/256,'BDF')
  er=float(sum(abs(a[:2]-r['center'])));sp=float(sum(abs(a[:2]-b[:2])))
  assert er<=r['carry']+sp+2e-15
  carry_observed=max(carry_observed,er);counts['reset_carry_checks']+=1
 a,_=compare(r['pre_queries'],'hybrid',0,seeds);b,_=compare(post['queries']+post['accepted'],'hybrid',1,seeds)
 hy.append({k:r[k]for k in ['solver','cert','order','rtol','radius','event','carry','attempts']}|{'post_bound':post['bound'],'observed_error':max(a,b),'carry_observed':carry_observed,'qualified':True})
 counts['native_chain']+=1
for r in ovs:
 c=r['certificate'];v=Q(r['v']);prev=v
 # Deliberately nonphysical constant comparator, exact residual.
 s={'times':c['times'],'certificate':c};p=[[Q(1)],[ -v]]
 exact_cells(s,p,30000000,r['method'])
 counts['overshoot_checks']+=1
summary={'counts':dict(counts),'max_reference_error_ratio':max_ratio,'max_reference_discrepancy':max_spread,'max_reference_error':max_err,'max_nominal_error':max_center,'initial_family_samples':len(seeds),'rows':out,'chains':hy,'raw_sha256':hashlib.sha256((P/'campaign.log').read_bytes()).hexdigest()}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,allow_nan=False)+'\n')
print({k:v for k,v in summary.items()if k not in ('rows','chains')})
print('qualified',sum(r['qualified']for r in out),'/72','hybrid',len(hy))
