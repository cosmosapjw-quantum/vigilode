# Independent review: complete structured RODAS5P backend

Reviewer: packet-completion agent, separate from structured-driver author.

Scope: changed `CollectiveStageFactors::refactor`, checked metadata, new `CollectiveRodas5pWorkspace`, observed adaptive driver, all-stage tests and comparison runner. Existing Woodbury algebra and inverse-bound derivations are reused; only their new mutable reuse and all-stage integration dependencies were reviewed.

## Mathematical and implementation assessment

The new workspace implements the native K-coordinate recurrence

\[
 (I-h\gamma J_0)K_i=h f(y+\sum_{j<i}\alpha_{ij}K_j)
                      +hJ_0\sum_{j<i}\Gamma_{ij}K_j.
\]

The extra native nonautonomous term h² gamma_i f_t is zero for this autonomous collective model. The frozen JVP is evaluated at the original y, and the same factorization is used for all eight stages. The candidate and embedded sums use the native b and btilde arrays. Comparing these statements against `sequential.rs` establishes coordinate consistency; no scaled-stage convention is substituted.

One block/rank-two factorization and eight O(N) solves give O(N) setup/attempt work and O(N) workspace for this fixed eight-stage method. Actual constants include all stage combinations, RHS/JVP work and factorization interval checks. The complete numerical driver issues embedded estimates and does not claim a global error enclosure.

The workspace privately owns the generative model and tableau. Refactorization invalidates old factors before any validation; failed solve and residual-witness entry points refuse. `step_into` invalidates old stage results before an attempt and only copies the candidate to caller output after successful stage/scale processing. A successful attempt can still have an embedded estimate above one; the adaptive caller separately rejects it.

The adaptive driver validates configuration even at zero span, follows the existing represented-clock and output-landing utilities, preserves accepted state on rejection, and retains unsuccessful integration status/partial outputs when min-step or max-attempt limits are reached. Carrier metadata is rejected by model admission rather than silently included in a different equation. Exact represented weight sum one is an explicit current factorization restriction.

## Findings

**SR01, closed:** adding mutable refactorization made the previously unconditional determinant getter stale after a failed refactor. The solve path refused correctly, but the getter could expose the previous finite determinant as current metadata. The author changed it to a validity-checked `CoreResult<[f64;2]>` and added a failure-invalidation regression. The review checked the correction in source.

**SR02, closed by scope correction:** the first comparison JSON recorded only two allocations/32 bytes for full setup and integration, contrary to the owned vector setup. A known-library-allocation positive control measured zero for three nonempty cloned vectors; a fresh-target run reproduced the failed calibration. The final runner makes this control mandatory, writes authoritative `allocation_calls`/`allocated_bytes` as `null` when it fails, and retains raw counters only in explicitly diagnostic fields. All six final smoke rows have null authoritative allocation fields and successful numerical outputs. Thus the misleading measurement claim is contained; empirical allocation remains **NOT_ESTABLISHED**. Structural O(N) and source-level workspace reuse remain established. No further allocator probing is necessary for this review scope.

## Validation evidence inspected

`collective_rodas5p_contracts.rs` compares every stage, endpoint and embedded estimate against dense sequential direct solves for N=1,2,4,8,16; chi=-0.7,0,0.9; kappa=0,0.6; h=0.001,0.07 (60 complete attempts). Its separate nonlinear one-mode exact solution covers both damping values and landed requested outputs. Refactor failure, transactional output, unsupported model, zero span and invalid tolerances are exercised. Existing direct-backend six-test evidence is retained. The reviewer inspected the author execution log instead of rerunning unchanged cases.

The RK4 comparison reference uses two resolutions and correctly labels refinement difference empirical. The runner includes adapter/setup/output work, retains failed arms and does not issue a speed promotion. A release campaign at matched achieved errors is a distinct local execution unit.


Final receipt: `structured_scoped_numerics.log` records 19 passing numerical tests, including the five structured-driver tests, six direct-backend tests and eight separate PR101 tests. The standalone allocation gate intentionally reports the unresolved calibration failure. `structured_completion.json` and `structured_review.json` bind the final six source/test/example file hashes. The reviewer checked the changed calibration guard and typed determinant accessor without rerunning unchanged mathematics or numerical cases.
