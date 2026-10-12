#!/usr/bin/env python3
"""Gate of research node research/py04_fused_normalization_20261011 (PY04; see its PREREGISTRATION.md): the
power-of-two normalization of the fused Taylor phi total certificate.

Inputs:
- ``--cases``: cases.json, written once by ``export_cases`` of crates/rodas5p-core/tests/fused_normalization_contract.rs
  (144 registered cases and the directed-scaling table).
- ``--output``: RESULTS.json (immutable: an existing file is never overwritten).

Target and oracle. F = sum_{k=0}^{4} phi_k(hA) w_k with the binary64 A, h, w_k taken as exact reals: the top block of
mpmath.expm of the exact augmented matrix M = [[hA, W], [0, J]] times v = [w_0; e_4] (W = [w_4, w_3, w_2, w_1], J the
4x4 upper shift). hA is formed exactly (a product of two binary64 numbers has at most 106 bits). The w_k are divided by
the power of two of their largest magnitude before expm and the top block is multiplied back (exact in mpf by
linearity; mpf has no exponent range limit). The reference is computed at 50 digits and again at 70 digits; the
scaling table is checked exactly with fractions.Fraction.

Evidence (INVALID, exit 2) before any gate:
- AS03 rules (tools/evidence_schema_v2.py primitives): strict JSON (no duplicate keys, no NaN/Infinity), exact key
  sets of every record, canonical binary64 hex (16 lowercase digits, finite; ``null`` only where a value may be
  non-finite), typed numbers and enumerations, unique raw labels, exactly the 144 registered cases and the 72
  registered table rows.
- Binding: the checker regenerates every matrix (PP11 generator, SplitMix64 seed 41000 + index), every base w_k and
  every input w = fl(s x base) of its family rule, and the table inputs (seed 41016), and compares bits; the
  registered Taylor budgets; the caller candidate equals the exported Taylor output on the original w (present
  exactly where Taylor accepted).
- References: the 50- and 70-digit references differ by at most 1e-45 ||F||_2 + 2^-1100 in every case (and likewise
  for the L references below).
- Provenance: the checker blob at the cases' source commit equals the checker that runs; the export's recorded tree
  status (``git status --porcelain``) and the checker's own (``--untracked-files=no``) are empty.

Gate, PASS iff all six items hold, else FAIL (exit 1); PASS exit 0:
1. Enclosure: for every Bounded normalized certificate, bound >= ||F_hat - F||_2 + ||ref50 - ref70||_2, and, where a
   caller candidate u is present, caller_bound >= ||u - F||_2 + ||ref50 - ref70||_2. Zero violations.
2. Homogeneity: in each S7 triple (s = 1, 2^600, 2^-600) the normalized internals (v, z, E, perturbation, stepped
   error, normalized-level distance) are bitwise identical; where every rescaling of the triple is exact (R = L = 0
   and sigma E normal, recomputed from the bits) bound_s = s bound_1 exactly. A triple with an inexact rescaling is
   reported and gated on its internals only.
3. Normalization accounting, recomputed from the bits: L = 0 iff every w_k / sigma is exact; R = 0 iff every entry of
   sigma z is exact; a nonzero L or R is > 0 and encloses its actual value (R exactly with fractions: R^2 >=
   sum (F_hat_i - sigma z_i)^2; L >= ||sigma sum_k phi_k(hA) (w_k / sigma - v_k)||_2 + its 50/70-digit difference);
   no Bounded certificate has a non-finite (null) candidate entry or bound.
4. Directed scaling: at every table row, down <= exact <= up, nearest equals the correctly rounded value, and the
   exact flag is correct (an overflowing row: nearest is +-inf (null), up and down are refused (null), not exact).
5. Usefulness at the new extremes: in S1, S2, S3, S4 and S6, every case is Bounded and its bound for F_hat is
   <= 1e-10 ||F||_2 + 16 sqrt(n) 2^-1074.
6. Overflow boundary: each S5 case is Bounded and valid (item 1 holds for it) or refused with
   FUSED_PHI_RANGE_UNSUPPORTED.

Reported, not gated: relative-only usefulness (bound <= 1e-10 ||F||) per family, the Taylor caller candidate's
usefulness, the unnormalized baseline's status, enclosure and usefulness, the dominant term, the normalization work,
bound/error ratios, the consistency of v, F_hat, e and the bound arithmetic with their definitions, and the registered
predictions.

Interpretations and deviations (fixed here, before the recorded run):
1. Seeds. "Fresh SplitMix64 seeds starting at 41000": one seed per matrix, 41000 + index with
   index = ((sym 2 + n_i) 2 + norm_i) 2 + h_i over (symmetric, nonsymmetric) x (5, 12) x (1, 8) x (0.3, 0.25); the
   base w_k are drawn from the same generator after the matrix (the PP11 order). The table uses seed 41016: per
   exponent, four normal values +-m 2^j (j = 0, 1, -1, -40), two subnormals, one short mantissa and one +-m 2^1000.
2. Taylor budgets. The registration fixes no budget: both Taylor calls (normalized data inside the entry point, and
   the caller candidate on the original w) use PP11's 1e-13 (absolute, 2-norm).
3. "sqrt(count)" in R and the norms ||w_k / sigma - v_k||_2 in L count the inexact entries only (each rounding of a
   power-of-two scaling lands in the subnormal range and errs by at most 2^-1075), which is no larger than
   sqrt(n) 2^-1075.
4. "For every Bounded certificate" in item 1 means the normalized certificates (the gated entry point); the
   unnormalized baseline is reported only, its enclosure included. Items 1 and 3 apply to Bounded normalized
   certificates; an Unbounded one has no finite bound to check and is reported (and fails item 5 or 6 in its
   families). A Bounded certificate with a null candidate entry or bound counts as an item-1 violation and an item-3
   failure.
5. The norms in item 1 are evaluated in mpmath at 50 digits; ||ref50 - ref70||_2 is added as the reference's
   uncertainty. Item 5's 1e-10 and 16 sqrt(n) 2^-1074 are evaluated in mpmath at 50 digits.
6. Item 3's "enclosed" for L uses the mpmath reference of sigma F(A, h, d) with d_k = w_k / sigma - v_k (exact
   rationals), computed at 50 and 70 digits like F.
7. The compile_fail type-separation doctests live in crates/rodas5p-core/src/taylor_phi_total.rs (doctests are not
   run by ``--test fused_normalization_contract``); they run under ``cargo test -p rodas5p-core --doc``.
8. An exporter outcome other than a certificate or a FUSED_PHI_RANGE_UNSUPPORTED refusal (``error``) is not INVALID:
   it fails item 5 (S1-S4, S6), item 6 (S5) or item 2 (S7).
9. Non-finite values are exported as JSON null (the AS03 hex rule admits finite values only); null is admitted only
   in fields that may be non-finite (bounds and terms, the rescaled candidate, the stepped candidate, the table's
   nearest/up/down); anywhere else it is INVALID.
10. When the certificate of the normalized data is Unbounded the entry point returns an Unbounded certificate (bound
   infinite), not a refusal; FUSED_PHI_RANGE_UNSUPPORTED is reserved for a rescaled candidate entry or a bound above
   f64::MAX, as registered.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
import subprocess
import sys
from fractions import Fraction
from pathlib import Path

sys.dont_write_bytecode = True
if str(Path(__file__).resolve().parent) not in sys.path:
    sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence_schema_v2 as evidence  # noqa: E402  (fail-closed evidence validation, re-audit AS03)

import mpmath as mp  # noqa: E402

SCHEMA = "vigilode-py04-fused-normalization-check-v1"
CASES_SCHEMA = "vigilode-py04-cases-v1"
NODE = "research/py04_fused_normalization_20261011"
CHECKER_PATH = "tools/py04_fused_normalization_check.py"
INNER_SCHEMA = "vigilode-fused-phi-total-v1"
NORMALIZED_SCHEMA = "vigilode-fused-phi-normalized-v1"
RANGE_CODE = "FUSED_PHI_RANGE_UNSUPPORTED"
TAYLOR_BUDGET = 1e-13
SEED_BASE = 41000
TABLE_SEED = 41016
TABLE_EXPONENTS = (-1074, -1073, -1023, -1022, -1021, 0, 1021, 1022, 1023)
TABLE_ROWS = 72
STAGE_LIMIT = 1000
TERMS = 5
DPS = 50
DPS_CHECK = 70
MAX = Fraction(sys.float_info.max)
MIN_NORMAL = Fraction(2) ** -1022
TINY = Fraction(1, 2 ** 1074)
FAMILIES = (
    ("S1", "S1", (-1016,) * 5),
    ("S2", "S2", (-1050,) * 5),
    ("S3", "S3", (-1068,) * 5),
    ("S4", "S4", (1018,) * 5),
    ("S5", "S5", (1023,) * 5),
    ("S6", "S6", (1000, 0, 0, 0, -1070)),
    ("S7", "S7-s0", (0,) * 5),
    ("S7", "S7-p600", (600,) * 5),
    ("S7", "S7-m600", (-600,) * 5),
)
USEFUL_FAMILIES = ("S1", "S2", "S3", "S4", "S6")

TOP_KEYS = frozenset(("schema", "node", "source_commit", "tree_status", "taylor_budget", "normalized_taylor_budget",
                      "seed_base", "table_seed", "cases", "scaling_table"))
CASE_KEYS = frozenset(("label", "family", "variant", "w_exponents", "matrix_index", "seed", "symmetric", "n",
                       "target_hA_one_norm", "h", "h_power_of_two", "a", "base_w", "w", "taylor_original",
                       "normalized", "baseline"))
TAYLOR_KEYS = frozenset(("accepted", "fused", "rejection"))
OUTCOME_KEYS = frozenset(("outcome", "error", "certificate"))
CERT_KEYS = frozenset(("schema", "status", "unbounded_reason", "exponent", "normalized_vectors",
                       "normalization_inexact", "normalized_candidate", "normalized_source",
                       "normalized_taylor_rejection", "inner", "candidate", "rescale_inexact", "scaled_certificate",
                       "rescale_rounding", "normalization_rounding", "log_norm_upper", "bound", "dominant",
                       "caller_candidate", "caller_distance", "caller_bound", "work"))
INNER_KEYS = frozenset(("schema", "status", "unbounded_reason", "bound", "candidate_distance", "stepped_error",
                        "perturbation", "dominant", "delta_two_norm", "omega", "v_two_norm", "chosen_metric",
                        "stepped_candidate", "attempts"))
ATTEMPT_KEYS = frozenset(("metric", "steps", "bounded", "error_upper"))
WORK_KEYS = frozenset(("input_entries", "input_stages", "candidate_entries", "candidate_stages", "bound_scalings",
                       "bound_stages", "entries_scaled", "stages"))
BASELINE_KEYS = frozenset(("outcome", "error", "candidate_source", "candidate", "certificate"))
TABLE_KEYS = frozenset(("x", "e", "nearest", "up", "down", "exact", "overflow", "stages", "largest_stage"))
STATUSES = ("bounded", "unbounded")
SOURCES = ("taylor", "stepped-top", "zeros")
DOMINANT = ("scaled-certificate", "rescale-rounding", "normalization-rounding")
INNER_DOMINANT = ("candidate-distance", "stepped-exp", "perturbation")
METRICS = ("identity", "osborne", None)
INNER_HEX_NULLABLE = ("bound", "candidate_distance", "stepped_error", "perturbation", "delta_two_norm", "omega",
                      "v_two_norm")
CERT_HEX_NULLABLE = ("scaled_certificate", "rescale_rounding", "normalization_rounding", "log_norm_upper", "bound")


# --------------------------------------------------------------------------------------- arithmetic
def hx(x: float) -> str:
    return struct.pack(">d", x).hex()


def from_bits(bits: int) -> float:
    return struct.unpack(">d", struct.pack(">Q", bits))[0]


def pow2f(k: int) -> float:
    """2^k as a binary64 for -1074 <= k <= 1023 (exact)."""
    return float(Fraction(2) ** k)


def nearest(x: Fraction) -> float:
    """Correctly rounded binary64 (ties to even); +-inf above the range."""
    try:
        return float(x)  # int / int true division: correctly rounded, subnormals included
    except OverflowError:
        return math.inf if x > 0 else -math.inf


def floor_log2(x: Fraction) -> int:
    x = abs(x)
    e = x.numerator.bit_length() - x.denominator.bit_length()
    if Fraction(2) ** e > x:
        e -= 1
    return e


def is_pow2(d: int) -> bool:
    return d > 0 and d & (d - 1) == 0


def mpf_exact(x: Fraction):
    """An exact binary rational as an mpf (raises if the denominator is not a power of two)."""
    if not is_pow2(x.denominator):
        raise ValueError("not a binary rational")
    return mp.ldexp(mp.mpf(x.numerator), -(x.denominator.bit_length() - 1))


class SplitMix:
    MASK = (1 << 64) - 1

    def __init__(self, seed: int):
        self.state = seed & self.MASK

    def next(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & self.MASK
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & self.MASK
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & self.MASK
        return z ^ (z >> 31)

    def symmetric(self) -> float:
        return 2.0 * (float(self.next() >> 11) * 2.0 ** -53) - 1.0


def base_matrix(n: int, symmetric: bool, rng: SplitMix) -> list[list[float]]:
    a = [[0.0] * n for _ in range(n)]
    for i in range(n):
        for j in range(n):
            if i == j or (symmetric and j < i):
                continue
            a[i][j] = rng.symmetric()
            if symmetric:
                a[j][i] = a[i][j]
    for i in range(n):
        row = 0.0
        col = 0.0
        for j in range(n):
            if j != i:
                row += abs(a[i][j])
        for j in range(n):
            if j != i:
                col += abs(a[j][i])
        a[i][i] = -(row + col) / 2.0 - 1.0
    return a


def one_norm(a: list[list[float]]) -> float:
    n = len(a)
    best = 0.0
    for j in range(n):
        s = 0.0
        for i in range(n):
            s += abs(a[i][j])
        best = max(best, s)
    return best


def fmt_norm(x: float) -> str:
    """Rust's Display of the f64 values used in labels (1, 8, 0.3, 0.25)."""
    return repr(x)[:-2] if repr(x).endswith(".0") else repr(x)


