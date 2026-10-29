"""Focused source-locked tests; does not solve any BASS initial-value problem."""
from __future__ import annotations
import io,json,unittest,zipfile,sys
from fractions import Fraction
from pathlib import Path
import numpy as np
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'code'))
import endpoint_atlas as ea

class OriginalEndpointAudit(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grid=ea.read_grid()
        cls.raw,cls.pairs,cls.curvs,cls.F,cls.J,cls.Y,cls.summary=ea.compute()
        cls.net,_=ea.get_network()
        cls.rmap={v['case_id']:v for v in cls.raw}

    def test_original_byte_and_git_identity(self):
        self.assertTrue(ea.locked_inputs())
        self.assertEqual(ea.git_blob(ea.ORIGINAL_CSV.read_bytes()),'a49aa5276b4256d6bf0ad443b6b830effe2ab738')

    def test_grid_125_plus_two_original_controls(self):
        self.assertEqual(len(self.grid),125)
        self.assertEqual(len({(s,m,r) for s,m,r,_ in self.grid}),125)
        self.assertEqual(len(ea.ORIGINAL_CSV.read_text().splitlines())-1,127)

    def test_f_j_shapes_and_finiteness(self):
        self.assertEqual(self.F.shape,(125,21))
        self.assertEqual(self.J.shape,(125,21,21))
        self.assertTrue(np.isfinite(self.J).all())
        self.assertTrue(np.isfinite(self.F).all())

    def test_native_electrons_and_baryon_inventory(self):
        self.assertLess(max(abs(x['source_ne_saved_residual']) for x in self.raw),1e-13)
        self.assertLess(max(abs(x['mass_sum']-1) for x in self.raw),1e-12)
        self.assertLess(max(abs(x['charge_xe_ne_discrepancy']) for x in self.raw),1e-14)

    def test_native_source_charge_balance(self):
        self.assertLess(max(abs(x['source_charge_balance_residual']) for x in self.raw),1e-28)
        self.assertLess(max(abs(x['A_dot_F_s_inv']) for x in self.raw),1e-27)

    def test_analytic_jacobian_independent_formula(self):
        # This reconstructs J from the original *rates* and explicit mass-action law.
        # It is not an independent rate-coefficient provider.
        for i in [0,8,24,50,72,99,124]:
            row=self.raw[i];Y=self.Y[i]
            r=self.net.rates(row['proper_T_K'],row['original_uv_scale'])
            M=r['G0']+r['D']; B=r['Gne']; nb=row['n_b_m3']; ne=row['n_e_m3_native']
            f=(M+ne*B)@Y
            jac=M+ne*B+nb*np.outer(B@Y,self.net.q)
            self.assertLess(float(np.max(np.abs(self.F[i]-f))),1e-29)
            self.assertLess(float(np.max(np.abs(self.J[i]-jac))),1e-26)

    def test_mean_clock_and_gas_bath_same_grid(self):
        for x in self.raw:
            self.assertAlmostEqual(x['z'],2.0,places=12)
            self.assertAlmostEqual(x['x'],np.log(7.0),places=12)
            self.assertEqual(x['proper_T_K'],11999.955652992518)
            self.assertEqual(x['original_uv_scale'],0.9999962733607158)
        self.assertEqual(len(set(v['n_b_m3'] for v in self.raw)),1)

    def test_25_paired_tau_shift_bounds_and_previous_scope(self):
        self.assertEqual(len(self.pairs),25)
        self.assertTrue(all(p['stored_delta_tau']<0 for p in self.pairs))
        self.assertAlmostEqual(np.mean([p['stored_delta_tau'] for p in self.pairs]),-0.00012407565482797477,places=14)
        # PR118 group readback is independently stored in prior immutable archive.
        self.assertAlmostEqual(np.mean([p['relative_delta_tau'] for p in self.pairs]),-0.0017893609445001236,places=12)

    def test_all_25_sampled_secant_slopes_decreasing(self):
        self.assertEqual(sum(all(np.diff(t['secant_slopes'])<0) for t in self.curvs),25)
        self.assertEqual(sum(t['has_positive_0_to_0p003_slope'] for t in self.curvs),7)

    def test_all_25_two_factor_exact_fraction_identity(self):
        for p in self.pairs:
            a=self.rmap[p['source_comp']['A']];b=self.rmap[p['source_comp']['B']]
            na=Fraction(str(a['n_e_m3_saved']));nb=Fraction(str(b['n_e_m3_saved']))
            ga=Fraction(str(a['gamma_T_per_H_saved']));gb=Fraction(str(b['gamma_T_per_H_saved']))
            R1=ga/na;R2=gb/nb
            self.assertEqual((gb-ga), (nb-na)*(R1+R2)/2+(nb+na)*(R2-R1)/2)

    def test_all_rate_splits_float_accurate(self):
        self.assertLess(max(p['max_abs_endpoint_rate_split_residual'] for p in self.pairs),5e-18)
        self.assertLess(max(abs(p['endpoint_rate_electron']/p['rate_delta']) for p in self.pairs),2e-8)
        self.assertEqual(set(p['endpoint_rate_baryon_density'] for p in self.pairs),{0.0})

    def test_original_five_pair_history_vs_endpoint(self):
        # Distinctly derived BASS historical data, not rerunning IVPs.
        with zipfile.ZipFile(ea.ORIGINAL_ZIP) as z:
            prior=json.loads(z.read('results/FIVE_PAIRS_SUMMARY.json'))
            with zipfile.ZipFile(io.BytesIO(z.read('source/BASS21_PR121_PARENT_FULL.zip'))) as q:
                old0=q.read('original/trajectory.npz')
                with np.load(io.BytesIO(old0),allow_pickle=False) as f:
                    self.assertAlmostEqual(f['diagnostics'][-1][list(f['diagnostic_names']).index('tau_T')],self.rmap['s00_m00_r00']['tau_T_saved'],places=14)
        for p in prior['pairs']:
            found=next(x for x in self.pairs if x['pair']==p['pair'])
            self.assertLess(abs(found['stored_delta_tau']-p['stored_delta_tau']),5e-16)

    def test_no_native_integration_path_and_gates(self):
        code=(ROOT/'code'/'endpoint_atlas.py').read_text()
        for forbidden in ('solve_ivp(', 'odeint(', 'RK45(', 'default_dispatcher('):self.assertNotIn(forbidden,code)
        self.assertEqual(self.summary['DAG']['original_VigilODE'],'7/23')
        self.assertEqual(self.summary['claims']['production'],'HOLD')
        self.assertEqual(self.summary['counts']['new_IVPs'],0)

    def test_mixed_interaction_25_grid_is_numeric_not_zero(self):
        m=self.summary['physical']['max_abs_m_r_mixed_interaction_tau']
        self.assertGreater(m,1e-12)
        self.assertLess(m,2e-10)

if __name__=='__main__':unittest.main()
