#!/usr/bin/env python3
"""Scalar dependency DAG of one RODAS5P attempt on a banded system (node SL-R5).

V05 (theory S1) established that the *vector-level* graph of the eight right-hand sides has
depth 8 for a black-box right-hand side. This script resolves every stage into its scalar
operations on a banded system with a row-local right-hand side and measures the longest
dependency path (critical path, unit cost per substitution row) of four schedules:

  lu       every stage solved with the same LU: forward sweep 0 -> n-1, backward n-1 -> 0
  alt      odd stages LU, even stages UL (LU of the flipped matrix): consecutive sweeps of
           neighbouring stages run in the same direction and can overlap with a lag
  twoway   elimination from both ends towards the middle (GEPP of a permuted matrix), every
           stage: two inward forward sweeps, a dense middle block, two outward sweeps
  spikeP   P equal partitions with independently factored diagonal blocks (restricted
           pivoting), dense reduced interface system, pointwise correction

Nodes per stage i and row k: G (stage argument), F (right-hand side), R (assembled linear
right-hand side), Z (after the first substitution), U (stage solution). Edges:

  G(i,k) <- U(j,k), A[i][j] != 0          F(i,k) <- G(i,k'), |k'-k| <= w
  R(i,k) <- F(i,k), U(j,k), C[i][j] != 0
  Z and U: the substitution recurrences of the schedule, with the windows of banded partial
  pivoting (forward step k reads rows k..k+l; backward row k reads k+1..k+u+l)

The tableau patterns A and C are read from the protected snapshot; no coefficient value is
used. Two cost models are reported: `sweep` charges one unit per Z or U row and nothing for
the pointwise nodes; `unit` charges one unit for every node.

Output: JSON with every case and the affine fits depth = slope * n + intercept, which are
exact for the cases listed (checked on three sizes per family). Standard library only.
"""
from __future__ import annotations

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
SNAP = json.loads((ROOT / "fixtures/rodas5p_coefficients_snapshot.json").read_text())
A_PAT = [[float(x) != 0.0 for x in row] for row in SNAP["A"]]
C_PAT = [[float(x) != 0.0 for x in row] for row in SNAP["C"]]
STAGES = len(A_PAT)


def stage_inputs(i, done_u, n, w, pointwise_cost):
    """Completion times of R(i, .) given the completed stage solutions `done_u`."""
    g = [0.0] * n
    for k in range(n):
        deps = [done_u[j][k] for j in range(i) if A_PAT[i][j]]
        g[k] = (max(deps) if deps else 0.0) + pointwise_cost
    f = [0.0] * n
    for k in range(n):
        lo, hi = max(0, k - w), min(n - 1, k + w)
        f[k] = max(g[lo:hi + 1]) + pointwise_cost
    r = [0.0] * n
    for k in range(n):
        deps = [done_u[j][k] for j in range(i) if C_PAT[i][j]]
        r[k] = max([f[k]] + deps) + pointwise_cost
    return r


def sweep_up(r, n, l, u):
    """LU orientation: forward 0 -> n-1 (reads rows k..k+l), backward n-1 -> 0."""
    z = [0.0] * n
    for k in range(n):
        deps = [max(r[k:min(n, k + l + 1)])]
        deps += z[max(0, k - l):k]
        z[k] = max(deps) + 1
    x = [0.0] * n
    for k in range(n - 1, -1, -1):
        deps = [z[k]] + x[k + 1:min(n, k + u + l + 1)]
        x[k] = max(deps) + 1
    return x


def sweep_down(r, n, l, u):
    """UL orientation = LU of the flipped matrix (lower and upper widths exchange)."""
    flipped = sweep_up(r[::-1], n, u, l)
    return flipped[::-1]


def sweep_twoway(r, n, l, u, middle_cost):
    """Two-way elimination with the split at m = n // 2."""
    m = n // 2
    top_cols = m - l            # columns eliminated by the top-down sweep
    bot_first = m + u           # first column eliminated by the bottom-up sweep
    z = [0.0] * n
    for k in range(top_cols):
        deps = [max(r[k:k + l + 1])] + z[max(0, k - l):k]
        z[k] = max(deps) + 1
    for k in range(n - 1, bot_first - 1, -1):
        deps = [max(r[max(0, k - u):k + 1])] + z[k + 1:min(n, k + u + 1)]
        z[k] = max(deps) + 1
    # dense middle block on columns [m - l, m + u): needs both fronts and its own rows
    deps = r[top_cols:bot_first]
    if top_cols > 0:
        deps = deps + z[max(0, top_cols - l):top_cols]
    if bot_first < n:
        deps = deps + z[bot_first:min(n, bot_first + u)]
    mid = max(deps) + middle_cost
    x = [0.0] * n
    for k in range(top_cols, bot_first):
        x[k] = mid
    for k in range(top_cols - 1, -1, -1):
        x[k] = max([z[k]] + x[k + 1:min(n, k + u + l + 1)]) + 1
    for k in range(bot_first, n):
        x[k] = max([z[k]] + x[max(0, k - u - l):k]) + 1
    return x


