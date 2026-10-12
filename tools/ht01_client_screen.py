#!/usr/bin/env python3
"""Client screen of research node HT01 (research/ht01_homotopy_client_contract_20261011, see its PREREGISTRATION.md).

Writes SCREEN.json: per candidate K1-K8 and per criterion S1-S4, the values or typed reasons, from static scans and
committed files only. Every file is read from the git object store at the source commit (HEAD, which must be clean)
and pinned by its SHA-256. No cargo, no solver run, no new profile: the only commands executed are git commands, and
they are recorded. The decision rule is applied here and again, independently, by tools/ht01_selection_check.py.

Registered rules and the interpretations fixed before the recorded run (also in the checker's docstring):

- Inputs: the registration, research/LEDGER.jsonl, PP07 RESULTS.json (L-0074), the L-0008 VERIFY.json (HOM-06
  campaign counters), every git-tracked research/*/PROFILE*.json, homotopy.rs, transactional_q1_q2.rs, the
  contract document, the candidates' sources, and every git-tracked crates/**/*.rs (static scans).
- Profile entries (I1): a profile is usable when its entries carry arm, problem, rtol, attempts, counters,
  ir_per_run, success, callgrind_deterministic, categories_by_file and categories_by_function, and the file is an
  output of a ledger row with the same SHA-256. An entry is usable when success and callgrind_deterministic hold.
- f (I2): the share of the problem-user-code category in categories_by_file (source-file attribution, the view of the
  registration's disclosed prior shares). The function view (categories_by_function) is recorded next to it and
  reported only: tools/speed_profile.py documents that a driver monomorphized into a CLI function
  (rodas5p::stiff_benchmark::run_small_full) is charged to problem-user-code in that view.
- Arm configurations (I3): each registered arm is the set of committed profile arms of that native driver, opt-in
  options included: small = rodas5p-fast-small{,-val,-land,-ovh,-static,-static-stages,-static-ovh};
  dense v2 = rodas5p-fast{,-val,-land,-ovh}; colext64 = rodas5p-fast-colext64; banded = rodas5p-fast-banded{,-slices};
  matrix-free U-form = rodas5p-mf-legacy. Every other profile arm (routed, colext, batch lanes, rodas5p, hairer-rodas)
  is outside the closed lists and never used.
- Comparison binary (I4): per case, the binary (binary_sha256) whose usable entries cover the most applicable arms;
  ties go to the newest node date (directory suffix), then to the larger binary hash. Same-binary Ir only.
- Comparator (I5): per case and rtol in that binary, the applicable configurations ranked by ir_per_run (ties by
  configuration name); the comparator at that rtol is the first one with a ledger row (the profile file is an output
  of the row with the same SHA-256) and accuracy evidence: another output of the same row, with matching SHA-256,
  whose JSON names the case's problem id. This is a membership check of recorded identity/accuracy evidence, not a
  re-verification of accuracy.
- f of a case (I6): every usable entry, in any committed profile and at any rtol, of each comparator configuration of
  the case; f_case = min. f of a candidate = min over its measured cases; unmeasured when no case is measured.
- S2 (I7): solves per attempt = counters.linear_solves / attempts of each comparator entry of the comparison binary
  (K2: the L-0008 sequential arm; K1: PP07 q1_solves); pass when every row has solves per attempt >= 2 (each
  attempt's stage solves become the independent tasks of a homotopy round). No committed counters: not met.
- S4a (I8): `impl Q2CertificateSource for X` in the candidate's sources, X the registered client type where the
  registration names one (K2: QuadraticModel); binding GeneratedFromModel when the impl block names
  ModelBinding::GeneratedFromModel, "delegated" when its binding() forwards to a wrapped model's (reported), else
  SampledAgreement (the trait default); capability Available when the default witness covers the case's structure
  (diagonal J at any n, or n <= 2). S4b: HomotopyWorkLedger fields
  matched to the registered terms; missing terms are HT02 obligations, not failures.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
if str(Path(__file__).resolve().parent) not in sys.path:
    sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence_schema_v2 as evidence  # noqa: E402  (strict JSON parsing, re-audit AS03)

SCHEMA = "vigilode-ht01-screen-v1"
NODE_DIR = "research/ht01_homotopy_client_contract_20261011"
REGISTRATION = f"{NODE_DIR}/PREREGISTRATION.md"
SCREEN_TOOL = "tools/ht01_client_screen.py"
CHECKER_TOOL = "tools/ht01_selection_check.py"
CONTRACT_DOC = "docs/HOMOTOPY_EXPENSIVE_CLIENT_CONTRACT.md"
LEDGER = "research/LEDGER.jsonl"
PP07_RESULTS = "research/pp07_fourier_shared_action_20261004/RESULTS.json"
L0008_VERIFY = "research/r3_matched_accuracy_hom06_20261001/VERIFY.json"
HOMOTOPY_RS = "crates/rodas5p-integrators/src/homotopy.rs"
TRANSACTIONAL_RS = "crates/rodas5p-integrators/src/transactional_q1_q2.rs"
CHART_RS = "crates/rodas5p-integrators/src/chart_transport.rs"
JET_RS = "crates/rodas5p-core/src/shared_shift_jet.rs"
PROFILE_RE = re.compile(r"^research/[^/]+/PROFILE[^/]*\.json$")
RUST_RE = re.compile(r"^crates/.*\.rs$")

F_MIN = 0.348
R_MULTIPLIER = 5
THREADS_P = 8
REQUIRED_SPEEDUP = 1.15
F_MIN_UNROUNDED = (1.0 - 1.0 / REQUIRED_SPEEDUP) / (1.0 - R_MULTIPLIER / THREADS_P)
MIN_SOLVES_PER_ATTEMPT = 2.0

ARM_CONFIGS = {
    "small": ("rodas5p-fast-small", "rodas5p-fast-small-val", "rodas5p-fast-small-land", "rodas5p-fast-small-ovh",
              "rodas5p-fast-small-static", "rodas5p-fast-small-static-stages", "rodas5p-fast-small-static-ovh"),
    "dense v2": ("rodas5p-fast", "rodas5p-fast-val", "rodas5p-fast-land", "rodas5p-fast-ovh"),
    "colext64": ("rodas5p-fast-colext64",),
    "banded": ("rodas5p-fast-banded", "rodas5p-fast-banded-slices"),
    "matrix-free U-form": ("rodas5p-mf-legacy",),
}
CONFIG_ARM = {config: arm for arm, configs in ARM_CONFIGS.items() for config in configs}

# Closed applicable-arm lists (registration, S3 table) and their size conditions.
CLOSED_ARMS = {
    "K1": [],
    "K2": ["small", "dense v2", "matrix-free U-form"],
    "K3": ["small", "dense v2"],
    "K4": ["small", "dense v2"],
    "K5": ["small", "dense v2", "matrix-free U-form"],
    "K6": ["dense v2", "colext64", "banded", "matrix-free U-form"],
    "K7": ["dense v2", "colext64", "banded", "matrix-free U-form"],
    "K8": ["small", "dense v2"],
}
ARM_CONDITIONS = {("K2", "small"): "n<=8", ("K3", "small"): "n<=8", ("K4", "small"): "n==1",
                  ("K6", "colext64"): "n>64", ("K8", "small"): "n<=8"}

BRUSSELATOR_CELLS = (30, 40, 50, 160, 200, 500)
CORPUS_FAMILIES = ("robertson-ramped", "hires-ramped", "van-der-pol-ramped", "rotating-nonnormal",
                   "nonautonomous-stiff-forcing", "semilinear-advection-diffusion-ramped")
CANDIDATES = [
    {"id": "K1", "client": "Fourier path client (PP05/PP06)",
     "sources": ["crates/rodas5p-integrators/src/fourier_path_candidate.rs",
                 "crates/rodas5p-integrators/src/fourier_path_certificate.rs"],
     "cases": [{"case": c, "dimension": 4, "structure": "dense"}
               for c in ("fast", "forty", "forty_tight", "negative", "resonance", "unit")]},
    {"id": "K2", "client": "R4 / HOM-06 / R-NEXT-06 quadratic family (QuadraticModel)",
     "sources": [TRANSACTIONAL_RS],
     "cases": [{"case": f"diagonal-quadratic-n{n}", "dimension": n, "structure": "diagonal"}
               for n in (1, 2, 4, 8, 16)]},
    {"id": "K3", "client": "Bateman two-timescale real client (feature audit2-bateman-authority)",
     "sources": ["crates/rodas5p-integrators/src/audit2_bateman_real_client_research.rs"],
     "cases": [{"case": "bateman-two-timescale-parent-stable-daughter-v1", "dimension": 4, "structure": "dense"}]},
    {"id": "K4", "client": "Darboux chart client y = x^2 (w + 1/kappa) (L-0043, L-0048)",
     "sources": [CHART_RS],
     "cases": [{"case": "darboux-chart", "dimension": 1, "structure": "scalar"}]},
    {"id": "K5", "client": "robertson, hires, van-der-pol-mu1000 (stiff_benchmark)",
     "sources": ["crates/rodas5p-cli/src/stiff_benchmark.rs"],
     "cases": [{"case": "robertson", "dimension": 3, "structure": "dense"},
               {"case": "hires", "dimension": 8, "structure": "dense"},
               {"case": "van-der-pol-mu1000", "dimension": 2, "structure": "dense"}]},
    {"id": "K6", "client": "brusselator-1d-N (stiff_benchmark)",
     "sources": ["crates/rodas5p-cli/src/stiff_benchmark.rs"],
     "cases": [{"case": f"brusselator-1d-{c}", "dimension": 2 * c, "structure": "banded"}
               for c in BRUSSELATOR_CELLS]},
    {"id": "K7", "client": "scientific corpus v2 calibration (6 families x n in {96, 384, 1536})",
     "sources": ["crates/rodas5p-integrators/src/scientific_corpus_v2.rs"],
     "cases": [{"case": f"{f}-n{n}", "dimension": n, "structure": "dense"}
               for f in CORPUS_FAMILIES for n in (96, 384, 1536)]},
    {"id": "K8", "client": "problems.rs fixtures",
     "sources": ["crates/rodas5p-integrators/src/problems.rs"],
     "cases": [{"case": c, "dimension": None, "structure": "fixture"}
               for c in ("scalar_linear_problem", "prothero_robinson_problem",
                         "oscillatory_prothero_robinson_problem", "manufactured_vector_problem",
                         "constant_affine_mass_problem", "manufactured_mass_nonlinear_problem",
                         "complex_dahlquist_problem", "semilinear_advection_diffusion_problem")]},
]

# The registered client type whose Q2CertificateSource impl decides S4a (registration: K2 "(QuadraticModel)").
CLIENT_TYPES = {"K2": "QuadraticModel"}

DECLARATION_RE = re.compile(r"\bExpensiveClientDeclaration\b")
BATCH_RE = re.compile(r"\b(BatchRhsFn|rhs_batch)\b")
SOLVER_TOKENS = ["LuFactorization", "solve_rows", "direct_solve", "solve_gmres", "solve_gcrodr",
                 "solve_lgmres", "shared_shift_jet", "LinearOperator", "inverse("]  # PP07's static scan list
Q2_IMPL_RE = re.compile(r"impl\s+Q2CertificateSource\s+for\s+([A-Za-z_][A-Za-z0-9_]*)")
LEDGER_STRUCT_RE = re.compile(r"pub struct HomotopyWorkLedger \{(.*?)\n\}", re.S)
FIELD_RE = re.compile(r"pub\s+([a-z_][a-z0-9_]*)\s*:")
LEDGER_TERMS = [("setup", r"factorization|pool_creation"), ("rounds", r"round"), ("max_task_per_round", r"max_task"),
                ("barriers", r"barrier"), ("certificate", r"certificate"), ("rejected_candidates", r"reject"),
                ("fallback", r"fallback"), ("dispatch", r"batch|pool"), ("peak_bytes", r"peak|bytes")]
REQUIRED_ENTRY_KEYS = ("arm", "problem", "rtol", "attempts", "counters", "ir_per_run", "success",
                       "callgrind_deterministic", "categories_by_file", "categories_by_function")
CATEGORY = "problem-user-code"
LIMIT_PHRASES = {CHART_RS: "No integrator calls this module",
                 JET_RS: "execute shifts in parallel or claim a"}


# ----------------------------------------------------------------------------------------------- git
class Git:
    """Reads blobs at one commit through `git cat-file --batch`; every executed command is recorded."""

    def __init__(self, root: Path):
        self.root = root
        self.commands: list[list[str]] = []
        self._batch = None

    def run(self, *args: str) -> str:
        argv = ["git", *args]
        if argv not in self.commands:
            self.commands.append(argv)
        return subprocess.run(argv, cwd=self.root, check=True, capture_output=True, text=True).stdout

    def read(self, commit: str, path: str) -> bytes:
        if self._batch is None:
            argv = ["git", "cat-file", "--batch"]
            self.commands.append(argv)
            self._batch = subprocess.Popen(argv, cwd=self.root, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
        self._batch.stdin.write(f"{commit}:{path}\n".encode())
        self._batch.stdin.flush()
        header = self._batch.stdout.readline().decode().split()
        if len(header) != 3 or header[1] != "blob":
            raise SystemExit(f"not a blob at {commit}: {path} ({' '.join(header)})")
        data = self._batch.stdout.read(int(header[2]))
        self._batch.stdout.read(1)
        return data

    def close(self) -> None:
        if self._batch is not None:
            self._batch.stdin.close()
            self._batch.wait()
            self._batch.stdout.close()


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


# ------------------------------------------------------------------------------------------- loading
class Inputs:
    def __init__(self, git: Git, commit: str):
        self.git, self.commit = git, commit
        self.files: dict[str, bytes] = {}
        self.roles: dict[str, str] = {}

    def get(self, path: str, role: str) -> bytes:
        if path not in self.files:
            self.files[path] = self.git.read(self.commit, path)
            self.roles[path] = role
        return self.files[path]

    def json(self, path: str, role: str):
        return evidence.loads_strict(self.get(path, role))

    def text(self, path: str, role: str) -> str:
        return self.get(path, role).decode("utf-8")

    def record(self) -> list[dict]:
        return [{"role": self.roles[p], "path": p, "sha256": sha256(self.files[p])} for p in sorted(self.files)]


def load_ledger(inputs: Inputs) -> list[dict]:
    rows = []
    for line in inputs.text(LEDGER, "ledger").splitlines():
        if line.strip():
            rows.append(evidence.loads_strict(line.encode()))
    return rows


def ledger_rows_for(ledger: list[dict], path: str, digest: str) -> list[dict]:
    return [r for r in ledger if r.get("outputs_sha256", {}).get(path) == digest]


def node_date(path: str) -> str:
    m = re.search(r"_(\d{8})/", path)
    return m.group(1) if m else "00000000"


def names_problem(doc, problem: str) -> bool:
    if isinstance(doc, str):
        return doc == problem
    if isinstance(doc, dict):
        return any(k == problem or names_problem(v, problem) for k, v in doc.items())
    if isinstance(doc, list):
        return any(names_problem(v, problem) for v in doc)
    return False


def load_profiles(inputs: Inputs, tracked: list[str], ledger: list[dict]) -> tuple[list[dict], list[dict]]:
    """All tracked PROFILE*.json files (summary) and the usable entries (flat)."""
    summaries, entries = [], []
    for path in sorted(p for p in tracked if PROFILE_RE.match(p)):
        data = inputs.get(path, "profile")
        doc = evidence.loads_strict(data)
        digest = sha256(data)
        rows = ledger_rows_for(ledger, path, digest)
        summary = {"path": path, "sha256": digest, "ledger_ids": [r["id"] for r in rows],
                   "binary_sha256": doc.get("binary_sha256") if isinstance(doc, dict) else None,
                   "node_date": node_date(path), "usable": False, "reason": None, "usable_entries": 0}
        items = doc.get("entries") if isinstance(doc, dict) else None
        if not rows:
            summary["reason"] = "not an output of any ledger row with this SHA-256"
        elif not isinstance(summary["binary_sha256"], str):
            summary["reason"] = f"no binary_sha256 (schema {doc.get('schema') if isinstance(doc, dict) else None})"
        elif not (isinstance(items, list) and items and all(
                isinstance(e, dict) and all(k in e for k in REQUIRED_ENTRY_KEYS) for e in items)):
            summary["reason"] = (f"entries without per-entry arm/categories/counters "
                                 f"(schema {doc.get('schema')})")
        else:
            summary["usable"] = True
            for index, e in enumerate(items):
                if not (e["success"] is True and e["callgrind_deterministic"] is True):
                    continue
                if CATEGORY not in e["categories_by_file"]:
                    continue
                entries.append({
                    "file": path, "index": index, "binary": summary["binary_sha256"], "date": summary["node_date"],
                    "ledger_ids": summary["ledger_ids"], "config": e["arm"], "problem": e["problem"],
                    "rtol": e["rtol"], "ir_per_run": e["ir_per_run"], "attempts": e["attempts"],
                    "linear_solves": e["counters"]["linear_solves"],
                    "share_file": e["categories_by_file"][CATEGORY]["share"],
                    "share_function": e["categories_by_function"].get(CATEGORY, {"share": 0.0})["share"]})
                summary["usable_entries"] += 1
        summaries.append(summary)
    return summaries, entries


# --------------------------------------------------------------------------------------- applicability
def arm_applies(cid: str, arm: str, dimension) -> bool:
    cond = ARM_CONDITIONS.get((cid, arm))
    if cond is None:
        return True
    if dimension is None:
        return False
    return {"n<=8": dimension <= 8, "n==1": dimension == 1, "n>64": dimension > 64}[cond]


def accuracy_evidence(inputs: Inputs, ledger: list[dict], entry: dict) -> dict | None:
    """First other output (sorted) of the entry's ledger rows whose JSON names the problem (I5)."""
    for lid in sorted(entry["ledger_ids"]):
        row = next(r for r in ledger if r["id"] == lid)
        for path in sorted(row["outputs_sha256"]):
            if path == entry["file"] or PROFILE_RE.match(path) or not path.endswith(".json"):
                continue
            data = inputs.get(path, "accuracy-evidence")
            if sha256(data) != row["outputs_sha256"][path]:
                continue
            if names_problem(evidence.loads_strict(data), entry["problem"]):
                return {"ledger_id": lid, "path": path, "sha256": sha256(data)}
    return None


