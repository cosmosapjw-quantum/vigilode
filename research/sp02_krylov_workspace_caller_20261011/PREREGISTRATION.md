# Preregistration: Krylov workspace and LGMRES-into wired into a real driver caller (SP02)

Node SP02 of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the October 10 re-audit
(`docs/reviews/20261010_accuracy_speed/REVIEW_KO.md`, section "이미 관측된 비용 개선의 활용"). It is executed on
branch `audit/rvj-reaudit-remaining-20261011`, which starts from `0ce7c32`. It depends on AS03, the fail-closed
evidence validator (`tools/evidence_schema_v2.py`), which is merged.

**Claim boundary.**
- Allocator events, peak live heap bytes, charged workspace bytes, same-binary callgrind instructions (Ir) and
  WorkCounters only. No wall-time claim; the timing authority stays on HOLD.
- Every new path is opt-in. No default changes, and no production default is promoted (DAG global invariant).
- Premise: faer 0.24.4 from `Cargo.lock`, with the host's global parallelism setting and `RAYON_NUM_THREADS=1`.
  SPD04's bitwise identity holds only under this premise. Another faer version needs a fresh premise review.
- SPD04 (L-0087) and SPD05 (L-0088) are library results. **A library allocation PASS without a driver caller is
  insufficient**: this node gates only driver-level, full-trajectory numbers.

## Question

SPD04 added a reusable least-squares workspace. It is reachable from the matrix-free U-form driver only through a
single-purpose entry point (`integrate_rodas5p_mf_fast_observed_gmres_into_ls_workspace`). SPD05 made LGMRES-into
solve one least-squares problem per cycle, but no driver calls `solve_lgmres_into`. The driver's LGMRES path still
uses `solve_lgmres_with_workspace` and clones the carried `LgmresState` before every attempt
(`rodas5p_matrix_free_fast.rs`, `let snapshot = recycle.clone()`). SPD04's corrections also record that
`GmresWorkspace::capacity_f64` does not count the least-squares buffers, so the memory the workspace keeps is not
fully charged.

Can one explicit driver option route GMRES and LGMRES stage solves through the workspace-owned paths, so that:
- trajectories stay bitwise identical, failures included;
- failure rollback and workspace growth keep defined semantics;
- full-trajectory allocations fall, with every retained byte charged?

## Change (opt-in; every existing entry point and default unchanged bitwise)

