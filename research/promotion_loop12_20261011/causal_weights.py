"""Exact causal beta-weight compiler for the Loop12 research reference.

This is a scalar arithmetic realization of the corrected cost contract, not a
native ODE solver or a bit-complexity/wall-time claim. A table belongs to the
declared maximum kernel/forcing degrees and can be reused for all modes/sweeps.
"""
from dataclasses import dataclass
from fractions import Fraction


@dataclass(frozen=True)
class CausalWeights:
    kernel_degree: int
    forcing_degree: int
    rows: tuple[tuple[Fraction, ...], ...]

    @classmethod
    def compile(cls, kernel_degree: int, forcing_degree: int):
        if any(type(x) is not int or x < 0 for x in (kernel_degree, forcing_degree)):
            raise ValueError("nonnegative integer degrees required")
        rows = []
        for j in range(kernel_degree + 1):
            row = [Fraction(1, j + 1)]
            for k in range(forcing_degree):
                row.append(row[-1] * Fraction(k + 1, j + k + 2))
            rows.append(tuple(row))
        return cls(kernel_degree, forcing_degree, tuple(rows))

    def check_shape(self, kernel, forcing):
        if not kernel or not forcing:
            raise ValueError("nonempty coefficient sequences required")
        if len(kernel) > self.kernel_degree + 1 or len(forcing) > self.forcing_degree + 1:
            raise ValueError("coefficient degrees exceed compiled table")


def convolve(kernel, forcing, table: CausalWeights):
    """Integral_0^t K(t-s)f(s)ds, with one coefficient term per pair.

    Coefficients may be exact rational or exact complex-rational scalars. The
    horizon scale for tau=t/T is applied separately by the caller. The caller
    must use a table returned by compile; this is a trusted internal reference.
    """
    table.check_shape(kernel, forcing)
    zero = kernel[0] * forcing[0] * 0
    out = [zero for _ in range(len(kernel) + len(forcing))]
    for j, a in enumerate(kernel):
        for k, b in enumerate(forcing):
            out[j + k + 1] += a * b * table.rows[j][k]
    return tuple(out)


def endpoint_force_coefficients(forcing, table: CausalWeights):
    """Shared sums sum_k f_k*j!*k!/(j+k+1)! for every kernel degree j.

    Multiplication by T*(i*Delta*T)^j/j! is separate. This O(pD) setup must be
    charged again at each distinct requested output time/forcing profile.
    """
    table.check_shape((1,), forcing)
    zero = forcing[0] * 0
    return tuple(sum((b * row[k] for k, b in enumerate(forcing)), zero)
                 for row in table.rows)
