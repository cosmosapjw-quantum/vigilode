#!/usr/bin/env python3
"""Gate of research node research/py01_laguerre_component_export_20261011 (PY01; see its PREREGISTRATION.md).

Inputs:
- ``--published``: research/rnext04_laguerre_admission_20261003/cases.json (R-NEXT-04, immutable, SHA-256 pinned).
- ``--pp09``: research/pp09_laguerre_envelope_20261004/RESULTS.json (PP09, immutable, SHA-256 pinned; reported
  comparison only).
- ``--components``: COMPONENTS.json, written by ``export_components`` of
  crates/rodas5p-core/tests/laguerre_component_export.rs (``joint_phi_action_laguerre_components`` on the nine
  registered cases).
- ``--output``: RESULTS.json (immutable: an existing file is never overwritten).

Evidence (INVALID, exit 2) before any gate:
- AS03 rules (tools/evidence_schema_v2.py primitives): strict JSON, the published and PP09 SHA-256 pins, exact key
  sets, canonical binary64 hex decoding (16 lowercase digits, finite), typed numbers, unique raw labels, exactly the
  nine registered cases.
- Binding: the exported A, h and w bits equal the published ones; the run's degree, ``laguerre_scale``,
  ``normalization_shift``, fused bits and ``laguerre_adjoint_total`` bits equal the published ``degree``,
  ``laguerre_scale``, ``normalization_shift``, ``fused`` and ``coverage.total``; the export's own copies (identity,
  assembly) agree with the report; the identity follows the registered construction (h, lambda = 0,
  rho = 1.2 rho_nominal (1 + 1e-12), Gershgorin evidence, truncation budget, input kind and scales of the label,
  proof version and depth, the operator fingerprint recomputed from the published A and the enclosure).
- Stamps: every term block's ``identity_sha256`` equals the digest the checker recomputes from the identity fields
  (canonical encoding: compact JSON with sorted keys); no two cases share an identity.
- Provenance: the checker blob at the COMPONENTS source commit equals the checker that runs; the recorded export
  tree status and the checker's own tree status are clean.

Gate (all nine cases), PASS iff all hold, else FAIL (exit 1), PASS exit 0:
1. Completeness: the five terms (truncation, coefficient, recurrence_adjoint, summation, normalization) for each
   column k = 0..4, ``fused_summation``, and per column m + 1 coefficients, m + 1 envelopes and m residual bounds
   (m the degree); every term finite and >= 0, none null.
2. Enclosure: the upward sum in the action's order (fused_summation; per column truncation, coefficient,
   summation; every recurrence_adjoint; every normalization), emulating directed.rs ``add_up`` in exact rational
   arithmetic, is >= the native total; equal bit for bit in the window (shift 0); out of the window at most
   total (1 + 1e-12) + (5 sqrt n + 64) 2^-1074.
3. Source binding of the adjoint: per column the upward sum of mul_up(beta_j, eps_(j-1)), j = 1..m, then
   mul_up(|s_k|, .) and out of the window the action's 2^shift scaling rounded up, equals the exported
   ``recurrence_adjoint`` bit for bit (mul_up emulated with the unconditional outward step below 1e-289); the
   EnvelopeKey digest matches the coefficients, extent and depth.

Reported, not gated: shares term / S (S the exact rational sum of the terms) with the bottleneck, the comparison
with PP09's derived truncation and coefficient, per column max_j beta_j, the index of the largest beta_j eps_(j-1),
the sampled lower bounds l_j <= sup_[0, L'] |z_j| (mpmath.iv, 60 digits, 4097 equispaced points, backward
recurrence of laguerre_adjoint.rs on the exact stored coefficients), beta_j / l_j, and the closability floor
F = N + sum_k |s_k| sum_j l_j eps_(j-1) against the admission budget; the registered predictions.

Interpretations and deviations (fixed here, before the recorded run):
1. Truncation budget. The registration says "the truncation budget is 1e-10". The R-NEXT-04 source ran its main
   suite at 1e-10 and the two scale controls at 1e-10 x amplitude (1e-320 and 1e290). Literally 1e-10 would
   execute the scale controls at another degree than the published one, which the registered binding rejects. The
   export uses the R-NEXT-04 rule 1e-10 x amplitude (amplitude 1 for the seven other cases); the binding check
   (degree, fused bits, total bits equal the published ones) remains the arbiter.
2. Item 2's 1e-12 is the exact decimal 10^-12 and the bound is evaluated exactly (5 sqrt n by squaring).
3. "The EnvelopeKey digest" is the key's ``coefficients_sha256`` (its only digest), recomputed from the exported
   coefficients as point intervals; the key's degree, extent bits, depth, proof version and limits are checked
   with it.
4. Out of the window the action scales a bound by ``ExpBound::exact(v).scaled_pow2(shift).to_f64_up()``: the
   nearest binary64 of v 2^shift when it is exact, else that value's successor (the smallest subnormal when it
   rounds to zero). Item 3 emulates exactly this rule.
5. A null, negative or non-finite term, or a wrong count, is an item-1 FAIL (completeness); a string that is not a
   canonical 16-digit hex of a finite binary64 is INVALID (AS03 hex rule). A missing or extra key is INVALID.
6. Tree status: the export records ``git status --porcelain`` (untracked files included), which must be empty. The
   checker records its own ``git status --porcelain --untracked-files=no`` (COMPONENTS.json is necessarily still
   untracked when checked); a non-empty status is INVALID.
7. Shares use the five DAG terms with assembly = fused_summation + sum_k normalization. The PP09 comparison uses
   the sums over the columns, printed to 8 significant digits as PP09 printed them.
8. l_j: the points are x_i = i L'/4096, i = 0..4096 (exact), evaluated as point intervals; l_j is stored rounded
   down to binary64 (hex) and as a decimal; F is formed exactly from the 60-digit l_j and, out of the window, scaled
   by 2^shift. Up to 4 worker processes compute the l_j; the result does not depend on their number.
9. The parity record of the export (report and counters equal to ``joint_phi_action``) is reported, not gated.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import struct
import subprocess
import sys
from fractions import Fraction
from pathlib import Path

sys.dont_write_bytecode = True
if str(Path(__file__).resolve().parent) not in sys.path:
    sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence_schema_v2 as evidence  # noqa: E402  (fail-closed evidence validation, re-audit AS03)

SCHEMA = "vigilode-py01-laguerre-component-check-v1"
COMPONENTS_SCHEMA = "vigilode-py01-laguerre-components-v1"
EXPORT_SCHEMA = "vigilode-laguerre-component-export-v1"
NODE = "research/py01_laguerre_component_export_20261011"
CHECKER_PATH = "tools/py01_laguerre_component_check.py"
PUBLISHED_PATH = "research/rnext04_laguerre_admission_20261003/cases.json"
PUBLISHED_SHA256 = "700726f0bd2d58d84fbd1e42bf4daa66efd80312057021bc1e69515f28a01b7d"
PP09_SHA256 = "c50789bc3fab65a4f600a329723d48e422c60f8162a9ff39cec0dbd63e318beb"
GATED = ("n6-rho400-h0.1-distinct", "n6-rho400-h0.1-same", "n12-rho400-h0.1-distinct", "n12-rho400-h0.1-same",
         "n20-rho400-h0.1-distinct", "n20-rho400-h0.1-same")
CONTROLS = ("n6-rho400-h0.01-distinct", "subnormal-1e-310", "large-1e300")
CASES = GATED + CONTROLS
SCALES = (1.0, -0.5, 0.25, 2.0, -1.0)
TERMS = 5
PROOF_VERSION = "laguerre-adjoint-bernstein-interval-v1"
ADJOINT_DEPTH = 3
DEGREE_LIMIT = 128
MAX_DEPTH = 12
EXACT_FMA_FLOOR = 1.0e-289
OUT_OF_WINDOW_RELATIVE = Fraction(1, 10 ** 12)
OUT_OF_WINDOW_ULPS = 64
TINY = Fraction(1, 2 ** 1074)
SAMPLE_POINTS = 4097
DPS = 60
WORKERS = 4
PRED_ADJOINT_SHARE = Fraction(999999, 1000000)
PRED_SUMMATION_SHARE = Fraction(1, 10 ** 12)
BUDGET_FLOOR = 1.0e-10

COMPONENTS_KEYS = frozenset(("schema", "node", "source_commit", "tree_status", "published", "cases"))
CASE_KEYS = frozenset(("label", "n", "a", "h", "w", "input", "scales", "rho_nominal", "admission_budget", "report",
                       "parity", "export"))
REPORT_KEYS = frozenset(("degree", "laguerre_scale", "normalization_shift", "fused", "laguerre_adjoint_total",
                         "truncation_bound_exact_arithmetic", "evidence", "admission"))
PARITY_KEYS = frozenset(("report_bitwise_equal", "counters_equal", "counters"))
EXPORT_KEYS = frozenset(("schema", "identity", "identity_sha256", "terms", "assembly", "adjoint"))
IDENTITY_KEYS = frozenset(("basis", "branch", "execution", "input", "degree", "laguerre_scale", "beta",
                           "laguerre_extent", "h", "truncation_budget", "enclosure_lambda", "enclosure_rho",
                           "enclosure_evidence", "operator_fingerprint", "normalization_shift",
                           "adjoint_proof_version", "adjoint_depth", "envelope_key_digests"))
IDENTITY_HEX = ("laguerre_scale", "beta", "laguerre_extent", "h", "truncation_budget", "enclosure_lambda",
                "enclosure_rho")
TERM_FIELDS = ("truncation", "coefficient", "recurrence_adjoint", "summation", "normalization")
TERM_KEYS = frozenset(("identity_sha256", "column", "recurrence_majorant") + TERM_FIELDS)
ASSEMBLY_KEYS = frozenset(("identity_sha256", "fused_summation", "laguerre_adjoint_total"))
ADJOINT_KEYS = frozenset(("identity_sha256", "column", "coefficients", "envelopes", "residuals", "scale_abs",
                          "envelope_key"))
KEY_KEYS = frozenset(("degree", "extent_bits", "depth", "coefficients_sha256", "proof_version", "degree_limit",
                      "max_depth"))


# --------------------------------------------------------------------------------------- arithmetic
def hx(x: float) -> str:
    return struct.pack(">d", x).hex()


def unhex(text: str) -> float:
    return struct.unpack(">d", bytes.fromhex(text))[0]


def next_up(x: float) -> float:
    return math.nextafter(x, math.inf)


def finite(x: float, what: str) -> float:
    if not math.isfinite(x):
        raise ArithmeticError(f"directed rounding: {what} is not finite")
    return x


def add_up(a: float, b: float) -> float:
    """directed.rs add_up: round to nearest, one step up when the exact sum lies above (TwoSum is exact)."""
    s = finite(a + b, "sum")
    return next_up(s) if Fraction(a) + Fraction(b) > Fraction(s) else s


def mul_up(a: float, b: float) -> float:
    """directed.rs mul_up, with the unconditional outward step below EXACT_FMA_FLOOR."""
    p = finite(a * b, "product")
    if a == 0.0 or b == 0.0:
        return p
    if abs(p) < EXACT_FMA_FLOOR:
        return next_up(p)
    return next_up(p) if Fraction(a) * Fraction(b) > Fraction(p) else p


def pow2(shift: int) -> Fraction:
    return Fraction(2) ** shift


def scale_bound_up(x: float, shift: int) -> float:
    """polynomial_action.rs scale_bound_up = ExpBound::exact(x).scaled_pow2(shift).to_f64_up()."""
    if x == 0.0:
        return 0.0
    exact = Fraction(abs(x)) * pow2(shift)
    _, e = math.frexp(abs(x))  # |x| = m 2^e, m in [1/2, 1) (as binary_split)
    e += shift
    if e > 1024:
        return math.inf
    if e < -1074:
        return math.ulp(0.0)
    nearest = float(exact)  # correctly rounded (int / int true division)
    if nearest == 0.0:
        return math.ulp(0.0)
    return nearest if Fraction(nearest) == exact else next_up(nearest)


def floor_f64(x: Fraction) -> float:
    f = float(x)
    if Fraction(f) > x:
        f = math.nextafter(f, -math.inf)
    return f


def sci(x: Fraction | float | None, digits: int = 8) -> str | None:
    """Decimal string of an exact rational (or float) to `digits` significant digits."""
    if x is None:
        return None
    from mpmath import mp, mpf, nstr
    with mp.workdps(max(DPS, digits + 10)):
        value = mpf(x.numerator) / x.denominator if isinstance(x, Fraction) else mpf(x)
        return nstr(value, digits)


def canonical_identity_sha256(identity: dict) -> str:
    text = json.dumps(identity, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
    return hashlib.sha256(text.encode()).hexdigest()


def envelope_key_digest(coefficients: list[float]) -> str:
    text = ",".join(f"{hx(c)}:{hx(c)}" for c in coefficients)
    return hashlib.sha256(text.encode()).hexdigest()


def operator_fingerprint(a: list[list[float]], lam: float, rho: float) -> str:
    data = struct.pack("<Q", len(a))
    for row in a:
        for x in row:
            data += struct.pack("<d", x)
    data += struct.pack("<d", lam) + struct.pack("<d", rho)
    return hashlib.sha256(data).hexdigest()


def registered_rho(label: str) -> float:
    return 50.0 if label in ("subnormal-1e-310", "large-1e300") else 400.0


def registered_truncation_budget(label: str) -> float:
    amplitude = {"subnormal-1e-310": 1.0e-310, "large-1e300": 1.0e300}.get(label, 1.0)
    return BUDGET_FLOOR * amplitude


def registered_input(label: str) -> str:
    return "same-vector" if label.endswith("-same") else "distinct"


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
def _hex_or_null(c, path: str, value) -> float | None:
    """A term value: null (an item-1 failure) or canonical finite hex (else INVALID)."""
    if value is None:
        return None
    try:
        return evidence.decode_hex64(value)
    except evidence.ValidationError as exc:
        c.fail(path, exc.reasons[0])
        return None


def _hex(c, path: str, value) -> float | None:
    try:
        return evidence.decode_hex64(value)
    except evidence.ValidationError as exc:
        c.fail(path, exc.reasons[0])
        return None


def _hex_list(c, path: str, value) -> list | None:
    """A list of term values (each null or hex); length is checked by gate item 1."""
    if not c.list_(path, value):
        return None
    return [_hex_or_null(c, f"{path}[{i}]", v) for i, v in enumerate(value)]


def _matrix(c, path: str, value, rows: int, cols: int) -> list[list[float]] | None:
    if not c.list_(path, value) or len(value) != rows:
        if isinstance(value, list):
            c.fail(path, f"{len(value)} rows != {rows}")
        return None
    out = []
    for i, row in enumerate(value):
        decoded = c.hex_vector(f"{path}[{i}]", row, cols)
        if decoded is None:
            return None
        out.append(decoded)
    return out


def _decode_case(c, p: str, case: dict) -> dict | None:
    """Type one exported case (exact keys, canonical hex); returns decoded values or None."""
    if not c.keys(p, case, CASE_KEYS):
        return None
    d: dict = {"label": case["label"]}
    if not c.int_(f"{p}.n", case["n"], minimum=1):
        return None
    n = case["n"]
    d["n"] = n
    d["a"] = _matrix(c, f"{p}.a", case["a"], n, n)
    d["w"] = _matrix(c, f"{p}.w", case["w"], TERMS, n)
    d["h"] = _hex(c, f"{p}.h", case["h"])
    c.str_(f"{p}.input", case["input"])
    d["scales"] = c.hex_vector(f"{p}.scales", case["scales"], TERMS)
    d["rho_nominal"] = _hex(c, f"{p}.rho_nominal", case["rho_nominal"])
    d["admission_budget"] = _hex(c, f"{p}.admission_budget", case["admission_budget"])
    r = case["report"]
    if c.keys(f"{p}.report", r, REPORT_KEYS):
        c.int_(f"{p}.report.degree", r["degree"])
        c.int_(f"{p}.report.normalization_shift", r["normalization_shift"], minimum=None)
        d["laguerre_scale"] = _hex(c, f"{p}.report.laguerre_scale", r["laguerre_scale"])
        d["fused"] = c.hex_vector(f"{p}.report.fused", r["fused"], n)
        d["total"] = _hex(c, f"{p}.report.laguerre_adjoint_total", r["laguerre_adjoint_total"])
        _hex(c, f"{p}.report.truncation_bound_exact_arithmetic", r["truncation_bound_exact_arithmetic"])
        c.str_(f"{p}.report.evidence", r["evidence"])
        c.dict_(f"{p}.report.admission", r["admission"])
    par = case["parity"]
    if c.keys(f"{p}.parity", par, PARITY_KEYS):
        c.bool_(f"{p}.parity.report_bitwise_equal", par["report_bitwise_equal"])
        c.bool_(f"{p}.parity.counters_equal", par["counters_equal"])
        if c.dict_(f"{p}.parity.counters", par["counters"]):
            for k, v in sorted(par["counters"].items()):
                c.int_(f"{p}.parity.counters.{k}", v)
    e = case["export"]
    if not c.keys(f"{p}.export", e, EXPORT_KEYS):
        return None
    c.equal(f"{p}.export.schema", e["schema"], EXPORT_SCHEMA)
    c.str_(f"{p}.export.identity_sha256", e["identity_sha256"])
    ident = e["identity"]
    if c.keys(f"{p}.export.identity", ident, IDENTITY_KEYS):
        for k in ("basis", "branch", "execution", "input", "enclosure_evidence", "operator_fingerprint",
                  "adjoint_proof_version"):
            c.str_(f"{p}.export.identity.{k}", ident[k])
        c.int_(f"{p}.export.identity.degree", ident["degree"])
        c.int_(f"{p}.export.identity.adjoint_depth", ident["adjoint_depth"])
        c.int_(f"{p}.export.identity.normalization_shift", ident["normalization_shift"], minimum=None)
        d["identity"] = {k: _hex(c, f"{p}.export.identity.{k}", ident[k]) for k in IDENTITY_HEX}
        if c.list_(f"{p}.export.identity.envelope_key_digests", ident["envelope_key_digests"]):
            for i, s in enumerate(ident["envelope_key_digests"]):
                c.str_(f"{p}.export.identity.envelope_key_digests[{i}]", s)
    terms = e["terms"]
    d["terms"] = []
    if c.list_(f"{p}.export.terms", terms):
        for i, t in enumerate(terms):
            tp = f"{p}.export.terms[{i}]"
            if not c.keys(tp, t, TERM_KEYS):
                continue
            c.str_(f"{tp}.identity_sha256", t["identity_sha256"])
            c.int_(f"{tp}.column", t["column"])
            row = {"column": t["column"]}
            for f in TERM_FIELDS:
                row[f] = _hex_or_null(c, f"{tp}.{f}", t[f])
            row["recurrence_majorant"] = _hex_or_null(c, f"{tp}.recurrence_majorant", t["recurrence_majorant"])
            d["terms"].append(row)
    asm = e["assembly"]
    if c.keys(f"{p}.export.assembly", asm, ASSEMBLY_KEYS):
        c.str_(f"{p}.export.assembly.identity_sha256", asm["identity_sha256"])
        d["fused_summation"] = _hex_or_null(c, f"{p}.export.assembly.fused_summation", asm["fused_summation"])
        d["assembly_total"] = _hex(c, f"{p}.export.assembly.laguerre_adjoint_total", asm["laguerre_adjoint_total"])
    d["adjoint"] = []
    if c.list_(f"{p}.export.adjoint", e["adjoint"]):
        for i, a in enumerate(e["adjoint"]):
            ap = f"{p}.export.adjoint[{i}]"
            if not c.keys(ap, a, ADJOINT_KEYS):
                continue
            c.str_(f"{ap}.identity_sha256", a["identity_sha256"])
            c.int_(f"{ap}.column", a["column"])
            row = {"column": a["column"],
                   "coefficients": _hex_list(c, f"{ap}.coefficients", a["coefficients"]),
                   "envelopes": _hex_list(c, f"{ap}.envelopes", a["envelopes"]),
                   "residuals": _hex_list(c, f"{ap}.residuals", a["residuals"]),
                   "scale_abs": _hex_or_null(c, f"{ap}.scale_abs", a["scale_abs"])}
            key = a["envelope_key"]
            row["envelope_key"] = key
            if c.keys(f"{ap}.envelope_key", key, KEY_KEYS):
                for k in ("degree", "depth", "degree_limit", "max_depth"):
                    c.int_(f"{ap}.envelope_key.{k}", key[k])
                c.str_(f"{ap}.envelope_key.coefficients_sha256", key["coefficients_sha256"])
                c.str_(f"{ap}.envelope_key.proof_version", key["proof_version"])
                _hex(c, f"{ap}.envelope_key.extent_bits", key["extent_bits"])
            d["adjoint"].append(row)
    return d


def _bind_case(c, p: str, case: dict, d: dict, pub: dict) -> None:
    """The registered binding to the published case and to the registered construction."""
    label = case["label"]
    pn = pub.get("n")
    if case["n"] != pn:
        c.fail(f"{p}.n", f"{case['n']} != published {pn}")
        return
    if case["a"] != pub.get("a"):
        c.fail(f"{p}.a", "A bits differ from the published case")
    if case["h"] != pub.get("h"):
        c.fail(f"{p}.h", "h bits differ from the published case")
    if case["w"] != pub.get("w"):
        c.fail(f"{p}.w", "w bits differ from the published case")
    r = case["report"]
    if r["degree"] != pub.get("degree"):
        c.fail(f"{p}.report.degree", f"{r['degree']} != published {pub.get('degree')}")
    if d.get("laguerre_scale") is None or d["laguerre_scale"] != pub.get("laguerre_scale"):
        c.fail(f"{p}.report.laguerre_scale", f"!= published {pub.get('laguerre_scale')}")
    if r["normalization_shift"] != pub.get("normalization_shift"):
        c.fail(f"{p}.report.normalization_shift",
               f"{r['normalization_shift']} != published {pub.get('normalization_shift')}")
    if r["fused"] != pub.get("fused"):
        c.fail(f"{p}.report.fused", "fused bits differ from the published case")
    total = (pub.get("coverage") or {}).get("total")
    if type(total) is not float or d.get("total") is None or hx(d["total"]) != hx(total):
        c.fail(f"{p}.report.laguerre_adjoint_total", f"bits differ from the published coverage.total {total!r}")
    budget = pub.get("budget")
    if type(budget) is not float or d.get("admission_budget") != budget:
        c.fail(f"{p}.admission_budget", f"!= published budget {budget!r}")
    # The export's own copies agree with the report.
    e = case["export"]
    ident = e["identity"]
    if d.get("assembly_total") is None or d.get("total") is None or hx(d["assembly_total"]) != hx(d["total"]):
        c.fail(f"{p}.export.assembly.laguerre_adjoint_total", "differs from the report total")
    for k, v in (("degree", r["degree"]), ("normalization_shift", r["normalization_shift"])):
        if ident[k] != v:
            c.fail(f"{p}.export.identity.{k}", f"{ident[k]!r} != report {v!r}")
    if ident["laguerre_scale"] != r["laguerre_scale"]:
        c.fail(f"{p}.export.identity.laguerre_scale", "differs from the report")
    # The registered construction.
    rho = registered_rho(label)
    rho_enc = 1.2 * rho * (1.0 + 1.0e-12)
    expected = {"basis": "laguerre", "branch": "recurrence", "execution": "certified-enclosures",
                "input": registered_input(label), "enclosure_evidence": "gershgorin",
                "adjoint_proof_version": PROOF_VERSION, "adjoint_depth": ADJOINT_DEPTH,
                "h": case["h"], "enclosure_lambda": hx(0.0), "enclosure_rho": hx(rho_enc),
                "truncation_budget": hx(registered_truncation_budget(label))}
    for k, v in expected.items():
        if ident[k] != v:
            c.fail(f"{p}.export.identity.{k}", f"{ident[k]!r} != registered {v!r}")
    if case["input"] != registered_input(label):
        c.fail(f"{p}.input", f"{case['input']!r} != registered {registered_input(label)!r}")
    if d.get("rho_nominal") != rho:
        c.fail(f"{p}.rho_nominal", f"!= registered {rho}")
    scales = list(SCALES) if registered_input(label) == "same-vector" else [1.0] * TERMS
    if d.get("scales") is None or [hx(x) for x in d["scales"]] != [hx(x) for x in scales]:
        c.fail(f"{p}.scales", f"!= registered {scales}")
    if d.get("a") is not None and ident["operator_fingerprint"] != operator_fingerprint(d["a"], 0.0, rho_enc):
        c.fail(f"{p}.export.identity.operator_fingerprint", "differs from the published A and the enclosure")


def validate(published_path: Path, pp09_path: Path, components_path: Path,
             blob_at=None, tree_status=None) -> evidence.Evidence:
    """Every INVALID condition; returns the validated documents (read once)."""
    blob_at = blob_at or checker_blob_at
    tree_status = tree_status or own_tree_status
    docs, digests, paths = evidence._load_all([
        ("published", published_path, PUBLISHED_SHA256),
        ("pp09", pp09_path, PP09_SHA256),
        ("components", components_path, None),
    ])
    c = evidence._Checks()
    comp = docs["components"]
    published = docs["published"]
    pub_index: dict = {}
    if c.dict_("published", published) and c.list_("published.cases", published.get("cases")):
        pub_index = c.unique_index("published.cases", [x for x in published["cases"] if isinstance(x, dict)],
                                   lambda r: (r.get("label"),))
    if not c.keys("components", comp, COMPONENTS_KEYS):
        c.done()
    c.equal("components.schema", comp["schema"], COMPONENTS_SCHEMA)
    c.equal("components.node", comp["node"], NODE)
    c.equal("components.published", comp["published"], {"path": PUBLISHED_PATH, "sha256": PUBLISHED_SHA256})
    if c.str_("components.source_commit", comp["source_commit"]) and (
            len(comp["source_commit"]) != 40 or any(ch not in "0123456789abcdef" for ch in comp["source_commit"])):
        c.fail("components.source_commit", "not a 40-digit commit id")
    if c.str_("components.tree_status", comp["tree_status"], nonempty=False) and comp["tree_status"] != "":
        c.fail("components.tree_status", f"the export tree was dirty: {comp['tree_status'][:200]!r}")
    status = tree_status()
    if status != "":
        c.fail("checker.tree_status", f"the checker tree is dirty: {status[:200]!r}")
    if isinstance(comp.get("source_commit"), str) and len(comp["source_commit"]) == 40:
        try:
            if blob_at(comp["source_commit"]) != Path(__file__).read_bytes():
                c.fail("provenance", f"the checker blob at {comp['source_commit']} differs from the checker run")
        except evidence.ValidationError as exc:
            c.fail("provenance", exc.reasons[0])
    decoded: dict = {}
    if c.list_("components.cases", comp["cases"]):
        raw = comp["cases"]
        labels = [x.get("label") if isinstance(x, dict) else None for x in raw]
        index = c.unique_index("components.cases", [x if isinstance(x, dict) else {} for x in raw],
                               lambda r: (r.get("label"),))
        if len(raw) != len(CASES) or sorted(str(x) for x in labels) != sorted(CASES):
            c.fail("components.cases", f"labels {labels} are not exactly the nine registered cases")
        for (label,), case in index.items():
            if label not in CASES:
                continue
            p = f"components.cases[{label}]"
            d = _decode_case(c, p, case)
            if d is None:
                continue
            if (label,) not in pub_index:
                c.fail(p, "no published case with this label")
                continue
            before = len(c.reasons)
            _bind_case(c, p, case, d, pub_index[(label,)])
            if len(c.reasons) == before:
                decoded[label] = d
            # Stamps.
            e = case["export"]
            digest = canonical_identity_sha256(e["identity"]) if isinstance(e.get("identity"), dict) else None
            d["identity_digest"] = digest
            stamps = [("export.identity_sha256", e.get("identity_sha256")),
                      ("export.assembly.identity_sha256", (e.get("assembly") or {}).get("identity_sha256"))]
            stamps += [(f"export.terms[{i}].identity_sha256", t.get("identity_sha256"))
                       for i, t in enumerate(e.get("terms") or []) if isinstance(t, dict)]
            stamps += [(f"export.adjoint[{i}].identity_sha256", a.get("identity_sha256"))
                       for i, a in enumerate(e.get("adjoint") or []) if isinstance(a, dict)]
            for where, stamp in stamps:
                if stamp != digest:
                    c.fail(f"{p}.{where}", f"stamp {stamp!r} != recomputed identity {digest}")
        seen: dict = {}
        for (label,), case in index.items():
            ident = (case.get("export") or {}).get("identity") if isinstance(case.get("export"), dict) else None
            if isinstance(ident, dict):
                digest = canonical_identity_sha256(ident)
                if digest in seen:
                    c.fail(f"components.cases[{label}]", f"shares its identity with {seen[digest]}")
                seen[digest] = label
    c.done()
    docs = dict(docs)
    docs["decoded"] = decoded
    return evidence.Evidence("PY01", docs, digests, paths)


# --------------------------------------------------------------------------------------------- gate
def item_completeness(d: dict) -> dict:
    m = d["identity_degree"]
    problems = []
    columns = sorted(t["column"] for t in d["terms"])
    if columns != list(range(TERMS)):
        problems.append(f"term columns {columns} != 0..4")
    for t in d["terms"]:
        for f in TERM_FIELDS:
            v = t[f]
            if v is None or not math.isfinite(v) or v < 0.0:
                problems.append(f"column {t['column']} {f} = {v!r}")
    fs = d.get("fused_summation")
    if fs is None or not math.isfinite(fs) or fs < 0.0:
        problems.append(f"fused_summation = {fs!r}")
    adj_columns = sorted(a["column"] for a in d["adjoint"])
    if adj_columns != list(range(TERMS)):
        problems.append(f"adjoint columns {adj_columns} != 0..4")
    for a in d["adjoint"]:
        k = a["column"]
        for name, count, nonneg in (("coefficients", m + 1, False), ("envelopes", m + 1, True),
                                    ("residuals", m, True)):
            values = a[name]
            if values is None or len(values) != count:
                problems.append(f"column {k} has {None if values is None else len(values)} {name}, needs {count}")
                continue
            bad = [i for i, v in enumerate(values) if v is None or not math.isfinite(v) or (nonneg and v < 0.0)]
            if bad:
                problems.append(f"column {k} {name} invalid at {bad[:5]}")
        s = a["scale_abs"]
        if s is None or not math.isfinite(s) or s < 0.0:
            problems.append(f"column {k} |s_k| = {s!r}")
    return {"pass": not problems, "problems": problems}


def by_column(rows: list) -> dict:
    return {r["column"]: r for r in rows}


def item_enclosure(d: dict, n: int, shift: int, total: float) -> dict:
    terms = by_column(d["terms"])
    try:
        s = d["fused_summation"]
        for k in range(TERMS):
            for f in ("truncation", "coefficient", "summation"):
                s = add_up(s, terms[k][f])
        for k in range(TERMS):
            s = add_up(s, terms[k]["recurrence_adjoint"])
        for k in range(TERMS):
            s = add_up(s, terms[k]["normalization"])
    except (TypeError, KeyError, ArithmeticError) as exc:
        return {"pass": False, "problem": f"the upward sum cannot be formed: {exc}"}
    out = {"upward_sum": hx(s), "upward_sum_f64": s, "native_total": hx(total), "native_total_f64": total,
           "shift": shift}
    if shift == 0:
        out["rule"] = "in window: bitwise equal"
        out["pass"] = hx(s) == hx(total)
    else:
        excess = Fraction(s) - Fraction(total) * (1 + OUT_OF_WINDOW_RELATIVE) - OUT_OF_WINDOW_ULPS * TINY
        # excess <= 5 sqrt(n) 2^-1074, exactly.
        within = excess <= 0 or excess * excess <= 25 * n * TINY * TINY
        out["rule"] = "out of window: total <= sum <= total (1 + 1e-12) + (5 sqrt n + 64) 2^-1074"
        out["sum_minus_total_over_2^-1074"] = sci((Fraction(s) - Fraction(total)) / TINY, 12)
        out["allowance_over_2^-1074_excluding_5sqrt_n"] = sci(
            (Fraction(total) * OUT_OF_WINDOW_RELATIVE) / TINY + OUT_OF_WINDOW_ULPS, 12)
        out["five_sqrt_n"] = 5.0 * math.sqrt(n)
        out["pass"] = s >= total and within
    return out


def item_source_binding(d: dict, shift: int) -> dict:
    terms = by_column(d["terms"])
    m = d["identity_degree"]
    extent_hex = d["identity_raw"]["laguerre_extent"]
    rows, ok = [], True
    for a in sorted(d["adjoint"], key=lambda r: r["column"]):
        k = a["column"]
        row = {"column": k}
        try:
            beta, eps = a["envelopes"], a["residuals"]
            if len(beta) != m + 1 or len(eps) != m:
                raise ValueError("counts")
            acc = 0.0
            for j in range(1, m + 1):
                acc = add_up(acc, mul_up(beta[j], eps[j - 1]))
            value = mul_up(a["scale_abs"], acc)
            if shift != 0:
                value = scale_bound_up(value, shift)
            exported = terms[k]["recurrence_adjoint"]
            row["recomputed"] = hx(value)
            row["exported"] = None if exported is None else hx(exported)
            row["adjoint_equal"] = exported is not None and hx(value) == hx(exported)
        except (TypeError, KeyError, ValueError, ArithmeticError) as exc:
            row["adjoint_equal"] = False
            row["problem"] = f"cannot recompute: {exc}"
        key = a["envelope_key"]
        coefficients = a["coefficients"]
        digest = None
        if coefficients is not None and all(v is not None for v in coefficients):
            digest = envelope_key_digest(coefficients)
        row["key_digest_recomputed"] = digest
        row["key_equal"] = (digest is not None and key["coefficients_sha256"] == digest
                            and d["identity_digests"][k] == digest
                            and key["degree"] == m and key["extent_bits"] == extent_hex
                            and key["depth"] == ADJOINT_DEPTH and key["proof_version"] == PROOF_VERSION
                            and key["degree_limit"] == DEGREE_LIMIT and key["max_depth"] == MAX_DEPTH)
        registered_scale = abs(SCALES[k]) if d["input"] == "same-vector" else 1.0
        row["scale_registered"] = a["scale_abs"] is not None and hx(a["scale_abs"]) == hx(registered_scale)
        ok &= row["adjoint_equal"] and row["key_equal"] and row["scale_registered"]
        rows.append(row)
    if len(rows) != TERMS:
        ok = False
    return {"pass": ok, "columns": rows}


# ----------------------------------------------------------------------------------------- reported
def _tuple_fraction(t: tuple) -> Fraction:
    """The exact rational value of a raw mpmath mpf tuple (sign, mantissa, exponent, bitcount)."""
    sign, man, exp, _ = t
    if man == 0:
        if t != (0, 0, 0, 0):
            raise ArithmeticError(f"non-finite interval endpoint {t!r}")
        return Fraction(0)
    value = Fraction(int(man)) * (Fraction(2) ** int(exp))
    return -value if sign else value


def sampled_lower_bounds(task: tuple) -> list[tuple[int, int]]:
    """l_j = max over the sample points x_i = i L' / (points - 1) of the lower end of the mpmath.iv enclosure of
    |z_j(x_i)|, j = 0..m, by the backward recurrence of laguerre_adjoint.rs
    (z_j = c_j + (2j+1-x)/(j+1) z_(j+1) - (j+1)/(j+2) z_(j+2), z_(m+1) = z_(m+2) = 0) on the exact stored
    coefficients. Returned exactly as (numerator, denominator) pairs."""
    coefficients_hex, extent_hex, points, dps = task
    from mpmath import iv
    iv.dps = dps
    c = [iv.mpf(unhex(x)) for x in coefficients_hex]
    m = len(c) - 1
    extent = iv.mpf(unhex(extent_hex))
    alpha = [iv.mpf(2 * j + 1) for j in range(m + 1)]
    damp = [iv.mpf(j + 1) / (j + 2) for j in range(m + 1)]
    best = [Fraction(0)] * (m + 1)
    for i in range(points):
        x = extent * i / (points - 1)
        z1 = iv.mpf(0)
        z2 = iv.mpf(0)
        for j in range(m, -1, -1):
            z = c[j] + (alpha[j] - x) / (j + 1) * z1 - damp[j] * z2
            lo, hi = (_tuple_fraction(t) for t in z._mpi_)
            low = lo if lo > 0 else (-hi if hi < 0 else Fraction(0))
            if low > best[j]:
                best[j] = low
            z2, z1 = z1, z
    return [(b.numerator, b.denominator) for b in best]


def compute_lower_bounds(decoded: dict) -> dict:
    tasks, keys = [], []
    for label in CASES:
        d = decoded[label]
        for a in sorted(d["adjoint"], key=lambda r: r["column"]):
            tasks.append(([hx(x) for x in a["coefficients"]], d["identity_raw"]["laguerre_extent"], SAMPLE_POINTS,
                          DPS))
            keys.append((label, a["column"]))
    workers = max(1, min(WORKERS, os.cpu_count() or 1))
    if workers > 1:
        import multiprocessing
        with multiprocessing.get_context("spawn").Pool(workers) as pool:
            results = pool.map(sampled_lower_bounds, tasks, chunksize=1)
    else:
        results = [sampled_lower_bounds(t) for t in tasks]
    return {key: [Fraction(nu, de) for nu, de in r] for key, r in zip(keys, results)}


def reported(decoded: dict, pp09: dict, published: dict, lower: dict | None) -> dict:
    pp09_rows = {}
    for row in ((pp09.get("g3") or {}).get("rows") or []):
        if isinstance(row, dict) and isinstance(row.get("derived_supplementary"), dict):
            pp09_rows[row.get("label")] = row["derived_supplementary"]
    cases, predictions = {}, {"terms_match_pp09": {}, "summation_plus_fused_below_1e-12_S": {},
                              "adjoint_share_at_least_0.999999": {}}
    for label in CASES:
        d = decoded[label]
        shift = d["shift"]
        terms = by_column(d["terms"])
        sums = {f: sum((Fraction(terms[k][f]) for k in range(TERMS)), Fraction(0)) for f in TERM_FIELDS}
        fused = Fraction(d["fused_summation"])
        dag = {"truncation": sums["truncation"], "coefficient": sums["coefficient"],
               "recurrence_adjoint": sums["recurrence_adjoint"], "summation": sums["summation"],
               "assembly": fused + sums["normalization"]}
        s_total = sum(dag.values(), Fraction(0))
        shares = {k: (v / s_total if s_total else None) for k, v in dag.items()}
        bottleneck = max(dag, key=lambda k: dag[k])
        budget = d["admission_budget"]
        non_adjoint = s_total - dag["recurrence_adjoint"]
        columns, floor_adjoint = [], Fraction(0)
        for a in sorted(d["adjoint"], key=lambda r: r["column"]):
            k = a["column"]
            beta, eps, m = a["envelopes"], a["residuals"], len(a["residuals"])
            products = [Fraction(beta[j]) * Fraction(eps[j - 1]) for j in range(1, m + 1)]
            j_star = 1 + max(range(m), key=lambda i: products[i]) if m else None
            col = {"column": k, "max_beta": sci(max(Fraction(b) for b in beta), 12),
                   "argmax_beta_j": max(range(m + 1), key=lambda j: beta[j]),
                   "j_of_largest_beta_eps": j_star,
                   "largest_beta_eps_share_of_column_sum": (
                       sci(products[j_star - 1] / sum(products, Fraction(0)), 8) if m and sum(products) else None),
                   "recurrence_adjoint": sci(Fraction(terms[k]["recurrence_adjoint"]), 12)}
            if lower is not None:
                lj = lower[(label, k)]
                col["l_j_hex_rounded_down"] = [hx(floor_f64(x)) for x in lj]
                col["l_j"] = [sci(x, 17) for x in lj]
                ratios = [(Fraction(beta[j]) / lj[j]) if lj[j] > 0 else None for j in range(m + 1)]
                finite_ratios = [r for r in ratios[1:] if r is not None]
                col["beta_over_l"] = [None if r is None else sci(r, 8) for r in ratios]
                col["beta_over_l_max_j>=1"] = sci(max(finite_ratios), 8) if finite_ratios else None
                col["beta_over_l_min_j>=1"] = sci(min(finite_ratios), 8) if finite_ratios else None
                col["beta_over_l_at_largest_product"] = (
                    None if j_star is None or ratios[j_star] is None else sci(ratios[j_star], 8))
                col["l_zero_count_j>=1"] = sum(1 for r in ratios[1:] if r is None)
                sampled = sum((lj[j] * Fraction(eps[j - 1]) for j in range(1, m + 1)), Fraction(0))
                sampled *= Fraction(a["scale_abs"]) * pow2(shift)
                col["sampled_adjoint_floor"] = sci(sampled, 12)
                col["adjoint_over_sampled_floor"] = (
                    sci(Fraction(terms[k]["recurrence_adjoint"]) / sampled, 8) if sampled else None)
                floor_adjoint += sampled
            columns.append(col)
        entry = {
            "gated": label in GATED, "degree": d["identity_degree"], "shift": shift,
            "native_total": sci(Fraction(d["total"]), 12), "admission_budget": budget,
            "terms_exact": {k: sci(v, 12) for k, v in dag.items()}, "S": sci(s_total, 15),
            "shares": {k: sci(v, 8) for k, v in shares.items()}, "bottleneck": bottleneck,
            "per_column_terms": [{f: sci(Fraction(terms[k][f]), 8) for f in TERM_FIELDS} | {
                "recurrence_majorant_diagnostic": sci(terms[k]["recurrence_majorant"], 8)} for k in range(TERMS)],
            "fused_summation": sci(fused, 8), "N_non_adjoint": sci(non_adjoint, 12), "columns": columns,
        }
        if lower is not None:
            f_floor = non_adjoint + floor_adjoint
            entry["closability_floor_F"] = sci(f_floor, 12)
            entry["F_over_budget"] = sci(f_floor / Fraction(budget), 8)
            entry["F_at_most_budget"] = f_floor <= Fraction(budget)
            entry["total_over_F"] = sci(Fraction(d["total"]) / f_floor, 8) if f_floor else None
        if label in GATED:
            derived = pp09_rows.get(label, {})
            ours = {"truncation": sci(sums["truncation"], 8), "coefficient": sci(sums["coefficient"], 8)}
            match = {k: ours[k] == derived.get(k) for k in ours}
            entry["pp09_comparison"] = {"ours_8_digits": ours,
                                        "pp09_derived": {k: derived.get(k) for k in ours}, "equal": match}
            predictions["terms_match_pp09"][label] = all(match.values())
            predictions["summation_plus_fused_below_1e-12_S"][label] = (
                dag["summation"] + fused) < PRED_SUMMATION_SHARE * s_total
            predictions["adjoint_share_at_least_0.999999"][label] = (
                dag["recurrence_adjoint"] >= PRED_ADJOINT_SHARE * s_total)
        cases[label] = entry
    for name in list(predictions):
        predictions[name] = {"cases": predictions[name], "holds": all(predictions[name].values())}
    return {"cases": cases, "predictions": predictions}


# --------------------------------------------------------------------------------------------- main
def evaluate(validated: evidence.Evidence, with_lower_bounds: bool = True) -> dict:
    comp = validated.docs["components"]
    raw = {c["label"]: c for c in comp["cases"]}
    decoded = validated.docs["decoded"]
    gate_cases, all_pass = {}, True
    for label in CASES:
        case, d = raw[label], decoded[label]
        ident = case["export"]["identity"]
        d["identity_degree"] = ident["degree"]
        d["identity_raw"] = ident
        d["identity_digests"] = ident["envelope_key_digests"] + [None] * TERMS
        d["input"] = ident["input"]
        d["shift"] = case["report"]["normalization_shift"]
        i1 = item_completeness(d)
        i2 = item_enclosure(d, case["n"], d["shift"], d["total"]) if i1["pass"] else {
            "pass": False, "problem": "not evaluated: item 1 fails"}
        i3 = item_source_binding(d, d["shift"]) if i1["pass"] else {
            "pass": False, "problem": "not evaluated: item 1 fails"}
        gate_cases[label] = {"1_completeness": i1, "2_enclosure": i2, "3_source_binding": i3,
                             "parity_reported": case["parity"]}
        all_pass &= i1["pass"] and i2["pass"] and i3["pass"]
    items = {name: all(gate_cases[label][name]["pass"] for label in CASES)
             for name in ("1_completeness", "2_enclosure", "3_source_binding")}
    verdict = "PASS" if all_pass else "FAIL"
    rep = None
    if items["1_completeness"]:
        lower = compute_lower_bounds(decoded) if with_lower_bounds else None
        rep = reported(decoded, validated.docs["pp09"], validated.docs["published"], lower)
        rep["predictions"]["verdict_PASS"] = {"holds": verdict == "PASS"}
    return {"verdict": verdict, "gate": items, "cases": gate_cases, "reported": rep}


def main(argv: list[str] | None = None, blob_at=None, tree_status=None, head=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--published", required=True, type=Path)
    parser.add_argument("--pp09", required=True, type=Path)
    parser.add_argument("--components", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args(argv)
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    inputs = {"published": args.published, "pp09": args.pp09, "components": args.components}
    try:
        validated = validate(args.published, args.pp09, args.components, blob_at, tree_status)
    except evidence.ValidationError as exc:
        return evidence.emit_invalid(args.output, SCHEMA, exc, inputs)
    result = evaluate(validated)
    comp = validated.docs["components"]
    try:
        from mpmath import __version__ as mpmath_version
    except ImportError:  # pragma: no cover
        mpmath_version = None
    report = {
        "schema": SCHEMA,
        "evidence_schema": evidence.VERSION,
        "node": NODE,
        "verdict": result["verdict"],
        "gate": result["gate"],
        "components_source_commit": comp["source_commit"],
        "components_tree_status": comp["tree_status"],
        "checker_head": (head or own_head)(),
        "checker_tree_status": (tree_status or own_tree_status)(),
        "inputs": {role: {"path": str(p), "sha256": validated.sha256[role]} for role, p in validated.paths.items()},
        "environment": {"python": sys.version.split()[0], "mpmath": mpmath_version, "dps": DPS,
                        "sample_points": SAMPLE_POINTS,
                        "workers": max(1, min(WORKERS, os.cpu_count() or 1))},
        "cases": result["cases"],
        "reported": result["reported"],
        "claim_scope": "Attribution of the existing native Laguerre totals of nine registered cases only: no bound "
                       "is changed, no admission changes, no routing, wall-time or default-promotion claim. F is "
                       "a floor for envelopes of the same z_j with the same residual bounds, not a bound.",
    }
    args.output.write_text(json.dumps(report, indent=1, sort_keys=True, allow_nan=False) + "\n")
    for k, v in result["gate"].items():
        print(f"{k}: {'PASS' if v else 'FAIL'}")
    print(f"verdict: {result['verdict']}")
    return 0 if result["verdict"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
