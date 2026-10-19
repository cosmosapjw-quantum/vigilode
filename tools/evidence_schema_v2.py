#!/usr/bin/env python3
"""Fail-closed evidence validator for the ALG04/ALG05/ALG06 research checkers (re-audit node AS03, finding F104).

Version: ``vigilode-evidence-schema-v2``. The checkers ``tools/alg04_coupled_target_v2_check.py``,
``tools/alg05_controller_v2_check.py`` and ``tools/alg06_guard_v2_check.py`` call the validator for their kind
before any scientific gate. A validator rejection is the verdict ``INVALID`` (malformed or unbound evidence); the
registered gates then decide ``PASS`` or ``FAIL`` on valid evidence only. Gate logic and thresholds are unchanged.

Rules (every rule raises :class:`ValidationError`; nothing here is an ``assert``, so ``python -O`` keeps them):

1. Strict JSON: UTF-8, no duplicate object keys, no ``NaN``/``Infinity`` constants, no JSON number that overflows
   to a non-finite float.
2. Immutable inputs: the base/reference files are bound to their registered SHA-256 (``REGISTERED_SHA256``): ALG01
   BASE for ALG04, ALG03 BASE for ALG06, ALG05 BASE and NATIVE.json for ALG05, and the optional reproduction
   inputs (ALG01/ALG03 RUNS) when they are read.
3. Exact key sets: every document, row, arm map, run record, counter map, stage-statistics map, charge row and
   reference has exactly the registered keys (ladder records: by ``ok``; references: by exact/dense kind).
4. Exact row and arm sets: raw row keys are unique *in the raw list* (before any dictionary can absorb a duplicate);
   the RUNS cell/rung set equals the base's (ALG06: plus exactly the 14 registered D5 cells; ALG05: every arm has
   exactly the base's 276 cells and there are exactly 828 rows); the arm names are exactly the registered arms.
5. Vectors: every state, twin, reference and output-time vector is nonempty; state, twin and reference vectors of
   a cell all have the cell's dimension (ALG05: the NATIVE reference length of the problem).
6. Numbers: hex-encoded binary64 strings are exactly 16 lowercase hex digits and decode to finite values; JSON
   numbers are finite with the registered type (``bool`` is never accepted as ``int``, ``int`` never as ``float``);
   counts are nonnegative; tolerances are positive.
7. Binding before science: ALG04 twins and LU ladder references equal the base's bit for bit; ALG06 twins equal the
   base's; a dense reference equals its own recorded 1e-13 run (and its ``y_1e-12`` the 1e-12 run); ALG05 recorded
   endpoint errors equal the error recomputed from the state bits and the NATIVE reference. (Registered identity
   gate items, e.g. legacy/rbig/I against BASE, stay scientific gate items and are not duplicated here.)

:func:`require_metric_pair` is the shared guard for the checkers' endpoint metrics (equal nonempty lengths, finite
floats), replacing the former length ``assert`` in ALG05.
"""

from __future__ import annotations

import hashlib
import json
import math
import re
import struct
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Iterable, Mapping

VERSION = "vigilode-evidence-schema-v2"
INVALID = "INVALID"
VALID = "VALID"
INVALID_EXIT_CODE = 2
MAX_REASONS = 200

# Immutable inputs (the committed bytes; ledger rows L-0094..L-0098 record the same digests).
REGISTERED_SHA256 = {
    "alg01_base": "ea18d0e41bdf3d4785c7aee1003d72186143358f11853d1221036816f66595aa",
    "alg01_runs": "605b5d4d482b03d4428c7f0986a6e39525fc90697e83d064bb6bd78f81edd041",
    "alg03_base": "bb920b759ab787e1b40552e13643e2994a1640f9d2874f98b4fc9f9c1e98cc7b",
    "alg03_runs": "a4126e75899cb1d53bc84608c949505a8ae01ad659bc40595ebeb6e7262041a9",
    "alg05_base": "16aee528f57c58a512c441118ed9a01dde54c0080a904102f58d263f8a73bb44",
    "native": "5064b638b411fb46089b336a351325a4dbc12acfeacdff2edb226aa80e8fd1ec",
}

HEX64 = re.compile(r"^[0-9a-f]{16}$")

COUNTERS_FULL = frozenset((
    "accepted_steps", "block_linear_iterations", "block_linear_solves", "block_matvecs",
    "block_preconditioner_apps", "diagnostic_matvecs", "direct_factorizations", "direct_solve_calls",
    "fallback_steps", "fast_accepts", "fast_attempts", "forced_stage_solves", "ft_calls", "harmonic_ritz_solves",
    "jacobian_builds", "jvp_calls", "jvp_vectors", "linear_iterations", "linear_matvec_vectors", "linear_matvecs",
    "linear_solve_failures", "linear_solves", "local_error_failures", "mass_matvecs", "nonfinite_step_failures",
    "nonlinear_failures", "nonlinear_iterations", "nonlinear_jacobian_evaluations",
    "nonlinear_residual_evaluations", "nonlinear_solve_failures", "nonlinear_solves",
    "orthogonalization_inner_products", "orthogonalization_vector_updates", "phi_actions", "phi_dense_oracle_calls",
    "phi_krylov_vectors", "phi_projected_exponentials", "phi_restarts", "preconditioner_apps",
    "preconditioner_vectors", "recycle_cross_operator_refreshes", "recycle_dropped_vectors",
    "recycle_projection_calls", "recycle_refresh_matvecs", "recycle_same_operator_uses", "recycle_updates",
    "recycle_vectors_selected", "rejected_steps", "rhs_batch_calls", "rhs_calls", "rhs_evaluations"))
COUNTERS_STAGED = COUNTERS_FULL - {"preconditioner_vectors"}
COUNTERS_DENSE = COUNTERS_FULL - {"preconditioner_vectors", "linear_matvec_vectors"}

STAGE_STATISTICS_FLOAT = frozenset(("charge_max", "charge_sum", "nu_max"))
STAGE_STATISTICS_INT = frozenset((
    "budget_exhausted", "charge_crosses_one", "charged_attempts", "columns", "confirmations", "converged", "cycles",
    "exhaustion_solves", "failed", "failed_breakdown", "failed_budget", "failed_confirmations", "failed_guard",
    "failed_nonfinite", "fallback_accepted", "fallback_after_guard", "floor_accepted", "guard_accepted_false",
    "guard_accepted_true", "guard_contraction", "guard_false", "guard_overrun", "guard_true", "max_columns",
    "nu_evaluations", "nu_flops", "nu_tightened", "roundoff_floor_binds", "solves", "stall_accepted",
    "true_residuals"))

ADAPTIVE_PLAIN_KEYS = frozenset(("accepted", "attempts", "counters", "internal_steps", "message", "ok",
                                 "output_clipped_steps", "rejected", "state_reuses", "success", "t", "y_last"))
ADAPTIVE_STAGED_KEYS = ADAPTIVE_PLAIN_KEYS | {"charges", "stage_statistics"}
DENSE_KEYS = frozenset(("accepted", "attempts", "counters", "message", "ok", "rejected", "success", "t", "y_last"))
LADDER_OK_KEYS = frozenset(("counters", "max_step_error_estimate", "ok", "rel_max_norm_error", "steps", "y_last"))
LADDER_FAILED_KEYS = frozenset(("counters", "error", "failed_step", "max_step_error_estimate", "ok", "steps",
                                "y_last"))
LU_KEYS = frozenset(("max_step_error_estimate", "ok", "rel_max_norm_error", "steps", "y_last"))
ARM_OPTION_KEYS = frozenset(("charge_fallback_residual", "classify_accepted_guard_aborts", "classify_guard_aborts",
                             "effective_cycle_overrun", "floor_at_confirmations", "policy", "production_fallback",
                             "stagnation_guard"))
CHARGE_COLUMNS = ["t", "h", "error", "charge", "stages", "charged"]
REFERENCE_KINDS_EXACT = frozenset(("exact", "exact-expm-pade13"))
REFERENCE_KINDS_DENSE = frozenset(("dense-fast-rtol-1e-13",))

# ALG04 (inputs: ALG01 BASE, ALG04 RUNS).
ALG04_ARMS = ("legacy", "dup_fix", "proj_l2", "l2_coupled", "coupled_guarded", "coupled_guarded2")
ALG04_PLAIN_ARMS = frozenset(("legacy", "dup_fix"))
ALG04_GROUPS = frozenset(("C1", "C2", "C3", "C5"))
ALG04_LADDERS = frozenset(("diagpr128", "semilin128", "semilin64"))

# ALG06 (inputs: ALG03 BASE, ALG06 RUNS).
ALG06_ARMS = ("b2", "b3a", "b3b", "b3", "rbig")
ALG06_D5_ARM = "coupled_guarded2_200"
ALG06_BASE_GROUPS = frozenset(("D1", "D2", "D3", "D4"))
ALG06_D5_CELLS = frozenset((
    ("robertson", 1e-06), ("robertson", 1e-08), ("van-der-pol-mu1000", 1e-06), ("van-der-pol-mu1000", 1e-08),
    ("hires", 1e-06), ("hires", 1e-08), ("brusselator-1d-50", 1e-06), ("brusselator-1d-50", 1e-08),
    ("prothero-robinson-forced", 1e-06), ("prothero-robinson-forced", 1e-08), ("quadratic-4", 1e-06),
    ("quadratic-4", 1e-08), ("brusselator-1d-160", 1e-06), ("brusselator-1d-160", 1e-08)))

# ALG05 (inputs: ALG05 BASE, ALG05 RUNS, NATIVE.json references).
ALG05_ARMS = ("I", "PREDcap", "PREDcap2")
ALG05_REFERENCE_ARM = "I"
ALG05_PROBLEMS = ("van-der-pol-mu1000", "hires", "robertson", "brusselator-1d-50")
ALG05_SEEDS = (3.0e-6, 3.0e-5, 3.0e-3)
ALG05_CELLS_PER_ARM = 276
ALG05_ERROR_FLOOR = 1.0e-10
ALG05_ROW_KEYS = frozenset((
    "accepted", "arm", "atol", "attempts", "counters", "error", "final_state", "internal_steps", "jacobian_builds",
    "jacobian_reuses", "linear_solve_failures", "lu_factorizations", "message", "nonfinite_step_failures", "ok",
    "output_clipped_steps", "problem", "rejected", "rhs_evaluations", "rtol", "seed", "success", "t_last"))
ALG05_ROW_INTS = ("accepted", "attempts", "internal_steps", "jacobian_builds", "jacobian_reuses",
                  "linear_solve_failures", "lu_factorizations", "nonfinite_step_failures", "output_clipped_steps",
                  "rejected", "rhs_evaluations")
ALG05_HEADER = {"driver": "integrate_rodas5p_fast_observed",
                "error_metric": "max_i |y_i - r_i| / max(|r_i|, 1e-10) against NATIVE.json references",
                "max_attempts": 1000000}
NATIVE_REFERENCE_KEYS = frozenset(("method", "final_state", "uncertainty", "lsoda_rtol_1e-12_difference", "seconds"))


