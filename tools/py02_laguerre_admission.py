#!/usr/bin/env python3
"""Stage-1 admission of research node research/py02_laguerre_adjoint_bound_20261011 (PY02; see its PREREGISTRATION.md).

Inputs:
- ``--py01``: research/py01_laguerre_component_export_20261011/RESULTS.json (PY01's recorded result, immutable,
  SHA-256 pinned). Items A1 and A2 read this file and nothing else.
- ``--plans``: PLANS.json, written once by ``plan_only`` of crates/rodas5p-core/tests/laguerre_adjoint_holdout.rs
  (the local plan-only helper ``plan_only_comparison`` on the 32 registered stage-2 cases). Item A3 reads it.
- ``--output``: ADMISSION.json (immutable: an existing file is never overwritten).

Exit codes: INVALID exit 2; ADMIT or ABSTAIN exit 0 (stage 1 has no FAIL outcome; exit 1 is never used).

Evidence (INVALID, exit 2) before any item:
- AS03 rules (tools/evidence_schema_v2.py primitives): strict JSON for both inputs, the PY01 SHA-256 pin, exact key
  sets of the plans document, constants, cases, plans and parity records, canonical binary64 hex (16 lowercase
  digits, finite), typed numbers (bool never int, int never float), unique labels.
- PY01 structure: the six registered gated cases are present in ``reported.cases`` with ``gated`` true and the
  fields A1 and A2 read (decimal strings that parse, a positive admission budget).
- Case set: exactly the 32 registered stage-2 labels, in the registered order; (n, rho, h) disjoint from R-NEXT-04
  ({6, 12, 20} x {1, 50, 400} x {1e-3, 1e-2, 0.1}), PP08 ({5, 16} x {2, 30, 200} x {1e-3, 0.03, 0.2}) and PY03
  ({8, 24} x {4, 64} x {0.005, 0.05, 0.12}).
- Binding of the inputs: every A equals the R-NEXT-04 generator for its seed bit for bit (seeds 9001.. in grid
  order, one per (n, rho, h)); every w_k equals the registered splitmix stream (distinct: seed + 9 + 77 k; same
  vector: s_k splitmix(seed + 5), s = (1, -0.5, 0.25, 2, -1)); the enclosure is [0, 1.2 rho (1 + 1e-12)] and
  verified; the constants are the registered ones.
- Budgets: total_budget = factor max(norm_f, 1) and truncation_budget = total_budget / 4, bit for bit from the
  recorded norm_f; norm_f agrees with the 50-digit ||F|| (F = sum_k phi_k(h A) w_k by an mpmath eigendecomposition
  of the binary A, checked at 70 digits; INVALID if the two references differ by more than 1e-40 ||F||) so that
  max(||F||, 1) differs by at most 1e-10 relative.
- Plans: recurrence branch, normalization shift 0, core budget = truncation budget bit for bit, width 5 (distinct)
  or 1 (same vector), the weight factor in [W, W (1 + 1e-12)] for the exact W = sum_k |s_k| ||w_k|| / k!; for each
  basis either a plan (degree, vector products = degree x width, truncation bound <= core budget, a Laguerre scale
  in {1, 2, 4, 8, 16} for Laguerre and none for Chebyshev, tail evaluations >= 1) or an error text, never both.
- Provenance: the checker blob at the plans' source commit equals the checker that runs; the recorded export tree
  status and the checker's own tree status are clean.

Items (all evaluated and recorded; the decision names the first failed one):
- PY01 evidence. If PY01's verdict is not PASS or its completeness item failed, the decision is ABSTAIN (no
  component evidence), ledger INCONCLUSIVE.
- A1 Dominance. In all six gated cases recurrence_adjoint / S >= 0.9.
- A2 Closability. In at least one gated case the closability floor F <= the case's admission budget.
- A3 Work feasibility. At least 8 of the 32 stage-2 cases have a Laguerre plan with strictly fewer vector products
  than the Chebyshev plan, both at total_budget / 4.
ADMIT iff all hold; else ABSTAIN (A_k), ledger mapping verdict PASS with claim "decision ABSTAIN (A_k)".

Reported, not gated: per-case degrees, scales and products of both bases; the parity of the local helper with
PY03's ``plan_joint_phi``; an exact-arithmetic emulation of both degree rules at 50 digits (the minimal degree whose
tail meets the budget); norm_f against the 50-digit ||F||; the registered predictions.

Interpretations and deviations (fixed here, before the recorded run):
1. Helper. The local plan-only helper is ``polynomial_action::plan_only_comparison``: the action's normalization,
   checks and weight factor, then ``choose_transform`` for Chebyshev and for Laguerre (scales {1, 2, 4, 8, 16}).
   It is written independently of PY03's ``plan_joint_phi`` (merged), whose parity is reported only.
2. ||F|| in the budget. The registered total budget is factor max(||F||, 1). R-NEXT-04 (the generator's source)
   took ||F|| from the executed action's fused result; the plan-only export may execute no recurrence, so norm_f is
   the binary64 norm of the dense augmented exponential (``dense_phi_combination``, no polynomial, no coefficient
   table, no admission). The checker binds it to the 50-digit ||F|| (relative 1e-10 on max(||F||, 1)); the plans
   use the recorded bits, which a stage-2 run would reuse.
3. Seeds and inputs follow R-NEXT-04's conventions with the first operator seed 9001: one seed per (n, rho, h) in
   grid order (9001..9008), the distinct input from seed + 9, the same-vector input from seed + 5; both budget
   factors share the inputs. Labels: n{n}-rho{rho}-h{h}-{distinct|same}-b{factor} (Rust Display / {:e}).
4. A3 counting. A case counts iff both plans exist and the Laguerre vector products are strictly below the
   Chebyshev ones. A failed plan in either basis does not count (the conservative reading). A Laguerre degree above
   128 is counted by products alone, as registered, and is reported.
5. A1 reads ``reported.cases[label].terms_exact.recurrence_adjoint`` (the exact sum over the five columns) and
   ``S`` (the exact component sum), decimal strings rounded to 12 significant digits by PY01. The share is evaluated
   as a lower bound, ra (1 - 1e-11) / (S (1 + 1e-11)), in exact rational arithmetic. The recorded
   ``shares.recurrence_adjoint`` and the 8-digit column sums are reported.
6. A2 reads ``closability_floor_F`` (12 digits) as F (1 + 1e-11), an upper bound, against ``admission_budget``;
   PY01's own ``F_at_most_budget`` is reported.
7. "PY01 is INVALID or FAIL on completeness": PY01's ``verdict`` is not "PASS", its ``evidence_schema`` is not the
   AS03 version, or ``gate.1_completeness`` is not true. The pinned file is PASS; the branch exists for
   completeness and is covered by the synthetic tests.
8. Only RESULTS.json is read from PY01, as registered (COMPONENTS.json is not read; RESULTS.json's own pinned input
   record of it is reported).
9. The checker's own tree status excludes untracked files (PLANS.json and ADMISSION.json are untracked when it
   runs); the export records ``git status --porcelain`` with untracked files.
10. The 50- and 70-digit references are computed once per (A, h, input) and shared by the two budget factors.
"""

