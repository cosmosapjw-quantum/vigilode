#!/usr/bin/env python3
"""Checker of research node AS05 (research/as05_residual_gain_core_20261011/PREREGISTRATION.md).

Evidence first (verdict INVALID, exit 2, before any gate): the inputs pass the fail-closed rules of
``tools/evidence_schema_v2.py`` (AS03: strict JSON, the re-audit NATIVE.json bound to its registered SHA-256) and the
AS05 schema defined here:
- the pins: NATIVE.json ``b9404714...``, gain_witness.rs ``22c97719...``, check.py ``238d52a2...``, re-audit
  RESULTS.json ``20a6cb32...`` (files on disk) and the gain_witness.rs digest recorded by the export;
- exact key sets, canonical binary64 hex, the registered case-ID set (307 IDs, no duplicate) and group counts;
- every input hex decodes to the registered input, regenerated here from NATIVE.json, the RODAS5P gamma of
  ``fixtures/rodas5p_coefficients_snapshot.json`` and the registration (no Rust value is trusted);
- the research reference intervals of the 30 cells A equal NATIVE.json bit for bit;
- the W-formation family (16 members): the exact rule of section C decides discrimination (Python ``Fraction``:
  the exact shifted-W error against the exported research ``GainWitness2`` enclosure, endpoints as exact binary64
  rationals); the export's flags and the fixture list must equal this decision, and at least 4 members must
  discriminate (a fixture-design defect otherwise);
- provenance: the RUNS and RECEIPTS source commit is the same, every recorded tree status is clean, and the checker
  blob at that commit is byte-identical to the checker that runs (so it does not postdate the run).

Gate (registered; PASS iff every item holds, otherwise FAIL with every number kept; exit 0 PASS, 1 FAIL):
1. Retained oracle: the 30 cells A are Verified and pass check.py's criteria recomputed here (component enclosure,
   wrms_upper^2 >= exact squared WRMS error, infinity bound, gain >= exact inverse gain); 11 of the 12 invalid
   controls are Unavailable with the mapped research class; ``invalid_determinant_overlap`` may be Verified (then it
   must enclose its oracle error) or Unavailable.
2. Parity: on every cell A every core interval (residual, correction, weighted correction) is contained in the
   research interval endpoint by endpoint, the core wrms_upper <= the research one and the core gain <= the research
   gain.
3. Negatives refused: no case of C is Verified and each has its registered class; the assembly and nu cases are
   Estimated; the route (d) negatives of D are not Verified and give NegativeShift / NonpositiveDenominator.
4. No false certificate: every Verified result of D (route (c) exact and delta with all 16 corners, the 60 transported
   certificates, route (b), route (d)) encloses the exact error in its stated metric, and is bound to its target.
5. Authority types: compile-fail doc tests (from_str::<VerifiedCorrection>, struct literals of VerifiedCorrection,
   LinearTargetId, ResidualGainWitness) exist in residual_gain.rs and passed in the doc-test receipt; no Deserialize
   derive or impl and no pub field on these types in residual_gain.rs.
6. Non-regression and isolation: the pre-existing rodas5p-core suite passed (receipt; including pp12_lognorm_decay,
   pp12b_chain_symmetrizer, int05_*, rev02_*, safe_enclosure_*); no pre-existing test file is modified; no file
   outside rodas5p-core references residual_gain.

Reported only (no usefulness threshold): bound/error per case (exact zero errors as null), core/research wrms ratios,
Estimated/Verified ratios against the LU comparator, route (d) 1 - a mu_up and mu_source, setup and verification flops
against the nominal LU flops, the vacuous corner checks, the projected-nu underestimation (F105), the transported gain.

Interpretations and deviations (fixed before the recorded run):
- I1  W-formation family: the right-hand side is b = (1, 1) (as in the research grid), the metric is unit (1, 1), and
      every witness and target carries model_epoch = 7 unless the case changes it.
- I2  Discrimination compares the exact shifted-W error with the research *weighted* correction enclosure (the
      certificate's enclosure in its stated, unit, metric; it contains the unweighted one). A member whose research
      witness rejects has no enclosure and does not discriminate.
- I3  A one-ulp change is next_up (towards +inf); the zero entry J[1][0] becomes 2^-1074.
- I4  Off-block coupling: the 4x4 base is blkdiag of discovery cells 1 and 2 (grid_k0_s0_p0, grid_k0_s0_p1), with
      their b, u and s; the dense 4x4 (route (a), UndeclaredStructure) is the same matrix declared Dense.
- I5  Route (b): n = 4, 6, 8 from the first 2, 3, 4 discovery cells in listed order; n = 3 is discovery cell 1 plus
      the 1x1 block W = 3, b = 1, u = fl(1/3), s = 1.
- I6  Route (d): h gamma is registered by value, so h = fl(1e-3) or fl(1e-1) and gamma = 1 (a = h exactly). First-order
      upwind convection with cell Peclet pe adds 100 pe (u_{k-1} - u_k): diagonal -200 - 100 pe, sub-diagonal
      100 + 100 pe, super-diagonal 100. b = (1, ..., 1); s_k = fl(1 + |sin k|) for k = 1..n with sin a faithful
      binary64 value (checked against mpmath); "coordinate 1" is the first coordinate, perturbed additively,
      u_1 = fl(lu_1 + 2^-30); assembly Exact. The negatives use n = 3: the Peclet-0 J with h = -1e-3, and 10 I_3 with
      h = 1, gamma = 1. Their candidates are formed the same way.
- I7  Projected nu: rhs e2, candidate 0 (so r = e2), Arnoldi start e1, at most n steps, unit metric; the estimate is
      nu ||D r||_2 / sqrt(n).
- I8  LinearTargetId::represented/shifted are part of the witness API (they validate and hash); the approximate-inverse
      route is a function returning Unavailable(RouteNotImplemented) (contract test, no RUNS case).
- I9  directed.rs and nonnormal_certificate.rs are unchanged: no helper was needed (symmetric_part_upper is pub).
- I10 Receipts: RECEIPTS.json (tools/as05_receipts.py) records registered commands 1 and 2 and
      ``cargo test --offline --locked -p rodas5p-core --all-targets`` at the RUNS commit on a clean tree; it is written
      outside the tree and then copied next to RUNS.json, so every recorded tree status is clean.
- I11 Item 6 isolation scope: Rust sources and Cargo manifests outside crates/rodas5p-core that contain the whole word
      residual_gain (git grep -w at the RUNS commit; the research prototype's identifier
      scaled_residual_gain_inf_upper is not a reference); text files (registration, checker, results) naming the node are not references. "No pre-existing test
      file is modified": git diff --name-status 8ce9bda..commit over crates/rodas5p-core/tests lists additions only.
- I12 Item 2 also compares the scalar gain (core <= research), as listed in B.
- I13 Item 4 also requires the Verified binding (operator SHA-256 recomputed here, metric bits, epoch) to equal the
      case target; item 1 the same for the cells A.
- I14 A route (c) delta case that is Unavailable has vacuous corner checks: reported, not passed. A route (d) positive
      that is Unavailable is reported (the registration gates only Verified results of D).
- I15 The overlap exception may be Unavailable with any class.
- I16 Flop counts: an interval add/sub counts 2 directed-primitive calls, mul/div 8, a scalar directed operation 1;
      route (d) setup excludes symmetric_part_upper's internal search. Reported only.
- I17 Disclosure: before this commit, a development export (scratch path, from the uncommitted tree that became
      commit A) and development receipts were used to debug the export and this checker's parsing (with a fake git
      layer). They are not recorded evidence; no case, input, threshold or gate was changed after them (the only
      checker change was the whole-word residual_gain match of I11, a parsing fix).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import struct
import subprocess
import sys
from fractions import Fraction as F
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence_schema_v2 as evidence  # noqa: E402  (fail-closed evidence validation, re-audit AS03)

SCHEMA = "vigilode-as05-residual-gain-check-v1"
RUNS_SCHEMA = "vigilode-as05-residual-gain-runs-v1"
RECEIPTS_SCHEMA = "vigilode-as05-receipts-v1"
REPO = Path(__file__).resolve().parent.parent
CHECKER_PATH = "tools/as05_residual_gain_check.py"
REGISTRATION_COMMIT = "8ce9bda"
SOURCE_PATH = "crates/rodas5p-core/src/residual_gain.rs"
COEFFICIENTS_PATH = "fixtures/rodas5p_coefficients_snapshot.json"

PINS = {
    "native": ("research/reaudit_accuracy_speed_20261010/NATIVE.json",
               "b9404714af837dfb099db564550e88127ff52a0c9341758862529261b85bd364"),
    "gain_witness": ("research/reaudit_accuracy_speed_20261010/gain_witness.rs",
                     "22c977199d3c60849db94227a973a5d83ffea381650043774e8510ad5e745c89"),
    "check_py": ("research/reaudit_accuracy_speed_20261010/check.py",
                 "238d52a2a41ca87debdd6a5bfc4b9421bccb95ba6c7170c7fb832fe281239b89"),
    "reaudit_results": ("research/reaudit_accuracy_speed_20261010/RESULTS.json",
                        "20a6cb32aa119142576bac4adbe836fbd6e13623cf4dd2c4c0fbda36fc0d0e21"),
}

EPOCH = 7
H_LIST = (0.1, 0.3, 0.7, 1.1, 1.3, 1.7, 1.9, 2.3)
K_LIST = (0.0, 1024.0)
GROUP_COUNTS = {
    "A_cell": 30, "A_invalid": 12, "C_metric": 60, "C_epoch": 60, "D_transport": 60, "C_w_formation": 4,
    "C_assembly": 4, "C_operator": 24, "D_shifted_exact": 4, "D_shifted_delta": 4, "C_offblock": 25, "C_nu": 2,
    "D_blocks": 4, "D_lognorm": 12, "D_lognorm_negative": 2,
}
TOTAL_CASES = 307
D_GROUPS = ("D_shifted_exact", "D_shifted_delta", "D_transport", "D_blocks", "D_lognorm", "D_lognorm_negative")
C_CLASS = {"C_w_formation": "TargetMismatch", "C_metric": "MetricMismatch", "C_epoch": "StaleEpoch",
           "C_operator": "OperatorMismatch"}
RESEARCH_CLASS = {
    "nonfinite_matrix": "NonfiniteMatrix",
    "nonpositive_or_nonfinite_scale": "InvalidMetric",
    "nonfinite_rhs": "NonfiniteRhs",
    "nonfinite_candidate": "NonfiniteCandidate",
    "nonfinite_intermediate_or_outward_endpoint": "ArithmeticOverflow",
    "determinant_interval_contains_zero": "DeterminantContainsZero",
}
OVERLAP_EXCEPTION = "invalid_determinant_overlap"
REJECTIONS = frozenset((
    "NonfiniteMatrix", "InvalidMetric", "InvalidShift", "InvalidAssembly", "InvalidStructure", "NonfiniteRhs",
    "NonfiniteCandidate", "DimensionMismatch", "ArithmeticOverflow", "DeterminantContainsZero", "OperatorMismatch",
    "TargetMismatch", "MetricMismatch", "StaleEpoch", "OffBlockCoupling", "UndeclaredStructure", "RouteNotApplicable",
    "RouteNotImplemented", "NegativeShift", "NonpositiveDenominator"))
ESTIMATE_SOURCES = frozenset(("ProjectedNu", "DenseLuComparator", "UndeclaredAssembly"))
ROUTES = frozenset(("represented", "shifted", "lognorm", "projected_nu"))
VERIFIED_ROUTES = frozenset(("represented_dense2", "represented_blocks", "shifted_enclosure", "lognorm"))
AUTHORITY_TYPES = ("VerifiedCorrection", "ResidualGainWitness", "LinearTargetId")
SUITE_REQUIRED = ("pp12_lognorm_decay", "pp12b_chain_symmetrizer", "int05_", "rev02_", "safe_enclosure_")

TOP_KEYS = ("schema", "source", "gain_witness_sha256", "gamma_hex", "model_epoch", "w_formation_family", "fixtures",
            "cases")
CASE_KEYS = ("id", "group", "route", "witness", "target", "rhs_hex", "candidate_hex", "evidence", "comparator",
             "research", "note")
BINDING_KEYS = ("operator", "metric_hex", "model_epoch", "assembly", "structure")
EVIDENCE_KEYS = ("kind", "rejection", "estimate", "verified")
VERIFIED_KEYS = ("route", "residual_hex", "correction_hex", "weighted_correction_hex", "weighted_inf_upper_hex",
                 "wrms_upper_hex", "gain_hex", "lognorm", "setup_flops", "verification_flops", "transport_factor_hex",
                 "bound_operator_sha256", "bound_metric_hex", "bound_model_epoch")
MEMBER_KEYS = ("id", "h_hex", "k_hex", "gamma_hex", "lambda_hex", "j_hex", "w_rep_hex", "rhs_hex", "candidate_hex",
               "research", "discriminates", "fixture")
HEX = re.compile(r"^[0-9a-f]{16}$")


# ------------------------------------------------------------------------------------------- helpers
def hx(x: float) -> str:
    return struct.pack(">d", x).hex()


def unhex(text) -> float:
    """Canonical 16-digit binary64 hex, finite or not (invalid controls carry NaN/inf inputs)."""
    if not isinstance(text, str) or not HEX.match(text):
        raise evidence.ValidationError([f"not a 16-digit lowercase binary64 hex string: {text!r}"])
    return struct.unpack(">d", bytes.fromhex(text))[0]


def fin(text) -> float:
    return evidence.decode_hex64(text)


def next_up(x: float) -> float:
    return math.nextafter(x, math.inf)


def rust_display(x: float) -> str:
    """Rust's ``{}`` rendering of the registered parameters (integral values without a fraction)."""
    return str(int(x)) if x == int(x) else repr(x)


