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