from __future__ import annotations

import argparse
from decimal import Decimal
from fractions import Fraction
import json
import struct
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
if str(Path(__file__).resolve().parent) not in sys.path:
    sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence_schema_v2 as evidence  # noqa: E402  (fail-closed evidence validation, re-audit AS03)

SCHEMA = "vigilode-py02-laguerre-admission-v1"
PLANS_SCHEMA = "vigilode-py02-laguerre-plans-v1"
NODE = "research/py02_laguerre_adjoint_bound_20261011"
CHECKER_PATH = "tools/py02_laguerre_admission.py"
PY01_SHA256 = "235dcd5c5266b33c8324d33f7987ee55ac21aa6d3e589f8ffcec103765555b18"

GATED = tuple(f"n{n}-rho400-h0.1-{kind}" for n in (6, 12, 20) for kind in ("distinct", "same"))
GRID_N = (7, 14)
GRID_RHO = (100.0, 300.0)
GRID_H = (0.08, 0.16)
GRID_INPUTS = ("distinct", "same")
BUDGET_FACTORS = (1e-6, 1e-8)
FIRST_SEED = 9001
INPUT_SCALES = (1.0, -0.5, 0.25, 2.0, -1.0)
LAGUERRE_SCALES = (1.0, 2.0, 4.0, 8.0, 16.0)
TRUNCATION_FRACTION = 0.25
A1_THRESHOLD = Fraction(9, 10)
A3_THRESHOLD = 8
DECIMAL_ROUNDING = Fraction(1, 10 ** 11)
NORM_TOLERANCE = Fraction(1, 10 ** 10)
WEIGHT_TOLERANCE = Fraction(1, 10 ** 12)
DEGREE_LIMIT = 128
TERMS = 5
DPS = 50
DPS_CHECK = 70
REFERENCE_AGREEMENT = "1e-40"
DISJOINT = {
    "rnext04": ((6, 12, 20), (1.0, 50.0, 400.0), (1e-3, 1e-2, 0.1)),
    "pp08": ((5, 16), (2.0, 30.0, 200.0), (1e-3, 0.03, 0.2)),
    "py03": ((8, 24), (4.0, 64.0), (0.005, 0.05, 0.12)),
}

DOC_KEYS = frozenset(("schema", "node", "source_commit", "tree_status", "constants", "cases"))
CONSTANT_KEYS = frozenset(("first_seed", "truncation_fraction", "laguerre_scales", "budget_factors", "input_scales",
                           "a3_threshold"))
CASE_KEYS = frozenset(("label", "n", "rho_nominal", "h", "input", "budget_factor", "seed", "input_seed", "a", "w",
                       "vector", "scales", "enclosure", "norm_f", "total_budget", "truncation_budget", "plan",
                       "parity_reported"))
ENCLOSURE_KEYS = frozenset(("lambda", "rho", "verified"))
PLAN_KEYS = frozenset(("branch", "normalization_shift", "width", "weight_factor", "core_budget", "chebyshev",
                       "laguerre"))
BASIS_KEYS = frozenset(("degree", "laguerre_scale", "truncation_bound", "vector_products", "tail_evaluations",
                        "error"))
PARITY_KEYS = frozenset(("chebyshev", "laguerre", "equal"))
PARITY_BASIS_KEYS = frozenset(("degree", "laguerre_scale", "vector_products", "tail_evaluations", "error"))
BASES = ("chebyshev", "laguerre")

PREDICTIONS = {
    "A1_holds": "A1 holds",
    "A3_fails_count_0": "A3 fails with a count of 0",
    "outcome_ABSTAIN_A3": "Most likely outcome: ABSTAIN (A3)",
}


# ------------------------------------------------------------------------------------------- helpers
def hx(x: float) -> str:
    return struct.pack(">d", x).hex()