def rust_exp(x: float) -> str:
    """Rust's ``{:e}`` rendering of 1e-3 and 1e-1."""
    return {1.0e-3: "1e-3", 1.0e-1: "1e-1"}[x]


def sha256_file(path: Path) -> str:
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


# ---------------------------------------------------------------------------------- exact oracle
def solve(w: list[list[F]], b: list[F]) -> list[F] | None:
    """Exact Gaussian elimination (any nonzero pivot); None if singular."""
    n = len(b)
    a = [row[:] + [b[i]] for i, row in enumerate(w)]
    for k in range(n):
        p = next((i for i in range(k, n) if a[i][k] != 0), None)
        if p is None:
            return None
        a[k], a[p] = a[p], a[k]
        for i in range(k + 1, n):
            if a[i][k] != 0:
                m = a[i][k] / a[k][k]
                for j in range(k, n + 1):
                    a[i][j] -= m * a[k][j]
    x = [F(0)] * n
    for i in reversed(range(n)):
        x[i] = (a[i][n] - sum(a[i][j] * x[j] for j in range(i + 1, n))) / a[i][i]
    return x


def inverse(w: list[list[F]]) -> list[list[F]] | None:
    n = len(w)
    cols = []
    for j in range(n):
        e = [F(int(i == j)) for i in range(n)]
        c = solve(w, e)
        if c is None:
            return None
        cols.append(c)
    return [[cols[j][i] for j in range(n)] for i in range(n)]


def exact_operator(binding: dict, corner: list[F] | None = None) -> list[list[F]]:
    """The exact W of a binding (shifted: I - h gamma (J + corner))."""
    op = binding["operator"]
    if op["kind"] == "represented":
        return [[F(v) for v in row] for row in op["w"]]
    hg = F(op["h"]) * F(op["gamma"])
    n = len(op["j"])
    out = []
    for i in range(n):
        row = []
        for k in range(n):
            jik = F(op["j"][i][k]) + (corner[i * n + k] if corner else 0)
            row.append(F(int(i == k)) - hg * jik)
        out.append(row)
    return out


def corners(binding: dict) -> list[list[F] | None]:
    a = binding["assembly"]
    if a["kind"] != "entrywise_abs":
        return [None]
    n = len(binding["operator"]["j"])
    delta = F(a["delta"])
    out = []
    for mask in range(2 ** (n * n)):
        out.append([delta if (mask >> e) & 1 else -delta for e in range(n * n)])
    return out


