"""Bounded physically source-qualified verification of 50 saved BASS21 histories."""
import csv, hashlib, json, pathlib, sys, unittest
from fractions import Fraction as Q
import numpy as np
ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'code'))
import full_history_replay as rep
sys.path.insert(0,str(ROOT/'source'/'previous_PR121'/'rei_pr269'/'code'))
import native_full_rate as source

class SourceFull25(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.manifest=json.loads((ROOT/'results'/'SOURCE_INPUT_MANIFEST.json').read_text())
  cls.report=json.loads((ROOT/'results'/'SCIENCE_SUMMARY.json').read_text())
  with (ROOT/'results'/'MATCHED_25_PAIR_SUMMARY.csv').open() as f:cls.pairs=list(csv.DictReader(f))
  with (ROOT/'results'/'MATCHED_25_X_193_OPTICAL_DEPTH.csv').open() as f:cls.history=list(csv.DictReader(f))
  with (ROOT/'results'/'ALL_50_NATIVE_193_CHEMISTRY.csv').open() as f:cls.native=list(csv.DictReader(f))
 def test_01_source_owner_model_exact(self):
  self.assertEqual(source.source_manifest_check()['BASS_ATOMIC_NETWORK.py'],'baecce07aded483fa60006cc576feff67d068c0d70394f7095abacc6d47eaa80')
  self.assertEqual(self.manifest['owner'],self.report['owner'])
 def test_02_full_original_exact_binary_sha_and_git_blob(self):
  self.assertEqual(len(self.manifest['sources']),50)
  for case,expected in self.manifest['sources'].items():
   raw=(ROOT/'inputs'/(case+'.npz')).read_bytes()
   self.assertEqual(hashlib.sha256(raw).hexdigest(),expected['sha256'])
   self.assertEqual(hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest(),expected['git_blob'])
 def test_03_case_coverage_and_scoped_DAG(self):
  self.assertEqual({f's{s:02d}_m{m:02d}_r{r:02d}' for s in (0,4) for m in range(5) for r in range(5)},set(self.manifest['sources']))
  self.assertEqual(self.report['DAG']['VigilODE'],'7/23');self.assertEqual(self.report['new_ivps'],0)
 def test_04_all_physical_row_counts(self):
  self.assertEqual(len(self.native),9650);self.assertEqual(len(self.history),4825);self.assertEqual(len(self.pairs),25)
  self.assertEqual(self.report['native_original_evaluate_calls'],9650)
 def test_05_fj_array_shapes_species(self):
  with np.load(ROOT/'results'/'ALL_50_NATIVE_FJ.npz',allow_pickle=False) as z:
   self.assertEqual(z['F'].shape,(50,193,21))
   self.assertEqual(z['J'].shape,(50,193,21,21))
   self.assertEqual(z['photo_source'].shape,(50,193))
   net=source.new_native_network()
   self.assertTrue(np.array_equal(z['q'],net.q) and np.array_equal(z['A'],net.A))
 def test_06_fj_direct_native_evaluate_representative(self):
  net=source.new_native_network()
  with np.load(ROOT/'results'/'ALL_50_NATIVE_FJ.npz',allow_pickle=False) as z:
   names=list(map(str,z['names']))
   for c in ['s00_m00_r00','s04_m02_r03','s04_m04_r04']:
    idx=names.index(c)
    with np.load(ROOT/'inputs'/(c+'.npz'),allow_pickle=False) as d:
     keys=list(map(str,d['diagnostic_names']))
     for k in [0,92,192]:
      T=float(d['diagnostics'][k,keys.index('T_K')]);uv=float(d['diagnostics'][k,keys.index('uv_scale')]);nb=float(d['diagnostics'][k,keys.index('nb_m3')]);Y=d['Y'][k]
      r=net.evaluate(Y,T,uv,nb)
      # Native rate evaluation has sub-ulp variation (BLAS summation order); numerical, not byte, comparison.
      self.assertLessEqual(float(np.max(np.abs(r['dY_dt']-z['F'][idx,k]))),5e-31)
      self.assertLessEqual(float(np.max(np.abs(r['jac']-z['J'][idx,k]))),5e-31)
 def test_07_native_gamma_ne_photo_exact_vs_saved(self):
  for k in ['max_ne_defect','max_gamma_defect','max_photo_power_defect']:
   self.assertEqual(self.report['validation']['saved_state_defects'][k],0.0)
 def test_08_baryon_conservation_original(self):
  self.assertLess(self.report['validation']['saved_state_defects']['max_baryon_defect'],3e-15)
 def test_09_initial_shear_and_bath_match(self):
  for m in range(5):
   for r in range(5):
    a,b=[np.load(ROOT/'inputs'/f's{s:02d}_m{m:02d}_r{r:02d}.npz',allow_pickle=False) for s in (0,4)]
    try:
     self.assertTrue(np.array_equal(a['x'],b['x']))
     for key in ['T_K','uv_scale','nb_m3']:
      j=list(a['diagnostic_names']).index(key)
      self.assertTrue(np.array_equal(a['diagnostics'][:,j],b['diagnostics'][:,j]))
     ia={str(s):i for i,s in enumerate(a['signed_observable_names'])}
     self.assertAlmostEqual(float(a['signed_observables'][0,ia['S_F']]),0,delta=2e-14)
     self.assertAlmostEqual(float(b['signed_observables'][0,ia['S_F']]),0.6,delta=2e-14)
     for key in ['P_m_F','P_r_F']:
      self.assertAlmostEqual(float(a['signed_observables'][0,ia[key]]-b['signed_observables'][0,ia[key]]),0,delta=2e-14)
    finally:a.close();b.close()
 def test_10_exact_rational_triply_factored_identity(self):
  for m in range(5):
   for r in range(5):
    a,b=[np.load(ROOT/'inputs'/f's{s:02d}_m{m:02d}_r{r:02d}.npz',allow_pickle=False) for s in (0,4)]
    try:
     k=list(map(str,a['diagnostic_names']));nb=k.index('nb_m3');H=k.index('H');q=a['q']
     for i in [0,64,128,192]:
      w0,w1=[Q.from_float(float(z['diagnostics'][i,nb])) for z in (a,b)]
      v0,v1=[1/Q.from_float(float(z['diagnostics'][i,H])) for z in (a,b)]
      y0,y1=[sum(Q.from_float(float(yi))*int(qj) for yi,qj in zip(z['Y'][i],q)) for z in (a,b)]
      bar=lambda v0,v1:(v0+v1)/2
      n0,n1=w0*y0,w1*y1
      rhs=bar(v0,v1)*bar(w0,w1)*(y1-y0)+bar(v0,v1)*bar(y0,y1)*(w1-w0)+bar(n0,n1)*(v1-v0)
      self.assertEqual((n1*v1-n0*v0)-rhs,0)
    finally:a.close();b.close()
 def test_11_pointwise_integrated_product_split(self):
  self.assertLess(self.report['science']['max_abs_rational_pointwise_residual_float'],1e-15)
  self.assertLess(self.report['science']['max_abs_cumulative_split_residual'],1e-15)
  for r in self.pairs:
   self.assertAlmostEqual(float(r['delta_tau_simpson']),float(r['geometry'])+float(r['chemistry'])+float(r['density']),delta=1e-15)
 def test_12_no_density_differences(self):
  self.assertEqual(self.report['science']['max_nb_delta'],0)
  self.assertTrue(all(float(r['density'])==0 for r in self.pairs))
 def test_13_gamma_coefficients_same_across_shear(self):
  rows={(r['case'],int(r['epoch'])):r for r in self.native}
  for m in range(5):
   for r in range(5):
    for e in range(193):
     a=rows[(f's00_m{m:02d}_r{r:02d}',e)];b=rows[(f's04_m{m:02d}_r{r:02d}',e)]
     for key in ['photo_HI_s_inv','photo_He3I_s_inv','photo_He3II_s_inv','photo_He4I_s_inv','photo_He4II_s_inv','photo_Li6I_s_inv','photo_Li7I_s_inv']:
      self.assertEqual(a[key],b[key])
 def test_14_history_reproduces_old_five_exact(self):
  for d in self.report['validation']['previous_five_pair_deltas'].values():
   self.assertEqual(d,{'stored_tau':0,'simpson_tau':0,'geometry':0,'ionization':0})
 def test_15_negative_shear_tau_25_pairs_without_observation_claim(self):
  self.assertTrue(self.report['science']['all_delta_tau_negative'])
  self.assertTrue(all(float(r['delta_tau_stored'])<0 for r in self.pairs))
  self.assertTrue(all(0.9200<float(r['geometry_fraction'])<0.9213 for r in self.pairs))
 def test_16_photo_and_recombination_signs(self):
  self.assertTrue(all(float(r['photo_e_source_m3_s'])>=0 for r in self.native))
  self.assertTrue(all(float(r['RR_e_source_m3_s'])<=0 for r in self.native))
  self.assertTrue(all(float(r['photo_primary_electron_W_m3'])<=float(r['photo_absorbed_W_m3'])+1e-40 for r in self.native))
 def test_17_chemistry_concentrated_in_source_transition_window(self):
  ratios=[float(r['window1_chemistry'])/float(r['chemistry']) for r in self.pairs]
  self.assertTrue(all(.9909<q<.9911 for q in ratios))
 def test_18_quadrature_error_observation_not_solver_certification(self):
  self.assertLess(self.report['science']['max_source_vs_saved_Simpson_diff'],1e-9)
  self.assertIn('solver-global BDF convergence',self.report['not_claimed'])
 def test_19_independent_gates_unrun(self):
  self.assertIn('independent R6 or C9',self.report['not_claimed'])
  self.assertEqual(self.report['DAG']['original_node_delta'],0)
 def test_20_no_new_ode_solver_calls(self):
  t=(ROOT/'code'/'full_history_replay.py').read_text()
  self.assertNotIn('solve_ivp(',t); self.assertNotIn('odeint(',t)
if __name__=='__main__':unittest.main()