def sweep_spike(r, n, l, u, parts, middle_cost):
    """P independently factored blocks, reduced system, pointwise correction."""
    bounds = [round(p * n / parts) for p in range(parts + 1)]
    tips = []
    local = [0.0] * n
    for p in range(parts):
        lo, hi = bounds[p], bounds[p + 1]
        seg = sweep_up(r[lo:hi], hi - lo, l, u)
        local[lo:hi] = seg
        tips.append(max(seg))            # the reduced system reads both ends of the block
    reduced = max(tips) + middle_cost
    return [max(v, reduced) + 1 for v in local]   # x = z - spikes * x_interface


def depth(schedule, n, l, u, w, pointwise_cost, middle_cost=1.0, parts=4):
    done_u = []
    for i in range(STAGES):
        r = stage_inputs(i, done_u, n, w, pointwise_cost)
        if schedule == "lu":
            x = sweep_up(r, n, l, u)
        elif schedule == "alt":
            x = sweep_up(r, n, l, u) if i % 2 == 0 else sweep_down(r, n, l, u)
        elif schedule == "twoway":
            x = sweep_twoway(r, n, l, u, middle_cost)
        elif schedule == "spike":
            x = sweep_spike(r, n, l, u, parts, middle_cost)
        else:
            raise ValueError(schedule)
        done_u.append(x)
    return max(done_u[-1]) + pointwise_cost    # y_new = g_8 + u_8


def affine_fit(schedule, l, u, w, pointwise_cost, sizes, **kw):
    values = [depth(schedule, n, l, u, w, pointwise_cost, **kw) for n in sizes]
    slope = (values[1] - values[0]) / (sizes[1] - sizes[0])
    intercept = values[0] - slope * sizes[0]
    exact = all(abs(v - (slope * n + intercept)) < 1e-9 for v, n in zip(values, sizes))
    return {"slope": slope, "intercept": intercept, "exact_on_sizes": exact,
            "sizes": list(sizes), "depths": values}


def main() -> int:
    out = {
        "schema": "vigilode.referee2.stage-lattice-dag.v1",
        "tableau_edges": {
            "A_strict_lower_nonzero": sum(A_PAT[i][j] for i in range(STAGES) for j in range(i)),
            "C_strict_lower_nonzero": sum(C_PAT[i][j] for i in range(STAGES) for j in range(i)),
            "subdiagonal_A_all_nonzero": all(A_PAT[i + 1][i] for i in range(STAGES - 1)),
        },
        "fits": [],
        "claims": {},
    }
    sizes = (480, 960, 1920)
    ok = True
    for (l, u, w) in [(1, 1, 1), (2, 2, 2), (2, 2, 1), (4, 3, 2), (16, 16, 16)]:
        for cost_name, pointwise_cost in [("sweep", 0.0), ("unit", 1.0)]:
            row = {"l": l, "u": u, "w": w, "cost_model": cost_name}
            row["lu"] = affine_fit("lu", l, u, w, pointwise_cost, sizes)
            row["alt"] = affine_fit("alt", l, u, w, pointwise_cost, sizes)
            row["twoway"] = affine_fit("twoway", l, u, w, pointwise_cost, sizes)
            row["spike4"] = affine_fit("spike", l, u, w, pointwise_cost, sizes, parts=4)
            row["spike8"] = affine_fit("spike", l, u, w, pointwise_cost, sizes, parts=8)
            out["fits"].append(row)
            ok &= all(row[s]["exact_on_sizes"] for s in ("lu", "alt", "twoway", "spike4", "spike8"))
            ok &= row["lu"]["slope"] == 2 * STAGES
            ok &= row["alt"]["slope"] == STAGES + 1
            ok &= row["twoway"]["slope"] == STAGES
            ok &= abs(row["spike4"]["slope"] - 2 * STAGES / 4) < 1e-9
            ok &= abs(row["spike8"]["slope"] - 2 * STAGES / 8) < 1e-9
    out["claims"] = {
        "lu_slope_is_2s": 2 * STAGES,
        "alternating_slope_is_s_plus_1": STAGES + 1,
        "twoway_slope_is_s": STAGES,
        "spikeP_slope_is_2s_over_P": "2 s / P",
        "all_fits_exact_and_slopes_as_claimed": bool(ok),
    }
    text = json.dumps(out, indent=1, sort_keys=True)
    if len(sys.argv) > 1:
        pathlib.Path(sys.argv[1]).write_text(text + "\n")
    for row in out["fits"]:
        print(row["l"], row["u"], row["w"], row["cost_model"],
              {s: (row[s]["slope"], row[s]["intercept"]) for s in ("lu", "alt", "twoway", "spike4", "spike8")})
    print("claims:", out["claims"])
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