def operator_sha256(binding: dict) -> str:
    """The core target hash, recomputed: tag, n, entry bits (row-major, LE), then h and gamma bits."""
    op = binding["operator"]
    h = hashlib.sha256()
    h.update(b"vigilode-as05-linear-target-v1\0")
    m = op["w"] if op["kind"] == "represented" else op["j"]
    h.update(bytes([0 if op["kind"] == "represented" else 1]))
    h.update(struct.pack("<Q", len(m)))
    for row in m:
        for v in row:
            h.update(struct.pack("<d", v))
    if op["kind"] == "shifted":
        h.update(struct.pack("<d", op["h"]))
        h.update(struct.pack("<d", op["gamma"]))
    return h.hexdigest()


def exact_errors(binding: dict, rhs: list[float], x: list[float], corner=None) -> list[F] | None:
    w = exact_operator(binding, corner)
    u = solve(w, [F(v) for v in rhs])
    if u is None:
        return None
    return [u[i] - F(x[i]) for i in range(len(x))]


def enclosure_check(errors: list[F], scales: list[float], ver: dict) -> dict:
    """check.py's criteria for one exact error vector against one Verified record (decoded)."""
    n = len(errors)
    weighted = [errors[i] / F(scales[i]) for i in range(n)]
    sq = sum(v * v for v in weighted) / n
    inf = max(abs(v) for v in weighted)
    out = {
        "wrms_ok": F(ver["wrms_upper"]) ** 2 >= sq and ver["wrms_upper"] >= 0,
        "inf_ok": F(ver["weighted_inf_upper"]) >= inf,
        "component_ok": True,
        "exact_wrms_error": math.sqrt(float(sq)),
        "zero_error": sq == 0,
    }
    if ver["weighted_correction"] is not None:
        out["component_ok"] = all(F(lo) <= e <= F(hi) for (lo, hi), e in zip(ver["weighted_correction"], weighted))
    if ver["correction"] is not None:
        out["component_ok"] = out["component_ok"] and all(
            F(lo) <= e <= F(hi) for (lo, hi), e in zip(ver["correction"], errors))
    out["pass"] = out["wrms_ok"] and out["inf_ok"] and out["component_ok"]
    return out


def exact_gain(binding: dict, scales: list[float]) -> F | None:
    inv = inverse(exact_operator(binding))
    if inv is None:
        return None
    n = len(scales)
    return max(sum(abs(inv[i][j]) * F(scales[j]) / F(scales[i]) for j in range(n)) for i in range(n))


# ------------------------------------------------------------------------------- registered cases
def gamma_from_snapshot(repo: Path) -> float:
    return float(json.loads((repo / COEFFICIENTS_PATH).read_text())["gamma"])


def binding(op: dict, metric: list[float], epoch=EPOCH, assembly=None, blocks=None) -> dict:
    return {"operator": op, "metric": list(metric), "model_epoch": epoch,
            "assembly": assembly or {"kind": "exact"}, "blocks": blocks}


def represented(w, metric, **kw) -> dict:
    return binding({"kind": "represented", "w": [list(r) for r in w]}, metric, **kw)


def shifted(j, h, gamma, metric, **kw) -> dict:
    return binding({"kind": "shifted", "j": [list(r) for r in j], "h": h, "gamma": gamma}, metric, **kw)


def binding_hex(b: dict) -> dict:
    """The export's JSON form of a binding."""
    op = b["operator"]
    if op["kind"] == "represented":
        ojson = {"kind": "represented", "w_hex": [[hx(v) for v in r] for r in op["w"]]}
    else:
        ojson = {"kind": "shifted", "j_hex": [[hx(v) for v in r] for r in op["j"]], "h_hex": hx(op["h"]),
                 "gamma_hex": hx(op["gamma"])}
    a = b["assembly"]
    ajson = {"kind": "entrywise_abs", "delta_hex": hx(a["delta"])} if a["kind"] == "entrywise_abs" else dict(a)
    sjson = ({"kind": "dense"} if b["blocks"] is None
             else {"kind": "isolated_blocks", "ranges": [list(r) for r in b["blocks"]]})
    return {"operator": ojson, "metric_hex": [hx(v) for v in b["metric"]], "model_epoch": b["model_epoch"],
            "assembly": ajson, "structure": sjson}


def native_value(v) -> float:
    if isinstance(v, str):
        return {"NaN": math.nan, "+Infinity": math.inf, "-Infinity": -math.inf}[v]
    return float(v)


def native_cells(native: dict, key: str) -> list[dict]:
    out = []
    for c in native["gain_study"][key]:
        out.append({"id": c["id"], "partition": c["partition"],
                    "w": [[native_value(v) for v in r] for r in c["W"]], "b": [native_value(v) for v in c["b"]],
                    "x": [native_value(v) for v in c["x"]], "s": [native_value(v) for v in c["scales"]], "raw": c})
    return out


def blocks_of(sizes):
    out, at = [], 0
    for s in sizes:
        out.append((at, at + s))
        at += s
    return out


def block_diag(parts):
    n = sum(len(p[1]) for p in parts)
    w = [[0.0] * n for _ in range(n)]
    b, x, s, at = [], [], [], 0
    for pw, pb, px, ps in parts:
        for i, row in enumerate(pw):
            for k, v in enumerate(row):
                w[at + i][at + k] = v
        b += pb
        x += px
        s += ps
        at += len(pb)
    return w, b, x, s


def lognorm_j(n: int, pe: float) -> list[list[float]]:
    return [[(-200.0 - 100.0 * pe) if i == k else (100.0 + 100.0 * pe) if k + 1 == i else 100.0 if k == i + 1
             else 0.0 for k in range(n)] for i in range(n)]


def faithful_metric(n: int) -> list[set[float]]:
    """For k = 1..n, the admissible binary64 s_k = fl(1 + |y|), y either binary64 neighbour of sin k (a faithful
    rounding; sin k is irrational for k >= 1, so it is never representable)."""
    import mpmath
    mpmath.mp.dps = 60
    out = []
    for k in range(1, n + 1):
        exact = mpmath.sin(k)
        y = float(exact)
        if mpmath.mpf(y) > exact:
            faithful = {math.nextafter(y, -math.inf), y}
        else:
            faithful = {y, math.nextafter(y, math.inf)}
        out.append({1.0 + abs(c) for c in faithful})
    return out


def w_formation_inputs(gamma: float) -> list[dict]:
    out = []
    for h in H_LIST:
        for k in K_LIST:
            hg = h * gamma
            lam = (1.0 - 2.0 ** -40) / hg
            j = [[lam, k], [0.0, lam]]
            w = [[float(int(r == c)) - hg * j[r][c] for c in range(2)] for r in range(2)]
            u = solve([[F(v) for v in row] for row in w], [F(1), F(1)])
            cand = [float(v) for v in u]  # Fraction -> float is correctly rounded (ties to even)
            out.append({"id": f"wf_h{rust_display(h)}_k{rust_display(k)}", "h": h, "k": k, "gamma": gamma,
                        "lambda": lam, "j": j, "w_rep": w, "rhs": [1.0, 1.0], "candidate": cand})
    return out


def discriminates(member: dict, research_weighted: list[tuple[float, float]] | None) -> bool:
    """Section C's exact rule: the exact shifted-W error lies outside the research enclosure (weighted, unit metric)."""
    if research_weighted is None:
        return False
    b = shifted(member["j"], member["h"], member["gamma"], [1.0, 1.0])
    e = exact_errors(b, member["rhs"], member["candidate"])
    if e is None:
        return True
    return any(not (F(lo) <= e[i] <= F(hi)) for i, (lo, hi) in enumerate(research_weighted))


