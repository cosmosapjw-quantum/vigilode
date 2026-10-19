#!/usr/bin/env python3
"""Decision engine of the component triage: RESULTS.json and CLASSIFICATION.json, recomputed from raw rows.

Inputs (strict JSON; nothing judged upstream is trusted, PA-03): RUNS.jsonl (internal rows of `rodas5p
triage-run`, typed process_abort rows of tools/triage/triage_runner.py, external rows of triage_external.py;
`--runs` is repeatable), REFERENCES.json, PROFILE.jsonl, ALLOCATIONS.jsonl, COMPONENTS.json, CORPUS.json,
THRESHOLDS.json, and optionally PROBES.jsonl (C3), TIMING.json (conflict flag), ATTRIBUTION.json (OPTIMIZE
headroom) and the `triage-list` JSON (applicability of arms without rows). Every threshold is read from
THRESHOLDS.json. A missing measurement is null with a reason, never 0 and never a pass.

A. Row accuracy (status `ok` rows; output times must equal the reference times bit for bit):
   E     = max_t WRMS((y - ref) * w),  w = 1 / (atol_scale_P + |ref|)   (atol_scale_P from CORPUS)
   E_rho = max_t WRMS(rho * w);  E_up = E + E_rho
   R, R_rho, R_up: the same with w = 1 / (atol_row + rtol_row * |ref|)  (C2 ratio).
   Non-finite states, or an ok row of the wrong shape, are integrity failures (C1), not unknowns.
   The code is tools/triage/triage_common.py::row_accuracy. Rows whose states live in a sidecar
   (`--states-dir`) are recomputed from the verified sidecar, else take the runner's accuracy_inline
   (flagged `accuracy_source: runner_inline`), else accuracy is null with a reason.
B. Groups (arm, problem, grid, rtol): at target eps the group is admissible iff rows for every h0 in
   accuracy.h0_scales_required are `ok` with E_up <= eps; inadmissible if one of them is not ok or exceeds eps;
   unknown otherwise (missing row or reference). (problem, grid) is reference_limited at eps when
   E_rho > reference_limited_factor * eps. Primary cost of a group = PROFILE ir_warm of its h0 = 1 cell (only
   groups in the CORPUS profile subset are cost candidates); secondary costs are reported, never combined.
   The configuration cost of an arm at eps = min over admissible candidate groups with a cost, ordered by
   (cost, E_up, rtol) as select_cheapest_below_target (crates/rodas5p-fair-ab/src/global_error.rs); it is
   *incomplete* when an admissible candidate lacks a cost or a candidate group is unknown.
C. Comparisons of component c (internal arms): for each arm a containing c, every arm a' whose component set
   equals a's outside c's slot and lacks c (slot content replaced, or an opt-in feature switched off) gives kind
   `attribution`; without such an a', kind
   `strongest` compares with the cheapest applicable arm containing another component of the slot (and not c).
   Per (problem, grid, eps): ratio = cost(a) / cost(a') -> GAIN (<= gain_max), NEUTRAL (<= neutral_max), LOSS;
   capability_gain (a admissible, no admissible rival), capability_loss (rival admissible, a not); excluded
   (reason) when neither is admissible, the reference is missing or limited, a state is unknown, or a cost is
   missing/incomplete. Only problems of a tier in c.tiers are compared.
D. Criteria. C1 integrity: process_abort, deterministic = false or non-finite/malformed ok rows of an arm
   containing c, attributed to c unless another arm containing c ran the same cell cleanly (exoneration;
   interaction faults are not separated). C2: over c's groups (reference-limited in requested units excluded:
   R_rho > reference_limited_factor), fraction with R_up <= ratio >= cell_fraction and max R_up <= ratio_max.
   Allocation: max warm-window allocations over c's arms <= warm_window_allocations_max, or, when allowed, equal
   to the component's optional `documented_warm_allocations` on rows of >= 2 distinct dimensions. C3 from
   PROBES.jsonl (rows of the row schema with `probe` = {id, must_refuse, components?}): a must-refuse probe
   passes only as `refused`/`error` with success false; process_abort always fails; without probes C3 is null
   (C3_MISSING).
E. Decision tree (rule_path entries {rule, kind, detail}; tools/triage/README.md explains every token):
   STATIC_PRUNE_RELOCATE (static_class) -> PRUNE(relocate). C1_FAIL / C3_FAIL -> PRUNE, except that a
   component with a non-null optional `repair_path` and unique_capability true gets C_FAIL_REPAIRABLE ->
   OPTIMIZE, provisional (prereg rule 1). Provisional causes: BLOCKED_BY (blocked_by non-empty), TIMING_CONFLICT, C3_MISSING (no probe
   covers the component; not applied to static components). NO_EVIDENCE (no non-excluded comparison) ->
   OPTIMIZE if unique_capability else PRUNE, both provisional. PROMOTE iff over non-excluded G/P comparisons
   the fraction of GAIN/NEUTRAL/capability_gain >= promote_cell_fraction, every ratio <= promote_max_ratio,
   C1 pass, C2 pass, allocation pass, and not provisional; a component meeting these criteria while provisional
   gets PROMOTE_BLOCKED_PROVISIONAL -> OPTIMIZE (never PRUNE by fall-through). Otherwise SPECIALIZE iff the win
   set (GAIN or capability_gain cells) is non-empty and a single-feature predicate (numeric threshold <=/>= or
   a category set) over specialize.features, found on calibration cells (no loss, most wins), passes the
   holdout test of tools/holdout_verdict.py (positives = holdout loss cells, min_positives, one-rank threshold
   shift; category sets have no threshold to shift, so their verdict is the raw verdict with min_positives)
   and recommends at least one holdout win (`rtol` is the comparison's target eps); a provisional component
   keeps SPECIALIZE. Otherwise OPTIMIZE_HEADROOM (every LOSS ratio adjusted by ATTRIBUTION removable Ir <=
   headroom_ratio_max), OPTIMIZE_EXTERNAL_GAP (unique_capability and median counted-work gap to the
   COMPONENTS external_peers > external_counted_work_gap), OPTIMIZE_UNCHARACTERIZED_WINS (unique_capability
   with a non-empty win set no validated predicate characterizes), else PRUNE_DEFAULT. Afterwards, of two
   non-PRUNE components of one slot whose mutual comparisons are all NEUTRAL, the one with more LOC is PRUNE
   (NEUTRAL_TIEBREAK_LOC; NEUTRAL_TIE_EQUAL_LOC when LOC is equal). A TIMING entry whose wall ratio deviates
   from 1 by more than timing.conflict_threshold opposite to the Ir ratio of the same cell, with an interval
   excluding 1, flags TIMING_CONFLICT.
F. External gap (THRESHOLDS.external): external rows are admissible per single row (the nominal h0 = 1 row;
   the h0 rule applies to internal arms only). Counted work = sum of external.counted_work units of the
   nominal row (WorkCounters rhs_evaluations / jacobian_builds / direct_factorizations; extra_work nfev / njev
   / nlu), minimum over admissible rtol groups; a cell with an unavailable unit or an unknown group on either
   side is excluded with its reason. Gap = work(c's arm) / work(peer) per (problem, grid, eps); median over
   matched cells.

Outputs carry the SHA-256 of every input and the campaign id; both are byte-deterministic for fixed inputs.
Validate them with tools/evidence_schema_v2.py::validate_triage.
"""

from __future__ import annotations

import argparse
import importlib.util
import math
import statistics
import struct
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import triage_common as tc  # noqa: E402

RESULTS_SCHEMA = "vigilode-triage-results-v1"
CLASSIFICATION_SCHEMA = "vigilode-triage-classification-v1"
TIMING_SCHEMA = "vigilode-triage-timing-v1"
ATTRIBUTION_SCHEMA = "vigilode-triage-attribution-v1"
GENERATOR = "tools/triage/triage_check.py"
LABELS = ("PROMOTE", "SPECIALIZE", "OPTIMIZE", "PRUNE")
OUTCOMES = ("GAIN", "NEUTRAL", "LOSS", "capability_gain", "capability_loss", "excluded")
WIN_OUTCOMES = ("GAIN", "capability_gain")
LOSS_OUTCOMES = ("LOSS", "capability_loss")
NOMINAL_H0 = 1.0  # contract: the primary cost is measured on the h0 = 1 * h0_base cell
INPUT_ROLES = ("runs", "references", "profile", "allocations", "probes", "timing", "attribution", "components",
               "corpus", "thresholds", "list")
NUMERIC_FEATURES = ("dimension", "rtol")
# THRESHOLDS.external.counted_work units -> WorkCounters field (internal rows) / extra_work key (external rows).
INTERNAL_UNITS = {"rhs_evaluations": "rhs_evaluations", "jacobian_evaluations": "jacobian_builds",
                  "lu_factorizations": "direct_factorizations"}
EXTERNAL_UNITS = {"rhs_evaluations": "nfev", "jacobian_evaluations": "njev", "lu_factorizations": "nlu"}
# rule_path tokens and their kinds (tools/triage/README.md explains each one).
RULE_KINDS = {
    "STATIC_PRUNE_RELOCATE": "static", "C1_FAIL": "correctness", "C3_FAIL": "correctness",
    "CORRECTNESS_OK": "correctness", "BLOCKED_BY": "contract", "TIMING_CONFLICT": "contract",
    "C3_MISSING": "contract", "PROMOTE_BLOCKED_PROVISIONAL": "contract", "C_FAIL_REPAIRABLE": "contract",
    "NO_EVIDENCE": "evidence",
    "PROMOTE": "frontier", "NOT_PROMOTE": "frontier", "SPECIALIZE": "frontier", "NOT_SPECIALIZE": "frontier",
    "OPTIMIZE_HEADROOM": "frontier", "OPTIMIZE_EXTERNAL_GAP": "frontier", "OPTIMIZE_UNCHARACTERIZED_WINS": "frontier",
    "PRUNE_DEFAULT": "frontier", "NEUTRAL_TIEBREAK_LOC": "tiebreak", "NEUTRAL_TIE_EQUAL_LOC": "tiebreak",
}


