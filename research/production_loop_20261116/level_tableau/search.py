#!/usr/bin/env python3
"""Loop L1 node V16 -- multi-start nonlinear least-squares search (protocol: PREDECLARED.json, key "search",
with amendment A-1).

    python3 -I search.py D6T --starts 2000 --workers 4                    > logs/search_D6T.jsonl      # phase A
    python3 -I search.py D6T --polish-top logs/search_D6T.jsonl           > logs/polish_D6T.jsonl      # phase B
    python3 -I search.py D6T --continuation logs/polish_D6T.jsonl         > logs/continuation_D6T.jsonl

Phase A, per start: least_squares (trf, bounds, complex-step Jacobian) on the stacked residual [hard; weighted
soft], then a polish on the manifold of the hard conditions (projected Levenberg-Marquardt on the weighted soft
terms; every iterate is projected back by minimum-norm Gauss-Newton steps). Phase B: longer polish of the best
passing starts. One JSON line per start; every start is kept, passing or not. A start passes the
double-precision screen if max |hard residual| <= 1e-12, gamma is inside its window and no coefficient
exceeds 100.
"""
import argparse
import json
import multiprocessing as mp_
import os
import sys
import time

os.environ.setdefault("OMP_NUM_THREADS", "1")
os.environ.setdefault("OPENBLAS_NUM_THREADS", "1")
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import numpy as np  # noqa: E402
import scipy  # noqa: E402
from scipy.optimize import least_squares  # noqa: E402

import layouts  # noqa: E402

BASE_SEED = 20261117
PASS_TOL = 1e-12
COEF_BOUND = 100.0
STAGE1_NFEV = 150
POLISH_A = 60
POLISH_B = 600
POLISH_C = 200
TOP_B = 40
TOP_C = 5
_F = {}


def formulation(fid):
    if fid not in _F:
        _F[fid] = layouts.Formulation(fid)
    return _F[fid]


def project(F, x, fixed_gamma=False, iters=30):
    """Gauss-Newton projection onto the hard conditions (minimum-norm steps)."""
    best = x.copy()
    best_r = float(np.abs(F.hard_fast(x[None])[0]).max())
    it = 0
    for it in range(1, iters + 1):
        if best_r <= 1e-14:
            break
        h = F.hard_fast(x[None])[0]
        J = F.hard_jac(x)
        if fixed_gamma and F.has_gamma:
            J[:, 0] = 0.0
        if not (np.all(np.isfinite(J)) and np.all(np.isfinite(h))):
            break
        dx = np.linalg.lstsq(J, h, rcond=1e-12)[0]
        x = x - dx
        r = float(np.abs(F.hard_fast(x[None])[0]).max())
        if not np.isfinite(r):
            break
        if r < best_r:
            best, best_r = x.copy(), r
        elif it > 3:
            break
    return best, best_r, it


def polish(F, x, iters, fixed_gamma=False):
    """Minimise the weighted soft terms on the manifold hard = 0 (projected Levenberg-Marquardt).

    Returns (x, max_hard, iterations, phi) with phi = squared norm of the weighted soft vector."""
    mh = F.m_hard
    lo_g, hi_g = layouts.GAMMA_WINDOW_6
    x, r, _ = project(F, x, fixed_gamma)
    if not np.isfinite(r) or r > 1e-9:
        return x, r, 0, None
    S = F.stacked(x[None])[0][mh:]
    phi = float(S @ S)
    lam = 1e-2
    stall = 0
    it = 0
    for it in range(1, iters + 1):
        J = F.stacked_jac(x)
        free = np.arange(1, F.n) if (fixed_gamma and F.has_gamma) else np.arange(F.n)
        Jh = J[:mh][:, free]
        Js = J[mh:][:, free]
        if not np.all(np.isfinite(J)):
            break
        _, sv, Vt = np.linalg.svd(Jh)
        rank = int((sv > 1e-11 * sv[0]).sum())
        Z = Vt[rank:].T
        A = Js @ Z
        g = A.T @ S
        H = A.T @ A
        dH = np.diag(H).copy() + 1e-30
        accepted = False
        for _ in range(10):
            try:
                dz = -np.linalg.solve(H + lam * np.diag(dH), g)
            except np.linalg.LinAlgError:
                lam *= 10
                continue
            xt = x.copy()
            xt[free] += Z @ dz
            fix_t = fixed_gamma
            if F.has_gamma and not fixed_gamma and not (lo_g <= xt[0] <= hi_g):
                xt[0] = min(max(xt[0], lo_g), hi_g)
                fix_t = True
            xt, rt_, _ = project(F, xt, fix_t, iters=8)
            if np.isfinite(rt_) and rt_ <= 1e-11 and np.all(np.isfinite(xt)):
                St = F.stacked(xt[None])[0][mh:]
                phit = float(St @ St)
                if np.isfinite(phit) and phit < phi:
                    rel = (phi - phit) / phi
                    x, S, phi, r = xt, St, phit, rt_
                    fixed_gamma = fix_t
                    lam = max(lam / 3, 1e-10)
                    accepted = True
                    stall = stall + 1 if rel < 1e-9 else 0
                    break
            lam *= 5
        if not accepted or stall >= 5:
            break
    x, r, _ = project(F, x, fixed_gamma, iters=8)
    return x, r, it, phi


