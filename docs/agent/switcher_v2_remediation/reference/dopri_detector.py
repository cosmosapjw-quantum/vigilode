"""Prototype: zero-cost, direction-aware stability-limit detector for DOPRI5.

For each accepted DOPRI5 step, stages 6 and 7 are both evaluated at t+h, so
k7-k6 = J (Y7-Y6) + O(|Y7-Y6|^2) with no time-derivative contamination.
Two such secant pairs (this step and the previous one) give a 2x2 least-squares
estimate of J on span{d_prev, d_now}; its eigenvalues lam_hat are placed on the
exact DOPRI5 stability polynomial R(z) = sum_{k<=5} z^k/k! + z^6/600.
A step is "stability-limited" when h|lam_hat| / r(arg lam_hat) >= theta (theta = 0.8),
r(phi) being the exact boundary radius of the stability region along the ray phi.
(An earlier criterion max |R(h lam_hat)| >= 1 - delta was wrong for oscillatory modes and was replaced.)
Also reported: Hairer's classical scalar test h*||k7-k6||/||Y7-Y6|| > 3.25.
No extra RHS evaluation is used.
"""
import json
import numpy as np

A = [[], [1/5], [3/40, 9/40], [44/45, -56/15, 32/9],
     [19372/6561, -25360/2187, 64448/6561, -212/729],
     [9017/3168, -355/33, 46732/5247, 49/176, -5103/18656],
     [35/384, 0, 500/1113, 125/192, -2187/6784, 11/84]]
Cn = [0, 1/5, 3/10, 4/5, 8/9, 1, 1]
E = np.array([71/57600, 0, -71/16695, 71/1920, -17253/339200, 22/525, -1/40])
R = lambda z: 1 + z + z**2/2 + z**3/6 + z**4/24 + z**5/120 + z**6/600
COEF = [1, 1, 1/2, 1/6, 1/24, 1/120, 1/600]


def boundary_radius(phi):
    """Smallest r>0 with |R(r e^{i phi})|^2 = 1 (exact polynomial root isolation)."""
    p = np.array([c * np.exp(1j * k * phi) for k, c in enumerate(COEF)])  # R(r e^{i phi}) = sum p_k r^k
    # |R|^2 - 1 = sum_{j,k} p_j conj(p_k) r^{j+k} - 1, real polynomial of degree 12
    q = np.zeros(13)
    for j in range(7):
        for k in range(7):
            q[j + k] += (p[j] * np.conj(p[k])).real
    q[0] -= 1.0
    q = q / r_scale if False else q
    roots = np.roots(q[::-1])
    pos = [r.real for r in roots if abs(r.imag) < 1e-9 and r.real > 1e-6]
    # skip the tangency at r=0 region: take the first root where |R| crosses from <=1 to >1
    pos.sort()
    for r in pos:
        if abs(R((r * 1.001) * np.exp(1j * phi))) > 1.0:
            return r
    return pos[-1] if pos else np.inf


PHI = np.linspace(np.pi / 2, np.pi, 181)
RAD = np.array([boundary_radius(ph) for ph in PHI])


def stab_fraction(hl):
    """|h lambda| / r(arg h lambda), arg folded into [pi/2, pi]."""
    ph = abs(np.angle(hl))
    if ph < np.pi / 2:
        return 0.0  # growing modes: not a stability limit of interest
    return abs(hl) / np.interp(ph, PHI, RAD)


