"""PROBE B6: GE-AMP-DETECT on the corpus-v2 n = 96 rows plus Robertson, HIRES, vdP-mu1000, Brusselator-1d-50.
EXPLORATORY pilot (no ledger authority).  Read-only w.r.t. /home/user/wt-speed (reads the coefficient fixture,
NATIVE.json references, and the corpus module of probe A2).

Direct RODAS5P replica (U form, dense LU, exact stage solves, production I controller) with, on every accepted step:
  * the frozen detector (protocol.json): ge <- M ge + U8, A_n = ||ge||_sc / sum err_k, two propagators
      res5  : (I - h g J)^-5 on the attempt's own LU (5 back-solves)
      exact : RODAS5P step map on y' = J y, same LU (8 back-solves + 8 J v)
    plus matrix-free versions of both (unrestarted GMRES, x0 = 0, 1e-2 relative) with counted JVP / dots / axpys;
  * truth: reference state (exact solution, or Radau chained node to node), true local error from the
    numerical state (Radau from y_k over [t_k, t_k + h]), exact-state local error (F-033 definition) where an
    exact solution exists, and control transports driven by the TRUE local errors;
  * work counters for the baseline driver and for each transport.
Usage: python3 geamp.py ROWKEY [ROWKEY ...]   (ROWKEY = kind:name:rtol, e.g. corpus:semilinear:1e-6, bench:hires:1e-8)
       python3 geamp.py fidelity              (bench driver at 1e-3/1e-5/1e-7 vs Rust counters, harness.md 7)
"""
import os
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[_v] = "1"
import sys, json, math, time, warnings
import numpy as np
from scipy.linalg import lu_factor, lu_solve
from scipy.integrate import solve_ivp

HERE = os.path.dirname(os.path.abspath(__file__))
CORPUS = os.path.join(os.path.dirname(HERE), "corpus")
sys.path.insert(0, CORPUS)
import corpus_v2 as cv  # noqa: E402

ROOT = "/home/user/wt-speed"
PROTO = json.load(open(os.path.join(HERE, "protocol.json")))
THRESH = 4.0          # frozen
GUARD = 0.1           # frozen
LIN_RTOL_MF = 1e-2    # frozen (candidate text: loose rtol 1e-2 solves)

snap = json.load(open(f"{ROOT}/fixtures/rodas5p_coefficients_snapshot.json"))
g = float(snap["gamma"])
A = np.array([[float(x) for x in r] for r in snap["A"]])
C = np.array([[float(x) for x in r] for r in snap["C"]])
c = np.array([float(x) for x in snap["c"]])
bc = np.array([float(x) for x in snap["b_code"]])
H = np.array([[float(x) for x in r] for r in snap["H"]])
S = 8
grow = np.linalg.inv(np.eye(S) / g - C).sum(axis=1)
NATIVE = json.load(open(f"{ROOT}/research/stiff_native_benchmark_20261001/NATIVE.json"))["references"]


class P:
    def __init__(self, **kw):
        self.__dict__.update(kw)


