# Preregistration: RODAS5P against native BDF (SUNDIALS CVODE) and native Radau/Rosenbrock (Hairer)

## Question

The first benchmark (`research/stiff_bdf_radau_benchmark_20261001`, L-0028) compared RODAS5P with the repository's
reference BDF and Radau and with SciPy, whose right-hand sides run in Python. On the same equations, how does RODAS5P
compare in accuracy per unit of work and per unit of wall time with production native codes, all compiled and run on
this host? The problems now include a 400-component Brusselator, where LU factorizations weigh more.

This is a descriptive benchmark. It preregisters the measurement, the error metric, the matched-error rule and a
validity gate; it states no performance hypothesis. Wall times are diagnostics: the statistical authority of timing
stays on hold (`rodas5p_fair_ab::timing_authority_registry`), so nothing here is a speed promotion.

## Commands

- `cargo build --release -p rodas5p-cli --locked`
- `tools/native_stiff/build.sh target/native_stiff`
- `RAYON_NUM_THREADS=1 target/release/rodas5p stiff-benchmark --problems robertson,hires,van-der-pol-mu1000,brusselator-1d-50,brusselator-1d-200 --arms rodas5p --repetitions 7 --warmups 1 --output research/stiff_native_benchmark_20261001/RUST.json`
- `python3 tools/stiff_benchmark_native.py --rust research/stiff_native_benchmark_20261001/RUST.json --driver target/native_stiff/native_stiff --native-output research/stiff_native_benchmark_20261001/NATIVE.json --analysis-output research/stiff_native_benchmark_20261001/ANALYSIS.json`
- Inputs:
  - `crates/rodas5p-cli/src/stiff_benchmark.rs`;
  - `tools/native_stiff/native_stiff.c` and `tools/native_stiff/build.sh`;
  - `tools/stiff_benchmark_native.py` and `tools/stiff_benchmark_scipy.py` (references and the analysis rule);
  - `Cargo.lock`.
- Native builds:
  - SUNDIALS 6.4.1 (`libsundials-dev` 6.4.1+dfsg1-3build4) and OpenBLAS 0.3.26 from the distribution;
  - gcc and gfortran 13.3, `-O3`;
  - Hairer's `radau_decsol.f` (version of July 9, 1996) and `rodas_decsol.f`, from the Assimulo 3.0 sdist on PyPI,
    each pinned by SHA-256 in `build.sh` and not vendored.
  - The Rust release profile is opt-level 3 with fat LTO.
  - Every arm runs on one thread (`RAYON_NUM_THREADS=1`, `OPENBLAS_NUM_THREADS=1`).

## Problems

- The four problems of the first benchmark, with the same intervals and atol scales.
- `brusselator-1d-200`: 200 cells, n = 400, interval [0, 10], atol = rtol.

The C driver implements the same equations and analytic dense Jacobians. They must match the Rust samples at two
states per problem, with relative difference at most 1e-13.

## Arms

| Arm | Code | LU |
|---|---|---|
| `rodas5p` | repository RODAS5P, sequential, dense direct | faer partial pivoting (blocked, vectorized) |
| `cvode-bdf` | SUNDIALS CVODE, BDF orders 1-5, Newton, analytic Jacobian | SUNDIALS dense (unblocked) |
| `cvode-bdf-lapack` | the same CVODE run, with a custom `SUNLinearSolver` | OpenBLAS `dgetrf`/`dgetrs` |
| `hairer-radau5` | RADAU5, Radau IIA order 5, real plus complex system, analytic Jacobian | DECSOL (unblocked; skips zero multipliers) |
| `hairer-rodas` | RODAS, Rosenbrock order 4(3), analytic Jacobian | DECSOL |

Settings:

- CVODE keeps its defaults, except a maximum of 1,000,000 steps and a stop time at the end of the interval.
- RADAU5 and RODAS keep their defaults, except a maximum of 1,000,000 steps and a first step of 1e-6 (the
  repository's choice). CVODE estimates its own first step.
- RADAU5 tightens the tolerance internally (`0.1 rtol^(2/3)`); the matched-error rule absorbs this.

Counters:

- Right-hand-side evaluations, Jacobian evaluations, factorizations and accepted and rejected steps, as each code
  reports them.
- For CVODE, factorizations are linear-solver setups, and rejected steps are error-test failures.
- For RADAU5, one factorization counts one real and one complex matrix.

The LU microbenchmark records the median time of one factorization by each LU implementation, at n = 100 and 400.
It uses two matrices: the Brusselator iteration matrix `I - 0.05 J(y0)` (banded, where DECSOL skips zero
multipliers) and the full matrix `1/(1 + |i - j|) + n delta_ij`. This separates the cost of the LU implementation
from that of the integration algorithm.

## Measurement

- **Ladder and timing:** the same as the first benchmark. rtol in {1e-3, ..., 1e-9}, one warmup and 7 timed runs,
  median reported. A failed run is recorded and not retried.
- **Error:** the same final-time error, `max_i |y_i - r_i| / max(|r_i|, 1e-10)`.
- **References and uncertainty:** the same procedure (SciPy Radau at rtol 1e-13, uncertainty against rtol 1e-12,
  LSODA cross-check), computed afresh and including the new problem.
- **Matched-error cost:** at targets E in {1e-3, 1e-5, 1e-7}, the cheapest run with error at most E, with ratios
  to `rodas5p`. Targets below 10 times the reference uncertainty are not evaluated.

## Gate (validity of the benchmark)

**PASS** if all of the following hold:

- parity holds on every problem;
- every reference uncertainty is at most 1e-8;
- the RODAS5P arm and every native arm are deterministic across their repetitions (same final state and counters);
- every completed run has a finite error.

Otherwise **FAIL**. Failed runs are outcomes, not gate failures. The verdict says the comparison is valid, not which
arm is faster. That reading is descriptive, follows the rule above, and is limited by the stated LU and compiler
differences.

## Prior information

- **First benchmark (L-0028).** On the four small problems, SciPy BDF used fewer factorizations and Jacobians than
  RODAS5P. RODAS5P's wall time against SciPy included Python overhead.
- **Before this commit:**
  - the Rust unit tests ran;
  - each native arm ran once on HIRES at rtol 1e-3. The first RODAS call failed, because its step limit belongs in
    IWORK(1), not IWORK(2) as in RADAU5. The driver was fixed, and the rerun completed.
  - one parity call ran on HIRES;
  - the LU microbenchmark ran at n = 100 with 5 repetitions.
- No run of this ladder with the native arms, and no run of the 400-component Brusselator, exists before this commit.