def case_comparison(inputs: Inputs, ledger: list[dict], entries: list[dict], cid: str, case: dict) -> dict | None:
    applicable = [e for e in entries if e["problem"] == case["case"] and e["config"] in CONFIG_ARM
                  and CONFIG_ARM[e["config"]] in CLOSED_ARMS[cid]
                  and arm_applies(cid, CONFIG_ARM[e["config"]], case["dimension"])]
    if not applicable:
        return None
    binaries = {}
    for e in applicable:
        b = binaries.setdefault(e["binary"], {"arms": set(), "date": "00000000"})
        b["arms"].add(CONFIG_ARM[e["config"]])
        b["date"] = max(b["date"], e["date"])
    binary = max(binaries, key=lambda k: (len(binaries[k]["arms"]), binaries[k]["date"], k))
    rtols = []
    winners = []
    for rtol in sorted({e["rtol"] for e in applicable if e["binary"] == binary}, reverse=True):
        ranked = sorted((e for e in applicable if e["binary"] == binary and e["rtol"] == rtol),
                        key=lambda e: (e["ir_per_run"], e["config"], e["file"], e["index"]))
        ranking = []
        winner = None
        for e in ranked:
            acc = accuracy_evidence(inputs, ledger, e)
            ranking.append({"config": e["config"], "arm": CONFIG_ARM[e["config"]], "ir_per_run": e["ir_per_run"],
                            "file": e["file"], "index": e["index"],
                            "ledger_id": sorted(e["ledger_ids"])[0], "accuracy_evidence": acc})
            if winner is None and acc is not None:
                winner = e
        rtols.append({"rtol": rtol, "ranking": ranking,
                      "comparator": None if winner is None else {"config": winner["config"],
                                                                 "arm": CONFIG_ARM[winner["config"]],
                                                                 "file": winner["file"], "index": winner["index"]}})
        if winner is not None:
            winners.append(winner)
    return {"case": case["case"], "binary": binary, "binary_arms": sorted(binaries[binary]["arms"]),
            "rtols": rtols, "winners": winners}