def unhex(text: str) -> float:
    return struct.unpack(">d", bytes.fromhex(text))[0]


def rust_display(x: float) -> str:
    """Rust's ``Display`` of an f64 in the labels (shortest round trip, no trailing ``.0``)."""
    text = repr(float(x))
    if "e" in text or "inf" in text or "nan" in text:
        raise ValueError(f"no plain display for {x!r}")
    return text[:-2] if text.endswith(".0") else text


def rust_exp(x: float) -> str:
    """Rust's ``{:e}`` of an f64 in the labels (``1e-6``)."""
    if x == 0.0:
        return "0e0"
    sign, digits, exponent = Decimal(repr(abs(float(x)))).as_tuple()
    text = "".join(map(str, digits)).rstrip("0") or "0"
    power = exponent + len(digits) - 1
    mantissa = text[0] + ("." + text[1:] if len(text) > 1 else "")
    return f"{'-' if x < 0 else ''}{mantissa}e{power}"


def splitmix(seed: int, n: int) -> list[float]:
    mask = (1 << 64) - 1
    state = seed & mask
    out = []
    for _ in range(n):
        state = (state + 0x9E3779B97F4A7C15) & mask
        z = state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & mask
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & mask
        z ^= z >> 31
        out.append(2.0 * ((z >> 11) / float(1 << 53)) - 1.0)
    return out


def generator_matrix(n: int, rho: float, seed: int) -> list[list[float]]:
    """The R-NEXT-04 generator: a_ii = -d_i, d_i = rho (0.65 + 0.35 u_i), symmetric off-diagonal entries."""
    d = [rho * (0.65 + 0.35 * u) for u in splitmix(seed, n)]
    raw = splitmix(seed + 1000, n * n)
    a = [[0.0] * n for _ in range(n)]
    for i in range(n):
        a[i][i] = -d[i]
        for j in range(i):
            cap = 0.2 * min(d[i], d[j]) / n
            v = cap * raw[i * n + j]
            a[i][j] = v
            a[j][i] = v
    return a


def registered_input(kind: str, n: int, seed: int) -> tuple[list[list[float]], list[float] | None]:
    """(w_0..w_4, v or None) for the operator seed."""
    if kind == "distinct":
        return [splitmix(seed + 9 + 77 * k, n) for k in range(TERMS)], None
    v = splitmix(seed + 5, n)
    return [[s * x for x in v] for s in INPUT_SCALES], v


def grid_cases() -> list[dict]:
    """The registered stage-2 grid in export order."""
    out, seed = [], FIRST_SEED
    for n in GRID_N:
        for rho in GRID_RHO:
            for h in GRID_H:
                for kind in GRID_INPUTS:
                    for factor in BUDGET_FACTORS:
                        label = f"n{n}-rho{rust_display(rho)}-h{rust_display(h)}-{kind}-b{rust_exp(factor)}"
                        out.append({"label": label, "n": n, "rho": rho, "h": h, "input": kind, "factor": factor,
                                    "seed": seed, "input_seed": seed + (5 if kind == "same" else 9)})
                seed += 1
    return out


def decimal_fraction(text) -> Fraction:
    if not isinstance(text, str):
        raise ValueError(f"not a decimal string: {text!r}")
    value = Decimal(text)
    if not value.is_finite():
        raise ValueError(f"not finite: {text!r}")
    return Fraction(value)


def fstr(x: Fraction, digits: int = 10) -> str:
    """A short decimal of an exact rational (for the report only)."""
    if x == 0:
        return "0"
    return f"{float(x):.{digits}g}"


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


# ------------------------------------------------------------------------------------------- reference
_EIGEN: dict = {}


def _phi(mp, k: int, z):
    """phi_k(z) = sum_j z^j / (j + k)!, phi_0 = exp."""
    if abs(z) < mp.mpf("0.5"):
        total, term, j = mp.mpf(0), mp.mpf(1) / mp.factorial(k), 0
        while abs(term) > mp.mpf(10) ** (-mp.mp.dps - 5):
            total += term
            j += 1
            term = term * z / (j + k)
        return total
    value = mp.e ** z
    for m in range(1, k + 1):
        value = (value - 1 / mp.factorial(m - 1)) / z
    return value


def exact_target(a: list[list[float]], h: float, w: list[list[float]], dps: int) -> list:
    """F = sum_k phi_k(h A) w_k for the binary64 A, h, w_k taken as exact reals, at ``dps`` digits."""
    import mpmath as mp
    n = len(a)
    with mp.workdps(dps):
        key = (tuple(hx(x) for row in a for x in row), dps)
        if key not in _EIGEN:
            _EIGEN[key] = mp.eigsy(mp.matrix([[mp.mpf(x) for x in row] for row in a]))
        eigenvalues, q = _EIGEN[key]
        hh = mp.mpf(h)
        result = [mp.mpf(0)] * n
        for k in range(TERMS):
            if all(x == 0 for x in w[k]):
                continue
            coords = [mp.fsum(q[i, j] * mp.mpf(w[k][i]) for i in range(n)) for j in range(n)]
            scaled = [_phi(mp, k, hh * eigenvalues[j]) * coords[j] for j in range(n)]
            for i in range(n):
                result[i] += mp.fsum(q[i, j] * scaled[j] for j in range(n))
        return [+x for x in result]