# ------------------------------------------------------------------------------------------------ problems
def bench(name):
    if name == "robertson":
        f = lambda t, y: np.array([-0.04 * y[0] + 1e4 * y[1] * y[2],
                                   0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] ** 2, 3e7 * y[1] ** 2])
        J = lambda t, y: np.array([[-0.04, 1e4 * y[2], 1e4 * y[1]],
                                   [0.04, -1e4 * y[2] - 6e7 * y[1], -1e4 * y[1]], [0, 6e7 * y[1], 0]])
        return P(f=f, J=J, ft=None, exact=None, y0=np.array([1.0, 0, 0]), span=(0.0, 40.0), ascale=1e-4, n=3,
                 ref_final=np.array(NATIVE["robertson"]["final_state"]), rhs_flops=13.0, kind="bench", name=name)
    if name == "vdp":
        mu = 1000.0
        f = lambda t, y: np.array([y[1], mu * (1 - y[0] ** 2) * y[1] - y[0]])
        J = lambda t, y: np.array([[0.0, 1.0], [-2 * mu * y[0] * y[1] - 1, mu * (1 - y[0] ** 2)]])
        return P(f=f, J=J, ft=None, exact=None, y0=np.array([2.0, 0.0]), span=(0.0, 2000.0), ascale=1.0, n=2,
                 ref_final=np.array(NATIVE["van-der-pol-mu1000"]["final_state"]), rhs_flops=8.0, kind="bench", name=name)
    if name == "hires":
        def f(t, y):
            q = 280.0 * y[5] * y[7]
            return np.array([-1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007, 1.71 * y[0] - 8.75 * y[1],
                             -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4], 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3],
                             -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6],
                             -q + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6], q - 1.81 * y[6],
                             -q + 1.81 * y[6]])

        def J(t, y):
            j = np.zeros((8, 8))
            j[0, :3] = [-1.71, 0.43, 8.32]; j[1, :2] = [1.71, -8.75]
            j[2, 2:5] = [-10.03, 0.43, 0.035]; j[3, 1:4] = [8.32, 1.71, -1.12]
            j[4, 4:7] = [-1.745, 0.43, 0.43]
            j[5, 3:8] = [0.69, 1.71, -280 * y[7] - 0.43, 0.69, -280 * y[5]]
            j[6, 5:8] = [280 * y[7], -1.81, 280 * y[5]]; j[7, 5:8] = [-280 * y[7], 1.81, -280 * y[5]]
            return j
        y0 = np.zeros(8); y0[0] = 1.0; y0[7] = 0.0057
        return P(f=f, J=J, ft=None, exact=None, y0=y0, span=(0.0, 321.8122), ascale=1e-4, n=8,
                 ref_final=np.array(NATIVE["hires"]["final_state"]), rhs_flops=50.0, kind="bench", name=name)
    if name.startswith("bruss"):
        cells = int(name[5:]); cc = (cells + 1.0) ** 2 / 50.0; n = 2 * cells

        def f(t, y):
            u, v = y[0::2], y[1::2]
            ul = np.r_[1.0, u[:-1]]; ur = np.r_[u[1:], 1.0]; vl = np.r_[3.0, v[:-1]]; vr = np.r_[v[1:], 3.0]
            out = np.empty(n)
            out[0::2] = 1 + u * u * v - 4 * u + cc * (ul - 2 * u + ur)
            out[1::2] = 3 * u - u * u * v + cc * (vl - 2 * v + vr)
            return out

        def J(t, y):
            j = np.zeros((n, n))
            for i in range(cells):
                u, v = y[2 * i], y[2 * i + 1]; a, b = 2 * i, 2 * i + 1
                j[a, a] = 2 * u * v - 4 - 2 * cc; j[a, b] = u * u; j[b, a] = 3 - 2 * u * v; j[b, b] = -u * u - 2 * cc
                if i > 0: j[a, a - 2] = cc; j[b, b - 2] = cc
                if i + 1 < cells: j[a, a + 2] = cc; j[b, b + 2] = cc
            return j
        x = (np.arange(cells) + 1.0) / (cells + 1.0)
        y0 = np.empty(n); y0[0::2] = 1 + np.sin(2 * np.pi * x); y0[1::2] = 3.0
        ref = NATIVE.get(f"brusselator-1d-{cells}", {}).get("final_state")
        return P(f=f, J=J, ft=None, exact=None, y0=y0, span=(0.0, 10.0), ascale=1.0, n=n,
                 ref_final=None if ref is None else np.array(ref), rhs_flops=19.0 * cells, kind="bench", name=name)
    raise KeyError(name)


CORPUS_SHORT = {"robertson": "robertson-ramped", "hires": "hires-ramped", "vdp": "van-der-pol-ramped",
                "rotating": "rotating-nonnormal", "forcing": "nonautonomous-stiff-forcing",
                "semilinear": "semilinear-advection-diffusion-ramped"}


