"""Finite self-hostile, provenance-bounded 3-lane original F05 audit tests."""
import copy
import json
import sys
import unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT))
import analyze_original26_three_lane as verifier


def original():
    return {k:json.loads((ROOT/f"ORIGINAL26_ROOT_ABBA_{k}_3LANE.json").read_text())
            for k in verifier.LANES}


class ThreeLaneOwnerAudit(unittest.TestCase):
    def test_three_original_physical_lanes_read(self):
        r=verifier.audit(original())
        self.assertEqual(r["full_half_half_trials"],12)
        self.assertEqual(r["actual_original_native_maps"],36)
        self.assertTrue(r["all_original_26event_output_bytes_identical"])
        self.assertTrue(r["all_lane_speed_ratios_above_one"])
        self.assertEqual(r["independent_astrophysical_realizations"],1)
        self.assertEqual(r["official_tick161"],"NOT_ACCEPTED")
        self.assertEqual(r["canonical_delta"],0)

    def test_tampered_accepted_state_sha_fails_closed(self):
        r=original()
        r["RECOMB"]["parent_sha256"]="0"*64
        with self.assertRaisesRegex(AssertionError,"ORIGINAL_LOCK_FAIL"):
            verifier.audit(r)

    def test_tampered_native_ledger_is_rejected(self):
        r=original()
        r["SHIELD"]["trials"]["B1_shadow"]["original_14_ledger_pass"]=False
        with self.assertRaisesRegex(AssertionError,"PHOTON_OTS_HHE_ENERGY_LEDGER"):
            verifier.audit(r)

    def test_tampered_crossbox_failure_is_rejected(self):
        r=original()
        r["PRIMARY"]["trials"]["A0_original"]["historical_crossbox_logT"]=1e-5
        with self.assertRaisesRegex(AssertionError,"HISTORIC_V1_GATE"):
            verifier.audit(r)

    def test_tampered_method_and_same_output_fails(self):
        r=original()
        r["PRIMARY"]["trials"]["B2_shadow"]["state_bytes_sha256"]="f"*64
        with self.assertRaisesRegex(AssertionError,"SOURCE_ARRAYS_NOT_EXACT"):
            verifier.audit(r)

    def test_tampered_performance_and_missing_lane_fails(self):
        r=original()
        r["SHIELD"]["diagnostic_wall_speedup"]=2.0
        with self.assertRaisesRegex(AssertionError,"WALL_SPEEDUP_MISSTATED"):
            verifier.audit(r)
        del r["SHIELD"]
        with self.assertRaisesRegex(AssertionError,"MISSING_OR_EXTRA_REAL_LANES"):
            verifier.audit(r)


if __name__=="__main__":
    unittest.main()