# ------------------------------------------------------------------------------------------- criteria
def scan_tokens(text: str, tokens) -> list[str]:
    return [t for t in tokens if t in text]


def q2_impls(text: str) -> list[dict]:
    out = []
    for m in Q2_IMPL_RE.finditer(text):
        end = text.find("\n}\n", m.end())
        block = text[m.start(): end if end >= 0 else len(text)]
        if "ModelBinding::GeneratedFromModel" in block:
            binding = "GeneratedFromModel"
        elif re.search(r"fn binding\(&self\) -> ModelBinding \{\s*self\.\w+\.binding\(\)", block):
            binding = "delegated"
        else:
            binding = "SampledAgreement"
        out.append({"type": m.group(1), "binding": binding})
    return out


def default_witness_facts(text: str) -> dict:
    m = re.search(r"fn witness\(&self, problem: &QuadraticStageProblem, gamma: f64\)(.*?)\n    \}\n", text, re.S)
    body = m.group(1) if m else ""
    limit = re.search(r'const SMALL_WITNESS_DIMENSION_LIMIT: &str = "([^"]*)";', text)
    return {"diagonal_branch": "InverseWitness::diagonal(" in body, "small_branch": "InverseWitness::small(" in body,
            "small_limit_message": limit.group(1) if limit else None}


