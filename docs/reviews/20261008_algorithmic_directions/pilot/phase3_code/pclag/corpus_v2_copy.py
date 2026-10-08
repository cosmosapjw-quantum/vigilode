"""Python transcription of VigilODE scientific-corpus-v2.1.

Source: /home/user/wt-speed/crates/rodas5p-integrators/src/scientific_corpus_v2.rs @ a49f7e4
(read-only).  The v2 corpus is self-contained: it does NOT reuse the legacy G4/S5B0 atlas builders
(g4_s5b0_regime_atlas.rs); its families differ from the atlas ones by the base-two radical-inverse
diversity multiplier and the 2-D semilinear grid.

EXPLORATORY research aid (probe A2), not ledger authority.  Importing this module has no side effects
(no file I/O, no computation beyond defining constants); stored references are read lazily.

What it exposes
---------------
* ``build(family, n=96, grid=None, allow_holdout=False) -> CorpusProblem``
* ``calibration_problems(n=96)`` -> the six calibration families at dimension n
* ``calibration_cases(n=96)`` -> the 18 (family, rtol) rows, each a dict with case_id/rtol/atol/problem
* error rule helpers: ``wrms_rows``, ``global_error_metrics``, ``case_units``, ``assess_error_budget``,
  ``reference_admissible`` (crates/rodas5p-fair-ab/src/global_error.rs:306-380, output_accuracy.rs:44-78,
  tools/reference_v2/generate_references_v2.py:1076-1082 anchor_wrms)
* ``load_stored_reference(problem)``: selected_raw artifacts for the n = 96 calibration families
  (research/scientific_validity_v2_20260829/external_reaudit_bundle/reference/selected_raw).
* ``radau_reference(problem, ...)``: SciPy Radau reference on the 101-point output grid.

CorpusProblem fields (attribute AND dict-style access, so it plugs into the hyp/ctrl replicas that use
``p['f'], p['J'], p['ft'], p['y0'], p['span'], p['ascale'], p['ref']``):
  f(t,y), jvp(t,y,v) (analytic, transcribed from the Rust jvp closures), J(t,y) dense ndarray,
  J_sparse(t,y) scipy.sparse.csr_matrix, ft(t,y) = partial f / partial t (analytic, transcribed from the
  Rust partial_t closures), exact(t) or None, y0 (fresh copy on each access), span, ascale = 0.01
  (atol = 0.01 * rtol, scientific_corpus_v2.rs:339), rtols = (1e-4, 1e-6, 1e-8), output_times
  (101 uniform points plus breakpoints), autonomous (False for all calibration families), segments
  (branch-fixed (t0, t1, f) pieces; one piece except the holdouts with a mandatory split).

Holdouts (oregonator, pollution, medical-akzo, brusselator-2d) are transcribed only behind
``allow_holdout=True``.  docs/HOLDOUT_HYGIENE.md: probes must not use holdouts for calibration and holdout
rows are read only by a committed verdict script.  Nothing in this module reads holdout references.
"""
from __future__ import annotations

import json
import math
import os
from typing import Callable, Optional

import numpy as np

VERSION = "scientific-corpus-v2.1"
SOURCE = "crates/rodas5p-integrators/src/scientific_corpus_v2.rs @ a49f7e4"
REPO = "/home/user/wt-speed"
CALIBRATION_DIMENSIONS = (96, 384, 1536)            # :15
CORPUS_RTOLS = (1.0e-4, 1.0e-6, 1.0e-8)             # :16
# Exploratory extension for regression-frontier ladders (OFF-CONTRACT: the Rust spec validator only admits
# CORPUS_RTOLS).  At rtol <= 1e-9 check reference admissibility: the stored n = 96 uncertainties are
# 2.6e-5..5.7e-5 tight units (semilinear 3.8e-3, use exact_reference() there), i.e. x(1e-8/rtol) case units.
EXTENDED_RTOLS = (1.0e-3, 1.0e-4, 1.0e-5, 1.0e-6, 1.0e-7, 1.0e-8, 1.0e-9, 1.0e-10)
UNIFORM_OUTPUT_POINTS = 101                         # :17
ATOL_FACTOR = 0.01                                  # :339 atol = 0.01 * rtol
SEMILINEAR_GRIDS = {96: (8, 12), 384: (16, 24), 1536: (32, 48)}   # :965-974
TIGHT_WRMS_ABS, TIGHT_WRMS_REL = 1.0e-10, 1.0e-8    # wrms-tight-radau-l2-anchor-v1
TWO_ARM_V3_BUDGET_CASE_UNITS = 10.0                 # addendum_20260929_two_arm_v3 (B = 10 case units)

CALIBRATION_FAMILIES = (
    "robertson-ramped",
    "hires-ramped",
    "van-der-pol-ramped",
    "rotating-nonnormal",
    "nonautonomous-stiff-forcing",
    "semilinear-advection-diffusion-ramped",
)
HOLDOUT_FAMILIES = ("oregonator", "pollution", "medical-akzo", "brusselator-2d")
BLOCK_WIDTH = {"robertson-ramped": 3, "hires-ramped": 8, "van-der-pol-ramped": 2,
               "rotating-nonnormal": 2, "nonautonomous-stiff-forcing": 1}           # :87-95
CALIBRATION_SPAN = {"robertson-ramped": (0.0, 0.10)}                                 # :307-317
HOLDOUT_DEF = {  # family: (dimension, span, breakpoints)                            # :197-208
    "oregonator": (3, (0.0, 360.0), ()),
    "pollution": (20, (0.0, 60.0), ()),
    "medical-akzo": (400, (0.0, 20.0), (5.0,)),
    "brusselator-2d": (512, (0.0, 11.5), (1.1,)),
}

# Campaign solver configuration of the canonical v2 runner (identical for all 54 calibration rows except
# the outer tolerances and initial_step = span/100, max_step = span).  Extracted from
# research/scientific_validity_v2_20260829/external_reaudit_bundle/rust/calibration_all_cases_compact.json.
CAMPAIGN_CONFIG = {
    "candidate_id": "sequential-rodas5p-gmres-wrms-forcing-v2",
    "code_revision": "ab8fbcdb709aa1e87603b1ef6f83c5e610c8cb04",
    "method": "RODAS5P", "linear_method": "GMRES", "inner_tolerance_policy": "wrms-stage-residual-heuristic-v2",
    "restart": 32, "max_arnoldi": 256, "preconditioner": "none", "initial_guess": "previous",
    "fallback_inner_atol": 1e-12, "fallback_inner_rtol": 1e-10,
    "initial_step": "span/100", "min_step": 1e-12, "max_step": "span", "max_attempts_per_arm": 200000,
    "controller": "integral", "controller_safety": 0.9, "controller_min_factor": 0.2,
    "controller_max_factor": 5.0, "controller_reject_max_factor": 0.9,
    "outer_atol": "0.01*rtol", "outer_rtol": "rtol",
}

# Approximate state-flop cost model of ONE analytic JVP (hand count of the Rust jvp closure arithmetic; per
# state component; excludes the O(1) ramp tanh and the integer radical-inverse recomputation).  Transcendentals
# (sin/cos) are listed separately.  JVP flops ~= flops_per_component * n.  An RHS call costs about the same plus
# the manufactured phi (rotating: 2 sin + 2 cos per component; semilinear: n multiplies + nx + ny sin).
JVP_COST_MODEL = {
    "robertson-ramped": {"flops_per_component": 25 / 3, "transcendentals_per_component": 0.0},   # 25 / 3-block
    "hires-ramped": {"flops_per_component": 48 / 8, "transcendentals_per_component": 0.0},       # 48 / 8-block
    "van-der-pol-ramped": {"flops_per_component": 11 / 2, "transcendentals_per_component": 0.0},  # 11 / 2-block
    "rotating-nonnormal": {"flops_per_component": 28 / 2, "transcendentals_per_component": 1.0},  # 28 + sin,cos / block
    "nonautonomous-stiff-forcing": {"flops_per_component": 8.0, "transcendentals_per_component": 1.0},
    "semilinear-advection-diffusion-ramped": {"flops_per_component": 22.0, "transcendentals_per_component": 0.0},
}

STORED_REFERENCE_DIR = (f"{REPO}/research/scientific_validity_v2_20260829/external_reaudit_bundle/"
                        "reference/selected_raw")
STORED_RUST_COMPACT = (f"{REPO}/research/scientific_validity_v2_20260829/external_reaudit_bundle/rust/"
                       "calibration_all_cases_compact.json")
STORED_RUST_RAW_N96_1E8 = (f"{REPO}/research/scientific_validity_v2_20260829/external_reaudit_bundle/rust/"
                           "selected_raw_n96_rtol1e-8.json")
ORACLE_CALIBRATION = f"{REPO}/fixtures/scientific_corpus_v2_1_calibration_oracle.json"
ORACLE_SEMILINEAR = f"{REPO}/fixtures/scientific_corpus_v2_1_semilinear_oracle.json"
GENERATED_REFERENCE_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "refs")


