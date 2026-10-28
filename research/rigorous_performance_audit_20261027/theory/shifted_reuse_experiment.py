"""Finite checks of a rigorously derived relaxation rule; no wall-time claim.

This is a research prototype for the native scaled system W(a)=I/a-J.
The exact theorem is conditional on a fixed dissipative J in the stated metric.
Floating point results below are numerical checks, not directed certificates.
"""
import json
from pathlib import Path
import numpy as np
from scipy.linalg import lu_factor, lu_solve


def parameters(a, a0):
    if not np.isfinite(a) or not np.isfinite(a0) or a <= 0 or a0 <= 0:
        raise ValueError("positive finite shifts required")
    # Avoid a+a0 and a/a0 overflow.
    if a <= a0:
        r = a/a0
        return 2*r/(1+r), (1-r)/(1+r)
    r = a0/a
    return 2/(1+r), (1-r)/(1+r)


def iterate(j, a, p_lu, b, omega, max_iter=512, tolerance=1e-10):
    w = np.eye(len(j))/a-j
    x = np.zeros(len(j))
    trace = []
    for k in range(max_iter+1):
        residual = b-w@x
        # J dissipative => ||W(a)^-1||_2 <= a. This is a numerical
        # evaluation of that bound, not an outward-rounding implementation.
        bound = a*np.linalg.norm(residual)
        trace.append(float(bound))
        if bound <= tolerance:
            return dict(converged=True,iterations=k,bound=float(bound),x=x,trace=trace)
        if not np.isfinite(bound) or bound > 1e40:
            break
        x += omega*lu_solve(p_lu,residual)
    return dict(converged=False,iterations=k,bound=float(bound),x=x,trace=trace)


def main():
    rng=np.random.default_rng(20261027)
    rows=[]
    for family in ['damped_nonnormal','skew','diagonal']:
        n=8
        a0=.1
        if family=='damped_nonnormal':
            d=np.geomspace(1,1e6,n)
            s=rng.normal(size=(n,n))*200
            j=-np.diag(d)+(s-s.T)
        elif family=='skew':
            s=rng.normal(size=(n,n))*100
            j=s-s.T
        else:
            j=-np.diag(np.geomspace(1,1e6,n))
        p=np.eye(n)/a0-j
        p_lu=lu_factor(p)
        r0=np.linalg.inv(np.eye(n)-a0*j)
        cayley=2*r0-np.eye(n)
        for ratio in [.1,.25,.5,.9,1.,1.1,2.,4.,10.]:
            a=a0*ratio
            omega,q=parameters(a,a0)
            w=np.eye(n)/a-j
            t=np.eye(n)-omega*lu_solve(p_lu,w)
            identity=t-(a-a0)/(a+a0)*cayley
            t_norm=float(np.linalg.norm(t,2))
            assert t_norm <= q + 3e-12
            assert np.linalg.norm(identity,2) <= 3e-12
            # Manufacture a known solution; compare every resulting state.
            exact=np.ones(n)
            b=w@exact
            relaxed=iterate(j,a,p_lu,b,omega)
            plain=iterate(j,a,p_lu,b,1.)
            error=float(np.linalg.norm(relaxed['x']-exact))
            rows.append(dict(family=family,ratio=ratio,omega=omega,q_bound=q,
              operator_norm=t_norm,identity_residual=float(np.linalg.norm(identity,2)),
              relaxed_iterations=relaxed['iterations'],plain_iterations=plain['iterations'],
              relaxed_converged=relaxed['converged'],relaxed_tail=relaxed['trace'][-5:],
              plain_converged=plain['converged'],relaxed_forward_error=error,
              relaxed_residual_bound=relaxed['bound']))
    # Nonnormal negative eigenvalues alone cannot certify a residual.
    j=np.array([[-1.,1e8],[0.,-1.]])
    b=np.array([0.,1e-8])
    x=np.linalg.solve(np.eye(2)-j,b)
    out=dict(status='NUMERICALLY_CHECKED',runtime='PYTHON_PASS',
      scope='27 finite shifted-system cells; no native ODE integration or speed claim',
      rows=rows,nonnormal_counterexample=dict(residual_norm=float(np.linalg.norm(b)),
      exact_solution=x.tolist(),forward_error=float(np.linalg.norm(x))),
      summary=dict(cells=len(rows),relaxed_converged=sum(r['relaxed_converged'] for r in rows),
        plain_failed=sum(not r['plain_converged'] for r in rows),
        max_identity_residual=max(r['identity_residual'] for r in rows),
        max_forward_error=max(r['relaxed_forward_error'] for r in rows)))
    path=Path(__file__).with_name('SHIFTED_REUSE_RESULTS.json')
    path.write_text(json.dumps(out,indent=2)+'\n')
    print(json.dumps(out['summary'],indent=2))


if __name__=='__main__':main()
