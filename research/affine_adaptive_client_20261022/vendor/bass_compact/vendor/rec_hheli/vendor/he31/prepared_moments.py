"""Prepared binary64 moments of the existing positive canonical FV cells.

This is an algebraic evaluator for an actual ``CanonicalMomentumFV`` instance,
not a new quadrature, distribution, closure, or time integrator. The caller
must load the source-pinned ``canonical_fv`` module before importing this file.
No vendor source is modified and no performance measurement is implied.

The prepared arrays are immutable byte-backed snapshots. Each scalar call uses
two packet-sized work arrays, one square root, one reciprocal, and three dot
reductions. Batch work arrays are capped independently of the number of input
states; the required input/output arrays are not part of that workspace cap.
Reduction order differs from NumPy's original elementwise sum: parity is
binary64 tolerance parity, not bitwise equality. Use one BLAS thread per worker
when the outer sweep supplies parallelism.

HE31 uses 0<=eta<=ln(7). At much larger eta the reordered normalization
can underflow later than the original; no subnormal relative/bitwise parity
is asserted outside the physical pilot interval.
"""
from __future__ import annotations

from dataclasses import dataclass, field
import math

import numpy as np

from canonical_fv import CanonicalMomentumFV, FVMoments


def _immutable_flat(values: np.ndarray) -> np.ndarray:
    """Own immutable bytes, so setting WRITEABLE=True cannot mutate the cache."""
    return np.frombuffer(np.asarray(values, dtype=np.float64).tobytes(order="C"),
                         dtype=np.float64)