def registered_cases(native: dict, gamma: float, fixtures: list[dict], lognorm: dict) -> dict:
    """Every registered case: id -> spec (group, route, witness, target, rhs, candidate). ``lognorm`` maps a route
    (d) case ID to its (metric, lu_base) taken from the evidence after validation."""
    cells = native_cells(native, "cells")
    invalid = native_cells(native, "invalid_controls")
    discovery = [c for c in cells if c["partition"] == "discovery"]
    out = {}

    def add(cid, group, route, witness, target, rhs, cand):
        if cid in out:
            raise ValueError(f"duplicate registered id {cid}")
        out[cid] = {"group": group, "route": route, "witness": witness, "target": target, "rhs": list(rhs),
                    "candidate": list(cand)}

    for c in cells:
        b = represented(c["w"], c["s"])
        add(c["id"], "A_cell", "represented", b, b, c["b"], c["x"])
    for c in invalid:
        b = represented(c["w"], c["s"])
        add(c["id"], "A_invalid", "represented", b, b, c["b"], c["x"])
    for c in cells:
        b = represented(c["w"], c["s"])
        variants = (("double_half", [2.0 * c["s"][0], c["s"][1] / 2.0]), ("next_up", [next_up(c["s"][0]), c["s"][1]]))
        for tag, m in variants:
            add(f"{c['id']}_metric_{tag}", "C_metric", "represented", b, represented(c["w"], m), c["b"], c["x"])
        for tag, epoch in (("8", 8), ("none", None)):
            add(f"{c['id']}_epoch_{tag}", "C_epoch", "represented", b, represented(c["w"], c["s"], epoch=epoch),
                c["b"], c["x"])
        for tag, m in variants:
            add(f"{c['id']}_transport_{tag}", "D_transport", "represented", b, represented(c["w"], m), c["b"],
                c["x"])
    for m in fixtures:
        unit = [1.0, 1.0]
        sb = shifted(m["j"], m["h"], m["gamma"], unit)
        add(f"{m['id']}_target_mismatch", "C_w_formation", "represented", represented(m["w_rep"], unit), sb,
            m["rhs"], m["candidate"])
        ub = shifted(m["j"], m["h"], m["gamma"], unit, assembly={"kind": "undeclared"})
        add(f"{m['id']}_assembly_undeclared", "C_assembly", "shifted", ub, ub, m["rhs"], m["candidate"])
        for tag in ("j00", "j01", "j10", "j11", "h", "gamma"):
            j = [r[:] for r in m["j"]]
            h, g = m["h"], m["gamma"]
            if tag.startswith("j"):
                r, cc = int(tag[1]), int(tag[2])
                j[r][cc] = next_up(j[r][cc])
            elif tag == "h":
                h = next_up(h)
            else:
                g = next_up(g)
            add(f"{m['id']}_operator_{tag}_ulp", "C_operator", "shifted", sb, shifted(j, h, g, unit), m["rhs"],
                m["candidate"])
        add(f"{m['id']}_exact", "D_shifted_exact", "shifted", sb, sb, m["rhs"], m["candidate"])
        delta = 2.0 ** -60 * max(abs(v) for r in m["j"] for v in r)
        db = shifted(m["j"], m["h"], m["gamma"], unit, assembly={"kind": "entrywise_abs", "delta": delta})
        add(f"{m['id']}_delta", "D_shifted_delta", "shifted", db, db, m["rhs"], m["candidate"])
    d0, d1 = discovery[0], discovery[1]
    w4, b4, x4, s4 = block_diag([(d0["w"], d0["b"], d0["x"], d0["s"]), (d1["w"], d1["b"], d1["x"], d1["s"])])
    for r, cc in ((0, 2), (0, 3), (1, 2), (1, 3), (2, 0), (2, 1), (3, 0), (3, 1)):
        for tag, value in (("min_subnormal", 5e-324), ("1e-300", 1.0e-300), ("one", 1.0)):
            w = [row[:] for row in w4]
            w[r][cc] = value
            b = represented(w, s4, blocks=blocks_of([2, 2]))
            add(f"offblock_r{r}c{cc}_{tag}", "C_offblock", "represented", b, b, b4, x4)
    dense4 = represented(w4, s4)
    add("dense4_route_a", "C_offblock", "represented", dense4, dense4, b4, x4)
    for k in (1024.0, 1048576.0):
        b = represented([[1.0, k], [0.0, 1.0]], [1.0, 1.0])
        add(f"nu_k{rust_display(k)}", "C_nu", "projected_nu", b, b, [0.0, 1.0], [0.0, 0.0])
    for count in (2, 3, 4):
        w, bb, x, s = block_diag([(c["w"], c["b"], c["x"], c["s"]) for c in discovery[:count]])
        b = represented(w, s, blocks=blocks_of([2] * count))
        add(f"blocks_n{2 * count}", "D_blocks", "represented", b, b, bb, x)
    w, bb, x, s = block_diag([(d0["w"], d0["b"], d0["x"], d0["s"]), ([[3.0]], [1.0], [1.0 / 3.0], [1.0])])
    b = represented(w, s, blocks=blocks_of([2, 1]))
    add("blocks_n3", "D_blocks", "represented", b, b, bb, x)
    specs = []
    for n in (3, 4, 8):
        for pe in (0.0, 10.0):
            for a in (1.0e-3, 1.0e-1):
                specs.append((f"lognorm_n{n}_pe{rust_display(pe)}_a{rust_exp(a)}", "D_lognorm", lognorm_j(n, pe), a))
    specs.append(("lognorm_negative_shift", "D_lognorm_negative", lognorm_j(3, 0.0), -1.0e-3))
    specs.append(("lognorm_nonpositive_denominator", "D_lognorm_negative",
                  [[10.0 if i == k else 0.0 for k in range(3)] for i in range(3)], 1.0))
    for cid, group, j, h in specs:
        metric, base = lognorm.get(cid, (None, None))
        if metric is None:
            metric = [math.nan] * len(j)
            base = [math.nan] * len(j)
        cand = list(base)
        cand[0] = cand[0] + 2.0 ** -30
        b = shifted(j, h, 1.0, metric)
        add(cid, group, "lognorm", b, b, [1.0] * len(j), cand)
    return out


# --------------------------------------------------------------------------------------- git layer
class Repo:
    """Read-only git queries (replaceable in the synthetic tests)."""

    def __init__(self, root: Path):
        self.root = Path(root)

    def _git(self, *args: str) -> subprocess.CompletedProcess:
        return subprocess.run(["git", *args], cwd=self.root, capture_output=True)

    def show(self, commit: str, path: str) -> bytes | None:
        out = self._git("show", f"{commit}:{path}")
        return out.stdout if out.returncode == 0 else None

    def diff_status(self, base: str, commit: str, path: str) -> list[tuple[str, str]] | None:
        out = self._git("diff", "--name-status", f"{base}..{commit}", "--", path)
        if out.returncode != 0:
            return None
        rows = []
        for line in out.stdout.decode().splitlines():
            parts = line.split("\t")
            rows.append((parts[0], parts[-1]))
        return rows

    def grep(self, commit: str, pattern: str) -> list[str] | None:
        out = self._git("grep", "-l", "-w", "-F", pattern, commit, "--", "*.rs", "*Cargo.toml")
        if out.returncode not in (0, 1):
            return None
        return [line.split(":", 1)[1] for line in out.stdout.decode().splitlines()]


# ------------------------------------------------------------------------------------ validation
class Checks(evidence._Checks):
    def hexv(self, path, value, length=None, finite=True):
        if not isinstance(value, list) or (length is not None and len(value) != length) or not value:
            self.fail(path, f"expected a hex vector of length {length}")
            return None
        try:
            return [fin(v) if finite else unhex(v) for v in value]
        except evidence.ValidationError as exc:
            self.fail(path, exc.reasons[0])
            return None

    def pairs(self, path, value, length):
        if not isinstance(value, list) or len(value) != length:
            self.fail(path, f"expected {length} interval pairs")
            return None
        out = []
        for i, p in enumerate(value):
            v = self.hexv(f"{path}[{i}]", p, 2)
            if v is None:
                return None
            if not v[0] <= v[1]:
                self.fail(f"{path}[{i}]", "interval lo > hi")
                return None
            out.append((v[0], v[1]))
        return out


