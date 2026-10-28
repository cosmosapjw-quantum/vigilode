# Preregistration: stage-indexed warm start from the previous accepted step in the matrix-free driver (speed research node SPD07)

Second speed research cycle (2026-10-07), branch `audit/rvj-speed-research-20261005`, base `bdcc903`. Candidate
KRY-STEP-X0 of the first cycle's DAG (both refuters kept it; their corrections are applied). **Counted Krylov work
only; no instruction count and no wall-time claim; the timing authority stays on HOLD.** The matrix-free U-form
driver is a research driver; a PASS changes the counted work of that opt-in driver, not of any default path.

## Question

In the matrix-free U-form fast driver (`rodas5p_matrix_free_fast.rs`) with `InitialGuess::Previous`, stage `i`
starts from `u_{i-1}` of the current attempt and stage 0 from zero (GMRES; GCRO-DR cold). The Krylov stopping
threshold is `max(atol, rtol ||b||)`, absolute in `||b||`, so a start whose residual is `k` decades below `||b||`
saves the iterations of those decades. Does a stage-indexed start, `u_i` of the last accepted step, reduce the
Krylov work per trajectory without changing the accepted solutions beyond the solver tolerance?

## Change (opt-in; `Zero` and `Previous` unchanged)

- `rodas5p-core/src/solver_types.rs`: `InitialGuess::{PreviousStep, PreviousStepScaled}`.
- The driver keeps the stage vectors `u` of the last accepted step (one `s n` copy per accepted step; rejected
  attempts leave it untouched). `PreviousStep`: stage `i` starts from the stored `u_i`. `PreviousStepScaled`
  (reported only): the same, multiplied by `h / h_prev`. Before the first accepted step both behave as `Previous`.
  A nonzero start at stage 0 costs one true-residual matvec that a zero start skips; it is counted in
  `linear_matvecs` and is part of the metric.
- Solver-internal warm starts are untouched: LGMRES and GCRO-DR's legacy and refresh policies keep their own
  `previous_solution`, so this node's arms are GMRES `solve_into` (gated) and GCRO-DR cold (reported) only.

## Base export (recorded before any source change)

`crates/rodas5p-integrators/tests/spd07_mf_step_warm_start.rs::export_base`, run on the unmodified source at the
registration commit: the matrix-free driver with GMRES `solve_into` (restart 40, linear rtol 1e-10, atol 1e-14, no
preconditioner) and `x0 = Zero` and `Previous`, and GCRO-DR cold with `Previous`, on the SAFE-RECYCLE problem set
(robertson, van-der-pol-mu1000, hires, prothero-robinson-forced, quadratic-4, brusselator-1d-50 (n = 100),
brusselator-1d-160 (n = 320)) at rtol 1e-6 and 1e-8: `BASE.json`. A case whose base `Previous` run does not succeed
is excluded from gate item 2 and reported.

## Systems and metrics

The same 7 problems x 2 tolerances, arms `Zero`, `Previous`, `PreviousStep`, `PreviousStepScaled`, for GMRES
`solve_into` and GCRO-DR cold. Per trajectory: attempts, accepted and rejected, `linear_solves`,
`linear_iterations`, `linear_matvecs`, orthogonalization inner products and vector updates, linear-solve failures,
least-squares solves (cycles) for GMRES-into, and the final error against a reference run of the dense fast driver
at rtol 1e-12.

## Commands

    SPD07_BASE=research/spd07_mf_step_warm_start_20261007/BASE.json cargo test --release -p rodas5p-integrators --locked --test spd07_mf_step_warm_start -- --ignored --nocapture --test-threads=1 export_base
    SPD07_RUNS=research/spd07_mf_step_warm_start_20261007/RUNS.json cargo test --release -p rodas5p-integrators --locked --test spd07_mf_step_warm_start -- --ignored --nocapture --test-threads=1 export_runs
    python3 tools/spd07_warm_start_check.py --base ...BASE.json --runs ...RUNS.json --output research/spd07_mf_step_warm_start_20261007/RESULTS.json

## Gate (GMRES `solve_into`)

**PASS** if all hold:

1. **Base reproduction.** The `Zero` and `Previous` arms (and the GCRO-DR cold `Previous` arm) reproduce `BASE.json`
   bitwise (output states, attempts, counters).
2. **Fewer matvecs.** On brusselator-1d-50 and brusselator-1d-160 at both tolerances, `PreviousStep`'s
   `linear_matvecs` per trajectory <= 0.85 x `Previous`'s and <= 0.90 x `Zero`'s (the second separates a better
   predictor from merely removing a harmful start), with attempts within max(1, 2 %), zero linear-solve failures and
   a final error <= 2 x `Previous`'s.
3. **No regression.** On no problem and tolerance (14 cases) does `PreviousStep` exceed 1.02 x `Previous`'s
   `linear_matvecs`.

