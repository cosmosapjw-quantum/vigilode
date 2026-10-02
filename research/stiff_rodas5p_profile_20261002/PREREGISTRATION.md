# Preregistration: where does RODAS5P's time per step go?

## Question

The native benchmark (`research/stiff_native_benchmark_20261001`, L-0029) found that RODAS5P needs about as many
factorizations as Hairer's RADAU5 at matched error, yet spends 8 to 16 times more wall time per attempted step than
Hairer's RODAS. RODAS is a Rosenbrock method that also builds one Jacobian and one factorization per attempt. Where
does RODAS5P's time per step go, and how does its instruction count per step compare with RODAS's?

This is a descriptive profile. It preregisters the workloads, the attribution rule and a validity gate, and states no
hypothesis about which category dominates. It changes no solver code.

## Commands

- `CARGO_TARGET_DIR=target-profile CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release -p rodas5p-cli --locked`.
  This is the release profile of the benchmark (opt-level 3, fat LTO) with line tables added for attribution.
- The native driver of the native benchmark: `tools/native_stiff/build.sh target/native_stiff`.
- `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 python3 tools/stiff_profile.py --rodas5p target-profile/release/rodas5p --driver target/native_stiff/native_stiff --perf <linux-tools perf> --benchmark research/stiff_native_benchmark_20261001/ANALYSIS.json --output-dir research/stiff_rodas5p_profile_20261002 --scratch <scratch dir>`
- Tools: valgrind/callgrind 3.22 and perf 6.8 (`linux-tools-6.8.0-146-generic`) from the distribution. This VM
  exposes no hardware counters, so perf samples the `cpu-clock` software event.
- Inputs:
  - `tools/stiff_profile.py`;
  - `crates/rodas5p-cli/src/stiff_benchmark.rs` (`stiff-profile-run`: one problem and arm, run k times, nothing else);
  - the solver sources it calls;
  - the native driver sources;
  - `Cargo.lock`.

## Workloads

RODAS5P (the benchmark's `rodas5p` arm) and Hairer's RODAS, at rtol 1e-6, on:

- `hires`: n = 8, overhead-dominated in the benchmark;
- `van-der-pol-mu1000`: n = 2;
- `brusselator-1d-200`: n = 400, where factorizations weigh more.

## Measurement

**Instructions (callgrind, deterministic).**

- Each workload is profiled with 1 and with 2 repetitions of the integration. The difference is one integration,
  with startup and problem construction cancelled out. The 1-repetition profile is taken twice, as a determinism
  check.
- Instructions per attempted step = instructions per run / attempted steps.
- The parser's self costs must add up to callgrind's own total.

**Attribution rule for RODAS5P.** Every instruction goes to the source file of its innermost inlined frame (callgrind
`fi`/`fe` records), and the file goes to the first matching category:

| Category | Source |
|---|---|
| lu-faer | the faer crates (factorization and triangular solves) and their kernels (pulp, gemm, dyn-stack, ...) |
| dense-matrix-wrapper | `rodas5p-core/src/matrix.rs` (`DenseMatrix`, `LuFactorization` wrapper, conversions) |
| shifted-operator-and-matvec | `rodas5p-core/src/operator.rs` (W construction, Jacobian and W products) |
| problem-user-code | `stiff_benchmark.rs` and `problems.rs` (right-hand sides and Jacobians) |
| problem-wrapper | `rodas5p-integrators/src/problem.rs` (validation, finiteness checks, counting) |
| stage-assembly | `rodas5p-integrators/src/sequential.rs` |
| adaptive-control-and-output | `integrate.rs`, `adaptive.rs`, `output.rs`, `dense_output*.rs` |
| coefficients | `rodas5p-core/src/coefficients.rs` |
| norms | `rodas5p-core/src/norms.rs` |
| work-counters | `rodas5p-core/src/work.rs` |
| rust-std-alloc-iter | the Rust standard library sources (`/library/alloc`, `core`, `std`: Vec, iterators, allocation glue) |
| libc-allocator-and-memory | code of `libc.so` without line tables (malloc, free, memcpy, memset) |
| other | everything else |

**Attribution rule for RODAS**, by function name, falling back to the object:

| Category | Functions or object |
|---|---|
| lu-decsol | DEC/SOL family |
| integrator-core | RODAS, ROSCOR, CONTRO, ROCOE, SLVROD, DECOMR, ... (matrix formation is in DECOMR) |
| problem-user-code | the C right-hand sides and Jacobians |
| problem-wrapper | `h_fcn`, `h_jac` |
| libc-allocator-and-memory, fortran-runtime, libm | by object |
| other | everything else |

The top 15 files, top 30 source lines and top 15 functions by self instructions are recorded for RODAS5P, and the
top 15 functions for RODAS.

**Wall time (perf).** The RODAS5P workload is repeated 2000 times (HIRES), 1500 (van der Pol) or 10 (Brusselator),
and sampled at 4999 Hz on `cpu-clock`. The self-time share is recorded by symbol, for symbols at 0.5% or more. This
cross-checks the instruction profile against time (cache and memory effects). It is not used for attribution, since
inlining under fat LTO merges symbols.

## Gate (validity of the profile)

**PASS** if all of the following hold:

- the profiled RODAS5P runs reproduce the benchmark's work at rtol 1e-6 (the same accepted steps, factorizations and
  right-hand-side evaluations as `ANALYSIS.json` of L-0029);
- the repeated 1-repetition callgrind profile has the same instruction total;
- the named categories cover at least 90% of the instructions per run, for RODAS5P and for RODAS, on every workload.

Otherwise **FAIL**. The verdict says the attribution is valid. Which category dominates is the descriptive result.

## Prior information

- **Before this commit:**
  - only the source was read;
  - the callgrind parser was checked on an `ls` run, where its self costs summed to callgrind's total of 557020.
- **Candidate costs** seen in the source but not measured:
  - a diagnostic residual product with W after every stage solve (`direct_report`);
  - a Jacobian product per stage for the gamma terms;
  - a copy of the coefficient tables per step;
  - copies of the Jacobian and of W;
  - the conversion to faer;
  - many small vector allocations per stage.
- No profile of these workloads exists before this commit.