def reference_norm(a: list[list[float]], h: float, w: list[list[float]]):
    """(||F|| at 70 digits, the 50/70-digit gap) as mpmath numbers."""
    import mpmath as mp
    f50 = exact_target(a, h, w, DPS)
    f70 = exact_target(a, h, w, DPS_CHECK)
    with mp.workdps(DPS_CHECK):
        norm = mp.sqrt(mp.fsum(x ** 2 for x in f70))
        gap = mp.sqrt(mp.fsum((x - y) ** 2 for x, y in zip(f50, f70)))
    return norm, gap


def exact_weight_factor(w: list[list[float]]):
    """W = sum_k ||w_k||_2 / k! at 50 digits (w_k = s_k v for the same-vector input, exactly)."""
    import mpmath as mp
    with mp.workdps(DPS):
        return mp.fsum(mp.sqrt(mp.fsum(mp.mpf(x) ** 2 for x in w[k])) / mp.factorial(k) for k in range(TERMS))


# ------------------------------------------------------------------------------------------- emulation
def emulated_degrees(h: float, enclosure_rho: float, weight: float, budget: float, cap: int = 4096) -> dict:
    """Exact-arithmetic degree rules at 50 digits (reported only): Chebyshev 2 exp(b (cosh t - 1) - (m + 1) t)
    with t = asinh((m + 1) / b), a + b = 0 and eta = 0 for the enclosure [0, rho]; Laguerre the smallest degree over
    the scales of e^{L/2} q^{m+1}, q = h beta / (1 + h beta), beta = rho / L (ties keep the first scale)."""
    import mpmath as mp
    with mp.workdps(DPS):
        rho, hh, wf, bud = mp.mpf(enclosure_rho), mp.mpf(h), mp.mpf(weight), mp.mpf(budget)
        b = hh * rho / 2
        cheb = None
        for m in range(cap + 1):
            r = mp.mpf(m + 1)
            t = mp.asinh(r / b)
            tail = 2 * mp.e ** (b * (mp.cosh(t) - 1) - r * t)
            if tail * wf <= bud:
                cheb = m
                break
        best = None
        for scale in LAGUERRE_SCALES:
            beta = rho / mp.mpf(scale)
            a = hh * beta
            q = a / (1 + a)
            norm = mp.e ** (mp.mpf(scale) / 2)
            limit = cap if best is None else best[0] - 1
            for m in range(limit + 1):
                if norm * q ** (m + 1) * wf <= bud:
                    best = (m, scale)
                    break
        return {"chebyshev": cheb, "laguerre": None if best is None else best[0],
                "laguerre_scale": None if best is None else best[1]}


# ------------------------------------------------------------------------------------------ evidence
def _load(path: Path, role: str, pin: str | None) -> tuple[dict, str]:
    data = Path(path).read_bytes()
    digest = evidence._sha256(data)
    reasons = []
    if pin is not None and digest != pin:
        reasons.append(f"{role}: {path} is not the registered immutable input (sha256 {digest} != {pin})")
    try:
        doc = evidence.loads_strict(data)
    except evidence.ValidationError as exc:
        raise evidence.ValidationError(reasons + [f"{role}: {r}" for r in exc.reasons]) from None
    if reasons:
        raise evidence.ValidationError(reasons)
    return doc, digest


def _hex(c: evidence._Checks, path: str, value) -> float | None:
    try:
        return evidence.decode_hex64(value)
    except evidence.ValidationError as exc:
        c.fail(path, exc.reasons[0])
        return None


def _validate_py01(c: evidence._Checks, doc) -> dict:
    """The PY01 fields A1 and A2 read, as exact rationals."""
    out: dict = {}
    if not c.dict_("py01", doc):
        return out
    for key in ("verdict", "gate", "reported", "evidence_schema", "inputs"):
        if key not in doc:
            c.fail(f"py01.{key}", "missing")
    if c.reasons:
        return out
    cases = doc["reported"].get("cases") if isinstance(doc["reported"], dict) else None
    if not c.dict_("py01.reported.cases", cases):
        return out
    for label in GATED:
        p = f"py01.reported.cases[{label}]"
        case = cases.get(label)
        if not c.dict_(p, case):
            continue
        if case.get("gated") is not True:
            c.fail(f"{p}.gated", "a registered gated case is not gated")
        try:
            terms = case["terms_exact"]
            out[label] = {
                "recurrence_adjoint": decimal_fraction(terms["recurrence_adjoint"]),
                "S": decimal_fraction(case["S"]),
                "F": decimal_fraction(case["closability_floor_F"]),
                "budget": Fraction(case["admission_budget"]),
                "recorded_share": case["shares"]["recurrence_adjoint"],
                "recorded_F_at_most_budget": case["F_at_most_budget"],
                "column_sum_8_digits": sum(decimal_fraction(col["recurrence_adjoint"])
                                           for col in case["per_column_terms"]),
            }
        except (KeyError, TypeError, ValueError, ArithmeticError) as exc:
            c.fail(p, f"A1/A2 field unreadable: {exc!r}")
            continue
        if type(case["admission_budget"]) is not float or not case["admission_budget"] > 0:
            c.fail(f"{p}.admission_budget", "not a positive float")
        if out[label]["S"] <= 0 or out[label]["recurrence_adjoint"] < 0 or out[label]["F"] < 0:
            c.fail(p, "S must be positive and the terms nonnegative")
    return out


