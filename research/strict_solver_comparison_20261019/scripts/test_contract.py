import copy
import math
import unittest
from contract import assess

class ContractTests(unittest.TestCase):
    def setUp(self):
        self.times = [0.0, 1.0]
        self.ref = [[1.0, 0.0], [0.5, 0.5]]
        self.rho = [[0.0, 0.0], [0.0, 0.0]]
        self.row = dict(status='ok', times=self.times, states=copy.deepcopy(self.ref), rtol=1e-6, atol=1e-10)
    def evaluate(self, row=None, ref=None, rho=None):
        return assess(self.row if row is None else row, self.ref if ref is None else ref,
                      self.rho if rho is None else rho, self.times, 1e-4)
    def test_exact_full_row_admitted(self):
        got = self.evaluate()
        self.assertTrue(got['admitted'], got)
        self.assertEqual(got['E_up'], 0.0)
        self.assertEqual(got['R_up'], 0.0)
    def test_incomplete_success_never_zero_error(self):
        self.row['states'] = self.row['states'][:1]
        got = self.evaluate()
        self.assertFalse(got['admitted'])
        self.assertIsNone(got['E_up'])
    def test_failed_final_time_not_scored(self):
        self.row['status'] = 'solver_failure'
        self.row['times'] = [0.0, 0.1]
        got = self.evaluate()
        self.assertFalse(got['admitted'])
        self.assertIsNone(got['R_up'])
    def test_nan_rejected(self):
        self.row['states'][1][0] = math.nan
        self.assertFalse(self.evaluate()['admitted'])
    def test_wrong_dimension_rejected(self):
        self.row['states'][1].append(0.0)
        self.assertFalse(self.evaluate()['admitted'])
    def test_truncated_reference_rejected(self):
        self.assertFalse(self.evaluate(ref=self.ref[:1])['admitted'])
    def test_negative_radius_rejected(self):
        self.rho[1][0] = -1.0
        self.assertFalse(self.evaluate()['admitted'])
    def test_empirical_radius_counted(self):
        self.rho[1] = [1e-8, 1e-8]
        got = self.evaluate()
        self.assertTrue(got['admitted'])
        self.assertGreater(got['E_up'], 0.0)
    def test_reference_limited_is_separate(self):
        self.rho[1] = [0.1, 0.1]
        got = self.evaluate()
        self.assertTrue(got['admitted'])
        self.assertGreater(got['E_reference'], 0.1)
    def test_times_exactly_required(self):
        self.row['times'] = [0.0, 1.00001]
        self.assertFalse(self.evaluate()['admitted'])

if __name__ == '__main__':
    unittest.main(verbosity=2)
