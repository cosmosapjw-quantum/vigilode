# RHS-only homotopy: theory and native coding loop

The subsequent user-directed exact-constant extension is documented in
`EXACT_CONSTANTS.md`. It adds ResidualDominance (eta=1), a sealed polynomial
physical-accuracy planner and outward model validation. Its newer execution
records are `EXACT_CONSTANTS_EXECUTION.json` and `ROUNDING_FLOOR_TEST.json`;
the earlier heuristic comparisons below remain source-bound controls.

## Result

A new native `rhs_homotopy` module solves a supplied fixed-point target using
only map values, vector arithmetic and a scalar depth-one extrapolation. It
never requests Jacobians, finite-difference derivatives, JVP, AD, a dense
matrix, a factorization, or a Newton correction. The caller's map remains
responsible for its own implementation and cost: putting a derivative-based
solver inside that callback does not make that callback derivative-free.

The module is exercised on actual implicit-midpoint RK stage equations for a
nonlinear ODE, on its full time-grid trajectories, and on an explicit RK4
causal-stage target. The RK method and its order are unchanged by the root
solver. A new generic integrator or production-default dispatch is outside
this module's implementation boundary.

Source baseline is PR90 `68a28357d960c4c2743c299770f0a33ef63245b2`.
The implementation is `crates/vigilode-coupled/src/rhs_homotopy.rs`, with
`tests/rhs_homotopy_contracts.rs` and `examples/rhs_homotopy_bench_20261027.rs`.
`PREREGISTRATION.md` was written before the first native experiment.

## Earlier ideas retained and changed

| Earlier work | Actual reused idea | New concrete step |
|---|---|---|
| PR74 coupled all-stage homotopy | One original stage target and explicit continuation parameter | Generic value-only map with final lambda=1 root admission |
| PR75 residual-first correction | Evaluate the target before spending derivative/update work; retain failed work | Evaluate the Picard candidate first; omit extrapolation after it already passes |
| Current `causal.rs` | Causal targets can have finite propagation independently of a global contraction constant | RK4 negative control compared to the exact four-call forward target |
| October 20 RHS-only research packet | Strong derivative-free distinction, direct original-model residual, retain negative controls | Actual Rust direct-RHS nonlinear RK-stage consumer and exact ODE observations |
| Existing stage warm starts | Reuse already accepted information from the previous step | Same-h previous-stage seed is measured as a separate trajectory arm |

The old `actual_k_homotopy.rs` constructs a frozen Jacobian and inverse witness;
the old projected coupled homotopy requests JVP and factors a reduced system.
Those remain useful separate paths. Their names do not supply the stronger
value-only contract implemented here. The original derivative-free packet is
`research/cumulative_20261020/original_sources/derivative_free/` and retains its
own `c03ca90f...` source identity and prior bounded-experiment scope.

## R1–R5: target, sources, construction and proof

Let `G:D->D` be a fixed-point map on a closed convex subset of Euclidean space,
and let `a in D` be the initial anchor. At a continuation station define

\[
G_\lambda(x)=(1-\lambda)a+\lambda G(x),\qquad
r_\lambda(x)=G_\lambda(x)-x,\qquad 0<\lambda\le1.
\]

The station list is strictly increasing and ends at exactly one. A completed
station supplies the next seed, and extrapolation history is cleared at every
station change. These are different residual maps and must not share secant
history without a separate derivation.

At an accepted iterate `x`, first form and evaluate the Picard candidate
`p=G_lambda(x)`. If its original-map residual meets tolerance, return it without
another map call. Otherwise, using the previous accepted pair `(x_old,r_old)`,
form

\[
\Delta r=r-r_{\rm old},\quad
\gamma={\langle r,\Delta r\rangle\over\|\Delta r\|^2},\quad
e=G_\lambda(x)-\gamma\bigl(G_\lambda(x)-G_\lambda(x_{\rm old})\bigr).
\]

The code checks coefficient finiteness/magnitude and domain membership, then
evaluates `r_lambda(e)` on the original map. It adopts `e` only when

\[
\|r_\lambda(e)\|_2\le\eta\|r_\lambda(p)\|_2,\qquad0<\eta\le1.
\]

Every rejected speculative callback is included in work. An observed callback
error is fatal, including one at an optional candidate. Two failed trial
screens disable further trials at that station in the default configuration.
`SlowOnly` additionally requires the observed ratio of the last two accepted
residuals to be at least 0.5. This is an empirical work heuristic; the original
residual screen remains the acceptance test.

### Conditional residual convergence theorem

Suppose `G` is Lipschitz with constant `q`, `lambda q<1`, and the actual
accepted and evaluated trial states lie in the common domain above. The
homotopy map has a unique fixed point. For each update of the screened method,

\[
\|r_\lambda(x_{k+1})\|\le\lambda q\|r_\lambda(x_k)\|,
\qquad
\|x_k-x_\lambda^*\|\le
{\|r_\lambda(x_k)\|\over1-\lambda q}.
\]

