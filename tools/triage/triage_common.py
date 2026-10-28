#!/usr/bin/env python3
"""Shared helpers of the component triage tools (research/component_triage_20261019).

Everything here is mechanical: strict JSON loading (via ``tools/evidence_schema_v2.loads_strict``), SHA-256 of
inputs, the binary64 hex codec, the contract's cell arithmetic (applicability, output grids, h0), git provenance
for ``VIGILODE_SOURCE_COMMIT``/``VIGILODE_TREE_CLEAN``, and the immutable-output guard. No threshold lives here;
thresholds are read from ``THRESHOLDS.json`` by the tools that judge.

Contract: ``research/component_triage_20261019/HARNESS_CONTRACT.md`` (CT-HARNESS v1).
"""

from __future__ import annotations

import gzip
import hashlib
import json
import math
import os
import struct
import subprocess
import sys
from pathlib import Path
from typing import Any, Iterable

HERE = Path(__file__).resolve().parent
TOOLS = HERE.parent
REPO = TOOLS.parent
for _path in (str(TOOLS), str(HERE)):
    if _path not in sys.path:
        sys.path.insert(0, _path)

import evidence_schema_v2 as evidence  # noqa: E402

ROW_SCHEMA = "vigilode-triage-row-v1"
LIST_SCHEMA = "vigilode-triage-list-v1"
ALLOC_SCHEMA = "vigilode-triage-alloc-v1"
COMPONENTS_SCHEMA = "vigilode-triage-components-v1"
CORPUS_SCHEMA = "vigilode-triage-corpus-v1"
THRESHOLDS_SCHEMA = "vigilode-triage-thresholds-v1"
PROFILE_SCHEMA = "vigilode-triage-profile-v1"
ALLOCATIONS_SCHEMA = "vigilode-triage-allocations-v1"
REFERENCES_SCHEMA = "vigilode-triage-references-v1"
EXACT_SCHEMA = "vigilode-triage-exact-v1"
PARITY_INPUT_SCHEMA = "vigilode-triage-parity-v1"
DEFAULT_NODE = REPO / "research" / "component_triage_20261019"

# Contract section 1 row keys (every row the binary prints carries exactly these).
ROW_KEYS = frozenset((
    "schema", "campaign", "arm", "components", "problem", "tier", "dimension", "rtol", "atol", "grid", "h0_scale",
    "h0", "status", "error", "success", "attempts", "accepted_steps", "rejected_steps", "output_times",
    "states_hex", "counters", "extra_work", "counter_semantics", "deterministic", "repetitions", "binary", "source"))
# Optional keys added by the Python drivers: `process` on process_abort rows, `probe` on PROBES.jsonl rows.
# Optional keys added by the Python drivers: `process` on process_abort rows, `probe` on PROBES.jsonl rows, and
# the state sidecar of a large row (states_hex replaced by states_sha256/states_shape/states_file, plus
# accuracy_inline computed by the runner with row_accuracy below).
STATE_SIDECAR_KEYS = frozenset(("states_sha256", "states_shape", "states_file"))
# Optional keys printed by the binary (crates/rodas5p-cli/src/triage.rs, from ee13822): `error_kind` (string or
# null: core_error_kind of the typed error), `segments` (int >= 1) and `segments_completed` (0..segments).
ROW_BINARY_OPTIONAL_KEYS = frozenset(("error_kind", "segments", "segments_completed"))
ROW_OPTIONAL_KEYS = frozenset(("process", "probe", "accuracy_inline")) | STATE_SIDECAR_KEYS | ROW_BINARY_OPTIONAL_KEYS
ROW_REQUIRED_KEYS = ROW_KEYS - {"states_hex"}  # states_hex xor the three sidecar keys
DEFAULT_INLINE_STATES_MAX_DIMENSION = 64
ACCURACY_FUNCTION = "tools/triage/triage_common.py::row_accuracy"
ACCURACY_KEYS = frozenset(("class", "E", "E_rho", "E_up", "R", "R_rho", "R_up", "reason", "integrity"))
ACCURACY_INLINE_KEYS = ACCURACY_KEYS | {"reference_sha256", "function"}
ROW_STATUSES = frozenset(("ok", "solver_failure", "refused", "error", "process_abort", "unavailable"))
GRIDS = ("endpoint", "uniform101")
EXTERNAL_PREFIX = "ext."
STDERR_TAIL_CHARS = 4000


