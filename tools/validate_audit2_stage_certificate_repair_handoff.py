#!/usr/bin/env python3
"""Validate the successor repair handoff without running science."""
from __future__ import annotations
import hashlib, json, pathlib, sys
DIRECTORY=pathlib.Path("research/audit2_stage_certificate_repair_20260831")
BASE_REQUIRED={"CLAIM_LEDGER.md","CODEX_START_HERE.md","EXECUTION_CONTRACT.json","FORMAL_SCOPE.md","README.md","handoff.json"}
CEILING="EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE"
EXPECTED_FAILURES={"F01_QUANTIFIER_SCOPE_INSUFFICIENT","F03_QUANTIFIER_SCOPE_INSUFFICIENT","F04_QUANTIFIER_SCOPE_INSUFFICIENT","TRACE_MAX_ARNOLDI_UNENFORCED","RECEIPT_SERIALIZATION_ROUNDTRIP_UNTESTED","DIRECTED_ROUNDING_ZERO_OVERFLOW_UNTESTED"}
EXPECTED_RECORDS={
 ("F01","lean-mathlib","UNIVERSAL_PROOF_AUTHORITY"),
 ("F01","rocq","UNIVERSAL_PROOF_AUTHORITY"),
 ("F02","wolfram-language","EXACT_DECLARED_OPERATOR_CROSSCHECK"),
 ("F02","sagemath","EXACT_DECLARED_OPERATOR_CROSSCHECK"),
 ("F02","singular","NUMERATOR_PATTERN_ONLY"),
 ("F03","lean-mathlib","UNIVERSAL_PROOF_AUTHORITY"),
 ("F03","rocq","UNIVERSAL_PROOF_AUTHORITY"),
 ("F03","wolfram-language","SYMBOLIC_SCALAR_IMPLICATION_CROSSCHECK"),
 ("F04","lean-mathlib","UNIVERSAL_PROOF_AUTHORITY"),
 ("F04","rocq","UNIVERSAL_PROOF_AUTHORITY"),
 ("F04","sagemath","GENERATED_DIMENSION_EXACT_CROSSCHECK"),
 ("F05","lean-mathlib","UNIVERSAL_PROOF_AUTHORITY"),
 ("F05","rocq","UNIVERSAL_PROOF_AUTHORITY"),
}
EXPECTED_POLICIES={"Universal Anti-Meta-Loop and Progress-First.md","Universal Audit-Compiled Execution Plan -.md","Universal Byte Identity vs Semantic Identity.md","Anti-Stall, Durable Checkpoint, and.md"}
EXPECTED_PHASES=["R0","R1","R2","R3","R4","R5"]
PROMPT_MARKERS={"USER_AUTHORIZED_SUCCESSOR_OBSERVED_DEFECT_CLOSURE","LATEST_VALID_DURABLE_CHECKPOINT","F01_QUANTIFIER_SCOPE_INSUFFICIENT","TRACE_MAX_ARNOLDI_UNENFORCED","TOTAL_ARNOLDI_VECTOR_CAP","integrated_closeout_review.json","PROCESS_DRIFT_DETECTED","STALL_PROCESS_ACCRETION","NOT_OPENED_OR_EXECUTED"}
FORBIDDEN_SUFFIXES=(".log",".npy",".npz",".csv",".parquet",".h5",".hdf5",".zip",".tar",".gz",".xz",".olean",".vo",".glob",".aux",".pyc",".o",".a",".so",".db",".sqlite")
def _load(path):
 value=json.loads(path.read_text(encoding="utf-8"))
 if not isinstance(value,dict): raise ValueError(f"{path} must contain a JSON object")
 return value
def _sha256(path):
 d=hashlib.sha256()
 with path.open("rb") as source:
  for block in iter(lambda:source.read(1024*1024),b""): d.update(block)
 return d.hexdigest()
