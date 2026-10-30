import unittest,pathlib,sys,hashlib,json,tempfile
import numpy as np
ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'code'))
from jc_generator import exact_angular_generator_matrices,sha256,SOURCE_SHA,PARENT_SHA,LJ,LC

class SourceOwnedJC(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.AJ,cls.AC,cls.exact=exact_angular_generator_matrices()
  with np.load(ROOT/'results/BASS21_50x193_JC_GENERATOR.npz') as z:
   cls.z={k:z[k].copy() for k in z.files}
  cls.s=json.loads((ROOT/'results/SCIENCE_SUMMARY.json').read_text())
  cls.base=ROOT.parent/'base'
  cls.parent=ROOT.parent/'parent'
 def test_exact_low_J(self):
  self.assertEqual(self.exact['A_J'][0][1],'-2')
  self.assertEqual(self.exact['A_J'][1][0],'-8/5')
  self.assertEqual(self.exact['A_J'][1][2],'36/35')
  self.assertEqual(self.exact['A_J'][5][6],'1188/161')
 def test_exact_low_spin2(self):
  self.assertEqual(self.exact['A_c'][0][0],'10/7')
  self.assertEqual(self.exact['A_c'][0][1],'10/7')
  self.assertEqual(self.exact['A_c'][1][2],'504/143')
  self.assertEqual(self.exact['A_c'][5][6],'880/87')
 def test_zero_nonlocal_parity(self):
  self.assertTrue(np.array_equal(self.AJ,np.triu(np.tril(self.AJ,1),-1)))
  self.assertTrue(np.array_equal(self.AC,np.triu(np.tril(self.AC,1),-1)))
 def test_source_hash(self):
  self.assertEqual(sha256(ROOT.parent/'ORIGINAL_BASS21_SOURCE_FULL.zip'),SOURCE_SHA)
  self.assertEqual(sha256(ROOT/'source/PR134_CANONICAL_FULL_SOURCE.zip'),PARENT_SHA)
 def test_tampered_source_rejected(self):
  with tempfile.TemporaryDirectory() as d:
   p=pathlib.Path(d)/'fake.zip';p.write_bytes(b'not original source')
   self.assertNotEqual(sha256(p),SOURCE_SHA)
 def test_real_rows_and_initial_zero(self):
  z=self.z
  self.assertEqual(z['generatorJ'].shape,(50,193,6))
  self.assertEqual(z['generatorC'].shape,(50,193,6))
  self.assertTrue(np.all(np.isfinite(z['generatorJ'])))
  self.assertTrue(np.all(np.isfinite(z['generatorC'])))
  self.assertEqual(np.max(abs(z['deltaJ12_14'][:,0,:])),0)
  self.assertLess(np.max(abs(z['spin2_c12_14_16'][:,0,:])),1e-16)
 def test_collision_J0_and_eigenvalues(self):
  z=self.z
  self.assertTrue(np.array_equal(z['collisionJ'][:,:,0],np.zeros((50,193))))
  ev=np.linalg.eigvals(np.array([[-.9,-np.sqrt(6)/10],[-np.sqrt(6)/10,-.4]]))
  self.assertTrue(np.allclose(np.sort(ev),[-1,-.3],atol=1e-15))
 def test_generator_decomposition(self):
  z=self.z
  for suf in ('J','C'):
   self.assertTrue(np.array_equal(z['generator'+suf],z['shear'+suf]+z['collision'+suf]))
 def test_parent_e2_shear_and_collision_reuse(self):
  z=self.z;alpha=12/(5*np.sqrt(6))
  with np.load(self.base/'results/BASS21_E2_SPIN2_50x193x5.npz') as old:
   gapS=np.max(abs(alpha*z['shearC'][:,:,0]-old['E2_shear_rate']))
   gapC=np.max(abs(alpha*z['collisionC'][:,:,0]-old['E2_collision_rate']))
   self.assertLess(gapS,1e-16)
   self.assertLess(gapC,1e-16)
 def test_parent_j2_reuse(self):
  z=self.z
  with np.load(self.parent/'results/STRESS_50x193x6_SAME_OWNER.npz') as p:
   self.assertLess(np.max(abs(z['shearJ'][:,:,1]-p['quadrupole_shear'])),1e-16)
   self.assertLess(np.max(abs(z['collisionJ'][:,:,1]-p['quadrupole_collision'])),1e-16)
 def test_j0_energy_shear(self):
  z=self.z
  # reconstruct bprime from J2 high-neighbour flux with no change to parent source
  from jc_generator import load_owner_archive
  h=load_owner_archive(ROOT.parent/'ORIGINAL_BASS21_SOURCE_FULL.zip')
  cases=z['cases'].tolist()
  with np.load(self.parent/'results/STRESS_50x193x6_SAME_OWNER.npz') as p:
   for ix in (0,25,49):
    hh=h[cases[ix]];bp=np.gradient(hh['b'],hh['x'],edge_order=2)
    self.assertLess(np.max(abs(z['generatorJ'][ix,:,0]+2*bp*p['deltaJ_even'][ix,:,1])),1e-15)
 def test_high_shear_is_resolved_and_low_shear_not(self):
  z=self.z
  c12=z['spin2_c12_14_16'][:,:,0]
  s00=max(abs(c12[:25]).ravel());s04=max(abs(c12[25:]).ravel())
  self.assertLess(s00,1e-17)
  self.assertGreater(s04,1e-10)
  self.assertLess(self.s['max_angular_GL64_96_c_gap'],1e-16)
 def test_unrun_claim_boundaries(self):
  self.assertEqual(self.s['new_ivps'],0)
  self.assertEqual(self.s['dag']['official_delta'],0)
  self.assertEqual(self.s['distinct_author_R6_C9'],'NOT_RUN')
  self.assertEqual(self.s['production'],'HOLD')

if __name__=='__main__':unittest.main()
