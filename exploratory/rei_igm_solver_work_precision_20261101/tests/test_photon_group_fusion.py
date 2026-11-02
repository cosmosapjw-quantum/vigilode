"""Original source-only 3-moment group-fused RHS/JVP tests."""
from __future__ import annotations
import sys
import unittest
from pathlib import Path
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import original26_photon_group_fusion as fused
import original26_photon_lowrank_check as baseline


class PhotonGroupFusionTests(unittest.TestCase):
    def setup(self):
        rng = np.random.default_rng(20261101)
        n = rng.uniform(.1, 4., size=(51, 3))
        v = rng.normal(0, .3, size=(51, 3))
        j = np.array([.3, .7, 1.1, 2.7])
        ext = np.array([.02, .09, .11, .14])
        alpha = np.asarray([[.1, .3, .2], [.2, .4, .01], [.1, .05, .3], [.02, .3, .1]], dtype=np.longdouble)
        excess = np.asarray([[1.0, 2., 3., 4.], [2., 1., 5., 2.], [6., 3., 1., 2.]])
        return n, v, j, ext, alpha, excess

    def test_source_algebra_and_ledger(self):
        n, v, j, ext, a, exc = self.setup()
        old_rate, old_heat, old_external = baseline.hybrid_source(n, j, ext, a, exc, 1.0)
        c = fused.coefficients(n, j, ext, a, exc, 1.0)
        rate, heat, external = fused.fused_photon_fields(n, c)
        np.testing.assert_allclose(rate, old_rate, rtol=2e-15, atol=1e-14)
        np.testing.assert_allclose(heat, old_heat, rtol=2e-15, atol=1e-14)
        self.assertAlmostEqual(external, old_external, places=13)
        self.assertLess(abs(rate.sum() + external - j.sum()), 1e-12)
        self.assertTrue((rate >= 0).all())
        self.assertTrue((heat >= 0).all())

    def test_rank_three_jvp_matches_finite_difference(self):
        n, v, j, ext, a, exc = self.setup()
        c = fused.coefficients(n, j, ext, a, exc, 1.)
        d = fused.fused_photon_jvp(n, v, c)
        eps = 5e-6
        plus = fused.fused_photon_fields(n + eps*v, fused.coefficients(n + eps*v, j, ext, a, exc, 1.))[0]
        minus = fused.fused_photon_fields(n - eps*v, fused.coefficients(n - eps*v, j, ext, a, exc, 1.))[0]
        fd = (plus - minus) / (2*eps)
        self.assertLess(float(np.max(np.abs(d-fd))) / max(float(np.max(np.abs(d))), 1e-300), 1e-8)
        np.testing.assert_allclose(d, baseline.hybrid_lowrank_jvp(n, v, j, ext, a), rtol=1e-11, atol=1e-12)

    def test_stage_frozen_cache_snapshots_state_and_rejects_nan(self):
        n, v, j, ext, a, exc = self.setup()
        before = n.copy()
        op = fused.FrozenPhotonJacobian(n, j, ext, a, exc, 1.)
        expected = baseline.hybrid_lowrank_jvp(before, v, j, ext, a)
        np.testing.assert_allclose(op.apply(v), expected, rtol=1e-12, atol=1e-12)
        n[:] = 9.  # Caller state mutation must NOT stale-update cached Jacobian.
        np.testing.assert_allclose(op.apply(v), expected, rtol=1e-12, atol=1e-12)
        with self.assertRaises(ValueError):
            op.apply(np.full_like(v, np.nan))
        with self.assertRaises(ValueError):
            op.apply(v[:, :2])

    def test_nonlocal_matrix_symmetric_positive_semidefinite(self):
        n, v, j, ext, a, exc = self.setup()
        c = fused.coefficients(n, j, ext, a, exc, 1.)
        np.testing.assert_allclose(c[2], c[2].T, atol=1e-18)
        self.assertGreaterEqual(float(np.min(np.linalg.eigvalsh(np.asarray(c[2], dtype=float)))), -1e-15)

    def test_invalid_denominator_and_input_fail_closed(self):
        n, v, j, ext, a, exc = self.setup()
        with self.assertRaises(ArithmeticError):
            fused.coefficients(n, np.zeros(4), np.zeros(4), np.zeros((4, 3)), exc, 1.)
        with self.assertRaises(ValueError):
            fused.coefficients(-n, j, ext, a, exc, 1.)
        with self.assertRaises(ValueError):
            fused.coefficients(n[:, :2], j, ext, a, exc, 1.)

    def test_missing_owner_does_not_fabricate_physics(self):
        with self.assertRaises(FileNotFoundError):
            fused.original_source_inputs(Path("/absent/rei-owner"), "LOCAL_NEUTRAL_HAZARD_PRIMARY")


if __name__ == "__main__":
    unittest.main()
