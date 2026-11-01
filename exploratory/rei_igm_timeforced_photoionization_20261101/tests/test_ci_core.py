import json,sys,unittest
from pathlib import Path
import numpy as np
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT))
from source_time_ivp import SourceIVP,ND
class NativeReady(unittest.TestCase):
 def setUp(self):self.model=SourceIVP()
 def test_source_shape(self):
  self.assertEqual(self.model.W.shape,(5,4,4));self.assertEqual(self.model.J.shape,(5,4))
 def test_source_five_knot_exact(self):
  for k,u in enumerate(self.model.times):
   J,W=self.model.forcing(u)
   np.testing.assert_allclose(J,self.model.J[k],atol=0,rtol=2e-14)
   np.testing.assert_allclose(W,self.model.W[k],atol=1e-323,rtol=2e-14)
 def test_all_group_photon_conservation(self):
  for u in (.0,.13,.2,.4,.67,.98,1.):
   _,p,e,_=self.model.rates(u,[.99,.999,1.00001])
   np.testing.assert_allclose(p.sum(axis=1)+e,1,atol=4e-14,rtol=0)
 def test_nuclei_photon_binding_rhs_identity(self):
  y=self.model.initial();dy=self.model.rhs(.21,y)
  np.testing.assert_allclose(self.model.invariants(y+0.03*dy),0,atol=2e-15)
 def test_jvp_source_two_knots(self):
  y=self.model.initial();v=np.linspace(-.3,.4,ND)
  for u in (.1,.2,.37,.8):
   e=1e-5;d=(self.model.rhs(u,y+e*v)-self.model.rhs(u,y-e*v))/(2*e)
   np.testing.assert_allclose(d,self.model.jvp(u,y,v),rtol=3e-7,atol=4e-10)
 def test_original_reference_ionization(self):
  a=self.model.solve();b=SourceIVP(frozen=True).solve()
  self.assertGreater(abs(a.y[4,-1]-b.y[4,-1]),2e-8)
  self.assertLess(abs(a.y[4,-1]-b.y[4,-1]),3e-8)
 def test_original_source_status(self):
  f=self.model.f
  self.assertEqual(f['canonical_REI_delta'],0)
  self.assertEqual(f['N'],46080)
  self.assertIn('piecewise-linear',f['model'])
 def test_fail_if_bad_time(self):
  with self.assertRaises(ValueError):self.model.forcing(-0.5)
 def test_photo_absorption_positive(self):
  r=self.model.solve();self.assertTrue(np.all(r.y[:3,:]>0));self.assertTrue(np.all(r.y[3:14,:]>=-1e-13))
 def test_binding_chemical_not_thermal(self):
  r=self.model.solve();self.assertGreater(r.y[14,-1],0)
if __name__=='__main__':unittest.main()
