#!/usr/bin/env python3
"""Checker of research node HT02 (research/ht02_homotopy_parallel_pilot_20261011; see its PREREGISTRATION.md).

Implemented: Branch A only (`--abstain-only`), the registered outcome when HT01 decided ABSTAIN. Nothing is built or
run. The checker reads HT01's committed RESULTS.json, pinned by its SHA-256 (HT01_RESULTS_SHA256, the bytes committed
in d443431), and writes RESULTS.json with decision "ABSTAIN_NOT_RUN", the HT01 decision, the failing criteria of every
candidate, runs = 0 and wall_time_speedup = null. The ledger verdict of Branch A is INCONCLUSIVE ("ABSTAIN, not run;
no evidence about homotopy parallelism"), never PASS.

Branch B (HT01 SELECTED) is not implemented: it needs the supplementary registration HT02b before any Branch-B code.
Invoking it (no --abstain-only) is refused as INVALID (exit 2).

INVALID (exit 2) on the AS03 primitives of tools/evidence_schema_v2.py (strict JSON, exact key sets, typed values)
applied to the fields of HT01's RESULTS.json that this checker reads, and when:
- HT01's RESULTS.json is not the pinned file (SHA-256);
- HT01's verdict is not PASS, or its decision is not ABSTAIN (Branch A requires an ABSTAIN decided by a PASS run);
- this checker is not committed unchanged at HEAD, or the tree is dirty apart from the output path.

Exit codes: 2 INVALID; 0 when the registered Branch-A outcome is written. Its verdict is INCONCLUSIVE, which is
neither PASS nor FAIL (the printed and recorded verdict says so). Interpretation fixed before the run: the registration
lists exit codes for INVALID / FAIL / PASS only; a Branch-A run with valid evidence has no gate to fail, so it exits 0
and records INCONCLUSIVE.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
if str(Path(__file__).resolve().parent) not in sys.path:
    sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence_schema_v2 as evidence  # noqa: E402  (fail-closed evidence validation, re-audit AS03)

SCHEMA = "vigilode-ht02-parallel-check-v1"
CHECKER_PATH = "tools/ht02_parallel_check.py"
HT01_RESULTS = "research/ht01_homotopy_client_contract_20261011/RESULTS.json"
HT01_RESULTS_SHA256 = "255670b231af2db9649935d8cbb2dddc6dfdcec6004d03b0695da79a06f64434"
HT01_SCHEMA = "vigilode-ht01-selection-check-v1"
IDS = ["K1", "K2", "K3", "K4", "K5", "K6", "K7", "K8"]
CRITERIA = ("S1", "S2", "S3", "S4")
ROW_KEYS = frozenset(("id", "S1", "S2", "S3", "S4", "f_min", "f_min_function_view", "f_status", "lead", "reasons",
                      "comparators"))
LEDGER_NOTE = "ABSTAIN, not run; no evidence about homotopy parallelism"


def blob_id(data: bytes) -> str:
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def git(root: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", *args], cwd=root, capture_output=True, text=True)


def provenance(root: Path, ignore: list[str]) -> dict:
    head = git(root, "rev-parse", "HEAD").stdout.strip()
    status = git(root, "status", "--porcelain", "--untracked-files=all").stdout
    blob = git(root, "rev-parse", f"HEAD:{CHECKER_PATH}")
    return {"source_commit": head,
            "tree_status": "\n".join(ln for ln in status.splitlines() if ln[3:] not in ignore),
            "checker_blob_at_head": blob.stdout.strip() if blob.returncode == 0 else None}


def evaluate(raw: bytes, running_checker: bytes, prov: dict, pin: str = HT01_RESULTS_SHA256) -> dict:
    """Branch A on HT01's RESULTS.json bytes; raises evidence.ValidationError for INVALID."""
    c = evidence._Checks()
    sha = hashlib.sha256(raw).hexdigest()
    if sha != pin:
        c.fail("ht01", f"HT01 RESULTS.json is not the pinned file (sha256 {sha} != {pin})")
    if prov["tree_status"] != "":
        c.fail("tree", f"dirty tree: {prov['tree_status']!r}")
    if prov["checker_blob_at_head"] is None or prov["checker_blob_at_head"] != blob_id(running_checker):
        c.fail("checker", f"the running checker (blob {blob_id(running_checker)}) is not the checker committed at "
                          f"HEAD ({prov['checker_blob_at_head']})")
    doc = evidence.loads_strict(raw)
    if c.dict_("ht01", doc):
        for key, check in (("schema", c.str_), ("verdict", c.str_), ("decision", c.str_),
                           ("source_commit", c.str_)):
            check(f"ht01.{key}", doc.get(key))
        c.list_("ht01.leads", doc.get("leads"))
        if c.list_("ht01.candidates", doc.get("candidates"), nonempty=True):
            for i, row in enumerate(doc["candidates"]):
                p = f"ht01.candidates[{i}]"
                if not c.keys(p, row, ROW_KEYS):
                    continue
                c.str_(f"{p}.id", row["id"])
                for k in CRITERIA:
                    c.bool_(f"{p}.{k}", row[k])
                if c.keys(f"{p}.reasons", row["reasons"], CRITERIA):
                    for k in CRITERIA:
                        if c.list_(f"{p}.reasons.{k}", row["reasons"][k]):
                            for j, r in enumerate(row["reasons"][k]):
                                c.str_(f"{p}.reasons.{k}[{j}]", r)
    c.done()
    if doc["schema"] != HT01_SCHEMA:
        c.fail("ht01.schema", f"expected {HT01_SCHEMA}, got {doc['schema']!r}")
    if doc["verdict"] != "PASS":
        c.fail("ht01.verdict", f"Branch A needs a PASS run of HT01, got {doc['verdict']!r}")
    if doc["decision"] != "ABSTAIN":
        c.fail("ht01.decision", f"Branch A needs HT01 ABSTAIN, got {doc['decision']!r} (Branch B needs HT02b)")
    if [r["id"] for r in doc["candidates"]] != IDS:
        c.fail("ht01.candidates", "not exactly K1-K8 in order")
    c.done()
    failing = {r["id"]: {"criteria_not_met": [k for k in CRITERIA if not r[k]],
                         "reasons": {k: r["reasons"][k] for k in CRITERIA if r["reasons"][k]},
                         "f_min": r["f_min"], "f_status": r["f_status"]}
               for r in doc["candidates"]}
    return {"ht01": {"path": HT01_RESULTS, "sha256": sha, "verdict": doc["verdict"], "decision": doc["decision"],
                     "leads": doc["leads"], "source_commit": doc["source_commit"]},
            "failing_criteria": failing}


