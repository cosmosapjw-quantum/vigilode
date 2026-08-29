import importlib.util
from pathlib import Path
import unittest
p=Path(__file__).with_name('audit2_output_policy_research.py')
spec=importlib.util.spec_from_file_location('audit2',p);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class ContractTests(unittest.TestCase):
 def test_accurate_but_sensitive(self):
  self.assertGreater(.001,.1*.002);self.assertEqual(m.interval(.002,0.,.01)['verdict'],'within-budget')
 def test_equal_bad_is_not_accurate(self):
  self.assertLessEqual(0.,.1*10.);self.assertEqual(m.interval(10.,0.,.01)['verdict'],'outside-budget')
 def test_missing_budget(self):self.assertEqual(m.interval(0.,0.)['verdict'],'budget-not-specified')
 def test_reference_uncertainty(self):self.assertEqual(m.interval(.9,.2,1.)['verdict'],'reference-unresolved')
 def test_invalid_values(self):
  for v in (float('nan'),float('inf'),-1.,True):
   with self.assertRaises(ValueError):m.interval(v,0.,1.)
   with self.assertRaises(ValueError):m.interval(1.,v,1.)
   with self.assertRaises(ValueError):m.interval(1.,0.,v)
 def test_zero(self):self.assertEqual(m.interval(-0.,0.,0.)['verdict'],'within-budget')
 def test_scalar_errors_are_not_distance(self):self.assertEqual(abs(abs(1.)-abs(-1.)),0.);self.assertEqual(abs(1.-(-1.)),2.)
 def test_manufactured_order_and_negative_control(self):
  solver=m.ScalarRodas(m.load(m.ROOT/'fixtures/rodas5p_coefficients_snapshot.json'));mp=m.mp;errors=[]
  for n in (4,8,16):
   h=mp.mpf(1)/n;t=mp.mpf('.3');y=mp.exp(-t);theta=mp.mpf('.37');yn,k=solver.step(t,y,h,'manufactured_nonlinear');exact=mp.exp(-(t+theta*h))
   errors.append([abs(yn-mp.exp(-(t+h))),abs(solver.sample(y,yn,k,theta)-exact),abs((1-theta)*y+theta*yn-exact)])
  for a,b in zip(errors,errors[1:]):
   for x,y,target in zip(a,b,[6,5,2]):self.assertLess(abs(mp.log(x/y,2)-target),mp.mpf('.3'))
if __name__=='__main__':unittest.main(verbosity=2)
