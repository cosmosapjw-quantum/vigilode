"""Unit tests of the independent P05 references (`p05_reference.py`).

    python3 -m unittest discover -s reference -p 'test_*.py'
"""
from __future__ import annotations

import math
import sys
import unittest
from fractions import Fraction
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))

import numpy as np  # noqa: E402

import p05_reference as F  # noqa: E402

R = F.R
EPS = float(np.finfo(float).eps)
GEOM = R.Geometry("power_law", hh=0.5, s0=0.2, p=1.5)
ABSORBER = R.Absorber(0.5, -0.5)


def exact_remap(src_edges, src, dst_edges):
    """Exact rational piecewise-constant overlap remap of the float data."""
    a = [Fraction(x) for x in src_edges]
    b = [Fraction(x) for x in dst_edges]
    q = [Fraction(v) for v in src]
    out = [Fraction(0)] * (len(b) - 1)
    for i in range(len(a) - 1):
        for j in range(len(b) - 1):
            ov = min(a[i + 1], b[j + 1]) - max(a[i], b[j])
            if ov > 0:
                out[j] += q[i] * ov / (a[i + 1] - a[i])
    return out


class AlignedMesh(unittest.TestCase):
    LO, HI, N = -3.0, 2.0, 32

    def test_face_at_x_and_width_bounds(self):
        D = (self.HI - self.LO) / self.N
        for x in np.linspace(self.LO + 0.26 * D, self.HI - 0.26 * D, 997):
            x = float(x)
            e, k = F.aligned_edges(self.LO, self.HI, self.N, x)
            self.assertIsNotNone(k)
            self.assertEqual(e[k], x)
            self.assertEqual(len(e), self.N + 1)
            w = np.diff(e)
            self.assertTrue(np.all(w > 0))
            if x - self.LO >= 0.5 * D and self.HI - x >= 0.5 * D:
                self.assertTrue(np.all(w >= 0.5 * D * (1 - 4 * EPS)) and np.all(w <= 1.5 * D * (1 + 4 * EPS)))
            else:
                self.assertTrue(np.all(w > 0.25 * D) and np.all(w < 1.75 * D))
            self.assertEqual(e[0], self.LO)
            self.assertEqual(e[-1], self.HI)

    def test_nearest_edge_ties_to_lower(self):
        e1 = F.base_edge(self.LO, self.HI, self.N, 10)
        e2 = F.base_edge(self.LO, self.HI, self.N, 11)
        mid = 0.5 * (e1 + e2)
        k = F.aligned_index(self.LO, self.HI, self.N, mid)
        if abs(mid - e1) == abs(mid - e2):
            self.assertEqual(k, 10)
        else:
            self.assertEqual(k, 10 if abs(mid - e1) < abs(mid - e2) else 11)

    def test_end_zones_and_nan_give_the_base_grid(self):
        D = (self.HI - self.LO) / self.N
        base = F.base_edges(self.LO, self.HI, self.N)
        for x in (self.LO + 0.25 * D, self.HI - 0.25 * D, self.LO - 1.0, self.HI + 1.0, float("nan")):
            e, k = F.aligned_edges(self.LO, self.HI, self.N, x)
            self.assertIsNone(k)
            self.assertEqual(e, base)

    def test_refine_halves_the_two_adjacent_cells(self):
        x = -0.43
        e0, k0 = F.aligned_edges(self.LO, self.HI, self.N, x)
        e, k = F.aligned_edges(self.LO, self.HI, self.N, x, refine=True)
        self.assertEqual(len(e), self.N + 3)
        self.assertEqual(k, k0 + 1)
        self.assertEqual(e[k], x)
        self.assertEqual(e[k - 1], 0.5 * (e0[k0 - 1] + x))
        self.assertEqual(e[k + 1], 0.5 * (x + e0[k0 + 1]))

    def test_base_edges_are_the_p02_uniform_grid(self):
        g = R.uniform_grid(self.LO, self.HI, self.N, 4, "comoving", "outflow")
        base = F.base_edges(self.LO, self.HI, self.N)
        self.assertEqual(len(base), len(g.q_edges))
        # linspace and lo + (hi - lo) k / n may differ by an ulp; the runner's rule is the latter
        self.assertTrue(all(abs(a - b) <= 4 * EPS for a, b in zip(base, g.q_edges)))


