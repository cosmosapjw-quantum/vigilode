"""Exact consumer of independently supplied stage/reconstruction bounds (S1–S3)."""
from fractions import Fraction


def rational(value):
    if type(value) not in (int, Fraction):
        raise TypeError('bound inputs must be exact int/Fraction')
    return Fraction(value)


def stage_bound(alpha, residual, lipschitz):
    """Return the finite triangular recurrence; values do not certify their source."""
    a = tuple(map(rational, alpha))
    d = tuple(map(rational, residual))
    l = tuple(tuple(map(rational, row)) for row in lipschitz)
    n = len(a)
    if n == 0 or len(d) != n or len(l) != n or any(len(row) != n for row in l):
        raise ValueError('nonempty equal dimensions required')
    if any(x <= 0 for x in a) or any(x < 0 for x in d):
        raise ValueError('positive inverse margins and nonnegative residual bounds required')
    if any(l[i][j] < 0 or (j >= i and l[i][j] != 0) for i in range(n) for j in range(n)):
        raise ValueError('L must be nonnegative and strictly lower triangular')
    out = []
    for i in range(n):
        out.append((d[i] + sum(l[i][j]*out[j] for j in range(i)))/a[i])
    return tuple(out)


def reconstruction_bound(base_residual, c0, c1, field_lipschitz, stage_bounds):
    """S3 route only. Do not add this again to a direct actual-path residual."""
    delta, c0, c1, lf = map(rational, (base_residual, c0, c1, field_lipschitz))
    eta = tuple(map(rational, stage_bounds))
    if not eta or min(delta, c0, c1, lf, *eta) < 0:
        raise ValueError('nonempty nonnegative bound inputs required')
    return delta + (c1+lf*c0)*max(eta)
