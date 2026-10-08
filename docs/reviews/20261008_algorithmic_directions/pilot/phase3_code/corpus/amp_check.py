"""Accuracy check of the exponential-midpoint propagator estimate (section d) on the worst family/interval:
refine to m substeps per grid interval using the dense Radau 1e-12 solution; also integrate the
variational equation exactly-ish with Radau for the 96 columns over the argmax interval."""
import numpy as np, corpus_v2 as cv
from scipy.linalg import expm
from scipy.integrate import solve_ivp
for fam, (ta, tb) in (("semilinear-advection-diffusion-ramped", (0.44, 0.73)), ("hires-ramped", (0.0, 1.0)),
                      ("robertson-ramped", (0.0, 0.02))):
    p = cv.build(fam, 96)
    _, _, info = cv.radau_reference(p, 1e-12, 1e-14, dense_output=True)
    sol = info["sol"][0]
    for m in (1, 4, 16):
        N = int(round((tb - ta) / ((p.span[1]-p.span[0]) / 100))) * m
        ts = np.linspace(ta, tb, N + 1)
        P = np.eye(p.n)
        for k in range(N):
            h = ts[k+1] - ts[k]; tm = ts[k] + h/2
            P = expm(h * p.J(tm, sol(tm))) @ P
        print(f"{fam} [{ta},{tb}] substeps x{m}: ||Phi||_2 = {np.linalg.norm(P, 2):.5g}")
    # variational equation with Radau (n + n^2 unknowns), dense J
    n = p.n
    def rhs(t, z):
        y = sol(t); Pm = z.reshape(n, n)
        return (p.J(t, y) @ Pm).ravel()
    r = solve_ivp(rhs, (ta, tb), np.eye(n).ravel(), method="LSODA", rtol=1e-9, atol=1e-12)
    print(f"{fam} [{ta},{tb}] variational LSODA 1e-9: ||Phi||_2 = {np.linalg.norm(r.y[:, -1].reshape(n, n), 2):.5g} ({r.message})")
