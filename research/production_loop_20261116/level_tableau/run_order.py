#!/usr/bin/env python3
"""Loop L1 node V16 -- fixed-step order check (PREDECLARED.json, key order_check) and the exploratory
Prothero-Robinson order.

    python3 -I run_order.py RODAS5P                     > logs/order_RODAS5P.jsonl
    python3 -I run_order.py candidates/D6T_r01.json     > logs/order_D6T_r01.jsonl

Emits one JSON line per (problem, number of steps). The observed order is computed by parse_results.py from
these lines; the summary line printed here is a convenience only.
"""
import json
import math
import os
import sys

os.environ.setdefault("OMP_NUM_THREADS", "1")
os.environ.setdefault("OPENBLAS_NUM_THREADS", "1")
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import mpmath  # noqa: E402
import numpy as np  # noqa: E402
import scipy  # noqa: E402
from mpmath import mp  # noqa: E402
from scipy.integrate import solve_ivp  # noqa: E402

import problems  # noqa: E402
import proto  # noqa: E402

ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
SNAPSHOT = os.path.join(ROOT, "fixtures", "rodas5p_coefficients_snapshot.json")
STEPS = [10, 20, 40, 80, 160, 320]
FLOOR = 1e-12      # amendment A-2 (was 1e-11)


def emit(**kw):
    print(json.dumps(kw, allow_nan=False), flush=True)


def load_arm(spec):
    if spec == "RODAS5P":
        return proto.rodas5p(SNAPSHOT)
    return proto.from_file(spec)


def reference(prob):
    mp.dps = 40
    F = prob.mp_rhs(mp)
    n = prob.nstate
    sol = mp.odefun(F, 0, [mp.mpf(repr(float(v))) for v in prob.y0[:n]])
    yref = [sol(mp.mpf(repr(prob.t_end)))[i] for i in range(n)]
    f64 = lambda t, y: [float(v) for v in F(t, [mp.mpf(float(z)) for z in y])]   # noqa: E731
    chk = solve_ivp(f64, [0.0, prob.t_end], [float(v) for v in prob.y0[:n]], method="DOP853", rtol=1e-13,
                    atol=1e-15)
    unc = max(abs(float(yref[i]) - chk.y[i, -1]) / max(1.0, abs(float(yref[i]))) for i in range(n))
    return [float(v) for v in yref], [mp.nstr(v, 30) for v in yref], unc


def err_measure(y, yref):
    return max(abs(y[i] - yref[i]) / max(1.0, abs(yref[i])) for i in range(len(yref)))


def main():
    arm_spec = sys.argv[1]
    tab = load_arm(arm_spec)
    emit(kind="header", script="run_order.py", arm=tab.name, arm_spec=arm_spec, stages=tab.s, levels=tab.levels,
         steps=STEPS, floor=FLOOR, numpy=np.__version__, scipy=scipy.__version__, mpmath=mpmath.__version__)
    for pid, ctor in problems.ORDER_PROBLEMS.items():
        prob = ctor()
        yref, yref_s, unc = reference(prob)
        emit(kind="order_reference", problem=pid, t_end=prob.t_end, y_ref=yref_s,
             reference="mpmath.odefun, 40 digits", uncertainty_vs_dop853=unc)
        errs = []
        for n in STEPS:
            y = proto.fixed(tab, prob, n)
            e = err_measure(y, yref) if np.all(np.isfinite(y)) else None
            errs.append(e)
            emit(kind="order_point", arm=tab.name, problem=pid, steps=n, h=prob.t_end / n, error=e)
        pairs = [math.log2(errs[i] / errs[i + 1]) for i in range(len(errs) - 1)
                 if errs[i] and errs[i + 1] and errs[i] >= FLOOR and errs[i + 1] >= FLOOR]
        emit(kind="order_summary_convenience", arm=tab.name, problem=pid, pair_orders=pairs,
             median=(float(np.median(pairs)) if pairs else None))
    # exploratory: Prothero-Robinson, lambda = -1e6, exact solution known
    prob = problems.prothero_robinson()
    errs = []
    for n in STEPS:
        y = proto.fixed(tab, prob, n)
        e = abs(y[0] - prob.exact(prob.t_end)) if np.all(np.isfinite(y)) else None
        errs.append(e)
        emit(kind="pr_point", arm=tab.name, problem=prob.id, lam=-1e6, steps=n, h=prob.t_end / n, error=e,
             label="exploratory")
    emit(kind="pr_summary_convenience", arm=tab.name,
         pair_orders=[(math.log2(errs[i] / errs[i + 1]) if errs[i] and errs[i + 1] else None)
                      for i in range(len(errs) - 1)], label="exploratory")


if __name__ == "__main__":
    main()