class OverlapRemap(unittest.TestCase):
    def test_hand_cases(self):
        self.assertEqual(F.remap_nonnegative([0.0, 1.0, 3.0], [2.0, 6.0], [0.0, 0.25, 1.0, 2.0, 3.0]),
                         [0.5, 1.5, 3.0, 3.0])
        self.assertEqual(F.remap_nonnegative([0.0, 1.0, 3.0], [2.0, 6.0], [0.0, 2.0, 3.0]), [5.0, 3.0])
        self.assertEqual(F.remap_nonnegative([0.0, 0.25, 1.0, 2.0, 3.0], [0.5, 1.5, 3.0, 3.0], [0.0, 1.0, 3.0]),
                         [2.0, 6.0])

    def test_against_exact_rationals_on_random_meshes(self):
        rng = np.random.default_rng(5)
        worst = 0.0
        for _ in range(40):
            a = np.sort(np.concatenate([[-3.0, 2.0], rng.uniform(-3, 2, 17)])).tolist()
            b = np.sort(np.concatenate([[-3.0, 2.0], rng.uniform(-3, 2, 13)])).tolist()
            q = rng.lognormal(0, 2, len(a) - 1).tolist()
            got = F.remap_nonnegative(a, q, b)
            ex = exact_remap(a, q, b)
            for g, e in zip(got, ex):
                if e != 0:
                    worst = max(worst, float(abs(Fraction(g) - e) / e))
            self.assertLessEqual(abs(Fraction(math.fsum(got)) - sum(ex)), 8 * EPS * sum(ex))
        self.assertLessEqual(worst, 4 * EPS)

    def test_identity_is_bitwise(self):
        a = F.aligned_edges(-3.0, 2.0, 32, -0.41)[0]
        q = np.random.default_rng(1).lognormal(0, 3, 32).tolist()
        self.assertEqual(F.remap_nonnegative(a, q, a), q)

    def test_constant_density_is_reproduced(self):
        a = F.base_edges(-3.0, 2.0, 32)
        b = F.aligned_edges(-3.0, 2.0, 32, -0.41)[0]
        rho = 3.7
        q = [rho * (y - x) for x, y in zip(a, a[1:])]
        got = F.remap_nonnegative(a, q, b)
        for g, x, y in zip(got, b, b[1:]):
            self.assertLessEqual(abs(g - rho * (y - x)), 8 * EPS * rho * (y - x))

    def test_signed_and_state_layout(self):
        rng = np.random.default_rng(3)
        nm, nq = 4, 32
        a = F.base_edges(-3.0, 2.0, nq)
        b = F.aligned_edges(-3.0, 2.0, nq, 0.07)[0]
        y = rng.standard_normal(2 + nq * (nm + 1)).tolist()
        out = F.remap_state(a, y, b, nm)
        self.assertEqual(out[0], y[0])
        self.assertEqual(out[-1], y[-1])
        stride = nm + 1
        for col in range(stride):
            s0 = math.fsum(y[1 + i * stride + col] for i in range(nq))
            s1 = math.fsum(out[1 + i * stride + col] for i in range(nq))
            mass = math.fsum(abs(y[1 + i * stride + col]) for i in range(nq))
            self.assertLessEqual(abs(s1 - s0), 8 * EPS * mass)
        with self.assertRaises(ValueError):
            F.remap_nonnegative(a, [-1.0] + [0.0] * (nq - 1), b)
        with self.assertRaises(ValueError):
            F.remap_nonnegative(a, [0.0] * nq, [-3.0, 1.9])


