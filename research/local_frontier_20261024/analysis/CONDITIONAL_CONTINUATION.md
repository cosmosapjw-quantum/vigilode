# Conditional fixed-P continuation: achievable accuracy and bounded work

This follow-on study uses the existing `fixed_point_continuation` kernel without changing default solver dispatch. It asks when an additional guard or a tighter stopping requirement improves reliability at modest cost. These are nonlinear root-continuation experiments, not ODE trajectory benchmarks. All declarations below are whole-domain mathematical provider obligations; successful samples do not verify an opaque provider's claimed bounds.

## Constructive error and work bounds

Fix a parameter station and a ball `B(c,r)`. Let `P` be a fixed invertible linear operator, and suppose `T(x)=x-PF(x)` is `q`-Lipschitz on the ball, `0<=q<1`. Write `a>=||P||`, `b>=||P^-1||`, and `beta>=||PF(c)||`. The condition `beta+qr<=r` gives a self-map and a unique root `x*` in the ball. These are the assumptions already required by the existing kernel.

Suppose the computed residual and inverse action satisfy

\[
\|\widehat f-F(x)\|\le e_F,\qquad
\|\widehat d-P\widehat f\|\le e_P.
\]

The computed update differs from `T(x)` by at most

\[
\eta=e_P+a e_F+\rho,
\]

where `rho` bounds rounding of `x-dhat`. If `eta<=eta_bar` uniformly and every iterate remains in the declared ball, then induction gives

\[
e_k:=\|x_k-x_*\|\le q^k e_0+
\bar\eta\frac{1-q^k}{1-q}.
\]

Indeed the triangle inequality gives `e_(k+1)<=q e_k+eta_bar`; summing the geometric series proves the statement. A sufficient condition that also keeps perturbed updates in the ball is

\[
\beta+qr+\bar\eta\le r.
\]

In a machine-enforced version the margin and error comparison must use outward arithmetic; the displayed planning values in this experiment are not an admission check. Without that extra margin the exact self-map theorem alone does not guarantee floating iterates remain inside. The current kernel still checks each represented iterate and refuses an excursion transactionally. It does not silently project an invalid iterate back onto the ball.

Since `PF(x)=(x-x*)-(T(x)-T(x*))`,

\[
\|F(x_k)\|\le b(1+q)e_k=:L e_k.
\]

For exact evaluation, `eta_bar=0`, so `e0<=2r` yields the explicit sufficient bound

\[
k\ge\left\lceil\frac{\log(2rL/\tau)}{-\log q}\right\rceil
\]

when `q>0` and the logarithm is positive. For `q=0`, one exact update reaches the root. With errors, the runtime residual upper bound is `Rhat=||fhat||+eF`, and `Rhat<=||F(x)||+2eF`. Consequently, with uniform `eF<=epsilonF`, a sufficient criterion is

\[
L\left(q^k e_0+\frac{\bar\eta}{1-q}\right)
+2\epsilon_F\le\tau.
\]

If `d=(tau-2 epsilonF)/L-eta_bar/(1-q)>0`, it is enough that `q^k e0<=d`. The resulting logarithmic bound predicts a finite per-station call budget: `k+1` residual and `k+1` inverse calls, including the final check. A failure of this sufficient inequality is **not** proof of impossibility. The unconditional practical impossibility test is narrower: if every residual callback reports a known lower bound `eF>=e_min>tau`, no residual upper bound can pass. One large sampled error bound does not establish such a uniform lower bound.

The executable's logarithmic count is a planning estimate computed with ordinary floating arithmetic. It is never used to admit a root or reject a mathematically feasible solve. The actual finite caps, residual enclosure, domain checks and final original residual remain authoritative.

## Root error is a separate requirement

The existing kernel supplies the conditional a posteriori bound

\[
E=\frac{\|\widehat d\|+e_P+a e_F}{1-q}.
\]

This follows from `||x-x*||<=||PF(x)||/(1-q)` and the action/residual enclosures. It does not include update-rounding error separately: it describes the root distance of the current represented point, regardless of how that point was obtained.

If `Rhat<=tau`, then

\[
E\le\frac{a\tau+2e_P}{1-q}.
\]

Thus a requested root tolerance cannot generally be replaced by an identical residual tolerance. The optional policy reserves half of a root-error budget for action errors by reducing the working residual tolerance to `min(tau,(1-q)root_tol/(2a))`. It then checks the **actual returned** `E<=root_tol`, retaining a private candidate until both requirements pass. This final check is necessary because the reservation alone does not bound an arbitrary provider's `eP`. A refusal preserves the caller's output.

## Practical policy for a supplied atlas

1. Retain the existing target, scaling, model-epoch, inverse-identity and branch checks. A policy may choose only among independently justified providers/patches; it cannot replace `P` and reuse the previous declaration.
2. If a provider supplies an independently established uniform lower bound on its reported residual uncertainty, reject a smaller requested tolerance before correction. This saves futile work without declaring the mathematical problem unsolvable.
3. At the first parameter station, use the caller's initial iterate only when it lies strictly inside the first declared ball. Otherwise use that ball's center. Banach uniqueness proves that this fallback still targets the same patch root. The kernel rechecks the chosen point. This rule is restricted to initial admission.
4. Across patches, preserve the prior root-enclosure inclusion test. Never use a new patch's center to bypass a failed branch-overlap test. Two disjoint roots can share identical labels; the negative test exercises exactly this situation.
5. If the user supplies a root-error tolerance, tighten the residual work target and independently check the returned root bound before publishing output. A tighter contract may cost more; record that cost rather than calling it a speedup.
6. Use the geometric estimate to propose a work cap, but charge all attempted calls and preserve failures. A `q` close to one is a reason to seek a better independently validated `P` or another algorithm. It is not evidence that continuation is impossible.

