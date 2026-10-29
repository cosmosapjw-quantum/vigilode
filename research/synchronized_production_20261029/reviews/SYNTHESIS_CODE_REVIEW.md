# Independent synthesis and code review — before execution

Reviewer: `/root/state_audit`, independent of the three theory/code authors and the parent synthesis/runner author. Date: 2026-10-29 KST. This review is static source and mathematical inspection; it executes no native or Python scientific tests and makes no performance claim.

**Verdict: ACCEPT_SCOPED for S1–S4, the three exact research consumers, A1 ownership projection and K1 private basis-only preparation.** The lane is admissible for the published-source bounded experiment once the parent supplies the build bindings and guards described below. Production qualification remains HOLD. This document does not count later unexecuted tests as evidence.

## Synthesis proof

S1 subtracts the same frozen reference stage equations, giving
`A_i(hat k_i-k_i)=b_i(hat k_<i)-b_i(k_<i)-r_i`.
The residual sign disappears under the norm. Applying the supplied inverse margin and RHS Lipschitz constants gives exactly the stated triangular recurrence. All coefficients are nonnegative; induction proves the componentwise bound. Strict lower triangularity gives `T^s=0`, so the finite sum is exact and requires no iterative convergence condition. It is a stage perturbation estimate, not a truncation-error estimator. The source must still supply each native method's constants, domains, common represented operator and residual uncertainty.

S2 follows by adding and subtracting `p'` and `f(t,p)`. The initial text omitted the monotonicity requirement when passing from componentwise stage bounds to a block norm; the revised text explicitly requires it. The implementation uses `max(eta)` and satisfies this requirement. The derivative reconstruction constant and `Lf*C0` have identical units. The direct residual route already includes stage error and must not add the stage-transfer expression again. The revised text explicitly sets source offset epsilon to zero when the bound is against the exact original field. A provider/carrier residual with an unevaluated source discrepancy instead retains its separately owned epsilon. **Both findings closed by read-back.**

S3 is an induction over actual chronological slabs, subject to complete coverage, common target IVP, domain containment and checked norm/source/clock transfers. The D4 invariant-radius admission is made after the incoming transfer. Storage ownership itself contributes identity transport and zero numerical jump; neither matching route letters nor returned endpoint bits provides the missing physical-provider hypotheses. S4 correctly separates charged proposed/discarded/certificate/export work and makes no generic policy-speedup inference. Python quotient/tube helpers are not installed as a native dispatcher.

## Exact consumers and test design

`stage_transfer.py` accepts exact integer/Fraction inputs only, rejects empty/mismatched or cyclic stage systems, requires positive inverse margins and nonnegative constants, and implements the S1 recurrence. Its reconstruction helper uses the documented infinity block norm. The four source tests include an independently formed signed triangular system, sharp inherited-stage contribution, a tube join, and invalid/cyclic/floating input. Their source is suitable for the announced exact check; no result is asserted here.

`directional_quotient.py` constructs the monic-polynomial quotient using exact arithmetic, refuses zero denominators independently of whether the original matrix is invertible, obtains the stated Euclidean logarithmic-norm upper bound from the symmetric-part Gershgorin bound, and requires positive coercivity margin. It recomputes the original residual and checks the exact relation identity before the requested error cap. The separately constructed Gaussian-elimination oracle, degree/zero-step/pole/nonnormal/near-relation and sign-mutation tests address independent obligations. Dense exact matrix use is explicitly research scope; neither native matvec cost nor floating enclosure is claimed.

`defect_composition.py` implements alpha, delta, invariant-radius margin and every degenerate branch with exact arithmetic. For positive beta, its affine maximum over the rational exponential interval is correctly oriented regardless of the sign of incoming-minus-equilibrium. The endpoint minimum with admitted radius remains an upper bound, and whole-slab maximum uses the monotone comparison envelope. Composition checks exact adjacent times and typed source/norm/clock bindings, then applies the boundary transfer before the next admission. The weighted-infinity norm ratio and strict event margin/crossing signs agree with D8–D9. The formerly implicit later-slab type validation is now performed before attribute access; the minor invalid-type finding is closed. The independent nonlinear `x'=(1-x)^2`, `x=t/(1+t)` fixture and equality-boundary cases are meaningful checks. Decimal80 use remains diagnostic rather than a proof of native outward rounding.

K4's requested fixed-RHS wording was also read back: it now includes `delta_b` when the RHS changes, and distinguishes metric/domain certificate invalidation from the algebraic shifted-image identity. The earlier review's outstanding editorial clarification is closed.

## Native diffs

### A1 consumed output

`solve.rs` reads routed metadata before matching the owned `RoutedRun`; it moves exactly the same observed rows from Banded/Dense/MatrixFree variants. It preserves `routed.partial`, constructs the scalar stop record before moving a possible partial row, and keeps the original strict append predicate. Dense extension handling is preserved. No arithmetic, tolerance, route guard, status conversion or counter update changes appear in this diff.

