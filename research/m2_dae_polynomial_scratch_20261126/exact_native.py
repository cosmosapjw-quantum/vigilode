from fractions import Fraction as Q
import bisect,collections
from structural import native,br,mag,sub,add,mul,scale,deriv,ev,f_rober
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
