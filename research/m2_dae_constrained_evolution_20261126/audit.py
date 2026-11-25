"""Independent exact represented-source and physical-reference checks."""
from pathlib import Path
import json,sys,math,functools
from fractions import Fraction as F
import numpy as np
import mpmath as mp
import sympy as sy
from scipy.integrate import solve_ivp
mp.mp.dps=70
P=Path(__file__).resolve().parent
def m(x):return mp.mpf(float(x))
def f(x):return F.from_float(float(x))
def read(p,tag):return [json.loads(l.split(tag,1)[1]) for l in p.read_text().splitlines() if tag+'{' in l]
class I:
 def __init__(self,a,b=None):
  self.a=a if isinstance(a,F) else f(a);self.b=self.a if b is None else b if isinstance(b,F) else f(b)
  assert self.a<=self.b
 def __add__(self,v):return I(self.a+v.a,self.b+v.b)
 def __neg__(self):return I(-self.b,-self.a)
 def __sub__(self,v):return self+-v
 def __mul__(self,v):
  z=[a*b for a in (self.a,self.b) for b in (v.a,v.b)]
  return I(min(z),max(z))
 def scale(self,v):return self*I(v)
 def mag(self):return max(abs(self.a),abs(self.b))
 def subset(self,v):return v.a<=self.a<=self.b<=v.b
def isum(v):return sum(v,I(0))
def prod(v):
 s=I(1)
 for a in v:s=s*a
 return s
def iev(polys,t,y):
 out=[];jac=[]
 for eq in polys:
  v=I(0);row=[I(0) for _ in range(len(y)+1)]
  for term in eq['terms']:
   fs=term['factors'];vs=[I(a['constant'])+t.scale(a['time'])+isum(y[j].scale(w) for j,w in a['state']) for a in fs];c=I(term['coefficient'])
   v=v+c*prod(vs)
   for j,a in enumerate(fs):
    z=c*prod(vs[:j]+vs[j+1:]);row[0]=row[0]+z.scale(a['time'])
    for k,w in a['state']:row[k+1]=row[k+1]+z.scale(w)
  out.append(v);jac.append(row)
 return out,jac
def padd(a,b):
 return [(a[k] if k<len(a) else I(0))+(b[k] if k<len(b) else I(0)) for k in range(max(len(a),len(b)))]
def pscale(a,c):return [v*c for v in a]
def pmul(a,b):
 v=[I(0) for _ in range(len(a)+len(b)-1)]
 for i,x in enumerate(a):
  for j,y in enumerate(b):v[i+j]=v[i+j]+x*y
 return v
def compose(e,t,y):
 v=[I(0)]
 for term in e['terms']:
  s=[I(term['coefficient'])]
  for a in term['factors']:
   q=padd([I(a['constant'])],pscale(t,I(a['time'])))
   for j,w in a['state']:q=padd(q,pscale(y[j],I(w)))
   s=pmul(s,q)
  v=padd(v,s)
 return v
def brange(a):
 n=len(a)-1;b=[isum(a[j].scale(F(math.comb(i,j),math.comb(n,j))) for j in range(i+1)) for i in range(n+1)]
 return I(min(x.a for x in b),max(x.b for x in b))
