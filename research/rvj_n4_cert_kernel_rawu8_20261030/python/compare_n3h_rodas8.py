"""Independent source-matched sampled accuracy comparison; no wall clock claim."""
import csv, json, sys
from pathlib import Path
import numpy as np
from scipy.integrate import solve_ivp
from rodas8_rank2_reference import physical,one
DATA=Path(__file__).resolve().parents[1]/'evidence/N3H_PARENT_WINDOWS.csv'
rows=list(csv.DictReader([s for s in DATA.read_text().splitlines() if not s.startswith('#')]))
out=[]
for n in (12,24):
 y,f,jac,energy=physical(n)
 ref=solve_ivp(f,(0,.25),y,method='BDF',jac=jac,rtol=2e-12,atol=2e-14,max_step=.001)
 cand=np.zeros_like(y)
 select=[r for r in rows if int(r['n'])==n and int(r['window'])==1]
 assert len(select)==n
 for r in select:
  j=int(r['mode']);cand[2*j]=float(r['re']);cand[2*j+1]=float(r['im'])
 dif=cand-ref.y[:,-1]
 scale=1e-10+1e-7*np.maximum(np.abs(y),np.abs(cand))
 old=one(n,.25)
 rec=dict(n=n,h=.25,n3h_candidate_max_abs=float(np.max(np.abs(dif))),n3h_candidate_wrms=float(np.sqrt(np.mean((dif/scale)**2))),n3h_energy=energy(cand),reference_energy=energy(ref.y[:,-1]),n3h_original_ideal_bound_header=float(select[0]['ideal_bound']),roda8_max_err_vs_bdf=old['max_endpoint_error'],roda8_wrms_vs_bdf=old['endpoint_error_wrms'],ratio_abs_error_Rodas8_over_N3H=float(old['max_endpoint_error']/np.max(np.abs(dif))),reference_success=bool(ref.success),bdf_nfev=int(ref.nfev))
 out.append(rec)
 print('N4_SOURCEMATCHED_COMPARE',rec)
Path(sys.argv[1]).write_text(json.dumps(out,indent=2)+'\n')
