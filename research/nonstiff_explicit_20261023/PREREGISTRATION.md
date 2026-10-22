# Nonstiff explicit integration adoption

Registered before new code execution. Baseline source: 4929c0f86cee03a06ac1ab2713e1b438dd10cfc5. Latest separate client research inspected: 69e83044aacfb9a8fd2c6c3fb1146911beeedd79. Prior unchanged proofs and numerical results are reused. No NCP/SSH execution.

## Question and implementation

Add a genuine RHS-only Dormand–Prince5(4) adaptive path for smooth nonstiff identity-mass ODEs, including time-dependent RHS. The existing Rodas5p entry remains a Rosenbrock solver. Expose a distinct Dopri5 facade with scalar/vector tolerance and explicit method/work records. Allocate step scratch once; reuse FSAL only for an unchanged endpoint and fixed RHS contract. Reuse represented-time and observation-landing semantics. Independent trajectories may run in a bounded Rayon pool with separate error tests and deterministic result ordering. Parallel stage Picard/homotopy is investigated through its exact dependency structure and cost; it is not presumed faster.

## Mathematical acceptance

Verify all rooted-tree order conditions through order5 (main) and4 (embedded), FSAL consistency and linear stability polynomial with exact rational arithmetic. Derive conditional global-order and embedded-controller exponent, RHS-work counts, and ensemble error-control invariance. A stability/cost selector cannot infer nonstiffness from absence of a Jacobian callback. Explicit nonstiff selection is user intent; automatic switching requires separate evidence.

## Implementation acceptance

- Nonautonomous exact/manufactured and nonlinear convergence, oscillator/damping and nonnormal cases; fixed-step refinement before roundoff.
- Zero Jacobian/JVP/Newton/factor/linear-solve work for explicit path, even with poison derivative callbacks supplied.
- FSAL counts verified against external callback counters on accepted/rejected/failed attempts. Model state must be fixed within a call; any new call starts without cached derivative.
- Invalid dimension/tolerances/mass/dense/event options rejected at the public boundary; finite values, partial callback writes, represented-time underflow, large-clock clipping and maximum attempts handled explicitly.
- Rejection and callback failure preserve accepted state. Partial failure is not reported as success.
- Workspace allocation remains independent of step count (output storage scales with requested samples). Scalar and uniform-vector tolerances agree.
- Ensemble workers1/2 preserve member results/work, input order and per-member failures; verify actual overlap separately from elapsed-time claims.
- Independent source review and targeted correction/retest only on concrete findings.

## Numerical campaign

Use existing measurement profile (opt3,lto=false,codegen-units16) and shared locked/offline Rust1.94.1 cache, at most2 build jobs. Native default campaign budget900seconds, per-configuration cap45seconds. Use one warm-up and3 timed repeats, deterministic randomized/interleaved execution order, identical sampled output times, preserve full state/reference/error/work and failures. Case families: smooth decay, nonautonomous forcing, nonlinear logistic/Riccati, oscillatory and rapidly oscillatory, damping, mixed scales, nonnormal, large vector, moderately stiff and stiff negative controls; several tolerances. Comparison: same achieved sampled-state error thresholds, not tolerance equality. Compare protected native dense or matrix-free routes appropriate to dimension, and optional VODE/DVODE, diffsol and Julia only when their pinned adapters are available. Missing external arms remain NOT_RUN. Do not use unsupported external timings as evidence.

Timing includes solver/workspace setup and integration; excludes reference evaluation, result serialization and independently reported process startup. No universal speed threshold. Report all losses, work and repeated-time spread, with local-only interpretation. No concurrent compilation during timing. Broad rerun tool has a local-only full mode with a configurable wall budget and exact source identity, receipts and safe new output directory.

## Commands

Root will publish implementation source before numerical execution and record exact commands/versions in receipts. Targeted commands:

```
cargo test --locked --offline -p rodas5p-integrators --test dopri5_contracts
cargo test --locked --offline -p vigilode --test explicit_contracts
cargo test --locked --offline -p vigilode --test explicit_ensemble_contracts
cargo build --locked --offline --profile measurement -p vigilode --example nonstiff_bench
python research/nonstiff_explicit_20261023/run_bench.py --help
```

The source audit may refine API names before execution while preserving these acceptance criteria. Broad production qualification remains tied to actual supported capabilities, full-state errors and client requirements; one new explicit path is a scoped capability increase.

## Campaign specification before execution

Default:10 analytic families (decay, driven, logistic, harmonic, rapid, damped, mixed-scales, nonnormal, large-vector4096, stiff-control), three requested tolerances1e-4/1e-6/1e-8 and two methods =60 configurations. One warm-up and3 interleaved timed rounds. Five common landing times. Primary error for matched comparisons is max_i,t |y-y_ref|/max(1,|y_initial|,|y_ref|), independent of requested solver tolerance; maximum absolute error is also retained. Large-vector baseline is matrix-free with supplied analytic JVP; small-case baseline is analytic dense-fast. Moderate stiffness is an additional full-mode family. All reference and computed state components are preserved for scoring without a numerical rerun.