def registered_cases() -> list[dict]:
    """Every registered case: label, family, matrix and inputs, regenerated."""
    out = []
    for si, symmetric in enumerate((True, False)):
        for ni, n in enumerate((5, 12)):
            for ti, norm in enumerate((1.0, 8.0)):
                for hi, h in enumerate((0.3, 0.25)):
                    index = ((si * 2 + ni) * 2 + ti) * 2 + hi
                    seed = SEED_BASE + index
                    rng = SplitMix(seed)
                    base = base_matrix(n, symmetric, rng)
                    base_w = [[rng.symmetric() for _ in range(n)] for _ in range(TERMS)]
                    factor = norm / (h * one_norm(base))
                    a = [[x * factor for x in row] for row in base]
                    for family, variant, exps in FAMILIES:
                        w = [[nearest(Fraction(x) * Fraction(2) ** exps[k]) for x in base_w[k]]
                             for k in range(TERMS)]
                        label = (f"{'sym' if symmetric else 'nonsym'}-n{n}-norm{fmt_norm(norm)}-h{fmt_norm(h)}-"
                                 f"{variant}")
                        out.append({"label": label, "family": family, "variant": variant, "w_exponents": list(exps),
                                    "matrix_index": index, "seed": seed, "symmetric": symmetric, "n": n,
                                    "target_hA_one_norm": norm, "h": h, "h_power_of_two": h == 0.25, "a": a,
                                    "base_w": base_w, "w": w})
    return out


