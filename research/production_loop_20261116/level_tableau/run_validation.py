#!/usr/bin/env python3
"""Loop L1 node V16 -- tool validation of the order-condition generator on the protected RODAS5P tableau.

    python3 -I run_validation.py > logs/validation.jsonl

Emits JSON lines. Frozen check: the 17 generated conditions of order <= 5 vanish on RODAS5P to <= 1e-12 in
binary64. Also: the index-1 extension on RODAS5P (selection of the soft targets, written to
index1_selected.json), Prothero-Robinson terms, error coefficients, and the two facts about the depth-5 layout.
Exit code 0 iff the frozen check passes.
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import mpmath  # noqa: E402
import numpy as np  # noqa: E402
from mpmath import mp, mpf  # noqa: E402

import rowtrees as rt  # noqa: E402

mp.dps = 60
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
SNAPSHOT = os.path.join(ROOT, "fixtures", "rodas5p_coefficients_snapshot.json")
SELECT_TOL = 1e-10
FROZEN_TOL = 1e-12


def emit(**kw):
    print(json.dumps(kw, allow_nan=False), flush=True)


def f(x):
    return float(x)


def main():
    emit(kind="header", script="run_validation.py", mp_dps=mp.dps, numpy=np.__version__, mpmath=mpmath.__version__,
         snapshot=os.path.relpath(SNAPSHOT, ROOT))
    tm = rt.load_rodas5p(SNAPSHOT, mpf)
    tf = rt.load_rodas5p(SNAPSHOT, float)
    s = tm["s"]

    # ---- mp evaluator and float64 evaluator
    em = rt.Evaluator(rt.ListOps(tm["alpha"], tm["beta"]))
    al = np.array(tf["alpha"], dtype=float)[None]
    be = np.array(tf["beta"], dtype=float)[None]
    ef = rt.Evaluator(rt.NumpyOps(al, be, np))
    bf = np.array(tf["b"], dtype=float)[None]
    bhf = np.array(tf["bhat"], dtype=float)[None]

    trees5 = rt.ode_trees(5)
    counts = {}
    for t in trees5:
        counts[rt.order(t)] = counts.get(rt.order(t), 0) + 1
    emit(kind="tree_counts", counts=[counts[p] for p in range(1, 6)], total=len(trees5),
         order6=len(rt.ode_trees(6)) - len(trees5))

    worst64 = 0.0
    worstmp = 0.0
    for t in trees5:
        r64 = float(ef.residual(bf, t)[0])
        rmp = em.residual(tm["b"], t)
        worst64 = max(worst64, abs(r64))
        worstmp = max(worstmp, abs(f(rmp)))
        emit(kind="ode_condition", weights="b", tree=rt.show(t), order=rt.order(t), density=rt.density(t),
             symmetry=rt.symmetry(t), residual_binary64=r64, residual_mp=f(rmp))
    ok = worst64 <= FROZEN_TOL
    emit(kind="frozen_tool_validation", conditions=len(trees5), max_abs_residual_binary64=worst64,
         max_abs_residual_mp60=worstmp, threshold=FROZEN_TOL, passed=bool(ok))

    # ---- embedded weights
    w4 = 0.0
    for t in trees5:
        rmp = em.residual(tm["bhat"], t)
        r64 = float(ef.residual(bhf, t)[0])
        if rt.order(t) <= 4:
            w4 = max(w4, abs(r64))
        emit(kind="ode_condition", weights="bhat", tree=rt.show(t), order=rt.order(t), density=rt.density(t),
             symmetry=rt.symmetry(t), residual_binary64=r64, residual_mp=f(rmp))
    emit(kind="embedded_order4", max_abs_residual_binary64=w4)

    # ---- W condition
    gi = [sum(row) for row in tm["Gamma"]]
    ci = [sum(row) for row in tm["alpha"]]
    emit(kind="w_condition", sum_b_gamma_i=f(sum(x * y for x, y in zip(tm["b"], gi))),
         sum_bhat_gamma_i=f(sum(x * y for x, y in zip(tm["bhat"], gi))), c=[f(x) for x in ci])

    # ---- error coefficients of order 6 (main) and order 5 (embedded)
    trees6 = [t for t in rt.ode_trees(6) if rt.order(t) == 6]
    e6 = [em.residual(tm["b"], t) / rt.symmetry(t) for t in trees6]
    e5h = [em.residual(tm["bhat"], t) / rt.symmetry(t) for t in trees5 if rt.order(t) == 5]
    emit(kind="error_coefficients", method="RODAS5P", e6_norm=f(mp.sqrt(sum(x * x for x in e6))),
         e6_max=f(max(abs(x) for x in e6)), e6=[f(x) for x in e6], e6_trees=[rt.show(t) for t in trees6],
         ehat5_norm=f(mp.sqrt(sum(x * x for x in e5h))), ehat5=[f(x) for x in e5h])

    # ---- index-1 extension
    ys, zs = rt.index1_trees(5)
    sel_main, sel_emb = [], []
    n_all = 0
    for root, lst in (("y", ys), ("z", zs)):
        for t in lst:
            n_all += 1
            rb = f(em.residual(tm["b"], t))
            rh = f(em.residual(tm["bhat"], t))
            use_b = abs(rb) <= SELECT_TOL
            use_h = rt.order(t) <= 4 and abs(rh) <= SELECT_TOL
            if use_b:
                sel_main.append(rt.show(t))
            if use_h:
                sel_emb.append(rt.show(t))
            emit(kind="index1_condition", root=root, tree=rt.show(t), order=rt.order(t), density=rt.density(t),
                 residual_b=rb, residual_bhat=rh, selected_main=bool(use_b), selected_embedded=bool(use_h))
    by_order = {}
    for root, lst in (("y", ys), ("z", zs)):
        for t in lst:
            k = "%s%d" % (root, rt.order(t))
            by_order[k] = by_order.get(k, 0) + 1
    emit(kind="index1_summary", generated=n_all, by_root_and_order=by_order, selected_main=len(sel_main),
         selected_embedded=len(sel_emb), select_tol=SELECT_TOL)
    with open(os.path.join(HERE, "index1_selected.json"), "w") as fh:
        json.dump({"schema": "v16-index1-selected-v1", "rule": "generated two-coloured trees with |residual| <= 1e-10 "
                   "on the RODAS5P main weights (order <= 5) and embedded weights (order <= 4)",
                   "main": sel_main, "embedded": sel_emb}, fh, indent=1, allow_nan=False)
        fh.write("\n")

    # ---- Prothero-Robinson terms of RODAS5P (L = 8 for the sequential chain)
    ops = rt.ListOps(tm["alpha"], tm["beta"])
    for name, w, qs in (("b", tm["b"], (2, 3, 4)), ("bhat", tm["bhat"], (2, 3))):
        terms = rt.pr_terms(ops, w, ci, 8, qs)
        for iq, q in enumerate(qs):
            emit(kind="pr_terms", method="RODAS5P", weights=name, q=q, k=list(range(8)),
                 values=[f(x) for x in terms[iq * 8:(iq + 1) * 8]])

    # ---- facts about the depth-5 layout
    g5 = mp.findroot(lambda g: 120 * g ** 5 - 600 * g ** 4 + 600 * g ** 3 - 200 * g ** 2 + 25 * g - 1, mpf("0.278"))
    obstruction = 24 * g5 ** 4 - 96 * g5 ** 3 + 72 * g5 ** 2 - 16 * g5 + 1
    roots4 = mp.polyroots([24, -96, 72, -16, 1], maxsteps=200, extraprec=200)
    roots5 = mp.polyroots([120, -600, 600, -200, 25, -1], maxsteps=200, extraprec=200)
    emit(kind="depth5_gamma", gamma=mp.nstr(g5, 50), polynomial="120 g^5 - 600 g^4 + 600 g^3 - 200 g^2 + 25 g - 1",
         residual=f(120 * g5 ** 5 - 600 * g5 ** 4 + 600 * g5 ** 3 - 200 * g5 ** 2 + 25 * g5 - 1),
         all_roots=[f(mp.re(r)) for r in roots5])
    emit(kind="depth5_embedded_obstruction", statement="embedded weights = row 7 of beta (four levels, R_hat(inf) = 0) "
         "need 24 g^4 - 96 g^3 + 72 g^2 - 16 g + 1 = 0", value_at_pinned_gamma=f(obstruction),
         roots_of_quartic=[f(mp.re(r)) for r in roots4],
         min_distance_to_a_root=f(min(abs(g5 - mp.re(r)) for r in roots4)))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
