#!/usr/bin/env python3
"""Judge model for node V27 (design phase). NOT the Rust drivers.

DOPRI5 and RODAS5P re-implemented with the repository's tableaus (dopri5.rs:34-90,
fixtures/rodas5p_coefficients_snapshot.json), controller constants (adaptive.rs:161-177:
safety 0.9, factors 0.2/5.0, 0.9 after a rejection, estimator order 5), WRMS error norm with
scale atol + rtol max(|y|,|y_new|), FSAL, FD Jacobian n+1 calls (fd.rs:524-532), driver f0 call
(rodas5p_fast.rs:1180), FD time derivative 2 calls (problem.rs:818-825), Jacobian/f0/ft reuse
after a rejection (rodas5p_fast.rs:26-27). Operations by the textbook counts of DESIGN_A 2.2.

Routes: T = today's Auto (screen + trial min(1000, N/2) + fallback), A = DESIGN_A (backward
one-attempt pilots, Pareto, tenure pilots at b*2^k, optional witness), F = recommended design
(forward one-attempt pilots, saturated window, doubling, horizon), D = DOPRI5 alone,
R = RODAS5P alone. Deterministic (no clock, no randomness).

Usage: python3 -I judge_model.py
"""
import json
import math
import sys

import numpy as np
from scipy.linalg import lu_factor, lu_solve

ROOT = "/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w3-design"
_snap = json.load(open(ROOT + "/fixtures/rodas5p_coefficients_snapshot.json"))
RG = float(_snap["gamma"])
RA = np.array([[float(x) for x in r] for r in _snap["A"]])
RCM = np.array([[float(x) for x in r] for r in _snap["C"]])
RCV = np.array([float(x) for x in _snap["c"]])
RB = np.array([float(x) for x in _snap["b_code"]])
GAMMA = np.linalg.inv(np.eye(8) / RG - RCM)
GROWS = GAMMA.sum(axis=1)

DA = np.zeros((7, 7))
DA[1, :1] = [1 / 5]
DA[2, :2] = [3 / 40, 9 / 40]
DA[3, :3] = [44 / 45, -56 / 15, 32 / 9]
DA[4, :4] = [19372 / 6561, -25360 / 2187, 64448 / 6561, -212 / 729]
DA[5, :5] = [9017 / 3168, -355 / 33, 46732 / 5247, 49 / 176, -5103 / 18656]
DA[6, :6] = [35 / 384, 0, 500 / 1113, 125 / 192, -2187 / 6784, 11 / 84]
DC = np.array([0, 1 / 5, 3 / 10, 4 / 5, 8 / 9, 1, 1])
DE = np.array([71 / 57600, 0, -71 / 16695, 71 / 1920, -17253 / 339200, 22 / 525, -1 / 40])
DW = np.array([-34913 / 12672, 355 / 33, -310624 / 36729, 787 / 2112, -3645 / 74624, 11 / 84])
RPOLY = [1, 1, 1 / 2, 1 / 6, 1 / 24, 1 / 120, 1 / 600]

SAFETY, MINF, MAXF, REJF = 0.9, 0.2, 5.0, 0.9
SQEPS = math.sqrt(np.finfo(float).eps)


class Fatal(Exception):
    pass


class Prob:
    def __init__(self, name, n, f, y0, span, autonomous=True):
        self.name, self.n, self._f, self.y0 = name, n, f, np.array(y0, float)
        self.span, self.autonomous = span, autonomous
        self.calls = 0

    def f(self, t, y):
        self.calls += 1
        return np.asarray(self._f(t, y), float)


def wrms(e, y, yn, atol, rtol):
    s = atol + rtol * np.maximum(np.abs(y), np.abs(yn))
    v = math.sqrt(float(np.mean((e / s) ** 2)))
    return v if math.isfinite(v) else math.inf