def registered_table_inputs() -> list[tuple[float, int]]:
    rng = SplitMix(TABLE_SEED)
    rows = []
    for e in TABLE_EXPONENTS:
        xs = []
        for j in (0, 1, -1, -40):
            sign = -1.0 if rng.next() & 1 == 1 else 1.0
            m = 1.0 + float(rng.next() >> 12) * 2.0 ** -52
            xs.append(sign * m * pow2f(j))
        for _ in range(2):
            sign = -1.0 if rng.next() & 1 == 1 else 1.0
            k = max(rng.next() >> 12, 1)
            xs.append(sign * from_bits(k))
        sign = -1.0 if rng.next() & 1 == 1 else 1.0
        xs.append(sign * (1.0 + float(rng.next() >> 44) * 2.0 ** -20))
        sign = -1.0 if rng.next() & 1 == 1 else 1.0
        xs.append(sign * (1.0 + float(rng.next() >> 12) * 2.0 ** -52) * pow2f(1000))
        rows.extend((x, e) for x in xs)
    return rows


# ---------------------------------------------------------------------------------------- reference
def fused_reference(h: float, a: list[list[float]], w: list[list[Fraction]], dps: int) -> list:
    """Top block of expm(M) [w_0; e_4] for exact binary rationals w, at `dps` digits (mpf list, unrounded scale)."""
    n = len(a)
    nonzero = [abs(x) for wk in w for x in wk if x != 0]
    if not nonzero:
        return [mp.mpf(0)] * n
    e = floor_log2(max(nonzero))
    with mp.workdps(dps):
        size = n + 4
        m = mp.zeros(size, size)
        for i in range(n):
            for j in range(n):
                m[i, j] = mp.mpf(h) * mp.mpf(a[i][j])
            for c in range(4):
                m[i, n + c] = mpf_exact(w[4 - c][i] / Fraction(2) ** e)
        for c in range(3):
            m[n + c, n + c + 1] = 1
        v = mp.matrix(size, 1)
        for i in range(n):
            v[i] = mpf_exact(w[0][i] / Fraction(2) ** e)
        v[size - 1] = 1
        y = mp.expm(m) * v
        return [mp.ldexp(y[i], e) for i in range(n)]


def norm2(xs) -> mp.mpf:
    with mp.workdps(DPS + 10):
        return mp.sqrt(mp.fsum(x * x for x in xs))


def references(h, a, w, reference) -> tuple[list, list, mp.mpf]:
    r50 = reference(h, a, w, DPS)
    r70 = reference(h, a, w, DPS_CHECK)
    with mp.workdps(DPS_CHECK + 10):
        diff = mp.sqrt(mp.fsum((x - y) ** 2 for x, y in zip(r50, r70)))
    return r50, r70, diff


# ------------------------------------------------------------------------------------------- git
def git(*args: str) -> str:
    root = Path(__file__).resolve().parents[1]
    out = subprocess.run(["git", *args], cwd=root, capture_output=True)
    if out.returncode != 0:
        raise evidence.ValidationError([f"git {' '.join(args)} failed: {out.stderr.decode(errors='replace')}"])
    return out.stdout.decode()


def checker_blob_at(commit: str) -> bytes:
    root = Path(__file__).resolve().parents[1]
    out = subprocess.run(["git", "show", f"{commit}:{CHECKER_PATH}"], cwd=root, capture_output=True)
    if out.returncode != 0:
        raise evidence.ValidationError([f"provenance: the checker is not in source commit {commit}"])
    return out.stdout


def own_tree_status() -> str:
    return git("status", "--porcelain", "--untracked-files=no")


def own_head() -> str:
    return git("rev-parse", "HEAD").strip()


# ------------------------------------------------------------------------------------------ evidence
def _hex(c, path: str, value) -> float | None:
    try:
        return evidence.decode_hex64(value)
    except evidence.ValidationError as exc:
        c.fail(path, exc.reasons[0])
        return None


def _hex_or_null(c, path: str, value) -> float | None:
    """Null (a non-finite value) or canonical finite hex (else INVALID). Returns math.nan for null."""
    if value is None:
        return math.nan
    return _hex(c, path, value)


def _vector(c, path: str, value, n: int, nullable: bool = False) -> list | None:
    if not c.list_(path, value):
        return None
    if len(value) != n:
        c.fail(path, f"length {len(value)} != {n}")
        return None
    out = [(_hex_or_null if nullable else _hex)(c, f"{path}[{i}]", x) for i, x in enumerate(value)]
    return None if any(x is None for x in out) else out


def _int(c, path: str, value, minimum: int | None = None) -> bool:
    if type(value) is not int:
        c.fail(path, f"expected int, got {type(value).__name__}")
        return False
    if minimum is not None and value < minimum:
        c.fail(path, f"{value} < {minimum}")
        return False
    return True


