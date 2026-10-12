#!/usr/bin/env python3
"""Gate of research node research/py03_single_backend_router_20261011 (PY03; see its PREREGISTRATION.md).

Input ``--cases``: cases.json written once by ``export_cases`` of
crates/rodas5p-core/tests/polynomial_single_backend_router.rs (compact JSON; a ``.gz`` path is read as gzip). Output
``--output``: RESULTS.json (immutable: an existing file is never overwritten). PASS exit 0, FAIL exit 1, INVALID
exit 2.

Evidence (INVALID, exit 2) before any gate:
- AS03 rules (tools/evidence_schema_v2.py primitives): strict JSON, exact key sets of every document, case, record,
  plan, preflight, execution, report, parity, admission, cost and counter map, canonical binary64 hex (16 lowercase
  digits, finite), typed numbers (bool never int, int never float), nonnegative counts.
- Complete and unique sets: exactly the 96 registered grid labels, the five registered boundary labels and one
  ForceLaguerreFirst case per operator family; per case exactly the registered (arm, cache state) records, none
  duplicated in the raw lists.
- Binding of the inputs: every grid case's parameters equal its label's; the diagonal matrices equal the registered
  construction bit for bit; every w_k equals the registered splitmix stream of the recorded input seed (and the grid
  seeds follow the registered order); the boundary cases use the registered fixtures, steps and budgets; the
  ForceLaguerreFirst cases follow the registered selection rule; the Laguerre degree of the degree-above-limit case
  exceeds 128.
- References: F = sum_k phi_k(h A) w_k from the exact binary inputs by an mpmath eigendecomposition at 50 digits,
  checked at 70 digits; INVALID if the two differ by more than 1e-40 ||F||.
- Export arithmetic: in every router record the recorded cost equals the field-by-field sum of the recorded plan,
  preflight and execution costs. (Whether the cost matches the counters is gate item 3, a FAIL.)
- Provenance: the checker blob at the cases' source commit equals the checker that runs; the recorded export tree
  status and the checker's own tree status are clean.

Gate (PASS iff all hold, else FAIL with every number preserved):
1. Certification. Every admitted ``single`` (and ``force_laguerre_first``) result has bound <= total budget and
   bound >= its 50-digit error, from a certified execution on a verified enclosure. In no record of any arm is a
   declared enclosure, a timing execution, a Laguerre degree > 128 or an estimate-only total admitted; the timing
   executions are rejected at the total budget and at 1e300; the declared and the zero-budget cases end in Fallback
   in every record; a record without an admitted execution is a Fallback with neither result nor bound.
2. Parity. Every execution equals ``joint_phi_action`` (``joint_phi_action_unbounded`` for timing) on a cache in
   the same state, bitwise (report, counters, fused bits, candidate total). Wherever ``single`` selects Chebyshev,
   its fused bits and bound equal ``cheb``'s in the same cache state.
3. Accounting. In every record the caller's counters equal the preflight work plus every execution's work, and the
   cost's vector products, block products, coefficient setups and cache hits equal the corresponding counters (in
   total and per execution); each executed backend and failed execution is counted once; an execution's products
   are its plan's prediction; PP08 records add up as in PP08. In the ForceLaguerreFirst cases the first execution is
   a charged, executed, rejected Laguerre recurrence and Chebyshev executes next.
4. One execution. Wherever the first executed backend is admitted, exactly one recurrence runs. ``single`` never
   needs more vector products than ``pp08`` (same case and cache state), and strictly fewer wherever both PP08
   attempts made nonzero products.
5. Niche benefit. N = the grid cases whose cold ``single`` preflight preselects Laguerre; |N| >= 6, and in every
   case of N under both cache states the Laguerre result is selected and admitted with vector products strictly
   below ``cheb`` and coefficient-series terms and certification operations (propagation-loop terms + envelope
   interval operations + adjoint-sum terms) not above ``cheb``. With |N| < 6 item 5 fails as "niche empty".

Reported, not gated: the vector-product ratios single/cheb and single/pp08 per case and cache state, the plan
overhead (tail evaluations, Laguerre table in the preflight), the cases admitted by one basis only (PP08 attempts),
the preflight abstentions and their reasons, the tightness of the admitted bounds and the registered predictions.

Interpretations and deviations (fixed here, before the recorded run):
1. Location. The plan/execute split lives in polynomial_action.rs (``plan_joint_phi``, ``execute_joint_phi``,
   ``execute_joint_phi_counted``, ``execute_joint_phi_unbounded``, ``laguerre_preflight_terms``,
   ``PolynomialCaches``); the router and ``RouteCost`` in polynomial_action/router.rs. The action core gained one
   trailing ``staging`` parameter (as PY01 added ``capture``); the series terms and tail evaluations are counted by
   a thread-local tally in the leaf loops. Nothing changes a value: PP08's ``route_joint_phi`` and every default
   path are bitwise unchanged.
2. Signatures. ``plan_joint_phi`` takes ``&PolynomialCaches`` (a lookup only: it records whether the table is
   cached and builds nothing). ``execute_joint_phi`` takes ``&mut PolynomialCaches`` (coefficient tables and
   envelopes); its parity is with ``joint_phi_action(.., Some(&mut caches.coefficients), ..)``; the counted variant
   charges a ``RouteCost``. ``route_joint_phi_single`` returns ``CoreResult``: a NaN, negative or infinite budget,
   an invalid step or a malformed input is an error before any work, as in PP08.
3. Comparator. ``cheb`` ("Chebyshev alone") is ``route_joint_phi_single`` with ``RoutePolicy::ChebyshevOnly``: one
   Chebyshev plan, one execution, the same admission, its own caches in the same cache state. ``pp08`` is
   ``route_joint_phi`` (no caches; its warm record repeats the call).
4. Unverified enclosure. ``route_admission`` rejects every report on a declared enclosure, so the router executes
   nothing there and returns Fallback with both reasons (the plans are charged). This is a refusal before
   execution, never an admission.
5. Preflight order. Products first (ties and a failed Chebyshev or Laguerre plan abstain to Chebyshev), then the
   degree limit, then the vector-independent terms (only then is the Laguerre table built or read, charged, and the
   execution reads the same table from the cache). The terms are the action's per-column truncation and coefficient
   components, summed upward, compared with the total budget.
6. Failed executions. A basis whose plan failed is recorded as a failed, unexecuted execution (no recurrence). On an
   action error the products are charged from the counters' change.
7. Boundary arms. Every case has single, cheb and pp08 in both cache states. The ForceLaguerreFirst cases add the arm
   ``force_laguerre_first`` (the same router with ``RoutePolicy::ForceLaguerreFirst``); the degree-above-limit case
   adds it too, so that a Laguerre execution of degree > 128 is actually run and rejected. The timing case adds two
   timing executions (each basis planned and executed in plain binary64). Item 1 gates the admitted results of
   ``single`` and ``force_laguerre_first``; item 4 compares ``single`` only.
8. Boundary fixtures. Declared: the rotated n = 8, rho = 64 matrix with the declared enclosure [-max |a_ii|, 0],
   h = 0.05, total budget 1e-10. Timing: the diagonal n = 8, rho = 64 fixture, h = 0.05, 1e-10. Degree above 128:
   the same diagonal fixture, the first h in (0.5, 1, 2, 4) whose Laguerre plan at 1e-10 / 4 exceeds 128, total
   budget 1e-10. Budget 0: the rotated n = 8, rho = 4 fixture, h = 0.05. Scalar: -3 I with n = 8, h = 0.12, 1e-10.
   Distinct inputs with seeds 200000 + 1000 k, k = 1..5.
9. ForceLaguerreFirst selection: per family, the first grid case in grid order whose cold PP08 Laguerre attempt
   executed with degree <= 128 and was rejected; if none, the family's n = 8, rho = 64 fixture at h = 0.12 with a
   distinct input of seed 200000 + 1000 (6 + family index) and the first total budget of (1e-13, 1e-15, 1e-17) with
   such a rejection.
10. Grid fixtures. Rotated seeds: in the order (n, rho) = (8, 4), (8, 64), (24, 4), (24, 64), the next seed from 7001
   whose Gershgorin enclosure verifies. Inputs: index i over (family, n, rho, h, input) in grid order, seed
   100000 + 1000 i, w_k = splitmix(seed + 77 k) (distinct) or s_k splitmix(seed) (same vector); both total budgets
   share the input. The diagonal enclosure is ``new(A, 0, rho)`` (verified, lambda = +0).
11. Cache states. Cold: every arm starts with empty caches. Warm: the same call repeated with the caches the cold
   call left (per arm).
12. Certification operations = propagation-loop terms + envelope interval operations + adjoint-sum terms (cache
   hits and misses are reported). Coefficient-series terms count every term of the exponential, 1F1, 2F1 and Bessel
   loops of a table build (and of the scalar branch's phi_k); a cache hit costs none.
13. "Admitted by one basis only" uses the PP08 attempts (which execute both bases).
14. Item 1's "never admitted" list is checked in every record of every arm (pp08 included). Item 2 counts an
   executed backend whose action failed (no report, so no parity record) as a parity failure.
15. The reference is computed once per case (all arms and cache states share the inputs); the 50-digit errors and
   the comparison with the bounds are evaluated at 50 digits.
"""

