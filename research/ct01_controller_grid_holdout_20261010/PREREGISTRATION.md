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

## Results

Everything above this heading is the registered text, unchanged. Executed on branch `ct01/impl` from `a45b8e5` (the
registration commit, a child of `f5a1a7b`); the registered text names `audit/rvj-controller-grid-ct01-20261010` as
the branch. Only the branch name differs; the content and the start point do not.

**Verdict: PASS** (evidence VALID). `PREDcap2` passes all five gate items against `I` on the six gated seeds. On this
driver, `PREDcap2` produced exactly the same output as `PREDcap` in all 644 cells, so the PASS holds for both arms. See
"PREDcap2 vs PREDcap" below. The claim boundary stands: the result covers counted attempts on a fixed deterministic
corpus of initial steps. These are not random samples, there is no wall-time or population claim, and nothing is
promoted to a default.

### Record

| Item | Commit | SHA-256 |
|---|---|---|
| Registration (text above) | `a45b8e5` | — |
| A: telemetry, parity tests, export test | `59f7fbe` | `controller_telemetry.rs`, `rodas5p_fast.rs`, `controller_grid_holdout.rs`, `ct01_controller_telemetry_parity.rs`: full digests below |
| B: checker and checker tests (before the run) | `088187b` | `controller_grid_check.py` `a905db7d7de32b4494b67d413357e6cfeea2279468f556f8c73b5b7f3bc6c972`; `test_controller_grid_check.py` `47da3a58834af65caa8a542fa6a26a152328e2654c31bd105c748fc9f3fd6054` |
| C: recorded run at `088187b` (clean tree) | `51948a9` | `RUNS.json` `b7c7b3655f19f1855cd1b0681ff918cd93c975afbbf08743f253711e61b07a38` (38,445,964 bytes, compact JSON); `RESULTS.json` `b2b4a849a4d7cd4d5b6036ce7d3b143f54a2f59367565dacf49c3240264c249c` |
| Inputs, unchanged | — | `NATIVE.json` `5064b638b411fb46089b336a351325a4dbc12acfeacdff2edb226aa80e8fd1ec` (AS03 pin); `alg05_controller_v2_check.py` `f5411366…f70e53a`; `evidence_schema_v2.py` `1d2bfe92…06ff10` |

Full digests:
- `crates/rodas5p-integrators/src/controller_telemetry.rs`: `fd343d09556c16bc3fe0868adae7702301002ee9b705ee3209e046d15610749d`
- `crates/rodas5p-integrators/src/rodas5p_fast.rs`: `911a740b89b9558ec124fa3d5da851ab02ee4d144b523a9279d0ff772c254651`
- `crates/rodas5p-cli/tests/controller_grid_holdout.rs`: `45604652f93e3ad156dfd34577338e868296db17677f81b836f0b399c9e7c5f1`
- `crates/rodas5p-cli/tests/ct01_controller_telemetry_parity.rs`: `7278d3f8f2c65f7775db98aeaf4a39a6de9abca146226403a859c545d05c4d1f`
- `tools/alg05_controller_v2_check.py`: `f5411366ace440a2b9df14791a7836f0702325d161f05886fa1fba076970e53a`
- `tools/evidence_schema_v2.py`: `1d2bfe9276252b2079914e4fa958612ed06ffb04d1fecf9016bd4f370206ff10`

**How the run was made.** The three registered commands were run with `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`,
using rustc and cargo 1.94.1 and Python 3.11.15.
- `export_runs` took 3.2 s of test time in release.
- The non-ignored `controller_grid_holdout` tests passed 13 of 13; this count includes the included benchmark module's
  own tests.
- The checker exited 0.

A second export from the same commit, written to a scratch path, was byte-identical to `RUNS.json`.

**Tests at commit A.**
- `cargo fmt` and `cargo clippy --workspace --all-targets -D warnings` are clean.
- `cargo test -p rodas5p-cli -p rodas5p-integrators --all-targets` passed: 167 test-result groups (unit, integration and doc tests), 0 failures.