def corpus(short, n=96):
    p = cv.build(CORPUS_SHORT[short], n)
    jm = cv.JVP_COST_MODEL[p.family]
    rhs_flops = (jm["flops_per_component"] + 20.0 * jm["transcendentals_per_component"]) * n
    return P(f=p.f, J=p.J, ft=p.ft, exact=p.exact, y0=p.y0, span=p.span, ascale=cv.ATOL_FACTOR, n=n, cp=p,
             rhs_flops=rhs_flops, jvp_flops=rhs_flops, kind="corpus", name=short, family=p.family)


# ------------------------------------------------------------------------------------------------ helpers
def wn(v, w):
    return math.sqrt(float(np.mean((v / w) ** 2)))


def attempt(p, J, f0, ftv, t, y, h, atol, rtol):
    n = len(y)
    lu = lu_factor(np.eye(n) / (h * g) - J)
    U = np.zeros((S, n))
    for i in range(S):
        r = f0.copy() if i == 0 else p.f(t + c[i] * h, y + A[i, :i] @ U[:i])
        r = r + (C[i, :i] / h) @ U[:i]
        if ftv is not None:
            r = r + grow[i] * h * ftv
        U[i] = lu_solve(lu, r)
    ynew = y + bc @ U
    sc = atol + rtol * np.maximum(np.abs(y), np.abs(ynew))
    err = wn(U[-1], sc)
    return ynew, (err if np.isfinite(err) else np.inf), U, lu, sc


def res5(lu, h, v):
    w = v.copy()
    for _ in range(5):
        w = lu_solve(lu, w / (h * g))
    return w


def lin_R(lu, J, h, v):
    V = np.zeros((S, len(v)))
    for i in range(S):
        V[i] = lu_solve(lu, J @ (v + A[i, :i] @ V[:i]) + (C[i, :i] / h) @ V[:i])
    return v + bc @ V


def gmres_count(mv, b, rtol):
    """Unrestarted GMRES, x0 = 0, MGS, Givens; stops when the Arnoldi residual <= rtol ||b||.
    Returns x and counts {jvp, dot, axpy, it}."""
    n = len(b); cnt = {"jvp": 0, "dot": 1, "axpy": 0, "it": 0}
    beta = float(np.linalg.norm(b))
    if beta == 0.0:
        return np.zeros(n), cnt
    m = n
    V = np.zeros((m + 1, n)); Hh = np.zeros((m + 1, m)); cs = np.zeros(m); sn = np.zeros(m)
    gv = np.zeros(m + 1); gv[0] = beta; V[0] = b / beta
    k = 0
    for j in range(m):
        w = mv(V[j]); cnt["jvp"] += 1; cnt["axpy"] += 1
        for i in range(j + 1):
            hij = float(w @ V[i]); w = w - hij * V[i]; Hh[i, j] = hij; cnt["dot"] += 1; cnt["axpy"] += 1
        hn = float(np.linalg.norm(w)); cnt["dot"] += 1; Hh[j + 1, j] = hn
        for i in range(j):
            a_, b_ = Hh[i, j], Hh[i + 1, j]
            Hh[i, j] = cs[i] * a_ + sn[i] * b_; Hh[i + 1, j] = -sn[i] * a_ + cs[i] * b_
        den = math.hypot(Hh[j, j], Hh[j + 1, j])
        cs[j] = Hh[j, j] / den; sn[j] = Hh[j + 1, j] / den
        Hh[j, j] = den; Hh[j + 1, j] = 0.0
        gv[j + 1] = -sn[j] * gv[j]; gv[j] = cs[j] * gv[j]
        k = j + 1
        if hn > 0:
            V[j + 1] = w / hn; cnt["axpy"] += 1
        if abs(gv[j + 1]) <= rtol * beta or hn == 0.0:
            break
    yk = np.linalg.solve(np.triu(Hh[:k, :k]), gv[:k])
    x = V[:k].T @ yk; cnt["axpy"] += k; cnt["it"] = k
    return x, cnt