# ----------------------------------------------------------------------------------------------------
# shared scalar helpers (bit-faithful operation order where cheap)
# ----------------------------------------------------------------------------------------------------
def diversity_multiplier(index: int) -> float:
    """Prefix-stable scale in [0.9, 1.1): 0.9 + 0.2 * radical_inverse_2(index + 1)  (:293-305)."""
    value = index + 1
    fraction = 0.5
    radical_inverse = 0.0
    while value != 0:
        if value & 1 == 1:
            radical_inverse += fraction
        value >>= 1
        fraction *= 0.5
    return 0.9 + 0.2 * radical_inverse


_DIV_CACHE: dict = {}


def diversity_array(m: int) -> np.ndarray:
    """diversity_multiplier(0..m-1) as an array (cached, read-only)."""
    if m not in _DIV_CACHE:
        arr = np.array([diversity_multiplier(k) for k in range(m)], dtype=np.float64)
        arr.setflags(write=False)
        _DIV_CACHE[m] = arr
    return _DIV_CACHE[m]


def smooth_ramp(t: float, center: float, width: float):
    """(r, dr/dt) with r = (1 + tanh((t - c)/w))/2   (:515-519)."""
    th = math.tanh((t - center) / width)
    return 0.5 * (1.0 + th), 0.5 * (1.0 - th * th) / width


def output_times(span, breakpoints=()) -> np.ndarray:
    """101 uniform points plus mandatory breakpoints (:365-377)."""
    t0, tf = span
    times = [t0 + (tf - t0) * k / (UNIFORM_OUTPUT_POINTS - 1) for k in range(UNIFORM_OUTPUT_POINTS)]
    for b in breakpoints:
        if b not in times:
            times.append(b)
    return np.array(sorted(times), dtype=np.float64)


def problem_id(family: str, n: int, grid=None) -> str:
    if family == "semilinear-advection-diffusion-ramped":
        nx, ny = grid if grid is not None else SEMILINEAR_GRIDS[n]
        return f"{family}-n{n}-grid{nx}x{ny}-v2.1"
    if family in HOLDOUT_FAMILIES:
        return f"{family}-holdout-v2"
    return f"{family}-n{n}-v2"


def case_id(family: str, n: int, rtol: float, grid=None) -> str:
    """Rust scientific_case_id (:349-363); rtol formatted like Rust {:.0e} -> '1e-4'."""
    label = f"{rtol:.0e}".replace("e-0", "e-").replace("e+0", "e")
    if family == "semilinear-advection-diffusion-ramped":
        nx, ny = grid if grid is not None else SEMILINEAR_GRIDS[n]
        return f"{family}-n{n}-grid{nx}x{ny}-rtol-{label}-v2.1"
    return f"{family}-n{n}-rtol-{label}-v2.1"


def case_units(tight_wrms: float, rtol: float) -> float:
    """Convert a tight-basis (1e-10, 1e-8) WRMS into case-tolerance (0.01 rtol, rtol) units.
    The two weight vectors are proportional, so case = tight * 1e-8 / rtol (addendum 2026-09-28)."""
    return tight_wrms * TIGHT_WRMS_REL / rtol


# ----------------------------------------------------------------------------------------------------
# error rule (global_error.rs:306-380; anchor_wrms in generate_references_v2.py)
# ----------------------------------------------------------------------------------------------------
def wrms_rows(Y, REF, absw: float = TIGHT_WRMS_ABS, relw: float = TIGHT_WRMS_REL) -> np.ndarray:
    """Per-output-time WRMS: sqrt(mean(((Y - REF)/(absw + relw*|REF|))^2)), weights anchored on REF."""
    Y = np.atleast_2d(np.asarray(Y, dtype=np.float64))
    REF = np.atleast_2d(np.asarray(REF, dtype=np.float64))
    W = absw + relw * np.abs(REF)
    return np.sqrt(np.mean(((Y - REF) / W) ** 2, axis=1))


def global_error_metrics(Y, REF, rtol: Optional[float] = None, reference_uncertainty_wrms: float = 0.0) -> dict:
    """endpoint / max-grid / rms-grid WRMS in the tight basis (and case units when rtol is given),
    plus conservative_max = max_grid + reference uncertainty (global_error.rs:364-380)."""
    w = wrms_rows(Y, REF)
    out = {
        "endpoint_wrms": float(w[-1]), "max_grid_wrms": float(w.max()),
        "rms_grid_wrms": float(math.sqrt(np.mean(w ** 2))), "argmax_grid": int(np.argmax(w)),
        "reference_uncertainty_wrms": float(reference_uncertainty_wrms),
        "conservative_max_wrms": float(w.max() + reference_uncertainty_wrms),
    }
    if rtol is not None:
        for k in ("endpoint_wrms", "max_grid_wrms", "rms_grid_wrms", "conservative_max_wrms"):
            out[k.replace("_wrms", "_case")] = case_units(out[k], rtol)
    return out


def reference_admissible(reference_uncertainty_wrms: float, max_grid_wrms: float) -> bool:
    """A row is reference-invalid if uncertainty > 0.1 * measured max-grid WRMS (global_error.rs:379-398)."""
    return not (reference_uncertainty_wrms > 0.1 * max_grid_wrms)


def assess_error_budget(error: float, uncertainty: float, budget: Optional[float]) -> dict:
    """output_accuracy.rs:44-78 (all quantities in the same units)."""
    lo, hi = max(error - uncertainty, 0.0), error + uncertainty
    if budget is None:
        verdict = "budget-not-specified"
    elif hi <= budget:
        verdict = "within-budget"
    elif lo > budget:
        verdict = "outside-budget"
    else:
        verdict = "reference-unresolved"
    return {"lower": lo, "upper": hi, "budget": budget, "verdict": verdict}


# ----------------------------------------------------------------------------------------------------
# problem container
# ----------------------------------------------------------------------------------------------------
class CorpusProblem:
    _KEYS = ("f", "J", "ft", "jvp", "y0", "span", "ascale", "ref", "n", "name", "auto", "exact",
             "J_sparse", "family", "rtols")

    def __init__(self, *, family, n, f, jvp, J, J_sparse, ft, y0, span, exact=None, grid=None,
                 breakpoints=(), segments=None, partition="calibration", autonomous=False, on_contract=True,
                 notes=""):
        self.family = family
        self.partition = partition
        self.n = int(n)
        self.grid_shape = grid
        self.name = problem_id(family, n, grid)
        self.f: Callable = f
        self.jvp: Callable = jvp
        self.J: Callable = J
        self.J_sparse: Callable = J_sparse
        self.ft: Callable = ft
        self.exact = exact
        self._y0 = np.array(y0, dtype=np.float64)
        self._y0.setflags(write=False)
        self.span = (float(span[0]), float(span[1]))
        self.t_span = self.span
        self.mandatory_breakpoints = tuple(breakpoints)
        self.output_times = output_times(self.span, breakpoints)
        self.segments = segments if segments is not None else [(self.span[0], self.span[1], f)]
        self.autonomous = autonomous
        self.auto = autonomous
        self.ascale = ATOL_FACTOR
        self.rtols = CORPUS_RTOLS
        self.on_contract = on_contract
        self.block_width = BLOCK_WIDTH.get(family)
        self.notes = notes
        self._ref_cache = None

    # dict-style compatibility with the hyp/ctrl replicas
    def __getitem__(self, key):
        if key not in self._KEYS:
            raise KeyError(key)
        return getattr(self, key)

    def get(self, key, default=None):
        try:
            return self[key]
        except KeyError:
            return default

    def keys(self):
        return list(self._KEYS)

    def __contains__(self, key):
        return key in self._KEYS

    @property
    def y0(self) -> np.ndarray:
        return self._y0.copy()

    @property
    def ref(self) -> Optional[np.ndarray]:
        """Final state of the stored n = 96 reference (or a generated one under refs/), else None."""
        r = self.reference()
        return None if r is None else r["states"][-1].copy()

    def atol(self, rtol: float) -> float:
        return ATOL_FACTOR * rtol

    def case_id(self, rtol: float) -> str:
        return case_id(self.family, self.n, rtol, self.grid_shape)

    def campaign_settings(self, rtol: float) -> dict:
        h = (self.span[1] - self.span[0]) / (UNIFORM_OUTPUT_POINTS - 1)
        return {"rtol": rtol, "atol": self.atol(rtol), "initial_step": h, "max_step": self.span[1] - self.span[0],
                "min_step": 1e-12, "controller_safety": 0.9, "min_factor": 0.2, "max_factor": 5.0,
                "reject_max_factor": 0.9}

    def reference(self):
        """Stored selected_raw artifact (n = 96 calibration) or a generated refs/<name>.npz, else None."""
        if self._ref_cache is None:
            self._ref_cache = load_stored_reference(self) or load_generated_reference(self) or False
        return self._ref_cache or None

    def exact_reference(self):
        """Manufactured exact solution on the output grid (rotating-nonnormal, semilinear), uncertainty 0."""
        if self.exact is None:
            return None
        return {"source": "exact", "times": self.output_times.copy(),
                "states": np.array([self.exact(t) for t in self.output_times]), "uncertainty_wrms": 0.0}

    def error_metrics(self, Y, rtol: Optional[float] = None, prefer_exact: bool = False) -> dict:
        r = self.exact_reference() if (prefer_exact and self.exact is not None) else self.reference()
        if r is None:
            raise RuntimeError(f"no reference for {self.name}")
        return global_error_metrics(Y, r["states"], rtol, r["uncertainty_wrms"])

    def __repr__(self):
        return f"CorpusProblem({self.name}, n={self.n}, span={self.span}, partition={self.partition})"


