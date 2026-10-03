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
