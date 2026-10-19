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
ROW_OPTIONAL_KEYS = frozenset(("process", "probe"))
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
    and profile_subset.h0_scales (default: 1.0 only; the primary cost of a group is the h0 = 1 cell). Filters narrow the plan; they never add cells outside it.
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