def capability(case: dict, facts: dict) -> str:
    n, structure = case["dimension"], case["structure"]
    if structure == "diagonal" and facts["diagonal_branch"]:
        return "Available"
    small_ok = facts["small_branch"] and facts["small_limit_message"] == "small witness needs n <= 2"
    if n is not None and n <= 2 and small_ok:
        return "Available"
    return "DimensionCutoff"


def work_ledger(inputs: Inputs) -> dict:
    text = inputs.text(HOMOTOPY_RS, "work-ledger-source")
    m = LEDGER_STRUCT_RE.search(text)
    fields = FIELD_RE.findall(m.group(1)) if m else []
    terms = {name: [f for f in fields if re.search(pattern, f)] for name, pattern in LEDGER_TERMS}
    return {"source": HOMOTOPY_RS, "fields": fields, "terms": terms,
            "missing_terms": [name for name, _ in LEDGER_TERMS if not terms[name]]}


def l0008_rows(inputs: Inputs) -> tuple[list[dict], dict]:
    verify = inputs.json(L0008_VERIFY, "l0008-campaign")
    seq = verify["arms"]["sequential-jf-gmres"]["cases"]
    rows = [{"case": c["case_id"], "arm": "sequential-jf-gmres", "file": L0008_VERIFY, "index": None, "rtol": None,
             "attempts": c["details"]["attempts"],
             "linear_solves": c["work"]["linear_solves"],
             "solves_per_attempt": c["work"]["linear_solves"] / c["details"]["attempts"]} for c in seq]
    p8 = {c["case_id"]: c for c in verify["arms"]["transactional-certified-p8"]["cases"]}
    multipliers = {c["case_id"]: p8[c["case_id"]]["work"]["rhs_evaluations"] / c["work"]["rhs_evaluations"]
                   for c in seq}
    cert = {cid: {"certificate_attempts": c["details"]["certificate_attempts"],
                  "certificate_admissions": c["details"]["certificate_admissions"]} for cid, c in p8.items()}
    return rows, {"rhs_multiplier_p8_over_sequential": multipliers,
                  "rhs_multiplier_median": sorted(multipliers.values())[len(multipliers) // 2],
                  "p8_certificates": cert, "reference_arm": "sequential-jf-gmres"}


def screen_candidate(spec, inputs, ledger, entries, declaration_matches, ledger_terms, l0008, pp07, facts) -> dict:
    cid = spec["id"]
    texts = {p: inputs.text(p, "candidate-source") for p in spec["sources"]}
    joined = "\n".join(texts.values())
    # S1
    decl = [m for m in declaration_matches if m[0] in spec["sources"]]
    comparisons = [] if cid == "K1" else [c for c in (case_comparison(inputs, ledger, entries, cid, case)
                                                       for case in spec["cases"]) if c is not None]
    f_values = []
    for comp in comparisons:
        seen = set()
        for w in comp["winners"]:
            if w["config"] in seen:
                continue
            seen.add(w["config"])
            for e in entries:
                if e["config"] == w["config"] and e["problem"] == comp["case"]:
                    f_values.append({"case": comp["case"], "config": e["config"], "file": e["file"],
                                     "index": e["index"], "rtol": e["rtol"], "share_file": e["share_file"],
                                     "share_function": e["share_function"]})
    f_values.sort(key=lambda v: (v["case"], v["config"], v["file"], v["index"]))
    f_min = min((v["share_file"] for v in f_values), default=None)
    s1a = bool(decl)
    s1b = None if f_min is None else f_min >= F_MIN
    s1_reasons = ([] if s1a else ["no_expensive_client_declaration"]) + (["f_below_f_min"] if s1b is False else [])
    s1 = {"declaration_matches": decl, "batch_rhs_references": len(BATCH_RE.findall(joined)), "a": s1a,
          "f_status": "unmeasured" if f_min is None else "measured", "f_values": f_values, "f_min": f_min,
          "f_min_function_view": min((v["share_function"] for v in f_values), default=None),
          "b": s1b, "pass": s1a and s1b is not False, "reasons": s1_reasons}
    # S2
    static_tokens = scan_tokens(joined, SOLVER_TOKENS)
    if cid == "K1":
        q1 = pp07["q1_solves"]
        s2 = {"source": "pp07", "rows": [], "q1_solves": q1,
              "pp07": "PP07 ABSTAIN preserved" if q1 == 0 else "PP07 shows shifted solves",
              "static_solver_tokens": static_tokens, "pass": False,
              "reasons": ["pp07_q1_solves_zero"] if q1 == 0 else ["pp07_inconsistent"]}
    elif cid == "K2":
        rows = l0008["rows"]
        ok = all(r["solves_per_attempt"] >= MIN_SOLVES_PER_ATTEMPT for r in rows)
        s2 = {"source": "l0008", "rows": rows, "q1_solves": None, "pp07": None, "static_solver_tokens": static_tokens,
              "pass": ok, "reasons": [] if ok else ["solves_per_attempt_below_2"]}
    else:
        rows = []
        for comp in comparisons:
            for r in comp["rtols"]:
                c = r["comparator"]
                if c is None:
                    continue
                e = next(x for x in entries if x["file"] == c["file"] and x["index"] == c["index"])
                rows.append({"case": comp["case"], "arm": c["config"], "file": e["file"], "index": e["index"],
                             "rtol": e["rtol"], "attempts": e["attempts"], "linear_solves": e["linear_solves"],
                             "solves_per_attempt": e["linear_solves"] / e["attempts"]})
        ok = bool(rows) and all(r["solves_per_attempt"] >= MIN_SOLVES_PER_ATTEMPT for r in rows)
        s2 = {"source": "profiles" if rows else "none", "rows": rows, "q1_solves": None, "pp07": None,
              "static_solver_tokens": static_tokens, "pass": ok,
              "reasons": [] if ok else (["solves_per_attempt_below_2"] if rows else ["no_committed_counters"])}
    # S3
    arms = []
    for arm in CLOSED_ARMS[cid]:
        measured = any(CONFIG_ARM.get(e["config"]) == arm and e["problem"] == case["case"]
                       and arm_applies(cid, arm, case["dimension"]) for e in entries for case in spec["cases"])
        arms.append({"arm": arm, "condition": ARM_CONDITIONS.get((cid, arm)),
                     "status": "measured" if measured else "unmeasured",
                     "reason": None if measured else "no_committed_profile"})
    per_case = [{"case": c["case"], "binary": c["binary"], "binary_arms": c["binary_arms"], "rtols": c["rtols"]}
                for c in comparisons]
    if cid == "K1":
        s3_reasons = ["no_applicable_arm"]
    elif not comparisons:
        s3_reasons = ["no_measured_applicable_arm"] + (["jf_gmres_only_where_direct_applies"] if cid == "K2" else [])
    elif any(r["comparator"] is None for c in comparisons for r in c["rtols"]):
        s3_reasons = ["no_accuracy_evidence"]
    else:
        s3_reasons = []
    unmeasured_cases = sorted({case["case"] for case in spec["cases"]} - {c["case"] for c in comparisons})
    s3 = {"applicable_arms": list(CLOSED_ARMS[cid]), "arms": arms, "per_case": per_case,
          "cases_without_comparator": [] if cid == "K1" else unmeasured_cases,
          "l0008_reference_arm": l0008["reference_arm"] if cid == "K2" else None,
          "pass": not s3_reasons and not unmeasured_cases, "reasons": s3_reasons}
    if not s3_reasons and unmeasured_cases:
        s3["reasons"] = ["no_measured_applicable_arm"]
    # S4
    impls = q2_impls(joined)
    client_type = CLIENT_TYPES.get(cid)
    own = [i for i in impls if client_type is None or i["type"] == client_type]
    binding = None if not own else ("GeneratedFromModel" if all(i["binding"] == "GeneratedFromModel" for i in own)
                                    else "SampledAgreement")
    caps = [{"case": case["case"], "capability": capability(case, facts) if own else None} for case in spec["cases"]]
    s4a = binding == "GeneratedFromModel" and all(c["capability"] == "Available" for c in caps)
    s4_reasons = []
    if not own:
        s4_reasons.append("no_q2_certificate_source")
    else:
        if binding != "GeneratedFromModel":
            s4_reasons.append("binding_not_generated_from_model")
        if not all(c["capability"] == "Available" for c in caps):
            s4_reasons.append("capability_not_available")
    s4 = {"impls": impls, "client_type": client_type, "binding": binding, "capabilities": caps,
          "capability_evidence": ({"source": L0008_VERIFY, "p8_certificates": l0008["p8_certificates"]}
                                  if cid == "K2" else None),
          "a": s4a, "b": True, "obligations": ledger_terms["missing_terms"], "pass": s4a, "reasons": s4_reasons}
    meets = {"S1": s1["pass"], "S2": s2["pass"], "S3": s3["pass"], "S4": s4["pass"]}
    lead = (not s1a) and f_min is not None and f_min >= F_MIN
    return {"id": cid, "client": spec["client"], "sources": spec["sources"], "cases": spec["cases"],
            "S1": s1, "S2": s2, "S3": s3, "S4": s4, "meets": meets, "all": all(meets.values()), "lead": lead}


def decide(candidates: list[dict]) -> dict:
    meeting = [c for c in candidates if c["all"]]
    if not meeting:
        selected = None
    elif len(meeting) == 1:
        selected = meeting[0]["id"]
    else:
        def key(c):
            f = c["S1"]["f_min"]
            return (f is not None, f if f is not None else -math.inf)
        best = max(key(c) for c in meeting)
        tied = [c["id"] for c in meeting if key(c) == best]
        selected = tied[0] if len(tied) == 1 else None
    return {"outcome": f"SELECTED({selected})" if selected else "ABSTAIN", "selected": selected,
            "meeting_all": [c["id"] for c in meeting], "leads": [c["id"] for c in candidates if c["lead"]]}


# ----------------------------------------------------------------------------------------------- main
def git_blob_id(data: bytes) -> str:
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def build_screen(root: Path, commit: str = "HEAD", tree_status: str | None = None) -> dict:
    git = Git(root)
    try:
        commit = git.run("rev-parse", commit).strip()
        if tree_status is None:
            tree_status = git.run("status", "--porcelain", "--untracked-files=all")
        tracked = git.run("ls-tree", "-r", "--name-only", commit).splitlines()
        inputs = Inputs(git, commit)
        inputs.get(REGISTRATION, "registration")
        inputs.get(CONTRACT_DOC, "contract")
        ledger = load_ledger(inputs)
        pp07 = inputs.json(PP07_RESULTS, "pp07-results")
        l0008_counter_rows, l0008_reported = l0008_rows(inputs)
        l0008 = {"rows": l0008_counter_rows, **l0008_reported}
        profiles, entries = load_profiles(inputs, tracked, ledger)
        rust = sorted(p for p in tracked if RUST_RE.match(p))
        rust_manifest = []
        declaration_matches = []
        for path in rust:
            data = git.read(commit, path)
            rust_manifest.append([path, sha256(data)])
            for lineno, line in enumerate(data.decode("utf-8", errors="replace").splitlines(), 1):
                if DECLARATION_RE.search(line):
                    declaration_matches.append([path, lineno])
        ledger_terms = work_ledger(inputs)
        facts = default_witness_facts(inputs.text(TRANSACTIONAL_RS, "certificate-source"))
        limits = {p: phrase in inputs.text(p, "stated-limit") for p, phrase in LIMIT_PHRASES.items()}
        candidates = [screen_candidate(spec, inputs, ledger, entries, declaration_matches, ledger_terms, l0008,
                                       pp07, facts) for spec in CANDIDATES]
        screen_blob = git.run("rev-parse", f"{commit}:{SCREEN_TOOL}").strip()
        checker_blob = git.run("rev-parse", f"{commit}:{CHECKER_TOOL}").strip()
    finally:
        git.close()
    applicable_all = [e for e in entries if e["config"] in CONFIG_ARM]
    doc = {
        "schema": SCHEMA, "node": "HT01",
        "provenance": {"source_commit": commit, "tree_status": tree_status, "screen_path": SCREEN_TOOL,
                       "screen_blob": screen_blob, "checker_path": CHECKER_TOOL, "checker_blob": checker_blob,
                       "executed_commands": git.commands, "solver_runs": 0,
                       "python": ".".join(str(x) for x in sys.version_info[:3])},
        "constants": {"f_min": F_MIN, "f_min_unrounded": F_MIN_UNROUNDED, "rhs_multiplier_r": R_MULTIPLIER,
                      "threads_P": THREADS_P, "required_speedup": REQUIRED_SPEEDUP,
                      "min_solves_per_attempt": MIN_SOLVES_PER_ATTEMPT, "category": CATEGORY,
                      "f_view": "categories_by_file"},
        "inputs": inputs.record(),
        "rust_sources": rust_manifest,
        "profiles": profiles,
        "declaration_scan": {"pattern": DECLARATION_RE.pattern, "matches": declaration_matches},
        "work_ledger": ledger_terms,
        "default_witness": facts,
        "stated_limits": limits,
        "pp07": {"q1_solves": pp07["q1_solves"], "q2_multi_shift_starts": pp07["q2_multi_shift_starts"],
                 "q2_jet_admissible_starts": pp07["q2_jet_admissible_starts"], "verdict": pp07["verdict"]},
        "l0008": {k: v for k, v in l0008.items() if k != "rows"},
        "candidates": candidates,
        "decision": decide(candidates),
        "reported": {
            "max_share_file_all_applicable_entries": max((e["share_file"] for e in applicable_all), default=None),
            "max_share_function_all_applicable_entries": max((e["share_function"] for e in applicable_all),
                                                             default=None),
            "entries_function_view_at_or_above_f_min": [
                {"file": e["file"], "index": e["index"], "config": e["config"], "problem": e["problem"],
                 "rtol": e["rtol"], "share_function": e["share_function"], "share_file": e["share_file"]}
                for e in applicable_all if e["share_function"] >= F_MIN],
            "function_view_note": "tools/speed_profile.py: a generic driver monomorphized into a CLI function "
                                  "(e.g. rodas5p::stiff_benchmark::run_small_full) is charged to that CLI function "
                                  "in the function view; the file view charges it to the driver's source file",
            "artificial_workloads": [], "reruns": [],
        },
    }
    return doc


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    doc = build_screen(args.repo_root)
    if doc["provenance"]["tree_status"] != "":
        raise SystemExit("the tree is not clean; the recorded screen runs on a clean tree at a commit\n"
                         + doc["provenance"]["tree_status"])
    args.output.write_text(json.dumps(doc, indent=1, allow_nan=False) + "\n")
    print(f"wrote {args.output}: decision {doc['decision']['outcome']}, leads {doc['decision']['leads']}")
    for c in doc["candidates"]:
        print(f"  {c['id']}: " + ", ".join(f"{k} {'met' if v else 'not met'}" for k, v in c["meets"].items())
              + f"; f_min {c['S1']['f_min']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
