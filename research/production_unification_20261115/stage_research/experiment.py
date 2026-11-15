#!/usr/bin/env python3
"""Bounded, standalone diagonal-SDC discriminator; not a production backend.

Reproduce: python experiment.py --output RESULTS.json
Dependencies: numpy, sympy. No network or repository mutation except output.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path

import numpy as np
import sympy as sp

BASE_COMMIT = "bee5563c51264a7da19c519295bdda2fb317caef"
Q = np.array([[5 / 12, -1 / 12], [3 / 4, 1 / 4]], dtype=float)
C = np.array([1 / 3, 1.0])
B = np.array([3 / 4, 1 / 4])
QNORM = math.sqrt((4 + math.sqrt(7)) / 18)
LIP = 0.2  # globally valid because |tanh'| <= 1


class SweepBudgetExceeded(RuntimeError):
    """Candidate failed its algebraic-error contract; no fallback is implicit."""


def forcing(t: float, h: float, kappa: float):
    exact = np.exp(-(t + h * C))

    def remainder(u):
        return (kappa - 1) * exact + LIP * (np.tanh(u) - np.tanh(exact))

    return remainder


def residual_bound(t, y, h, kappa, u):
    """Exact-real theorem evaluated in binary64; not interval certification."""
    if h <= 0 or kappa < 0 or not h * QNORM * LIP < 1:
        raise ValueError("requires h>0, kappa>=0, h*||Q||_B*Lip(g)<1")
    rhs = -kappa * u + forcing(t, h, kappa)(u)
    residual = u - y - h * (Q @ rhs)
    return float(2 * np.sqrt(np.sum(B * residual * residual)) / (1 - h * QNORM * LIP))


def step(t, y, h, kappa, mode, max_sweeps=32, local_budget=None):
    if not all(math.isfinite(v) for v in (t, y, h, kappa)):
        raise ValueError("finite inputs required")
    if h <= 0 or kappa < 0 or max_sweeps < 1:
        raise ValueError("h>0, kappa>=0 and max_sweeps>=1 required")
    if mode not in ("fixed_two", "residual_stopped"):
        raise ValueError("unknown mode")
    if mode == "fixed_two" and max_sweeps < 2:
        raise ValueError("fixed_two requires a budget of at least two sweeps")
    if mode == "residual_stopped" and (local_budget is None or not math.isfinite(local_budget) or local_budget <= 0):
        raise ValueError("finite positive local algebraic-error budget required")
    u = np.full(2, y)
    remainder = forcing(t, h, kappa)
    stop = 2 if mode == "fixed_two" else max_sweeps
    for sweep in range(1, stop + 1):
        # Two independent shifted scalar solves; no threading in this oracle.
        d = C / ((sweep - 1) % 2 + 1)
        old = u
        u = (y + h * ((Q - np.diag(d)) @ (-kappa * old) + Q @ remainder(old))) / (1 + h * d * kappa)
        if not np.isfinite(u).all():
            raise SweepBudgetExceeded("nonfinite stage iterate")
        bound = residual_bound(t, y, h, kappa, u)
        if mode == "residual_stopped" and bound <= local_budget:
            return u, sweep, bound
    if mode == "fixed_two":
        return u, stop, bound
    raise SweepBudgetExceeded(f"max_sweeps={max_sweeps}; endpoint_bound={bound:.17g}; budget={local_budget:.17g}")


def collocation_oracle(t, y, h, kappa):
    """Coupled 2x2 Picard reference, distinct from diagonal candidate sweeps."""
    a = np.eye(2) + h * kappa * Q
    g = forcing(t, h, kappa)
    u = np.full(2, y)
    for _ in range(100):
        nxt = np.linalg.solve(a, y + h * (Q @ g(u)))
        if np.max(np.abs(nxt - u)) < 2e-15:
            return nxt
        u = nxt
    raise RuntimeError("reference fixed point failed")


def exact_checks():
    z, w, x = sp.symbols("z w x", real=True)
    q = sp.Matrix([[sp.Rational(5, 12), -sp.Rational(1, 12)], [sp.Rational(3, 4), sp.Rational(1, 4)]])
    b = sp.diag(sp.Rational(3, 4), sp.Rational(1, 4))
    ds = [sp.diag(sp.Rational(1, 3), 1), sp.diag(sp.Rational(1, 6), sp.Rational(1, 2))]
    one = sp.ones(2, 1)
    u = one
    for d in ds:
        u = (sp.eye(2) - z * d).inv() * (one + (z * (q - d) + w * q) * u)
    r = sp.factor(u[1])
    r0 = sp.factor(r.subs(w, 0))
    boundary = sp.factor(1 - r0.subs(z, sp.I * x) * r0.subs(z, -sp.I * x))
    expected = x**4 * (10 + x**2) / ((1 + x**2) * (4 + x**2) * (9 + x**2))
    assert sp.simplify(boundary - expected) == 0
    assert sp.limit(r0, z, sp.oo) == 0
    assert sp.series(r0, z, 0, 3).removeO() == 1 + z + z**2 / 2
    assert (sp.eye(2)-ds[1].inv()*q)*(sp.eye(2)-ds[0].inv()*q) == sp.zeros(2)
    assert b*q+q.T*b == sp.Matrix([[sp.Rational(5, 8),sp.Rational(1, 8)],[sp.Rational(1, 8),sp.Rational(1, 8)]])
    assert sp.expand((1-z)*(1+z)*(1+z**2)*(1+z**4)) == 1-z**8
    return {"status":"exact_symbolic_identities_checked", "linear_stability_function":str(r0),
            "imex_stability_function":str(r), "imaginary_axis_deficit":str(boundary),
            "origin_series":str(sp.series(r0,z,0,5)),
            "weighted_q_norm_squared":"(4+sqrt(7))/18", "scalar_linear_L_stable":True,
            "full_nonlinear_L_stability_claim":False}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--output",type=Path,default=Path(__file__).with_name("RESULTS.json"))
    args = ap.parse_args()
    rows=[]
    bound_rows=[]
    for kappa in (1.0,1000.0):
        for n in (20,40,80,160,320):
            h=1/n
            for mode in ("fixed_two","residual_stopped"):
                y=1.0; sweeps=[];bounds=[]
                for i in range(n):
                    u,k,bd=step(i*h,y,h,kappa,mode,local_budget=1e-9*h)
                    y=float(u[-1]);sweeps.append(k);bounds.append(bd)
                rows.append({"kappa":kappa,"steps":n,"h":h,"mode":mode,"endpoint":y,
                             "endpoint_abs_error":abs(y-math.exp(-1)),"mean_sweeps":float(np.mean(sweeps)),
                             "max_sweeps":max(sweeps),"parallel_node_solve_depth":sum(sweeps),
                             "total_scalar_shifted_solves":2*sum(sweeps),
                             "max_endpoint_algebraic_bound":max(bounds),
                             "local_algebraic_budget":1e-9*h if mode=="residual_stopped" else None})
            oracle=collocation_oracle(0.,1.,h,kappa)
            u,k,bd=step(0.,1.,h,kappa,"residual_stopped",local_budget=1e-9*h)
            observed=abs(float(u[-1]-oracle[-1]))
            assert observed <= bd + 5e-14
            bound_rows.append({"kappa":kappa,"h":h,"bound":bd,"observed_endpoint_error_to_collocation":observed,
                               "comparison_slack":5e-14,"sweeps":k})
    failed=False
    try:
        step(0.,1.,.05,1000.,"residual_stopped",max_sweeps=2,local_budget=5e-11)
    except SweepBudgetExceeded as exc:
        failed=True;failure_message=str(exc)
    assert failed
    invalid_budget_rejections=0
    for budget in (float("nan"),float("inf"),0.,-1.):
        try:
            step(0.,1.,.05,1000.,"residual_stopped",local_budget=budget)
        except ValueError:
            invalid_budget_rejections+=1
    assert invalid_budget_rejections==4
    try:
        step(0.,1.,.05,1000.,"fixed_two",max_sweeps=1)
    except ValueError:
        invalid_budget_rejections+=1
    assert invalid_budget_rejections==5
    # A stable spectrum alone does not imply the theorem's dissipativity.
    ln=np.array([[-1.,100.],[0.,-1.]])
    a=np.eye(4)-np.kron(Q,ln)
    weights=np.sqrt(np.repeat(B,2))
    inv=np.linalg.inv(a)
    nonnormal_gain=float(np.linalg.norm(weights[:,None]*inv/weights[None,:],2))
    assert nonnormal_gain>1
    outcome={"schema":"vigilode.parallel-stage-discriminator.v1","base_commit":BASE_COMMIT,
             "source_script_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
             "runtime":{"numpy":np.__version__,"sympy":sp.__version__},
             "exact_checks":exact_checks(),"full_ivp_rows":rows,"residual_bound_checks":bound_rows,
             "max_sweep_failure":{"observed":failed,"type":"SweepBudgetExceeded","message":failure_message},
             "invalid_budget_rejections":invalid_budget_rejections,
             "nonnormal_negative_control":{"L":[[-1.,100.],[0.,-1.]],"weighted_inverse_gain":nonnormal_gain,
                                           "conclusion":"negative real eigenvalues alone do not license beta=1"},
             "verdict":"RESEARCH_ONLY_FIXED_TWO_FAILS_STRONG_STIFF_ACCURACY_RESIDUAL_STOP_RECOVERS_ACCURACY_WITH_HIGHER_DEPTH",
             "not_claimed":["runtime speedup","threaded implementation","production admissibility","novel method",
                            "rigorous floating-point enclosure","uniform stiff order","original Rodas5P equivalence"],
             "causality":{"macro_step":True,"strict_internal_time_order":False}}
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(outcome,indent=2,allow_nan=False)+"\n")
    print(json.dumps({"output":str(args.output),"rows":len(rows),"verdict":outcome["verdict"],
                      "nonnormal_gain":nonnormal_gain}))


if __name__=="__main__":
    main()
