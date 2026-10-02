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

---

## Results (appended after the run at `cc4071b`)

Outputs: `PROFILE.json` and the 12 raw callgrind profiles in `callgrind/` (gzipped), plus the post-hoc
`POSTHOC.json` described below.

**Deviation (disclosed).** The first run of the tool aborted before writing anything to the node. It called
`gzip.open` with an `mtime` argument, which `gzip.open` does not take; `GzipFile` does. That run had written one
scratch callgrind profile, deleted unread, and an empty `callgrind/` directory, removed. The tool was fixed in
`cc4071b` and run again. The profiled binary was the same one, built from `629c950`; `cc4071b` changes only the
Python tool.

**Gate: FAIL.** The verdict follows the preregistered rule:

- The profiled runs reproduced the benchmark's work: 210, 469 and 92 attempts, with the same factorizations and
  right-hand sides as L-0029.
- The repeated callgrind profiles were identical.
- The native attribution covered at least 99.8% of instructions.
- **The RODAS5P attribution did not reach 90%.** "other" was 41%, 57% and 41%. The preregistered mapping had two
  defects:
  - **glibc has line tables in this container.** Its code is filed under `./malloc/malloc.c` and
    `sysdeps/.../memmove-*.S`, not under the `libc.so` object the `libc-allocator-and-memory` rule expected.
  - **faer's gemm microkernels have no line tables at all.** They appear as unsymbolized addresses in the binary
    (23.6% of the Brusselator run).

**Instructions per attempted step** (callgrind, one integration). These numbers do not depend on the attribution:

| Problem | Attempts | RODAS5P Ir per attempt | Hairer RODAS Ir per attempt | Ratio |
|---|---|---|---|---|
| HIRES | 210 | 115,000 | 9,160 | 12.6 |
| van der Pol | 469 | 79,500 | 2,950 | 27.0 |
| Brusselator n = 400 | 92 | 36.4 M | 8.83 M | 4.1 |

The per-step wall-time ratios of L-0029 (8 to 16 on the small problems) are of the same order. Instructions explain
the gap; no memory-bound effect is needed to account for it.

**Where RODAS5P's instructions go: post-hoc reading (not preregistered).** `tools/stiff_profile_posthoc.py` reads the
kept raw profiles and does two things:

- It corrects the two mapping defects. glibc source paths go to the allocator and memory category, and the
  unsymbolized code is resolved with `addr2line`; every such address was a faer gemm microkernel and goes to LU.
- It attributes the inclusive cost of calls into the hot callees to their callers.

Rerunning it from the committed script reproduced `POSTHOC.json` bit for bit. Its shares are exploratory:

| Share of one run | HIRES | van der Pol | Brusselator n = 400 |
|---|---|---|---|
| glibc allocator and memory (malloc, free, calloc, memalign, memcpy, memset) | 40.5% | 55.9% | 9.4% |
| Rust std (Vec growth and zeroing, iterator code inlined from `core`) | 31.7% | 23.3% | 49.6% |
| faer (factorization, solves, gemm kernels) | 15.3% | 8.4% | 38.3% |
| Everything in the repository's own files together | 11% | 10% | 2% |

The inclusive view, by callee and caller:

- **Allocator.**
  - Small problems: 37% (HIRES) and 52% (van der Pol) of all instructions are spent inside malloc and free.
  - The largest callers are:
    - the stage loop `sequential_stages_refined`, 9% and 12% (per-stage `Vec` allocations for stages, states,
      right-hand sides, combinations and residuals);
    - the zeroed allocations of `__rust_alloc_zeroed`, 4% and 5%;
    - `sequential_step`, 3% and 5% (step results and copies of y and of the stages);
    - `row_combination`, 2% and 3%;
    - the adaptive loop, 2% and 3%.
- **faer on small matrices.**
  - The per-stage solve took 17% (HIRES) and 12% (van der Pol). It goes through a heap-allocated faer matrix for
    each right-hand side.
  - The factorization itself took 4% and 1%.
  - For n = 2 to 8, faer's generic machinery costs far more than the arithmetic. Hairer's RODAS spends 24% to 61% of
    its far smaller instruction count in DEC and SOL.
- **Dense matrix-vector products.** These went through the shifted operator's `apply`: 7% (HIRES), 3% (van der Pol)
  and **27% (Brusselator n = 400)**. They are the diagnostic residual `W x - b` after every direct stage solve
  (`direct_report`, 8 per step) and the Jacobian product for the gamma terms of every stage after the first (7 per
  step). The loop is an iterator `zip`/`sum` that the compiler does not vectorize; its body is the `core::iter` share
  above. perf measured `matvec_into` at 35% of the Brusselator wall time.
- **Brusselator n = 400, the rest.**
  - The faer factorization took 43%. Hairer's DEC and SOL took 79% of RODAS's run, on a matrix whose zero
    multipliers DEC skips.
  - Matrix construction took 14%: zeroed `DenseMatrix` allocations, W assembly, conversion to faer.
  - The per-stage solves took about 8%.
- **perf cross-check (wall time, cpu-clock).**
  - The malloc, free and memory-copy symbols took 38% (HIRES) and 50% (van der Pol) of samples, against 37% and
    52% of instructions.
  - `matvec_into` took 35% on the Brusselator.
  - Samples: 27K, 32K and 48K.

**Implications.** These are not tested here; each needs its own measurement:

1. Reuse workspace across stages and steps instead of allocating per stage. On the small problems this is the
   largest single item, about 40% to 55% of instructions.
2. Drop or gate the diagnostic residual product after direct solves. A direct solve's residual is not needed for
   acceptance; this is 8 dense products per step.
3. Use the standard transformed Rosenbrock formulation, which needs no Jacobian products for the gamma terms.
   Together with item 2, this removes the 27% (n = 400).
4. Use a small-matrix LU and solve path (unblocked, in place, no allocation) below a dimension threshold, and a
   banded or sparse factorization where the structure allows.
5. Assemble W in place and stop zeroing new matrices every step.
