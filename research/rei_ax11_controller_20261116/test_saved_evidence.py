#!/usr/bin/env python3
"""Hostile tests on actual saved native comparisons, no IVP reruns."""
import contextlib,csv,io,json,os,shutil,tempfile,unittest
from pathlib import Path
from audit_probe import audit
ROOT=Path(os.environ['AX11_PROBE_EVIDENCE'])
UNIT=Path(__file__).resolve().parent

class EvidenceTests(unittest.TestCase):
 def check_mutation(self,change,pattern):
  with tempfile.TemporaryDirectory(prefix='ax11_hostile_',dir=ROOT) as tmp:
   d=Path(tmp)
   shutil.copytree(ROOT/'inputs',d/'inputs')
   shutil.copytree(ROOT/'diagnostic',d/'diagnostic')
   (d/'receipts').symlink_to(ROOT/'receipts',target_is_directory=True)
   shutil.copyfile(ROOT/'NATIVE_LAUNCH_MANIFEST.json',d/'NATIVE_LAUNCH_MANIFEST.json')
   p=d/'diagnostic/ERROR_COMPONENTS.csv'
   with p.open() as f:rows=list(csv.DictReader(f))
   change(rows,d)
   with p.open('w',newline='') as f:
    w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
   with contextlib.redirect_stdout(io.StringIO()):
    with self.assertRaisesRegex(ValueError,pattern):audit(d,UNIT/'PROFILE_ALLOCATION.json',d/'bad_result.json')
 def test_wrong_photon_error_is_rejected(self):
  self.check_mutation(lambda rows,d:rows[0].update(photon_energy_L1='0.0'),'COMPONENT_BITS_MISMATCH')
 def test_wrong_controlling_component_is_rejected(self):
  self.check_mutation(lambda rows,d:rows[0].update(winner='thermal_w'),'WRONG_DOMINANT_COMPONENT')
 def test_missing_comparison_is_rejected(self):
  self.check_mutation(lambda rows,d:rows.pop(),'PROBE_CENSUS')
 def test_forged_next_step_is_rejected(self):
  self.check_mutation(lambda rows,d:rows[0].update(native_next_dln_a='0.0001'),'NEXT_STEP_MISMATCH')
 def test_nonfinite_metric_is_rejected(self):
  self.check_mutation(lambda rows,d:rows[0].update(HII='nan'),'COMPONENT_BITS_MISMATCH')
 def test_changed_original_checkpoint_is_rejected(self):
  def alter(rows,d):
   p=d/'inputs/ZERO/checkpoint.txt';p.write_text(p.read_text()+'\n')
  self.check_mutation(alter,'SOURCE_INPUT_SHA_CHANGED')

if __name__=='__main__':unittest.main()