def _basis(c: evidence._Checks, p: str, value, basis: str, width: int, core_budget: float | None) -> dict | None:
    if not c.keys(p, value, BASIS_KEYS):
        return None
    if value["error"] is not None:
        c.str_(f"{p}.error", value["error"])
        for k in BASIS_KEYS - {"error"}:
            if value[k] is not None:
                c.fail(f"{p}.{k}", "a failed plan has no fields")
        return {"error": value["error"]}
    ok = c.int_(f"{p}.degree", value["degree"]) & c.int_(f"{p}.vector_products", value["vector_products"]) \
        & c.int_(f"{p}.tail_evaluations", value["tail_evaluations"], minimum=1)
    bound = _hex(c, f"{p}.truncation_bound", value["truncation_bound"])
    scale = None if value["laguerre_scale"] is None else _hex(c, f"{p}.laguerre_scale", value["laguerre_scale"])
    if not ok:
        return None
    if value["vector_products"] != value["degree"] * width:
        c.fail(f"{p}.vector_products", f"{value['vector_products']} != degree x width {value['degree']} x {width}")
    if bound is not None and core_budget is not None and not bound <= core_budget:
        c.fail(f"{p}.truncation_bound", "above the core budget")
    if basis == "laguerre":
        if scale not in LAGUERRE_SCALES:
            c.fail(f"{p}.laguerre_scale", f"not a registered Laguerre scale: {value['laguerre_scale']!r}")
    elif value["laguerre_scale"] is not None:
        c.fail(f"{p}.laguerre_scale", "a Chebyshev plan has no Laguerre scale")
    return {"error": None, "degree": value["degree"], "vector_products": value["vector_products"],
            "laguerre_scale": scale, "truncation_bound": bound, "tail_evaluations": value["tail_evaluations"]}


def _parity_basis(c: evidence._Checks, p: str, value) -> None:
    if not c.keys(p, value, PARITY_BASIS_KEYS):
        return
    for k in ("degree", "vector_products", "tail_evaluations"):
        if value[k] is not None:
            c.int_(f"{p}.{k}", value[k])
    if value["laguerre_scale"] is not None:
        _hex(c, f"{p}.laguerre_scale", value["laguerre_scale"])
    if value["error"] is not None:
        c.str_(f"{p}.error", value["error"])


def _validate_case(c: evidence._Checks, case, spec: dict, reference) -> dict | None:
    p = f"plans.cases[{spec['label']}]"
    if not c.keys(p, case, CASE_KEYS):
        return None
    n = spec["n"]
    c.equal(f"{p}.n", case["n"], n)
    c.equal(f"{p}.rho_nominal", case["rho_nominal"], hx(spec["rho"]))
    c.equal(f"{p}.h", case["h"], hx(spec["h"]))
    c.equal(f"{p}.input", case["input"], spec["input"])
    c.equal(f"{p}.budget_factor", case["budget_factor"], hx(spec["factor"]))
    c.equal(f"{p}.seed", case["seed"], spec["seed"])
    c.equal(f"{p}.input_seed", case["input_seed"], spec["input_seed"])
    a = generator_matrix(n, spec["rho"], spec["seed"])
    if case["a"] != [[hx(x) for x in row] for row in a]:
        c.fail(f"{p}.a", "not the R-NEXT-04 generator matrix of the registered seed")
    w, v = registered_input(spec["input"], n, spec["seed"])
    if case["w"] != [[hx(x) for x in row] for row in w]:
        c.fail(f"{p}.w", "not the registered input stream")
    if spec["input"] == "same":
        c.equal(f"{p}.vector", case["vector"], [hx(x) for x in v])
        c.equal(f"{p}.scales", case["scales"], [hx(s) for s in INPUT_SCALES])
    else:
        c.equal(f"{p}.vector", case["vector"], None)
        c.equal(f"{p}.scales", case["scales"], None)
    enclosure_rho = 1.2 * spec["rho"] * (1.0 + 1.0e-12)
    if c.keys(f"{p}.enclosure", case["enclosure"], ENCLOSURE_KEYS):
        c.equal(f"{p}.enclosure.lambda", case["enclosure"]["lambda"], hx(0.0))
        c.equal(f"{p}.enclosure.rho", case["enclosure"]["rho"], hx(enclosure_rho))
        if case["enclosure"]["verified"] is not True:
            c.fail(f"{p}.enclosure.verified", "the Gershgorin enclosure is not verified")
    norm_f = _hex(c, f"{p}.norm_f", case["norm_f"])
    total = _hex(c, f"{p}.total_budget", case["total_budget"])
    trunc = _hex(c, f"{p}.truncation_budget", case["truncation_budget"])
    if None in (norm_f, total, trunc):
        return None
    if total != spec["factor"] * max(norm_f, 1.0):
        c.fail(f"{p}.total_budget", "not factor x max(norm_f, 1) bit for bit")
    if trunc != total * TRUNCATION_FRACTION:
        c.fail(f"{p}.truncation_budget", "not total_budget / 4 bit for bit")
    plan = case["plan"]
    out = {"label": spec["label"], "a": a, "w": w, "h": spec["h"], "norm_f": norm_f, "total_budget": total,
           "truncation_budget": trunc, "enclosure_rho": enclosure_rho, "spec": spec}
    if not c.keys(f"{p}.plan", plan, PLAN_KEYS):
        return None
    width = 1 if spec["input"] == "same" else 5
    c.equal(f"{p}.plan.branch", plan["branch"], "recurrence")
    c.equal(f"{p}.plan.normalization_shift", plan["normalization_shift"], 0)
    c.equal(f"{p}.plan.width", plan["width"], width)
    core = _hex(c, f"{p}.plan.core_budget", plan["core_budget"])
    if core is not None and core != trunc:
        c.fail(f"{p}.plan.core_budget", "differs from the truncation budget (no normalization in the window)")
    weight = _hex(c, f"{p}.plan.weight_factor", plan["weight_factor"])
    out["weight_factor"] = weight
    for basis in BASES:
        out[basis] = _basis(c, f"{p}.plan.{basis}", plan[basis], basis, width, core)
    par = case["parity_reported"]
    if c.keys(f"{p}.parity_reported", par, PARITY_KEYS):
        for basis in BASES:
            _parity_basis(c, f"{p}.parity_reported.{basis}", par[basis])
        c.bool_(f"{p}.parity_reported.equal", par["equal"])
    return out


