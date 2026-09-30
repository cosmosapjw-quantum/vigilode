"""Exact-model/float-evaluation toy for physical defect under similarity.

This is NOT the repository's zeta34 rule or a production error certificate.
"""
from pathlib import Path
import json
import numpy as np
from scipy.linalg import expm
from scipy.integrate import quad_vec

M=np.array([[-1.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.],[0.,0.,0.,0.]])
q=np.array([0.,0.,0.,1.]); P=np.array([[1.,0.,0.,0.]])
expected=.5-np.exp(-1.)
rows=[]
for alpha in [.25,1.,4.,16.,64.]:
    S=np.diag([1.,alpha**2,alpha,1.]); Sinv=np.diag(1/np.diag(S))
    Mh=Sinv@M@S; qh=Sinv@q; Ph=P@S
    def residual(u): return -u*(Mh@Mh@qh)
    approx=qh+Mh@qh
    error=float((Ph@(expm(Mh)@qh-approx))[0])
    integ,quadrature_indicator=quad_vec(lambda u:-(Ph@expm((1-u)*Mh)@residual(u)),0.,1.,epsabs=1e-13,epsrel=1e-13)
    norm=float(np.linalg.norm(residual(1.)))
    rows.append(dict(alpha=alpha,auxiliary_diagonal=list(np.diag(S)[1:]),
        residual_norm_at_1=norm,physical_error=error,projected_defect_integral=float(integ[0]),
        quadrature_indicator=float(quadrature_indicator),
        illustrative_residual_gate_below_half=norm<.5,
        actual_physical_error_below_point_one=abs(error)<.1))
    assert abs(error-expected)<1e-13
    assert abs(float(integ[0])-expected)<1e-13
    assert abs(norm-alpha**-2)<1e-13
result=dict(schema='vigilode.telemetry_covariance_toy.v1',status='numerically checked',
    source_dependency='independent explicit 4x4 block model; not VigilODE zeta34 implementation',
    analytic_reference='1/2-exp(-1)',expected_physical_error=float(expected),
    conclusion='physical projected error invariant; auxiliary-coordinate Euclidean residual gate changes',
    production_claim=False,rows=rows)
Path(__file__).with_name('telemetry_covariance_results.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
