#!/usr/bin/env python3
"""Opt-in original-26-event 46,080-cell photon-source group fusion.

Research source-only experiment. Does not replace the REI owner 26-event
MPRK22/SDIRK2 map or certify its full step/Jacobian/thermal closures.
"""
from __future__ import annotations
import argparse
import importlib.util
import json
import math
import statistics
import sys
from pathlib import Path
from time import perf_counter_ns

import numpy as np
import original26_photon_lowrank_check as baseline

LD = np.longdouble
N = baseline.N


def coefficients(n, currents, external, alphas, excess, ev):
    """Factor four group denominators into 3 source and 3x3 JVP coefficients.

    P_is = n_is*kappa_s, heat_i = sum_s n_is*q_s,
    dP_is = kappa_s*dn_is - n_is*sum_t B_st*sum_j dn_jt.
    All global reductions and coefficient sums are in extended precision.
    The final local fields are f64, matching the original shadow experiment.
    """
    n = np.asarray(n, dtype=np.float64)
    a = np.asarray(alphas, dtype=LD)
    j = np.asarray(currents, dtype=LD)
    e = np.asarray(external, dtype=LD)
    excess = np.asarray(excess, dtype=LD)
    if n.ndim != 2 or n.shape[1] != 3 or a.shape != (4, 3) or j.shape != (4,) or e.shape != (4,) or excess.shape != (3, 4):
        raise ValueError("PHOTON_FUSION_SHAPE")
    if not (np.all(np.isfinite(n)) and np.all(n >= 0) and
            np.all(np.isfinite(a)) and np.all(a >= 0) and
            np.all(np.isfinite(j)) and np.all(j >= 0) and
            np.all(np.isfinite(e)) and np.all(e >= 0)):
        raise ValueError("PHOTON_FUSION_INVALID_INPUT")
    totals = np.sum(n.astype(LD), axis=0, dtype=LD)
    den = e + a @ totals
    if not np.all(np.isfinite(den) & (den > 0)):
        raise ArithmeticError("NONPOSITIVE_PHOTON_DENOMINATOR")
    c = (j[:, None] * a) / den[:, None]
    kappa = np.sum(c, axis=0, dtype=LD)
    q = np.sum(c * excess.T * LD(ev), axis=0, dtype=LD)
    B = np.sum(
        (j / (den * den))[:, None, None] * a[:, :, None] * a[:, None, :],
        axis=0, dtype=LD
    )
    escaping = np.sum(j * e / den, dtype=LD)
    if not all(np.all(np.isfinite(x)) for x in (kappa, q, B, np.asarray(escaping))):
        raise ArithmeticError("PHOTON_FUSION_NONFINITE_COEFFICIENT")
    return kappa, q, B, escaping


def fused_photon_fields(n, coeff):
    kappa, q, _B, escaping = coeff
    arr = np.asarray(n, dtype=np.float64)
    # The only local rounding is at extended->f64 coefficient conversion
    # and vector arithmetic; admission still needs original owner intervals.
    rates = arr * np.asarray(kappa, dtype=np.float64)[None, :]
    heat = np.sum(arr * np.asarray(q, dtype=np.float64)[None, :], axis=1)
    return rates, heat, float(escaping)


def fused_photon_jvp(n, v, coeff):
    kappa, _q, B, _escaping = coeff
    moment_delta = np.sum(np.asarray(v, dtype=LD), axis=0, dtype=LD)
    nonlocal_response = B @ moment_delta
    return (np.asarray(v) * np.asarray(kappa, dtype=np.float64)[None, :]
            - np.asarray(n) * np.asarray(nonlocal_response, dtype=np.float64)[None, :])


class FrozenPhotonJacobian:
    """Source-only rank-three JVP at exactly one immutable stage state.

    A new instance is mandatory for any new stage state, forcing clock, or
    owner source revision. This is not the full 26-event Jacobian.
    """

    def __init__(self, n, currents, external, alphas, excess, ev):
        self._n = np.array(n, dtype=np.float64, copy=True)
        kappa, _q, B, _escaping = coefficients(
            self._n, currents, external, alphas, excess, ev)
        self._kappa = np.asarray(kappa, dtype=np.float64).copy()
        self._B = np.asarray(B, dtype=LD).copy()
        self._n.setflags(write=False)
        self._kappa.setflags(write=False)
        self._B.setflags(write=False)

    def apply(self, v):
        v = np.asarray(v, dtype=np.float64)
        if v.shape != self._n.shape or not np.all(np.isfinite(v)):
            raise ValueError("INVALID_FROZEN_JVP_DIRECTION")
        delta = np.sum(v.astype(LD), axis=0, dtype=LD)
        nonlocal_response = self._B @ delta
        return (v * self._kappa[None, :]
                - self._n * np.asarray(nonlocal_response, dtype=np.float64)[None, :])


