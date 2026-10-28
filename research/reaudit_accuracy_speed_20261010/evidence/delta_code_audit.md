# Delta code audit — 2026-10-10

Scope: read-only inspection of Rust additions between a49f7e4abacfd88c6da3200055467fcd717464cf and cfed140bf127f5aa8fcf4fe4c16689d7de062b87. No scientific execution, no old campaign rerun, no production changes. Repository research-node policy read. These are source findings and proposed discriminating checks, not claimed runtime reproductions.

## A. New finite-input overflow fail-open in the coupled fallback (priority high)

`crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs:810-811,843-849` builds `production_threshold = linear_atol.max(production_rtol * safe_l2(stage_rhs))`, then accepts the fallback iff `safe_l2(unscaled_residual) <= production_threshold`. Neither norm nor threshold is checked for finiteness. Finite components do not imply finite representable L2 norm. If both overflow, `Inf <= Inf` is true. The diagonal scaling can keep the staged RHS and true scaled residual finite, so staged GMRES's own finite threshold validation does not protect this branch. The unscaled GMRES path calls `common::residual_threshold` and rejects an overflowing threshold.

Discriminating construction (preregister before execution): four-dimensional cyclic shift W, restart=max_columns=1, scaled RHS `[1.5,1.5,0,0]`, scale `1e308` in every coordinate. Corresponding finite physical RHS `[1.5e308,1.5e308,0,0]` has real norm about 2.121e308, overflowing binary64. The one-column iterate is approximately `[.75,.75,0,0]`; scaled residual approximately `[1.5,.75,-.75,0]` has norm 1.837..., and its physical L2 also overflows. A production relative tolerance 1e-10 should reject by approximately ten orders of magnitude, while the literal fallback predicate accepts. All iterate components remain finite. This construction diagnoses stage acceptance, not a demonstrated accepted full ODE trajectory.

Suggested port: centralize finite residual/threshold acceptance; conservatively reject nonfinite summaries or compare scaled norm representations without forming overflowing products. The fallback must check the unscaled residual of the returned physical iterate if it is to make that exact contract. Test all finite/Inf/NaN combinations and the full staged invocation with the construction, plus a moderate-scale positive control. Do not claim a global accuracy fix from this local repair.

## B. Predictive factor underflow reverses the clamp (priority medium)

`crates/rodas5p-integrators/src/adaptive.rs:403-413` computes `safety * (h/h_acc) * (err_acc/(error*error)).powf(1/k)`. For valid positive finite `h_acc=1`, `err_prev=.5`, `h=1e-100`, `error=1e-200`, `k=5`, the mathematical predictive factor is `0.9 * .5^(1/5) * 1e-20`, strictly below min_factor .2. In binary64 `error*error` becomes zero, so raw becomes infinity; infinity is explicitly allowed past the invalid check and clamps to max_factor 5. Integral factor is also capped at 5. Therefore the direction of the controller adjustment changes, rather than merely its last bits. Still more extreme finite h ratios can produce 0*Inf -> NaN and error out.

Suggested test: two accepted updates, first `(h=1,error=.5)`, second `(h=1e-100,error=1e-200)`; no clipping and no rejection. Compare returned factor against a high-precision or logarithmic independent oracle, then controls where neither intermediate underflows. Suggested implementation is log-factor accumulation and clamp in the log domain before exponentiation. Existing ordinary-value operation-order identity can be preserved with a finite normal-arithmetic fast path if required; extreme cases must be separated from bitwise identity claims.

## C. Actual next-step cap differs from the factor cap on informative output clipping (policy clarification)

`adaptive.rs:415-420` caps the factor to one, but `574-582` clears rejection_pending and may return `requested_h.max(trial_h*factor)`. After rejection, an informative clipped accepted trial with requested=1, trial=.75, error=.001 has predicted error below one. It restores h=1 and clears the pending flag, even with factor capped to one. Thus a contract saying the next actual step never grows after rejection is false for output landings. This is not a breach of the literal ALG05 registration, which explicitly preserves the old clipped-sample update; classify as semantics/contract gap, not evidence that its published FAIL should be rerun. Existing informative-clip test at `tests/alg05_predictive_capped2.rs:404-440` covers only predicted>1.

Suggested bounded test covers both sides of predicted=1 with informative clipping after rejection. Decide explicitly whether no-growth applies to the original controller request or the actual landed step. Preserve the required output schedule. A future controller claim should name that choice.

