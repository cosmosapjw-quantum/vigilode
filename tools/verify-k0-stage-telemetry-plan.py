#!/usr/bin/env python3
from __future__ import annotations
import argparse, hashlib, json, pathlib, re, sys

PACKAGE = pathlib.Path("docs/exec-plans/k0-stage-telemetry-integration-20260827")
EXPECTED_MAIN = "8d0c79184e09efb5bdadc24a6315c60a71a44264"
EXPECTED_BASE = "e1124586a4029f86669e7489278c61ef676d61aa"
EXPECTED_JIRA = "PM-7"
REQUIRED_WU = [
    "WU-00-authority-intake.json",
    "WU-01-schema-and-observation-types.json",
    "WU-02-solver-observation-hooks.json",
    "WU-03-stage-receipts-and-aggregate.json",
    "WU-04-frozen-six-family-replay.json",
    "WU-05-review-audit-and-atlassian-sync.json",
]
FORBIDDEN_PLACEHOLDERS = ("TBD", "TODO", "<command>", "<path>", "K1_EVIDENCE_DIR_TOKEN")

def load(path):
    with path.open(encoding="utf-8") as f:
        return json.load(f)

def fail(msg):
    print(json.dumps({"status":"FAIL","error":msg}, indent=2))
    raise SystemExit(1)

def package_check(root):
    pkg = root / PACKAGE
    plan = load(pkg / "plan.json")
    if plan.get("schema") != "vigilode-audit-compiled-execution-plan/v1":
        fail("bad plan schema")
    if plan["authority"]["canonical_main"]["sha"] != EXPECTED_MAIN:
        fail("canonical main drift")
    if plan["authority"]["stacked_implementation_base"]["sha"] != EXPECTED_BASE:
        fail("implementation base drift")
    if plan["integrations"]["atlassian"]["jira_issue"] != EXPECTED_JIRA:
        fail("jira binding drift")
    bound = plan["publication_state"] == "BOUND"
    if bound:
        for value, label in [
            (plan["integrations"]["github"]["control_package_pr_number"], "control PR"),
            (plan["integrations"]["atlassian"]["confluence_page_id"], "Confluence page"),
        ]:
            if value in (None, "", "BOOTSTRAP_PENDING"):
                fail(f"missing bound {label}")
    paths = plan["work_units"]
    if len(paths) != len(REQUIRED_WU):
        fail("wrong work-unit count")
    ids = []
    for name in REQUIRED_WU:
        path = pkg / "work-units" / name
        if not path.is_file():
            fail(f"missing {path}")
        wu = load(path)
        if wu.get("schema") != "audit-compiled-work-unit/v1":
            fail(f"bad work-unit schema {name}")
        ids.append(wu["id"])
        for key in ("authority","objective","risk","scope","preconditions","invariants","failure_modes","implementation","verification","completion_evidence","agent_policy","review_gate"):
            if key not in wu:
                fail(f"{name} missing {key}")
        if not wu["failure_modes"]:
            fail(f"{name} has no failure modes")
        if not wu["invariants"]:
            fail(f"{name} has no invariants")
    if ids != ["WU-00","WU-01","WU-02","WU-03","WU-04","WU-05"]:
        fail("work-unit order/id mismatch")
    for path in pkg.rglob("*"):
        if path.is_file() and path.suffix in {".md",".json",".py"}:
            raw = path.read_bytes()
            bad_controls = [byte for byte in raw if byte < 32 and byte not in (9, 10)]
            if bad_controls:
                fail(f"control character in {path}: {sorted(set(bad_controls))}")
            text = raw.decode("utf-8")
            for token in FORBIDDEN_PLACEHOLDERS:
                if token in text:
                    fail(f"placeholder {token} in {path}")
    manifest = root / "PACKAGE_MANIFEST.sha256"
    if not manifest.is_file():
        fail("missing package manifest")
    for line in manifest.read_text(encoding="utf-8").splitlines():
        digest, rel = line.split("  ", 1)
        p = root / rel
        if not p.is_file():
            fail(f"manifest missing file {rel}")
        raw = p.read_bytes()
        bad_controls = [byte for byte in raw if byte < 32 and byte not in (9, 10)]
        if bad_controls:
            fail(f"control character in manifest file {rel}: {sorted(set(bad_controls))}")
        got = hashlib.sha256(raw).hexdigest()
        if got != digest:
            fail(f"manifest mismatch {rel}")
    return {"status":"PASS","publication_state":plan["publication_state"],"work_units":ids,"jira":EXPECTED_JIRA}