def _bool(c, path: str, value) -> bool:
    if type(value) is not bool:
        c.fail(path, f"expected bool, got {type(value).__name__}")
        return False
    return True


def _opt_str(c, path: str, value) -> None:
    if value is not None and not isinstance(value, str):
        c.fail(path, f"expected string or null, got {type(value).__name__}")


def _enum(c, path: str, value, allowed) -> bool:
    if value not in allowed or (value is not None and not isinstance(value, str)):
        c.fail(path, f"{value!r} not in {list(allowed)}")
        return False
    return True


def _inner(c, path: str, rec, n: int) -> dict | None:
    if not c.keys(path, rec, INNER_KEYS):
        return None
    c.equal(f"{path}.schema", rec["schema"], INNER_SCHEMA)
    _enum(c, f"{path}.status", rec["status"], STATUSES)
    _opt_str(c, f"{path}.unbounded_reason", rec["unbounded_reason"])
    _enum(c, f"{path}.dominant", rec["dominant"], INNER_DOMINANT)
    _enum(c, f"{path}.chosen_metric", rec["chosen_metric"], METRICS)
    out = {k: _hex_or_null(c, f"{path}.{k}", rec[k]) for k in INNER_HEX_NULLABLE}
    out["stepped_candidate"] = _vector(c, f"{path}.stepped_candidate", rec["stepped_candidate"], n + 4, True)
    if c.list_(f"{path}.attempts", rec["attempts"]):
        if len(rec["attempts"]) != 2:
            c.fail(f"{path}.attempts", "expected two attempts")
        for i, att in enumerate(rec["attempts"]):
            p = f"{path}.attempts[{i}]"
            if c.keys(p, att, ATTEMPT_KEYS):
                _enum(c, f"{p}.metric", att["metric"], METRICS[:2])
                if att["steps"] is not None:
                    _int(c, f"{p}.steps", att["steps"], 0)
                _bool(c, f"{p}.bounded", att["bounded"])
                _hex_or_null(c, f"{p}.error_upper", att["error_upper"])
    return out


def _certificate(c, path: str, rec, n: int) -> None:
    if not c.keys(path, rec, CERT_KEYS):
        return
    c.equal(f"{path}.schema", rec["schema"], NORMALIZED_SCHEMA)
    _enum(c, f"{path}.status", rec["status"], STATUSES)
    _opt_str(c, f"{path}.unbounded_reason", rec["unbounded_reason"])
    _int(c, f"{path}.exponent", rec["exponent"])
    if c.list_(f"{path}.normalized_vectors", rec["normalized_vectors"]):
        if len(rec["normalized_vectors"]) != TERMS:
            c.fail(f"{path}.normalized_vectors", "expected five vectors")
        for k, vk in enumerate(rec["normalized_vectors"]):
            _vector(c, f"{path}.normalized_vectors[{k}]", vk, n)
    if c.list_(f"{path}.normalization_inexact", rec["normalization_inexact"]):
        if len(rec["normalization_inexact"]) != TERMS:
            c.fail(f"{path}.normalization_inexact", "expected five counts")
        for k, x in enumerate(rec["normalization_inexact"]):
            _int(c, f"{path}.normalization_inexact[{k}]", x, 0)
    _vector(c, f"{path}.normalized_candidate", rec["normalized_candidate"], n)
    _enum(c, f"{path}.normalized_source", rec["normalized_source"], SOURCES)
    _opt_str(c, f"{path}.normalized_taylor_rejection", rec["normalized_taylor_rejection"])
    _inner(c, f"{path}.inner", rec["inner"], n)
    _vector(c, f"{path}.candidate", rec["candidate"], n, True)
    _int(c, f"{path}.rescale_inexact", rec["rescale_inexact"], 0)
    for k in CERT_HEX_NULLABLE:
        _hex_or_null(c, f"{path}.{k}", rec[k])
    _enum(c, f"{path}.dominant", rec["dominant"], DOMINANT)
    if rec["caller_candidate"] is not None:
        _vector(c, f"{path}.caller_candidate", rec["caller_candidate"], n)
    for k in ("caller_distance", "caller_bound"):
        _hex_or_null(c, f"{path}.{k}", rec[k])
    if c.keys(f"{path}.work", rec["work"], WORK_KEYS):
        for k in sorted(WORK_KEYS):
            _int(c, f"{path}.work.{k}", rec["work"][k], 0)


def _case(c, path: str, case, reg: dict) -> None:
    n = reg["n"]
    for key in ("family", "variant", "w_exponents", "matrix_index", "seed", "symmetric", "n", "h_power_of_two"):
        c.equal(f"{path}.{key}", case[key], reg[key])
    if type(case["target_hA_one_norm"]) is not float or case["target_hA_one_norm"] != reg["target_hA_one_norm"]:
        c.fail(f"{path}.target_hA_one_norm", f"!= registered {reg['target_hA_one_norm']}")
    if case["h"] != hx(reg["h"]):
        c.fail(f"{path}.h", f"!= registered {hx(reg['h'])}")
    for key, rows in (("a", n), ("base_w", TERMS), ("w", TERMS)):
        value = case[key]
        if not c.list_(f"{path}.{key}", value) or len(value) != rows:
            c.fail(f"{path}.{key}", f"expected {rows} rows")
            continue
        for i, row in enumerate(value):
            got = _vector(c, f"{path}.{key}[{i}]", row, n)
            if got is not None and [hx(x) for x in got] != [hx(x) for x in reg[key][i]]:
                c.fail(f"{path}.{key}[{i}]", "bits differ from the regenerated registered input")
    t = case["taylor_original"]
    if c.keys(f"{path}.taylor_original", t, TAYLOR_KEYS) and _bool(c, f"{path}.taylor_original.accepted",
                                                                     t["accepted"]):
        if t["accepted"]:
            _vector(c, f"{path}.taylor_original.fused", t["fused"], n)
            if t["rejection"] is not None:
                c.fail(f"{path}.taylor_original.rejection", "present although accepted")
        else:
            if t["fused"] is not None:
                c.fail(f"{path}.taylor_original.fused", "present although rejected")
            if not isinstance(t["rejection"], str):
                c.fail(f"{path}.taylor_original.rejection", "missing")
    o = case["normalized"]
    if c.keys(f"{path}.normalized", o, OUTCOME_KEYS) and _enum(c, f"{path}.normalized.outcome", o["outcome"],
                                                                ("certificate", "refused", "error")):
        if o["outcome"] == "certificate":
            if o["error"] is not None:
                c.fail(f"{path}.normalized.error", "present with a certificate")
            _certificate(c, f"{path}.normalized.certificate", o["certificate"], n)
            cert = o["certificate"]
            if isinstance(cert, dict) and "caller_candidate" in cert and isinstance(t, dict):
                expected = t.get("fused") if t.get("accepted") is True else None
                if cert["caller_candidate"] != expected:
                    c.fail(f"{path}.normalized.certificate.caller_candidate",
                           "is not the Taylor output on the original w (or present without it)")
        else:
            if o["certificate"] is not None:
                c.fail(f"{path}.normalized.certificate", "present with a refusal or error")
            if not isinstance(o["error"], str):
                c.fail(f"{path}.normalized.error", "missing")
            elif (o["outcome"] == "refused") != (RANGE_CODE in o["error"]):
                c.fail(f"{path}.normalized.outcome", "does not match the error's code")
    b = case["baseline"]
    if c.keys(f"{path}.baseline", b, BASELINE_KEYS) and _enum(c, f"{path}.baseline.outcome", b["outcome"],
                                                               ("certificate", "error")):
        _enum(c, f"{path}.baseline.candidate_source", b["candidate_source"],
              ("taylor_phi_action", "stepped_top", "zeros"))
        _vector(c, f"{path}.baseline.candidate", b["candidate"], n)
        _opt_str(c, f"{path}.baseline.error", b["error"])
        if b["outcome"] == "certificate":
            _inner(c, f"{path}.baseline.certificate", b["certificate"], n)
        elif b["certificate"] is not None:
            c.fail(f"{path}.baseline.certificate", "present with an error")


