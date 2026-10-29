#!/usr/bin/env python3
"""Bounded VODE BDF comparison against the VigilODE benchmark's identical five IVPs.
This is a NEW host and no Diffsol timing measurement occurs here.
"""
import sys, time, csv, json, warnings, platform
from pathlib import Path
import numpy as np, scipy
from scipy.integrate import ode, solve_ivp
from scipy.sparse import csr_matrix
sys.path.insert(0, str(Path(__file__).resolve().parent))
from vode_problem_models import robertson, hires, vdp, brusselator, packed
P=Path(__file__).resolve().parent
P.mkdir(exist_ok=True)

CASES=[robertson(),hires(),vdp(),brusselator(50),brusselator(200)]

def solve(p,rtol,grid,mode='dense',budget=100000):
    count=[0,0]
    def f(t,y): count[0]+=1;return p.rhs(t,y)
    def j(t,y):
        count[1]+=1; v=p.jac(t,y)
        return packed(v,p.lband,p.uband) if mode=='banded' else v
    opts=dict(method='bdf',rtol=rtol,atol=rtol*p.atol_scale,nsteps=budget,first_step=1e-6,min_step=1e-14,max_step=p.tf)
    if mode=='banded': opts.update(lband=p.lband,uband=p.uband)
    solver=ode(f,j).set_integrator('vode',**opts).set_initial_value(p.y0,0.)
    ys=[p.y0.copy()];warnings_list=[]
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter('always')
        for t in grid[1:]:
            solver.integrate(float(t));ys.append(solver.y.copy())
            if not solver.successful() or not np.all(np.isfinite(solver.y)):break
        warnings_list=[str(x.message) for x in caught]
    return {'success':solver.successful() and len(ys)==len(grid) and abs(solver.t-grid[-1])<=max(1e-12,1e-12*abs(grid[-1])), 't_reached':float(solver.t),'code':int(solver.get_return_code()),'states':np.asarray(ys),'nfe':count[0], 'nje':count[1],'nst':int(solver._integrator.iwork[10]),'warnings':warnings_list}

def ref(p,grid):
    jac=lambda t,y:csr_matrix(p.jac(t,y)) if p.n>=100 else p.jac(t,y)
    s=solve_ivp(p.rhs,[0,p.tf],p.y0,method='Radau',rtol=2e-12,atol=1e-14*p.atol_scale,jac=jac,t_eval=grid)
    assert s.success and len(s.t)==len(grid)
    return s.y.T

def main():
    out=[];refs={};rtols=[1e-4,1e-5,1e-6]
    for p in CASES:
        grid=np.array([0.,p.tf]); rf=ref(p,grid);refs[p.name]=rf[-1].tolist()
        for rtol in rtols:
            modes=['dense'] if p.n<=8 else ['banded'] if p.n>=400 else ['dense','banded']
            for mode in modes:
                st=time.perf_counter();first=solve(p,rtol,grid,mode);t0=time.perf_counter()-st
                measurements=[];last=first
                if first['success']:
                    for i in range(3):
                        st=time.perf_counter();r=solve(p,rtol,grid,mode);dt=time.perf_counter()-st
                        assert r['success'] and np.array_equal(r['states'][-1],first['states'][-1])
                        measurements.append(dt);last=r
                row={'case':p.name,'n':p.n,'mode':mode,'rtol':rtol,'atol':rtol*p.atol_scale,'status':'ok' if first['success'] else 'solver_failure','t_final':first['t_reached'],'return_code':first['code'],'nfe':first['nfe'],'nje':first['nje'],'nst':first['nst'],'warmup_ms':t0*1000,'timed_reps':len(measurements),'median_ms':float(np.median(measurements)*1000) if measurements else None,'warning':' | '.join(first['warnings'])}
                if first['success']:
                    delta=first['states'][-1]-rf[-1]
                    row['endpoint_max_relative_error']=float(np.max(np.abs(delta)/np.maximum(np.abs(rf[-1]),1e-10)))
                    row['endpoint_WRMS_at_requested_tol']=float(np.sqrt(np.mean((delta/(rtol*p.atol_scale+rtol*np.abs(rf[-1])))**2)))
                out.append(row)
                print('VODE',p.name,rtol,mode,row['status'],'nst',first['nst'],'nfe',first['nfe'],'ms',row['median_ms'],'err',row.get('endpoint_WRMS_at_requested_tol'),flush=True)
    # Robertson uniform101 accuracy at the same grid as the published Diffsol A07 comparison
    p=CASES[0];grid=np.linspace(0,p.tf,101);rf=ref(p,grid)
    for rtol in [1e-6,1e-8]:
        st=time.perf_counter();r=solve(p,rtol,grid,'dense',budget=50000)
        row={'case':'robertson','n':3,'mode':'dense','grid':'uniform101','rtol':rtol,'atol':rtol*p.atol_scale,'status':'ok' if r['success'] else 'solver_failure','t_final':r['t_reached'],'return_code':r['code'],'nfe':r['nfe'],'nje':r['nje'],'nst':r['nst'],'timed_reps':1,'median_ms':(time.perf_counter()-st)*1000,'warning':' | '.join(r['warnings'])}
        if r['success']:
            delta=r['states']-rf
            row['grid_WRMS_at_requested_tol']=float(np.max(np.sqrt(np.mean((delta/(rtol*p.atol_scale+rtol*np.abs(rf)))**2,axis=1))))
            row['endpoint_WRMS_at_requested_tol']=float(np.sqrt(np.mean(((delta[-1])/(rtol*p.atol_scale+rtol*np.abs(rf[-1])))**2)))
            row['endpoint_max_relative_error']=float(np.max(np.abs(delta[-1])/np.maximum(np.abs(rf[-1]),1e-10)))
        out.append(row);print('VODE Robertson 101',rtol,row['status'],row['t_final'],'WRMS',row.get('grid_WRMS_at_requested_tol'),flush=True)
    fields=list(dict.fromkeys(k for r in out for k in r))
    with (P/'vode_bdf_targeted.csv').open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(out)
    (P/'reference_endpoints.json').write_text(json.dumps({'method':'SciPy Radau','rtol':2e-12,'atol_rule':'1e-14*atol_scale','endpoints':refs},indent=2))
    (P/'environment.json').write_text(json.dumps({'python':sys.version,'scipy':scipy.__version__,'numpy':np.__version__,'platform':platform.platform(), 'note':'DIFFERENT runtime and host from archived VigilODE and Diffsol; compare accuracy/work, not times cross-host.'},indent=2))
if __name__=='__main__':main()