def decode_verified(c: Checks, path: str, v, n: int) -> dict | None:
    if not c.keys(path, v, VERIFIED_KEYS):
        return None
    out = {"route": v["route"]}
    if v["route"] not in VERIFIED_ROUTES:
        c.fail(f"{path}.route", f"unknown route {v['route']!r}")
    out["residual"] = c.pairs(f"{path}.residual_hex", v["residual_hex"], n)
    for key in ("correction", "weighted_correction"):
        raw = v[f"{key}_hex"]
        out[key] = None if raw is None else c.pairs(f"{path}.{key}_hex", raw, n)
    for key in ("weighted_inf_upper", "wrms_upper"):
        try:
            out[key] = fin(v[f"{key}_hex"])
        except evidence.ValidationError as exc:
            c.fail(f"{path}.{key}_hex", exc.reasons[0])
            out[key] = math.nan
    for key in ("gain", "transport_factor"):
        raw = v[f"{key}_hex"]
        try:
            out[key] = None if raw is None else fin(raw)
        except evidence.ValidationError as exc:
            c.fail(f"{path}.{key}_hex", exc.reasons[0])
            out[key] = None
    ln = v["lognorm"]
    out["lognorm"] = None
    if ln is not None and c.keys(f"{path}.lognorm", ln, ("shift_hex", "mu_up_hex", "mu_source",
                                                           "denominator_lower_hex")):
        shift = c.hexv(f"{path}.lognorm.shift_hex", ln["shift_hex"], 2)
        try:
            out["lognorm"] = {"shift": shift, "mu_up": fin(ln["mu_up_hex"]), "mu_source": ln["mu_source"],
                              "denominator_lower": fin(ln["denominator_lower_hex"])}
        except evidence.ValidationError as exc:
            c.fail(f"{path}.lognorm", exc.reasons[0])
        c.str_(f"{path}.lognorm.mu_source", ln["mu_source"])
    for key in ("setup_flops", "verification_flops"):
        c.int_(f"{path}.{key}", v[key])
        out[key] = v[key]
    c.str_(f"{path}.bound_operator_sha256", v["bound_operator_sha256"])
    out["bound_operator_sha256"] = v["bound_operator_sha256"]
    out["bound_metric"] = c.hexv(f"{path}.bound_metric_hex", v["bound_metric_hex"], n)
    out["bound_model_epoch"] = v["bound_model_epoch"]
    return out


def decode_evidence(c: Checks, path: str, e, n: int) -> dict | None:
    if not c.keys(path, e, EVIDENCE_KEYS):
        return None
    kind = e["kind"]
    out = {"kind": kind, "rejection": None, "estimate": None, "verified": None}
    if kind == "unavailable":
        if e["rejection"] not in REJECTIONS or e["estimate"] is not None or e["verified"] is not None:
            c.fail(path, "malformed unavailable evidence")
        out["rejection"] = e["rejection"]
    elif kind == "estimated":
        est = e["estimate"]
        if e["rejection"] is not None or e["verified"] is not None or not c.keys(f"{path}.estimate", est,
                                                                                   ("source", "value_hex")):
            c.fail(path, "malformed estimated evidence")
            return out
        if est["source"] not in ESTIMATE_SOURCES:
            c.fail(f"{path}.estimate.source", f"unknown source {est['source']!r}")
        try:
            out["estimate"] = {"source": est["source"], "value": fin(est["value_hex"])}
        except evidence.ValidationError as exc:
            c.fail(f"{path}.estimate.value_hex", exc.reasons[0])
    elif kind == "verified":
        if e["rejection"] is not None or e["estimate"] is not None:
            c.fail(path, "malformed verified evidence")
        out["verified"] = decode_verified(c, f"{path}.verified", e["verified"], n)
    else:
        c.fail(f"{path}.kind", f"unknown evidence kind {kind!r}")
    return out


def research_expected(raw: dict) -> dict:
    if raw["status"] != "admitted":
        return {"status": raw["status"], "error": raw["error"], "stage": raw["rejection_stage"]}
    cert = raw["certificate"]

    def pairs(key):
        return [[hx(float(v)) for v in p] for p in cert[key]]

    return {"status": "admitted", "error": None, "stage": None,
            "residual_hex": pairs("residual_intervals"), "correction_hex": pairs("correction_intervals"),
            "weighted_correction_hex": pairs("weighted_correction_intervals"),
            "weighted_inf_upper_hex": hx(float(cert["weighted_inf_upper"])),
            "wrms_upper_hex": hx(float(cert["wrms_upper"])),
            "gain_hex": hx(float(cert["scaled_residual_gain_inf_upper"]))}


