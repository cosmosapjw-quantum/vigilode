#!/usr/bin/env python3
"""PP09: where the stiff Laguerre bound is lost, and a certified finite-degree
envelope (research node research/pp09_laguerre_envelope_20261004, RVJ DAG node
PP09). Python only (mpmath 60 digits, mpmath.iv interval arithmetic). Admits
nothing and changes no solver.

Part A (gated by G1, published data only). From the published R-NEXT-04
cases.json, every Laguerre recurrence case with h rho >= 20: the components
truncation, coefficient, recurrence_adjoint, summation, normalization (and the
fused-sum rounding) as fractions of the published total, and the bottleneck.
The fractions are formed only from component fields present in the published
file. A case whose published record lacks a component field gets no fractions
and the missing fields are listed; G1 holds only if every selected case has
all its fractions. rho is the enclosure rho the native test used; cases.json
does not publish it, so it is derived from the test source
(crates/rodas5p-core/tests/rnext04_laguerre_admission.rs):
rho_enc = 1.2 * rho_nominal * (1 + 1e-12) in binary64, rho_nominal from the
label (grid cases) or the test constants (near-cancellation rho 1, the two
amplitude cases rho 50). The published matrix is checked to be Gershgorin-
verified inside [-rho_enc, 0] (the native check, emulated).

Supplementary (NOT registered, NOT gated, decided before the recorded run):
because cases.json carries only the total, the two components that carry
e^{L'/2} are re-derived by a bit-faithful emulation of polynomial_action.rs
(choose_transform, laguerre_coefficients, exp_nonneg, norm_up and the
directed-rounding primitives of directed.rs) from the published A, h, w,
degree and scale. The emulated degree and scale are compared with the
published ones. The remainder total - truncation - coefficient is
recurrence_adjoint + summation + fused_summation + normalization, which the
published fields cannot separate. These numbers are labelled derived.

Part B. L' = div_up(rho_enc, beta), beta = rho_enc / scale, exactly as
choose_transform forms it. E_n = max_{x in [0, L']} |L_n(x)| for n = 0..128 is
enclosed by evaluating the three-term recurrence
L_{n+1} = ((2n+1-x) L_n - n L_{n-1}) / (n+1) in mpmath.iv (60 digits) on
dyadic subintervals [c-r, c+r] of [0, L']. Implementation choice, fixed before
any run (disclosed): the recurrence is run on local polynomials in t = x - c
with interval coefficients (each step multiplies by (2n+1-c) - t, adds and
divides, all in iv), and |L_n| <= sum_k mag(a_k) r^k on the piece. The naive
interval extension (x itself an interval) has a dependency width that grows
like (1 + sqrt 2)^n * (2r) (the width recurrence w_{n+1} ~ 2 w_n + w_{n-1}),
so with 2^20 pieces of [0, 16] (2r ~ 1.5e-5) it cannot meet the 1 % rule
beyond n of about 15-20. The naive enclosure is still computed on the final
cover and reported as a diagnostic. Subdivision rule: pieces are bisected
level by level; a piece is final once, for every n, its enclosure upper end
is within 1 % of the width-free value, taken as the largest certified point
lower bound of |L_n| found on [0, L'] so far (point values at every piece's
ends and centre; this is at least the piece's own width-free value), so the
reported E_n is at most 1.01 times an attained value; or when the cover
would exceed 2^20 pieces (then the remaining pieces stay as they are). The
bound is the maximum upper end over the final cover.

G2: every E_n is checked against 60-digit point evaluations of L_n at
10,000 equispaced points of [0, L'] (no point above the bound); a subset is
cross-checked against mpmath.laguerre.

G3 (reported): the factor e^{L'/2} / max_n E_n; the published coefficient
radii and tails do not exist in cases.json, so the registered recomputation
reduces to the ratio. Implied facts reported: any recomputed total is at
least total / (e^{L'/2} / max_n E_n) when max_n E_n replaces e^{L'/2}, and at
least total / e^{L'/2} for any valid envelope (E_n >= |L_n(0)| = 1), compared
with the published budgets. The supplementary derived recomputation uses the
per-n E_n in the derived coefficient term.

Verdict PASS iff G1 and G2.

Usage:
  python3 tools/pp09_laguerre_envelope.py --self-test
  python3 tools/pp09_laguerre_envelope.py \
      --cases research/rnext04_laguerre_admission_20261003/cases.json \
      --output research/pp09_laguerre_envelope_20261004/RESULTS.json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import multiprocessing
import re
import struct
import subprocess
import sys
import time
from fractions import Fraction
from pathlib import Path

import mpmath
from mpmath import iv, mp

SCHEMA = "vigilode-pp09-laguerre-envelope-v1"
NODE = "research/pp09_laguerre_envelope_20261004"
DPS = 60
N_MAX = 128
PIECE_CAP = 2 ** 20
REL_TOL = mp.mpf("1.01")  # set again after mp.dps
G2_POINTS = 10_000
G2_CROSS_STRIDE = 100  # every 100th point is also checked with mpmath.laguerre
HRHO_MIN = 20

COMPONENT_FIELDS = ["truncation", "coefficient", "recurrence_adjoint", "summation",
                    "normalization", "fused_summation"]


# ---------------------------------------------------------------------------
# Emulation of directed.rs (binary64, round to nearest then one outward step
# when the exact result lies beyond it; EXACT_FMA_FLOOR handled as there).
# ---------------------------------------------------------------------------

EXACT_FMA_FLOOR = 1.0e-289
INF = float("inf")


def next_up(x: float) -> float:
    return math.nextafter(x, INF)


def next_down(x: float) -> float:
    return math.nextafter(x, -INF)


def _finite(x: float) -> float:
    if not math.isfinite(x):
        raise ArithmeticError("directed rounding: not finite")
    return x


def add_up(a: float, b: float) -> float:
    s = _finite(a + b)
    return next_up(s) if Fraction(a) + Fraction(b) > Fraction(s) else s


def add_down(a: float, b: float) -> float:
    s = _finite(a + b)
    return next_down(s) if Fraction(a) + Fraction(b) < Fraction(s) else s


def sub_up(a: float, b: float) -> float:
    return add_up(a, -b)


def sub_down(a: float, b: float) -> float:
    return add_down(a, -b)


def mul_up(a: float, b: float) -> float:
    p = _finite(a * b)
    if a == 0.0 or b == 0.0:
        return p
    if abs(p) < EXACT_FMA_FLOOR:
        return next_up(p)
    return next_up(p) if Fraction(a) * Fraction(b) > Fraction(p) else p


def mul_down(a: float, b: float) -> float:
    p = _finite(a * b)
    if a == 0.0 or b == 0.0:
        return p
    if abs(p) < EXACT_FMA_FLOOR:
        return next_down(p)
    return next_down(p) if Fraction(a) * Fraction(b) < Fraction(p) else p


def div_up(a: float, b: float) -> float:
    if b == 0.0:
        raise ZeroDivisionError("directed rounding: division by zero")
    q = _finite(a / b)
    if a == 0.0:
        return q
    if abs(q) < EXACT_FMA_FLOOR or abs(q * b) < EXACT_FMA_FLOOR:
        return next_up(q)
    return next_up(q) if Fraction(a) / Fraction(b) > Fraction(q) else q


def div_down(a: float, b: float) -> float:
    return -div_up(-a, b)


def sqrt_up(x: float) -> float:
    s = _finite(math.sqrt(x))
    if x == 0.0:
        return 0.0
    if x < EXACT_FMA_FLOOR:
        return next_up(s)
    return next_up(s) if Fraction(s) * Fraction(s) < Fraction(x) else s


def sum_up(values) -> float:
    total = 0.0
    for v in values:
        total = add_up(total, v)
    return total


def norm_up(values) -> float:
    """ExpBound::l2_norm_upper for normal-range entries (power-of-two scaling
    does not change directed rounding there): sqrt_up(sum_up(mul_up(x, x)))."""
    values = [abs(v) for v in values if v != 0.0]
    if not values:
        return 0.0
    if min(values) < 1e-150 or max(values) > 1e150:
        raise ValueError("norm_up emulation only for normal-range entries")
    return sqrt_up(sum_up(mul_up(v, v) for v in values))


SERIES_RELATIVE_TAIL = 1.0 / float(1 << 60)


def geometric_tail_up(term: float, ratio: float) -> float:
    return div_up(mul_up(term, ratio), sub_down(1.0, ratio))


def exp_nonneg(x: float, upward: bool) -> float:
    if not (0.0 <= x <= 709.0):
        raise ValueError("exponent out of range")
    total, term = 1.0, 1.0
    for i in range(1, 200_000):
        step = float(i)
        term = div_up(mul_up(term, x), step) if upward else div_down(mul_down(term, x), step)
        total = add_up(total, term) if upward else add_down(total, term)
        ratio = x / (step + 1.0)
        if ratio < 0.5 and term <= total * SERIES_RELATIVE_TAIL:
            return add_up(total, geometric_tail_up(term, div_up(x, step + 1.0))) if upward else total
    raise ArithmeticError("exponential series did not converge")


def exp_up(x: float) -> float:
    if x >= 0.0:
        return exp_nonneg(x, True)
    return div_up(1.0, exp_nonneg(min(-x, 700.0), False))


def hyp2f1_laguerre(n: int, k: int, q: float, upward: bool) -> float:
    if q == 0.0:
        return 1.0
    total, term = 1.0, 1.0
    for i in range(200_000):
        numerator = float(n + 1 + i) * float(k + i)
        denominator = float(n + k + 1 + i) * float(i + 1)
        if upward:
            term = div_up(mul_up(mul_up(term, numerator), q), denominator)
            total = add_up(total, term)
        else:
            term = div_down(mul_down(mul_down(term, numerator), q), denominator)
            total = add_down(total, term)
        ratio = div_up(mul_up(q, float(k + i + 1)), float(i + 2))
        if ratio < 1.0:
            tail = geometric_tail_up(term, ratio)
            if tail <= total * SERIES_RELATIVE_TAIL:
                return add_up(total, tail) if upward else total
    raise ArithmeticError("2F1 series did not converge")


def inverse_rising(p: int, k: int):
    lo = hi = 1.0
    for i in range(1, k + 1):
        lo = div_down(lo, float(p + i))
        hi = div_up(hi, float(p + i))
    return (lo, hi)


def positive_mul(x, y):
    return (mul_down(x[0], y[0]), mul_up(x[1], y[1]))


def laguerre_coefficients(a, degree: int):
    """Emulation of polynomial_action.rs laguerre_coefficients for a = (lo, hi)."""
    if a[0] < 0.0:
        raise ValueError("Laguerre coefficients need h beta >= 0")
    q = (div_down(a[0], add_up(1.0, a[0])), div_up(a[1], add_down(1.0, a[1])))
    if q[1] >= 1.0:
        raise ValueError("Laguerre ratio q rounds to 1")
    one_minus_q = (div_down(1.0, add_up(1.0, a[1])), div_up(1.0, add_down(1.0, a[0])))
    out = []
    power = (1.0, 1.0)
    for n in range(degree + 1):
        if n > 0:
            power = positive_mul(power, q)
        base = positive_mul(one_minus_q, power)
        row = [base]
        for k in range(1, 5):
            series = (hyp2f1_laguerre(n, k, q[0], False), hyp2f1_laguerre(n, k, q[1], True))
            row.append(positive_mul(positive_mul(base, inverse_rising(n, k)), series))
        out.append(row)
    return out


def midpoint(e) -> float:
    mid = 0.5 * e[0] + 0.5 * e[1]
    return min(max(mid, e[0]), e[1])


def distance_up(computed: float, e) -> float:
    return max(sub_up(computed, e[0]), sub_up(e[1], computed), 0.0)


def factorial_f64(k: int) -> float:
    acc = 1.0
    for i in range(1, k + 1):
        acc *= float(i)
    return acc


LAGUERRE_SCALES = [1.0, 2.0, 4.0, 8.0, 16.0]
MAX_POLYNOMIAL_DEGREE = 4096


def choose_laguerre_transform(rho: float, h: float, weight_factor: float, budget: float):
    """Emulation of choose_transform (Laguerre branch)."""
    best = None
    for scale in LAGUERRE_SCALES:
        beta = rho / scale
        a = (mul_down(h, beta), mul_up(h, beta))
        q_hi = div_up(a[1], add_down(1.0, a[1]))
        scale_up = div_up(rho, beta)
        if q_hi >= 1.0 or scale_up > 1400.0:
            continue
        norm = exp_up(mul_up(scale_up, 0.5))
        limit = MAX_POLYNOMIAL_DEGREE if best is None else max(best["degree"] - 1, 0)
        power = q_hi
        for degree in range(limit + 1):
            if degree > 0:
                power = mul_up(power, q_hi)
            tail = mul_up(norm, power)
            if math.isfinite(tail) and mul_up(tail, weight_factor) <= budget:
                best = {"beta": beta, "degree": degree, "scale": scale, "tail_factor": tail,
                        "norm": norm, "extent": scale_up, "a": a, "q_hi": q_hi}
                break
    if best is None:
        raise ValueError("no Laguerre degree meets the budget")
    return best


# ---------------------------------------------------------------------------
# Published-data helpers
# ---------------------------------------------------------------------------

def unhex(text: str) -> float:
    return struct.unpack(">d", bytes.fromhex(text))[0]


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


# Test constants of crates/rodas5p-core/tests/rnext04_laguerre_admission.rs.
SPECIAL_RHO = {"near-cancellation": 1.0, "subnormal-1e-310": 50.0, "large-1e300": 50.0}
SPECIAL_TRUNCATION_BUDGET = {"near-cancellation": 1.0e-12}
GRID_TRUNCATION_BUDGET = 1.0e-10
SAME_SCALES = [1.0, -0.5, 0.25, 2.0, -1.0]


def nominal_rho(label: str):
    m = re.match(r"^n\d+-rho([0-9.]+)-h", label)
    if m:
        return float(m.group(1))
    return SPECIAL_RHO.get(label)


def enclosure_rho(rho_nominal: float) -> float:
    # Rust: 1.2 * rho * (1.0 + 1.0e-12), evaluated left to right in binary64.
    return 1.2 * rho_nominal * (1.0 + 1.0e-12)


def gershgorin(a):
    n = len(a)
    lo, hi = INF, -INF
    for i in range(n):
        radius = sum_up(abs(a[i][j]) for j in range(n) if j != i)
        lo = min(lo, sub_down(a[i][i], radius))
        hi = max(hi, add_up(a[i][i], radius))
    return lo, hi


def find_component_fields(record) -> dict:
    found = {}

    def walk(obj, path):
        if isinstance(obj, dict):
            for key, value in obj.items():
                if key in COMPONENT_FIELDS or key == "column_errors":
                    found.setdefault(key, []).append(path + "." + key)
                walk(value, path + "." + key)
        elif isinstance(obj, list):
            for i, value in enumerate(obj):
                walk(value, f"{path}[{i}]")

    walk(record, "")
    return found


def published_fractions(record, total: float):
    """Fractions from published component fields, if every one is present."""
    errors = record.get("column_errors")
    if not isinstance(errors, list) or "fused_summation" not in record:
        return None
    sums = {name: mp.mpf(0) for name in COMPONENT_FIELDS}
    for column in errors:
        for name in COMPONENT_FIELDS[:-1]:
            if name not in column or column[name] is None:
                return None
            sums[name] += mp.mpf(column[name])
    sums["fused_summation"] = mp.mpf(record["fused_summation"])
    return {name: float(value / total) for name, value in sums.items()}


# ---------------------------------------------------------------------------
# Part B: interval evaluation of the Laguerre recurrence
# ---------------------------------------------------------------------------

def _init_worker():
    mp.dps = DPS
    iv.dps = DPS


def _ivpoint(x: mp.mpf):
    return iv.mpf(x)


def _upper(x) -> mp.mpf:
    return mp.mpf(x._mpi_[1])


def _lower(x) -> mp.mpf:
    return mp.mpf(x._mpi_[0])


def _mag(x) -> mp.mpf:
    return max(abs(_lower(x)), abs(_upper(x)))


def _mig(x) -> mp.mpf:
    lo, hi = _lower(x), _upper(x)
    if lo <= 0 <= hi:
        return mp.mpf(0)
    return min(abs(lo), abs(hi))


def iv_recurrence(x, n_max: int):
    """Enclosures of L_0(x) .. L_{n_max}(x) for an iv x (point or interval)."""
    values = [iv.mpf(1)]
    if n_max >= 1:
        values.append(1 - x)
    for n in range(1, n_max):
        values.append(((2 * n + 1 - x) * values[n] - n * values[n - 1]) / (n + 1))
    return values


def taylor_upper(c: mp.mpf, r: mp.mpf, n_max: int):
    """Upper ends of sum_k mag(a_k) r^k with L_n(c + t) = sum_k a_k t^k, the
    a_k enclosed by the three-term recurrence on polynomials in iv."""
    ci = _ivpoint(c)
    ri = _ivpoint(r)
    polys = [[iv.mpf(1)]]
    if n_max >= 1:
        polys.append([1 - ci, iv.mpf(-1)])
    for n in range(1, n_max):
        p, q = polys[n], polys[n - 1]
        alpha = (2 * n + 1) - ci
        new = [iv.mpf(0)] * (len(p) + 1)
        for k, v in enumerate(p):
            new[k] = new[k] + alpha * v
            new[k + 1] = new[k + 1] - v
        for k, v in enumerate(q):
            new[k] = new[k] - n * v
        polys.append([v / (n + 1) for v in new])
    uppers = []
    for p in polys:
        acc = iv.mpf(0)
        for v in reversed(p):
            acc = acc * ri + iv.mpf(_mag(v))
        uppers.append(_upper(acc))
    return uppers


def eval_piece(args):
    """For the piece [a, b]: Taylor-form upper ends U_n, certified point lower
    bounds V_n (max over a, c, b of the lower end of |L_n|), and the naive
    interval-extension upper ends."""
    a_raw, b_raw, n_max = args
    a, b = mp.mpf(a_raw), mp.mpf(b_raw)
    c = (a + b) / 2
    r = (b - a) / 2
    uppers = taylor_upper(c, r, n_max)
    lowers = [mp.mpf(0)] * (n_max + 1)
    for x in (a, c, b):
        for n, v in enumerate(iv_recurrence(_ivpoint(x), n_max)):
            lowers[n] = max(lowers[n], _mig(v))
    naive = [_mag(v) for v in iv_recurrence(iv.mpf([a, b]), n_max)]
    return ([u._mpf_ for u in uppers], [v._mpf_ for v in lowers], [v._mpf_ for v in naive])


def envelope(extent, n_max: int, workers: int, log=print):
    """Adaptive certified envelope on [0, extent] for n = 0..n_max."""
    extent = mp.mpf(extent)
    pending = [(mp.mpf(0), extent)]
    final = []
    best_lower = [mp.mpf(0)] * (n_max + 1)
    pieces = 1
    level = 0
    cap_reached = False
    evaluations = 0
    pool = multiprocessing.Pool(workers, initializer=_init_worker) if workers > 1 else None
    try:
        while pending:
            jobs = [(a._mpf_, b._mpf_, n_max) for a, b in pending]
            if pool is not None:
                results = pool.map(eval_piece, jobs, chunksize=max(1, len(jobs) // (4 * workers)))
            else:
                results = [eval_piece(j) for j in jobs]
            evaluations += len(jobs)
            parsed = []
            for (a, b), (u, v, naive) in zip(pending, results):
                u = [mp.mpf(x) for x in u]
                v = [mp.mpf(x) for x in v]
                naive = [mp.mpf(x) for x in naive]
                for n in range(n_max + 1):
                    if v[n] > best_lower[n]:
                        best_lower[n] = v[n]
                parsed.append((a, b, u, v, naive))
            failing = []
            for item in parsed:
                ok = all(item[2][n] <= REL_TOL * best_lower[n] for n in range(n_max + 1))
                if ok:
                    final.append(item)
                else:
                    failing.append(item)
            if failing and pieces + len(failing) > PIECE_CAP:
                cap_reached = True
                final.extend(failing)
                failing = []
            pending = []
            for a, b, *_ in failing:
                m = (a + b) / 2
                pending += [(a, m), (m, b)]
                pieces += 1
            log(f"  level {level}: evaluated {len(jobs)}, split {len(failing)}, cover {pieces}", flush=True)
            level += 1
    finally:
        if pool is not None:
            pool.close()
            pool.join()
    bounds = [max(item[2][n] for item in final) for n in range(n_max + 1)]
    naive_bounds = [max(item[4][n] for item in final) for n in range(n_max + 1)]
    return {
        "bounds": bounds,
        "lower": best_lower,
        "naive": naive_bounds,
        "pieces": len(final),
        "levels": level,
        "evaluations": evaluations,
        "cap_reached": cap_reached,
        "min_width": min(item[1] - item[0] for item in final),
    }


def up_str(x: mp.mpf, digits: int = 25) -> str:
    """Decimal string not below x."""
    s = mp.nstr(x, digits)
    if mp.mpf(s) >= x:
        return s
    bumped = mp.mpf(s) + abs(mp.mpf(s)) * mp.mpf(10) ** (1 - digits) + mp.mpf(10) ** (-DPS)
    s = mp.nstr(bumped, digits)
    assert mp.mpf(s) >= x
    return s


def nstr(x, digits: int = 8) -> str:
    return mp.nstr(mp.mpf(x), digits)


# ---------------------------------------------------------------------------
# G2: point evaluations
# ---------------------------------------------------------------------------

def point_values(args):
    """|L_n(x)| for n = 0..n_max at the given points (60-digit mpf recurrence):
    the per-n maxima, the (n, index) pairs above the bound, and the full rows
    of the points marked for the mpmath.laguerre cross-check."""
    xs_raw, start, n_max, bounds_raw = args
    bounds = [mp.mpf(b) for b in bounds_raw]
    maxima = [mp.mpf(0)] * (n_max + 1)
    violations = []
    cross = {}
    for offset, x_raw in enumerate(xs_raw):
        index = start + offset
        x = mp.mpf(x_raw)
        values = [mp.mpf(1), 1 - x]
        for n in range(1, n_max):
            values.append(((2 * n + 1 - x) * values[n] - n * values[n - 1]) / (n + 1))
        for n in range(n_max + 1):
            value = abs(values[n])
            if value > maxima[n]:
                maxima[n] = value
            if value > bounds[n]:
                violations.append((n, index))
        if index % G2_CROSS_STRIDE == 0:
            cross[index] = [values[n]._mpf_ for n in range(n_max + 1)]
    return [m._mpf_ for m in maxima], violations, cross


def g2_check(extent, bounds, n_max: int, workers: int):
    extent = mp.mpf(extent)
    xs = [min(extent, extent * i / (G2_POINTS - 1)) for i in range(G2_POINTS)]
    step = 250
    jobs = [([x._mpf_ for x in xs[i:i + step]], i, n_max, [b._mpf_ for b in bounds])
            for i in range(0, len(xs), step)]
    if workers > 1:
        with multiprocessing.Pool(workers, initializer=_init_worker) as pool:
            results = pool.map(point_values, jobs)
    else:
        results = [point_values(j) for j in jobs]
    max_point = [mp.mpf(0)] * (n_max + 1)
    violations = []
    cross = {}
    for maxima, viol, cr in results:
        for n in range(n_max + 1):
            max_point[n] = max(max_point[n], mp.mpf(maxima[n]))
        violations += viol
        cross.update(cr)
    # Cross-check the marked points against mpmath.laguerre.
    worst = mp.mpf(0)
    for index, row in sorted(cross.items()):
        for n in range(n_max + 1):
            ref = mp.laguerre(n, 0, xs[index])
            worst = max(worst, abs(ref - mp.mpf(row[n])) / max(1, abs(ref)))
    return {
        "points_per_n": G2_POINTS,
        "points": f"x_i = L' i / {G2_POINTS - 1}, i = 0..{G2_POINTS - 1}",
        "violations": len(violations),
        "first_violations": violations[:10],
        "max_point_value": max_point,
        "cross_check_points": len(cross),
        "cross_check_max_relative_difference": worst,
    }


# ---------------------------------------------------------------------------
# Self-test (no case data)
# ---------------------------------------------------------------------------

def self_test(workers: int) -> None:
    mp.dps = DPS
    iv.dps = DPS
    print("self-test: directed rounding")
    assert add_up(0.1, 0.2) >= 0.30000000000000004 and Fraction(add_up(0.1, 0.2)) >= Fraction(0.1) + Fraction(0.2)
    assert Fraction(add_down(0.1, 0.2)) <= Fraction(0.1) + Fraction(0.2)
    assert Fraction(mul_up(0.1, 0.3)) >= Fraction(0.1) * Fraction(0.3) >= Fraction(mul_down(0.1, 0.3))
    assert Fraction(div_up(1.0, 3.0)) >= Fraction(1, 3) >= Fraction(div_down(1.0, 3.0))
    assert exp_nonneg(1.0, True) >= mp.e >= exp_nonneg(1.0, False)
    print("self-test: emulated Laguerre coefficient enclosures contain the exact c_{n,k} (a = 0.5, degree 6)")
    a_val = 0.5
    table = laguerre_coefficients((a_val, a_val), 6)
    q = mp.mpf(a_val) / (1 + mp.mpf(a_val))
    for n, row in enumerate(table):
        for k in range(5):
            exact = (1 - q) * q ** n if k == 0 else (
                mp.factorial(n) / mp.factorial(n + k) * q ** n * (1 - q)
                * mp.hyp2f1(n + 1, k, n + k + 1, q))
            assert mp.mpf(row[k][0]) <= exact <= mp.mpf(row[k][1]), (n, k)
    # phi_k(-a x) = sum_n c_{n,k} L_n(x): check at x = 0.7 with many terms.
    x = mp.mpf("0.7")
    big = laguerre_coefficients((a_val, a_val), 120)
    lag = [mp.laguerre(n, 0, x) for n in range(121)]
    for k in range(5):
        series = mp.fsum(midpoint(big[n][k]) * lag[n] for n in range(121))
        z = -a_val * x
        ref = mp.e ** z
        for m in range(1, k + 1):
            ref = (ref - 1 / mp.factorial(m - 1)) / z
        assert abs(series - ref) < 1e-12, (k, series, ref)
    print("self-test: envelope n <= 5 on [0, 10]")
    result = envelope(10, 5, workers)
    bounds = result["bounds"]
    exact_known = {0: mp.mpf(1), 1: mp.mpf(9), 2: mp.mpf(31)}
    dense = [mp.mpf(0)] * 6
    for i in range(20001):
        xx = mp.mpf(10) * i / 20000
        vals = [mp.mpf(1), 1 - xx]
        for n in range(1, 5):
            vals.append(((2 * n + 1 - xx) * vals[n] - n * vals[n - 1]) / (n + 1))
        for n in range(6):
            dense[n] = max(dense[n], abs(vals[n]))
    for n in range(6):
        ok_lo = bounds[n] >= dense[n]
        ok_hi = bounds[n] <= REL_TOL * dense[n] * (1 + mp.mpf(10) ** -6)
        print(f"  n={n}: E_n <= {nstr(bounds[n], 12)}  dense max {nstr(dense[n], 12)}  "
              f"naive {nstr(result['naive'][n], 6)}  ok={ok_lo and ok_hi}")
        assert ok_lo and ok_hi, n
        if n in exact_known:
            assert exact_known[n] <= bounds[n] <= REL_TOL * exact_known[n]
    g2 = g2_check(10, bounds, 5, workers)
    assert g2["violations"] == 0
    assert g2["cross_check_max_relative_difference"] < mp.mpf(10) ** -40
    print(f"  pieces {result['pieces']}, levels {result['levels']}, cap {result['cap_reached']}")
    print("self-test: PASS")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def git_head() -> str:
    try:
        return subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True,
                              check=True).stdout.strip()
    except Exception:  # noqa: BLE001
        return "unknown"


def main() -> None:
    global REL_TOL
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--workers", type=int, default=max(1, multiprocessing.cpu_count()))
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    mp.dps = DPS
    iv.dps = DPS
    REL_TOL = mp.mpf("1.01")
    if args.self_test:
        self_test(args.workers)
        return
    if args.cases is None or args.output is None:
        raise SystemExit("--cases and --output are required")
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    started = time.time()
    data = json.loads(args.cases.read_text())

    # ---------------- Part A ----------------
    part_a_rows = []
    supplementary = []
    selected = []
    for case in data["cases"]:
        if "error" in case or case.get("basis") != "Laguerre" or case.get("branch") != "recurrence":
            continue
        rho_nom = nominal_rho(case["label"])
        if rho_nom is None:
            raise SystemExit(f"no rho derivation for {case['label']}")
        rho = enclosure_rho(rho_nom)
        h = unhex(case["h"])
        h_rho = Fraction(h) * Fraction(rho)
        if h_rho >= HRHO_MIN:
            selected.append((case, rho, h, h_rho))
    for case, rho, h, h_rho in selected:
        a = [[unhex(x) for x in row] for row in case["a"]]
        g_lo, g_hi = gershgorin(a)
        consistent = (-rho <= g_lo) and (g_hi <= 0.0) and case["evidence"] == "Gershgorin"
        scale = case["laguerre_scale"]
        beta = rho / scale
        extent = div_up(rho, beta)
        total = case["coverage"]["total"]
        found = find_component_fields(case)
        missing = [name for name in COMPONENT_FIELDS if name not in found]
        fractions = published_fractions(case, total) if not missing else None
        bottleneck = max(fractions, key=fractions.get) if fractions else None
        part_a_rows.append({
            "label": case["label"], "n": case["n"], "h": h, "rho_enclosure": rho,
            "h_rho": float(h_rho), "degree": case["degree"], "laguerre_scale": scale,
            "beta": beta, "L_prime": extent, "gershgorin_lo": g_lo, "gershgorin_hi": g_hi,
            "enclosure_consistent": consistent, "budget": case["budget"],
            "published_total": total, "admitted": case["admission"]["admitted"],
            "admission_reason": case["admission"].get("reason"),
            "published_component_fields": sorted(found),
            "missing_component_fields": missing,
            "fractions": fractions, "bottleneck": bottleneck,
        })

        # ---------- supplementary derived components (not gated) ----------
        same = case["label"].endswith("-same")
        w = [[unhex(x) for x in vec] for vec in case["w"]]
        if same:
            v = w[0]
            assert all(wk == [s * x for x in v] for wk, s in zip(w, SAME_SCALES))
            block, scales = [v], SAME_SCALES
            norms = [mul_up(abs(s), norm_up(v)) for s in scales]
        else:
            block, scales = w, [1.0] * 5
            norms = [norm_up(wk) for wk in w]
        weight = 0.0
        for k in range(5):
            weight = add_up(weight, div_up(norms[k], factorial_f64(k)))
        trunc_budget = SPECIAL_TRUNCATION_BUDGET.get(case["label"], GRID_TRUNCATION_BUDGET)
        transform = choose_laguerre_transform(rho, h, weight, trunc_budget)
        coefficients = laguerre_coefficients(transform["a"], transform["degree"])
        chosen = [[midpoint(e) for e in row] for row in coefficients]
        radii = [[distance_up(chosen[n][k], coefficients[n][k]) for k in range(5)]
                 for n in range(len(coefficients))]
        truncation = []
        coefficient = []
        for k in range(5):
            column = 0 if same else k
            source_norm = norm_up(block[column])
            ce = 0.0
            for n in range(len(coefficients)):
                ce = add_up(ce, mul_up(radii[n][k], transform["norm"]))
            ce = mul_up(ce, source_norm)
            coefficient.append(mul_up(abs(scales[k]), ce))
            truncation.append(mul_up(transform["tail_factor"], div_up(norms[k], factorial_f64(k))))
        supplementary.append({
            "label": case["label"], "same": same, "scales": scales,
            "source_norms": [norm_up(block[0 if same else k]) for k in range(5)],
            "radii": radii, "transform": transform,
            "truncation_columns": truncation, "coefficient_columns": coefficient,
            "truncation_budget": trunc_budget,
        })
    g1 = bool(part_a_rows) and all(r["fractions"] is not None for r in part_a_rows)
    print(f"Part A: {len(part_a_rows)} cases with h rho >= {HRHO_MIN}; G1 = {g1}", flush=True)

    # ---------------- Part B ----------------
    extents = sorted({r["L_prime"] for r in part_a_rows})
    part_b = []
    envelopes = {}
    for extent in extents:
        print(f"Part B: envelope on [0, {extent!r}] for n = 0..{N_MAX}", flush=True)
        t0 = time.time()
        res = envelope(extent, N_MAX, args.workers)
        t_env = time.time() - t0
        print(f"Part B: G2 point check on [0, {extent!r}]", flush=True)
        t0 = time.time()
        g2 = g2_check(extent, res["bounds"], N_MAX, args.workers)
        t_g2 = time.time() - t0
        exp_half = mp.e ** (mp.mpf(extent) / 2)
        exp_half_native = exp_up(mul_up(extent, 0.5))
        max_e = max(res["bounds"])
        argmax = res["bounds"].index(max_e)
        naive_first_loose = next((n for n in range(N_MAX + 1)
                                  if res["naive"][n] > REL_TOL * res["bounds"][n]), None)
        envelopes[extent] = (res, exp_half, exp_half_native)
        part_b.append({
            "L_prime": extent, "n_max": N_MAX,
            "E_n_upper": [up_str(x) for x in res["bounds"]],
            "attained_lower": [nstr(x, 20) for x in res["lower"]],
            "upper_over_lower_max": nstr(max(res["bounds"][n] / res["lower"][n]
                                             for n in range(N_MAX + 1)), 10),
            "max_E_n": up_str(max_e), "argmax_n": argmax,
            "exp_half_L_prime": nstr(exp_half, 25),
            "exp_half_L_prime_native_binary64": exp_half_native,
            "ratio_exp_half_over_max_E": nstr(exp_half / max_e, 12),
            "ratio_exp_half_over_E_n": [nstr(exp_half / x, 6) for x in res["bounds"]],
            "pieces": res["pieces"], "levels": res["levels"], "evaluations": res["evaluations"],
            "cap_reached": res["cap_reached"], "min_piece_width": nstr(res["min_width"], 6),
            "naive_interval_extension_on_final_cover": [nstr(x, 6) for x in res["naive"]],
            "naive_first_n_above_1pct": naive_first_loose,
            "seconds_envelope": round(t_env, 1),
            "g2": {
                "violations": g2["violations"], "first_violations": g2["first_violations"],
                "points_per_n": g2["points_per_n"], "points": g2["points"],
                "max_point_value": [nstr(x, 20) for x in g2["max_point_value"]],
                "min_bound_over_max_point": nstr(min(res["bounds"][n] / g2["max_point_value"][n]
                                                     for n in range(N_MAX + 1)), 12),
                "max_bound_over_max_point": nstr(max(res["bounds"][n] / g2["max_point_value"][n]
                                                     for n in range(N_MAX + 1)), 12),
                "naive_violations": sum(1 for n in range(N_MAX + 1)
                                        if g2["max_point_value"][n] > res["naive"][n]),
                "cross_check_points": g2["cross_check_points"],
                "cross_check_max_relative_difference": nstr(g2["cross_check_max_relative_difference"], 3),
                "seconds": round(t_g2, 1),
            },
        })
        print(f"  max E_n {nstr(max_e, 10)} at n={argmax}; e^(L'/2)/max E_n = "
              f"{nstr(exp_half / max_e, 8)}; pieces {res['pieces']}; G2 violations {g2['violations']}",
              flush=True)
    g2_ok = bool(part_b) and all(b["g2"]["violations"] == 0 for b in part_b)

    # ---------------- G3 (reported) ----------------
    g3_rows = []
    for row, supp in zip(part_a_rows, supplementary):
        res, exp_half, exp_half_native = envelopes[row["L_prime"]]
        max_e = max(res["bounds"])
        ratio = exp_half / max_e
        total = mp.mpf(row["published_total"])
        budget = mp.mpf(row["budget"])
        lb_uniform = total / ratio
        lb_any = total / exp_half
        # supplementary derived recomputation
        trunc = mp.fsum(mp.mpf(x) for x in supp["truncation_columns"])
        coef = mp.fsum(mp.mpf(x) for x in supp["coefficient_columns"])
        remainder = total - trunc - coef
        coef_e = mp.mpf(0)
        for k in range(5):
            norm = mp.mpf(supp["source_norms"][k]) * abs(mp.mpf(supp["scales"][k]))
            coef_e += norm * mp.fsum(mp.mpf(supp["radii"][n][k]) * res["bounds"][n]
                                     for n in range(len(supp["radii"])))
        total_e = remainder + trunc + coef_e
        shares = {"truncation": trunc / total, "coefficient": coef / total,
                  "remainder_adjoint_summation_fused_normalization": remainder / total}
        g3_rows.append({
            "label": row["label"], "published_total": row["published_total"],
            "published_budget": row["budget"],
            "ratio_exp_half_over_max_E": nstr(ratio, 10),
            "implied_lower_bound_uniform_maxE": nstr(lb_uniform, 8),
            "implied_lower_bound_any_envelope": nstr(lb_any, 8),
            "could_meet_budget_uniform_maxE": bool(lb_uniform <= budget),
            "could_meet_budget_any_envelope": bool(lb_any <= budget),
            "derived_supplementary": {
                "emulated_degree": supp["transform"]["degree"],
                "emulated_scale": supp["transform"]["scale"],
                "matches_published_degree_and_scale": (
                    supp["transform"]["degree"] == row["degree"]
                    and supp["transform"]["scale"] == row["laguerre_scale"]),
                "emulated_L_prime": supp["transform"]["extent"],
                "truncation": nstr(trunc, 8), "coefficient": nstr(coef, 8),
                "remainder": nstr(remainder, 12),
                "shares": {k: nstr(v, 6) for k, v in shares.items()},
                "bottleneck": max(shares, key=shares.get),
                "coefficient_with_E_n": nstr(coef_e, 8),
                "total_with_E_n": nstr(total_e, 12),
                "total_with_E_n_meets_budget": bool(total_e <= budget),
                "truncation_kept": "tail term kept with e^{L'/2} (its sum runs past n = 128)",
            },
        })
    elapsed = time.time() - started
    report = {
        "schema": SCHEMA, "node": NODE, "mpmath": mpmath.__version__, "dps": DPS,
        "python": sys.version.split()[0], "source_commit": git_head(),
        "command": " ".join(["python3", "tools/pp09_laguerre_envelope.py"] + sys.argv[1:]),
        "inputs": {"cases": str(args.cases), "cases_sha256": sha256(args.cases)},
        "part_a": {
            "rule": f"Laguerre recurrence cases with h * rho_enc >= {HRHO_MIN}",
            "rho_derivation": "rho_enc = 1.2 * rho_nominal * (1 + 1e-12) (binary64), from the R-NEXT-04 test "
                              "source; not published in cases.json",
            "component_fields_sought": COMPONENT_FIELDS + ["column_errors"],
            "cases": part_a_rows,
            "covers_all": g1,
        },
        "part_b": {
            "method": "three-term recurrence on local polynomials in t = x - c with mpmath.iv coefficients "
                      "(60 digits) on dyadic pieces [c - r, c + r]; |L_n| <= sum_k mag(a_k) r^k",
            "rule": "bisect level by level; a piece is final when for every n its upper end <= 1.01 x the "
                    "largest certified point lower bound of |L_n| found so far, or when the cover would "
                    "exceed 2^20 pieces; E_n = max upper end over the final cover",
            "envelopes": part_b,
        },
        "g3": {
            "registered_recomputation": "not possible from published fields (cases.json has no coefficient "
                                        "radii, tails or other components); ratio only",
            "rows": g3_rows,
        },
        "gate": {"G1": g1, "G2": g2_ok},
        "verdict": "PASS" if (g1 and g2_ok) else "FAIL",
        "runtime_seconds": round(elapsed, 1),
        "workers": args.workers,
    }
    args.output.write_text(json.dumps(report, indent=1, default=str) + "\n")
    print(json.dumps({"gate": report["gate"], "verdict": report["verdict"],
                      "runtime_seconds": report["runtime_seconds"]}))


if __name__ == "__main__":
    main()
