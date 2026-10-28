# Preregistration: predictive step-size control with a post-rejection cap (ALG02)

Algorithmic-directions cycle, Tier 1 node N2 (`docs/reviews/20261008_algorithmic_directions/ALGORITHMIC_DIRECTIONS.md`,
sections 2.3 and 4). Branch `audit/rvj-algorithmic-directions-20261008`, base `a793ecd`.

**Claim boundary.** The node measures counted work (attempts, which equal LU factorizations on the dense driver, plus
RHS calls and JVPs) and endpoint accuracy. It makes no instruction-count or wall-time claim; the timing authority
stays on HOLD. The new controller is opt-in: `ControllerKind::Integral` and `ControllerKind::Pi` keep their arithmetic
bit for bit.

## Question

The production integral controller rejects 15-37 % of van der Pol attempts at loose tolerances (Hairer's RODAS:
1-14 %). After a rejection it may grow the step 5x (F-078, open). The exploratory replica (probe B5, pilot
`phase3_reports/ctrl.md`) found that Hairer's predictive (Gustafsson) form does three things:

- it cuts rejections 3-4x;
- it reduces work at matched accuracy (van der Pol 0.68-0.84, HIRES and Brusselator 0.93, Robertson about 1.01);
- its gain is not a set-point effect.

Does the Rust implementation reproduce this, and does it shift the error calibration at equal rtol (the pilot's
Robertson 1e-10 warning)?

## Change (opt-in)

- **New controller kinds.** `ControllerKind::{Predictive, PredictiveCapped}` in `adaptive.rs`. The controller state
  additionally remembers the last accepted step size and whether the previous attempt was rejected. The new fields
  are skipped in serialization, so serialized state does not change.
- **On an accepted, non-clipped step** (k = estimator order 5, err > 0):
  - `f_I = clamp(safety err^(-1/k), min, max)`.
  - If a previous accepted step exists, also compute
    `f_P = clamp(safety (h / h_acc) (err_acc / err^2)^(1/k), min, max)` and use `f = min(f_I, f_P)`. Otherwise
    `f = f_I`. Here `err_acc = max(1e-2, err_prev)`.
  - `PredictiveCapped` additionally takes `f = min(f, 1)` when the previous attempt was rejected or failed.
  - Then `h_acc <- h` and `err_prev <- err`.
- **Clipped output landings** update `(h_acc, err_acc)` only for informative samples, the existing
  `CLIPPED_SAMPLE_INFORMATIVE_RATIO` rule.
- **err = 0, rejections, and non-finite or linear failures** are unchanged from the production rule.
- **Drivers.** Every driver that calls the shared controller update (dense fast, small-n, banded, matrix-free U form)
  gets the new kinds through `AdaptiveStepConfig::controller`. No driver default changes.

## Cells

- **Problems.** The stiff benchmark problems (`rodas5p-cli` `benchmark_problems()`): van-der-pol-mu1000, HIRES,
  Robertson, brusselator-1d-50, with their recorded atol rule.
  - Reference: `research/stiff_native_benchmark_20261001/NATIVE.json`.
  - Error metric: `max_i |y_i - r_i| / max(|r_i|, 1e-10)`, as in the benchmark.
- **Driver.** The dense fast driver (`integrate_rodas5p_fast_observed`).
- **Ladders.**
  - van der Pol and Brusselator-50: rtol 1e-3 to 1e-7 in quarter decades (17 points).
  - HIRES and Robertson: 1e-3 to 1e-10 (29 points).
- **Initial-step seeds.** h0 in {1e-6, 1e-4, 1e-2}.
- **Arms.**
  - `I`: production Integral.
  - `I725`: Integral with safety 0.725, the set-point rival.
  - `PRED`: `Predictive`.
  - `PREDcap`: `PredictiveCapped`, the gated arm.
- **Reported matrix-free cells.** The U-form driver (Legacy stage target, GMRES into, zero start) on
  brusselator-1d-50 and brusselator-1d-160, ladder 1e-3 to 1e-7 in half decades, seeds as above. References: dense
  fast driver at rtol 1e-13.

## Base export (before any source change)

A test-only commit adds the export. Its `export_base` runs arm `I` on all dense cells on the unmodified source
(`BASE.json`).

## Commands

    ALG02_BASE=research/alg02_predictive_controller_20261008/BASE.json cargo test --release -p rodas5p-cli --locked --test alg02_predictive_controller -- --ignored --nocapture --test-threads=1 export_base
    ALG02_RUNS=research/alg02_predictive_controller_20261008/RUNS.json cargo test --release -p rodas5p-cli --locked --test alg02_predictive_controller -- --ignored --nocapture --test-threads=1 export_runs
    python3 tools/alg02_controller_check.py --base .../BASE.json --runs .../RUNS.json --native research/stiff_native_benchmark_20261001/NATIVE.json --output research/alg02_predictive_controller_20261008/RESULTS.json

