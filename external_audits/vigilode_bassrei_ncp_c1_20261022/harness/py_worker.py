#!/usr/bin/env python3
"""One Python comparator run (scipy VODE / DOP853) or the Radau reference.

    python3 py_worker.py REQUEST.json        (JSON result on stdout)

REQUEST: {"case": id, "arm": arm, "rtol": rtol}. Arms:

- py_vode_adams_mf10: scipy ode('vode', method='adams', with_jacobian=False)
- py_vode_bdf_mf21 / mf22: BDF, dense Jacobian (user / internal FD)
- py_vode_bdf_mf24 / mf25: BDF, banded Jacobian (user, packed band of the
  reference Jacobian restricted to the band / internal FD), lband = uband = nm+1
- py_dop853: solve_ivp DOP853, t_eval = outputs
- py_dop853_tight: DOP853 at rtol 3e-11 with max_step = 1e-3 T, t_eval = outputs.
  rtol and the relative step cap follow the DOP853 call of the rei C15 source
  (research/rei_c15_sigma_pi_feedback_20261015/coupled.py lines 329-332, git
  blob 99d1217f35c5333684efd206db7cb2d2abe14a47: rtol=3e-11, max_step=0.001 in its ln a time); atol is the campaign
  rule (the source's atol belongs to a different state vector)
- py_radau_ref: solve_ivp Radau with the reference Jacobian, restarted at
  every output (the P02-P04 reference practice)

The RHS and Jacobians are the independent Python references of P02 (oracle
operator), P03 (bordered) and P04 (chemistry). The reported solve time is the
wall time of the integration only (diagnostic_only).
"""
import json
import sys
import time
import traceback
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import numpy as np  # noqa: E402
import scenarios as S  # noqa: E402
from common import sanitize  # noqa: E402

MF = {"py_vode_adams_mf10": ("adams", False, False, False), "py_vode_bdf_mf21": ("bdf", True, True, False),
      "py_vode_bdf_mf22": ("bdf", True, False, False), "py_vode_bdf_mf24": ("bdf", True, True, True),
      "py_vode_bdf_mf25": ("bdf", True, False, True)}


class Counted:
    def __init__(self, fn):
        self.fn, self.calls = fn, 0

    def __call__(self, *a):
        self.calls += 1
        return self.fn(*a)


def dense_jac(cid, t, y):
    J = S.py_jac(cid, t, y)
    return J.toarray() if hasattr(J, "toarray") else np.asarray(J, float)


def packed_band(cid, t, y, bw):
    """jac_packed[i - j + uband, j] = J[i, j] for |i - j| <= bw (entries outside dropped)."""
    return packed_band_of(S.py_jac(cid, t, y), S.case_data(cid)["n"], bw)


def packed_band_of(J, n, bw):
    P = np.zeros((2 * bw + 1, n))
    if hasattr(J, "tocoo"):
        c = J.tocoo()
        rows, cols, vals = c.row, c.col, c.data
    else:
        J = np.asarray(J, float)
        rows, cols = np.nonzero(J)
        vals = J[rows, cols]
    keep = np.abs(rows - cols) <= bw
    np.add.at(P, (rows[keep] - cols[keep] + bw, cols[keep]), vals[keep])
    dropped = int(np.count_nonzero(~keep))
    return P, dropped