**Tests at commit B.** `python3 -m unittest tools.test_controller_grid_check` passed 21 tests, both normally and with
`-O`:
- one valid fixture that gives PASS;
- 12 INVALID fixtures;
- 6 FAIL fixtures (one per item, two for item 5);
- the anchor-never-gated test;
- the CLI exit-code and immutable-output test.

No bug was found after the recorded run. There is no `*_FIRST.json` file and nothing was re-run for the record.

### Validity

All registered conditions hold:
- The AS03 rules pass (strict JSON and the NATIVE pin; the telemetry is bound to its run).
- The row set is 1932 of 1932, with no duplicates.
- Every grid error and endpoint error, the four reference uncertainties and the four NATIVE differences, recomputed
  from the state bits, equal the exported values.
- Both parity test files and all three registered parity tests are present.
- Every successful run and every reference has clipped landings. The smallest `output_clipped_steps` of any
  successful run is 37.
- All 1932 runs succeeded.

Reference-limited cells: **12 of 552 gated cells (2.17%; the limit is 5%).** All 12 are HIRES at rtol 1e-10 and
1.78e-10, for every gated seed. There the grid error is 1.04e-10 to 2.22e-10, below 100 u = 3.13e-10. These cells are
excluded from the gated statistics. The gate items computed *without* the exclusion also all pass
(`gate_items_without_reference_limited_exclusion`).

| Problem | u (check rtol 1e-12 against reference rtol 1e-13, grid) | 100 u | Reference endpoint against NATIVE final state | NATIVE's own uncertainty | Reference / check attempts |
|---|---|---|---|---|---|
| van der Pol | 4.64e-11 | 4.64e-9 | 9.07e-12 | 3.76e-13 | 14214 / 7990 |
| HIRES | 3.13e-12 | 3.13e-10 | 2.89e-12 | 1.63e-13 | 16930 / 9645 |
| Robertson | 3.64e-12 | 3.64e-10 | 3.57e-13 | 1.24e-15 | 6199 / 2729 |
| Brusselator-50 | 1.27e-11 | 1.27e-9 | 1.35e-13 | 1.50e-13 | 1873 / 1191 |

### Gate (PREDcap2 against I, 6 gated seeds, grid error)

| Item | Rule | Measured | Result |
|---|---|---|---|
| 1 van der Pol | ratio <= 0.90 at >= 3 of 5 E | E=1e-3: **0.801**; 1e-4: **0.831**; 1e-5: **0.863** (all in range). Excluded as extrapolated: 1e-6 (0.894), 3e-7 (0.910). 3 passing of 3 in-range E | pass |
| 1 HIRES | worst in-range half-decade ratio <= 1.06 | worst **0.989** (12 E, 3.2e-4 to 1e-9; geometric mean 0.985) | pass |
| 1 Robertson | same | worst **1.017** at E=1e-9 (8 E, 3.2e-6 to 1e-9; geometric mean 1.008) | pass |
| 1 Brusselator-50 | same | worst **1.002** at 3.2e-4 (5 E, 3.2e-4 to 3.2e-6; geometric mean 0.983) | pass |
| 2 worst seed | max over seeds of per-seed ratio <= 1.20 at every in-range E | van der Pol 0.867; HIRES 0.998; Robertson **1.020** (seed 7e-4, E=1e-9); Brusselator-50 1.016 | pass |
| 3 rejections | pooled van der Pol fraction of PREDcap2 <= 0.5 x I's | I 9269/38486 = 0.2408; PREDcap2 2201/32241 = 0.0683; **ratio 0.283** | pass |
| 4a calibration fit | median over seeds of max over the rtol range of fit ratio <= 2.0 | van der Pol 1.021; HIRES 0.913; Robertson 1.048; Brusselator-50 1.003 (largest per-seed value 1.184, van der Pol, seed 2e-6) | pass |
| 4b equal-rtol median | median of err_P2 / err_I in [0.5, 2.0] | van der Pol 1.000 (n=102); HIRES 0.990 (n=162); Robertson 1.000 (n=174); Brusselator-50 0.994 (n=102) | pass |
| 5 catastrophic cell | none with err_P2/rtol > 10 and err_P2 > 3 err_I; none with P2 failing where I succeeds | 0 cells; 0 failures | pass |