class ValidationError(Exception):
    """Malformed or unbound evidence. Checkers report it as the verdict INVALID (never PASS or FAIL)."""

    def __init__(self, reasons: Iterable[str]):
        self.reasons = tuple(reasons) or ("unspecified evidence violation",)
        super().__init__(f"{len(self.reasons)} evidence violation(s); first: {self.reasons[0]}")


@dataclass(frozen=True)
class Evidence:
    """Validated documents: the checker evaluates exactly these parsed objects (read once)."""

    kind: str
    docs: Mapping[str, Any]
    sha256: Mapping[str, str]
    paths: Mapping[str, Path]
    status: str = field(default=VALID)


class _StrictJSONError(ValueError):
    pass


def _parse_constant(name: str):
    raise _StrictJSONError(f"non-standard JSON constant {name}")


def _parse_float(text: str) -> float:
    number = float(text)
    if not math.isfinite(number):
        raise _StrictJSONError(f"JSON number {text!r} is not a finite binary64 value")
    return number


def _unique_pairs(pairs: list[tuple[str, Any]]) -> dict:
    out: dict = {}
    for key, value in pairs:
        if key in out:
            raise _StrictJSONError(f"duplicate object key {key!r}")
        out[key] = value
    return out


def loads_strict(data: bytes) -> Any:
    """Parse JSON bytes; raise ValidationError on any non-strict construct."""
    try:
        text = data.decode("utf-8")
        return json.loads(text, object_pairs_hook=_unique_pairs, parse_constant=_parse_constant,
                          parse_float=_parse_float)
    except (UnicodeDecodeError, ValueError, RecursionError) as exc:  # JSONDecodeError is a ValueError
        raise ValidationError([f"strict JSON: {exc}"]) from None


def decode_hex64(text: Any) -> float:
    """Decode one big-endian binary64 hex string; raise ValidationError unless canonical and finite."""
    if not isinstance(text, str) or not HEX64.match(text):
        raise ValidationError([f"not a 16-digit lowercase binary64 hex string: {text!r}"])
    number = struct.unpack(">d", bytes.fromhex(text))[0]
    if not math.isfinite(number):
        raise ValidationError([f"non-finite binary64 value {text}"])
    return number


def require_metric_pair(state: Any, reference: Any) -> None:
    """Guard for an endpoint metric: nonempty, equal-length lists of finite floats (bool/int/str rejected)."""
    reasons = []
    for name, vector in (("state", state), ("reference", reference)):
        if not isinstance(vector, list) or not vector:
            reasons.append(f"metric {name} must be a nonempty list, got {type(vector).__name__} "
                           f"of length {len(vector) if isinstance(vector, list) else 'n/a'}")
            continue
        for i, v in enumerate(vector):
            if type(v) is not float or not math.isfinite(v):
                reasons.append(f"metric {name}[{i}] is not a finite float: {v!r}")
                break
    if not reasons and len(state) != len(reference):
        reasons.append(f"metric length mismatch: state {len(state)} vs reference {len(reference)}")
    if reasons:
        raise ValidationError(reasons)


# --------------------------------------------------------------------------------------------- checks
class _Checks:
    """Collects violations with their JSON paths; ``done`` raises them together."""

    def __init__(self) -> None:
        self.reasons: list[str] = []

    def fail(self, path: str, message: str) -> None:
        self.reasons.append(f"{path}: {message}")

    def done(self) -> None:
        if self.reasons:
            raise ValidationError(self.reasons)

    # scalar types -------------------------------------------------------------------------------
    def dict_(self, path: str, value: Any) -> bool:
        if not isinstance(value, dict):
            self.fail(path, f"expected object, got {type(value).__name__}")
            return False
        return True

    def list_(self, path: str, value: Any, nonempty: bool = False) -> bool:
        if not isinstance(value, list):
            self.fail(path, f"expected array, got {type(value).__name__}")
            return False
        if nonempty and not value:
            self.fail(path, "empty array")
            return False
        return True

    def keys(self, path: str, value: Any, expected: Iterable[str]) -> bool:
        if not self.dict_(path, value):
            return False
        expected = set(expected)
        missing, extra = sorted(expected - set(value)), sorted(set(value) - expected)
        if missing or extra:
            self.fail(path, f"key set differs (missing {missing}, unexpected {extra})")
            return False
        return True

    def bool_(self, path: str, value: Any) -> bool:
        if type(value) is not bool:
            self.fail(path, f"expected bool, got {type(value).__name__} {value!r}")
            return False
        return True

    def int_(self, path: str, value: Any, minimum: int | None = 0) -> bool:
        if type(value) is not int:
            self.fail(path, f"expected int, got {type(value).__name__} {value!r}")
            return False
        if minimum is not None and value < minimum:
            self.fail(path, f"expected int >= {minimum}, got {value}")
            return False
        return True

    def float_(self, path: str, value: Any, minimum: float | None = None, positive: bool = False) -> bool:
        if type(value) is not float or not math.isfinite(value):
            self.fail(path, f"expected finite float, got {type(value).__name__} {value!r}")
            return False
        if positive and not value > 0.0:
            self.fail(path, f"expected positive float, got {value!r}")
            return False
        if minimum is not None and value < minimum:
            self.fail(path, f"expected float >= {minimum}, got {value!r}")
            return False
        return True

    def str_(self, path: str, value: Any, nonempty: bool = True) -> bool:
        if not isinstance(value, str) or (nonempty and not value):
            self.fail(path, f"expected {'nonempty ' if nonempty else ''}string, got {value!r}")
            return False
        return True

    def equal(self, path: str, value: Any, expected: Any) -> bool:
        if type(value) is not type(expected) or value != expected:
            self.fail(path, f"expected {expected!r}, got {value!r}")
            return False
        return True

    # vectors ------------------------------------------------------------------------------------
    def hex_vector(self, path: str, value: Any, length: int | None = None) -> list[float] | None:
        if not self.list_(path, value, nonempty=True):
            return None
        out, bad = [], 0
        for i, item in enumerate(value):
            try:
                out.append(decode_hex64(item))
            except ValidationError as exc:
                bad += 1
                if bad <= 3:
                    self.fail(f"{path}[{i}]", exc.reasons[0])
        if bad > 3:
            self.fail(path, f"{bad} invalid components in total")
        if length is not None and len(value) != length:
            self.fail(path, f"length {len(value)} != required {length}")
            return None
        return out if not bad else None

    def float_vector(self, path: str, value: Any, length: int | None = None) -> list[float] | None:
        if not self.list_(path, value, nonempty=True):
            return None
        bad = [i for i, v in enumerate(value) if type(v) is not float or not math.isfinite(v)]
        for i in bad[:3]:
            self.fail(f"{path}[{i}]", f"expected finite float, got {value[i]!r}")
        if length is not None and len(value) != length:
            self.fail(path, f"length {len(value)} != required {length}")
            return None
        return None if bad else list(value)

    def unique_index(self, path: str, rows: list, key: Callable[[dict], tuple]) -> dict[tuple, dict]:
        """Index the raw row list; every duplicate key is a violation (no dictionary absorbs it)."""
        index: dict[tuple, dict] = {}
        first: dict[tuple, int] = {}
        for i, row in enumerate(rows):
            k = key(row)
            if k in first:
                self.fail(f"{path}[{i}]", f"duplicate raw key {k!r} (first at index {first[k]})")
                continue
            first[k] = i
            index[k] = row
        return index

    # records ------------------------------------------------------------------------------------
    def counters(self, path: str, value: Any, expected: frozenset) -> None:
        if self.keys(path, value, expected):
            for k in sorted(value):
                self.int_(f"{path}.{k}", value[k])

    def stage_statistics(self, path: str, value: Any) -> None:
        if self.keys(path, value, STAGE_STATISTICS_FLOAT | STAGE_STATISTICS_INT):
            for k in sorted(value):
                if k in STAGE_STATISTICS_FLOAT:
                    self.float_(f"{path}.{k}", value[k])
                else:
                    self.int_(f"{path}.{k}", value[k])

    def charges(self, path: str, value: Any) -> None:
        if not self.list_(path, value):
            return
        for i, row in enumerate(value):
            p = f"{path}[{i}]"
            if not self.list_(p, row) or len(row) != len(CHARGE_COLUMNS):
                if isinstance(row, list):
                    self.fail(p, f"charge row length {len(row)} != {len(CHARGE_COLUMNS)}")
                continue
            for j, column in enumerate(CHARGE_COLUMNS[:4]):
                self.float_(f"{p}.{column}", row[j])
            self.int_(f"{p}.stages", row[4])
            self.bool_(f"{p}.charged", row[5])

    def adaptive(self, path: str, rec: Any, kind: str, dimension: int | None) -> None:
        """kind: 'plain' (legacy-family), 'staged' (stage-target arms) or 'dense' (twin / dense runs)."""
        expected = {"plain": ADAPTIVE_PLAIN_KEYS, "staged": ADAPTIVE_STAGED_KEYS, "dense": DENSE_KEYS}[kind]
        if not self.keys(path, rec, expected):
            return
        self.bool_(f"{path}.ok", rec["ok"])
        self.bool_(f"{path}.success", rec["success"])
        self.str_(f"{path}.message", rec["message"], nonempty=False)
        for k in ("accepted", "attempts", "rejected", "internal_steps", "output_clipped_steps", "state_reuses"):
            if k in rec:
                self.int_(f"{path}.{k}", rec[k])
        self.hex_vector(f"{path}.t", rec["t"])
        self.hex_vector(f"{path}.y_last", rec["y_last"], dimension)
        self.counters(f"{path}.counters", rec["counters"],
                      {"plain": COUNTERS_FULL, "staged": COUNTERS_STAGED, "dense": COUNTERS_DENSE}[kind])
        if kind == "staged":
            self.stage_statistics(f"{path}.stage_statistics", rec["stage_statistics"])
            self.charges(f"{path}.charges", rec["charges"])

    def ladder(self, path: str, rec: Any, staged: bool, dimension: int | None) -> None:
        if not self.dict_(path, rec) or not self.bool_(f"{path}.ok", rec.get("ok")):
            return
        expected = LADDER_OK_KEYS if rec["ok"] else LADDER_FAILED_KEYS
        if not self.keys(path, rec, expected | ({"stage_statistics"} if staged else set())):
            return
        self.int_(f"{path}.steps", rec["steps"])
        self.float_(f"{path}.max_step_error_estimate", rec["max_step_error_estimate"])
        if rec["ok"]:
            self.float_(f"{path}.rel_max_norm_error", rec["rel_max_norm_error"], minimum=0.0)
        else:
            self.str_(f"{path}.error", rec["error"])
            self.int_(f"{path}.failed_step", rec["failed_step"])
        self.hex_vector(f"{path}.y_last", rec["y_last"], dimension)
        self.counters(f"{path}.counters", rec["counters"], COUNTERS_STAGED if staged else COUNTERS_FULL)
        if staged:
            self.stage_statistics(f"{path}.stage_statistics", rec["stage_statistics"])

    def lu(self, path: str, rec: Any, dimension: int | None) -> None:
        if not self.keys(path, rec, LU_KEYS):
            return
        self.bool_(f"{path}.ok", rec["ok"])
        self.int_(f"{path}.steps", rec["steps"])
        self.float_(f"{path}.max_step_error_estimate", rec["max_step_error_estimate"])
        self.float_(f"{path}.rel_max_norm_error", rec["rel_max_norm_error"], minimum=0.0)
        self.hex_vector(f"{path}.y_last", rec["y_last"], dimension)

    def reference(self, path: str, ref: Any, dimension: int | None) -> None:
        """A BASE reference: exact (closed form) or dense (bound to its own recorded 1e-13 / 1e-12 runs)."""
        if not self.dict_(path, ref) or not self.str_(f"{path}.kind", ref.get("kind")):
            return
        kind = ref["kind"]
        if kind in REFERENCE_KINDS_EXACT:
            expected = {"dense_1e-12", "dense_1e-13", "kind", "y"}
        elif kind in REFERENCE_KINDS_DENSE:
            expected = {"dense_1e-12", "dense_1e-13", "kind", "y", "y_1e-12"}
        else:
            self.fail(f"{path}.kind", f"unregistered reference kind {kind!r}")
            return
        if not self.keys(path, ref, expected):
            return
        self.hex_vector(f"{path}.y", ref["y"], dimension)
        for run in ("dense_1e-12", "dense_1e-13"):
            self.adaptive(f"{path}.{run}", ref[run], "dense", dimension)
        if kind in REFERENCE_KINDS_DENSE:
            self.hex_vector(f"{path}.y_1e-12", ref["y_1e-12"], dimension)
            if isinstance(ref["dense_1e-13"], dict) and ref["y"] != ref["dense_1e-13"].get("y_last"):
                self.fail(f"{path}.y", "dense reference is not the state of its recorded dense_1e-13 run")
            if isinstance(ref["dense_1e-12"], dict) and ref["y_1e-12"] != ref["dense_1e-12"].get("y_last"):
                self.fail(f"{path}.y_1e-12", "1e-12 reference is not the state of its recorded dense_1e-12 run")

    def arm_list(self, path: str, arms: Any, expected: Iterable[str], keys: Callable[[dict], set]) -> None:
        if not self.list_(path, arms, nonempty=True):
            return
        names = []
        for i, a in enumerate(arms):
            p = f"{path}[{i}]"
            if not self.dict_(p, a) or not self.keys(p, a, keys(a)):
                continue
            if self.str_(f"{p}.arm", a["arm"]):
                names.append(a["arm"])
            if self.keys(f"{p}.options", a["options"], ARM_OPTION_KEYS):
                for k in sorted(ARM_OPTION_KEYS - {"policy"}):
                    self.bool_(f"{p}.options.{k}", a["options"][k])
                self.str_(f"{p}.options.policy", a["options"]["policy"])
            if "budget" in a:
                self.int_(f"{p}.budget", a["budget"], minimum=1)
        if sorted(names) != sorted(expected) or len(set(names)) != len(names):
            self.fail(path, f"arm names {names} != registered {list(expected)}")


