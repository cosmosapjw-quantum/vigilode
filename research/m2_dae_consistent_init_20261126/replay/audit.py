"""Independent SymPy exact source reconstruction, 85-digit branch and tangent oracle."""
import json,sys,struct,functools
from pathlib import Path
import sympy as s
import mpmath as mp
mp.mp.dps=85
P=Path(__file__).resolve().parent
def read(f,tag):return [json.loads(l.split(tag,1)[1]) for l in f.read_text().splitlines() if tag+'{' in l]
def Q(v):return s.Rational(float(v))
def M(v):return mp.mpf(float(v))
def MP(v):return mp.mpf(str(s.N(v,88)))
def same_bits(a,b):return len(a)==len(b) and all(struct.pack('d',u)==struct.pack('d',v) for u,v in zip(a,b))
def source_expr(src):
 nx,nz=len(src['f']),len(src['g']);t=s.Symbol('t');y=s.symbols(f'y0:{nx+nz}')
 def expr(eq):
  return s.expand(sum(Q(term['coefficient'])*s.prod(Q(a['constant'])+Q(a['time'])*t+sum(Q(w)*y[i] for i,w in a['state']) for a in term['factors']) for term in eq['terms']))
 f=s.Matrix([expr(v) for v in src['f']]);g=s.Matrix([expr(v) for v in src['g']])
 return t,y,f,g
def matnorm(a):return max(sum(abs(v) for v in row) for row in a.tolist())
def oracle(src):
 t,y,f,g=source_expr(src['src']);nx=len(src['x']);nz=len(src['z'])
 fixed={t:Q(src['t']),**dict(zip(y[:nx],map(Q,src['x'])))}
 gf=g.subs(fixed);guess=list(map(Q,src['root_hint']))
 if all(v==0 for v in gf.subs(dict(zip(y[nx:],guess)))):
  root=guess
 else:
  assert nz==1,src['case']
  roots=s.polys.polytools.Poly(gf[0],y[nx]).all_roots()
  roots=[r for r in roots if r.is_real and Q(src['branch'][0][0])<r<Q(src['branch'][0][1])]
  assert len(roots)==1,(src['case'],roots)
  root=roots
 sub={**fixed,**dict(zip(y[nx:],root))}
 fr=f.subs(sub);gz=g.jacobian(y[nx:]).subs(sub);gx=g.jacobian(y[:nx]).subs(sub);gt=g.diff(t).subs(sub)
 zx=-gz.inv()*gx;zt=-gz.inv()*gt;zd=zt+zx*fr
 jr=f.jacobian(y[:nx]).subs(sub)+f.jacobian(y[nx:]).subs(sub)*zx
 return dict(t=t,y=y,f=f,g=g,fixed=fixed,root=root,xdot=list(fr),zdot=list(zd),zt=list(zt),zx_columns=[list(zx[:,i]) for i in range(nx)],reduced_jacobian=jr.tolist(),inverse_norm=matnorm(gz.inv()),condition=matnorm(gz)*matnorm(gz.inv()))
def maxerr(v,w):
 return max((abs(M(a)-MP(b)) for a,b in zip(v,w)),default=mp.mpf(0))
def audit(src,r,o,policy):
 nx,nz=len(src['x']),len(src['z'])
 assert r['qualified'] and r['rank']==nz and same_bits(r['x'],src['x']) and same_bits(r['original_z'],src['z'])
 assert r['t']==src['t'] and r['branch']==src['branch'] and r['mode']==src['mode']
 if r['mode']=='Strict':assert same_bits(r['z'],src['z']) and r['correction_upper']==0
 assert all(M(l)<=MP(z)<=M(h) for (l,h),z in zip(r['root_box'],o['root']))
 assert all(b[0]<=q[0]<=q[1]<=b[1] for q,b in zip(r['root_box'],r['branch']))
 assert maxerr(r['z'],o['root'])<=M(r['state_error_upper'])+mp.mpf('1e-75')
 assert r['state_error_upper']<=policy['state_tol']
 t,y,f,g=o['t'],o['y'],o['f'],o['g']
 at={**o['fixed'],**dict(zip(y[nx:],map(Q,r['z'])))}
 orig={**o['fixed'],**dict(zip(y[nx:],map(Q,src['z'])))}
 g0=max(abs(v) for v in g.subs(orig));gc=max(abs(v) for v in g.subs(at))
 assert g0<=Q(r['original_residual_upper']) and gc<=Q(r['residual_upper'])
 assert r['residual_upper']<=policy['residual_tol']
 assert not r['exact_center_consistency'] or gc==0
 corr=max(abs(Q(a)-Q(b)) for a,b in zip(r['z'],src['z']))
 assert corr<=Q(r['correction_upper'])
 tangent=g.diff(t).subs(at)+g.jacobian(y).subs(at)*s.Matrix(list(map(Q,r['xdot']+r['zdot'])))
 assert max(abs(v) for v in tangent)<=Q(r['tangent_residual_upper'])
 assert r['tangent_residual_upper']<=policy['derivative_tol']
 err=max(maxerr(r[k],o[k]) for k in ['xdot','zdot','zt'])
 for k in ['zx_columns','reduced_jacobian']:
  err=max(err,max(maxerr(a,b) for a,b in zip(r[k],o[k])))
 assert err<=M(r['derivative_error_upper'])+mp.mpf('1e-75'),(src['case'],r['route'],err,r['derivative_error_upper'])
 assert r['derivative_error_upper']<=policy['derivative_tol']
 assert MP(o['condition'])<=M(r['condition_upper'])+mp.mpf('1e-75')
 assert MP(o['inverse_norm'])<=M(r['inverse_norm_upper'])+mp.mpf('1e-75')
 assert r['condition_upper']<=policy['condition_cap'] and 0<=r['local_defect_upper']<1
 if r['branch_authority']=='exact scalar root count':
  a,b=map(Q,src['branch'][0]);p=s.Poly(g[0].subs(o['fixed']),y[nx])
  assert p.eval(a)!=0 and p.eval(b)!=0 and p.count_roots(a,b)==1
  assert p.diff().eval(o['root'][0])!=0
 else:assert 0<=r['branch_defect_upper']<1
 return {'max_state_error':float(maxerr(r['z'],o['root'])),'max_derivative_error':float(err),'state_bound':r['state_error_upper'],'derivative_bound':r['derivative_error_upper'],'residual_upper':r['residual_upper'],'condition_upper':r['condition_upper'],'work':r['work'],'actual_route':r['route'],'branch_authority':r['branch_authority']}