Two facts limit the van der Pol E values:
- The van der Pol grid error never falls below about 4.7e-6 on the ladder. The smallest measured grid error is
  4.74e-6 for PREDcap2 and 4.91e-6 for I, at rtol 1e-7.
- Because of that, two of the five registered E values lie outside the measured range. They are excluded, as in
  ALG05, and item 1 passes on exactly the three remaining E values.

### Reported, not gated

**Cheapest-run ratios (PREDcap2/I).**
- van der Pol: 0.708, 0.789, 0.926 at E = 1e-3, 1e-4, 1e-5.
- HIRES: geometric mean 0.982, worst 1.192.
- Robertson: geometric mean 1.007, worst 1.015.
- Brusselator-50: geometric mean 0.973, worst 1.008.

**Fit residual RMS and measured error range per arm and seed.** Full values are in
`reported.fit_diagnostics`.

| | Frontier RMS (decades), I | Frontier RMS, PREDcap2 | Calibration RMS, I | Calibration RMS, PREDcap2 |
|---|---|---|---|---|
| van der Pol | 0.048–0.049 | 0.054–0.059 | 0.298 | 0.266–0.302 |
| HIRES | 0.084–0.086 | 0.090–0.093 | 0.139–0.141 | 0.165–0.166 |
| Robertson | 0.093–0.098 | 0.092–0.096 | 0.074–0.105 | 0.068–0.094 |
| Brusselator-50 | 0.030–0.041 | 0.021–0.040 | 0.155–0.181 | 0.162–0.213 |

Measured grid-error ranges (I and PREDcap2):
- van der Pol: 4.7e-6 to 0.135.
- HIRES: 1.0e-10 to 5.0e-4.
- Robertson: 5.7e-10 to 1.3e-5.
- Brusselator-50: 1.5e-6 to 7.5e-4.

**Equal-rtol error ratios** (gated, non-excluded cells), PREDcap2/I, given as median [min, max]:
- van der Pol: 1.000 [0.506, 2.235].
- HIRES: 0.990 [0.541, 1.504].
- Robertson: 1.000 [0.569, 1.714].
- Brusselator-50: 0.994 [0.361, 1.964].

**Pooled rejection fractions** (gated seeds):

| Arm | van der Pol | HIRES | Robertson | Brusselator-50 |
|---|---|---|---|---|
| I | 0.241 | 0.0111 | 0.0205 | 0.0635 |
| PREDcap2 | 0.068 | 0.0066 | 0.0177 | 0.0386 |

**Cells with grid err/rtol > 10 for any arm.** There are 154 cells, all listed with every arm and both neighbouring
seeds in `reported.cells_err_over_rtol_above_10`. Of these, 132 are gated cells and 22 are anchor cells.
- **van der Pol: 105 cells**, 15 rungs (rtol 1e-7 to 1e-3) x 7 seeds.
  - In 103 cells, all three arms exceed 10.
  - In the other 2 cells (rtol 3.16e-4, seeds 2e-5 and 7e-4), only I exceeds 10: I 18.38, PREDcap2 9.30.
  - Gated maxima: 135.49 for every arm, at rtol 1e-3 (the same grid error to 7 digits).
  - PREDcap2/I in these gated cells: 0.506 to 1.387.
- **Brusselator-50: 49 cells**, rtol 1e-7 to 3.16e-6, all seeds, all three arms above 10.
  - Gated maxima: PREDcap2 17.98, I 19.79.
  - PREDcap2/I: 0.824 to 1.152.
- HIRES and Robertson have no cells above 10.

The van der Pol median grid err/rtol is 29.1 for I and 28.0 for PREDcap2. The Brusselator-50 median is 7.37 for I and
7.39 for PREDcap2. On these two problems the grid metric sits well above rtol for every controller; the cause is not
the predictive controller. ALG05's absolute bound (err/rtol <= 10 in every cell) would have failed all three arms here.

**Telemetry totals** (all 7 seeds, 644 runs per arm):

