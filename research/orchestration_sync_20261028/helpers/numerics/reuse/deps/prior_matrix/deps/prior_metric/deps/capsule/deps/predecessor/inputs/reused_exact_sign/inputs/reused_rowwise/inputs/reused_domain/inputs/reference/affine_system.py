"""Opt-in 30-coordinate restriction of the pinned 33-coordinate H--Li model.

The three eliminated neutral fractions are reconstructed from conserved totals.
No positivity clipping or rate change is performed. Adaptive error control must
use the lifted full33 error if its original norm is to be retained.
"""
from __future__ import annotations
import math
import numpy as np

BLOCKS = ((23, 24), (25, 26, 27), (28, 29, 30, 31))
NEUTRAL = np.array([b[0] for b in BLOCKS])
KEEP = np.array([i for i in range(33) if i not in NEUTRAL])
SELECT = np.eye(33)[KEEP]
LIFT = SELECT.T.copy()
for block in BLOCKS:
    for charged in block[1:]:
        LIFT[block[0], np.flatnonzero(KEEP == charged)[0]] = -1.
for a in (NEUTRAL, KEEP, SELECT, LIFT):
    a.setflags(write=False)


class AffineSystem:
    """Wrap an already validated CompactSystem; preserve its source and IC scope."""
    size = 30

    def __init__(self, compact):
        if getattr(compact, 'size', None) != 33:
            raise ValueError('expected validated CompactSystem of size 33')
        initial = np.asarray(compact.y0, dtype=float)
        if initial.shape != (33,) or not np.isfinite(initial).all():
            raise ValueError('finite full33 initial state required')
        if np.any(initial[23:32] < 0):
            raise ValueError('initial fractions must be nonnegative')
        self.compact = compact
        self.totals = np.array([math.fsum(initial[list(b)]) for b in BLOCKS])
        if np.any(self.totals <= 0):
            raise ValueError('positive elemental fraction totals required')
        self.totals.setflags(write=False)
        self.y0 = self.restrict(initial)

    @staticmethod
    def restrict(full):
        full = np.asarray(full)
        if full.ndim < 1 or full.shape[0] != 33:
            raise ValueError('expected leading full33 axis')
        return full[KEEP].copy()

    def lift(self, reduced):
        reduced = np.asarray(reduced)
        if reduced.ndim < 1 or reduced.shape[0] != 30:
            raise ValueError('expected leading reduced30 axis')
        full = np.zeros((33,) + reduced.shape[1:], dtype=np.result_type(reduced, float))
        full[KEEP] = reduced
        for total, block in zip(self.totals, BLOCKS):
            full[block[0]] = total - np.sum(full[list(block[1:])], axis=0)
        return full

    @staticmethod
    def lift_tangent(direction):
        direction = np.asarray(direction)
        if direction.ndim < 1 or direction.shape[0] != 30:
            raise ValueError('expected leading reduced30 tangent axis')
        return LIFT @ direction

    def rhs(self, x, reduced):
        return self.restrict(self.compact.rhs(x, self.lift(reduced)))

    def jac(self, x, reduced):
        full_jac = self.compact.jac(x, self.lift(reduced))
        return full_jac[KEEP] @ LIFT

    def full62(self, reduced):
        # Import belongs to the unchanged owner package selected by the caller.
        from compact_system import lift
        return lift(self.lift(reduced))
