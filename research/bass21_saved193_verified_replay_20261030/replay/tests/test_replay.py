"""Finite owner/source contract tests, no new IVP and no claim of independent R6/C9."""
import hashlib
import json
import math
import sys
import unittest
from pathlib import Path
import numpy as np

R=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(R/'code'))
import replay

class TestActualOriginalSavedHistory(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.result,cls.rows,cls.arr=replay.compute()
        with np.load(R.parent/'original'/'trajectory.npz',allow_pickle=False) as data:
            cls.saved={k:data[k] for k in data.files}

    def test_01_exact_immutable_source_size_and_sha(self):
        p=R.parent/'original'/'trajectory.npz'
        self.assertEqual(p.stat().st_size,254301)
        self.assertEqual(hashlib.sha256(p.read_bytes()).hexdigest(),replay.SOURCE_SHA256)

    def test_02_exact_prior_PR118_archive(self):
        self.assertEqual(replay.sha256(R.parent/'original'/'PR118_SOURCE_FULL.zip'),replay.PARENT_SHA256)

    def test_03_native_original_network_byte_lock(self):
        self.assertEqual(self.result['source']['original_BASS_network_sha256'],'baecce07aded483fa60006cc576feff67d068c0d70394f7095abacc6d47eaa80')

    def test_04_all_193_sample_and_matrix_shape(self):
        self.assertEqual(len(self.rows),193)
        self.assertEqual(self.arr['F_proper_s'].shape,(193,21))
        self.assertEqual(self.arr['J_proper_s'].shape,(193,21,21))
        self.assertEqual(self.result['counts']['new_IVPs'],0)

    def test_05_clock_x_z_and_monotonicity(self):
        x=self.saved['x'];z=self.saved['z'];self.assertEqual(x[0],0.)
        self.assertLess(np.max(abs(z-(21*np.exp(-x)-1))),2e-13)
        self.assertTrue(np.all(np.diff(x)>0))
        self.assertTrue(np.all(np.diff(z)<0))

    def test_06_primitive_Y_matches_62_state_exactly(self):
        self.assertEqual(self.result['source_invariants']['max_abs_Y_raw62_scales'],0.)
        self.assertEqual(float(np.min(self.saved['Y'])),0.)

    def test_07_source_exact_nuclear_charges_and_mass(self):
        self.assertLess(self.result['source_invariants']['max_baryon_defect'],1e-11)
        self.assertLess(self.result['source_invariants']['max_xe_diff'],2e-12)
        self.assertGreaterEqual(self.result['source_invariants']['min_Y'],0.)

    def test_08_same_owner_ne_rate_not_mixed(self):
        self.assertLess(self.result['source_invariants']['max_ne_diff_m3'],1e-11)
        self.assertEqual(self.result['source']['commit'],replay.BASS_COMMIT)
        self.assertFalse(self.result['model_limits']['REI17_P19_as_same_owner'])

    def test_09_native_F_against_original_channel_sum(self):
        self.assertLess(self.result['source_invariants']['max_F_defect_per_s'],5e-26)
        self.assertLess(self.result['source_invariants']['max_e_source_decomposition_m3_s'],1e-24)

    def test_10_native_J_includes_rank_one_feedback(self):
        self.assertEqual(self.result['source_invariants']['max_J_defect_per_s'],0.)
        self.assertGreater(np.max(np.abs(self.arr['J_proper_s'])),0.)

    def test_11_native_baryon_charge_source(self):
        self.assertLess(self.result['source_invariants']['max_A_F_per_s'],1e-26)
        # Native binary64 matrix accumulation is not an exact symbolic zero.
        self.assertLess(self.result['source_invariants']['max_A_J_per_s'],1e-20)

    def test_12_photo_charge_and_electron_source_decomposition(self):
        self.assertLess(self.result['source_invariants']['max_photo_event_charge_m3_s'],1e-24)
        self.assertLess(self.result['source_invariants']['max_e_source_decomposition_m3_s'],1e-24)

    def test_13_saved_absorption_and_energy_provenance(self):
        for k,v in self.result['source_invariants']['max_direct_native_energetic_diagnostic_diff'].items():
            self.assertEqual(v,0.,k)

    def test_14_native_opacity_matches_original_geometry_and_ne(self):
        self.assertLess(self.result['tau']['native_gamma_max_rel_diff'],2e-12)
        self.assertTrue(np.all(self.arr['tau_x']>0.))

    def test_15_stored_tau_boundary_and_monotonic(self):
        self.assertEqual(float(self.saved['diagnostics'][0,8]),0.)
        self.assertEqual(self.result['tau']['stored_final'],replay.ORIGINAL_TAU_FINAL)
        self.assertGreater(self.result['tau']['min_stored_delta_tau'],0.)

    def test_16_trap_simpson_differences_are_recorded_not_promoted(self):
        self.assertNotEqual(self.result['tau']['trap_endpoint_signed_error'],0.)
        self.assertNotEqual(self.result['tau']['simpson_endpoint_signed_error'],0.)
        self.assertTrue(math.isfinite(self.result['tau']['simpson_max_abs_error_all_epochs']))
        self.assertGreater(self.result['tau']['simpson_max_abs_error_all_epochs'],0.)

    def test_17_elapsed_proper_time_nonzero_and_replay(self):
        self.assertGreater(self.result['proper_time']['stored_elapsed_final_s'],0.)
        self.assertTrue(math.isfinite(self.result['proper_time']['simpson_end_rel_err']))

    def test_18_nontrivial_source_ionization_and_recombination(self):
        self.assertGreater(self.result['chemistry']['xe_end'],self.result['chemistry']['xe_start'])
        self.assertGreater(self.result['chemistry']['proper_photo_event_exposure_per_baryon'],0.)
        self.assertGreater(self.result['chemistry']['proper_RR_event_exposure_per_baryon'],0.)
        self.assertGreater(self.rows[-1]['Gamma_Li6I_s'],self.rows[0]['Gamma_Li6I_s'])

    def test_19_no_new_ivp_no_production_no_rodas(self):
        self.assertEqual(self.result['counts']['new_IVPs'],0)
        self.assertEqual(self.result['model_limits']['production'],'HOLD')
        self.assertEqual(self.result['model_limits']['native_RODAS_transaction'],'NOT_RUN')

    def test_20_independent_reviewer_not_done(self):
        self.assertEqual(self.result['model_limits']['independent_R6_C9'],'NOT_RUN')
        self.assertFalse(self.result['model_limits']['photo_absorbed_power_is_thermal_heating'])

if __name__=='__main__':unittest.main(verbosity=2)
