"""B6 control: why does the loose (1e-2) matrix-free res5 transport drift from the direct one?
Per accepted step, apply direct res5 and GMRES res5 at linear rtol {1e-2, 1e-4, 1e-6} to the SAME vector (the direct
transported ge), and also carry separate inexact transports.  Reports the per-application relative error
||M_mf v - M v|| / ||M v|| and the A_max reached by each carried transport.  EXPLORATORY."""
import sys, json, math
import numpy as np
import geamp as G

def diag(kind, name, rtol):
    p = G.get_problem(kind, name)
    atol = rtol * p.ascale; t0, tf = p.span; span = tf - t0
    t, y = t0, p.y0.copy(); h = span / 100.0 if kind == "corpus" else 1e-6
    n = p.n; ge = np.zeros(n); carried = {lt: np.zeros(n) for lt in (1e-2, 1e-4, 1e-6)}
    sumerr = 0.0; amax = {lt: 0.0 for lt in carried}; a_dir = 0.0
    relerr = {lt: [] for lt in carried}; last_rej = None; its = {lt: 0 for lt in carried}; nacc = 0
    while t < tf:
        h = min(h, tf - t, span)
        if kind != "corpus" and last_rej is not None and h >= last_rej:
            h = np.nextafter(t + last_rej, -np.inf) - t
        J = p.J(t, y); f0 = p.f(t, y); ftv = p.ft(t, y) if p.ft is not None else None
        ynew, err, U, lu, sc = G.attempt(p, J, f0, ftv, t, y, h, atol, rtol)
        if err <= 1:
            nacc += 1; sumerr += err
            Mv = G.res5(lu, h, ge)
            for lt in carried:
                tot = {}
                Mmf = G.res5_mf(J, h, ge, tot, lt)
                if np.linalg.norm(Mv) > 0:
                    relerr[lt].append(float(np.linalg.norm(Mmf - Mv) / np.linalg.norm(Mv)))
                tot2 = {}
                carried[lt] = G.res5_mf(J, h, carried[lt], tot2, lt) + U[-1]
                its[lt] += tot2.get("it", 0)
                amax[lt] = max(amax[lt], G.wn(carried[lt], sc) / sumerr)
            ge = Mv + U[-1]; a_dir = max(a_dir, G.wn(ge, sc) / sumerr)
            t = t + h if tf - (t + h) > 0 else tf; y = ynew; last_rej = None
            h *= min(max(0.9 * err ** -0.2, 0.2), 5.0)
        else:
            last_rej = h; h *= min(max(0.9 * err ** -0.2, 0.2), 0.9)
    out = {"row": f"{kind}:{name}:{rtol:g}", "accepted": nacc, "A_direct": a_dir}
    for lt in carried:
        r = np.array(relerr[lt])
        out[f"lin{lt:g}"] = {"A_carried": amax[lt], "rel_err_median": float(np.median(r)), "rel_err_max": float(r.max()),
                             "gmres_its_per_solve": its[lt] / (5 * nacc)}
    print(json.dumps(out), flush=True)
    return out

if __name__ == "__main__":
    res = [diag(*a.split(":")[:2], float(a.split(":")[2])) for a in sys.argv[1:]]
    json.dump(res, open(G.os.path.join(G.HERE, "mfdiag.json"), "w"), indent=1)
