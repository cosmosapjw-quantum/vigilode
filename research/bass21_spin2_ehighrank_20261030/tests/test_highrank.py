from __future__ import annotations
import json,pathlib,sys,unittest,math,hashlib,tempfile
import numpy as np
from numpy.polynomial.legendre import Legendre
ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'code'))
from highrank_atlas import matrices_exact,project,project_derivative_independent,half_gl,k_ell,LL,OWNER_SHA,PARENT_SHA,sha

class SourceOwnedEHighRank(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.C,cls.A,cls.meta=matrices_exact()
        cls.summary=json.loads((ROOT/'results/SCIENCE_SUMMARY.json').read_text())
        with np.load(ROOT/'results/HIGH_E_HIERARCHY_50x193.npz') as z:
            cls.data={k:z[k].copy() for k in z.files}

    def test_source_parent_exact(self):
        self.assertEqual(sha(ROOT/'source/PARENT_PR143_FULL_SOURCE.zip'),PARENT_SHA)
        self.assertEqual(self.summary['source_sha256'],OWNER_SHA)
        self.assertEqual(len(self.summary['source_by_case_SHA256']),50)
        with tempfile.TemporaryDirectory() as d:
            f=pathlib.Path(d)/'forged_source.zip';f.write_bytes(b'forged')
            self.assertNotEqual(sha(f),OWNER_SHA)

    def test_exact_band_and_known_coefficients(self):
        A=self.meta['A_c_rows']
        self.assertEqual(A[0]['2'],'10/7')
        self.assertEqual(A[0]['4'],'10/7')
        self.assertEqual(A[1]['2'],'-36/35')
        self.assertEqual(A[1]['6'],'504/143')
        self.assertEqual(A[7]['18'],'3420/259')
        self.assertTrue(np.array_equal(self.C,np.triu(np.tril(self.C,1),-1)))
        k=np.array([k_ell(l) for l in LL])
        self.assertTrue(np.allclose(self.A,k[:,None]*self.C/k[None,:],atol=0,rtol=0))

    def test_l16_l18_E_normalization_exact_and_hostile_wrong_radicand(self):
        # Regression: the OLD human-readable 360 sqrt(7315)/2849 was wrong;
        # the numerical Galerkin source was already correct.  Check exact
        # factorial normalisation, not merely a rounded tabulated number.
        import sympy as sp
        ell,nxt=16,18
        ac=sp.Rational(3420,259)
        def k_over_sqrt4pi(l):
            return sp.sqrt(sp.factorial(l+2)/(sp.Integer(2*l+1)*sp.factorial(l-2)))
        corrected=sp.Rational(360,7)*sp.sqrt(sp.Rational(19,407))
        alternative=sp.Rational(360,2849)*sp.sqrt(7733)
        wrong=sp.Rational(360,2849)*sp.sqrt(7315)
        exact=sp.simplify(ac*k_over_sqrt4pi(ell)/k_over_sqrt4pi(nxt))
        self.assertEqual(sp.simplify(exact-corrected),0)
        self.assertEqual(sp.simplify(exact-alternative),0)
        self.assertNotEqual(sp.simplify(exact-wrong),0)
        self.assertAlmostEqual(float(self.A[7,8]),float(corrected),places=12)
        self.assertGreater(abs(float(self.A[7,8])-float(wrong)),0.2)

    def test_manufactured_legendre_orthogonality_and_independent_projection(self):
        m,w=half_gl(96)
        for ell in (2,8,18,24):
            p=(1-m*m)*Legendre.basis(ell).deriv(2)(m)
            q=p[None,:]
            E=project(q,m,w)[0]
            ref=project_derivative_independent(q,m,w)[0]
            pos=list(LL).index(ell)
            self.assertLess(np.max(np.abs(E-ref)),1e-7)
            self.assertAlmostEqual(E[pos]/k_ell(ell),1,places=9)
            self.assertLess(np.max(np.abs(np.delete(E,pos)))/k_ell(ell),1e-9)

    def test_direct_liouville_spin2_adjoint(self):
        m,w=half_gl(96)
        q=(1-m*m)*Legendre.basis(6).deriv(2)(m)
        dq=Legendre.basis(6).deriv(2)(m)*(-2*m)+(1-m*m)*Legendre.basis(6).deriv(3)(m)
        source=4*(1-3*m*m)*q + 3*m*(1-m*m)*dq
        rate=project(source[None,:],m,w)[0]
        c0=np.zeros(len(LL));c0[list(LL).index(6)]=1
        e0=c0*np.array([k_ell(ell) for ell in LL])
        expected=self.A@e0
        self.assertLess(np.max(np.abs(rate-expected)),3e-9)

    def test_physics_shapes_and_parent_reuse(self):
        z=self.data;s=self.summary
        self.assertEqual(z['E_l0_plus'].shape,(50,193,12))
        self.assertEqual(z['local_generator_E'].shape,(50,193,11))
        self.assertEqual(z['boundary_flux_16_18_20_22'].shape,(50,193,4))
        self.assertTrue(np.array_equal(z['local_generator_E'],z['shear_E']+z['collision_E']))
        self.assertLess(s['max_reuse_E_l2_16'],1e-10)
        self.assertLess(s['max_reuse_c_l2_16'],1e-10)
        self.assertLess(s['max_GL64_96_gap'],1e-10)
        self.assertLess(s['max_independent_basis_gap'],1e-10)

    def test_original_J2_collision_and_l2_shear(self):
        z=self.data
        with np.load(ROOT/'source/PR141_EHARM_50x193.npz') as old:
            J2=old['J2_total'];E2=old['E_l0_plus'][:,:,0]
        assert J2.shape==(50,193)
        # parent owner g(x) and b'(x) are stored in previous source eigenmode ZIP;
        # our collision source coefficient uses total, not delta-J2.
        prev=ROOT/'source/PARENT_PR143_FULL_SOURCE.zip'
        import zipfile,io
        with zipfile.ZipFile(prev) as arc:
            raw=arc.read('results/EIGENMODE_50x193_SOURCE_PHYSICS.npz')
        with np.load(io.BytesIO(raw)) as y:
            g=y['g'];bp=y['bprime']
        expect=-g*(k_ell(2)*J2/4+.4*E2)
        self.assertLess(np.max(np.abs(z['collision_E'][:,:,0]-expect)),5e-15)
        predict=bp*(10/7*E2+10/7*(k_ell(2)/k_ell(4))*z['E_l0_plus'][:,:,1])
        self.assertLess(np.max(np.abs(z['shear_E'][:,:,0]-predict)),5e-15)
        for i in range(1,11):
            self.assertLess(np.max(np.abs(z['collision_E'][:,:,i]+g*z['E_l0_plus'][:,:,i])),5e-15)

    def test_boundaries_depend_on_measured_higher_mode(self):
        z=self.data
        with __import__('zipfile').ZipFile(ROOT/'source/PARENT_PR143_FULL_SOURCE.zip') as arc:
            pass
        import zipfile,io
        with zipfile.ZipFile(ROOT/'source/PARENT_PR143_FULL_SOURCE.zip') as arc:
            raw=arc.read('results/EIGENMODE_50x193_SOURCE_PHYSICS.npz')
        with np.load(io.BytesIO(raw)) as y:bp=y['bprime']
        for j,ell in enumerate((16,18,20,22)):
            i=list(LL).index(ell)
            expected=bp*self.A[i,i+1]*z['E_l0_plus'][:,:,i+1]
            self.assertLess(np.max(abs(expected-z['boundary_flux_16_18_20_22'][:,:,j])),3e-16)
            self.assertGreater(np.max(np.abs(z['boundary_flux_16_18_20_22'][:,:,j])),1e-14)

    def test_signal_vs_angular_noise_not_uniformly_detected(self):
        s=self.summary
        self.assertLess(s['max_GL64_96_gap'],1e-15)
        for j in range(8,12):
            self.assertGreater(s['max_abs_by_shear_family']['s04'][j],10*s['max_gap_by_mode'][j])
            self.assertLess(s['max_abs_by_shear_family']['s00'][j],10*s['max_gap_by_mode'][j])

    def test_dag_hold_and_no_ivps(self):
        s=self.summary
        self.assertEqual(s['new_ivps'],0)
        self.assertEqual(s['dag']['official_delta'],0)
        self.assertEqual(s['dag']['vigilode'],'7/23')
        self.assertEqual(s['dag']['successor_admitted'],'0/12')
        self.assertEqual(s['production'],'HOLD')
        self.assertEqual(s['distinct_author_R6_C9'],'NOT_RUN')
        self.assertGreater(s['max_saved_grid_derivative_defect'],1e-8)

    def test_positivity_and_limits(self):
        self.assertLess(self.summary['max_polarization_fraction'],1)
        assert np.all(np.isfinite(self.data['E_l0_plus']))
        m,w=half_gl(64)
        zero=np.zeros((1,len(m)))
        self.assertTrue(np.array_equal(project(zero,m,w),np.zeros((1,len(LL)))))
        self.assertTrue(np.array_equal(self.A@np.zeros(len(LL)),np.zeros(len(LL))))
        self.assertEqual(0*float(np.sum(self.A)),0)

if __name__=='__main__':unittest.main()
