# Preregistration: one least-squares solve per cycle in the augmented Arnoldi of LGMRES-into (speed research node SPD05)

Second speed research cycle (2026-10-07), branch `audit/rvj-speed-research-20261005`, base `bdcc903`. Candidate
KRY-AUG-LS-ONCE of the first cycle's DAG (both refuters kept it; their corrections are applied). **Counted allocator
events only; no instruction count and no wall-time claim; the timing authority stays on HOLD.**

## Question

`gmres::arnoldi_augmented_with_workspace` (`gmres.rs:110-187`), which both LGMRES entry points call, resizes and
copies the Hessenberg prefix and calls `small::least_squares` after every Arnoldi column, and uses only the last
solution. L-0061 left exactly this: `solve_lgmres_into` still allocates 455-4,099 times per solve on the rev04
fixtures (failing CDR: 333 = 30 columns x 11 + 3). Does solving the least-squares problem once per cycle, on the same
final prefix and right-hand side, give bitwise the same LGMRES-into results with about 11 allocations per cycle
instead of 11 per column?

## Change (opt-in; legacy GMRES, legacy LGMRES and the default LGMRES-into unchanged)

- `rodas5p-krylov/src/gmres.rs`: a separate crate-private function, a copy of
  `arnoldi_augmented_with_workspace` whose column loop no longer copies the prefix or calls the least-squares solve;
  after the loop (normal end or happy breakdown) it copies the final `(actual + 1) x actual` prefix and solves once
  with the same right-hand side `[beta, 0, ...]`. The legacy function is not edited, so the existing bitwise
  contracts (rnext02, rev04, krylov contracts) are untouched by construction.
- `rodas5p-krylov/src/lgmres_into.rs`: `LgmresIntoWorkspace` gains an opt-in switch (`set_least_squares_once(bool)`,
  default false); `LgmresIntoReport` gains `least_squares_solves` and `inner_iterations` (reported, not part of any
  existing comparison).
- The one possible behavioural deviation: an intermediate least-squares solution that is non-finite no longer aborts
  the solve (only the final one is computed). Such solves are enumerated by the legacy error text; predicted none.

## Systems

The rev04 fixture set unchanged: 6 CDR-120 sequences (6 operators x 8 right-hand sides, carried state, inner 30,
outer k = 8, rtol 1e-9), the Brusselator-50 trajectory set (40 attempts x 8 stages, rtol 1e-11), and both in the
forced-failure configuration (max_outer 1, rtol 1e-14): 1,216 solves, 344 failures. Arms: legacy
`solve_lgmres_with_workspace` (reference), `solve_lgmres_into` default (control), `solve_lgmres_into` with the
switch (candidate); reported, not gated: the candidate together with SPD04's least-squares workspace if SPD04 has run.
New harness `crates/rodas5p-integrators/tests/spd05_lgmres_ls_once.rs`; the frozen rev04 test file is not edited.

## Commands

    SPD05_OUTPUT=research/spd05_lgmres_ls_once_20261007/RESULTS_RAW.json cargo test --release -p rodas5p-integrators --locked --test spd05_lgmres_ls_once -- --ignored --nocapture --test-threads=1
    python3 tools/spd05_ls_once_check.py --raw research/spd05_lgmres_ls_once_20261007/RESULTS_RAW.json --output research/spd05_lgmres_ls_once_20261007/RESULTS.json

## Gate

**PASS** if all hold:

1. **Identity.** The candidate equals the legacy path bitwise on all 1,216 solves (solution bits, residual norm,
   iterations, matvecs, `WorkCounters`, carried state before and after, rollback on failure), except solves the
   legacy path aborted on an intermediate non-finite least-squares solution; each such solve is listed and the
   candidate must then succeed or fail by the final residual rule. The failure kind of every failed legacy solve is
   recorded, so the exception is shown empty rather than assumed.
2. **Output and rollback.** On every failed solve the caller's output is unchanged and both states are rolled back.
3. **Allocations.** Allocations per solve (counted after each sequence's first solve) <= 0.10 x control on every
   non-failing sequence and strictly fewer on every sequence. Predicted per sequence: CDR-120 about 36-110 (from
   1,255-4,059), Brusselator-50 trajectory about 15-50 (from 455.7), failing sets about 14 (from 333).

Reported: least-squares solves and inner iterations per solve, so allocations per cycle and per Arnoldi column are
derivable (the rev04 open question). Otherwise **FAIL**. Kill: any bit difference outside the enumerated deviation,
or any non-failing sequence above 0.5 x control.

## Prior information

L-0061 (LGMRES-into, 0.94-0.99x allocations; the remainder traced to this loop), L-0045 (the GMRES counterpart,
`gmres_into::cycle`). No driver calls `solve_lgmres_into` (the matrix-free driver's LGMRES path uses
`solve_lgmres_with_workspace`); a PASS is an allocation result for the research entry point, and adopting it in the
driver is a separate node. No code of this node exists before this commit.