def run(name):
 srcs=read(P/(name+'.log'),'SOURCE ');rows=read(P/(name+'.log'),'CELL ')
 expected=144 if name=='training' else sum(v['selected'] is not None for g in json.loads((P/'SELECTION.json').read_text())['groups'].values() for v in g.values())
 assert len(srcs)==(9 if name=='training' else 4) and len(rows)==expected,(len(srcs),len(rows),expected)
 assert len({(r['case'],r['candidate'],r['tight']) for r in rows})==len(rows)
 src={r['case']:r for r in srcs};oracles={k:oracle(v) for k,v in src.items()};result=[]
 for r in rows:
  d={k:r[k] for k in ['case','group','candidate','tight','median_ns']}
  q=r['result']
  if q is None or q['report'] is None:
   d.update(qualified=False,reason=r.get('reason') if q is None else 'no qualified adaptive route')
  else:
   d.update(qualified=True,**audit(src[r['case']],q['report'],oracles[r['case']],r['policy']))
  result.append(d)
 out={'cases':len(rows),'qualified':sum(v['qualified'] for v in result),'refused':sum(not v['qualified'] for v in result),'root_oracles':len(oracles),'precision_dps':mp.mp.dps,'sympy':s.__version__,'rows':result}
 (P/(name.upper()+'_AUDIT.json')).write_text(json.dumps(out,indent=2,sort_keys=True,allow_nan=False)+'\n')
 print(name,{k:v for k,v in out.items() if k!='rows'})
 for r in result:
  if not r['qualified']:print('REFUSED',r['case'],r['candidate'],r['tight'],r['reason'])
def mass():
 import numpy as np
 from scipy.integrate import solve_ivp
 out=[]
 rows=read(P/'contracts.log','MASS ');assert len(rows)==2
 for r in rows:
  t=np.array(r['times']);y=np.array(r['states']);n=len(r['init']['x'])
  if r['case']=='analytic':ref=np.array([np.exp(-t),np.exp(-2*t)+t]).T
  else:
   def rhs(t,y):
    a,b=y;c=1-a-b
    return [-.04*a+1e4*b*c,.04*a-1e4*b*c-3e7*b*b]
   sol=solve_ivp(rhs,(0.,r['end']),[1.,0.],method='Radau',rtol=2.5e-14,atol=1e-16,t_eval=t)
   assert sol.success and np.array_equal(sol.t,t)
   ref=np.column_stack([sol.y.T,1-sol.y.sum(axis=0)])
  sym,sy,f,g=source_expr(r['source'])
  exactres=max(abs(v) for t0,yy in zip(t,y) for v in g.subs({sym:Q(t0),**dict(zip(sy,map(Q,yy)))}))
  err=float(np.max(abs(y-ref)));assert err<1e-5 and exactres< s.Rational('1e-5')
  out.append({'case':r['case'],'nodes':len(t),'max_abs_error':err,'max_original_constraint':float(exactres),'rhs_calls':r['counters']['rhs_calls'],'scope':'empirical accepted-node smoke, no global enclosure'})
 (P/'MASS_AUDIT.json').write_text(json.dumps(out,indent=2)+'\n')
 print(out)
if __name__=='__main__':
 if sys.argv[1]=='mass':mass()
 else:run(sys.argv[1])