def validate(runs_path: Path, native_path: Path, receipts_path: Path, repo_root: Path, git: Repo,
             checker_bytes: bytes) -> dict:
    """AS03 primitives first, then the AS05 schema; raises evidence.ValidationError (INVALID)."""
    docs, digests, _ = evidence._load_all([
        ("runs", runs_path, None),
        ("retained", native_path, PINS["native"][1]),
        ("receipts", receipts_path, None),
    ])
    c = Checks()
    runs, native, receipts = docs["runs"], docs["retained"], docs["receipts"]
    for role, (rel, pin) in PINS.items():
        try:
            digest = sha256_file(repo_root / rel)
        except OSError as exc:
            c.fail(f"pin.{role}", f"cannot read {rel}: {exc}")
            continue
        if digest != pin:
            c.fail(f"pin.{role}", f"{rel} sha256 {digest} != registered {pin}")
    if not c.keys("runs", runs, TOP_KEYS):
        c.done()
    c.equal("runs.schema", runs["schema"], RUNS_SCHEMA)
    c.equal("runs.gain_witness_sha256", runs["gain_witness_sha256"], PINS["gain_witness"][1])
    c.equal("runs.model_epoch", runs["model_epoch"], EPOCH)
    gamma = gamma_from_snapshot(repo_root)
    c.equal("runs.gamma_hex", runs["gamma_hex"], hx(gamma))

    # Provenance.
    commit = None
    if c.keys("runs.source", runs["source"], ("commit", "tree_status")):
        commit = runs["source"]["commit"]
        if not (isinstance(commit, str) and re.fullmatch(r"[0-9a-f]{40}", commit)):
            c.fail("runs.source.commit", "not a 40-digit commit")
            commit = None
        if runs["source"]["tree_status"] != "":
            c.fail("runs.source.tree_status", "the recorded tree is dirty")
    if commit is not None:
        blob = git.show(commit, CHECKER_PATH)
        if blob is None:
            c.fail("provenance", f"the checker does not exist at the RUNS commit {commit} (it postdates the run)")
        elif blob != checker_bytes:
            c.fail("provenance", "the checker blob at the RUNS commit differs from the checker that runs")
    receipts_ok = c.keys("receipts", receipts, ("schema", "commit", "tree_status_before", "tree_status_after",
                                                 "commands"))
    if receipts_ok:
        c.equal("receipts.schema", receipts["schema"], RECEIPTS_SCHEMA)
        if receipts["commit"] != commit:
            c.fail("receipts.commit", "differs from the RUNS commit")
        for key in ("tree_status_before", "tree_status_after"):
            if receipts[key] != "":
                c.fail(f"receipts.{key}", "the recorded tree is dirty")
        if not isinstance(receipts["commands"], list) or len(receipts["commands"]) != 3:
            c.fail("receipts.commands", "expected 3 command receipts")
        else:
            for i, r in enumerate(receipts["commands"]):
                if c.keys(f"receipts.commands[{i}]", r, ("name", "argv", "exit_code", "lines")):
                    c.int_(f"receipts.commands[{i}].exit_code", r["exit_code"], minimum=None)
                    c.list_(f"receipts.commands[{i}].lines", r["lines"])

    # W-formation family.
    members = w_formation_inputs(gamma)
    fam = runs["w_formation_family"]
    decision = []
    if c.list_("runs.w_formation_family", fam) and len(fam) == len(members):
        for i, (row, m) in enumerate(zip(fam, members)):
            path = f"runs.w_formation_family[{i}]"
            if not c.keys(path, row, MEMBER_KEYS):
                decision.append(False)
                continue
            expected = {"id": m["id"], "h_hex": hx(m["h"]), "k_hex": hx(m["k"]), "gamma_hex": hx(m["gamma"]),
                        "lambda_hex": hx(m["lambda"]), "j_hex": [[hx(v) for v in r] for r in m["j"]],
                        "w_rep_hex": [[hx(v) for v in r] for r in m["w_rep"]], "rhs_hex": [hx(v) for v in m["rhs"]],
                        "candidate_hex": [hx(v) for v in m["candidate"]]}
            for key, value in expected.items():
                if row[key] != value:
                    c.fail(f"{path}.{key}", "does not decode to the registered input")
            res = row["research"]
            weighted = None
            if c.keys(f"{path}.research", res, ("status", "correction_hex", "weighted_correction_hex",
                                                 "wrms_upper_hex")):
                if res["status"] == "admitted":
                    weighted = c.pairs(f"{path}.research.weighted_correction_hex", res["weighted_correction_hex"], 2)
                    c.pairs(f"{path}.research.correction_hex", res["correction_hex"], 2)
                    c.hexv(f"{path}.research.wrms_upper_hex", [res["wrms_upper_hex"]], 1)
            dec = discriminates(m, weighted)
            decision.append(dec)
            if row["discriminates"] is not dec:
                c.fail(f"{path}.discriminates", f"export says {row['discriminates']}, the exact rule says {dec}")
    else:
        c.fail("runs.w_formation_family", f"expected {len(members)} members")
    chosen = [m for m, dec in zip(members, decision) if dec][:4]
    if len(decision) == len(members) and len(chosen) < 4:
        c.fail("w_formation", f"only {len(chosen)} members discriminate under the exact rule (fixture-design defect)")
    if runs["fixtures"] != [m["id"] for m in chosen]:
        c.fail("runs.fixtures", "differs from the first four discriminating members")
    if len(fam) == len(members) and all(isinstance(r, dict) and "fixture" in r for r in fam):
        flags = [r["fixture"] for r in fam]
        if flags != [m in chosen for m in members]:
            c.fail("runs.w_formation_family", "fixture flags differ from the exact selection")
    c.done()

    # Cases: identity, counts, inputs.
    rows = runs["cases"]
    if not c.list_("runs.cases", rows, nonempty=True):
        c.done()
    ids = [r.get("id") if isinstance(r, dict) else None for r in rows]
    if len(set(ids)) != len(ids):
        c.fail("runs.cases", "duplicated case id")
    if len(rows) != TOTAL_CASES:
        c.fail("runs.cases", f"{len(rows)} cases, registered {TOTAL_CASES}")
    lognorm_inputs = {}
    by_id = {}
    for i, r in enumerate(rows):
        if not c.keys(f"runs.cases[{i}]", r, CASE_KEYS):
            continue
        by_id[r["id"]] = r
        if r["group"] in ("D_lognorm", "D_lognorm_negative"):
            note = r["note"]
            if c.keys(f"runs.cases[{i}].note", note, ("lu_base_hex",)):
                base = c.hexv(f"runs.cases[{i}].note.lu_base_hex", note["lu_base_hex"])
                metric = c.hexv(f"runs.cases[{i}].target.metric_hex", r["target"].get("metric_hex")
                                if isinstance(r["target"], dict) else None)
                if base is not None and metric is not None:
                    allowed = faithful_metric(len(metric))
                    if any(s not in a for s, a in zip(metric, allowed)):
                        c.fail(f"runs.cases[{i}].target.metric_hex", "s_k is not fl(1 + |faithful sin k|)")
                    lognorm_inputs[r["id"]] = (metric, base)
        elif r["note"] is not None:
            c.fail(f"runs.cases[{i}].note", "unexpected note")
    c.done()
    registered = registered_cases(native, gamma, chosen, lognorm_inputs)
    if set(by_id) != set(registered):
        missing, extra = sorted(set(registered) - set(by_id)), sorted(set(by_id) - set(registered))
        c.fail("runs.cases", f"case-ID set differs (missing {missing[:5]}.. {len(missing)}, "
                             f"unexpected {extra[:5]}.. {len(extra)})")
        c.done()
    counts = {}
    for cid, spec in registered.items():
        counts[spec["group"]] = counts.get(spec["group"], 0) + 1
    if counts != GROUP_COUNTS:
        c.fail("registration", f"group counts {counts} differ from {GROUP_COUNTS}")
    cases = {}
    for cid, spec in registered.items():
        r = by_id[cid]
        path = f"case {cid}"
        n = len(spec["rhs"])
        if r["group"] != spec["group"]:
            c.fail(f"{path}.group", f"{r['group']!r} != {spec['group']!r}")
        if r["route"] != spec["route"]:
            c.fail(f"{path}.route", f"{r['route']!r} != {spec['route']!r}")
        for key in ("witness", "target"):
            if r[key] != binding_hex(spec[key]):
                c.fail(f"{path}.{key}", "does not decode to the registered binding")
        if r["rhs_hex"] != [hx(v) for v in spec["rhs"]]:
            c.fail(f"{path}.rhs_hex", "does not decode to the registered input")
        if r["candidate_hex"] != [hx(v) for v in spec["candidate"]]:
            c.fail(f"{path}.candidate_hex", "does not decode to the registered input")
        ev = decode_evidence(c, f"{path}.evidence", r["evidence"], n)
        comp = None
        if spec["group"] in D_GROUPS:
            cm = r["comparator"]
            if c.keys(f"{path}.comparator", cm, ("source", "estimate_hex", "nominal_flops", "rejection")):
                c.equal(f"{path}.comparator.source", cm["source"], "DenseLuComparator")
                c.float_(f"{path}.comparator.nominal_flops", cm["nominal_flops"], minimum=0.0)
                try:
                    comp = {"estimate": None if cm["estimate_hex"] is None else fin(cm["estimate_hex"]),
                            "nominal_flops": cm["nominal_flops"], "rejection": cm["rejection"]}
                except evidence.ValidationError as exc:
                    c.fail(f"{path}.comparator.estimate_hex", exc.reasons[0])
        elif r["comparator"] is not None:
            c.fail(f"{path}.comparator", "unexpected comparator")
        if spec["group"] in ("A_cell", "A_invalid"):
            raw = next(x for x in native["gain_study"]["cells" if spec["group"] == "A_cell" else "invalid_controls"]
                       if x["id"] == cid)
            if r["research"] != research_expected(raw):
                c.fail(f"{path}.research", "research reference differs from NATIVE.json")
        elif r["research"] is not None:
            c.fail(f"{path}.research", "unexpected research reference")
        cases[cid] = {**spec, "evidence": ev, "comparator": comp, "research": r["research"]}
    c.done()
    return {"runs": runs, "native": native, "receipts": receipts, "cases": cases, "members": members,
            "decision": decision, "chosen": chosen, "fam": fam, "commit": commit, "digests": digests}


# ------------------------------------------------------------------------------------------- gates
def verified_of(case: dict) -> dict | None:
    ev = case["evidence"]
    return ev["verified"] if ev and ev["kind"] == "verified" else None


def binding_ok(case: dict, ver: dict) -> bool:
    t = case["target"]
    return (ver["bound_operator_sha256"] == operator_sha256(t)
            and ver["bound_metric"] is not None and [hx(v) for v in ver["bound_metric"]] == [hx(v) for v in t["metric"]]
            and ver["bound_model_epoch"] == t["model_epoch"])


def gate_retained(cases: dict) -> dict:
    rows, failures = [], []
    for cid, case in cases.items():
        if case["group"] != "A_cell":
            continue
        ver = verified_of(case)
        if ver is None:
            failures.append(f"{cid}: not Verified ({case['evidence']['kind']}, {case['evidence']['rejection']})")
            continue
        t = case["target"]
        e = exact_errors(t, case["rhs"], case["candidate"])
        chk = enclosure_check(e, t["metric"], ver)
        gain = exact_gain(t, t["metric"])
        chk["gain_ok"] = ver["gain"] is not None and F(ver["gain"]) >= gain
        chk["binding_ok"] = binding_ok(case, ver)
        ok = chk["pass"] and chk["gain_ok"] and chk["binding_ok"]
        if not ok:
            failures.append(f"{cid}: {chk}")
        rows.append({"id": cid, "pass": ok, "wrms_upper": ver["wrms_upper"],
                     "exact_wrms_error": chk["exact_wrms_error"],
                     "bound_over_error": None if chk["zero_error"] else ver["wrms_upper"] / chk["exact_wrms_error"],
                     "gain_upper": ver["gain"], "gain_exact": float(gain)})
    invalid = []
    for cid, case in cases.items():
        if case["group"] != "A_invalid":
            continue
        ev = case["evidence"]
        research = case["research"]
        expected = RESEARCH_CLASS.get(research.get("error"))
        row = {"id": cid, "kind": ev["kind"], "rejection": ev["rejection"], "research_error": research.get("error"),
               "mapped_class": expected}
        if cid == OVERLAP_EXCEPTION:
            if ev["kind"] == "verified":
                ver = ev["verified"]
                t = case["target"]
                e = exact_errors(t, case["rhs"], case["candidate"])
                chk = enclosure_check(e, t["metric"], ver)
                gain = exact_gain(t, t["metric"])
                row["oracle"] = {k: chk[k] for k in ("wrms_ok", "inf_ok", "component_ok", "exact_wrms_error")}
                row["gain_ok"] = ver["gain"] is not None and F(ver["gain"]) >= gain
                row["pass"] = chk["pass"] and row["gain_ok"] and binding_ok(case, ver)
                row["wrms_upper"] = ver["wrms_upper"]
            else:
                row["pass"] = ev["kind"] == "unavailable"
        else:
            row["pass"] = ev["kind"] == "unavailable" and ev["rejection"] == expected
        if not row["pass"]:
            failures.append(f"{cid}: {row}")
        invalid.append(row)
    verified = sum(r["pass"] for r in rows)
    rejected = sum(1 for r in invalid if r["id"] != OVERLAP_EXCEPTION and r["pass"])
    return {"pass": not failures and len(rows) == 30 and verified == 30 and rejected == 11,
            "cells_verified_and_enclosing": verified, "cells": len(rows), "invalid_rejected_with_class": rejected,
            "overlap_exception": next((r for r in invalid if r["id"] == OVERLAP_EXCEPTION), None),
            "failures": failures, "cell_rows": rows, "invalid_rows": invalid}