| Event | I | PREDcap | PREDcap2 |
|---|---|---|---|
| clipped landings | 25102 | 25051 | 25051 |
| sliver landings | 16582 | 16364 | 16364 |
| sliver landings while a rejection is pending | 0 | 0 | 0 |
| informative clipped landings after a rejection | 0 | 0 | 0 |
| accepted err = 0 while a rejection is pending | 0 | 0 | 0 |
| accepted steps whose next request exceeds the trial | 122100 | 120102 | 120102 |
| controller updates (= attempts) | 217778 | 211473 | 211473 |
| runs with at least one clipped landing / sliver landing | 644 / 644 | 644 / 644 | 644 / 644 |

Per-problem totals are in `reported.telemetry_totals`.

**PREDcap2 vs PREDcap.** The two arms differ in **0 of 644 cells**. This covers every exported field: grid states,
counters, attempts, rejections and telemetry. The in-memory decision traces agree as well: `first_difference_from_PREDcap`
is null in every PREDcap2 row, and there are 0 row-vs-trace inconsistencies. So there is no first difference, and no
events precede one.

The cause (interpretation, from the code, consistent with the zero counts above):
- After a rejected or failed attempt, the dense fast driver retries from the same time with a strictly shorter step:
  factor <= 0.9, and `adaptive_end_step`'s strict-shortening rule.
- The rejected trial ended at or before the next output point, so the retry ends strictly before it. The landing
  extension only covers a rounding residue, and an extended step is not counted as clipped.
- So the first accepted attempt after a rejection is never a clipped landing, and it clears the pending flag.
- Sliver landings while a rejection is pending, and informative clipped landings after a rejection, therefore cannot
  occur on this driver, whatever the output grid.
- That leaves err = 0 while pending as PREDcap2's only possible point of difference, and it did not occur.

Consequence: ALG05's two `PredictiveCapped2` fixes were still **not exercised**, even though the grid was active in
every run (25051 clipped and 16364 sliver landings for PREDcap2). The gated PASS is therefore equally a PASS of
`PredictiveCapped`; PREDcap's numbers on every item are identical (`reported.PREDcap_against_the_items`).

**Endpoint-error versions of items 1 and 4** (endpoint error against the grid reference at tf; the same excluded
cells). All would pass:
- Item 1, van der Pol: E=1e-3: 0.736 (extrapolated); 1e-4: 0.777; 1e-5: 0.820; 1e-6: 0.866; 3e-7: 0.892.
- Item 1, worst ratios: HIRES 0.995, Robertson 1.013, Brusselator-50 0.988.
- Item 4a/4b: van der Pol 1.237/1.078; HIRES 0.956/0.844; Robertson 1.024/1.000; Brusselator-50 0.982/0.999.

**Anchor seed h0 = 3e-6** (reported only; never gated and never excluded). All 276 anchor rows are in `RUNS.json`, and
all 92 anchor cells, with errors, attempts and rejections for every arm, are in `reported.anchor_seed`. Anchor-only
statistics, PREDcap2/I:

| Problem | Frontier ratios (in-range E) | Calibration fit ratio | Equal-rtol median / max | Rejection fraction (I / PREDcap2) |
|---|---|---|---|---|
| van der Pol | 0.808, 0.836, 0.864 at 1e-3..1e-5 | 1.053 | 1.000 / 1.327 | 0.240 / 0.069 |
| HIRES | 0.977..0.986 | 0.938 | 0.982 / 1.503 | 0.0087 / 0.0051 |
| Robertson | 0.983..1.028 | 1.080 | 1.000 / 1.298 | 0.0152 / 0.0156 |
| Brusselator-50 | 0.963..0.970 | 0.985 | 0.992 / 1.254 | 0.064 / 0.038 |

These anchor statistics lie in the same ranges as the gated seeds.

ALG05's outlier cell (van der Pol, rtol 5.62e-4, h0 3e-6, endpoint err/rtol 65 for both arms against NATIVE) does not
recur on the grid. The rows for that cell, given as grid err/rtol, endpoint err/rtol and attempts:

| Seed | I | PREDcap2 |
|---|---|---|
| 3e-6 (anchor) | 4.44, 0.17, 214 | 4.44, 0.039, 173 |
| 2e-6 (neighbour) | 4.44, 0.17, 214 | 9.93, 0.48, 175 |
| 7e-6 (neighbour) | 4.44, 0.17, 214 | 6.70, 0.48, 169 |