class NonuniformModel(unittest.TestCase):
    """The P02 operator on an aligned (nonuniform) comoving mesh."""

    @classmethod
    def setUpClass(cls):
        cls.oracle = R.load_oracle()
        cls.edges = F.aligned_edges(-3.0, 2.0, 16, -0.43)[0]
        cls.mu = np.linspace(-1.0, 1.0, 5).tolist()
        cls.grid = R.Grid(cls.edges, cls.mu, "comoving", "outflow")
        cls.model = R.Model(cls.oracle, cls.grid, GEOM, 0.5, ABSORBER, (0.2, 0.4))

    def test_columns_conserve(self):
        A = self.model.A(0.3).toarray()
        scale = np.max(np.abs(A))
        self.assertLessEqual(float(np.max(np.abs(A.sum(axis=0)))), 16 * EPS * scale)

    def test_kbar_closed_matches_mpmath_on_the_nonuniform_mesh(self):
        for t in (0.2, 0.3, 0.37):
            k, dk = R.kbar_closed(self.grid, ABSORBER, GEOM, t)
            km, dkm, dev, _ = R.kbar_mpmath(self.grid, ABSORBER, GEOM, t)
            km = np.array([float(v) for v in km])
            dkm = np.array([float(v) for v in dkm])
            self.assertLessEqual(float(np.max(np.abs(k - km))), 1e-14 * float(np.max(np.abs(km))))
            self.assertLessEqual(float(np.max(np.abs(dk - dkm))), 1e-14 * float(np.max(np.abs(dkm))))

    def test_transport_matches_a_direct_upwind_sum(self):
        g = self.grid
        rng = np.random.default_rng(9)
        y = np.zeros(g.n)
        y[g.cells] = rng.lognormal(0, 1, g.cells.shape)
        t = 0.3
        s = float(GEOM.S(t))
        got = self.model.transport_matrix(t) @ y
        n = y[g.cells]
        rho = n / (g.dq[:, None] * g.dm[None, :])
        rhs = np.zeros_like(n)
        lo = hi = 0.0
        for j in range(g.nm):
            a, b = g.me[j], g.me[j + 1]
            v = -s * (3 * (b**3 - a**3) / (3 * (b - a)) - 1)  # comoving: no H in the energy drift
            for k in range(g.nq + 1):
                up = k - 1 if v >= 0 else k
                f = v * (rho[up, j] if 0 <= up < g.nq else 0.0) * g.dm[j]
                if k > 0:
                    rhs[k - 1, j] -= f
                else:
                    lo -= f
                if k < g.nq:
                    rhs[k, j] += f
                else:
                    hi += f
        for i in range(g.nq):
            for k in range(1, g.nm):
                m = g.me[k]
                v = -3 * s * m * (1 - m * m)
                up = k - 1 if v >= 0 else k
                f = v * rho[i, up] * g.dq[i]
                rhs[i, k - 1] -= f
                rhs[i, k] += f
        ref = np.zeros(g.n)
        ref[g.cells] = rhs
        ref[g.lo], ref[g.hi] = lo, hi
        scale = float(np.max(np.abs(ref)))
        self.assertLessEqual(float(np.max(np.abs(got - ref))), 1e-14 * scale)


class Segmented(unittest.TestCase):
    def test_no_remap_equals_a_plain_run_and_conserves(self):
        oracle = R.load_oracle()
        mu = np.linspace(-1.0, 1.0, 5).tolist()
        g = R.Grid(F.base_edges(-3.0, 2.0, 16), mu, "comoving", "outflow")
        y0, tot = R.initial_state(g)
        st, meshes, _ = F.segmented_reference(oracle, GEOM, 0.5, ABSORBER, (-3.0, 2.0, 16), mu, y0, 1.0, [],
                                              [0.0, 0.5, 1.0], "Radau", 1e-10, 1e-10 * 1e-6 * R.C_AMP)
        model = R.Model(oracle, g, GEOM, 0.5, ABSORBER, (0.0, 1.0))
        plain, _ = R.ref_ivp(model, y0, [0.0, 0.5, 1.0], "Radau", 1e-10, F.atol_rule(g, 1e-10 * 1e-6 * R.C_AMP))
        self.assertEqual(len(st), 3)
        for a, b in zip(st, plain):
            self.assertTrue(np.array_equal(a, b))
        self.assertLessEqual(abs(math.fsum(st[-1]) - tot) / tot, 1e-13)

    def test_remapped_run_conserves_and_records_the_meshes(self):
        oracle = R.load_oracle()
        mu = np.linspace(-1.0, 1.0, 5).tolist()
        g = R.Grid(F.base_edges(-3.0, 2.0, 16), mu, "comoving", "outflow")
        y0, tot = R.initial_state(g)
        st, meshes, info = F.segmented_reference(oracle, GEOM, 0.5, ABSORBER, (-3.0, 2.0, 16), mu, y0, 1.0,
                                                 [0.3, 0.6], [0.0, 0.3, 0.8, 1.0], "Radau", 1e-10,
                                                 1e-10 * 1e-6 * R.C_AMP)
        self.assertEqual(len(st), 4)
        self.assertEqual(meshes[1], meshes[0])  # the output at a remap time is on the old mesh
        self.assertNotEqual(meshes[2], meshes[1])
        x = F.x_chi(GEOM, ABSORBER, 0.6)
        self.assertIn(x, meshes[2])
        for s in st:
            self.assertLessEqual(abs(math.fsum(s) - tot) / tot, 1e-13)
        self.assertEqual(len(info["segments"]), 3)


if __name__ == "__main__":
    unittest.main()