def pnorm(a):return max(brange(v).mag() for v in a)
def imat(a,b):return [isum(v.scale(c) for c,v in zip(r,b)) for r in a]
def inorm(a):return max(v.mag() for v in a)
def rnorm(a):return max(sum(v.mag() for v in row) for row in a)
def audit_slab(src,s):
 c=s['certificate'];nx=len(src['x']);nz=len(src['z'])
 if c is None:return
 a,b=map(f,s['times']);h=b-a
 y=[[I(*v) for v in p] for p in s['power']]
 native=s['native'];H=list(map(f,native['coefficients']));n=len(native['y0'])
 assert len(native['stages'])==8 and len(H)==24
 for i in range(n):
  d=[sum(H[k*8+j]*f(native['stages'][j][i])for j in range(8))for k in range(3)]
  a0,a1=f(native['y0'][i]),f(native['y1'][i])
  exact=[a0,a1-a0+d[0],d[1]-d[0],d[2]-d[1],-d[2]]
  assert all(I(v).subset(b) for v,b in zip(exact,y[i]))

 rng=[brange(v) for v in y];X=[I(*v) for v in c['x_range']];B=[I(*v) for v in src['branch']]
 assert all((r+I(-c['tube_radius'],c['tube_radius'])).subset(x) for r,x in zip(rng,X))
 assert all(r.subset(I(*v)) and I(*v).subset(box) for r,v,box in zip(rng[nx:],c['z_range'],B))
 C=[list(map(f,c['preconditioner'][i*nz:(i+1)*nz])) for i in range(nz)]
 tt=I(a,b);_,J=iev(src['src']['g'],tt,X+B);Jz=[v[1+nx:] for v in J]
 E=[[I(int(i==j))-isum(Jz[k][j].scale(C[i][k]) for k in range(nz)) for j in range(nz)]for i in range(nz)]
 q=rnorm(E);assert q<=f(c['branch_defect'])<1
 gap=1-f(c['branch_defect'])
 # Independently check the actual supplied preconditioner and self-map.
 zc=list(map(I,c['self_map_center']));L=[I(*v)for v in c['self_map_box']]
 assert all(z.subset(v) and v.subset(b) for z,v,b in zip(zc,L,B))
 gv,_=iev(src['src']['g'],tt,X+zc)
 cg=imat(C,gv)
 _,Jl=iev(src['src']['g'],tt,X+L);Jl=[v[1+nx:]for v in Jl]
 El=[[I(int(i==j))-isum(Jl[k][j].scale(C[i][k]) for k in range(nz)) for j in range(nz)]for i in range(nz)]
 db=[v-z for v,z in zip(L,zc)]
 root=[z-v+isum(e*d for e,d in zip(row,db)) for z,v,row in zip(zc,cg,El)]
 assert all(v.subset(I(*w)) and I(*w).subset(box) for v,w,box in zip(root,c['branch_root_box'],L))
 assert max(sum(abs(v) for v in r) for r in C)/gap*rnorm(Jz)<=f(c['condition_upper'])
 gp=[compose(g,[I(a),I(h)],y) for g in src['src']['g']]
 assert pnorm(gp)<=f(c['original_g_upper'])
 cp=[]
 for row in C:
  v=[I(0)]
  for w,g in zip(row,gp):v=padd(v,pscale(g,I(w)))
  cp.append(v)
 assert pnorm(cp)/gap<=f(c['algebraic_defect'])
 fp=[compose(v,[I(a),I(h)],y) for v in src['src']['f']]
 dx=[padd([v.scale(F(k)) for k,v in enumerate(y[i]) if k],pscale(fp[i],I(-h))) for i in range(nx)]
 assert pnorm(dx)<=f(c['differential_defect'])
 _,fj=iev(src['src']['f'],tt,X+B)
 fz=[v[1+nx:] for v in fj]
 assert f(c['differential_defect'])+h*rnorm(fz)*f(c['algebraic_defect'])<=f(c['normalized_reduced_defect'])
 # Actual interval Schur enclosure with same represented preconditioner.
 zx=[]
 for k in range(nx):
  w=imat(C,[-r[k+1] for r in J]);d=f(c['branch_defect'])*inorm(w)/gap;zx.append([v+I(-d,d) for v in w])
 assert max(sum(zx[k][i].mag() for k in range(nx))for i in range(nz))<=f(c['zx_norm_upper'])
 mu=max(sum((v.b if i==k else v.mag()) for k,v in enumerate([r[k+1]+isum(a*b for a,b in zip(r[1+nx:],zx[k])) for k in range(nx)])) for i,r in enumerate(fj))
 assert mu<=f(c['reduced_mu_upper'])
 if c['global_qualified']:
  assert c['whole_x_error']<c['tube_radius'] and c['whole_x_error']>=c['incoming'] and c['whole_x_error']>=c['outgoing']
  e=m(c['incoming']);R=m(c['normalized_reduced_defect']);u=m(c['reduced_mu_upper'])*mp.mpf(h.numerator)/h.denominator
  expected=mp.exp(u)*e+(mp.expm1(u)/u if u else 1)*R
  assert expected<=m(c['outgoing'])+mp.mpf('1e-65')
  assert f(c['algebraic_defect'])+f(c['zx_norm_upper'])*f(c['whole_x_error'])<=f(c['whole_z_error'])
