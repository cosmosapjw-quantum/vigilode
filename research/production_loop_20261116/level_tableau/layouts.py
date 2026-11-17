#!/usr/bin/env python3
"""Loop L1 node V16 -- level-structured layouts, unknown vectors and residuals (see PREDECLARED.json).

Formulations: D5F, D5G (8 stages, levels 1-2-2-2-1, gamma pinned), D6T, D6F (10 stages, levels 1-2-2-2-2-1).
All functions are batched: X has shape (B, n); dtype float64, complex128 (complex-step derivative) or object
(mpmath numbers; hard conditions only).
"""
import json
import os

import numpy as np
from mpmath import mp, mpf

import rowtrees as rt

HERE = os.path.dirname(os.path.abspath(__file__))
LEVELS = {5: [[1], [2, 3], [4, 5], [6, 7], [8]], 6: [[1], [2, 3], [4, 5], [6, 7], [8, 9], [10]]}
ASTABLE_WINDOW_6 = (0.18391465, 0.33414237)   # theory S9
GAMMA_WINDOW_6 = (0.19, 0.33)                    # searched range (amendment A-1), inside the A-stable window
SEED_CODE = {"D5F": 51, "D5G": 52, "D6T": 61, "D6F": 62, "D6L": 63}
WEIGHTS = {"E6": 0.1, "D": 1.0, "Dhat": 1.0, "PR": 1.0, "PRhat": 0.3, "REG": 1e-5}
RHO_HAT = 0.2          # R_hat(infinity) of formulation D5F
ENTRY_BOUND = 50.0


def gamma5_mp():
    """A-stable root of the z^5 coefficient of exp(z)(1 - g z)^5 (theory S9), at the current mp precision."""
    return mp.findroot(lambda g: 120 * g ** 5 - 600 * g ** 4 + 600 * g ** 3 - 200 * g ** 2 + 25 * g - 1,
                       mpf("0.2780538411364523"))


def gamma5_str():
    """The pinned gamma of depth 5 as a decimal string with 50 significant digits."""
    with mp.workdps(80):
        return mp.nstr(gamma5_mp(), 50)


def gamma6l_str():
    """Depth 6, linear order 6 (amendment A-3): root in [0.33, 0.34] of the z^6 coefficient of
    exp(z)(1 - g z)^6, rounded DOWN to 40 decimals so that the value stays inside the A-stable window."""
    with mp.workdps(80):
        g = mp.findroot(lambda g: 720 * g ** 6 - 4320 * g ** 5 + 5400 * g ** 4 - 2400 * g ** 3 + 450 * g ** 2
                        - 36 * g + 1, mpf("0.33414237"))
        n = int(mp.floor(g * mpf(10) ** 40))
    return "0." + str(n).zfill(40)


def _trees_from_strings(strings, pool):
    table = {rt.show(t): t for t in pool}
    return [table[s] for s in strings]


class Plan:
    """Vectorised evaluation of Phi for a closed set of trees (same recursion as rowtrees.Evaluator)."""

    def __init__(self, roots):
        allt = set()

        def visit(t):
            if t not in allt:
                allt.add(t)
                for c in t[1]:
                    visit(c)

        for t in roots:
            visit(t)
        self.trees = sorted(allt, key=lambda t: (rt.order(t), t))
        self.index = {t: i + 1 for i, t in enumerate(self.trees)}      # slot 0 is the padding (ones)
        self.T = len(self.trees) + 1
        self.need_omega = any(t[0] == 1 for t in self.trees)
        self.groups = []
        for k in sorted({rt.order(t) for t in self.trees}):
            g = [t for t in self.trees if rt.order(t) == k]
            idx = np.array([self.index[t] for t in g])
            kind = np.array([1 if (t[0] == 0 and len(t[1]) == 1) else (3 if t[0] == 1 else 2) for t in g])
            child = np.zeros((len(g), max(1, max(len(t[1]) for t in g))), dtype=int)
            for r, t in enumerate(g):
                for ci, c in enumerate(t[1]):
                    child[r, ci] = self.index[c]
            self.groups.append((idx, kind == 1, kind == 3, child))

    def ids(self, trees):
        return np.array([self.index[t] for t in trees])

    def run(self, alpha, beta):
        B, s = alpha.shape[0], alpha.shape[1]
        PHI = np.ones((B, self.T, s), dtype=alpha.dtype)
        APHI = np.ones((B, self.T, s), dtype=alpha.dtype)
        BPHI = np.ones((B, self.T, s), dtype=alpha.dtype)
        omega = np.linalg.inv(beta) if self.need_omega else None
        for idx, m1, m3, child in self.groups:
            phi = APHI[:, child[:, 0], :]
            for ci in range(1, child.shape[1]):
                phi = phi * APHI[:, child[:, ci], :]
            if m3.any():
                phi[:, m3, :] = np.matmul(omega, phi[:, m3, :].transpose(0, 2, 1)).transpose(0, 2, 1)
            if m1.any():
                phi[:, m1, :] = BPHI[:, child[m1, 0], :]
            PHI[:, idx, :] = phi
            pt = phi.transpose(0, 2, 1)                                # (B, s, T_k): one gemm per batch item
            APHI[:, idx, :] = np.matmul(alpha, pt).transpose(0, 2, 1)
            BPHI[:, idx, :] = np.matmul(beta, pt).transpose(0, 2, 1)
        return PHI


