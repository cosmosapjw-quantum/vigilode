import unittest

import numpy as np

from fisher_demo import build_problem, cg_solve, occupation_and_weight, sphere_rule


class FisherTests(unittest.TestCase):
    def test_harmonic_quadrature(self):
        for L in (0, 2, 5):
            _, _, w, Y, _ = sphere_rule(L)
            np.testing.assert_allclose(Y.T @ (w[:, None] * Y), np.eye((L + 1)**2), atol=2e-14)
            self.assertAlmostEqual(float(w.sum()), 4 * np.pi, places=13)

    def test_three_statistics_and_susceptibility(self):
        x = np.linspace(-6, -0.5, 31)
        step = 1e-5
        for s in ("MB", "FD", "BE"):
            _, w = occupation_and_weight(x, s)
            derivative = (occupation_and_weight(x + step, s)[0] - occupation_and_weight(x - step, s)[0]) / (2*step)
            np.testing.assert_allclose(w, derivative, rtol=2e-9)

    def test_invalid_be_and_nonfinite_rejected(self):
        for psi in (np.array([0.0]), np.array([0.1]), np.array([np.nan])):
            with self.assertRaises(ValueError):
                occupation_and_weight(psi, "BE")

    def test_centered_reference_identity(self):
        for narrow in (False, True):
            p = build_problem(3, 2, 0, "MB", narrow)
            np.testing.assert_allclose(p["B0"], np.eye(p["dimension"]), atol=2e-11)
            np.testing.assert_allclose(p["B"], p["B0"], atol=2e-13)

    def test_generalized_eigen_enclosure(self):
        for statistics in ("MB", "FD", "BE"):
            p = build_problem(3, 2, 0.6, statistics)
            ev = np.linalg.eigvalsh(p["B"])
            self.assertGreaterEqual(ev[0] + 1e-12, p["lower"])
            self.assertLessEqual(ev[-1] - 1e-12, p["upper"])
            self.assertLess(p["natural_family_max_residual"], 2e-12)

    def test_pcg_and_cg_physical_residual(self):
        p = build_problem(3, 8, 0.6)
        for blocks in (None, p["blocks"]):
            x, info = cg_solve(p["H"], p["rhs"], blocks)
            self.assertTrue(info["converged"])
            self.assertLessEqual(np.linalg.norm(p["H"] @ x - p["rhs"]) / np.linalg.norm(p["rhs"]), 1e-9)

    def test_zero_rhs(self):
        x, info = cg_solve(np.eye(2), np.zeros(2))
        np.testing.assert_array_equal(x, [0, 0])
        self.assertEqual(info["iterations"], 0)


if __name__ == "__main__":
    unittest.main(verbosity=2)
