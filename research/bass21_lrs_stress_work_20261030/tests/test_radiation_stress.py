import pathlib,sys,unittest
import numpy as np
root=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(root/'code'))
from radiation_stress_atlas import angular_moments,stress_from_j0_j2,intensity_direct_stress,restore_source
from coupled_thomson_volterra import gl_half
class StressTests(unittest.TestCase):
 def test_exact_two_moment_stress_trace(self):
  a=np.array([.25,-.1]);b=np.array([.03,.2])
  p,q,tr=stress_from_j0_j2(a,b)
  self.assertTrue(np.array_equal(tr,np.zeros(2)) or np.max(abs(tr))<1e-16)
  self.assertTrue(np.allclose(q-p,b,atol=1e-16,rtol=0))
 def test_direct_cartesian_stress_vs_multipole(self):
  mu,w=gl_half(64)
  for j0,j2 in [(1.,0.),(.2,-.03),(.1,.015)]:
   yy=j0+5*j2*(3*mu**2-1)/2
   J=angular_moments(yy[None,:],mu,w)[0]
   p,par,tr=stress_from_j0_j2(J[0],J[1]);rho,dp,da=intensity_direct_stress(yy[None,:],mu,w)
   self.assertLess(abs(float(p)-dp[0]),1e-14)
   self.assertLess(abs(float(par)-da[0]),1e-14)
   self.assertLess(abs(rho[0]-j0),1e-14)
 def test_moment_mixing_shear_identity(self):
  # Pure j0 alone must produce j2 shear coefficient -8/5, and pure j2 changes j0 by -2.
  from scipy.special import eval_legendre
  mu,w=gl_half(96)
  for l,expected0,expected2 in [(0,0,-8/5),(2,-2,-10/7),(4,0,36/35)]:
   I=(2*l+1)*eval_legendre(l,mu)
   # derivative identity: ∫P_l [3μ(1-μ²) I_mu +4(1-3μ²)I].
   coeff=np.zeros(l+1);coeff[l]=1
   from numpy.polynomial.legendre import Legendre
   p=Legendre(coeff);d=(2*l+1)*p.deriv()(mu)
   L=3*mu*(1-mu*mu)*d+4*(1-3*mu*mu)*I
   jj=angular_moments(L[None,:],mu,w)[0]
   self.assertAlmostEqual(jj[0],expected0,places=12)
   self.assertAlmostEqual(jj[1],expected2,places=12)
 def test_collision_monopole_exact(self):
  mu,w=gl_half(96)
  f= .13+5*.02*(3*mu*mu-1)/2
  B=.037
  S=.13+(.02/2-3*B/8)*(3*mu*mu-1)/2
  self.assertLess(abs(float((S-f)@w)),1e-14)
 def test_null_collision_zero_shear(self):
  # This isolates an analytic implication, not a claim that BASS itself has no shear.
  mu,w=gl_half(64)
  f=np.ones((3,len(mu)))*.011
  J=angular_moments(f,mu,w)
  self.assertLess(max(abs(J[:,1])),1e-14)
  self.assertLess(max(abs(J[:,2])),1e-14)
 def test_source_archive_fails_closed(self):
  import tempfile
  with tempfile.TemporaryDirectory() as d:
   p=pathlib.Path(d)/'not_the_source.zip';p.write_bytes(b'fake')
   with self.assertRaises(ValueError):restore_source(p,d)
if __name__=='__main__':unittest.main()
