# Task 2 report: B1b-gmres (node `b1b-gmres`)

Status: DONE. Kill gate FAIL on both problems (preregistered gate applied as is).
Branch `production/b1b-gmres` (from a5a48f7), worktree /home/claude/wt/b1b-gmres, all commits pushed.
Node report: /home/claude/wt/b1b-gmres/docs/plan/nodes/b1b-gmres.md (preregistration + result).

## Commits (in order)
- 0879a7e docs(plan): preregister the b1b-gmres card before any measurement (pushed before any measurement)
- 5134cd2 test(lagged): b1b-gmres contracts (RED: StageSolver and monitored GMRES absent)
- 8677bb5 feat(lagged): opt-in GMRES stage solver right-preconditioned with the lagged LU (measured commit)
- c8b7e11 docs(plan): b1b-gmres result, kill gate FAIL on HIRES and Brusselator

## What was done
- rodas5p_lagged.rs: `StageSolver::{Richardson (default), Gmres}` in `Rodas5pLaggedOptions::stage_solver`.
  GMRES arm: right-preconditioned by the stored LU, restart 10, x0 = P^-1 c, stops on rho_i tested on the
  explicit WRMS(P^-1 r_k) at k = 0 and after every Arnoldi column; refresh if (nu_k/nu_0)^(1/k) > 0.5 or a
  7th column is needed; on convergence the computed correction P^-1 r_k is applied (as Richardson). Driver id
  `rodas5p-lagged-gmres-v1`; LaggedWork + gmres_iterations, gmres_cycles, residual_applications.
- rodas5p-krylov gmres_into.rs: additive `solve_gmres_into_right_monitored` (+ GmresMonitorDecision,
  GmresMonitoredReport, GmresColumnMonitor). Reason: `gmres_into` tests only the true residual and only at
  cycle boundaries, so it cannot stop between columns or on P^-1 r. Existing entry points untouched.
- rodas5p_fast.rs: additive `Workspace::solve_dense_into(&self, b)` (same arithmetic as `solve`).
- Tests: new tests/b1b_gmres_contracts.rs + shared tests/b1_problems/mod.rs. Two existing option literals
  (b1_lagged_contracts, a1_zero_tolerance_scale) gained `..Default::default()`.

## Tests (all exit 0)
b1b_gmres_contracts 7 passed + 1 ignored (Richardson default pinned bit-for-bit to endpoint bits/counts
printed at 0879a7e on Robertson/HIRES/Brusselator; GMRES forced refresh = fast within 1e-12 on Robertson and
HIRES at rtol 1e-4/1e-6/1e-8; GMRES ledger; HIRES+JVP GMRES iterations > 0 with all W·x via JVP; monitored
GMRES bitwise = solve_gmres_into at last column of a full cycle; abandon/exhaust; left-pc refusal);
b1_lagged_contracts 5+1 ignored; a1_zero_tolerance_scale 9; rodas5p_fast_contracts 4;
rodas5p_fast_allocations 2; int03_banded_fast_contracts 4; lib rodas5p_fast 17; krylov
rnext02_gmres_into_contracts 5. clippy: no warnings in touched files; fmt: touched files clean.

## Measurement (rtol 1e-6, ref = fast at 1e-11, debug build)
| problem | arm | accepted | rejected | fact | fact/acc | err (tol units) |
|---|---|---|---|---|---|---|
| HIRES | fast | 204 | 6 | 210 | 1.029 | 0.2587 |
| HIRES | gmres | 205 | 10 | 15 | 0.073 | 0.9697 |
| Brusselator n=100 | fast | 82 | 10 | 92 | 1.122 | 1.686 |
| Brusselator n=100 | gmres | 980 | 859 | 9 | 0.009 | 19.99 |
Kill gate: HIRES FAIL (error 3.75x > 1.5); Brusselator FAIL (accepted 11.95x > 1.05, error 11.9x > 1.5).
Factorization criterion passes on both. Full counters (iterations, W·x, LU applications, refresh reasons,
wall time) in the node doc.

## Interpretation (I, untested)
GMRES converges for any nonsingular P^-1 W, so the preregistered progress-based refresh rule no longer
bounds P's age (Richardson diverged when q -> 1 and so refreshed). With a stale P, WRMS(P^-1 r) <= rho allows
a stage error up to ||(P^-1 W)^-1|| rho: consistent with the error growth and 859 local-error rejections on
Brusselator. Next card candidates (not tried): bound P's age via a q estimate or h/h_P and Jacobian drift.

## Concerns
1. Measurement invoked twice: first run's console filter dropped the table rows (kill-gate lines kept);
   re-run unchanged to capture tables; kill-gate/refresh lines identical (deterministic). Disclosed in doc.
2. Scope: rodas5p-krylov gained an additive public API (brief named gmres_into only).
3. If a krylov call errors mid-stage (NonFinite op), W·x/LU apps are charged but that stage's columns are
   not added to gmres_iterations (the report is lost), so `operator_applications = gmres_iterations +
   residual_applications` holds only for runs without such errors.
4. Counter convention: an accepted GMRES stage adds 1 to linear_solves (Krylov convention) while each LU
   application adds to direct_solve_calls; Richardson adds both per LU application.
5. GMRES arm wall time is large (565 ms vs 8 ms HIRES; 11.3 s vs 50 ms Brusselator; debug, diagnostic).
Disk: target dir ~0.6 GB; /home/claude 15 GB free.

## Fix round 1 (review Important 1)

Commit: `refactor(krylov): share the Arnoldi column and least-squares helpers in gmres_into` (pushed; SHA in
`git log` of production/b1b-gmres, after c8b7e11).
- crates/rodas5p-krylov/src/gmres_into.rs: the copied column loop and least-squares block (cycle, cycle_right,
  solve_gmres_into_right_monitored) are now private helpers `begin_cycle`, `arnoldi_column` (side Left:
  `apply_left_with_raw`; Right: P^-1 then A; then MGS, Hessenberg, breakdown, normalise/zero) and
  `least_squares_correction` (prefix, beta e_1, reused or allocating LS, V y). `cycle`/`cycle_right` are
  thin wrappers over `run_cycle`. Same operations, same order; no arithmetic change.
- New test `monitored_gmres_matches_gmres_into_over_restarted_cycles_on_a_large_system` (b1b_gmres_contracts):
  60 x 60 non-normal system, right diagonal pc, restart 8, gmres_into converges after >= 2 cycles; the
  monitored iterate accepted at the same column count is bitwise equal, with both the allocating and the
  reused least-squares paths; cycle counts equal.
- Runs (target /home/claude/targets/b1b-gmres, -j 1, --locked): `cargo test -p rodas5p-krylov --no-fail-fast`
  27 targets, 123 passed, 0 failed (incl. rnext02 bitwise test); b1b_gmres_contracts 8 passed + 1 ignored;
  b1_lagged_contracts 5 passed + 1 ignored; clippy (krylov lib) no warnings in gmres_into.rs; fmt clean.
- Measurement not re-run; the lagged driver code is untouched. The node doc's results stand.