from __future__ import annotations

import argparse
import gzip
from decimal import Decimal
import json
import math
import struct
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
if str(Path(__file__).resolve().parent) not in sys.path:
    sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence_schema_v2 as evidence  # noqa: E402  (fail-closed evidence validation, re-audit AS03)

SCHEMA = "vigilode-py03-single-backend-router-check-v1"
CASES_SCHEMA = "vigilode-py03-single-backend-router-cases-v1"
NODE = "research/py03_single_backend_router_20261011"
CHECKER_PATH = "tools/py03_single_backend_router_check.py"

GRID_FAMILIES = ("diagonal", "rotated")
GRID_N = (8, 24)
GRID_RHO = (4.0, 64.0)
GRID_H = (0.005, 0.05, 0.12)
GRID_INPUTS = ("distinct", "same")
GRID_BUDGET = (1e-6, 1e-10)
BOUNDARY = {"declared-enclosure": "declared", "unbounded-timing": "timing",
            "laguerre-degree-above-limit": "degree-above-limit", "total-budget-zero": "budget-zero",
            "scalar-minus3I": "scalar"}
FORCE_GROUP = "force-laguerre-first"
DEGREE_ABOVE_LIMIT_STEPS = (0.5, 1.0, 2.0, 4.0)
FORCE_FALLBACK_BUDGETS = (1e-13, 1e-15, 1e-17)
FIRST_ROTATED_SEED = 7001
NICHE_MIN = 6
DEGREE_LIMIT = 128
SCALES = (1.0, -0.5, 0.25, 2.0, -1.0)
TERMS = 5
DPS = 50
DPS_CHECK = 70
REFERENCE_AGREEMENT = "1e-40"
EXEC_CERTIFIED = "certified-enclosures"
EXEC_TIMING = "unbounded-timing"
POLICY = {"single": "chebyshev-first-preflight", "cheb": "chebyshev-only",
          "force_laguerre_first": "force-laguerre-first"}
ROUTER_ARMS = ("single", "cheb", "force_laguerre_first")
STATES = ("cold", "warm")

COST_KEYS = ("plan_tail_evaluations", "coefficient_setups", "coefficient_cache_hits", "coefficient_series_terms",
             "executed_backends", "failed_executions", "vector_products", "block_products",
             "chebyshev_propagation_terms", "envelope_interval_operations", "envelope_cache_hits",
             "envelope_cache_misses", "adjoint_sum_terms")
WORK_KEYS = frozenset((
    "rhs_calls", "rhs_batch_calls", "rhs_evaluations", "ft_calls", "jacobian_builds", "jvp_calls", "jvp_vectors",
    "mass_matvecs", "nonlinear_solves", "nonlinear_iterations", "nonlinear_residual_evaluations",
    "nonlinear_jacobian_evaluations", "nonlinear_failures", "linear_solves", "linear_iterations", "linear_matvecs",
    "preconditioner_apps", "direct_factorizations", "direct_solve_calls", "recycle_projection_calls",
    "recycle_same_operator_uses", "recycle_cross_operator_refreshes", "recycle_refresh_matvecs", "recycle_updates",
    "recycle_vectors_selected", "recycle_dropped_vectors", "harmonic_ritz_solves", "orthogonalization_inner_products",
    "orthogonalization_vector_updates", "diagnostic_matvecs", "phi_actions", "phi_krylov_vectors",
    "phi_projected_exponentials", "phi_restarts", "phi_dense_oracle_calls", "block_linear_solves",
    "block_linear_iterations", "block_matvecs", "block_preconditioner_apps", "fast_attempts", "fast_accepts",
    "fallback_steps", "accepted_steps", "rejected_steps", "local_error_failures", "linear_solve_failures",
    "nonlinear_solve_failures", "nonfinite_step_failures", "forced_stage_solves", "jacobian_matvecs",
    "linear_matvec_vectors", "preconditioner_vectors", "merged_unknown_vector_calls", "phi_weight_underflows",
    "poly_block_products", "poly_vector_products", "poly_coefficient_setups", "poly_coefficient_reuses",
    "poly_block_allocations", "poly_fallbacks", "recycle_update_refreshes"))
# RouteCost field -> WorkCounters field (the shared units, counted separately).
COST_WORK = (("vector_products", "poly_vector_products"), ("block_products", "poly_block_products"),
             ("coefficient_setups", "poly_coefficient_setups"),
             ("coefficient_cache_hits", "poly_coefficient_reuses"))

DOC_KEYS = frozenset(("schema", "node", "source_commit", "tree_status", "constants", "force_selection", "cases"))
CONSTANT_KEYS = frozenset(("truncation_fraction", "degree_limit", "scales", "first_rotated_seed",
                           "lambda_min_fraction", "max_givens_angle"))
SELECTION_KEYS = frozenset(("family", "label", "rule"))
CASE_KEYS = frozenset(("label", "group", "family", "n", "rho_nominal", "seed", "input_seed", "a", "h", "input", "w",
                       "enclosure", "total_budget", "records", "timing"))
ENCLOSURE_KEYS = frozenset(("lambda", "rho", "verified"))
ROUTER_RECORD_KEYS = frozenset(("arm", "cache_state", "policy", "choice", "fallback_reasons", "fused", "bound",
                                "plans", "preflight", "executions", "cost", "caller_counters", "caches_before",
                                "caches_after"))
PP08_RECORD_KEYS = frozenset(("arm", "cache_state", "choice", "fallback_reasons", "fused", "bound", "attempts",
                              "caller_counters"))
PLAN_KEYS = frozenset(("basis", "plan", "error", "cost"))
PLAN_BODY_KEYS = frozenset(("branch", "degree", "width", "laguerre_scale", "beta", "laguerre_extent",
                            "predicted_vector_products", "predicted_block_products", "tail_evaluations",
                            "normalization_shift", "coefficients_cached"))
PREFLIGHT_KEYS = frozenset(("preselected", "abstain_reason", "laguerre_table_used", "vector_independent_terms",
                            "cost", "work"))
EXEC_KEYS = frozenset(("basis", "executed", "degree", "admission", "cost", "work", "report", "parity"))
ADMISSION_KEYS = frozenset(("admitted", "bound", "reason"))
REPORT_KEYS = frozenset(("basis", "branch", "execution", "evidence_verified", "degree", "fused",
                         "total_error_is_estimate", "candidate_total", "coefficient_cache_hit"))
PARITY_KEYS = frozenset(("report_bitwise_equal", "counters_equal", "replay_fused", "replay_candidate_total"))
ATTEMPT_KEYS = frozenset(("basis", "degree", "admission", "vector_products", "work", "report"))
TIMING_KEYS = frozenset(("basis", "execution", "admission_at_total_budget", "admission_at_1e300", "report",
                         "parity"))