class TriageError(Exception):
    """Malformed input or a refused operation; the tools exit nonzero with the reasons."""

    def __init__(self, reasons: Iterable[str] | str):
        self.reasons = (reasons,) if isinstance(reasons, str) else tuple(reasons)
        super().__init__("; ".join(self.reasons[:5]))


# ----------------------------------------------------------------------------------------- loading
def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_input_bytes(path: Path) -> bytes:
    """The content of an input file; a `.gz` file is read decompressed.

    Row files outgrow the 100 MB per-file limit of the hosting service, so they are committed as
    `gzip -n` copies; every digest and parse sees the same bytes as the uncompressed file.
    """
    data = Path(path).read_bytes()
    if str(path).endswith(".gz"):
        data = gzip.decompress(data)
    return data


def sha256_file(path: Path) -> str:
    return sha256_bytes(read_input_bytes(path))


def load_json(path: Path) -> Any:
    """Strict JSON (no duplicate keys, no NaN/Infinity)."""
    try:
        return evidence.loads_strict(read_input_bytes(path))
    except evidence.ValidationError as exc:
        raise TriageError([f"{path}: {r}" for r in exc.reasons]) from None
    except OSError as exc:
        raise TriageError(f"cannot read {path}: {exc}") from None


def load_jsonl(path: Path) -> list[dict]:
    """Strict JSON Lines; blank lines are ignored, every other line must be one JSON object."""
    rows = []
    try:
        data = read_input_bytes(path)
    except (OSError, EOFError, gzip.BadGzipFile) as exc:
        raise TriageError(f"cannot read {path}: {exc}") from None
    reasons = []
    for number, line in enumerate(data.split(b"\n"), 1):
        if not line.strip():
            continue
        try:
            row = evidence.loads_strict(line)
        except evidence.ValidationError as exc:
            reasons.append(f"{path}:{number}: {exc.reasons[0]}")
            continue
        if not isinstance(row, dict):
            reasons.append(f"{path}:{number}: expected a JSON object")
            continue
        rows.append(row)
    if reasons:
        raise TriageError(reasons)
    return rows


def dumps(doc: Any, indent: int | None = 1) -> str:
    """Deterministic JSON text (sorted keys, no NaN)."""
    return json.dumps(doc, indent=indent, sort_keys=True, allow_nan=False)


def dump_line(doc: Any) -> str:
    return json.dumps(doc, sort_keys=True, allow_nan=False, separators=(",", ":"))


def refuse_existing(path: Path, force: bool) -> None:
    """Immutable outputs (as tools/speed_profile.py): refuse to overwrite unless --force."""
    if Path(path).exists() and not force:
        raise SystemExit(f"immutable output exists: {path} (pass --force to overwrite)")


def write_text(path: Path, text: str, force: bool) -> None:
    refuse_existing(path, force)
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_text(text)


# --------------------------------------------------------------------------------------- hex codec
def hex64(value: float) -> str:
    return struct.pack(">d", float(value)).hex()


def unhex64(text: str) -> float:
    """Decode one binary64 hex string, allowing non-finite values (callers decide what they mean)."""
    if not isinstance(text, str) or not evidence.HEX64.match(text):
        raise TriageError(f"not a 16-digit lowercase binary64 hex string: {text!r}")
    return struct.unpack(">d", bytes.fromhex(text))[0]


def hex_matrix(rows: Any) -> list[list[float]]:
    if not isinstance(rows, list):
        raise TriageError(f"expected a list of hex vectors, got {type(rows).__name__}")
    out = []
    for row in rows:
        if not isinstance(row, list):
            raise TriageError(f"expected a hex vector, got {type(row).__name__}")
        out.append([unhex64(v) for v in row])
    return out


# ------------------------------------------------------------------------------------------ cells
def is_external(arm: str) -> bool:
    return arm.startswith(EXTERNAL_PREFIX)


