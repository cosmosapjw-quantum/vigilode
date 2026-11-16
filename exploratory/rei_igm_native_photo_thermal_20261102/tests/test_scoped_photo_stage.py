"""Scoped negative tests for actual original 46k Rust photon-to-gas stage."""
import copy,gzip,hashlib,json,sys,unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT))
from audit_native_photo_thermal import validate_receipt,STAGE_RAW_SHA,PARENT_RAW_SHA,N
class NativePhotoThermalTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.audit=json.loads((ROOT/"PHOTO_THERMAL_ORIGINAL_F05_AUDIT.json").read_text())
        cls.stage=json.loads((ROOT/"NATIVE_PHOTO_THERMAL_STAGE.json").read_text())
    def test_actual_46k_stage_source_and_release_reproducibility(self):
        self.assertTrue(validate_receipt(self.audit))
        gzip_file=(ROOT/"ORIGINAL_F05_T160_RUST_PHOTO_THERMAL_STAGE.bin.gz").read_bytes()
        raw=gzip.decompress(gzip_file)
        self.assertEqual(hashlib.sha256(raw).hexdigest(),STAGE_RAW_SHA)
        self.assertEqual(raw[:8],b"REIF05D1")
        self.assertEqual(int.from_bytes(raw[8:16],"little"),N)
        self.assertEqual(len(raw),16+8*10*N)
        self.assertEqual(self.stage["accepted_original_nodes"],N)
        self.assertTrue(self.stage["photo_only_thermal_first_law_at_all_nodes"])
    def test_tampered_accepted_authority_fails_closed(self):
        x=copy.deepcopy(self.audit);x["original_tick161"]="ACCEPTED"
        with self.assertRaisesRegex(AssertionError,"FALSE_CANONICAL"):
            validate_receipt(x)
    def test_tampered_source_identity_fails_closed(self):
        x=copy.deepcopy(self.audit);x["real_original_parent_raw_sha256"]="0"*64
        with self.assertRaisesRegex(AssertionError,"SOURCE_SHA"):
            validate_receipt(x)
    def test_fake_full_ivp_and_firstorder_misreport_fails(self):
        x=copy.deepcopy(self.audit);x["original_adaptive_MPRK_SDIRK_IVP"]="PASS"
        with self.assertRaisesRegex(AssertionError,"FALSE_FULL"):
            validate_receipt(x)
        x=copy.deepcopy(self.audit);x["first_order_photo_over_absolute_preview_delta_range"]=[0.8,0.9]
        with self.assertRaisesRegex(AssertionError,"SOURCE_FRACTION_FABRICATED"):
            validate_receipt(x)
    def test_fake_physical_sign_count_fails(self):
        x=copy.deepcopy(self.audit);x["native_photo_source_and_original_net_opposite_sign_cells"]=N+1
        with self.assertRaisesRegex(AssertionError,"SIGN_COUNT"):
            validate_receipt(x)
if __name__=="__main__":
    unittest.main()
