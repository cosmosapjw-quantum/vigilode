"""Independent Python references for BASS/rei DAG node P05 (accepted-step
conservative remap of the LRS photon transport adapter).

Nothing here is ported from the Rust `remesh.rs`. The pieces are written
from their definitions:

* the aligned mesh of a uniform base grid: the interior base edge nearest
  `x_chi` (ties to the lower index) moves onto `x_chi` unless `x_chi` lies
  within a quarter base width of an end (then the base grid); with
  `refine` the two cells beside the moved edge are halved;
* the piecewise-constant overlap remap of cell integrals: each new cell
  receives `N_i |I_i cap J_j| / |I_i|` from every old cell it overlaps,
  summed exactly rounded (`math.fsum`); signed columns are remapped by sign
  (nonnegative parts and negated negative parts separately, then their
  difference); the angular columns and the absorbed-ledger column are
  remapped, `L_lo` and `L_hi` are copied;
* the segmented semi-discrete reference: on each segment the P02 operator
  `A(t)` (assembled from the read-only formula oracle by the P02 reference
  `research/bassrei_p02_fixed_grid_20261021/reference/lrs_reference.py`,
  loaded by file path, never copied or modified) on the segment's mesh is
  integrated by scipy (Radau with the sparse analytic Jacobian, or DOP853),
  restarted at every output and remap time; at a remap time the state is
  remapped onto the aligned mesh of `x_chi(t)` computed here.

The P02 reference supports nonuniform energy edges (the oracle's densities
are `N / (dq_i dm_j)` per cell); `test_p05_reference.py` checks this on the
aligned meshes (conservation, the absorber cell averages against mpmath
quadrature, the transport against a direct upwind sum).

Python allocation is intentional (formula references, not a solver port).
"""
from __future__ import annotations

import importlib.util
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True

import numpy as np  # noqa: E402

NODE_DIR = Path(__file__).resolve().parents[1]
REPO = NODE_DIR.parents[1]
P02_REFERENCE = REPO / "research" / "bassrei_p02_fixed_grid_20261021" / "reference" / "lrs_reference.py"


def load_p02_reference(path=P02_REFERENCE):
    """Import the P02 reference read-only by file path (no bytecode)."""
    prev = sys.dont_write_bytecode
    sys.dont_write_bytecode = True
    try:
        name = "p02_lrs_reference_readonly"
        if name in sys.modules:
            return sys.modules[name]
        spec = importlib.util.spec_from_file_location(name, str(path))
        mod = importlib.util.module_from_spec(spec)
        sys.modules[name] = mod
        spec.loader.exec_module(mod)
    finally:
        sys.dont_write_bytecode = prev
    return mod


R = load_p02_reference()


# ---------------------------------------------------------------------------
# Meshes
# ---------------------------------------------------------------------------


def base_edge(lo, hi, n, k):
    """Uniform base edge k (hi exactly at k = n)."""
    return hi if k == n else lo + (hi - lo) * k / n


def base_edges(lo, hi, n):
    return [base_edge(lo, hi, n, k) for k in range(n + 1)]


def aligned_index(lo, hi, n, x):
    """The interior base edge nearest x (ties to the lower index), or None
    when x is within a quarter base width of an end, outside, or NaN."""
    if not (isinstance(x, float) and math.isfinite(x)):
        return None
    quarter = 0.25 * ((hi - lo) / n)
    if not (x - lo > quarter and hi - x > quarter):
        return None
    best, dist = None, math.inf
    for k in range(1, n):
        d = abs(x - base_edge(lo, hi, n, k))
        if d < dist:  # strict: a tie keeps the lower index
            best, dist = k, d
    return best


def aligned_edges(lo, hi, n, x, refine=False):
    """(edges, face index or None) of the aligned mesh of x."""
    k = aligned_index(lo, hi, n, x)
    if k is None:
        return base_edges(lo, hi, n), None
    edges = []
    for m in range(n + 1):
        e = x if m == k else base_edge(lo, hi, n, m)
        if refine and m in (k, k + 1):
            edges.append(0.5 * (edges[-1] + e))
        edges.append(e)
    return edges, (k + 1 if refine else k)


def x_chi(geom, absorber, t):
    """Comoving threshold position q_chi + ln a(t)."""
    return float(absorber.q_chi + float(geom.ln_a(t)))


# ---------------------------------------------------------------------------
# Overlap remap
# ---------------------------------------------------------------------------


def overlaps(a, b):
    """Every (i, j, |a_i cap b_j|) with positive overlap, by direct search."""
    out = []
    for i in range(len(a) - 1):
        for j in range(len(b) - 1):
            left, right = max(a[i], b[j]), min(a[i + 1], b[j + 1])
            if right > left:
                out.append((i, j, right - left))
    return out