def validate(cases_path: Path, blob_at=None, tree_status=None) -> evidence.Evidence:
    """Every INVALID condition that needs no reference; returns the validated documents (read once)."""
    blob_at = blob_at or checker_blob_at
    tree_status = tree_status or own_tree_status
    docs, digests, paths = evidence._load_all([("cases", cases_path, None)])
    doc = docs["cases"]
    c = evidence._Checks()
    if not c.keys("cases", doc, TOP_KEYS):
        c.done()
    c.equal("cases.schema", doc["schema"], CASES_SCHEMA)
    c.equal("cases.node", doc["node"], NODE)
    c.equal("cases.taylor_budget", doc["taylor_budget"], hx(TAYLOR_BUDGET))
    c.equal("cases.normalized_taylor_budget", doc["normalized_taylor_budget"], hx(TAYLOR_BUDGET))
    c.equal("cases.seed_base", doc["seed_base"], SEED_BASE)
    c.equal("cases.table_seed", doc["table_seed"], TABLE_SEED)
    commit = doc["source_commit"]
    if not (isinstance(commit, str) and len(commit) == 40 and all(ch in "0123456789abcdef" for ch in commit)):
        c.fail("cases.source_commit", "not a 40-digit commit id")
        commit = None
    if not isinstance(doc["tree_status"], str) or doc["tree_status"] != "":
        c.fail("cases.tree_status", f"the export tree was dirty: {str(doc['tree_status'])[:200]!r}")
    status = tree_status()
    if status != "":
        c.fail("checker.tree_status", f"the checker tree is dirty: {status[:200]!r}")
    if commit is not None:
        try:
            if blob_at(commit) != Path(__file__).read_bytes():
                c.fail("provenance", f"the checker blob at {commit} differs from the checker run")
        except evidence.ValidationError as exc:
            c.fail("provenance", exc.reasons[0])
    registered = {r["label"]: r for r in registered_cases()}
    if c.list_("cases.cases", doc["cases"]):
        labels = [x.get("label") if isinstance(x, dict) else None for x in doc["cases"]]
        if len(labels) != len(set(map(str, labels))):
            c.fail("cases.cases", "duplicate labels in the raw list")
        if set(map(str, labels)) != set(registered) or len(labels) != len(registered):
            c.fail("cases.cases", f"case set is not the {len(registered)} registered cases "
                                  f"(missing {len(set(registered) - set(map(str, labels)))}, "
                                  f"unexpected {len(set(map(str, labels)) - set(registered))})")
        for i, case in enumerate(doc["cases"]):
            p = f"cases.cases[{i}]"
            if c.keys(p, case, CASE_KEYS) and case["label"] in registered:
                _case(c, p, case, registered[case["label"]])
    inputs = registered_table_inputs()
    if c.list_("cases.scaling_table", doc["scaling_table"]):
        if len(doc["scaling_table"]) != TABLE_ROWS or len(inputs) != TABLE_ROWS:
            c.fail("cases.scaling_table", f"expected {TABLE_ROWS} rows")
        for i, row in enumerate(doc["scaling_table"]):
            p = f"cases.scaling_table[{i}]"
            if not c.keys(p, row, TABLE_KEYS):
                continue
            x = _hex(c, f"{p}.x", row["x"])
            if _int(c, f"{p}.e", row["e"]) and x is not None and i < len(inputs):
                if (hx(x), row["e"]) != (hx(inputs[i][0]), inputs[i][1]):
                    c.fail(p, "input is not the registered seeded table input")
            for k in ("nearest", "up", "down"):
                _hex_or_null(c, f"{p}.{k}", row[k])
            _bool(c, f"{p}.exact", row["exact"])
            _bool(c, f"{p}.overflow", row["overflow"])
            _int(c, f"{p}.stages", row["stages"], 0)
            _int(c, f"{p}.largest_stage", row["largest_stage"], 0)
    c.done()
    return evidence.Evidence("PY04", docs, digests, paths)


# --------------------------------------------------------------------------------------------- gate
def f(x) -> float | None:
    """Decoded value of a validated hex-or-null field (None for null)."""
    return None if x is None else struct.unpack(">d", bytes.fromhex(x))[0]


def fr(x) -> Fraction | None:
    v = f(x)
    return None if v is None else Fraction(v)


def sci(x) -> str | None:
    if x is None:
        return None
    if isinstance(x, Fraction):
        with mp.workdps(30):
            return mp.nstr(mpf_exact(x) if is_pow2(x.denominator) else mp.mpf(x.numerator) / x.denominator, 6)
    if isinstance(x, float):
        return repr(x)
    return mp.nstr(x, 6)


def distance(u: list[float], ref: list) -> mp.mpf:
    with mp.workdps(DPS + 10):
        return norm2([mp.mpf(ui) - ri for ui, ri in zip(u, ref)])


def table_row_check(row: dict) -> dict:
    x = f(row["x"])
    e = row["e"]
    exact = Fraction(x) * Fraction(2) ** e
    overflow = abs(exact) >= Fraction(2) ** 1024
    nearest_v, up, down = f(row["nearest"]), f(row["up"]), f(row["down"])
    if overflow:
        ok = (nearest_v is None and up is None and down is None and row["exact"] is False
              and row["overflow"] is True)
        detail = "overflow refused"
    else:
        expected = nearest(exact)
        ok = (nearest_v is not None and up is not None and down is not None
              and hx(nearest_v) == hx(expected)
              and Fraction(down) <= exact <= Fraction(up)
              and row["exact"] == (Fraction(expected) == exact)
              and row["overflow"] is False)
        detail = "ok" if ok else f"nearest {nearest_v!r} expected {expected!r}, down {down!r}, up {up!r}"
    tight = (overflow or (up is not None and down is not None and nearest_v is not None
                          and up in (nearest_v, math.nextafter(nearest_v, math.inf))
                          and down in (nearest_v, math.nextafter(nearest_v, -math.inf))))
    return {"x": row["x"], "e": e, "holds": bool(ok), "detail": detail, "outward_by_at_most_one_step": bool(tight),
            "stages": row["stages"], "largest_stage": row["largest_stage"]}