def applicable(arm: dict, problem: dict) -> tuple[bool, str | None]:
    """Contract section 1: a problem is applicable to an arm iff every `requires` field is satisfied.

    jacobian: `analytic` needs problem.jacobian == "analytic"; `jvp` needs problem.jvp true (an analytic
    Jacobian problem with `jvp` false is not applicable); `any` always. structure: null always, otherwise equal
    to problem.structure. mass: False means identity mass only (a mass-matrix problem is not applicable); True means a mass
    matrix is accepted (any problem). fixed_dimension: equal to problem.dimension when set (a list means
    any listed dimension). An `applicable_problems` list printed by the binary is authoritative.
    """
    listed = arm.get("applicable_problems")
    if listed is not None and problem.get("id") is not None:
        # The binary's own applicability list is authoritative when present.
        if problem.get("id") in listed:
            return True, None
        return False, "not in the arm's applicable_problems"
    req = arm.get("requires") or {}
    jac = req.get("jacobian", "any")
    if jac == "analytic" and problem.get("jacobian") != "analytic":
        return False, f"requires analytic jacobian, problem has {problem.get('jacobian')!r}"
    if jac == "jvp" and problem.get("jvp") is not True:
        return False, "requires jvp, problem has none"
    if jac not in ("analytic", "jvp", "any"):
        return False, f"unknown requires.jacobian {jac!r}"
    structure = req.get("structure")
    if structure is not None and problem.get("structure") != structure:
        return False, f"requires structure {structure!r}, problem has {problem.get('structure')!r}"
    mass = req.get("mass")
    # The binary's semantics: mass true = the arm accepts a mass matrix, false = identity mass only.
    if mass is False and problem.get("mass"):
        return False, "arm supports identity mass only, problem has a mass matrix"
    fixed = req.get("fixed_dimension")
    if isinstance(fixed, list):
        if problem.get("dimension") not in fixed:
            return False, f"requires dimension in {fixed}, problem has {problem.get('dimension')}"
    elif fixed is not None and problem.get("dimension") != fixed:
        return False, f"requires dimension {fixed}, problem has {problem.get('dimension')}"
    return True, None


def grid_times(t_span: list[float], grid: str) -> list[float]:
    """Requested output times: endpoint [t0, tf]; uniform101 t0 + (tf - t0) * i / 100 with both ends exact.

    The binary's own times are authoritative for rows; triage_check compares them bit for bit with the
    reference times and reports a mismatch instead of interpolating.
    """
    t0, tf = float(t_span[0]), float(t_span[1])
    if grid == "endpoint":
        return [t0, tf]
    if grid == "uniform101":
        times = [t0 + (tf - t0) * float(i) / 100.0 for i in range(101)]
        times[0], times[-1] = t0, tf
        return times
    raise TriageError(f"unknown grid {grid!r}")


def h0_base(problem: dict) -> float:
    """Contract: h0_base = 1e-6 * (tf - t0) unless the problem declares its own `h0_base`."""
    if problem.get("h0_base") is not None:
        return float(problem["h0_base"])
    t0, tf = problem["t_span"]
    return 1.0e-6 * (float(tf) - float(t0))


def cell_key(row: dict) -> tuple:
    """(campaign, arm, problem, rtol, grid, h0_scale): the identity of one run cell."""
    return (row.get("campaign"), row.get("arm"), row.get("problem"), row.get("rtol"), row.get("grid"),
            row.get("h0_scale"))


def fmt_float(value: float) -> str:
    return repr(float(value))


def corpus_problems(corpus: dict) -> dict[str, dict]:
    out = {}
    for p in corpus.get("problems", []):
        if p["id"] in out:
            raise TriageError(f"CORPUS: duplicate problem id {p['id']!r}")
        out[p["id"]] = p
    return out


def merged_problem(corpus_problem: dict, list_problem: dict | None) -> dict:
    """CORPUS fields win for classification features; the list adds jvp/breakpoints/h0_base."""
    merged = dict(list_problem or {})
    merged.update(corpus_problem)
    if list_problem:
        for key in ("jvp", "breakpoints", "h0_base", "source"):
            if key in list_problem and key not in corpus_problem:
                merged[key] = list_problem[key]
    return merged