def _holdout_module():
    spec = importlib.util.spec_from_file_location("holdout_verdict", tc.TOOLS / "holdout_verdict.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


hv = _holdout_module()


def fnum(value):
    """JSON-safe float: finite floats pass, everything else is null (the reason is recorded beside it)."""
    if value is None:
        return None
    value = float(value)
    return value if math.isfinite(value) else None


# ===================================================================================== input loading
class Inputs:
    def __init__(self):
        self.digests: dict[str, Any] = {role: None for role in INPUT_ROLES}
        self.digests["runs"] = []


def _digest(path: Path) -> dict:
    return {"name": Path(path).name, "sha256": tc.sha256_file(path)}


def validate_thresholds(th: Any) -> list[str]:
    reasons = []
    if not isinstance(th, dict) or th.get("schema") != tc.THRESHOLDS_SCHEMA:
        return [f"THRESHOLDS: schema is not {tc.THRESHOLDS_SCHEMA!r}"]
    need = {"accuracy": ("targets", "reference_limited_factor", "h0_scales_required"),
            "reliability_c2": ("cell_fraction", "ratio", "ratio_max"),
            "frontier": ("gain_max", "neutral_max", "promote_cell_fraction", "promote_max_ratio", "primary_cost",
                         "secondary_costs"),
            "allocation": ("warm_window_allocations_max", "allow_documented_constant_independent_of_n"),
            "specialize": ("min_positives", "threshold_rank_shift", "features"),
            "optimize": ("headroom_ratio_max", "external_counted_work_gap"),
            "timing": ("conflict_threshold",),
            "parity": ("max_relative_error",),
            "external": ("peers_field", "counted_work")}
    for section, keys in need.items():
        if not isinstance(th.get(section), dict):
            reasons.append(f"THRESHOLDS: missing section {section!r}")
            continue
        for key in keys:
            if key not in th[section]:
                reasons.append(f"THRESHOLDS: missing {section}.{key}")
    if reasons:
        return reasons
    if th["frontier"]["primary_cost"] != "ir_warm":
        reasons.append("THRESHOLDS: frontier.primary_cost must be 'ir_warm' (the only cost PROFILE.jsonl measures)")
    if th["specialize"]["threshold_rank_shift"] != 1:
        reasons.append("THRESHOLDS: specialize.threshold_rank_shift must be 1 (tools/holdout_verdict.py shifts one rank)")
    units = th["external"]["counted_work"]
    if not isinstance(units, list) or not units or any(u not in INTERNAL_UNITS for u in units):
        reasons.append(f"THRESHOLDS: external.counted_work must be a nonempty subset of {sorted(INTERNAL_UNITS)}")
    if NOMINAL_H0 not in [float(h) for h in th["accuracy"]["h0_scales_required"]]:
        reasons.append("THRESHOLDS: accuracy.h0_scales_required must contain 1.0 (the profiled cell)")
    return reasons


def validate_components(doc: Any, peers_field: str = "external_peers") -> list[str]:
    if not isinstance(doc, dict) or doc.get("schema") != tc.COMPONENTS_SCHEMA:
        return [f"COMPONENTS: schema is not {tc.COMPONENTS_SCHEMA!r}"]
    reasons, seen = [], set()
    required = ("id", "family", "slot", "reach", "tiers", "arms", "blocked_by", "loc", "static_class",
                "unique_capability")
    for i, c in enumerate(doc.get("components") or []):
        if not isinstance(c, dict):
            reasons.append(f"COMPONENTS.components[{i}]: not an object")
            continue
        missing = [k for k in required if k not in c]
        if missing:
            reasons.append(f"COMPONENTS.components[{i}]: missing {missing}")
            continue
        if c["id"] in seen:
            reasons.append(f"COMPONENTS: duplicate component id {c['id']!r}")
        seen.add(c["id"])
        if type(c["loc"]) is not int or c["loc"] < 0:
            reasons.append(f"COMPONENTS[{c['id']}].loc: expected int >= 0")
        if c["static_class"] not in (None, "PRUNE_RELOCATE"):
            reasons.append(f"COMPONENTS[{c['id']}].static_class: {c['static_class']!r}")
        if not isinstance(c["blocked_by"], list) or not isinstance(c["arms"], list) or not isinstance(c["tiers"], list):
            reasons.append(f"COMPONENTS[{c['id']}]: blocked_by, arms and tiers must be lists")
        if type(c["unique_capability"]) is not bool:
            reasons.append(f"COMPONENTS[{c['id']}].unique_capability: expected bool")
        repair = c.get("repair_path")
        if repair is not None and (not isinstance(repair, str) or not repair.strip()):
            reasons.append(f"COMPONENTS[{c['id']}].repair_path: expected a nonempty string or null")
        peers = c.get(peers_field, [])
        if not isinstance(peers, list) or not all(isinstance(a, str) and tc.is_external(a) for a in peers):
            reasons.append(f"COMPONENTS[{c['id']}].{peers_field}: expected a list of ext.* arm ids")
    if not seen:
        reasons.append("COMPONENTS: no components")
    return reasons


def validate_corpus(doc: Any) -> list[str]:
    if not isinstance(doc, dict) or doc.get("schema") != tc.CORPUS_SCHEMA:
        return [f"CORPUS: schema is not {tc.CORPUS_SCHEMA!r}"]
    reasons = []
    try:
        problems = tc.corpus_problems(doc)
    except tc.TriageError as exc:
        return list(exc.reasons)
    for pid, p in problems.items():
        for key in ("tier", "partition", "dimension", "t_span", "atol_scale"):
            if key not in p:
                reasons.append(f"CORPUS[{pid}]: missing {key}")
        if p.get("partition") not in ("calibration", "holdout"):
            reasons.append(f"CORPUS[{pid}].partition: {p.get('partition')!r}")
        if not tc.finite(p.get("atol_scale")) or not p.get("atol_scale") > 0:
            reasons.append(f"CORPUS[{pid}].atol_scale: expected positive number")
    sweep = doc.get("sweep") or {}
    for key in ("rtols", "grids", "h0_scales"):
        if not isinstance(sweep.get(key), list) or not sweep.get(key):
            reasons.append(f"CORPUS.sweep.{key}: expected a nonempty list")
    subset = doc.get("profile_subset") or {}
    if not isinstance(subset.get("rtols"), list) or type(subset.get("max_dimension")) is not int:
        reasons.append("CORPUS.profile_subset: expected rtols list and int max_dimension")
    return reasons


def validate_row(row: dict, where: str) -> list[str]:
    reasons = []
    keys = set(row)
    missing, extra = sorted(tc.ROW_REQUIRED_KEYS - keys), sorted(keys - tc.ROW_KEYS - tc.ROW_OPTIONAL_KEYS)
    if missing or extra:
        return [f"{where}: key set differs (missing {missing}, unexpected {extra})"]
    if row["schema"] != tc.ROW_SCHEMA:
        reasons.append(f"{where}: schema {row['schema']!r}")
    for key in ("arm", "problem", "campaign"):
        if not isinstance(row[key], str) or not row[key]:
            reasons.append(f"{where}.{key}: expected nonempty string")
    if row["grid"] not in tc.GRIDS:
        reasons.append(f"{where}.grid: {row['grid']!r}")
    for key in ("rtol", "h0_scale"):
        if type(row[key]) is not float or not math.isfinite(row[key]) or not row[key] > 0:
            reasons.append(f"{where}.{key}: expected positive finite float, got {row[key]!r}")
    if row["status"] not in tc.ROW_STATUSES:
        reasons.append(f"{where}.status: {row['status']!r}")
    if type(row["success"]) is not bool:
        reasons.append(f"{where}.success: expected bool")
    elif row["status"] == "ok" and row["success"] is not True:
        reasons.append(f"{where}: status ok with success false")
    if not isinstance(row["components"], list) or not all(isinstance(c, str) for c in row["components"]):
        reasons.append(f"{where}.components: expected list of strings")
    if row["deterministic"] is not None and type(row["deterministic"]) is not bool:
        reasons.append(f"{where}.deterministic: expected bool or null")
    sidecar = tc.STATE_SIDECAR_KEYS & keys
    if "states_hex" in row:
        if sidecar:
            reasons.append(f"{where}: states_hex and state sidecar keys {sorted(sidecar)} together")
        if not isinstance(row["states_hex"], list):
            reasons.append(f"{where}: states_hex must be a list")
    elif sidecar != tc.STATE_SIDECAR_KEYS:
        reasons.append(f"{where}: no states_hex and an incomplete state sidecar (have {sorted(sidecar)})")
    if not isinstance(row["output_times"], list):
        reasons.append(f"{where}: output_times must be a list")
    if row["status"] == "ok" and not (tc.finite(row["atol"]) and row["atol"] > 0):
        reasons.append(f"{where}.atol: expected positive number on an ok row")
    if row["counters"] is not None and not isinstance(row["counters"], dict):
        reasons.append(f"{where}.counters: expected object or null")
    if row["extra_work"] is not None and not isinstance(row["extra_work"], dict):
        reasons.append(f"{where}.extra_work: expected object or null")
    return reasons


def load_inputs(args) -> tuple[Inputs, list[str]]:
    inp = Inputs()
    reasons: list[str] = []

    def load(role, path, jsonl=False, optional=False):
        if path is None:
            return None
        try:
            doc = tc.load_jsonl(path) if jsonl else tc.load_json(path)
        except tc.TriageError as exc:
            reasons.extend(exc.reasons)
            return None
        if role == "runs":
            inp.digests["runs"].append(_digest(path))
        else:
            inp.digests[role] = _digest(path)
        return doc

    inp.thresholds = load("thresholds", args.thresholds)
    inp.components = load("components", args.components)
    inp.corpus = load("corpus", args.corpus)
    inp.references = load("references", args.references)
    inp.profile = load("profile", args.profile, jsonl=True)
    inp.allocations = load("allocations", args.allocations, jsonl=True)
    inp.probes = load("probes", args.probes, jsonl=True)
    inp.timing = load("timing", args.timing)
    inp.attribution = load("attribution", args.attribution)
    inp.list = load("list", args.list)
    runs = []
    for path in args.runs:
        runs.extend((path.name, i, r) for i, r in enumerate(load("runs", path, jsonl=True) or [], 1))
    if reasons:
        return inp, reasons
    reasons += validate_thresholds(inp.thresholds)
    external = inp.thresholds.get("external") if isinstance(inp.thresholds, dict) else None
    reasons += validate_components(inp.components, (external or {}).get("peers_field") or "external_peers")
    reasons += validate_corpus(inp.corpus)
    if not isinstance(inp.references, dict) or inp.references.get("schema") != tc.REFERENCES_SCHEMA:
        reasons.append(f"REFERENCES: schema is not {tc.REFERENCES_SCHEMA!r}")
    if inp.timing is not None and (not isinstance(inp.timing, dict) or inp.timing.get("schema") != TIMING_SCHEMA):
        reasons.append(f"TIMING: schema is not {TIMING_SCHEMA!r}")
    if inp.attribution is not None and (not isinstance(inp.attribution, dict)
                                        or inp.attribution.get("schema") != ATTRIBUTION_SCHEMA):
        reasons.append(f"ATTRIBUTION: schema is not {ATTRIBUTION_SCHEMA!r}")
    if inp.list is not None and (not isinstance(inp.list, dict) or inp.list.get("schema") != tc.LIST_SCHEMA):
        reasons.append(f"LIST: schema is not {tc.LIST_SCHEMA!r}")
    if reasons:
        return inp, reasons
    campaign = args.campaign
    inp.campaign = campaign
    inp.states_dirs = [Path(d) for d in (getattr(args, "states_dir", None) or [])]
    inp.rows = {}
    inp.ignored_rows = 0
    for name, line, row in runs:
        where = f"{name}:{line}"
        row_reasons = validate_row(row, where)
        if row_reasons:
            reasons += row_reasons
            continue
        if row["campaign"] != campaign:
            inp.ignored_rows += 1
            continue
        key = (row["arm"], row["problem"], row["grid"], row["rtol"], row["h0_scale"])
        if key in inp.rows:
            reasons.append(f"{where}: duplicate cell {key!r} in campaign {campaign!r}")
            continue
        inp.rows[key] = row
    inp.profile_rows = _index_cells(inp.profile or [], tc.PROFILE_SCHEMA, campaign, "PROFILE", reasons)
    inp.alloc_rows = _index_cells(inp.allocations or [], tc.ALLOCATIONS_SCHEMA, campaign, "ALLOCATIONS", reasons)
    inp.probe_rows = []
    for i, row in enumerate(inp.probes or [], 1):
        where = f"PROBES:{i}"
        row_reasons = validate_row(row, where)
        probe = row.get("probe")
        if not isinstance(probe, dict) or not isinstance(probe.get("id"), str) or \
                type(probe.get("must_refuse")) is not bool:
            row_reasons.append(f"{where}.probe: expected {{id: str, must_refuse: bool, components?: [str]}}")
        reasons += row_reasons
        if not row_reasons and row["campaign"] == campaign:
            inp.probe_rows.append(row)
    inp.refs = {}
    for i, entry in enumerate(inp.references.get("references") or []):
        key = (entry.get("problem"), entry.get("grid"))
        if key in inp.refs:
            reasons.append(f"REFERENCES.references[{i}]: duplicate (problem, grid) {key!r}")
        inp.refs[key] = entry
    inp.problems = tc.corpus_problems(inp.corpus)
    inp.comp = {c["id"]: c for c in inp.components["components"]}
    unknown_problems = sorted({k[1] for k in inp.rows} - set(inp.problems))
    if unknown_problems:
        reasons.append(f"RUNS: problems not in CORPUS: {unknown_problems}")
    unknown_components = sorted({c for r in inp.rows.values() if not tc.is_external(r["arm"])
                                 for c in r["components"]} - set(inp.comp))
    if unknown_components:
        reasons.append(f"RUNS: components not in COMPONENTS: {unknown_components}")
    arm_components: dict[str, tuple] = {}
    for key, row in inp.rows.items():
        comps = tuple(sorted(row["components"]))
        if row["status"] == "process_abort" and not comps:
            continue
        if arm_components.setdefault(key[0], comps) != comps:
            reasons.append(f"RUNS: arm {key[0]!r} has different component sets {arm_components[key[0]]} / {comps}")
    if inp.list is not None:
        for arm in inp.list.get("arms", []):
            if arm.get("id") in arm_components and tuple(sorted(arm.get("components", []))) != arm_components[arm["id"]]:
                reasons.append(f"LIST: arm {arm['id']!r} components differ from its rows")
    inp.arm_components = {a: set(c) for a, c in arm_components.items()}
    return inp, reasons


def _index_cells(rows: list[dict], schema: str, campaign: str, name: str, reasons: list[str]) -> dict:
    out = {}
    for i, row in enumerate(rows, 1):
        if row.get("schema") != schema:
            reasons.append(f"{name}:{i}: schema {row.get('schema')!r} != {schema!r}")
            continue
        if row.get("campaign") != campaign:
            continue
        key = (row.get("arm"), row.get("problem"), row.get("grid"), row.get("rtol"), row.get("h0_scale"))
        if key in out:
            reasons.append(f"{name}:{i}: duplicate cell {key!r}")
            continue
        out[key] = row
    return out


# ======================================================================================== accuracy
# The implementation lives in tools/triage/triage_common.py so that the runner's accuracy_inline and this
# checker's recomputation are the same code.
decode_vector = tc.decode_vector
wrms_max = tc.wrms_max
Reference = tc.Reference
row_accuracy = tc.row_accuracy
ACCURACY_SOURCES = ("recomputed_hex", "recomputed_sidecar", "recomputed_status", "runner_inline", "none")


# ======================================================================================== the engine

def profile_is_deterministic(prof: dict | None, tolerance: float) -> bool:
    """Ir(1) and Ir(1b) agree exactly, or within THRESHOLDS.profile.ir_determinism_relative_tolerance."""
    if prof is None:
        return False
    if prof.get("deterministic") is True:
        return True
    ir = prof.get("ir") or {}
    a, b = ir.get("1"), ir.get("1b")
    if type(a) is not int or type(b) is not int:
        return False
    return abs(a - b) <= tolerance * max(1, abs(b))

class Engine:
    def __init__(self, inp: Inputs):
        self.inp = inp
        self.th = inp.thresholds
        self.targets = [float(t) for t in self.th["accuracy"]["targets"]]
        self.h0_required = [float(h) for h in self.th["accuracy"]["h0_scales_required"]]
        self.ref_factor = float(self.th["accuracy"]["reference_limited_factor"])
        # Absent in thresholds frozen before the smoke amendment: exact equality.
        self.ir_tolerance = float((self.th.get("profile") or {}).get("ir_determinism_relative_tolerance", 0.0))
        sweep = inp.corpus["sweep"]
        self.rtols = sorted({float(r) for r in sweep["rtols"]})
        self.grids = list(sweep["grids"])
        subset = inp.corpus["profile_subset"]
        self.profile_rtols = {float(r) for r in subset["rtols"]}
        self.profile_max_dim = subset["max_dimension"]
        self.profile_grids = set(subset["grids"]) if subset.get("grids") is not None else None
        self.secondary_names = list(self.th["frontier"]["secondary_costs"])
        self.work_units = list(self.th["external"]["counted_work"])
        self.peers_field = self.th["external"]["peers_field"]
        self.refs: dict[tuple, Reference] = {}
        self.acc: dict[tuple, dict] = {}
        self.groups: dict[tuple, dict] = {}
        self.arm_cell_cache: dict[tuple, dict] = {}
        self.sidecar_events: dict[str, int] = {}
        self.arms = sorted({k[0] for k in inp.rows})
        self.internal_arms = [a for a in self.arms if not tc.is_external(a)]
        self.arm_problems: dict[str, set] = defaultdict(set)
        for key in inp.rows:
            self.arm_problems[key[0]].add(key[1])
        if inp.list is not None:
            list_problems = {p["id"]: p for p in inp.list.get("problems", [])}
            for arm in inp.list.get("arms", []):
                if not arm.get("wired") or tc.is_external(arm["id"]):
                    continue
                self.inp.arm_components.setdefault(arm["id"], set(arm.get("components", [])))
                if arm["id"] not in self.internal_arms:
                    self.internal_arms.append(arm["id"])
                for pid, cp in inp.problems.items():
                    if pid in list_problems and tc.applicable(arm, tc.merged_problem(cp, list_problems[pid]))[0]:
                        self.arm_problems[arm["id"]].add(pid)
            self.internal_arms.sort()

    # ---------------------------------------------------------------- references and rows
    def reference(self, problem_id: str, grid: str) -> Reference:
        key = (problem_id, grid)
        if key not in self.refs:
            self.refs[key] = Reference(self.inp.refs.get(key), self.inp.problems[problem_id], grid)
        return self.refs[key]

    def accuracy(self, key: tuple) -> dict | None:
        """Row accuracy with its `source`: recomputed from states_hex, from a verified state sidecar, from the
        row status alone (non-ok rows), the runner's accuracy_inline (flagged), or none (null with a reason)."""
        if key not in self.inp.rows:
            return None
        if key not in self.acc:
            row = self.inp.rows[key]
            problem = self.inp.problems[row["problem"]]
            ref = self.reference(row["problem"], row["grid"])
            sidecar_note = None
            if "states_hex" in row:
                acc, source = row_accuracy(row, problem, ref), "recomputed_hex"
            elif row["status"] != "ok":
                acc, source = row_accuracy(row, problem, ref, []), "recomputed_status"
            else:
                states, sidecar_source, sidecar_note = (tc.load_states_sidecar(self.inp.states_dirs, row)
                                                        if self.inp.states_dirs else
                                                        (None, "sidecar_missing", "no --states-dir given"))
                self.sidecar_events[sidecar_source] = self.sidecar_events.get(sidecar_source, 0) + 1
                if states is not None:
                    acc, source = row_accuracy(row, problem, ref, states), "recomputed_sidecar"
                else:
                    acc, source = self._inline(row, ref, sidecar_note)
            acc = dict(acc, source=source, sidecar=sidecar_note)
            self.acc[key] = acc
        return self.acc[key]

    @staticmethod
    def _inline(row: dict, ref, sidecar_note: str | None) -> tuple[dict, str]:
        inline = row.get("accuracy_inline")
        empty = {"class": "unknown", "E": None, "E_rho": None, "E_up": None, "R": None, "R_rho": None,
                 "R_up": None, "integrity": None}
        if inline is None:
            return dict(empty, reason=f"accuracy: null: no states and no accuracy_inline ({sidecar_note})"), "none"
        problem_with_inline = tc.check_accuracy_record(inline)
        if problem_with_inline is None and inline["function"] != tc.ACCURACY_FUNCTION:
            problem_with_inline = f"accuracy_inline from {inline['function']!r}"
        if problem_with_inline is None and inline["reference_sha256"] != ref.sha256:
            problem_with_inline = "accuracy_inline was computed against a different reference"
        if problem_with_inline is not None:
            return dict(empty, reason=f"accuracy: null: {problem_with_inline} ({sidecar_note})"), "none"
        return {k: inline[k] for k in tc.ACCURACY_KEYS}, "runner_inline"

    def reference_limited(self, problem_id: str, grid: str, eps: float) -> bool:
        ref = self.reference(problem_id, grid)
        return ref.ok and ref.e_rho > self.ref_factor * eps

    # ---------------------------------------------------------------------------- groups
    def ensure_groups(self) -> None:
        """Materialize every (arm, problem, grid, rtol) group of every arm, so that criteria iterating
        `self.groups` (C2) never depend on which groups the comparisons happened to touch."""
        for arm in sorted(set(self.arms) | set(self.internal_arms)):
            for pid in sorted(self.arm_problems.get(arm, set())):
                for grid in self.grids:
                    for rtol in self.rtols:
                        self.group(arm, pid, grid, rtol)

    def in_profile_subset(self, problem_id: str, rtol: float, grid: str | None = None) -> bool:
        # CORPUS profile_subset.grids, when present, limits which grids are profiled at all.
        if grid is not None and self.profile_grids is not None and grid not in self.profile_grids:
            return False
        return rtol in self.profile_rtols and self.inp.problems[problem_id]["dimension"] <= self.profile_max_dim

    def group(self, arm: str, problem_id: str, grid: str, rtol: float) -> dict:
        key = (arm, problem_id, grid, rtol)
        if key in self.groups:
            return self.groups[key]
        required = self.required_h0(arm)
        rows = {h: self.inp.rows.get((arm, problem_id, grid, rtol, h)) for h in required}
        accs = {h: self.accuracy((arm, problem_id, grid, rtol, h)) for h in required}
        g = {"arm": arm, "problem": problem_id, "grid": grid, "rtol": rtol, "external": tc.is_external(arm),
             "present": any(r is not None for r in rows.values())}
        e_ups = [a["E_up"] for a in accs.values() if a is not None and a["class"] == "computed"]
        g["E_up_max"] = max(e_ups) if e_ups else None
        r_ups = [(a["R_up"], a["R_rho"]) for a in accs.values() if a is not None and a["class"] == "computed"]
        g["R_up_max"] = max(r for r, _ in r_ups) if r_ups else None
        g["R_rho_max"] = max(r for _, r in r_ups) if r_ups else None
        g["targets"] = [self._group_state(rows, accs, eps, required) for eps in self.targets]
        g["in_profile_subset"] = (not g["external"]) and self.in_profile_subset(problem_id, rtol, grid)
        g["cost"], g["cost_reason"] = self._cost(arm, problem_id, grid, rtol, g)
        g["secondary"], g["secondary_reasons"] = self._secondary(arm, problem_id, grid, rtol)
        g["counted_work"], g["counted_work_reason"] = self._counted_work(rows.get(NOMINAL_H0))
        self.groups[key] = g
        return g

    def required_h0(self, arm: str) -> list[float]:
        """Internal arms: every h0 of accuracy.h0_scales_required. External arms (THRESHOLDS.external): one row,
        the nominal h0 = 1 cell."""
        return [NOMINAL_H0] if tc.is_external(arm) else self.h0_required

    def _group_state(self, rows: dict, accs: dict, eps: float, required: list[float]) -> dict:
        bad, unknown = [], []
        for h in required:
            row, acc = rows[h], accs[h]
            if row is None:
                unknown.append(f"h0x{h!r}: missing row")
            elif acc["class"] == "failed":
                bad.append(f"h0x{h!r}: {acc['reason']}")
            elif acc["class"] == "unknown":
                unknown.append(f"h0x{h!r}: {acc['reason']}")
            elif acc["E_up"] > eps:
                bad.append(f"h0x{h!r}: E_up {acc['E_up']!r} > {eps!r}")
        if bad:
            return {"target": eps, "state": "inadmissible", "reason": "; ".join(bad)}
        if unknown:
            return {"target": eps, "state": "unknown", "reason": "; ".join(unknown)}
        return {"target": eps, "state": "admissible", "reason": None}

    def state_at(self, group: dict, eps: float) -> dict:
        return group["targets"][self.targets.index(eps)]

    def _cost(self, arm, problem_id, grid, rtol, g) -> tuple[int | None, str | None]:
        if g["external"]:
            return None, "external arm (no instruction count)"
        if not g["in_profile_subset"]:
            return None, "not_in_profile_subset"
        prof = self.inp.profile_rows.get((arm, problem_id, grid, rtol, NOMINAL_H0))
        if prof is None:
            return None, "profile_missing"
        if prof.get("status") != "ok":
            return None, f"profile_failed: {prof.get('reason')}"
        if not profile_is_deterministic(prof, self.ir_tolerance):
            return None, "callgrind_nondeterministic (Ir(1) != Ir(1b) beyond tolerance)"
        if not isinstance(prof.get("summary"), dict) or prof["summary"].get("success") is not True:
            return None, "profile run not successful"
        run = self.inp.rows.get((arm, problem_id, grid, rtol, NOMINAL_H0))
        run_sha = ((run or {}).get("binary") or {}).get("sha256")
        prof_sha = (prof.get("binary") or {}).get("sha256")
        if run_sha and prof_sha and run_sha != prof_sha:
            return None, "binary_mismatch between RUNS and PROFILE"
        ir = prof.get(self.th["frontier"]["primary_cost"])
        if type(ir) is not int or ir <= 0:
            return None, f"non-positive or missing {self.th['frontier']['primary_cost']}: {ir!r}"
        return ir, None

    def _counted_work(self, row: dict | None) -> tuple[int | None, str | None]:
        """Sum of THRESHOLDS.external.counted_work units of the nominal (h0 = 1) row: WorkCounters fields for
        internal rows, normalized upstream counters in extra_work for external rows (INTERNAL_UNITS /
        EXTERNAL_UNITS). Null with a reason when the row is not ok or a unit is unavailable."""
        if row is None or row["status"] != "ok":
            return None, "nominal row missing or not ok"
        external = tc.is_external(row["arm"])
        source = (row["extra_work"] if external else row["counters"]) or {}
        mapping = EXTERNAL_UNITS if external else INTERNAL_UNITS
        total, missing = 0, []
        for unit in self.work_units:
            value = source.get(mapping[unit])
            if type(value) is not int or value < 0:
                missing.append(f"{unit} ({'extra_work' if external else 'counters'}.{mapping[unit]})")
            else:
                total += value
        if missing:
            return None, "unit unavailable: " + ", ".join(missing)
        return total, None

    def _secondary(self, arm, problem_id, grid, rtol) -> tuple[dict, dict]:
        key = (arm, problem_id, grid, rtol, NOMINAL_H0)
        row, prof, alloc = self.inp.rows.get(key), self.inp.profile_rows.get(key), self.inp.alloc_rows.get(key)
        values, reasons = {}, {}
        for name in self.secondary_names:
            value, reason = None, None
            if name == "ir_cold":
                ok = prof is not None and prof.get("status") == "ok" and profile_is_deterministic(prof, self.ir_tolerance)
                value = prof.get("ir_cold") if ok else None
                reason = None if ok else "profile missing, failed or nondeterministic"
            elif name in ("allocations_warm", "allocated_bytes_warm"):
                field = "allocations" if name == "allocations_warm" else "allocated_bytes"
                ok = alloc is not None and alloc.get("status") == "ok" and isinstance(alloc.get("warm"), dict)
                value = alloc["warm"].get(field) if ok else None
                reason = None if ok else "allocation row missing or not ok"
            else:
                if row is None or row["status"] != "ok":
                    reason = "run row missing or not ok"
                elif tc.is_external(arm):
                    upstream = {"rhs_evaluations": "nfev", "direct_factorizations": "nlu"}.get(name)
                    value = (row["extra_work"] or {}).get(upstream) if upstream else None
                    reason = None if value is not None else f"external arm reports no {name}"
                else:
                    value = (row["counters"] or {}).get(name)
                    reason = None if value is not None else f"counters lack {name}"
            if value is not None and (type(value) is not int or value < 0):
                value, reason = None, f"{name} is not a nonnegative int"
            values[name] = value
            if reason:
                reasons[name] = reason
        return values, reasons

    # ------------------------------------------------------------------------ arm cells
    def applicable(self, arm: str, problem_id: str) -> bool:
        return problem_id in self.arm_problems.get(arm, set())

    def arm_cell(self, arm: str, problem_id: str, grid: str, eps: float) -> dict:
        """State, configuration cost and counted work of an arm at (problem, grid, eps)."""
        key = (arm, problem_id, grid, eps)
        if key in self.arm_cell_cache:
            return self.arm_cell_cache[key]
        groups = [self.group(arm, problem_id, grid, r) for r in self.rtols]
        states = [(g, self.state_at(g, eps)) for g in groups]
        admissible = [g for g, s in states if s["state"] == "admissible"]
        unknown = [g for g, s in states if s["state"] == "unknown"]
        if admissible:
            state = "admissible"
        elif unknown:
            state = "unknown"
        else:
            state = "inadmissible"
        reasons = [f"rtol {g['rtol']!r}: {s['reason']}" for g, s in states if s["state"] != "admissible"]
        out = {"state": state, "reasons": reasons, "cost": None, "rtol": None, "complete": False,
               "cost_reason": None, "E_up": None, "work": None, "work_complete": False, "work_rtol": None,
               "work_reason": None if state == "admissible" else f"state {state}", "group": None}
        if state == "admissible":
            candidates = [g for g in admissible if g["in_profile_subset"]]
            known = [g for g in candidates if g["cost"] is not None]
            missing = [f"rtol {g['rtol']!r}: {g['cost_reason']}" for g in candidates if g["cost"] is None]
            unknown_candidates = [f"rtol {g['rtol']!r}: state unknown" for g in unknown if g["in_profile_subset"]]
            if known:
                best = min(known, key=lambda g: (g["cost"], g["E_up_max"], g["rtol"]))
                out.update(cost=best["cost"], rtol=best["rtol"], E_up=best["E_up_max"], group=best)
            out["complete"] = bool(known) and not missing and not unknown_candidates
            if not candidates:
                out["cost_reason"] = "no admissible group in the profile subset" if not tc.is_external(arm) \
                    else "external arm (no instruction count)"
            elif not out["complete"]:
                out["cost_reason"] = "cost incomplete: " + "; ".join(missing + unknown_candidates)
            missing_work = [f"rtol {g['rtol']!r}: {g['counted_work_reason']}" for g in admissible
                            if g["counted_work"] is None]
            if unknown:
                out["work_reason"] = "insufficient_evidence: " + "; ".join(
                    f"rtol {g['rtol']!r} state unknown" for g in unknown)
            elif missing_work:
                out["work_reason"] = "; ".join(missing_work)
            else:
                best = min(admissible, key=lambda g: (g["counted_work"], g["rtol"]))
                out.update(work=best["counted_work"], work_rtol=best["rtol"], work_complete=True)
        self.arm_cell_cache[key] = out
        return out

    # ------------------------------------------------------------------------ comparisons
    def outcome_for_ratio(self, ratio: float) -> str:
        fr = self.th["frontier"]
        if ratio <= fr["gain_max"]:
            return "GAIN"
        if ratio <= fr["neutral_max"]:
            return "NEUTRAL"
        return "LOSS"

    def compare(self, component: str, arm: str, kind: str, rivals: list[str], problem_id: str, grid: str,
                eps: float, named: str | None = None) -> dict:
        """One comparison cell. `rivals`: the applicable counterpart arms (attribution: [a'] or [] when a' is not
        applicable to the problem, then `named` = a'; strongest: every applicable arm of another slot member)."""
        problem = self.inp.problems[problem_id]
        rec = {"component": component, "arm": arm, "kind": kind, "counterpart": named, "problem": problem_id,
               "grid": grid, "target": eps, "tier": problem["tier"], "partition": problem["partition"],
               "outcome": "excluded", "ratio": None, "cost": None, "counterpart_cost": None, "rtol": None,
               "counterpart_rtol": None, "reason": None, "secondary": None}
        ref = self.reference(problem_id, grid)
        if not ref.ok:
            rec["reason"] = f"reference: {ref.reason}"
            return rec
        if self.reference_limited(problem_id, grid, eps):
            rec["reason"] = f"reference_limited: E_rho {ref.e_rho!r} > {self.ref_factor!r} * {eps!r}"
            return rec
        mine = self.arm_cell(arm, problem_id, grid, eps)
        rec["cost"], rec["rtol"] = mine["cost"], mine["rtol"]
        if mine["state"] == "unknown":
            rec["reason"] = "insufficient_evidence: " + "; ".join(mine["reasons"])
            return rec
        cells = {r: self.arm_cell(r, problem_id, grid, eps) for r in rivals}
        undetermined = [r for r, s in cells.items()
                        if s["state"] == "unknown" or (kind == "strongest" and s["state"] == "admissible"
                                                       and not s["complete"])]
        if undetermined:
            rec["reason"] = f"insufficient_evidence: rival(s) {undetermined} unknown or with incomplete cost"
            return rec
        admissible = {r: s for r, s in cells.items() if s["state"] == "admissible"}
        if admissible:
            counterpart = min(admissible, key=lambda r: (admissible[r]["cost"] if admissible[r]["cost"] is not None
                                                         else math.inf, r))
        else:
            counterpart = sorted(cells)[0] if cells else named
        rec["counterpart"] = counterpart
        if counterpart in cells:
            rec["counterpart_cost"], rec["counterpart_rtol"] = cells[counterpart]["cost"], cells[counterpart]["rtol"]
        if mine["state"] == "admissible" and not admissible:
            rec["outcome"] = "capability_gain"
            rec["reason"] = ("counterpart not applicable to the problem" if kind == "attribution" and not rivals
                             else "no applicable rival arm" if not rivals else "no rival admissible")
            return rec
        if mine["state"] == "inadmissible":
            if admissible:
                rec["outcome"] = "capability_loss"
            else:
                rec["reason"] = "neither_admissible" if rivals else "neither_admissible (no applicable rival)"
            return rec
        other = cells[counterpart]
        if not mine["complete"] or not other["complete"] or mine["cost"] is None or other["cost"] is None:
            rec["reason"] = ("insufficient_evidence: cost: "
                             f"{mine['cost_reason'] or 'ok'} | counterpart: {other['cost_reason'] or 'ok'}")
            return rec
        rec["ratio"] = mine["cost"] / other["cost"]
        rec["outcome"] = self.outcome_for_ratio(rec["ratio"])
        rec["secondary"] = {name: {"value": mine["group"]["secondary"][name],
                                   "counterpart": other["group"]["secondary"][name]}
                            for name in self.secondary_names}
        return rec

    def counterparts(self, component: str, arm: str) -> list[str]:
        """Arms identical to `arm` outside c's slot and without c.

        A slot can hold several components of one arm (the Legacy LU policy of the fast driver is small LU,
        faer LU and the density switch together), and an opt-in feature's baseline is the arm without it, so
        the counterpart replaces the whole slot content rather than one component.
        """
        slot = self.inp.comp[component]["slot"]
        in_slot = {c for c, spec in self.inp.comp.items() if spec["slot"] == slot}
        outside = self.inp.arm_components[arm] - in_slot
        out = []
        for other in self.internal_arms:
            other_comps = self.inp.arm_components.get(other, set())
            if other != arm and component not in other_comps and other_comps - in_slot == outside:
                out.append(other)
        return sorted(out)

    def strongest_rivals(self, component: str) -> list[str]:
        slot = self.inp.comp[component]["slot"]
        rivals = {c for c, spec in self.inp.comp.items() if spec["slot"] == slot and c != component}
        return sorted(a for a in self.internal_arms
                      if component not in self.inp.arm_components.get(a, set())
                      and self.inp.arm_components.get(a, set()) & rivals)

    def comparisons(self, component: str) -> list[dict]:
        spec = self.inp.comp[component]
        tiers = set(spec["tiers"])
        out = []
        arms = sorted(a for a in self.internal_arms if component in self.inp.arm_components.get(a, set()))
        strongest = self.strongest_rivals(component)
        for arm in arms:
            counterparts = self.counterparts(component, arm)
            for pid in sorted(self.arm_problems.get(arm, set())):
                if self.inp.problems[pid]["tier"] not in tiers:
                    continue
                for grid in self.grids:
                    for eps in self.targets:
                        if counterparts:
                            for cp in counterparts:
                                rivals = [cp] if self.applicable(cp, pid) else []
                                out.append(self.compare(component, arm, "attribution", rivals, pid, grid, eps,
                                                        named=cp))
                        else:
                            applicable = [r for r in strongest if self.applicable(r, pid)]
                            out.append(self.compare(component, arm, "strongest", applicable, pid, grid, eps))
        return out


# ================================================================================ component criteria
def integrity_failures(engine: Engine) -> dict[str, list[str]]:
    """C1: integrity failures attributed to components (with exoneration by a clean arm on the same cell)."""
    inp = engine.inp
    clean: dict[tuple, set] = defaultdict(set)
    failures: list[tuple] = []
    for key, row in sorted(inp.rows.items()):
        if tc.is_external(row["arm"]):
            continue
        acc = engine.accuracy(key)
        problem = None
        if acc["integrity"] is not None:
            problem = acc["integrity"]
        elif row["deterministic"] is False:
            problem = "nondeterministic"
        comps = inp.arm_components.get(row["arm"], set(row["components"]))
        cell = key[1:]
        if problem is None:
            for c in comps:
                clean[(c, cell)].add(row["arm"])
        else:
            failures.append((row["arm"], cell, problem, comps))
    out: dict[str, list[str]] = defaultdict(list)
    for arm, cell, problem, comps in failures:
        for c in sorted(comps):
            if not clean.get((c, cell)):
                out[c].append(f"{arm} {cell[0]} {cell[1]} rtol {cell[2]!r} h0x{cell[3]!r}: {problem}")
    return out


def probe_results(engine: Engine) -> dict[str, list[tuple[str, bool, str]]]:
    out: dict[str, list] = defaultdict(list)
    for row in engine.inp.probe_rows:
        probe = row["probe"]
        comps = probe.get("components") or row["components"]
        status, success = row["status"], row["success"]
        if status == "process_abort":
            ok, why = False, "process_abort"
        elif probe["must_refuse"]:
            ok = status in ("refused", "error") and success is False
            why = "typed refusal" if ok else f"must refuse, got status {status} success {success}"
        else:
            ok, why = True, f"status {status}"
        for c in comps:
            out[c].append((probe["id"], ok, why))
    return out


def c2_criterion(engine: Engine, component: str) -> dict:
    rel = engine.th["reliability_c2"]
    groups = []
    excluded = 0
    for (arm, pid, grid, rtol), g in sorted(engine.groups.items()):
        if arm not in engine.inp.arm_components or component not in engine.inp.arm_components[arm]:
            continue
        if g["R_up_max"] is None:
            continue
        if g["R_rho_max"] > engine.ref_factor:
            excluded += 1
            continue
        groups.append(g["R_up_max"])
    if not groups:
        return {"status": None, "groups": 0, "within_ratio": 0, "fraction": None, "max_R_up": None,
                "reference_limited_groups": excluded, "reason": "insufficient_evidence: no group with a computed R_up"}
    within = sum(1 for r in groups if r <= rel["ratio"])
    fraction = within / len(groups)
    worst = max(groups)
    passed = fraction >= rel["cell_fraction"] and worst <= rel["ratio_max"]
    return {"status": "pass" if passed else "fail", "groups": len(groups), "within_ratio": within,
            "fraction": fraction, "max_R_up": fnum(worst), "reference_limited_groups": excluded,
            "reason": None if passed else f"fraction {fraction!r} (need >= {rel['cell_fraction']!r}) or max R_up "
                                          f"{worst!r} (need <= {rel['ratio_max']!r})"}


def allocation_criterion(engine: Engine, component: str) -> dict:
    th = engine.th["allocation"]
    rows = [(k, r) for k, r in sorted(engine.inp.alloc_rows.items())
            if component in engine.inp.arm_components.get(k[0], set())]
    if not rows:
        return {"status": None, "rows": 0, "max_warm_allocations": None, "rule": None,
                "reason": "insufficient_evidence: no ALLOCATIONS rows for the component's arms"}
    bad = [f"{k}: {r.get('status')} {r.get('reason')}" for k, r in rows
           if r.get("status") != "ok" or not isinstance(r.get("warm"), dict)]
    if bad:
        return {"status": None, "rows": len(rows), "max_warm_allocations": None, "rule": None,
                "reason": "insufficient_evidence: allocation rows not ok: " + "; ".join(bad[:5])}
    warm = [r["warm"]["allocations"] for _, r in rows]
    worst = max(warm)
    if worst <= th["warm_window_allocations_max"]:
        return {"status": "pass", "rows": len(rows), "max_warm_allocations": worst, "rule": "warm_window_max",
                "reason": None}
    documented = engine.inp.comp[component].get("documented_warm_allocations")
    if th["allow_documented_constant_independent_of_n"] and type(documented) is int:
        dims = {engine.inp.problems[k[1]]["dimension"] for k, _ in rows}
        if all(w == documented for w in warm) and len(dims) >= 2:
            return {"status": "pass", "rows": len(rows), "max_warm_allocations": worst,
                    "rule": "documented_constant_independent_of_n", "reason": None}
        return {"status": "fail", "rows": len(rows), "max_warm_allocations": worst,
                "rule": "documented_constant_independent_of_n",
                "reason": f"warm allocations {sorted(set(warm))} != documented {documented} on >= 2 dimensions "
                          f"(dimensions {sorted(dims)})"}
    return {"status": "fail", "rows": len(rows), "max_warm_allocations": worst, "rule": "warm_window_max",
            "reason": f"max warm allocations {worst} > {th['warm_window_allocations_max']}"}


def cell_features(engine: Engine, rec: dict) -> dict:
    p = engine.inp.problems[rec["problem"]]
    features = {}
    for name in engine.th["specialize"]["features"]:
        if name == "grid":
            features[name] = rec["grid"]
        elif name == "rtol":
            features[name] = rec["target"]
        else:
            features[name] = p.get(name)
    return features


def aggregate_cells(engine: Engine, comps: list[dict]) -> list[dict]:
    """One label per (problem, grid, target): loss if any comparison loses, else win if any wins, else neutral."""
    cells: dict[tuple, dict] = {}
    for rec in comps:
        if rec["outcome"] == "excluded":
            continue
        key = (rec["problem"], rec["grid"], rec["target"])
        cell = cells.setdefault(key, {"problem": rec["problem"], "grid": rec["grid"], "target": rec["target"],
                                      "partition": rec["partition"], "outcomes": set(),
                                      "features": cell_features(engine, rec)})
        cell["outcomes"].add(rec["outcome"])
    out = []
    for key in sorted(cells):
        cell = cells[key]
        if cell["outcomes"] & set(LOSS_OUTCOMES):
            cell["label"] = "loss"
        elif cell["outcomes"] & set(WIN_OUTCOMES):
            cell["label"] = "win"
        else:
            cell["label"] = "neutral"
        out.append(cell)
    return out


def _cat_key(v):
    return (str(type(v).__name__), str(v))


def _is_numeric(values) -> bool:
    return bool(values) and all(type(v) in (int, float) and not isinstance(v, bool) for v in values)


def find_predicate(engine: Engine, cells: list[dict]) -> tuple[dict | None, str]:
    """Best single-feature predicate on calibration cells: no loss inside, most wins inside."""
    cal = [c for c in cells if c["partition"] == "calibration"]
    wins = [c for c in cal if c["label"] == "win"]
    losses = [c for c in cal if c["label"] == "loss"]
    if not wins:
        return None, "no calibration win cell"
    best = None
    for order, name in enumerate(engine.th["specialize"]["features"]):
        values = [c["features"][name] for c in cal]
        candidates = []
        if _is_numeric(values) and name in NUMERIC_FEATURES:
            for op, sign in (("le", 1.0), ("ge", -1.0)):
                scores = sorted({sign * v for v in values})
                loss_scores = [sign * c["features"][name] for c in losses]
                limit = min(loss_scores) if loss_scores else math.inf
                valid = [s for s in scores if s < limit
                         and any(sign * c["features"][name] <= s for c in wins)]
                if valid:
                    tau = max(valid)
                    covered = sum(1 for c in wins if sign * c["features"][name] <= tau)
                    candidates.append(({"feature": name, "op": op, "threshold": sign * tau, "values": None}, covered))
        else:
            win_values = {_cat_key(c["features"][name]): c["features"][name] for c in wins}
            loss_values = {_cat_key(c["features"][name]) for c in losses}
            chosen = sorted((k for k in win_values if k not in loss_values))
            if chosen:
                covered = sum(1 for c in wins if _cat_key(c["features"][name]) in set(chosen))
                candidates.append(({"feature": name, "op": "in", "threshold": None,
                                    "values": [win_values[k] for k in chosen]}, covered))
        for op_order, (pred, covered) in enumerate(candidates):
            rank = (-covered, order, op_order)
            if best is None or rank < best[0]:
                best = (rank, pred, covered)
    if best is None:
        return None, "no single-feature predicate separates calibration wins from losses"
    pred = dict(best[1])
    pred["calibration"] = {"cells": len(cal), "wins": len(wins), "losses": len(losses),
                           "wins_covered": best[2]}
    return pred, "found"


def predicate_holds(pred: dict, features: dict) -> bool:
    v = features[pred["feature"]]
    if pred["op"] == "in":
        return _cat_key(v) in {_cat_key(x) for x in pred["values"]}
    if pred["op"] == "le":
        return v <= pred["threshold"]
    return v >= pred["threshold"]


def holdout_validation(engine: Engine, pred: dict, cells: list[dict]) -> dict:
    sp = engine.th["specialize"]
    hold = [c for c in cells if c["partition"] == "holdout"]
    wins_recommended = sum(1 for c in hold if c["label"] == "win" and predicate_holds(pred, c["features"]))
    if pred["op"] in ("le", "ge"):
        sign = 1.0 if pred["op"] == "le" else -1.0
        events = [{"score": sign * float(c["features"][pred["feature"]]), "positive": c["label"] == "loss"}
                  for c in hold]
        raw = hv.holdout_verdict(events, sign * pred["threshold"])
        n_pos, rank_stable, at_tau = raw["n_positives"], raw["rank_stable"], raw["raw_verdict_at_tau"]
        details = {"tau_one_rank_down": fnum(raw["tau_one_rank_down"]),
                   "tau_one_rank_up": fnum(raw["tau_one_rank_up"]),
                   "verdict_one_rank_down": raw["verdict_one_rank_down"],
                   "verdict_one_rank_up": raw["verdict_one_rank_up"], "rank_shift": "one distinct-score rank"}
    else:
        events = [{"score": 0.0 if predicate_holds(pred, c["features"]) else 1.0, "positive": c["label"] == "loss"}
                  for c in hold]
        at_tau = hv.verdict_at(events, 0.0)
        n_pos = sum(1 for e in events if e["positive"])
        rank_stable = None
        details = {"tau_one_rank_down": None, "tau_one_rank_up": None, "verdict_one_rank_down": None,
                   "verdict_one_rank_up": None, "rank_shift": "not_applicable_category_set"}
    if n_pos < sp["min_positives"]:
        verdict = "INCONCLUSIVE_INSUFFICIENT_POSITIVES"
    elif rank_stable is False:
        verdict = "INCONCLUSIVE_RANK_UNSTABLE"
    else:
        verdict = at_tau
    out = {"verdict": verdict, "raw_verdict_at_tau": at_tau, "n_events": len(events), "n_positives": n_pos,
           "minimum_positives": sp["min_positives"], "rank_stable": rank_stable,
           "holdout_wins_recommended": wins_recommended,
           "miss_rate_upper_95_if_zero_misses": fnum(hv.miss_rate_upper_95(n_pos))}
    out.update(details)
    out["passed"] = verdict == "PASS" and wins_recommended >= 1
    return out


def external_gap_cells(engine: Engine, component: str) -> list[dict]:
    """OPTIMIZE(b) matched-error frontier of counted work against the component's external peers
    (COMPONENTS[<THRESHOLDS.external.peers_field>]): per (arm, peer, problem, grid, eps) the minimum counted work
    over admissible groups of each side (internal: all required h0; external: the single nominal row)."""
    spec = engine.inp.comp[component]
    peers = sorted(spec.get(engine.peers_field) or [])
    arms = sorted(a for a in engine.internal_arms if component in engine.inp.arm_components.get(a, set()))
    out = []
    for arm in arms:
        for peer in peers:
            for pid in sorted(engine.arm_problems.get(arm, set()) & engine.arm_problems.get(peer, set())):
                if engine.inp.problems[pid]["tier"] not in set(spec["tiers"]):
                    continue
                for grid in engine.grids:
                    ref = engine.reference(pid, grid)
                    for eps in engine.targets:
                        rec = {"component": component, "arm": arm, "peer": peer, "problem": pid, "grid": grid,
                               "target": eps, "work": None, "peer_work": None, "rtol": None, "peer_rtol": None,
                               "gap": None, "reason": None}
                        if not ref.ok:
                            rec["reason"] = f"reference: {ref.reason}"
                        elif engine.reference_limited(pid, grid, eps):
                            rec["reason"] = "reference_limited"
                        else:
                            mine, theirs = engine.arm_cell(arm, pid, grid, eps), engine.arm_cell(peer, pid, grid, eps)
                            rec.update(work=mine["work"], peer_work=theirs["work"], rtol=mine["work_rtol"],
                                       peer_rtol=theirs["work_rtol"])
                            if mine["state"] != "admissible" or theirs["state"] != "admissible":
                                rec["reason"] = f"not matched: arm {mine['state']}, peer {theirs['state']}"
                            elif not mine["work_complete"]:
                                rec["reason"] = f"arm counted work: {mine['work_reason']}"
                            elif not theirs["work_complete"]:
                                rec["reason"] = f"peer counted work: {theirs['work_reason']}"
                            elif theirs["work"] == 0:
                                rec["reason"] = "peer counted work is 0"
                            else:
                                rec["gap"] = mine["work"] / theirs["work"]
                        out.append(rec)
    return out


def optimize_external_gap(engine: Engine, component: str, cells: list[dict]) -> dict:
    peers = sorted(engine.inp.comp[component].get(engine.peers_field) or [])
    gaps = [c["gap"] for c in cells if c["gap"] is not None]
    base = {"peers": peers, "units": engine.work_units, "cells": len(cells), "matched_cells": len(gaps),
            "median_gap": None, "threshold": engine.th["optimize"]["external_counted_work_gap"], "reason": None}
    if not peers:
        base["reason"] = f"no {engine.peers_field} in COMPONENTS"
    elif not gaps:
        base["reason"] = "insufficient_evidence: no matched cell with complete counted work on both sides"
    else:
        base["median_gap"] = statistics.median(gaps)
    return base


def optimize_headroom(engine: Engine, component: str, comps: list[dict]) -> dict:
    losses = [r for r in comps if r["outcome"] == "LOSS"]
    limit = engine.th["optimize"]["headroom_ratio_max"]
    if not losses:
        return {"status": None, "loss_cells": 0, "adjusted_max": None, "reason": "no LOSS comparison"}
    if engine.inp.attribution is None:
        return {"status": None, "loss_cells": len(losses), "adjusted_max": None,
                "reason": "insufficient_evidence: no ATTRIBUTION.json"}
    table = {}
    for e in engine.inp.attribution.get("entries") or []:
        table[(e.get("component"), e.get("arm"), e.get("problem"), e.get("grid"), e.get("rtol"))] = e
    adjusted, missing = [], []
    for r in losses:
        entry = table.get((component, r["arm"], r["problem"], r["grid"], r["rtol"]))
        removable = None if entry is None else entry.get("removable_ir")
        if type(removable) is not int or removable < 0 or removable > r["cost"]:
            missing.append(f"{r['arm']} {r['problem']} {r['grid']} rtol {r['rtol']!r}")
            continue
        adjusted.append((r["cost"] - removable) / r["counterpart_cost"])
    if missing:
        return {"status": None, "loss_cells": len(losses), "adjusted_max": fnum(max(adjusted)) if adjusted else None,
                "reason": f"insufficient_evidence: no valid attribution for {len(missing)} LOSS cell(s): "
                          + "; ".join(missing[:5])}
    worst = max(adjusted)
    return {"status": "pass" if worst <= limit else "fail", "loss_cells": len(losses), "adjusted_max": worst,
            "reason": None if worst <= limit else f"adjusted ratio {worst!r} > {limit!r}"}


def timing_entries(engine: Engine) -> tuple[list[dict], bool | None]:
    if engine.inp.timing is None:
        return [], None
    threshold = engine.th["timing"]["conflict_threshold"]
    host = engine.inp.timing.get("host")
    diagnostic = not (isinstance(host, dict) and host.get("wall_clock_admissible") is True)
    out = []
    for e in engine.inp.timing.get("entries") or []:
        rec = {"arm": e.get("arm"), "counterpart": e.get("counterpart"), "problem": e.get("problem"),
               "grid": e.get("grid"), "rtol": e.get("rtol"), "wall_ratio": fnum(e.get("wall_ratio")),
               "interval": e.get("interval"), "ir_ratio": None, "conflict": False, "reason": None}
        costs = []
        for arm in (rec["arm"], rec["counterpart"]):
            prof = engine.inp.profile_rows.get((arm, rec["problem"], rec["grid"], rec["rtol"], NOMINAL_H0))
            ok = prof is not None and prof.get("status") == "ok" \
                and profile_is_deterministic(prof, engine.ir_tolerance) \
                and type(prof.get("ir_warm")) is int and prof["ir_warm"] > 0
            costs.append(prof["ir_warm"] if ok else None)
        interval = rec["interval"]
        if None in costs:
            rec["reason"] = "no deterministic PROFILE ir_warm for both arms"
        elif rec["wall_ratio"] is None or not (isinstance(interval, list) and len(interval) == 2
                                               and all(tc.finite(x) for x in interval)):
            rec["reason"] = "malformed wall_ratio or interval"
        else:
            rec["ir_ratio"] = costs[0] / costs[1]
            wall, ir = rec["wall_ratio"], rec["ir_ratio"]
            opposite = (wall - 1.0) * (ir - 1.0) < 0.0
            excludes_one = interval[0] > 1.0 or interval[1] < 1.0
            rec["conflict"] = bool(opposite and abs(wall - 1.0) > threshold and excludes_one)
        out.append(rec)
    return out, diagnostic


# ================================================================================== classification
def step(path: list[dict], rule: str, detail: str | None) -> None:
    """Append one rule_path entry {rule, kind, detail}; the kinds are fixed in RULE_KINDS."""
    path.append({"rule": rule, "kind": RULE_KINDS[rule], "detail": detail})


def classify(engine: Engine) -> tuple[list[dict], list[dict], list[dict]]:
    th = engine.th
    fr = th["frontier"]
    engine.ensure_groups()
    c1 = integrity_failures(engine)
    probes = probe_results(engine)
    timing, _ = timing_entries(engine)
    all_comparisons: dict[str, list[dict]] = {}
    all_gap_cells: list[dict] = []
    for cid in sorted(engine.inp.comp):
        all_comparisons[cid] = engine.comparisons(cid)
    entries = []
    for cid in sorted(engine.inp.comp):
        spec = engine.inp.comp[cid]
        comps = all_comparisons[cid]
        path, reasons, insufficient, flags = [], [], [], []
        counts = {o: sum(1 for r in comps if r["outcome"] == o) for o in OUTCOMES}
        ratios = [r["ratio"] for r in comps if r["ratio"] is not None]
        arms = sorted(a for a in engine.internal_arms if cid in engine.inp.arm_components.get(a, set()))
        runs_rows = sum(1 for k in engine.inp.rows if k[0] in arms)
        # criteria
        c1_failures = c1.get(cid, [])
        crit_c1 = {"status": "fail" if c1_failures else ("pass" if runs_rows else None),
                   "failures": c1_failures[:50], "failures_total": len(c1_failures),
                   "reason": None if runs_rows else "insufficient_evidence: no rows of the component's arms"}
        probe_list = probes.get(cid, [])
        crit_c3 = {"status": None if not probe_list else ("pass" if all(ok for _, ok, _ in probe_list) else "fail"),
                   "probes": len(probe_list),
                   "failures": [f"{pid}: {why}" for pid, ok, why in probe_list if not ok][:50],
                   "reason": None if probe_list else "insufficient_evidence: no PROBES rows for the component"}
        crit_c2 = c2_criterion(engine, cid)
        crit_alloc = allocation_criterion(engine, cid)
        for name, crit in (("C1", crit_c1), ("C2", crit_c2), ("C3", crit_c3), ("allocation", crit_alloc)):
            if crit["status"] is None:
                insufficient.append(f"{name}: {crit['reason']}")
        # provisional
        provisional = bool(spec["blocked_by"])
        if provisional:
            step(path, "BLOCKED_BY", f"blocked_by {spec['blocked_by']} -> provisional, PROMOTE forbidden")
        my_timing = [t for t in timing if cid in engine.inp.arm_components.get(t["arm"], set())
                     and cid not in engine.inp.arm_components.get(t["counterpart"], set())]
        conflicts = [t for t in my_timing if t["conflict"]]
        if conflicts:
            flags.append("TIMING_CONFLICT")
            provisional = True
            step(path, "TIMING_CONFLICT", f"wall ratio contradicts Ir on {len(conflicts)} cell(s) -> provisional")
        crit_timing = {"status": "fail" if conflicts else ("pass" if my_timing else None), "entries": len(my_timing),
                       "conflicts": len(conflicts),
                       "reason": None if my_timing else "no TIMING entries for the component (diagnostic only)"}
        # PROMOTE
        gp = [r for r in comps if r["outcome"] != "excluded" and r["tier"] in ("G", "P")]
        good = [r for r in gp if r["outcome"] in ("GAIN", "NEUTRAL", "capability_gain")]
        fraction = len(good) / len(gp) if gp else None
        max_ratio = max(ratios) if ratios else None
        promote_checks = {
            "gp_cells": len(gp) > 0,
            "fraction": fraction is not None and fraction >= fr["promote_cell_fraction"],
            "max_ratio": max_ratio is None or max_ratio <= fr["promote_max_ratio"],
            "c2": crit_c2["status"] == "pass",
            "allocation": crit_alloc["status"] == "pass",
            "c1": crit_c1["status"] == "pass",
        }
        promote_met = all(promote_checks.values())
        crit_promote = {"status": "pass" if promote_met else "fail", "c3": crit_c3["status"],
                        "gp_cells": len(gp), "gp_good": len(good),
                        "fraction": fraction, "max_ratio": max_ratio, "checks": promote_checks,
                        "reason": None if promote_met else "failed: " + ", ".join(k for k, v in promote_checks.items()
                                                                                  if not v)}
        # SPECIALIZE
        cells = aggregate_cells(engine, comps)
        win_cells = [c for c in cells if c["label"] == "win"]
        predicate, why = (find_predicate(engine, cells) if win_cells else (None, "empty win set"))
        holdout = holdout_validation(engine, predicate, cells) if predicate else None
        specialize_met = bool(win_cells) and predicate is not None and holdout["passed"]
        crit_specialize = {"status": "pass" if specialize_met else "fail", "win_cells": len(win_cells),
                           "loss_cells": sum(1 for c in cells if c["label"] == "loss"),
                           "neutral_cells": sum(1 for c in cells if c["label"] == "neutral"),
                           "search": why, "holdout_verdict": None if holdout is None else holdout["verdict"],
                           "candidate_predicate": predicate, "holdout": holdout,
                           "reason": None if specialize_met else (why if predicate is None else
                                                                  f"holdout {holdout['verdict']}, holdout wins "
                                                                  f"recommended {holdout['holdout_wins_recommended']}")}
        # OPTIMIZE
        headroom = optimize_headroom(engine, cid, comps)
        gap_cells = external_gap_cells(engine, cid)
        all_gap_cells.extend(gap_cells)
        gap = optimize_external_gap(engine, cid, gap_cells)
        uncharacterized = spec["unique_capability"] and bool(win_cells) and not specialize_met
        optimize_paths = {
            "headroom": headroom["status"] == "pass",
            "external_gap": bool(spec["unique_capability"]) and gap["median_gap"] is not None
            and gap["median_gap"] > th["optimize"]["external_counted_work_gap"],
            "unique_uncharacterized_wins": uncharacterized,
        }
        crit_optimize = {"status": "pass" if any(optimize_paths.values()) else "fail", "paths": optimize_paths,
                         "headroom": headroom, "external_gap": {k: (fnum(v) if k == "median_gap" else v)
                                                                for k, v in gap.items()},
                         "reason": None}
        # decision tree (tools/triage/README.md, "Decision tree")
        static = spec["static_class"] == "PRUNE_RELOCATE"
        decided = sum(counts[o] for o in OUTCOMES if o != "excluded")
        label, prune_kind, pred_out = None, None, None
        causes = []
        if spec["blocked_by"]:
            causes.append(f"blocked_by {spec['blocked_by']}")
        if conflicts:
            causes.append("TIMING_CONFLICT")
        if not static and crit_c3["status"] is None:
            flags.append("C3_MISSING")
            provisional = True
            causes.append("C3 missing")
            step(path, "C3_MISSING", crit_c3["reason"])
        if static:
            label, prune_kind = "PRUNE", "relocate"
            step(path, "STATIC_PRUNE_RELOCATE", "static_class PRUNE_RELOCATE -> PRUNE(relocate)")
        elif crit_c1["status"] == "fail" or crit_c3["status"] == "fail":
            repair = spec.get("repair_path")
            repairable = repair is not None and spec["unique_capability"]
            outcome = "see C_FAIL_REPAIRABLE" if repairable else "-> PRUNE"
            if crit_c1["status"] == "fail":
                step(path, "C1_FAIL", f"{crit_c1['failures_total']} integrity failure(s) {outcome}")
            if crit_c3["status"] == "fail":
                step(path, "C3_FAIL", f"{len(crit_c3['failures'])} fail-closed probe failure(s) {outcome}")
            if repairable:
                label = "OPTIMIZE"
                provisional = True
                step(path, "C_FAIL_REPAIRABLE", repair)
            else:
                label = "PRUNE"
                if repair is not None:
                    reasons.append("repair_path given but not a unique_capability: PRUNE (prereg rule 1)")
        else:
            step(path, "CORRECTNESS_OK", f"C1 {crit_c1['status'] or 'null'}, C3 {crit_c3['status'] or 'null'}")
            if decided == 0:
                flags.append("NO_EVIDENCE")
                provisional = True
                insufficient.append("no usable (non-excluded) comparison")
                label = "OPTIMIZE" if spec["unique_capability"] else "PRUNE"
                step(path, "NO_EVIDENCE", f"no usable comparison; unique_capability {spec['unique_capability']} "
                                          f"-> {label} (provisional, insufficient_evidence)")
            elif promote_met and not provisional:
                label = "PROMOTE"
                step(path, "PROMOTE", f"G/P fraction {fraction!r} >= {fr['promote_cell_fraction']!r}, max ratio "
                                      f"{max_ratio!r} <= {fr['promote_max_ratio']!r}, C1/C2/C3/allocation pass")
            elif promote_met:
                label = "OPTIMIZE"
                flags.append("PROMOTE_BLOCKED_PROVISIONAL")
                step(path, "PROMOTE_BLOCKED_PROVISIONAL", "PROMOTE criteria met but provisional ("
                     + ", ".join(causes) + ") -> OPTIMIZE")
            else:
                step(path, "NOT_PROMOTE", crit_promote["reason"])
                if specialize_met:
                    label = "SPECIALIZE"
                    pred_out = dict(predicate)
                    pred_out["holdout"] = holdout
                    value = predicate["threshold"] if predicate["op"] != "in" else predicate["values"]
                    step(path, "SPECIALIZE", f"{predicate['feature']} {predicate['op']} {value!r}, holdout "
                                             f"{holdout['verdict']}" + (" (provisional)" if provisional else ""))
                else:
                    step(path, "NOT_SPECIALIZE", crit_specialize["reason"])
                    if optimize_paths["headroom"]:
                        label = "OPTIMIZE"
                        step(path, "OPTIMIZE_HEADROOM", f"adjusted max ratio {headroom['adjusted_max']!r} <= "
                                                        f"{th['optimize']['headroom_ratio_max']!r}")
                    elif optimize_paths["external_gap"]:
                        label = "OPTIMIZE"
                        step(path, "OPTIMIZE_EXTERNAL_GAP", f"median counted-work gap {gap['median_gap']!r} > "
                                                            f"{gap['threshold']!r} over {gap['matched_cells']} cells")
                    elif optimize_paths["unique_uncharacterized_wins"]:
                        label = "OPTIMIZE"
                        step(path, "OPTIMIZE_UNCHARACTERIZED_WINS", f"unique capability, {len(win_cells)} win "
                                                                    f"cell(s) without a validated predicate")
                    else:
                        label = "PRUNE"
                        step(path, "PRUNE_DEFAULT", f"headroom: {headroom['reason'] or headroom['status']}; external "
                                                    f"gap: {gap['reason'] or gap['median_gap']}")
        if counts["excluded"]:
            reasons.append(f"{counts['excluded']} comparison(s) excluded")
        entries.append({
            "id": cid, "family": spec["family"], "slot": spec["slot"], "loc": spec["loc"], "reach": spec["reach"],
            "label": label, "prune_kind": prune_kind, "provisional": provisional, "blocked_by": list(spec["blocked_by"]),
            "flags": sorted(set(flags)), "predicate": pred_out, "rule_path": path,
            "evidence": {"arms": arms, "runs_rows": runs_rows, "comparisons": len(comps), "outcomes": counts,
                         "gp_cells": len(gp), "gp_good": len(good), "max_ratio": fnum(max_ratio),
                         "min_ratio": fnum(min(ratios)) if ratios else None,
                         "kinds": sorted({r["kind"] for r in comps})},
            "criteria": {"c1": crit_c1, "c2": crit_c2, "c3": crit_c3, "allocation": crit_alloc,
                         "promote": crit_promote, "specialize": crit_specialize, "optimize": crit_optimize,
                         "timing": crit_timing},
            "reasons": reasons, "insufficient_evidence": insufficient,
        })
    neutral_tiebreak(engine, entries, all_comparisons)
    flat = [r for cid in sorted(all_comparisons) for r in all_comparisons[cid]]
    for rec in all_gap_cells:
        rec["gap"] = fnum(rec["gap"])
    return entries, flat, all_gap_cells


def neutral_tiebreak(engine: Engine, entries: list[dict], comparisons: dict[str, list[dict]]) -> None:
    by_id = {e["id"]: e for e in entries}
    for slot in sorted({e["slot"] for e in entries}):
        members = sorted(e["id"] for e in entries if e["slot"] == slot)
        for i, a in enumerate(members):
            for b in members[i + 1:]:
                ea, eb = by_id[a], by_id[b]
                if ea["label"] == "PRUNE" or eb["label"] == "PRUNE":
                    continue
                outcomes = set()
                for me, other in ((a, b), (b, a)):
                    for r in comparisons[me]:
                        cp = r["counterpart"]
                        if cp and other in engine.inp.arm_components.get(cp, set()) and r["outcome"] != "excluded":
                            outcomes.add(r["outcome"])
                if outcomes != {"NEUTRAL"}:
                    continue
                if ea["loc"] == eb["loc"]:
                    for e, o in ((ea, b), (eb, a)):
                        step(e["rule_path"], "NEUTRAL_TIE_EQUAL_LOC", f"all comparisons with {o} NEUTRAL at equal "
                                                                      f"LOC {e['loc']}: label kept")
                    continue
                loser, winner = (ea, eb) if ea["loc"] > eb["loc"] else (eb, ea)
                step(loser["rule_path"], "NEUTRAL_TIEBREAK_LOC", f"all comparisons with {winner['id']} NEUTRAL, LOC "
                                                                  f"{loser['loc']} > {winner['loc']} -> PRUNE (was "
                                                                  f"{loser['label']})")
                loser["label"] = "PRUNE"
                loser["predicate"] = None
                loser["prune_kind"] = None


# ========================================================================================= outputs
def results_document(engine: Engine, comparisons: list[dict], gap_cells: list[dict], timing: list[dict],
                     diagnostic) -> dict:
    inp = engine.inp
    rows = []
    for key in sorted(inp.rows):
        acc = engine.accuracy(key)
        row = inp.rows[key]
        rows.append({"arm": key[0], "problem": key[1], "grid": key[2], "rtol": key[3], "h0_scale": key[4],
                     "status": row["status"], "external": tc.is_external(key[0]), "class": acc["class"],
                     "E": fnum(acc["E"]), "E_rho": fnum(acc["E_rho"]), "E_up": fnum(acc["E_up"]), "R": fnum(acc["R"]),
                     "R_rho": fnum(acc["R_rho"]), "R_up": fnum(acc["R_up"]), "reason": acc["reason"],
                     "integrity": acc["integrity"], "accuracy_source": acc["source"]})
    accuracy_summary = {source: sum(1 for r in rows if r["accuracy_source"] == source) for source in ACCURACY_SOURCES}
    accuracy_summary["sidecars_missing"] = engine.sidecar_events.get("sidecar_missing", 0)
    accuracy_summary["sidecars_refused"] = engine.sidecar_events.get("sidecar_refused", 0)
    engine.ensure_groups()
    groups = []
    for key in sorted(engine.groups):
        g = engine.groups[key]
        if not g["present"]:
            continue
        groups.append({"arm": g["arm"], "problem": g["problem"], "grid": g["grid"], "rtol": g["rtol"],
                       "external": g["external"], "targets": g["targets"], "E_up_max": fnum(g["E_up_max"]),
                       "R_up_max": fnum(g["R_up_max"]), "R_rho_max": fnum(g["R_rho_max"]),
                       "in_profile_subset": g["in_profile_subset"], "cost": g["cost"], "cost_reason": g["cost_reason"],
                       "secondary": g["secondary"], "secondary_reasons": g["secondary_reasons"],
                       "counted_work": g["counted_work"], "counted_work_reason": g["counted_work_reason"]})
    references = []
    for pid in sorted(inp.problems):
        for grid in engine.grids:
            ref = engine.reference(pid, grid)
            limited = [eps for eps in engine.targets if engine.reference_limited(pid, grid, eps)]
            references.append({"problem": pid, "grid": grid, "status": ref.status, "reason": None if ref.ok else ref.reason,
                               "E_rho": fnum(ref.e_rho), "reference_limited_targets": limited})
    out_comparisons = []
    for r in comparisons:
        rec = dict(r)
        rec["ratio"] = fnum(rec["ratio"])
        out_comparisons.append(rec)
    return {"schema": RESULTS_SCHEMA, "campaign": inp.campaign, "generator": GENERATOR, "inputs": inp.digests,
            "thresholds": engine.th, "ignored_rows_other_campaigns": inp.ignored_rows, "references": references,
            "rows": rows, "accuracy_summary": accuracy_summary, "groups": groups, "comparisons": out_comparisons,
            "external_gaps": gap_cells,
            "timing": {"present": inp.timing is not None, "diagnostic": diagnostic, "entries": timing}}


def classification_document(engine: Engine, entries: list[dict], results_sha: str) -> dict:
    summary = {label: sum(1 for e in entries if e["label"] == label) for label in LABELS}
    summary["provisional"] = sum(1 for e in entries if e["provisional"])
    summary["components"] = len(entries)
    for e in entries:
        crit = e["criteria"]
        for name in ("c2",):
            for key in ("fraction", "max_R_up"):
                crit[name][key] = fnum(crit[name][key])
        for key in ("fraction", "max_ratio"):
            crit["promote"][key] = fnum(crit["promote"][key])
        crit["optimize"]["headroom"]["adjusted_max"] = fnum(crit["optimize"]["headroom"]["adjusted_max"])
        for pred in (e["predicate"], crit["specialize"]["candidate_predicate"]):
            if pred is not None and pred.get("threshold") is not None:
                pred["threshold"] = fnum(pred["threshold"])
    return {"schema": CLASSIFICATION_SCHEMA, "campaign": engine.inp.campaign, "generator": GENERATOR,
            "inputs": engine.inp.digests, "results_sha256": results_sha, "components": entries, "summary": summary}


def run(args) -> tuple[str, str]:
    inp, reasons = load_inputs(args)
    if reasons:
        raise tc.TriageError(reasons)
    engine = Engine(inp)
    entries, comparisons, gap_cells = classify(engine)
    timing, diagnostic = timing_entries(engine)
    results_text = tc.dumps(results_document(engine, comparisons, gap_cells, timing, diagnostic)) + "\n"
    results_sha = tc.sha256_bytes(results_text.encode())
    classification_text = tc.dumps(classification_document(engine, entries, results_sha)) + "\n"
    return results_text, classification_text


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    node = tc.DEFAULT_NODE
    parser.add_argument("--campaign", required=True, help="campaign id; rows of other campaigns are ignored")
    parser.add_argument("--runs", type=Path, action="append", required=True, help="RUNS.jsonl (repeatable)")
    parser.add_argument("--references", type=Path, required=True)
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--allocations", type=Path, required=True)
    parser.add_argument("--components", type=Path, default=node / "COMPONENTS.json")
    parser.add_argument("--corpus", type=Path, default=node / "CORPUS.json")
    parser.add_argument("--thresholds", type=Path, default=node / "THRESHOLDS.json")
    parser.add_argument("--probes", type=Path, help="PROBES.jsonl (C3); without it C3 is null")
    parser.add_argument("--timing", type=Path, help="TIMING.json (diagnostic; conflict flag only)")
    parser.add_argument("--attribution", type=Path, help="ATTRIBUTION.json (OPTIMIZE headroom)")
    parser.add_argument("--list", type=Path, help="triage-list JSON (applicability of arms without rows)")
    parser.add_argument("--states-dir", type=Path, action="append", default=[],
                        help="directory of <sha256>.f64le.gz state sidecars (repeatable); rows without "
                             "states_hex are recomputed from a verified sidecar, else use accuracy_inline")
    parser.add_argument("--results", type=Path, required=True, help="output RESULTS.json")
    parser.add_argument("--classification", type=Path, required=True, help="output CLASSIFICATION.json")
    parser.add_argument("--force", action="store_true", help="overwrite existing outputs")
    args = parser.parse_args(argv)
    for path in (args.results, args.classification):
        tc.refuse_existing(path, args.force)
    try:
        results_text, classification_text = run(args)
    except tc.TriageError as exc:
        print(f"INVALID inputs ({len(exc.reasons)} reason(s)):", file=sys.stderr)
        for reason in exc.reasons[:50]:
            print("  ", reason, file=sys.stderr)
        return 2
    tc.write_text(args.results, results_text, args.force)
    tc.write_text(args.classification, classification_text, args.force)
    print(f"wrote {args.results} and {args.classification}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
