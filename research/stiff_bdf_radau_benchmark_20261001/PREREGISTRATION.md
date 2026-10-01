# Preregistration: work-precision benchmark of RODAS5P against BDF and Radau on standard stiff problems

## Question

On the same equations, how does RODAS5P (sequential, dense direct LU) compare with BDF and Radau IIA integrators
in accuracy per unit of work and per unit of wall time? This is a descriptive benchmark. It preregisters the
measurement, the error metric, the matched-error rule and a validity gate. It states no performance hypothesis, and
its wall times are diagnostics: the statistical authority of timing stays on hold
(`rodas5p_fair_ab::timing_authority_registry`), so nothing here is a speed promotion.

## Commands

- `cargo build --release -p rodas5p-cli --locked`
- `RAYON_NUM_THREADS=1 target/release/rodas5p stiff-benchmark --repetitions 7 --warmups 1 --output research/stiff_bdf_radau_benchmark_20261001/RUST.json`
- `python3 tools/stiff_benchmark_scipy.py --rust research/stiff_bdf_radau_benchmark_20261001/RUST.json --scipy-output research/stiff_bdf_radau_benchmark_20261001/SCIPY.json --analysis-output research/stiff_bdf_radau_benchmark_20261001/ANALYSIS.json`
- Inputs: `crates/rodas5p-cli/src/stiff_benchmark.rs`, `tools/stiff_benchmark_scipy.py`, the integrators they call,
  `Cargo.lock`. SciPy 1.17.1 with NumPy, installed into this container; BLAS threads are pinned to 1.

## Problems

| Id | n | Interval | atol / rtol |
|---|---|---|---|
| `robertson` (repository `robertson_problem`) | 3 | [0, 40] | 1e-4 |
| `hires` (Hairer and Wanner test set) | 8 | [0, 321.8122] | 1e-4 |
| `van-der-pol-mu1000` (repository `stiff_van_der_pol_problem(1e3)`, y0 = (2, 0)) | 2 | [0, 2000] | 1 |
| `brusselator-1d-50` (Hairer and Wanner II, IV.1, alpha = 1/50, 50 cells, interleaved u, v) | 100 | [0, 10] | 1 |

Every arm uses the analytic dense Jacobian. The Python right-hand sides and Jacobians must match the samples the
Rust binary exports at two states per problem (relative difference at most 1e-13).

## Arms

| Arm | Implementation | Fidelity |
|---|---|---|
| `rodas5p` | repository RODAS5P, `IntegrationMethod::Sequential`, dense direct LU | the method under study |
| `repo-bdf2` | repository adaptive BDF (orders 1-2), `BdfConfig::default()` | reference implementation only |
| `repo-bdf2-tier-a` | the same with Newton tolerances scaled to the outer tolerance | Tier-A flag, still a reference implementation |
| `repo-radau5` | repository Radau IIA, 3 stages (order 5), `RadauConfig::default()`: Newton rtol 1e-12, full real 3n stage system | reference implementation only |
| `repo-radau5-tier-a` | the same with scaled Newton tolerances and the stage LU reused for the error estimate | Tier-A flags, still a reference implementation |
| `scipy-bdf` | `scipy.integrate.solve_ivp(method="BDF")`: variable order 1-5 (NDF) | production algorithm, Python right-hand side |
| `scipy-radau` | `solve_ivp(method="Radau")`: Radau IIA order 5 with the complex transform | production algorithm, Python right-hand side |

Repository arms start from `initial_step = 1e-6`, with `max_step` equal to the interval and at most 1,000,000
attempts. SciPy arms choose their own first step. The repository BDF and Radau are reference implementations: the
repository forbids reading a production cost ranking from them (audit F-052/F-056). The SciPy arms are the
production BDF and Radau algorithms, but their right-hand sides and Jacobians run in Python, so their wall time
includes interpreter overhead per call. Work counters (right-hand-side evaluations, Jacobian builds, LU
factorizations, steps) are compared across languages; wall time is compared within one language only.

## Measurement

Tolerance ladder: rtol in {1e-3, 1e-4, ..., 1e-9}, atol = rtol x the problem's scale. For each problem, arm and
tolerance: one untimed warmup, then 7 timed runs on one thread; the median is reported. A run that fails is
recorded with its message and is not retried. Repository runs must end in the same state with the same counters in
all 7 repetitions.

**Error.** Final-time error `max_i |y_i - r_i| / max(|r_i|, 1e-10)` against a reference `r`.

**Reference.** SciPy Radau at rtol 1e-13 and atol 1e-14 x scale. Its uncertainty is the same error metric between
it and a second Radau solve at rtol 1e-12; an LSODA solve at rtol 1e-12 is reported as a cross-check. A run whose
error is below 10 times the uncertainty is marked reference-limited.

**Matched-error cost.** For error targets E in {1e-3, 1e-5, 1e-7}, an arm's cost at E is that of its cheapest
(lowest median wall) run with error at most E. The minimum counters over those runs are reported beside it. Wall
ratios are given relative to `rodas5p`. A target below 10 times the reference uncertainty is not evaluated, and an
arm that reaches no run at most E in the ladder is reported as not reaching E.

## Gate (validity of the benchmark)

**PASS** if all of the following hold:

- parity holds on every problem;
- every reference uncertainty is at most 1e-8;
- every repository arm is deterministic across its repetitions;
- every completed run has a finite error.

Otherwise **FAIL**. Failed runs of an arm are outcomes, not gate failures. The verdict says the benchmark is valid.
It says nothing about which arm is faster: that reading is descriptive, made by the rule above, and carries the
caveats above.

## Prior information

The adaptive global-error screens of this repository already compare these arms on small analytic problems; they
mark the repository BDF and Radau as reference implementations. No run of this ladder on these four problems exists
before this commit. Only the module's unit tests ran: the analytic Jacobians against central differences, and one
rtol 1e-3 HIRES run per repository arm, which every arm completed. The Python Jacobians were checked against central
differences at one state per problem, without any solver run.
