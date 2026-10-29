from __future__ import annotations
import sys, unittest, random, math
from pathlib import Path
from fractions import Fraction as Q
import numpy as np
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'code'))
from bass21_stage_box import build_native_box, evaluate_formula, dot, vec, outward, frac, norm_w

class FrozenNativeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.net,cls.rate,cls.box,cls.T,cls.uv=build_native_box()
        cls.samples=0;cls.j_inclusions=0;cls.f_inclusions=0

    def test_source_native_129_numeric_identity(self):
        # Native source integrity function and independent existing 129-float numeric-content fingerprint.
        from bass21_stage_box import native
        self.assertEqual(len(native.source_manifest_check()),3)
        self.assertEqual(len(self.box.y),21)
        self.assertGreater(float(self.rate['photo_s'][1]),0)

    def test_native_eval_matches_exact_source_polynomial(self):
        b=self.box
        rb=self.net.evaluate(np.array([float(x) for x in b.y]),self.T,self.uv,float(b.nb))
        d=np.array([float(x) for x in b.F0]);j=np.array([[float(x) for x in row] for row in b.J0])
        self.assertTrue(np.allclose(rb['dY_dt'],d,rtol=3e-15,atol=1e-28))
        self.assertTrue(np.allclose(rb['jac'],j,rtol=3e-15,atol=1e-25))

    def test_baryon_assembly_roundoff_explicit(self):
        b=self.box
        # Source's rounded G0+D is NOT an exactly conservative rational matrix.
        # This is an observed representation defect, not a weakened solver tolerance.
        residues=[]
        for j in range(21):
            loss=sum((b.baryon[i]*b.J0[i][j] for i in range(21)),Q(0))
            magnitude=sum((abs(b.baryon[i]*b.J0[i][j]) for i in range(21)),Q(0))
            self.assertLessEqual(abs(loss),magnitude/Q(2**48))
            residues.append(loss)
        self.assertNotEqual(residues[5],Q(0)) # preserve the observed negative evidence
        self.assertEqual(dot(b.baryon,b.F0),Q(0))

    def test_exact_21x21_box_inclusions(self):
        b=self.box;rng=random.Random(20261029)
        for k in range(48):
            z=[yi+di*Q(rng.randint(-1000,1000),1000) for yi,di in zip(b.y,b.d)]
            # Positive for every initial nonzero represented species.
            self.assertTrue(all(v>=0 for v in z))
            f,j=b.physical_sample(z)
            for i in range(21):
                self.assertLessEqual(abs(f[i]-b.F0[i]),b.Frad[i]);self.__class__.f_inclusions+=1
                for h in range(21):
                    self.assertLessEqual(abs(j[i][h]-b.J0[i][h]),b.Jrad[i][h]);self.__class__.j_inclusions+=1
            self.__class__.samples+=1
        self.assertEqual(self.__class__.samples,48)
        self.assertEqual(self.__class__.j_inclusions,21168)
        self.assertEqual(self.__class__.f_inclusions,1008)

    def test_chemistry_same_epoch_jvp_error(self):
        b=self.box;rng=random.Random(7001)
        z=[yi+di*Q(rng.randint(-10,10),20) for yi,di in zip(b.y,b.d)]
        _,j=b.physical_sample(z)
        v=[Q(rng.randint(-4,4),7) for _ in b.y]
        err=[dot([a-c for a,c in zip(j[i],b.J0[i])],v) for i in range(21)]
        predicted=max(sum((b.Jrad[i][h]*abs(v[h]) for h in range(21)),Q(0))/b.w[i] for i in range(21))
        self.assertLessEqual(norm_w(err,b.w),predicted)
        # A uniform operator norm bound for the source-exact J(Y) only.
        op=max(sum((b.Jrad[i][h]*b.w[h] for h in range(21)),Q(0))/b.w[i] for i in range(21))
        self.assertLessEqual(predicted,op*norm_w(v,b.w))

    def test_frozen_source_same_b_and_j_stage_error(self):
        b=self.box
        J=np.array([[float(x) for x in row] for row in b.J0]);f=np.array([float(x) for x in b.F0])
        x=np.linalg.solve(np.eye(21)-float(b.sigma)*J,float(b.sigma)*f)
        cert=b.certify(x,Q(1,100000),Q(1,100000))
        self.assertTrue(cert['accepted'])
        self.assertLess(cert['physical_stage_error'],1e-5)
        # Direct non-rigorous float spot-check on sampled real states, not an IVP claim.
        for k in range(3):
            rng=random.Random(100+k)
            z=[yi+di*Q(rng.randint(-100,100),100) for yi,di in zip(b.y,b.d)]
            ff,jj=b.physical_sample(z)
            jj=np.array([[float(v) for v in row] for row in jj]);ff=np.array([float(v) for v in ff])
            sol=np.linalg.solve(np.eye(21)-float(b.sigma)*jj,float(b.sigma)*ff)
            actual=max(abs(xi-si)/float(pi) for xi,si,pi in zip(x,sol,b.p))
            self.assertLessEqual(actual,cert['physical_stage_error']*(1+1e-10))

    def test_negative_original_and_physical_gates(self):
        b=self.box
        x=[Q(0)]*21
        rejected=b.certify(x,Q(0),Q(0))
        self.assertFalse(rejected['accepted'])
        r,e=rejected['exact']
        self.assertGreater(r,0);self.assertGreater(e,0)
        self.assertTrue(b.certify(x,r,e)['accepted'])
        self.assertFalse(b.certify(x,r/2,e)['accepted'])
        self.assertFalse(b.certify(x,r,e/2)['accepted'])

    def test_no_stage_clock_extrapolation_and_invalid_inputs(self):
        with self.assertRaises(ValueError):build_native_box(sigma=500.)
        with self.assertRaises(ValueError):build_native_box(sigma=-1.)
        with self.assertRaises(ValueError):build_native_box(sigma=float('nan'))
        b=self.box
        with self.assertRaises(ValueError):b.physical_sample([y+d*2 for y,d in zip(b.y,b.d)])
        with self.assertRaises(ValueError):b.certify([0.,1.],1.,1.)

    def test_outward_rational_endpoints(self):
        for x in [Q(1,10),Q(-1,3),Q(1,10**350),Q(2**100,3),Q(0)]:
            self.assertLessEqual(frac(outward(x,False)),x)
            self.assertGreaterEqual(frac(outward(x,True)),x)

if __name__=='__main__':unittest.main()