# ------------------------------------------------------------------------------------------ loading
def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _load(role: str, path: Path, pin: str | None, docs: dict, digests: dict, reasons: list[str]) -> None:
    try:
        data = Path(path).read_bytes()
    except OSError as exc:
        reasons.append(f"{role}: cannot read {path}: {exc}")
        return
    digests[role] = _sha256(data)
    if pin is not None and digests[role] != pin:
        reasons.append(f"{role}: {path} is not the registered immutable input "
                       f"(sha256 {digests[role]} != {pin})")
    try:
        docs[role] = loads_strict(data)
    except ValidationError as exc:
        reasons.extend(f"{role}: {r}" for r in exc.reasons)


def _load_all(spec: list[tuple[str, Path | None, str | None]]) -> tuple[dict, dict, dict]:
    docs: dict = {}
    digests: dict = {}
    paths: dict = {}
    reasons: list[str] = []
    for role, path, pin in spec:
        if path is None:
            continue
        paths[role] = Path(path)
        _load(role, Path(path), pin, docs, digests, reasons)
    if reasons:
        raise ValidationError(reasons)
    return docs, digests, paths


def _pin(pins: Mapping[str, str] | None, role: str) -> str | None:
    return None if pins is None else pins[role]


def _optional(path: Path | None) -> Path | None:
    """Optional reproduction inputs follow the checkers' rule: read only when the file exists."""
    return Path(path) if path is not None and Path(path).exists() else None


def _cell_key_ok(c: _Checks, path: str, row: Any, fields: dict[str, str]) -> bool:
    if not c.dict_(path, row):
        return False
    ok = True
    for name, kind in fields.items():
        value = row.get(name)
        if kind == "str":
            ok &= c.str_(f"{path}.{name}", value)
        elif kind == "float+":
            ok &= c.float_(f"{path}.{name}", value, positive=True)
        elif kind == "int+":
            ok &= c.int_(f"{path}.{name}", value, minimum=1)
    return ok


def _rows(c: _Checks, path: str, rows: Any, fields: dict[str, str], key: Callable[[dict], tuple]
          ) -> dict[tuple, dict]:
    """Type the key fields of every raw row, then index with raw-duplicate detection."""
    if not c.list_(path, rows, nonempty=True):
        return {}
    typed = [r for i, r in enumerate(rows) if _cell_key_ok(c, f"{path}[{i}]", r, fields)]
    if len(typed) != len(rows):
        return {}
    return c.unique_index(path, rows, key)


def _case_dimensions(c: _Checks, path: str, rows: Iterable[dict]) -> dict[str, int]:
    dims: dict[str, int] = {}
    for r in rows:
        d = dims.setdefault(r["case"], r["dimension"])
        if d != r["dimension"]:
            c.fail(path, f"case {r['case']!r} has dimensions {d} and {r['dimension']}")
    return dims


def _validate_reproduction(c: _Checks, role: str, doc: Any, arms: Iterable[str], ladders: bool) -> None:
    """Optional reported-only reproduction input: typed unique rows carrying the compared arms."""
    keys = {"arms", "export", "node", "rows"} | ({"budget", "fd_variant", "ladders", "pilot_ladder_budget"}
                                                  if ladders else set())
    if not c.keys(role, doc, keys):
        return
    index = _rows(c, f"{role}.rows", doc["rows"], {"group": "str", "case": "str", "rtol": "float+",
                                                     "dimension": "int+"},
                  lambda r: (r["group"], r["case"], r["rtol"]))
    for k, r in index.items():
        p = f"{role}.rows{list(k)}"
        if c.dict_(f"{p}.arms", r.get("arms")):
            for arm in arms:
                rec = r["arms"].get(arm)
                if c.dict_(f"{p}.arms.{arm}", rec):
                    _reproduction_record(c, f"{p}.arms.{arm}", rec, r["dimension"])
    if ladders:
        lindex = _rows(c, f"{role}.ladders", doc["ladders"], {"ladder": "str", "rtol": "float+", "k": "int+",
                                                               "dimension": "int+"},
                       lambda r: (r["ladder"], r["rtol"], r["k"]))
        for k, r in lindex.items():
            p = f"{role}.ladders{list(k)}"
            if c.dict_(f"{p}.arms_pilot_budget", r.get("arms_pilot_budget")):
                for arm in arms:
                    rec = r["arms_pilot_budget"].get(arm)
                    if c.dict_(f"{p}.arms_pilot_budget.{arm}", rec):
                        _reproduction_record(c, f"{p}.arms_pilot_budget.{arm}", rec, r["dimension"])


def _reproduction_record(c: _Checks, path: str, rec: dict, dimension: int) -> None:
    """Older (ALG01/ALG03) record schema: the compared fields are typed; states are full-length and finite."""
    c.bool_(f"{path}.ok", rec.get("ok"))
    c.hex_vector(f"{path}.y_last", rec.get("y_last"), dimension)
    if "t" in rec:
        c.hex_vector(f"{path}.t", rec["t"])
    for k in ("accepted", "attempts", "rejected", "steps", "internal_steps", "output_clipped_steps",
              "state_reuses", "failed_step"):
        if k in rec:
            c.int_(f"{path}.{k}", rec[k])
    if c.dict_(f"{path}.counters", rec.get("counters")):
        for k, v in rec["counters"].items():
            c.int_(f"{path}.counters.{k}", v)