def add_cnt(tot, cnt):
    for k_, v_ in cnt.items():
        tot[k_] = tot.get(k_, 0) + v_


def res5_mf(J, h, v, tot, lrt=None):
    lrt = LIN_RTOL_MF if lrt is None else lrt
    hg = h * g
    mv = lambda x: x - hg * (J @ x)
    w = v.copy()
    for _ in range(5):
        w, cn = gmres_count(mv, w, lrt); add_cnt(tot, cn)
    return w


def lin_R_mf(J, h, v, tot, lrt=None):
    lrt = LIN_RTOL_MF if lrt is None else lrt
    hg = h * g; n = len(v)
    mv = lambda x: x - hg * (J @ x)
    V = np.zeros((S, n))
    for i in range(S):
        r = J @ (v + A[i, :i] @ V[:i]) + (C[i, :i] / h) @ V[:i]; tot["jvp"] = tot.get("jvp", 0) + 1
        x, cn = gmres_count(mv, hg * r, lrt); add_cnt(tot, cn)
        V[i] = x
    tot["axpy"] = tot.get("axpy", 0) + 2 * 28 + 8
    return v + bc @ V


def ref_tols(rtol, atol):
    return max(2.5e-13, 1e-5 * rtol), 1e-5 * atol


def flow(p, t0, t1, y, rr, ra):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        r = solve_ivp(p.f, (t0, t1), y, method="Radau", rtol=rr, atol=ra, jac=p.J, t_eval=[t1])
    if not r.success:
        raise RuntimeError(f"Radau failed {p.name} [{t0},{t1}]: {r.message}")
    return r.y[:, -1], int(r.nfev)


def bandwidths(p):
    kl = ku = 0; nnz = 0
    t0, tf = p.span
    for tt in (t0, 0.5 * (t0 + tf)):
        y = p.y0 + 0.01 * (1 + np.abs(p.y0))
        Jm = p.J(tt, y)
        r, cc_ = np.nonzero(Jm)
        if len(r):
            kl = max(kl, int(np.max(r - cc_))); ku = max(ku, int(np.max(cc_ - r)))
        nnz = max(nnz, len(r))
    return kl, ku, nnz