The interior landings change the step sequence before tf. The ALG05 record is not altered or re-scored.

The 22 anchor cells above err/rtol 10, all shared with I, with both neighbouring seeds:

| Problem | rtol | I err/rtol (attempts, rejections) | PREDcap2 err/rtol (attempts, rejections) | PREDcap2/I | Seed 2e-6: I / P2 | Seed 7e-6: I / P2 |
|---|---|---|---|---|---|---|
| brusselator-1d-50 | 1e-07 | 18.08 (151, 7) | 14.89 (151, 4) | 0.824 | 18.10 / 14.93 | 18.12 / 14.94 |
| brusselator-1d-50 | 1.78e-07 | 18.11 (142, 8) | 16.56 (138, 3) | 0.914 | 18.24 / 16.53 | 18.22 / 16.58 |
| brusselator-1d-50 | 3.16e-07 | 18.74 (128, 9) | 17.24 (125, 4) | 0.920 | 18.62 / 17.09 | 18.70 / 17.09 |
| brusselator-1d-50 | 5.62e-07 | 19.67 (117, 8) | 17.96 (115, 6) | 0.913 | 19.67 / 17.98 | 19.56 / 17.83 |
| brusselator-1d-50 | 1e-06 | 15.52 (104, 8) | 17.82 (101, 3) | 1.148 | 15.51 / 17.81 | 15.54 / 17.77 |
| brusselator-1d-50 | 1.78e-06 | 15.06 (99, 8) | 15.45 (97, 5) | 1.026 | 15.13 / 15.57 | 15.21 / 15.56 |
| brusselator-1d-50 | 3.16e-06 | 10.86 (93, 8) | 11.17 (88, 3) | 1.029 | 10.96 / 11.34 | 10.95 / 11.47 |
| van-der-pol-mu1000 | 1e-07 | 49.13 (670, 96) | 47.37 (615, 18) | 0.964 | 49.14 / 47.38 | 49.18 / 47.64 |
| van-der-pol-mu1000 | 1.78e-07 | 45.75 (594, 87) | 46.29 (550, 19) | 1.012 | 45.80 / 46.34 | 45.74 / 46.28 |
| van-der-pol-mu1000 | 3.16e-07 | 42.44 (548, 92) | 44.89 (492, 16) | 1.058 | 42.44 / 44.88 | 42.45 / 44.89 |
| van-der-pol-mu1000 | 5.62e-07 | 46.92 (496, 90) | 41.77 (443, 18) | 0.890 | 46.94 / 41.78 | 46.92 / 42.55 |
| van-der-pol-mu1000 | 1e-06 | 35.93 (460, 95) | 35.08 (400, 19) | 0.977 | 35.93 / 34.94 | 35.93 / 33.88 |
| van-der-pol-mu1000 | 1.78e-06 | 31.54 (454, 122) | 31.06 (362, 19) | 0.985 | 31.54 / 31.24 | 31.53 / 35.54 |
| van-der-pol-mu1000 | 3.16e-06 | 29.12 (419, 117) | 26.80 (332, 20) | 0.920 | 29.12 / 30.23 | 29.12 / 27.78 |
| van-der-pol-mu1000 | 5.62e-06 | 31.38 (389, 115) | 28.30 (298, 18) | 0.902 | 31.38 / 27.96 | 31.38 / 27.96 |
| van-der-pol-mu1000 | 1e-05 | 20.51 (357, 109) | 26.46 (278, 23) | 1.290 | 20.51 / 22.13 | 20.51 / 22.44 |
| van-der-pol-mu1000 | 1.78e-05 | 19.17 (336, 107) | 19.44 (259, 28) | 1.014 | 19.17 / 19.35 | 19.17 / 19.44 |
| van-der-pol-mu1000 | 3.16e-05 | 14.57 (307, 96) | 14.73 (240, 27) | 1.011 | 14.57 / 14.73 | 14.57 / 14.78 |
| van-der-pol-mu1000 | 5.62e-05 | 19.17 (275, 82) | 16.97 (217, 23) | 0.886 | 19.17 / 11.47 | 19.17 / 16.87 |
| van-der-pol-mu1000 | 1e-04 | 10.52 (263, 82) | 13.96 (202, 22) | 1.327 | 10.52 / 14.19 | 10.52 / 14.59 |
| van-der-pol-mu1000 | 3.16e-04 | 18.38 (222, 64) | 18.38 (185, 27) | 1.000 | 18.38 / 18.38 | 18.38 / 18.38 |
| van-der-pol-mu1000 | 1e-03 | 135.49 (205, 63) | 135.49 (163, 22) | 1.000 | 135.49 / 135.49 | 135.49 / 135.49 |