# ----------------------------------------------------------------------------------------------------
# helpers for sparse/dense assembly
# ----------------------------------------------------------------------------------------------------
def _csr(n, rows, cols, vals):
    from scipy.sparse import csr_matrix
    return csr_matrix((np.asarray(vals, dtype=np.float64), (np.asarray(rows), np.asarray(cols))), shape=(n, n))


def _dense_from_sparse_fn(Js):
    return lambda t, y: Js(t, y).toarray()


# ----------------------------------------------------------------------------------------------------
# robertson-ramped  (:527-604)
# ----------------------------------------------------------------------------------------------------
def robertson_v2(n: int) -> CorpusProblem:
    blocks = n // 3
    s = diversity_array(max(blocks, 1))[:blocks]
    pad = slice(3 * blocks, n)

    def act(t):
        r, dr = smooth_ramp(t, 0.045, 0.010)
        return 0.05 + 0.95 * r, 0.95 * dr

    def f(t, y):
        a, _ = act(t)
        k1 = 0.04 * s; k2 = 1.0e4 * a * s; k3 = 3.0e7 * a * s
        y1, y2, y3 = y[0:3 * blocks:3], y[1:3 * blocks:3], y[2:3 * blocks:3]
        out = np.empty(n)
        out[0:3 * blocks:3] = -k1 * y1 + k2 * y2 * y3
        out[1:3 * blocks:3] = k1 * y1 - k2 * y2 * y3 - k3 * y2 * y2
        out[2:3 * blocks:3] = k3 * y2 * y2
        out[pad] = -(20.0 * a) * y[pad]
        return out

    def jvp(t, y, v):
        a, _ = act(t)
        k1 = 0.04 * s; k2 = 1.0e4 * a * s; k3 = 3.0e7 * a * s
        y2, y3 = y[1:3 * blocks:3], y[2:3 * blocks:3]
        v1, v2, v3 = v[0:3 * blocks:3], v[1:3 * blocks:3], v[2:3 * blocks:3]
        out = np.empty(n)
        out[0:3 * blocks:3] = -k1 * v1 + k2 * y3 * v2 + k2 * y2 * v3
        out[1:3 * blocks:3] = k1 * v1 + (-k2 * y3 - 2.0 * k3 * y2) * v2 - k2 * y2 * v3
        out[2:3 * blocks:3] = 2.0 * k3 * y2 * v2
        out[pad] = -20.0 * a * v[pad]
        return out

    def Js(t, y):
        a, _ = act(t)
        k1 = 0.04 * s; k2 = 1.0e4 * a * s; k3 = 3.0e7 * a * s
        i = 3 * np.arange(blocks)
        y2, y3 = y[i + 1], y[i + 2]
        rows = [i, i, i, i + 1, i + 1, i + 1, i + 2]
        cols = [i, i + 1, i + 2, i, i + 1, i + 2, i + 1]
        vals = [-k1, k2 * y3, k2 * y2, k1, -k2 * y3 - 2.0 * k3 * y2, -k2 * y2, 2.0 * k3 * y2]
        p = np.arange(3 * blocks, n)
        rows.append(p); cols.append(p); vals.append(np.full(len(p), -20.0 * a))
        return _csr(n, np.concatenate(rows), np.concatenate(cols), np.concatenate(vals))

    def ft(t, y):
        _, da = act(t)
        dk2 = 1.0e4 * da * s; dk3 = 3.0e7 * da * s
        y2, y3 = y[1:3 * blocks:3], y[2:3 * blocks:3]
        out = np.zeros(n)
        out[0:3 * blocks:3] = dk2 * y2 * y3
        out[1:3 * blocks:3] = -dk2 * y2 * y3 - dk3 * y2 * y2
        out[2:3 * blocks:3] = dk3 * y2 * y2
        out[pad] = -20.0 * da * y[pad]
        return out

    y0 = np.zeros(n); y0[0:3 * blocks:3] = 1.0
    return CorpusProblem(family="robertson-ramped", n=n, f=f, jvp=jvp, J=_dense_from_sparse_fn(Js), J_sparse=Js,
                         ft=ft, y0=y0, span=(0.0, 0.10), on_contract=n in CALIBRATION_DIMENSIONS,
                         notes="n/3 Robertson blocks k=(0.04, 1e4 act, 3e7 act)*div(block); act=0.05+0.95 ramp(.045,.010)")


# ----------------------------------------------------------------------------------------------------
# hires-ramped  (:606-692)
# ----------------------------------------------------------------------------------------------------
_HIRES_LIN = np.array([
    [-1.71, 0.43, 8.32, 0, 0, 0, 0, 0],
    [1.71, -8.75, 0, 0, 0, 0, 0, 0],
    [0, 0, -10.03, 0.43, 0.035, 0, 0, 0],
    [0, 8.32, 1.71, -1.12, 0, 0, 0, 0],
    [0, 0, 0, 0, -1.745, 0.43, 0.43, 0],
    [0, 0, 0, 0.69, 1.71, -0.43, 0.69, 0],
    [0, 0, 0, 0, 0, 0, -1.81, 0],
    [0, 0, 0, 0, 0, 0, 1.81, 0],
])


def hires_v2(n: int) -> CorpusProblem:
    blocks = n // 8
    s = diversity_array(max(blocks, 1))[:blocks]
    m = 8 * blocks
    pad = slice(m, n)

    def act(t):
        r, dr = smooth_ramp(t, 0.45, 0.08)
        return 0.1 + 0.9 * r, 0.9 * dr

    def f(t, y):
        a, _ = act(t)
        Y = y[:m].reshape(blocks, 8)
        y1, y2, y3, y4, y5, y6, y7, y8 = (Y[:, k] for k in range(8))
        q = 280.0 * a * y6 * y8
        O = np.empty((blocks, 8))
        O[:, 0] = s * (-1.71 * y1 + 0.43 * y2 + 8.32 * y3 + 0.0007)
        O[:, 1] = s * (1.71 * y1 - 8.75 * y2)
        O[:, 2] = s * (-10.03 * y3 + 0.43 * y4 + 0.035 * y5)
        O[:, 3] = s * (8.32 * y2 + 1.71 * y3 - 1.12 * y4)
        O[:, 4] = s * (-1.745 * y5 + 0.43 * y6 + 0.43 * y7)
        O[:, 5] = s * (-q + 0.69 * y4 + 1.71 * y5 - 0.43 * y6 + 0.69 * y7)
        O[:, 6] = s * (q - 1.81 * y7)
        O[:, 7] = s * (-q + 1.81 * y7)
        out = np.empty(n)
        out[:m] = O.ravel()
        out[pad] = -(2.0 + 20.0 * a) * y[pad]
        return out

    def jvp(t, y, v):
        a, _ = act(t)
        Y = y[:m].reshape(blocks, 8); V = v[:m].reshape(blocks, 8)
        qv = 280.0 * a * (Y[:, 7] * V[:, 5] + Y[:, 5] * V[:, 7])
        O = np.empty((blocks, 8))
        O[:, 0] = s * (-1.71 * V[:, 0] + 0.43 * V[:, 1] + 8.32 * V[:, 2])
        O[:, 1] = s * (1.71 * V[:, 0] - 8.75 * V[:, 1])
        O[:, 2] = s * (-10.03 * V[:, 2] + 0.43 * V[:, 3] + 0.035 * V[:, 4])
        O[:, 3] = s * (8.32 * V[:, 1] + 1.71 * V[:, 2] - 1.12 * V[:, 3])
        O[:, 4] = s * (-1.745 * V[:, 4] + 0.43 * V[:, 5] + 0.43 * V[:, 6])
        O[:, 5] = s * (-qv + 0.69 * V[:, 3] + 1.71 * V[:, 4] - 0.43 * V[:, 5] + 0.69 * V[:, 6])
        O[:, 6] = s * (qv - 1.81 * V[:, 6])
        O[:, 7] = s * (-qv + 1.81 * V[:, 6])
        out = np.empty(n)
        out[:m] = O.ravel()
        out[pad] = -(2.0 + 20.0 * a) * v[pad]
        return out

    lin_r, lin_c = np.nonzero(_HIRES_LIN)
    lin_v = _HIRES_LIN[lin_r, lin_c]

    def Js(t, y):
        a, _ = act(t)
        rows, cols, vals = [], [], []
        base = 8 * np.arange(blocks)
        for r_, c_, v_ in zip(lin_r, lin_c, lin_v):
            rows.append(base + r_); cols.append(base + c_); vals.append(s * v_)
        y6, y8 = y[base + 5], y[base + 7]
        q6 = 280.0 * a * y8; q8 = 280.0 * a * y6
        for r_, c_, sign, q in ((5, 5, -1, q6), (5, 7, -1, q8), (6, 5, 1, q6), (6, 7, 1, q8), (7, 5, -1, q6), (7, 7, -1, q8)):
            rows.append(base + r_); cols.append(base + c_); vals.append(s * sign * q)
        p = np.arange(m, n)
        rows.append(p); cols.append(p); vals.append(np.full(len(p), -(2.0 + 20.0 * a)))
        return _csr(n, np.concatenate(rows), np.concatenate(cols), np.concatenate(vals))  # duplicates summed

    def ft(t, y):
        _, da = act(t)
        Y = y[:m].reshape(blocks, 8)
        dq = 280.0 * da * Y[:, 5] * Y[:, 7]
        O = np.zeros((blocks, 8))
        O[:, 5] = -s * dq
        O[:, 6] = s * dq
        O[:, 7] = -s * dq
        out = np.zeros(n)
        out[:m] = O.ravel()
        out[pad] = -20.0 * da * y[pad]
        return out

    y0 = np.zeros(n); y0[0:m:8] = 1.0; y0[7:m:8] = 0.0057
    return CorpusProblem(family="hires-ramped", n=n, f=f, jvp=jvp, J=_dense_from_sparse_fn(Js), J_sparse=Js, ft=ft,
                         y0=y0, span=(0.0, 1.0), on_contract=n in CALIBRATION_DIMENSIONS,
                         notes="n/8 HIRES blocks scaled by div(block); q=280 act y6 y8; act=0.1+0.9 ramp(.45,.08)")