# -------------------------------------------------------------------------------------------- ALG04
def validate_alg04(base_path: Path, runs_path: Path, alg01_runs_path: Path | None = None,
                   pins: Mapping[str, str] | None = REGISTERED_SHA256) -> Evidence:
    """ALG04: ALG01 BASE (immutable) + ALG04 RUNS (+ optional ALG01 RUNS, reported reproduction only)."""
    docs, digests, paths = _load_all([("base", base_path, _pin(pins, "alg01_base")),
                                      ("runs", runs_path, None),
                                      ("alg01_runs", _optional(alg01_runs_path), _pin(pins, "alg01_runs"))])
    base, runs = docs["base"], docs["runs"]
    c = _Checks()
    base_ok = c.keys("base", base, {"budget", "export", "ladder_references", "ladders", "node",
                                    "pilot_ladder_budget", "references", "rows"})
    runs_ok = c.keys("runs", runs, {"arms", "budget", "export", "ladder_budget", "ladders", "node", "rows"})
    if not (base_ok and runs_ok):
        c.done()
    c.equal("base.node", base["node"], "alg01_coupled_stage_target_20261008")
    c.equal("base.export", base["export"], "export_base")
    c.equal("runs.node", runs["node"], "alg04_coupled_target_v2_20261010")
    c.equal("runs.export", runs["export"], "export_runs_alg04")
    if c.int_("base.budget", base["budget"], 1):
        c.equal("runs.budget", runs["budget"], base["budget"])
    if c.int_("base.pilot_ladder_budget", base["pilot_ladder_budget"], 1):
        c.equal("runs.ladder_budget", runs["ladder_budget"], base["pilot_ladder_budget"])
    c.arm_list("runs.arms", runs["arms"], ALG04_ARMS, lambda a: {"arm", "options"})

    cell_fields = {"group": "str", "case": "str", "rtol": "float+", "dimension": "int+"}
    cell = lambda r: (r["group"], r["case"], r["rtol"])  # noqa: E731
    base_rows = _rows(c, "base.rows", base["rows"], cell_fields, cell)
    run_rows = _rows(c, "runs.rows", runs["rows"], cell_fields, cell)
    rung_fields = {"ladder": "str", "rtol": "float+", "k": "int+", "dimension": "int+"}
    rung = lambda r: (r["ladder"], r["rtol"], r["k"])  # noqa: E731
    base_ladders = _rows(c, "base.ladders", base["ladders"], rung_fields, rung)
    run_ladders = _rows(c, "runs.ladders", runs["ladders"], rung_fields, rung)
    c.done()  # keys of every row are typed and unique from here on

    if set(base_rows) != set(run_rows):
        c.fail("runs.rows", f"cell set differs from BASE (missing {sorted(set(base_rows) - set(run_rows))}, "
                            f"unexpected {sorted(set(run_rows) - set(base_rows))})")
    if set(base_ladders) != set(run_ladders):
        c.fail("runs.ladders", "rung set differs from BASE "
                               f"(missing {sorted(set(base_ladders) - set(run_ladders))}, "
                               f"unexpected {sorted(set(run_ladders) - set(base_ladders))})")
    for k in base_rows:
        if k[0] not in ALG04_GROUPS:
            c.fail(f"base.rows{list(k)}", f"unregistered group {k[0]!r}")
    for k in base_ladders:
        if k[0] not in ALG04_LADDERS:
            c.fail(f"base.ladders{list(k)}", f"unregistered ladder {k[0]!r}")
    dims = _case_dimensions(c, "base.rows", base_rows.values())

    # References: exactly the cases of the cells, each with the cell dimension.
    if c.dict_("base.references", base["references"]):
        if set(base["references"]) != set(dims):
            c.fail("base.references", f"reference cases {sorted(base['references'])} != cell cases {sorted(dims)}")
        for case in sorted(set(base["references"]) & set(dims)):
            c.reference(f"base.references.{case}", base["references"][case], dims[case])
    ladder_dims = _case_dimensions(c, "base.ladders", [{"case": r["ladder"], "dimension": r["dimension"]}
                                                       for r in base_ladders.values()])
    if c.dict_("base.ladder_references", base["ladder_references"]):
        if set(base["ladder_references"]) != set(ladder_dims):
            c.fail("base.ladder_references", "ladder reference set differs from the ladders")
        for name in sorted(set(base["ladder_references"]) & set(ladder_dims)):
            p = f"base.ladder_references.{name}"
            ref = base["ladder_references"][name]
            if c.keys(p, ref, {"kind", "y"}):
                c.str_(f"{p}.kind", ref["kind"])
                c.hex_vector(f"{p}.y", ref["y"], ladder_dims[name])

    for k, b in base_rows.items():
        p = f"base.rows{list(k)}"
        if c.keys(p, b, {"case", "dimension", "group", "legacy", "rtol", "twin"}):
            c.adaptive(f"{p}.legacy", b["legacy"], "plain", b["dimension"])
            c.adaptive(f"{p}.twin", b["twin"], "dense", b["dimension"])
    for k, b in base_ladders.items():
        p = f"base.ladders{list(k)}"
        if c.keys(p, b, {"dimension", "k", "ladder", "legacy", "legacy_pilot_budget", "lu", "rtol"}):
            c.ladder(f"{p}.legacy", b["legacy"], False, b["dimension"])
            c.ladder(f"{p}.legacy_pilot_budget", b["legacy_pilot_budget"], False, b["dimension"])
            c.lu(f"{p}.lu", b["lu"], b["dimension"])

    for k, r in run_rows.items():
        p = f"runs.rows{list(k)}"
        if not c.keys(p, r, {"arms", "case", "dimension", "group", "rtol", "twin"}):
            continue
        b = base_rows.get(k)
        if b is not None and r["dimension"] != b["dimension"]:
            c.fail(f"{p}.dimension", f"{r['dimension']} != BASE {b['dimension']}")
        if c.keys(f"{p}.arms", r["arms"], ALG04_ARMS):
            for arm in ALG04_ARMS:
                c.adaptive(f"{p}.arms.{arm}", r["arms"][arm], "plain" if arm in ALG04_PLAIN_ARMS else "staged",
                           r["dimension"])
        c.adaptive(f"{p}.twin", r["twin"], "dense", r["dimension"])
        if b is not None and r["twin"] != b.get("twin"):
            c.fail(f"{p}.twin", "twin is not bound to BASE (differs from BASE twin)")
    for k, r in run_ladders.items():
        p = f"runs.ladders{list(k)}"
        if not c.keys(p, r, {"arms", "dimension", "k", "ladder", "legacy_budget_200", "lu", "rtol"}):
            continue
        b = base_ladders.get(k)
        if b is not None and r["dimension"] != b["dimension"]:
            c.fail(f"{p}.dimension", f"{r['dimension']} != BASE {b['dimension']}")
        if c.keys(f"{p}.arms", r["arms"], ALG04_ARMS):
            for arm in ALG04_ARMS:
                c.ladder(f"{p}.arms.{arm}", r["arms"][arm], arm not in ALG04_PLAIN_ARMS, r["dimension"])
        c.ladder(f"{p}.legacy_budget_200", r["legacy_budget_200"], False, r["dimension"])
        c.lu(f"{p}.lu", r["lu"], r["dimension"])
        if b is not None and r["lu"] != b.get("lu"):
            c.fail(f"{p}.lu", "LU reference rung is not bound to BASE (differs from BASE lu)")

    if "alg01_runs" in docs:
        _validate_reproduction(c, "alg01_runs", docs["alg01_runs"],
                               ("legacy", "proj_l2", "l2_coupled", "coupled_guarded"), ladders=True)
    c.done()
    return Evidence("ALG04", docs, digests, paths)


# -------------------------------------------------------------------------------------------- ALG06
def validate_alg06(base_path: Path, runs_path: Path, alg03_runs_path: Path | None = None,
                   pins: Mapping[str, str] | None = REGISTERED_SHA256) -> Evidence:
    """ALG06: ALG03 BASE (immutable) + ALG06 RUNS (+ optional ALG03 RUNS, reported reproduction only)."""
    docs, digests, paths = _load_all([("base", base_path, _pin(pins, "alg03_base")),
                                      ("runs", runs_path, None),
                                      ("alg03_runs", _optional(alg03_runs_path), _pin(pins, "alg03_runs"))])
    base, runs = docs["base"], docs["runs"]
    c = _Checks()
    if not (c.keys("base", base, {"export", "node", "references", "rows"})
            and c.keys("runs", runs, {"arms", "charge_columns", "export", "node", "rows"})):
        c.done()
    c.equal("base.node", base["node"], "alg03_stage_budget_guard_20261008")
    c.equal("base.export", base["export"], "export_base_alg03")
    c.equal("runs.node", runs["node"], "alg06_guard_v2_20261010")
    c.equal("runs.export", runs["export"], "export_runs_alg06")
    c.equal("runs.charge_columns", runs["charge_columns"], CHARGE_COLUMNS)
    c.arm_list("runs.arms", runs["arms"], (*ALG06_ARMS, ALG06_D5_ARM),
               lambda a: {"arm", "budget", "options"} | ({"cells"} if a.get("arm") == ALG06_D5_ARM else set()))
    if isinstance(runs["arms"], list):
        for i, a in enumerate(runs["arms"]):
            if isinstance(a, dict) and a.get("arm") == ALG06_D5_ARM:
                c.equal(f"runs.arms[{i}].cells", a.get("cells"), "D5")

    fields = {"group": "str", "case": "str", "rtol": "float+", "dimension": "int+", "max_attempts": "int+"}
    cell = lambda r: (r["group"], r["case"], r["rtol"])  # noqa: E731
    base_rows = _rows(c, "base.rows", base["rows"], fields, cell)
    run_rows = _rows(c, "runs.rows", runs["rows"], fields, cell)
    c.done()

    for k in base_rows:
        if k[0] not in ALG06_BASE_GROUPS:
            c.fail(f"base.rows{list(k)}", f"unregistered group {k[0]!r}")
    d14 = {k for k in run_rows if k[0] != "D5"}
    d5 = {(k[1], k[2]) for k in run_rows if k[0] == "D5"}
    if d14 != set(base_rows):
        c.fail("runs.rows", f"D1-D4 cell set differs from BASE (missing {sorted(set(base_rows) - d14)}, "
                            f"unexpected {sorted(d14 - set(base_rows))})")
    if d5 != ALG06_D5_CELLS:
        c.fail("runs.rows", f"D5 cell set differs from the registered 14 cells (missing "
                            f"{sorted(ALG06_D5_CELLS - d5)}, unexpected {sorted(d5 - ALG06_D5_CELLS)})")
    dims = _case_dimensions(c, "base.rows", base_rows.values())
    _case_dimensions(c, "runs.rows", run_rows.values())
    if c.dict_("base.references", base["references"]):
        if set(base["references"]) != set(dims):
            c.fail("base.references", f"reference cases {sorted(base['references'])} != cell cases {sorted(dims)}")
        for case in sorted(set(base["references"]) & set(dims)):
            c.reference(f"base.references.{case}", base["references"][case], dims[case])

    for k, b in base_rows.items():
        p = f"base.rows{list(k)}"
        if c.keys(p, b, {"case", "dimension", "group", "legacy_200", "legacy_2000", "max_attempts", "rtol",
                         "twin"}):
            for rec in ("legacy_200", "legacy_2000"):
                c.adaptive(f"{p}.{rec}", b[rec], "plain", b["dimension"])
            c.adaptive(f"{p}.twin", b["twin"], "dense", b["dimension"])
    for k, r in run_rows.items():
        p = f"runs.rows{list(k)}"
        d5_row = k[0] == "D5"
        expected = {"arms", "case", "dimension", "group", "max_attempts", "rtol"} | (set() if d5_row else {"twin"})
        if not c.keys(p, r, expected):
            continue
        arms = (*ALG06_ARMS, ALG06_D5_ARM) if d5_row else ALG06_ARMS
        if c.keys(f"{p}.arms", r["arms"], arms):
            for arm in arms:
                c.adaptive(f"{p}.arms.{arm}", r["arms"][arm], "plain" if arm == "rbig" else "staged",
                           r["dimension"])
        b = base_rows.get(k)
        if d5_row or b is None:  # an unexpected D1-D4 cell is already reported against the BASE cell set
            continue
        for name in ("dimension", "max_attempts"):
            if r[name] != b[name]:
                c.fail(f"{p}.{name}", f"{r[name]} != BASE {b[name]}")
        c.adaptive(f"{p}.twin", r["twin"], "dense", r["dimension"])
        if r["twin"] != b.get("twin"):
            c.fail(f"{p}.twin", "twin is not bound to BASE (differs from BASE twin)")

    if "alg03_runs" in docs:
        _validate_reproduction(c, "alg03_runs", docs["alg03_runs"], ("rbig", "b2"), ladders=False)
    c.done()
    return Evidence("ALG06", docs, digests, paths)


# -------------------------------------------------------------------------------------------- ALG05
def alg05_endpoint_error(state: list[float], reference: list[float]) -> float:
    """The ALG05 metric (identical expression to the checker's), behind the metric guard."""
    require_metric_pair(state, reference)
    return max(abs(a - r) / max(abs(r), ALG05_ERROR_FLOOR) for a, r in zip(state, reference))