CACHES_KEYS = frozenset(("coefficient_tables", "envelopes"))
FALLBACK_KEYS = frozenset(("chebyshev", "laguerre"))
BASES = ("Chebyshev", "Laguerre")
CHOICES = ("Chebyshev", "Laguerre", "Fallback")


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
    """Rust's ``{:e}`` of an f64 in the labels (shortest digits: ``1e-6``, ``2.5e-11``, ``0e0``)."""
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


def registered_w(kind: str, n: int, seed: int) -> list[list[float]]:
    if kind == "distinct":
        return [splitmix(seed + 77 * k, n) for k in range(TERMS)]
    v = splitmix(seed, n)
    return [[s * x for x in v] for s in SCALES]


def diagonal_matrix(n: int, rho: float) -> list[list[float]]:
    a = [[0.0] * n for _ in range(n)]
    for i in range(n):
        a[i][i] = -(rho * float(i)) / float(n - 1)
    return a


def grid_cases() -> list[dict]:
    """The registered grid in export order, with the input seed index."""
    out, index = [], 0
    for family in GRID_FAMILIES:
        for n in GRID_N:
            for rho in GRID_RHO:
                for h in GRID_H:
                    for kind in GRID_INPUTS:
                        seed = 100_000 + 1000 * index
                        index += 1
                        for budget in GRID_BUDGET:
                            label = (f"{family}-n{n}-rho{rust_display(rho)}-h{rust_display(h)}-{kind}"
                                     f"-b{rust_exp(budget)}")
                            out.append({"label": label, "family": family, "n": n, "rho": rho, "h": h,
                                        "input": kind, "budget": budget, "input_seed": seed})
    return out


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


def distance(values: list[float], exact: list):
    """||values - exact||_2 at 50 digits (never at mpmath's default precision)."""
    import mpmath as mp
    with mp.workdps(DPS):
        return +mp.sqrt(mp.fsum((mp.mpf(v) - e) ** 2 for v, e in zip(values, exact)))


# ------------------------------------------------------------------------------------------ evidence
def load(path: Path) -> tuple[dict, str]:
    data = Path(path).read_bytes()
    digest = evidence._sha256(data)
    if str(path).endswith(".gz"):
        try:
            data = gzip.decompress(data)
        except OSError as exc:
            raise evidence.ValidationError([f"cases: not gzip: {exc}"]) from None
    try:
        doc = evidence.loads_strict(data)
    except evidence.ValidationError as exc:
        raise evidence.ValidationError([f"cases: {r}" for r in exc.reasons]) from None
    return doc, digest


class _V:
    """Typed reads with paths, on top of the AS03 primitives."""

    def __init__(self, c: evidence._Checks):
        self.c = c

    def hex_(self, path: str, value) -> float | None:
        try:
            return evidence.decode_hex64(value)
        except evidence.ValidationError as exc:
            self.c.fail(path, exc.reasons[0])
            return None

    def hex_or_null(self, path: str, value) -> float | None:
        return None if value is None else self.hex_(path, value)

    def str_or_null(self, path: str, value) -> None:
        if value is not None:
            self.c.str_(path, value)

    def int_or_null(self, path: str, value) -> None:
        if value is not None:
            self.c.int_(path, value)

    def cost(self, path: str, value) -> dict | None:
        if not self.c.keys(path, value, COST_KEYS):
            return None
        ok = all([self.c.int_(f"{path}.{k}", value[k]) for k in COST_KEYS])
        return value if ok else None

    def work(self, path: str, value) -> dict | None:
        if not self.c.keys(path, value, WORK_KEYS):
            return None
        ok = all([self.c.int_(f"{path}.{k}", value[k]) for k in sorted(WORK_KEYS)])
        return value if ok else None

    def admission(self, path: str, value) -> None:
        if not self.c.keys(path, value, ADMISSION_KEYS):
            return
        if self.c.bool_(f"{path}.admitted", value["admitted"]):
            if value["admitted"]:
                self.hex_(f"{path}.bound", value["bound"])
                if value["reason"] is not None:
                    self.c.fail(f"{path}.reason", "an admission has no reason")
            else:
                self.c.str_(f"{path}.reason", value["reason"])
                if value["bound"] is not None:
                    self.c.fail(f"{path}.bound", "a rejection has no bound")

    def report(self, path: str, value, n: int) -> None:
        if value is None or not self.c.keys(path, value, REPORT_KEYS):
            return
        if value["basis"] not in BASES:
            self.c.fail(f"{path}.basis", f"unknown basis {value['basis']!r}")
        if value["branch"] not in ("recurrence", "scalar"):
            self.c.fail(f"{path}.branch", f"unknown branch {value['branch']!r}")
        if value["execution"] not in (EXEC_CERTIFIED, EXEC_TIMING):
            self.c.fail(f"{path}.execution", f"unknown execution {value['execution']!r}")
        self.c.bool_(f"{path}.evidence_verified", value["evidence_verified"])
        self.c.int_(f"{path}.degree", value["degree"])
        self.c.hex_vector(f"{path}.fused", value["fused"], n)
        self.c.bool_(f"{path}.total_error_is_estimate", value["total_error_is_estimate"])
        self.hex_or_null(f"{path}.candidate_total", value["candidate_total"])
        self.c.bool_(f"{path}.coefficient_cache_hit", value["coefficient_cache_hit"])

    def parity(self, path: str, value, n: int) -> None:
        if value is None or not self.c.keys(path, value, PARITY_KEYS):
            return
        self.c.bool_(f"{path}.report_bitwise_equal", value["report_bitwise_equal"])
        self.c.bool_(f"{path}.counters_equal", value["counters_equal"])
        self.c.hex_vector(f"{path}.replay_fused", value["replay_fused"], n)
        self.hex_or_null(f"{path}.replay_candidate_total", value["replay_candidate_total"])

    def fallback(self, path: str, value, choice) -> None:
        if choice == "Fallback":
            if self.c.keys(path, value, FALLBACK_KEYS):
                self.c.str_(f"{path}.chebyshev", value["chebyshev"])
                self.c.str_(f"{path}.laguerre", value["laguerre"])
        elif value is not None:
            self.c.fail(path, "only a fallback has reasons")


