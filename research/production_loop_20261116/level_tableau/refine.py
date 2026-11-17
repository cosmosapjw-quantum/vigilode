#!/usr/bin/env python3
"""Loop L1 node V16 -- multiprecision refinement of the best search results and coefficient files.

    python3 -I refine.py D6T logs/polish_D6T.jsonl logs/continuation_D6T.jsonl > logs/refine_D6T.jsonl
    python3 -I refine.py D5F logs/polish_D5F.jsonl                             > logs/refine_D5F.jsonl

Pool: rows of kind polish / continuation that pass the double-precision screen; the 8 best by score with
pairwise max-norm distance > 1e-3. Refinement: mpmath at 60 digits, simplified Newton on the hard conditions
with the pseudo-inverse of the binary64 Jacobian (minimum-norm corrections), at most 25 iterations.
The file stores the independent coefficients rounded to 40 significant digits; the dependent ones (tied row,
b, bhat) are exact decimal sums of those strings, so the structural identities hold exactly in the file.
"""
import decimal
import json
import os
import sys

os.environ.setdefault("OMP_NUM_THREADS", "1")
os.environ.setdefault("OPENBLAS_NUM_THREADS", "1")
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import numpy as np  # noqa: E402
from mpmath import mp, mpf  # noqa: E402

import layouts  # noqa: E402

mp.dps = 60
DIGITS = 40
TOP = 8
DIST = 1e-3
decimal.getcontext().prec = 200


def emit(**kw):
    print(json.dumps(kw, allow_nan=False), flush=True)


def pool(paths):
    rows = []
    for p in paths:
        for line in open(p):
            r = json.loads(line)
            if r.get("kind") in ("polish", "continuation") and r.get("pass_double") and "score" in r:
                r["_log"] = os.path.basename(p)
                rows.append(r)
    rows.sort(key=lambda r: r["score"])
    chosen = []
    for r in rows:
        x = np.array([float(v) for v in r["x"]])
        if all(np.abs(x - c[1]).max() > DIST for c in chosen):
            chosen.append((r, x))
        if len(chosen) == TOP:
            break
    return chosen, len(rows)


def refine(F, x):
    Jp = np.linalg.pinv(F.hard_jac(x), rcond=1e-12)
    Jm = [[mpf(float(v)) for v in row] for row in Jp]
    xm = [mpf(float(v)) for v in x]
    hist = []
    for it in range(1, 26):
        h = F.hard(np.array([xm], dtype=object))[0]
        r = max(abs(v) for v in h)
        hist.append(float(r))
        if r <= mpf(10) ** -45:
            break
        for i in range(F.n):
            xm[i] -= mp.fsum(Jm[i][j] * h[j] for j in range(F.m_hard))
    return xm, hist


def dstr(v):
    """mpf -> decimal string with DIGITS significant digits."""
    return mp.nstr(v, DIGITS, min_fixed=-mp.inf, max_fixed=mp.inf) if v != 0 else "0"


def dsum(a, b, sign=1):
    r = decimal.Decimal(a) + sign * decimal.Decimal(b)
    return format(r, "f") if r != 0 else "0"


def write_candidate(F, xm, cid, meta):
    s = F.s
    o = 0
    if F.has_gamma:
        gamma = dstr(xm[0])
        o = 1
    else:
        gamma = F.gamma_fixed_str
    alpha = [["0"] * s for _ in range(s)]
    gam = [["0"] * s for _ in range(s)]
    for (i, j), v in zip(F.alpha_pairs, xm[o:o + F.n_alpha]):
        alpha[i][j] = dstr(v)
    o += F.n_alpha
    for (i, j), v in zip(F.gam_pairs, xm[o:o + F.n_gam]):
        gam[i][j] = dstr(v)
    o += F.n_gam
    for i in range(s):
        gam[i][i] = gamma
    if F.tied:
        alpha[s - 1] = [dsum(alpha[s - 2][j], gam[s - 2][j]) for j in range(s)]
    b = [dsum(alpha[s - 1][j], gam[s - 1][j]) for j in range(s)]
    if F.has_d:
        bhat = [dsum(b[j], dstr(xm[o + j]), -1) for j in range(s)]
    else:
        bhat = [dsum(alpha[s - 2][j], gam[s - 2][j]) for j in range(s)]
    doc = {"schema": "v16-candidate-v1", "id": cid, "formulation": F.id, "stages": s, "levels": F.levels,
           "convention": "k-form of theory S9: (I - h gamma J) k_i = h f(y + sum_j alpha_ij k_j) + h J sum_{j<i} "
                         "Gamma_ij k_j; Gamma has gamma on the diagonal; beta = alpha + Gamma; b = last row of beta; "
                         "y_hat = y + sum_i bhat_i k_i",
           "significant_digits": DIGITS, "gamma": gamma, "alpha": alpha, "Gamma": gam, "b": b, "bhat": bhat,
           "provenance": meta}
    path = os.path.join(HERE, "candidates", cid + ".json")
    with open(path, "w") as fh:
        json.dump(doc, fh, indent=1, allow_nan=False)
        fh.write("\n")
    return path


def main():
    fid = sys.argv[1]
    paths = sys.argv[2:]
    prefix = os.environ.get("V16_CANDIDATE_PREFIX", "")
    F = layouts.Formulation(fid)
    chosen, npool = pool(paths)
    emit(kind="header", script="refine.py", fid=fid, logs=[os.path.basename(p) for p in paths], pool=npool,
         chosen=len(chosen), mp_dps=mp.dps, digits=DIGITS, top=TOP, distance=DIST)
    for rank, (r, x) in enumerate(chosen, start=1):
        xm, hist = refine(F, x)
        cid = "%s%s_r%02d" % (prefix, fid, rank)
        meta = {"log": r["_log"], "kind": r["kind"], "start": r.get("k", r.get("source_start")),
                "gamma_fixed": r.get("gamma_fixed"), "score_double": r["score"], "parts_double": r["parts"]}
        path = write_candidate(F, xm, cid, meta)
        emit(kind="refined", fid=fid, rank=rank, id=cid, file=os.path.relpath(path, HERE), source=meta,
             newton_history=hist, iterations=len(hist), max_hard_mp=hist[-1],
             max_shift_from_double=float(max(abs(xm[i] - mpf(float(x[i]))) for i in range(F.n))))
    emit(kind="footer", refined=len(chosen))


if __name__ == "__main__":
    main()
