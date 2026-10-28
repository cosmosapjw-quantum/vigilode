"""Exact-rational chemical6 signed lognorm for the frozen BASS source topology.
No actual owner-accepted rate provider is supplied. Results are conditional.
"""
from __future__ import annotations
from dataclasses import dataclass
from fractions import Fraction as F
from itertools import product
from typing import Sequence

N, NR = 6, 12
TH = F(2**53 - 1, 2**53)
TOTALS = (TH, F(1), F(1))
# photo H0,He0,He1,Li0,Li1,Li2; recomb H1,He1,He2,Li1,Li2,Li3
P_BASIS = (
    ((0, 0, -1),),
    ((1, 1, -1), (1, 2, -1)),
    ((1, 1, -1), (2, 1, 1)),
    ((3, 3, -1), (3, 4, -1), (3, 5, -1)),
    ((3, 3, -1), (4, 3, 1)),
    ((4, 4, -1), (5, 4, 1)),
)
Q_BASIS = (
    ((0, 0, -1),),
    ((1, 1, -1),),
    ((1, 2, 1), (2, 2, -1)),
    ((3, 3, -1),),
    ((3, 4, 1), (4, 4, -1)),
    ((4, 5, 1), (5, 5, -1)),
)


def vertices(totals=TOTALS):
    th, the, tli = totals
    assert min(th, the, tli) > 0
    for h in (F(0), th):
        for he in ((F(0), F(0)), (the, F(0)), (F(0), the)):
            for li in ((F(0), F(0), F(0)), (tli, F(0), F(0)),
                       (F(0), tli, F(0)), (F(0), F(0), tli)):
                yield (h, *he, *li)


def jac_basis(z: Sequence[F], e: Sequence[F], a: int):
    """J for one unit rate, including the electron feedback outer product."""
    assert len(z) == len(e) == N and 0 <= a < NR
    j = [[F(0) for _ in range(N)] for _ in range(N)]
    if a < 6:
        for i, k, c in P_BASIS[a]:
            j[i][k] += F(c)
        return j
    q = [[F(0) for _ in range(N)] for _ in range(N)]
    for i, k, c in Q_BASIS[a - 6]:
        q[i][k] += F(c)
    E = sum(e[k] * z[k] for k in range(N))
    qz = [sum(q[i][k] * z[k] for k in range(N)) for i in range(N)]
    for i in range(N):
        for k in range(N):
            j[i][k] = E * q[i][k] + qz[i] * e[k]
    return j


def full_jac(z, e, rates):
    out = [[F(0) for _ in range(N)] for _ in range(N)]
    for a in range(NR):
        ja = jac_basis(z, e, a)
        for i in range(N):
            for k in range(N):
                out[i][k] += rates[a] * ja[i][k]
    return out


def mu_weighted(j, weights):
    return max(j[i][i] + sum(abs(j[i][k]) * weights[k] / weights[i]
                                for k in range(N) if k != i)
               for i in range(N))


@dataclass(frozen=True)
class Witness:
    value: F
    row: int
    vertex_index: int
    signs: tuple[int, ...]
    rate_endpoints: tuple[F, ...]
    state: tuple[F, ...]


def sharp_mu(e: Sequence[F], weights: Sequence[F], bounds, totals=TOTALS) -> Witness:
    """Exact max over 24 state vertices, 32 row signs and 12 rate endpoints."""
    assert len(e) == len(weights) == N and len(bounds) == NR
    assert all(v > 0 for v in weights) and all(v > 0 for v in (e[0], e[1], e[3]))
    assert all(lo >= 0 and hi >= lo for lo, hi in bounds)
    best = None
    for vi, z in enumerate(vertices(totals)):
        basis = [jac_basis(z, e, a) for a in range(NR)]
        for i in range(N):
            others = [k for k in range(N) if k != i]
            for signs in product((-1, 1), repeat=N - 1):
                coeff = [b[i][i] + sum(F(s) * weights[k] / weights[i] * b[i][k]
                                        for k, s in zip(others, signs)) for b in basis]
                chosen = tuple(hi if c >= 0 else lo for c, (lo, hi) in zip(coeff, bounds))
                val = sum(c * rate for c, rate in zip(coeff, chosen))
                if best is None or val > best.value:
                    best = Witness(val, i, vi, signs, chosen, z)
    assert best is not None
    return best


def entrywise_upper(e, weights, bounds, totals=TOTALS):
    """Valid but correlation-losing 36-entry upper bound."""
    lower = [[None] * N for _ in range(N)]
    upper = [[None] * N for _ in range(N)]
    for z in vertices(totals):
        basis = [jac_basis(z, e, a) for a in range(NR)]
        for i in range(N):
            for j in range(N):
                c = [b[i][j] for b in basis]
                lo = sum(v * (rlo if v >= 0 else rhi) for v, (rlo, rhi) in zip(c, bounds))
                hi = sum(v * (rhi if v >= 0 else rlo) for v, (rlo, rhi) in zip(c, bounds))
                lower[i][j] = lo if lower[i][j] is None else min(lower[i][j], lo)
                upper[i][j] = hi if upper[i][j] is None else max(upper[i][j], hi)
    return max(upper[i][i] + sum(max(abs(lower[i][j]), abs(upper[i][j]))
                                   * weights[j] / weights[i] for j in range(N) if j != i)
               for i in range(N))


def full_corner_mu(e, weights, bounds, totals=TOTALS):
    """Independent direct J and all corners for at most four variable rates."""
    assert sum(lo != hi for lo, hi in bounds) <= 4
    return max(mu_weighted(full_jac(z, e, rates), weights)
               for z in vertices(totals)
               for rates in product(*[(lo, hi) if lo != hi else (lo,) for lo, hi in bounds]))


def rational_error_radius(mu: F, h: F, delta: F, E0: F) -> F | None:
    assert h > 0 and delta >= 0 and E0 >= 0
    m = max(mu, F(0))
    if m * h >= 1:
        return None
    return (E0 + h * delta) / (1 - m * h)


def default_fixture():
    # Synthetic dyadic values, not the owner-accepted isotope weights/rates.
    e = (F(1, 8), F(1, 8), F(1, 4), F(1, 8), F(1, 4), F(3, 8))
    w = (F(1),) * N
    rates = tuple((F(1, 16), F(1, 8)) for _ in range(6)) + \
            tuple((F(1, 32), F(1, 16)) for _ in range(6))
    return e, w, rates


if __name__ == "__main__":
    import json, time
    t = time.monotonic()
    e, w, rates = default_fixture()
    best = sharp_mu(e, w, rates)
    naive = entrywise_upper(e, w, rates)
    assert best.value <= naive
    assert mu_weighted(full_jac(best.state, e, best.rate_endpoints), w) == best.value
    print(json.dumps({
        "schema": "vigilode.chem6.mu01.exact.v1",
        "scope": "synthetic rational 12-rate box; not owner accepted",
        "sharp_mu": str(best.value), "sharp_mu_float": float(best.value),
        "entrywise_mu": str(naive), "entrywise_mu_float": float(naive),
        "difference": str(naive - best.value),
        "witness": {"row": best.row, "vertex_index": best.vertex_index,
                    "state": list(map(str, best.state)), "signs": best.signs,
                    "rates": list(map(str, best.rate_endpoints))},
        "runtime_seconds": time.monotonic() - t,
        "core_DAG": "7/23 inherited scoped", "central_REI": "28/88", "production": "HOLD"
    }, indent=2))
