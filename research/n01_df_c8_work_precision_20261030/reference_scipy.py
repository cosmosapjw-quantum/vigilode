#!/usr/bin/env python3
"""Independent SciPy reference for original Robertson physical source, diagnostic (NOT rigorous bound)."""
import json
from pathlib import Path
import numpy as np
from scipy.integrate import solve_ivp
ROOT=Path(__file__).resolve().parents[1]
CASES=[
 {"id":"mild", "rates":[0.5,1.0,0.5], "initial":[0.75,0.125,0.125], "end":0.1, "step_counts":[8,16,32,64,128]},
 {"id":"moderate", "rates":[0.5,10.0,200.0], "initial":[0.75,0.125,0.125], "end":0.02, "step_counts":[8,16,32,64,128]},
 {"id":"stiff_negative_control", "rates":[0.04,10000.0,30000000.0], "initial":[0.75,0.125,0.125], "end":0.001, "step_counts":[8,16,32,64]},
]
def evaluate(t,z,k):
 a,b,c=z;k1,k2,k3=k
 p=k1*a;q=k2*b*c;r=k3*b*b
 return [-p+q,p-q-r,r]
def jac(t,z,k):
 a,b,c=z;k1,k2,k3=k
 return [[-k1,k2*c,k2*b],[k1,-k2*c-2*k3*b,-k2*b],[0,2*k3*b,0]]
summary=[]
for case in CASES:
 k=case['rates']; y=case['initial']; end=case['end']
 vals={}
 for method in ['Radau','BDF']:
  out=solve_ivp(lambda t,z:evaluate(t,z,k),(0,end),y,method=method,jac=lambda t,z:jac(t,z,k),rtol=5e-13,atol=1e-14,dense_output=False)
  assert out.success,(case['id'],method,out.message)
  vals[method]=out.y[:,-1].tolist()
  print(case['id'],method,'nfev',out.nfev,'njev',out.njev,'nlu',out.nlu,'y',vals[method])
 cross=max(abs(a-b) for a,b in zip(vals['Radau'],vals['BDF']))
 summary.append({**case,'reference':vals,'reference_cross_difference_linf':cross,'reference_unity_mass_deviation':max(abs(sum(z)-sum(y)) for z in vals.values()),'reference_uncertainty_claim':'diagnostic cross-method discrepancy only; not certified exact IVP bound'})
ROOT.joinpath('results/reference.json').write_text(json.dumps({'schema':'vigilode.c8.scipy_reference.v1','python_library':'scipy','scipy_version':__import__('scipy').__version__,'numpy_version':np.__version__,'rtol':5e-13,'atol':1e-14,'cases':summary},indent=2)+'\n')
