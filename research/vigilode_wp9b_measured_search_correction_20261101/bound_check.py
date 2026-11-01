#!/usr/bin/env python3
"""Evaluate source-specific forced weighted L1 stability on actual WP9B trajectories.

Simpson quadrature gives nonrigorous numerical evidence for an exact analytic
inequality. Not an interval certificate, not a new solver simulation.
"""
from __future__ import annotations

import hashlib
import importlib.util
import json
from collections import defaultdict
from pathlib import Path

import numpy as np
from scipy.integrate import simpson

HERE = Path(__file__).resolve().parent
SPEC=importlib.util.spec_from_file_location("vigilode_wp9b_impl", HERE/"experiment.py")
m=importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)


def analyze():
    m.verify_locks()
    cases=m.source_cases()
    buckets=defaultdict(dict)
    for line in (HERE/"TRAJECTORIES.jsonl").read_text().splitlines():
        rec=json.loads(line)
        # Every registered numerical pair has both methods; use Radau here and
        # compare DOP853 identities already checked in source simulation.
        if rec["method"] != "Radau":
            continue
        cohort=m.canonical_candidate((rec["cohort"],rec["axis"]))[0]
        key=(cohort,rec["axis"],rec["h"],rec["horizon"])
        assert rec["theta"] not in buckets[key]
        buckets[key][rec["theta"]]=rec
    witnesses=[]
    for (cohort,axis,h,end),by_theta in sorted(buckets.items()):
        assert set(by_theta)=={0.0,+h,-h},(cohort,axis,h)
        p=cases[cohort]
        baseline=by_theta[0.0]
        t=np.asarray(baseline["time"],dtype=float)
        y=np.asarray(baseline["state"],dtype=float)
        _,_,_,_,q0,Q0=m.matrices(p,axis,0.0)
        for theta in (+h,-h):
            rec=by_theta[theta]
            assert np.array_equal(t,np.asarray(rec["time"],dtype=float))
            ktheta,gtheta,ctheta,y0,qtheta,Qtheta=m.matrices(p,axis,theta)
            k0,g0,c0,y0_,q00,Q00=m.matrices(p,axis,0.0)
            pi=m.stationary(Qtheta)
            pi0=m.stationary(Q0)
            assert np.all(pi>0) and np.all(pi0>0)
            # Source-exact f_theta(y0)-f_0(y0) evaluated on sampled
            # unperturbed trajectory, without freezing the new stationary pi.
            forcing=(y @ (Qtheta-Q0).T -
                     (ktheta-k0)[None,:]*y -
                     (gtheta-g0)[None,:]*y**3 +
                     (qtheta-q00)[None,:])
            integrand=(np.abs(forcing) @ pi) * np.exp(
                -np.min(ktheta)*(end-t))
            numerical_convolution=float(simpson(integrand,x=t))
            ytheta=np.asarray(rec["state"],dtype=float)
            discrepancy=float(np.sum(pi*np.abs(ytheta[-1]-y[-1])))
            weight_shift=float(np.sum(np.abs(pi-pi0)*np.abs(y[-1])))
            Wgap=float(abs(pi@ytheta[-1]-pi0@y[-1]))
            # Simpson quadrature is not rigorously an upper bound. A relative
            # check tolerates modest sampling errors; report the true ratio.
            eps=1e-11+1e-4*numerical_convolution
            assert discrepancy<=numerical_convolution+eps,(
                cohort,axis,theta,discrepancy,numerical_convolution)
            assert Wgap<=numerical_convolution+weight_shift+eps
            witnesses.append({
                "cohort":cohort,"axis":axis,"theta":theta,"horizon":end,
                "weighted_state_difference":discrepancy,
                "sampled_simpson_forcing_convolution":numerical_convolution,
                "weighted_stationary_change":weight_shift,
                "observed_W_difference":Wgap,
                "finite_grid_delta_over_integral":
                    discrepancy/numerical_convolution if numerical_convolution>0 else 0,
                "finite_grid_numerical_check":True,
                "rigorous_interval_upper":False,
            })
    assert len(witnesses)==2*len(buckets)
    result={
        "schema":"vigilode.wp9b.source_forced_weighted_l1_witness.v1",
        "witness_cases":len(witnesses),
        "registered_physical_experiments":len(buckets),
        "max_ratio":max(x["finite_grid_delta_over_integral"] for x in witnesses),
        "source_blobs":m.SOURCE_LOCKS,
        "theorem":"DERIVED_ANALYTIC",
        "simpson_discrete_check":"NUMERICALLY_CHECKED_NONRIGOROUS",
        "formal_interval_certificate":"NOT_RUN",
        "official_vigilode_dag":"7/23",
        "production":"HOLD",
        "witnesses":witnesses,
    }
    path=HERE/"CONTRACTIVITY_WITNESSES.json"
    path.write_text(json.dumps(result,sort_keys=True,indent=2,allow_nan=False)+"\n")
    m.verify_locks()
    print("WP9B_WEIGHTED_STABILITY_NUMERIC_CHECK",len(witnesses),
          "MAX_RATIO",result["max_ratio"],
          "SHA256",hashlib.sha256(path.read_bytes()).hexdigest())
    return result


if __name__=="__main__":
    analyze()