def original_source_inputs(owner: Path, lane: str):
    if not (owner / "stages").is_dir():
        raise FileNotFoundError("REI_ORIGINAL_OWNER_STAGES_MISSING")
    sys.path.insert(0, str(owner))
    f05 = baseline.load("rei_fusion_f05", owner / baseline.REI_F05_PATH)
    edge = f05.load("rei_fusion_edge", owner / "research/rei_igm_original26_temperature_edges_20261101/edge_probe.py")
    stage = owner / edge.REL_STAGE
    worker = f05.load("rei_fusion_worker", stage / "analysis/attempt_worker.py")
    worker.jax_guard.install_if_missing()
    kernel = worker._kernel()
    if f05.sha(worker.KERNEL_PATH) != baseline.KERNEL_SHA:
        raise ArithmeticError("ORIGINAL_KERNEL_DRIFT")
    decoded, source = edge.source_state(stage, lane, worker.state_io)
    if source["archive_sha256"] != baseline.SOURCE_SHA or source["accepted_array_sha256"] != baseline.STATE_SHA:
        raise ArithmeticError("ORIGINAL_PARENT_DRIFT")
    base, _, _, nh, _nhe = worker._context(kernel, lane)
    if len(nh) != N:
        raise ValueError("WRONG_SOURCE_CELL_COUNT")
    model = kernel.rim.ReducedIntervalModel.from_repo(owner)
    tick_time = float(base.forcing.duration_seconds(0)) * 160 / 131072
    forcing = model.forcing_bounds(tick_time, tick_time)
    p = np.asarray((decoded.population_lower + decoded.population_upper) * 0.5, dtype=np.float64)
    if p.shape != (N, 5) or np.any(~np.isfinite(p)) or np.any(p <= 0):
        raise ArithmeticError("INVALID_PARENT_POPULATIONS")
    n = p[:, [0, 2, 3]].copy()
    z = float(baseline.midpoint(forcing.z))
    currents = np.array([float(baseline.midpoint(x)) for x in forcing.current])
    external = np.array([float(baseline.midpoint(x)) for x in forcing.external_subgrid])
    scale = LD(kernel.rim.NH0_CM3) * (LD(1) + LD(z))**2 * LD(kernel.rim.MPC_CM)
    denom = (model.n_h_total, model.n_he_total, model.n_h_total)
    yhe = (1.0, kernel.rim.YHE, kernel.rim.YHE)
    a = np.zeros((4, 3), dtype=LD)
    for g in range(4):
        for s in range(3):
            sig = model.inputs.sigma_cm2[s, g] if model.inputs.owner_support[s + 1, g] else 0.0
            a[g, s] = LD(yhe[s]) * scale * LD(sig) / LD(denom[s])
    owner_boxes = model._explicit_photo_fields(tuple(kernel._I(p[:, s], p[:, s]) for s in range(5)), forcing)
    return n, currents, external, a, model.excess_eV, kernel.rim.EV_ERG, owner_boxes


def bench(fn, count=9):
    for _ in range(2):
        fn()
    samples = []
    for _ in range(count):
        start = perf_counter_ns()
        fn()
        samples.append((perf_counter_ns() - start) * 1e-9)
    return {"median_seconds": statistics.median(samples),
            "min_seconds": min(samples), "max_seconds": max(samples),
            "n": count}


