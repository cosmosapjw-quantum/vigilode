# A05: sealed PR101 continuous-output implementation

The cloud completion supplies a native PR101 model owner, cancellation-preserving continuous defect verifier, and an opt-in adaptive RODAS5P driver. The driver completes the original 101-output stiff nonlinear witness with unchanged `atol=1e-11`, `rtol=1e-8`. Every returned binary64 output has an outward radius at most `atol`; every accepted exact represented quartic has a continuous ODE error bound at most `atol/2`. This establishes the named model route under the existing directed-primitive contracts. The generic `OdeProblem` driver still needs its own model-specific interval provider.

## Source and evidence identity

Baseline `c513f47` contains the old PR101 independent review and execution logs but contains **neither** `bound_pr101.rs` nor its claimed provider/oracle test files; `git log --all -- path` also returns no source history for that path. Therefore this is a new implementation and new proof review. Earlier source-review passes are not attached to these bytes. Existing immutable quartic representation and directed arithmetic contracts are reused.

The previous `continuation_a05_pr101_provider/INDEPENDENT_REVIEW.json` imposed a one-repair-episode limit. The current user's instruction explicitly authorizes finishing feasible proof and native coding here. This replaces that operational cap for this unit; no scientific acceptance criterion is relaxed. Historical review and failed source authority remain intact.

Files:

- `crates/rodas5p-integrators/src/bound_pr101.rs`: new model, verifier, output owner and driver.
- `crates/rodas5p-integrators/src/dense_output_v2.rs`: private scalar power-coefficient extraction; individual attempted-primitive charging.
- `crates/rodas5p-integrators/tests/production_audit_pr101_cloud.rs`: end-to-end and transactional tests.
- `crates/rodas5p-integrators/tests/pr101_cloud_fraction_oracle.py`: independent exact rational oracle.
- Existing `a05_dense_global_output.rs`: strengthened overflow ledger check.

## Contract and mathematical result

Let

\[
f(t,y)=\lambda(y-\sin t)+\cos t+\eta(y-\sin t)^2,
\quad 0\le t\le1,
\]

where the finite binary64 parameters represent exact real constants and the mass is identity. The initial mathematical solution satisfies `|y(t0)-initial_center| <= initial_radius`. There is no claim that a caller-supplied zero radius covers an unknown rounded initial condition. For the standard witness, `t0=y0=0` are exact. For the nonzero-start test, the supplied `1e-15` initial disk covers the reference construction.

An immutable quartic segment defines the exact curve

\[
q(\theta)=(1-\theta)y_0+\theta\{y_1+(1-\theta)(d_0+\theta(d_1+\theta d_2))\},
\quad\theta=(t-t_0)/h.
\]

The coefficients and endpoints are stored binary64 values interpreted exactly. Admission requires that `t1-t0` be an exact positive normal binary64 value; this same `h` defines both differentiation and evaluation. Interval power coefficients are

\[
q_0=y_0,\quad q_1=y_1-y_0+d_0,\quad q_2=d_1-d_0,
\quad q_3=d_2-d_1,\quad q_4=-d_2.
\]

Every nontrivial arithmetic transformation is outward.

### Cancellation-preserving defect

A degree-24 Taylor expansion about zero encloses `sin(t0),cos(t0)` using a remainder `t0^25/25!`; finite factorial ratios are formed with interval division, without first rounding `1/k!`. Derivatives of both trig functions have magnitude at most one. These certified centers generate degree-12 local polynomials `S(theta),C(theta)` with uniform remainder

\[
\rho=h^{13}/13!.
\]

All powers/factorials and coefficients are outward intervals. Set `Z=q-S`, and form the polynomial

\[
P=q'/h-C-\lambda Z-\eta Z^2
\]

**coefficient by coefficient before taking magnitudes**. This preserves the cancellation between the candidate and the oscillatory forcing even when `|lambda|` is large. The physical derivative here means `q'` differentiates with respect to theta. With `B=sum_k mag(Z_k)`,

\[
\sup|q_t'-f(t,q)|\le
\sum_k\operatorname{mag}(P_k)
+|\lambda|\rho+|\eta|(2B\rho+\rho^2)+\rho=:r.
\]

Proof: write `sin=S+epsilon_s`, `cos=C+epsilon_c`, with both errors bounded by rho. The exact defect differs from P by `lambda epsilon_s +2 eta Z epsilon_s-eta epsilon_s^2-epsilon_c`; triangle inequality gives the stated remainder. On `theta in [0,1]`, each coefficient magnitude sums to a valid polynomial supremum bound.

