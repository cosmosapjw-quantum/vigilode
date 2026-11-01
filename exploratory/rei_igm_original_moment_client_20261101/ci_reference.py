"""CI: reproduce Radau/DOP853 reference from immutable original PR332 numeric fixture alone.
Requires only NumPy/SciPy; full 46080-cell source replay is in the source-inclusive ZIP.
"""
from pathlib import Path
import json
import numpy as np
from scipy.integrate import solve_ivp
from rei_moments import MomentSource
if __name__=='__main__':
    m=MomentSource();exp=json.loads((Path(__file__).resolve().parent/'reference_trajectory.json').read_text())
    t=np.array([p['u'] for p in exp['states']],float)
    ref=np.array([p['y'] for p in exp['states']],float)
    x0=np.r_[np.ones(3),np.zeros(3)]
    a=solve_ivp(m.rhs_aug,(0,1),x0,method='Radau',jac=m.jac_aug,rtol=3e-12,atol=2e-14,t_eval=t)
    b=solve_ivp(m.rhs_aug,(0,1),x0,method='DOP853',rtol=3e-12,atol=2e-14,t_eval=t)
    assert a.success and b.success
    d=float(np.max(np.abs(a.y.T-ref)));d2=float(np.max(np.abs(b.y.T-ref)))
    assert d<2e-11 and d2<2e-11,(d,d2)
    print('SCOPED_SOURCE_CONDITIONAL_RADAU_REFERENCE_PASS',d,d2)