def _router_record(v: _V, p: str, rec: dict, n: int, arm: str) -> None:
    c = v.c
    if not c.keys(p, rec, ROUTER_RECORD_KEYS):
        return
    c.equal(f"{p}.policy", rec["policy"], POLICY[arm])
    if rec["choice"] not in CHOICES:
        c.fail(f"{p}.choice", f"unknown choice {rec['choice']!r}")
    v.fallback(f"{p}.fallback_reasons", rec["fallback_reasons"], rec["choice"])
    if rec["fused"] is not None:
        c.hex_vector(f"{p}.fused", rec["fused"], n)
    v.hex_or_null(f"{p}.bound", rec["bound"])
    expected_bases = ["Chebyshev"] if arm == "cheb" else ["Chebyshev", "Laguerre"]
    if c.list_(f"{p}.plans", rec["plans"]):
        if [x.get("basis") if isinstance(x, dict) else None for x in rec["plans"]] != expected_bases:
            c.fail(f"{p}.plans", f"bases are not {expected_bases}")
        for i, plan in enumerate(rec["plans"]):
            q = f"{p}.plans[{i}]"
            if not c.keys(q, plan, PLAN_KEYS):
                continue
            v.cost(f"{q}.cost", plan["cost"])
            if (plan["plan"] is None) == (plan["error"] is None):
                c.fail(q, "exactly one of plan and error")
            v.str_or_null(f"{q}.error", plan["error"])
            body = plan["plan"]
            if body is not None and c.keys(f"{q}.plan", body, PLAN_BODY_KEYS):
                if body["branch"] not in ("recurrence", "scalar", "zero-input"):
                    c.fail(f"{q}.plan.branch", f"unknown branch {body['branch']!r}")
                for key in ("degree", "width", "predicted_vector_products", "predicted_block_products",
                            "tail_evaluations"):
                    c.int_(f"{q}.plan.{key}", body[key])
                c.int_(f"{q}.plan.normalization_shift", body["normalization_shift"], minimum=None)
                c.bool_(f"{q}.plan.coefficients_cached", body["coefficients_cached"])
                for key in ("laguerre_scale", "beta", "laguerre_extent"):
                    v.hex_or_null(f"{q}.plan.{key}", body[key])
    pf = rec["preflight"]
    if c.keys(f"{p}.preflight", pf, PREFLIGHT_KEYS):
        if pf["preselected"] not in (None,) + BASES:
            c.fail(f"{p}.preflight.preselected", f"unknown basis {pf['preselected']!r}")
        v.str_or_null(f"{p}.preflight.abstain_reason", pf["abstain_reason"])
        c.bool_(f"{p}.preflight.laguerre_table_used", pf["laguerre_table_used"])
        v.hex_or_null(f"{p}.preflight.vector_independent_terms", pf["vector_independent_terms"])
        v.cost(f"{p}.preflight.cost", pf["cost"])
        v.work(f"{p}.preflight.work", pf["work"])
    if c.list_(f"{p}.executions", rec["executions"]):
        for i, ex in enumerate(rec["executions"]):
            q = f"{p}.executions[{i}]"
            if not c.keys(q, ex, EXEC_KEYS):
                continue
            if ex["basis"] not in BASES:
                c.fail(f"{q}.basis", f"unknown basis {ex['basis']!r}")
            c.bool_(f"{q}.executed", ex["executed"])
            v.int_or_null(f"{q}.degree", ex["degree"])
            v.admission(f"{q}.admission", ex["admission"])
            v.cost(f"{q}.cost", ex["cost"])
            v.work(f"{q}.work", ex["work"])
            v.report(f"{q}.report", ex["report"], n)
            v.parity(f"{q}.parity", ex["parity"], n)
            if (ex["report"] is None) != (ex["parity"] is None):
                c.fail(q, "a report needs its parity record and vice versa")
    v.cost(f"{p}.cost", rec["cost"])
    v.work(f"{p}.caller_counters", rec["caller_counters"])
    for key in ("caches_before", "caches_after"):
        if c.keys(f"{p}.{key}", rec[key], CACHES_KEYS):
            c.int_(f"{p}.{key}.coefficient_tables", rec[key]["coefficient_tables"])
            c.int_(f"{p}.{key}.envelopes", rec[key]["envelopes"])


def _pp08_record(v: _V, p: str, rec: dict, n: int) -> None:
    c = v.c
    if not c.keys(p, rec, PP08_RECORD_KEYS):
        return
    if rec["choice"] not in CHOICES:
        c.fail(f"{p}.choice", f"unknown choice {rec['choice']!r}")
    v.fallback(f"{p}.fallback_reasons", rec["fallback_reasons"], rec["choice"])
    if rec["fused"] is not None:
        c.hex_vector(f"{p}.fused", rec["fused"], n)
    v.hex_or_null(f"{p}.bound", rec["bound"])
    if c.list_(f"{p}.attempts", rec["attempts"]):
        if [x.get("basis") if isinstance(x, dict) else None for x in rec["attempts"]] != list(BASES):
            c.fail(f"{p}.attempts", "bases are not Chebyshev, Laguerre")
        for i, at in enumerate(rec["attempts"]):
            q = f"{p}.attempts[{i}]"
            if not c.keys(q, at, ATTEMPT_KEYS):
                continue
            v.int_or_null(f"{q}.degree", at["degree"])
            v.admission(f"{q}.admission", at["admission"])
            c.int_(f"{q}.vector_products", at["vector_products"])
            v.work(f"{q}.work", at["work"])
            v.report(f"{q}.report", at["report"], n)
    v.work(f"{p}.caller_counters", rec["caller_counters"])


def _sum_costs(costs: list[dict]) -> dict:
    return {k: sum(x[k] for x in costs) for k in COST_KEYS}


def _decode_case(v: _V, p: str, case: dict) -> dict | None:
    """Types and shapes of one case; returns the decoded inputs or None."""
    c = v.c
    before = len(c.reasons)
    if not c.keys(p, case, CASE_KEYS):
        return None
    if not c.int_(f"{p}.n", case["n"], minimum=2):
        return None
    n = case["n"]
    c.str_(f"{p}.group", case["group"])
    c.str_(f"{p}.family", case["family"])
    c.float_(f"{p}.rho_nominal", case["rho_nominal"], positive=True)
    v.int_or_null(f"{p}.seed", case["seed"])
    c.int_(f"{p}.input_seed", case["input_seed"])
    if case["input"] not in GRID_INPUTS:
        c.fail(f"{p}.input", f"unknown input {case['input']!r}")
    a = None
    if c.list_(f"{p}.a", case["a"], nonempty=True) and len(case["a"]) == n:
        rows = [c.hex_vector(f"{p}.a[{i}]", row, n) for i, row in enumerate(case["a"])]
        a = rows if all(r is not None for r in rows) else None
    else:
        c.fail(f"{p}.a", f"not {n} rows")
    w = None
    if c.list_(f"{p}.w", case["w"], nonempty=True) and len(case["w"]) == TERMS:
        rows = [c.hex_vector(f"{p}.w[{k}]", row, n) for k, row in enumerate(case["w"])]
        w = rows if all(r is not None for r in rows) else None
    else:
        c.fail(f"{p}.w", "not five vectors")
    h = v.hex_(f"{p}.h", case["h"])
    budget = v.hex_(f"{p}.total_budget", case["total_budget"])
    enc = case["enclosure"]
    if c.keys(f"{p}.enclosure", enc, ENCLOSURE_KEYS):
        v.hex_(f"{p}.enclosure.lambda", enc["lambda"])
        v.hex_(f"{p}.enclosure.rho", enc["rho"])
        c.bool_(f"{p}.enclosure.verified", enc["verified"])
    if c.list_(f"{p}.records", case["records"], nonempty=True):
        for i, rec in enumerate(case["records"]):
            q = f"{p}.records[{i}]"
            if not c.dict_(q, rec):
                continue
            arm = rec.get("arm")
            if rec.get("cache_state") not in STATES:
                c.fail(f"{q}.cache_state", f"unknown cache state {rec.get('cache_state')!r}")
            if arm in ROUTER_ARMS:
                _router_record(v, q, rec, n, arm)
            elif arm == "pp08":
                _pp08_record(v, q, rec, n)
            else:
                c.fail(f"{q}.arm", f"unknown arm {arm!r}")
    if case["timing"] is not None and c.list_(f"{p}.timing", case["timing"]):
        if [x.get("basis") if isinstance(x, dict) else None for x in case["timing"]] != list(BASES):
            c.fail(f"{p}.timing", "bases are not Chebyshev, Laguerre")
        for i, t in enumerate(case["timing"]):
            q = f"{p}.timing[{i}]"
            if c.keys(q, t, TIMING_KEYS):
                c.str_(f"{q}.execution", t["execution"])
                v.admission(f"{q}.admission_at_total_budget", t["admission_at_total_budget"])
                v.admission(f"{q}.admission_at_1e300", t["admission_at_1e300"])
                v.report(f"{q}.report", t["report"], n)
                v.parity(f"{q}.parity", t["parity"], n)
    if len(c.reasons) != before or a is None or w is None or h is None or budget is None:
        return None
    return {"n": n, "a": a, "w": w, "h": h, "budget": budget}


def _expected_arms(group: str) -> set:
    arms = {"single", "cheb", "pp08"}
    if group in (FORCE_GROUP, "degree-above-limit"):
        arms.add("force_laguerre_first")
    return {(arm, state) for arm in arms for state in STATES}


def _laguerre_total_rejected(rec: dict) -> bool:
    """A PP08 record whose Laguerre attempt executed with degree <= 128 and was rejected."""
    at = rec["attempts"][1]
    return (at["report"] is not None and at["degree"] is not None and at["degree"] <= DEGREE_LIMIT
            and not at["admission"]["admitted"])


