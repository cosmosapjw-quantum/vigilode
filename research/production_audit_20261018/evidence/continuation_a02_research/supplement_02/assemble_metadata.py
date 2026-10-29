#!/usr/bin/env python3
"""Assemble the contract-only A02 v2 metadata; does not execute a solver/test."""
import copy
import datetime
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[5]
OUT = Path(__file__).resolve().parent
OLD = OUT.parent / "supplement_01"
BASE = ROOT / "research/production_audit_20261018"
UNIT = "A02-CONTRACT-SUPPLEMENT-02"
PROOF_SHA = "ecc8841db6ad2b2f40c7245102372e31e31120e38ecdb68b91ed0cfa4ddcdcb5"
commands = []


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def emit(name, data):
    (OUT / name).write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")


def capture(argv, name):
    result = subprocess.run(argv, cwd=ROOT, text=True, capture_output=True, check=False)
    stdout = name + ".stdout.txt"
    stderr = name + ".stderr.txt"
    (OUT / stdout).write_text(result.stdout)
    (OUT / stderr).write_text(result.stderr)
    commands.append({"argv": argv, "cwd": str(ROOT), "exit_code": result.returncode,
                     "stdout": stdout, "stderr": stderr,
                     "kind": "METADATA_ONLY_READ"})
    if result.returncode:
        raise RuntimeError(f"metadata command failed: {argv}: {result.returncode}")
    return result.stdout.strip()


started = datetime.datetime.now(datetime.timezone.utc).isoformat()
if ROOT != Path("/home/cosmosapjw/vigilode"):
    raise RuntimeError(f"unexpected repository root {ROOT}")
historical_before = {str(p.relative_to(ROOT)): digest(p)
                     for p in sorted(OLD.rglob("*")) if p.is_file()}
contract = BASE / "A02_VERIFIED_TRAJECTORY_CONTRACT.md"
decision = BASE / "A02_RESEARCH_DECISION.json"
protected_before = {str(p.relative_to(ROOT)): digest(p) for p in [contract, decision]}
head = capture(["git", "rev-parse", "HEAD"], "git_head")
tree = capture(["git", "rev-parse", "HEAD^{tree}"], "git_tree")
branch = capture(["git", "branch", "--show-current"], "git_branch")
capture(["git", "status", "--short"], "git_status")
capture(["git", "remote", "-v"], "git_remotes")
remote = capture(["git", "rev-parse", "refs/remotes/origin/audit/production-readiness-20261018"], "git_remote_tracking_ref")

contract_bytes = contract.read_bytes()
proof = contract_bytes[contract_bytes.index(b"## 2. "):contract_bytes.index(b"## 7. ")]
observed_proof_sha = hashlib.sha256(proof).hexdigest()
if observed_proof_sha != PROOF_SHA:
    raise RuntimeError("original D1-D5 proof digest changed")

v1path = OLD / "FROZEN_ACCEPTANCE_INPUTS.json"
v1 = json.loads(v1path.read_text())
v2 = copy.deepcopy(v1)
overflow = next(c for c in v2["acceptance_inputs"]["AT08"]["cases"] if c["id"] == "OVERFLOW_OMEGA")
old_overflow = copy.deepcopy(overflow)
overflow["xi"] = [{"bits": "0x3ff0000000000008", "display": "1.0000000000000018"} for _ in range(3)]
old_overflow["id"] = "OVERFLOW_OMEGA_V1_DOMAIN_REFUSAL"
old_overflow["expected"] = "typed CollectiveAdmissionError::DetuningOutOfRange before multiplication; domain refusal only; not arithmetic overflow coverage or numerical inclusion"
v2["acceptance_inputs"]["AT08"]["cases"].append(old_overflow)
v2["acceptance_inputs"]["AT06"]["first_window_query_order"] = [
    {"query_index": 0, "role": "requested_interior", "global_time_bits": "0x3fa0000000000000", "exact_rational": "1/32"},
    {"query_index": 1, "role": "mandatory_endpoint", "global_time_bits": "0x3fb0000000000000", "exact_rational": "1/16", "insert_into_requested_grid": False}
]
for seam in v2["acceptance_inputs"]["AT06"]["fault_sites"]:
    if seam["id"] == "TXN_FAIL_FIRST":
        seam["private_test_seam_only"] = "return existing typed OutwardRefusal before query_index=0 requested global bits(1/32); before commit"
    elif seam["id"] == "TXN_FAIL_SECOND":
        seam["private_test_seam_only"] = "complete query_index=0 requested global bits(1/32), then return existing typed OutwardRefusal before query_index=1 mandatory endpoint bits(1/16); before commit"