def vode_orientation(banded):
    """Which layout this scipy's VODE wrapper reads a user Jacobian in.

    scipy 1.11 (NCP) and 1.17 (local) differ; a nonsymmetric 6x6 linear
    test, y' = A y against expm, decides: "normal" (jac[i, j] = df_i/dy_j;
    packed[i - j + u, j]) or "transposed". Returns (choice, evidence).
    """
    from scipy.integrate import ode
    from scipy.linalg import expm
    n, bw = 6, 1
    A = np.diag(-(10.0 ** np.arange(n))) + np.diag(np.full(n - 1, 1e5), 1)  # stiff, one-way coupled
    y0 = np.ones(n)
    exact = expm(A * 0.5) @ y0
    ev = {}
    for choice in ("normal", "transposed"):
        M = A if choice == "normal" else A.T.copy()
        if banded:
            P = np.zeros((2 * bw + 1, n))
            for i in range(n):
                for j in range(max(0, i - bw), min(n, i + bw + 1)):
                    P[i - j + bw, j] = M[i, j]
            jac = Counted(lambda t, y, P=P: P)
        else:
            jac = Counted(lambda t, y, M=M: M)
        r = ode(lambda t, y: A @ y, jac)
        kw = dict(method="bdf", with_jacobian=True, rtol=1e-10, atol=1e-14, nsteps=20000)
        if banded:
            kw.update(lband=bw, uband=bw)
        r.set_integrator("vode", **kw)
        r.set_initial_value(y0, 0.0)
        import warnings
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            y = r.integrate(0.5)
        err = float(np.max(np.abs(y - exact)) / np.max(np.abs(exact)))
        ev[choice] = {"successful": bool(r.successful()), "rel_err": err, "njev": jac.calls}
    good = sorted((c for c in ("normal", "transposed") if ev[c]["successful"] and ev[c]["rel_err"] < 1e-7),
                  key=lambda c: ev[c]["njev"])
    if not good:
        return None, ev
    other = "transposed" if good[0] == "normal" else "normal"
    discriminated = (other not in good) or ev[other]["njev"] >= 3 * ev[good[0]]["njev"]
    ev["discriminated"] = bool(discriminated)
    return (good[0] if discriminated else None), ev


def run_vode(cid, arm, rtol):
    from scipy.integrate import ode
    d = S.case_data(cid)
    method, with_jac, user_jac, banded = MF[arm]
    orient, orient_ev = vode_orientation(banded) if user_jac else ("n/a", None)
    if user_jac and orient is None:
        return {"status": "ADAPTER_FAILURE", "reason": "VODE user-Jacobian layout calibration failed",
                "orientation_evidence": orient_ev}
    bw = d.get("bandwidth")
    atol = np.array(S.atol_of(d, rtol))
    f = Counted(lambda t, y: S.py_rhs(cid, t, y))
    dropped = {"max": 0}
    jac = None
    if user_jac:
        if banded:
            def _j(t, y):
                if orient == "transposed":
                    Jt = S.py_jac(cid, t, y)
                    Jt = Jt.T.tocoo() if hasattr(Jt, "tocoo") else np.asarray(Jt, float).T
                    P, k = packed_band_of(Jt, d["n"], bw)
                else:
                    P, k = packed_band(cid, t, y, bw)
                dropped["max"] = max(dropped["max"], k)
                return P
            jac = Counted(_j)
        elif orient == "transposed":
            jac = Counted(lambda t, y: np.ascontiguousarray(dense_jac(cid, t, y).T))
        else:
            jac = Counted(lambda t, y: dense_jac(cid, t, y))
    r = ode(f, jac)
    kw = dict(method=method, with_jacobian=with_jac, rtol=rtol, atol=atol, nsteps=10 ** 7)
    if banded:
        kw.update(lband=bw, uband=bw)
    r.set_integrator("vode", **kw)
    r.set_initial_value(np.asarray(d["y0"], float), 0.0)
    states, times = [list(map(float, d["y0"]))], [0.0]
    t0 = time.perf_counter()
    ok = True
    for t in d["outputs"][1:]:
        y = r.integrate(t)
        if not r.successful():
            ok = False
            break
        states.append([float(v) for v in y])
        times.append(float(t))
    solve = time.perf_counter() - t0
    integ = getattr(r, "_integrator", None)
    mf = getattr(integ, "mf", None) if integ is not None else None
    return {"status": "OK" if ok else "NUMERICAL_FAILURE", "reason": None if ok else f"istate={r.get_return_code()}",
            "times": times, "states": states, "solve_seconds": solve,
            "counters": {"nfev": f.calls, "njev": jac.calls if jac is not None else None,
                         "vode_mf": int(mf) if mf is not None else None, "jac_layout": orient,
                         "jac_layout_evidence": orient_ev,
                         "band_entries_dropped_max": dropped["max"] if banded and user_jac else None},
            "notes": "scipy ode('vode') itask=1: outputs by VODE interpolation; FD Jacobians are VODE's own"}


