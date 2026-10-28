# Independent review of new solver fixes

Reviewer: `benchmark_audit`, which did not author the solver changes below. Review method: direct diff and call-path review, independent algebra for the resolvent identity and quartic expansion, inspection of hostile-test inputs and retained execution logs. This is a source review; native execution is attributed to the original workstreams, not re-run or counted twice here.

Decision: **no blocking correctness finding in the reviewed new fixes**, subject to the explicitly scoped diagnostic authority of B1d. This is not an independent approval of all historical modules or broad production promotion.

## B1d resolvent path and staleness guards

With `P=(h_P gamma)^(-1) I-J_P`, `W=(h gamma)^(-1) I-J`, and `R=(I-h_P gamma J_P)^(-1)`, direct multiplication gives

`I-P^(-1)W = R [(1-h_P/h)I + h_P gamma (J-J_P)]`.

The implemented weighted infinity-norm product bound is therefore valid in exact arithmetic for exact `abs(R)`. The refresh code solves each column of `P^-1`, then divides by `h_P gamma`, so the stored matrix has the intended units. Additional solves are charged before the solve call; failed calls remain charged. The old Jacobian is copied on refresh, the valid flag is cleared before rebuilding, and it is set only after all resolvent columns succeed. `StalenessRule::Resolvent` has separate driver IDs and no default facade routing change.

The implementation uses ordinary LU/float operations; its source and report correctly call it a diagnostic rather than an outward enclosure. There is no justified promotion to exact stage or global certificates. The HIRES experiment still misses the original factorization gate (0.441176 > 0.4), so the measured improvement cannot be used to erase B1c's FAIL. The invalid finite weight-ratio case is genuinely guarded: nonfinite or underflowed ratios are refused before `f64::max` can hide a poisoned row. Empty matrices and nonfinite gamma are also refused.

Reviewed tests: `b1d_resolvent_contracts.rs` uses explicit `T` multiplication and a nonnormal closed-form case; forced refresh checks same arithmetic as fast and extra-solve counts. `reaudit_staleness_guards.rs` targets the former `0 * infinity` / NaN-row masking defect. The retained native log records six active tests passing; the separate diagnostic run is labelled as such.

## Dense enforcement and represented curve

An estimator failure in enforcing mode now sets `accepted=false`, a classified failure kind and infinite controller error. The existing rejected branch restores Krylov recycle/history; collector, physical state and dense segment are committed only in the accepted branch. Reporting modes retain their previous noncontrolling behavior. The failure-injection test triggers a midpoint callback after a successful candidate, so it exercises the previously unsafe acceptance path rather than only a failed stage.

The power coefficients for the represented quartic are correctly

`[y0, y1-y0+d0, d1-d0, d2-d1, -d2]`.

Expanding the original blend gives the same polynomial. The derivative Horner loop yields its exact formal derivative and divides by the outward represented time span. Operation counters are incremented at each attempted interval primitive, preserving the actually reached failure prefix. The imported verified-output owner's scope is not interchangeable with arbitrary-callback dense output.

**Review clarification IR-M01:** the statement that attempted estimator work is retained applies to recoverable rejected attempts and returned runs. An unclassified fatal estimator error returns the existing plain `Err` before a `DriverPartial`/work ledger is returned. The author acknowledged this limitation and added it to the report. Moving a local counter update would not by itself make that error-path cost externally observable. No false acceptance remains from this case.

## C1 admission accounts

Updates are computed on a copied `BinState` and committed only after every cumulative floating account is finite. A rejected overflow leaves both existing bins and absent bins unchanged. This closes the `inf <= inf` guard failure without changing normal finite accounts. Configuration guards cover unrepresentable prior sums and quantile arithmetic. Public documentation properly requires actual work to remain in the separate transaction ledger when the admission account refuses.

The deterministic overhead statement still requires the caller's attempted/saved protected costs to represent the claimed counterfactual. The fixed-z Wilson proxy is not a rigorous Beta quantile bound or a UCB1 regret result. The audit report already distinguishes this; no broad regret or wall-time guarantee is accepted here.

## Facade events and finite differences

The signed perturbation fallback activates only when the positive FD shift overflows. All ordinary finite forward increments preserve their operation order and bits. At the positive finite boundary, the negative represented delta is used consistently in dense and banded quotients. Nonfinite Jacobian entries now fail before factorization. The exact power-of-two derivative at `±f64::MAX` is a useful independent oracle rather than a restatement of the difference implementation.

Event interpolation failure is now distinct from a nonfinite user event callback. The former silent endpoint-state substitution is removed. If an event state cannot be produced, events from that step are removed and the solution is truncated to the step start; earlier-step events are retained. Existing driver failure reasons are not overwritten. The reused crossing and next-state vectors retain capacity, and the two half-intervals fit on the stack. The order of event sampling, root refinement and stable event ordering is unchanged. The claimed allocation-request savings apply to fully scanned accepted steps, not to eliminated interpolation or root work. Terminal-event handling is still postprocessing, as documented.

## Review limits and release decision

This review does not run the native test suite again, audit every imported A01–A12 proof, certify generic floating callbacks, or approve benchmark code authored by this reviewer. The original workstream execution receipts provide test evidence. These new fixes close concrete failure paths and support scoped use; they do not establish global requested-tolerance accuracy, arbitrary-event completeness, or production speed superiority.