This avoids interval dependency loss in separately evaluating `q_t'` and a large stiff RHS box. For the dyadic linear curve `q(t)=t`, `h=2^-10`, `lambda=-10^6`, `eta=10`, the returned residual is `1.55697273700166152e-4` and uniform radius `1.55697273700169781e-10`. The independent Fraction oracle gives whole-interval exact curve error at most `1.552204217010931e-10`.

### Continuous global transport

On the candidate-centered tube `|y-q|<=R`,

\[
f_y=\lambda+2\eta(y-\sin t)
\le\lambda+2|\eta|(B+\rho+R)=:\mu.
\]

For scalar error `e=|y-q|`, the mean value theorem gives the upper Dini derivative inequality `D+e <= mu e+r`. Its comparison solution is

\[
v(s)=e^{\mu s}e_0+r\int_0^s e^{\mu u}\,du.
\]

This function is monotone (its derivative has the sign of `mu*e0+r`); therefore `max(e0,v(h))<=R` proves the closed tube on the full segment by comparison and continuation. Local existence is sufficient: the verified bounded tube prevents finite-time escape before the segment end for this polynomial state field. The endpoint radius `v(h)`, not its whole-segment maximum, is carried to the next segment.

The signed integral uses outward `(exp(mu*h)-1)/mu` and a stable bound `h` for `mu<0` or `h*exp(mu*h)` for `mu>0`. Taking the minimum of two upper bounds preserves enclosure. If the signed quotient overflows, the valid coarse bound is used. If an essential exponential or final sum cannot be represented, the trial refuses.

Induct over accepted adjacent segments: the previous endpoint bound encloses the next initial error; `q_next(0)` is the exact same stored endpoint state. Thus each accepted segment has a bound relative to the original initial value problem, not a newly initialized local solution.

### Binary64 output centers

The sealed verified owner stores the precise segment used in the proof. To emit a binary64 center `c`, it recomputes an outward interval `Q` for that same polynomial at the exact requested time. The returned radius is

\[
R_{out}=R_{uniform}+\operatorname{mag}(Q-[c,c]).
\]

Consequently `|y(t)-c|<=Rout`. Requested times are compared exactly, including adjacent representable timestamps. The driver accepts only if every pending requested output satisfies `Rout<=atol`. The mathematical curve uses `Runiform<=atol/2`, which reserves room for output evaluation rounding. This absolute rule is stronger than the unchanged `atol+rtol*|y|` target. Pure relative tolerances are explicitly refused until a verified scale is implemented.

## Transaction and ownership

`Pr101Model` owns private immutable `(lambda,eta)`. Candidate RHS/JVP/Jacobian/time-derivative callbacks are built inside this owner; no exact solution callback is attached. The candidate generator's floating trigonometric calls do not require certification: their generated curve is independently checked against interval trigonometric enclosures.

`verify_segment` accepts any scalar candidate curve because it recomputes the full defect against its target. It does not assume the curve was generated by a matching callback. A private `Pr101VerifiedSegment` couples the model proof radius with its precise curve. Diagnostic bound copies cannot replace the stored bound. Verified segment/output owners cannot be deserialized or edited through public fields.

Each trial stages its verified segment and all requested outputs in local temporary values. Only complete verification commits time/state/inherited radius/output cursor/segment/output arrays. Rejected work remains in both solver and verification ledgers. The scalar direct driver has no recyclable Krylov state or multistep/controller history requiring rollback. Its step-size rule halves rejected intervals and grows accepted intervals by 1.5, subject to the existing represented-clock cap. A per-trial tube reserves global-error headroom according to elapsed-time fraction and never exceeds the fixed `atol/2` budget. This is a scheduling policy, not a universal completion guarantee for every coefficient/initial state. An interrupted run returns only the verified accepted prefix and a failure reason.

The initial driver is deliberately separate from generic `solve_stiff`; existing generic production behavior is unchanged. It stores accepted segments (linear memory in segment count) and allocates candidate/interval-output vectors. No zero-allocation or wall-clock speed promotion is claimed.

## Deterministic arithmetic work

Each provider arithmetic API primitive is charged immediately before its attempt; no later call is precharged across a fallible call. Point constructors, comparisons, copies and integer loop control are excluded from this provider arithmetic count; its unit is explicitly an API arithmetic operation, not a CPU instruction or FLOP. Dense construction includes copied identity items, so summed work is a structural ledger.

