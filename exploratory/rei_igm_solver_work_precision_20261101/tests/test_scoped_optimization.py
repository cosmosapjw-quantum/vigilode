"""Locally inspect saved REAL 46k original-owner receipts, preserving failure."""
from __future__ import annotations
import importlib.util,json,sys,unittest
from pathlib import Path
import numpy as np
ROOT=Path(__file__).resolve().parents[1]
p=ROOT/'original26_photon_lowrank_check.py'
s=importlib.util.spec_from_file_location('rei_photon_source_physical_test',p)
mod=importlib.util.module_from_spec(s);sys.modules[s.name]=mod;s.loader.exec_module(mod)

class LowRankTheoryTests(unittest.TestCase):
    def test_matched_source_jvp_nonlocal_three_sums(self):
        n=np.linspace(.2,5.,57*3).reshape(57,3)
        v=np.cos(np.arange(57*3).reshape(57,3)*.37)*n
        j=np.array([3.,7.,11.,2.]);e=np.array([.01,.02,.07,.04])
        alpha=np.array([[.1,.3,.2],[.2,.5,.3],[.01,.6,.4],[.2,.07,.1]],dtype=np.longdouble)
        exc=np.ones((3,4))
        ans=mod.hybrid_lowrank_jvp(n,v,j,e,alpha)
        ep=.0002
        plus=mod.hybrid_source(n+ep*v,j,e,alpha,exc,1.)[0]
        minus=mod.hybrid_source(n-ep*v,j,e,alpha,exc,1.)[0]
        error=float(np.max(np.abs((plus-minus)/(2*ep)-ans)))
        self.assertLess(error/max(float(np.max(np.abs(ans))),1e-15),1e-6)
    def test_photon_accounting_identity(self):
        n=np.arange(1,82,dtype=float).reshape(27,3)
        j=np.array([1.,3.,2.,1.]);e=np.array([.5,1.,.2,.1])
        a=np.array([[.1,.02,.03],[.2,.3,.01],[.3,.01,.01],[.04,.05,.06]],dtype=np.longdouble)
        f,heat,ext=mod.hybrid_source(n,j,e,a,np.ones((3,4)),1.)
        self.assertLess(abs((f.sum()+ext)-j.sum()),1e-13)
        self.assertTrue((f>=0).all() and (heat>=0).all())
    def test_missing_original_source_fails_closed(self):
        with self.assertRaises(FileNotFoundError):
            mod.compute(Path('/a/nonexistent/rei-owner'), 'LOCAL_NEUTRAL_HAZARD_PRIMARY',.0002)

class FrozenRealOwnerReceiptTests(unittest.TestCase):
    def test_original_46080_derived_hybrid_and_failure_preserved(self):
        r=json.loads((ROOT/'ORIGINAL26_PHOTON_JVP_RESULT.json').read_text())
        f=json.loads((ROOT/'ORIGINAL26_INITIAL_INTERVAL_FAIL.json').read_text())
        self.assertFalse(f['source_owner_comparison_pass'])
        self.assertEqual(r['original_26event_kernel_sha256'],mod.KERNEL_SHA)
        self.assertEqual(r['original_accepted_array_sha256'],mod.STATE_SHA)
        self.assertEqual(r['frozen_original_archive_sha256'],mod.SOURCE_SHA)
        self.assertEqual(r['hybrid_owner_interval_included_cell_species'],[46080]*3)
        self.assertEqual(r['hybrid_owner_interval_included_heat_cells'],46080)
        self.assertFalse(r['unguarded_fast_float_source_admitted'])
        self.assertTrue(r['float64_underflow_is_not_physical_zero'])
        self.assertEqual(r['heii_trace_nonzero_extended_cells'],46080)
        self.assertEqual(r['heii_trace_underflowed_to_f64_zero_cells'],46080)
        self.assertTrue(r['source_owner_comparison_pass'])
        self.assertLess(r['jvp_finite_difference_normalized_max_error'],1e-6)
        self.assertLess(r['photon_ledger_relative_residual'],1e-12)
        self.assertEqual(r['full_dh_jacobian'],'NOT_RUN')
        self.assertEqual(r['canonical_delta'],0)
        self.assertEqual(r['production'],'HOLD')
    def test_all_four_existing_native_solver_routes_source_scoped(self):
        j=json.loads((ROOT/'WORK_PRECISION.json').read_text())
        self.assertTrue(j['all_methods_source_scoped_pass'])
        self.assertEqual(len(j['results']),4)
        for r in j['results']:
            self.assertTrue(r['both_cases_pass'])
            self.assertEqual({x['data']['case'] for x in r['cases']},{'frozen','timevarying'})
            for x in r['cases']:
                self.assertEqual(x['status'],'PASS')
                self.assertLessEqual(x['data']['max_abs_native_minus_Radau'],r['reference_budget'])
                self.assertLess(x['data']['max_photon_nuclei_binding_invariant'],1e-12)
                self.assertEqual(x['data']['timing_authority'],'diagnostic_only')
        self.assertEqual(j['canonical_REI_delta'],0)
        self.assertEqual(j['owner26event_computation'],'NOT_RUN')

if __name__=='__main__':unittest.main()