def validate(py01_path: Path, plans_path: Path, blob_at=None, tree_status=None, reference=None,
             weight=None, py01_pin: str | None = PY01_SHA256) -> evidence.Evidence:
    """Every INVALID condition; returns the validated documents with decoded cases."""
    blob_at = blob_at or checker_blob_at
    tree_status = tree_status or own_tree_status
    reference = reference or reference_norm
    weight = weight or exact_weight_factor
    reasons = []
    py01 = plans = None
    digests = {}
    for role, path, pin in (("py01", py01_path, py01_pin), ("plans", plans_path, None)):
        try:
            doc, digest = _load(path, role, pin)
            digests[role] = digest
            if role == "py01":
                py01 = doc
            else:
                plans = doc
        except evidence.ValidationError as exc:
            reasons.extend(exc.reasons)
        except OSError as exc:
            reasons.append(f"{role}: cannot read {path}: {exc}")
    if reasons:
        raise evidence.ValidationError(reasons)
    c = evidence._Checks()
    py01_fields = _validate_py01(c, py01)
    # Plans document.
    if not c.keys("plans", plans, DOC_KEYS):
        c.done()
    c.equal("plans.schema", plans["schema"], PLANS_SCHEMA)
    c.equal("plans.node", plans["node"], NODE)
    if c.str_("plans.source_commit", plans["source_commit"]) and (
            len(plans["source_commit"]) != 40
            or any(ch not in "0123456789abcdef" for ch in plans["source_commit"])):
        c.fail("plans.source_commit", "not a 40-digit commit id")
    if c.str_("plans.tree_status", plans["tree_status"], nonempty=False) and plans["tree_status"] != "":
        c.fail("plans.tree_status", f"the export tree was dirty: {plans['tree_status'][:200]!r}")
    status = tree_status()
    if status != "":
        c.fail("checker.tree_status", f"the checker tree is dirty: {status[:200]!r}")
    if isinstance(plans.get("source_commit"), str) and len(plans["source_commit"]) == 40:
        try:
            if blob_at(plans["source_commit"]) != Path(__file__).read_bytes():
                c.fail("provenance", f"the checker blob at {plans['source_commit']} differs from the checker run")
        except evidence.ValidationError as exc:
            c.fail("provenance", exc.reasons[0])
    if c.keys("plans.constants", plans["constants"], CONSTANT_KEYS):
        k = plans["constants"]
        c.equal("plans.constants.first_seed", k["first_seed"], FIRST_SEED)
        c.equal("plans.constants.truncation_fraction", k["truncation_fraction"], TRUNCATION_FRACTION)
        c.equal("plans.constants.laguerre_scales", k["laguerre_scales"], [hx(s) for s in LAGUERRE_SCALES])
        c.equal("plans.constants.budget_factors", k["budget_factors"], [hx(f) for f in BUDGET_FACTORS])
        c.equal("plans.constants.input_scales", k["input_scales"], [hx(s) for s in INPUT_SCALES])
        c.equal("plans.constants.a3_threshold", k["a3_threshold"], A3_THRESHOLD)
    grid = grid_cases()
    for spec in grid:
        for name, (ns, rhos, hs) in DISJOINT.items():
            if spec["n"] in ns and spec["rho"] in rhos and spec["h"] in hs:
                c.fail("grid", f"{spec['label']} shares (n, rho, h) with {name}")
    decoded: list[dict] = []
    if c.list_("plans.cases", plans["cases"], nonempty=True):
        rows = plans["cases"]
        labels = [r.get("label") if isinstance(r, dict) else None for r in rows]
        if labels != [g["label"] for g in grid]:
            c.fail("plans.cases", f"not the 32 registered labels in order (got {len(labels)}; first difference "
                                  f"{next(((i, x) for i, (x, y) in enumerate(zip(labels, grid)) if x != y['label']), None)})")
        else:
            for row, spec in zip(rows, grid):
                d = _validate_case(c, row, spec, reference)
                if d is not None:
                    decoded.append(d)
    c.done()
    # Weight factors and references (exact arithmetic).
    import mpmath as mp
    refs: dict = {}
    for d in decoded:
        p = f"plans.cases[{d['label']}]"
        if d["weight_factor"] is not None:
            exact = Fraction(str(mp.nstr(weight(d["w"]), 45, strip_zeros=False)))
            rec = Fraction(d["weight_factor"])
            if not (exact * (1 - Fraction(1, 10 ** 40)) <= rec <= exact * (1 + WEIGHT_TOLERANCE)):
                c.fail(f"{p}.plan.weight_factor", f"{d['weight_factor']!r} is not in [W, W (1 + 1e-12)], "
                                                  f"W = {fstr(exact, 17)}")
        key = (d["spec"]["seed"], d["spec"]["input"])
        if key not in refs:
            refs[key] = reference(d["a"], d["h"], d["w"])
        norm, gap = refs[key]
        with mp.workdps(DPS_CHECK):
            if gap > mp.mpf(REFERENCE_AGREEMENT) * norm:
                c.fail(f"{p}.reference", f"50- and 70-digit references differ by {mp.nstr(gap, 5)}")
            exact_norm = Fraction(str(mp.nstr(norm, 60, strip_zeros=False)))
        ref_max = max(exact_norm, Fraction(1))
        rec_max = max(Fraction(d["norm_f"]), Fraction(1))
        rel = abs(rec_max - ref_max) / ref_max
        d["norm_reference"] = exact_norm
        d["norm_relative_difference"] = abs(Fraction(d["norm_f"]) - exact_norm) / exact_norm
        if rel > NORM_TOLERANCE:
            c.fail(f"{p}.norm_f", f"max(norm_f, 1) differs from the 50-digit max(||F||, 1) by {fstr(rel)} relative")
    c.done()
    return evidence.Evidence("PY02-admission", {"py01": py01, "py01_fields": py01_fields, "plans": plans,
                                                "decoded": decoded},
                             digests, {"py01": Path(py01_path), "plans": Path(plans_path)})