Under A1's value-observable callback contract, these are ownership transfers of identical floating values. Allocator-address/count dependent callback behavior and resource failure are outside the stated successful-value theorem. Compilation still must establish the Rust partial-move legality and native tests must establish the intended fixture behavior.

The source-identical ownership probe includes eleven cases: dense/JVP/declared-band routes, vector atol, step ceiling, callback fatal, zero span, dense event, event nonfinite, finite-difference and a nested solve inside an RHS callback. It exports complete serialized solution observations, explicit time/state bits, dense-sample bits and callback counts, separately from allocation and timing fields. Expected failures are deliberately retained. A zero-span inner solve exercises nested facade invocation, not a long concurrent mutable session; do not extend its claim beyond that scope.

### K1 private directions omission

`gmres_into::begin_cycle` alone changes to basis-only preparation. That preparation preserves the old common Arnoldi fields and leaves existing direction storage retained. Full `prepare` and public `reserve` still size directions for legacy/augmented use. Into column and final-correction reads use basis/raw scratch, so the omitted directions allocation is not a numerical dependency. The order of full-path allocations changes, but successful arithmetic/callback semantics remain the stated target; resource-failure timing is not part of this equivalence.

The new integration contracts compare cold versus fully reserved into results/counters/operator input words; mixed legacy/into shape changes; and right-preconditioned monitored candidate words. The private unit contract poisons retained directions with NaNs, changes basis-only shapes and compares a later augmented solve with a fresh workspace. These address the dangerous stale-size/read paths. Capacity and growth telemetry are intentionally excluded from numerical equality. The cold fixed-size capacity prediction is an assertion awaiting actual execution, not measured savings.

The time-FD test change `b.clone()` to `*b` preserves the copied `WorkCounters` value and addresses the observed clone-on-Copy CI lint. Its indentation needs ordinary formatting before source publication; this is not a numerical correction.

## Runner review and closure

The initial runner allowed later timings after failed checks, accepted matching failed probe outcomes, ignored warmup status, omitted binary/source binding and did not preserve aggregate failure state. The reviewed revised source now:

- Stops `--phase all` after failed proof/native checks; requires that proof discovery actually ran tests.
- Requires matching fixture metadata, exact returned samples/callback counts, successful probe statuses and finite non-null endpoint diagnostic errors within the unchanged cap before timing each case.
- Checks warmup/timed success, preserves all raw subprocess receipts, writes timings after each block, and writes a terminal state on ordinary exception/exit through `atexit`.
- Requires binary digest/build-binding equality and remotely reachable source for both benchmark arms, with candidate source equal to the published pin.
- Counts all listed prior phase wall times against the same 900-second campaign cap, records command timeout/CPU/wall/log hashes and retains production HOLD.
- Adds33 full-observation ownership comparisons before ordinary benchmark timing, retaining expected failure observations.

The review leaves three explicit orchestration obligations rather than claiming that the runner proves them autonomously. First, the parent must supply trustworthy source/toolchain/profile/Cargo.lock build receipts; a self-declared JSON source identity alone is not proof of compilation provenance. Second, every split phase must list all prior phase receipts; separate phase invocation must be admitted after reviewing earlier failures, as EXECUTION_PLAN states. Third, the workspace and binary inputs must stay immutable during execution. The initial tracked-source diff plus HEAD check is not a concurrency lock and does not detect all untracked scientific input changes.

One concrete final requested guard is pending parent read-back: `production_from_json` should receive the same candidate source/digest binding validation before a deployment-only invocation. The benchmark loop validates the two benchmark binaries; it does not by itself validate a third deployment executable. The parent may satisfy this by adding the check or by retaining an explicit independently checked build binding in the execution record.

The runner's endpoint cap is inherited binary64 analytic diagnostic coverage. Timed arms assert status after preflight parity; they do not provide rigorously enclosed reference uncertainty or every-interior-path error. The existing two selective-JVP manufactured invocations establish configuration/accounting only. The inherited Givens, API, historical profile, interaction, release/platform and authority gates remain open.

## Correction and evidence ownership

All findings above are pre-publication source/proof review. They do not convert earlier failed runs to successes or reset any historical budget. A1 remains mapped to PR-N04, K1 to PR-N01, lint to its existing owner, and the exact consumers remain research-only. The maximum three new coherent correction episodes is an additional campaign ceiling. In particular, Q-LEDGER, PR-N05, R-STRUCT and R-ADAPT exhausted counters remain exhausted. Native endpoint cache reuse is not admitted here.

Following publication, C6/C7 results must come from the actual pinned tree and its raw receipts. C8/C9 claims must preserve adverse samples, failed commands and the narrowness of the physical diagnostic. This review admits an experiment and scoped transplantation contracts, not production promotion.
