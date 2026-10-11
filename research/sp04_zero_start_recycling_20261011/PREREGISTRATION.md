# Preregistration: zero-start selection separated from recycle-state reuse (SP04)

Node SP04 of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the October 10 re-audit
(`docs/reviews/20261010_accuracy_speed/REVIEW_KO.md`: "zero-start의 별도 이점은 prospective policy 실험으로 검토할
수 있지만, 다른 recycle/preconditioner mode까지 자동으로 적용하지 않는다"). It is executed on branch
`audit/rvj-reaudit-remaining-20261011`, which starts from `0ce7c32`. It depends on AS03, the fail-closed evidence
validator (`tools/evidence_schema_v2.py`), which is merged.

**Claim boundary.**
- Counted work (WorkCounters and the registered cost model below) and same-binary callgrind Ir only. No wall-time
  claim; the timing authority stays on HOLD.
- Every arm is opt-in. No default changes, and no production default is promoted (DAG global invariant).
- Premise: faer 0.24.4 from `Cargo.lock`, run with `RAYON_NUM_THREADS=1`.
- **SPD07's previous-step warm start is not revived.** `InitialGuess::PreviousStep` and `PreviousStepScaled`
  (L-0084, FAIL) are neither run nor proposed in any form.
- **No stale-W claim.** Every stage solve is a solve with the current W = I - h gamma J(t, y) of its attempt. No arm
  applies an operator from an earlier W, and no claim is made about order of accuracy with a stale or frozen W.

## Question

The U-form matrix-free driver (`rodas5p_matrix_free_fast.rs`) mixes two separate decisions.

- **The initial guess x0.** GMRES takes it from `InitialGuess`. SPD07 reported that `Zero` is the cheapest of four
  starts in all 14 cases (L-0084).
- **Solver-carried state.** LGMRES and GCRO-DR keep a `previous_solution` and a recycle or augmentation subspace.
  - With `x0 = None` they start from `x0.or(previous_solution)`. Within one attempt, every stage shares one system
    identity, so a configured `Zero` silently becomes the previous stage's solution for stages 1..s-1.
  - So today no arm runs a true zero start with a carried recycle space. "Zero is cheaper" cannot be transferred to
    recycling solvers without separating the two decisions.

With the two decisions separated, and every reused vector valid for, or recomputed against, the current W: does a
true zero start combined with refreshed GCRO-DR recycling beat the strongest current policy in total counted work,
refresh cost included, at equal accuracy and failures? The test runs on fresh cells with expensive JVPs, near-normal
and nonnormal.

## Change (opt-in; every existing arm unchanged bitwise)

1. **Start policy.** `rodas5p_matrix_free_fast.rs` gets `KrylovStartPolicy { Configured, ExplicitZero }`.
   - `Configured` is today's behaviour, and the default.
   - `ExplicitZero` hands every stage solve an explicit zero vector, a buffer the workspace owns. So
     `previous_solution` is never used as a start, for any method.
   - The carried recycle or augmentation subspace is not touched by the start policy.
2. **Recycle scope.** GCRO-DR and LGMRES get `RecycleScope { Carried, CurrentW }`.
   - `Carried` is today's behaviour.
   - `CurrentW` empties the carried subspace at the start of each attempt, so recycled vectors live only across the s
     stages of one W.
   - The start policy and the recycle scope are fields on the existing driver workspace (setters next to the
     existing `set_gcrodr_policy`), not a new entry point (see "Coordination with sibling nodes"). The result reports
     all three identifiers (start policy, recycle scope, `GcrodrRecyclePolicy`).
3. **Telemetry** (opt-in, default off; it must leave every state, decision and counter bitwise unchanged).
   - Driver: the kind of start of each stage solve (zero, previous stage, solver-carried), and the number of stage
     solves that began with a nonempty carried basis under a new system identity.
   - `gmres_into.rs`: `GmresIntoReport` reports the initial residual norm. This is a report field only, with no
     algorithmic change.
4. **Arms.**
   - Existing arms (the comparator set): `G-Z` GMRES-into `Zero`; `G-P` GMRES-into `Previous` (default start);
     `G-Z2` = `G-Z` with SP01's `ReuseConfirmed` accounting; `R-P` GCRO-DR `RefreshAfterUpdate` `Previous`
     (SAFE-RECYCLE); `C-P` GCRO-DR `Cold` `Previous`; `C-Z` GCRO-DR `Cold` `Zero` (a true zero start: a cold state
     has no previous solution); `Lg-P` GCRO-DR `Legacy` `Previous`; `L-P` LGMRES `Previous`.
   - New arms: `R-Z` = GCRO-DR `RefreshAfterUpdate`, `ExplicitZero`, `Carried` (**the only gated candidate**);
     `R-Zw` = `R-Z` with `CurrentW` scope (reported); `L-Z` = LGMRES, `ExplicitZero`, `Carried` (reported).
   - **LGMRES path.** `L-P` and `L-Z` use the existing driver LGMRES path (`solve_lgmres_with_workspace`, with SP02's
     option at its default `krylov-ws-legacy-v1`), so they differ only in the start policy, whatever SP02's outcome.

## Cells (prospectively registered; never run by SAFE-RECYCLE, SPD07, ALG01 or ALG04)

**Problems.** Three problems built with `semilinear_advection_diffusion_problem(n, d, a, r, nl, 0)` on [0, 1],
JVP-only clone, with an exact solution. The JVP is a dense n x n product (2n^2 flops), so it is expensive by
construction: 256 times one length-n inner product.
- `E-normal`: (256, 0.05, 0.0, -1.0, 0.5). The operator is symmetric.
- `E-nonnormal`: (256, 0.005, 2.0, -1.0, 0.5). Cell Peclet a dx / d = 1.56.
- `E-nonnormal-strong`: (256, 0.001, 2.0, -1.0, 0.5). Cell Peclet 7.8.

**Tolerances.** rtol 1e-6 and 1e-8, atol = 1e-2 rtol. These are 6 cells.

**Configuration.**
- Linear rtol 1e-10, atol 1e-14, restart 40, maxiter 200, recycle_dim 8, inner_m 30, outer_k 8, no preconditioner.
- 5,000 attempts, initial step 1e-6.
- n = 256 is far above recycle_dim 8. On the small SAFE-RECYCLE problems (n <= 8) the recycle space spans R^n, which
  makes those cells degenerate.

**Error.** The endpoint error is max_i |y_i - y*_i| / max_i |y*_i| against the exact solution, recomputed by the
checker from the state bits.

**Base export.** Before any source change of this node, `export_base` runs every comparator arm on the 6 cells at
this registration commit: `BASE.json`.
- `BASE.json` records its source commit (BASE). `git diff BASE..RUNS -- crates/*/src`, with RUNS the source commit
  recorded in `RUNS.json`, touches only the registered edit targets (`rodas5p_matrix_free_fast.rs` and
  `gmres_into.rs`).
- A cell is **feasible** if at least one comparator arm succeeds with zero linear-solve failures.
- Infeasible cells are excluded and reported.
- More than 2 infeasible cells puts the node on **HOLD** (no verdict), recorded in the ledger as INCONCLUSIVE. New
  parameters would then need a new registration, and `BASE.json` is kept.

## Registered cost model (a hand count, a design choice; the Ir check below guards it)

C = F_jvp jvp_vectors + F_rhs rhs_evaluations + 2n (orthogonalization_inner_products +
orthogonalization_vector_updates)

- F_jvp = 2n^2 + 7n and F_rhs = 2n^2 + 8n, counted from the problem's code.
- `jvp_vectors` includes the diagnostic and the recycle-refresh products. The checker verifies the identity
  `jvp_vectors = linear_matvecs + diagnostic_matvecs + recycle_refresh_matvecs` in every row. It holds in all 42
  SAFE-RECYCLE rows.
- Small dense work (harmonic Ritz and least squares) is not in C. It is covered by Ir.

**S\*, the strongest current policy of a cell.** Among the comparator arms that succeed with zero linear-solve
failures, S\* is the arm with minimal C. The checker selects it after the run.

## Commands

    SP04_BASE=research/sp04_zero_start_recycling_20261011/BASE.json cargo test --offline --locked --release -p rodas5p-integrators --test zero_start_policy_holdout -- --ignored --nocapture --test-threads=1 export_base
    SP04_RUNS=research/sp04_zero_start_recycling_20261011/RUNS.json cargo test --offline --locked --release -p rodas5p-integrators --test zero_start_policy_holdout -- --ignored --nocapture --test-threads=1 export_runs
    cargo test --offline --locked -p rodas5p-integrators --test zero_start_policy_holdout
    python3 tools/sp04_zero_start_profile.py --runs research/sp04_zero_start_recycling_20261011/RUNS.json --output research/sp04_zero_start_recycling_20261011/PROFILE.json
    python3 tools/sp04_zero_start_check.py --base research/sp04_zero_start_recycling_20261011/BASE.json --runs research/sp04_zero_start_recycling_20261011/RUNS.json --profile research/sp04_zero_start_recycling_20261011/PROFILE.json --output research/sp04_zero_start_recycling_20261011/RESULTS.json

All commands run with `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`. The two exports are the DAG command, filtered.

**Profiling.** Callgrind on one release example (`examples/sp04_zero_start_profile.rs`). Ir per trajectory = 2 minus
1 integrations, plus a determinism repeat. It profiles `R-Z` and that cell's S\* on the three rtol-1e-6 cells.

**Checker.**
- `tools/sp04_zero_start_check.py` calls `tools/evidence_schema_v2.py` first.
- INVALID exits 2, FAIL exits 1, PASS exits 0.
- The checker and its mutation tests are committed before `export_runs` is recorded.

## Gate

**Validity.** The result is INVALID (exit 2, never FAIL) if AS03 validation fails; a (cell, arm) row is missing or
duplicated; an error recomputed from the state bits differs from the exported one; the JVP identity fails in any row;
a callgrind repeat is not identical; `Cargo.lock` does not pin faer 0.24.4; `BASE.json` is not bound to its
SHA-256 (recorded in the commit that adds it, before any source change of this node); `BASE.json` lacks its source
commit, or `git diff BASE..RUNS -- crates/*/src` touches a file outside the registered edit targets; or the checker
blob at the RUNS source commit differs from the checker that was run, or any recorded tree status is dirty.

**PASS** if all of the following hold, on every feasible cell.

1. **Existing arms unchanged.** Every comparator arm in RUNS equals `BASE.json` bitwise (times, state bits,
   attempts, counters). The default entry point equals `G-P`.
2. **Separation contract.** These are contract tests together with the telemetry.
   - Under `ExplicitZero`, the start telemetry shows zero starts equal to `linear_solves`, with no previous-stage and
     no solver-carried starts.
   - `ExplicitZero` with GMRES-into equals `G-Z` bitwise, and `ExplicitZero` with GCRO-DR `Cold` equals `C-Z`
     bitwise.
   - The telemetry-on run equals the telemetry-off run bitwise.
3. **Current-W contract.**
   - A carried GCRO-DR pair from W1 used under W2 has its images recomputed against W2: bitwise the fresh products,
     charged as `recycle_refresh_matvecs`.
   - Under `CurrentW`, the carried subspace is empty at every attempt start.
   - In every `R-Z` row, `recycle_update_refreshes` equals `recycle_updates`, as SAFE-RECYCLE G2 required.
4. **Equal accuracy and failures.** `R-Z` succeeds with zero linear-solve failures, an error <= 1.5x S\*'s, and
   `|attempts(R-Z) - attempts(S*)| <= max(1, ceil(0.02 attempts(S*)))`.
5. **Total work gain.** C(`R-Z`) <= 0.90 C(S\*), refresh products included.
6. **No hidden cost.** On the three profiled cells, Ir(`R-Z`) <= 1.00x Ir(S\*).

Everything else is **FAIL**, with every number preserved.

**Kill and hold rules.**
- Any silent reuse across a W change, uncharged refresh work, or a changed default is a FAIL regardless of item 5.
- If C(`R-Z`) >= C(S\*) on every feasible cell, the zero-start x refresh-recycling hypothesis is killed for this
  driver. It is not re-tuned (recycle_dim, restart, cells) under this registration.
- L-0084, L-0066 and L-0059 are not rerun or edited. A FAIL is never re-scored after a threshold change.
- A PASS supports an explicit opt-in policy on expensive-JVP cells of this kind only. It promotes no default and
  says nothing about cheap-JVP problems.

**Reported, not gated:**
- `R-Zw`, `L-Z`, and every comparator's C, JVPs, orthogonalization, refresh products, harmonic Ritz solves and Ir
  shares.
- `G-P` / `G-Z` on the new cells, as a replication of SPD07's direction.
- The GMRES-into initial-residual ratios.
- Cheap-JVP sensitivity: C rescored with F_jvp = 5n and F_rhs = 8n, the cost a tridiagonal implementation of the
  same operator would have.

## Coordination with sibling nodes

AS04, SP02, SP04 and PC01 all edit `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs`. Their changes are
merged in the fixed order AS04 -> SP02 -> SP04 -> PC01, each as orthogonal opt-in fields, default off, with no new
entry point per node where a field on the existing options or workspace suffices. SP04's contract tests include the
earlier nodes' options-off identity: with AS04's telemetry off and SP02's option at `krylov-ws-legacy-v1`, every
comparator arm equals `BASE.json` (item 1). `L-Z` uses the existing LGMRES path, as stated under Arms. When the SP02
option is on together with `CurrentW`, a rejected attempt first restores the carried `LgmresState` from SP02's
snapshot, and only then is the subspace emptied at the next attempt start; the restore never sees an emptied state.

## Prior information (disclosed)

**SPD07 (L-0084).** `Previous` costs 1.02-1.57x the matvecs of `Zero` with GMRES-into on all 14 SAFE-RECYCLE cases.
GCRO-DR `Cold`: `Zero` / `Previous` JVPs are 0.74-0.98.

**SAFE-RECYCLE (L-0066) runs.** `R-P` / `G-Z` JVPs are:
- 1.07 and 1.07 on Brusselator-50 (1e-6, 1e-8);
- 1.27 and 1.34 on Brusselator-160;
- with 1.8-2.3x the orthogonalization inner products.

On the small problems the ratio is 0.42-1.07. Those cells are degenerate (recycle_dim >= n).

**L-0059.** Refresh costs 1.08-1.32x cold GCRO-DR's operator products on Brusselator trajectories: "recycling saves
nothing here".

**How the thresholds were set.** These data set the 0.90 bound: a gain smaller than 10% would not pay for the extra
carried state. The bound, the 1.5x error band and the cost-model weights are design choices. The new cells' results
are unknown, and no code or run of this node exists before this commit.

## Predictions

- Items 1-3 hold. Item 4 holds (errors dominated by time discretization).
- Item 5 probably **FAILS**: S\* is expected to be `G-Z2` or `C-Z`, and `R-Z` at 0.95-1.15x S\*. The zero start
  should remove the extra previous-start matvecs, but refresh products and the larger orthogonalization are not
  expected to be repaid at n = 256. A gain, if any, is likeliest on `E-normal`; extra iterations are likeliest on
  `E-nonnormal-strong`.
- `R-Zw` near `C-Z`; `L-Z` below `L-P`. Overall: FAIL more likely than PASS.