def report(F, x, r, t0, extra):
    gamma = float(x[0]) if F.has_gamma else F.gamma5
    cmax = F.coef_max(x) if np.all(np.isfinite(x)) else float("inf")
    in_window = (not F.has_gamma) or (layouts.GAMMA_WINDOW_6[0] <= gamma <= layouts.GAMMA_WINDOW_6[1])
    ok = bool(np.isfinite(r) and r <= PASS_TOL and in_window and cmax <= COEF_BOUND)
    out = dict(extra)
    out.update({"max_hard": r, "gamma": gamma, "coef_max": cmax, "gamma_in_window": bool(in_window),
                "pass_double": ok, "seconds": time.time() - t0})
    if np.isfinite(r) and np.all(np.isfinite(x)):
        sc, parts = F.score(x)
        if np.isfinite(sc):
            out["score"] = sc
            out["parts"] = parts
    if ok:
        out["x"] = [repr(float(v)) for v in x]
    return out


def solve(F, x0):
    lo, hi = F.bounds()
    x0 = np.clip(x0, lo + 1e-15, hi - 1e-15)
    t0 = time.time()
    try:
        sol = least_squares(lambda x: F.stacked(x[None])[0], x0, jac=F.stacked_jac, bounds=(lo, hi),
                            method="trf", ftol=1e-14, xtol=1e-14, gtol=1e-14, max_nfev=STAGE1_NFEV)
        lsq = {"status": int(sol.status), "nfev": int(sol.nfev), "cost": float(sol.cost),
               "optimality": float(sol.optimality)}
        hard_before = float(np.abs(F.hard_fast(sol.x[None])[0]).max())
        xp, r, its, phi = polish(F, sol.x, POLISH_A)
    except Exception as exc:  # kept as a failed start
        return {"failed": repr(exc)[:200], "seconds": time.time() - t0, "pass_double": False}
    return report(F, xp, r, t0, {"lsq": lsq, "hard_before_projection": hard_before, "polish_iterations": its})


def run_start(args):
    fid, k = args
    F = formulation(fid)
    seed = [BASE_SEED, layouts.SEED_CODE[fid], k]
    rng = np.random.default_rng(np.random.SeedSequence(seed))
    out = solve(F, F.random_start(rng))
    out.update(kind="start", fid=fid, k=k, seed=seed)
    return out


def run_polish(args):
    fid, k, x_list, score_a = args
    F = formulation(fid)
    t0 = time.time()
    x = np.array([float(v) for v in x_list])
    try:
        xp, r, its, phi = polish(F, x, POLISH_B)
    except Exception as exc:
        return {"kind": "polish", "fid": fid, "k": k, "failed": repr(exc)[:200], "pass_double": False}
    out = report(F, xp, r, t0, {"polish_iterations": its, "score_phase_a": score_a})
    out.update(kind="polish", fid=fid, k=k)
    return out


