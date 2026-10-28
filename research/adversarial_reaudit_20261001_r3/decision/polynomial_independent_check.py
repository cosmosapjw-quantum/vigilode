#!/usr/bin/env python3
"""Reviewer-selected input, independent scalar Taylor-series reference."""
import importlib.util,sys,json,math
from decimal import Decimal,localcontext
from pathlib import Path
import numpy as np
out=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('candidate',(out.parent/'polynomial/joint_polynomial_probe.py'))
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
A=np.array([[-4.25,3.75],[3.75,-4.25]])
W=np.array([[1.,-.5,.125,-.03125,.015625],[-.25,.75,-.0625,.125,-.0078125]])
h=.75
# A has eigenvalues -.5 and -8, with exact spectral projectors
# Pplus=1/2[[1,1],[1,1]], Pminus=1/2[[1,-1],[-1,1]].
with localcontext() as ctx:
    ctx.prec=160
    phi=[]
    for z in [Decimal('-0.375'),Decimal('-6')]:
        phi.append([sum(z**j/Decimal(math.factorial(j+k)) for j in range(250)) for k in range(5)])
    columns=[]
    for i in range(2):
        row=[]
        for k in range(5):
            v0=Decimal.from_float(float(W[0,k]));v1=Decimal.from_float(float(W[1,k]))
            plus=(v0+v1)/2; minus=(v0-v1)/2
            row.append(plus*phi[0][k]+(minus if i==0 else -minus)*phi[1][k])
        columns.append(row)
    ref=np.array([[float(v) for v in row] for row in columns]); fused=np.array([float(sum(row)) for row in columns])
rows=[]
for kind in ['laguerre','chebyshev']:
    value,meta=m.action(kind,A,W,h,.5,8.)
    err=float(np.linalg.norm(value.sum(axis=1)-fused)); individual=np.linalg.norm(value-ref,axis=0).tolist()
    assert err<1e-10 and max(individual)<1e-10
    rows.append({'algorithm':kind,'error_fused_L2':err,'column_errors':individual,'meta':meta})
result={'status':'PASS_WITHIN_STATED_SCOPE','candidate_source':'polynomial/joint_polynomial_probe.py','reference':'Reviewer-selected 2x2 exact dyadic spectral projectors and independent 250-term Decimal160 Taylor series; no candidate scalar_phi/reference functions reused.','A':A.tolist(),'W':W.tolist(),'h':h,'rows':rows,'proof_review':{'chebyshev_tail':'ACCEPT_DERIVED_FOR_REAL_SYMMETRIC_NONPOSITIVE_A_H_NONNEGATIVE','general_nonnormal':'NOT_SUPPORTED','quadrature_recurrence_roundoff':'NOT_CERTIFIED','wall_speedup':'NOT_MEASURED'}}
(out/'polynomial_independent_check.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
