# vigilode-coupled: opt-in native coupled-stage research step

This is the first native step proposed by the supplied integration-research report. It is a companion crate in the root workspace. It is opt-in and is **not** the default dispatcher. The existing default methods are unchanged.

## Implemented contract

- Autonomous, identity-mass ODE with a supplied JVP. The candidate uses `linearize_matrix_free`; an explicit Jacobian callback is never queried. This is not an RHS-only finite-difference JFNK implementation.
- One shared twice-reorthogonalized basis and stored J0V. Dense storage is limited to the small projected m-by-m and8m-by8m systems, with m<=64 and a conservative storage-plan check. Affine and Newton corrections now factor only the common m-by-m diagonal block and use eight block-forward substitutions. Their arithmetic cost is O(m³+s²m²), while the full small matrix storage and residual check remain. No full state-coordinate Jacobian/W is assembled in the candidate. For small problems m can equal n; the bound is dimensional, not a claim that every allocated matrix is smaller than n.
- Raw-U coupled RODAS5P target. Eight full-dimensional stage callbacks evaluate the SAME frozen iterate concurrently through `ParallelExecution`; the candidate does not solve8large sequential W systems. Small-system factorization, basis construction, continuation levels and output combination retain sequential work.
- Affine seed, direct lambda1 correction first, bounded lambda subdivision if iteration fails, original-space residual verification, then residual-driven basis expansion if needed.
- Finite/dimension/configuration/callback budgets, explicit work for failed parallel tasks, model-epoch invalidation, immutable caller state. Callbacks without epoch reporting must remain pure during an attempt. Callback panics are not converted into recoverable numerical errors.
- Optional explicit protected sequential GMRES fallback, JVP-only with no assembled preconditioner. Candidate and fallback work are separately retained, including failed fallback counts. The fallback uses the native-K target and is NOT labelled bit-identical to raw-U.

`Candidate` means a complete root candidate passed the **operational full-space residual screen**. `embedded_test_passes` is separate. Neither is a certified nonlinear output bound or global/continuous-time guarantee. Do not update physical time merely because `candidate.is_some()`. Use the opt-in `integrate_coupled_observed` driver below for physical acceptance and the shared adaptive controller. Observation times are physical landings; this driver does not provide dense output.

## Build and execute the tested example

From the repository root, after installing Rust1.94.1:

```bash
cargo +1.94.1 test -p vigilode-coupled --locked
cargo +1.94.1 build -p vigilode-coupled \
  --profile measurement --example coupled_bench --locked
target/measurement/examples/coupled_bench \
  run few-mode 12288 1 0.001 coupled-4 3 1
```

The example positional arguments are `MODE MODEL N EPS H ARM REPS WARMUPS`. Its registered arms are `coupled-1`, `coupled-2`, `coupled-4`, `sequential-mf`, `transactional-4`. `MODE=reference` runs an explicitly untimed independent raw-stage recurrence. The example deliberately includes fixed steps that fail the embedded time-error criterion; inspect `root_completed`, candidate error and the separate embedded flag.

Public API: `coupled_step(problem,t,y,h,config,execution) -> Attempt` and `coupled_or_sequential(...,linear_config) -> SelectedStep`. Construct and reuse `ParallelExecution` at application scope to amortize pool setup. Do not assume4workers beat1worker; benchmark costs depend on dimension, basis size and operator work.

The default Config has initial basis8, maximum32, four expansions, eight Newton updates per corrector, six lambda subdivisions and finite callback/storage budgets. These are bounded research defaults, not a calibrated optimal policy. Rank deficiency, singular projection or exhausted budgets produce a typed decline; no dense assembly is used as a silent rescue.

## Evidence and limits

See [critical review](../../research/coupled_native_20261020/CRITICAL_REVIEW_KO.md), [actual run](../../research/coupled_native_20261020/runs/39349031422), [frozen closeout](../../research/coupled_native_20261020/closeout_runs/39349959195), and [reproduction](../../research/coupled_native_20261020/REPRODUCE.md).

Actual native tests:19passed. Benchmark:80accuracy rows,800fresh timing processes,3700timed step attempts. Worker output bits agree on15completed cases; the broad-spectrum candidate declines. CodeRabbit installation succeeded on the runner but login/authentication did not, so a CodeRabbit review is NOT claimed. No whole-repository test, global solver promotion, adaptive trajectory speedup or comparison against the strongest available matrix-free implementation is claimed.

## Coupled adaptive integration and same-target fallback

`integrate_coupled_observed(problem, span, y0, adaptive, output, coupled, execution, linear, policy)` connects the residual-first candidate to actual physical time advancement. The adaptive configuration owns absolute and relative tolerances. A candidate passing its root screen can still be rejected by the embedded time-error estimate. Rejected state is never committed. `AdaptiveOutputSession` reuses the native represented-clock and output-landing rules.

`RawStepSession` supplies a same-target **raw-U** fallback through `Rodas5pMfFastWorkspace`. It preserves candidate work, failed work and the additional original-space fallback residual validation separately. This differs intentionally from the older `coupled_or_sequential` API, whose fallback uses native K coordinates. The raw fallback reuses allocated buffers, refreshes its operator and disables recycling; no cross-step factor/subspace reuse is claimed.

`RootErrorPolicy::Operational` keeps the original residual screen and embedded estimate. `ConditionalL2` adds a finite causal estimate for algebraic stage error, an endpoint budget, and embedded-error perturbation. The caller must justify the inverse norm, Lipschitz/domain and residual-roundoff assumptions for **every attempted step**. Floating-point evaluation is not interval arithmetic, and the policy is not a verified certificate or global trajectory-error guarantee. Large causal amplification can make this estimate unusable; that is reported, not hidden by a projected residual.

The new path supports autonomous, identity-mass ODEs with a supplied JVP. Unsupported problems are refused. Real callback errors and model-epoch changes stop with partial outputs. Numerical solve failures can trigger step reduction; no callback error is silently relabelled as a successful fallback. Current memory use includes fresh basis and stage buffers per candidate plus compact per-attempt receipts; zero allocation and production-wide speedups are not claimed.

Reproduction and new evidence: [coupled-stage adoption](../../research/coupled_stage_adoption_20261022/REPORT_KO.md). Earlier step-only results above remain historical evidence for their exact source versions.
