from pathlib import Path
import sys,unittest,hashlib,json,io,csv,gzip
import numpy as np
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from rei_moments import MomentSource
ROOT=Path(__file__).resolve().parents[1]
class Physics(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.m=MomentSource()
    def test_original_source_identity(self):
        self.assertEqual(self.m.f['source_artifact_sha256'],'83d94b5383ac7c1f36d89486e83bf8b4a56dd1993a25af3ae1f8fed06a695ba9')
        self.assertEqual(self.m.f['nodes'],46080)
    def test_four_group_photon_normalization(self):
        for w in (np.ones(3),np.array([.8,.4,1.01]),np.array([1.1,.9,1.02])):
            self.assertLess(self.m.normalize_source(0,w),5e-15)
    def test_ionization_signs(self):
        dy=self.m.rhs_aug(0,np.r_[np.ones(3),np.zeros(3)])
        self.assertLess(dy[0],0);self.assertLess(dy[1],0);self.assertGreater(dy[2],0)
        self.assertGreater(dy[3],0);self.assertGreater(dy[4],0)
        self.assertGreaterEqual(dy[5],0)
    def test_original_fourth_group_subnormal_not_as_measurement(self):
        self.assertLess(self.m.J[3],1e-300)
        self.assertGreater(self.m.p0[3,2],.99)
    def test_element_and_photon_identity(self):
        f=self.m.rhs_aug(0,np.r_[np.ones(3),np.zeros(3)])
        self.assertAlmostEqual(f[0]+f[3],0,delta=1e-18)
        self.assertAlmostEqual(f[1]+f[4],0,delta=1e-18)
        self.assertAlmostEqual(f[2]-self.m.S0[1]/self.m.S0[2]*f[4]+f[5],0,delta=1e-18)
    def test_jvp_6d_source(self):
        y=np.r_[np.array([.99993,.9991,1.000005]),np.zeros(3)]
        v=np.array([.1,-.7,1.3,.2,.3,.1]);h=3e-5
        direct=self.m.jvp_aug(0,y,v)
        fd=(self.m.rhs_aug(0,y+h*v)-self.m.rhs_aug(0,y-h*v))/(2*h)
        np.testing.assert_allclose(direct,fd,atol=5e-11,rtol=2e-8)
    def test_jvp_integral_components_zero(self):
        y=np.r_[np.ones(3),np.zeros(3)];v=np.array([0.,0.,0.,1.,-1.,2.])
        self.assertTrue(np.all(self.m.jvp_aug(0,y,v)==0))
    def test_moment_source_negative_fail_closed(self):
        with self.assertRaises(ValueError):self.m.rates([-1,1,1])
        with self.assertRaises(ValueError):self.m.jvp_aug(0,np.ones(6),np.ones(4))
    def test_original_photon_rate_aggregation(self):
        r,p,pe=self.m.rates([1,1,1])
        for i in range(3):
            self.assertAlmostEqual(r[i],float(np.sum(self.m.J*self.m.p0[:,i])),delta=1e-15*max(1.,r[i]))
        self.assertAlmostEqual(float(np.sum(r)+self.m.J@pe),float(np.sum(self.m.J)),delta=3e-15*sum(self.m.J))
    def test_physics_reference_generated_and_source_46080(self):
        out=ROOT/'generated'
        if not (out/'RESULTS.json').exists():self.skipTest('reference calculation not yet executed')
        j=json.loads((out/'RESULTS.json').read_text())
        self.assertEqual(j['source_rows'],46080)
        self.assertLess(j['two_independent_solves']['max_state_difference'],1e-11)
        self.assertLess(j['spatial_source_replay_relative_error'],4e-13)
        data=out/'SOURCE_46080_CELL_PHOTOELECTRONS.csv.gz'
        if data.exists():
            with gzip.open(data,'rt',encoding='utf-8') as f: r=list(csv.DictReader(f))
            self.assertEqual(len(r),46080)
        else:
            with (out/'SOURCE_24_NODE_WITNESS.csv').open(newline='') as f: r=list(csv.DictReader(f))
            self.assertEqual(len(r),24)
        self.assertTrue(all(float(x['new_electrons_source_only_one_tick'])>=0 for x in r))
        self.assertEqual(j['canonical_rei_delta'],0)
if __name__=='__main__':unittest.main()