Every anchor cell above 10 is above 10 for I as well. The largest anchor PREDcap2/I ratio among them is 1.327, at van
der Pol rtol 1e-4. The neighbouring seeds show the same pattern, so the anchor seed is not special on this grid.

### Prediction check

| Prediction | Observed | Held? |
|---|---|---|
| Item 1: van der Pol 0.70–0.90 | 0.801 / 0.831 / 0.863 (in range); 0.894 / 0.910 at the two excluded E | yes |
| Item 1: others 1.00–1.05 | HIRES worst 0.989 (a gain), Robertson 1.017, Brusselator-50 1.002 | partly (HIRES better than predicted) |
| Item 3: about 0.25–0.40x | 0.283 | yes |
| Item 4a: Brusselator-50 about 1.6 | 1.003 | no (closer to 1) |
| Item 4b in [0.8, 1.3] | 0.990–1.000 | yes |
| Item 5: no catastrophic cell | none | yes |
| Telemetry: clipped landings in every run | every run (at least 37) | yes |
| Telemetry: sliver landings in many runs | every run | yes |
| PREDcap2 differs from PREDcap in some cells, through a sliver landing while pending | 0 cells; 0 such events (structural, see above) | **no** |
| Accepted err = 0 rare or absent | absent | yes |
| Overall probably PASS, item 2 least certain | PASS; item 2 worst 1.020 against 1.20 | yes |

### Interpretations and deviations

These were fixed in commits A and B, before the run, unless stated otherwise.
1. **AS03 validator.** `tools/evidence_schema_v2.py` has no CT01 kind and was left unchanged. The checker calls its
   primitives first: `_load_all` with the NATIVE pin, the strict loader, `_Checks`, `_rows` raw-duplicate detection,
   `require_metric_pair` and `emit_invalid` (exit 2). The checker adds one binding rule in the AS03 spirit: telemetry
   clipped landings must equal `output_clipped_steps`, and updates must equal attempts.
2. **Reference-limited cell.** A gated cell (problem, gated seed, rtol) is reference-limited when the smaller successful
   grid error of the two compared arms, I and PREDcap2, is below 100 u. The cell is then dropped for both arms from
   every gated statistic, including item 3's pooling and item 5's error clause; item 5's failure clause still covers
   it. The unexcluded items are reported and also pass.
3. **Telemetry definitions.**
   - "Rejection pending" is the observer's own flag, using the PredictiveCapped2 rule (set by every rejected or failed
     attempt; cleared by an accepted non-sliver step). It is applied under every kind so the arms compare. Under
     PREDcap2 the parity test shows it equals the controller's `rejection_pending()` after every update.
   - "After a rejection" is read as "while a rejection is pending".
   - Events are counted on accepted attempts.
4. **Endpoint error.** The endpoint error is measured against the grid reference at tf, not against NATIVE; the
   reference-to-NATIVE difference is reported separately. The grid is t_k = t0 + (k (tf - t0)) / 40, and the last
   point equals tf exactly (asserted).
5. **Driver.** The export uses `integrate_rodas5p_fast_observed_with_telemetry`, which is
   `integrate_rodas5p_fast_observed` with the observer on. Both parity test files show the two are bitwise identical.
6. **Extra exported field.** `RUNS.json` is compact JSON. Each PREDcap2 row also carries
   `first_difference_from_PREDcap`, computed from in-memory decision traces; it is reported only.
7. **Seeds.** The seeds are a fixed deterministic corpus. The per-seed spread is narrow; for example, the van der Pol
   per-seed frontier ratios lie within 0.79–0.87 at the in-range E. This describes the corpus only and is not a
   population estimate.
