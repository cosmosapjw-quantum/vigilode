# vigilode-coupled: opt-in native coupled-stage research step

This is the first native step proposed by the supplied integration-research report. It is a standalone companion crate and is **not** in the root workspace members or default dispatcher. Existing numerical crates and their defaults are unchanged.

## Implemented contract

- Autonomous, identity-mass ODE with a supplied JVP. The candidate uses `linearize_matrix_free`; an explicit Jacobian callback is never queried. This is not an RHS-only finite-difference JFNK implementation.
- One shared twice-reorthogonalized basis and stored J0V. Dense algebra is limited to the small projected m-by-m and8m-by8m systems, with m<=64 and a conservative storage-plan check. No full state-coordinate Jacobian/W is assembled in the candidate. For small problems m can equal n; the bound is dimensional, not a claim that every allocated matrix is smaller than n.
- Raw-U coupled RODAS5P target. Eight full-dimensional stage callbacks evaluate the SAME frozen iterate concurrently through `ParallelExecution`; the candidate does not solve8large sequential W systems. Small-system factorization, basis construction, continuation levels and output combination retain sequential work.
- Affine seed, direct lambda1 correction first, bounded lambda subdivision if iteration fails, original-space residual verification, then residual-driven basis expansion if needed.
- Finite/dimension/configuration/callback budgets, explicit work for failed parallel tasks, model-epoch invalidation, immutable caller state. Callbacks without epoch reporting must remain pure during an attempt. Callback panics are not converted into recoverable numerical errors.
- Optional explicit protected sequential GMRES fallback, JVP-only with no assembled preconditioner. Candidate and fallback work are separately retained, including failed fallback counts. The fallback uses the native-K target and is NOT labelled bit-identical to raw-U.

`Candidate` means a complete root candidate passed the **operational full-space residual screen**. `embedded_test_passes` is separate. Neither is a certified nonlinear output bound or global/continuous-time guarantee. Do not update physical time merely because `candidate.is_some()`. This crate does not supply the final adaptive acceptance/controller/dense-output loop.

## Build and execute the tested example

From the repository root, after installing Rust1.94.1:

```bash
cargo +1.94.1 test --manifest-path crates/vigilode-coupled/Cargo.toml --locked
cargo +1.94.1 build --manifest-path crates/vigilode-coupled/Cargo.toml \
  --profile measurement --example coupled_bench --locked
crates/vigilode-coupled/target/measurement/examples/coupled_bench \
  run few-mode 12288 1 0.001 coupled-4 3 1
```

The example positional arguments are `MODE MODEL N EPS H ARM REPS WARMUPS`. Its registered arms are `coupled-1`, `coupled-2`, `coupled-4`, `sequential-mf`, `transactional-4`. `MODE=reference` runs an explicitly untimed independent raw-stage recurrence. The example deliberately includes fixed steps that fail the embedded time-error criterion; inspect `root_completed`, candidate error and the separate embedded flag.

Public API: `coupled_step(problem,t,y,h,config,execution) -> Attempt` and `coupled_or_sequential(...,linear_config) -> SelectedStep`. Construct and reuse `ParallelExecution` at application scope to amortize pool setup. Do not assume4workers beat1worker; benchmark costs depend on dimension, basis size and operator work.

The default Config has initial basis8, maximum32, four expansions, eight Newton updates per corrector, six lambda subdivisions and finite callback/storage budgets. These are bounded research defaults, not a calibrated optimal policy. Rank deficiency, singular projection or exhausted budgets produce a typed decline; no dense assembly is used as a silent rescue.

## Evidence and limits

See [critical review](../../research/coupled_native_20261020/CRITICAL_REVIEW_KO.md), [actual run](../../research/coupled_native_20261020/runs/39349031422), [frozen closeout](../../research/coupled_native_20261020/closeout_runs/39349959195), and [reproduction](../../research/coupled_native_20261020/REPRODUCE.md).

Actual native tests:19passed. Benchmark:80accuracy rows,800fresh timing processes,3700timed step attempts. Worker output bits agree on15completed cases; the broad-spectrum candidate declines. CodeRabbit installation succeeded on the runner but login/authentication did not, so a CodeRabbit review is NOT claimed. No whole-repository test, global solver promotion, adaptive trajectory speedup or comparison against the strongest available matrix-free implementation is claimed.
