# Integrated plan: external thread-transfer review and native re-audit

This file merges the open work of three audit packages on this branch into one DAG. The packages themselves are
left byte for byte as they are.

| Package | Origin | Plan | Executed so far |
|---|---|---|---|
| `docs/reviews/20261002_thread_transfer/` | external review (Python exact probes, no Rust run) | findings TF-01..TF-09, tasks N1, N2, N3, P1, M1, V1 (`IMPLEMENTATION_PLAN.md`, `REVIEW.json`) | never mapped node by node; see below |
| `docs/reviews/thread_transfer_20261002/` | thread-transfer review | DAG P0/P1/P2 nodes | all run (L-0034..L-0043), `EXECUTION_STATUS.md` |
| `docs/reviews/20261003_native_reaudit/` | native re-audit | R-NEXT-01..09 | all run (L-0044..L-0051), `RNEXT_EXECUTION_STATUS.md` |

## Where each external task stands

| Task | Finding | Status before this plan | Evidence |
|---|---|---|---|
| N1 action-first certificate | TF-02, TF-05 | done: `blocked_action_doubling_certificate_with_execution`, exact containment, workers 1/2/4/8 bit identical | L-0035 |
| N2 residual-seeded radius | TF-01, TF-05 | done: `residual_seeded_common_radius`, `tests/thread_transfer_radius_policy.rs`, old policy kept | L-0034, L-0036 |
| N3 structural witness and prepared context | TF-04, TF-05 | **open**: `InverseWitness` stores a dense `n x n` bound, `QuadraticStageProblem` a dense `J`, and one q2 attempt builds the stage problem and the witness twice (capability, then certificate) | `outward_certificate.rs`, `transactional_q1_q2.rs` |
| P1 small kernels and banded pipeline | TF-03, TF-08 | small kernels done (L-0041); **banded part open**: `rodas5p_fast` skips zero multipliers but still assembles and scans dense `J`/`W` (O(n^2) floor) | L-0041, `rodas5p_fast.rs` |
| M1 stage-coordinate candidate adapter | TF-06, TF-07 | **open** | - |
| V1 nonnormal output-error contract | TF-09 | **open** (Laguerre admission, L-0047, is symmetric-only) | L-0039, L-0047 |
| q2 native activation (`native_q2_activation_requires`) | TF-05 | needs N1, N2, N3, cost evidence and the budget gates; cost evidence is L-0050 (abstain for n <= 8) | L-0050 |

## Open items from the native re-audit after its run

| Item | Source | Disposition here |
|---|---|---|
| GCRO-DR recycle pairs that break `A U = C` are reused unverified; the trace omits aborted cycles; the review's acceptance test on unused synthetic cases was not met (its held-out set failed for every solver) | R-NEXT-03, L-0046 post-hoc | **INT-01** |
| `solve_into` for LGMRES and GCRO-DR | R-NEXT-02 (review section 3) | deferred until INT-01 settles the GCRO-DR reuse rule |
| A residual-to-output budget that resolves the Robertson h = 1e-2 decision | R-NEXT-01, L-0049 | deferred: the stage states there are 1e117, so the case needs a decision rule for blown-up candidates, not a tighter mean-value box; no candidate rule is ready |
| A generic no-chart replacement (derivative-light peer, stiff-order conditions) | R-NEXT-08, L-0051 | stopped by the DAG stop condition (RVJ5 not uniform); not resumed |
| Timing campaign, timing authority | R-NEXT-09, P2-TIMING-GATE | HOLD unchanged |

## Integrated DAG

Each research node is preregistered and pushed before its code and runs, gets a ledger row, and has its results
appended below its preregistration. Existing ledger rows, fixtures, holdouts, coefficients and the timing authority
are not changed. New interfaces are opt-in; defaults do not change.

| Node | Covers | Depends on | Deliverable | Stop condition |
|---|---|---|---|---|
| INT-01 verified recycle reuse | R-NEXT-03 follow-up, review section 4 acceptance | - | opt-in invariant check of the carried recycle pair before reuse (its products charged), aborted cycles in the trace, a fresh synthetic family fixed before the run | the check does not remove the recycle-induced failures, or any false convergence |
| INT-02 structural certificate pipeline | N3, TF-04, TF-05 | - | diagonal `J` and witness stored and applied as O(n) data through residual, recurrence and projection; a prepared attempt context that builds problem and witness once | any bound or admission decision differs from the dense path, or a stale/edited input is accepted |
| INT-03 banded fast pipeline | P1 banded part, TF-03 | - | an explicit band provider for the fast driver: banded `W`, LU with the pivot region bounded under row swaps, banded solves, dense fallback | lost trajectory parity, or counted work not O(n b^2) |
| INT-04 stage-coordinate candidate adapter | M1, TF-06, TF-07 | - | `K = Psi(Z)` adapter with chain-rule residual action; the restored `K` is judged only by the original target certificate | the adapter's residual or a chart status is ever used as acceptance |
| INT-05 nonnormal metric output bound | V1, TF-09 | - | a certified output bound for polynomial actions of nonnormal matrices from a verified metric numerical-range enclosure (Crouzeix-Palencia), with recurrence rounding and norm transport | the bound fails to enclose a high-precision reference, or a sampled residual is promoted |
| INT-06 q2 activation decision | TF-05 | INT-02 | a decision record on native q2 activation from N1, N2, N3 and L-0050 | - (document) |
| INT-07 publication and validation | R-NEXT-09 practice | all | validation matrix on the final head, status file, PR #70 body appended | - |

No timing is run and no speed is claimed. A FAIL is recorded as a FAIL.