class Formulation:
    def __init__(self, fid):
        assert fid in SEED_CODE
        self.id = fid
        self.depth = int(fid[1])
        self.levels = LEVELS[self.depth]
        self.L = len(self.levels)
        self.s = sum(len(lv) for lv in self.levels)
        s = self.s
        level_of = {}
        for li, lv in enumerate(self.levels):
            for st in lv:
                level_of[st - 1] = li
        self.level_of = level_of
        self.pairs = [(i, j) for i in range(s) for j in range(i) if level_of[j] < level_of[i]]
        self.tied = fid in ("D6T", "D6L")
        self.alpha_pairs = [p for p in self.pairs if not (self.tied and p[0] == s - 1)]
        self.gam_pairs = list(self.pairs)
        self.has_gamma = fid in ("D6T", "D6F")
        self.has_d = not self.tied
        self.n_gamma = 1 if self.has_gamma else 0
        self.n_alpha = len(self.alpha_pairs)
        self.n_gam = len(self.gam_pairs)
        self.n_d = s if self.has_d else 0
        self.n = self.n_gamma + self.n_alpha + self.n_gam + self.n_d
        self.ai = np.array([p[0] for p in self.alpha_pairs])
        self.aj = np.array([p[1] for p in self.alpha_pairs])
        self.gi = np.array([p[0] for p in self.gam_pairs])
        self.gj = np.array([p[1] for p in self.gam_pairs])
        self.trees5 = rt.ode_trees(5)
        self.trees4 = [t for t in self.trees5 if rt.order(t) <= 4]
        self.trees6 = [t for t in rt.ode_trees(6) if rt.order(t) == 6]
        sel = json.load(open(os.path.join(HERE, "index1_selected.json")))
        ys, zs = rt.index1_trees(5)
        self.d_main = _trees_from_strings(sel["main"], ys + zs)
        self.d_emb = _trees_from_strings(sel["embedded"], ys + zs)
        self.gamma_fixed_str = None if self.has_gamma else (gamma6l_str() if fid == "D6L" else gamma5_str())
        self.gamma_fixed = None if self.has_gamma else float(self.gamma_fixed_str)
        if fid == "D5F":
            self.hard_names = ["O5"] * 17 + ["E4"] * 8 + ["W", "C", "N"]
        elif self.tied:
            self.hard_names = ["O5"] * 17 + ["E4"] * 8 + ["W", "C"]
        else:
            self.hard_names = ["O5"] * 17 + ["E4"] * 8 + ["W", "N2"]
        self.m_hard = len(self.hard_names)
        self.plan_h = Plan(self.trees5)
        self.plan_a = Plan(self.trees5 + self.trees6 + self.d_main + self.d_emb)
        self._rhs = {}
        for name, trees in (("t5", self.trees5), ("t4", self.trees4), ("t6", self.trees6), ("dm", self.d_main),
                            ("dh", self.d_emb)):
            self._rhs[name] = np.array([1.0 / rt.density(t) for t in trees])
        self._sym6 = np.array([float(rt.symmetry(t)) for t in self.trees6])

    # ------------------------------------------------------------------------------------------ bounds, starts
    def bounds(self, fixed_gamma=None):
        lo = np.full(self.n, -ENTRY_BOUND)
        hi = np.full(self.n, ENTRY_BOUND)
        if self.has_gamma:
            if fixed_gamma is None:
                lo[0], hi[0] = GAMMA_WINDOW_6
            else:
                lo[0], hi[0] = fixed_gamma - 1e-13, fixed_gamma + 1e-13
        return lo, hi

    def random_start(self, rng):
        x = np.empty(self.n)
        o = 0
        if self.has_gamma:
            x[0] = rng.uniform(*GAMMA_WINDOW_6)
            o = 1
        na = self.n_alpha + self.n_gam
        x[o:o + na] = rng.normal(0.0, 0.6, na)
        if self.has_d:
            x[o + na:] = rng.normal(0.0, 0.3, self.n_d)
        return x

    # -------------------------------------------------------------------------------------------------- tableau
    def unpack(self, X):
        B = X.shape[0]
        s = self.s
        obj = X.dtype == object
        if obj:
            alpha = np.full((B, s, s), mpf(0), dtype=object)
            gam = np.full((B, s, s), mpf(0), dtype=object)
        else:
            alpha = np.zeros((B, s, s), dtype=X.dtype)
            gam = np.zeros((B, s, s), dtype=X.dtype)
        o = 0
        if self.has_gamma:
            gamma = X[:, 0]
            o = 1
        else:
            gfix = mpf(self.gamma_fixed_str) if obj else self.gamma_fixed
            gamma = np.full(B, gfix, dtype=X.dtype)
        alpha[:, self.ai, self.aj] = X[:, o:o + self.n_alpha]
        o += self.n_alpha
        gam[:, self.gi, self.gj] = X[:, o:o + self.n_gam]
        o += self.n_gam
        idx = np.arange(s)
        gam[:, idx, idx] = gamma[:, None]
        if self.tied:
            alpha[:, s - 1, :] = alpha[:, s - 2, :] + gam[:, s - 2, :]
        beta = alpha + gam
        b = beta[:, s - 1, :]
        if self.has_d:
            d = X[:, o:o + s]
            bhat = b - d
        else:
            bhat = beta[:, s - 2, :]
            d = b - bhat
        return {"gamma": gamma, "alpha": alpha, "Gamma": gam, "beta": beta, "b": b, "bhat": bhat, "d": d}

    # ------------------------------------------------------------------------------------------------ residuals
    def hard(self, X, tab=None, ev=None):
        tab = tab or self.unpack(X)
        ev = ev or rt.Evaluator(rt.NumpyOps(tab["alpha"], tab["beta"], np))
        s = self.s
        b, d, bhat = tab["b"], tab["d"], tab["bhat"]
        res = [ev.residual(b, t) for t in self.trees5]
        if self.tied:
            res += [ev.residual(bhat, t) for t in self.trees4]
        else:
            res += [ev.residual(d, t, homogeneous=True) for t in self.trees4]
        gi = tab["Gamma"].sum(axis=2)
        c = tab["alpha"].sum(axis=2)
        res.append((b * gi).sum(axis=-1))
        if self.tied:
            res.append(c[:, s - 2] - 1)
        elif self.id == "D5F":
            res.append(c[:, s - 1] - 1)
            v = ev.ops.ones
            for _ in range(4):
                v = ev.ops.mv(tab["beta"], v)
            res.append((d * v).sum(axis=-1) - RHO_HAT * tab["gamma"] ** 5)
        else:
            res.append((d * d).sum(axis=-1) - 1)
        return np.stack(res, axis=1)

    def soft_parts(self, X, tab=None, ev=None):
        tab = tab or self.unpack(X)
        ev = ev or rt.Evaluator(rt.NumpyOps(tab["alpha"], tab["beta"], np))
        b, bhat = tab["b"], tab["bhat"]
        c = tab["alpha"].sum(axis=2)
        e6 = np.stack([ev.residual(b, t) / rt.symmetry(t) for t in self.trees6], axis=1)
        dm = np.stack([ev.residual(b, t) for t in self.d_main], axis=1)
        dh = np.stack([ev.residual(bhat, t) for t in self.d_emb], axis=1)
        pr = np.stack(rt.pr_terms(ev.ops, b, c, self.L, (2, 3, 4)), axis=1)
        prh = np.stack(rt.pr_terms(ev.ops, bhat, c, self.L, (2, 3)), axis=1)
        return {"E6": e6, "D": dm, "Dhat": dh, "PR": pr, "PRhat": prh}

    # ------------------------------------------------------------------- vectorised residuals (numeric dtypes)
    @staticmethod
    def _wsum(w, PHI, ids):
        return (PHI[:, ids, :] * w[:, None, :]).sum(axis=-1)

    def _hard_fast(self, tab, PHI, plan):
        s = self.s
        b, d, bhat = tab["b"], tab["d"], tab["bhat"]
        res = [self._wsum(b, PHI, plan.ids(self.trees5)) - self._rhs["t5"]]
        if self.tied:
            res.append(self._wsum(bhat, PHI, plan.ids(self.trees4)) - self._rhs["t4"])
        else:
            res.append(self._wsum(d, PHI, plan.ids(self.trees4)))
        gi = tab["Gamma"].sum(axis=2)
        c = tab["alpha"].sum(axis=2)
        extra = [(b * gi).sum(axis=-1)]
        if self.tied:
            extra.append(c[:, s - 2] - 1)
        elif self.id == "D5F":
            extra.append(c[:, s - 1] - 1)
            v = np.ones_like(b)
            for _ in range(4):
                v = np.matmul(tab["beta"], v[..., None])[..., 0]
            extra.append((d * v).sum(axis=-1) - RHO_HAT * tab["gamma"] ** 5)
        else:
            extra.append((d * d).sum(axis=-1) - 1)
        res.append(np.stack(extra, axis=1))
        return np.concatenate(res, axis=1)

    def _pr_fast(self, tab):
        beta, b, bhat = tab["beta"], tab["b"], tab["bhat"]
        c = tab["alpha"].sum(axis=2)
        one = np.ones_like(c)
        mv = lambda v: np.matmul(beta, v[..., None])[..., 0]   # noqa: E731
        c2 = c * c
        c3 = c2 * c
        u2 = mv(mv(one)) - c2 / 2
        u3 = mv(c2) / 2 - c3 / 6
        u4 = mv(c3) / 6 - c3 * c / 24
        U = np.stack([u2, u3, u4, u2, u3], axis=1)                 # (B, 5, s)
        Wt = np.stack([b, b, b, bhat, bhat], axis=1)
        cols = []
        for _ in range(self.L):
            cols.append((Wt * U).sum(axis=-1))                     # (B, 5)
            U = np.matmul(beta, U.transpose(0, 2, 1)).transpose(0, 2, 1)
        T = np.stack(cols, axis=2)                                 # (B, 5, L)
        B = T.shape[0]
        return T[:, :3, :].reshape(B, -1), T[:, 3:, :].reshape(B, -1)

    def _soft_fast(self, tab, PHI, plan):
        b, bhat = tab["b"], tab["bhat"]
        e6 = (self._wsum(b, PHI, plan.ids(self.trees6)) - self._rhs["t6"]) / self._sym6
        dm = self._wsum(b, PHI, plan.ids(self.d_main)) - self._rhs["dm"]
        dh = self._wsum(bhat, PHI, plan.ids(self.d_emb)) - self._rhs["dh"]
        pr, prh = self._pr_fast(tab)
        return {"E6": e6, "D": dm, "Dhat": dh, "PR": pr, "PRhat": prh}

    def hard_fast(self, X):
        tab = self.unpack(X)
        return self._hard_fast(tab, self.plan_h.run(tab["alpha"], tab["beta"]), self.plan_h)

    def stacked(self, X):
        tab = self.unpack(X)
        PHI = self.plan_a.run(tab["alpha"], tab["beta"])
        h = self._hard_fast(tab, PHI, self.plan_a)
        p = self._soft_fast(tab, PHI, self.plan_a)
        blocks = [h] + [WEIGHTS[k] * p[k] for k in ("E6", "D", "Dhat", "PR", "PRhat")] + [WEIGHTS["REG"] * X]
        return np.concatenate(blocks, axis=1)

    def stacked_reference(self, X):
        tab = self.unpack(X)
        ev = rt.Evaluator(rt.NumpyOps(tab["alpha"], tab["beta"], np))
        h = self.hard(X, tab, ev)
        p = self.soft_parts(X, tab, ev)
        blocks = [h] + [WEIGHTS[k] * p[k] for k in ("E6", "D", "Dhat", "PR", "PRhat")] + [WEIGHTS["REG"] * X]
        return np.concatenate(blocks, axis=1)

    # ------------------------------------------------------------------------------- complex-step derivatives
    def _cs(self, fun, x):
        n = self.n
        hstep = 1e-30
        Xc = np.tile(x.astype(complex), (n, 1))
        Xc[np.arange(n), np.arange(n)] += 1j * hstep
        return fun(Xc).imag.T / hstep

    def stacked_jac(self, x):
        return self._cs(self.stacked, x)

    def hard_jac(self, x):
        return self._cs(self.hard_fast, x)

    # ---------------------------------------------------------------------------------------------- diagnostics
    def score(self, x):
        tab = self.unpack(x[None])
        p = self._soft_fast(tab, self.plan_a.run(tab["alpha"], tab["beta"]), self.plan_a)
        n2 = {k: float((v[0].real ** 2).sum()) for k, v in p.items()}
        sc = (n2["D"] + n2["Dhat"] + n2["PR"] + 0.09 * n2["PRhat"] + 0.01 * n2["E6"]) ** 0.5
        return sc, {k: v ** 0.5 for k, v in n2.items()}

    def coef_max(self, x):
        tab = self.unpack(x[None])
        return float(max(abs(tab["gamma"][0]), np.abs(tab["alpha"]).max(), np.abs(tab["Gamma"]).max(),
                         np.abs(tab["b"]).max(), np.abs(tab["bhat"]).max()))
