"""Independent sparse diagonal BDF/Radau work-precision controls.

Run on the same host as modal_work_precision. The implementation languages and
tolerance controllers differ. Match achieved errors before comparing wall time.
"""
import csv
import sys
from time import perf_counter_ns

import numpy as np
import scipy
from scipy.integrate import solve_ivp
from scipy.sparse import diags

writer = csv.writer(sys.stdout, lineterminator="\n")
writer.writerow(["dimension","rtol","rep","method","elapsed_ns","max_abs_error","nfev","njev","nlu","scipy"])
for n in (64,512):
    k = np.arange(n,dtype=float)
    rates = -1000*(k/(n-1))**2
    y0 = 1/(1+0.0001*k*k)
    jac = diags(rates,format="csc")
    ref = np.exp(.5*rates)*y0
    for tol in (1e-4,1e-6,1e-8):
        for rep in range(6):
            for method in (("BDF","Radau") if rep%2==0 else ("Radau","BDF")):
                start=perf_counter_ns()
                sol=solve_ivp(lambda t,y: rates*y,(0.,.5),y0,method=method,jac=jac,
                              rtol=tol,atol=1e-3*tol,first_step=.3,t_eval=[.5])
                elapsed=perf_counter_ns()-start
                if not sol.success:
                    raise RuntimeError(sol.message)
                err=np.max(np.abs(sol.y[:,-1]-ref))
                if rep:
                    writer.writerow([n,tol,rep,method,elapsed,format(err,".16e"),sol.nfev,sol.njev,sol.nlu,scipy.__version__])
