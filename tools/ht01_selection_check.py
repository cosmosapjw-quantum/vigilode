#!/usr/bin/env python3
"""Selection checker of research node HT01 (research/ht01_homotopy_client_contract_20261011; PREREGISTRATION.md).

Input: SCREEN.json written by tools/ht01_client_screen.py at a clean commit. Every file the checker reads (profiles,
PP07 RESULTS.json, the L-0008 VERIFY.json, the ledger, the sources, the contract) is read from the git object store
at the screen's source commit, so the check is independent of the working tree.

INVALID (exit 2), before any gate, on the AS03 primitives of tools/evidence_schema_v2.py (strict JSON: no duplicate
keys, no NaN/Infinity; exact key sets; typed finite numbers; bool never accepted as int) and on the registered
validity rules:
- an input not pinned by its SHA-256 at the source commit (registration, contract, ledger, PP07 RESULTS.json with
  its registered digest, the L-0008 VERIFY.json with its registered digest, every git-tracked PROFILE*.json, every
  scanned source, every crates/**/*.rs of the static scans);
- a candidate's applicable-arm list differing from the registered closed list, or any other arm in the screen;
- a registered constant changed (f_min = 0.348, r = 5, P = 8, 1.15, the category and its view);
- the checker blob at the source commit differing from the checker that runs, the recorded screen/checker blobs
  differing from the source commit's, or any recorded tree status dirty (the screen's, and the checker's own, which
  ignores only the SCREEN and output paths).

Gate (PASS exit 0, otherwise FAIL exit 1, every number preserved):
1. Exactly the candidates K1-K8, in order, each with S1-S4 values or typed reasons.
2. Every number, flag and reason is recomputed here, by the checker's own code, from the pinned files: the checker
   re-derives each candidate row (profile entries, comparison binary, rankings, comparators, accuracy-evidence
   membership, f values, S2 counters, static scans, S4) and the global records, and requires equality with the
   screen (same JSON types and values).
3. K1 cites q1_solves = 0 and "PP07 ABSTAIN preserved"; no artificial many-shift workload and no rerun appear: the
   screen executed git commands only, ran no solver, and lists no artificial workload or rerun.
4. The decision equals the registered rule applied to the checker's rows (SELECTED(k) if exactly one candidate meets
   S1-S4; several: the largest minimum f, unmeasured below measured; else ABSTAIN), and the contract document exists
   at the source commit.

Interpretations fixed before the recorded run (I1-I8, identical to the screen's docstring):
- I1 usable profiles/entries: entries with arm, problem, rtol, attempts, counters, ir_per_run, success,
  callgrind_deterministic and both category views, in a file that is an output of a ledger row with the same
  SHA-256; an entry is used when success and callgrind_deterministic hold.
- I2 f is the problem-user-code share of categories_by_file (the view of the registration's disclosed prior shares);
  categories_by_function is reported only (tools/speed_profile.py: a driver monomorphized into a CLI function is
  charged to problem-user-code in that view).
- I3 arms are driver families with their opt-in configurations: small = rodas5p-fast-small{,-val,-land,-ovh,-static,
  -static-stages,-static-ovh}; dense v2 = rodas5p-fast{,-val,-land,-ovh}; colext64 = rodas5p-fast-colext64;
  banded = rodas5p-fast-banded{,-slices}; matrix-free U-form = rodas5p-mf-legacy. Other profile arms are never used.
- I4 comparison binary per case: most applicable arms covered, then newest node date, then larger binary hash.
- I5 comparator per case and rtol: cheapest ir_per_run (ties by configuration name) with a ledger row and accuracy
  evidence (another output of the same ledger row, matching SHA-256, whose JSON names the problem id): a membership
  check of recorded identity/accuracy evidence, not a re-verification of accuracy.
- I6 f_case = min over every usable entry (any profile, any rtol) of the case's comparator configurations;
  f_candidate = min over measured cases; unmeasured when no case is measured.
- I7 S2 = counters.linear_solves / attempts >= 2 on every comparator entry of the comparison binary (K2: the L-0008
  sequential arm; K1: PP07 q1_solves); no committed counters = not met.
- I8 S4a: the Q2CertificateSource impl of the registered client type (K2: QuadraticModel) with
  ModelBinding::GeneratedFromModel, capability Available when the default witness covers the case's structure
  (diagonal J, or n <= 2). S4b: missing HomotopyWorkLedger terms are HT02 obligations, not failures.
The output file is immutable. Design screen only: no solver run, no timing, no speed claim, no default promotion.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
if str(Path(__file__).resolve().parent) not in sys.path:
    sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence_schema_v2 as evidence  # noqa: E402  (fail-closed evidence validation, re-audit AS03)

SCHEMA = "vigilode-ht01-selection-check-v1"
SCREEN_SCHEMA = "vigilode-ht01-screen-v1"
CHECKER_PATH = "tools/ht01_selection_check.py"
SCREEN_TOOL_PATH = "tools/ht01_client_screen.py"
REGISTRATION = "research/ht01_homotopy_client_contract_20261011/PREREGISTRATION.md"
CONTRACT = "docs/HOMOTOPY_EXPENSIVE_CLIENT_CONTRACT.md"
LEDGER = "research/LEDGER.jsonl"
PP07 = "research/pp07_fourier_shared_action_20261004/RESULTS.json"
VERIFY = "research/r3_matched_accuracy_hom06_20261001/VERIFY.json"
HOMOTOPY = "crates/rodas5p-integrators/src/homotopy.rs"
TRANSACTIONAL = "crates/rodas5p-integrators/src/transactional_q1_q2.rs"
PROFILE_PATH = re.compile(r"^research/[^/]+/PROFILE[^/]*\.json$")
REGISTERED_SHA256 = {  # immutable inputs, equal to their ledger rows (L-0074, L-0008)
    PP07: "b4b745c0f1cc96d2074f2a6ed27b823dbeab2fbfe205d36fed7453b10a8ac3cc",
    VERIFY: "cdaf676814fe216237137b60455cd4754bc8578c63ffe31fc4874a50143eff45",
}
CONSTANTS = {"f_min": 0.348, "f_min_unrounded": (1.0 - 1.0 / 1.15) / (1.0 - 5 / 8), "rhs_multiplier_r": 5,
             "threads_P": 8, "required_speedup": 1.15, "min_solves_per_attempt": 2.0,
             "category": "problem-user-code", "f_view": "categories_by_file"}
F_MIN = CONSTANTS["f_min"]
IDS = ["K1", "K2", "K3", "K4", "K5", "K6", "K7", "K8"]
CLOSED = {"K1": [], "K2": ["small", "dense v2", "matrix-free U-form"], "K3": ["small", "dense v2"],
          "K4": ["small", "dense v2"], "K5": ["small", "dense v2", "matrix-free U-form"],
          "K6": ["dense v2", "colext64", "banded", "matrix-free U-form"],
          "K7": ["dense v2", "colext64", "banded", "matrix-free U-form"], "K8": ["small", "dense v2"]}
CONDITION = {("K2", "small"): "n<=8", ("K3", "small"): "n<=8", ("K4", "small"): "n==1", ("K6", "colext64"): "n>64",
             ("K8", "small"): "n<=8"}
FAMILY = {"small": ["rodas5p-fast-small", "rodas5p-fast-small-val", "rodas5p-fast-small-land",
                    "rodas5p-fast-small-ovh", "rodas5p-fast-small-static", "rodas5p-fast-small-static-stages",
                    "rodas5p-fast-small-static-ovh"],
          "dense v2": ["rodas5p-fast", "rodas5p-fast-val", "rodas5p-fast-land", "rodas5p-fast-ovh"],
          "colext64": ["rodas5p-fast-colext64"], "banded": ["rodas5p-fast-banded", "rodas5p-fast-banded-slices"],
          "matrix-free U-form": ["rodas5p-mf-legacy"]}
ARM_OF = {cfg: arm for arm, cfgs in FAMILY.items() for cfg in cfgs}
CLIENT_TYPE = {"K2": "QuadraticModel"}
PP07_TOKENS = ["LuFactorization", "solve_rows", "direct_solve", "solve_gmres", "solve_gcrodr", "solve_lgmres",
               "shared_shift_jet", "LinearOperator", "inverse("]
TERMS = [("setup", "factorization|pool_creation"), ("rounds", "round"), ("max_task_per_round", "max_task"),
         ("barriers", "barrier"), ("certificate", "certificate"), ("rejected_candidates", "reject"),
         ("fallback", "fallback"), ("dispatch", "batch|pool"), ("peak_bytes", "peak|bytes")]
LIMITS = {"crates/rodas5p-integrators/src/chart_transport.rs": "No integrator calls this module",
          "crates/rodas5p-core/src/shared_shift_jet.rs": "execute shifts in parallel or claim a"}
CORPUS = ["robertson-ramped", "hires-ramped", "van-der-pol-ramped", "rotating-nonnormal",
          "nonautonomous-stiff-forcing", "semilinear-advection-diffusion-ramped"]


def registered_cases() -> dict:
    """The registered candidates' sources and cases (closed list of the registration)."""
    bench = ["crates/rodas5p-cli/src/stiff_benchmark.rs"]
    return {
        "K1": (["crates/rodas5p-integrators/src/fourier_path_candidate.rs",
                "crates/rodas5p-integrators/src/fourier_path_certificate.rs"],
               [(c, 4, "dense") for c in ["fast", "forty", "forty_tight", "negative", "resonance", "unit"]]),
        "K2": ([TRANSACTIONAL], [(f"diagonal-quadratic-n{n}", n, "diagonal") for n in [1, 2, 4, 8, 16]]),
        "K3": (["crates/rodas5p-integrators/src/audit2_bateman_real_client_research.rs"],
               [("bateman-two-timescale-parent-stable-daughter-v1", 4, "dense")]),
        "K4": (["crates/rodas5p-integrators/src/chart_transport.rs"], [("darboux-chart", 1, "scalar")]),
        "K5": (bench, [("robertson", 3, "dense"), ("hires", 8, "dense"), ("van-der-pol-mu1000", 2, "dense")]),
        "K6": (bench, [(f"brusselator-1d-{c}", 2 * c, "banded") for c in [30, 40, 50, 160, 200, 500]]),
        "K7": (["crates/rodas5p-integrators/src/scientific_corpus_v2.rs"],
               [(f"{f}-n{n}", n, "dense") for f in CORPUS for n in [96, 384, 1536]]),
        "K8": (["crates/rodas5p-integrators/src/problems.rs"],
               [(c, None, "fixture") for c in ["scalar_linear_problem", "prothero_robinson_problem",
                                               "oscillatory_prothero_robinson_problem", "manufactured_vector_problem",
                                               "constant_affine_mass_problem", "manufactured_mass_nonlinear_problem",
                                               "complex_dahlquist_problem", "semilinear_advection_diffusion_problem"]]),
    }