def run_dop853(cid, arm, rtol):
    from scipy.integrate import solve_ivp
    d = S.case_data(cid)
    atol = np.array(S.atol_of(d, rtol))
    f = Counted(lambda t, y: S.py_rhs(cid, t, y))
    kw = {}
    if arm == "py_dop853_tight":
        kw["max_step"] = 1e-3 * float(d["T"])
    t0 = time.perf_counter()
    sol = solve_ivp(f, (0.0, float(d["T"])), np.asarray(d["y0"], float), method="DOP853",
                    t_eval=d["outputs"], rtol=rtol, atol=atol, **kw)
    solve = time.perf_counter() - t0
    ok = bool(sol.success) and sol.y.shape[1] == len(d["outputs"])
    states = [[float(v) for v in sol.y[:, k]] for k in range(sol.y.shape[1])]
    return {"status": "OK" if ok else "NUMERICAL_FAILURE", "reason": None if ok else str(sol.message),
            "times": [float(t) for t in sol.t], "states": states, "solve_seconds": solve,
            "counters": {"nfev": int(sol.nfev), "solve_ivp_nfev": int(sol.nfev)},
            "config": {"max_step": kw.get("max_step")}}


def run_radau(cid, rtol):
    from scipy.integrate import solve_ivp
    d = S.case_data(cid)
    atol = np.array(S.atol_of(d, rtol))
    f = Counted(lambda t, y: S.py_rhs(cid, t, y))
    jac = Counted(lambda t, y: S.py_jac(cid, t, y))
    y = np.asarray(d["y0"], float)
    states, times = [[float(v) for v in y]], [0.0]
    nlu = 0
    t0 = time.perf_counter()
    outs = d["outputs"]
    for a, b in zip(outs[:-1], outs[1:]):
        sol = solve_ivp(f, (a, b), y, method="Radau", jac=jac, rtol=rtol, atol=atol)
        if not sol.success:
            return {"status": "NUMERICAL_FAILURE", "reason": str(sol.message), "times": times, "states": states,
                    "solve_seconds": time.perf_counter() - t0}
        y = sol.y[:, -1].copy()
        states.append([float(v) for v in y])
        times.append(float(b))
        nlu += int(getattr(sol, "nlu", 0))
    return {"status": "OK", "times": times, "states": states, "solve_seconds": time.perf_counter() - t0,
            "counters": {"nfev": f.calls, "njev": jac.calls, "nlu": nlu},
            "notes": "Radau restarted at every output (no dense-output interpolation)"}


def main():
    req = json.loads(Path(sys.argv[1]).read_text())
    cid, arm, rtol = req["case"], req["arm"], float(req["rtol"])
    t0 = time.perf_counter()
    out = {"case": cid, "arm": arm, "rtol": rtol, "timing_authority": "diagnostic_only"}
    try:
        S.case_data(cid)
        out["setup_seconds"] = time.perf_counter() - t0
        if arm in MF:
            out.update(run_vode(cid, arm, rtol))
        elif arm in ("py_dop853", "py_dop853_tight"):
            out.update(run_dop853(cid, arm, rtol))
        elif arm == "py_radau_ref":
            out.update(run_radau(cid, rtol))
        else:
            out.update(status="REFUSED", reason=f"unknown arm {arm}")
    except Exception:  # noqa: BLE001
        out.update(status="ADAPTER_FAILURE", reason=traceback.format_exc()[-2000:])
    print(json.dumps(sanitize(out), allow_nan=False))


if __name__ == "__main__":
    main()