Splitting a regular domain into more patches can improve local bounds and enable branch tracking, but each added station has a cost. The two-patch case in this study is tested rather than assumed faster. Sequential parameter dependence remains sequential here; there is no stage/time parallelism claim.

## Implemented experiment

Executable: `crates/rodas5p-integrators/examples/conditional_continuation_sweep.rs`.

```
cargo run --profile measurement --locked --offline \
  -p rodas5p-integrators --example conditional_continuation_sweep -- --repeats 31
```

`--list` emits all cases; `--case ID` selects one; repetitions are bounded by `1..10000`. Output is JSONL with `header`, `result`, and `guard` records. Default configuration has 16 numeric cases plus a branch-refusal guard. Serial callback execution is used throughout.

The main fixture is `F_i(x,lambda)=x_i+kappa*x_i^3-[1/4+lambda/2+i/(128n)]`, `kappa=1/64`, `n=1` or `128`. Its derivative is positive, so a scalar bracket is an appropriate strong comparator. On `[0,1]`, `P=I` gives `q=3/64`; the deliberately weak choice `P=I/16` gives `q=15/16`. Both inverse providers have valid fixed-P self-map declarations; their different work is caused by their different contraction quality. The runtime only evaluates F and fixed P, with no derivative callbacks.

The sweep compares the existing kernel with an opt-in wrapper implementing the policy above. It includes tolerances `1e-6` and `1e-10`, center versus target-proxy initialization, two regular patches, finite iteration/cumulative-call caps, an actual deterministic residual bias enclosed by a declared uncertainty, impossible reported-error floors, and a separate root-error requirement. The scalar quadratic guard presents two individually contractive patches around opposite roots; their common branch label does not defeat overlap refusal.

Bisection solves the **final target only**, whereas continuation solves the initial and final parameter stations (and the midpoint in the two-patch case). Bisection uses the same residual uncertainty, avoids a sign update when its residual interval contains zero, and has an explicit call cap. This is intentionally a stronger method for the separable fixture; it is not a replacement comparison for general coupled continuation. The JSON field `speed_comparison_eligible` refers only to the raw-versus-conditional pair under the same requested contract. A bisection ratio is descriptive unless `bisection_final_only.success` is also true and its narrower final-only scope is disclosed; failed comparators and refused continuation rows must never enter a speed ranking. All eligible final-run pairs happened to have successful bisection, but consumers must check this explicitly. The tight diagnostic reference uses unperturbed polynomial evaluation and a `2e-15` stopping threshold. It is a binary64 check, not an independent interval root certificate.

All solve allocations and policy guards are timed. Model/atlas construction, the tight diagnostic reference, diagnostic error comparisons and JSON construction are excluded for every arm. Orders are rotated across repetitions. Revision 1 accidentally included asymmetric result/diagnostic construction in timers; revision 2 removed this. Revision-1 raw evidence and source are retained, and revision 3 additionally bound the inverse identity to the actual P value and tested stale inverse reuse. Only revision-3 times support the final conclusions.

## Validation and measured outcomes

Five targeted tests cover floor/initialization/root-budget policy, finite work caps, disjoint branches, actual residual/reference-error checks across every default case and both policies, and refusal of a changed inverse using an old atlas before any callback. The original kernel's previously published 15 tests were not redundantly rerun because its source did not change. New study used three correction rounds, within the three-round bound.

The final run used 31 repetitions of three arms on 16 cases (1,488 timed solves) plus one disjoint-branch guard. All five tests and every runtime assertion passed. Routine same-contract conditional/raw median ratios ranged from approximately 1.000 to 1.094; this is an observed range, not an overhead guarantee. The difficult inverse used 339 and 621 vector residual calls at tolerances `1e-6` and `1e-10`, compared with 8 and 12 for `P=I`. The latter vector cases were approximately 1.29 and 1.49 times faster than final-only bisection; scalar fixed-P remained slower than its appropriate bracket comparator.

The impossible-reported-error case changed from 33 futile F calls to immediate refusal with zero callbacks. The outside initial point changed from domain refusal to a valid result in 9 F calls. Root-error refinement reduced the returned conditional root bound from `1.2764e-7` to `2.3724e-12`, at 13 rather than 8 F calls and a measured 1.63 time ratio. This is a useful accuracy option when requested, but not a claim that a stronger accuracy requirement is free or costs less than 25%. The two-patch version used 12 rather than 8 F calls; retain it for domain/branch reasons, not as a speed optimization.

Final measurements and decision rows are in `CONDITIONAL_CONTINUATION.json`, with raw JSONL and hashes under `evidence/conditional-continuation/`. This report makes no universal slowdown bound: it measures the overhead on these fixtures and gives a formula for the iteration cost. Sub-microsecond scalar times and one-process repetitions require independent-process local confirmation before any performance promotion.

Recommendations are conditional: retain cheap early floor refusal and initial center fallback; enable root-error refinement only when that error contract is requested; prefer a well-conditioned verified inverse; reserve multi-patch continuation for a demonstrated domain/branch need; use a scalar bracket when its stronger special structure is available. None of these results changes broad production readiness or default ODE dispatch.

The inexact-contraction, residual, and error-floor arguments received an independent algebraic review from the geometry-transfer worker. That review confirmed the sufficient conditions, emphasized outward comparisons for any future admission rule, and retained the narrower interpretation of reported-error-floor refusal.