# ------------------------------------------------------------------------------------------------ driver
def run(p, rtol, truth=True, mf=True, outgrid=None, h0mult=1.0):
    atol = rtol * p.ascale; t0, tf = p.span; span = tf - t0
    corpus_mode = p.kind == "corpus"
    T = p.cp.output_times if corpus_mode else (np.linspace(t0, tf, 101) if outgrid is None else outgrid)
    h = (span / 100.0 if corpus_mode else 1e-6) * h0mult
    clip = not corpus_mode
    n = p.n
    y = p.y0.copy(); t = t0
    outs = {0: y.copy()}
    cnt = {"attempts": 0, "accepted": 0, "rejected": 0, "rhs": 0, "ft": 0, "jac": 0, "lu": 0, "backsolve": 0,
           "stage_axpy": 0, "wrms": 0}
    ov = {"res5": {"backsolve": 0, "axpy": 0, "wrms": 0}, "exact": {"backsolve": 0, "matvec": 0, "axpy": 0, "wrms": 0},
          "res5_mf": {}, "exact_mf": {}, "res5_mf4": {}, "exact_mf4": {}}
    ge5 = np.zeros(n); geR = np.zeros(n); ge5t = np.zeros(n); geRt = np.zeros(n); ge5m = np.zeros(n); geRm = np.zeros(n); ge5m4 = np.zeros(n); geRm4 = np.zeros(n)
    sumerr = 0.0
    rr, ra = ref_tols(rtol, atol)
    yref = p.y0.copy()
    sum_le = 0.0; sum_le_ex = 0.0; ref_nfev = 0
    steps = {k: [] for k in ("t", "h", "err", "A5", "AR", "A5m", "ARm", "g5", "gR", "G", "le", "le_ex",
                             "R5t", "RRt", "gR_ref", "g5_ref", "Rrun", "A5m4", "ARm4")}
    last_rej = None; fresh = True
    t_start = time.time()
    while t < tf and cnt["attempts"] < 200000:
        h = min(h, tf - t, span)
        if clip and last_rej is not None and h >= last_rej:
            h = np.nextafter(t + last_rej, -np.inf) - t
        if fresh:
            J = p.J(t, y); f0 = p.f(t, y); cnt["jac"] += 1; cnt["rhs"] += 1
            ftv = p.ft(t, y) if p.ft is not None else None
            if ftv is not None:
                cnt["ft"] += 1
        cnt["attempts"] += 1
        ynew, err, U, lu, sc = attempt(p, J, f0, ftv, t, y, h, atol, rtol)
        cnt["rhs"] += 7; cnt["lu"] += 1; cnt["backsolve"] += 8; cnt["wrms"] += 1
        cnt["stage_axpy"] += 28 + 28 + 8 + (8 if ftv is not None else 0)
        if err <= 1.0:
            cnt["accepted"] += 1
            tn = t + h if (tf - (t + h)) > 0 else tf
            U8 = U[-1]
            sumerr += err
            # ---- detector transports (direct, own LU)
            ge5 = res5(lu, h, ge5) + U8
            ov["res5"]["backsolve"] += 5; ov["res5"]["axpy"] += 6; ov["res5"]["wrms"] += 1
            geR = lin_R(lu, J, h, geR) + U8
            ov["exact"]["backsolve"] += 8; ov["exact"]["matvec"] += 8; ov["exact"]["axpy"] += 2 * 28 + 8 + 1
            ov["exact"]["wrms"] += 1
            g5n = wn(ge5, sc); gRn = wn(geR, sc)
            steps["A5"].append(g5n / sumerr); steps["AR"].append(gRn / sumerr)
            steps["g5"].append(g5n); steps["gR"].append(gRn)
            # ---- matrix-free transports (loose GMRES)
            if mf:
                ge5m = res5_mf(J, h, ge5m, ov["res5_mf"]) + U8
                geRm = lin_R_mf(J, h, geRm, ov["exact_mf"]) + U8
                steps["A5m"].append(wn(ge5m, sc) / sumerr); steps["ARm"].append(wn(geRm, sc) / sumerr)
                ge5m4 = res5_mf(J, h, ge5m4, ov["res5_mf4"], 1e-4) + U8
                geRm4 = lin_R_mf(J, h, geRm4, ov["exact_mf4"], 1e-4) + U8
                steps["A5m4"].append(wn(ge5m4, sc) / sumerr); steps["ARm4"].append(wn(geRm4, sc) / sumerr)
            # ---- truth
            if truth:
                if p.exact is not None:
                    yref1 = p.exact(tn)
                    ye0 = p.exact(t)
                    Je = p.J(t, ye0); fe = p.f(t, ye0); fte = p.ft(t, ye0) if p.ft is not None else None
                    yl, _, _, _, _ = attempt(p, Je, fe, fte, t, ye0, h, atol, rtol)
                    le_ex = yl - yref1
                else:
                    yref1, nf = flow(p, t, tn, yref, rr, ra); ref_nfev += nf
                    le_ex = None
                yfl, nf = flow(p, t, tn, y, rr, ra); ref_nfev += nf
                le = ynew - yfl
                wref = atol + rtol * np.abs(yref1)
                G = wn(ynew - yref1, wref); len_ = wn(le, wref)
                sum_le += len_
                ge5t = res5(lu, h, ge5t) + le
                geRt = lin_R(lu, J, h, geRt) + le
                steps["G"].append(G); steps["le"].append(len_)
                steps["Rrun"].append(G / sum_le if sum_le > 0 else float("nan"))
                if le_ex is not None:
                    le_exn = wn(le_ex, wref); sum_le_ex += le_exn; steps["le_ex"].append(le_exn)
                steps["R5t"].append(wn(ge5t, wref)); steps["RRt"].append(wn(geRt, wref))
                steps["gR_ref"].append(wn(geR, wref)); steps["g5_ref"].append(wn(ge5, wref))
                yref = yref1
            steps["t"].append(tn); steps["h"].append(h); steps["err"].append(err)
            # ---- dense output (H interpolant) on the output grid
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
            fresh = True; last_rej = None
            fac = 5.0 if err == 0 else min(max(0.9 * err ** -0.2, 0.2), 5.0)
        else:
            cnt["rejected"] += 1
            fresh = False; last_rej = h
            fac = min(max(0.9 * max(err, 1e-16) ** -0.2, 0.2), 0.9)
        h *= fac
    Y = np.array([outs[k] for k in range(len(T))]) if len(outs) == len(T) else None
    res = {"rtol": rtol, "atol": atol, "counters": cnt, "overhead": ov, "steps": steps, "y_final": y.tolist(),
           "wall": time.time() - t_start, "ref_nfev": ref_nfev, "ref_tols": [rr, ra]}
    a5 = np.array(steps["A5"]); aR = np.array(steps["AR"])
    g5 = np.array(steps["g5"]); gR = np.array(steps["gR"])
    det = {}
    for lab, a, gg in (("res5", a5, g5), ("exact", aR, gR)):
        amax = float(a.max()); guard = float(rtol * gg.max())
        det[lab] = {"A_max": amax, "A_end": float(a[-1]), "t_Amax": float(steps["t"][int(a.argmax())]),
                    "guard_value": guard, "AMP": amax > THRESH, "INVALID": guard > GUARD,
                    "FLAG": (amax > THRESH) or (guard > GUARD), "max_transport_tol_units": float(gg.max())}
    if mf:
        for lab, key in (("res5_mf", "A5m"), ("exact_mf", "ARm"), ("res5_mf4", "A5m4"), ("exact_mf4", "ARm4")):
            a = np.array(steps[key])
            det[lab] = {"A_max": float(a.max()), "AMP": float(a.max()) > THRESH}
    res["detector"] = det
    if truth:
        G = np.array(steps["G"]); le = np.array(steps["le"])
        tr = {"G_max_nodes": float(G.max()), "t_Gmax": float(steps["t"][int(G.argmax())]), "G_end": float(G[-1]),
              "sum_le": float(le.sum()), "max_le": float(le.max()),
              "R_tot": float(G.max() / le.sum()), "R_run_max": float(np.nanmax(steps["Rrun"])),
              "transport_true_exact_max": float(np.max(steps["RRt"])),
              "transport_true_res5_max": float(np.max(steps["R5t"])),
              "transport_est_exact_max_refw": float(np.max(steps["gR_ref"])),
              "transport_est_res5_max_refw": float(np.max(steps["g5_ref"])),
              "y_ref_final": yref.tolist()}
        if steps["le_ex"]:
            tr["sum_le_exact_state"] = float(np.sum(steps["le_ex"])); tr["max_le_exact_state"] = float(np.max(steps["le_ex"]))
            tr["R_tot_exact_state"] = float(G.max() / np.sum(steps["le_ex"]))
        res["truth"] = tr
    if Y is not None:
        res["Y_grid"] = Y
    return res


