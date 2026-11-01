"""Native real F05 46,080-cell event and OTS source proof-of-integrity tests.

Portable: uses committed actual original event oracle/native source data only.
Full stage is native Rust. These Python checks do NOT solve any physics ODE.
"""
from __future__ import annotations
import copy,gzip,hashlib,json,struct,sys,unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT))
from audit_rust_original_event import (
    validate_saved_receipt,EVENT_SHA,ORIGINAL_NATIVE_SHA,N,need
)

class OriginalNativeNonphotoTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.report=json.loads((ROOT/"ORIGINAL_NONPHOTO_INDEPENDENT_AUDIT.json").read_text())
        cls.native=json.loads((ROOT/"NATIVE_RELEASE_0.json").read_text())
        cls.event=gzip.decompress((ROOT/"ORIGINAL_F05_T160_NONPHOTO_EVENT_ORACLE.bin.gz").read_bytes())
        cls.raw=gzip.decompress((ROOT/"ORIGINAL_REI_F05_T160_RUST_NONPHOTO_POINT.bin.gz").read_bytes())

    def test_original_real_46k_owner_receipt(self):
        self.assertTrue(validate_saved_receipt(self.report))
        self.assertEqual(hashlib.sha256(self.event).hexdigest(),EVENT_SHA)
        self.assertEqual(hashlib.sha256(self.raw).hexdigest(),ORIGINAL_NATIVE_SHA)
        self.assertEqual(self.report["native_frozen_rate_firstorder_electron_inside_preview_interval_cells"],N)
        self.assertEqual(self.report["native_frozen_rate_firstorder_logT_inside_preview_interval_cells"],N)

    def test_real_full_25_flux_and_7_energy_interval_inclusion(self):
        e=memoryview(self.event)[16:]
        raw=memoryview(self.raw)[16:]
        self.assertEqual(len(raw),8*40*N)
        self.assertEqual(len(e),8*70*N)
        for i in range(N):
            v=struct.unpack_from("<40d",raw,i*40*8)
            for j in range(25):
                lo=struct.unpack_from("<d",e,(6*N+25*i+j)*8)[0]
                hi=struct.unpack_from("<d",e,(31*N+25*i+j)*8)[0]
                self.assertLessEqual(lo,v[15+j])
                self.assertLessEqual(v[15+j],hi)
            for j in range(7):
                lo=struct.unpack_from("<d",e,(56*N+7*i+j)*8)[0]
                hi=struct.unpack_from("<d",e,(63*N+7*i+j)*8)[0]
                self.assertLessEqual(lo,v[5+j])
                self.assertLessEqual(v[5+j],hi)

    def test_false_canonical_admission_fails_closed(self):
        fake=copy.deepcopy(self.report)
        fake["original_tick161"]="ACCEPTED"
        with self.assertRaisesRegex(AssertionError,"FALSE_CANONICAL_PROMOTION"):
            validate_saved_receipt(fake)

    def test_invalid_source_and_ledger_fails_closed(self):
        fake=copy.deepcopy(self.report)
        fake["original_event_raw_sha256"]="0"*64
        with self.assertRaisesRegex(AssertionError,"SOURCE_IDENTITY_DRIFT"):
            validate_saved_receipt(fake)
        fake=copy.deepcopy(self.report)
        fake["max_46k_ots_energy_partition_residual_relative"]=1.0
        with self.assertRaisesRegex(AssertionError,"PHYSICAL_LEDGER_FAILURE"):
            validate_saved_receipt(fake)

    def test_false_full_ivp_and_preview_claims_fail_closed(self):
        fake=copy.deepcopy(self.report)
        fake["original_14_time_map_ledgers"]="PASS"
        with self.assertRaisesRegex(AssertionError,"FAKE_COMPLETE_ORIGINAL_IVP"):
            validate_saved_receipt(fake)
        fake=copy.deepcopy(self.report)
        fake["native_frozen_rate_firstorder_electron_inside_preview_interval_cells"]=0
        with self.assertRaisesRegex(AssertionError,"PREVIEW_INCLUSION_CLAIM_NOT_OBSERVED"):
            validate_saved_receipt(fake)

if __name__=="__main__":
    unittest.main()