def run(ht01: Path, output: Path, abstain_only: bool, root: Path) -> int:
    if output.exists():
        raise SystemExit(f"immutable output exists: {output}")
    try:
        ignore = [str(output.resolve().relative_to(root.resolve()))]
    except ValueError:
        ignore = []
    prov = provenance(root, ignore)
    try:
        if not abstain_only:
            raise evidence.ValidationError([
                "Branch B (HT01 SELECTED) is not implemented: it needs the supplementary registration HT02b "
                "(research/ht02_homotopy_parallel_pilot_20261011/PREREGISTRATION_HT02B.md) before any Branch-B code"])
        found = evaluate(ht01.read_bytes(), Path(__file__).read_bytes(), prov)
    except (evidence.ValidationError, OSError) as exc:
        error = exc if isinstance(exc, evidence.ValidationError) else evidence.ValidationError([f"ht01: {exc}"])
        return evidence.emit_invalid(output, SCHEMA, error, {"ht01": ht01})
    out = {"schema": SCHEMA, "evidence_schema": evidence.VERSION, "branch": "A",
           "decision": "ABSTAIN_NOT_RUN", "verdict": "INCONCLUSIVE",
           "ledger_verdict": "INCONCLUSIVE", "ledger_note": LEDGER_NOTE,
           "ledger_order_rule": "HT02's first ledger row must follow immediately a node whose ledger row is a "
                                "numeric PASS or FAIL, never another INCONCLUSIVE node (tools/check-research-node.py "
                                "rule (d))",
           "inputs": {"ht01_results": {"path": found["ht01"]["path"], "sha256": found["ht01"]["sha256"]}},
           "provenance": prov, "ht01": found["ht01"], "failing_criteria": found["failing_criteria"],
           "runs": 0, "wall_time_speedup": None, "built": [], "executed": [],
           "claim_scope": "Not run: HT01 selected no client, so no homotopy code was built or run. No evidence about "
                          "homotopy parallelism, work, span or wall time; no default promotion."}
    output.write_text(json.dumps(out, indent=1, allow_nan=False) + "\n")
    print(f"verdict INCONCLUSIVE (Branch A); decision ABSTAIN_NOT_RUN; HT01 decision {found['ht01']['decision']}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--ht01", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--abstain-only", action="store_true")
    parser.add_argument("--runs", type=Path, help="Branch B only (not implemented; needs HT02b)")
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args()
    return run(args.ht01, args.output, args.abstain_only, args.repo_root)


if __name__ == "__main__":
    sys.exit(main())