Successful contracting verification has exactly 639 provider operations:

| Part | Operations |
|---|---:|
| t0 trigonometric enclosure | 78 |
| local trigonometric coefficients | 53 |
| q-S coefficients | 13 |
| difference polynomial norm and remainder | 27 |
| derivative/linear polynomial | 47 |
| nonlinear convolution | 351 |
| residual polynomial norm | 50 |
| trig tail propagation | 8 |
| tube log norm | 4 |
| contracting transport | 8 |

The polynomial square uses the exact identity `sum_ij z_i*z_j = sum_i z_i² + 2 sum_{i<j} z_i*z_j`. For 13 coefficients, the old ordered loop requires `3*169=507` primitive calls; 13 diagonal terms use 3 each and 78 off-diagonal terms use 4 each, totaling 351. This saves exactly 156 attempted primitive calls on each successful verification through this block, without changing degree or acceptance targets. In the original witness, all 784 verifications traverse it, saving 122304 structural operations. The proof guarantees this operation-count reduction; it does not assert an identical wall-time ratio or uniformly narrower rounded intervals.

The independent failure contracts require 2 attempted operations when `exp(1000)` refuses, 3 when the final sum overflows after two successful products, and exactly 14 dense derivative operations when its first final sum overflows (the later add/div are unexecuted). Prior grouped dense arithmetic charges are replaced with individual calls.

## Actual validation

- `pr101_cloud_final.log`: 7/7 focused tests pass; original nonlinear PR101 witness returns 101 outputs, 492 accepted and 292 rejected attempts, maximum sampled reference error `8.38218383591993188e-15`, 6272 RHS evaluations, 574094 structural verification operations. This is an accuracy/contract run, not a fair speed benchmark.
- `pr101_fraction.json`: 131 rational checks; independent entire-interval linear-fixture error bound from monotonicity `d(t-sin t)/dt=1-cos t>=0`; residual at 65 exact rational points. The proof above provides general interval residual coverage.
- `a05_dense_cloud.log`: 15/15 existing A05 contracts pass with strengthened precise early-refusal work expectation.
- `pr101_ledger.log`: 3/3 primitive-refusal/branch unit tests pass.

Commands:

```sh
source ../runtime/env.sh
cargo test --locked --offline -p rodas5p-integrators --test production_audit_pr101_cloud -- --nocapture
cargo test --locked --offline -p rodas5p-integrators --test a05_dense_global_output
cargo test --locked --offline -p rodas5p-integrators --lib bound_pr101::tests
python3 crates/rodas5p-integrators/tests/pr101_cloud_fraction_oracle.py research/cloud_completion_20261018/evidence/pr101_cloud_final.log
```

The independent reviewer suggested an actually nonzero nonlinear orbit: `lambda=0, eta=1, y(0)=1/16`, whose independent exact reference is `sin(t)+1/(16-t)`. Its first execution exposed **global-budget starvation**, not an incorrect certificate: a full constant tube could be consumed early and positive transport prevented further acceptance. The exact failed log is retained as `pr101_positive_orbit_failure.log`. The elapsed-time reservation repairs this case without changing any acceptance tolerance. The final 17-output expanding-orbit test passes. A native overflow test additionally checks that a candidate's internal disposition cannot double-count an outer rejection; only disposition counters are reconciled, while all attempted cost remains.

The runnable example `examples/pr101_verified_run.rs` emits machine-readable solver work, verification work, all 101 centers/radii, tolerances and success status. Its actual debug execution is `evidence/pr101_run.json`; build/run log is `pr101_example_build.log`.

## Next local execution

The implementation, exact oracle and small tests are complete here. Local work can run the commands and a release-profile PR101 experiment. Further generic A05 promotion requires additional concrete model owners/providers and insertion into each relevant generic driver's transaction, including its state/history/cache. A tighter verified relative scale and adaptive Bernstein residual bounds can reduce this conservative driver's accepted-step count; their acceptance must retain the current continuous/output enclosures and account for their added work.

Local ready command (release timing is a fresh measurement, not a rerun prerequisite for the proof):

```sh
cargo run --locked --offline -p rodas5p-integrators --example pr101_verified_run --release > pr101_run.json
```

`output.rs` retains the separately tested generic output-staging helpers for the next generic-provider integration. Narrow `cfg_attr(not(test), allow(dead_code))` annotations explain their intentionally pending library use; test builds retain ordinary dead-code checking. No staging tests or generic-driver authority are removed.