@dataclass(frozen=True, init=False, eq=False)
class PreparedCanonicalMoments:
    """Drop-in moment/cell interface consumed by ``CanonicalFVSpecies``.

    Public scalar metadata and canonical cell masses retain the source values.
    Private source quadrature arrays are deliberately not exposed as a second
    mutable distribution. Construction is one-time O(number of packets).
    """

    mass: float
    amplitude: float
    radial_cells: int
    angular_cells: int
    gauss_order: int
    reference_energy_norm: float
    _mass_squared: float
    _number_sum: float
    _min_cell_number: float
    _cell_numbers: np.ndarray = field(repr=False)
    _q_perp2: np.ndarray = field(repr=False)
    _q_parallel2: np.ndarray = field(repr=False)
    _weights: np.ndarray = field(repr=False)
    _weighted_q_perp2: np.ndarray = field(repr=False)
    _weighted_q_parallel2: np.ndarray = field(repr=False)

    def __init__(self, source: CanonicalMomentumFV):
        if not isinstance(source, CanonicalMomentumFV):
            raise TypeError("prepare an actual source-pinned CanonicalMomentumFV")
        for key in ("mass", "amplitude", "radial_cells", "angular_cells",
                    "gauss_order", "reference_energy_norm"):
            object.__setattr__(self, key, getattr(source, key))
        object.__setattr__(self, "_mass_squared", self.mass * self.mass)
        object.__setattr__(self, "_number_sum", source.total_cell_number)
        object.__setattr__(self, "_min_cell_number", float(np.min(source.cell_numbers)))
        cells = _immutable_flat(source.cell_numbers).reshape(source.cell_numbers.shape)
        object.__setattr__(self, "_cell_numbers", cells)
        q2 = np.asarray(source._q, dtype=np.float64) ** 2
        qp2 = _immutable_flat(q2 * (1.0 - source._mu2))
        qz2 = _immutable_flat(q2 * source._mu2)
        weights = _immutable_flat(source._numbers / source.reference_energy_norm)
        object.__setattr__(self, "_q_perp2", qp2)
        object.__setattr__(self, "_q_parallel2", qz2)
        object.__setattr__(self, "_weights", weights)
        object.__setattr__(self, "_weighted_q_perp2", _immutable_flat(weights * qp2))
        object.__setattr__(self, "_weighted_q_parallel2", _immutable_flat(weights * qz2))

    @property
    def cell_numbers(self) -> np.ndarray:
        return self._cell_numbers

    @property
    def total_cell_number(self) -> float:
        return self._number_sum

    @property
    def packet_count(self) -> int:
        return self._weights.size

    @property
    def prepared_array_bytes(self) -> int:
        """Persistent array bytes, excluding Python object/header overhead."""
        return sum(a.nbytes for a in (self._cell_numbers, self._q_perp2,
            self._q_parallel2, self._weights, self._weighted_q_perp2,
            self._weighted_q_parallel2))

    def distribution_change_deta(self) -> np.ndarray:
        return np.zeros_like(self._cell_numbers)

    def moments(self, eta: float, b: float) -> FVMoments:
        # Preserve the original state domain, including arbitrary finite eta>=0.
        if not all(map(math.isfinite, (eta, b))) or eta < 0 or abs(b) > 1:
            raise ValueError("finite positive cosmological time and Bianchi chart")
        r = math.exp(-eta)
        r3 = r ** 3
        scale_perp = r * r * math.exp(2 * b)
        scale_parallel = r * r * math.exp(-4 * b)
        energy = np.empty_like(self._weights)
        inverse = np.empty_like(self._weights)
        np.multiply(self._q_perp2, scale_perp, out=energy)
        np.multiply(self._q_parallel2, scale_parallel, out=inverse)
        np.add(energy, inverse, out=energy)
        np.add(energy, self._mass_squared, out=energy)
        np.sqrt(energy, out=energy)
        np.reciprocal(energy, out=inverse)
        rho = r3 * float(np.dot(self._weights, energy))
        pp = 0.5 * r3 * scale_perp * float(np.dot(self._weighted_q_perp2, inverse))
        pa = r3 * scale_parallel * float(np.dot(self._weighted_q_parallel2, inverse))
        return FVMoments(rho, pp, pa, self._number_sum * r3, self._min_cell_number)

    def moment_batch(self, eta, b, chunk: int = 32, *,
                     max_work_bytes: int = 64 * 1024 * 1024) -> np.ndarray:
        """Return rho/P_perp/P_parallel; cap two packet work matrices in bytes.

        ``chunk`` retains the vendor domain 1..256 and is a maximum. The actual
        chunk also obeys ``max_work_bytes``. State vectors and the (N,3) output
        require O(N) storage; persistent prepared arrays require O(packets).
        """
        xx = np.asarray(eta, dtype=np.float64)
        bb = np.asarray(b, dtype=np.float64)
        if (xx.ndim != 1 or bb.shape != xx.shape or
                np.any(~np.isfinite(xx)) or np.any(xx < 0) or
                np.any(~np.isfinite(bb)) or np.any(abs(bb) > 1) or
                not isinstance(chunk, int) or not 1 <= chunk <= 256):
            raise ValueError("bounded physical batch")
        bytes_per_state = 2 * self.packet_count * np.dtype(np.float64).itemsize
        if (not isinstance(max_work_bytes, int) or isinstance(max_work_bytes, bool)
                or max_work_bytes < bytes_per_state):
            raise ValueError("workspace must fit two binary64 packet arrays")
        out = np.empty((len(xx), 3), dtype=np.float64)
        if len(xx) == 0:
            return out
        width = min(chunk, max_work_bytes // bytes_per_state, len(xx))
        energy_buffer = np.empty((width, self.packet_count), dtype=np.float64)
        inverse_buffer = np.empty_like(energy_buffer)
        for start in range(0, len(xx), width):
            end = min(start + width, len(xx))
            energy = energy_buffer[:end-start]
            inverse = inverse_buffer[:end-start]
            r = np.exp(-xx[start:end])
            r3 = r ** 3
            scale_perp = r * r * np.exp(2 * bb[start:end])
            scale_parallel = r * r * np.exp(-4 * bb[start:end])
            np.multiply(self._q_perp2[None, :], scale_perp[:, None], out=energy)
            np.multiply(self._q_parallel2[None, :], scale_parallel[:, None], out=inverse)
            np.add(energy, inverse, out=energy)
            np.add(energy, self._mass_squared, out=energy)
            np.sqrt(energy, out=energy)
            np.reciprocal(energy, out=inverse)
            out[start:end, 0] = r3 * np.dot(energy, self._weights)
            out[start:end, 1] = (0.5 * r3 * scale_perp *
                               np.dot(inverse, self._weighted_q_perp2))
            out[start:end, 2] = (r3 * scale_parallel *
                               np.dot(inverse, self._weighted_q_parallel2))
        return out