def validate_alg05(base_path: Path, runs_path: Path, native_path: Path,
                   pins: Mapping[str, str] | None = REGISTERED_SHA256) -> Evidence:
    """ALG05: ALG05 BASE (immutable) + RUNS + NATIVE.json references (immutable)."""
    docs, digests, paths = _load_all([("base", base_path, _pin(pins, "alg05_base")),
                                      ("runs", runs_path, None),
                                      ("native", native_path, _pin(pins, "native"))])
    base, runs, native = docs["base"], docs["runs"], docs["native"]
    c = _Checks()
    header = {"driver", "error_metric", "max_attempts", "rows", "schema", "seeds"}
    if not (c.keys("base", base, header) and c.keys("runs", runs, header | {"arms"})
            and c.dict_("native", native) and c.dict_("native.references", native.get("references"))):
        c.done()
    c.equal("base.schema", base["schema"], "vigilode-alg05-base-v1")
    c.equal("runs.schema", runs["schema"], "vigilode-alg05-runs-v1")
    for doc_name, doc in (("base", base), ("runs", runs)):
        for k, v in ALG05_HEADER.items():
            c.equal(f"{doc_name}.{k}", doc[k], v)
        c.equal(f"{doc_name}.seeds", doc["seeds"], list(ALG05_SEEDS))
    c.equal("runs.arms", runs["arms"], list(ALG05_ARMS))

    refs: dict[str, list[float]] = {}
    for problem, ref in sorted(native["references"].items()):
        p = f"native.references.{problem}"
        if c.keys(p, ref, NATIVE_REFERENCE_KEYS):
            c.str_(f"{p}.method", ref["method"])
            for k in ("uncertainty", "lsoda_rtol_1e-12_difference", "seconds"):
                c.float_(f"{p}.{k}", ref[k], minimum=0.0)
            y = c.float_vector(f"{p}.final_state", ref["final_state"])
            if y is not None:
                refs[problem] = y
    for problem in ALG05_PROBLEMS:
        if problem not in native["references"]:
            c.fail("native.references", f"missing reference for {problem!r}")

    fields = {"problem": "str", "arm": "str", "rtol": "float+", "seed": "float+"}
    base_index = _rows(c, "base.rows", base["rows"], fields, lambda r: (r["problem"], r["rtol"], r["seed"]))
    run_index = _rows(c, "runs.rows", runs["rows"], fields,
                      lambda r: (r["problem"], r["arm"], r["rtol"], r["seed"]))
    c.done()

    def row(path: str, r: dict) -> None:
        if not c.keys(path, r, ALG05_ROW_KEYS):
            return
        if r["problem"] not in ALG05_PROBLEMS:
            c.fail(f"{path}.problem", f"unregistered problem {r['problem']!r}")
        if r["arm"] not in ALG05_ARMS:
            c.fail(f"{path}.arm", f"unregistered arm {r['arm']!r}")
        if r["seed"] not in ALG05_SEEDS:
            c.fail(f"{path}.seed", f"unregistered seed {r['seed']!r}")
        c.float_(f"{path}.atol", r["atol"], positive=True)
        c.bool_(f"{path}.ok", r["ok"])
        c.bool_(f"{path}.success", r["success"])
        c.str_(f"{path}.message", r["message"], nonempty=False)
        for k in ALG05_ROW_INTS:
            c.int_(f"{path}.{k}", r[k])
        c.counters(f"{path}.counters", r["counters"], COUNTERS_DENSE)
        c.hex_vector(f"{path}.t_last", r["t_last"], 1)
        ref = refs.get(r["problem"])
        y = c.hex_vector(f"{path}.final_state", r["final_state"], len(ref) if ref is not None else None)
        if c.float_(f"{path}.error", r["error"], minimum=0.0) and y is not None and ref is not None \
                and r["ok"] is True and r["success"] is True:
            recomputed = alg05_endpoint_error(y, ref)
            if recomputed != r["error"]:
                c.fail(f"{path}.error", f"recorded {r['error']!r} != recomputed {recomputed!r} from the state bits "
                                        f"and the NATIVE reference")

    for k, r in base_index.items():
        row(f"base.rows{list(k)}", r)
        if r["arm"] != ALG05_REFERENCE_ARM:
            c.fail(f"base.rows{list(k)}.arm", f"BASE holds arm {ALG05_REFERENCE_ARM!r} only, got {r['arm']!r}")
    for k, r in run_index.items():
        row(f"runs.rows{list(k)}", r)
    if len(base_index) != ALG05_CELLS_PER_ARM:
        c.fail("base.rows", f"{len(base_index)} cells != registered {ALG05_CELLS_PER_ARM}")
    if len(runs["rows"]) != len(ALG05_ARMS) * ALG05_CELLS_PER_ARM:
        c.fail("runs.rows", f"{len(runs['rows'])} rows != registered {len(ALG05_ARMS) * ALG05_CELLS_PER_ARM}")
    cells = set(base_index)
    for arm in ALG05_ARMS:
        arm_cells = {(k[0], k[2], k[3]) for k in run_index if k[1] == arm}
        if arm_cells != cells:
            c.fail("runs.rows", f"arm {arm!r} cell set differs from BASE (missing {len(cells - arm_cells)}, "
                                f"unexpected {len(arm_cells - cells)})")
    c.done()
    return Evidence("ALG05", docs, digests, paths)


# -------------------------------------------------------------------------------------------- PY05
PY05_NODE = "research/py05_leja_total_certificate_20261011"
PY05_STAGE0_SCHEMA = "vigilode-py05-stage0-cases-v1"
PY05_STAGE1_SCHEMA = "vigilode-py05-stage1-cases-v1"
PY05_RUNS_SCHEMA = "vigilode-py05-runs-v1"
PY05_PROFILE_SCHEMA = "vigilode-py05-profile-v1"
PY05_ARMS = frozenset(("leja-cert", "cheb-cert", "leja-est", "router"))
PY05_CACHE_STATES = frozenset(("cold", "warm"))


def validate_py05(cases_path: Path, runs_path: Path | None = None,
                  profile_path: Path | None = None) -> Evidence:
    """PY05: Newton-Leja total certificate evidence validation."""
    specs: list[tuple[str, Path | None, str | None]] = [("cases", cases_path, None)]
    if runs_path is not None:
        specs.append(("runs", runs_path, None))
    if profile_path is not None:
        specs.append(("profile", profile_path, None))
    docs, digests, paths = _load_all(specs)
    cases = docs["cases"]
    c = _Checks()
    if not c.dict_("cases", cases):
        c.done()

    cases_schema = cases.get("schema")
    if cases_schema not in (PY05_STAGE0_SCHEMA, PY05_STAGE1_SCHEMA):
        c.fail("cases.schema", f"expected {PY05_STAGE0_SCHEMA!r} or {PY05_STAGE1_SCHEMA!r}, got {cases_schema!r}")
    c.equal("cases.node", cases.get("node"), PY05_NODE)

    if cases_schema == PY05_STAGE0_SCHEMA:
        if c.list_("cases.cases", cases.get("cases"), nonempty=True):
            if len(cases["cases"]) != 60:
                c.fail("cases.cases", f"expected 60 stage0 fixtures, got {len(cases['cases'])}")
            for i, fixture in enumerate(cases["cases"]):
                p = f"cases.cases[{i}]"
                c.str_(f"{p}.label", fixture.get("label"))
                c.int_(f"{p}.n", fixture.get("n"), minimum=1)
                c.str_(f"{p}.h_hex", fixture.get("h_hex"))
                c.str_(f"{p}.tolerance_hex", fixture.get("tolerance_hex"))
    elif cases_schema == PY05_STAGE1_SCHEMA:
        if c.list_("cases.cells", cases.get("cells"), nonempty=True):
            if len(cases["cells"]) != 40:
                c.fail("cases.cells", f"expected 40 registered cells, got {len(cases['cells'])}")
            cell_ids = set()
            for i, cell in enumerate(cases["cells"]):
                p = f"cases.cells[{i}]"
                cid = cell.get("cell_id")
                if c.str_(f"{p}.cell_id", cid):
                    if cid in cell_ids:
                        c.fail(f"{p}.cell_id", f"duplicate cell_id {cid!r}")
                    cell_ids.add(cid)
                if cell.get("n") not in (8, 32):
                    c.fail(f"{p}.n", f"expected n in (8, 32), got {cell.get('n')}")
                if cell.get("hrho") not in (10, 20, 40, 80, 160):
                    c.fail(f"{p}.hrho", f"expected hrho in (10, 20, 40, 80, 160), got {cell.get('hrho')}")
                if cell.get("input") not in ("distinct", "same"):
                    c.fail(f"{p}.input", f"expected input in ('distinct', 'same'), got {cell.get('input')}")

    if "runs" in docs:
        runs = docs["runs"]
        if c.dict_("runs", runs):
            c.equal("runs.schema", runs.get("schema"), PY05_RUNS_SCHEMA)
            c.equal("runs.node", runs.get("node"), PY05_NODE)
            c.str_("runs.source_commit", runs.get("source_commit"))
            c.bool_("runs.tree_clean", runs.get("tree_clean"))
            if c.list_("runs.rows", runs.get("rows"), nonempty=True):
                if len(runs["rows"]) != 320:
                    c.fail("runs.rows", f"expected 320 rows (40 cells * 4 arms * 2 cache states), got {len(runs['rows'])}")
                row_keys = set()
                for i, r in enumerate(runs["rows"]):
                    p = f"runs.rows[{i}]"
                    cid = r.get("cell_id")
                    arm = r.get("arm")
                    cs = r.get("cache_state")
                    key = (cid, arm, cs)
                    if key in row_keys:
                        c.fail(p, f"duplicate row key {key!r}")
                    row_keys.add(key)
                    if arm not in PY05_ARMS:
                        c.fail(f"{p}.arm", f"unregistered arm {arm!r}")
                    if cs not in PY05_CACHE_STATES:
                        c.fail(f"{p}.cache_state", f"unregistered cache_state {cs!r}")
                    c.bool_(f"{p}.admitted", r.get("admitted"))
                    c.str_(f"{p}.status", r.get("status"))
                    c.int_(f"{p}.vector_products", r.get("vector_products"), minimum=0)
                    c.int_(f"{p}.complete_cost", r.get("complete_cost"), minimum=0)

    if "profile" in docs:
        profile = docs["profile"]
        if c.dict_("profile", profile):
            c.equal("profile.schema", profile.get("schema"), PY05_PROFILE_SCHEMA)
            c.equal("profile.node", profile.get("node"), PY05_NODE)

    c.done()
    return Evidence("py05", docs, digests, paths)


# ------------------------------------------------------------------------------------- ME01
ME01_SCHEMA = "vigilode-me01-results-v1"
ME01_PROFILE_SCHEMA = "vigilode-me01-profile-v1"
ME01_ALLOCATIONS_SCHEMA = "rodas5p-stiff-allocations-v1"
ME01_NODE = "me01"