## Scoring

Each rule is applied per seed, and ratios are the median over seeds.

- **Frontier (R).** An ordinary least-squares fit of log(attempts) against log(error) over the whole ladder, evaluated
  at E.
- **Cheapest run (C).** The fewest attempts among runs with error <= E.
- **Evaluation points.**
  - van der Pol: E in {1e-3, 1e-4, 1e-5, 1e-6, 3e-7}.
  - Other problems: half-decade E inside both arms' measured error ranges. E outside either range is flagged as
    extrapolated and excluded.

## Gate (arm `PREDcap` against `I`)

**PASS** if all of the following hold.

1. **Identity.** `I` in RUNS reproduces BASE bit for bit.
2. **van der Pol gain.** R ratio <= 0.90 at >= 3 of the 5 van der Pol E.
3. **No regression.** On HIRES, Robertson and Brusselator-50: the worst in-range R ratio <= 1.06, and the worst
   in-range C ratio <= 1.10.
4. **Rejections.** The pooled van der Pol rejection fraction (all rungs and seeds) is <= 0.5x `I`'s.
5. **Calibration.** In every dense cell and seed: `err(PREDcap) / rtol <= max(2.0, 1.5 err(I) / rtol)`.

Everything else is **FAIL**, with every ratio preserved.

**Reported, not gated.**
- `PRED`, `I725`, and the factorial reading (does `PREDcap` beat `I725` by >= 3 % in geometric-mean R on HIRES or
  Brusselator-50?).
- The matrix-free cells.
- Per-cell equal-rtol error ratios.

**Predictions** (pilot).
- Gate 2: 5 of 5, R ratios 0.68, 0.72, 0.76, 0.81, 0.84.
- Gate 3: HIRES worst about 0.99, Robertson about 1.03, Brusselator about 0.95.
- Gate 4: 28.6 % -> about 7.1 %.
- Gate 5: **uncertain.** The pilot's per-cell equal-rtol ratios range from 0.34 to 9.48, and the largest are at
  Robertson 1e-10. The item may fail; a failure means the controller shifts the tolerance calibration and needs its
  own treatment before adoption.
- Matrix-free Brusselator-160: about 0.91 in JVPs.

## Prior information

- Pilot: `docs/reviews/20261008_algorithmic_directions/pilot/phase3_reports/ctrl.md` and `stack.md` (section 5,
  noise).
- Audit F-078.
- `research/stiff_native_benchmark_20261001` (rejection rates).
- No code of this node exists before this commit.

## Results

Appended after the recorded run; the registered text above is unchanged. Branch `alg/alg02-controller` from
`b0d5ea3` (the registration commit of ALG01-ALG03).

**Verdict: FAIL** (items 3 and 5). Items 1, 2 and 4 pass. Counted work only; no wall-time claim.

### Record

| commit | content |
|---|---|
| `f2094d0` | test only: `crates/rodas5p-cli/tests/alg02_predictive_controller.rs` with the dense cells and `export_base` |
| `46d115f` | `BASE.json`: arm `I` on the 276 dense cells, run on `f2094d0` (unmodified solver source) |
| `02bdf0a` | `ControllerKind::{Predictive, PredictiveCapped}` in `adaptive.rs`, with unit tests |
| `8c348f9` | `export_runs` (four dense arms, matrix-free cells) and `tools/alg02_controller_check.py` |
| `cae9486` | `RUNS.json` (run on `8c348f9`, clean tree) and `RESULTS.json` |

Commands (with `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`):

    ALG02_BASE=research/alg02_predictive_controller_20261008/BASE.json cargo test --release -p rodas5p-cli --locked --test alg02_predictive_controller -- --ignored --nocapture --test-threads=1 export_base
    ALG02_RUNS=research/alg02_predictive_controller_20261008/RUNS.json cargo test --release -p rodas5p-cli --locked --test alg02_predictive_controller -- --ignored --nocapture --test-threads=1 export_runs
    python3 tools/alg02_controller_check.py --base research/alg02_predictive_controller_20261008/BASE.json --runs research/alg02_predictive_controller_20261008/RUNS.json --native research/stiff_native_benchmark_20261001/NATIVE.json --output research/alg02_predictive_controller_20261008/RESULTS.json

SHA-256: `BASE.json` `bc3630ef…d224825`, `RUNS.json` `2d5d1331…d7e0`, `RESULTS.json` `38329e7c…c023` (full digests in
`RESULTS.json` `inputs` and the commit log). This is the first and only recorded run; nothing was changed after it.

