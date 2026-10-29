"""Exact, opt-in oracle for K3; no production dispatcher integration.

All inputs represent exact integers/rationals. This executable oracle verifies
the original dense matrix residual and a Euclidean coercivity error bound.
Dense inspection and exact arithmetic are reference work, not native timings.
"""

from dataclasses import dataclass
from fractions import Fraction


class QuotientRefused(ValueError):
    """The chosen representation or supplied certificate is not admissible."""


def _q(value):
    if isinstance(value, bool) or not isinstance(value, (int, Fraction)):
        raise TypeError("only exact int/Fraction inputs are accepted")
    return Fraction(value)


def matvec(matrix, vector):
    return tuple(sum((a * b for a, b in zip(row, vector)), Fraction(0))
                 for row in matrix)


@dataclass(frozen=True)
class QuotientResult:
    candidate: tuple
    original_residual: tuple
    relation_defect: tuple
    predicted_residual: tuple
    denominator: Fraction
    alpha: Fraction
    squared_error_bound: Fraction
    requested_error_cap: Fraction | None
    admitted: bool | None
    matrix_vector_actions: int


def symmetric_gershgorin_upper(matrix):
    """Exact upper bound on lambda_max((J+J.T)/2), not on eigenvalues(J)."""
    n = len(matrix)
    return max(matrix[i][i] + sum(
        (abs(matrix[i][j] + matrix[j][i]) / 2 for j in range(n) if j != i),
        Fraction(0)) for i in range(n))


def directional_quotient(matrix, rhs, polynomial, sigma, error_cap=None):
    """Evaluate the exact quotient and verify its original residual.

    `polynomial` has ascending coefficients and must be monic, degree >= 1.
    A conservative Euclidean numerical-abscissa bound is computed from the
    dense symmetric part. A nonpositive coercivity alpha refuses certification.
    A zero quotient denominator refuses this representation even if A is
    nonsingular. A candidate exceeding `error_cap` is returned with admitted
    false, retaining the adverse residual and error bound. Without a requested
    cap, admitted is None: bound availability is not an accuracy decision.
    """
    n = len(rhs)
    if n == 0 or len(matrix) != n or any(len(row) != n for row in matrix):
        raise ValueError("nonempty square matrix and matching rhs required")
    matrix = tuple(tuple(_q(v) for v in row) for row in matrix)
    rhs = tuple(_q(v) for v in rhs)
    p = tuple(_q(v) for v in polynomial)
    sigma = _q(sigma)
    if sigma < 0:
        raise ValueError("sigma must be nonnegative")
    if len(p) < 2 or p[-1] != 1:
        raise ValueError("a monic polynomial of positive degree is required")
    if error_cap is not None:
        error_cap = _q(error_cap)
        if error_cap < 0:
            raise ValueError("error cap must be nonnegative")
    d = len(p) - 1
    denominator = sum((p[j] * sigma ** (d - j) for j in range(d + 1)),
                      Fraction(0))
    if denominator == 0:
        raise QuotientRefused("quotient denominator is zero")
    alpha = 1 - sigma * symmetric_gershgorin_upper(matrix)
    if alpha <= 0:
        raise QuotientRefused("positive coercivity bound is unavailable")
    powers = [rhs]
    for _ in range(d):
        powers.append(matvec(matrix, powers[-1]))
    coefficients = [sum((p[j] * sigma ** (d - j + k)
                         for j in range(k + 1, d + 1)), Fraction(0))
                    for k in range(d)]
    candidate = tuple(sum((coefficients[k] * powers[k][i] for k in range(d)),
                          Fraction(0)) / denominator for i in range(n))
    jx = matvec(matrix, candidate)
    residual = tuple(rhs[i] - candidate[i] + sigma * jx[i] for i in range(n))
    defect = tuple(sum((p[k] * powers[k][i] for k in range(d + 1)),
                       Fraction(0)) for i in range(n))
    predicted = tuple(sigma ** d * v / denominator for v in defect)
    if residual != predicted:
        raise ArithmeticError("original residual differs from polynomial identity")
    error_squared = sum((v * v for v in residual), Fraction(0)) / alpha ** 2
    admitted = None if error_cap is None else error_squared <= error_cap ** 2
    return QuotientResult(candidate, residual, defect, predicted, denominator,
                          alpha, error_squared, error_cap, admitted, d + 1)
