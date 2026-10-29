"""Focused source-owned physics tests; never solve an IVP."""
import hashlib
import sys
import unittest
from pathlib import Path
from fractions import Fraction
import numpy as np
R=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(R/'code'))
import paired_replay as p
from scipy.integrate import cumulative_simpson

PAIRS=('m00_r00','m02_r02','m04_r04','m00_r04','m04_r00')
class TestMatchedPhysics(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.runs={k:p.run(k) for k in PAIRS}

    def test_exact_source_bytes_all_ten(self):
        self.assertEqual(len(p.PIN),10)
        for c,(rel,expected,blob) in p.PIN.items():
            f=(R/'work'/rel) if c=='s00_m00_r00' else (R/rel)
            self.assertEqual(hashlib.sha256(f.read_bytes()).hexdigest(),expected)
            self.assertEqual(len(blob),40)

    def test_all_5_pairs_and_193_samples(self):
        self.assertEqual(len(self.runs),5)
        for name,(s,rows) in self.runs.items():
            self.assertEqual(len(rows),193,name)
            self.assertEqual(s['sources']['AtomicNetwork_evaluate_calls'],386)
            self.assertEqual(s['sources']['new_ivps'],0)

    def test_BASS_native_rate_identity(self):
        for key,(s,rows) in self.runs.items():
            self.assertEqual(s['sources']['max_source_native_gamma_mismatch'],0.,key)
            self.assertEqual(s['sources']['max_saved_photo_energy_mismatch'],0.,key)
            self.assertLess(s['sources']['max_saved_ne_mismatch'],1.e-12,key)

    def test_same_epoch_and_owner_bath(self):
        for k,(s,rows) in self.runs.items():
            self.assertTrue(s['sources']['source_T_identical'])
            self.assertTrue(s['sources']['source_UV_identical'])
            self.assertEqual(s['sources']['n_b_max_abs_diff'],0)
            self.assertEqual(s['sources']['LiI_photo_rate_max_abs_diff'],0)

    def test_baryon_conservation_compatible(self):
        for key,(s,_) in self.runs.items():
            self.assertLess(s['sources']['max_baryon_defect'],2e-12,key)

    def test_exact_product_decomposition_numeric(self):
        for key,(s,rows) in self.runs.items():
            self.assertLess(s['check']['max_pointwise_exact_decomposition_residual'],1e-15,key)
            self.assertLess(s['check']['max_integral_decomposition_residual'],1e-15,key)
            self.assertLess(s['check']['max_derivative_electron_split_residual'],2e-15,key)
            self.assertEqual(s['contributions_simpson']['density'],0.)

    def test_rational_product_identity(self):
        # Independent exact-real check for five representative source-recorded rows.
        for pair,(s,rows) in self.runs.items():
            for i in (0,39,78,137,192):
                row=rows[i];F=lambda k:Fraction.from_float(row[k]);
                a,b=F('ne_A_m3'),F('ne_B_m3')
                u,v=F('H_A_s'),F('H_B_s')
                lhs=b/v-a/u
                rhs=((b-a)*(1/u+1/v)+(a+b)*(1/v-1/u))/2
                self.assertEqual(lhs,rhs,(pair,i))

    def test_saved_tau_matches_simpson_as_postprocess_only(self):
        for key,(s,rows) in self.runs.items():
            self.assertLess(s['check']['max_simpson_vs_stored_delta_tau'],1e-9,key)
            self.assertLess(abs(s['contributions_simpson']['total']-s['saved']['delta_tau_stored_end']),1e-10,key)

    def test_matched_history_negative_tau(self):
        for key,(s,rows) in self.runs.items():
            self.assertLess(s['saved']['delta_tau_stored_end'],0.,key)
            self.assertLess(s['contributions_simpson']['geometry'],0.,key)
            self.assertLess(s['contributions_simpson']['chemistry'],0.,key)

    def test_geometry_dominates_without_universal_admission(self):
        for key,(s,_) in self.runs.items():
            g=s['contributions_simpson']['geometry']/s['contributions_simpson']['total']
            self.assertGreater(g,0.90,key)
            self.assertLess(g,0.95,key)
            self.assertEqual(s['claimed_scope']['physical_causal_attribution'],'NOT_ESTABLISHED')

    def test_intermediate_xe_difference_vs_final(self):
        for key,(s,_) in self.runs.items():
            self.assertGreater(s['sources']['max_electron_fraction_abs_diff'],5.e-4,key)
            self.assertLess(abs(s['saved']['delta_xe_end']),2.e-11,key)

    def test_no_cross_source_equivalence(self):
        for _,(s,_) in self.runs.items():
            self.assertEqual(s['claimed_scope']['REI_cross_owner_physical'],'NOT_ADMITTED')
            self.assertEqual(s['claimed_scope']['production'],'HOLD')

    def test_no_new_ivp_code_path(self):
        t=(R/'code/paired_replay.py').read_text()
        self.assertNotIn('solve_ivp(',t)
        self.assertNotIn('.advance(',t)
        self.assertIn('net.evaluate(',t)

    def test_source_identity_failure_hard(self):
        old=p.PIN['s04_m00_r00']
        try:
            p.PIN['s04_m00_r00']=(old[0], '0'*64,old[2])
            with self.assertRaisesRegex(ValueError,'BASS_AUTHENTICATED_INPUT_MISMATCH'):
                p.read_one('s04_m00_r00')
        finally:
            p.PIN['s04_m00_r00']=old

    def test_phase_contributions_close_to_total_each_interval(self):
        for key,(s,rows) in self.runs.items():
            for row in rows:
                v=row['gamma_delta_chemical']+row['gamma_delta_density']+row['gamma_delta_geometry']
                self.assertLess(abs(v-row['delta_gamma']),1e-15,(key,row['index']))
            self.assertEqual(len(s['quartile_integrals']),3)

if __name__=='__main__':unittest.main(verbosity=2)