Checks at `02bdf0a`: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`
and `cargo test -p rodas5p-integrators -p rodas5p-cli --all-targets --locked` (575 passed, 0 failed, 46 ignored).

### Base

All 276 dense runs complete; LU factorizations equal attempts in every run. van der Pol at rtol 1e-3, seed 1e-6, gives
181/114/67 attempts/accepted/rejected, the recorded Rust counters the pilot's replica missed by one. Pooled rejection
fractions of `I`: van der Pol 28.5 %, HIRES 1.5 %, Robertson 3.4 %, Brusselator-50 13.7 %. Every endpoint error the
checker recomputed from the state bits equals the exported one.

### Gate (`PREDcap` against `I`)

| item | measured | threshold | result |
|---|---|---|---|
| 1. identity | 276 of 276 `I` rows of RUNS equal BASE | all | pass |
| 2. van der Pol gain, R | 0.682, 0.724, 0.770, 0.818, 0.839 at E = 1e-3, 1e-4, 1e-5, 1e-6, 3e-7 (none extrapolated) | <= 0.90 at >= 3 of 5 | pass (5 of 5) |
| 3. no regression, HIRES | worst R 1.007 (E 1e-11), worst C 1.025 (E 1e-7); 15 in-range E | R <= 1.06, C <= 1.10 | pass |
| 3. no regression, Robertson | worst R 1.039 (E 1e-10), worst C **1.160** (E 3.16e-9); 12 in-range E | R <= 1.06, C <= 1.10 | **FAIL** |
| 3. no regression, Brusselator-50 | worst R 0.955, worst C 0.992; 5 in-range E | R <= 1.06, C <= 1.10 | pass |
| 4. rejections | van der Pol pooled 7.02 % against 28.45 % (0.247x) | <= 0.5x | pass |
| 5. calibration | 3 of 276 cells exceed the bound: van der Pol rtol 5.62e-7, all three seeds, err/rtol 2.552, 2.567, 2.566 against bounds 2.282, 2.049, 2.048 | every cell | **FAIL** |

van der Pol C ratios: 0.735, 0.623, 0.737, 0.942, 0.814. Geometric means over the in-range E (R / C): van der Pol
0.765 / 0.763, HIRES 0.950 / 0.964, Robertson 1.013 / 1.013, Brusselator-50 0.935 / 0.931.

**Item 3.** The Robertson failure is the cheapest-run rule at one E. At E = 3.16e-9, `I`'s rtol-5.62e-9 rung lands
just inside on all three seeds (error 3.08e-9). `PREDcap`'s lands just outside on seeds 1e-6 and 1e-2 (3.19e-9 and
3.21e-9), so its cheapest run there is the next rung: 137/118 and 138/119 attempts (seed 1e-4: 117/116). The two arms
are within 3 attempts of each other on every Robertson rung and seed; the frontier rule gives 1.025 at that E. The
registered rule stands, and the item fails.

**Item 5.** At van der Pol rtol 5.62e-7 the predictive kinds reach err/rtol 2.55-2.57 on all three seeds (`PRED` gives
the same states), against 1.37-1.52 for `I`. It is a seed-independent shift on one rung, not noise. Elsewhere 16 of 276
`PREDcap` cells have err/rtol > 2 (31 for `I`). The pilot's Robertson 1e-10 warning is reproduced but stays inside the
bound: equal-rtol error ratios `PREDcap`/`I` there are 9.48, 1.91 and 3.36 (err/rtol 1.80, 0.71, 0.64).

### Reported, not gated

Pooled rejection fractions:

| arm | van der Pol | HIRES | Robertson | Brusselator-50 |
|---|---|---|---|---|
| I | 28.5 % | 1.50 % | 3.39 % | 13.7 % |
| I725 | 7.8 % | 0.32 % | 1.44 % | 2.6 % |
| PRED | 7.05 % | 0.73 % | 3.09 % | 6.09 % |
| PREDcap | 7.02 % | 0.67 % | 3.03 % | 6.08 % |

Geometric-mean R / C over in-range E, and worst in-range R:

| ratio | van der Pol | HIRES | Robertson | Brusselator-50 |
|---|---|---|---|---|
| PRED / I | 0.761 / 0.764 | 0.946 / 0.966, worst 1.005 | 1.021 / 1.001, worst 1.051 | 0.937 / 0.931 |
| I725 / I | 0.831 / 0.849 (1e-3 extrapolated) | 1.036 / 1.058, worst 1.099 | 1.037 / 0.995, worst 1.225 | 0.936 / 0.925 |
| PREDcap / I725 | 0.949 / 0.929 | 0.922 / 0.913 | 0.970 / 1.037, worst 1.130 | 0.999 / 1.013 |
| PREDcap / PRED | 0.994 / 0.997 | 0.997 / 0.998 | 0.990 / 1.016 | 1.001 / 1.000 |

- **Factorial reading:** yes, on HIRES only. `PREDcap` beats `I725` by 7.8 % in gm R on HIRES (0.922) and not on
  Brusselator-50 (0.999).
- **Equal-rtol error ratios to `I`** (median [min, max] over cells): `PREDcap` van der Pol 0.995 [0.27, 4.59], HIRES
  0.974 [0.14, 4.14], Robertson 0.993 [0.65, 9.48], Brusselator-50 1.225 [0.78, 2.17]. Median err/rtol `I` against
  `PREDcap`: 1.67/1.71, 0.126/0.096, 0.547/0.568, 1.03/1.26.
- **Ladder attempt totals** (3 seeds): van der Pol 18,947 (`I`) against 14,936 (`PREDcap`); HIRES 57,150 against
  57,219; Robertson 8,668 against 8,684; Brusselator-50 3,655 against 3,404.
- **Matrix-free** (U form, Legacy target through `gmres_into`, zero start; work in JVPs, `counters.jvp_vectors`; all
  108 runs complete):
  - Brusselator-160: gm R 0.903, gm C 0.908 (worst R 0.959); rejections 17.2 % -> 8.9 %; linear-solve failures 144 ->
    81; ladder JVPs 1,523,696 -> 1,374,298. The pilot predicted about 0.91.
  - Brusselator-50: gm R 0.930, gm C 0.948 (worst R 0.941); rejections 13.1 % -> 6.0 %; no linear-solve failures.
  - References: the dense fast driver at rtol 1e-13 (1,852 and 1,860 attempts, no rejections).

All pilot predictions for items 2, 3 (R) and 4 and for the matrix-free cell hold within 0.01-0.02. The pilot did not
predict the Robertson cheapest-run edge or the van der Pol 5.6e-7 calibration cell.

### Interpretations and deviations

- **Extrapolated van der Pol E** would not count toward item 2 (the registered "flagged and excluded" read for all E).
  No van der Pol E was extrapolated for `PREDcap` against `I`, so this had no effect.
- **Half-decade grid:** E = 10^(-j/2). In range means inside both arms' measured error ranges for every seed.
- **`I725`** sets `AdaptiveStepConfig::safety = 0.725`. In Rust the safety also scales rejection proposals (clamped to
  0.9); the pilot's replica applied 0.725 to accepted proposals only. `I725` is reported, not gated.
- **The previous-attempt flag** is the existing `last_rejected_trial` (already `#[serde(skip)]`), read before it is
  overwritten. The only new state field is `h_acc` (`#[serde(skip)]`), kept only by the predictive kinds.
