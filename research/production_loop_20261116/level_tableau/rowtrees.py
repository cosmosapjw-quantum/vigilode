#!/usr/bin/env python3
"""Loop L1 node V16 -- order conditions of Rosenbrock-Wanner (ROW) methods from rooted trees.

Method (k-form, theory S9 notation):
    (I - h gamma J) k_i = h f(y_n + sum_{j<i} alpha_ij k_j) + h J sum_{j<i} gamma_ij k_j,
    y_{n+1} = y_n + sum_i b_i k_i,     beta = alpha + Gamma,  Gamma_ii = gamma.

Order conditions (Hairer and Wanner, Solving Ordinary Differential Equations II, section IV.7; cited from
memory, re-derived in NOTES.txt, validated numerically on RODAS5P by run_validation.py):
    Phi_j(leaf) = 1
    vertex with one child            : Phi_j = sum_k beta_jk Phi_k(child)
    vertex with two or more children : Phi_j = prod_children sum_k alpha_jk Phi_k(child)
    condition for weights w          : sum_j w_j Phi_j(t) = 1 / gamma(t)

Index-1 extension (section VI.4 of the same book, from memory; NOT part of the frozen tool validation):
two-coloured trees, colour 0 = meagre (a derivative of f), colour 1 = fat (an algebraic solve). A fat vertex
has at least two children and contributes sum_k omega_jk prod_children sum_l alpha_kl Phi_l(child) with
omega = beta^-1. Order = number of meagre vertices. gamma(meagre root) = order * prod gamma(children),
gamma(fat root) = prod gamma(children). A fat vertex with one child is the identity (omega beta = I) and a
meagre vertex whose only child is fat reproduces the tree with the fat vertex merged into it; both are skipped.

A tree is the tuple (colour, children) with children a sorted tuple of trees.
"""
import functools
import math

LEAF = (0, ())


def mk(colour, children):
    return (colour, tuple(sorted(children)))


@functools.lru_cache(maxsize=None)
def order(t):
    return (1 if t[0] == 0 else 0) + sum(order(c) for c in t[1])


@functools.lru_cache(maxsize=None)
def density(t):
    p = 1
    for c in t[1]:
        p *= density(c)
    return order(t) * p if t[0] == 0 else p


@functools.lru_cache(maxsize=None)
def symmetry(t):
    s = 1
    seen = {}
    for c in t[1]:
        seen[c] = seen.get(c, 0) + 1
    for c, m in seen.items():
        s *= symmetry(c) ** m * math.factorial(m)
    return s


def has_fat(t):
    return t[0] == 1 or any(has_fat(c) for c in t[1])


def redundant(t):
    """Meagre vertex whose only child is fat (same condition as the merged tree)."""
    if t[0] == 0 and len(t[1]) == 1 and t[1][0][0] == 1:
        return True
    return any(redundant(c) for c in t[1])


def show(t):
    inner = ",".join(show(c) for c in t[1])
    if t[0] == 0:
        return "o" if not t[1] else "[" + inner + "]"
    return "{" + inner + "}"


def _multisets(total, items, min_parts):
    """Sorted tuples of trees from `items` (repetition allowed) whose orders sum to `total`."""
    out = []

    def rec(remaining, start, acc):
        if remaining == 0:
            if len(acc) >= min_parts:
                out.append(tuple(acc))
            return
        for idx in range(start, len(items)):
            o = order(items[idx])
            if o <= remaining:
                acc.append(items[idx])
                rec(remaining - o, idx, acc)
                acc.pop()

    rec(total, 0, [])
    return out


def generate(pmax, two_coloured=False):
    """Return {order: [trees]} for meagre roots and for fat roots (empty dict if not two_coloured)."""
    ytrees = {1: [LEAF]}
    ztrees = {}
    for p in range(2, pmax + 1):
        items = []
        for q in range(1, p):
            items += ytrees.get(q, [])
            items += ztrees.get(q, [])
        ytrees[p] = sorted({mk(0, ms) for ms in _multisets(p - 1, items, 1)})
        if two_coloured:
            ztrees[p] = sorted({mk(1, ms) for ms in _multisets(p, items, 2)})
    return ytrees, ztrees


def ode_trees(pmax):
    """Monochrome trees of order 1..pmax as a flat list, ordered by order."""
    ytrees, _ = generate(pmax, two_coloured=False)
    return [t for p in range(1, pmax + 1) for t in ytrees[p]]


def index1_trees(pmax):
    """Two-coloured trees with at least one fat vertex, non-redundant; (y_rooted, z_rooted) flat lists."""
    ytrees, ztrees = generate(pmax, two_coloured=True)
    ys = [t for p in range(1, pmax + 1) for t in ytrees[p] if has_fat(t) and not redundant(t)]
    zs = [t for p in range(2, pmax + 1) for t in ztrees.get(p, []) if not redundant(t)]
    return ys, zs


# ----------------------------------------------------------------------------------------------- arithmetic
class NumpyOps:
    """Batched evaluation: alpha, beta of shape (B, s, s); vectors of shape (B, s). Works for float64,
    complex128 (complex-step differentiation) and, without `solve`, for object arrays of mpmath numbers."""

    def __init__(self, alpha, beta, np):
        self.np = np
        self.alpha = alpha
        self.beta = beta
        self._omega = None
        self.ones = np.ones(alpha.shape[:2], dtype=alpha.dtype) if alpha.dtype != object else \
            np.full(alpha.shape[:2], alpha.flat[0] * 0 + 1, dtype=object)

    def mv(self, m, v):
        return self.np.matmul(m, v[..., None])[..., 0]

    def mul(self, u, v):
        return u * v

    def solve_beta(self, v):
        if self._omega is None:
            self._omega = self.np.linalg.inv(self.beta)
        return self.mv(self._omega, v)

    def dot(self, w, v):
        return (w * v).sum(axis=-1)