# --------------------------------------------------------------------------------------------- git access
class Repo:
    def __init__(self, root: Path, commit: str):
        self.root, self.commit = root, commit
        self.cache: dict[str, bytes] = {}
        self.proc = None
        self._tracked = None

    def git(self, *args: str, check: bool = True) -> subprocess.CompletedProcess:
        return subprocess.run(["git", *args], cwd=self.root, capture_output=True, text=True, check=check)

    def blob(self, path: str) -> bytes:
        if path not in self.cache:
            if self.proc is None:
                self.proc = subprocess.Popen(["git", "cat-file", "--batch"], cwd=self.root, stdin=subprocess.PIPE,
                                             stdout=subprocess.PIPE)
            self.proc.stdin.write(f"{self.commit}:{path}\n".encode())
            self.proc.stdin.flush()
            head = self.proc.stdout.readline().decode().split()
            if len(head) != 3 or head[1] != "blob":
                raise evidence.ValidationError([f"{path}: not a blob at the source commit {self.commit}"])
            data = self.proc.stdout.read(int(head[2]))
            self.proc.stdout.read(1)
            self.cache[path] = data
        return self.cache[path]

    def text(self, path: str) -> str:
        return self.blob(path).decode("utf-8", errors="replace")

    def json(self, path: str):
        return evidence.loads_strict(self.blob(path))

    def tracked(self) -> list[str]:
        if self._tracked is None:
            self._tracked = self.git("ls-tree", "-r", "--name-only", self.commit).stdout.splitlines()
        return self._tracked

    def close(self):
        if self.proc is not None:
            self.proc.stdin.close()
            self.proc.wait()
            self.proc.stdout.close()


