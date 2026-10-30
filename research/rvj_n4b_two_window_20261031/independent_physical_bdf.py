#!/usr/bin/env python3
"""Scientific cross-check only (not rigorous IVP interval). Source model and IC
reimplemented independently using represented float coefficients.
"""
import csv,json,time
from pathlib import Path
import numpy as np
from scipy.integrate import solve_ivp
root=Path(__file__).resolve().parent
n4b=list(csv.DictReader((root/'evidence/N4B_TWO_WINDOW_NATIVE.csv').open()))
initial=list(csv.DictReader((Path('/mnt/data/VIGILODE_RVJ_N4A_CARRIER_20261030/evidence/N4A_FROZEN_NATIVE_CARRIER.csv')).open()))
allout=[]
for n in [12,24,65]:
    a0=np.array([complex(float(r['source_re']),float(r['source_im'])) for r in initial if int(r['n'])==n],dtype=np.complex128)
    wt=np.full(n,1./n)
    xs=np.array([2.*float(j)/float(n-1)-1. for j in range(n)])
    kap=np.array([.25 if j%2==0 else 1000. for j in range(n)])
    basis=np.array([[1.]*n, [1. if j%2==0 else -1. for j in range(n)]])
    chi=np.array([.05,.02])
    def rhs(t,z):
        v=z[:n]+1j*z[n:]
        means=(basis*wt).dot(v)
        forcing=1j*(basis.T@(chi*(np.abs(means)**2)*means))
        dv=(-kap+1j*.5*xs)*v+forcing
        return np.r_[dv.real,dv.imag]
    out=solve_ivp(rhs,(0.,.5),np.r_[a0.real,a0.imag],method='BDF',rtol=2e-13,atol=2e-15,t_eval=[.25,.5])
    assert out.success,out.message
    arms=[]
    for i,w in enumerate([1,2]):
        native=[r for r in n4b if int(r['n'])==n and int(r['window'])==w];native.sort(key=lambda z:int(z['j']))
        exact=out.y[:n,i]+1j*out.y[n:,i]
        fv=np.array([complex(float(r['float_re']),float(r['float_im'])) for r in native])
        er=np.abs(fv.real-exact.real)+np.abs(fv.imag-exact.imag)
        bd=np.array([float(r['physical_conditional_l1_upper']) for r in native])
        arms.append({'window':w,'max_l1_discrepancy':float(max(er)),'max_reported_bound':float(max(bd)),'sampled_within_bound':bool(np.all(er<=bd))})
    allout.append({'n':n,'BDF_success':out.success,'nfev':out.nfev,'njev':out.njev,'t_end':float(out.t[-1]),'arms':arms})
    print(json.dumps(allout[-1]),flush=True)
report={'type':'floating BDF cross-check, NON_RIGOROUS','result':allout}
(root/'evidence/N4B_BDF_COMPARISON.json').write_text(json.dumps(report,indent=2)+'\n')
