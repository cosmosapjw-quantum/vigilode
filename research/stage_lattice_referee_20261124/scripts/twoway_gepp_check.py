#!/usr/bin/env python3
"""Two-way banded elimination equals partial pivoting on a symmetrically permuted matrix.

Claim (node SL-R5b). Let W be banded with half widths (l, u), split at row m with
l <= m <= n - u, K = m - l. The two-way elimination

  top    : columns 0 .. K-1 in ascending order, pivot among rows k .. k+l
  bottom : columns n-1 .. m+u in descending order, pivot among rows j-u .. j
  middle : dense partial pivoting on the remaining (l + u) x (l + u) block

performs exactly Gaussian elimination with partial pivoting on Q W Q^T, where Q orders the
unknowns as [0..K-1, n-1..m+u, K..m+u-1]. The pivot candidates of the two fronts never
include a row of the other front, so the two fronts are independent of each other.

Checks, on seeded synthetic matrices (diagonally dominant, weak diagonal, pivoting-hostile,
nearly singular shifted Laplacian, stage-like W = I/(h gamma) - J of a 1D two-species
reaction-diffusion Jacobian):

  E1  the pivot sequence of the two-way elimination equals the pivot sequence of an
      independent dense partial-pivoting elimination of Q W Q^T (ties excluded by the data);
  E2  normwise backward error eta = ||b - W x|| / (||W|| ||x|| + ||b||) of the two-way solve
      next to LAPACK gbsv (scipy.linalg.solve_banded);
  E3  the two fronts touch disjoint row sets (asserted during the elimination).

Exit status 0 if every case passes its checks.
"""
from __future__ import annotations

import json
import pathlib
import sys

import numpy as np
import scipy.linalg as sla


def banded(n, l, u, kind, rng):
    w = np.zeros((n, n))
    for i in range(n):
        for j in range(max(0, i - l), min(n, i + u + 1)):
            w[i, j] = rng.standard_normal()
    if kind == "dominant":
        w += np.diag(np.sum(np.abs(w), axis=1) + 1.0)
    elif kind == "weak_diagonal":
        w[np.diag_indices(n)] *= 1e-3
    elif kind == "hostile":
        w[np.diag_indices(n)] = 1e-9 * rng.standard_normal(n)
    elif kind == "shifted_laplacian":
        w = np.zeros((n, n))
        for i in range(n):
            w[i, i] = 2.0 + 1e-8
            for d in range(1, min(l, u) + 1):
                if i - d >= 0:
                    w[i, i - d] = -1.0 / min(l, u)
                if i + d < n:
                    w[i, i + d] = -1.0 / min(l, u)
    elif kind == "stage_like":
        # two interleaved species on a line: diffusion couples k and k +- 2, reaction k and k +- 1
        inv = 1.0 / (0.2119 * 10 ** rng.uniform(-4, 1))
        jac = np.zeros((n, n))
        for i in range(n):
            d = 40.0 * (1.0 + 0.3 * rng.standard_normal())
            jac[i, i] = -2.0 * d - abs(rng.standard_normal()) * 5.0
            for off, val in ((-2, d), (2, d), (-1, rng.standard_normal()), (1, rng.standard_normal())):
                if 0 <= i + off < n and -l <= off <= u:
                    jac[i, i + off] = val
        w = inv * np.eye(n) - jac
    else:
        raise ValueError(kind)
    return w


def dense_gepp_pivots(a):
    """Plain partial pivoting; returns the pivot row chosen at every column (as an index of
    the working matrix before the interchange) and the factors, for the cross-check."""
    a = a.copy()
    n = a.shape[0]
    piv = []
    for k in range(n):
        p = k + int(np.argmax(np.abs(a[k:, k])))
        piv.append(p)
        if p != k:
            a[[k, p]] = a[[p, k]]
        a[k + 1:, k] /= a[k, k]
        a[k + 1:, k + 1:] -= np.outer(a[k + 1:, k], a[k, k + 1:])
    return piv, a