v2["acceptance_input_version"] = 2
v2["status"] = "FROZEN_V2_INPUTS_READY_FOR_FRESH_DELTA_REVIEW"
v2["amendment_binding"] = {
    "unit": UNIT, "previous_input": "../supplement_01/FROZEN_ACCEPTANCE_INPUTS.json",
    "previous_input_sha256": digest(v1path), "contract_overlay": "AMENDMENT_KO.md",
    "numerical_input_changes": "Only three xi values in AT08 OVERFLOW_OMEGA are replaced by bits 0x3ff0000000000008; all other original numerical inputs are unchanged.",
    "historical_case": "AT08 OVERFLOW_OMEGA_V1_DOMAIN_REFUSAL keeps the original width/xi input with domain-refusal classification.",
    "unchanged_proof_section_2_through_6_sha256": PROOF_SHA,
    "implementation_approved": False
}
v2["implementation_choices"] = {
    "DC1": {
        "choice": "narrow immutable accessor; original section6 option(a)",
        "accessors": ["PacketCurveOutputView::weighted_center_displacement(&self) -> f64", "PacketCertifiedView::weighted_center_displacement(&self) -> f64"],
        "definition": "private delta_w = sqrt_up(sum_j_outward(w_j * delta_j^2)); reuse each same-query delta_j enclosing exact represented Y_j minus same returned binary64 C_j",
        "summation_order": "stored j=0..N-1; initialize exact zero; mul_up(delta_j,delta_j), mul_up(w_j,square), add_up(accumulator,term), final sqrt_up",
        "existing_weighted_radius_semantics": "unchanged base T1 radius around exact frozen polynomial; excludes returned-center displacement",
        "getter_cost": "immutable field read, no recomputation or allocation",
        "new_success_helper_calls": "3*N+1 curve_output_certificate_ops under current helper-call accounting; failed attempted helpers also charged",
        "required_future_checks": ["outward displacement enclosure", "existing centers/mode-radii/mean-radius bit identity", "private immutable fields compile-fail", "typed arithmetic refusal", "source-derived exact work deltas"]
    }
}
emit("FROZEN_ACCEPTANCE_INPUTS_V2.json", v2)

# A metadata readback proves that unamended inputs did not drift. This is not a numerical test.
restored = copy.deepcopy(v2)
for key in ["acceptance_input_version", "amendment_binding", "implementation_choices"]:
    del restored[key]
restored["status"] = v1["status"]
restored["acceptance_inputs"]["AT08"]["cases"].pop()
restored_overflow = next(c for c in restored["acceptance_inputs"]["AT08"]["cases"] if c["id"] == "OVERFLOW_OMEGA")
restored_overflow["xi"] = next(c for c in v1["acceptance_inputs"]["AT08"]["cases"] if c["id"] == "OVERFLOW_OMEGA")["xi"]
del restored["acceptance_inputs"]["AT06"]["first_window_query_order"]
restored["acceptance_inputs"]["AT06"]["fault_sites"] = copy.deepcopy(v1["acceptance_inputs"]["AT06"]["fault_sites"])
if restored != v1:
    raise RuntimeError("unregistered original-input delta")