# -------------------------------------------------------------------------------------- provenance
def git_provenance(repo: Path = REPO) -> dict:
    """Commit and tree cleanliness for the environment of the binary (it never shells out to git)."""
    try:
        commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=repo, capture_output=True, text=True,
                                check=True).stdout.strip()
        status = subprocess.run(["git", "status", "--porcelain", "--untracked-files=no"], cwd=repo,
                                capture_output=True, text=True, check=True).stdout
        return {"commit": commit, "tree_clean": status.strip() == ""}
    except (OSError, subprocess.CalledProcessError):
        return {"commit": None, "tree_clean": None}


def provenance_env(prov: dict) -> dict:
    env = dict(os.environ)
    for var in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "RAYON_NUM_THREADS"):
        env.setdefault(var, "1")
    if prov.get("commit"):
        env["VIGILODE_SOURCE_COMMIT"] = prov["commit"]
    if prov.get("tree_clean") is not None:
        env["VIGILODE_TREE_CLEAN"] = "true" if prov["tree_clean"] else "false"
    return env


def describe_exit(code: int) -> str:
    if code < 0:
        import signal
        try:
            return f"killed by signal {-code} ({signal.Signals(-code).name})"
        except ValueError:
            return f"killed by signal {-code}"
    return f"exit status {code}"


def tail(text: str | bytes | None, limit: int = STDERR_TAIL_CHARS) -> str:
    if text is None:
        return ""
    if isinstance(text, bytes):
        text = text.decode("utf-8", "replace")
    return text[-limit:]


def list_document(list_path: Path | None, binary: str | None) -> tuple[dict, str]:
    """`rodas5p triage-list` JSON from a file or by running the binary; returns (doc, sha256 of the bytes)."""
    if list_path is not None:
        data = Path(list_path).read_bytes()
    elif binary is not None:
        proc = subprocess.run([binary, "triage-list"], capture_output=True)
        if proc.returncode != 0:
            raise TriageError(f"{binary} triage-list: {describe_exit(proc.returncode)}: {tail(proc.stderr, 400)}")
        data = proc.stdout
    else:
        raise TriageError("need --list PATH or --binary PATH for the triage-list document")
    try:
        doc = evidence.loads_strict(data)
    except evidence.ValidationError as exc:
        raise TriageError([f"triage-list: {r}" for r in exc.reasons]) from None
    if not isinstance(doc, dict) or doc.get("schema") != LIST_SCHEMA:
        raise TriageError(f"triage-list: schema is not {LIST_SCHEMA!r}")
    return doc, sha256_bytes(data)


def parse_json_output(stdout: str) -> dict | None:
    """The JSON object a tool printed: the whole stdout, else its last line starting with `{`."""
    text = (stdout or "").strip()
    if not text:
        return None
    candidates = [text] + [line for line in reversed(text.splitlines()) if line.startswith("{")]
    for candidate in candidates:
        try:
            doc = evidence.loads_strict(candidate.encode())
        except evidence.ValidationError:
            continue
        if isinstance(doc, dict):
            return doc
    return None


def finite(value: Any) -> bool:
    return type(value) in (int, float) and not isinstance(value, bool) and math.isfinite(value)


# ------------------------------------------------------------------------------------ cell planning
PLAN_KINDS = ("runs", "profile", "alloc")


