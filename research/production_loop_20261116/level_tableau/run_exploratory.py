#!/usr/bin/env python3
"""Loop L1 node V16 -- exploratory numbers for candidates and for RODAS5P (nothing here enters a verdict).

    python3 -I run_exploratory.py RODAS5P candidates/D6L_r01.json ... > logs/exploratory.jsonl

Per method: order-6 error coefficients, estimator coefficients, index-1 and Prothero-Robinson residuals, Taylor
defects of the stability function, the scalar linear bound (largest |z|, z < 0 real, with |R(z) - exp(z)| <= eps),
code-form magnitudes, two-processor schedule. Also the layout-level linear bound from the stability function
alone (depth 5 at the pinned gamma, depth 6 over a gamma grid).
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from mpmath import mp, mpf  # noqa: E402

import rowtrees as rt  # noqa: E402

mp.dps = 50
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
SNAPSHOT = os.path.join(ROOT, "fixtures", "rodas5p_coefficients_snapshot.json")
EPS_LIST = [10.0 ** (-k) for k in range(3, 11)]


def emit(**kw):
    kw["label"] = "exploratory"
    print(json.dumps(kw, allow_nan=False), flush=True)


def load(spec):
    if spec == "RODAS5P":
        t = rt.load_rodas5p(SNAPSHOT, mpf)
        t["name"] = "RODAS5P"
        t["levels"] = [[i] for i in range(1, 9)]
        return t
    d = json.load(open(os.path.join(HERE, spec)))
    al = [[mpf(x) for x in r] for r in d["alpha"]]
    gm = [[mpf(x) for x in r] for r in d["Gamma"]]
    s = d["stages"]
    return {"name": d["id"], "s": s, "gamma": mpf(d["gamma"]), "alpha": al, "Gamma": gm,
            "beta": [[al[i][j] + gm[i][j] for j in range(s)] for i in range(s)], "b": [mpf(x) for x in d["b"]],
            "bhat": [mpf(x) for x in d["bhat"]], "levels": d["levels"]}


def stab(beta, w, z):
    """R(z) = 1 + z w^T (I - z beta)^-1 1 for a lower-triangular beta."""
    x = []
    for i in range(len(w)):
        acc = mpf(1)
        for j in range(i):
            acc += z * beta[i][j] * x[j]
        x.append(acc / (1 - z * beta[i][i]))
    return 1 + z * mp.fsum(w[i] * x[i] for i in range(len(w)))


def design_stab(gamma, L, z):
    """trunc_{L-1}[exp(z)(1 - gamma z)^L] / (1 - gamma z)^L: the stability function of a stiffly accurate
    depth-L tableau of linear order >= L - 1 (order 5 needs L = 6, or L = 5 at the pinned gamma)."""
    coef = [mp.fsum(mp.binomial(L, k) * (-gamma) ** k / mp.factorial(j - k) for k in range(0, j + 1))
            for j in range(L)]
    return mp.fsum(c * z ** j for j, c in enumerate(coef)) / (1 - gamma * z) ** L


def linear_bound(fun):
    """For each eps: the largest |z| (z < 0) reachable before |R(z) - exp(z)| first exceeds eps."""
    out = []
    for eps in EPS_LIST:
        lo, hi = mpf("1e-4"), None
        a = lo
        while a < 1e4:
            if abs(fun(-a) - mp.exp(-a)) > eps:
                hi = a
                break
            lo = a
            a *= mpf("1.05")
        if hi is None:
            out.append(None)
            continue
        for _ in range(60):
            mid = (lo + hi) / 2
            if abs(fun(-mid) - mp.exp(-mid)) > eps:
                hi = mid
            else:
                lo = mid
        out.append(float(lo))
    return out


def main():
    specs = sys.argv[1:]
    emit(kind="header", script="run_exploratory.py", mp_dps=mp.dps, eps=EPS_LIST)
    trees5 = rt.ode_trees(5)
    trees6 = [t for t in rt.ode_trees(6) if rt.order(t) == 6]
    sel = json.load(open(os.path.join(HERE, "index1_selected.json")))
    ys, zs = rt.index1_trees(5)
    table = {rt.show(t): t for t in ys + zs}
    for spec in specs:
        t = load(spec)
        s = t["s"]
        L = len(t["levels"])
        ops = rt.ListOps(t["alpha"], t["beta"])
        ev = rt.Evaluator(ops)
        e6 = [ev.residual(t["b"], tr) / rt.symmetry(tr) for tr in trees6]
        eh5 = [ev.residual(t["bhat"], tr) / rt.symmetry(tr) for tr in trees5 if rt.order(tr) == 5]
        dm = [ev.residual(t["b"], table[x]) for x in sel["main"]]
        dh = [ev.residual(t["bhat"], table[x]) for x in sel["embedded"]]
        c = [mp.fsum(r) for r in t["alpha"]]
        pr = rt.pr_terms(ops, t["b"], c, L, (2, 3, 4))
        prh = rt.pr_terms(ops, t["bhat"], c, L, (2, 3))
        nrm = lambda v: float(mp.sqrt(mp.fsum(x * x for x in v)))   # noqa: E731
        emit(kind="error_terms", method=t["name"], stages=s, depth=L, e6_norm=nrm(e6),
             e6_max=float(max(abs(x) for x in e6)), e6=[float(x) for x in e6], e6_trees=[rt.show(x) for x in trees6],
             ehat5_norm=nrm(eh5), ehat5=[float(x) for x in eh5], index1_main_norm=nrm(dm),
             index1_main_max=float(max(abs(x) for x in dm)), index1_embedded_norm=nrm(dh),
             pr_main_norm=nrm(pr), pr_main_by_q={str(q): nrm(pr[i * L:(i + 1) * L]) for i, q in enumerate((2, 3, 4))},
             pr_embedded_norm=nrm(prh), c=[float(x) for x in c],
             c_min=float(min(c)), c_max=float(max(c)))
        # stability function
        v = [mpf(1)] * s
        defects = []
        for k in range(1, 9):
            defects.append(float(mp.fsum(x * y for x, y in zip(t["b"], v)) - 1 / mp.factorial(k)))
            v = ops.mv(t["beta"], v)
        lb = linear_bound(lambda z: stab(t["beta"], t["b"], z))
        emit(kind="stability", method=t["name"], gamma=float(t["gamma"]), taylor_defects_z1_to_z8=defects,
             linear_reach_abs_z=lb, R_hat_infinity=float(stab(t["beta"], t["bhat"], mpf("-1e40"))))
        # code form
        import numpy as np
        gam = np.array([[float(x) for x in r] for r in t["Gamma"]])
        alf = np.array([[float(x) for x in r] for r in t["alpha"]])
        ginv = np.linalg.inv(gam)
        A = alf @ ginv
        C = np.eye(s) / float(t["gamma"]) - ginv
        emit(kind="code_form", method=t["name"], max_abs_A=float(np.abs(np.tril(A, -1)).max()),
             max_abs_C=float(np.abs(np.tril(C, -1)).max()),
             max_abs_kform=float(max(np.abs(alf).max(), np.abs(gam).max())))
        emit(kind="schedule", method=t["name"], levels=t["levels"], depth=L, stages=s,
             processors=max(len(lv) for lv in t["levels"]),
             schedule=[{"round": i + 1, "processor_1": lv[0], "processor_2": (lv[1] if len(lv) > 1 else None)}
                       for i, lv in enumerate(t["levels"])],
             depth_gain_vs_8=8.0 / L, total_solves_vs_8=s / 8.0)
    # layout-level linear bound from the stability function alone
    rod = load("RODAS5P")
    base = linear_bound(lambda z: stab(rod["beta"], rod["b"], z))
    emit(kind="linear_bound_reference", method="RODAS5P", linear_reach_abs_z=base)
    g5 = mp.findroot(lambda g: 120 * g ** 5 - 600 * g ** 4 + 600 * g ** 3 - 200 * g ** 2 + 25 * g - 1, mpf("0.278"))
    g6 = mp.findroot(lambda g: 720 * g ** 6 - 4320 * g ** 5 + 5400 * g ** 4 - 2400 * g ** 3 + 450 * g ** 2 - 36 * g + 1,
                     mpf("0.33414237"))
    cases = [(5, g5, "depth 5, pinned gamma")] + [(6, mpf(x), "depth 6") for x in
                                                   ("0.19", "0.22", "0.25", "0.28", "0.30", "0.32", "0.33")]
    cases.append((6, g6, "depth 6, linear order 6"))
    for L, g, note in cases:
        reach = linear_bound(lambda z: design_stab(g, L, z))
        ratio = [(base[i] / reach[i] if base[i] and reach[i] else None) for i in range(len(EPS_LIST))]
        emit(kind="linear_bound_layout", depth=L, gamma=float(g), note=note, linear_reach_abs_z=reach,
             step_ratio_vs_rodas5p=ratio, depth_gain=8.0 / L,
             modelled_chain_gain=[(8.0 / L / r if r else None) for r in ratio])


if __name__ == "__main__":
    main()
