#!/usr/bin/env python3
"""Replicate the published A07 Robertson inputs using SciPy VODE (BDF); reference is independent Radau+LSODA."""
import sys,time,json,platform, warnings
from pathlib import Path
import numpy as np, scipy
from scipy.integrate import solve_ivp
sys.path.insert(0,str(Path(__file__).resolve().parent))
from vode_problem_models import robertson
from vode_targeted import solve
P=Path(__file__).resolve().parent
p=robertson();p.atol_scale=1e-3
# Here 1e-8 * p.atol_scale = 1e-11, matching the archived Diffsol A07.
grid=np.linspace(0.0,40.0,101);rtol=1e-8;atol=1e-11
radau=solve_ivp(p.rhs,(0,40),p.y0,method='Radau',jac=p.jac,rtol=2e-12,atol=1e-17,t_eval=grid)
lsoda=solve_ivp(p.rhs,(0,40),p.y0,method='LSODA',jac=p.jac,rtol=2e-12,atol=1e-17,t_eval=grid)
assert radau.success and lsoda.success
ref=radau.y.T
D=lambda a,b: np.max(np.sqrt(np.mean(((a-b)/(atol+rtol*np.abs(ref)))**2,axis=1)))
refdis=float(D(radau.y.T,lsoda.y.T))
observations=[]
for i in range(4):
    st=time.perf_counter();q=solve(p,rtol,grid,budget=200000);dt=(time.perf_counter()-st)
    if not q['success']: raise RuntimeError('VODE did not finish')
    observations.append({'median_ms_input':dt*1000,'nfe':q['nfe'],'nje':q['nje'],'nst':q['nst'],'code':q['code'],'output_hash':__import__('hashlib').sha256(q['states'].tobytes()).hexdigest()})
assert len(set(x['output_hash'] for x in observations))==1
results={'experiment':'A07 VODE 1e-8 rtol 1e-11 atol 101 uniformly spaced output points','runtime':{'scipy':scipy.__version__,'python':sys.version,'host':platform.platform()}, 'vode':{'method':'bdf','analytic_dense_jacobian':True,'nsteps_cap_each_output_call':200000,'rtol':rtol,'atol':atol,'cold_ms':observations[0]['median_ms_input'],'replicated_median_ms':float(np.median([v['median_ms_input'] for v in observations[1:]])),'rms_scaled_max_grid':float(D(q['states'],ref)),'reference_discrepancy_scaled_max_grid':refdis,'reference_plus_discrepancy':float(D(q['states'],ref))+refdis,'nst':q['nst'],'nfe':q['nfe'],'nje':q['nje'],'repeated_output_identical_sha256':observations[0]['output_hash'],'output_count':len(q['states']), 'max_mass_conservation_defect':float(np.max(np.abs(q['states'].sum(axis=1)-1)))}, 'repetitions':observations, 'historical_A07':{'native_vigilode_scaled_max_grid':0.33753007149457903,'diffsol_0_16_1_scaled_max_grid':3.3026992500504733,'accuracy_gate_native':True,'accuracy_gate_diffsol':False,'diagnostic_within_A07_warm_pair_ratio_diffsol_over_native':1.4194023547437313,'source':'https://github.com/cosmosapjw-quantum/vigilode/blob/claude/nice-dijkstra-208xq0/research/production_audit_20261018/evidence/continuation_a07/REPORT.md'},'scope':'VODE time from a different host, compare accuracy against independent references, not cross-host runtime ratios'}
(P/'a07_vode_exact.json').write_text(json.dumps(results,indent=2))
print(json.dumps(results['vode'],indent=2))