Everything else is **FAIL**, with the measured ratios preserved (the band 0.85-0.97, where the recorded F-013
warm-start data lie, is a FAIL). `PreviousStepScaled` and all GCRO-DR cold arms are reported, never gated.
Predicted: 0.75-0.90x on the Brusselators, about 1.0x on the small problems.

## Prior information

L-0066 (SAFE-RECYCLE policies), L-0045 (GMRES `solve_into`), audit F-013 (warm-start data 0.905x/0.94x). No code of
this node exists before this commit; the base export test file is added in the commit after this one and runs on the
unmodified solver source.

## Results (appended after the run; source commit recorded in the ledger row)

Base export: `BASE.json` from the registration commit's solver source (test file and export added in `2280537`,
no solver source changed). Recorded run on `0c731b0`: `RUNS.json` (release build, `RAYON_NUM_THREADS=1
OPENBLAS_NUM_THREADS=1`), `RESULTS.json` from `tools/spd07_warm_start_check.py`. Contract tests (3) passed.

**Gate: FAIL.**

| Gate item | Outcome |
|---|---|
| 1. Base reproduction | **holds**: GMRES-into `Zero` and `Previous` and GCRO-DR cold `Previous` equal `BASE.json` on all 14 cases (times, final state bits, attempts, counters) |
| 2. Fewer matvecs on the Brusselators | **fails**: `PreviousStep` / `Previous` linear matvecs = 0.986 (bruss-50, 1e-6), 1.001 (bruss-50, 1e-8), 0.971 (bruss-160, 1e-6), 0.963 (bruss-160, 1e-8); gate <= 0.85. Against `Zero`: 1.025, 1.025, 1.035, 1.021; gate <= 0.90. Attempts equal, no linear-solve failures, final errors equal to 2 digits |
| 3. No regression | **fails**: `PreviousStep` / `Previous` exceeds 1.02 on Robertson (1.040, 1.045), van der Pol (1.032, 1.032) and Prothero-Robinson at 1e-8 (1.029) |

The prediction (0.75-0.90 on the Brusselators) was wrong. Iterations per stage solve fall by at most 4 % (bruss-160,
1e-8: 50.6 -> 48.5; restart cycles 1,952 -> 1,873), so the last accepted step's stages are not a much better start than
the previous stage of the same attempt. On the small problems a stage solve takes 1-7 iterations and the stage-0
true-residual matvec of a nonzero start is a visible part of the cost.

Reported, not gated (all from `RESULTS.json`):

- `PreviousStepScaled` / `Previous`: 0.963-1.038, the same picture (bruss-160 0.973 and 0.963).
- GCRO-DR cold: `PreviousStep` / `Previous` 0.962-1.062, the same picture.
- **The default start `Previous` costs more matvecs than `Zero` in all 14 cases** of this driver with GMRES-into:
  `Previous` / `Zero` = 1.024-1.066 on the Brusselators, 1.11 on HIRES, 1.22-1.29 on Robertson, van der Pol and
  quadratic-4, 1.51-1.57 on Prothero-Robinson, with equal attempts and equal final errors to 2 digits. Zero is the
  cheapest of the four starts everywhere. This was not a question of this node and is not a claim about any other
  driver; switching the default would be its own node (with GCRO-DR and LGMRES, whose warm start interacts with the
  recycle space, measured separately).

Claim ceiling: counted Krylov work of the opt-in matrix-free U-form research driver on the 7 SAFE-RECYCLE problems at
two tolerances; no wall-time claim. Per-stage iteration and `||r0|| / ||b||` records (suggested by the refuters, not
registered) were not recorded; only per-trajectory counters and GMRES-into cycles are.

## Corrections and changes after the run (2026-10-07, from the independent review; appended)

- `PreviousStepScaled` / `Previous` ranges 0.958-1.038, not 0.963-1.038 (minimum: Prothero-Robinson at 1e-6).
- The ledger claim (L-0084) names the regressions on Robertson and van der Pol; Prothero-Robinson at 1e-8 (1.029)
  also exceeds 1.02, as the gate table above says.
- The checker fails gate item 2 if any Brusselator case were excluded, while the registration says to exclude and
  report it; stricter than registered, and no case was excluded.
- Code (after the run, `c26a235`): the doc comment of `InitialGuess::PreviousStep` claimed every other stage
  solver refuses it; in fact the sequential stage solver refuses it and the matrix-free workspace honours it only
  when accepted steps are recorded, which its driver does and a caller of `attempt` must do itself. A reused
  workspace also kept the previous integration's stages; the driver now clears the record at the start of each
  integration (`clear_accepted_step`). The recorded runs build a fresh workspace per integration and are unaffected.
  The test `step_indexed_starts_are_used` only asserted that the work changes; a new contract test re-attempts the
  recorded system and requires zero Krylov iterations with every stage unchanged (a wrong stage index fails it),
  checks that clearing restores the `Previous` start and that the scaled start differs after a step change.
No verdict and no gated number changes.