def oracle(src):
 name=src['group'];x=list(map(m,src['x']))
 if name=='stiff':
  # two independent SciPy Radau tolerances, compare dense outputs before
  # using the finer reference. This is empirical, not interval arithmetic.
  def fun(t,v):
   a,b=v;z=1-a-b
   return [-.04*a+1e4*b*z,.04*a-1e4*b*z-3e7*b*b]
  ss=[solve_ivp(fun,(0,src['end']),src['x'],method='Radau',rtol=rt,atol=rt*.01,dense_output=True)for rt in [1e-12,2.5e-14]]
  assert all(s.success for s in ss)
  tt=np.linspace(0,src['end'],257);spread=float(np.max(abs(ss[0].sol(tt)-ss[1].sol(tt))));assert spread<1e-10
  def state(t):
   v=list(ss[1].sol(float(t)));return list(map(m,v+[1-sum(v)]))
  return state,spread
 if name=='coupled':
  G=mp.matrix([[2,mp.mpf('.125')],[mp.mpf('.125'),2]]);A=mp.diag([-2,-1])+mp.mpf('.25')*G**-1
  @functools.lru_cache(None)
  def state(t):
   v=mp.expm(A*t)*mp.matrix(x);return list(v)+list(G**-1*v)
  return state,0.
 def state(t):
  v=x[0]*mp.exp(-t)
  if name=='analytic':return [v,v*v+t]
  if name=='nonlinear':return [v,mp.sqrt(v)]
  assert name=='pencil'
  v*=1+t/8;return [v,v/(1+t/8)]
 return state,0.
def algebraic_at(src,t,x):
 if src['group']=='analytic':return [x[0]*x[0]+t]
 if src['group']=='nonlinear':return [mp.sqrt(x[0])]
 if src['group']=='pencil':return [x[0]/(1+t/8)]
 if src['group']=='stiff':return [1-sum(x)]
 return list(mp.lu_solve(mp.matrix([[2,mp.mpf('.125')],[mp.mpf('.125'),2]]),mp.matrix(x)))
def query_check(src,q,ref,policy):
 nx=len(src['x']);t=m(q['t']);raw=q['raw'];v=q['state'];r=q['projection']
 assert v[:nx]==raw[:nx] and r['x']==v[:nx] and r['z']==v[nx:]
 gg,_=iev(src['src']['g'],I(q['t']),[I(a) for a in v])
 assert inorm(gg)<=f(q['projected_constraint_upper'])<=f(policy['init']['residual_tol'])
 assert r['state_error_upper']<=policy['init']['state_tol']
 zv=algebraic_at(src,t,list(map(m,v[:nx])))
 assert all(m(a)<=b<=m(c) for (a,c),b in zip(r['root_box'],zv))
 assert max(abs(m(a)-b) for a,b in zip(v[nx:],zv))<=m(r['state_error_upper'])+mp.mpf('1e-65')

 if q['raw_constraint_upper'] is not None:
  gg,_=iev(src['src']['g'],I(q['t']),[I(a) for a in raw]);assert inorm(gg)<=f(q['raw_constraint_upper'])
 err=max(abs(m(a)-b) for a,b in zip(v,ref(t)))
 if q['physical_error_upper'] is not None and src['group']!='stiff':assert err<=m(q['physical_error_upper'])+mp.mpf('1e-65')
 return float(err)