def _bind(c: evidence._Checks, raw: dict, decoded: dict, grid: list[dict], doc: dict) -> None:
    """Binding of every case to the registered construction."""
    cases = raw
    seeds = []
    for g in grid:
        case, d = raw.get(g["label"]), decoded.get(g["label"])
        if case is None or d is None:
            continue
        p = f"cases[{g['label']}]"
        c.equal(f"{p}.group", case["group"], "grid")
        c.equal(f"{p}.family", case["family"], g["family"])
        c.equal(f"{p}.n", case["n"], g["n"])
        c.equal(f"{p}.rho_nominal", case["rho_nominal"], g["rho"])
        c.equal(f"{p}.input", case["input"], g["input"])
        c.equal(f"{p}.input_seed", case["input_seed"], g["input_seed"])
        if d["h"] != g["h"]:
            c.fail(f"{p}.h", f"{d['h']!r} != {g['h']!r}")
        if d["budget"] != g["budget"]:
            c.fail(f"{p}.total_budget", f"{d['budget']!r} != {g['budget']!r}")
        enc = case["enclosure"]
        if not enc["verified"]:
            c.fail(f"{p}.enclosure", "a grid enclosure must be verified")
        if g["family"] == "diagonal":
            if d["a"] != diagonal_matrix(g["n"], g["rho"]):
                c.fail(f"{p}.a", "differs from the registered diagonal construction")
            if unhex(enc["lambda"]) != 0.0 or unhex(enc["rho"]) != g["rho"] or case["seed"] is not None:
                c.fail(f"{p}.enclosure", "the diagonal enclosure is [-rho, 0] without a seed")
        else:
            if not isinstance(case["seed"], int) or case["seed"] < FIRST_ROTATED_SEED:
                c.fail(f"{p}.seed", f"a rotated seed starts at {FIRST_ROTATED_SEED}")
            else:
                seeds.append((g["n"], g["rho"], case["seed"]))
            a = d["a"]
            if any(a[i][j] != a[j][i] for i in range(g["n"]) for j in range(i)):
                c.fail(f"{p}.a", "not symmetric")
    # Rotated seeds: one per (n, rho), increasing in the registered fixture order.
    per_fixture = {}
    for n, rho, seed in seeds:
        per_fixture.setdefault((n, rho), set()).add(seed)
    order = [(n, rho) for n in GRID_N for rho in GRID_RHO]
    chosen = []
    for key in order:
        found = per_fixture.get(key, set())
        if len(found) != 1:
            c.fail("cases.rotated", f"fixture n={key[0]} rho={key[1]} has seeds {sorted(found)}")
        else:
            chosen.append(next(iter(found)))
    if len(chosen) == len(order) and (chosen != sorted(chosen) or len(set(chosen)) != len(chosen)):
        c.fail("cases.rotated", f"seeds {chosen} are not increasing in the registered order")
    # Inputs: every w_k is the registered stream of its seed.
    for label, case in raw.items():
        d = decoded.get(label)
        if d is None:
            continue
        expected = registered_w(case["input"], d["n"], case["input_seed"])
        if d["w"] != expected:
            c.fail(f"cases[{label}].w", "differs from the registered splitmix stream of its input seed")

    def grid_a(family: str, n: int, rho: float):
        for g in grid:
            if g["family"] == family and g["n"] == n and g["rho"] == rho and g["label"] in decoded:
                return decoded[g["label"]]["a"], raw[g["label"]]
        return None, None

    def boundary(label: str, family: str, n: int, rho: float, h: float, budget: float, k: int) -> None:
        case, d = raw.get(label), decoded.get(label)
        if case is None or d is None:
            return
        p = f"cases[{label}]"
        c.equal(f"{p}.group", case["group"], BOUNDARY[label])
        c.equal(f"{p}.family", case["family"], family)
        c.equal(f"{p}.input", case["input"], "distinct")
        c.equal(f"{p}.input_seed", case["input_seed"], 200_000 + 1000 * k)
        if d["n"] != n or (h is not None and d["h"] != h) or d["budget"] != budget:
            c.fail(p, "n, h or total budget differs from the registered boundary case")

    rot64, rot64_case = grid_a("rotated", 8, 64.0)
    diag64, _ = grid_a("diagonal", 8, 64.0)
    rot4, _ = grid_a("rotated", 8, 4.0)
    boundary("declared-enclosure", "rotated", 8, 64.0, 0.05, 1e-10, 1)
    boundary("unbounded-timing", "diagonal", 8, 64.0, 0.05, 1e-10, 2)
    boundary("laguerre-degree-above-limit", "diagonal", 8, 64.0, None, 1e-10, 3)
    boundary("total-budget-zero", "rotated", 8, 4.0, 0.05, 0.0, 4)
    boundary("scalar-minus3I", "scalar", 8, 3.0, 0.12, 1e-10, 5)
    if "declared-enclosure" in decoded:
        case = raw["declared-enclosure"]
        if rot64 is not None and decoded["declared-enclosure"]["a"] != rot64:
            c.fail("cases[declared-enclosure].a", "not the rotated n = 8, rho = 64 fixture")
        if case["enclosure"]["verified"]:
            c.fail("cases[declared-enclosure].enclosure", "the enclosure must be declared")
        diag_max = max(abs(decoded["declared-enclosure"]["a"][i][i]) for i in range(8))
        if unhex(case["enclosure"]["rho"]) != diag_max or unhex(case["enclosure"]["lambda"]) != 0.0:
            c.fail("cases[declared-enclosure].enclosure", "not [-max |a_ii|, 0]")
        if rot64_case is not None and case["seed"] != rot64_case["seed"]:
            c.fail("cases[declared-enclosure].seed", "not the rotated fixture's seed")
    for label in ("unbounded-timing", "laguerre-degree-above-limit"):
        if label in decoded and diag64 is not None and decoded[label]["a"] != diag64:
            c.fail(f"cases[{label}].a", "not the diagonal n = 8, rho = 64 fixture")
    if "total-budget-zero" in decoded and rot4 is not None and decoded["total-budget-zero"]["a"] != rot4:
        c.fail("cases[total-budget-zero].a", "not the rotated n = 8, rho = 4 fixture")
    if "scalar-minus3I" in decoded:
        expected = [[-3.0 if i == j else 0.0 for j in range(8)] for i in range(8)]
        enc = raw["scalar-minus3I"]["enclosure"]
        if decoded["scalar-minus3I"]["a"] != expected:
            c.fail("cases[scalar-minus3I].a", "not -3 I")
        if unhex(enc["lambda"]) != 3.0 or unhex(enc["rho"]) != 3.0 or not enc["verified"]:
            c.fail("cases[scalar-minus3I].enclosure", "not the verified [-3, -3]")
    for label, case in raw.items():
        if case["group"] not in ("grid", FORCE_GROUP, "declared") and label in BOUNDARY:
            if not case["enclosure"]["verified"]:
                c.fail(f"cases[{label}].enclosure", "every boundary enclosure but the declared one is verified")
    # Degree above the limit: its Laguerre plan exceeds 128 at a registered step.
    if "laguerre-degree-above-limit" in decoded:
        d = decoded["laguerre-degree-above-limit"]
        if d["h"] not in DEGREE_ABOVE_LIMIT_STEPS:
            c.fail("cases[laguerre-degree-above-limit].h", "not a registered step")
        for rec in raw["laguerre-degree-above-limit"]["records"]:
            if rec.get("arm") == "single":
                plan = rec["plans"][1]["plan"]
                if plan is None or plan["degree"] <= DEGREE_LIMIT:
                    c.fail("cases[laguerre-degree-above-limit]", "its Laguerre plan does not exceed 128")
    # ForceLaguerreFirst: one per family, by the registered selection rule.
    selection = doc["force_selection"]
    force = {label: case for label, case in raw.items() if case["group"] == FORCE_GROUP}
    families = sorted(case["family"] for case in force.values())
    if families != list(GRID_FAMILIES):
        c.fail("cases.force", f"force families {families} are not one per family")
    if not (isinstance(selection, list) and len(selection) == 2
            and all(c.keys(f"force_selection[{i}]", s, SELECTION_KEYS) for i, s in enumerate(selection))):
        c.fail("force_selection", "not two selection records")
        return
    for slot, family in enumerate(GRID_FAMILIES):
        s = selection[slot]
        if s["family"] != family or s["label"] not in force or force[s["label"]]["family"] != family:
            c.fail(f"force_selection[{slot}]", "does not name its family's force case")
            continue
        case, d = force[s["label"]], decoded.get(s["label"])
        if d is None:
            continue
        rows = [g for g in grid if g["family"] == family]
        qualifying = None
        for g in rows:
            source = cases.get(g["label"])
            if source is None:
                continue
            pp08 = [r for r in source["records"] if r.get("arm") == "pp08" and r.get("cache_state") == "cold"]
            if len(pp08) == 1 and isinstance(pp08[0].get("attempts"), list) and len(pp08[0]["attempts"]) == 2 \
                    and _laguerre_total_rejected(pp08[0]):
                qualifying = g
                break
        if qualifying is not None:
            expected = "force-" + qualifying["label"]
            if s["rule"] != "first-grid-case" or s["label"] != expected:
                c.fail(f"force_selection[{slot}]",
                       f"the rule first-grid-case selects {expected}; recorded {s['rule']} {s['label']}")
            elif d["a"] != decoded.get(qualifying["label"], {}).get("a") or case["input_seed"] != \
                    qualifying["input_seed"] or case["input"] != qualifying["input"] or d["h"] != \
                    qualifying["h"] or d["budget"] != qualifying["budget"]:
                c.fail(f"cases[{s['label']}]", "differs from its source grid case")
        else:
            if s["rule"] != "fallback-budget":
                c.fail(f"force_selection[{slot}]", "no grid case qualifies, so the fallback rule applies")
            src, _ = grid_a(family, 8, 64.0)
            if src is not None and d["a"] != src or d["h"] != 0.12 or case["input"] != "distinct" or \
                    case["input_seed"] != 200_000 + 1000 * (6 + slot) or d["budget"] not in FORCE_FALLBACK_BUDGETS:
                c.fail(f"cases[{s['label']}]", "not the registered fallback construction")