def blob_id(data: bytes) -> str:
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


# ---------------------------------------------------------------------------------------------- schema
STR, INT, FLOAT, BOOL = "str", "int", "float", "bool"


def opt(spec):
    return ("opt", spec)


def lst(spec):
    return ("list", spec)


def obj(fields: dict):
    return ("obj", fields)


def dmap(spec):
    return ("map", spec)


ACC = opt(obj({"ledger_id": STR, "path": STR, "sha256": STR}))
CERTS = dmap(obj({"certificate_attempts": INT, "certificate_admissions": INT}))
SPEC = obj({
    "schema": STR, "node": STR,
    "provenance": obj({"source_commit": STR, "tree_status": ("strany",), "screen_path": STR, "screen_blob": STR,
                       "checker_path": STR, "checker_blob": STR, "executed_commands": lst(lst(STR)),
                       "solver_runs": INT, "python": STR}),
    "constants": obj({"f_min": FLOAT, "f_min_unrounded": FLOAT, "rhs_multiplier_r": INT, "threads_P": INT,
                      "required_speedup": FLOAT, "min_solves_per_attempt": FLOAT, "category": STR, "f_view": STR}),
    "inputs": lst(obj({"role": STR, "path": STR, "sha256": STR})),
    "rust_sources": lst(("pair",)),
    "profiles": lst(obj({"path": STR, "sha256": STR, "ledger_ids": lst(STR), "binary_sha256": opt(STR),
                         "node_date": STR, "usable": BOOL, "reason": opt(STR), "usable_entries": INT})),
    "declaration_scan": obj({"pattern": STR, "matches": lst(("match",))}),
    "work_ledger": obj({"source": STR, "fields": lst(STR), "terms": dmap(lst(STR)), "missing_terms": lst(STR)}),
    "default_witness": obj({"diagonal_branch": BOOL, "small_branch": BOOL, "small_limit_message": opt(STR)}),
    "stated_limits": dmap(BOOL),
    "pp07": obj({"q1_solves": INT, "q2_multi_shift_starts": INT, "q2_jet_admissible_starts": INT, "verdict": STR}),
    "l0008": obj({"rhs_multiplier_p8_over_sequential": dmap(FLOAT), "rhs_multiplier_median": FLOAT,
                  "p8_certificates": CERTS, "reference_arm": STR}),
    "candidates": lst(obj({
        "id": STR, "client": STR, "sources": lst(STR),
        "cases": lst(obj({"case": STR, "dimension": opt(INT), "structure": STR})),
        "S1": obj({"declaration_matches": lst(("match",)), "batch_rhs_references": INT, "a": BOOL,
                   "f_status": STR,
                   "f_values": lst(obj({"case": STR, "config": STR, "file": STR, "index": INT, "rtol": FLOAT,
                                        "share_file": FLOAT, "share_function": FLOAT})),
                   "f_min": opt(FLOAT), "f_min_function_view": opt(FLOAT), "b": opt(BOOL), "pass": BOOL,
                   "reasons": lst(STR)}),
        "S2": obj({"source": STR,
                   "rows": lst(obj({"case": STR, "arm": STR, "file": STR, "index": opt(INT), "rtol": opt(FLOAT),
                                    "attempts": INT, "linear_solves": INT, "solves_per_attempt": FLOAT})),
                   "q1_solves": opt(INT), "pp07": opt(STR), "static_solver_tokens": lst(STR), "pass": BOOL,
                   "reasons": lst(STR)}),
        "S3": obj({"applicable_arms": lst(STR),
                   "arms": lst(obj({"arm": STR, "condition": opt(STR), "status": STR, "reason": opt(STR)})),
                   "per_case": lst(obj({"case": STR, "binary": STR, "binary_arms": lst(STR), "rtols": lst(obj({
                       "rtol": FLOAT,
                       "ranking": lst(obj({"config": STR, "arm": STR, "ir_per_run": INT, "file": STR, "index": INT,
                                           "ledger_id": STR, "accuracy_evidence": ACC})),
                       "comparator": opt(obj({"config": STR, "arm": STR, "file": STR, "index": INT}))}))})),
                   "cases_without_comparator": lst(STR), "l0008_reference_arm": opt(STR), "pass": BOOL,
                   "reasons": lst(STR)}),
        "S4": obj({"impls": lst(obj({"type": STR, "binding": STR})), "client_type": opt(STR), "binding": opt(STR),
                   "capabilities": lst(obj({"case": STR, "capability": opt(STR)})),
                   "capability_evidence": opt(obj({"source": STR, "p8_certificates": CERTS})),
                   "a": BOOL, "b": BOOL, "obligations": lst(STR), "pass": BOOL, "reasons": lst(STR)}),
        "meets": obj({"S1": BOOL, "S2": BOOL, "S3": BOOL, "S4": BOOL}), "all": BOOL, "lead": BOOL})),
    "decision": obj({"outcome": STR, "selected": opt(STR), "meeting_all": lst(STR), "leads": lst(STR)}),
    "reported": obj({"max_share_file_all_applicable_entries": opt(FLOAT),
                     "max_share_function_all_applicable_entries": opt(FLOAT),
                     "entries_function_view_at_or_above_f_min": lst(obj({
                         "file": STR, "index": INT, "config": STR, "problem": STR, "rtol": FLOAT,
                         "share_function": FLOAT, "share_file": FLOAT})),
                     "function_view_note": STR, "artificial_workloads": lst(STR), "reruns": lst(STR)}),
})