def run_continuation(args):
    fid, k, x_list, grid = args
    F = formulation(fid)
    x_src = np.array([float(v) for v in x_list])
    g0 = x_src[0]
    rows = []
    for direction in (+1, -1):
        x = x_src.copy()
        gs = [g for g in grid if (g - g0) * direction > 0]
        gs.sort(key=lambda g: abs(g - g0))
        for g in gs:
            t0 = time.time()
            x0 = x.copy()
            x0[0] = g
            try:
                xp, r, its, phi = polish(F, x0, POLISH_C, fixed_gamma=True)
                out = report(F, xp, r, t0, {"polish_iterations": its})
            except Exception as exc:
                out = {"failed": repr(exc)[:200], "pass_double": False}
            out.update(kind="continuation", fid=fid, source_start=k, gamma_fixed=g, direction=direction)
            rows.append(out)
            if out.get("pass_double"):
                x = np.array([float(v) for v in out["x"]])
    return rows


def emit(obj):
    print(json.dumps(clean(obj), allow_nan=False), flush=True)


def clean(o):
    """Replace non-finite floats by None so that the log stays strict JSON."""
    if isinstance(o, dict):
        return {a: clean(b) for a, b in o.items()}
    if isinstance(o, list):
        return [clean(b) for b in o]
    if isinstance(o, float) and not np.isfinite(o):
        return None
    return o


def best_passing(path, kinds, top):
    rows = [json.loads(line) for line in open(path)]
    passing = [r for r in rows if r.get("kind") in kinds and r.get("pass_double") and "score" in r]
    passing.sort(key=lambda r: (r["score"], r["k"]))
    return passing[:top], len(passing)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("fid")
    ap.add_argument("--starts", type=int, default=2000)
    ap.add_argument("--first", type=int, default=0)
    ap.add_argument("--workers", type=int, default=4)
    ap.add_argument("--polish-top", default=None)
    ap.add_argument("--continuation", default=None)
    a = ap.parse_args()
    F = formulation(a.fid)
    mode = "continuation" if a.continuation else ("polish" if a.polish_top else "multistart")
    emit({"kind": "header", "script": "search.py", "fid": a.fid, "unknowns": F.n, "hard_conditions": F.m_hard,
          "hard_names": F.hard_names, "stages": F.s, "levels": F.levels, "weights": layouts.WEIGHTS,
          "soft_counts": {"E6": len(F.trees6), "D": len(F.d_main), "Dhat": len(F.d_emb), "PR": 3 * F.L,
                          "PRhat": 2 * F.L},
          "base_seed": BASE_SEED, "seed_code": layouts.SEED_CODE[a.fid], "numpy": np.__version__,
          "scipy": scipy.__version__, "mode": mode, "first": a.first, "starts": a.starts,
          "stage1_max_nfev": STAGE1_NFEV, "polish_iterations": {"A": POLISH_A, "B": POLISH_B, "C": POLISH_C},
          "top_b": TOP_B, "top_c": TOP_C})
    t0 = time.time()
    with mp_.Pool(a.workers) as pool:
        if a.continuation:
            top, _ = best_passing(a.continuation, ("polish",), TOP_C)
            grid = [round(0.19 + 0.01 * i, 2) for i in range(15)]
            n = 0
            for res in pool.imap(run_continuation, [(a.fid, r["k"], r["x"], grid) for r in top]):
                for row in res:
                    emit(row)
                    n += 1
            emit({"kind": "footer", "solves": n, "sources": [r["k"] for r in top], "grid": grid,
                  "seconds": time.time() - t0})
        elif a.polish_top:
            top, npass = best_passing(a.polish_top, ("start",), TOP_B)
            n = 0
            for row in pool.imap(run_polish, [(a.fid, r["k"], r["x"], r["score"]) for r in top]):
                emit(row)
                n += 1
            emit({"kind": "footer", "polished": n, "passing_in_phase_a": npass, "seconds": time.time() - t0})
        else:
            n = npass = 0
            for row in pool.imap(run_start, [(a.fid, k) for k in range(a.first, a.first + a.starts)], chunksize=2):
                emit(row)
                n += 1
                npass += 1 if row.get("pass_double") else 0
            emit({"kind": "footer", "starts": n, "passed_double": npass, "seconds": time.time() - t0})


if __name__ == "__main__":
    main()
