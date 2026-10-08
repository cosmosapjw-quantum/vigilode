"""End-to-end fidelity of corpus_v2 + a dense-LU RODAS5P replica of the v2 campaign DENSE arm
(U form, analytic f_t, integral controller 0.9 err^(-1/5) clamped [0.2, 5] / [0.2, 0.9], initial step span/100,
max step span, RODAS5P dense interpolant with H from the coefficient fixture, error estimate = last U stage in
WRMS with scale atol + rtol max(|y|, |y_new|)).  The Rust arm solves stages with matrix-free GMRES and a WRMS
inner-forcing heuristic; this replica solves them exactly (dense LU), which the v3 addendum found leaves the step
sequence and global error unchanged on the semilinear rows (R_inner = 0.80 / 1.00 / 1.00).
Compares attempts / accepted / rejected / max-grid and endpoint errors with the recorded Rust dense arm.
EXPLORATORY.  Usage: python3 replica_check.py [n]"""
import os
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[_v] = "1"
import json, math, sys
import numpy as np
from scipy.linalg import lu_factor, lu_solve
from scipy.sparse.linalg import splu
from scipy.sparse import identity
import corpus_v2 as cv

snap = json.load(open("/home/user/wt-speed/fixtures/rodas5p_coefficients_snapshot.json"))
g = float(snap["gamma"])
A = np.array([[float(x) for x in r] for r in snap["A"]])
C = np.array([[float(x) for x in r] for r in snap["C"]])
c = np.array([float(x) for x in snap["c"]])
bc = np.array([float(x) for x in snap["b_code"]])
H = np.array([[float(x) for x in r] for r in snap["H"]])
S = 8
grow = np.linalg.inv(np.eye(S) / g - C).sum(axis=1)


def attempt(p, Js, f0, ftv, t, y, h, atol, rtol):
    n = len(y)
    lu = splu((identity(n, format="csc") / (h * g) - Js).tocsc())
    U = np.zeros((S, n))
    for i in range(S):
        r = f0.copy() if i == 0 else p.f(t + c[i] * h, y + A[i, :i] @ U[:i])
        r = r + (C[i, :i] / h) @ U[:i] + grow[i] * h * ftv
        U[i] = lu.solve(r)
    ynew = y + bc @ U
    sc = atol + rtol * np.maximum(np.abs(y), np.abs(ynew))
    err = math.sqrt(np.mean((U[-1] / sc) ** 2))
    return ynew, err, U


def run_dense_arm(p, rtol, maxatt=200000):
    atol = p.atol(rtol); t0, tf = p.span; span = tf - t0
    T = p.output_times
    t, y, h = t0, p.y0, span / 100.0
    outs = {0: y.copy()}
    att = acc = rej = 0
    rhs = 0
    acc_h, errs = [], []
    while t < tf and att < maxatt:
        h = min(h, tf - t, span)
        Js = p.J_sparse(t, y); f0 = p.f(t, y); ftv = p.ft(t, y); rhs += 1
        att += 1
        ynew, err, U = attempt(p, Js, f0, ftv, t, y, h, atol, rtol); rhs += 7
        errs.append(err)
        if err <= 1.0:
            acc += 1; acc_h.append(h)
            tn = t + h if (tf - (t + h)) > 0 else tf
            d = H @ U
            for k in range(len(T)):
                if k in outs:
                    continue
                if T[k] > t and T[k] <= tn:
                    if T[k] == tn:
                        outs[k] = ynew.copy()
                    else:
                        th = (T[k] - t) / h; cm = 1.0 - th
                        outs[k] = cm * y + th * (ynew + cm * (d[0] + th * (d[1] + th * d[2])))
            t, y = tn, ynew
            fac = 5.0 if err == 0 else min(max(0.9 * err ** -0.2, 0.2), 5.0)
        else:
            rej += 1
            fac = min(max(0.9 * err ** -0.2, 0.2), 0.9)
        h *= fac
    Y = np.array([outs[k] for k in range(len(T))])
    return {"attempts": att, "accepted": acc, "rejected": rej, "rhs": rhs, "Y": Y, "acc_h": acc_h, "errs": errs}


def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 96
    rows = cv.rust_recorded_rows()
    res = {}
    print(f"{'case':66s} {'att R/py':>9s} {'rej R/py':>8s} {'maxgrid case R / py':>22s} {'endpoint case R / py':>22s} {'first-acc-h match':>6s}")
    for case in cv.calibration_cases(n):
        p = case["problem"]; rtol = case["rtol"]
        r = run_dense_arm(p, rtol)
        ref = p.reference()
        m = cv.global_error_metrics(r["Y"], ref["states"], rtol, ref["uncertainty_wrms"])
        rr = rows[case["case_id"]]["dense"]
        # longest common prefix of accepted step sizes (rel 1e-6)
        k = 0
        for a, b in zip(rr["accepted_step_sizes"], r["acc_h"]):
            if abs(a - b) > 1e-6 * abs(a):
                break
            k += 1
        res[case["case_id"]] = {
            "rust": {"attempts": rr["attempts"], "rejected": rr["rejected"], "max_grid_case": rr["max_grid_case"],
                     "endpoint_case": rr["endpoint_case"]},
            "replica": {"attempts": r["attempts"], "rejected": r["rejected"], "max_grid_case": m["max_grid_case"],
                        "endpoint_case": m["endpoint_case"], "rhs": r["rhs"]},
            "accepted_prefix_match": k, "rust_accepted": rr["accepted"],
            "max_rel_dev_accepted_h": (float(np.max(np.abs(np.array(rr["accepted_step_sizes"]) - np.array(r["acc_h"]))
                                                    / np.array(rr["accepted_step_sizes"])))
                                       if len(r["acc_h"]) == rr["accepted"] else None),
        }
        print(f"{case['case_id']:66s} {rr['attempts']:4d}/{r['attempts']:<4d} {rr['rejected']:3d}/{r['rejected']:<3d} "
              f"{rr['max_grid_case']:10.4g} / {m['max_grid_case']:<10.4g} {rr['endpoint_case']:10.4g} / {m['endpoint_case']:<10.4g} "
              f"{k}/{rr['accepted']} dev={res[case['case_id']]['max_rel_dev_accepted_h']}", flush=True)
    json.dump(res, open(os.path.join(os.path.dirname(os.path.abspath(__file__)), f"replica_dense_arm_n{n}.json"), "w"), indent=1)


if __name__ == "__main__":
    main()