def plan_cells(list_doc: dict, corpus: dict, kind: str = "runs", arms: Iterable[str] | None = None,
               problems: Iterable[str] | None = None, tiers: Iterable[str] | None = None,
               rtols: Iterable[float] | None = None, grids: Iterable[str] | None = None,
               h0_scales: Iterable[float] | None = None) -> tuple[list[dict], list[str]]:
    """Cells of the wired internal arms x applicable CORPUS problems x sweep.

    kind `runs`: CORPUS sweep rtols x grids x h0_scales. kind `profile`/`alloc`: CORPUS profile_subset rtols,
    problems with dimension <= profile_subset.max_dimension, profile_subset.grids (default: the sweep grids)
    and profile_subset.h0_scales (default: 1.0 only; the primary cost of a group is the h0 = 1 cell).
    Filters narrow the plan; they never add cells outside it.
    Returns (cells sorted by (arm, problem, rtol, grid, h0_scale), notes on skipped corpus problems).
    """
    if kind not in PLAN_KINDS:
        raise TriageError(f"unknown plan kind {kind!r}")
    sweep = corpus.get("sweep") or {}
    if kind == "runs":
        plan_rtols = [float(r) for r in sweep.get("rtols", [])]
        plan_h0 = [float(h) for h in sweep.get("h0_scales", [])]
        max_dim = None
        plan_grids = list(sweep.get("grids", []))
    else:
        subset = corpus.get("profile_subset") or {}
        plan_rtols = [float(r) for r in subset.get("rtols", [])]
        plan_h0 = [float(h) for h in subset.get("h0_scales", [1.0])]
        max_dim = subset.get("max_dimension")
        plan_grids = list(subset.get("grids", sweep.get("grids", [])))
    sel = lambda values, chosen: [v for v in values if chosen is None or v in set(chosen)]  # noqa: E731
    plan_rtols = sel(plan_rtols, None if rtols is None else [float(r) for r in rtols])
    plan_grids = sel(plan_grids, grids)
    plan_h0 = sel(plan_h0, None if h0_scales is None else [float(h) for h in h0_scales])
    list_arms = {a["id"]: a for a in list_doc.get("arms", [])}
    list_problems = {p["id"]: p for p in list_doc.get("problems", [])}
    notes = []
    chosen_arms = sorted(a for a, spec in list_arms.items() if spec.get("wired") and not is_external(a)
                         and (arms is None or a in set(arms)))
    cells = []
    for pid, cp in sorted(corpus_problems(corpus).items()):
        if problems is not None and pid not in set(problems):
            continue
        if tiers is not None and cp.get("tier") not in set(tiers):
            continue
        if pid not in list_problems:
            notes.append(f"problem {pid}: not in triage-list (no cells)")
            continue
        problem = merged_problem(cp, list_problems[pid])
        if max_dim is not None and problem.get("dimension", 0) > max_dim:
            continue
        for arm in chosen_arms:
            ok, _ = applicable(list_arms[arm], problem)
            if not ok:
                continue
            for rtol in plan_rtols:
                for grid in plan_grids:
                    for h0 in plan_h0:
                        cells.append({"arm": arm, "problem": pid, "rtol": rtol, "grid": grid, "h0_scale": h0,
                                      "tier": problem.get("tier"), "dimension": problem.get("dimension")})
    cells.sort(key=lambda c: (c["arm"], c["problem"], c["rtol"], c["grid"], c["h0_scale"]))
    return cells, notes


def cell_args(cell: dict) -> list[str]:
    return ["--arm", cell["arm"], "--problem", cell["problem"], "--rtol", fmt_float(cell["rtol"]),
            "--grid", cell["grid"], "--h0-scale", fmt_float(cell["h0_scale"])]


def add_selection_arguments(parser) -> None:
    """The cell filters shared by runner, profile, alloc and external tools."""
    parser.add_argument("--tier", action="append", help="keep problems of this tier (repeatable)")
    parser.add_argument("--arm", action="append", help="keep this arm (repeatable)")
    parser.add_argument("--problem", action="append", help="keep this problem (repeatable)")
    parser.add_argument("--rtol", action="append", type=float, help="keep this rtol (repeatable)")
    parser.add_argument("--grid", action="append", choices=GRIDS, help="keep this grid (repeatable)")
    parser.add_argument("--h0-scale", action="append", type=float, help="keep this h0 scale (repeatable)")


def selection(args) -> dict:
    return {"arms": args.arm, "problems": args.problem, "tiers": args.tier, "rtols": args.rtol,
            "grids": args.grid, "h0_scales": args.h0_scale}


class OrderedAppender:
    """Append JSONL rows in plan order while parallel workers finish out of order (resumable and deterministic:
    a row is flushed as soon as every earlier planned cell has been flushed)."""

    def __init__(self, path: Path, total: int):
        self.handle = open(path, "a")
        self.pending: dict[int, dict] = {}
        self.next = 0
        self.total = total

    def add(self, index: int, row: dict) -> None:
        self.pending[index] = row
        while self.next in self.pending:
            self.handle.write(dump_line(self.pending.pop(self.next)) + "\n")
            self.handle.flush()
            self.next += 1

    def close(self) -> None:
        for index in sorted(self.pending):
            self.handle.write(dump_line(self.pending[index]) + "\n")
        self.pending.clear()
        self.handle.close()