# --------------------------------------------------------------------------------------------- items
def py01_evidence(py01: dict) -> dict:
    verdict = py01.get("verdict")
    gate = py01.get("gate") if isinstance(py01.get("gate"), dict) else {}
    completeness = gate.get("1_completeness")
    holds = verdict == "PASS" and completeness is True and py01.get("evidence_schema") == evidence.VERSION
    return {"pass": holds, "verdict": verdict, "1_completeness": completeness,
            "evidence_schema": py01.get("evidence_schema")}


def item_a1(fields: dict) -> dict:
    cases, ok, low = {}, True, None
    for label in GATED:
        f = fields[label]
        share = f["recurrence_adjoint"] * (1 - DECIMAL_ROUNDING) / (f["S"] * (1 + DECIMAL_ROUNDING))
        holds = share >= A1_THRESHOLD
        ok &= holds
        low = share if low is None else min(low, share)
        cases[label] = {"share_lower_bound": fstr(share, 13), "holds": holds,
                        "recurrence_adjoint": fstr(f["recurrence_adjoint"], 12), "S": fstr(f["S"], 12),
                        "recorded_share": f["recorded_share"],
                        "column_sum_8_digits_over_S": fstr(f["column_sum_8_digits"] / f["S"], 10)}
    return {"pass": ok, "threshold": "0.9", "min_share_lower_bound": fstr(low, 13), "cases": cases}


def item_a2(fields: dict) -> dict:
    cases, count = {}, 0
    for label in GATED:
        f = fields[label]
        upper = f["F"] * (1 + DECIMAL_ROUNDING)
        holds = upper <= f["budget"]
        count += holds
        cases[label] = {"F": fstr(f["F"], 12), "F_upper": fstr(upper, 13), "admission_budget": float(f["budget"]),
                        "F_over_budget": fstr(upper / f["budget"], 6), "holds": holds,
                        "recorded_F_at_most_budget": f["recorded_F_at_most_budget"]}
    return {"pass": count >= 1, "rule": "at least one of the six gated cases has F <= its admission budget",
            "cases_holding": count, "cases": cases}


def item_a3(decoded: list[dict]) -> dict:
    cases, count = {}, 0
    for d in decoded:
        cheb, lag = d["chebyshev"], d["laguerre"]
        both = cheb is not None and lag is not None and cheb["error"] is None and lag["error"] is None
        fewer = both and lag["vector_products"] < cheb["vector_products"]
        count += fewer
        row = {"laguerre_fewer": fewer, "both_planned": both,
               "chebyshev_error": None if cheb is None else cheb["error"],
               "laguerre_error": None if lag is None else lag["error"],
               "total_budget": d["total_budget"], "truncation_budget": d["truncation_budget"]}
        if both:
            row.update({"chebyshev_degree": cheb["degree"], "laguerre_degree": lag["degree"],
                        "laguerre_scale": lag["laguerre_scale"],
                        "chebyshev_products": cheb["vector_products"], "laguerre_products": lag["vector_products"],
                        "laguerre_over_chebyshev": lag["vector_products"] / cheb["vector_products"]
                        if cheb["vector_products"] else None,
                        "laguerre_degree_above_128": lag["degree"] > DEGREE_LIMIT})
        cases[d["label"]] = row
    return {"pass": count >= A3_THRESHOLD, "threshold": A3_THRESHOLD, "count": count,
            "rule": "Laguerre plan vector products (degree x width) strictly below Chebyshev's, both at "
                    "total_budget / 4", "cases": cases}