- **err = 0** on an accepted step gives `max_factor` with no cap (the production rule, as registered). The step enters
  the history.
- **History-free `propose_factor`** has no step sizes, so for the new kinds it returns the integral factor `f_I`. The
  predictive term and the cap act only through `adaptive_next_step_after_attempt`, the shared update every listed driver
  calls. `integrate_adaptive` always uses `Integral`. The fused exponential driver calls `propose_factor` directly, so
  under the new kinds it gets `f_I`.
- **Problem sources.** The CLI crate is binary-only, so the test includes `src/stiff_benchmark.rs` by `#[path]` for
  `benchmark_problems()`. Its own unit tests also run in this test binary. The matrix-free cells use the JVP
  Brusselator of `crates/rodas5p-integrators/tests/rnext_common` (the SPD07 fixture, same equations), also by `#[path]`.
  `benchmark_problems()` has no JVP and no 160-cell problem.
- **Matrix-free linear configuration:** SPD07's `gmres_into` `Zero` arm (GMRES, rtol 1e-10, atol 1e-14, restart 40,
  maxiter 200, no preconditioner). Adaptive configuration as the dense cells, atol scale 1.

## Correction after the independent review (appended 2026-10-08; no number or verdict changes)

An independent reviewer re-ran the checker; its output equals the committed RESULTS.json, and the verdict stays **FAIL**. Two behaviours of `PredictiveCapped` are literal readings of the registration that a follow-up node should change:

- **err = 0 bypasses the cap.** An accepted step with err = 0 right after a rejection still gets the maximum factor 5, because the registration keeps err = 0 unchanged. This is exactly the F-078 growth case, and the pilot's controller capped it.
- **A sliver landing consumes the rejection flag.** After a rejection, an accepted sliver landing clears the "previous attempt rejected" flag, so the next full step can grow (2.26x in the reviewer's probe). This matches Hairer's REJECT reset.

The factorial reading ("PREDcap beats I725 on HIRES only") depends on `I725` also scaling rejection proposals by 0.725, which is disclosed above. The pilot applied 0.725 to accepted steps only.