# ------------------------------------------------------------------------------------------------ row runner
def get_problem(kind, name):
    if kind == "extra":
        import extra_probs
        d = extra_probs.osc_amp(float(name[3:]))
        return P(**d, ascale=cv.ATOL_FACTOR, kind="corpus", jvp_flops=d["rhs_flops"])
    if kind.startswith("corpus") and kind != "corpus":
        return corpus(name, int(kind[6:]))
    return corpus(name) if kind == "corpus" else bench(name)


def grid_reference(p, rtol, T):
    """bench rows: Radau chained over the output grid from y0 (tight)."""
    atol = rtol * p.ascale; rr, ra = ref_tols(rtol, atol)
    Y = [p.y0.copy()]
    for k in range(1, len(T)):
        yk, _ = flow(p, T[k - 1], T[k], Y[-1], rr, ra); Y.append(yk)
    return np.array(Y)


def grid_err(Y, R, rtol, atol):
    W = atol + rtol * np.abs(R)
    return np.sqrt(np.mean(((Y - R) / W) ** 2, axis=1))


def run_row(key):
    parts = key.split(":"); kind, name, rs = parts[:3]; rtol = float(rs)
    h0mult = float(parts[3]) if len(parts) > 3 else 1.0
    p = get_problem(kind, name)
    t0 = time.time()
    r = run(p, rtol, truth=True, mf=True, h0mult=h0mult)
    out = {"key": key, "kind": kind, "name": name, "rtol": rtol, "n": p.n, "h0mult": h0mult}
    atol = rtol * p.ascale
    T = p.cp.output_times if p.kind == "corpus" else np.linspace(p.span[0], p.span[1], 101)
    Y = r.pop("Y_grid")
    # ---- repository grid metric
    if p.kind == "corpus":
        prefer_exact = p.exact is not None
        m = p.cp.error_metrics(Y, rtol, prefer_exact=prefer_exact)
        ref = p.cp.exact_reference() if prefer_exact else p.cp.reference()
        Rg = ref["states"]
        out["grid"] = {"max_grid_case": m["max_grid_case"], "endpoint_case": m["endpoint_case"],
                       "ref_source": "exact" if prefer_exact else "stored", "ref_unc_case": cv.case_units(ref["uncertainty_wrms"], rtol)}
        # reference check: chained / exact final reference vs stored final reference (tight units)
        stored = p.cp.reference()
        yrf = np.array(r["truth"]["y_ref_final"])
        out["ref_check_tight"] = float(cv.wrms_rows(yrf[None, :], stored["states"][-1][None, :])[0])
        out["ref_check_case"] = cv.case_units(out["ref_check_tight"], rtol)
        ru = cv.rust_recorded_rows().get(p.cp.case_id(rtol))
        if ru is not None:
            d = ru["dense"]
            out["rust"] = {k: d[k] for k in ("attempts", "accepted", "rejected", "max_grid_case", "endpoint_case",
                                              "rhs_evaluations", "ft_calls", "jvp_vectors", "linear_solves",
                                              "linear_iterations", "linear_matvecs", "orthogonalization_inner_products",
                                              "orthogonalization_vector_updates", "preconditioner_apps",
                                              "direct_factorizations")}
    else:
        Rg = grid_reference(p, rtol, T)
        ge = grid_err(Y, Rg, rtol, atol)
        out["grid"] = {"max_grid_case": float(ge.max()), "endpoint_case": float(ge[-1]), "ref_source": "radau-chain-grid"}
        if p.ref_final is not None:
            yrf = np.array(r["truth"]["y_ref_final"])
            w = atol + rtol * np.abs(p.ref_final)
            out["ref_check_case"] = wn(yrf - p.ref_final, w)        # node chain vs NATIVE final state
            out["ref_check_grid_case"] = wn(Rg[-1] - p.ref_final, w)
            out["final_err_native_case"] = wn(np.array(r["y_final"]) - p.ref_final, w)
            out["final_relerr_native"] = float(np.max(np.abs(np.array(r["y_final"]) - p.ref_final) /
                                                      np.maximum(np.abs(p.ref_final), 1e-10)))
    # ---- rival: two-tolerance rerun
    r10 = run(p, rtol / 10.0, truth=False, mf=False, h0mult=h0mult)
    Y10 = r10.pop("Y_grid")
    est = grid_err(Y, Y10, rtol, atol)
    out["rival"] = {"ge_est_max_grid": float(est.max()), "FLAG_exceed": bool(est.max() > 1.0),
                    "attempts": r10["counters"]["attempts"], "accepted": r10["counters"]["accepted"],
                    "counters": r10["counters"],
                    "est_over_true": float(est.max() / out["grid"]["max_grid_case"]) if out["grid"]["max_grid_case"] > 0 else None}
    out["bandwidth"] = dict(zip(("kl", "ku", "nnz"), bandwidths(p)))
    out["rhs_flops"] = p.rhs_flops
    out.update(r)
    out["wall_total"] = time.time() - t0
    return out


