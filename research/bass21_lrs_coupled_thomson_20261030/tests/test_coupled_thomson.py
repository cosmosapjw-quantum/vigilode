import pathlib,sys,unittest
import numpy as np
from numpy.polynomial.legendre import leggauss
from scipy.linalg import expm
ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'code'))
from coupled_thomson_volterra import (gl_half,solve_case,FrozenTransfer,sampled_simpson_weights,observed_moments)


class PolarizedThomsonTests(unittest.TestCase):
  @classmethod
  def setUpClass(cls):
    cls.n=193
    x=np.linspace(0,1.9459101490553132,cls.n);g=np.full(cls.n,.07)
    m=np.zeros((cls.n,9));m[:,0]=1;m[:,2]=.1
    cls.h=dict(x=x,b=np.zeros(cls.n),g=g,tau=.07*x,photon_energy_moments=m)

  def test_exact_thomson_tensor_quadrature(self):
    # Truly independent polarization coherency calculation in Cartesian frame.
    nodes,w=leggauss(48); phi=np.arange(96)*2*np.pi/96
    u=nodes[:,None]; cphi=np.cos(phi)[None,:]; sphi=np.sin(phi)[None,:]
    r=np.sqrt(1-u**2)
    n=np.stack([np.broadcast_to(r*cphi,(48,96)),np.broadcast_to(r*sphi,(48,96)),np.broadcast_to(u,(48,96))],axis=-1)
    et=np.stack([np.broadcast_to(u*cphi,(48,96)),np.broadcast_to(u*sphi,(48,96)),np.broadcast_to(-r,(48,96))],axis=-1)
    ef=np.stack([np.broadcast_to(-sphi,(48,96)),np.broadcast_to(cphi,(48,96)),np.zeros((48,96))],axis=-1)
    I0=1.4;J2=-.15;A=.08
    sourceI=I0+5*J2*.5*(3*u*u-1); sourceQ=A*(1-u*u)
    P=np.eye(3)-np.einsum('...i,...j->...ij',n,n)
    T=(sourceI[:,:,None,None]/2)*P+sourceQ[:,:,None,None]/2*(np.einsum('...i,...j->...ij',et,et)-np.einsum('...i,...j->...ij',ef,ef))
    M=np.einsum('u,uvij->ij',w*2*np.pi/96,T)
    for mu in (0,.35,.81,1):
      ct=np.sqrt(1-mu*mu);nt=np.array([ct,0,mu]);t=np.array([mu,0,-ct]);p=np.array([0,1,0]);project=np.eye(3)-np.outer(nt,nt)
      S=(3/(8*np.pi))*project@M@project
      I=np.trace(S); Q=t@S@t-p@S@p
      p2=.5*(3*mu*mu-1)
      self.assertAlmostEqual(I,I0+(.5*J2-.4*A)*p2,places=13)
      self.assertAlmostEqual(Q,(-.75*J2+.6*A)*(1-mu*mu),places=13)

  def test_gl_moments_normalization(self):
    mu,w=gl_half(24)
    dI=(3*mu*mu-1)*2.5
    Q=.25*(1-mu*mu)
    j0,j2,b=observed_moments(dI[None,:],Q[None,:],mu,w)
    self.assertAlmostEqual(j0[0],0,places=13)
    self.assertAlmostEqual(j2[0],1,places=13)
    self.assertAlmostEqual(b[0],4/15,places=13)

  def test_constant_rate_analytic_full_two_field_matrix(self):
    r=solve_case(self.h,nangular=24)
    M=np.array([[-.9,-.08],[-.75,-.4]],float)
    source=np.array([0,-.75*.1])
    for j in (48,96,144,192):
      tau=self.h['tau'][j]
      expected=np.linalg.solve(M,(expm(M*tau)-np.eye(2))@source)
      gotJ2=r['j2'][j]
      gotA=r['Q'][j,0]
      self.assertLess(abs(gotJ2-expected[0]),2e-10)
      self.assertLess(abs(gotA-expected[1]),2e-10)

  def test_initial_and_pole_exact_zero(self):
    r=solve_case(self.h)
    self.assertTrue(np.array_equal(r['Q'][0],np.zeros(17)))
    self.assertTrue(np.array_equal(r['Q'][:,-1],np.zeros(self.n)))

  def test_exact_first_gain_limit(self):
    r=solve_case(self.h)
    tau=self.h['tau'][-1];expected=-.75*.1*(1-np.exp(-tau))
    self.assertLess(abs(r['first_gain'][-1,0]-expected),1e-12)

  def test_angular_stability_and_convergence(self):
    r24=solve_case(self.h,24);r48=solve_case(self.h,48)
    self.assertLess(r24['fixed_res'],1e-12)
    self.assertLess(np.max(np.abs(r24['Q']-r48['Q'])),1e-12)
    self.assertLess(np.max(np.abs(r24['deltaI']-r48['deltaI'])),1e-12)

  def test_source_tamper_rejected(self):
    h=self.h.copy();h['tau']=self.h['tau'].copy();h['tau'][75]=-10
    with self.assertRaises(ValueError):solve_case(h)

  def test_bad_sample_grid_rejected(self):
    with self.assertRaises(ValueError):sampled_simpson_weights(np.array([0.,.1,.05]))

  def test_no_fake_polarization_at_zero_opacity(self):
    h=self.h.copy();h['g']=np.zeros(self.n);h['tau']=np.zeros(self.n)
    r=solve_case(h)
    self.assertEqual(float(np.max(np.abs(r['Q']))),0.)
    self.assertEqual(float(np.max(np.abs(r['deltaI']))),0.)

  def test_polarized_feedback_distinct_from_first_gain(self):
    r=solve_case(self.h)
    self.assertGreater(np.max(np.abs(r['Q']-r['first_gain'])),0)

if __name__=='__main__': unittest.main()
