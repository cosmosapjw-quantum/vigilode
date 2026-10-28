#!/usr/bin/env python3
"""Fail-closed tests for the stage-certificate repair handoff."""
from __future__ import annotations
import importlib.util,json,pathlib,shutil,tempfile,unittest
ROOT=pathlib.Path(__file__).resolve().parents[1]
VALIDATOR_PATH=ROOT/"tools"/"validate_audit2_stage_certificate_repair_handoff.py"
SOURCE_DIRECTORY=ROOT/"research/audit2_stage_certificate_repair_20260831"
spec=importlib.util.spec_from_file_location("stage_certificate_repair_handoff",VALIDATOR_PATH)
assert spec and spec.loader
validator=importlib.util.module_from_spec(spec); spec.loader.exec_module(validator)
class Tests(unittest.TestCase):
 def copied_root(self,temporary):
  root=pathlib.Path(temporary); destination=root/validator.DIRECTORY; destination.parent.mkdir(parents=True); shutil.copytree(SOURCE_DIRECTORY,destination)
  lock=SOURCE_DIRECTORY/"HANDOFF_INPUT_LOCK.json"
  paths=json.loads(lock.read_text())["paths"] if lock.is_file() else {"tools/validate_audit2_stage_certificate_repair_handoff.py":"","tools/test_audit2_stage_certificate_repair_handoff.py":"","tools/check-audit2-readiness.sh":"",".github/workflows/audit2-research.yml":""}
  for relative in paths:
   source=ROOT/relative; target=root/relative
   if target.exists(): continue
   target.parent.mkdir(parents=True,exist_ok=True); shutil.copy2(source,target)
  return root
 def mutate(self,root,callback):
  path=root/validator.DIRECTORY/"EXECUTION_CONTRACT.json"; value=json.loads(path.read_text()); callback(value); path.write_text(json.dumps(value))
 def test_valid(self):
  result=validator.validate(ROOT); self.assertIn(result["status"],{"STAGE_CERTIFICATE_REPAIR_HANDOFF_BOOTSTRAP_VALID","STAGE_CERTIFICATE_REPAIR_HANDOFF_FINAL_VALID"}); self.assertEqual(result["candidate_executions"],0); self.assertEqual(result["formal_role_records"],13)
 def test_candidate_tamper(self):
  with tempfile.TemporaryDirectory() as t:
   r=self.copied_root(t); self.mutate(r,lambda v:v.__setitem__("candidate_executions",1))
   with self.assertRaisesRegex(ValueError,"candidate"): validator.validate(r)
 def test_fixed_dimension_tamper(self):
  with tempfile.TemporaryDirectory() as t:
   r=self.copied_root(t); self.mutate(r,lambda v:v["repair_scope"]["formal"].__setitem__("dimension_scope","FIXED_3X3"))
   with self.assertRaisesRegex(ValueError,"dimension scope"): validator.validate(r)
 def test_role_escalation(self):
  with tempfile.TemporaryDirectory() as t:
   r=self.copied_root(t); self.mutate(r,lambda v:v["repair_scope"]["formal"]["exact_backend_role_records"][2].__setitem__("role","UNIVERSAL_PROOF_AUTHORITY"))
   with self.assertRaisesRegex(ValueError,"role matrix"): validator.validate(r)
 def test_max_arnoldi_tamper(self):
  with tempfile.TemporaryDirectory() as t:
   r=self.copied_root(t); self.mutate(r,lambda v:v["repair_scope"]["software"]["max_arnoldi"].__setitem__("independent_of_iteration_limit",False))
   with self.assertRaisesRegex(ValueError,"independence"): validator.validate(r)
 def test_review_budget_tamper(self):
  with tempfile.TemporaryDirectory() as t:
   r=self.copied_root(t); self.mutate(r,lambda v:v["compiled_plan"]["process_controls"].__setitem__("integrated_review_events_exact",2))
   with self.assertRaisesRegex(ValueError,"review count"): validator.validate(r)
 def test_identity_conflation(self):
  with tempfile.TemporaryDirectory() as t:
   r=self.copied_root(t); self.mutate(r,lambda v:v["identity_contract"]["classes"][3].__setitem__("comparators",["SHA256_EXACT"]))
   with self.assertRaisesRegex(ValueError,"typed identity"): validator.validate(r)
 def test_manifest_tamper(self):
  with tempfile.TemporaryDirectory() as t:
   r=self.copied_root(t); self.mutate(r,lambda v:v["predecessor"].__setitem__("external_manifest_sha256","0"*64))
   with self.assertRaisesRegex(ValueError,"manifest"): validator.validate(r)
 def test_raw_suffix(self):
  with tempfile.TemporaryDirectory() as t:
   r=self.copied_root(t); (r/validator.DIRECTORY/"raw.log").write_text("x")
   with self.assertRaisesRegex(ValueError,"forbidden"): validator.validate(r)
 def test_lock_tamper(self):
  if not (SOURCE_DIRECTORY/"HANDOFF_INPUT_LOCK.json").is_file(): self.skipTest("bootstrap")
  with tempfile.TemporaryDirectory() as t:
   r=self.copied_root(t); path=r/validator.DIRECTORY/"README.md"; path.write_text(path.read_text()+"\ntamper\n")
   with self.assertRaisesRegex(ValueError,"input lock"): validator.validate(r)
if __name__=="__main__": unittest.main(verbosity=2)
