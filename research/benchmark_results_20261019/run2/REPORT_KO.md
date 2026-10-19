# VigilODE second benchmark, 2026-10-19 (Korean report)

**Evidence classes:** `PRIMARY_CSV` = original 2,700-row result file and metrics recomputed from it. `PRIOR_REPORT_ONLY` = an earlier benchmark report in the conversation whose complete receipts are no longer present in this workspace; preserved as an attributed historical observation, **not** newly revalidated.

## Source identity and scope

- Working branch: `claude/nice-dijkstra-208xq0`.
- **Measured Rust source/binary commit:** `8a8bd409b1d96c7b7421f496d229e119fd3d284e`.
- Previous measurement reference: `ccaeaa50beb4580505a36d25ccb919ec3ca38c7f`.
- Later checked branch tip: `a21d573980f66fe02891bd216153443b5352f921`. Its subsequent Python harness edits do not authorize relabeling the binary as a fresh a21d573 build.
- 12 selected problems, 20 arm names (subject to applicability), rtol 1e-4/1e-6/1e-8, output grids endpoint/uniform101, and h0 scales 0.5/1/2.
- The observed 2,700 rows are a **bounded subset** of the preregistered 142-component program, not the full triage campaign.
- Each row contains status, per-grid empirical reference error metrics (`E_up`, `R_up`), accepted/rejected steps and emitted work counters. It does **not** contain wall-clock replicate receipts or Callgrind/allocator event logs.

## PRIMARY_CSV: observed status and requested-grid error

| Result | Count | Denominator |
|---|---:|---:|
| All attempted cells | 2700 | 2700 |
| `status=ok` | 2639 | 2700 |
| `status=refused` | 54 | 2700 |
| `status=solver_failure` | 7 | 2700 |
| Successful cells with `R_up <= 1` | 2329 | 2639 |
| Successful cells with `R_up <= 10` | 2580 | 2639 |
| Successful cells with `R_up <= 100` | 2626 | 2639 |

Thus **88.25% of successful cells** met the observed requested-grid `R_up <= 1` threshold, but only **86.26% of all attempted cells** both completed and met that threshold. The first percentage does not conceal 61 non-successful cells.

The output-grid split is:

| Grid | Successful | `R_up<=1` | Observed maximum `R_up` |
|---|---:|---:|---:|
| endpoint | 1320 | 1240 | 14.062535 |
| uniform101 | 1319 | 1089 | 1751.990413 |

The `fast-ctl-pi` arm has `144/144` successful and `R_up<=1` cases **within its selected and applicable tested subset**; maximum `R_up` is `0.8860318531740459`. This is no global accuracy or speed guarantee.

### Nonlinear Prothero-Robinson dense-output counterexample

All three rows below use `prothero-robinson-nl`, `uniform101`, `rtol=1e-8`, `h0_scale=1` and are marked `ok`:

| Arm | `R_up` | Accepted / rejected |
|---|---:|---:|
| `seq-direct` (step at output times) | `6.632710004483773e-7` | 106 / 0 |
| `seq-direct-dense-off` | `1751.990412875421` | 11 / 0 |
| `seq-direct-dense-enforce` | `1.0933030660056111` | 21 / 23 |

The dense-enforce route drastically reduced the sampled error but **did not reach `R_up<=1` in this cell**. Solver success, local estimator acceptance and actual requested-grid accuracy are different contracts. `R_up` incorporates the campaign's empirical independent-reference radius; it is **not** a mathematically certified supremum over all time.

The seven `solver_failure` rows and 54 `refused` rows remain in the original CSV. Inspect `error`, `problem`, `arm`, `rtol`, `grid` and `h0_scale` for their exact machine-readable identity rather than treating non-success as zero-cost or excluding it silently.

## PRIOR_REPORT_ONLY: measurements not recoverable from the currently mounted artifacts

The prior detailed run reported the following, but its corresponding raw receipts and instrument files are **not** included here. These facts should not be used as independent promotion evidence without recovery or rerunning.

- **Old/new paired timing:** 120 fresh processes, 10 paired blocks, 154 old-vs-new configurations. At the reported endpoint/output observable level, the two versions' outputs and six work counters agreed; among 22 aggregated problem/arm groups, 7 had a 95% interval fully inside ±5%, none had an interval entirely indicating >5% gain/loss, and 15 remained unresolved. Shared-VM host clock was not admitted as authoritative.
- **Same-achieved-error timing in Run 2:** for the 400-state Brusselator with the reported `1e-5` endpoint target, previous conversational results were base `387.745482 ms`, fast `26.418277 ms`, banded-slices `7.335145 ms`. These are within-version arm contrasts, **not** commit-to-commit speedups. Raw paired time samples are not present.
- **Allocation and instruction probes:** the prior report described 84 allocation conditions and 14 Callgrind conditions. At the n=96 advection-diffusion example, `mf` versus `mf-into` showed allocations `373558` versus `11922`, cumulative allocated bytes `610386479` versus `42976927`, window peak live bytes `246576` versus `243000`, and warm Callgrind Ir `5899710427` versus `1769147834`. These are distinct units, not interchangeable time or memory-speedup measures.
- **Latest Python harness caveats:** prior verification reported mass applicability omissions, a partial-state-length acceptance defect, and schema mismatches. The current CSV does not independently test these Python tool defects. Treat each as a reported test finding requiring the saved failing fixture/patch and exact HEAD to confirm closure.

## Current verdict and publication limits

1. `PRIMARY_CSV` is sufficient to substantiate the bounded 2,700-cell status and measured output-grid accuracy summaries above.
2. It does not substantiate the old/new paired timing CI or the allocation/Callgrind results. They are retained as historical context with an explicit weaker provenance label.
3. No global accuracy theorem or physical-client certification follows from these samples. Do not PROMOTE or PRUNE all 142 registered components from this bounded dataset.
4. No broad performance nonregression or regression claim is justified from the missing raw paired timings and unadmitted shared-host clock.
5. Preserve `refused`, `solver_failure`, and historical FAIL outcomes. Subsequent improvements require new measured receipts and new commit identities.

Primary artifact: [ct_results_2700.csv](ct_results_2700.csv). SHA-256 and source policy: [../README.md](../README.md). For scoped validation run `python3 research/benchmark_results_20261019/verify_csv.py`.