def reported(decoded: list[dict], plans: dict, a1: dict, a3: dict, decision: str) -> dict:
    parity, emulation, norms = {}, {}, {}
    equal_count = emu_equal = 0
    ratios = []
    for case, d in zip(plans["cases"], decoded):
        par = case["parity_reported"]
        same = True
        for basis in BASES:
            local = case["plan"][basis]
            same &= all(local[k] == par[basis][k] for k in ("degree", "laguerre_scale", "vector_products",
                                                            "tail_evaluations")) \
                and (local["error"] is None) == (par[basis]["error"] is None)
        equal_count += same
        parity[d["label"]] = {"recomputed_equal": same, "recorded_equal": par["equal"]}
        emu = emulated_degrees(d["h"], d["enclosure_rho"], d["weight_factor"], d["truncation_budget"])
        native = {"chebyshev": (d["chebyshev"] or {}).get("degree"), "laguerre": (d["laguerre"] or {}).get("degree"),
                  "laguerre_scale": (d["laguerre"] or {}).get("laguerre_scale")}
        agree = emu == native
        emu_equal += agree
        emulation[d["label"]] = {"emulated": emu, "native": native, "equal": agree}
        norms[d["label"]] = {"norm_f": d["norm_f"], "relative_difference_to_50_digits":
                             fstr(d["norm_relative_difference"], 4), "budget_uses_one": d["norm_f"] < 1.0}
        row = a3["cases"][d["label"]]
        if row.get("laguerre_over_chebyshev") is not None:
            ratios.append(row["laguerre_over_chebyshev"])
    return {
        "plan_parity_with_py03": {"cases_equal": equal_count, "cases": len(decoded), "per_case": parity},
        "degree_emulation_50_digits": {"cases_equal": emu_equal, "cases": len(decoded), "per_case": emulation},
        "norm_f": norms,
        "laguerre_over_chebyshev_products": {"min": min(ratios) if ratios else None,
                                             "max": max(ratios) if ratios else None},
        "predictions": {
            "A1_holds": {"registered": PREDICTIONS["A1_holds"], "holds": a1["pass"]},
            "A3_fails_count_0": {"registered": PREDICTIONS["A3_fails_count_0"],
                                 "holds": (not a3["pass"]) and a3["count"] == 0, "count": a3["count"]},
            "outcome_ABSTAIN_A3": {"registered": PREDICTIONS["outcome_ABSTAIN_A3"],
                                   "holds": decision == "ABSTAIN (A3)"},
        },
    }


def evaluate(validated: evidence.Evidence) -> dict:
    docs = validated.docs
    evidence_item = py01_evidence(docs["py01"])
    a1 = item_a1(docs["py01_fields"])
    a2 = item_a2(docs["py01_fields"])
    a3 = item_a3(docs["decoded"])
    items = {"A1_dominance": a1, "A2_closability": a2, "A3_work_feasibility": a3}
    if not evidence_item["pass"]:
        decision, first, verdict = "ABSTAIN (no component evidence)", None, "ABSTAIN"
        ledger = {"verdict": "INCONCLUSIVE", "claim": "decision ABSTAIN (no component evidence)"}
    else:
        first = next((k for k, item in (("A1", a1), ("A2", a2), ("A3", a3)) if not item["pass"]), None)
        if first is None:
            decision, verdict = "ADMIT", "ADMIT"
            ledger = None
        else:
            decision, verdict = f"ABSTAIN ({first})", "ABSTAIN"
            ledger = {"verdict": "PASS", "claim": f"decision ABSTAIN ({first})"}
    return {"verdict": verdict, "decision": decision, "first_failed": first, "py01_evidence": evidence_item,
            "admission": {"A1": a1["pass"], "A2": a2["pass"], "A3": a3["pass"]}, "items": items,
            "ledger_mapping": ledger,
            "stage2": "admitted: stage 2 may run as registered" if verdict == "ADMIT"
            else "not admitted: stage 2 is not implemented or run",
            "reported": reported(docs["decoded"], docs["plans"], a1, a3, decision)}


def main(argv: list[str] | None = None, blob_at=None, tree_status=None, head=None, reference=None,
         py01_pin: str | None = PY01_SHA256) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--py01", required=True, type=Path)
    parser.add_argument("--plans", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args(argv)
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    inputs = {"py01": args.py01, "plans": args.plans}
    try:
        validated = validate(args.py01, args.plans, blob_at, tree_status, reference, py01_pin=py01_pin)
    except evidence.ValidationError as exc:
        return evidence.emit_invalid(args.output, SCHEMA, exc, inputs)
    result = evaluate(validated)
    plans = validated.docs["plans"]
    try:
        from mpmath import __version__ as mpmath_version
    except ImportError:  # pragma: no cover
        mpmath_version = None
    report = {
        "schema": SCHEMA,
        "evidence_schema": evidence.VERSION,
        "node": NODE,
        **result,
        "plans_source_commit": plans["source_commit"],
        "plans_tree_status": plans["tree_status"],
        "checker_head": (head or own_head)(),
        "checker_tree_status": (tree_status or own_tree_status)(),
        "inputs": {"py01": {"path": str(args.py01), "sha256": validated.sha256["py01"]},
                   "plans": {"path": str(args.plans), "sha256": validated.sha256["plans"]},
                   "py01_recorded_inputs": validated.docs["py01"].get("inputs")},
        "environment": {"python": sys.version.split()[0], "mpmath": mpmath_version, "dps": DPS,
                        "dps_check": DPS_CHECK},
        "claim_scope": "Stage-1 admission of PY02 only: A1 and A2 from PY01's recorded RESULTS.json, A3 from "
                       "plan-only degree searches (no recurrence, coefficient table, envelope or admission on a "
                       "stage-2 case). ABSTAIN is not a negative result about Laguerre in general; no bound, "
                       "admission, routing, default or wall-time claim changes.",
    }
    args.output.write_text(json.dumps(report, indent=1, sort_keys=True, allow_nan=False) + "\n")
    for k, v in result["admission"].items():
        print(f"{k}: {'holds' if v else 'fails'}")
    print(f"A3 count: {result['items']['A3_work_feasibility']['count']} (threshold {A3_THRESHOLD})")
    print(f"decision: {result['decision']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