# ========================================================================= row accuracy (shared)
# The one implementation of contract row accuracy: triage_check.py recomputes it from states_hex or a state
# sidecar, and triage_runner.py / triage_external.py store it as `accuracy_inline` when they move the states
# of a large row to a sidecar. Same inputs give bit-identical results in both places.
def decode_vector(values: list) -> list[float] | None:
    try:
        return list(struct.unpack(f">{len(values)}d", bytes.fromhex("".join(values))))
    except (ValueError, TypeError, struct.error):
        return None


def wrms_max(rows_a: list[list[float]], weights: list[list[float]]) -> float:
    worst = 0.0
    for a, w in zip(rows_a, weights):
        value = math.sqrt(math.fsum((x * y) ** 2 for x, y in zip(a, w)) / len(a))
        if not value <= worst:
            worst = value  # NaN or larger
    return worst


def reference_entry_sha256(entry: dict | None) -> str | None:
    """Digest of one REFERENCES entry (canonical JSON): binds an inline accuracy to the reference it used."""
    return None if entry is None else sha256_bytes(dump_line(entry).encode())


class Reference:
    def __init__(self, entry: dict | None, problem: dict, grid: str):
        self.sha256 = reference_entry_sha256(entry)
        self.status = "missing" if entry is None else entry.get("status")
        self.reason = "reference_missing" if entry is None else entry.get("reason")
        self.times = None
        self.states = None
        self.rho = None
        self.e_rho = None
        if entry is None or self.status != "ok":
            if entry is not None and not self.reason:
                self.reason = f"reference status {self.status!r}"
            return
        dim = problem.get("dimension")
        try:
            self.times = [float(t) for t in entry["output_times"]]
            self.states = [decode_vector(r) for r in entry["states_hex"]]
            self.rho = [decode_vector(r) for r in entry["rho_hex"]]
        except (KeyError, TypeError, ValueError):
            self.status, self.reason = "invalid", "reference entry malformed"
            return
        bad = (len(self.states) != len(self.times) or len(self.rho) != len(self.times)
               or any(v is None or len(v) != dim for v in self.states + self.rho)
               or not all(math.isfinite(x) for v in self.states + self.rho for x in v)
               or any(x < 0 for v in self.rho for x in v))
        if bad:
            self.status, self.reason = "invalid", "reference states/rho malformed, non-finite or wrong dimension"
            return
        atol_scale = float(problem["atol_scale"])
        self.w_problem = [[1.0 / (atol_scale + abs(r)) for r in row] for row in self.states]
        self.e_rho = wrms_max(self.rho, self.w_problem)
        self._requested: dict[tuple, tuple] = {}

    @property
    def ok(self) -> bool:
        return self.status == "ok"

    def requested(self, atol: float, rtol: float) -> tuple[list, float]:
        key = (atol, rtol)
        if key not in self._requested:
            w = [[1.0 / (atol + rtol * abs(r)) for r in row] for row in self.states]
            self._requested[key] = (w, wrms_max(self.rho, w))
        return self._requested[key]


def row_states(row: dict) -> list | None:
    """The decoded states_hex of a row (None entries for malformed vectors), or None without states_hex."""
    if "states_hex" not in row:
        return None
    return [decode_vector(v) if isinstance(v, list) else None for v in row["states_hex"]]