def validate_me01(profile_path: Path, allocations_path: Path, ledger_path: Path) -> Evidence:
    """Validate ME01 full-cost evidence before running scientific gates."""
    specs = [
        ("profile", Path(profile_path), None),
        ("allocations", Path(allocations_path), None),
    ]
    docs, digests, paths = _load_all(specs)
    paths["ledger"] = Path(ledger_path)
    if not paths["ledger"].is_file():
        raise ValidationError([f"ledger file not found: {paths['ledger']}"])
    digests["ledger"] = _sha256(paths["ledger"].read_bytes())

    c = _Checks()
    profile = docs.get("profile")
    if c.dict_("profile", profile):
        c.equal("profile.schema", profile.get("schema"), ME01_PROFILE_SCHEMA)
        c.equal("profile.node", profile.get("node"), ME01_NODE)
        c.bool_("profile.tree_clean", profile.get("tree_clean"))
        c.str_("profile.source_commit", profile.get("source_commit"))
        c.str_("profile.binary_sha256", profile.get("binary_sha256"))
        if c.list_("profile.rows", profile.get("rows"), nonempty=True):
            seen_profile = set()
            for i, r in enumerate(profile["rows"]):
                p = f"profile.rows[{i}]"
                prob = r.get("problem")
                arm = r.get("arm")
                rtol = r.get("rtol")
                key = (prob, arm, rtol)
                if key in seen_profile:
                    c.fail(p, f"duplicate cell {key!r}")
                seen_profile.add(key)
                if r.get("callgrind_deterministic") is not True:
                    c.fail(p, f"callgrind determinism check failed for {key!r}")
                if r.get("grid_states") and r.get("grid_error") is not None:
                    c.float_(f"{p}.grid_error", r.get("grid_error"), positive=True)

    allocations = docs.get("allocations")
    if c.dict_("allocations", allocations):
        c.equal("allocations.schema", allocations.get("schema"), ME01_ALLOCATIONS_SCHEMA)
        c.str_("allocations.binary_sha256", allocations.get("binary_sha256"))
        if c.list_("allocations.entries", allocations.get("entries"), nonempty=True):
            seen_alloc = set()
            for i, entry in enumerate(allocations["entries"]):
                p = f"allocations.entries[{i}]"
                prob = entry.get("problem")
                arm = entry.get("arm")
                rtol = entry.get("rtol")
                key = (prob, arm, rtol)
                if key in seen_alloc:
                    c.fail(p, f"duplicate cell {key!r}")
                seen_alloc.add(key)

    c.done()
    return Evidence("me01", docs, digests, paths)


# ------------------------------------------------------------------------------------- PC01
PC01_STAGE0_CONTRACTS_SCHEMA = "vigilode-pc01-stage0-contracts-v1"
PC01_STAGE0_SCHEMA = "vigilode-pc01-stage0-v1"
PC01_NODE = "research/pc01_right_preconditioner_20261011"
PC01_PREREQUISITES = ("F1", "F2", "F3", "F4", "R")


def validate_pc01_stage0(contracts_path: Path) -> Evidence:
    """Validate PC01 Stage 0 contracts evidence before evaluating Stage 0 outcome."""
    specs = [("contracts", Path(contracts_path), None)]
    docs, digests, paths = _load_all(specs)
    contracts = docs.get("contracts")
    c = _Checks()
    if c.dict_("contracts", contracts):
        c.equal("contracts.schema", contracts.get("schema"), PC01_STAGE0_CONTRACTS_SCHEMA)
        c.equal("contracts.node", contracts.get("node"), PC01_NODE)
        prereqs = contracts.get("prerequisites")
        if c.dict_("contracts.prerequisites", prereqs):
            for key in PC01_PREREQUISITES:
                if key not in prereqs:
                    c.fail("contracts.prerequisites", f"missing prerequisite {key!r}")
                else:
                    item = prereqs[key]
                    p = f"contracts.prerequisites[{key}]"
                    if c.dict_(p, item):
                        c.str_(f"{p}.description", item.get("description"))
                        c.str_(f"{p}.test", item.get("test"))
                        c.bool_(f"{p}.passed", item.get("passed"))
                        c.dict_(f"{p}.details", item.get("details"))
    c.done()
    return Evidence("pc01", docs, digests, paths)


# ------------------------------------------------------------------------------------- TRIAGE
# Component triage (research/component_triage_20261019): RESULTS.json and CLASSIFICATION.json written by
# tools/triage/triage_check.py. Exact key sets, enumerations, internal consistency (outcome vs ratio vs the
# recorded thresholds, recomputed E_up/R_up sums, per-component outcome counts vs the RESULTS comparisons) and
# SHA-256 bindings (CLASSIFICATION.results_sha256 = the RESULTS bytes; optional re-hash of the input files).
TRIAGE_RESULTS_SCHEMA = "vigilode-triage-results-v1"
TRIAGE_CLASSIFICATION_SCHEMA = "vigilode-triage-classification-v1"
TRIAGE_THRESHOLDS_SCHEMA = "vigilode-triage-thresholds-v1"
TRIAGE_RESULTS_KEYS = frozenset(("schema", "campaign", "generator", "inputs", "thresholds",
                                 "ignored_rows_other_campaigns", "references", "rows", "groups", "comparisons",
                                 "external_gaps", "timing"))
TRIAGE_CLASSIFICATION_KEYS = frozenset(("schema", "campaign", "generator", "inputs", "results_sha256",
                                        "components", "summary"))
TRIAGE_INPUT_ROLES = frozenset(("runs", "references", "profile", "allocations", "probes", "timing", "attribution",
                                "components", "corpus", "thresholds", "list"))
TRIAGE_REQUIRED_INPUTS = frozenset(("references", "profile", "allocations", "components", "corpus", "thresholds"))
TRIAGE_REFERENCE_KEYS = frozenset(("problem", "grid", "status", "reason", "E_rho", "reference_limited_targets"))
TRIAGE_ROW_KEYS = frozenset(("arm", "problem", "grid", "rtol", "h0_scale", "status", "external", "class", "E",
                             "E_rho", "E_up", "R", "R_rho", "R_up", "reason", "integrity"))
TRIAGE_GROUP_KEYS = frozenset(("arm", "problem", "grid", "rtol", "external", "targets", "E_up_max", "R_up_max",
                               "R_rho_max", "in_profile_subset", "cost", "cost_reason", "secondary",
                               "secondary_reasons", "counted_work", "counted_work_reason"))
TRIAGE_GAP_KEYS = frozenset(("component", "arm", "peer", "problem", "grid", "target", "work", "peer_work", "rtol",
                             "peer_rtol", "gap", "reason"))
TRIAGE_COMPARISON_KEYS = frozenset(("component", "arm", "kind", "counterpart", "problem", "grid", "target", "tier",
                                    "partition", "outcome", "ratio", "cost", "counterpart_cost", "rtol",
                                    "counterpart_rtol", "reason", "secondary"))
TRIAGE_COMPONENT_KEYS = frozenset(("id", "family", "slot", "loc", "reach", "label", "prune_kind", "provisional",
                                   "blocked_by", "flags", "predicate", "rule_path", "evidence", "criteria", "reasons",
                                   "insufficient_evidence"))
TRIAGE_CRITERIA = frozenset(("c1", "c2", "c3", "allocation", "promote", "specialize", "optimize", "timing"))
TRIAGE_PREDICATE_KEYS = frozenset(("feature", "op", "threshold", "values", "calibration", "holdout"))
TRIAGE_LABELS = ("PROMOTE", "SPECIALIZE", "OPTIMIZE", "PRUNE")
TRIAGE_OUTCOMES = ("GAIN", "NEUTRAL", "LOSS", "capability_gain", "capability_loss", "excluded")
TRIAGE_RATIO_OUTCOMES = frozenset(("GAIN", "NEUTRAL", "LOSS"))
TRIAGE_ROW_STATUSES = frozenset(("ok", "solver_failure", "refused", "error", "process_abort", "unavailable"))
TRIAGE_ROW_CLASSES = frozenset(("computed", "failed", "unknown"))
TRIAGE_STATES = frozenset(("admissible", "inadmissible", "unknown"))
TRIAGE_FLAGS = frozenset(("TIMING_CONFLICT", "PROMOTE_BLOCKED_PROVISIONAL", "NO_EVIDENCE", "C3_MISSING"))
# rule_path tokens -> kind (tools/triage/triage_check.py::RULE_KINDS, explained in tools/triage/README.md)
TRIAGE_RULE_KINDS = {
    "STATIC_PRUNE_RELOCATE": "static", "C1_FAIL": "correctness", "C3_FAIL": "correctness",
    "CORRECTNESS_OK": "correctness", "BLOCKED_BY": "contract", "TIMING_CONFLICT": "contract",
    "C3_MISSING": "contract", "PROMOTE_BLOCKED_PROVISIONAL": "contract", "C_FAIL_REPAIRABLE": "contract",
    "NO_EVIDENCE": "evidence",
    "PROMOTE": "frontier", "NOT_PROMOTE": "frontier", "SPECIALIZE": "frontier", "NOT_SPECIALIZE": "frontier",
    "OPTIMIZE_HEADROOM": "frontier", "OPTIMIZE_EXTERNAL_GAP": "frontier", "OPTIMIZE_UNCHARACTERIZED_WINS": "frontier",
    "PRUNE_DEFAULT": "frontier", "NEUTRAL_TIEBREAK_LOC": "tiebreak", "NEUTRAL_TIE_EQUAL_LOC": "tiebreak",
}
# The label a rule decides (the last deciding rule of a rule_path fixes the label).
TRIAGE_RULE_LABELS = {
    "STATIC_PRUNE_RELOCATE": ("PRUNE",), "C1_FAIL": ("PRUNE",), "C3_FAIL": ("PRUNE",),
    "C_FAIL_REPAIRABLE": ("OPTIMIZE",),
    "NO_EVIDENCE": ("OPTIMIZE", "PRUNE"), "PROMOTE": ("PROMOTE",), "PROMOTE_BLOCKED_PROVISIONAL": ("OPTIMIZE",),
    "SPECIALIZE": ("SPECIALIZE",), "OPTIMIZE_HEADROOM": ("OPTIMIZE",), "OPTIMIZE_EXTERNAL_GAP": ("OPTIMIZE",),
    "OPTIMIZE_UNCHARACTERIZED_WINS": ("OPTIMIZE",), "PRUNE_DEFAULT": ("PRUNE",), "NEUTRAL_TIEBREAK_LOC": ("PRUNE",),
}
TRIAGE_GRIDS = frozenset(("endpoint", "uniform101"))
SHA256_HEX = re.compile(r"^[0-9a-f]{64}$")


def _triage_digest(c: _Checks, path: str, value: Any, required: bool) -> bool:
    if value is None:
        if required:
            c.fail(path, "required input digest is null")
        return False
    if c.keys(path, value, {"name", "sha256"}):
        c.str_(f"{path}.name", value["name"])
        if not isinstance(value["sha256"], str) or not SHA256_HEX.match(value["sha256"]):
            c.fail(f"{path}.sha256", f"not a lowercase SHA-256: {value['sha256']!r}")
            return False
        return True
    return False


def _triage_inputs(c: _Checks, path: str, inputs: Any) -> None:
    if not c.keys(path, inputs, TRIAGE_INPUT_ROLES):
        return
    if c.list_(f"{path}.runs", inputs["runs"], nonempty=True):
        for i, d in enumerate(inputs["runs"]):
            _triage_digest(c, f"{path}.runs[{i}]", d, True)
    for role in sorted(TRIAGE_INPUT_ROLES - {"runs"}):
        _triage_digest(c, f"{path}.{role}", inputs[role], role in TRIAGE_REQUIRED_INPUTS)