def factor(err, accepted):
    if err == 0.0:
        return MAXF if accepted else REJF
    if not math.isfinite(err):
        return MINF
    raw = SAFETY * err ** (-1 / 5)
    return min(max(raw, MINF), MAXF if accepted else REJF)


class Cost:
    def __init__(self):
        self.cb = 0
        self.op = 0
        self.att = 0

    def add(self, cb, op, att):
        self.cb += cb
        self.op += op
        self.att += att

    def plus(self, o):
        c = Cost()
        c.add(self.cb + o.cb, self.op + o.op, self.att + o.att)
        return c

    def t(self):
        return (self.cb, self.op, self.att)


def ops_d(n):
    return 52 * n


def ops_r(n, fresh):
    return (2 * n ** 3) / 3 + 16 * n * n + (2 * n * n if fresh else 0) + 128 * n


class D5:
    """DOPRI5 session: one attempt per call."""

    def __init__(self, p, t, y, t1, h0, rtol, atol):
        self.p, self.t, self.y, self.t1, self.h = p, t, y.copy(), t1, h0
        self.rtol, self.atol, self.k1 = rtol, atol, None
        self.cost = Cost()

    def attempt(self):
        p, n, t, y = self.p, self.p.n, self.t, self.y
        h = min(self.h, self.t1 - t)
        before = p.calls
        if self.k1 is None:
            self.k1 = p.f(t, y)
        K = np.zeros((7, n))
        K[0] = self.k1
        for i in range(1, 7):
            K[i] = p.f(t + DC[i] * h, y + h * (DA[i, :i] @ K[:i]))
        yn = y + h * (DA[6, :6] @ K[:6])
        err = wrms(h * (DE @ K), y, yn, self.atol, self.rtol)
        acc = err <= 1.0
        d = h * (DW @ K[:6])
        g = K[6] - K[5]
        dd = float(d @ d)
        q = h * float(d @ g) / dd if dd > 0 else 0.0
        rho = h * math.sqrt(float(g @ g) / dd) if dd > 0 else 0.0
        fac = factor(err, acc)
        rec = dict(t_before=t, req=self.h, h=h, acc=acc, capped=acc and fac == MAXF, err=err,
                   q=q, rho=rho)
        if acc:
            self.t, self.y, self.k1 = (self.t1 if h == self.t1 - t else t + h), yn, K[6]
        self.h = h * fac
        self.cost.add(p.calls - before, ops_d(n), 1)
        return rec


def witness(rec):
    q, rho = rec["q"], rec["rho"]
    if not q < 0:
        return False
    z = complex(q, math.sqrt(max(rho * rho - q * q, 0.0)))
    return abs(sum(c * z ** k for k, c in enumerate(RPOLY))) >= 1.0


class R5:
    """RODAS5P (transformed stages, FD Jacobian): one attempt per call."""

    def __init__(self, p, t, y, t1, h0, rtol, atol):
        self.p, self.t, self.y, self.t1, self.h = p, t, y.copy(), t1, h0
        self.rtol, self.atol, self.fresh = rtol, atol, True
        self.cost = Cost()

    def attempt(self):
        p, n, t, y = self.p, self.p.n, self.t, self.y
        h = min(self.h, self.t1 - t)
        before = p.calls
        fresh = self.fresh
        if fresh:
            self._before = before
            f0 = p.f(t, y)
            J = np.zeros((n, n))
            for j in range(n):
                ys = y.copy()
                ys[j] = y[j] + SQEPS * max(abs(y[j]), 1.0)
                J[:, j] = (p.f(t, ys) - f0) / (ys[j] - y[j])
            self.J, self.f0 = J, p.f(t, y)
            if not p.autonomous:
                e = SQEPS * max(abs(t), 1.0)
                self.ft = (p.f(t + e, y) - p.f(t - e, y)) / (2 * e)
            self.fresh = False
        lu = lu_factor(np.eye(n) / (h * RG) - self.J)
        U = np.zeros((8, n))
        for i in range(8):
            with np.errstate(all="ignore"):
                r = self.f0.copy() if i == 0 else p.f(t + RCV[i] * h, y + RA[i, :i] @ U[:i])
                r = r + (RCM[i, :i] / h) @ U[:i]
                if not p.autonomous:
                    r = r + GROWS[i] * h * self.ft
            if not np.all(np.isfinite(r)):
                U[:] = np.inf
                break
            U[i] = lu_solve(lu, r)
        with np.errstate(all="ignore"):
            yn = y + RB @ U
        err = wrms(U[7], y, yn, self.atol, self.rtol) if np.all(np.isfinite(yn)) else math.inf
        acc = err <= 1.0
        fac = factor(err, acc)
        rec = dict(t_before=t, h=h, acc=acc, err=err)
        if acc:
            self.t, self.y, self.fresh = (self.t1 if h == self.t1 - t else t + h), yn, True
        self.h = h * fac
        self.cost.add(p.calls - before, ops_r(n, fresh), 1)
        return rec


