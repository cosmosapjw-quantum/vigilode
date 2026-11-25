"""Independent exact-binary weight audit, high precision affine flow, convex dual.
No solver or function table regeneration. All inputs are preserved Rust receipts.
"""
import json,math,statistics
from fractions import Fraction as F
from pathlib import Path
import mpmath as mp
import numpy as np
from scipy.optimize import minimize
import scipy
root=Path(__file__).resolve().parent
def records(name, marker):
    return [json.JSONDecoder().raw_decode(l.split(marker,1)[1])[0]
            for l in (root/name).read_text().splitlines() if marker in l]
rr=records('regression.log','RUN ')
wp=records('timing.log','WP ')
ev=records('regression.log','EVENT ')
assert len(rr)==6 and len(wp)==15 and len(ev)==1
mp.mp.dps=100
rates=[4.,4.,4.,4.,80.,80.,.25,.25,.25,.25,.25,.25]
eq=[1.,-.25,.75,.25,-.5,.6,.2,1.,-.4,.5,.7,.1]
lower=[4.,4.,4.,4.,40.,40.,.25,.25,.25,.25,.25,.25]
end=[(i+1)/4 for i in range(12)]
force=[a*b for a,b in zip(rates,eq)]
exact=[mp.mpf(.2)]
for a,b in zip(rates,force):
    g=mp.exp(-mp.mpf(a)/4)
    exact.append(g*exact[-1]+(1-g)*mp.mpf(b)/mp.mpf(a))
exact_count=0;weight_count=0;max_ratio=mp.mpf(0);min_margin=mp.inf
def fractions_row(g,k,dense):
    return [math.prod(g[i+1:k if dense else k+1]) if i<k else F(int(i==k)) for i in range(len(g))]
def rows(g,dense):
    return [fractions_row(g,11,False)]+([fractions_row(g,k,True) for k in range(12)] if dense else [])
for rec in rr+wp:
    m=rec['metrics'];p=m['plan'];budgets=list(map(F.from_float,p['budgets']))
    gs=list(map(F.from_float,p['gain_upper']))
    assert not p['forecast_fallback']
    for i,g in enumerate(gs):
        ref=mp.exp(-mp.mpf(lower[i])/4)
        assert mp.mpf(g.numerator)/g.denominator>=ref
    ww=rows(gs,rec['dense'])
    assert len(ww)==len(p['planned_target_upper'])
    for w,b in zip(ww,p['planned_target_upper']):
        actual=sum(a*d for a,d in zip(w,budgets))
        assert actual<=F.from_float(b)<=F.from_float(rec['tol'])
        weight_count+=1
    prev=mp.mpf(.2)
    for i,s in enumerate(m['slabs']):
        observed=mp.mpf(s['endpoint'])
        g=mp.exp(-mp.mpf(rates[i])/4)
        local=abs(observed-(g*prev+(1-g)*mp.mpf(force[i])/mp.mpf(rates[i])))
        err=abs(observed-exact[i+1])
        assert local<=mp.mpf(s['local_endpoint']),(rec['mode'],i,'local',local,s['local_endpoint'])
        assert err<=mp.mpf(s['endpoint_upper']),(rec['mode'],i,'global',err,s['endpoint_upper'])
        assert mp.mpf(s['local_endpoint'])<=mp.mpf(p['budgets'][i])
        if rec['dense']:
            assert mp.mpf(s['local_dense'])<=mp.mpf(p['budgets'][i])
        max_ratio=max(max_ratio,err/mp.mpf(s['endpoint_upper']) if s['endpoint_upper'] else 0)
        min_margin=min(min_margin,mp.mpf(s['endpoint_upper'])-err)
        prev=observed;exact_count+=1
event=ev[0]['bracket']['Unique'];root_exact=mp.log(3)/2
assert mp.mpf(event['lo'])<=root_exact<=mp.mpf(event['hi']) and event['qualified']
# Same p=4 cost proxy, normalized x=delta/tolerance, capped at 1e-2/tol.
# Dual lambda>=0, gamma=W.T lambda. Analytic separable infimum of Lagrangian.
# A feasible rescaled primal plus a dual point provides a numerical gap.
opt=[]
for rec in wp:
    if rec['mode']!='Retro':continue
    p=rec['metrics']['plan'];tol=rec['tol'];cap=.01/tol
    W=np.array([[float(a) for a in w] for w in rows(list(map(F.from_float,p['gain_upper'])),rec['dense'])])
    rhs=np.full(W.shape[0],.98)
    def calc(lam):
        gamma=W.T@lam
        x=np.full(12,cap)
        positive=gamma>0
        x[positive]=np.minimum(cap,(.25/gamma[positive])**.8)
        val=np.sum(x**(-.25)+gamma*x)-lam@rhs
        grad=W@x-rhs
        return val,grad,x
    def fun(lam):
        val,grad,x=calc(lam)
        return -val,-grad
    sol=minimize(fun,np.ones(W.shape[0]),jac=True,bounds=[(0.,None)]*W.shape[0],
                 method='L-BFGS-B',options={'ftol':1e-14,'gtol':1e-9,'maxiter':3000,'maxls':60})
    dual,_,x=calc(sol.x)
    x=x*min(1.,float(np.min(rhs/(W@x))))*(1-1e-12)
    primal=float(np.sum(x**(-.25)))
    heuristic=float(np.sum((np.array(p['budgets'])/tol)**(-.25)))
    assert np.max(W@x-rhs)<=1e-12
    assert primal>=dual-1e-10
    opt.append(dict(dense=rec['dense'],tol=tol,dual=float(dual),primal=primal,
                    relative_gap=float((primal-dual)/primal),heuristic=heuristic,
                    heuristic_over_primal=heuristic/primal,success=bool(sol.success),
                    iterations=int(sol.nit),optimal_budget_proposal=(x*tol).tolist()))
out={'schema':'retro-causal-independent-audit-v1','endpoint_cases':exact_count,
     'rational_budget_rows':weight_count,'max_endpoint_error_over_bound':float(max_ratio),
     'minimum_endpoint_margin':float(min_margin),'event_width':event['hi']-event['lo'],
     'convex_dual_diagnostic':opt,'versions':{'mpmath':mp.__version__,'numpy':np.__version__,'scipy':scipy.__version__}}
(root/'AUDIT.json').write_text(json.dumps(out,indent=2,sort_keys=True)+'\n')
print(json.dumps(out,indent=2))
