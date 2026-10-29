import sys, unittest, json, numpy as np
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'code'))
import generate_endpoint_physics as gp

class TestOriginalSourcePhysicalEndpoint(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.owner,cls.net,cls.rows,cls.live=gp.build()

    def test_original_source_identity(self):
        self.assertEqual(len(self.rows),5)
        self.assertEqual(self.owner['source_git_blob'],'a49aa5276b4256d6bf0ad443b6b830effe2ab738')
        self.assertEqual(self.net.size,21)

    def test_chemical_state_population_nonnegative(self):
        for row in self.rows: self.assertGreaterEqual(row['state_min_Y'],0.)

    def test_electron_density_and_charge(self):
        for row in self.rows:
            self.assertAlmostEqual(row['ne_m3']/row['nb_m3'],row['x_e'],delta=1e-13)
            self.assertAlmostEqual(row['x_e'],.877456451844,delta=2e-11)

    def test_total_baryon_number_normalization(self):
        for row in self.rows:self.assertAlmostEqual(row['baryon_total'],1,delta=2e-13)

    def test_photoabsorption_one_electron_in_source_model(self):
        for row in self.rows:self.assertAlmostEqual(row['photo_electron_source_m3_s'],row['photo_events_per_m3_s'],delta=2e-28)

    def test_recombination_removes_electrons(self):
        for row in self.rows:
            self.assertAlmostEqual(row['RR_electron_sink_m3_s'],-row['RR_events_per_m3_s'],delta=2e-28)
            self.assertAlmostEqual(row['DR_electron_sink_m3_s'],-row['DR_events_per_m3_s'],delta=2e-33)

    def test_electron_source_decomposition(self):
        for row in self.rows:
            got=(row['photo_electron_source_m3_s']+row['RR_electron_sink_m3_s']+
                 row['DR_electron_sink_m3_s']+row['decay_electron_source_m3_s'])
            self.assertAlmostEqual(row['chemical_dne_dt_fixed_nb_m3_s'],got,delta=3e-31)

    def test_photo_energy_is_not_heating(self):
        for row in self.rows:
            self.assertGreaterEqual(row['photo_absorbed_W_m3'],row['primary_kinetic_W_m3'])
            self.assertGreaterEqual(row['photo_absorbed_W_m3'],row['unresolved_core_W_m3'])

    def test_li6_fraction_bounded(self):
        for row in self.rows:
            self.assertGreater(row['Li6_electron_fraction_of_all'],0.)
            self.assertLess(row['Li6_electron_fraction_of_all'],1e-12)

    def test_tau_gradient_uses_stored_bianchi_H(self):
        for row in self.rows:self.assertAlmostEqual(row['dtau_dx_recomputed'],row['dtau_dx_stored'],delta=4e-17)

    def test_dynamics_not_falsely_identical_between_cases(self):
        self.assertGreater(self.rows[0]['tau_T_stored']-self.rows[3]['tau_T_stored'],1e-4)
        self.assertLess(abs(self.rows[0]['x_e']-self.rows[3]['x_e']),1e-11)

    def test_actual_source_jacobian_includes_electron_feedback(self):
        # Finite exact quadratic polarization identity, evaluated at real saved state.
        for name in ('s00_m00_r00','s04_m04_r04'):
            d=self.live[name];Y=d['Y'];J=d['J']; rates=d['rates']
            nb=next(x for x in self.rows if x['case_id']==name)['nb_m3']
            qq=self.net.q;R=rates['Gne']; v=np.zeros(21);v[2]=1.;v[12]=-.25
            actual=nb*np.outer(R@Y,qq)@v
            self.assertGreater(float(np.linalg.norm(actual)),0.)
            self.assertLess(float(np.linalg.norm((J-(rates['G0']+rates['D']+nb*(qq@Y)*R))@v-actual)),2e-28)

    def test_original_F_equals_all_named_channels(self):
        for d in self.live.values():
            r=d['rates'];Y=d['Y'];F=d['F'];row=next(x for x in self.rows if x['case_id'] == next(k for k,v in self.live.items() if v is d))
            nb=row['nb_m3'];ne=nb*(self.net.q@Y)
            self.assertLess(float(np.linalg.norm(F-((r['G0']+r['D'])@Y+ne*r['Gne']@Y))),2e-29)

if __name__=='__main__':unittest.main()