**Proof.** Convexity preserves the selfmap property. Banach's theorem gives
the unique root. Since `p=G_lambda(x)`,

\[
\|r_\lambda(p)\|=
\|G_\lambda(G_\lambda(x))-G_\lambda(x)\|
\le\lambda q\|G_\lambda(x)-x\|.
\]

The selected trial has residual no greater than that of `p`, so the first
inequality follows. The triangle inequality and contraction at the root give
`||x-x*|| <= ||r(x)|| + lambda q ||x-x*||`, proving the second. The particular
extrapolation formula and coefficient cap are not needed for this convergence
argument; they select inexpensive trial vectors. For an affine scalar map,
the depth-one trial is its exact root when the denominator is nonzero and the
coefficient is admitted. Wolfram simplified that identity to zero.

If a true residual differs from the measured residual by at most `sigma`, the
root-error inequality instead uses `(measured residual+sigma)/(1-lambda q)`.
The present module returns operational binary64 residuals and does not
construct a callback-rounding enclosure. Its contract states this explicitly.

### Actual nonlinear RK target, with a stronger inverse bound

For `y'=-k y-g y^3`, `k>0`, `g>=0`, the implicit-midpoint stage increment solves

\[
F(K)=K+h\{k(y+K/2)+g(y+K/2)^3\}=0.
\]

The direct callback is `G(K)=-h[k(y+K/2)+g(y+K/2)^3]`. It uses exactly one
vector RHS evaluation, with no derivative or matrix operation. With anchor
zero, the homotopy equation is `F_lambda(K)=K+lambda h Phi(y+K/2)=0`. Its
derivative satisfies

\[
F_\lambda'(K)=1+\frac{\lambda h}{2}
\{k+3g(y+K/2)^2\}\ge1+\lambda hk/2.
\]

Consequently each scalar component has one global root, bracketed by `[-2y,0]`
for `y>=0`, and

\[
|K-K_\lambda^*|\le
{|F_\lambda(K)|\over1+\lambda hk/2}.
\]

This monotone inverse estimate also applies when the raw Picard map is not a
contraction. For vectors of independent components, apply it componentwise
and combine squared errors. The validation oracle uses monotone bisection,
independent of the accelerator. The exact continuous ODE solution is

\[
y(t)=\frac{y_0e^{-kt}}{\sqrt{1+(g/k)y_0^2(1-e^{-2kt})}}.
\]

Wolfram verified its differential-equation residual as zero. The symbolic
receipt preserves an undefined-symbol warning emitted by the tool; all three
requested final symbolic outputs were returned.

## R6–R9: independent review and executed evidence

The separate `theory_derivative_free` agent reviewed the actual implementation:
the trial formula, history reset, residual reuse, normalized dot product,
already-converged baseline shortcut and failed callback accounting. No
correctness defect was found in that review. This is a separate native agent
with inherited context, not an independently selected model or formal proof.
The theory lane strengthened the midpoint inverse bound shown above.

SciSpace and Exa retrieved primary Anderson literature. Walker–Ni (2011),
DOI `10.1137/10078356X`, provides the fixed-point acceleration framework;
Toth–Kelley (2015), DOI `10.1137/130919398`, analyzes contractive-map convergence.
The present residual-dominance safeguard is proved directly above. We claim
the native project connection and measured behavior, not a new discovery of
Anderson acceleration. Tool outputs are retained in `EXA_RECEIPT.json`,
`SCISPACE_RECEIPT.json`, `WOLFRAM_CONTEXT.json`, `WOLFRAM_CHECK.json`.

### Same-target RK-stage work

All successful rows meet residual L2 `1e-12` and independent component root
error `1e-10`. Counts are vector RHS callback entries including screens.

| RK stage case | Picard | Screened | SlowOnly |
|---|---:|---:|---:|
| Scalar linear, hk/2=0.1 | 13 | 4 | 13 |
| Scalar linear, hk/2=0.99 | 2819 | 4 | 4 |
| 64-vector linear, hk/2=0.99 | 3029 | 4 | 4 |
| Scalar cubic, k=1,g=.5,h=.5 | 37 | 12 | 35 |
| 64-vector cubic, k=10,g=.5,h=.18 | 495 | 80 | 16 |
| Scalar linear, hk/2=1.1 | 4097, update limit | 4 | 4 |
| Scalar cubic with lambda=.25,.5,1 | 72 | 28 | 72 |

Twenty of 21 stage cells return accepted roots. The failed Picard row is
retained; it establishes behavior of that particular iteration on that input.
The successful accelerator there is checked by the monotone root bound and
bisection, not by a contraction assumption that the input violates.

The coarse near-unit-contraction rows compare **the same RK stage equation**;
their reduction in root work is not a matched physical-accuracy ODE speedup.
One-step ODE errors are included separately in `RESULTS.json`.

### Causal and continuation overhead