def check_spec(c, path: str, value, spec) -> None:
    """Exact key sets and typed values (evidence_schema_v2 primitives) for the screen document."""
    if isinstance(spec, str):
        if spec == STR:
            c.str_(path, value)
        elif spec == INT:
            c.int_(path, value)
        elif spec == FLOAT:
            c.float_(path, value)
        else:
            c.bool_(path, value)
        return
    kind = spec[0]
    if kind == "strany":
        c.str_(path, value, nonempty=False)
    elif kind == "opt":
        if value is not None:
            check_spec(c, path, value, spec[1])
    elif kind == "list":
        if c.list_(path, value):
            for i, v in enumerate(value):
                check_spec(c, f"{path}[{i}]", v, spec[1])
    elif kind == "map":
        if c.dict_(path, value):
            for k, v in value.items():
                check_spec(c, f"{path}.{k}", v, spec[1])
    elif kind == "obj":
        if c.keys(path, value, spec[1].keys()):
            for k, s in spec[1].items():
                check_spec(c, f"{path}.{k}", value[k], s)
    elif kind in ("pair", "match"):
        if isinstance(value, list) and len(value) == 2:
            c.str_(f"{path}[0]", value[0])
            if kind == "pair":
                c.str_(f"{path}[1]", value[1])
            else:
                c.int_(f"{path}[1]", value[1], minimum=1)
        else:
            c.fail(path, f"expected a two-element {'[path, sha256]' if kind == 'pair' else '[path, line]'} array")


# --------------------------------------------------------------------------------------------- validity
def own_tree_status(root: Path, ignore: list[str]) -> str:
    out = subprocess.run(["git", "status", "--porcelain", "--untracked-files=all"], cwd=root, capture_output=True,
                         text=True, check=True).stdout
    return "\n".join(ln for ln in out.splitlines() if ln[3:] not in ignore)


def validate(doc, repo: Repo, running_checker: bytes, tree_status_now: str) -> None:
    c = evidence._Checks()
    check_spec(c, "screen", doc, SPEC)
    c.done()
    if doc["schema"] != SCREEN_SCHEMA or doc["node"] != "HT01":
        c.fail("screen.schema", f"expected {SCREEN_SCHEMA} / HT01")
    for k, v in CONSTANTS.items():
        if type(doc["constants"][k]) is not type(v) or doc["constants"][k] != v:
            c.fail(f"screen.constants.{k}", f"registered {v!r}, got {doc['constants'][k]!r}")
    prov = doc["provenance"]
    if prov["tree_status"] != "":
        c.fail("screen.provenance.tree_status", "the screen ran on a dirty tree")
    if tree_status_now != "":
        c.fail("checker.tree_status", f"the checker runs on a dirty tree: {tree_status_now!r}")
    if prov["screen_path"] != SCREEN_TOOL_PATH or prov["checker_path"] != CHECKER_PATH:
        c.fail("screen.provenance", "tool paths differ from the registered tools")
    if not re.fullmatch(r"[0-9a-f]{40}", prov["source_commit"]) or repo.git(
            "cat-file", "-e", f"{prov['source_commit']}^{{commit}}", check=False).returncode != 0:
        c.fail("screen.provenance.source_commit", "not a commit of this repository")
        c.done()
    for tool, key in ((CHECKER_PATH, "checker_blob"), (SCREEN_TOOL_PATH, "screen_blob")):
        try:
            at_commit = blob_id(repo.blob(tool))
        except evidence.ValidationError as exc:
            c.reasons.extend(exc.reasons)
            continue
        if prov[key] != at_commit:
            c.fail(f"screen.provenance.{key}", f"{prov[key]} != blob at the source commit {at_commit}")
    if blob_id(running_checker) != prov["checker_blob"]:
        c.fail("checker", f"the running checker (blob {blob_id(running_checker)}) differs from the checker blob "
                          f"recorded for the source commit ({prov['checker_blob']})")
    # input pins
    tracked = repo.tracked()
    recorded = {}
    for i, item in enumerate(doc["inputs"]):
        if item["path"] in recorded:
            c.fail(f"screen.inputs[{i}]", f"duplicate input {item['path']}")
        recorded[item["path"]] = item["sha256"]
    for path, pin in recorded.items():
        try:
            actual = digest(repo.blob(path))
        except evidence.ValidationError as exc:
            c.reasons.extend(exc.reasons)
            continue
        if actual != pin:
            c.fail("screen.inputs", f"{path}: recorded sha256 {pin} != {actual} at the source commit")
    for path, pin in REGISTERED_SHA256.items():
        if recorded.get(path) != pin:
            c.fail("screen.inputs", f"{path}: not pinned to its registered sha256 {pin}")
    required = {REGISTRATION, CONTRACT, LEDGER, PP07, VERIFY, HOMOTOPY, TRANSACTIONAL, *LIMITS}
    required |= {p for p in tracked if PROFILE_PATH.match(p)}
    for sources, _ in registered_cases().values():
        required |= set(sources)
    for path in sorted(required - set(recorded)):
        c.fail("screen.inputs", f"{path}: read under the registered rules but not pinned")
    rust = sorted(p for p in tracked if re.match(r"^crates/.*\.rs$", p))
    manifest = doc["rust_sources"]
    if [m[0] for m in manifest] != rust:
        c.fail("screen.rust_sources", "the scanned source list differs from crates/**/*.rs at the source commit")
    else:
        bad = [m[0] for m in manifest if digest(repo.blob(m[0])) != m[1]]
        if bad:
            c.fail("screen.rust_sources", f"{len(bad)} sources not pinned (first {bad[0]})")
    # closed arm lists
    for i, cand in enumerate(doc["candidates"]):
        cid = cand["id"]
        if cid not in CLOSED:
            continue  # an unregistered candidate is gate item 1
        if cand["S3"]["applicable_arms"] != CLOSED[cid]:
            c.fail(f"screen.candidates[{i}].S3.applicable_arms",
                   f"{cand['S3']['applicable_arms']} != registered closed list {CLOSED[cid]}")
        used = {a["arm"] for a in cand["S3"]["arms"]}
        configs = {v["config"] for v in cand["S1"]["f_values"]}
        for pc in cand["S3"]["per_case"]:
            used |= set(pc["binary_arms"])
            for r in pc["rtols"]:
                used |= {x["arm"] for x in r["ranking"]}
                configs |= {x["config"] for x in r["ranking"]}
                if r["comparator"] is not None:
                    used.add(r["comparator"]["arm"])
                    configs.add(r["comparator"]["config"])
        used |= {ARM_OF.get(cfg, f"unregistered configuration {cfg}") for cfg in configs}
        for arm in sorted(used - set(CLOSED[cid])):
            c.fail(f"screen.candidates[{i}].S3", f"arm {arm!r} outside the registered closed list of {cid}")
    c.done()


