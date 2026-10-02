"""Nonfinite and overflow guards of the optional chart, not solver tests."""
import unittest
from thread_transfer_chart import SemilinearChart, ChartDomainError

class ChartDomainTests(unittest.TestCase):
    def test_parameter_finiteness(self):
        for idx in (0, 2, 3):
            a=[40.0,1,1e-6,1e6]; a[idx]=float('inf')
            with self.assertRaises(ValueError): SemilinearChart(*a)
    def test_nonfinite_and_overflow_coordinates(self):
        for sign in [-1,1]:
            c=SemilinearChart(40.0,sign,1e-6,1e6)
            for x in [float('nan'),sign*float('inf'),sign*1e300]:
                with self.assertRaises(ChartDomainError): c.to_chart(x,1.0)
            for bad in [float('inf'),float('nan')]:
                with self.assertRaises(ChartDomainError): c.to_chart(sign*1.0,bad)
                with self.assertRaises(ChartDomainError): c.from_chart(sign*1.0,bad)
    def test_finite_round_trip(self):
        c=SemilinearChart(2.0,-1,0.25,4.0)
        self.assertEqual(c.from_chart(-1.0,c.to_chart(-1.0,0.75)),0.75)

if __name__=='__main__': unittest.main()
