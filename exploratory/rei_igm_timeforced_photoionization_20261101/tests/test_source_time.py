import hashlib,io,json,math,sys,tempfile,unittest,zipfile
from pathlib import Path
import numpy as np
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT))
from prepare_sources import read_sources,PR332_SHA,PR342_SHA
from source_time_ivp import SourceIVP

class OriginalSourcePhysics(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.f,cls.hi,cls.he,_=read_sources()
        cls.model=SourceIVP()
    def test_source_full_sha_and_original_nodal_rates(self):
        self.assertEqual(self.f['source_pr332_artifact_sha256'],PR332_SHA)
        self.assertEqual(self.f['source_pr342_artifact_sha256'],PR342_SHA)
        self.assertEqual(len(self.hi),46080)
        self.assertTrue(np.all(self.hi>=0) and np.all(self.he>=0))
    def test_original_knot_order_and_binding(self):
        self.assertEqual(len(self.f['sites']),5)
        self.assertAlmostEqual(self.f['sites'][0]['u'],0)
        self.assertAlmostEqual(self.f['sites'][-1]['u'],1)
        for s in self.f['sites']:
            J,W=self.model.forcing(s['u'])
            np.testing.assert_allclose(J,s['J_per_s'],rtol=1e-14,atol=0)
            np.testing.assert_allclose(W,s['weights'],rtol=2e-14,atol=1e-323)
    def test_group_photon_normalization(self):
        for u in np.linspace(0,1,25):
            for w in ([1,1,1],[.99,.98,1.001]):
                _,p,e,_=self.model.rates(u,w)
                np.testing.assert_allclose(p.sum(axis=1)+e,np.ones(4),atol=4e-14,rtol=0)
                self.assertTrue(np.all(p>=0) and np.all(e>=0))
    def test_global_neutral_nuclei_and_photon_ledger_rhs(self):
        for u in (.0,.14,.29,.5,.9,1):
            y=np.r_[np.array([.9998,.9995,1.000001]),np.zeros(12)]
            rhs=self.model.rhs(u,y)
            inv=self.model.invariants(y+.001*rhs)-self.model.invariants(y)
            self.assertLess(float(np.max(np.abs(inv))),2e-15)
    def test_jvp_finite_difference(self):
        y=np.r_[np.array([.9998,.9995,1.000001]),np.zeros(12)]
        v=np.linspace(-.3,.4,15)
        for u in (.01,.2,.4,.7,.95):
            e=1e-5
            direct=(self.model.rhs(u,y+e*v)-self.model.rhs(u,y-e*v))/(2*e)
            analytic=self.model.jvp(u,y,v)
            np.testing.assert_allclose(direct,analytic,rtol=4e-7,atol=4e-10)
    def test_structural_zero_and_subnormal_noninterpretation(self):
        self.assertEqual(self.f['sites'][0]['weights'][0][:3],[0,0,0])
        self.assertGreater(self.f['sites'][0]['J_per_s'][3],0.)
    def test_new_source_not_frozen_duplicate(self):
        a=SourceIVP();b=SourceIVP(frozen=True)
        y=a.initial();x=a.rhs(1,y);z=b.rhs(1,y)
        self.assertGreater(abs(x[4]-z[4]),1e-9)
    def test_all_actual_rei_observables_evolved(self):
        a=self.model.solve();b=self.model.solve(method='DOP853')
        self.assertLess(float(np.max(np.abs(a.y-b.y))),3e-12)
        self.assertLess(float(np.max(np.abs(self.model.invariants(a.y.T)))),8e-13)
        self.assertTrue(np.all(a.y[:3,:]>0))
        self.assertGreater(a.y[14,-1],0.)
    def test_old_frozen_reference_reused(self):
        a=SourceIVP(frozen=True).solve()
        self.assertAlmostEqual(float(a.y[0,-1]),0.9999956246077383,delta=3e-11)
        self.assertAlmostEqual(float(a.y[1,-1]),0.999397103960242,delta=3e-11)
    def test_frozen_vs_time_response_is_resolved(self):
        a=self.model.solve();b=SourceIVP(frozen=True).solve()
        self.assertGreater(abs(a.y[4,-1]-b.y[4,-1]),2e-8)
        self.assertLess(abs(a.y[4,-1]-b.y[4,-1]),3e-8)
    def test_fail_closed_missing_source(self):
        with tempfile.NamedTemporaryFile() as f:
            f.write(b'wrong');f.flush()
            with self.assertRaises(ValueError):read_sources(path332=f.name)
    def test_sci_final_node_replay(self):
        f=ROOT/'results/SCIENCE_RESULT.json'
        if not f.exists():self.skipTest('science output not present')
        j=json.loads(f.read_text())
        self.assertEqual(j['source_cell_count'],46080)
        self.assertEqual(j['canonical_delta'],0)
        self.assertEqual(j['time_history_rows'],11)
        self.assertLess(j['independent_reference_max_discrepancy']['forced_Radau_vs_DOP853'],3e-12)
if __name__=='__main__':unittest.main()