def row_accuracy(row: dict, problem: dict, ref: Reference, states: list | None = None) -> dict:
    """{class: computed|failed|unknown, E, E_rho, E_up, R, R_rho, R_up, reason, integrity}.

    `states` (one float list per output time) defaults to the row's decoded states_hex.
    """
    out = {"class": "unknown", "E": None, "E_rho": None, "E_up": None, "R": None, "R_rho": None, "R_up": None,
           "reason": None, "integrity": None}
    if row["status"] != "ok":
        out.update({"class": "failed", "reason": f"status {row['status']}"})
        if row["status"] == "process_abort":
            # A crash or signal is an integrity failure; a runner timeout only means the cell did not finish
            # inside its budget, which makes the cell inadmissible without implicating correctness.
            if ((row.get("process") or {}).get("kind")) == "timeout":
                out["reason"] = "status process_abort (timeout: cell budget exhausted)"
            else:
                out["integrity"] = "process_abort"
        return out
    dim = problem.get("dimension")
    if states is None:
        states = row_states(row) or []
    if any(s is None or len(s) != dim for s in states) or not states:
        out.update({"class": "failed", "reason": "ok row with malformed states", "integrity": "malformed_ok_row"})
        return out
    if not all(math.isfinite(x) for s in states for x in s):
        out.update({"class": "failed", "reason": "non-finite state", "integrity": "non_finite_state"})
        return out
    if not ref.ok:
        out["reason"] = f"reference: {ref.reason}"
        return out
    times = row["output_times"]
    if len(times) != len(ref.times) or any(float(a) != b for a, b in zip(times, ref.times)):
        out["reason"] = "output_times differ from the reference times"
        return out
    diff = [[y - r for y, r in zip(ys, rs)] for ys, rs in zip(states, ref.states)]
    e = wrms_max(diff, ref.w_problem)
    w_req, r_rho = ref.requested(float(row["atol"]), float(row["rtol"]))
    r = wrms_max(diff, w_req)
    if not (math.isfinite(e) and math.isfinite(r)):
        out.update({"class": "failed", "reason": "non-finite error norm"})
        return out
    out.update({"class": "computed", "E": e, "E_rho": ref.e_rho, "E_up": e + ref.e_rho, "R": r, "R_rho": r_rho,
                "R_up": r + r_rho})
    return out


def check_accuracy_record(acc: Any) -> str | None:
    """Why a stored accuracy record (accuracy_inline) is malformed, or None."""
    if not isinstance(acc, dict) or set(acc) != ACCURACY_INLINE_KEYS:
        return "accuracy_inline key set differs"
    norms = ("E", "E_rho", "E_up", "R", "R_rho", "R_up")
    if acc["class"] == "computed":
        if not all(type(acc[k]) is float and math.isfinite(acc[k]) and acc[k] >= 0 for k in norms):
            return "accuracy_inline norms are not finite nonnegative floats"
        if acc["E_up"] != acc["E"] + acc["E_rho"] or acc["R_up"] != acc["R"] + acc["R_rho"]:
            return "accuracy_inline E_up/R_up are not the sums"
    elif acc["class"] in ("failed", "unknown"):
        if any(acc[k] is not None for k in norms) or not isinstance(acc["reason"], str):
            return "accuracy_inline of a failed/unknown row carries norms or no reason"
    else:
        return f"accuracy_inline class {acc['class']!r}"
    return None


# ============================================================================== state sidecars
def states_bytes(states: list[list[float]]) -> bytes:
    """Little-endian float64, row-major [time][component]."""
    flat = [x for s in states for x in s]
    return struct.pack(f"<{len(flat)}d", *flat)


def write_states_sidecar(directory: Path, states: list[list[float]]) -> tuple[str, str]:
    """Write `<sha256>.f64le.gz` (gzip without name or mtime, like `gzip -n`; sha256 over the uncompressed
    bytes) into directory; content addressed, so an existing file is kept. Returns (sha256, file name)."""
    data = states_bytes(states)
    digest = sha256_bytes(data)
    name = f"{digest}.f64le.gz"
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    target = directory / name
    if not target.exists():
        tmp = directory / f".{name}.{os.getpid()}.tmp"
        tmp.write_bytes(gzip.compress(data, compresslevel=6, mtime=0))
        os.replace(tmp, target)
    return digest, name


def load_states_sidecar(directories: Iterable[Path], row: dict) -> tuple[list | None, str, str | None]:
    """(states, source, reason): source is `sidecar` (loaded, sha256 and shape verified), `sidecar_missing` or
    `sidecar_refused` (sha256, shape or gzip mismatch; the file is never used)."""
    name, digest, shape = row.get("states_file"), row.get("states_sha256"), row.get("states_shape")
    if not (isinstance(name, str) and "/" not in name and isinstance(digest, str)
            and isinstance(shape, list) and len(shape) == 2 and all(type(x) is int and x >= 0 for x in shape)):
        return None, "sidecar_refused", "malformed states_file/states_sha256/states_shape"
    for directory in directories:
        path = Path(directory) / name
        if not path.is_file():
            continue
        try:
            data = gzip.decompress(path.read_bytes())
        except (OSError, EOFError, gzip.BadGzipFile) as exc:
            return None, "sidecar_refused", f"{name}: unreadable gzip: {exc}"
        if sha256_bytes(data) != digest:
            return None, "sidecar_refused", f"{name}: sha256 of the uncompressed bytes != states_sha256"
        times, dim = shape
        if len(data) != 8 * times * dim:
            return None, "sidecar_refused", f"{name}: {len(data)} bytes != 8 x {times} x {dim}"
        flat = struct.unpack(f"<{times * dim}d", data)
        return [list(flat[i * dim:(i + 1) * dim]) for i in range(times)], "sidecar", None
    return None, "sidecar_missing", f"{name}: not found in the states directories"


