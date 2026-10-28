"""Replicate bordered_probe arithmetic (same op order) in pure Python float64 + exact rational check."""
import json, sys
from fractions import Fraction as Fr
import numpy as np


def probe(n):
    m = 2
    a = [0.0] * (3 * n); b = [0.0] * (2 * n); c = [0.0] * (2 * n)
    d = [2.0, 0.125, 0.125, 2.0]
    for i in range(n):
        a[3 * i] = -0.5 if i > 0 else 0.0
        a[3 * i + 1] = 3.0
        a[3 * i + 2] = -0.5 if i + 1 < n else 0.0
        b[2 * i] = 0.125
        b[2 * i + 1] = 0.0625 if i % 2 == 0 else -0.0625
        c[i] = b[2 * i] / n
        c[n + i] = b[2 * i + 1] / n

    def apply(x):
        y = [0.0] * (n + m)
        for i in range(n):
            y[i] = 3.0 * x[i] + b[2 * i] * x[n] + b[2 * i + 1] * x[n + 1]
            if i > 0:
                y[i] -= 0.5 * x[i - 1]
            if i + 1 < n:
                y[i] -= 0.5 * x[i + 1]
        for j in range(2):
            y[n + j] = d[2 * j] * x[n] + d[2 * j + 1] * x[n + 1]
            for i in range(n):
                y[n + j] += c[j * n + i] * x[i]
        return y

    exact = [((i % 17) - 8.0) * 0.125 for i in range(n + m)]
    rhs = apply(exact)
    # factor (lower=upper=1, stride 3)
    L, U, S = 1, 1, 3
    lu = list(a)
    for k in range(n):
        piv = lu[k * S + L]
        for i in range(k + 1, min(n, k + L + 1)):
            ik = i * S + k + L - i
            lu[ik] /= piv
            for j in range(k + 1, min(n, k + U + 1)):
                lu[i * S + j + L - i] -= lu[ik] * lu[k * S + j + L - k]

    def bsolve(x):
        for i in range(n):
            for j in range(max(0, i - L), i):
                x[i] -= lu[i * S + j + L - i] * x[j]
        for i in range(n - 1, -1, -1):
            for j in range(i + 1, min(n, i + U + 1)):
                x[i] -= lu[i * S + j + L - i] * x[j]
            x[i] /= lu[i * S + L]

    W = [0.0] * (n * m)
    for j in range(m):
        col = [b[i * m + j] for i in range(n)]
        bsolve(col)
        for i in range(n):
            W[i * m + j] = col[i]
    sch = [0.0] * 4
    for i in range(m):
        for j in range(m):
            s = d[i * m + j]
            for k in range(n):
                s -= c[i * n + k] * W[k * m + j]
            sch[i * m + j] = s
    piv = [0, 0]
    for k in range(m):
        p = k
        for i in range(k + 1, m):
            if abs(sch[i * m + k]) > abs(sch[p * m + k]):
                p = i
        piv[k] = p
        pv = sch[p * m + k]
        for j in range(m):
            sch[k * m + j], sch[p * m + j] = sch[p * m + j], sch[k * m + j]
        for i in range(k + 1, m):
            sch[i * m + k] /= pv
            for j in range(k + 1, m):
                sch[i * m + j] -= sch[i * m + k] * sch[k * m + j]
    y = list(rhs[:n]) + [0.0, 0.0]
    yy = y[:n]
    bsolve(yy)
    y[:n] = yy
    for i in range(m):
        r = rhs[n + i]
        for k in range(n):
            r -= c[i * n + k] * y[k]
        y[n + i] = r
    for k in range(m):
        y[n + k], y[n + piv[k]] = y[n + piv[k]], y[n + k]
    for i in range(m):
        for j in range(i):
            y[n + i] -= sch[i * m + j] * y[n + j]
    for i in range(m - 1, -1, -1):
        for j in range(i + 1, m):
            y[n + i] -= sch[i * m + j] * y[n + j]
        y[n + i] /= sch[i * m + i]
    for i in range(n):
        for j in range(m):
            y[i] -= W[i * m + j] * y[n + j]
    res = apply(y)
    maxerr = max(abs(p - q) for p, q in zip(y, exact))
    resinf = max(abs(p - q) for p, q in zip(res, rhs))
    # dense numpy solve of the same (rounded) matrix for comparison
    M = np.zeros((n + m, n + m))
    for i in range(n):
        M[i, i] = 3.0
        if i > 0: M[i, i - 1] = -0.5
        if i + 1 < n: M[i, i + 1] = -0.5
        M[i, n] = b[2 * i]; M[i, n + 1] = b[2 * i + 1]
    for j in range(2):
        M[n + j, n] = d[2 * j]; M[n + j, n + 1] = d[2 * j + 1]
        M[n + j, :n] = c[j * n:(j + 1) * n]
    ynp = np.linalg.solve(M, np.array(rhs))
    # true error of computed rhs vs exact*M in exact arithmetic (rhs rounding)
    rhs_err = max(abs(float(sum((Fr(M[r, k]) * Fr(exact[k]) for k in range(n + m) if M[r, k] != 0), Fr(0)) - Fr(rhs[r]))) for r in (n, n + 1))
    return dict(n=n, slots=3 * n + 2 * n * m + m * m + n, max_solution_error=maxerr, residual_inf=resinf,
                numpy_dense_err_vs_exact=float(np.max(np.abs(ynp - np.array(exact)))),
                cond=float(np.linalg.cond(M)) if n <= 1600 else None, rhs_matter_rounding_err=rhs_err)


for n in [int(x) for x in sys.argv[1:]]:
    print(json.dumps(probe(n)))