1. **Explicit driver option** in `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs`.
   - `KrylovWorkspaceOptions { ls_workspace: bool, lgmres_into: bool }` has stable identifiers. The default
     `{false, false}` is `krylov-ws-legacy-v1`.
   - It is set by `Rodas5pMfFastWorkspace::set_krylov_workspace(options) -> CoreResult<()>`, a field on the existing
     driver workspace. No new entry point is added, because the field suffices (see "Coordination with sibling
     nodes"). The result reports the option identifier.
   - For GMRES, `ls_workspace` implies the GMRES-into path with SPD04's `LeastSquaresWorkspace`. The old
     `..._gmres_into_ls_workspace` entry point is kept, and must equal the new option bit for bit.
   - Inconsistent requests are refused with a typed `InvalidInput` before any integration step:
     - GCRO-DR with either flag (GCRO-DR has no into path);
     - `lgmres_into` with a method other than LGMRES.
2. **LGMRES-into caller.**
   - With `lgmres_into`, LGMRES stage solves call `solve_lgmres_into` and write directly into the stage storage.
     The call uses SPD05's `set_least_squares_once(true)` and, with `ls_workspace`, SPD04's workspace through
     `LgmresIntoWorkspace::set_ls_workspace(true)`.
   - The x0, configuration, carried state and counters are those of the existing call.
3. **Rollback on failure.** There are two levels, and both must keep today's semantics.
   - *Solve level.* `solve_lgmres_into` restores the carried state and leaves the stage output untouched on
     failure. GMRES-into writes the stage only on success.
   - *Attempt level.* A rejected attempt (local error, linear-solve failure or non-finite stage) restores the carried
     `LgmresState` to its value before the attempt. Under the option, the per-attempt `recycle.clone()` becomes a
     copy into reused snapshot buffers that the driver workspace owns. The restored state must be bitwise the
     clone's: directions, image presence and values, operator token, system identity, previous solution and
     generation.
4. **Workspace growth semantics.**
   - A buffer grows only when a solve needs more than every earlier one. Growth is counted per component
     (`GmresIntoReport::workspace_grew`, `LeastSquaresWorkspace::growth_events`, pool and snapshot growth) and
     exported per trajectory.
   - A cloned driver workspace starts with empty least-squares buffers (SPD04 deviation). It regrows them and gives
     bitwise the same results.
5. **Charged workspace memory.**
   - `Rodas5pMfFastWorkspace::krylov_workspace_bytes()` returns the bytes retained by each Krylov component: GMRES,
     least-squares (the faer matrices, the permutations and the scratch `MemBuffer`), the LGMRES-into snapshot and
     pool, and the attempt snapshot.
   - The existing `capacity_f64` methods are unchanged, because pre-existing tests read them.

## Cells

- **Problems.**
  - The six L-0038 problems of `tests/rnext_common` (robertson, van-der-pol-mu1000, hires, prothero-robinson-forced,
    quadratic-4, brusselator-1d-50), as in SPD04.
  - brusselator-1d-160 (n = 320), as in SPD07.
- **Tolerances:** rtol 1e-6 (SPD04's driver cells) and 1e-8.
- **Replication versus new evidence.** The GMRES cells at rtol 1e-6 on the six L-0038 problems are a replication of
  SPD04: item 3's 0.10 bound was set knowing SPD04's 0.096 there. The new evidence of this node is LGMRES, rtol 1e-8
  and brusselator-1d-160. The two groups are reported separately.
- **Linear configuration:** as SPD04's driver runs: `linear_config(method, 1e-11)`, i.e. atol 1e-14, restart 40,
  inner_m 30, outer_k 8, no preconditioner, x0 `Previous`. `adaptive(rtol, atol_scale, span)` with 5,000 attempts.
- **Arms.** GMRES: control = GMRES-into without the workspace, candidate = `{ls_workspace}`. LGMRES: control = the
  existing driver path, candidate = `{ls_workspace, lgmres_into}`; `{lgmres_into}` alone is reported for
  attribution. 7 problems x 2 rtols x 2 methods x 2 gated arms = 56 gated runs.
- **Contract tests** (`crates/rodas5p-integrators/tests/krylov_workspace_driver_contract.rs`, not ignored):
  - **Caller parity:** one attempt and one short trajectory per method, each compared with the control.
  - **Forced failures:** an LGMRES and a GMRES stage solve at linear rtol 1e-14 with max_outer / maxiter 1. The
    stage output is untouched; the carried state after the rejected attempt is bitwise the state before it; the rest
    of the trajectory equals the control's.
  - **Non-finite JVP:** an injected non-finite JVP takes the same rejection path.
  - **Growth:** a reused driver workspace runs a second integration with zero growth events, bitwise equal to a
    fresh one. A clone regrows and is bitwise equal.
  - **Refusals:** every refused combination returns the typed error before any integration step.
  - **faer premise:** `Cargo.lock` pins faer 0.24.4.
- **Charge contract** (`crates/rodas5p-krylov/tests/sp02_workspace_charge.rs`).
  - For the least-squares and the LGMRES-into workspaces after reuse sequences, the live heap bytes released when
    the workspace is dropped are compared with the charged bytes.

## Measurement

- **Events: the SPD04 method** (`tests/rnext_common/mod.rs::Counting`). A global counting allocator in the test
  binary and one test thread; events = alloc + alloc_zeroed + realloc, one each. The whole `integrate_*` call is
  measured, workspace construction included.
- **Peak bytes: new, recorded by no prior node.** The new binary's allocator also keeps live bytes (+size on alloc,
  -size on dealloc, +new-old on realloc) and a high-water mark reset at each measured call's start. Peak bytes =
  high-water mark - live bytes at the call start. The event definition is unchanged.

## Commands

    SP02_RUNS=research/sp02_krylov_workspace_caller_20261011/RUNS.json cargo test --offline --release -p rodas5p-integrators --locked --test krylov_workspace_driver_contract -- --ignored --nocapture --test-threads=1 export_runs
    cargo test --offline --locked -p rodas5p-integrators --test krylov_workspace_driver_contract
    cargo test --offline --locked -p rodas5p-krylov --test sp02_workspace_charge
    python3 tools/sp02_workspace_profile.py --output research/sp02_krylov_workspace_caller_20261011/PROFILE.json
    python3 tools/sp02_workspace_check.py --runs research/sp02_krylov_workspace_caller_20261011/RUNS.json --profile research/sp02_krylov_workspace_caller_20261011/PROFILE.json --output research/sp02_krylov_workspace_caller_20261011/RESULTS.json

All commands run with `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`.

**Profiling.** As in SP01: callgrind on one release example built with line tables
(`examples/sp02_workspace_profile.rs`). Ir per trajectory = 2 minus 1 integrations, plus a repeated 1-integration
determinism check. Profiled cells: van der Pol, HIRES and Brusselator-50 at rtol 1e-6, for both methods and both
gated arms (12 profiles).

**Checker.**
- `tools/sp02_workspace_check.py` calls `tools/evidence_schema_v2.py` first.
- Malformed or unbound evidence gives INVALID (exit 2); a gate failure gives FAIL (exit 1); a pass gives PASS
  (exit 0).
- The checker and its mutation tests (`tools/test_sp02_workspace_check.py`) are committed before any recorded run.

## Gate

**Validity.** The result is INVALID (exit 2, never FAIL) if AS03 validation fails; a (problem, rtol, method, arm)
row is missing or duplicated; the export did not record one test thread and `RAYON_NUM_THREADS=1`; `Cargo.lock` does
not pin faer 0.24.4; a repeated callgrind 1-integration run is not identical; a profiled run does not reproduce
its RUNS record; or the checker blob at the RUNS source commit differs from the checker that was run, or any recorded
tree status is dirty. A non-0.24.4 faer pin is INVALID only; a fresh premise review precedes any rerun.

The result is HOLD (no verdict) if, for either method, fewer than 10 of the 14 (problem, rtol) controls succeed. In
the ledger this HOLD is recorded as INCONCLUSIVE, with claim "HOLD: k/14 controls" (k the smaller success count).

**PASS** if all of the following hold.

1. **Parity.**
   - In all 56 runs, the candidate equals its control bitwise: output times and state bits, attempts, accepted and
     rejected steps, every WorkCounters field, the failure kinds, and the SHA-256 of the stage-solve log (residual
     norm, relative residual, iterations).
   - The only allowed exception is SPD05's enumerated one: a stage solve where the control aborts on an intermediate
     non-finite least-squares solution. Such solves are listed, and the list is predicted empty.
   - The GMRES candidate also equals the old `..._gmres_into_ls_workspace` entry point.
2. **Caller contracts.** Every contract test passes: parity, both forced-failure rollbacks, the non-finite JVP, the
   growth and clone tests, the refusals and the faer premise.
3. **Full-trajectory allocations.** In every cell whose control succeeds, measured over the complete integration
   call:
   - the candidate's allocator events are <= 0.10x the control's on the five non-HIRES L-0038 problems and on
     brusselator-1d-160, for both methods;
   - the candidate has strictly fewer events than the control in every cell, HIRES included.
4. **Peak memory is charged.**
   - In every cell, candidate peak bytes <= control peak bytes + the candidate's charged bytes for the components the
     option adds (least-squares workspace, LGMRES-into snapshot and pool, attempt snapshot).
   - In the charge contract, the charged bytes are never below the bytes released on drop, and exceed them by at most
     128 bytes per faer-owned buffer (alignment padding).
5. **Instructions.** On the 12 profiles, candidate Ir per trajectory is <= 1.000x the control's.

Everything else is **FAIL**, with every number preserved.

**Kill and hold rules.**
- Any bit difference outside the enumerated exception is a kill. The failing run and its first differing attempt are
  recorded, and the option stays unadopted.
- Uncharged retained memory (item 4) is a FAIL, whatever item 3 shows.
- A FAIL is never re-scored after a threshold change, and SPD04/SPD05 records are not rerun or edited.
- A PASS adopts the option as an explicit, versioned driver option only.

**Reported, not gated:** allocations and peak bytes per attempt; growth events per component; the
`{lgmres_into}`-only attribution arm; `krylov_workspace_bytes()` by component; Ir by `tools/speed_profile.py`
category, including the malloc share.

## Coordination with sibling nodes

AS04, SP02, SP04 and PC01 all edit `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs`. Their changes are
merged in the fixed order AS04 -> SP02 -> SP04 -> PC01, each as orthogonal opt-in fields, default off, with no new
entry point per node where a field on the existing options or workspace suffices. Each later node's parity tests
include the earlier nodes' options-off identity: SP02's contract tests check that AS04's telemetry off leaves the
SP02 control and candidate bitwise unchanged, and SP04 and PC01 re-check SP02's options-off identity
(`krylov-ws-legacy-v1`).

## Prior information (disclosed)

**SPD04 (L-0087).** On the same six problems at rtol 1e-6 and linear rtol 1e-11, GMRES-into driver allocations
per attempt went from 89.9-93.5 to 1.65-8.67 (0.018-0.096).
- Prothero-Robinson's 0.096 (12 attempts) lies close to the 0.10 bound of item 3. That bound is SPD04's item 4,
  kept unchanged and set with this knowledge.
- HIRES went from 168.76 to 81.12 (0.481), because its test JVP allocates per application. That is why HIRES is
  held only to "strictly fewer".

**SPD05 (L-0088).** LGMRES-into with ls_once and the SPD04 workspace allocated 0.0009-0.009x the default
LGMRES-into per library solve, with 344 forced failures rolled back.

**L-0038.** The driver's LGMRES path allocated 317-3,954 times per attempt on the six problems at rtol 1e-6. That
source is from 2026-10-02, and the number is not re-measured here before the run.

**Design choices.** The peak-bytes rule (item 4), the 128-byte padding allowance and the Ir bound 1.000x are
design choices made without prior data. No code of this node exists before this commit.

## Predictions

- Item 1: parity in all 56 runs; empty exception list. Item 2: all contracts pass.
- Item 3: GMRES 0.02-0.10 (as SPD04 at 1e-6; lower at 1e-8, more attempts); LGMRES 0.002-0.03 on the non-HIRES
  cells, HIRES about 0.1-0.3.
- Item 4: the candidate's peak at or below the control's in most cells (same-size transient faer allocations vanish).
- Item 5: GMRES 0.85-1.00; LGMRES 0.3-0.9 on van der Pol and HIRES (per-column least squares dominate), uncertain on
  Brusselator-50.
- Least certain: whether the Prothero-Robinson GMRES cell stays below 0.10. Overall: probably PASS.