def validate(root):
 research=root/DIRECTORY
 if not research.is_dir(): raise ValueError(f"missing handoff directory: {DIRECTORY}")
 files={p.relative_to(research).as_posix() for p in research.rglob("*") if p.is_file()}
 missing=sorted(BASE_REQUIRED-files)
 if missing: raise ValueError(f"missing required handoff files: {', '.join(missing)}")
 c=_load(research/"EXECUTION_CONTRACT.json"); h=_load(research/"handoff.json")
 a=c.get("authority",{}); p=c.get("predecessor",{}); plan=c.get("compiled_plan",{}); pc=plan.get("process_controls",{})
 f=c.get("repair_scope",{}).get("formal",{}); s=c.get("repair_scope",{}).get("software",{})
 if c.get("schema")!="vigilode-audit2-stage-certificate-repair-contract/v1": raise ValueError("repair contract schema mismatch")
 if a.get("repository")!="cosmosapjw-quantum/vigilode" or a.get("predecessor_pr")!=41: raise ValueError("repository or predecessor PR mismatch")
 if a.get("predecessor_head")!="9fbdd84c64e99620805ebf634dcaf57aaad05cbc" or a.get("predecessor_tree")!="12a5bb79f94f2cb47d4f808f7254e01af0446cdb": raise ValueError("predecessor identity mismatch")
 if a.get("successor_branch")!="research/audit2-stage-certificate-repair-handoff-20260831": raise ValueError("successor branch mismatch")
 if a.get("executor")!="LOCAL_CODEX_JOB_ONLY" or a.get("local_llm_allowed") is not False: raise ValueError("executor mismatch")
 if a.get("merge_tag_release_authorized") is not False: raise ValueError("merge tag release must remain forbidden")
 if p.get("terminal_disposition")!="STOP_INVALID" or p.get("repair_closeout_round_exhausted") is not True: raise ValueError("predecessor disposition or repair budget mismatch")
 if p.get("external_manifest_sha256")!="3812075cc7457f1797d512fde55b49b3202053d257e8a350a6cf48c2cea5029f": raise ValueError("predecessor manifest mismatch")
 if p.get("sha256sums_sha256")!="057a1f5ccafc5889684caa72af0cd716edc3e04ec0f58b2b771a3971baeeb3d8": raise ValueError("predecessor SHA256SUMS mismatch")
 if p.get("candidate_executions")!=0 or p.get("holdout_access")!="NOT_OPENED_OR_EXECUTED": raise ValueError("candidate or holdout boundary mismatch")
 if plan.get("mode")!="USER_AUTHORIZED_SUCCESSOR_OBSERVED_DEFECT_CLOSURE" or plan.get("resume_point")!="LATEST_VALID_DURABLE_CHECKPOINT": raise ValueError("successor mode or durable resume mismatch")
 phases=plan.get("phases")
 if not isinstance(phases,list) or [x.get("phase_id") for x in phases]!=EXPECTED_PHASES: raise ValueError("compiled phase order mismatch")
 if any(x.get("major_objective_count")!=1 or x.get("verification_bundle_count")!=1 for x in phases): raise ValueError("phase cardinality mismatch")
 if pc.get("max_diagnostic_retries_total")!=1: raise ValueError("diagnostic retry budget mismatch")
 if pc.get("integrated_review_events_exact")!=1: raise ValueError("integrated review count mismatch")
 if pc.get("repair_after_integrated_review")!=0 or pc.get("recursive_review") is not False: raise ValueError("post-review repair or recursive review forbidden")
 if pc.get("guard_policy")!="OBSERVED_FAILURE_CLASSES_ONLY" or set(plan.get("allowed_failure_classes",[]))!=EXPECTED_FAILURES: raise ValueError("observed failure class set mismatch")
 classes={x.get("class"):tuple(x.get("comparators",[])) for x in c.get("identity_contract",{}).get("classes",[])}
 if classes!={"BYTE":("SHA256_EXACT",),"CONTENT":("CANONICAL_JSON_EQUAL",),"NUMERICAL":("BITWISE_BINARY64","DIRECTED_BOUND","FIXED_NORM_TOLERANCE"),"SEMANTIC":("QUANTIFIED_OBLIGATION","POLICY_CONFORMANCE")}: raise ValueError("typed identity contract mismatch")
 if {x.get("title") for x in c.get("policy_adoption",[])}!=EXPECTED_POLICIES: raise ValueError("policy adoption mismatch")
 if f.get("dimension_scope")!="ARBITRARY_FINITE_DIMENSION_N_GE_1": raise ValueError("formal dimension scope mismatch")
 if f.get("norm_scale_bits")!="0x3ff0000000000000": raise ValueError("formal norm scale bits mismatch")
 records=f.get("exact_backend_role_records"); tuples={(x.get("obligation"),x.get("backend"),x.get("role")) for x in records or [] if isinstance(x,dict)}
 if not isinstance(records,list) or len(records)!=13 or len(tuples)!=13 or tuples!=EXPECTED_RECORDS: raise ValueError("formal backend role matrix mismatch")
 if f.get("fixed_dimension_policy")!="FIXED_3X3_OR_NUMERIC_FIXTURES_ARE_SUPPLEMENTAL_ONLY_EXCEPT_FROZEN_F02_OPERATOR": raise ValueError("fixed-dimension policy mismatch")
 probe=f.get("scope_probe",{})
 if probe.get("pointwise_hypotheses_required") is not True or probe.get("no_sorry_admitted_or_project_local_axiom") is not True: raise ValueError("generic scope or assumption audit weakened")
 if f.get("directed_rounding_bridge_required") is not True: raise ValueError("directed-rounding bridge missing")
 cap=s.get("max_arnoldi",{})
 if cap.get("meaning")!="TOTAL_ARNOLDI_VECTOR_CAP_PER_COMPLETED_GMRES_LINEAR_SOLVE_TRACE_ROW": raise ValueError("max_arnoldi meaning mismatch")
 if cap.get("equality_is_legal") is not True or cap.get("max_plus_one_rejects") is not True: raise ValueError("max_arnoldi boundary mismatch")
 if cap.get("independent_of_iteration_limit") is not True: raise ValueError("max_arnoldi independence mismatch")
 if cap.get("restart_is_per_cycle_not_total_cap") is not True: raise ValueError("restart semantics mismatch")
 if s.get("receipt_roundtrip",{}).get("operation")!="serde_json serialize concrete receipt then deserialize same type then canonical reserialize": raise ValueError("receipt roundtrip mismatch")
 rounding=s.get("directed_binary64",{})
 if rounding.get("exact_zero_preserved") is not True or rounding.get("overflow_result")!="POSITIVE_INFINITY_THEN_TYPED_NONFINITE_REJECTION_NO_DECISION": raise ValueError("rounding boundary mismatch")
 if c.get("candidate_executions")!=0 or h.get("execution",{}).get("candidate_executions")!=0: raise ValueError("candidate count mismatch")
 if c.get("claim_ceiling")!=CEILING or h.get("claim_ceiling")!=CEILING: raise ValueError("claim ceiling mismatch")
 prompt_text=(research/"CODEX_START_HERE.md").read_text(encoding="utf-8")
 missing_markers=sorted(x for x in PROMPT_MARKERS if x not in prompt_text)
 if missing_markers: raise ValueError(f"prompt markers missing: {', '.join(missing_markers)}")
 total=0
 for relative in sorted(files):
  path=research/relative; size=path.stat().st_size; total+=size
  if size>262144: raise ValueError(f"handoff file exceeds size limit: {relative}")
  if relative.endswith(FORBIDDEN_SUFFIXES): raise ValueError(f"forbidden checked-in suffix: {relative}")
 if total>2000000: raise ValueError("handoff directory exceeds total size limit")
 phase=a.get("successor_phase")
 if phase!=h.get("phase"): raise ValueError("successor phase mismatch")
 locked_paths=0
 if phase=="BOOTSTRAP_UNBOUND":
  if a.get("successor_draft_pr") is not None or "HANDOFF_INPUT_LOCK.json" in files: raise ValueError("bootstrap binding mismatch")
  status="STAGE_CERTIFICATE_REPAIR_HANDOFF_BOOTSTRAP_VALID"
 elif phase=="FINAL_BOUND":
  if type(a.get("successor_draft_pr")) is not int or a["successor_draft_pr"]<=41: raise ValueError("successor Draft PR binding mismatch")
  if not isinstance(a.get("successor_control_commit"),str) or len(a["successor_control_commit"])!=40: raise ValueError("successor control commit missing")
  if not isinstance(a.get("successor_control_tree"),str) or len(a["successor_control_tree"])!=40: raise ValueError("successor control tree missing")
  if "HANDOFF_INPUT_LOCK.json" not in files: raise ValueError("final phase missing input lock")
  lock=_load(research/"HANDOFF_INPUT_LOCK.json")
  if lock.get("schema")!="vigilode-audit2-stage-certificate-repair-input-lock/v1" or lock.get("draft_pr")!=a.get("successor_draft_pr") or lock.get("control_commit")!=a.get("successor_control_commit"): raise ValueError("input lock authority mismatch")
  paths=lock.get("paths")
  if not isinstance(paths,dict) or not paths: raise ValueError("input lock paths missing")
  for relative,expected in sorted(paths.items()):
   path=root/relative
   if not path.is_file() or _sha256(path)!=expected: raise ValueError(f"input lock hash mismatch: {relative}")
  locked_paths=len(paths); status="STAGE_CERTIFICATE_REPAIR_HANDOFF_FINAL_VALID"
 else: raise ValueError("unknown successor phase")
 return {"status":status,"phase":phase,"candidate_executions":0,"formal_role_records":len(records),"locked_paths":locked_paths,"checked_files":len(files),"checked_bytes":total,"claim_ceiling":CEILING}
def main():
 root=pathlib.Path(__file__).resolve().parents[1]
 try: result=validate(root)
 except (OSError,ValueError,json.JSONDecodeError) as error:
  print(f"HANDOFF_INVALID: {error}",file=sys.stderr); return 1
 print(json.dumps(result,sort_keys=True,separators=(",",":"))); return 0
if __name__=="__main__": raise SystemExit(main())