def gate_parity(cases: dict) -> dict:
    rows, failures = [], []
    for cid, case in cases.items():
        if case["group"] != "A_cell":
            continue
        ver = verified_of(case)
        res = case["research"]
        if ver is None or res.get("status") != "admitted":
            failures.append(f"{cid}: no core or research certificate")
            continue
        contained = True
        for core_key, res_key in (("residual", "residual_hex"), ("correction", "correction_hex"),
                                  ("weighted_correction", "weighted_correction_hex")):
            core = ver[core_key] or []
            outer = [(unhex(p[0]), unhex(p[1])) for p in res[res_key]]
            if len(core) != len(outer) or any(not (o[0] <= i[0] and i[1] <= o[1]) for i, o in zip(core, outer)):
                contained = False
        r_wrms = unhex(res["wrms_upper_hex"])
        r_gain = unhex(res["gain_hex"])
        wrms_ok = ver["wrms_upper"] <= r_wrms
        gain_ok = ver["gain"] is not None and ver["gain"] <= r_gain
        ok = contained and wrms_ok and gain_ok
        if not ok:
            failures.append(f"{cid}: contained={contained} wrms_ok={wrms_ok} gain_ok={gain_ok}")
        rows.append({"id": cid, "pass": ok, "core_wrms_upper": ver["wrms_upper"], "research_wrms_upper": r_wrms,
                     "core_over_research_wrms": ver["wrms_upper"] / r_wrms if r_wrms else None,
                     "core_gain": ver["gain"], "research_gain": r_gain})
    ratios = [r["core_over_research_wrms"] for r in rows if r["core_over_research_wrms"] is not None]
    return {"pass": not failures and len(rows) == 30, "cells": len(rows), "failures": failures,
            "min_core_over_research_wrms": min(ratios) if ratios else None,
            "max_core_over_research_wrms": max(ratios) if ratios else None, "rows": rows}


def gate_negatives(cases: dict) -> dict:
    failures, summary = [], {}
    for cid, case in cases.items():
        g = case["group"]
        ev = case["evidence"]
        if not (g.startswith("C_") or g == "D_lognorm_negative"):
            continue
        if ev["kind"] == "verified":
            failures.append(f"{cid}: Verified")
            continue
        if g in C_CLASS:
            want = ("unavailable", C_CLASS[g], None)
        elif g == "C_offblock":
            want = ("unavailable", "UndeclaredStructure" if cid == "dense4_route_a" else "OffBlockCoupling", None)
        elif g == "C_assembly":
            want = ("estimated", None, "UndeclaredAssembly")
        elif g == "C_nu":
            want = ("estimated", None, "ProjectedNu")
        else:
            want = ("unavailable", "NegativeShift" if cid == "lognorm_negative_shift" else "NonpositiveDenominator",
                    None)
        got = (ev["kind"], ev["rejection"], ev["estimate"]["source"] if ev["estimate"] else None)
        key = f"{g}:{got[1] or got[2]}"
        summary[key] = summary.get(key, 0) + 1
        if got != want:
            failures.append(f"{cid}: {got} != {want}")
    return {"pass": not failures, "failures": failures, "classes": dict(sorted(summary.items()))}


def gate_no_false_certificate(cases: dict) -> dict:
    failures, rows, vacuous, unavailable = [], [], [], []
    for cid, case in cases.items():
        g = case["group"]
        if g not in D_GROUPS or g == "D_lognorm_negative":
            continue
        ver = verified_of(case)
        if ver is None:
            (vacuous if g == "D_shifted_delta" else unavailable).append(
                {"id": cid, "kind": case["evidence"]["kind"], "rejection": case["evidence"]["rejection"]})
            continue
        t = case["target"]
        checks = []
        for corner in corners(t):
            e = exact_errors(t, case["rhs"], case["candidate"], corner)
            if e is None:
                checks.append({"pass": False, "singular_corner": True})
                continue
            checks.append(enclosure_check(e, t["metric"], ver))
        bind = binding_ok(case, ver)
        ok = all(ch["pass"] for ch in checks) and bind
        if not ok:
            failures.append(f"{cid}: corners_failed={sum(not ch['pass'] for ch in checks)} binding_ok={bind}")
        errs = [ch.get("exact_wrms_error") for ch in checks if "exact_wrms_error" in ch]
        worst = max(errs) if errs else None
        gain_exact = exact_gain(t, t["metric"]) if ver["gain"] is not None else None
        rows.append({"id": cid, "group": g, "pass": ok, "corners_checked": len(checks),
                     "wrms_upper": ver["wrms_upper"], "worst_exact_wrms_error": worst,
                     "bound_over_error": (ver["wrms_upper"] / worst) if worst else None,
                     "gain_upper": ver["gain"],
                     "gain_enclosed_reported": None if gain_exact is None else F(ver["gain"]) >= gain_exact})
    corners_total = sum(r["corners_checked"] for r in rows if r["group"] == "D_shifted_delta")
    return {"pass": not failures, "verified_checked": len(rows), "failures": failures,
            "delta_corner_checks": corners_total, "vacuous_corner_cases": vacuous,
            "unavailable_positives_reported": unavailable, "rows": rows}


def doc_blocks(source: str) -> list[tuple[str, str]]:
    """(info string, body) of every fenced code block in the //! and /// comments."""
    lines = [re.sub(r"^\s*//[!/] ?", "", ln) for ln in source.splitlines() if re.match(r"^\s*//[!/]", ln)]
    out, info, body = [], None, []
    for ln in lines:
        if ln.startswith("```"):
            if info is None:
                info, body = ln[3:].strip(), []
            else:
                out.append((info, "\n".join(body)))
                info = None
        elif info is not None:
            body.append(ln)
    return out


def struct_body(source: str, name: str) -> str | None:
    m = re.search(r"pub struct " + name + r"\s*\{", source)
    if not m:
        return None
    depth, i = 1, m.end()
    while depth and i < len(source):
        depth += {"{": 1, "}": -1}.get(source[i], 0)
        i += 1
    return source[m.end():i - 1]