def run(f, jac, y0, T, rtol, atol, h0, windows=8, theta=0.8):
    t, y, h = 0.0, np.array(y0, float), h0
    k1 = f(t, y)
    prev = None
    rows = []
    rhs = 1
    while t < T:
        h = min(h, T - t)
        K = [k1]
        Ys = [y]
        for i in range(1, 7):
            Yi = y + h * sum(a * k for a, k in zip(A[i], K))
            Ys.append(Yi)
            K.append(f(t + Cn[i] * h, Yi))
            rhs += 1
        ynew = Ys[6]
        err_vec = h * sum(e * k for e, k in zip(E, K))
        sc = atol + rtol * np.maximum(np.abs(y), np.abs(ynew))
        err = np.sqrt(np.mean((err_vec / sc) ** 2))
        if err <= 1.0:
            d = Ys[6] - Ys[5]
            g = K[6] - K[5]
            nd = np.linalg.norm(d)
            hair = h * np.linalg.norm(g) / max(nd, 1e-300)
            lam_hat = None
            if prev is not None and nd > 0 and y.size >= 2:
                D = np.column_stack([prev[0], d])
                F = np.column_stack([prev[1], g])
                s = np.linalg.svd(D, compute_uv=False)
                if s[-1] > 1e-8 * s[0]:
                    # least-squares J_proj with D J_proj ~ F in D coordinates: J D c = F c
                    Jp = np.linalg.lstsq(D, F, rcond=None)[0]
                    lam_hat = np.linalg.eigvals(Jp)
            if lam_hat is None and nd > 0:
                lam_hat = np.array([np.dot(d, g) / np.dot(d, d)])
            sh = max(stab_fraction(h * l) for l in lam_hat) if lam_hat is not None else 0.0
            lh = max(lam_hat, key=lambda l: stab_fraction(h * l)) if lam_hat is not None else 0.0
            ev = np.linalg.eigvals(np.atleast_2d(jac(t + h, ynew)))
            st = max(stab_fraction(h * l) for l in ev)
            diss = abs(np.angle(lh)) >= 3 * np.pi / 4
            rows.append(dict(t=t + h, h=h, hair=hair, Rhat=sh, Rtrue=st, diss=diss, err=float(err)))
            prev = (d, g)
            t, y, k1 = t + h, ynew, K[6]
        fac = 0.9 * max(err, 1e-10) ** (-0.2)
        h = h * min(10.0, max(0.2, fac))
    win = []
    for w in range(windows):
        lo, hi = T * w / windows, T * (w + 1) / windows
        rs = [r for r in rows if lo < r["t"] <= hi]
        if not rs:
            win.append(None)
            continue
        win.append(dict(steps=len(rs),
                        frac_stab_limited_hat=float(np.mean([r["Rhat"] >= theta for r in rs])),
                        frac_dissipative_limited=float(np.mean([r["Rhat"] >= theta and r["diss"] for r in rs])),
                        frac_stab_limited_true=float(np.mean([r["Rtrue"] >= theta for r in rs])),
                        frac_hairer=float(np.mean([r["hair"] > 3.25 for r in rs])),
                        median_err_pow=float(np.median([max(r['err'],1e-16) ** 0.2 for r in rs])),
                        mean_h=float(np.mean([r['h'] for r in rs]))))
    agree = float(np.mean([(r["Rhat"] >= theta) == (r["Rtrue"] >= theta) for r in rows]))
    return dict(steps=len(rows), rhs=rhs, step_agreement=agree, windows=win)


def models():
    rot = (lambda t, y: np.array([-100 * y[1], 100 * y[0]]),
           lambda t, y: np.array([[0, -100.0], [100.0, 0]]), [1.0, 0.0], 1.0, 1e-7, 1e-10)
    rot_fast = (lambda t, y: np.array([-1e4 * y[1], 1e4 * y[0]]),
                lambda t, y: np.array([[0, -1e4], [1e4, 0]]), [1.0, 0.0], 0.01, 1e-3, 1e-6)
    sf = (lambda t, y: np.array([-1e4 * (y[0] - np.cos(t)) - np.sin(t)]),
          lambda t, y: np.array([[-1e4]]), [1.0], 1.0, 1e-7, 1e-10)

    def trans(tau):
        lam = lambda t: 5e4 * (1 + np.tanh(60 * (t - tau)))

        def f(t, y):
            l = lam(t)
            s, c = np.sin(100 * t), np.cos(100 * t)
            return np.array([-100 * y[1] - l * (y[0] - c), 100 * y[0] - l * (y[1] - s)])

        def J(t, y):
            l = lam(t)
            return np.array([[-l, -100.0], [100.0, -l]])
        return (f, J, [1.0, 0.0], 1.0, 1e-5, 1e-13)
    return {"rotation": rot, "rotation-fast(omega=1e4,rtol=1e-3)": rot_fast, "stiff-forced": sf,
            "transition-39": trans(0.39), "transition-79": trans(0.79)}


if __name__ == "__main__":
    print("boundary radius r(pi)=%.6f r(3pi/4)=%.6f r(pi/2)=%.6f" % (np.interp(np.pi, PHI, RAD), np.interp(3*np.pi/4, PHI, RAD), RAD[0]))
    out = {}
    for name, (f, J, y0, T, rtol, atol) in models().items():
        res = run(f, J, y0, T, rtol, atol, h0=1e-7)
        out[name] = res
        print(name, "steps", res["steps"], "rhs", res["rhs"], "step agreement hat/true", round(res["step_agreement"], 3))
        for w, r in enumerate(res["windows"]):
            if r:
                print(f"  w{w}: steps={r['steps']:6d} stab-limited hat={r['frac_stab_limited_hat']:.2f} "
                      f"true={r['frac_stab_limited_true']:.2f} dissipative={r['frac_dissipative_limited']:.2f} hairer={r['frac_hairer']:.2f}")
    json.dump(out, open("dopri_detector_results.json", "w"), indent=1)