prior_bindings_path = OLD / "SOURCE_BINDINGS.json"
prior_bindings = json.loads(prior_bindings_path.read_text())
source_bindings = []
for entry in prior_bindings["source_bindings"]:
    path = ROOT / entry["path"]
    sha = digest(path)
    if sha != entry["observed_sha256"]:
        raise RuntimeError(f"frozen relevant source changed: {path}")
    source_bindings.append({"path": entry["path"], "sha256": sha,
                            "same_as_supplement_01": True,
                            "identity_class": "IMMUTABLE_SOURCE_INPUT"})
emit("SOURCE_BINDINGS.json", {"unit": UNIT, "source_head": head,
    "prior_source_bindings": "../supplement_01/SOURCE_BINDINGS.json",
    "prior_source_bindings_sha256": digest(prior_bindings_path),
    "source_bindings": source_bindings, "proof_section_2_through_6_sha256": PROOF_SHA})
emit("SOURCE_IDENTITY.json", {"unit": UNIT, "cwd": str(ROOT), "branch": branch,
    "head": head, "tree": tree, "remote_tracking_ref": "refs/remotes/origin/audit/production-readiness-20261018",
    "remote_tracking_sha": remote, "network_fetch_performed_by_this_unit": False,
    "remote_verification_scope": "local remote-tracking ref only; parent owns fetch/push",
    "dirty_state_preserved": True, "status_receipt": "git_status.stdout.txt",
    "requested_executor": {"model": "gpt-6.1-sol", "reasoning_effort": "xhigh"},
    "observed_runtime_identity": "NOT_INDEPENDENTLY_OBSERVED",
    "executor_agent": "/root/a02_fixture_amendment_research"})
emit("PRIOR_REVIEW_HANDOFF.json", {"provenance": "PARENT_MESSAGE_ONLY; direct reviewer artifact absent at unit start",
    "prior_verdict": "HOLD", "approval_reused": False,
    "finding": "AT08 OVERFLOW_OMEGA xi=[2,2,2] is refused by CollectiveModel detuning admission before multiplication.",
    "parent_requested_delta": ["xi bits0x3ff0000000000008 with same MAX width and exact product proof >2^1024", "keep original as domain refusal", "fix DC1 immutable outward sqrt(sum w delta^2) accessor", "freeze AT06 requested1/32 then mandatory endpoint1/16"],
    "next_review": "fresh independent read-only delta review"})
emit("INPUT_HASHES.json", {"unit": UNIT,
    "historical_supplement_01_files": historical_before,
    "protected_existing_contract": {"path": str(contract.relative_to(ROOT)), "sha256": protected_before[str(contract.relative_to(ROOT))]},
    "protected_existing_decision": {"path": str(decision.relative_to(ROOT)), "sha256": protected_before[str(decision.relative_to(ROOT))]},
    "v2_input": {"path": "FROZEN_ACCEPTANCE_INPUTS_V2.json", "sha256": digest(OUT / "FROZEN_ACCEPTANCE_INPUTS_V2.json")},
    "overlay": {"path": "AMENDMENT_KO.md", "sha256": digest(OUT / "AMENDMENT_KO.md")},
    "proof_section_2_through_6_sha256": PROOF_SHA})
emit("DECISION.json", {"unit": UNIT, "status": "READY_FOR_FRESH_DELTA_REVIEW",
    "implementation_approved": False, "scientific_admission": "NOT_ADMITTED",
    "prior_hold": "ADMISSION_BEFORE_MULTIPLICATION_FIXTURE_DEFECT",
    "delta": ["AT08 arithmetic-overflow witness admitted xi replacement", "original AT08 input preserved and classified domain refusal", "DC1 immutable accessor fixed", "AT06 query order fixed"],
    "input": "FROZEN_ACCEPTANCE_INPUTS_V2.json", "input_sha256": digest(OUT / "FROZEN_ACCEPTANCE_INPUTS_V2.json"),
    "overlay": "AMENDMENT_KO.md", "overlay_sha256": digest(OUT / "AMENDMENT_KO.md"),
    "D1_D5_proof_sha256": PROOF_SHA, "D1_D5_changed": False,
    "protected_tolerances": "unchanged original exact bits: T_FIXED1e-5/1e-5; T_INHERITED1e-4/1e-4; T_REJECTION2e-4/2e-4",
    "numerical_execution": "NOT_RUN", "Rust_modified": False, "tests_modified": False,
    "builds_run": False, "solver_or_reference_runs": False,
    "A02_scientific_decision": "HOLD_PENDING_REVIEW_IMPLEMENTATION_VALIDATION",
    "overall_production": "HOLD", "allocation": "NOT_MEASURED_FOR_A02",
    "timing_authority": "DIAGNOSTIC_ONLY",
    "next_action": "Fresh independent read-only delta review; READY then frozen-contract lower-tier implementation and original scoped AT01-AT12 execution."})