def gate_authority(source: str, receipts: dict) -> dict:
    failures = []
    code = "\n".join(ln for ln in source.splitlines() if not re.match(r"^\s*//", ln))
    if re.search(r"\bDeserialize\b", code):
        failures.append("Deserialize appears in residual_gain.rs code")
    for name in AUTHORITY_TYPES:
        body = struct_body(code, name)
        if body is None:
            failures.append(f"{name}: no struct definition")
            continue
        if re.search(r"(^|\n)\s*pub(\s|\()", body):
            failures.append(f"{name}: has a pub field")
    blocks = [b for info, b in doc_blocks(source) if info.split(",")[0] == "compile_fail"]
    required = {
        "from_str::<VerifiedCorrection>": lambda b: re.search(r"from_str::<[\w:]*VerifiedCorrection>", b),
        "VerifiedCorrection literal": lambda b: re.search(r"=\s*VerifiedCorrection\s*\{", b),
        "LinearTargetId literal": lambda b: re.search(r"=\s*LinearTargetId\s*\{", b),
        "ResidualGainWitness literal": lambda b: re.search(r"=\s*ResidualGainWitness\s*\{", b),
    }
    present = {k: any(f(b) for b in blocks) for k, f in required.items()}
    for k, ok in present.items():
        if not ok:
            failures.append(f"missing compile-fail doc test: {k}")
    doc = next((r for r in receipts["commands"] if r["name"] == "doc_residual_gain"), None)
    passed_cf = 0
    if doc is None or doc["exit_code"] != 0:
        failures.append("doc-test receipt missing or failed")
    else:
        passed_cf = sum(1 for ln in doc["lines"]
                        if "residual_gain.rs" in ln and "compile fail" in ln and ln.rstrip().endswith("... ok"))
        failed = [ln for ln in doc["lines"] if ln.startswith("test ") and ln.rstrip().endswith("FAILED")]
        if failed:
            failures.append(f"doc tests failed: {failed}")
        if passed_cf < len(blocks) or passed_cf < 4:
            failures.append(f"{passed_cf} compile-fail doc tests passed, {len(blocks)} present")
    return {"pass": not failures, "failures": failures, "compile_fail_blocks": len(blocks),
            "compile_fail_passed": passed_cf, "required_present": present}


def gate_isolation(receipts: dict, git: Repo, commit: str) -> dict:
    failures = []
    suite = next((r for r in receipts["commands"] if r["name"] == "rodas5p_core_all_targets"), None)
    binaries = []
    if suite is None or suite["exit_code"] != 0:
        failures.append("pre-existing suite receipt missing or failed")
    else:
        current = None
        results = {}
        for ln in suite["lines"]:
            m = re.search(r"Running (\S+)", ln)
            if m:
                current = Path(m.group(1)).name
                continue
            m = re.match(r"test result: (\w+)\. (\d+) passed; (\d+) failed", ln)
            if m and current is not None:
                results[current] = (m.group(1), int(m.group(2)), int(m.group(3)))
                current = None
        binaries = sorted(results)
        for name, (status, _, failed) in results.items():
            if status != "ok" or failed:
                failures.append(f"{name}: {status}, {failed} failed")
        for req in SUITE_REQUIRED:
            if not any(b.startswith(req) for b in binaries):
                failures.append(f"suite receipt lacks {req}*")
    for r in receipts["commands"]:
        if r["exit_code"] != 0:
            failures.append(f"command {r['name']} exited {r['exit_code']}")
    status = git.diff_status(REGISTRATION_COMMIT, commit, "crates/rodas5p-core/tests")
    if status is None:
        failures.append("git diff of the test directory failed")
        status = []
    modified = [f"{s} {p}" for s, p in status if s != "A"]
    if modified:
        failures.append(f"pre-existing test files changed: {modified}")
    refs = git.grep(commit, "residual_gain")
    if refs is None:
        failures.append("git grep failed")
        refs = []
    outside = [p for p in refs if not p.startswith("crates/rodas5p-core/")]
    if outside:
        failures.append(f"files outside rodas5p-core reference residual_gain: {outside}")
    return {"pass": not failures, "failures": failures, "suite_binaries": binaries,
            "test_dir_changes": [f"{s} {p}" for s, p in status], "rust_references": refs}


def reported(v: dict) -> dict:
    cases = v["cases"]
    comparator, flops, lognorm, nu = [], [], [], []
    for cid, case in cases.items():
        ver = verified_of(case)
        comp = case["comparator"]
        n = len(case["rhs"])
        if comp is not None:
            comparator.append({"id": cid, "lu_estimate_inf": comp["estimate"],
                               "verified_inf_upper": ver["weighted_inf_upper"] if ver else None,
                               "lu_over_verified": (comp["estimate"] / ver["weighted_inf_upper"])
                               if ver and comp["estimate"] is not None and ver["weighted_inf_upper"] else None})
        if ver is not None:
            flops.append({"id": cid, "n": n, "setup_flops": ver["setup_flops"],
                          "verification_flops": ver["verification_flops"],
                          "lu_nominal_flops": 2.0 / 3.0 * n ** 3 + 2.0 * n ** 2,
                          "setup_over_lu": ver["setup_flops"] / (2.0 / 3.0 * n ** 3 + 2.0 * n ** 2)})
            if ver["lognorm"] is not None:
                ln = ver["lognorm"]
                lognorm.append({"id": cid, "mu_up": ln["mu_up"], "mu_source": ln["mu_source"],
                                "one_minus_a_mu_lower": ln["denominator_lower"], "shift": ln["shift"]})
        if case["group"] == "C_nu":
            e = exact_errors(case["target"], case["rhs"], case["candidate"])
            sq = sum(x * x for x in e) / len(e)
            est = case["evidence"]["estimate"]["value"] if case["evidence"]["estimate"] else None
            nu.append({"id": cid, "estimate": est, "exact_wrms_error": math.sqrt(float(sq)),
                       "underestimates": est is not None and F(est) ** 2 < sq})
    family = []
    for m, row, dec in zip(v["members"], v["fam"], v["decision"]):
        e = exact_errors(shifted(m["j"], m["h"], m["gamma"], [1.0, 1.0]), m["rhs"], m["candidate"])
        family.append({"id": m["id"], "discriminates": dec, "fixture": m in v["chosen"],
                       "exact_shifted_error": [float(x) for x in e],
                       "research_weighted": row["research"]["weighted_correction_hex"]})
    setup_ratios = [r["setup_over_lu"] for r in flops if r["n"] == 2]
    return {"lu_comparator": comparator, "flops": flops, "lognorm": lognorm, "projected_nu": nu,
            "w_formation_family": family,
            "setup_over_lu_n2_min": min(setup_ratios) if setup_ratios else None,
            "setup_over_lu_n2_max": max(setup_ratios) if setup_ratios else None}


# ----------------------------------------------------------------------------------------------- main
def main(argv=None, git: Repo | None = None, repo_root: Path = REPO, source_path: Path | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--runs", required=True, type=Path)
    parser.add_argument("--retained", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--receipts", type=Path, default=None,
                        help="default: RECEIPTS.json next to --runs")
    args = parser.parse_args(argv)
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    receipts = args.receipts or args.runs.parent / "RECEIPTS.json"
    git = git or Repo(repo_root)
    inputs = {"runs": args.runs, "retained": args.retained, "receipts": receipts}
    checker_bytes = Path(__file__).read_bytes()
    try:
        v = validate(args.runs, args.retained, receipts, repo_root, git, checker_bytes)
    except evidence.ValidationError as exc:
        return evidence.emit_invalid(args.output, SCHEMA, exc, inputs)
    source = (source_path or repo_root / SOURCE_PATH).read_text()
    gates = {
        "1_retained_oracle": gate_retained(v["cases"]),
        "2_parity": gate_parity(v["cases"]),
        "3_negatives_refused": gate_negatives(v["cases"]),
        "4_no_false_certificate": gate_no_false_certificate(v["cases"]),
        "5_authority_types": gate_authority(source, v["receipts"]),
        "6_non_regression_isolation": gate_isolation(v["receipts"], git, v["commit"]),
    }
    verdict = "PASS" if all(g["pass"] for g in gates.values()) else "FAIL"
    report = {
        "schema": SCHEMA,
        "evidence_schema": evidence.VERSION,
        "verdict": verdict,
        "source_commit": v["commit"],
        "inputs": {role: {"path": str(p), "sha256": v["digests"][role]} for role, p in inputs.items()},
        "pins": {role: pin for role, (_, pin) in PINS.items()},
        "fixtures": [m["id"] for m in v["chosen"]],
        "gate": gates,
        "reported": reported(v),
        "claim_scope": "A Verified result encloses the forward error of one linear solve W u = b in a frozen "
                       "diagonal metric for the bound target, on the registered routes and sizes only; no stage, "
                       "local or global ODE error, no matrix-free certificate, no driver use, no wall-time claim.",
    }
    args.output.write_text(json.dumps(report, indent=1, allow_nan=False) + "\n")
    print(f"verdict: {verdict}")
    for name, g in gates.items():
        print(f"  {name}: {'holds' if g['pass'] else 'FAILS'}")
    return 0 if verdict == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
