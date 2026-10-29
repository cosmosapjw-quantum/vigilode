import math
import unittest
from rotation_diagnostic import endpoint_metrics

class EndpointContract(unittest.TestCase):
    def case(self):
        return {'model':'rotation','parameters':{'omega':2.},'t_span':[3.,3.+math.pi/4], 'initial_state':[2.,0.], 'rtol':1e-6,'atol':[1e-10,1e-10]}
    def test_shifted_nonunit_exact(self):
        r=endpoint_metrics(self.case(),[0.,2.])
        self.assertTrue(r['endpoint_accuracy_pass'])
        self.assertLess(abs(r['endpoint_phase_error_rad']),1e-15)
    def test_phase_failure_at_zero_crossing_preserves_budget(self):
        r=endpoint_metrics(self.case(),[1e-7,2.])
        self.assertFalse(r['endpoint_accuracy_pass'])
        self.assertGreater(r['endpoint_scaled_error'],999.)
    def test_invalid_values_refused(self):
        with self.assertRaises(ValueError): endpoint_metrics(self.case(),[float('nan'),2.])
        with self.assertRaises(ValueError): endpoint_metrics(self.case(),[2.])

if __name__=='__main__': unittest.main()