# ------------------------------------------------------------------------------------ independent derivation
def ledger_of(repo: Repo) -> list[dict]:
    return [evidence.loads_strict(line.encode()) for line in repo.text(LEDGER).splitlines() if line.strip()]


def derive_profiles(repo: Repo, ledger: list[dict]):
    rows, used = [], []
    need = ("arm", "problem", "rtol", "attempts", "counters", "ir_per_run", "success", "callgrind_deterministic",
            "categories_by_file", "categories_by_function")
    for path in sorted(p for p in repo.tracked() if PROFILE_PATH.match(p)):
        raw = repo.blob(path)
        sha = digest(raw)
        doc = evidence.loads_strict(raw)
        lids = [r["id"] for r in ledger if r.get("outputs_sha256", {}).get(path) == sha]
        m = re.search(r"_(\d{8})/", path)
        binary = doc.get("binary_sha256") if isinstance(doc, dict) else None
        rec = {"path": path, "sha256": sha, "ledger_ids": lids, "binary_sha256": binary,
               "node_date": m.group(1) if m else "00000000", "usable": False, "reason": None, "usable_entries": 0}
        entries = doc.get("entries") if isinstance(doc, dict) else None
        if not lids:
            rec["reason"] = "not an output of any ledger row with this SHA-256"
        elif not isinstance(binary, str):
            rec["reason"] = f"no binary_sha256 (schema {doc.get('schema') if isinstance(doc, dict) else None})"
        elif not isinstance(entries, list) or not entries or any(
                not isinstance(e, dict) or any(k not in e for k in need) for e in entries):
            rec["reason"] = f"entries without per-entry arm/categories/counters (schema {doc.get('schema')})"
        else:
            rec["usable"] = True
            for i, e in enumerate(entries):
                if e["success"] is not True or e["callgrind_deterministic"] is not True:
                    continue
                if "problem-user-code" not in e["categories_by_file"]:
                    continue
                fn = e["categories_by_function"].get("problem-user-code")
                used.append({"file": path, "index": i, "binary": binary, "date": rec["node_date"], "lids": lids,
                             "config": e["arm"], "problem": e["problem"], "rtol": e["rtol"],
                             "ir": e["ir_per_run"], "attempts": e["attempts"],
                             "solves": e["counters"]["linear_solves"],
                             "share_file": e["categories_by_file"]["problem-user-code"]["share"],
                             "share_function": 0.0 if fn is None else fn["share"]})
                rec["usable_entries"] += 1
        rows.append(rec)
    return rows, used


def applies(cid: str, arm: str, n) -> bool:
    cond = CONDITION.get((cid, arm))
    if cond is None:
        return True
    if n is None:
        return False
    return (cond == "n<=8" and n <= 8) or (cond == "n==1" and n == 1) or (cond == "n>64" and n > 64)


def mentions(value, problem: str) -> bool:
    stack = [value]
    while stack:
        v = stack.pop()
        if isinstance(v, str):
            if v == problem:
                return True
        elif isinstance(v, dict):
            if problem in v:
                return True
            stack.extend(v.values())
        elif isinstance(v, list):
            stack.extend(v)
    return False


def evidence_for(repo: Repo, ledger: list[dict], e: dict):
    for lid in sorted(e["lids"]):
        row = [r for r in ledger if r["id"] == lid][0]
        for path in sorted(row["outputs_sha256"]):
            if path == e["file"] or not path.endswith(".json") or PROFILE_PATH.match(path):
                continue
            raw = repo.blob(path)
            if digest(raw) == row["outputs_sha256"][path] and mentions(evidence.loads_strict(raw), e["problem"]):
                return {"ledger_id": lid, "path": path, "sha256": digest(raw)}
    return None


def derive_case(repo, ledger, used, cid, case, n):
    pool = [e for e in used if e["problem"] == case and e["config"] in ARM_OF and ARM_OF[e["config"]] in CLOSED[cid]
            and applies(cid, ARM_OF[e["config"]], n)]
    if not pool:
        return None
    arms_by_binary, date_by_binary = {}, {}
    for e in pool:
        arms_by_binary.setdefault(e["binary"], set()).add(ARM_OF[e["config"]])
        date_by_binary[e["binary"]] = max(date_by_binary.get(e["binary"], "00000000"), e["date"])
    best = None
    for b in sorted(arms_by_binary):  # ascending hash: a later equal key replaces the earlier (larger hash wins)
        key = (len(arms_by_binary[b]), date_by_binary[b])
        if best is None or key >= best[0]:
            best = (key, b)
    binary = best[1]
    rtols, winners = [], []
    for rtol in sorted({e["rtol"] for e in pool if e["binary"] == binary}, reverse=True):
        cand = sorted((e for e in pool if e["binary"] == binary and e["rtol"] == rtol),
                      key=lambda e: (e["ir"], e["config"], e["file"], e["index"]))
        ranking, chosen = [], None
        for e in cand:
            acc = evidence_for(repo, ledger, e)
            ranking.append({"config": e["config"], "arm": ARM_OF[e["config"]], "ir_per_run": e["ir"],
                            "file": e["file"], "index": e["index"], "ledger_id": min(e["lids"]),
                            "accuracy_evidence": acc})
            if chosen is None and acc is not None:
                chosen = e
        comparator = None
        if chosen is not None:
            comparator = {"config": chosen["config"], "arm": ARM_OF[chosen["config"]], "file": chosen["file"],
                          "index": chosen["index"]}
            winners.append(chosen)
        rtols.append({"rtol": rtol, "ranking": ranking, "comparator": comparator})
    return {"case": case, "binary": binary, "binary_arms": sorted(arms_by_binary[binary]), "rtols": rtols}, winners


def witness_facts(text: str) -> dict:
    start = text.find("fn witness(&self, problem: &QuadraticStageProblem, gamma: f64)")
    end = text.find("\n    }\n", start) if start >= 0 else -1
    body = text[start:end] if start >= 0 and end >= 0 else ""
    m = re.search(r'const SMALL_WITNESS_DIMENSION_LIMIT: &str = "([^"]*)";', text)
    return {"diagonal_branch": "InverseWitness::diagonal(" in body, "small_branch": "InverseWitness::small(" in body,
            "small_limit_message": None if m is None else m.group(1)}


