"""Exact and source-data tests for BASS21 frame-free Thomson rank-nine reduction."""
import pathlib,sys,unittest,math,json,tempfile
import numpy as np
import sympy as s
from numpy.polynomial.legendre import leggauss
ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'code'))
import rank9_coherency as r

class Rank9Tests(unittest.TestCase):
    def test_exact_tensor_reduction(self):
        j0,j2,bq,mu=s.symbols('j0 j2 bq mu',real=True)
        p=4*s.pi*(j0/s.Integer(3)+j2/s.Integer(6)-bq/s.Integer(8))
        z=4*s.pi*((j0-j2)/s.Integer(3)+bq/s.Integer(4))
        self.assertEqual(s.simplify(2*p+z-4*s.pi*j0),0)
        SI=s.Rational(3,8)/s.pi*(p*(1+mu**2)+z*(1-mu**2))
        SQ=s.Rational(3,8)/s.pi*(z-p)*(1-mu**2)
        self.assertEqual(s.simplify(SI-(j0+(j2/s.Integer(2)-3*bq/s.Integer(8))*(3*mu**2-1)/2)),0)
        self.assertEqual(s.simplify(SQ-(-3*j2/s.Integer(4)+9*bq/s.Integer(16))*(1-mu**2)),0)
        self.assertEqual(s.simplify(2*s.integrate(SQ*(1-mu**2),(mu,0,1))-(-4*j2/s.Integer(5)+3*bq/s.Integer(5))),0)
    def test_collision_matrix(self):
        A=s.Matrix([[-s.Rational(9,10),-s.Rational(3,40)],[-s.Rational(4,5),-s.Rational(2,5)]])
        self.assertEqual(set(A.eigenvals()),{-1,-s.Rational(3,10)})
    def test_harmonic_mapping_exact(self):
        l=2
        g=s.sqrt(4*s.pi/(2*l+1))*s.sqrt(s.factorial(l+2)/s.factorial(l-2))
        ec=s.Rational(12,5)/s.sqrt(6)
        self.assertEqual(s.simplify(g/ec-s.sqrt(20*s.pi)),0)
    def test_zero_and_quadrupole_manufactured(self):
        mu,w=r.glhalf(64);I=np.ones((1,len(mu)));Q=np.zeros_like(I)
        j0,j2,bq=r.moments(I,Q,mu,w);p,z=r.direct_tensor(I,Q,mu,w)
        self.assertAlmostEqual(float(j0[0]),1.,places=13)
        self.assertAlmostEqual(float(j2[0]),0.,places=13)
        self.assertAlmostEqual(float(p[0]),float(z[0]),places=13)
        si,sq=r.gains_from_tensor(p,z)
        self.assertLess(np.max(np.abs(sq)),1e-13)
        self.assertAlmostEqual(float(si[0,0]),1.,places=13)
        # nonzero pure intensity quadrupole generates Q but no monopole collision
        I=1+.2*(3*mu**2-1)/2
        j0,j2,bq=r.moments(I[None,:],Q,mu,w)
        p,z=r.direct_tensor(I[None,:],Q,mu,w)
        si,sq=r.gains_from_tensor(p,z)
        self.assertAlmostEqual(float(sq[0,0]),float(-.75*j2[0]),places=13)
        self.assertAlmostEqual(float(j0[0]),1.,places=13)
    def test_pure_spin2_manufactured(self):
        mu,w=r.glhalf(96);amp=.071;Q=amp*(1-mu**2)
        c,E=r.screen_modes(Q[None,:],mu,w)
        self.assertAlmostEqual(c[0,0],amp/3,places=13)
        self.assertLess(float(np.max(np.abs(c[0,1:]))),1e-14)
        self.assertAlmostEqual(E[0,0],float(s.sqrt(20*s.pi))*float(12/(5*np.sqrt(6)))*amp/3,places=12)
    def test_three_dimensional_positive_coherency(self):
        mu,w=r.glhalf(64);I=np.ones(len(mu))+.1*(3*mu**2-1)/2;Q=.05*(1-mu**2)
        d=r.direct_3d_tensor(I,Q,mu,w,nphi=16)
        p,z=r.direct_tensor(I[None,:],Q[None,:],mu,w)
        self.assertLess(np.max(abs(d-np.diag([p[0],p[0],z[0]]))),1e-12)
        self.assertTrue(np.all(np.linalg.eigvalsh(d)>0))
    def test_output_shape_and_source_sample(self):
        with np.load(ROOT/'results/BASS21_RANK9_EHARM_50x193.npz') as z:
            self.assertEqual(z['E_l0_plus'].shape,(50,193,8))
            self.assertEqual(z['Mperp'].shape,(50,193))
            self.assertEqual(z['stokes_gain_Q5'].shape,(50,193,5))
            self.assertTrue(np.array_equal(z['B_l0_plus'],np.zeros((50,193,8))))
            self.assertTrue(np.all(z['Mperp']>0))
            self.assertTrue(np.all(z['Mparallel']>0))
            self.assertEqual(len(z['cases']),50)
            self.assertTrue(np.max(np.abs(z['E_l0_plus'][:,:,0]))>1e-3)
    def test_source_summary_and_claim_scope(self):
        j=json.loads((ROOT/'results/SCIENCE_SUMMARY.json').read_text())
        self.assertEqual(j['cases'],50);self.assertEqual(j['epochs'],193)
        self.assertEqual(j['new_ivps'],0)
        self.assertEqual(j['new_E_l0_values'],77200)
        self.assertEqual(j['new_coherency_scalar_values'],28950)
        self.assertEqual(j['new_stokes_gain_samples'],96500)
        self.assertEqual(j['dag']['official_delta'],0)
        self.assertEqual(j['production'],'HOLD')
        self.assertIn('not source-native',j['claim_ceiling'])
        for k in ('tensor_crosswalk','trace','gain_I','gain_Q','GL64_96_tensor','GL64_96_c','GL64_96_E','pr138_c_high','pr134_c_low','explicit_3d_tensor','transverse_offdiag'):
            self.assertLessEqual(j['max_angular_replay_and_tensor_discrepancy'][k],1e-10)
    def test_negative_parent_identity(self):
        with tempfile.TemporaryDirectory() as td:
            bad=pathlib.Path(td)/'corrupted.zip';bad.write_bytes(b'not our original')
            self.assertNotEqual(r.sha(bad),r.PR138_SHA)

if __name__=='__main__':unittest.main()