def main(name):
 srcs=read(P/(name+'.log'),'SOURCE ');rows=read(P/(name+'.log'),'CELL ')
 assert len(srcs)==5 and len(rows)==(80 if name=='training' else 10 if name=='recovery' else sum(v['selected'] is not None for g in json.loads((P/'SELECTION.json').read_text())['groups'].values() for v in g.values()))
 assert len({(r['case'],r['candidate'],r['tight'])for r in rows})==len(rows)
 sources={s['case']:s for s in srcs};oracles={k:oracle(s) for k,s in sources.items()}
 result=[]
 for r in rows:
  src=sources[r['case']];ref,spread=oracles[r['case']]
  v={k:r[k] for k in ['case','group','candidate','tight','median_ns']}
  run=None if r['result'] is None else r['result']['run']
  v.update(qualified=False,global_qualified=False,reason=r.get('reason') or 'no qualified route',reference_spread=spread)
  if run is None:result.append(v);continue
  raw=run['driver'];assert len(raw['t'])==len(raw['y'])
  errs=[query_check(src,q,ref,r['policy'])for q in run['accepted']+run['queries']]
  ideal_checks=0
  for j,s in enumerate(run['slabs']):
   assert s['native']['y0']==raw['y'][j] and s['native']['y1']==raw['y'][j+1]
   assert s['times']==raw['t'][j:j+2]
   audit_slab(src,s)
   c=s['certificate']
   if c and c['global_qualified'] and src['group']!='stiff':
    n=len(src['x']);a,b=map(m,s['times'])
    native=s['native'];H=list(map(f,native['coefficients']));co=[]
    for k in range(len(native['y0'])):
     d=[sum(H[i*8+j]*f(native['stages'][j][k])for j in range(8))for i in range(3)]
     aa,bb=f(native['y0'][k]),f(native['y1'][k])
     co.append([aa,bb-aa+d[0],d[1]-d[0],d[2]-d[1],-d[2]])
    for row in s['power'][len(co):]:
     assert all(a==b for a,b in row)
     co.append([f(a)for a,b in row])
    for u in [F(k,4) for k in range(5)]:
     vals=[sum(v*u**i for i,v in enumerate(row))for row in co]
     true=ref(a+(b-a)*mp.mpf(u.numerator)/u.denominator)
     for k,(value,z) in enumerate(zip(vals,true)):
      err=abs(mp.mpf(value.numerator)/value.denominator-z)
      assert err<=m(c['whole_x_error'] if k<n else c['whole_z_error'])+mp.mpf('1e-65')
     ideal_checks+=1

  certs=[s['certificate'] for s in run['slabs'] if s['certificate'] is not None]
  if run['pointwise_qualified']:
   assert len(run['accepted'])==len(raw['t']) and [q['t']for q in run['queries']]==src['grid']
   assert [q['raw']for q in run['accepted']]==raw['y']
  if run['branch_qualified']:assert len(certs)==len(raw['t'])-1
  if run['global_qualified']:
   assert all(c['global_qualified'] for c in certs) and run['max_physical_error']<=r['policy']['global_tol']
   assert all(q['physical_error_upper'] is not None for q in run['accepted']+run['queries'])
  err=max(errs,default=0.);tol=2e-7 if r['tight'] else 2e-5
  v.update(qualified=run['pointwise_qualified'] and run['branch_qualified'] and err<=tol,global_qualified=run['global_qualified'],pointwise=run['pointwise_qualified'],branch=run['branch_qualified'],max_error=err,physical_bound=run['max_physical_error'],
    raw_constraint=max((q['raw_constraint_upper']or 0 for q in run['queries']),default=0),projected_constraint=max((q['projected_constraint_upper']for q in run['accepted']+run['queries']),default=0),slabs=len(certs),checked_queries=len(errs),ideal_checks=ideal_checks,green_slabs=sum(c['green_used']for c in certs),work=run['work'],rhs=raw['stats']['counters']['rhs_calls'],actual_route=run['route'],reason=run['reason'],failed_slabs=[{'times':s['times'],'reason':s['reason']}for s in run['slabs']if s['certificate'] is None])
  result.append(v)
 d={'cases':len(rows),'qualified':sum(r['qualified'] for r in result),'global_qualified':sum(r['global_qualified'] for r in result),'slab_checks':sum(r.get('slabs',0) for r in result),'query_checks':sum(r.get('checked_queries',0) for r in result),'ideal_checks':sum(r.get('ideal_checks',0) for r in result),'rows':result}
 (P/(name.upper()+'_AUDIT.json')).write_text(json.dumps(d,indent=2,sort_keys=True,allow_nan=False)+'\n')
 print(name,{k:v for k,v in d.items()if k!='rows'})
 for r in result:
  if not r['qualified']:print('UNQUALIFIED',r['case'],r['candidate'],r['tight'],r['reason'],r.get('failed_slabs',[])[:1],r.get('max_error'))
if __name__=='__main__':main(sys.argv[1])
