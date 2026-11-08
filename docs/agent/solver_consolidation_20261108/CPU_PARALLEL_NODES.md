# CPU parallelization nodes (GPU forbidden)

## P1. Stage-internal CPU parallelism
Purpose: shorten operations inside a causally dependent original RODAS5P stage without altering the tableau or pretending later stage inputs exist.

Existing code: ParallelExecution, stage_batch.rs, rodas5p_matrix_free_fast.rs and operator.rs. These are a reusable CPU foundation. stage_batch is an independent-row feasibility probe, not proof that true nonlinear RODAS stages are independent.

Tasks: profile RHS rows/components, JVP/operator apply, preconditioner, Krylov reductions, factorization, solve, error norm and interpolation. Use one bounded local CPU pool, no nested pools. For one-step RODAS5P preserve stage order. When multiple RHS truly share one frozen W, reuse one factorization and solve a block. Compare independent Krylov and block Krylov including basis cost. Instrument that no stale stage input is used.
Acceptance: unchanged tableau/dependency graph, residual/output parity, complete counters, >=7 matched-achieved-error paired full-IVP timings, workers 1/2/4/8. No GPU.

## P2. Independent IVP/parameter ensemble CPU execution
Generalize existing DOPRI5Ensemble to portfolio jobs. Each member owns Problem, method preference, tolerance, time span, initial state, controller, norm, history, work ledger, partial result and typed failure. Preserve input order and failure isolation. Share caches only with exact operator/callback epoch identity. Replayed pilots/retries require explicit callback side-effect contract. Never pool WRMS denominators or step-size history.
Acceptance: serial parity member by member, failure isolation, load-balance report, setup-amortized throughput and tail latency for workers 1/2/4/8. No GPU.

## P3. Spatial MOL CPU parallelism
Sphere: spherical harmonics/spin harmonics, sphere finite-volume or cubed-sphere, area measure/weighted norms, constant/harmonic mode and Laplace-Beltrami tests, pole/chart overlap and conservation under refinement.
2D/3D: anisotropic diffusion, advection-diffusion and reaction-diffusion; sparse/block/matrix-free operator apply; CPU row/cell partitioning and optional separately declared CPU distributed-memory halo exchange; refinement, conservation, spectrum and preconditioner scaling.
Cross-cutting admissible work: error-norm reductions, independent output evaluations and post-step event samples only where semantics permit. Do not alter adaptive acceptance via undocumented reduction-order changes.
Acceptance: verified spatial tests, serial/parallel operator parity, full setup/communication/preconditioner cost and achieved-error-matched solver comparisons. No GPU.

Shared policy: no CUDA/HIP/OpenCL/SYCL, accelerator-specific dependency, build feature, kernel or benchmark. CPU shared-memory, CPU SIMD and separately declared CPU distributed-memory are allowed. If threading loses, retain sequential CPU path.