# ----------------------------------------------------------------------------------------------------
# van-der-pol-ramped  (:694-754)
# ----------------------------------------------------------------------------------------------------
def van_der_pol_v2(n: int) -> CorpusProblem:
    blocks = n // 2
    s = diversity_array(max(blocks, 1))[:blocks]
    m = 2 * blocks
    pad = slice(m, n)

    def mu_of(t):
        r, dr = smooth_ramp(t, 0.50, 0.08)
        return 10.0 + 490.0 * r, 490.0 * dr

    def f(t, y):
        mu, _ = mu_of(t)
        lm = mu * s
        y1, y2 = y[0:m:2], y[1:m:2]
        out = np.empty(n)
        out[0:m:2] = y2
        out[1:m:2] = lm * (1.0 - y1 * y1) * y2 - y1
        out[pad] = -(5.0 + mu) * y[pad]
        return out

    def jvp(t, y, v):
        mu, _ = mu_of(t)
        lm = mu * s
        y1, y2 = y[0:m:2], y[1:m:2]
        out = np.empty(n)
        out[0:m:2] = v[1:m:2]
        out[1:m:2] = (-2.0 * lm * y1 * y2 - 1.0) * v[0:m:2] + lm * (1.0 - y1 * y1) * v[1:m:2]
        out[pad] = -(5.0 + mu) * v[pad]
        return out

    def Js(t, y):
        mu, _ = mu_of(t)
        lm = mu * s
        i = 2 * np.arange(blocks)
        y1, y2 = y[i], y[i + 1]
        p = np.arange(m, n)
        rows = np.concatenate([i, i + 1, i + 1, p])
        cols = np.concatenate([i + 1, i, i + 1, p])
        vals = np.concatenate([np.ones(blocks), -2.0 * lm * y1 * y2 - 1.0, lm * (1.0 - y1 * y1),
                               np.full(len(p), -(5.0 + mu))])
        return _csr(n, rows, cols, vals)

    def ft(t, y):
        _, dmu = mu_of(t)
        ldmu = dmu * s
        y1, y2 = y[0:m:2], y[1:m:2]
        out = np.zeros(n)
        out[1:m:2] = ldmu * (1.0 - y1 * y1) * y2
        out[pad] = -dmu * y[pad]
        return out

    y0 = np.zeros(n); y0[0:m:2] = 2.0
    return CorpusProblem(family="van-der-pol-ramped", n=n, f=f, jvp=jvp, J=_dense_from_sparse_fn(Js), J_sparse=Js,
                         ft=ft, y0=y0, span=(0.0, 1.0), on_contract=n in CALIBRATION_DIMENSIONS,
                         notes="vdP blocks mu=(10+490 ramp(.5,.08))*div(block); y0=(2,0)")


