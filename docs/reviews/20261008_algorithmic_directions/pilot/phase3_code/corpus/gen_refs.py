"""Generate Radau references for the v2 calibration families at n = 384 / 1536 (no stored artifacts exist;
18 of the 22 manifest paths are missing in the tree).  Same ladder and uncertainty formula as
tools/reference_v2/generate_references_v2.py: Radau L0 (1e-8/1e-10), L1 (1e-10/1e-12), L2 (1e-12/1e-14) and
LSODA (3e-14/3e-16, banded Jacobian); D0 = W(L0,L1), D1 = W(L1,L2), q = D1/D0,
uncertainty = D1 q/(1-q) + W(L2, LSODA), W = max-grid anchor WRMS (1e-10, 1e-8) anchored on L2.
Writes refs/<problem_id>.npz.  EXPLORATORY (not the repository's reference authority).
Usage: python3 gen_refs.py N [family ...]"""
import os
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[_v] = "1"
import sys, time, json
import numpy as np
from scipy.integrate import solve_ivp
import corpus_v2 as cv

BAND = {"robertson-ramped": 2, "hires-ramped": 7, "van-der-pol-ramped": 1, "rotating-nonnormal": 1,
        "nonautonomous-stiff-forcing": 0}


def packed(Js, lo, up):
    A = Js.tocsc()
    n = A.shape[0]
    P = np.zeros((lo + up + 1, n))
    coo = A.tocoo()
    P[up + coo.row - coo.col, coo.col] = coo.data
    return P


def run(p, method, rtol, atol):
    t0 = time.perf_counter()
    if method == "Radau":
        ts, Y, info = cv.radau_reference(p, rtol, atol)
        ok = info["success"]; nfev = info["nfev"]; nlu = info["nlu"]
    else:
        bw = BAND.get(p.family, p.grid_shape[0] if p.grid_shape else None)
        r = solve_ivp(p.f, p.span, p.y0, method="LSODA", rtol=rtol, atol=atol, t_eval=p.output_times,
                      lband=bw, uband=bw, jac=lambda t, y: packed(p.J_sparse(t, y), bw, bw))
        ts, Y, ok, nfev, nlu = r.t, r.y.T, r.success, r.nfev, r.nlu
    return Y, {"method": method, "rtol": rtol, "atol": atol, "ok": bool(ok), "nfev": int(nfev), "nlu": int(nlu),
               "wall": time.perf_counter() - t0}


def main():
    n = int(sys.argv[1])
    fams = [cv.SHORT.get(a, a) for a in sys.argv[2:]] or list(cv.CALIBRATION_FAMILIES)
    os.makedirs(cv.GENERATED_REFERENCE_DIR, exist_ok=True)
    for fam in fams:
        p = cv.build(fam, n)
        out = os.path.join(cv.GENERATED_REFERENCE_DIR, p.name + ".npz")
        if os.path.exists(out):
            print("exists", out, flush=True); continue
        levels, ev = [], []
        for rt, at in ((1e-8, 1e-10), (1e-10, 1e-12), (1e-12, 1e-14)):
            Y, e = run(p, "Radau", rt, at); levels.append(Y); ev.append(e)
            print(f"{p.name} Radau {rt:.0e}: {e}", flush=True)
        L2 = levels[2]
        d0 = float((np.sqrt(np.mean(((levels[0] - levels[1]) / (1e-10 + 1e-8 * np.abs(L2))) ** 2, axis=1))).max())
        d1 = float((np.sqrt(np.mean(((levels[1] - L2) / (1e-10 + 1e-8 * np.abs(L2))) ** 2, axis=1))).max())
        q = d1 / d0
        rich = d1 * q / (1 - q)
        Yl, e = run(p, "LSODA", 3e-14, 3e-16); ev.append(e)
        print(f"{p.name} LSODA: {e}", flush=True)
        dis = float(cv.wrms_rows(Yl, L2).max()) if e["ok"] else float("nan")
        unc = rich + dis
        np.savez(out, times=p.output_times, states=L2, uncertainty_wrms=unc, d0=d0, d1=d1, q=q, richardson=rich,
                 disagreement=dis, method="Radau L2 1e-12/1e-14 (probe A2 regeneration)",
                 evidence=json.dumps(ev))
        print(f"WROTE {out} d0={d0:.3e} d1={d1:.3e} q={q:.3e} unc={unc:.3e} (rich {rich:.2e} + lsoda {dis:.2e})",
              flush=True)


if __name__ == "__main__":
    main()
