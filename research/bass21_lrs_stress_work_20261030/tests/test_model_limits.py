import pathlib,sys,unittest,numpy as np
root=pathlib.Path(__file__).resolve().parents[1]
sys.path[:0]=[str(root/'code'),str(root/'source_code')]
from coupled_thomson_volterra import solve_case,gl_half
from radiation_stress_atlas import angular_moments
class Limits(unittest.TestCase):
 def setUp(self):
  x=np.linspace(0,1.3,73);M=np.zeros((73,9));M[:,0]=1.;M[:,2]=.05
  self.h=dict(x=x,b=np.zeros(73),g=np.full(73,.07),tau=.07*x,photon_energy_moments=M)
 def test_isotropic_geometry_collision_monopole(self):
  mu,w=gl_half(64);r=solve_case(self.h,nangular=32,mu_out=mu)
  J=angular_moments(r['deltaI'],mu,w)
  self.assertLess(float(max(abs(J[:,0]))),1e-13)
  self.assertGreater(float(max(abs(J[:,1]))),1e-7)
 def test_source_quadrupole_absent(self):
  h=self.h.copy();h['photon_energy_moments']=h['photon_energy_moments'].copy();h['photon_energy_moments'][:,2]=0
  r=solve_case(h,nangular=24)
  self.assertEqual(float(np.max(abs(r['deltaI']))),0.)
  self.assertEqual(float(np.max(abs(r['Q']))),0.)
 def test_thomson_off(self):
  h=self.h.copy();h['g']=np.zeros(73);h['tau']=np.zeros(73)
  mu,w=gl_half(32);r=solve_case(h,nangular=24,mu_out=mu)
  J=angular_moments(r['deltaI'],mu,w)
  self.assertEqual(float(np.max(abs(J))),0.)
if __name__=='__main__':unittest.main()