def evaluate(validated: evidence.Evidence, reference=fused_reference) -> dict:
    doc = validated.docs["cases"]
    registered = {r["label"]: r for r in registered_cases()}
    rows = []
    invalid = []
    item1_violations = []
    item3_failures = []
    item5_failures = []
    item6_failures = []
    by_label = {}
    for case in doc["cases"]:
        reg = registered[case["label"]]
        n = reg["n"]
        h = reg["h"]
        a = reg["a"]
        w_frac = [[Fraction(x) for x in wk] for wk in reg["w"]]
        r50, r70, diff = references(h, a, w_frac, reference)
        f_norm = norm2(r50)
        with mp.workdps(DPS + 10):
            tolerance = mp.mpf("1e-45") * f_norm + mp.ldexp(1, -1100)
            budget_abs = 16 * mp.sqrt(n) * mp.ldexp(1, -1074)
            budget = mp.mpf("1e-10") * f_norm + budget_abs
            relative_budget = mp.mpf("1e-10") * f_norm
        if diff > tolerance:
            invalid.append(f"{case['label']}: 50/70-digit references differ by {mp.nstr(diff, 3)} "
                           f"> {mp.nstr(tolerance, 3)}")
        row = {"label": case["label"], "family": case["family"], "variant": case["variant"], "n": n,
               "symmetric": case["symmetric"], "h_power_of_two": case["h_power_of_two"],
               "target_hA_one_norm": case["target_hA_one_norm"],
               "F_norm": mp.nstr(f_norm, 8), "reference_difference": mp.nstr(diff, 3),
               "mixed_budget": mp.nstr(budget, 6),
               "taylor_original_accepted": case["taylor_original"]["accepted"],
               "outcome": case["normalized"]["outcome"], "error": case["normalized"]["error"]}
        o = case["normalized"]
        cert = o["certificate"]
        if o["outcome"] == "certificate":
            status = cert["status"]
            row.update({"status": status, "exponent": cert["exponent"], "dominant": cert["dominant"],
                        "normalized_source": cert["normalized_source"],
                        "inner_dominant": cert["inner"]["dominant"], "work": cert["work"],
                        "bound": sci(f(cert["bound"])), "scaled_certificate": sci(f(cert["scaled_certificate"])),
                        "R": sci(f(cert["rescale_rounding"])), "L": sci(f(cert["normalization_rounding"])),
                        "E": sci(f(cert["inner"]["bound"]))})
            e = cert["exponent"]
            sigma = Fraction(2) ** e
            v = [[f(x) for x in vk] for vk in cert["normalized_vectors"]]
            z = [f(x) for x in cert["normalized_candidate"]]
            fhat = [f(x) for x in cert["candidate"]]
            bound = f(cert["bound"])
            # Consistency with the definitions (reported).
            largest = max((abs(x) for wk in w_frac for x in wk), default=Fraction(0))
            e_expected = floor_log2(largest) if largest != 0 else 0
            v_nearest = all(hx(v[k][i]) == hx(nearest(w_frac[k][i] / sigma)) for k in range(TERMS) for i in range(n))
            fhat_nearest = all(fh is not None and hx(fh) == hx(nearest(Fraction(zi) * sigma))
                               for fh, zi in zip(fhat, z))
            row["consistency"] = {"exponent_is_floor_log2_max_w": e == e_expected,
                                  "v_is_nearest": v_nearest, "candidate_is_nearest": fhat_nearest}
            if status == "bounded":
                # Item 3: finiteness, exactness iff, enclosure of R and L.
                finite = all(x is not None for x in fhat) and all(
                    cert[k] is not None for k in ("bound", "scaled_certificate", "rescale_rounding",
                                                  "normalization_rounding"))
                if cert["caller_candidate"] is not None:
                    finite &= cert["caller_bound"] is not None and cert["caller_distance"] is not None
                if not finite:
                    item3_failures.append(f"{case['label']}: a Bounded certificate has a non-finite value")
                inexact_v = [sum(Fraction(v[k][i]) != w_frac[k][i] / sigma for i in range(n)) for k in range(TERMS)]
                v_exact = sum(inexact_v) == 0
                L = f(cert["normalization_rounding"])
                R = f(cert["rescale_rounding"])
                l_check = {"v_exact": v_exact, "inexact_per_k": inexact_v,
                           "counts_match": inexact_v == cert["normalization_inexact"]}
                if L is None:
                    l_ok = False
                elif v_exact:
                    l_ok = L == 0.0
                else:
                    d = [[w_frac[k][i] / sigma - Fraction(v[k][i]) for i in range(n)] for k in range(TERMS)]
                    l50, l70, ldiff = references(h, a, d, reference)
                    with mp.workdps(DPS + 10):
                        l_actual = norm2([mp.ldexp(x, e) for x in l50])
                        l_unc = mp.ldexp(ldiff, e)
                        l_tol = mp.mpf("1e-45") * norm2(l50) + mp.ldexp(1, -1100)
                    if ldiff > l_tol:
                        invalid.append(f"{case['label']}: L references differ by {mp.nstr(ldiff, 3)}")
                    l_ok = L > 0 and mp.mpf(L) >= l_actual + l_unc
                    l_check.update({"L_actual": mp.nstr(l_actual, 6), "L_over_actual":
                                    mp.nstr(mp.mpf(L) / l_actual, 6) if l_actual > 0 else None})
                if fhat_ok := all(x is not None for x in fhat):
                    r_err = [Fraction(fh) - Fraction(zi) * sigma for fh, zi in zip(fhat, z)]
                    r_inexact = sum(x != 0 for x in r_err)
                    r_sq = sum(x * x for x in r_err)
                else:
                    r_inexact, r_sq = None, None
                if R is None or not fhat_ok:
                    r_ok = False
                elif r_inexact == 0:
                    r_ok = R == 0.0
                else:
                    r_ok = R > 0 and Fraction(R) ** 2 >= r_sq
                r_check = {"inexact": r_inexact, "counts_match": r_inexact == cert["rescale_inexact"],
                           "R_over_actual": mp.nstr(mp.mpf(R) / mp_sqrt_fraction(r_sq), 6)
                           if r_sq and R is not None else None}
                row["accounting"] = {"L": l_check, "L_holds": bool(l_ok), "R": r_check, "R_holds": bool(r_ok)}
                if not l_ok:
                    item3_failures.append(f"{case['label']}: L accounting fails")
                if not r_ok:
                    item3_failures.append(f"{case['label']}: R accounting fails")
                # Item 1: enclosure for both candidates.
                if bound is None or not fhat_ok:
                    item1_violations.append(f"{case['label']}: non-finite bound or candidate")
                    enc = False
                    error = None
                else:
                    error = distance(fhat, r50)
                    enc = mp.mpf(bound) >= error + diff
                    if not enc:
                        item1_violations.append(f"{case['label']}: bound {bound!r} < error {mp.nstr(error, 6)} "
                                                f"+ {mp.nstr(diff, 3)}")
                row.update({"error": mp.nstr(error, 6) if error is not None else None, "encloses": bool(enc),
                            "bound_over_error": mp.nstr(mp.mpf(bound) / error, 6)
                            if error is not None and error > 0 else None,
                            "relative_bound": mp.nstr(mp.mpf(bound) / f_norm, 6)
                            if bound is not None and f_norm > 0 else None})
                caller = None
                if cert["caller_candidate"] is not None:
                    u = [f(x) for x in cert["caller_candidate"]]
                    cb = f(cert["caller_bound"])
                    u_error = distance(u, r50)
                    c_enc = cb is not None and mp.mpf(cb) >= u_error + diff
                    if not c_enc:
                        item1_violations.append(f"{case['label']}: caller bound {cb!r} < error "
                                                f"{mp.nstr(u_error, 6)} + {mp.nstr(diff, 3)}")
                    caller = {"bound": sci(cb), "error": mp.nstr(u_error, 6), "encloses": bool(c_enc),
                              "useful_mixed": bool(cb is not None and mp.mpf(cb) <= budget),
                              "useful_relative": bool(cb is not None and mp.mpf(cb) <= relative_budget)}
                row["caller"] = caller
                useful = bool(bound is not None and mp.mpf(bound) <= budget)
                useful_rel = bool(bound is not None and mp.mpf(bound) <= relative_budget)
                row.update({"useful_mixed": useful, "useful_relative": useful_rel})
            else:
                row.update({"useful_mixed": False, "useful_relative": False, "encloses": None})
        else:
            row.update({"status": None, "useful_mixed": False, "useful_relative": False, "encloses": None})
        # Item 5 / 6.
        fam = case["family"]
        if fam in USEFUL_FAMILIES and not (row.get("status") == "bounded" and row["useful_mixed"]):
            item5_failures.append(f"{case['label']}: {row.get('status') or o['outcome']}, useful "
                                  f"{row['useful_mixed']}")
        if fam == "S5":
            valid_bounded = row.get("status") == "bounded" and row.get("encloses") is True and (
                row.get("caller") is None or row["caller"]["encloses"])
            refused = o["outcome"] == "refused" and RANGE_CODE in (o["error"] or "")
            row["S5_outcome"] = "bounded-valid" if valid_bounded else ("refused" if refused else "other")
            if not (valid_bounded or refused):
                item6_failures.append(f"{case['label']}: neither Bounded and valid nor refused")
        # Baseline (reported).
        b = case["baseline"]
        base = {"outcome": b["outcome"], "candidate_source": b["candidate_source"]}
        if b["outcome"] == "certificate":
            bb = f(b["certificate"]["bound"])
            u = [f(x) for x in b["candidate"]]
            b_error = distance(u, r50)
            base.update({"status": b["certificate"]["status"], "bound": sci(bb),
                         "unbounded_reason": b["certificate"]["unbounded_reason"],
                         "error": mp.nstr(b_error, 6),
                         "encloses": bool(mp.mpf(bb) >= b_error + diff) if bb is not None else None,
                         "useful_mixed": bool(bb is not None and mp.mpf(bb) <= budget),
                         "useful_relative": bool(bb is not None and mp.mpf(bb) <= relative_budget)})
        else:
            base.update({"status": None, "useful_mixed": False, "useful_relative": False, "error_text": b["error"]})
        row["baseline"] = base
        rows.append(row)
        by_label[case["label"]] = (case, row)
        print(f"{case['label']}: {row.get('status') or o['outcome']} bound {row.get('bound')} "
              f"error {row.get('error')} ||F|| {row['F_norm']} useful {row['useful_mixed']}", flush=True)
    if invalid:
        raise evidence.ValidationError(invalid)

    # Item 2: homogeneity of the S7 triples.
    item2_failures = []
    triples = []
    for label, (case, _) in by_label.items():
        if case["variant"] != "S7-s0":
            continue
        stem = label[: -len("S7-s0")]
        members = [by_label.get(stem + v) for v in ("S7-s0", "S7-p600", "S7-m600")]
        entry = {"stem": stem.rstrip("-"), "internals_identical": False, "all_exact": None, "bound_scaled": None}
        if any(m is None or m[0]["normalized"]["outcome"] != "certificate" for m in members):
            item2_failures.append(f"{stem}: not every S7 case is a certificate")
            triples.append(entry)
            continue
        certs = [m[0]["normalized"]["certificate"] for m in members]

        def internals(cert):
            inner = cert["inner"]
            return (cert["normalized_vectors"], cert["normalized_candidate"], inner["bound"], inner["perturbation"],
                    inner["stepped_error"], inner["candidate_distance"])

        identical = all(internals(x) == internals(certs[0]) for x in certs)
        entry["internals_identical"] = identical
        if not identical:
            item2_failures.append(f"{stem}: normalized internals differ")

        def exact(cert):
            big_e = fr(cert["inner"]["bound"])
            if big_e is None or cert["rescale_rounding"] != hx(0.0) or cert["normalization_rounding"] != hx(0.0):
                return False
            scaled = big_e * Fraction(2) ** cert["exponent"]
            return scaled == 0 or MIN_NORMAL <= scaled <= MAX

        all_exact = all(exact(x) for x in certs)
        entry["all_exact"] = all_exact
        if all_exact:
            b1 = fr(certs[0]["bound"])
            ok = b1 is not None and all(
                fr(x["bound"]) is not None and fr(x["bound"]) == b1 * Fraction(2) ** s
                for x, s in zip(certs, (0, 600, -600)))
            entry["bound_scaled"] = ok
            if not ok:
                item2_failures.append(f"{stem}: bound_s != s bound_1 although every rescaling is exact")
        triples.append(entry)

    # Item 4: directed scaling table.
    table = [table_row_check(r) for r in doc["scaling_table"]]
    item4_failures = [f"x {t['x']} e {t['e']}: {t['detail']}" for t in table if not t["holds"]]

    items = {
        "1_enclosure": not item1_violations,
        "2_homogeneity": not item2_failures and len(triples) == 16,
        "3_normalization_accounting": not item3_failures,
        "4_directed_scaling": not item4_failures,
        "5_usefulness_new_extremes": not item5_failures,
        "6_overflow_boundary": not item6_failures,
    }
    verdict = "PASS" if all(items.values()) else "FAIL"
    return {"verdict": verdict, "items": items,
            "failures": {"1": item1_violations, "2": item2_failures, "3": item3_failures, "4": item4_failures,
                         "5": item5_failures, "6": item6_failures},
            "rows": rows, "triples": triples, "table": table,
            "reported": summarize(rows, triples, table)}


