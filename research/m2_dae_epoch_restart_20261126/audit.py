"""Independent 85-digit oracle of the numerical query receipts."""
from pathlib import Path
import json,mpmath as mp,hashlib
P=Path(__file__).parent
mp.mp.dps=85
def m(x): return mp.mpf(float(x))
def oracle(kind,s,t,x0):
 if kind=='forcing':x=x0+(t*t-s*s)/2;z=x+t
 elif kind=='analytic':x=x0*mp.exp(-(t-s));z=x*x+t
 elif kind=='pencil':x=x0*mp.exp(-(t-s))*(1+t/8)/(1+s/8);z=x/(1+t/8)
 else:raise ValueError(kind)
 return x,z
rows=[json.loads(l.split('CELL ',1)[1]) for l in (P/'training.log').read_text().splitlines() if 'CELL {' in l]
keys={(r['kind'],r['start'],r['route'],r['green']) for r in rows}
assert len(rows)==len(keys)==126
assert keys=={(k,s,r,g) for k in ['analytic','pencil','forcing'] for s in [-0.5,0.5,1.] for r in ['Mass','Reduced3','Reduced5','Prepared5','Scalar5','WarpDirect','WarpFolded'] for g in [False,True]}
summary=[];samples=0;maxerr=0.;maxratio=0.
for r in rows:
 k,s,route,g=r['kind'],r['start'],r['route'],r['green']
 good=bool(r['global'] and r.get('branch') and r.get('pointwise') and r.get('bound',1e9)<=0.005)
 out={v:r[v] for v in ['kind','start','route','green','global']}
 out['qualified']=good
 if 'error' in r:out['error']=r['error']
 else:
  out.update({'bound':r['bound'],'driver_rhs':r['driver_rhs'],'projection_work':r['work'],'rhs_proxy':r['driver_rhs']+r['work']['rhs_count']})
  obs=0.
  assert len(r['queries'])==17
  for i,q in enumerate(r['queries']):
   t=m(q['t']);assert t==m(s)+mp.mpf(i)/32
   for j in (-1,0,1):
    a,b=oracle(k,m(s),t,1+j*mp.mpf(2)**-12)
    err=max(abs(a-m(q['state'][0])),abs(b-m(q['state'][1])))
    obs=max(obs,float(err)); samples+=1
    if good:
     bnd=m(q['physical_error_upper'])
     assert err<=bnd,(k,s,route,g,i,j,err,bnd)
     if bnd>0:maxratio=max(maxratio,float(err/bnd))
  out['independent_max_error']=obs;maxerr=max(maxerr,obs)
 summary.append(out)
res={'rows':summary,'cells':len(rows),'qualified':sum(r['qualified'] for r in summary),'refused':sum('error' in r for r in summary),'returned_unqualified':sum(not r['qualified'] and 'error' not in r for r in summary),'oracle_samples':samples,'max_error':maxerr,'max_qualified_error_ratio':maxratio,'input_sha256':hashlib.sha256((P/'training.log').read_bytes()).hexdigest(),'scope':'finite represented samples only; no holdout'}
(P/'AUDIT.json').write_text(json.dumps(res,indent=2,sort_keys=True,allow_nan=False)+'\n')
print({k:v for k,v in res.items() if k!='rows'})
