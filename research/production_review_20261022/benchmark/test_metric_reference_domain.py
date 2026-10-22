"""Regression witnesses for the domain of depletion-weighted observable errors."""
import math
import sys
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / 'external_audits/vigilode_bassrei_ncp_c1_20261022/harness'))
import scenarios as S


class ReferenceDomain(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = S.case_data('S1-G1')
        cls.y0 = list(cls.data['y0'])
        cls.nph = cls.data['grid'].n

    def test_negative_reference_density_is_never_zero_error(self):
        bad = self.y0.copy()
        for i in range(self.nph):
            bad[i] = -bad[i]
        error, per = S.bapp_error('S1-G1', [0.0, 1.0], [self.y0, bad], [self.y0, bad])
        self.assertEqual(error, math.inf)
        self.assertEqual(per['reference_density_domain'], math.inf)

    def test_nonpositive_initial_reference_density_refused(self):
        vacuum = self.y0.copy()
        for i in range(self.nph):
            vacuum[i] = 0.0
        error, _ = S.bapp_error('S1-G1', [0.0, 1.0], [vacuum, vacuum], [vacuum, vacuum])
        self.assertEqual(error, math.inf)

    def test_positive_equal_trajectory_keeps_zero_error(self):
        error, _ = S.bapp_error('S1-G1', [0.0, 1.0], [self.y0, self.y0], [self.y0, self.y0])
        self.assertEqual(error, 0.0)


if __name__ == '__main__':
    unittest.main()