def evidence_check(root, evidence_dir):
    path = root / evidence_dir
    cells = sorted(path.glob("*.json"))
    if len(cells) != 12:
        fail(f"expected 12 cell JSON files, got {len(cells)}")
    identities = set()
    states = {}
    for cell in cells:
        obj = load(cell)
        if obj.get("execution_state") not in {"COMPLETE","STOP_INVALID","ERROR"}:
            fail(f"bad terminal state {cell}")
        identity = (obj.get("kernel_arm"), obj.get("family"))
        if identity in identities:
            fail(f"duplicate identity {identity}")
        identities.add(identity)
        states[cell.name] = obj["execution_state"]
        tol = obj.get("tolerance", {})
        if tol.get("rtol") != 1e-10 or tol.get("atol") != 1e-12:
            fail(f"tolerance drift {cell}")
        for stage in obj.get("stages", []):
            work = stage.get("work")
            if work:
                if work["operator_applies_total"] != work["linear_matvecs"] + work["diagnostic_matvecs"]:
                    fail(f"accounting mismatch {cell}")
    return {"status":"PASS","cells":len(cells),"states":states}

def claim_scan(root):
    forbidden = [
        r"\bauthoritative speedup\b", r"\bproduction-safe\b",
        r"\bRODAS5P is faster than BDF\b", r"\bactivate in production\b"
    ]
    hits = []
    for path in root.rglob("*"):
        if path.is_file() and path.suffix in {".md",".json",".rs",".py"}:
            text = path.read_text(encoding="utf-8", errors="ignore")
            for pat in forbidden:
                if re.search(pat, text, flags=re.I):
                    hits.append((str(path), pat))
    if hits:
        fail(f"forbidden claims: {hits}")
    return {"status":"PASS","forbidden_claim_hits":0}

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo-root", default=".")
    ap.add_argument("--check-package", action="store_true")
    ap.add_argument("--evidence-dir")
    ap.add_argument("--scan-claims", action="store_true")
    ap.add_argument("--check-final-diff", action="store_true")
    ap.add_argument("--check-atlassian-binding", action="store_true")
    args = ap.parse_args()
    root = pathlib.Path(args.repo_root).resolve()
    results = {}
    if args.check_package or not any([args.evidence_dir,args.scan_claims,args.check_final_diff,args.check_atlassian_binding]):
        results["package"] = package_check(root)
    if args.evidence_dir:
        results["evidence"] = evidence_check(root, pathlib.Path(args.evidence_dir))
    if args.scan_claims:
        results["claims"] = claim_scan(root)
    if args.check_final_diff:
        results["final_diff"] = {"status":"DEFERRED_TO_IMPLEMENTATION_RUN","required":True}
    if args.check_atlassian_binding:
        plan = load(root / PACKAGE / "plan.json")
        a = plan["integrations"]["atlassian"]
        if not a["jira_issue"]:
            fail("missing Jira issue")
        if plan["publication_state"] == "BOUND" and not a["confluence_page_id"]:
            fail("missing Confluence page")
        results["atlassian"] = {"status":"PASS","jira":a["jira_issue"],"confluence":a["confluence_page_id"]}
    print(json.dumps({"status":"PASS","results":results}, indent=2))

if __name__ == "__main__":
    main()
