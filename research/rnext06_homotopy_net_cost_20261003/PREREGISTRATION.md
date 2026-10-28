# Preregistration: net-cost admission of causal homotopy candidates (remaining-only DAG node R-NEXT-06)

## Question

R4 (L-0024) found that the serial stage certificate is the cheapest certificate by count, and that the idealized
batch budget leaves under one solve of margin. L-0035/L-0036 added an action-first path sum (0.565x the operations
of the matrix order) and a residual-seeded radius that closes the R4 fixtures with two evaluations. Those fixtures
used the crude candidate `K_i = h f(y)`. On the actual q=2 candidates of certified transactional runs, with the same
target and the same physical output budget, which certificate costs less? And with every cost charged (certificate,
preflight, failed radii, witness, W batches, dispatch, fallback), is there any net margin over the sequential step?
If not, the router must abstain in that regime.

## Implementation (written after this commit)

- `TransactionalQ1Q2StepReport` gains `q2_candidate_stages` (certificate mode only): the stages the q=2 certificate
  was attempted on. Nothing else in the step changes.
- Study test `crates/rodas5p-integrators/tests/rnext06_homotopy_net_cost.rs` (writes `RESULTS.json`).

## Design (fixed now)

- Models: the R4 family, `A = diag(-1 - i)`, `q_i = -0.05 (1 + i mod 3)`, `y_i(0) = 1 + 0.1 i`, n in
  {1, 2, 4, 8, 16}, on `(0, 0.5)`. Certified transactional steps
  (`Q2Admission::NativeTargetCertificate(model)`, default config, `threads = 1`) are driven by an adaptive loop with
  `rtol = 1e-6`, `atol = 1e-6`, `h0 = 0.05`. The loop uses `rodas_next_step_after_attempt`, the same controller as
  the sequential comparator.
- For every attempt with a q=2 candidate:
  - **Serial arm:** the step's own certificate (`directed_operations`, witness included), recomputed in the test
    and required to be bitwise the step's.
  - **Action-first arm:** `DiagonalMajorant` on the same `(target, problem, candidate, witness)`, then
    `residual_seeded_common_radius(factor 2, ActionFirst)`, then `blocked_box_certificate_with_execution` at the
    proposed box. Its cost is every evaluation's directed operations (preflight and failed radii included) plus the
    certificate's, plus the same witness cost as the serial arm.
  - Recorded per arm: whether the box closes (closure), whether `output_wrms_upper <= budget_lower` (outer
    acceptance), the output bound relative to the serial arm, and the operations.
- **Solve cost unit:** a sequential RODAS5P step on the same `(t, y, h)` (`sequential_step`, the same GMRES
  configuration). Its counted vector work is converted to operations as `2 nnz(W)` per matvec, `2n` per inner
  product or vector update, and `n` per preconditioner application. `c_solve` is that total over the 8 stages,
  divided by 8.
- **Net margin per attempt**, with P = 8 ideal workers, in units of `c_solve`:
  `M = 8 - critical_path_depth - (certificate operations on the critical path) / c_solve - 1[dispatch]`.
  `critical_path_depth` is the step's own (W batches plus 8 for a fallback). One dispatch unit is charged per batch
  when `threads > 1`; it is 0 here and stated. M is reported per n, both as a mean over attempts and in total, for
  each certificate arm.
- **Abstention rule:** a regime (n) is admitted for parallel expansion only if the total margin with the cheaper
  certificate arm is positive. Otherwise the router abstains there.

## Gate

**PASS** if all hold:

1. **Same target.** For every q=2 candidate, the recomputed serial certificate is bitwise the step's (bounds and
   operations), and both arms run on the identical `(target, problem, candidate, y_hat, e_hat, witness)`.
2. **Closure vs acceptance.** Closure and outer acceptance are reported separately for both arms. Whenever the
   action-first arm closes, its certificate's output bound is valid in its own terms: it is reported against the
   serial arm, and never used to accept anything here.
3. **Everything charged.** The action-first arm's operations equal the sum over all of its evaluations plus its
   certificate. The margins charge W batches, fallbacks and certificate work. No timing is run.
4. **Decision.** The abstention rule is evaluated for every n, and the stop condition is stated: parallel
   expansion stops in every regime where certificate overhead consumes the margin.

Otherwise **FAIL**. The margins and the abstain/admit outcome are measurements: either answer is recorded. Timing
authority stays on HOLD, and `SPEEDUP_UNPROVEN` is retained.

## Prior information

R4/L-0024 (serial cheapest, margin under one solve), L-0035 (action-first 0.565x of matrix order), L-0036 (seeded
radius closes the R4 fixtures with two evaluations). No code of this node exists before this commit.

---

## Results (appended after the run at `a06d385`)

Output: `RESULTS.json`. Ledger row L-0050.

**Gate: FAIL** (items 1, 2 and 4 hold; item 3 fails).

| Gate item | Outcome |
|---|---|
| 1. Same target | **holds**: for every q=2 candidate, the recomputed serial certificate matches the step's bit for bit (operations, output bound, stage bounds), and both arms used the same inputs |
| 2. Closure vs acceptance | **holds**: both arms close on every candidate, and both accept every candidate within the certified budget. The action-first output bound is 1.0000000000000002x to 1.00000000006x the serial bound |
| 3. Everything charged | **fails**: the test charged the action-first arm with its evaluations plus the witness, assuming the box certificate adds nothing else. Its own check found that the certificate also counts its finishing work (output and embedded bounds): 68 operations per component (73 = 68 + 5 at n = 1, 292 at n = 4, 1,168 at n = 16; post-hoc diagnostic). So the recorded action-first margins are too favourable by `68 n / c_solve` per candidate |
| 4. Decision | **holds** (as recorded): the rule is evaluated for every n |

All 39 attempts took the q=2 lane (q=1 never passed, and no fallback was needed), with critical-path depth 7.
Margins in sequential-solve units (`c_solve`: unpreconditioned GMRES on the diagonal `W`, 20 to 18,316 operations
per stage solve). The action-first column below is **corrected post hoc** by the missing `68 n` per candidate:

| n | serial / action-first operations | mean margin, serial | mean margin, action-first (corrected) | decision |
|---|---|---|---|---|
| 1 | 425 / 1,095 | -20.5 | -54.4 | abstain |
| 2 | 1,010 / 2,190 | -12.0 | -27.3 | abstain |
| 4 | 2,660 / 4,380 | -5.6 | -9.9 | abstain |
| 8 | 7,880 / 8,760 | -2.07 | -2.41 | abstain |
| 16 | 26,000 / 17,520 | -0.42 | +0.043 (total +0.65 over 15 attempts) | admit by the rule, marginally |

Reading: the certificate costs more than the eight sequential stage solves it replaces, except at n = 16. There
the action-first arm becomes cheaper than serial, and the margin is 0.04 solve units per step with no dispatch
charge (threads = 1). Real dispatch, a pool, or a preconditioned (cheaper) solve would remove that margin.
Parallel expansion therefore stops on this family for n <= 8. At n = 16 the regime is admitted only by a margin
smaller than any uncounted overhead, so no speed claim follows. Timing authority stays on HOLD, and
`SPEEDUP_UNPROVEN` is retained.

Disclosure: the post-hoc diagnostic was a temporary test that was not committed. It printed the evaluation, box and
certificate operations for n = 1, 4 and 16, and the corrected column uses those measured 68 n finishing operations.
The recorded verdict stays FAIL. No development run of the study preceded the recorded run.
