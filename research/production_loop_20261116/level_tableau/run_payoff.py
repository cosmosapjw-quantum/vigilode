#!/usr/bin/env python3
"""Loop L1 node V16 -- adaptive runs for the development screen and the pay-off screen
(PREDECLARED.json, keys payoff_screen, candidate_selection, prototype_integrator).

    python3 -I run_payoff.py --label dev    --problems HIRES,OREGO         --arms RODAS5P candidates/X.json ...
    python3 -I run_payoff.py --label screen --problems ROBER,VDPOL,BURGERS --arms RODAS5P candidates/X.json ...

Emits JSON lines: one reference line per problem and one run line per (arm, problem, tolerance). Ratios at
matched error are computed by parse_results.py from these lines only.
"""
import argparse
import json
import os
import sys
import time

os.environ.setdefault("OMP_NUM_THREADS", "1")
os.environ.setdefault("OPENBLAS_NUM_THREADS", "1")
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import numpy as np  # noqa: E402
import scipy  # noqa: E402
from scipy.integrate import solve_ivp  # noqa: E402

import problems  # noqa: E402
import proto  # noqa: E402

ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
SNAPSHOT = os.path.join(ROOT, "fixtures", "rodas5p_coefficients_snapshot.json")
LADDER_K = list(range(6, 19))          # rtol = 10^(-k/2)
H0 = 1e-6
MAX_ATTEMPTS = 500000


def emit(**kw):
    print(json.dumps(kw, allow_nan=False), flush=True)


def measure(y, yref, a_scale):
    return float(np.max(np.abs(y - yref) / (a_scale + np.abs(yref))))


def scipy_ref(prob, method, rtol):
    sol = solve_ivp(lambda t, y: prob.f(y), [0.0, prob.t_end], prob.y0, method=method, rtol=rtol,
                    atol=rtol * prob.a_scale, jac=(lambda t, y: prob.jac(y)))
    return (sol.y[:, -1].copy() if sol.success else None), int(sol.nfev)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--label", required=True)
    ap.add_argument("--problems", required=True)
    ap.add_argument("--arms", nargs="+", required=True)
    a = ap.parse_args()
    rod = proto.rodas5p(SNAPSHOT)
    arms = [(spec, rod if spec == "RODAS5P" else proto.from_file(os.path.join(HERE, spec))) for spec in a.arms]
    emit(kind="header", script="run_payoff.py", label=a.label, problems=a.problems.split(","),
         arms=[t.name for _, t in arms], arm_specs=a.arms, ladder_rtol=[10.0 ** (-k / 2) for k in LADDER_K],
         h0=H0, max_attempts=MAX_ATTEMPTS, numpy=np.__version__, scipy=scipy.__version__,
         controller={"safety": proto.SAFETY, "min_factor": proto.MIN_FACTOR, "max_factor": proto.MAX_FACTOR,
                     "reject_max_factor": proto.REJECT_MAX_FACTOR, "estimator_order": proto.ESTIMATOR_ORDER})
    for pid in a.problems.split(","):
        prob = problems.STIFF_PROBLEMS[pid]()
        t0 = time.time()
        yref, nfev = scipy_ref(prob, "Radau", 1e-13)
        y11, _ = scipy_ref(prob, "Radau", 1e-11)
        ylsoda, _ = scipy_ref(prob, "LSODA", 1e-12)
        r11 = proto.adaptive(rod, prob, 1e-11, 1e-11 * prob.a_scale, H0, MAX_ATTEMPTS)
        d_radau = measure(y11, yref, prob.a_scale)
        d_proto = measure(r11["y"], yref, prob.a_scale) if r11["finished"] else None
        d_lsoda = measure(ylsoda, yref, prob.a_scale) if ylsoda is not None else None
        u_ref = max(d_radau, d_proto) if d_proto is not None else None
        emit(kind="reference", label=a.label, problem=pid, n=int(prob.y0.size), t_end=prob.t_end,
             a_scale=prob.a_scale, method="scipy Radau, rtol 1e-13, atol 1e-13 * a_scale, exact Jacobian",
             y_ref=[repr(float(v)) for v in yref], radau_nfev=nfev, diff_radau_rtol_1em11=d_radau,
             diff_rodas5p_prototype_rtol_1em11=d_proto, diff_lsoda_rtol_1em12_reported_only=d_lsoda,
             u_ref=u_ref, seconds=time.time() - t0)
        for spec, tab in arms:
            for k in LADDER_K:
                rtol = 10.0 ** (-k / 2)
                atol = rtol * prob.a_scale
                t1 = time.time()
                res = proto.adaptive(tab, prob, rtol, atol, H0, MAX_ATTEMPTS)
                row = {"kind": "run", "label": a.label, "problem": pid, "arm": tab.name, "arm_spec": spec,
                       "stages": tab.s, "k": k, "rtol": rtol, "atol": atol, "finished": bool(res["finished"]),
                       "accepted": res["accepted"], "rejected": res["rejected"],
                       "attempts": res["accepted"] + res["rejected"], "seconds_diagnostic_only": time.time() - t1}
                if res["finished"]:
                    row["error"] = measure(res["y"], yref, prob.a_scale)
                else:
                    row["reason"] = res.get("reason")
                emit(**row)
    emit(kind="footer")


if __name__ == "__main__":
    main()
