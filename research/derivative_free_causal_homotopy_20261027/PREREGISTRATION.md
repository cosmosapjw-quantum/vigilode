# Derivative-free causal / homotopy research, 2026-10-27 KST

Baseline: PR90, commit 68a28357d960c4c2743c299770f0a33ef63245b2.
Branch: research/derivative-free-causal-homotopy-20261027.
User-selected full theory and coding loops use the supplied PhysMath 1.0.0
contract. The plugin catalog also exposes 1.3.0; the explicit user-supplied
instructions govern this run. Prior completed audits and experiments are
reused at their original scope.

## Production boundaries and new construction

1. Causal stage lane: preserve legacy BatchedPicard and add opt-in exact-input
   cache / active-suffix reuse. Frozen JVP and projected factorization remain
   visible; this lane is Newton-free, not fully derivative-free.
2. RHS-only homotopy lane: direct-map fixed point, screened depth-one residual
   extrapolation and continuation; no derivative oracle or matrix solve.
   Actual residual checks, failed trials and continuation calls are charged.
3. Structural raw-U lane: exact mean-field affine physical model
   f(y)=b-d*y+kappa*mean(y)*1, d>=kappa>=0. Two scalar tableaux plus full-state
   lifting preserve arbitrary initial and forcing transverse components.
   Compare against a strong derivative-free sequential structural baseline,
   not only a generic JVP solver. Both keep the original-space residual screen.
4. Theory: finite-DAG residual majorant / prefix reuse, safeguarded residual
   convergence, exact quotient reduction and scope-appropriate error/order
   transfer. Prior failures are retained as evidence for redesign.

## Before-execution criteria

No tolerance or model coefficient will be tuned against timing results.
Causal arms compare identical native target, full-space checks and true work.
Homotopy arms compare one original nonlinear target with common residual cap,
and an ODE/RK consumer with independently known reference.
Structural trajectory: d=20, kappa=19, forcing_i=.2+.05*cos(1.9*i),
y0_i=.5+.1*sin(.7*i), [0,1], 64 equal steps, n=64 and n=2048.
All 65 common states and every component must pass max absolute exact error
1e-6; structural same-target difference cap 1e-11; operational root residual
cap 1e-9. All arms keep these caps. Independent dense small-n target and
closed-form step-refinement tests precede timing.

Structural timing: five ABBA/BAAB blocks, approximately .1 s per baseline
batch; native pool creation outside timers, full trajectory allocation,
plan creation, callbacks and full-space screen inside. One-/two-worker
comparisons are reported separately. Observed ranges are descriptive.
Shared-host speed is inferred only for qualified cells; failed, inactive
and slower cells remain in results. Scalar setup and callback work are
reported separately from wall time.

## Execution

Native agents actually dispatched: causal_derivative_free,
homotopy_derivative_free, theory_derivative_free. Shared Cargo builds and
native timings serialize through /tmp/vigilode-cargo-20261027.lock.
Pinned local Rust1.94.1, existing Cargo.lock and cached offline dependencies.
Use cargo --profile measurement (release opt-level3, lto=false, cgu=16).
No unrelated V6/V8 physical campaign is reopened.
