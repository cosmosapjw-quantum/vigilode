import sys,unittest,numpy as np,math,tempfile,pathlib
from numpy.polynomial.legendre import leggauss
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]/'code'))
from e2_spin2_atlas import spin2_projection,B_and_C,C2_TO_E2,BQ_TO_E2,restore_owner,PARENT_SHA,L_EVEN
from scipy.special import lpmv
class PhysicsContracts(unittest.TestCase):
    def setUp(self):
        x,w=leggauss(64);self.mu=(x+1)/2;self.w=w/2
    def test_l2_projection(self):
        A=.125;Q=A*(1-self.mu**2)[None,:]
        c=spin2_projection(Q,self.mu,self.w)
        self.assertAlmostEqual(float(c[0,0]),A/3,places=14)
        self.assertLess(np.max(abs(c[0,1:])),1e-13)
    def test_e2_BQ(self):
        Q=np.exp(self.mu)[None,:]*(1-self.mu**2)
        c=spin2_projection(Q,self.mu,self.w)
        B,C=B_and_C(Q,self.mu,self.w)
        self.assertAlmostEqual(float(C2_TO_E2*c[0,0]),float(BQ_TO_E2*B[0]),places=14)
    def test_shear_C_c4(self):
        Q=np.stack([np.exp(self.mu)*(1-self.mu**2), np.cos(self.mu)*(1-self.mu**2)])
        c=spin2_projection(Q,self.mu,self.w)
        B,C=B_and_C(Q,self.mu,self.w)
        self.assertLess(np.max(abs(C-B/7-(32/21)*c[:,1])),1e-14)
    def test_thomson_j2(self):
        E=.23;B=E/BQ_TO_E2;g=.17
        self.assertAlmostEqual(-g*np.sqrt(6)/10*E,-3*g/40*B,places=14)
    def test_homogeneous_constant_source_limit(self):
        # Q=A(1-mu²) on b'=0, energy monochromatic source fixed.
        j2=.031;E=.41;g=.7
        dE=-g*(np.sqrt(6)/10*j2+2/5*E)
        dB=dE/BQ_TO_E2
        expected=-g*(4/5*j2+2/5*(E/BQ_TO_E2))
        self.assertAlmostEqual(dB,expected,places=13)
    def test_legendre_basis_orthogonality(self):
        for i,l in enumerate(L_EVEN):
            for m in L_EVEN:
                mat=np.dot(self.w,lpmv(2,l,self.mu)*lpmv(2,m,self.mu))
                expected=math.factorial(l+2)/((2*l+1)*math.factorial(l-2)) if m==l else 0
                self.assertAlmostEqual(mat,expected,places=9)
    def test_shear_observer_zero_rate(self):
        g=.2;j2=.14;E=.12;c4=.09
        for s in (0,0.1,-0.1):
            da=s*(10*E/7+4*np.sqrt(6)*c4/7)-g*(np.sqrt(6)*j2/10+2*E/5)
            if s==0:self.assertAlmostEqual(da,-g*(np.sqrt(6)*j2/10+2*E/5))
    def test_input_parent_rejection(self):
        with tempfile.TemporaryDirectory() as tmp:
            bad=pathlib.Path(tmp)/'bad.zip';bad.write_bytes(b'unsafe')
            with self.assertRaisesRegex(ValueError,'PARENT_PR133_ARCHIVE_SHA_DRIFT'):
                restore_owner(bad,pathlib.Path(tmp)/'out.zip')
    def test_positive_norm(self):
        for l in L_EVEN:
            self.assertGreater(math.factorial(l+2)/((2*l+1)*math.factorial(l-2)),0)
if __name__=='__main__':unittest.main()
