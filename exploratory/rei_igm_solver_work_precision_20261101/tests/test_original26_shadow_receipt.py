"""Verify preserved original-owner and opt-in native physical source receipts.

These tests validate immutable recorded artifacts; they do NOT rerun the 26-event
physics or turn a shadow pass into accepted REI production history.
"""
import json
import unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]


def read(name):
    return json.loads((ROOT/name).read_text())


class OriginalOwnerResearchReceipts(unittest.TestCase):
    def test_original_26event_real_full_half_half_profile(self):
        r=read("ORIGINAL26_OWNER_MAP_PROFILE.json")
        self.assertEqual(r["status"],"SCOPED_PASS")
        self.assertEqual(r["nodes"],46080)
        self.assertEqual(r["original_14_ledger"]["evaluated"],14)
        self.assertTrue(r["original_14_ledger"]["passed"])
        self.assertEqual({k:v["native_certified"] for k,v in r["map_timings"].items()},
                         {"full":True,"half1":True,"half2":True})
        self.assertTrue(r["original_historical_failure_retained"])
        self.assertGreater(r["historical_independent_output_logT_upper"],2e-4)
        self.assertEqual(r["original_owner_admission"],"NOT_RUN")

    def test_shadow_is_exact_output_and_lowers_real_root_work(self):
        r=read("ORIGINAL26_ROOT_ABBA.json")
        self.assertEqual(r["status"],"SCOPED_SHADOW_PASS")
        self.assertTrue(r["strict_shadow_pass"])
        self.assertTrue(r["all_six_original_state_arrays_byte_identical"])
        self.assertEqual(len({x["state_bytes_sha256"] for x in r["trials"].values()}),1)
        self.assertEqual(r["source_average_rhs_calls_original"],90)
        self.assertEqual(r["source_average_rhs_calls_shadow"],72)
        self.assertEqual(set(r["trials"]),{"A0_original","B1_shadow","B2_shadow","A3_original"})
        for arm in r["trials"].values():
            self.assertEqual(arm["original_14_ledger_count"],14)
            self.assertTrue(arm["original_14_ledger_pass"])
            self.assertGreater(arm["historical_crossbox_logT"],2e-4)
        self.assertEqual(r["canonical_delta"],0)
        self.assertEqual(r["production"],"HOLD")

    def test_pr181_source_corrected_native_replay_not_full_owner(self):
        r=read("NATIVE_PR181_NONAUTON_REPLAY.json")
        self.assertEqual(r["source_scoped_successful_routes"],8)
        self.assertEqual(r["preregistered_candidate_count"],8)
        self.assertEqual(r["posthoc_adversarial_control_count"],1)
        self.assertEqual(r["attempted_routes"],9)
        self.assertEqual(r["original_26event"],"NOT_RUN")
        self.assertEqual(r["original_tick161"],"NOT_ACCEPTED")
        self.assertEqual(r["production"],"HOLD")
        self.assertEqual(r["canonical_delta"],0)
        bad=[x for x in r["runs"] if x["posthoc_adversarial_negative_control"]]
        self.assertEqual(len(bad),1)
        self.assertFalse(bad[0]["scientific_declaration_valid"])


if __name__=="__main__":
    unittest.main()
