import sys,pathlib,math,io,zipfile,unittest,os
import numpy as np
ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'code'))
import compute_boundary_memory as cm
SOURCE=pathlib.Path(os.environ.get('VIGILODE_SOURCE_ZIP',str(ROOT/'source'/'PR145_CORRECTED_FULL_SOURCE.zip')))

class PhysicalMemoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.a,cls.p=cm.read_source(SOURCE)
        cls.x=cls.a['x'];cls.f=cls.a['boundary_flux_16_18_20_22'];cls.tau=cls.p['tau']
        cls.memo=cm.direct_weighted_trapezoid(cls.x,cls.tau,cls.f)
        cls.gross=cm.direct_weighted_trapezoid(cls.x,cls.tau,np.abs(cls.f))

    def test_original_input_identity(self):
        self.assertEqual(cm.sha(SOURCE.read_bytes()),cm.SOURCE_SHA)
        self.assertEqual(self.a['E_l0_plus'].shape,(50,193,12))
        self.assertEqual(self.p['tau'].shape,(50,193))
    def test_flux_from_original_bprime_and_E(self):
        self.assertLess(np.max(abs(cm.reconstruct_flux(self.a,self.p)-self.f)),5e-26)
    def test_parent_clock_identity(self):
        self.assertTrue(np.all(np.diff(self.x)>0))
        self.assertTrue(np.all(np.diff(self.tau,axis=1)>=0))
        self.assertTrue(np.all(self.p['g']>=0))
        self.assertEqual(float(self.tau[:,0].max()),0.)
    def test_no_new_source_integration(self):
        text=(ROOT/'code'/'compute_boundary_memory.py').read_text()
        self.assertNotIn('solve_ivp(',text)
        self.assertNotIn('solve_case(',text)
        self.assertNotIn('ODEProblem(',text)
    def test_zero_boundary_flux(self):
        self.assertTrue(np.all(cm.direct_weighted_trapezoid(self.x,self.tau,np.zeros_like(self.f))==0))
    def test_zero_start(self):
        self.assertTrue(np.all(self.memo[:,0]==0))
    def test_positive_weight_l1_upper_envelope(self):
        self.assertLessEqual(float(np.max(abs(self.memo)-self.gross)),1e-25)
    def test_sign_involution(self):
        neg=cm.direct_weighted_trapezoid(self.x,self.tau,-self.f)
        self.assertLess(np.max(abs(neg+self.memo)),1e-25)
    def test_independent_recursion_all_38600(self):
        self.assertLess(np.max(abs(cm.recurrence_weighted_trapezoid(self.x,self.tau,self.f)-self.memo)),2e-25)
    def test_independent_gross_recursion_all_38600(self):
        self.assertLess(np.max(abs(cm.recurrence_weighted_trapezoid(self.x,self.tau,abs(self.f))-self.gross)),2e-25)
    def test_g0_flinear_closed_form(self):
        x=np.linspace(0,1.7,193)
        f=(np.outer([1,-2.5],x)[:,:,None])*np.ones((2,1,4))
        tau=np.zeros((2,len(x)))
        got=cm.direct_weighted_trapezoid(x,tau,f)
        want=(np.array([1,-2.5])[:,None,None]*x[None,:,None]**2/2)*np.ones((2,1,4))
        self.assertLess(np.max(abs(got-want)),2e-15)
    def test_const_g_exp_source_closed_form(self):
        x=np.linspace(0,1.7,193);tau=np.broadcast_to(0.21*x,(2,len(x))).copy()
        f=np.exp(-tau)[:,:,None]*np.ones((2,1,4))
        got=cm.direct_weighted_trapezoid(x,tau,f)
        want=x[None,:,None]*np.exp(-tau[:,:,None])*np.ones((2,1,4))
        self.assertLess(np.max(abs(got-want)),2e-15)
    def test_wrong_anti_decay_not_equal(self):
        x=np.linspace(0,1,193);tau=np.broadcast_to(.8*x,(1,len(x)))
        f=np.ones((1,len(x),1))
        correct=cm.direct_weighted_trapezoid(x,tau,f)[0,-1,0]
        incorrect=np.exp(tau[0,-1])*np.trapezoid(np.exp(-tau[0])*f[0,:,0],x)
        self.assertGreater(abs(correct-incorrect),.2)
    def test_shear_coefficient_exact_value(self):
        k=cm.ac_up(16)*cm.factor(16)/cm.factor(18)
        self.assertAlmostEqual(k,360*math.sqrt(7733)/2849,places=12)
    def test_expected_flux_maxima(self):
        expected=[3.473637892750975e-11,7.100757483884745e-12,1.6893969153510992e-12,4.2466143828288015e-13]
        self.assertLess(np.max(abs(np.max(abs(self.f),axis=(0,1))-expected)),5e-25)
    def test_nontrivial_signed_memory(self):
        self.assertGreater(np.max(abs(self.memo)),2e-11)
    def test_no_negative_gross(self):
        self.assertGreaterEqual(float(self.gross.min()),0)
    def test_original_case_matching(self):
        names=set(map(str,self.a['cases']))
        self.assertEqual(len(names),50)
        for m in range(5):
            for r in range(5):
                self.assertIn(f's00_m{m:02d}_r{r:02d}',names)
                self.assertIn(f's04_m{m:02d}_r{r:02d}',names)
    def test_single_case_slope_local(self):
        # Smallest first-grid interval: weighted trapezoid self-consistency.
        i,j=0,0;h=self.x[1]-self.x[0]
        expect=.5*h*(np.exp(-(self.tau[i,1]-self.tau[i,0]))*self.f[i,0,j]+self.f[i,1,j])
        self.assertLess(abs(self.memo[i,1,j]-expect),1e-29)

if __name__=='__main__':unittest.main()