def fidelity():
    """bench driver vs Rust rodas5p-fast counters (harness.md section 7 table)."""
    rust = {"robertson": ((22, 21, 1, 9.19e-5), (33, 32, 1, 4.87e-6), (66, 63, 3, 1.10e-7)),
            "vdp": ((181, 114, 67, 3.49e-3), (349, 228, 121, 2.21e-5), (654, 559, 95, 2.29e-7)),
            "hires": ((52, 41, 11, 1.71e-4), (123, 105, 18, 6.28e-6), (406, 402, 4, 1.38e-8)),
            "bruss50": ((41, 33, 8, 8.02e-5), (69, 57, 12, 1.04e-5), (130, 123, 7, 2.42e-7))}
    out = {}
    for name, rows in rust.items():
        p = bench(name)
        for rtol, (ra, rac, rrj, rerr) in zip((1e-3, 1e-5, 1e-7), rows):
            r = run(p, rtol, truth=False, mf=False)
            yf = np.array(r["y_final"]); ref = p.ref_final
            err = float(np.max(np.abs(yf - ref) / np.maximum(np.abs(ref), 1e-10)))  # rep.py err_bench
            cn = r["counters"]
            out[f"{name}:{rtol:.0e}"] = {"rust": [ra, rac, rrj, rerr], "replica": [cn["attempts"], cn["accepted"], cn["rejected"], err]}
            print(f"{name:9s} {rtol:.0e} rust att/acc/rej/err {ra}/{rac}/{rrj}/{rerr:.3g}  replica {cn['attempts']}/{cn['accepted']}/{cn['rejected']}/{err:.3g}", flush=True)
    json.dump(out, open(os.path.join(HERE, "fidelity_bench.json"), "w"), indent=1)