def twoway(w, b, l, u):
    """Reference implementation of the two-way elimination on dense storage."""
    n = w.shape[0]
    a = w.copy()
    x = b.astype(float).copy()
    m = n // 2
    K = m - l
    M0 = m + u
    assert K >= 0 and M0 <= n
    rows_top, rows_bot = set(), set()
    piv_top, piv_bot = [], []
    # top front: ascending columns
    for k in range(K):
        cand = range(k, min(k + l, m - 1) + 1)
        p = max(cand, key=lambda r: abs(a[r, k]))
        piv_top.append(p)
        rows_top.update(cand)
        if p != k:
            a[[k, p]] = a[[p, k]]
            x[[k, p]] = x[[p, k]]
        for i in range(k + 1, min(k + l, m - 1) + 1):
            f = a[i, k] / a[k, k]
            a[i, k:] -= f * a[k, k:]
            a[i, k] = 0.0
            x[i] -= f * x[k]
    # bottom front: descending columns
    for j in range(n - 1, M0 - 1, -1):
        cand = range(max(j - u, m), j + 1)
        p = max(cand, key=lambda r: abs(a[r, j]))
        piv_bot.append(p)
        rows_bot.update(cand)
        if p != j:
            a[[j, p]] = a[[p, j]]
            x[[j, p]] = x[[p, j]]
        for i in range(max(j - u, m), j):
            f = a[i, j] / a[j, j]
            a[i, :j + 1] -= f * a[j, :j + 1]
            a[i, j] = 0.0
            x[i] -= f * x[j]
    disjoint = rows_top.isdisjoint(rows_bot)
    # middle block
    mid = list(range(K, M0))
    block = a[np.ix_(mid, mid)]
    outside = np.delete(a[mid, :], mid, axis=1)
    assert np.all(outside == 0.0), "the leftover rows must live on the middle columns"
    xm = sla.solve(block, x[mid]) if mid else np.zeros(0)
    sol = np.zeros(n)
    sol[mid] = xm
    for k in range(K - 1, -1, -1):
        sol[k] = (x[k] - a[k, k + 1:] @ sol[k + 1:]) / a[k, k]
    for j in range(M0, n):
        sol[j] = (x[j] - a[j, :j] @ sol[:j]) / a[j, j]
    return sol, piv_top, piv_bot, disjoint, (K, M0)


def eta(w, x, b):
    return float(np.max(np.abs(b - w @ x)) / (np.linalg.norm(w, np.inf) * np.max(np.abs(x)) + np.max(np.abs(b))))


def main() -> int:
    rng = np.random.default_rng(20261124)
    cases = []
    ok = True
    for kind in ("dominant", "weak_diagonal", "hostile", "shifted_laplacian", "stage_like"):
        for (n, l, u) in ((24, 1, 1), (61, 2, 2), (120, 2, 2), (97, 3, 1), (150, 1, 4), (200, 6, 6)):
            w = banded(n, l, u, kind, rng)
            b = rng.standard_normal(n)
            sol, piv_top, piv_bot, disjoint, (K, M0) = twoway(w, b, l, u)
            # the permuted matrix and its independent dense elimination
            order = list(range(K)) + list(range(n - 1, M0 - 1, -1)) + list(range(K, M0))
            q = np.array(order)
            wq = w[np.ix_(q, q)]
            dense_piv, _ = dense_gepp_pivots(wq)
            pos = {orig: idx for idx, orig in enumerate(order)}
            # Translate the fronts' pivot rows into positions of the permuted working matrix.
            # Interchanges within a front keep its rows inside the front's own position range,
            # so a row label of the two-way elimination maps to the same working position.
            mine = [pos[p] for p in piv_top] + [pos[p] for p in piv_bot]
            pivots_equal = mine == dense_piv[:len(mine)]
            ab = np.zeros((l + u + 1, n))
            for i in range(n):
                for j in range(max(0, i - l), min(n, i + u + 1)):
                    ab[u + i - j, j] = w[i, j]
            ref = sla.solve_banded((l, u), ab, b)
            row = {
                "kind": kind, "n": n, "l": l, "u": u,
                "eta_twoway": eta(w, sol, b), "eta_lapack": eta(w, ref, b),
                "relative_difference": float(np.max(np.abs(sol - ref)) / np.max(np.abs(ref))),
                "condition_inf": float(np.linalg.cond(w, np.inf)),
                "pivots_equal_dense_gepp_of_permuted": bool(pivots_equal),
                "fronts_disjoint": bool(disjoint),
                "interchanges_top": int(sum(p != k for k, p in enumerate(piv_top))),
            }
            # eta of a backward-stable solve: a modest multiple of the unit roundoff
            row["pass"] = bool(pivots_equal and disjoint and row["eta_twoway"] <= max(4 * row["eta_lapack"], 2e-15))
            ok &= row["pass"]
            cases.append(row)
    worst = max(c["eta_twoway"] for c in cases)
    worst_ref = max(c["eta_lapack"] for c in cases)
    out = {
        "schema": "vigilode.referee2.twoway-gepp.v1",
        "seed": 20261124,
        "cases": cases,
        "summary": {
            "cases": len(cases),
            "passed": sum(c["pass"] for c in cases),
            "pivot_sequences_equal": sum(c["pivots_equal_dense_gepp_of_permuted"] for c in cases),
            "worst_eta_twoway": worst,
            "worst_eta_lapack_gbsv": worst_ref,
        },
    }
    text = json.dumps(out, indent=1, sort_keys=True)
    if len(sys.argv) > 1:
        pathlib.Path(sys.argv[1]).write_text(text + "\n")
    print(json.dumps(out["summary"], indent=1))
    for c in cases:
        if not c["pass"]:
            print("FAIL", c)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
