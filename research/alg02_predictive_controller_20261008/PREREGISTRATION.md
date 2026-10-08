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