historical_after = {str(p.relative_to(ROOT)): digest(p)
                    for p in sorted(OLD.rglob("*")) if p.is_file()}
protected_after = {str(p.relative_to(ROOT)): digest(p) for p in [contract, decision]}
if historical_before != historical_after or protected_before != protected_after:
    raise RuntimeError("historical source/input modified during metadata assembly")
emit("METADATA_READBACK.json", {"status": "METADATA_CONSISTENT",
    "numerical_validation_performed": False, "historical_supplement_01_unchanged": True,
    "original_contract_decision_unchanged": True, "D1_D5_digest_matches": True,
    "all_17_relevant_source_bindings_match_prior": len(source_bindings) == 17,
    "unamended_input_subtrees_structurally_identical": True,
    "new_overflow_xi_bits": [x["bits"] for x in overflow["xi"]],
    "original_width_bits_retained": overflow["width_bits"],
    "original_case_preserved_in_v1_and_explicit_domain_arm": True,
    "all_original_tolerance_and_model_config_fixture_bits_unchanged": True,
    "first_window_query_order": v2["acceptance_inputs"]["AT06"]["first_window_query_order"],
    "proof_is_direct_integer_algebra": True})
manifest_paths = [p for p in sorted(OUT.iterdir()) if p.is_file() and p.name not in {"SHA256SUMS.txt", "RUN_RECORD.json"}]
(OUT / "SHA256SUMS.txt").write_text("".join(f"{digest(p)}  {p.name}\n" for p in manifest_paths))
emit("RUN_RECORD.json", {"unit": UNIT, "started_utc": started,
    "completed_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "scope": "contract amendment and metadata assembly/readback only; no Rust/tests/build/solver execution",
    "assembly_command": {"argv": ["python3", str(Path(__file__).relative_to(ROOT))], "exit_code": 0,
                         "source_sha256": digest(Path(__file__)),
                         "exit_code_observation": "success tool return and final RUN_RECORD creation; if script raises, this success record is not produced"},
    "commands": commands, "source_identity": "SOURCE_IDENTITY.json",
    "input_hashes": "INPUT_HASHES.json", "metadata_readback": "METADATA_READBACK.json",
    "result": "READY_FOR_FRESH_DELTA_REVIEW", "implementation_approved": False,
    "scientific_test_exit_codes": [], "scientific_tests": "NOT_RUN",
    "builds": "NOT_RUN", "solver_reference_execution": "NOT_RUN",
    "source_or_input_mismatches": [], "past_failures_preserved": True,
    "first_failure_in_this_metadata_unit": "NONE_OBSERVED",
    "output_manifest": "SHA256SUMS.txt", "output_manifest_sha256": digest(OUT / "SHA256SUMS.txt")})
print(json.dumps({"unit": UNIT, "metadata_status": "METADATA_CONSISTENT",
                  "input_sha256": digest(OUT / "FROZEN_ACCEPTANCE_INPUTS_V2.json"),
                  "overlay_sha256": digest(OUT / "AMENDMENT_KO.md"),
                  "decision_sha256": digest(OUT / "DECISION.json"),
                  "run_record_sha256": digest(OUT / "RUN_RECORD.json"),
                  "numerical_runs": 0}, sort_keys=True))