def _opt_float(c: _Checks, path: str, value: Any, minimum: float | None = None) -> bool:
    return value is None or c.float_(path, value, minimum=minimum)


def _opt_str(c: _Checks, path: str, value: Any) -> bool:
    return value is None or c.str_(path, value)


def _triage_thresholds(c: _Checks, th: Any) -> dict | None:
    if not c.dict_("results.thresholds", th):
        return None
    c.equal("results.thresholds.schema", th.get("schema"), TRIAGE_THRESHOLDS_SCHEMA)
    ok = True
    for section, keys in (("accuracy", ("targets",)), ("frontier", ("gain_max", "neutral_max", "secondary_costs"))):
        for key in keys:
            if not isinstance(th.get(section), dict) or key not in th[section]:
                c.fail(f"results.thresholds.{section}", f"missing {key}")
                ok = False
    if not ok:
        return None
    targets = th["accuracy"]["targets"]
    if not c.list_("results.thresholds.accuracy.targets", targets, nonempty=True) or \
            not all(type(t) is float and t > 0 for t in targets):
        c.fail("results.thresholds.accuracy.targets", "expected positive floats")
        return None
    return th


def _triage_results(c: _Checks, results: Any) -> tuple[dict | None, dict]:
    if not c.keys("results", results, TRIAGE_RESULTS_KEYS):
        return None, {}
    c.equal("results.schema", results["schema"], TRIAGE_RESULTS_SCHEMA)
    c.str_("results.campaign", results["campaign"])
    c.str_("results.generator", results["generator"])
    c.int_("results.ignored_rows_other_campaigns", results["ignored_rows_other_campaigns"])
    _triage_inputs(c, "results.inputs", results["inputs"])
    th = _triage_thresholds(c, results["thresholds"])
    targets = th["accuracy"]["targets"] if th else None
    secondary = set(th["frontier"]["secondary_costs"]) if th else None
    if c.list_("results.references", results["references"]):
        c.unique_index("results.references", [r for r in results["references"] if isinstance(r, dict)],
                       lambda r: (r.get("problem"), r.get("grid")))
        for i, r in enumerate(results["references"]):
            p = f"results.references[{i}]"
            if c.keys(p, r, TRIAGE_REFERENCE_KEYS):
                c.str_(f"{p}.problem", r["problem"])
                if r["grid"] not in TRIAGE_GRIDS:
                    c.fail(f"{p}.grid", f"unregistered grid {r['grid']!r}")
                c.str_(f"{p}.status", r["status"])
                _opt_float(c, f"{p}.E_rho", r["E_rho"], 0.0)
                if r["status"] == "ok" and r["E_rho"] is None:
                    c.fail(f"{p}.E_rho", "ok reference without E_rho")
                if r["status"] != "ok":
                    c.str_(f"{p}.reason", r["reason"])
                if c.list_(f"{p}.reference_limited_targets", r["reference_limited_targets"]) and targets:
                    for t in r["reference_limited_targets"]:
                        if t not in targets:
                            c.fail(f"{p}.reference_limited_targets", f"{t!r} is not a registered target")
    if c.list_("results.rows", results["rows"]):
        c.unique_index("results.rows", [r for r in results["rows"] if isinstance(r, dict)],
                       lambda r: (r.get("arm"), r.get("problem"), r.get("grid"), r.get("rtol"), r.get("h0_scale")))
        for i, r in enumerate(results["rows"]):
            p = f"results.rows[{i}]"
            if not c.keys(p, r, TRIAGE_ROW_KEYS):
                continue
            c.str_(f"{p}.arm", r["arm"])
            c.str_(f"{p}.problem", r["problem"])
            c.float_(f"{p}.rtol", r["rtol"], positive=True)
            c.float_(f"{p}.h0_scale", r["h0_scale"], positive=True)
            c.bool_(f"{p}.external", r["external"])
            if r["status"] not in TRIAGE_ROW_STATUSES:
                c.fail(f"{p}.status", f"unregistered status {r['status']!r}")
            if r["class"] not in TRIAGE_ROW_CLASSES:
                c.fail(f"{p}.class", f"unregistered class {r['class']!r}")
                continue
            norms = ("E", "E_rho", "E_up", "R", "R_rho", "R_up")
            if r["class"] == "computed":
                if all(c.float_(f"{p}.{k}", r[k], minimum=0.0) for k in norms):
                    if r["E_up"] != r["E"] + r["E_rho"] or r["R_up"] != r["R"] + r["R_rho"]:
                        c.fail(p, "E_up/R_up are not E + E_rho / R + R_rho")
                if r["status"] != "ok":
                    c.fail(p, f"computed accuracy on a {r['status']!r} row")
            else:
                for k in norms:
                    if r[k] is not None:
                        c.fail(f"{p}.{k}", f"{r['class']} row carries a norm")
                c.str_(f"{p}.reason", r["reason"])
            _opt_str(c, f"{p}.integrity", r["integrity"])
    if c.list_("results.groups", results["groups"]):
        c.unique_index("results.groups", [g for g in results["groups"] if isinstance(g, dict)],
                       lambda g: (g.get("arm"), g.get("problem"), g.get("grid"), g.get("rtol")))
        for i, g in enumerate(results["groups"]):
            p = f"results.groups[{i}]"
            if not c.keys(p, g, TRIAGE_GROUP_KEYS):
                continue
            c.bool_(f"{p}.external", g["external"])
            c.bool_(f"{p}.in_profile_subset", g["in_profile_subset"])
            for k in ("E_up_max", "R_up_max", "R_rho_max"):
                _opt_float(c, f"{p}.{k}", g[k], 0.0)
            if g["cost"] is None:
                c.str_(f"{p}.cost_reason", g["cost_reason"])
            elif c.int_(f"{p}.cost", g["cost"], minimum=1) and g["cost_reason"] is not None:
                c.fail(f"{p}.cost_reason", "a cost with a missing-cost reason")
            if g["counted_work"] is not None:
                c.int_(f"{p}.counted_work", g["counted_work"])
            else:
                c.str_(f"{p}.counted_work_reason", g["counted_work_reason"])
            if secondary is not None and c.keys(f"{p}.secondary", g["secondary"], secondary):
                for k in sorted(secondary):
                    if g["secondary"][k] is not None:
                        c.int_(f"{p}.secondary.{k}", g["secondary"][k])
                    elif not isinstance(g["secondary_reasons"], dict) or k not in g["secondary_reasons"]:
                        c.fail(f"{p}.secondary.{k}", "null secondary cost without a reason")
            if c.list_(f"{p}.targets", g["targets"]) and targets is not None:
                listed = [t.get("target") if isinstance(t, dict) else None for t in g["targets"]]
                if listed != targets:
                    c.fail(f"{p}.targets", f"targets {listed} != registered {targets}")
                for j, t in enumerate(g["targets"]):
                    q = f"{p}.targets[{j}]"
                    if c.keys(q, t, {"target", "state", "reason"}):
                        if t["state"] not in TRIAGE_STATES:
                            c.fail(f"{q}.state", f"unregistered state {t['state']!r}")
                        elif t["state"] == "admissible" and t["reason"] is not None:
                            c.fail(f"{q}.reason", "admissible state with a reason")
                        elif t["state"] != "admissible":
                            c.str_(f"{q}.reason", t["reason"])
                        if t["state"] == "admissible" and isinstance(g["E_up_max"], float) and g["E_up_max"] > t["target"]:
                            c.fail(q, f"admissible although E_up_max {g['E_up_max']!r} > target {t['target']!r}")
    counts: dict[str, dict[str, int]] = {}
    if c.list_("results.comparisons", results["comparisons"]):
        for i, r in enumerate(results["comparisons"]):
            p = f"results.comparisons[{i}]"
            if not c.keys(p, r, TRIAGE_COMPARISON_KEYS):
                continue
            c.str_(f"{p}.component", r["component"])
            c.str_(f"{p}.arm", r["arm"])
            if r["kind"] not in ("attribution", "strongest"):
                c.fail(f"{p}.kind", f"unregistered kind {r['kind']!r}")
            if r["outcome"] not in TRIAGE_OUTCOMES:
                c.fail(f"{p}.outcome", f"unregistered outcome {r['outcome']!r}")
                continue
            if targets is not None and r["target"] not in targets:
                c.fail(f"{p}.target", f"{r['target']!r} is not a registered target")
            if r["partition"] not in ("calibration", "holdout"):
                c.fail(f"{p}.partition", f"{r['partition']!r}")
            counts.setdefault(r["component"], {o: 0 for o in TRIAGE_OUTCOMES})[r["outcome"]] += 1
            if r["outcome"] in TRIAGE_RATIO_OUTCOMES:
                ok = c.float_(f"{p}.ratio", r["ratio"], positive=True)
                ok &= c.int_(f"{p}.cost", r["cost"], minimum=1) and c.int_(f"{p}.counterpart_cost",
                                                                           r["counterpart_cost"], minimum=1)
                if ok and r["ratio"] != r["cost"] / r["counterpart_cost"]:
                    c.fail(f"{p}.ratio", "ratio is not cost / counterpart_cost")
                if ok and th is not None:
                    gain_max, neutral_max = th["frontier"]["gain_max"], th["frontier"]["neutral_max"]
                    expected = "GAIN" if r["ratio"] <= gain_max else ("NEUTRAL" if r["ratio"] <= neutral_max else "LOSS")
                    if expected != r["outcome"]:
                        c.fail(f"{p}.outcome", f"{r['outcome']} but ratio {r['ratio']!r} gives {expected}")
                if secondary is not None and c.keys(f"{p}.secondary", r["secondary"], secondary):
                    for k in sorted(secondary):
                        c.keys(f"{p}.secondary.{k}", r["secondary"][k], {"value", "counterpart"})
            else:
                if r["ratio"] is not None or r["secondary"] is not None:
                    c.fail(p, f"{r['outcome']} comparison carries a ratio or secondary costs")
                if r["outcome"] == "excluded":
                    c.str_(f"{p}.reason", r["reason"])
    if c.list_("results.external_gaps", results["external_gaps"]):
        for i, r in enumerate(results["external_gaps"]):
            p = f"results.external_gaps[{i}]"
            if not c.keys(p, r, TRIAGE_GAP_KEYS):
                continue
            if not (isinstance(r["peer"], str) and r["peer"].startswith("ext.")):
                c.fail(f"{p}.peer", f"not an external arm id: {r['peer']!r}")
            if r["gap"] is None:
                c.str_(f"{p}.reason", r["reason"])
            elif c.float_(f"{p}.gap", r["gap"], minimum=0.0) and (
                    not (c.int_(f"{p}.work", r["work"]) and c.int_(f"{p}.peer_work", r["peer_work"], minimum=1))
                    or r["gap"] != r["work"] / r["peer_work"]):
                c.fail(f"{p}.gap", "gap is not work / peer_work")
    if c.keys("results.timing", results["timing"], {"present", "diagnostic", "entries"}):
        c.bool_("results.timing.present", results["timing"]["present"])
        c.list_("results.timing.entries", results["timing"]["entries"])
    return results, counts


