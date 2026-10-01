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

---

## Results (appended after the run at `e6a82a6`)

Outputs:

- `RUST.json`: the RODAS5P arm on 5 problems x 7 tolerances, 7 timed repetitions each, plus the faer LU
  microbenchmark.
- `NATIVE.json`: build identity, parity, references, the native LU microbenchmark, and 4 native arms x 5 problems x 7
  tolerances.
- `ANALYSIS.json`: errors and matched-error costs.

The native build used the `build.sh` flags above. The driver was built into the Cargo target directory, which this
container sets with `CARGO_TARGET_DIR`; that is the `target/native_stiff` of the commands.

**Validity gate: PASS.**

- Parity: the C right-hand sides and Jacobians equal the Rust samples bit for bit on all five problems.
- Reference uncertainty: at most 3.8e-13; for the new Brusselator n = 400 it is 1.7e-13, and LSODA differs by
  3.0e-11.
- RODAS5P and every native arm were deterministic.
- All 175 runs completed; none failed.

**Matched-error wall time, ratio to `rodas5p`** (below 1: the native arm is faster; "-": target not reached):

| Problem | E | rodas5p (ms) | cvode-bdf | cvode-bdf-lapack | hairer-radau5 | hairer-rodas |
|---|---|---|---|---|---|---|
| Robertson | 1e-3 | 0.19 | 0.66 | 0.70 | 0.07 | 0.07 |
| Robertson | 1e-5 | 0.28 | 1.24 | 1.24 | 0.08 | 0.06 |
| Robertson | 1e-7 | 0.89 | 0.69 | 0.74 | 0.05 | 0.08 |
| HIRES | 1e-3 | 0.65 | 0.98 | 1.25 | 0.16 | 0.10 |
| HIRES | 1e-5 | 1.5 | 0.98 | 1.34 | 0.15 | 0.14 |
| HIRES | 1e-7 | 4.7 | 0.73 | 0.68 | 0.08 | 0.21 |
| van der Pol | 1e-3 | 2.1 | 0.30 | 0.32 | 0.05 | 0.04 |
| van der Pol | 1e-5 | 3.9 | 0.98 | 0.78 | 0.04 | 0.10 |
| van der Pol | 1e-7 | 8.0 | - | - | 0.04 | 0.15 |
| Brusselator n=100 | 1e-3 | 14 | 0.52 | 0.35 | 0.25 | 0.11 |
| Brusselator n=100 | 1e-5 | 30 | 0.55 | 0.28 | 0.16 | 0.11 |
| Brusselator n=100 | 1e-7 | 63 | 0.55 | 0.32 | 0.13 | 0.15 |
| Brusselator n=400 | 1e-3 | 268 | 0.44 | 0.25 | 0.20 | 0.10 |
| Brusselator n=400 | 1e-5 | 611 | 0.27 | 0.19 | 0.14 | 0.10 |
| Brusselator n=400 | 1e-7 | 1260 | 0.34 | 0.20 | 0.13 | 0.14 |

**Wall time per attempted step at rtol 1e-6** (microseconds):

| Problem | rodas5p | cvode-bdf | cvode-bdf-lapack | hairer-radau5 | hairer-rodas |
|---|---|---|---|---|---|
| Robertson | 8.3 | 1.4 | 1.4 | 1.0 | 0.52 |
| HIRES | 11 | 2.6 | 2.9 | 1.9 | 1.2 |
| van der Pol | 8.4 | 1.2 | 1.3 | 0.47 | 0.61 |
| Brusselator n=100 | 329 | 90 | 42 | 81 | 47 |
| Brusselator n=400 | 6640 | 836 | 583 | 1700 | 816 |

**Median time of one LU factorization** (ms):

| n | Matrix | faer (rodas5p) | SUNDIALS dense | OpenBLAS dgetrf | DECSOL |
|---|---|---|---|---|---|
| 100 | banded | 0.063 | 0.082 | 0.063 | 0.016 |
| 100 | full | 0.063 | 0.92 | 0.13 | 0.11 |
| 400 | banded | 3.0 | 1.3 | 2.1 | 0.31 |
| 400 | full | 4.2 | 58 | 2.2 | 7.3 |

**Reading, by the preregistered rule and with its caveats:**

- **Hairer's RADAU5 and RODAS were faster than RODAS5P at every matched target on every problem.** RADAU5's wall
  time was 0.04 to 0.25 of RODAS5P's, and RODAS's 0.04 to 0.21. CVODE was faster at 13 of 14 targets with SUNDIALS'
  own LU (0.27 to 1.24) and at 11 of 14 with OpenBLAS (0.19 to 1.34). RODAS5P was faster only on Robertson at 1e-5
  (1.24x) and on HIRES at 1e-3 and 1e-5 (1.25x and 1.34x, against the OpenBLAS arm).
- **Work counts do not explain the gap on the small problems.**
  - At matched error, RODAS5P needed a similar number of factorizations as RADAU5. RADAU5's count is of pairs of a
    real and a complex matrix, each pair several times the flops of one real matrix.
  - RODAS5P needed fewer factorizations than the order-4 RODAS at tight tolerances: 406 against 859 at HIRES 1e-7,
    and 947 against 1987 at van der Pol 1e-7.
  - Yet RODAS5P spent 8 to 11 us per attempted step on n = 2 to 8, against 0.5 to 1.2 us for RODAS. RODAS is the
    same kind of method: one Jacobian and one factorization per attempt.
  - The difference is per-step overhead in the repository's RODAS5P code path, about 8 to 16 times. This benchmark
    does not locate that overhead; a profile would.
- **On the 400-component Brusselator the gap has two parts.**
  - **LU implementation.** The faer LU took 3.0 ms on the banded iteration matrix. DECSOL took 0.31 ms because it
    skips zero multipliers, and OpenBLAS took 2.1 ms.
  - **Everything else.** RODAS5P's remaining time per attempt was about 3.6 ms of 6.6 ms.
  - **LU reuse.** CVODE needed 4 to 10 Jacobians and 15 to 49 factorizations for the whole run, against 41 to 298
    for RODAS5P, which builds both at every attempt.
- **Tolerance proportionality.** RODAS5P's error stayed within about 0.1 to 3.5 times rtol. CVODE's ran up to 14x
  above it (HIRES rtol 1e-3) and up to 230x (van der Pol rtol 1e-6). CVODE did not reach 1e-7 on van der Pol
  within the ladder.
- **Scope.**
  - This is one host, single-threaded, five problems and the final-time error only.
  - The algorithms and LU implementations differ as stated: faer does not exploit band structure, and DECSOL does.
  - It does not show RODAS5P's method to be inferior. It shows that the repository implementation is not yet
    competitive in wall time with these production codes, and how far it is from them.