# ----------------------------------------------------------------------------------------------------
# rotating-nonnormal  (:756-893)
# ----------------------------------------------------------------------------------------------------
def rotating_nonnormal_v2(n: int) -> CorpusProblem:
    blocks = n // 2
    m = 2 * blocks
    sb = diversity_array(max(blocks, 1))[:blocks]                      # per-block scale
    idx = np.arange(n)
    freq = (1.0 + (idx % 7).astype(np.float64)) * diversity_array(n // 2 + 1)[idx // 2]
    pad_idx = np.arange(m, n)
    s_pad = diversity_array(n)[pad_idx] if len(pad_idx) else np.zeros(0)   # NB Rust uses div(i), not div(block)

    def shape(t, d):
        if d == 0:
            return 0.4 * np.sin(freq * t) + 0.2 * np.cos(0.5 * freq * t)
        if d == 1:
            return 0.4 * freq * np.cos(freq * t) - 0.1 * freq * np.sin(0.5 * freq * t)
        return -0.4 * freq * freq * np.sin(freq * t) - 0.05 * freq * freq * np.cos(0.5 * freq * t)

    def params(t):
        r, dr = smooth_ramp(t, 0.50, 0.08)
        bs = 20.0 + 480.0 * r
        eta = 0.1 + 0.8 * r
        bth = 8.0 * t + 0.4 * math.sin(4.0 * t)
        return r, dr, bs, eta, bth

    def apply_op(t, x):
        _, _, bs, eta, bth = params(t)
        st = bs * sb; th = bth * sb
        c, sn = np.cos(th), np.sin(th)
        x0, x1 = x[0:m:2], x[1:m:2]
        xr0 = c * x0 + sn * x1
        xr1 = -sn * x0 + c * x1
        ar0 = -st * xr0 + eta * st * xr1
        ar1 = -0.35 * st * xr1
        out = np.empty(n)
        out[0:m:2] = c * ar0 - sn * ar1
        out[1:m:2] = sn * ar0 + c * ar1
        if len(pad_idx):
            out[m:] = -bs * s_pad * x[m:]
        return out

    def apply_op_t(t, x):
        _, dr, bs, eta, bth = params(t)
        dbs = 480.0 * dr; deta = 0.8 * dr; dbth = 8.0 + 1.6 * math.cos(4.0 * t)
        st = bs * sb; dst = dbs * sb; th = bth * sb; dth = dbth * sb
        c, sn = np.cos(th), np.sin(th)
        x0, x1 = x[0:m:2], x[1:m:2]
        xr0 = c * x0 + sn * x1
        xr1 = -sn * x0 + c * x1
        dxr0 = dth * xr1
        dxr1 = -dth * xr0
        ar0 = -st * xr0 + eta * st * xr1
        ar1 = -0.35 * st * xr1
        dar0 = -dst * xr0 - st * dxr0 + deta * st * xr1 + eta * dst * xr1 + eta * st * dxr1
        dar1 = -0.35 * (dst * xr1 + st * dxr1)
        out = np.empty(n)
        out[0:m:2] = c * dar0 - sn * dar1 - dth * (sn * ar0 + c * ar1)
        out[1:m:2] = sn * dar0 + c * dar1 + dth * (c * ar0 - sn * ar1)
        if len(pad_idx):
            out[m:] = -dbs * s_pad * x[m:]
        return out

    def nl_of(t):
        r, dr = smooth_ramp(t, 0.60, 0.06)
        return 40.0 * r, 40.0 * dr

    def f(t, y):
        phi = shape(t, 0); dphi = shape(t, 1)
        out = apply_op(t, y - phi)
        nl, _ = nl_of(t)
        return out + (dphi + nl * (y * y - phi * phi))

    def jvp(t, y, v):
        nl, _ = nl_of(t)
        return apply_op(t, v) + 2.0 * nl * y * v

    def block_matrices(t):
        """2x2 blocks B_k = R_k^T A_k R_k (columns are apply_op of unit vectors)."""
        _, _, bs, eta, bth = params(t)
        st = bs * sb; th = bth * sb
        c, sn = np.cos(th), np.sin(th)
        Bm = np.empty((blocks, 2, 2))
        for col, (e0, e1) in enumerate(((1.0, 0.0), (0.0, 1.0))):
            xr0 = c * e0 + sn * e1
            xr1 = -sn * e0 + c * e1
            ar0 = -st * xr0 + eta * st * xr1
            ar1 = -0.35 * st * xr1
            Bm[:, 0, col] = c * ar0 - sn * ar1
            Bm[:, 1, col] = sn * ar0 + c * ar1
        return Bm, bs

    def Js(t, y):
        Bm, bs = block_matrices(t)
        nl, _ = nl_of(t)
        i = 2 * np.arange(blocks)
        rows = np.concatenate([i, i, i + 1, i + 1, pad_idx, idx])
        cols = np.concatenate([i, i + 1, i, i + 1, pad_idx, idx])
        vals = np.concatenate([Bm[:, 0, 0], Bm[:, 0, 1], Bm[:, 1, 0], Bm[:, 1, 1], -bs * s_pad, 2.0 * nl * y])
        return _csr(n, rows, cols, vals)

    def ft(t, y):
        phi = shape(t, 0); dphi = shape(t, 1); ddphi = shape(t, 2)
        out = apply_op_t(t, y - phi)
        op_phi_t = apply_op(t, dphi)
        nl, dnl = nl_of(t)
        return out + (-op_phi_t + ddphi + dnl * (y * y - phi * phi) - 2.0 * nl * phi * dphi)

    exact = lambda t: shape(t, 0)
    return CorpusProblem(family="rotating-nonnormal", n=n, f=f, jvp=jvp, J=_dense_from_sparse_fn(Js), J_sparse=Js,
                         ft=ft, y0=shape(0.0, 0), span=(0.0, 1.0), exact=exact,
                         on_contract=n in CALIBRATION_DIMENSIONS,
                         notes="y'=R^T A R (y-phi)+phi'+40 ramp(.6,.06)(y^2-phi^2); A=[[-s,eta s],[0,-.35 s]]; exact phi")


# ----------------------------------------------------------------------------------------------------
# nonautonomous-stiff-forcing  (:895-963)
# ----------------------------------------------------------------------------------------------------
def nonautonomous_forcing_v2(n: int) -> CorpusProblem:
    s = diversity_array(n)
    phase = (np.arange(n) % 11).astype(np.float64) * 0.17

    def params(t):
        r, dr = smooth_ramp(t, 0.45, 0.07)
        return r, dr, 30.0 + 470.0 * r, 2.0 + 28.0 * r

    def f(t, y):
        r, _, st, fr = params(t)
        arg = fr * t + phase
        phi = np.sin(arg)
        forcing = s * fr * np.cos(arg)
        d = y - phi
        return -s * st * d + forcing + 20.0 * r * d * d

    def jvp(t, y, v):
        r, _, st, fr = params(t)
        phi = np.sin(fr * t + phase)
        d = y - phi
        return (-s * st + 40.0 * r * d) * v

    def diag(t, y):
        r, _, st, fr = params(t)
        phi = np.sin(fr * t + phase)
        return -s * st + 40.0 * r * (y - phi)

    def Js(t, y):
        i = np.arange(n)
        return _csr(n, i, i, diag(t, y))

    def ft(t, y):
        r, dr, st, fr = params(t)
        dst = 470.0 * dr; dfr = 28.0 * dr
        arg = fr * t + phase
        phi = np.sin(arg)
        darg = fr + t * dfr
        d = y - phi
        dd = -darg * np.cos(arg)
        dforcing = s * (dfr * np.cos(arg) - fr * np.sin(arg) * darg)
        return -s * dst * d - s * st * dd + dforcing + 20.0 * dr * d * d + 40.0 * r * d * dd

    y0 = np.sin(phase)
    return CorpusProblem(family="nonautonomous-stiff-forcing", n=n, f=f, jvp=jvp, J=lambda t, y: np.diag(diag(t, y)),
                         J_sparse=Js, ft=ft, y0=y0, span=(0.0, 1.0), on_contract=n in CALIBRATION_DIMENSIONS,
                         notes="diagonal; y_i'=-div(i) s d + div(i) W cos(Wt+p_i) + 20 ramp d^2; no exact solution")


# ----------------------------------------------------------------------------------------------------
# semilinear-advection-diffusion-ramped (2-D)  (:965-1082)
# ----------------------------------------------------------------------------------------------------
def semilinear_operator_matrices(nx: int, ny: int):
    """Sparse (Lap, Up) with J = D*Lap - a(t)*Up - I + diag(2 nl y); x-fast index p = i + nx*j."""
    from scipy.sparse import csr_matrix
    hx = 1.0 / (nx + 1); hy = 1.0 / (ny + 1)
    n = nx * ny
    rows, cols, lv, uv = [], [], [], []
    for j in range(ny):
        for i in range(nx):
            k = i + nx * j
            rows.append(k); cols.append(k); lv.append(-2.0 / (hx * hx) - 2.0 / (hy * hy)); uv.append(1.0 / hx + 1.0 / hy)
            if i > 0:
                rows.append(k); cols.append(k - 1); lv.append(1.0 / (hx * hx)); uv.append(-1.0 / hx)
            if i + 1 < nx:
                rows.append(k); cols.append(k + 1); lv.append(1.0 / (hx * hx)); uv.append(0.0)
            if j > 0:
                rows.append(k); cols.append(k - nx); lv.append(1.0 / (hy * hy)); uv.append(-1.0 / hy)
            if j + 1 < ny:
                rows.append(k); cols.append(k + nx); lv.append(1.0 / (hy * hy)); uv.append(0.0)
    Lap = csr_matrix((lv, (rows, cols)), shape=(n, n))
    Up = csr_matrix((uv, (rows, cols)), shape=(n, n))
    return Lap, Up


def semilinear_advection_diffusion_v2(n: int = 96, grid=None) -> CorpusProblem:
    from scipy.sparse import identity, diags
    on_contract = grid is None and n in SEMILINEAR_GRIDS
    if grid is None:
        if n not in SEMILINEAR_GRIDS:
            raise ValueError(f"no scientific-corpus-v2.1 semilinear grid for dimension {n}; pass grid=(nx, ny)")
        grid = SEMILINEAR_GRIDS[n]
    nx, ny = grid
    if nx * ny != n:
        raise ValueError("grid does not match n")
    D = 0.002
    hx = 1.0 / (nx + 1); hy = 1.0 / (ny + 1)
    # exact state, bit-faithful order: (exp(-t) * sin(pi x)) * sin(pi y), libm sin via math
    sx = np.array([math.sin(math.pi * ((i + 1) * hx)) for i in range(nx)])
    sy = np.array([math.sin(math.pi * ((j + 1) * hy)) for j in range(ny)])

    def exact(t):
        decay = math.exp(-t)
        return ((decay * sx)[None, :] * sy[:, None]).ravel()

    def apply_ad(t, x):
        r, _ = smooth_ramp(t, 0.50, 0.08)
        adv = 0.5 + 3.5 * r
        X = x.reshape(ny, nx)
        left = np.zeros_like(X); left[:, 1:] = X[:, :-1]
        right = np.zeros_like(X); right[:, :-1] = X[:, 1:]
        down = np.zeros_like(X); down[1:, :] = X[:-1, :]
        up = np.zeros_like(X); up[:-1, :] = X[1:, :]
        lap = (left - 2.0 * X + right) / (hx * hx) + (down - 2.0 * X + up) / (hy * hy)
        bu = (X - left) / hx + (X - down) / hy
        return (D * lap - adv * bu - X).ravel()

    def backward_upwind(x):
        X = x.reshape(ny, nx)
        left = np.zeros_like(X); left[:, 1:] = X[:, :-1]
        down = np.zeros_like(X); down[1:, :] = X[:-1, :]
        return ((X - left) / hx + (X - down) / hy).ravel()

    def nl_of(t):
        r, dr = smooth_ramp(t, 0.50, 0.08)
        return 2.0 + 48.0 * r, 48.0 * dr

    def f(t, y):
        phi = exact(t)
        out = apply_ad(t, y - phi)
        nl, _ = nl_of(t)
        return out + (-phi + nl * (y * y - phi * phi))

    def jvp(t, y, v):
        nl, _ = nl_of(t)
        return apply_ad(t, v) + 2.0 * nl * y * v

    Lap, Up = semilinear_operator_matrices(nx, ny)
    I = identity(n, format="csr")

    def Js(t, y):
        r, _ = smooth_ramp(t, 0.50, 0.08)
        adv = 0.5 + 3.5 * r
        nl, _ = nl_of(t)
        return (D * Lap - adv * Up - I + diags(2.0 * nl * y)).tocsr()

    def ft(t, y):
        phi = exact(t)
        defect = y - phi
        op_phi = apply_ad(t, phi)
        r, dr = smooth_ramp(t, 0.50, 0.08)
        dadv = 3.5 * dr
        op_t = -dadv * backward_upwind(defect)
        nl, dnl = nl_of(t)
        return op_t + op_phi + phi + dnl * (y * y - phi * phi) + 2.0 * nl * phi * phi

    return CorpusProblem(family="semilinear-advection-diffusion-ramped", n=n, f=f, jvp=jvp,
                         J=_dense_from_sparse_fn(Js), J_sparse=Js, ft=ft, y0=exact(0.0), span=(0.0, 1.0),
                         exact=exact, grid=(nx, ny), on_contract=on_contract,
                         notes="2-D x-fast grid, zero Dirichlet, D=0.002 five-point, backward upwind a=0.5+3.5 ramp(.5,.08), "
                               "reaction -1, nl=2+48 ramp; exact phi=exp(-t) sin(pi x) sin(pi y)  (F-033 family)")


# ----------------------------------------------------------------------------------------------------
# holdouts (guarded; definitions only)  (:1084-1426)
# ----------------------------------------------------------------------------------------------------
def _holdout_oregonator():
    S_, Q_, W_ = 77.27, 8.375e-6, 0.161

    def f(t, y):
        return np.array([S_ * (y[1] + y[0] * (1.0 - Q_ * y[0] - y[1])),
                         (y[2] - (1.0 + y[0]) * y[1]) / S_, W_ * (y[0] - y[2])])

    def jvp(t, y, v):
        return np.array([S_ * ((1.0 - 2.0 * Q_ * y[0] - y[1]) * v[0] + (1.0 - y[0]) * v[1]),
                         (-y[1] * v[0] - (1.0 + y[0]) * v[1] + v[2]) / S_, W_ * (v[0] - v[2])])
    return f, jvp, np.array([1.0, 2.0, 3.0]), None


_POLLUTION_K = np.array([0.35, 26.6, 12_300.0, 8.6e-4, 8.2e-4, 15_000.0, 1.3e-4, 24_000.0, 16_500.0, 9_000.0, 0.022,
                         12_000.0, 1.88, 16_300.0, 4.8e6, 3.5e-4, 0.0175, 1.0e8, 4.44e11, 1_240.0, 2.1, 5.78, 0.0474,
                         1_780.0, 3.12])


def _holdout_pollution():
    k = _POLLUTION_K
    pairs = [(0, None), (1, 3), (4, 1), (6, None), (6, None), (6, 5), (8, None), (8, 5), (10, 1), (10, 0), (12, None),
             (9, 1), (13, None), (0, 5), (2, None), (3, None), (3, None), (15, None), (15, None), (16, 5), (18, None),
             (18, None), (0, 3), (18, 0), (19, None)]

    def rates(y):
        return np.array([k[i] * y[a] * (1.0 if b is None else y[b]) for i, (a, b) in enumerate(pairs)])

    def drates(y, v):
        return np.array([k[i] * (v[a] if b is None else v[a] * y[b] + y[a] * v[b]) for i, (a, b) in enumerate(pairs)])

    def assemble(r):
        o = np.empty(20)
        o[0] = -r[0] - r[9] - r[13] - r[22] - r[23] + r[1] + r[2] + r[8] + r[10] + r[11] + r[21] + r[24]
        o[1] = -r[1] - r[2] - r[8] - r[11] + r[0] + r[20]
        o[2] = -r[14] + r[0] + r[16] + r[18] + r[21]
        o[3] = -r[1] - r[15] - r[16] - r[22] + r[14]
        o[4] = -r[2] + 2.0 * r[3] + r[5] + r[6] + r[12] + r[19]
        o[5] = -r[5] - r[7] - r[13] - r[19] + r[2] + 2.0 * r[17]
        o[6] = -r[3] - r[4] - r[5] + r[12]
        o[7] = r[3] + r[4] + r[5] + r[6]
        o[8] = -r[6] - r[7]
        o[9] = -r[11] + r[6] + r[8]
        o[10] = -r[8] - r[9] + r[7] + r[10]
        o[11] = r[8]
        o[12] = -r[10] + r[9]
        o[13] = -r[12] + r[11]
        o[14] = r[13]
        o[15] = -r[17] - r[18] + r[15]
        o[16] = -r[19]
        o[17] = r[19]
        o[18] = -r[20] - r[21] - r[23] + r[22] + r[24]
        o[19] = -r[24] + r[23]
        return o

    y0 = np.array([0.0, 0.2, 0.0, 0.04, 0.0, 0.0, 0.1, 0.3, 0.01, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.007, 0.0, 0.0, 0.0])
    return (lambda t, y: assemble(rates(y))), (lambda t, y, v: assemble(drates(y, v))), y0, None


def _holdout_medical_akzo():
    N, H, K, C_ = 200, 0.005, 100.0, 4.0
    zeta = (np.arange(N) + 1.0) * H
    a = 2.0 * (zeta - 1.0) ** 3 / (C_ * C_)
    b = (zeta - 1.0) ** 4 / (C_ * C_)

    def f_phi(phi_fixed):
        def f(t, y):
            phi = phi_fixed if phi_fixed is not None else (2.0 if t <= 5.0 else 0.0)
            u, v = y[0::2], y[1::2]
            um = np.r_[phi, u[:-1]]; up = np.r_[u[1:], u[-1]]
            reac = K * u * v
            out = np.empty(2 * N)
            out[0::2] = a * (up - um) / (2.0 * H) + b * (um - 2.0 * u + up) / (H * H) - reac
            out[1::2] = -reac
            return out
        return f

    def jvp(t, y, d):
        pu, pv = d[0::2], d[1::2]
        pm = np.r_[0.0, pu[:-1]]; pp = np.r_[pu[1:], pu[-1]]
        reac = K * (y[1::2] * pu + y[0::2] * pv)
        out = np.empty(2 * N)
        out[0::2] = a * (pp - pm) / (2.0 * H) + b * (pm - 2.0 * pu + pp) / (H * H) - reac
        out[1::2] = -reac
        return out

    y0 = np.zeros(2 * N); y0[1::2] = 1.0
    segs = [(0.0, 5.0, f_phi(2.0)), (5.0, 20.0, f_phi(0.0))]
    return f_phi(None), jvp, y0, segs


def _holdout_brusselator_2d():
    SIDE, A_, B_, ALPHA, H = 16, 3.4, 1.0, 10.0, 1.0 / 15.0
    PLANE = SIDE * SIDE
    xs = np.arange(SIDE) * H
    X, Yy = np.meshgrid(xs, xs)            # X[j, i] = i H, Yy[j, i] = j H ; offset i + 16 j
    mask = (((X - 0.3) ** 2 + (Yy - 0.6) ** 2) <= 0.01).ravel()
    diff = ALPHA / (H * H)

    def lap(Z):
        Z = Z.reshape(SIDE, SIDE)
        return (np.roll(Z, 1, 1) + np.roll(Z, -1, 1) + np.roll(Z, 1, 0) + np.roll(Z, -1, 0) - 4.0 * Z).ravel()

    def f_forc(fixed):
        def f(t, y):
            u, v = y[:PLANE], y[PLANE:]
            on = fixed if fixed is not None else (t >= 1.1)
            forcing = np.where(mask, 5.0, 0.0) if on else 0.0
            uv2 = u * u * v
            return np.concatenate([diff * lap(u) + B_ + uv2 - (A_ + 1.0) * u + forcing, diff * lap(v) + A_ * u - uv2])
        return f

    def jvp(t, y, d):
        u, v = y[:PLANE], y[PLANE:]
        pu, pv = d[:PLANE], d[PLANE:]
        return np.concatenate([diff * lap(pu) + (2.0 * u * v - A_ - 1.0) * pu + u * u * pv,
                               diff * lap(pv) + (A_ - 2.0 * u * v) * pu - u * u * pv])

    y0 = np.concatenate([(22.0 * (Yy * (1.0 - Yy)) ** 1.5).ravel(), (27.0 * (X * (1.0 - X)) ** 1.5).ravel()])
    segs = [(0.0, 1.1, f_forc(False)), (1.1, 11.5, f_forc(True))]
    return f_forc(None), jvp, y0, segs


def _holdout(family: str) -> CorpusProblem:
    n, span, bps = HOLDOUT_DEF[family]
    f, jvp, y0, segs = {"oregonator": _holdout_oregonator, "pollution": _holdout_pollution,
                        "medical-akzo": _holdout_medical_akzo, "brusselator-2d": _holdout_brusselator_2d}[family]()

    def J(t, y):
        E = np.eye(n)
        return np.column_stack([jvp(t, y, E[:, k]) for k in range(n)])

    def Js(t, y):
        from scipy.sparse import csr_matrix
        return csr_matrix(J(t, y))

    zero = lambda t, y: np.zeros(n)
    return CorpusProblem(family=family, n=n, f=f, jvp=jvp, J=J, J_sparse=Js, ft=zero, y0=y0, span=span,
                         breakpoints=bps, segments=segs, partition="holdout",
                         autonomous=family in ("oregonator", "pollution"),
                         notes="HOLDOUT: definition only; do not calibrate on it (docs/HOLDOUT_HYGIENE.md). "
                               "ft = 0 on each branch-fixed segment, as in the Rust partial_t.")


# ----------------------------------------------------------------------------------------------------
# public builders
# ----------------------------------------------------------------------------------------------------
_BUILDERS = {
    "robertson-ramped": robertson_v2,
    "hires-ramped": hires_v2,
    "van-der-pol-ramped": van_der_pol_v2,
    "rotating-nonnormal": rotating_nonnormal_v2,
    "nonautonomous-stiff-forcing": nonautonomous_forcing_v2,
    "semilinear-advection-diffusion-ramped": semilinear_advection_diffusion_v2,
}
SHORT = {"rob": "robertson-ramped", "hires": "hires-ramped", "vdp": "van-der-pol-ramped",
         "rot": "rotating-nonnormal", "forc": "nonautonomous-stiff-forcing",
         "semi": "semilinear-advection-diffusion-ramped"}


def build(family: str, n: int = 96, grid=None, allow_holdout: bool = False) -> CorpusProblem:
    family = SHORT.get(family, family)
    if family in HOLDOUT_FAMILIES:
        if not allow_holdout:
            raise PermissionError(f"{family} is a v2 HOLDOUT family; pass allow_holdout=True for definition checks "
                                  "only (docs/HOLDOUT_HYGIENE.md)")
        return _holdout(family)
    if family not in _BUILDERS:
        raise KeyError(family)
    if family == "semilinear-advection-diffusion-ramped":
        return semilinear_advection_diffusion_v2(n, grid)
    if n < BLOCK_WIDTH[family]:
        raise ValueError("n smaller than one block")
    return _BUILDERS[family](n)


def calibration_problems(n: int = 96) -> list:
    return [build(fam, n) for fam in CALIBRATION_FAMILIES]


def calibration_cases(n: int = 96) -> list:
    """The 18 (family x rtol) rows at dimension n, in the Rust spec order (:169-187)."""
    out = []
    for p in calibration_problems(n):
        for rtol in CORPUS_RTOLS:
            out.append({"case_id": p.case_id(rtol), "family": p.family, "n": n, "rtol": rtol,
                        "atol": p.atol(rtol), "problem": p})
    return out


# ----------------------------------------------------------------------------------------------------
# references
# ----------------------------------------------------------------------------------------------------
def load_stored_reference(problem: CorpusProblem):
    """selected_raw reference artifact (Radau L2 1e-12/1e-14 on the 101-point grid) or None."""
    if problem.partition != "calibration":
        return None                                  # holdout references are never read here
    path = os.path.join(STORED_REFERENCE_DIR, problem.name + ".json")
    if not os.path.exists(path):
        return None
    with open(path) as fh:
        art = json.load(fh)
    conv = art["convergence"]
    return {"source": path, "times": np.array(art["requested_times"]), "states": np.array(art["states"]),
            "uncertainty_wrms": float(conv["reference_uncertainty_wrms"]), "d0": conv["d0_max_grid_wrms"],
            "d1": conv["d1_max_grid_wrms"], "q": conv["q"],
            "method_disagreement_wrms": conv["method_disagreement_wrms"],
            "canonical_method": art["canonical_method"]}


def load_generated_reference(problem: CorpusProblem):
    path = os.path.join(GENERATED_REFERENCE_DIR, problem.name + ".npz")
    if problem.partition != "calibration" or not os.path.exists(path):
        return None
    z = np.load(path)
    return {"source": path, "times": z["times"], "states": z["states"],
            "uncertainty_wrms": float(z["uncertainty_wrms"]), "canonical_method": str(z["method"])}


def radau_reference(problem: CorpusProblem, rtol: float = 1e-12, atol: float = 1e-14, times=None,
                    first_step=None, dense_output=False, sparse=True):
    """SciPy Radau with the analytic Jacobian on the output grid (segment-wise for split problems).
    Returns (times, states, info)."""
    from scipy.integrate import solve_ivp
    times = problem.output_times if times is None else np.asarray(times)
    y = problem.y0
    states, ts = [], []
    info = {"nfev": 0, "njev": 0, "nlu": 0, "steps": 0, "success": True, "message": ""}
    jac = problem.J_sparse if sparse else problem.J
    sols = []
    for k, (a, b, fseg) in enumerate(problem.segments):
        seg_t = times[(times >= a) & (times <= b)]
        kw = {}
        if first_step is not None:
            kw["first_step"] = first_step
        r = solve_ivp(fseg, (a, b), y, method="Radau", rtol=rtol, atol=atol, jac=jac, t_eval=seg_t,
                      dense_output=dense_output, **kw)
        info["nfev"] += int(r.nfev); info["njev"] += int(r.njev); info["nlu"] += int(r.nlu)
        info["success"] = info["success"] and bool(r.success); info["message"] = str(r.message)
        if r.sol is not None:
            info["steps"] += len(r.sol.ts) - 1
            sols.append(r.sol)
        first = 0 if k == 0 else 1
        states.extend(r.y.T[first:]); ts.extend(seg_t[first:])
        y = r.y[:, -1].copy()
    info["sol"] = sols
    return np.array(ts), np.array(states), info


_RUST_ROWS = None


def rust_recorded_rows() -> dict:
    """Recorded canonical v2 campaign rows (all 54 calibration cases, rev ab8fbcd), keyed by case_id:
    clipped/dense max-grid and endpoint errors (tight basis and case units) and the work counters.
    Lazily read from calibration_all_cases_compact.json."""
    global _RUST_ROWS
    if _RUST_ROWS is None:
        with open(STORED_RUST_COMPACT) as fh:
            d = json.load(fh)
        rows = {}
        for rec in d["records"]:
            a = rec["artifact"]; s = a["spec"]
            row = {"family": s["family"], "n": s["dimension"], "rtol": s["rtol"], "atol": s["atol"],
                   "reference_uncertainty_wrms": a["reference_uncertainty_wrms"], "status": a["row"]["status"]}
            for arm in ("clipped", "dense"):
                m = a[arm]["metrics"]; c = a[arm]["counters"]; dg = a[arm]["diagnostics"]
                row[arm] = {"max_grid_wrms": m["max_grid_wrms"], "endpoint_wrms": m["endpoint_wrms"],
                            "rms_grid_wrms": m["rms_grid_wrms"],
                            "max_grid_case": case_units(m["max_grid_wrms"], s["rtol"]),
                            "endpoint_case": case_units(m["endpoint_wrms"], s["rtol"]),
                            "attempts": dg["attempts"], "accepted": dg["accepted_macro_steps"],
                            "rejected": dg["rejected_macro_steps"], "accepted_step_sizes": dg["accepted_step_sizes"],
                            "error_norms": dg["error_norms"],
                            **{k: c[k] for k in ("rhs_evaluations", "ft_calls", "jvp_vectors", "linear_solves",
                                                 "linear_iterations", "linear_matvecs", "preconditioner_apps",
                                                 "orthogonalization_inner_products",
                                                 "orthogonalization_vector_updates", "diagnostic_matvecs",
                                                 "direct_factorizations")}}
            row["dense_exceeds_tolerance"] = row["dense"]["max_grid_case"] > 1.0
            rows[s["id"]] = row
        _RUST_ROWS = rows
    return _RUST_ROWS


# ----------------------------------------------------------------------------------------------------
# operator diagnostics
# ----------------------------------------------------------------------------------------------------
def operator_stats(Jm: np.ndarray, fov_angles: int = 0) -> dict:
    """Spectral radius, spectral abscissa, mu_2 (2-norm logarithmic norm = numerical abscissa),
    Henrici departure from normality (absolute and relative to ||J||_F), eigenvalue sector angle
    max |arg(-lambda)| (deg), and optionally the field-of-values sector angle (deg) from fov_angles
    support-function samples (only meaningful when mu_2 < 0)."""
    ev = np.linalg.eigvals(Jm)
    fro2 = float(np.sum(Jm * Jm))
    dep2 = max(fro2 - float(np.sum(np.abs(ev) ** 2)), 0.0)
    mu2 = float(np.linalg.eigvalsh(0.5 * (Jm + Jm.T))[-1])
    rho = float(np.abs(ev).max())
    tol = 1e-9 * max(rho, 1e-300)
    stable = ev[ev.real < -tol]
    ang = float(np.degrees(np.max(np.abs(np.angle(-stable))))) if len(stable) else 0.0
    re = -stable.real
    out = {"rho": rho, "alpha": float(ev.real.max()), "mu2": mu2,
           "henrici": math.sqrt(dep2), "henrici_rel": math.sqrt(dep2 / fro2) if fro2 > 0 else 0.0,
           "eig_sector_deg": ang,                                  # over eigenvalues with Re < -1e-9 rho
           "n_unstable": int(np.sum(ev.real > tol)), "n_near_zero": int(np.sum(np.abs(ev.real) <= tol)),
           "stiffness_ratio": float(re.max() / re.min()) if len(re) else float("nan"),
           "n_complex": int(np.sum(np.abs(ev.imag) > 1e-12 * rho))}
    if fov_angles:
        zs = []
        for th in np.linspace(0.0, 2 * np.pi, fov_angles, endpoint=False):
            Hm = 0.5 * (np.exp(1j * th) * Jm + np.exp(-1j * th) * Jm.T)
            w, V = np.linalg.eigh(Hm)
            x = V[:, -1]
            zs.append(np.vdot(x, Jm @ x))
        zs = np.array(zs)
        out["fov_sector_deg"] = float(np.degrees(np.max(np.abs(np.angle(-zs))))) if mu2 < 0 else float("nan")
        out["fov_min_re"] = float(zs.real.min())
    return out


# ----------------------------------------------------------------------------------------------------
# self-test (fast; the heavy validation lives in validate_corpus.py)
# ----------------------------------------------------------------------------------------------------
def _check_oracles(verbose=True) -> dict:
    res = {}
    with open(ORACLE_CALIBRATION) as fh:
        orc = json.load(fh)
    t = float(orc["time"])
    for case in orc["cases"]:
        p = build(case["family"], int(case["dimension"]))
        n = p.n
        y = p.y0 + 0.01 * np.cos(0.17 * (np.arange(n) + 1.0))
        v = np.sin(0.23 * (np.arange(n) + 1.0))
        fy, jv = p.f(t, y), p.jvp(t, y, v)
        jd = p.J(t, y) @ v
        worst = 0.0
        for smp in case["samples"]:
            i = int(smp["index"])
            for act, dec in ((p.y0[i], smp["y0"]), (fy[i], smp["rhs"]), (jv[i], smp["jvp"]), (jd[i], smp["jvp"])):
                e = float(dec)
                worst = max(worst, abs(float(act) - e) / max(1.0, abs(e)))
        res[case["family"]] = worst
    with open(ORACLE_SEMILINEAR) as fh:
        ors = json.load(fh)
    for case in ors["cases"]:
        n = int(case["dimension"])
        p = build("semilinear-advection-diffusion-ramped", n)
        t = float(case["time"])
        phi = p.exact(t)
        y = phi + 0.01 * np.cos(0.17 * (np.arange(n) + 1.0))
        v = np.sin(0.23 * (np.arange(n) + 1.0))
        fphi, fy, jv, ftv = p.f(t, phi), p.f(t, y), p.jvp(t, y, v), p.ft(t, y)
        worst = 0.0; bits_ok = True
        for smp in case["samples"]:
            i = int(smp["index"])
            bits = int(np.array(phi[i], dtype=">f8").view(">u8"))
            bits_ok &= bits == int(smp["phi_f64_bits"])
            for act, dec in ((phi[i], smp["phi"]), (fphi[i], smp["rhs_at_phi"]), (fy[i], smp["rhs_perturbed"]),
                             (jv[i], smp["jvp"]), (ftv[i], smp["partial_t"])):
                e = float(dec)
                worst = max(worst, abs(float(act) - e) / max(1.0, abs(e)))
        Js = p.J_sparse(t, y).tocoo()
        bw = int(np.max(np.abs(Js.row - Js.col)))
        res[f"semilinear-n{n}"] = worst
        res[f"semilinear-n{n}-phi-bits-identical"] = bool(bits_ok)
        res[f"semilinear-n{n}-half-bandwidth"] = bw == int(case["expected_half_bandwidth"])
    if verbose:
        for k, v in res.items():
            print(f"  oracle {k:45s} {v}")
    return res


def _fd_checks(p: CorpusProblem, rng, verbose=True) -> dict:
    """analytic JVP vs central FD, dense J vs JVP columns, sparse vs dense, ft vs FD in t."""
    n = p.n
    t0, tf = p.span
    worst = {"jvp_fd": 0.0, "J_vs_jvp": 0.0 if n <= 600 else float("nan"),
             "Jsparse_vs_J": 0.0 if n <= 600 else float("nan"), "ft_fd": 0.0}   # dense J checked for n <= 600
    for tt in (t0, t0 + 0.37 * (tf - t0), t0 + 0.5 * (tf - t0), t0 + 0.61 * (tf - t0), tf):
        y = p.y0 + 0.05 * rng.standard_normal(n) * (1.0 + np.abs(p.y0))
        v = rng.standard_normal(n)
        jv = p.jvp(tt, y, v)
        eps = 1e-6 * (1.0 + np.linalg.norm(y)) / np.linalg.norm(v)
        fd = (p.f(tt, y + eps * v) - p.f(tt, y - eps * v)) / (2 * eps)
        worst["jvp_fd"] = max(worst["jvp_fd"], np.linalg.norm(fd - jv) / max(np.linalg.norm(jv), 1e-300))
        if n <= 600:
            Jm = p.J(tt, y)
            worst["J_vs_jvp"] = max(worst["J_vs_jvp"], np.linalg.norm(Jm @ v - jv) / max(np.linalg.norm(jv), 1e-300))
            worst["Jsparse_vs_J"] = max(worst["Jsparse_vs_J"], float(np.abs(p.J_sparse(tt, y).toarray() - Jm).max()
                                                                 / max(np.abs(Jm).max(), 1e-300)))
        if p.partition == "calibration":
            dt = 1e-6 * max(1.0, abs(tf - t0))
            ftd = (p.f(tt + dt, y) - p.f(tt - dt, y)) / (2 * dt)
            ftv = p.ft(tt, y)
            worst["ft_fd"] = max(worst["ft_fd"], np.linalg.norm(ftd - ftv) / max(np.linalg.norm(ftv), 1.0))
    if p.exact is not None:
        # manufactured solution: f(t, phi) = phi'(t)
        tt = t0 + 0.43 * (tf - t0); dt = 1e-6
        dphi = (p.exact(tt + dt) - p.exact(tt - dt)) / (2 * dt)
        worst["exact_residual"] = float(np.linalg.norm(p.f(tt, p.exact(tt)) - dphi) / max(np.linalg.norm(dphi), 1.0))
    if verbose:
        print(f"  fd {p.name:55s} " + " ".join(f"{k}={v:.2e}" for k, v in worst.items()))
    return worst


def self_test(verbose=True) -> bool:
    rng = np.random.default_rng(20261008)
    ok = True
    if verbose:
        print(f"corpus_v2 self-test ({VERSION}, {SOURCE})")
    orc = _check_oracles(verbose)
    for k, v in orc.items():
        if isinstance(v, bool):
            ok &= v
        else:
            ok &= v <= (5e-14 if k.startswith("semilinear") else 1e-13)
    for n in (96, 384):
        for fam in CALIBRATION_FAMILIES:
            w = _fd_checks(build(fam, n), rng, verbose)
            ok &= w["jvp_fd"] < 1e-6 and w["J_vs_jvp"] < 1e-13 and w["Jsparse_vs_J"] < 1e-15 and w["ft_fd"] < 1e-5
            ok &= w.get("exact_residual", 0.0) < 1e-6
    # off-contract parameterisations (padding paths, other grids)
    for fam, n in (("robertson-ramped", 98), ("hires-ramped", 100), ("van-der-pol-ramped", 97),
                   ("rotating-nonnormal", 97), ("nonautonomous-stiff-forcing", 50)):
        w = _fd_checks(build(fam, n), rng, verbose)
        ok &= w["jvp_fd"] < 1e-6 and w["J_vs_jvp"] < 1e-13 and w["ft_fd"] < 1e-5
    w = _fd_checks(semilinear_advection_diffusion_v2(60, grid=(6, 10)), rng, verbose)
    ok &= w["jvp_fd"] < 1e-6 and w["J_vs_jvp"] < 1e-13 and w["ft_fd"] < 1e-5
    for fam in HOLDOUT_FAMILIES:   # definition check only, no trajectory, no reference read
        w = _fd_checks(build(fam, allow_holdout=True), rng, verbose)
        ok &= w["jvp_fd"] < 1e-6 and w["J_vs_jvp"] < 1e-13
    # case ids and spec metadata
    ids = [c["case_id"] for c in calibration_cases(96)]
    ok &= len(ids) == 18 and ids[0] == "robertson-ramped-n96-rtol-1e-4-v2.1" and \
        ids[-1] == "semilinear-advection-diffusion-ramped-n96-grid8x12-rtol-1e-8-v2.1"
    ok &= abs(diversity_multiplier(0) - 1.0) < 1e-15 and abs(diversity_multiplier(1) - 0.95) < 1e-15
    try:
        build("brusselator-2d")
        ok = False
    except PermissionError:
        pass
    if verbose:
        print("SELF-TEST", "PASS" if ok else "FAIL")
    return bool(ok)


if __name__ == "__main__":
    import sys
    sys.exit(0 if self_test() else 1)
