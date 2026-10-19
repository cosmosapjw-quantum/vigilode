#!/usr/bin/env python3
"""Version-pinned SciPy wrapper differential audit, not native speed evidence."""
import argparse, json, subprocess, sys, time, warnings
from pathlib import Path
import numpy as np
import scipy
from scipy.integrate import ode
from models import model, packed_band, exact_pivot
from contract import assess

def one(c, variant, ref):
    f,j=model(c['name'],c['dimension']);n=c['dimension']
    kwargs={}
    if variant=='analytic-C': jac=lambda t,y:np.ascontiguousarray(j(t,y))
    elif variant=='analytic-F': jac=lambda t,y:np.asfortranarray(j(t,y))
    elif variant=='transpose-control': jac=lambda t,y:np.ascontiguousarray(j(t,y).T)
    elif variant=='internal-FD':jac=None;kwargs['with_jacobian']=True
    elif variant in ('full-band','band-FD'):
        jac=(lambda t,y:packed_band(j(t,y),n-1,n-1)) if variant=='full-band' else None
        kwargs.update(lband=n-1,uband=n-1,with_jacobian=True)
    else:raise ValueError(variant)
    rtol=1e-6;atol=rtol*c['scale'];ts=ref['times']
    saved=[c['y0']];got=[0.0];start=time.perf_counter_ns()
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter('always')
        s=ode(f,jac).set_integrator('vode',method='bdf',rtol=rtol,atol=atol,first_step=1e-6,min_step=1e-14,max_step=c['tf'],nsteps=100000,**kwargs).set_initial_value(c['y0'],0.)
        for t in ts[1:]:
            y=s.integrate(t)
            if not s.successful() or s.t!=t or not np.isfinite(y).all():break
            saved.append(y.tolist());got.append(float(s.t))
    row={'case':c['name'],'variant':variant,'scipy':scipy.__version__,'status':'ok' if len(saved)==len(ts) and s.successful() else 'solver_failure',
        'rtol':rtol,'atol':atol,'times':got,'states':saved,'last_reached_time':float(s.t),'istate':int(s.get_return_code()),
        'wall_ns_diagnostic_only':time.perf_counter_ns()-start,'warning_messages':[str(w.message) for w in caught],
        'iwork':s._integrator.iwork.tolist(),'comparison_scope':'wrapper_regression_diagnostic_not_native_speed'}
    row['metrics']=assess(row,ref['states'],ref['radius'],ts,c['scale'])
    return row

def main():
    p=argparse.ArgumentParser();p.add_argument('--out',type=Path,required=True);p.add_argument('--one');p.add_argument('--variant');a=p.parse_args()
    listing=json.loads((a.out/'models.json').read_text())
    refs=json.loads((a.out/'references.json').read_text())
    cases=[c for c in listing['cases'] if c['dimension']<=8]
    if a.one:
        c=next(c for c in cases if c['name']==a.one)
        print(json.dumps(one(c,a.variant,refs[c['name']+'|endpoint']),allow_nan=False));return
    rows=[]
    for c in cases:
        for v in ['analytic-C','analytic-F','transpose-control','internal-FD','full-band','band-FD']:
            cmd=[sys.executable,__file__,'--out',str(a.out),'--one',c['name'],'--variant',v]
            try:
                r=subprocess.run(cmd,capture_output=True,text=True,timeout=20)
                row=json.loads(r.stdout) if r.returncode==0 else {'case':c['name'],'variant':v,'status':'process_failure','stderr':r.stderr[-6000:]}
            except subprocess.TimeoutExpired:
                row={'case':c['name'],'variant':v,'status':'timeout','timeout_seconds':20}
            rows.append(row)
            (a.out/'SCIPY117_DIAG.json').write_text(json.dumps(rows,indent=2,allow_nan=False)+'\n')
            print(c['name'],v,row['status'],row.get('metrics',{}).get('R_up'),flush=True)
    compact=[{'case':r['case'],'variant':r['variant'],'status':r['status'],'R_up':r.get('metrics',{}).get('R_up'),
        'nst':r.get('iwork',[0]*11)[10] if 'iwork' in r else None,'nfe':r.get('iwork',[0]*12)[11] if 'iwork' in r else None,
        'last_time':r.get('last_reached_time')} for r in rows]
    (a.out/'SCIPY117_DIAG_SUMMARY.json').write_text(json.dumps(compact,indent=2,allow_nan=False)+'\n')
if __name__=='__main__':main()