def evaluate(owner: Path, lane: str, repetitions: int = 9):
    n, j, e, a, exc, ev, boxes = original_source_inputs(owner, lane)
    v = n * np.cos((np.arange(N)[:, None] + 1) * np.array([0.101, 0.203, 0.307])[None, :])
    ordinary = lambda: baseline.hybrid_source(n, j, e, a, exc, ev)
    fusion = lambda: fused_photon_fields(n, coefficients(n, j, e, a, exc, ev))
    ordinary_jvp = lambda: baseline.hybrid_lowrank_jvp(n, v, j, e, a)
    fusion_jvp = lambda: fused_photon_jvp(n, v, coefficients(n, j, e, a, exc, ev))
    old_rates, old_heat, old_ext = ordinary()
    new_rates, new_heat, new_ext = fusion()
    jvp_old, jvp_new = ordinary_jvp(), fusion_jvp()
    counts = []
    for s in range(3):
        box = boxes[s]
        counts.append(int(np.count_nonzero((box.lo <= new_rates[:, s]) & (new_rates[:, s] <= box.hi))))
    heat_box = boxes[3]
    heat_in = int(np.count_nonzero((heat_box.lo <= new_heat) & (new_heat <= heat_box.hi)))
    jscale = max(float(np.max(np.abs(jvp_old))), 1e-300)
    jdiff = float(np.max(np.abs(jvp_old - jvp_new)) / jscale)
    diff = float(np.max(np.abs(new_rates - old_rates)) / max(float(np.max(np.abs(old_rates))), 1e-300))
    hdiff = float(np.max(np.abs(new_heat - old_heat)) / max(float(np.max(np.abs(old_heat))), 1e-300))
    total_emitted = float(np.sum(j))
    ledger = abs(float(np.sum(new_rates)) + new_ext - total_emitted) / max(abs(total_emitted), 1e-300)
    valid = all(c == N for c in counts) and heat_in == N and jdiff < 1e-6 and ledger < 1e-12
    # Source-only benchmark, same original cells, not full IVP or RODAS solves.
    time_old = bench(ordinary, repetitions)
    time_fusion = bench(fusion, repetitions)
    jvp_old_times = bench(ordinary_jvp, repetitions)
    jvp_fusion_times = bench(fusion_jvp, repetitions)
    make_cached = lambda: FrozenPhotonJacobian(n, j, e, a, exc, ev)
    setup = bench(make_cached, repetitions)
    cached = make_cached()
    cached_jvp = lambda: cached.apply(v)
    cached_times = bench(cached_jvp, repetitions)
    cached_difference = float(np.max(np.abs(cached_jvp() - jvp_old)) / jscale)
    if cached_difference >= 1e-6:
        valid = False
    gain = jvp_old_times["median_seconds"]-cached_times["median_seconds"]
    crossover = max(1, math.ceil(setup["median_seconds"] / gain)) if gain > 0 else None
    return {
        "schema": "VIGILODE_REI_ORIGINAL26_46080_PHOTON_GROUP_FUSION_V1",
        "original_vigilode_head": "ab52c102cb7d30e1e0f030d05a9a04950001a494",
        "original_rei_owner_head": "fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120",
        "source_sha256": baseline.SOURCE_SHA, "accepted_parent_sha256": baseline.STATE_SHA,
        "kernel_sha256": baseline.KERNEL_SHA, "tick": "160/131072",
        "n": N, "groups": 4, "global_moments": 3, "nonlocal_rank_upper": 3,
        "scope": "original26 stage-0 photo RHS and source-frozen JVP only",
        "algebra": "P_is=n_is*kappa_s; dP_is=kappa_s*dn_is-n_is*sum_t B_st*sum_j dn_jt",
        "owner_included_species_rows": counts, "owner_included_heat_rows": heat_in,
        "source_ledger_relative_residual": ledger,
        "max_relative_rhs_difference_vs_prior_hybrid": diff,
        "max_relative_heat_difference_vs_prior_hybrid": hdiff,
        "max_relative_jvp_difference_vs_prior_hybrid": jdiff,
        "external_rate_difference": abs(new_ext-old_ext),
        "original_heii_f64_underflow_still_unresolved": True,
        "owner_source_scoped_pass": valid,
        "bench_source_old": time_old, "bench_source_fused": time_fusion,
        "bench_jvp_old": jvp_old_times, "bench_jvp_fused": jvp_fusion_times,
        "source_median_speedup": time_old["median_seconds"] / time_fusion["median_seconds"],
        "jvp_median_speedup": jvp_old_times["median_seconds"] / jvp_fusion_times["median_seconds"],
        "bench_cached_jvp_setup": setup, "bench_cached_jvp_apply": cached_times,
        "max_relative_cached_jvp_difference": cached_difference,
        "cached_jvp_repeat_speedup_excluding_setup":
          jvp_old_times["median_seconds"] / cached_times["median_seconds"],
        "cached_jvp_breakeven_calls_estimate": crossover,
        "stage_state_cache_invalidation": "new instance for every changed state, time, source; no persisted cross-step cache",
        "jvp_speedup_integration_status": "NOT_RUN_FULL_NATIVE_RODAS",
        "timing_authority": "single-host exploratory diagnostic; only source operations, no end-to-end IVP",
        "full_26event_ivp": "NOT_RUN", "original_tick161_admitted": False,
        "original_owner_mutated": False, "canonical_delta": 0, "production": "HOLD"
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--owner-root", type=Path, required=True)
    parser.add_argument("--lane", default="LOCAL_NEUTRAL_HAZARD_PRIMARY")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=9)
    args = parser.parse_args()
    if not 3 <= args.repetitions <= 21:
        raise ValueError("REPETITIONS_OUT_OF_BOUNDS")
    result = evaluate(args.owner_root, args.lane, args.repetitions)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2, sort_keys=True, allow_nan=False)+"\n")
    print(json.dumps({key: result[key] for key in
          ["owner_source_scoped_pass", "owner_included_species_rows",
           "owner_included_heat_rows", "source_median_speedup",
           "jvp_median_speedup", "cached_jvp_repeat_speedup_excluding_setup",
           "cached_jvp_breakeven_calls_estimate",
           "max_relative_jvp_difference_vs_prior_hybrid"]}))
    return 0 if result["owner_source_scoped_pass"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
