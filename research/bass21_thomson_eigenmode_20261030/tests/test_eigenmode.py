"""Actual exact and source-bound tests for BASS21 frozen-source collision eigenmodes."""
from __future__ import annotations
import csv,json,pathlib,sys,tempfile,unittest
import numpy as np
import sympy as sp
ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'code'))
import eigenmode_atlas as e

class RealEigenmodeChecks(unittest.TestCase):
    def test_exact_thomson_eigenspectrum(self):
        A=sp.Matrix([[sp.Rational(9,10),sp.Rational(3,40)],[sp.Rational(4,5),sp.Rational(2,5)]])
        R=sp.Matrix([[3,1],[4,-8]])
        self.assertEqual(R.inv()*A*R,sp.diag(1,sp.Rational(3,10)))
        self.assertEqual(R.inv(),sp.Matrix([[sp.Rational(2,7),sp.Rational(1,28)],[sp.Rational(1,7),-sp.Rational(3,28)]]))
    def test_exact_full_tensor_anisotropy(self):
        j,b=sp.symbols('j b',real=True)
        mpar=4*sp.pi*((1-j)/3+b/4)
        mperp=4*sp.pi*(sp.Rational(1,3)+j/6-b/8)
        self.assertEqual(sp.simplify(mpar-mperp+sp.pi*(4*j-3*b)/2),0)
    def test_exact_shear_projection(self):
        m,J0,J2,J4,c2,c4=sp.symbols('m J0 J2 J4 c2 c4',real=True)
        P2=sp.legendre(2,m);P4=sp.legendre(4,m)
        I=J0+5*J2*P2+9*J4*P4
        LI=3*m*(1-m**2)*sp.diff(I,m)+4*(1-3*m**2)*I
        J2rate=sp.integrate(P2*LI,(m,0,1))
        self.assertEqual(sp.simplify(J2rate+sp.Rational(8,5)*J0+sp.Rational(10,7)*J2-sp.Rational(36,35)*J4),0)
        Q=c2*sp.assoc_legendre(2,2,m)+c4*sp.assoc_legendre(4,2,m)
        LQ=3*m*(1-m**2)*sp.diff(Q,m)+4*(1-3*m**2)*Q
        B=2*sp.integrate(Q*(1-m**2),(m,0,1))
        Brate=2*sp.integrate(LQ*(1-m**2),(m,0,1))
        self.assertEqual(sp.simplify(Brate-sp.Rational(10,7)*B-sp.Rational(32,7)*c4),0)
    def test_manufactured_mode_inversion(self):
        j=np.array([-.3,0.,.06]);b=np.array([.01,.1,-.2])
        u,v=e.source_modes(j,b);j2,b2=e.reconstructed_jb(u,v)
        np.testing.assert_allclose(j,j2,atol=1e-16,rtol=0)
        np.testing.assert_allclose(b,b2,atol=1e-16,rtol=0)
    def test_zero_shear_and_zero_opacity_limits(self):
        x=np.linspace(0,2,101);zero=np.zeros_like(x)
        for lam in (1.,.3):
            np.testing.assert_array_equal(e.optical_green(zero,zero,x,lam),zero)
            np.testing.assert_allclose(e.optical_green(np.ones_like(x),zero,x,lam),x,atol=1e-15,rtol=0)
        aa=e.projected_source(np.ones(3),np.ones(3),np.ones(3),np.ones(3),np.ones(3),np.zeros(3))
        for val in aa:np.testing.assert_array_equal(val,np.zeros(3))
    def test_analytic_greens_function_constant_source(self):
        x=np.linspace(0,1,5001);g=.37;source=np.ones_like(x)*.28;tau=g*x
        for lam in (1.,.3):
            got=e.optical_green(source,tau,x,lam)
            exact=.28*(1-np.exp(-lam*g*x))/(lam*g)
            self.assertLess(float(np.max(np.abs(got-exact))),2e-10)
    def test_negative_source_identity_not_accepted(self):
        with tempfile.TemporaryDirectory() as td:
            path=pathlib.Path(td)/'fake_source.zip';path.write_bytes(b'owner-mismatch')
            self.assertNotEqual(e.digest(path),e.OWNER_SHA)
            self.assertNotEqual(e.digest(path),e.PARENT_SHA)
    def test_original_50_case_all_output_arrays(self):
        with np.load(ROOT/'results/EIGENMODE_50x193_SOURCE_PHYSICS.npz',allow_pickle=False) as z:
            self.assertEqual(len(z['cases']),50)
            self.assertEqual(z['U_fast'].shape,(50,193));self.assertEqual(z['J4_total'].shape,(50,193))
            u,v=z['U_fast'],z['V_slow']
            self.assertTrue(np.all(z['g']>=0))
            self.assertTrue(np.all(np.isfinite(z['U_rate_defect'])))
            np.testing.assert_allclose(z['C_U'],-z['g']*u,rtol=0,atol=1e-10)
            np.testing.assert_allclose(z['C_V'],-.3*z['g']*v,rtol=0,atol=1e-10)
            self.assertLess(float(np.max(np.abs(z['U_dynamic_defect']))),1e-3)
            self.assertLess(float(np.max(np.abs(z['V_dynamic_defect']))),1e-3)
            self.assertGreater(float(np.max(np.abs(z['V_dynamic_defect']))),1e-7) # do not erase real defect
    def test_pair_initial_and_residual_not_silently_dropped(self):
        with np.load(ROOT/'results/EIGENMODE_50x193_SOURCE_PHYSICS.npz',allow_pickle=False) as z:
            self.assertGreater(float(np.max(np.abs(z['U_fast'][25:,0]-z['U_fast'][:25,0]))),.04)
        with open(ROOT/'results/EIGENMODE_PAIR25_ENDPOINT.csv',newline='') as f:rows=list(csv.DictReader(f))
        self.assertEqual(len(rows),25)
        for row in rows:
            u=float(row['delta_U_final']);p=float(row['delta_P_U_final']);h=float(row['delta_H_U_final']);r=float(row['delta_model_V_defect_final'])
            self.assertTrue(np.isfinite([u,p,h,r]).all())
            self.assertLess(abs(u-p-h),.001)
    def test_science_summary_claim_and_dag(self):
        summary=json.loads((ROOT/'results/SCIENCE_SUMMARY.json').read_text())
        self.assertEqual(summary['source_cases'],50)
        self.assertEqual(summary['count_causal_memory_U_V'],19300)
        self.assertEqual(summary['dag']['vigilode'],'7/23')
        self.assertEqual(summary['dag']['official_delta'],0)
        self.assertEqual(summary['new_ivps'],0)
        self.assertEqual(summary['dynamical_native_E_l'],'NOT_RUN')
        self.assertEqual(summary['production'],'HOLD')
        self.assertGreater(summary['max_tau_derivative_minus_g'],0)
        self.assertLess(summary['max_falsifier_residuals']['anisotropy_V'],1e-10)
        self.assertGreater(summary['max_falsifier_residuals']['green_integrator_fast'],0)
        self.assertEqual(len(summary['source_sha256_by_case']),50)

if __name__=='__main__':unittest.main()