For the classical RK4 causal target at h=.25, plain all-stage Picard uses 20
RHS calls, screened extrapolation 36, and SlowOnly 20. All stage vectors equal
the four-RHS sequential target. Direct causal elimination is therefore the
better work route for this input. The result motivates skipping a generic
root accelerator when the target already has a cheap exact causal schedule.

Likewise, intermediate homotopy stations on the easy nonlinear scalar target
increase screened work from 12 to 28. Continuation is available for branch
tracking and more difficult maps; it is not unconditionally cheaper. SlowOnly
also misses useful acceleration in the fast scalar cases, while it substantially
outperforms always-screened extrapolation in the heterogeneous nonlinear vector
case. These results do not support one globally optimal threshold.

### Actual nonlinear ODE trajectories

Every returned grid state is scored against the exact solution at T=1.
The same-h previous accepted stage is the only warm-start information used.

| Steps | Zero-seed Picard RHS | Warm-seed Screened RHS | Max grid error, both arms |
|---:|---:|---:|---:|
| 8 | 99 | 58 | 9.19602e-4 |
| 16 | 153 | 99 | 2.29895e-4 |
| 32 | 251 | 181 | 5.74519e-5 |
| 64 | 424 | 296 | 1.43644e-5 |

The last comparison reduces actual RHS calls by 30.19%. Separate arms in
`SUMMARY.json` disentangle warm-start reuse from extrapolation: at 64 steps,
warm Picard uses 377 and zero-seed Screened 384. All 24 trajectories complete;
the error-halving slopes remain approximately two, consistent with the
unchanged implicit-midpoint method. These are measured callback savings, with
no wall-clock performance claim in this experiment.

`analyze_results.py` independently reconciles callback counters, re-evaluates
all trajectory observations, checks all root caps, and evaluates represented
stage roots in Decimal precision 80. Its maximum exact-polynomial residual is
`9.9313441892400763464569623820365e-13`. That computation is an independent
high-precision diagnostic, not an outward-rounded interval certificate.

## C1–C10 execution and closeout

| Stage | Actual completion evidence |
|---|---|
| C1 baseline | PR90 source and explicit fixed target/caps in preregistration |
| C2 formula to code | `G_lambda`, cached values/residuals, `evaluate`, `run`, RK stage callbacks |
| C3 numerical design | Screened AA1, station history reset, strict callback/domain/epoch contracts |
| C4 execution | Shared cargo flock, cached Rust1.94.1 measurement profile, jobs2, serial deterministic run |
| C5 implementation | Public native module, 11 contracts, actual RK stage/trajectory example |
| C6 implementation checks | 11/11 native contracts PASS, fatal/speculative callbacks, partial writes, epoch, budget, invalid schedule |
| C7 science checks | Nonlinear midpoint monotone oracle, exact RK4 causal root, exact continuous ODE observations |
| C8 numeric/reproducibility | 21 stage cells, three RK4 cells, 24 trajectories, retained failures, source/binary hashes and raw logs |
| C9 review | Separate theory agent code/proof review plus independent Decimal and symbolic checks |
| C10 result | Public module retained as opt-in; work benefits and expensive cases explicitly recorded |

The theory stages R1–R5 are the source/target/construction/proof sections above;
R6–R9 are the review, symbolic/native discrimination, decision and code mapping;
R10 is this retained result and next obligation. No new strong no-go theorem is
admitted, so the seven strong-no-go novelty criteria do not support any broader
exclusion claim from the failed cell.

Two productive repairs are preserved. Initial test compilation needed the
explicit `sum::<u64>()` type because `serde_json` supplies another equality
implementation; the first compiler output is in `initial_type_inference_*`.
The first negative-control callback unnecessarily evaluated `0 * y^3`; once
`y^3` overflowed, that produced `0 * Inf`. The zero coefficient is now removed
algebraically before evaluating the cubic. Original outputs remain in
`initial_zero_term_*`; only the example was rebuilt/rerun. The final negative
control reaches its honest bounded Picard update limit. Production solver and
the 11 contracts were unchanged by this example repair.

### Reproduce and next integration point

`run_native.py` runs the exact focused test, build and example commands with
locked offline dependencies. `analyze_results.py` checks the returned results.
The source/binary hashes and real exit statuses are in `EXECUTION_RECEIPT.json`
and `EXPERIMENT_REPLAY_RECEIPT.json`. The final example replay is authoritative
for the experiment; the first receipt remains authoritative for unchanged
native contracts and module source.

The next useful integration is a family-specific direct-RHS/structural stage
map whose true residual-to-output bound is supplied by the causal majorant or
monotone inverse estimate, and whose policy compares complete attempted work
to the current causal/structural path. Selective JVP/AD belongs only to cohorts
where that fully value-only path demonstrably loses the relevant cost/accuracy
comparison. A universal policy requires additional model evidence; it is not
inferred from the few scalar/vector cases here.