def remap_nonnegative(src_edges, src, dst_edges):
    """Piecewise-constant overlap remap of nonnegative cell integrals."""
    if src_edges[0] != dst_edges[0] or src_edges[-1] != dst_edges[-1]:
        raise ValueError("domains differ")
    if any((not math.isfinite(v)) or v < 0 for v in src):
        raise ValueError("nonnegative finite integrals required")
    parts = [[] for _ in range(len(dst_edges) - 1)]
    for i, j, ov in overlaps(src_edges, dst_edges):
        if src[i] != 0.0:
            parts[j].append(src[i] * (ov / (src_edges[i + 1] - src_edges[i])))
    return [math.fsum(p) for p in parts]


def remap_signed(src_edges, src, dst_edges):
    pos = [v if v > 0 else 0.0 for v in src]
    neg = [-v if v < 0 else 0.0 for v in src]
    p = remap_nonnegative(src_edges, pos, dst_edges)
    q = remap_nonnegative(src_edges, neg, dst_edges)
    return [a - b for a, b in zip(p, q)]


def remap_state(src_edges, y, dst_edges, nm):
    """P02 layout `[L_lo, (N_i0..N_i(nm-1), A_i)_i, L_hi]`: every angular
    column and the absorbed column by sign; L_lo, L_hi copied."""
    y = [float(v) for v in y]
    nq_old, nq_new = len(src_edges) - 1, len(dst_edges) - 1
    stride = nm + 1
    if len(y) != 2 + nq_old * stride:
        raise ValueError("state length")
    out = [0.0] * (2 + nq_new * stride)
    out[0], out[-1] = y[0], y[-1]
    for col in range(stride):
        column = [y[1 + i * stride + col] for i in range(nq_old)]
        new = remap_signed(src_edges, column, dst_edges)
        for i, v in enumerate(new):
            out[1 + i * stride + col] = v
    return out


def column_rel_diff(a, b, nq, nm):
    """Per column (angular columns, then the absorbed ledgers): max |a - b|
    over the column / max |b| over the column (0 for an all-zero column
    with a == b)."""
    a, b = np.asarray(a, float), np.asarray(b, float)
    stride = nm + 1
    out = []
    for col in range(stride):
        idx = 1 + np.arange(nq) * stride + col
        scale = float(np.max(np.abs(b[idx])))
        d = float(np.max(np.abs(a[idx] - b[idx])))
        out.append(d / scale if scale > 0 else (0.0 if d == 0 else math.inf))
    return out


# ---------------------------------------------------------------------------
# Segmented semi-discrete reference
# ---------------------------------------------------------------------------


def atol_rule(grid, scale):
    """The runner's cell rule: scale x cell measure on cells, scale on the
    ledgers."""
    a = np.full(grid.n, float(scale))
    a[grid.cells] = scale * grid.measure()
    return a


def segmented_reference(oracle, geom, gamma, absorber, base, mu_edges, y0, T, remap_times, outputs,
                        method, rtol, atol_scale):
    """Integrate the P02 model segment by segment with independent remaps at
    `remap_times`. `base = (lo, hi, n)`. Returns (states at `outputs`,
    meshes of those outputs, info)."""
    lo, hi, n = base
    nm = len(mu_edges) - 1
    edges = base_edges(lo, hi, n)
    y = np.array(y0, float)
    t = 0.0
    outs = list(outputs)
    states, meshes = [], []
    if outs and outs[0] == 0.0:
        states.append(y.copy())
        meshes.append(list(edges))
        outs = outs[1:]
    seg_ends = list(remap_times) + [T]
    info = {"segments": [], "nfev": 0, "njev": 0, "nlu": 0}
    for seg_end in seg_ends:
        grid = R.Grid(edges, mu_edges, "comoving", "outflow")
        model = R.Model(oracle, grid, geom, gamma, absorber, (t, seg_end))
        inner = [o for o in outs if t < o < seg_end]
        times = [t] + inner + [seg_end]
        traj, rinfo = R.ref_ivp(model, y, times, method, rtol, atol_rule(grid, atol_scale))
        for key in ("nfev", "njev", "nlu"):
            info[key] += rinfo[key]
        for o, s in zip(times[1:-1], traj[1:-1]):
            states.append(np.array(s))
            meshes.append(list(edges))
        if seg_end in outs:
            states.append(np.array(traj[-1]))
            meshes.append(list(edges))
        outs = [o for o in outs if o > seg_end]
        y = np.array(traj[-1])
        seg = {"t0": t, "t1": seg_end, "cells": grid.nq,
               "split_deviation": (model.split_info or {}).get("max_split_deviation_rel")}
        if seg_end < T:
            x = x_chi(geom, absorber, seg_end)
            new, face = aligned_edges(lo, hi, n, x, False)
            y = np.array(remap_state(edges, y, new, nm))
            seg.update({"remap_x_chi": x, "face_index": face})
            edges = new
        info["segments"].append(seg)
        t = seg_end
    return states, meshes, info