## D. A residual contamination ledger must cover every accepted outcome (accuracy research gap)

`gmres_staged.rs:437-453,550-568` admits `StallAccepted`/`FloorAccepted` when the true residual misses the requested target. `rodas5p_matrix_free_fast.rs:870-875` only adds contamination charges for `FallbackAccepted`. The relative roundoff term at 823-824 may also exceed the absolute per-stage eps target even for `Converged`. Consequently a future `sum_i tau_i*eps_i` budget cannot be inferred from acceptance labels or from G3's current fallback-only charge. This is an implementation of the stated ALG06 rule, not an accidental divergence. The already published ALG06 sensitivity result remains sufficient reason to hold promotion; no rerun needed.

Suggested port: every stage returns measured current residual, target, acceptance reason, and any amplification-bound provenance. Accumulate actual residual contributions for all accepted outcomes. For a global/step-budget guarantee use the assigned contamination budget, rather than adding a tiny charge only to a local error threshold of one. Add a synthetic controlled test that makes each acceptance reason exceed its nominal target, verifies accounting, and forces the new budget gate to reject. Gate initially on constant linear normal/dissipative cases with certified transfer bounds. General nonnormal/nonlinear cases remain research-only.

## E. Scalar sampled tau is neither a certified supremum nor a matrix gain bound

`rodas5p_matrix_free_fast.rs:274-281,313-325` describes tau as a supremum but implements a maximum over 1501 imaginary-axis points. Maximum modulus reduces a scalar analytic half-plane supremum to a full boundary supremum, not to a finite grid. Moreover scalar rational bounds do not extend to arbitrary nonnormal J or nonlinear stage Jacobians without extra structure. The computed smallest singular value of the Krylov triangular factor (`gmres_staged.rs:513-520`) is at best partial information: restricting a matrix to a subspace gives sigma_min(restriction)>=sigma_min(full), so its inverse is a lower estimate of the global inverse norm, not an upper certificate.

Suggested concrete next step: rename sampled quantities as estimates; introduce certified-vs-estimated gain metadata. For 2x2/very small systems use a validated full inverse/triangular bound; for verified dissipative matrices use the appropriate norm and resolvent inequality. A bounded fresh proof/counterexample can show why the current nu cannot certify unseen directions; no need to reproduce the published single-block VIG failure. Dense broad certificates, frozen-linear certificates and nonlinear defect propagation must have separate claim ceilings.

## F. Counted-work gains still need a native cost gate

The source correctly says no wall-time claim. New coupled work allocates `Mutex<Vec>` per stage (`matrix_free_fast.rs:836-840`), a new Vec per fallback (`844-848`), scales vectors without work-counter charges (`459-462`), and recomputes 1501-point tableau constants per new coupled workspace (`289-325,713-723`). `sigma_min` constructs a dense matrix and calls an SVD per evaluation (`gmres_staged.rs:335-346`); `4*j^3` is a proxy, not instruction count. These are engineering opportunities, not evidence that the reported JVP reduction is wrong.

Suggested next port after a correctness gate: workspace-owned scaling/fallback buffers, one coefficient-snapshot-bound transfer table per process/config, explicit scalar-vector/SVD counters, separate setup/steady-state measurement, and classify_guard_aborts disabled in timing. Keep modest n and PDE-size n strata distinct. Promote DupFix independently first if its pure fixed-linear-operator contract and native time/memory gate pass; it has no need to inherit the failed coupled-target gate.

## Lower-priority contract discrepancy

`matrix_free_fast.rs:807` uses `max(maxiter,restart)`, while public docs at 1643-1644 say the budget is maxiter. LinearSolverConfig validation permits maxiter<restart. This convention is inherited from legacy construction, so do not change one arm silently. Either document the effective budget or explicitly clamp/reject configuration consistently; one maxiter<restart test distinguishes it. Published campaigns used maxiter>=restart, so no rerun is warranted.

## Reuse decision

Reuse all published ALG01-ALG06 raw results and corrections as historical evidence. The new work justified by this source audit is finite-input fallback validation, controller extreme-arithmetic and clipping semantics, and typed total residual/gain contracts. These target gaps not exercised by old campaigns. No model/default promotion recommendation follows merely from these source findings.