def validate(cases_path: Path, blob_at=None, tree_status=None) -> evidence.Evidence:
    """Every INVALID condition; returns the validated document with decoded inputs and references."""
    blob_at = blob_at or checker_blob_at
    tree_status = tree_status or own_tree_status
    doc, digest = load(cases_path)
    c = evidence._Checks()
    v = _V(c)
    if not c.keys("cases", doc, DOC_KEYS):
        c.done()
    c.equal("schema", doc["schema"], CASES_SCHEMA)
    c.equal("node", doc["node"], NODE)
    if c.str_("source_commit", doc["source_commit"]) and (
            len(doc["source_commit"]) != 40 or any(ch not in "0123456789abcdef" for ch in doc["source_commit"])):
        c.fail("source_commit", "not a 40-digit commit id")
    if c.str_("tree_status", doc["tree_status"], nonempty=False) and doc["tree_status"] != "":
        c.fail("tree_status", f"the export tree was dirty: {doc['tree_status'][:200]!r}")
    status = tree_status()
    if status != "":
        c.fail("checker.tree_status", f"the checker tree is dirty: {status[:200]!r}")
    if isinstance(doc.get("source_commit"), str) and len(doc["source_commit"]) == 40:
        try:
            if blob_at(doc["source_commit"]) != Path(__file__).read_bytes():
                c.fail("provenance", f"the checker blob at {doc['source_commit']} differs from the checker run")
        except evidence.ValidationError as exc:
            c.fail("provenance", exc.reasons[0])
    if c.keys("constants", doc["constants"], CONSTANT_KEYS):
        k = doc["constants"]
        c.equal("constants.truncation_fraction", k["truncation_fraction"], 0.25)
        c.equal("constants.degree_limit", k["degree_limit"], DEGREE_LIMIT)
        c.equal("constants.scales", k["scales"], [hx(s) for s in SCALES])
        c.equal("constants.first_rotated_seed", k["first_rotated_seed"], FIRST_ROTATED_SEED)
        c.equal("constants.lambda_min_fraction", k["lambda_min_fraction"], 0.1)
        c.equal("constants.max_givens_angle", k["max_givens_angle"], 0.3)
    grid = grid_cases()
    raw: dict = {}
    decoded: dict = {}
    if c.list_("cases", doc["cases"], nonempty=True):
        rows = doc["cases"]
        index = c.unique_index("cases", [x if isinstance(x, dict) else {} for x in rows], lambda r: (r.get("label"),))
        labels = [x.get("label") if isinstance(x, dict) else None for x in rows]
        expected = {g["label"] for g in grid} | set(BOUNDARY)
        force = [lab for lab in labels if isinstance(lab, str) and lab.startswith("force-")]
        if set(labels) - set(force) != expected or len(labels) != len(expected) + 2 or len(force) != 2:
            missing = sorted(expected - set(labels))[:5]
            extra = sorted(set(labels) - expected - set(force))[:5]
            c.fail("cases", f"not the registered case set: {len(labels)} cases, missing {missing}, unexpected "
                            f"{extra}, {len(force)} force cases")
        for (label,), case in index.items():
            if not isinstance(label, str):
                continue
            p = f"cases[{label}]"
            d = _decode_case(v, p, case)
            if d is None:
                continue
            raw[label] = case
            decoded[label] = d
            group = case["group"]
            if label.startswith("force-") and group != FORCE_GROUP:
                c.fail(f"{p}.group", f"a force case has group {FORCE_GROUP}")
            arm_rows = c.unique_index(f"{p}.records", case["records"],
                                      lambda r: (r.get("arm"), r.get("cache_state")))
            if set(arm_rows) != _expected_arms(group) or len(case["records"]) != len(_expected_arms(group)):
                c.fail(f"{p}.records", f"(arm, cache state) set {sorted(arm_rows)} is not the registered one")
            if (group == "timing") != (case["timing"] is not None):
                c.fail(f"{p}.timing", "timing executions belong to the timing case only")
            # Export arithmetic.
            for rec in case["records"]:
                if rec.get("arm") in ROUTER_ARMS and isinstance(rec.get("cost"), dict):
                    parts = [pl["cost"] for pl in rec["plans"]] + [rec["preflight"]["cost"]] + \
                            [ex["cost"] for ex in rec["executions"]]
                    if _sum_costs(parts) != {k: rec["cost"][k] for k in COST_KEYS}:
                        c.fail(f"{p}.records[{rec['arm']},{rec['cache_state']}].cost",
                               "differs from the sum of its plan, preflight and execution costs")
        c.done()
        _bind(c, raw, decoded, grid, doc)
    c.done()
    # References at 50 and 70 digits.
    import mpmath as mp
    tolerance = mp.mpf(REFERENCE_AGREEMENT)
    for label, d in decoded.items():
        f50 = exact_target(d["a"], d["h"], d["w"], DPS)
        f70 = exact_target(d["a"], d["h"], d["w"], DPS_CHECK)
        with mp.workdps(DPS_CHECK):
            norm = mp.sqrt(mp.fsum(x ** 2 for x in f70))
            gap = mp.sqrt(mp.fsum((x - y) ** 2 for x, y in zip(f50, f70)))
            if gap > tolerance * norm:
                c.fail(f"cases[{label}].reference", f"50- and 70-digit references differ by {mp.nstr(gap, 5)}")
        d["reference"] = f50
        d["reference_norm"] = norm
    c.done()
    return evidence.Evidence("PY03", {"cases": doc, "raw": raw, "decoded": decoded}, {"cases": digest},
                             {"cases": Path(cases_path)})


# --------------------------------------------------------------------------------------------- gate
def records(case: dict) -> dict:
    return {(r["arm"], r["cache_state"]): r for r in case["records"]}


def admitted_executions(rec: dict) -> list[dict]:
    rows = rec["executions"] if rec["arm"] != "pp08" else rec["attempts"]
    return [x for x in rows if x["admission"]["admitted"]]


def certification_operations(cost: dict) -> int:
    return cost["chebyshev_propagation_terms"] + cost["envelope_interval_operations"] + cost["adjoint_sum_terms"]


