"""Independent original-system tests for the exact K3 research oracle."""

from fractions import Fraction as F
import unittest

from directional_quotient import QuotientRefused, directional_quotient, matvec


def exact_solve(matrix, rhs):
    # Independent rational Gaussian elimination, unrelated to the quotient.
    a = [[F(v) for v in row] + [F(b)] for row, b in zip(matrix, rhs)]
    n = len(rhs)
    for j in range(n):
        pivot = next(i for i in range(j, n) if a[i][j])
        a[j], a[pivot] = a[pivot], a[j]
        divisor = a[j][j]
        a[j] = [v / divisor for v in a[j]]
        for i in range(n):
            if i != j:
                factor = a[i][j]
                a[i] = [v - factor * w for v, w in zip(a[i], a[j])]
    return tuple(row[-1] for row in a)


class DirectionalQuotientContracts(unittest.TestCase):
    def test_directional_relation_is_weaker_than_full_space_relation(self):
        j = ((-1, 2, 0), (0, -2, 0), (0, 0, -9))
        for sigma in (F(0), F(1, 16), F(1, 2), F(2)):
            result = directional_quotient(j, (1, 1, 0), (2, 3, 1), sigma, 0)
            self.assertEqual(result.original_residual, (0, 0, 0))
            self.assertTrue(result.admitted)
            # The same polynomial on the third coordinate is nonzero.
            third = directional_quotient(j, (0, 0, 1), (2, 3, 1), sigma)
            self.assertEqual(third.relation_defect, (0, 0, 56))

    def test_near_relation_original_residual_and_coercivity_bound(self):
        j = ((-1, 2, 0), (0, -2, 0), (0, 0, -9))
        rhs = (F(1), F(1), F(1, 1000))
        expected = (F(0), F(7, 38250), F(7, 1500), F(28, 1875))
        for sigma, residual in zip((F(0), F(1, 16), F(1, 2), F(2)), expected):
            result = directional_quotient(j, rhs, (2, 3, 1), sigma)
            self.assertEqual(result.original_residual[2], residual)
            a = tuple(tuple(F(i == k) - sigma * j[i][k] for k in range(3))
                      for i in range(3))
            exact = exact_solve(a, rhs)
            actual_error_sq = sum((x - y) ** 2 for x, y in zip(result.candidate, exact))
            self.assertLessEqual(actual_error_sq, result.squared_error_bound)

    def test_arbitrary_polynomial_does_not_gain_false_exactness(self):
        j = ((-4, 1), (-2, -5))
        for degree in range(1, 6):
            p = tuple(F(k - 2) for k in range(degree)) + (F(1),)
            for sigma in (F(0), F(1, 16), F(1, 8)):
                result = directional_quotient(j, (2, -3), p, sigma)
                self.assertEqual(result.original_residual, result.predicted_residual)
                a = tuple(tuple(F(i == k) - sigma * j[i][k] for k in range(2))
                          for i in range(2))
                exact = exact_solve(a, (2, -3))
                err = sum((x - y) ** 2 for x, y in zip(result.candidate, exact))
                self.assertLessEqual(err, result.squared_error_bound)
                self.assertEqual(result.matrix_vector_actions, degree + 1)
                self.assertIsNone(result.admitted)

    def test_sigma_zero_is_identity_even_without_annihilation(self):
        result = directional_quotient(((-3, 2), (0, -1)), (2, 7), (9, 1), 0, 0)
        self.assertEqual(result.candidate, (2, 7))
        self.assertEqual(result.denominator, 1)
        self.assertEqual(result.squared_error_bound, 0)

    def test_representation_pole_is_refused_for_nonsingular_system(self):
        # I - J = 2I is nonsingular, but chosen p(z)=z-1 gives D(1)=0.
        with self.assertRaisesRegex(QuotientRefused, "denominator"):
            directional_quotient(((-1,),), (1,), (-1, 1), 1)

    def test_nonnormal_eigenvalues_do_not_supply_coercivity(self):
        with self.assertRaisesRegex(QuotientRefused, "coercivity"):
            directional_quotient(((-1, 100), (0, -1)), (1, 1), (1, 2, 1), 1)

    def test_cap_failure_preserves_candidate_and_negative_evidence(self):
        result = directional_quotient(((-3,),), (1,), (1, 1), F(1, 2), F(1, 100))
        self.assertFalse(result.admitted)
        self.assertNotEqual(result.original_residual, (0,))
        self.assertGreater(result.squared_error_bound, F(1, 100) ** 2)

    def test_sign_mutation_is_detected_by_original_equation(self):
        j = ((-1, 2), (0, -2))
        rhs, sigma = (F(1), F(1)), F(1, 2)
        good = directional_quotient(j, rhs, (2, 3, 1), sigma, 0)
        wrong = tuple(-v for v in good.candidate)
        jw = matvec(j, wrong)
        residual = tuple(rhs[i] - wrong[i] + sigma * jw[i] for i in range(2))
        self.assertNotEqual(residual, good.original_residual)

    def test_float_and_malformed_inputs_are_refused(self):
        with self.assertRaises(TypeError):
            directional_quotient(((-1.0,),), (1,), (1, 1), 1)
        with self.assertRaises(ValueError):
            directional_quotient(((-1,),), (1,), (1, 2), 1)
        with self.assertRaises(ValueError):
            directional_quotient(((-1,),), (1,), (1, 1), -1)


if __name__ == "__main__":
    unittest.main()