OPT = dict(rtol=1e-6, atol=1e-9, h0=1e-3)


def run_r(p, t, y, budget, h0, cost):
    """RODAS5P leg from (t, y) to t1; returns (t, y, status)."""
    r = R5(p, t, y, p.span[1], h0, OPT["rtol"], OPT["atol"])
    while r.t < p.span[1]:
        if r.cost.att >= budget:
            cost.add(*r.cost.t())
            return r.t, r.y, "max-steps"
        r.attempt()
        if r.h < 1e-14:
            cost.add(*r.cost.t())
            return r.t, r.y, "step-too-small"
    cost.add(*r.cost.t())
    return r.t, r.y, "ok"


def run_d(p, budget, cost):
    d = D5(p, p.span[0], p.y0, p.span[1], OPT["h0"], OPT["rtol"], OPT["atol"])
    while d.t < p.span[1]:
        if d.cost.att >= budget:
            cost.add(*d.cost.t())
            return d, "max-steps"
        d.attempt()
    cost.add(*d.cost.t())
    return d, "ok"


def cvec(p):
    n = p.n
    cr_cb = n + 9 + (0 if p.autonomous else 2)
    return (cr_cb, ops_r(n, True)), (6, ops_d(n))


def b_of(p, rule):
    (rcb, rop), (dcb, dop) = cvec(p)
    bcb, bop = rcb / dcb, rop / dop
    if rule == "pareto":
        return int(max(math.ceil(bcb), math.ceil(bop))), bcb, bop
    if rule == "callbacks":
        return int(math.ceil(bcb)), bcb, bop
    return int(math.ceil(math.sqrt(bcb * bop))), bcb, bop