def _forbidden_admission(x: dict) -> str | None:
    """Why an admitted execution or attempt must never have been admitted, or None."""
    rep = x["report"]
    if rep is None:
        return "admitted without a report"
    if rep["execution"] != EXEC_CERTIFIED:
        return f"admitted a {rep['execution']} execution"
    if not rep["evidence_verified"]:
        return "admitted a declared enclosure"
    if rep["basis"] == "Laguerre" and rep["branch"] == "recurrence":
        if rep["degree"] > DEGREE_LIMIT:
            return f"admitted a Laguerre degree {rep['degree']} > {DEGREE_LIMIT}"
        if rep["candidate_total"] is None:
            return "admitted a Laguerre estimate-only total"
    elif rep["total_error_is_estimate"]:
        return "admitted an estimate-only total"
    return None


def item_certification(raw: dict, decoded: dict) -> dict:
    import mpmath as mp
    problems, checked, tightness = [], 0, []
    for label, case in raw.items():
        d = decoded[label]
        budget = d["budget"]
        recs = records(case)
        for (arm, state), rec in recs.items():
            where = f"{label} {arm} {state}"
            admitted = admitted_executions(rec)
            for x in admitted:
                why = _forbidden_admission(x)
                if why:
                    problems.append(f"{where}: {why}")
                if unhex(x["admission"]["bound"]) > budget:
                    problems.append(f"{where}: admitted bound above the total budget")
            if rec["choice"] == "Fallback":
                if admitted or rec["fused"] is not None or rec["bound"] is not None:
                    problems.append(f"{where}: a fallback with an admitted execution, result or bound")
            elif not admitted or rec["fused"] is None or rec["bound"] is None:
                problems.append(f"{where}: a choice without an admitted execution, result and bound")
            if arm not in ("single", "force_laguerre_first") or rec["choice"] == "Fallback" or not admitted:
                continue
            chosen = admitted[-1]
            if chosen["basis"] != rec["choice"] or chosen["admission"]["bound"] != rec["bound"] or \
                    chosen["report"] is None or chosen["report"]["fused"] != rec["fused"]:
                problems.append(f"{where}: the route's result is not its admitted execution's")
                continue
            if rec["executions"][-1] is not chosen:
                problems.append(f"{where}: an execution ran after the admitted one")
            bound = unhex(rec["bound"])
            error = distance([unhex(x) for x in rec["fused"]], d["reference"])
            checked += 1
            if bound > budget:
                problems.append(f"{where}: bound {bound:.3e} > total budget {budget:.3e}")
            if error > bound:
                problems.append(f"{where}: error {mp.nstr(error, 6)} > bound {bound:.6e}")
            tightness.append(float(bound / error) if error > 0 else math.inf)
        if case["group"] in ("declared", "budget-zero"):
            for (arm, state), rec in recs.items():
                if rec["choice"] != "Fallback":
                    problems.append(f"{label} {arm} {state}: must end in Fallback")
        if case["timing"] is not None:
            for t in case["timing"]:
                if t["execution"] != EXEC_TIMING or t["report"]["execution"] != EXEC_TIMING or \
                        t["admission_at_total_budget"]["admitted"] or t["admission_at_1e300"]["admitted"]:
                    problems.append(f"{label} timing {t['basis']}: a timing execution was admitted")
    finite = sorted(x for x in tightness if math.isfinite(x))
    return {"pass": not problems and checked > 0, "admitted_results_checked": checked,
            "tightness_min": finite[0] if finite else None,
            "tightness_median": finite[len(finite) // 2] if finite else None,
            "tightness_max": finite[-1] if finite else None, "problems": problems[:50],
            "problems_total": len(problems)}


def item_parity(raw: dict) -> dict:
    problems, executions = [], 0
    for label, case in raw.items():
        recs = records(case)
        for (arm, state), rec in recs.items():
            if arm not in ROUTER_ARMS:
                continue
            for i, ex in enumerate(rec["executions"]):
                if not ex["executed"]:
                    continue
                executions += 1
                where = f"{label} {arm} {state} execution {i}"
                rep, par = ex["report"], ex["parity"]
                if rep is None:
                    problems.append(f"{where}: executed without a report (parity not verifiable)")
                    continue
                if not (par["report_bitwise_equal"] and par["counters_equal"]
                        and par["replay_fused"] == rep["fused"]
                        and par["replay_candidate_total"] == rep["candidate_total"]):
                    problems.append(f"{where}: differs from joint_phi_action")
        if case["timing"] is not None:
            for t in case["timing"]:
                executions += 1
                par, rep = t["parity"], t["report"]
                if not (par["report_bitwise_equal"] and par["counters_equal"] and par["replay_fused"] == rep["fused"]
                        and par["replay_candidate_total"] == rep["candidate_total"]):
                    problems.append(f"{label} timing {t['basis']}: differs from joint_phi_action_unbounded")
        for state in STATES:
            single, cheb = recs[("single", state)], recs[("cheb", state)]
            if single["choice"] == "Chebyshev":
                if cheb["choice"] != "Chebyshev" or single["fused"] != cheb["fused"] or \
                        single["bound"] != cheb["bound"]:
                    problems.append(f"{label} {state}: single's Chebyshev result differs from cheb's")
    return {"pass": not problems and executions > 0, "executions_compared": executions,
            "problems": problems[:50], "problems_total": len(problems)}


def _add_work(parts: list[dict]) -> dict:
    return {k: sum(p[k] for p in parts) for k in WORK_KEYS}


def item_accounting(raw: dict) -> dict:
    problems, checked = [], 0
    for label, case in raw.items():
        recs = records(case)
        for (arm, state), rec in recs.items():
            where = f"{label} {arm} {state}"
            checked += 1
            caller = rec["caller_counters"]
            if arm == "pp08":
                if _add_work([a["work"] for a in rec["attempts"]]) != {k: caller[k] for k in WORK_KEYS}:
                    problems.append(f"{where}: caller counters differ from the sum of the attempts")
                continue
            parts = [rec["preflight"]["work"]] + [ex["work"] for ex in rec["executions"]]
            if _add_work(parts) != {k: caller[k] for k in WORK_KEYS}:
                problems.append(f"{where}: caller counters differ from the preflight plus every execution")
            cost = rec["cost"]
            for ck, wk in COST_WORK:
                if cost[ck] != caller[wk]:
                    problems.append(f"{where}: cost {ck} {cost[ck]} != counters {wk} {caller[wk]}")
            plans = {pl["basis"]: pl for pl in rec["plans"]}
            for pl in rec["plans"]:
                if pl["plan"] is not None and pl["cost"]["plan_tail_evaluations"] != pl["plan"]["tail_evaluations"]:
                    problems.append(f"{where}: {pl['basis']} plan tail evaluations not charged")
            for i, ex in enumerate(rec["executions"]):
                e = f"{where} execution {i}"
                for ck, wk in COST_WORK:
                    if ex["cost"][ck] != ex["work"][wk]:
                        problems.append(f"{e}: cost {ck} != counters {wk}")
                if ex["cost"]["executed_backends"] != int(ex["executed"]):
                    problems.append(f"{e}: executed backend not counted once")
                if ex["cost"]["failed_executions"] != int(not ex["admission"]["admitted"]):
                    problems.append(f"{e}: failed execution not counted once")
                plan = plans.get(ex["basis"], {}).get("plan")
                if ex["report"] is not None:
                    if plan is None or ex["cost"]["vector_products"] != plan["predicted_vector_products"] or \
                            ex["report"]["degree"] != plan["degree"]:
                        problems.append(f"{e}: products or degree differ from the plan's prediction")
            if arm == "force_laguerre_first" and case["group"] == FORCE_GROUP:
                ex = rec["executions"]
                ok = (len(ex) == 2 and ex[0]["basis"] == "Laguerre" and ex[0]["executed"]
                      and ex[0]["report"] is not None and not ex[0]["admission"]["admitted"]
                      and ex[0]["cost"]["vector_products"] > 0 and ex[0]["cost"]["failed_executions"] == 1
                      and ex[1]["basis"] == "Chebyshev" and ex[1]["executed"])
                if not ok:
                    problems.append(f"{where}: the failed Laguerre execution is not charged before Chebyshev")
    return {"pass": not problems and checked > 0, "records_checked": checked, "problems": problems[:50],
            "problems_total": len(problems)}


def pp08_products(rec: dict) -> int:
    return sum(a["vector_products"] for a in rec["attempts"])


def item_one_execution(raw: dict) -> dict:
    problems, strict_cases = [], 0
    for label, case in raw.items():
        recs = records(case)
        for state in STATES:
            single, pp08 = recs[("single", state)], recs[("pp08", state)]
            where = f"{label} {state}"
            ex = single["executions"]
            if ex and ex[0]["admission"]["admitted"]:
                if len(ex) != 1 or single["cost"]["executed_backends"] != 1 or \
                        single["cost"]["vector_products"] != ex[0]["cost"]["vector_products"]:
                    problems.append(f"{where}: more than one recurrence after an admitted first execution")
            ps, pp = single["cost"]["vector_products"], pp08_products(pp08)
            if ps > pp:
                problems.append(f"{where}: single {ps} > pp08 {pp} vector products")
            if all(a["vector_products"] > 0 for a in pp08["attempts"]):
                strict_cases += 1
                if not ps < pp:
                    problems.append(f"{where}: single {ps} not < pp08 {pp} where pp08 executed both bases")
    return {"pass": not problems, "strict_comparisons": strict_cases, "problems": problems[:50],
            "problems_total": len(problems)}


def niche(raw: dict) -> list[str]:
    return [label for label, case in raw.items() if case["group"] == "grid"
            and records(case)[("single", "cold")]["preflight"]["preselected"] == "Laguerre"]


def item_niche(raw: dict) -> dict:
    members = niche(raw)
    problems, rows = [], []
    for label in members:
        recs = records(raw[label])
        for state in STATES:
            single, cheb = recs[("single", state)], recs[("cheb", state)]
            s, c = single["cost"], cheb["cost"]
            row = {"label": label, "state": state, "choice": single["choice"],
                   "vector_products": [s["vector_products"], c["vector_products"]],
                   "coefficient_series_terms": [s["coefficient_series_terms"], c["coefficient_series_terms"]],
                   "certification_operations": [certification_operations(s), certification_operations(c)]}
            rows.append(row)
            if single["choice"] != "Laguerre":
                problems.append(f"{label} {state}: the selected Laguerre result is not admitted")
            if not s["vector_products"] < c["vector_products"]:
                problems.append(f"{label} {state}: vector products not below cheb")
            if s["coefficient_series_terms"] > c["coefficient_series_terms"]:
                problems.append(f"{label} {state}: coefficient-series terms above cheb")
            if certification_operations(s) > certification_operations(c):
                problems.append(f"{label} {state}: certification operations above cheb")
    if len(members) < NICHE_MIN:
        problems.insert(0, f"niche empty: |N| = {len(members)} < {NICHE_MIN}")
    return {"pass": not problems, "niche_size": len(members), "niche_minimum": NICHE_MIN, "niche": members,
            "rows": rows, "problems": problems[:50], "problems_total": len(problems)}


def reported(raw: dict, gate: dict) -> dict:
    ratios, abstentions, one_basis, overhead = [], {}, [], []
    grid_equal, table_free, tails = True, True, True
    pp08_ratio_range = [math.inf, -math.inf]
    for label, case in raw.items():
        recs = records(case)
        for state in STATES:
            single, cheb, pp08 = recs[("single", state)], recs[("cheb", state)], recs[("pp08", state)]
            ps, pc, pp = (single["cost"]["vector_products"], cheb["cost"]["vector_products"], pp08_products(pp08))
            ratios.append({"label": label, "state": state, "single": ps, "cheb": pc, "pp08": pp,
                           "single_over_cheb": ps / pc if pc else None,
                           "single_over_pp08": ps / pp if pp else None})
            lag_plan = single["plans"][1]
            overhead.append({"label": label, "state": state,
                             "laguerre_plan_tail_evaluations": lag_plan["cost"]["plan_tail_evaluations"],
                             "chebyshev_plan_tail_evaluations": single["plans"][0]["cost"]["plan_tail_evaluations"],
                             "laguerre_table_in_preflight": single["preflight"]["laguerre_table_used"],
                             "preflight_coefficient_setups": single["preflight"]["cost"]["coefficient_setups"]})
            reason = single["preflight"]["abstain_reason"]
            key = reason.split(":")[0] if reason else "none (Laguerre preselected)"
            abstentions[key] = abstentions.get(key, 0) + 1
            if case["group"] == "grid":
                grid_equal &= ps == pc
                table_free &= not single["preflight"]["laguerre_table_used"]
                tails &= lag_plan["cost"]["plan_tail_evaluations"] > 0
                if pp:
                    r = ps / pp
                    pp08_ratio_range = [min(pp08_ratio_range[0], r), max(pp08_ratio_range[1], r)]
        admits = [a["basis"] for a in recs[("pp08", "cold")]["attempts"] if a["admission"]["admitted"]]
        if len(admits) == 1:
            one_basis.append({"label": label, "admits": admits[0]})
    items_1_4 = all(gate[k] for k in ("1_certification", "2_parity", "3_accounting", "4_one_execution"))
    lo, hi = pp08_ratio_range
    predictions = {
        "items_1_to_4_hold": {"holds": items_1_4},
        "item_5_niche_empty": {"holds": len(niche(raw)) == 0, "niche_size": len(niche(raw))},
        "single_equals_cheb_products_on_grid": {"holds": grid_equal},
        "overhead_laguerre_tails_without_table": {"holds": table_free and tails},
        "single_over_pp08_in_0.25_to_0.50": {"holds": 0.25 <= lo and hi <= 0.50,
                                             "range": [lo, hi] if math.isfinite(lo) else None},
    }
    return {"vector_product_ratios": ratios, "plan_overhead": overhead,
            "admitted_by_one_basis_only_pp08": one_basis, "preflight_abstentions": abstentions,
            "predictions": predictions}


def evaluate(validated: evidence.Evidence) -> dict:
    raw, decoded = validated.docs["raw"], validated.docs["decoded"]
    items = {
        "1_certification": item_certification(raw, decoded),
        "2_parity": item_parity(raw),
        "3_accounting": item_accounting(raw),
        "4_one_execution": item_one_execution(raw),
        "5_niche_benefit": item_niche(raw),
    }
    gate = {k: v["pass"] for k, v in items.items()}
    verdict = "PASS" if all(gate.values()) else "FAIL"
    return {"verdict": verdict, "gate": gate, "items": items, "reported": reported(raw, gate)}


def main(argv: list[str] | None = None, blob_at=None, tree_status=None, head=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--cases", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args(argv)
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    inputs = {"cases": args.cases}
    try:
        validated = validate(args.cases, blob_at, tree_status)
    except evidence.ValidationError as exc:
        return evidence.emit_invalid(args.output, SCHEMA, exc, inputs)
    result = evaluate(validated)
    doc = validated.docs["cases"]
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
        "items": result["items"],
        "reported": result["reported"],
        "cases_source_commit": doc["source_commit"],
        "cases_tree_status": doc["tree_status"],
        "checker_head": (head or own_head)(),
        "checker_tree_status": (tree_status or own_tree_status)(),
        "inputs": {"cases": {"path": str(args.cases), "sha256": validated.sha256["cases"]}},
        "environment": {"python": sys.version.split()[0], "mpmath": mpmath_version, "dps": DPS,
                        "dps_check": DPS_CHECK},
        "claim_scope": "Counted work only (vector products, coefficient-series terms, certification operations); "
                       "no wall-time claim. The router is opt-in and research-only; route_joint_phi and every "
                       "default dispatch are unchanged; no basis enters any default.",
    }
    args.output.write_text(json.dumps(report, indent=1, sort_keys=True, allow_nan=False) + "\n")
    for k, v in result["gate"].items():
        print(f"{k}: {'PASS' if v else 'FAIL'}")
    print(f"verdict: {result['verdict']}")
    return 0 if result["verdict"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