def _jsonable(o):
    if isinstance(o, dict):
        return {k: _jsonable(v) for k, v in o.items()}
    if isinstance(o, (list, tuple)):
        return [_jsonable(v) for v in o]
    if isinstance(o, (np.floating,)):
        return float(o)
    if isinstance(o, (np.integer,)):
        return int(o)
    if isinstance(o, np.bool_):
        return bool(o)
    if isinstance(o, np.ndarray):
        return o.tolist()
    return o


if __name__ == "__main__":
    if sys.argv[1] == "fidelity":
        fidelity(); sys.exit(0)
    os.makedirs(os.path.join(HERE, "rows"), exist_ok=True); os.makedirs(os.path.join(HERE, "rows_seed"), exist_ok=True)
    for key in sys.argv[1:]:
        o = run_row(key)
        sub = "rows_extra" if (key.startswith("extra") or key.startswith("corpus384") or key.startswith("corpus1536")) else ("rows" if len(key.split(":")) == 3 else "rows_seed")
        os.makedirs(os.path.join(HERE, sub), exist_ok=True)
        fn = os.path.join(HERE, sub, key.replace(":", "_") + ".json")
        json.dump(_jsonable(o), open(fn, "w"))
        d5 = o["detector"]["res5"]; dR = o["detector"]["exact"]; tr = o["truth"]
        print(f"{key:28s} att {o['counters']['attempts']:5d} acc {o['counters']['accepted']:5d} | grid {o['grid']['max_grid_case']:9.3g} "
              f"Gnode {tr['G_max_nodes']:9.3g} sum_le {tr['sum_le']:8.3g} R_tot {tr['R_tot']:7.3g} R_run {tr['R_run_max']:7.3g} | "
              f"A5 {d5['A_max']:9.3g} AR {dR['A_max']:9.3g} guard5 {d5['guard_value']:8.2g} F5 {d5['FLAG']} FR {dR['FLAG']} | "
              f"rival {o['rival']['ge_est_max_grid']:8.3g} | {o['wall_total']:.1f}s", flush=True)