def route_T(p, N=100000):
    cost = Cost()
    n = p.n
    t0, t1 = p.span
    B = min(1000, N // 2)
    if n > 10 or B == 0:
        _, _, st = run_r(p, t0, p.y0, N, OPT["h0"], cost)
        return cost, st, "R"
    c0 = p.calls
    f0 = p.f(t0, p.y0)
    L = 0.0
    for j in range(n):
        ys = p.y0.copy()
        ys[j] = p.y0[j] + SQEPS * max(abs(p.y0[j]), 1.0)
        L = max(L, float(np.linalg.norm(p.f(t0, ys) - f0)) / (ys[j] - p.y0[j]))
    cost.add(p.calls - c0, 0, 0)
    if L * (t1 - t0) > 3.3 * B:
        _, _, st = run_r(p, t0, p.y0, N, OPT["h0"], cost)
        return cost, st, "screen->R"
    d, st = run_d(p, B, cost)
    if st == "ok":
        return cost, "ok", "D"
    _, _, st = run_r(p, d.t, d.y, N - B, OPT["h0"], cost)
    return cost, st, "D(%d)->R@%.4g" % (B, d.t)


def route_A(p, N=100000, use_witness=True):
    cost, P = Cost(), Cost()
    t0, t1 = p.span
    (rcb, rop), _ = cvec(p)
    b = b_of(p, "pareto")[0]
    cR = (rcb, rop, 1)
    d = D5(p, t0, p.y0, t1, OPT["h0"], OPT["rtol"], OPT["atol"])
    s_at = (0, 0, 0)
    anchor = (t0, p.y0.copy(), (0, 0, 0), None)
    k, wused, tenure_done, pilots = 0, False, set(), 0
    while True:
        if d.cost.att + P.att >= N:
            cost = d.cost.plus(P)
            return cost, "max-steps", "A:D only, %d pilots" % pilots
        rec = d.attempt()
        if anchor is not None and anchor[3] is None:
            anchor = (anchor[0], anchor[1], anchor[2], rec["req"])
        if rec["acc"]:
            s_at = d.cost.t()
        if d.t >= t1:
            return d.cost.plus(P), "ok", "A:D, %d pilots" % pilots
        w = use_witness and witness(rec)
        A_ = d.cost.att
        k_now = int(math.floor(math.log2(A_ / b))) if A_ >= 2 * b else 0
        if k_now > k:
            k, wused = k_now, False
        if anchor is None:
            if A_ >= 2 * b and A_ >= b * 2 ** k and k not in tenure_done:
                anchor = (d.t, d.y.copy(), s_at, None)
                tenure_done.add(k)
            elif w and not wused:
                anchor = (d.t, d.y.copy(), s_at, None)
                wused = True
        if anchor is not None:
            a, ya, Sa, req = anchor
            S = d.cost.t()
            if d.t > a:
                M = tuple(x - y for x, y in zip(s_at, Sa))
                h = d.t - a
            else:
                M = (S[0] - Sa[0] + 6, S[1] - Sa[1] + 52 * p.n, S[2] - Sa[2] + 1)
                h = req if req is not None else OPT["h0"]
            if all(m >= c for m, c in zip(M, cR)) and all(
                    x + c <= y for x, y, c in zip(P.t(), S, cR)) and d.cost.att + P.att < N:
                r = R5(p, a, ya, min(a + h, t1), h, OPT["rtol"], OPT["atol"])
                rec_r = r.attempt()
                P.add(*r.cost.t())
                pilots += 1
                anchor = None
                pc = r.cost.t()
                if rec_r["acc"] and all(x <= y for x, y in zip(pc, M)) and any(
                        x < y for x, y in zip(pc, M)):
                    cost = d.cost.plus(P)
                    used = cost.att
                    _, _, st = run_r(p, d.t, d.y, N - used, OPT["h0"], cost)
                    return cost, st, "A:D->R@%.4g after %d D att, %d pilots" % (d.t, d.cost.att, pilots)


def located_run(run, prev_rej, rec):
    """Number of DOPRI5 attempts since DOPRI5 located its step size: the first accepted
    attempt whose controller factor was below max_factor or that followed a rejection
    (B's saturation). The initial search and growth ramp before it do not count; after it
    every attempt counts (no reset)."""
    if run > 0:
        return run + 1, not rec["acc"]
    if not rec["acc"]:
        return 0, True
    if rec["capped"] and not prev_rej:
        return 0, False
    return 1, False


def route_F(p, N=100000, rule="product", horizon=True, h0_leg="pilot", stationary=False,
            fatal_mode="terminal"):
    cost, P = Cost(), Cost()
    t0, t1 = p.span
    b = b_of(p, rule)[0]
    d = D5(p, t0, p.y0, t1, OPT["h0"], OPT["rtol"], OPT["atol"])
    hist = []
    run, k, pilots, prev_rej = 0, 0, 0, False
    while True:
        if d.cost.att + P.att >= N:
            return d.cost.plus(P), "max-steps", "F:D only, %d pilots" % pilots
        rec = d.attempt()
        hist.append(rec)
        if d.t >= t1:
            return d.cost.plus(P), "ok", "F:D, %d pilots" % pilots
        run, prev_rej = located_run(run, prev_rej, rec)
        A_ = d.cost.att
        if k is not None and A_ >= b * 2 ** k and run >= b:
            first = hist[-b]
            H = d.t - first["t_before"]
            if H <= 0:
                H = first["req"]
            if horizon and t1 - d.t <= H:
                k = None
                continue
            if d.cost.att + P.att >= N:
                continue
            r = R5(p, d.t, d.y, t1, H, OPT["rtol"], OPT["atol"])
            try:
                rec_r = r.attempt()
            except Fatal:
                P.add(*r.cost.t())
                pilots += 1
                k += 1
                if fatal_mode == "terminal":
                    return d.cost.plus(P), "callback-failed@pilot t=%.3g" % d.t, "F:fatal in pilot"
                continue
            P.add(*r.cost.t())
            pilots += 1
            k += 1
            if rec_r["acc"] and (not stationary or r.h >= rec_r["h"]):
                cost = d.cost.plus(P)
                used = cost.att
                hl = rec_r["h"] if h0_leg == "pilot" else r.h
                _, _, st = run_r(p, r.t, r.y, N - used, hl, cost)
                return cost, st, "F:D->R@%.4g after %d D att, %d pilots" % (
                    d.t, d.cost.att, pilots)


def route_M(p, N=100000, rule="product", horizon=True):
    """Matched backward pilot over the last b saturated DOPRI5 attempts (recommended)."""
    cost, P = Cost(), Cost()
    t0, t1 = p.span
    b = b_of(p, rule)[0]
    d = D5(p, t0, p.y0, t1, OPT["h0"], OPT["rtol"], OPT["atol"])
    hist = []
    run, k, pilots, prev_rej = 0, 0, 0, False
    while True:
        if d.cost.att + P.att >= N:
            return d.cost.plus(P), "max-steps", "M:D only, %d pilots" % pilots
        y_before = d.y.copy()
        rec = d.attempt()
        hist.append((rec, y_before))
        if len(hist) > b + 1:
            hist.pop(0)
        if d.t >= t1:
            return d.cost.plus(P), "ok", "M:D, %d pilots" % pilots
        run, prev_rej = located_run(run, prev_rej, rec)
        if k is not None and d.cost.att >= b * 2 ** k and run >= b:
            first, ya = hist[-b]
            a = first["t_before"]
            H = d.t - a
            if H <= 0:
                a, ya, H = d.t, d.y.copy(), first["req"]
            if horizon and t1 - d.t <= H:
                k = None
                continue
            if d.cost.att + P.att >= N:
                continue
            r = R5(p, a, ya, min(a + H, t1), H, OPT["rtol"], OPT["atol"])
            rec_r = r.attempt()
            P.add(*r.cost.t())
            pilots += 1
            k += 1
            if rec_r["acc"]:
                cost = d.cost.plus(P)
                _, _, st = run_r(p, d.t, d.y, N - cost.att, H, cost)
                return cost, st, "M:D->R@%.4g after %d D att, %d pilots" % (
                    d.t, d.cost.att, pilots)


def problems():
    ps = []
    ps.append(Prob("scalar_fd -1e6(y-cos t) [0,1]", 1, lambda t, y: -1e6 * (y - math.cos(t)),
                   [1.0], (0.0, 1.0), autonomous=False))
    for lam in (2000.0, 3200.0, 3400.0):
        ps.append(Prob("y'=-%g y [0,1]" % lam, 1, (lambda L: lambda t, y: -L * y)(lam), [1.0],
                       (0.0, 1.0)))

    def heat(n):
        dx = 1.0 / (n + 1)

        def f(t, y):
            yp = np.concatenate(([0.0], y, [0.0]))
            return (yp[:-2] - 2 * yp[1:-1] + yp[2:]) / dx ** 2
        x = np.arange(1, n + 1) * dx
        return f, np.sin(np.pi * x)
    f8, y8 = heat(8)
    ps.append(Prob("heat n=8 FD [0,100]", 8, f8, y8, (0.0, 100.0)))

    def rob(t, y):
        return [-0.04 * y[0] + 1e4 * y[1] * y[2],
                0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] ** 2,
                3e7 * y[1] ** 2]
    ps.append(Prob("Robertson FD [0,40]", 3, rob, [1, 0, 0], (0.0, 40.0)))
    ps.append(Prob("Robertson FD [0,1e5]", 3, rob, [1, 0, 0], (0.0, 1e5)))
    ps.append(Prob("oscillator [0,1]", 2, lambda t, y: [y[1], -y[0]], [1, 0], (0.0, 1.0)))
    ps.append(Prob("oscillator [0,10]", 2, lambda t, y: [y[1], -y[0]], [1, 0], (0.0, 10.0)))
    ps.append(Prob("oscillator [0,1000]", 2, lambda t, y: [y[1], -y[0]], [1, 0],
                   (0.0, 1000.0)))
    ps.append(Prob("Lotka-Volterra [0,100]", 2,
                   lambda t, y: [1.5 * y[0] - y[0] * y[1], -3 * y[1] + y[0] * y[1]], [1, 1],
                   (0.0, 100.0)))
    w, z = 100.0, 1e-3
    ps.append(Prob("damped osc w=100 z=1e-3 [0,10]", 2,
                   lambda t, y: [y[1], -w * w * y[0] - 2 * z * w * y[1]], [1, 0], (0.0, 10.0)))
    ps.append(Prob("nonnormal [[-1,1e3],[0,-2]] [0,10]", 2,
                   lambda t, y: [-y[0] + 1e3 * y[1], -2 * y[1]], [1, 1], (0.0, 10.0)))
    ps.append(Prob("flame d=1e-3 [0,2000]", 1, lambda t, y: y * y - y ** 3, [1e-3],
                   (0.0, 2000.0)))
    ps.append(Prob("Arenstorf-like (Lorenz) [0,10]", 3,
                   lambda t, y: [10 * (y[1] - y[0]), y[0] * (28 - y[2]) - y[1],
                                 y[0] * y[1] - 8 / 3 * y[2]], [1, 1, 1], (0.0, 10.0)))
    mu = 0.012277471
    mup = 1 - mu

    def aren(t, y):
        d1 = ((y[0] + mu) ** 2 + y[1] ** 2) ** 1.5
        d2 = ((y[0] - mup) ** 2 + y[1] ** 2) ** 1.5
        return [y[2], y[3],
                y[0] + 2 * y[3] - mup * (y[0] + mu) / d1 - mu * (y[0] - mup) / d2,
                y[1] - 2 * y[2] - mup * y[1] / d1 - mu * y[1] / d2]
    ps.append(Prob("Arenstorf orbit [0,T]", 4, aren, [0.994, 0, 0, -2.00158510637908252240537862224],
                   (0.0, 17.0652165601579625588917206249)))
    ps.append(Prob("Brusselator n=2 [0,20]", 2,
                   lambda t, y: [1 + y[0] ** 2 * y[1] - 4 * y[0], 3 * y[0] - y[0] ** 2 * y[1]],
                   [1.5, 3.0], (0.0, 20.0)))
    ps.append(Prob("Van der Pol mu=1 [0,20]", 2,
                   lambda t, y: [y[1], (1 - y[0] ** 2) * y[1] - y[0]], [2.0, 0.0], (0.0, 20.0)))
    ps.append(Prob("Van der Pol mu=1000 [0,1]", 2,
                   lambda t, y: [y[1], 1000.0 * (1 - y[0] ** 2) * y[1] - y[0]], [2.0, 0.0],
                   (0.0, 1.0)))

    def hires(t, y):
        f7 = 280 * y[5] * y[7] - 1.81 * y[6]
        return [-1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007,
                1.71 * y[0] - 8.75 * y[1],
                -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4],
                8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3],
                -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6],
                -280 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6],
                f7, -f7]
    ps.append(Prob("HIRES [0,321.8122]", 8, hires, [1, 0, 0, 0, 0, 0, 0, 0.0057],
                   (0.0, 321.8122)))
    ps.append(Prob("OREGO [0,360]", 3,
                   lambda t, y: [77.27 * (y[1] + y[0] * (1 - 8.375e-6 * y[0] - y[1])),
                                 (y[2] - (1 + y[0]) * y[1]) / 77.27, 0.161 * (y[0] - y[2])],
                   [1.0, 2.0, 3.0], (0.0, 360.0)))
    fh, yh = heat(64)
    ps.append(Prob("heat n=64 FD [0,10]", 64, fh, yh, (0.0, 10.0)))

    def l96(t, y):
        return (np.roll(y, -1) - np.roll(y, 2)) * np.roll(y, 1) - y + 8.0
    y96 = 8.0 * np.ones(40)
    y96[19] += 0.01
    ps.append(Prob("Lorenz-96 n=40 [0,10]", 40, l96, y96, (0.0, 10.0)))

    def guarded(t, y):
        if y[0] > 1.0:
            raise Fatal("guarded")
        return [-y[0]]
    ps.append(Prob("F2 guarded y'=-y fatal y>1 [0,1]", 1, guarded, [1.0], (0.0, 1.0)))

    def colfatal(t, y):
        if y[0] > 1.0:
            raise Fatal("column")
        return [0.0, -1000.0 * y[1]]
    ps.append(Prob("N2 fatal at FD column only, stiff [0,1]", 2, colfatal, [1.0, 1.0], (0.0, 1.0)))
    return ps