def _triage_component(c: _Checks, p: str, e: Any, counts: dict) -> str | None:
    if not c.keys(p, e, TRIAGE_COMPONENT_KEYS):
        return None
    c.str_(f"{p}.id", e["id"])
    c.str_(f"{p}.family", e["family"])
    c.str_(f"{p}.slot", e["slot"])
    c.int_(f"{p}.loc", e["loc"])
    if e["label"] not in TRIAGE_LABELS:
        c.fail(f"{p}.label", f"unregistered label {e['label']!r}")
    c.bool_(f"{p}.provisional", e["provisional"])
    if c.list_(f"{p}.blocked_by", e["blocked_by"]):
        for i, b in enumerate(e["blocked_by"]):
            c.str_(f"{p}.blocked_by[{i}]", b)
        if e["blocked_by"] and e["provisional"] is not True:
            c.fail(f"{p}.provisional", "blocked_by is non-empty but the label is not provisional")
    if e["label"] == "PROMOTE" and e["provisional"] is not False:
        c.fail(f"{p}.label", "PROMOTE is forbidden while provisional")
    if e["prune_kind"] not in (None, "relocate"):
        c.fail(f"{p}.prune_kind", f"unregistered prune kind {e['prune_kind']!r}")
    elif e["prune_kind"] == "relocate" and e["label"] != "PRUNE":
        c.fail(f"{p}.prune_kind", "relocate on a label other than PRUNE")
    if c.list_(f"{p}.flags", e["flags"]):
        for f in e["flags"]:
            if f not in TRIAGE_FLAGS:
                c.fail(f"{p}.flags", f"unregistered flag {f!r}")
        for f in ("TIMING_CONFLICT", "C3_MISSING", "NO_EVIDENCE", "PROMOTE_BLOCKED_PROVISIONAL"):
            if f in e["flags"] and e["provisional"] is not True:
                c.fail(f"{p}.flags", f"{f} without provisional")
    rules = []
    if c.list_(f"{p}.rule_path", e["rule_path"], nonempty=True):
        for i, step in enumerate(e["rule_path"]):
            q = f"{p}.rule_path[{i}]"
            if not c.keys(q, step, {"rule", "kind", "detail"}):
                continue
            if step["rule"] not in TRIAGE_RULE_KINDS:
                c.fail(f"{q}.rule", f"unregistered rule {step['rule']!r}")
                continue
            if step["kind"] != TRIAGE_RULE_KINDS[step["rule"]]:
                c.fail(f"{q}.kind", f"{step['kind']!r} != {TRIAGE_RULE_KINDS[step['rule']]!r} for {step['rule']}")
            _opt_str(c, f"{q}.detail", step["detail"])
            rules.append(step["rule"])
        deciding = [r for r in rules if r in TRIAGE_RULE_LABELS]
        if not deciding:
            c.fail(f"{p}.rule_path", "no rule decides the label")
        elif e["label"] not in TRIAGE_RULE_LABELS[deciding[-1]]:
            c.fail(f"{p}.label", f"{e['label']} but the deciding rule {deciding[-1]} gives "
                                 f"{'/'.join(TRIAGE_RULE_LABELS[deciding[-1]])}")
        for flag in ("TIMING_CONFLICT", "C3_MISSING", "NO_EVIDENCE", "PROMOTE_BLOCKED_PROVISIONAL"):
            if (flag in rules) != (isinstance(e["flags"], list) and flag in e["flags"]):
                c.fail(f"{p}.flags", f"flag {flag} and rule_path disagree")
        if "C_FAIL_REPAIRABLE" in rules and (e["provisional"] is not True
                                             or not ({"C1_FAIL", "C3_FAIL"} & set(rules))):
            c.fail(f"{p}.rule_path", "C_FAIL_REPAIRABLE needs a C1_FAIL/C3_FAIL step and a provisional label")
        if bool(e["blocked_by"]) != ("BLOCKED_BY" in rules):
            c.fail(f"{p}.rule_path", "blocked_by and the BLOCKED_BY rule disagree")
    for key in ("reasons", "insufficient_evidence"):
        if c.list_(f"{p}.{key}", e[key]):
            for i, s in enumerate(e[key]):
                c.str_(f"{p}.{key}[{i}]", s)
    if e["label"] == "SPECIALIZE":
        if c.keys(f"{p}.predicate", e["predicate"], TRIAGE_PREDICATE_KEYS):
            pred = e["predicate"]
            if pred["op"] not in ("le", "ge", "in"):
                c.fail(f"{p}.predicate.op", f"unregistered op {pred['op']!r}")
            elif pred["op"] == "in":
                c.list_(f"{p}.predicate.values", pred["values"], nonempty=True)
            else:
                c.float_(f"{p}.predicate.threshold", pred["threshold"])
            if not isinstance(pred["holdout"], dict) or pred["holdout"].get("verdict") != "PASS" \
                    or pred["holdout"].get("passed") is not True:
                c.fail(f"{p}.predicate.holdout", "SPECIALIZE without a passing holdout verdict")
    elif e["predicate"] is not None:
        c.fail(f"{p}.predicate", f"predicate on a {e['label']} label")
    if c.keys(f"{p}.criteria", e["criteria"], TRIAGE_CRITERIA):
        for name in sorted(TRIAGE_CRITERIA):
            crit = e["criteria"][name]
            if c.dict_(f"{p}.criteria.{name}", crit) and crit.get("status") not in ("pass", "fail", None):
                c.fail(f"{p}.criteria.{name}.status", f"{crit.get('status')!r}")
        if e["label"] == "PROMOTE" and isinstance(e["criteria"].get("promote"), dict) \
                and e["criteria"]["promote"].get("status") != "pass":
            c.fail(f"{p}.label", "PROMOTE without a passing promote criterion")
    if c.dict_(f"{p}.evidence", e["evidence"]) and isinstance(e["id"], str):
        outcomes = e["evidence"].get("outcomes")
        expected = counts.get(e["id"], {o: 0 for o in TRIAGE_OUTCOMES})
        if c.keys(f"{p}.evidence.outcomes", outcomes, TRIAGE_OUTCOMES) and outcomes != expected:
            c.fail(f"{p}.evidence.outcomes", f"{outcomes} != counts of the RESULTS comparisons {expected}")
    return e["id"] if isinstance(e["id"], str) else None


def validate_triage(results_path: Path, classification_path: Path,
                    input_paths: Mapping[str, Any] | None = None) -> Evidence:
    """Component triage: RESULTS.json + CLASSIFICATION.json of tools/triage/triage_check.py.

    ``input_paths`` optionally maps input roles to files (``runs`` to a list of files) whose SHA-256 must equal
    the digests recorded in both documents.
    """
    docs, digests, paths = _load_all([("results", results_path, None), ("classification", classification_path, None)])
    results, classification = docs["results"], docs["classification"]
    c = _Checks()
    results, counts = _triage_results(c, results)
    if c.keys("classification", classification, TRIAGE_CLASSIFICATION_KEYS):
        cl = classification
        c.equal("classification.schema", cl["schema"], TRIAGE_CLASSIFICATION_SCHEMA)
        if cl["results_sha256"] != digests["results"]:
            c.fail("classification.results_sha256", f"{cl['results_sha256']!r} is not the SHA-256 of the RESULTS "
                                                    f"bytes ({digests['results']})")
        if results is not None:
            c.equal("classification.campaign", cl["campaign"], results["campaign"])
            if cl["inputs"] != results["inputs"]:
                c.fail("classification.inputs", "input digests differ from RESULTS.inputs")
        _triage_inputs(c, "classification.inputs", cl["inputs"])
        ids = []
        if c.list_("classification.components", cl["components"], nonempty=True):
            for i, e in enumerate(cl["components"]):
                cid = _triage_component(c, f"classification.components[{i}]", e, counts)
                if cid is not None:
                    if cid in ids:
                        c.fail(f"classification.components[{i}].id", f"duplicate component {cid!r}")
                    ids.append(cid)
        unknown = sorted(set(counts) - set(ids))
        if unknown:
            c.fail("results.comparisons", f"components without a classification: {unknown}")
        if c.keys("classification.summary", cl["summary"], (*TRIAGE_LABELS, "provisional", "components")) and \
                isinstance(cl["components"], list):
            comps = [e for e in cl["components"] if isinstance(e, dict)]
            expected = {label: sum(1 for e in comps if e.get("label") == label) for label in TRIAGE_LABELS}
            expected["provisional"] = sum(1 for e in comps if e.get("provisional") is True)
            expected["components"] = len(comps)
            if cl["summary"] != expected:
                c.fail("classification.summary", f"{cl['summary']} != recount {expected}")
    if input_paths and results is not None and isinstance(results.get("inputs"), dict):
        recorded = results["inputs"]
        for role, given in sorted(input_paths.items()):
            if role not in TRIAGE_INPUT_ROLES:
                c.fail("inputs", f"unknown input role {role!r}")
                continue
            files = given if isinstance(given, (list, tuple)) else [given]
            actual = []
            for f in files:
                try:
                    actual.append(_sha256(Path(f).read_bytes()))
                except OSError as exc:
                    c.fail(f"inputs.{role}", f"cannot read {f}: {exc}")
            want = recorded.get(role)
            want = [d.get("sha256") for d in want] if isinstance(want, list) else \
                ([want.get("sha256")] if isinstance(want, dict) else [])
            if actual != want:
                c.fail(f"inputs.{role}", f"files hash to {actual}, RESULTS records {want}")
    c.done()
    return Evidence("triage", docs, digests, paths)


# ------------------------------------------------------------------------------------- INVALID output
def invalid_report(checker_schema: str, error: ValidationError, inputs: Mapping[str, Path | None]) -> dict:
    """The checker output for rejected evidence: no gate is evaluated, so no gate result is reported."""
    hashes = {}
    for role, path in inputs.items():
        if path is None:
            continue
        try:
            hashes[role] = {"path": str(path), "sha256": _sha256(Path(path).read_bytes())}
        except OSError:
            hashes[role] = {"path": str(path), "sha256": None}
    return {
        "schema": checker_schema,
        "evidence_schema": VERSION,
        "verdict": INVALID,
        "gate": None,
        "inputs": hashes,
        "reasons": list(error.reasons[:MAX_REASONS]),
        "reasons_total": len(error.reasons),
        "claim_scope": "Malformed or unbound evidence: no scientific gate was evaluated; INVALID is neither PASS "
                       "nor FAIL.",
    }


def emit_invalid(output: Path, checker_schema: str, error: ValidationError,
                 inputs: Mapping[str, Path | None]) -> int:
    """Write the INVALID report (immutable output, as for PASS/FAIL), print it, return the exit code."""
    report = invalid_report(checker_schema, error, inputs)
    Path(output).write_text(json.dumps(report, indent=1, sort_keys=True, allow_nan=False) + "\n")
    print(f"verdict: {INVALID} ({VERSION}; {len(error.reasons)} violation(s))")
    for reason in error.reasons[:20]:
        print("  invalid:", reason)
    return INVALID_EXIT_CODE