def mp_sqrt_fraction(x: Fraction):
    with mp.workdps(DPS):
        return mp.sqrt(mp.mpf(x.numerator) / mp.mpf(x.denominator))


def summarize(rows: list[dict], triples: list[dict], table: list[dict]) -> dict:
    fams = {}
    for r in rows:
        fam = r["variant"] if r["family"] == "S7" else r["family"]
        e = fams.setdefault(fam, {"cases": 0, "bounded": 0, "unbounded": 0, "refused": 0, "error": 0,
                                  "useful_mixed": 0, "useful_relative": 0, "taylor_original_accepted": 0,
                                  "caller_useful_mixed": 0, "caller_useful_relative": 0,
                                  "baseline_bounded": 0, "baseline_useful_mixed": 0, "baseline_useful_relative": 0,
                                  "baseline_encloses_when_bounded": 0, "dominant": {}, "inner_dominant": {},
                                  "relative_bound_max": None, "bound_over_error_min": None,
                                  "entries_scaled_max": 0, "stages_max": 0, "L_nonzero": 0, "R_nonzero": 0})
        e["cases"] += 1
        if r["outcome"] == "certificate":
            e[r["status"]] += 1
            e["dominant"][r["dominant"]] = e["dominant"].get(r["dominant"], 0) + 1
            e["inner_dominant"][r["inner_dominant"]] = e["inner_dominant"].get(r["inner_dominant"], 0) + 1
            e["entries_scaled_max"] = max(e["entries_scaled_max"], r["work"]["entries_scaled"])
            e["stages_max"] = max(e["stages_max"], r["work"]["stages"])
            if r["L"] not in (None, "0.0"):
                e["L_nonzero"] += 1
            if r["R"] not in (None, "0.0"):
                e["R_nonzero"] += 1
            if r.get("relative_bound") is not None:
                cur = e["relative_bound_max"]
                if cur is None or mp.mpf(r["relative_bound"]) > mp.mpf(cur):
                    e["relative_bound_max"] = r["relative_bound"]
            if r.get("bound_over_error") is not None:
                cur = e["bound_over_error_min"]
                if cur is None or mp.mpf(r["bound_over_error"]) < mp.mpf(cur):
                    e["bound_over_error_min"] = r["bound_over_error"]
        else:
            e[r["outcome"]] += 1
        e["useful_mixed"] += r["useful_mixed"]
        e["useful_relative"] += r["useful_relative"]
        e["taylor_original_accepted"] += bool(r["taylor_original_accepted"])
        if r.get("caller"):
            e["caller_useful_mixed"] += r["caller"]["useful_mixed"]
            e["caller_useful_relative"] += r["caller"]["useful_relative"]
        b = r["baseline"]
        e["baseline_bounded"] += b.get("status") == "bounded"
        e["baseline_useful_mixed"] += b["useful_mixed"]
        e["baseline_useful_relative"] += b["useful_relative"]
        e["baseline_encloses_when_bounded"] += b.get("status") == "bounded" and b.get("encloses") is True
    s5 = [r for r in rows if r["family"] == "S5"]
    s5_bounded = sum(r.get("S5_outcome") == "bounded-valid" for r in s5)
    predictions = {
        "items_1_to_4_and_6_hold": None,  # filled by main from the gate
        "S1_S4_S6_useful": all(r["useful_mixed"] for r in rows if r["family"] in USEFUL_FAMILIES),
        "S5_mostly_bounded": s5_bounded * 2 > len(s5),
        "S5_bounded_count": s5_bounded,
        "baseline_unbounded_in_S4_S5": all(r["baseline"].get("status") != "bounded"
                                           for r in rows if r["family"] in ("S4", "S5")),
        "baseline_not_useful_in_S1_S3": all(not r["baseline"]["useful_mixed"]
                                            for r in rows if r["family"] in ("S1", "S2", "S3")),
    }
    return {"by_family": fams, "triples_all_exact": sum(bool(t["all_exact"]) for t in triples),
            "triples": len(triples),
            "table_outward_by_at_most_one_step": all(t["outward_by_at_most_one_step"] for t in table),
            "table_largest_stage_max": max((t["largest_stage"] for t in table), default=0),
            "table_largest_stage_within_limit": all(t["largest_stage"] <= STAGE_LIMIT for t in table),
            "consistency_all": all(all(r["consistency"].values()) for r in rows if "consistency" in r),
            "predictions": predictions}