def main():
    import os
    OPT["rtol"] = float(os.environ.get("JM_RTOL", OPT["rtol"]))
    OPT["atol"] = float(os.environ.get("JM_ATOL", OPT["atol"]))
    arms = os.environ.get("JM_ARMS", "D,R,T,A2,A2noW,F,Fpareto,Fstat,Fpareto_stat").split(",")
    dcap = int(os.environ.get("JM_DCAP", 10 ** 6))
    only = sys.argv[1:] or None
    out = []
    for p in problems():
        if only and not any(s in p.name for s in only):
            continue
        row = dict(problem=p.name, n=p.n, rtol=OPT["rtol"], atol=OPT["atol"])
        table = (("D", lambda: (lambda c: (c, run_d(p, dcap, c)[1], "D"))(Cost())),
                 ("R", lambda: (lambda c: (c, run_r(p, p.span[0], p.y0, 10 ** 6,
                                                    OPT["h0"], c)[2], "R"))(Cost())),
                 ("T", lambda: route_T(p)),
                 ("A2", lambda: route_A(p, use_witness=True)),
                 ("A2noW", lambda: route_A(p, use_witness=False)),
                 ("F", lambda: route_F(p)),
                 ("Fpareto", lambda: route_F(p, rule="pareto")),
                 ("Fstat", lambda: route_F(p, stationary=True)),
                 ("Fpareto_stat", lambda: route_F(p, rule="pareto", stationary=True)),
                 ("Fpareto_cont", lambda: route_F(p, rule="pareto", fatal_mode="continue")),
                 ("T500", lambda: route_T(p, N=500)),
                 ("Fpareto500", lambda: route_F(p, N=500, rule="pareto")),
                 ("M", lambda: route_M(p)),
                 ("Mpareto", lambda: route_M(p, rule="pareto")))
        for label, fn in table:
            if label not in arms:
                continue
            p.calls = 0
            try:
                c, st, how = fn()
            except Fatal as e:
                c, st, how = Cost(), "callback-failed", "fatal: %s" % e
            row[label] = dict(cb=c.cb, op=int(c.op), att=c.att, status=st, how=how,
                              entries=p.calls)
        out.append(row)
        print(json.dumps(row))
        sys.stdout.flush()


if __name__ == "__main__":
    main()