class ListOps:
    """Scalar evaluation on lists of lists; any exact or multiprecision number type."""

    def __init__(self, alpha, beta):
        self.alpha = alpha
        self.beta = beta
        s = len(alpha)
        one = beta[0][0] * 0 + 1
        self.ones = [one] * s
        self.s = s

    def mv(self, m, v):
        return [sum((m[i][j] * v[j] for j in range(self.s) if m[i][j] != 0), v[0] * 0) for i in range(self.s)]

    def mul(self, u, v):
        return [x * y for x, y in zip(u, v)]

    def solve_beta(self, v):
        b = self.beta
        x = []
        for i in range(self.s):
            assert all(b[i][j] == 0 for j in range(i + 1, self.s)), "beta must be lower triangular"
            x.append((v[i] - sum((b[i][j] * x[j] for j in range(i)), v[0] * 0)) / b[i][i])
        return x

    def dot(self, w, v):
        return sum((x * y for x, y in zip(w, v)), v[0] * 0)


class Evaluator:
    def __init__(self, ops):
        self.ops = ops
        self._phi = {}
        self._aphi = {}
        self._bphi = {}

    def phi(self, t):
        r = self._phi.get(t)
        if r is not None:
            return r
        ops = self.ops
        colour, ch = t
        if colour == 0:
            if not ch:
                r = ops.ones
            elif len(ch) == 1:
                r = self.bphi(ch[0])
            else:
                r = self.aphi(ch[0])
                for c in ch[1:]:
                    r = ops.mul(r, self.aphi(c))
        else:
            assert len(ch) >= 2
            r = self.aphi(ch[0])
            for c in ch[1:]:
                r = ops.mul(r, self.aphi(c))
            r = ops.solve_beta(r)
        self._phi[t] = r
        return r

    def aphi(self, t):
        r = self._aphi.get(t)
        if r is None:
            r = self.ops.mv(self.ops.alpha, self.phi(t))
            self._aphi[t] = r
        return r

    def bphi(self, t):
        r = self._bphi.get(t)
        if r is None:
            r = self.ops.mv(self.ops.beta, self.phi(t))
            self._bphi[t] = r
        return r

    def residual(self, w, t, homogeneous=False):
        """sum_j w_j Phi_j(t) - 1/gamma(t)  (without the right-hand side if homogeneous)."""
        v = self.ops.dot(w, self.phi(t))
        if homogeneous:
            return v
        return v - self._one() / density(t)

    def _one(self):
        o = self.ops.ones
        return o[..., 0] if hasattr(o, "shape") else o[0]


def pr_vectors(ops, c):
    """u_2, u_3, u_4 of the Prothero-Robinson local error factors W_q(z) = sum_k z^(k+1) w^T beta^k u_q."""
    one = ops.ones
    b1 = ops.mv(ops.beta, one)
    c2 = ops.mul(c, c)
    c3 = ops.mul(c2, c)
    c4 = ops.mul(c3, c)
    u2 = _sub(ops, ops.mv(ops.beta, b1), _scale(ops, c2, 2))
    u3 = _sub(ops, _scale(ops, ops.mv(ops.beta, c2), 2), _scale(ops, c3, 6))
    u4 = _sub(ops, _scale(ops, ops.mv(ops.beta, c3), 6), _scale(ops, c4, 24))
    return {2: u2, 3: u3, 4: u4}


def _scale(ops, v, k):
    if hasattr(v, "shape"):
        return v / k
    return [x / k for x in v]


def _sub(ops, u, v):
    if hasattr(u, "shape"):
        return u - v
    return [x - y for x, y in zip(u, v)]


def pr_terms(ops, w, c, levels, qs):
    """List of w^T beta^k u_q for q in qs, k = 0..levels-1."""
    us = pr_vectors(ops, c)
    out = []
    for q in qs:
        v = us[q]
        for _ in range(levels):
            out.append(ops.dot(w, v))
            v = ops.mv(ops.beta, v)
    return out


# ------------------------------------------------------------------------------------------- RODAS5P loader
def load_rodas5p(snapshot_path, parse):
    """Reconstruction of theory section 0 with the number type of `parse` (e.g. mpmath.mpf or Fraction)."""
    import json
    raw = json.load(open(snapshot_path))
    n = 8
    gamma = parse(raw["gamma"])
    zero = gamma * 0
    a = [[parse(x) for x in r] for r in raw["A"]]
    cm = [[parse(x) for x in r] for r in raw["C"]]
    b_code = [parse(x) for x in raw["b_code"]]
    ginv = [[(1 / gamma if i == j else zero) - cm[i][j] for j in range(n)] for i in range(n)]
    gam = [[zero] * n for _ in range(n)]
    for col in range(n):
        for i in range(col, n):
            rhs = (zero + 1 if i == col else zero) - sum((ginv[i][k] * gam[k][col] for k in range(col, i)), zero)
            gam[i][col] = rhs / ginv[i][i]
    alpha = [[sum((a[i][k] * gam[k][j] for k in range(n)), zero) for j in range(n)] for i in range(n)]
    beta = [[alpha[i][j] + gam[i][j] for j in range(n)] for i in range(n)]
    b = [sum((b_code[k] * gam[k][j] for k in range(n)), zero) for j in range(n)]
    return {"s": n, "gamma": gamma, "alpha": alpha, "Gamma": gam, "beta": beta, "b": b, "bhat": list(alpha[7]),
            "A": a, "C": cm, "b_code": b_code}