def impls_of(text: str) -> list[dict]:
    out = []
    for m in re.finditer(r"impl\s+Q2CertificateSource\s+for\s+([A-Za-z_][A-Za-z0-9_]*)", text):
        stop = text.find("\n}\n", m.end())
        block = text[m.start():] if stop < 0 else text[m.start():stop]
        if "ModelBinding::GeneratedFromModel" in block:
            kind = "GeneratedFromModel"
        elif re.search(r"fn binding\(&self\) -> ModelBinding \{\s*self\.\w+\.binding\(\)", block):
            kind = "delegated"
        else:
            kind = "SampledAgreement"
        out.append({"type": m.group(1), "binding": kind})
    return out


def derive(repo: Repo) -> dict:
    """The checker's own derivation of every screened quantity (compared with the screen in gate item 2)."""
    ledger = ledger_of(repo)
    pp07 = repo.json(PP07)
    verify = repo.json(VERIFY)
    profiles, used = derive_profiles(repo, ledger)
    matches = []
    for path in sorted(p for p in repo.tracked() if re.match(r"^crates/.*\.rs$", p)):
        for no, line in enumerate(repo.blob(path).decode("utf-8", errors="replace").splitlines(), start=1):
            if re.search(r"\bExpensiveClientDeclaration\b", line):
                matches.append([path, no])
    struct = re.search(r"pub struct HomotopyWorkLedger \{(.*?)\n\}", repo.text(HOMOTOPY), re.S)
    fields = re.findall(r"pub\s+([a-z_][a-z0-9_]*)\s*:", struct.group(1)) if struct else []
    terms = {t: [f for f in fields if re.search(p, f)] for t, p in TERMS}
    work = {"source": HOMOTOPY, "fields": fields, "terms": terms,
            "missing_terms": [t for t, _ in TERMS if not terms[t]]}
    facts = witness_facts(repo.text(TRANSACTIONAL))
    seq = verify["arms"]["sequential-jf-gmres"]["cases"]
    p8 = {x["case_id"]: x for x in verify["arms"]["transactional-certified-p8"]["cases"]}
    mult = {x["case_id"]: p8[x["case_id"]]["work"]["rhs_evaluations"] / x["work"]["rhs_evaluations"] for x in seq}
    certs = {k: {"certificate_attempts": v["details"]["certificate_attempts"],
                 "certificate_admissions": v["details"]["certificate_admissions"]} for k, v in p8.items()}
    l0008 = {"rhs_multiplier_p8_over_sequential": mult,
             "rhs_multiplier_median": sorted(mult.values())[len(mult) // 2],
             "p8_certificates": certs, "reference_arm": "sequential-jf-gmres"}
    seq_rows = [{"case": x["case_id"], "arm": "sequential-jf-gmres", "file": VERIFY, "index": None, "rtol": None,
                 "attempts": x["details"]["attempts"], "linear_solves": x["work"]["linear_solves"],
                 "solves_per_attempt": x["work"]["linear_solves"] / x["details"]["attempts"]} for x in seq]
    cases = registered_cases()
    candidates = []
    for cid in IDS:
        sources, case_list = cases[cid]
        text = "\n".join(repo.text(p) for p in sources)
        derived = []
        if cid != "K1":
            for case, n, _ in case_list:
                got = derive_case(repo, ledger, used, cid, case, n)
                if got is not None:
                    derived.append(got)
        # S1
        fv = []
        for comp, winners in derived:
            for config in dict.fromkeys(w["config"] for w in winners):
                fv += [{"case": comp["case"], "config": config, "file": e["file"], "index": e["index"],
                        "rtol": e["rtol"], "share_file": e["share_file"], "share_function": e["share_function"]}
                       for e in used if e["config"] == config and e["problem"] == comp["case"]]
        fv.sort(key=lambda v: (v["case"], v["config"], v["file"], v["index"]))
        fmin = min([v["share_file"] for v in fv]) if fv else None
        own_matches = [m for m in matches if m[0] in sources]
        a = len(own_matches) > 0
        b = None if fmin is None else (fmin >= F_MIN)
        s1 = {"declaration_matches": own_matches,
              "batch_rhs_references": len(re.findall(r"\b(BatchRhsFn|rhs_batch)\b", text)), "a": a,
              "f_status": "measured" if fv else "unmeasured", "f_values": fv, "f_min": fmin,
              "f_min_function_view": min([v["share_function"] for v in fv]) if fv else None, "b": b,
              "pass": a and b is not False,
              "reasons": ([] if a else ["no_expensive_client_declaration"]) + (["f_below_f_min"] if b is False else [])}
        # S2
        tokens = [t for t in PP07_TOKENS if t in text]
        if cid == "K1":
            q1 = pp07["q1_solves"]
            s2 = {"source": "pp07", "rows": [], "q1_solves": q1,
                  "pp07": "PP07 ABSTAIN preserved" if q1 == 0 else "PP07 shows shifted solves",
                  "static_solver_tokens": tokens, "pass": False,
                  "reasons": ["pp07_q1_solves_zero" if q1 == 0 else "pp07_inconsistent"]}
        elif cid == "K2":
            good = all(r["solves_per_attempt"] >= 2.0 for r in seq_rows)
            s2 = {"source": "l0008", "rows": seq_rows, "q1_solves": None, "pp07": None,
                  "static_solver_tokens": tokens, "pass": good,
                  "reasons": [] if good else ["solves_per_attempt_below_2"]}
        else:
            rows = []
            for comp, _ in derived:
                for r in comp["rtols"]:
                    if r["comparator"] is None:
                        continue
                    e = [x for x in used if x["file"] == r["comparator"]["file"]
                         and x["index"] == r["comparator"]["index"]][0]
                    rows.append({"case": comp["case"], "arm": e["config"], "file": e["file"], "index": e["index"],
                                 "rtol": e["rtol"], "attempts": e["attempts"], "linear_solves": e["solves"],
                                 "solves_per_attempt": e["solves"] / e["attempts"]})
            good = len(rows) > 0 and all(r["solves_per_attempt"] >= 2.0 for r in rows)
            s2 = {"source": "profiles" if rows else "none", "rows": rows, "q1_solves": None, "pp07": None,
                  "static_solver_tokens": tokens, "pass": good,
                  "reasons": [] if good else (["solves_per_attempt_below_2"] if rows else ["no_committed_counters"])}
        # S3
        arms = []
        for arm in CLOSED[cid]:
            hit = any(ARM_OF.get(e["config"]) == arm and e["problem"] == case and applies(cid, arm, n)
                      for e in used for case, n, _ in case_list)
            arms.append({"arm": arm, "condition": CONDITION.get((cid, arm)),
                         "status": "measured" if hit else "unmeasured",
                         "reason": None if hit else "no_committed_profile"})
        covered = {comp["case"] for comp, _ in derived}
        missing = [] if cid == "K1" else sorted({case for case, _, _ in case_list} - covered)
        if cid == "K1":
            reasons = ["no_applicable_arm"]
        elif not derived:
            reasons = ["no_measured_applicable_arm"] + (["jf_gmres_only_where_direct_applies"] if cid == "K2" else [])
        elif any(r["comparator"] is None for comp, _ in derived for r in comp["rtols"]):
            reasons = ["no_accuracy_evidence"]
        elif missing:
            reasons = ["no_measured_applicable_arm"]
        else:
            reasons = []
        s3 = {"applicable_arms": list(CLOSED[cid]), "arms": arms, "per_case": [comp for comp, _ in derived],
              "cases_without_comparator": missing,
              "l0008_reference_arm": "sequential-jf-gmres" if cid == "K2" else None,
              "pass": not reasons, "reasons": reasons}
        # S4
        impls = impls_of(text)
        ctype = CLIENT_TYPE.get(cid)
        mine = [i for i in impls if ctype is None or i["type"] == ctype]
        if not mine:
            binding = None
        elif all(i["binding"] == "GeneratedFromModel" for i in mine):
            binding = "GeneratedFromModel"
        else:
            binding = "SampledAgreement"
        caps = []
        for case, n, structure in case_list:
            cap = None
            if mine:
                diagonal_ok = structure == "diagonal" and facts["diagonal_branch"]
                small_ok = (n is not None and n <= 2 and facts["small_branch"]
                            and facts["small_limit_message"] == "small witness needs n <= 2")
                cap = "Available" if (diagonal_ok or small_ok) else "DimensionCutoff"
            caps.append({"case": case, "capability": cap})
        available = all(x["capability"] == "Available" for x in caps)
        s4a = binding == "GeneratedFromModel" and available
        if not mine:
            r4 = ["no_q2_certificate_source"]
        else:
            r4 = (["binding_not_generated_from_model"] if binding != "GeneratedFromModel" else []) \
                + ([] if available else ["capability_not_available"])
        s4 = {"impls": impls, "client_type": ctype, "binding": binding, "capabilities": caps,
              "capability_evidence": {"source": VERIFY, "p8_certificates": certs} if cid == "K2" else None,
              "a": s4a, "b": True, "obligations": work["missing_terms"], "pass": s4a, "reasons": r4}
        meets = {"S1": s1["pass"], "S2": s2["pass"], "S3": s3["pass"], "S4": s4["pass"]}
        candidates.append({"id": cid, "client": None, "sources": sources,
                           "cases": [{"case": cs, "dimension": n, "structure": st} for cs, n, st in case_list],
                           "S1": s1, "S2": s2, "S3": s3, "S4": s4, "meets": meets,
                           "all": all(meets.values()), "lead": (not a) and fmin is not None and fmin >= F_MIN})
    applicable = [e for e in used if e["config"] in ARM_OF]
    return {
        "profiles": profiles, "declaration_scan": {"pattern": r"\bExpensiveClientDeclaration\b", "matches": matches},
        "work_ledger": work, "default_witness": facts,
        "stated_limits": {p: phrase in repo.text(p) for p, phrase in LIMITS.items()},
        "pp07": {k: pp07[k] for k in ("q1_solves", "q2_multi_shift_starts", "q2_jet_admissible_starts", "verdict")},
        "l0008": l0008, "candidates": candidates,
        "max_file": max([e["share_file"] for e in applicable]) if applicable else None,
        "max_function": max([e["share_function"] for e in applicable]) if applicable else None,
        "function_hits": [{"file": e["file"], "index": e["index"], "config": e["config"], "problem": e["problem"],
                           "rtol": e["rtol"], "share_function": e["share_function"], "share_file": e["share_file"]}
                          for e in applicable if e["share_function"] >= F_MIN],
    }


def rule(rows: list[dict]) -> dict:
    """The registered decision rule."""
    ok = [r for r in rows if r["meets"]["S1"] and r["meets"]["S2"] and r["meets"]["S3"] and r["meets"]["S4"]]
    chosen = None
    if len(ok) == 1:
        chosen = ok[0]["id"]
    elif len(ok) > 1:
        def rank(r):
            f = r["S1"]["f_min"]
            return (0, 0.0) if f is None else (1, f)
        top = max(rank(r) for r in ok)
        best = [r["id"] for r in ok if rank(r) == top]
        chosen = best[0] if len(best) == 1 else None
    return {"outcome": "ABSTAIN" if chosen is None else f"SELECTED({chosen})", "selected": chosen,
            "meeting_all": [r["id"] for r in ok], "leads": [r["id"] for r in rows if r["lead"]]}


def same(a, b) -> bool:
    if type(a) is not type(b):
        return False
    if isinstance(a, dict):
        return a.keys() == b.keys() and all(same(a[k], b[k]) for k in a)
    if isinstance(a, list):
        return len(a) == len(b) and all(same(x, y) for x, y in zip(a, b))
    return a == b


def differences(a, b, path: str = "", out=None, limit: int = 200) -> list[str]:
    out = [] if out is None else out
    if len(out) >= limit:
        return out
    if type(a) is not type(b):
        out.append(f"{path}: screen {a!r:.80} vs checker {b!r:.80}")
    elif isinstance(a, dict):
        if a.keys() != b.keys():
            out.append(f"{path}: keys differ")
        for k in sorted(a.keys() & b.keys()):
            differences(a[k], b[k], f"{path}.{k}", out, limit)
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append(f"{path}: length {len(a)} vs {len(b)}")
        for i, (x, y) in enumerate(zip(a, b)):
            differences(x, y, f"{path}[{i}]", out, limit)
    elif a != b:
        out.append(f"{path}: screen {a!r:.80} vs checker {b!r:.80}")
    return out


# ------------------------------------------------------------------------------------------------- gate
def evaluate(raw: bytes, root: Path, running_checker: bytes, tree_status_now: str) -> tuple[str, dict]:
    """Returns (verdict, report); raises evidence.ValidationError for INVALID."""
    doc = evidence.loads_strict(raw)
    if not isinstance(doc, dict) or not isinstance(doc.get("provenance"), dict) or not isinstance(
            doc["provenance"].get("source_commit"), str):
        raise evidence.ValidationError(["screen: no provenance.source_commit"])
    repo = Repo(root, doc["provenance"]["source_commit"])
    try:
        validate(doc, repo, running_checker, tree_status_now)
        mine = derive(repo)
        contract_exists = repo.git("cat-file", "-e", f"{repo.commit}:{CONTRACT}", check=False).returncode == 0
    finally:
        repo.close()
    rows = doc["candidates"]
    by_id = {r["id"]: r for r in rows}
    ids = [r["id"] for r in rows]
    item1 = {"pass": ids == IDS, "candidates": ids}
    diffs = []
    for key in ("profiles", "declaration_scan", "work_ledger", "default_witness", "stated_limits", "pp07", "l0008"):
        diffs += differences(doc[key], mine[key], key)
    for cand in mine["candidates"]:
        row = by_id.get(cand["id"])
        if row is None:
            diffs.append(f"{cand['id']}: missing")
            continue
        diffs += differences(row, dict(cand, client=row["client"]), cand["id"])
    rep = doc["reported"]
    for key, value in (("max_share_file_all_applicable_entries", mine["max_file"]),
                       ("max_share_function_all_applicable_entries", mine["max_function"]),
                       ("entries_function_view_at_or_above_f_min", mine["function_hits"])):
        diffs += differences(rep[key], value, f"reported.{key}")
    item2 = {"pass": not diffs, "differences": diffs[:40], "differences_total": len(diffs)}
    k1 = by_id.get("K1", {}).get("S2", {})
    git_only = all(cmd and cmd[0] == "git" for cmd in doc["provenance"]["executed_commands"])
    item3 = {"pass": (k1.get("q1_solves") == 0 and k1.get("pp07") == "PP07 ABSTAIN preserved"
                      and mine["pp07"]["q1_solves"] == 0 and rep["artificial_workloads"] == []
                      and rep["reruns"] == [] and doc["provenance"]["solver_runs"] == 0 and git_only),
             "k1_q1_solves": k1.get("q1_solves"), "k1_note": k1.get("pp07"), "executed_commands_git_only": git_only,
             "solver_runs": doc["provenance"]["solver_runs"], "artificial_workloads": rep["artificial_workloads"],
             "reruns": rep["reruns"]}
    decision = rule(mine["candidates"])
    item4 = {"pass": same(doc["decision"], decision) and contract_exists, "screen_decision": doc["decision"],
             "rule_decision": decision, "contract_at_source_commit": contract_exists}
    gates = {"1_rows": item1, "2_recomputed": item2, "3_k1_and_no_artificial_workload": item3,
             "4_decision_rule_and_contract": item4}
    verdict = "PASS" if all(g["pass"] for g in gates.values()) else "FAIL"
    table = [{"id": r["id"], "S1": r["meets"]["S1"], "S2": r["meets"]["S2"], "S3": r["meets"]["S3"],
              "S4": r["meets"]["S4"], "f_min": r["S1"]["f_min"], "f_min_function_view": r["S1"]["f_min_function_view"],
              "f_status": r["S1"]["f_status"], "lead": r["lead"],
              "reasons": {k: r[k]["reasons"] for k in ("S1", "S2", "S3", "S4")},
              "comparators": sorted({x["comparator"]["config"] for pc in r["S3"]["per_case"] for x in pc["rtols"]
                                     if x["comparator"] is not None})}
             for r in mine["candidates"]]
    report = {"gates": gates, "verdict": verdict, "decision": decision["outcome"], "leads": decision["leads"],
              "candidates": table, "f_min_threshold": F_MIN, "source_commit": doc["provenance"]["source_commit"],
              "ht02_obligations": mine["work_ledger"]["missing_terms"],
              "reported": {"max_share_file_all_applicable_entries": mine["max_file"],
                           "max_share_function_all_applicable_entries": mine["max_function"],
                           "entries_function_view_at_or_above_f_min": len(mine["function_hits"]),
                           "l0008_rhs_multiplier_median": mine["l0008"]["rhs_multiplier_median"]}}
    return verdict, report


def run(screen: Path, output: Path, root: Path) -> int:
    if output.exists():
        raise SystemExit(f"immutable output exists: {output}")
    ignore = []
    for p in (screen, output):
        try:
            ignore.append(str(p.resolve().relative_to(root.resolve())))
        except ValueError:
            pass
    raw = b""
    try:
        raw = screen.read_bytes()
        status = own_tree_status(root, ignore)
        verdict, report = evaluate(raw, root, Path(__file__).read_bytes(), status)
    except (evidence.ValidationError, OSError) as exc:
        error = exc if isinstance(exc, evidence.ValidationError) else evidence.ValidationError([f"screen: {exc}"])
        return evidence.emit_invalid(output, SCHEMA, error, {"screen": screen})
    out = {"schema": SCHEMA, "evidence_schema": evidence.VERSION,
           "inputs": {"screen": {"path": str(screen), "sha256": digest(raw)}},
           "checker_tree_status": status,
           "claim_scope": "Design screen and client selection only: static scans and committed files, no solver run, "
                          "no timing, no speed or span claim; a SELECTED client is a client worth piloting, not "
                          "evidence that parallel homotopy helps it; no default promotion.",
           **report}
    output.write_text(json.dumps(out, indent=1, allow_nan=False) + "\n")
    print(f"verdict {verdict}; decision: {report['decision']}; leads {report['leads']}")
    for name, g in report["gates"].items():
        print(f"  {name}: {'pass' if g['pass'] else 'FAIL'}")
    return 0 if verdict == "PASS" else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--screen", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args()
    return run(args.screen, args.output, args.repo_root)


if __name__ == "__main__":
    sys.exit(main())