def load_references(paths: Iterable[Path]) -> dict[tuple, dict]:
    """(problem, grid) -> REFERENCES entry over one or more REFERENCES files (.gz allowed); duplicates refused."""
    out: dict[tuple, dict] = {}
    for path in paths:
        doc = load_json(path)
        if not isinstance(doc, dict) or doc.get("schema") != REFERENCES_SCHEMA:
            raise TriageError(f"{path}: schema is not {REFERENCES_SCHEMA!r}")
        for entry in doc.get("references") or []:
            key = (entry.get("problem"), entry.get("grid"))
            if key in out:
                raise TriageError(f"{path}: duplicate reference {key!r}")
            out[key] = entry
    return out


class StateExternalizer:
    """Moves the states of rows with dimension > max_dimension to sidecars and stores `accuracy_inline`
    (shared row_accuracy against the matching reference). Without a states directory large rows are kept
    inline and one warning is printed."""

    def __init__(self, directory: Path | None, max_dimension: int, references: dict[tuple, dict],
                 problems: dict[str, dict]):
        self.directory = Path(directory) if directory is not None else None
        self.max_dimension = max_dimension
        self.references = references
        self.problems = problems
        self._refs: dict[tuple, Reference] = {}
        self.warned = False
        self.moved = 0

    def reference(self, problem_id: str, grid: str) -> Reference:
        key = (problem_id, grid)
        if key not in self._refs:
            self._refs[key] = Reference(self.references.get(key), self.problems.get(problem_id) or {}, grid)
        return self._refs[key]

    def __call__(self, row: dict) -> dict:
        states_hex = row.get("states_hex")
        dimension = row.get("dimension")
        if not isinstance(dimension, int) or isinstance(dimension, bool):
            dimension = len(states_hex[0]) if states_hex else 0
        if not states_hex or dimension <= self.max_dimension:
            return row
        if self.directory is None:
            if not self.warned:
                print(f"warning: rows with dimension > {self.max_dimension} keep states_hex inline "
                      f"(no --states-dir)", file=sys.stderr)
                self.warned = True
            return row
        states = row_states(row)
        if any(s is None for s in states) or len({len(s) for s in states}) != 1:
            return row  # malformed states stay inline; the checker reports them
        problem = self.problems.get(row.get("problem"))
        if problem is None:
            accuracy = {"class": "unknown", "E": None, "E_rho": None, "E_up": None, "R": None, "R_rho": None,
                        "R_up": None, "reason": "problem not in CORPUS", "integrity": None}
            ref_sha = None
        else:
            ref = self.reference(row["problem"], row["grid"])
            accuracy = row_accuracy(row, problem, ref, states)
            ref_sha = ref.sha256
        digest, name = write_states_sidecar(self.directory, states)
        out = {k: v for k, v in row.items() if k != "states_hex"}
        out.update(states_sha256=digest, states_shape=[len(states), len(states[0])], states_file=name,
                   accuracy_inline=dict(accuracy, reference_sha256=ref_sha, function=ACCURACY_FUNCTION))
        self.moved += 1
        return out


def add_state_arguments(parser) -> None:
    parser.add_argument("--references", type=Path, action="append", default=[],
                        help="REFERENCES.json (.gz allowed; repeatable) for accuracy_inline of sidecar rows")
    parser.add_argument("--states-dir", type=Path, help="directory for <sha256>.f64le.gz state sidecars")
    parser.add_argument("--inline-states-max-dimension", type=int, default=DEFAULT_INLINE_STATES_MAX_DIMENSION,
                        help="rows with a larger dimension move their states to --states-dir (default 64)")

