"""Lightweight Git-mirror-only checks. NOT full source replay of archived BASS21."""
import csv,hashlib,json,math,os,pathlib,unittest
ROOT=pathlib.Path(__file__).resolve().parents[1]
class GitScopedPhysicsReceipt(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data=json.loads((ROOT/'results/SCIENCE_SUMMARY.json').read_text())
        with (ROOT/'results/ENDPOINT_50_E2_SPIN2.csv').open(newline='',encoding='utf-8') as f:
            cls.ends=list(csv.DictReader(f))
        with (ROOT/'results/PAIRED_25_E2_SPIN2.csv').open(newline='',encoding='utf-8') as f:
            cls.pairs=list(csv.DictReader(f))
    def test_exact_histories_and_rows(self):
        self.assertEqual(len(self.ends),50);self.assertEqual(len(self.pairs),25)
        self.assertEqual(self.data['new_spin2_mode_entries'],50*193*5)
        self.assertEqual(self.data['paired_mode_entries'],25*193*5)
        self.assertEqual(self.data['new_ivps'],0)
    def test_source_identity(self):
        self.assertEqual(self.data['original_source_sha256'],'6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8')
        self.assertEqual(self.data['parent_archive_sha256'],'ffd7623ba2e2bd99233a1244ac7945470f6eeed348a63a4fd02f5967ad449b14')
    def test_collision_crosswalk_bound(self):
        self.assertLess(self.data['max_j2feedback_crosswalk'],1e-15)
        self.assertLess(self.data['max_projection_E2_gap'],1e-14)
        self.assertLess(self.data['max_C_crosswalk_gap'],1e-14)
    def test_scalar_modes_physics(self):
        self.assertAlmostEqual(sum(float(r['E2_final']) for r in self.ends)/50,self.data['mean_E2_end'],delta=1e-14)
        self.assertAlmostEqual(sum(float(r['c4_final']) for r in self.ends)/50,self.data['mean_c4_end'],delta=1e-14)
    def test_paired_data(self):
        self.assertGreater(min(float(r['paired_E2_final']) for r in self.pairs),0)
        self.assertAlmostEqual(sum(float(r['paired_E2_final']) for r in self.pairs)/25,self.data['mean_paired_E2_end'],delta=1e-14)
    def test_full_source_not_claimed(self):
        self.assertEqual(self.data['production'],'HOLD')
        self.assertEqual(self.data['distinct_author_R6'],'NOT_RUN')
        self.assertEqual(self.data['distinct_author_C9'],'NOT_RUN')
        self.assertEqual(self.data['dag']['official_delta'],0)
    def test_shear_transport_not_standalone(self):
        self.assertGreater(self.data['s04_max_c4'],self.data['s00_max_c4'])
        self.assertLess(self.data['max_rate_crosswalk_gap'],1e-15)
    def test_result_file_hashes(self):
        filemap=json.loads((ROOT/'GIT_MANIFEST.json').read_text())['files_sha256']
        for path,digest in filemap.items():
            p=ROOT/path
            self.assertTrue(p.is_file(),path)
            self.assertEqual(hashlib.sha256(p.read_bytes()).hexdigest(),digest,path)
if __name__=='__main__':unittest.main()
