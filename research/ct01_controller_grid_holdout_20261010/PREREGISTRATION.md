# Preregistration: predictive controller on a fresh interior output grid (CT01)

Node CT01 of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the October 10 re-audit. It is
executed on branch `audit/rvj-controller-grid-ct01-20261010`, which starts from `f5a1a7b` (the head of wave 1). It
depends on:
- AS02, the log-domain predictive factor (merged);
- AS03, the fail-closed evidence validator (merged).

**Claim boundary.**
- Counted attempts (= LU factorizations on the dense fast driver), rejections, error/rtol, and work at matched error.
  No instruction-count or wall-time claim.
- Every arm is opt-in, and the default `Integral` controller is unchanged.
- The seeds are a fixed deterministic corpus of initial steps. They are **not** independent random samples from a
  population, so no statistical-population claim is made.
- A PASS supports "PredictiveCapped2 holds its ALG05 gains and calibration on this corpus with interior outputs". It
  is not a default promotion: the DAG global invariant "no production default promotion in this audit" stands.

## Question

ALG05 (L-0095, superseded in wording by L-0098) measured `PredictiveCapped2` against `I` with endpoint-only output on
three seeds. Its result: FAIL on the absolute single-cell calibration bound, in a cell where the reference arm `I`
itself was at err/rtol 65.

ALG05 could not exercise its two controller fixes, because no run had a clipped output landing. Its gates also mixed
an absolute cell bound with relative statistics.

Under a fresh interior output grid and a fresh, larger fixed corpus of initial steps, does `PredictiveCapped2` pass:
- a predeclared baseline-relative calibration rule,
- a catastrophic-cell guard,
- matched-error work gates,

while the grid exercises clipping, sliver landings and the post-rejection history?

## Arms and change

**Arms.**
- `I`: `Integral`, the production default. This is the reference.
- `PREDcap2`: `PredictiveCapped2`, as merged with the AS02 log-domain factor. This arm is gated.
- `PREDcap`: `PredictiveCapped`. This arm is reported only.

**No controller change.** Controller code is not changed by this node.

**Telemetry.** The only source change allowed is opt-in, default-off telemetry of the controller events below. It must
leave every state, decision and counter bitwise unchanged; a parity test shows this. The telemetry counts per run:
- clipped output landings;
- sliver landings (trial < `CLIPPED_SAMPLE_INFORMATIVE_RATIO` x request);
- sliver landings while a rejection is pending;
- informative clipped landings after a rejection;
- accepted steps with err = 0 while a rejection is pending;
- accepted steps whose next request exceeds the actual trial.

## Cells

- **Problems.** Dense fast driver (`integrate_rodas5p_fast_observed`) on van-der-pol-mu1000, hires, robertson and
  brusselator-1d-50 from the stiff benchmark's `benchmark_problems()`.
- **Configuration.** As ALG05: atol = rtol x atol_scale, min step 1e-14, max step = span, 1,000,000 attempts,
  everything else default except the initial step.
- **Ladders.** As ALG05: quarter decades, 1e-3 to 1e-7 (17 points) for van der Pol and Brusselator-50, and 1e-3 to
  1e-10 (29 points) for HIRES and Robertson.
- **Fresh output grid** (never used by ALG02 or ALG05): 40 uniform interior points t_k = t0 + k (tf - t0)/40,
  k = 1..40, so the last point is tf. The spans are 2000 (van der Pol), 321.8122 (HIRES), 40 (Robertson) and 10
  (Brusselator-50).
- **Fresh gated seed corpus:** h0 in {2e-6, 7e-6, 2e-5, 7e-5, 2e-4, 7e-4}. None of these was used by ALG02
  ({1e-6, 1e-4, 1e-2}) or ALG05 ({3e-6, 3e-5, 3e-3}).
- **Anchor seed: h0 = 3e-6.** This is ALG05's seed, which carries the shared outlier, run on the new grid. It is
  reported only. It is never gated and never excluded.
- **Total:** 92 rungs x 7 seeds x 3 arms = 1932 runs.

**Error metric.** For each run, the grid error is the maximum over the 40 output points of the benchmark metric
`max_i |y_i - r_i| / max(|r_i|, 1e-10)` against a reference trajectory at the same points. The endpoint error (the
same metric at tf only) is also exported, and is reported only.

**References.**
- For each problem, arm `I` is run on the grid with h0 = 1e-6 at rtol 1e-13 (the reference) and at rtol 1e-12 (the
  check), with atol = rtol x atol_scale.
- The reference uncertainty u(problem) is the grid error of the check against the reference.
- A gated cell whose grid error is below 100 u is **reference-limited**. It is excluded from the gated statistics and
  counted.
- The reference endpoint is also compared with the NATIVE.json final state; this comparison is reported.

## Commands

    CT01_RUNS=research/ct01_controller_grid_holdout_20261010/RUNS.json cargo test --offline --locked --release -p rodas5p-cli --test controller_grid_holdout -- --ignored --nocapture --test-threads=1 export_runs
    cargo test --offline --locked -p rodas5p-cli --test controller_grid_holdout
    python3 tools/controller_grid_check.py --runs research/ct01_controller_grid_holdout_20261010/RUNS.json --native research/stiff_native_benchmark_20261001/NATIVE.json --output research/ct01_controller_grid_holdout_20261010/RESULTS.json

Both commands run with `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`.

