"""Bounded hostile tests of actual Rust source evidence and authorization."""
from __future__ import annotations
import copy
import json
import sys
import unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT))
from audit_native_original_f05 import validate


class NativeAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.meta=json.loads((ROOT/"ORIGINAL_F05_SOURCE_FIXTURE.json").read_text())
        cls.native=json.loads((ROOT/"NATIVE_RELEASE_0.json").read_text())
        cls.fixture=(ROOT/"ORIGINAL_REI_F05_T160_PHOTON_SOURCE.bin.gz").read_bytes()

    def test_actual_source_pass_only_with_explicit_scope(self):
        out=validate(self.meta,[self.native],self.fixture)
        self.assertEqual(out["original_source_interval_inclusion"],
                         "ALL_THREE_SPECIES_AND_HEAT_46080_PASS")
        self.assertEqual(out["original_26event_full_ivp_speedup"],"NOT_ESTABLISHED")
        self.assertEqual(out["native_original_14_time_map_ledgers"],"NOT_RUN")
        self.assertTrue(out["original_heii_source_positive_46080_cells"])

    def test_tampered_owner_source_sha_rejected(self):
        altered=copy.deepcopy(self.meta)
        altered["original_26event_kernel_sha256"]="0"*64
        with self.assertRaisesRegex(AssertionError,"KERNEL_DRIFT"):
            validate(altered,[self.native],self.fixture)

    def test_tampered_native_owner_interval_rejected(self):
        altered=copy.deepcopy(self.native)
        altered["source_owner_interval_HI_HeI_HeII_inside"]=[46080,46080,0]
        with self.assertRaisesRegex(AssertionError,"OWNER_SOURCE_INTERVAL_FAIL"):
            validate(self.meta,[altered],self.fixture)

    def test_fake_heii_zero_and_full_ivp_rejected(self):
        altered=copy.deepcopy(self.native)
        altered["no_claim_exact_zero_heii"]=False
        with self.assertRaisesRegex(AssertionError,"HEII_UNDERFLOW_FALSE_ZERO"):
            validate(self.meta,[altered],self.fixture)
        altered=copy.deepcopy(self.native)
        altered["original_complete_26event_ode"]="PASS"
        with self.assertRaisesRegex(AssertionError,"FAKE_FULL_SOURCE_OWNER_SOLVER"):
            validate(self.meta,[altered],self.fixture)

    def test_bad_rust_W_residual_and_speedup_fabrication_rejected(self):
        altered=copy.deepcopy(self.native)
        altered["native_existing_LowRankBorderedPreconditioner_W_residual_linf"]=1.0
        with self.assertRaisesRegex(AssertionError,"ORIGINAL_NATIVE_W_RESIDUAL_FAIL"):
            validate(self.meta,[altered],self.fixture)
        altered=copy.deepcopy(self.native)
        altered["native_repeated_jvp_speedup_excluding_setup"]=100.0
        with self.assertRaisesRegex(AssertionError,"JVP_SPEEDUP_FABRICATED"):
            validate(self.meta,[altered],self.fixture)

    def test_bad_fixture_bytes_rejected(self):
        bad=bytearray(self.fixture)
        bad[-1]^=1
        with self.assertRaisesRegex(AssertionError,"FIXTURE_GZIP_SHA_DRIFT"):
            validate(self.meta,[self.native],bytes(bad))


if __name__=="__main__":
    unittest.main()
