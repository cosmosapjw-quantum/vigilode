# G1 PHYS–MATH–CODE AUDIT

## Equation-to-code map

| Mathematical object | Code path |
|---|---|
| \(Wv=Mv-h\gamma Jv\) | `build_step_context_matrix_free`, `MatrixFreeCommonWSolver` |
| q1 truncated inverse | `truncated_inverse_rows_with_last_term(..., q=1)` |
| retained q1 endpoint | `stages`, `last_q1_causal_term`, `q1_candidate_y` |
| q2 extra causal term | `coupling_apply(last_q1_causal_term)` + one common-W solve |
| post-q2 remaining defect | one matrix-free q0 diagnostic solve |
| operational gate | `operational_gate` |
| protected fallback | `sequential_stages` on strict matrix-free context |
| adaptive transactional commit | `integrate_transactional_q1_q2_adaptive_observed` |
| complete work accounting | `WorkCounters::accumulate`, `HomotopyWorkLedger` |
| generic one-step gate | `g1_transactional_gate` |
| common adaptive authority | `run_g1_adaptive_global_error_screen` |

## Genuinely fixed

1. Missing JVP fails before explicit Jacobian construction.
2. q2 escalation does not recompute the q1 start/path.
3. q1/q2 path attempts and selected final lane are separately recorded.
4. Fallback retains speculative work and uses the original matrix-free RODAS implementation.
5. Rejected adaptive trials do not commit output state.
6. BDF/Radau byte-freeze and physical-client exclusion tests remain active.
7. Eight-family generic calibration and bounded adaptive screen are deterministic CLI gates.
8. `WorkCounters::accumulate` uses saturating fieldwise accounting.

## Remaining risks

### P1 — certificate batch removes nominal 7-depth advantage

The q2 causal upgrade itself changes depth 6→7, but the post-q2 defect diagnostic makes accepted q2 depth 8.
Dropping it for speed would weaken the only online correctness evidence and is forbidden.

### P1 — fallback duplicates expensive work

A failed q2 attempt pays 8 batched common-W rounds and then 8 protected sequential stages, producing depth 16.
This dominates nonlinear/nonnormal problems.

### P2 — provided-JVP requirement

The strict path correctly refuses explicit-J fallback. A production generic solver still needs a fresh AD,
complex-step, or quality-controlled finite-difference JVP mode. That is outside G1.

### P2 — unrelated deferred comparator fragility

The expanded general adaptive suite exposed a pre-existing GCRO-DR generalized-eigen buffer panic. G1 avoids
letting that unrelated deferred backend mask the candidate by using a dedicated five-candidate G1 runner.
The panic is not classified as a G1 candidate failure and was not repaired here.

## Test sufficiency

Added tests cover explicit-J fail-closed, q1 depth 6, q2 depth 8, fallback depth/work preservation,
adaptive rollback, generic problem/JVP contracts, work-counter accumulation, eight-family coverage,
CLI determinism, and frozen comparator scope. This is enough to close G1 semantics and apply the declared
finite kill rule; it is not a proof over arbitrary ODEs.

## Final-validation repair — scheduler-dependent diagnostic hash

첫 tagged-head 전체 검증은 기존 `stage_batch_feasibility` report의 scientific checksum이
`observed_max_parallel_tasks`의 정확한 값(2–4)에 의존해 실패했다. 이 값은 OS/Rayon scheduler가
결정하는 진단량이며 scientific result가 아니다. Exact maximum은 report에 보존하되 checksum에는
`stage_parallelism_observed: bool`만 포함하도록 최소 수정했다. 기존 반복-determinism tests가 이
regression을 직접 검증하며, stage physics/timing row와 G1 scientific checksum은 변경되지 않았다.