**RUNS.json contents.** For every run it stores:
- the full state at every grid point, as hex bits;
- counters;
- attempts, accepted and rejected steps;
- the telemetry;
- the grid and endpoint errors.

The references are stored the same way.

**Checker.** The checker calls `tools/evidence_schema_v2.py` first. It recomputes every error from the state bits.
Malformed evidence gives INVALID (exit 2), a gate failure gives FAIL (exit 1), and a pass gives PASS (exit 0). The
checker is committed before the recorded run.

## Scoring

The scoring follows ALG05's definitions in `tools/alg05_controller_v2_check.py`, applied to the grid error.

- **Frontier fit.** Per seed, an ordinary least-squares fit of log(attempts) against log(error) over the whole ladder.
- **Frontier ratio** at an error level E is fit_arm(E) / fit_I(E). The median is taken over the 6 gated seeds.
- **E values.**
  - van der Pol: E in {1e-3, 1e-4, 1e-5, 1e-6, 3e-7}.
  - The other problems: half-decade E inside both arms' measured error ranges for every gated seed.
- **Extrapolation.** An E outside either arm's measured range for some seed is excluded, as in ALG05.

## Gate (arm `PREDcap2` against `I`, gated seeds)

**Validity.** The result is INVALID if any of the following holds:
- The AS03 validator fails.
- The row set is incomplete: any (problem, seed, rtol, arm) is missing or duplicated.
- An error recomputed from the state bits differs from the exported error.
- The telemetry parity test is missing.
- Any successful run has `output_clipped_steps` = 0, which would mean the grid is not active.
- More than 5% of the gated cells are reference-limited.

**PASS** if all of the following hold.

1. **Matched-error work.**
   - van der Pol: frontier ratio <= 0.90 at >= 3 of the 5 E.
   - HIRES, Robertson and Brusselator-50: the worst in-range frontier ratio is <= 1.06.
   - These thresholds are ALG05's items 2-3, replicated.
2. **Worst-seed guard.** For every problem and every in-range E, the maximum over the 6 gated seeds of the per-seed
   frontier ratio is <= 1.20.
3. **Rejections.** The pooled van der Pol rejection fraction of `PREDcap2` is <= 0.5x `I`'s.
4. **Relative calibration.** For each problem:
   - (a) Per seed, fit log10(err) = a + b log10(rtol) over the ladder for each arm. The median over seeds of
     max over the rtol range of fit_PREDcap2 / fit_I must be <= 2.0 (ALG05's item 5a). The maximum is taken at the
     ends of the range.
   - (b) The median over all non-excluded cells of the equal-rtol error ratio err_PREDcap2 / err_I must lie in
     [0.5, 2.0].
5. **Catastrophic-cell guard.**
   - No gated cell has both err_PREDcap2 / rtol > 10 and err_PREDcap2 > 3 err_I in the same cell.
   - No gated cell has `PREDcap2` failing (`success` false) where `I` succeeds.

Everything else is **FAIL**, with every number preserved.

**Kill and hold rules.**
- The ALG05 record, including its shared err/rtol 65 outlier, is never deleted, edited or re-scored.
- A FAIL here is never re-scored as a PASS after a threshold revision.
- The anchor-seed rows are reported in full, including any cell above err/rtol 10 for either arm, with its neighbouring
  seeds.

**Reported, not gated.**
- Cheapest-run ratios.
- Fit residual RMS and the measured error range per arm and seed.
- Equal-rtol error ratio distributions (median, min, max).
- Every cell with err/rtol > 10 for any arm.
- The telemetry totals per arm.
- The number of cells where `PREDcap2` differs from `PREDcap`, and which events precede the first difference.
- Endpoint-error versions of items 1 and 4.
- The anchor seed.
- The reference uncertainty and the NATIVE comparison.

## Prior information (disclosed)

**ALG05 data shaped items 4 and 5.**
- ALG05's largest `PREDcap2` cell was err/rtol 65.17, where `I` gave 65.04 (ratio 1.002).
- ALG05's largest equal-rtol ratio was 9.46 (Robertson, rtol 1e-10, seed 3e-6), at err/rtol 4.01.
- Applied to ALG05's RUNS, the item-5 rule flags 0 cells; this was computed before registration.
- Item 4a is ALG05's item 5a, unchanged; on ALG05 data it gave 0.98-1.56.

**Thresholds set with this knowledge.**
- The thresholds 10 and 3 were chosen knowing the values above. They are meant to catch a controller-caused blow-up
  rather than a hard cell shared with the reference arm.
- The worst-seed bound of 1.20 and the [0.5, 2.0] band of item 4b were set without per-seed or grid data.

**Corpus and grid.** The gated seeds and the grid are fresh. No run of this node exists before this commit.

## Predictions

- Item 1: van der Pol 0.70-0.90. Grid clipping caps large steps, which may dilute the gain. The other problems are at
  1.00-1.05.
- Item 3: about 0.25-0.40x.
- Item 4a: Brusselator-50 about 1.6; item 4b inside [0.8, 1.3].
- Item 5: no catastrophic cell.
- Telemetry: clipped landings in every run, and sliver landings in many runs. `PREDcap2` differs from `PREDcap` in some
  cells, through a sliver landing while a rejection is pending. Accepted steps with err = 0 are rare or absent.
- Overall: probably PASS, with item 2 the least certain.