def main(argv: list[str] | None = None, blob_at=None, tree_status=None, head=None,
         reference=fused_reference) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--cases", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args(argv)
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    inputs = {"cases": args.cases}
    try:
        validated = validate(args.cases, blob_at, tree_status)
        result = evaluate(validated, reference)
    except evidence.ValidationError as exc:
        return evidence.emit_invalid(args.output, SCHEMA, exc, inputs)
    doc = validated.docs["cases"]
    result["reported"]["predictions"]["items_1_to_4_and_6_hold"] = all(
        result["items"][k] for k in ("1_enclosure", "2_homogeneity", "3_normalization_accounting",
                                     "4_directed_scaling", "6_overflow_boundary"))
    report = {
        "schema": SCHEMA,
        "evidence_schema": evidence.VERSION,
        "node": NODE,
        "verdict": result["verdict"],
        "gate": result["items"],
        "gate_failures": result["failures"],
        "cases_source_commit": doc["source_commit"],
        "cases_tree_status": doc["tree_status"],
        "checker_head": (head or own_head)(),
        "checker_tree_status": (tree_status or own_tree_status)(),
        "inputs": {role: {"path": str(p), "sha256": validated.sha256[role]} for role, p in validated.paths.items()},
        "environment": {"python": sys.version.split()[0], "mpmath": mp.__version__, "dps": DPS,
                        "dps_check": DPS_CHECK},
        "rows": result["rows"],
        "triples": result["triples"],
        "table": result["table"],
        "reported": result["reported"],
        "claim_scope": "Opt-in normalized fused phi certificate on 144 registered cases at fresh scales: validity "
                       "against a 50-digit (70-digit checked) mpmath reference and usefulness under the declared "
                       "mixed absolute-relative budget. No relative-only claim, no PP11 rerun, no default "
                       "promotion, no wall-time claim.",
    }
    args.output.write_text(json.dumps(report, indent=1, sort_keys=True, allow_nan=False) + "\n")
    for k, v in result["items"].items():
        print(f"{k}: {'PASS' if v else 'FAIL'}")
    print(f"verdict: {result['verdict']}")
    return 0 if result["verdict"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
