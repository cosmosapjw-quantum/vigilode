"""Verify RODAS5P modal contraction against the exact raw tableau and time flow."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
from fractions import Fraction as F
from pathlib import Path

SOURCE = "fixtures/rodas5p_coefficients_snapshot.json"
DIGEST = "211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6"


def tableau(repo):
    data = (repo / SOURCE).read_bytes()
    assert hashlib.sha256(data).hexdigest() == DIGEST
    raw = json.loads(data)
    matrix = lambda key: [[F(v) for v in row] for row in raw[key]]
    return F(raw["gamma"]), matrix("A"), matrix("C"), [F(v) for v in raw["b_code"]]


def polynomial(tab):
    # x = gamma*z/(1-gamma*z), p=1+x. Raw increments obey
    # U_i = x*(1+sum_j A_ij U_j) + gamma*(1+x)*sum_j C_ij U_j.
    gamma, a, c, b = tab
    stages = []
    for i in range(8):
        u = [F(0)] * 9
        u[1] = F(1)
        for j in range(i):
            for k in range(8):
                u[k] += gamma * c[i][j] * stages[j][k]
                u[k+1] += (a[i][j] + gamma * c[i][j]) * stages[j][k]
        stages.append(u)
    high = [F(int(k == 0)) + sum(b[i] * stages[i][k] for i in range(8)) for k in range(9)]
    error = stages[-1]
    shift = lambda v: [sum(v[k] * math.comb(k,j) * (-1)**(k-j) for k in range(j,9)) for j in range(9)]
    return dict(STEP_NEAR=high, ERROR_NEAR=error, STEP_STIFF=shift(high), ERROR_STIFF=shift(error))


def raw_step(z, tab):
    gamma, a, c, b = tab
    stages = []
    for i in range(8):
        state = 1 + sum(a[i][j]*stages[j] for j in range(i))
        mix = sum(c[i][j]*stages[j] for j in range(i))
        stages.append(gamma*(z*state+mix)/(1-gamma*z))
    return 1+sum(b[i]*stages[i] for i in range(8)), stages[-1]


def estrin(a, x):
    x2 = x*x
    x4 = x2*x2
    left = (a[0]+x*a[1]) + x2*(a[2]+x*a[3])
    right = (a[4]+x*a[5]) + x2*(a[6]+x*a[7])
    return left+x4*(right+x4*a[8])


def transfer(z, gamma, tables):
    u = gamma*z
    if u >= -1:
        x = u/(1-u)
        return estrin(tables['STEP_NEAR'],x), estrin(tables['ERROR_NEAR'],x)
    p = 1/(1-u)
    return estrin(tables['STEP_STIFF'],p), estrin(tables['ERROR_STIFF'],p)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("repo", type=Path)
    parser.add_argument("--rust", type=Path)
    parser.add_argument("--emit", action="store_true")
    args = parser.parse_args()
    tab = tableau(args.repo)
    gamma = float(tab[0])
    tables = polynomial(tab)
    b64 = {k: list(map(float,v)) for k,v in tables.items()}
    if args.emit:
        for name, v in b64.items():
            print("const " + name + ": [f64; 9] = [" + ", ".join(repr(x) for x in v) + "];")
        return
    src = (args.rust or (args.repo / 'crates/vigilode/src/modal_rodas5p.rs')).read_text()
    rust_gamma = float(re.search(r'const GAMMA: f64 = ([\d.e+-]+);', src).group(1))
    assert rust_gamma == gamma
    for name, coeffs in b64.items():
        pattern = r'const '+name+r': \[f64; 9\] = \[(.*?)\];'
        stored = re.search(pattern, src, re.S).group(1)
        assert [float(x) for x in stored.split(",") if x.strip()] == coeffs
    maxima = [0.,0.]
    rows = [0., -1/gamma, math.nextafter(-1/gamma,0), math.nextafter(-1/gamma,-math.inf)]
    rows += [-10.**(j/8) for j in range(-128,161)]
    for z in rows:
        exact = raw_step(F.from_float(z),tab)
        got = transfer(z,gamma,b64)
        for i in range(2):
            maxima[i] = max(maxima[i], abs(got[i]-float(exact[i])))
    assert max(maxima) < 2e-14
    # Low modes over one unit of time. Order six is the autonomous-linear
    # superconvergence of this tableau; the embedded local defect is order five.
    rates = []
    for steps in (4,8,16,32):
        r, e = transfer(-4/steps,gamma,b64)
        rates.append(dict(steps=steps,endpoint_abs=abs(r**steps-math.exp(-4)),embedded_abs=abs(e)))
    assert all(rates[j]['endpoint_abs'] > 40*rates[j+1]['endpoint_abs'] for j in (1,2))
    assert all(rates[j]['embedded_abs'] > 12*rates[j+1]['embedded_abs'] for j in (1,2))
    # Rotating eigenbasis counterexample. Even exact frozen eigenvalue multipliers
    # miss -Q^T Q' in modal coordinates. Closed form for real two-by-two B:
    # rotating frame B=[[-1,omega],[-omega,-9]], x(t)=Q(t) exp(t B) (1,0).
    time, omega = 0.2, 2.
    kappa = math.sqrt(16-omega**2)
    v0 = math.exp(-5*time)*(math.cosh(kappa*time)+4*math.sinh(kappa*time)/kappa)
    v1 = -math.exp(-5*time)*omega*math.sinh(kappa*time)/kappa
    physical = (math.cos(omega*time)*v0-math.sin(omega*time)*v1,
                math.sin(omega*time)*v0+math.cos(omega*time)*v1)
    wrong = (math.exp(-time), 0.)
    counterexample_error = math.dist(physical, wrong)
    assert counterexample_error > 0.05
    print(json.dumps(dict(snapshot_sha256=DIGEST,exact_raw_cases=len(rows),
                     rotating_basis_frozen_flow_error=counterexample_error,
                     max_abs_endpoint_error=maxima[0],max_abs_signed_embedded_error=maxima[1],
                     temporal_convergence=rates,stiff_chart_constants=[b64['STEP_STIFF'][0],
                     b64['ERROR_STIFF'][0]],scope='homogeneous real nonpositive eigenvalues'),indent=2))


if __name__ == "__main__":
    main()
