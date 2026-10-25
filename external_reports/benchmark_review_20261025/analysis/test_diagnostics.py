import math
import unittest
from fractions import Fraction
from diagnostics import endpoint_phase, error_interval, reuse_threshold, end_to_end_speedup


class DiagnosticsTests(unittest.TestCase):
    def test_exact_pythagorean_phase_decomposition(self):
        for r in (0.01, 1, 100):
            for R in (0.03, 1, 20):
                for delta in (-3, -.1, 0, .2, 3):
                    d = endpoint_phase(r*complex(math.cos(delta), math.sin(delta)), R)
                    self.assertAlmostEqual((d['radial_squared']+d['angular_squared'])/(1+d['absolute_error']**2),
                                           d['absolute_error']**2/(1+d['absolute_error']**2), places=13)

    def test_phase_does_not_invent_cycles(self):
        d = endpoint_phase(complex(math.cos(8*math.pi+.1), math.sin(8*math.pi+.1)), 1)
        self.assertAlmostEqual(d['principal_phase_error'], .1, places=13)

    def test_zero_radius_and_nonfinite(self):
        self.assertFalse(endpoint_phase(0, 1)['phase_defined'])
        with self.assertRaises(ValueError): endpoint_phase(complex(float('nan'), 0), 1)

    def test_reference_enclosure(self):
        lower, upper = error_interval(.1, .01)
        self.assertLessEqual(Fraction(lower), Fraction(.1)-Fraction(.01))
        self.assertGreaterEqual(Fraction(upper), Fraction(.1)+Fraction(.01))
        self.assertEqual(error_interval(.001, .01)[0], 0)
        with self.assertRaises(ValueError): error_interval(-1, 0)

    def test_strict_amortization(self):
        self.assertEqual(reuse_threshold(10, 3, 5), 6)
        self.assertIsNone(reuse_threshold(10, 5, 5))
        self.assertIsNone(reuse_threshold(10, 6, 5))
        self.assertEqual(reuse_threshold(0, 3, 5), 1)

    def test_component_and_total(self):
        self.assertEqual(end_to_end_speedup(0, 100), 1)
        self.assertAlmostEqual(end_to_end_speedup(.5, 100), 1/.505)
        self.assertEqual(end_to_end_speedup(1, 10), 10)

    def test_dynamic_range_contract(self):
        with self.assertRaises(ValueError): endpoint_phase(1e200, 1e200)
        with self.assertRaises(ValueError): endpoint_phase(complex(1.7e308, 1.7e308), 1)
        with self.assertRaises(ValueError): reuse_threshold(1e308, 0, 1e-308)
        self.assertEqual(end_to_end_speedup(1, 1e-320), 1e-320)


if __name__ == '__main__': unittest.main()
