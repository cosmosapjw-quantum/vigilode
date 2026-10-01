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

---

## Results (appended after the run at `fad4dc3`)

Outputs: `RUST.json` (5 repository arms x 4 problems x 7 tolerances, 7 timed repetitions each), `SCIPY.json`
(references, parity, 2 SciPy arms) and `ANALYSIS.json` (errors and matched-error costs). SciPy 1.17.1, NumPy 2.4.6,
Python 3.11.15, one BLAS thread.

**Validity gate: PASS.**

- Parity: the right-hand sides and Jacobians agree bit for bit (relative difference 0) at both states of every
  problem.
- Reference uncertainty: 1.2e-15 (Robertson), 1.6e-13 (HIRES), 3.8e-13 (van der Pol) and 1.5e-13 (Brusselator).
  LSODA at rtol 1e-12 differs by 2e-11 to 4e-10. No target is reference-limited.
- Every repository arm was deterministic across its 7 repetitions, and every completed run has a finite error.
- Three runs failed with "maximum step count or minimum step reached": `repo-bdf2` at rtol 1e-9 on Robertson and
  van der Pol, and `repo-bdf2-tier-a` at rtol 1e-9 on van der Pol.

**Matched-error wall time** (median, ms; in parentheses the ratio to `rodas5p`; "-" means not reached in the ladder):

| Problem | E | rodas5p | repo-bdf2 | repo-bdf2 tier-a | repo-radau5 | repo-radau5 tier-a | scipy-bdf | scipy-radau |
|---|---|---|---|---|---|---|---|---|
| Robertson | 1e-3 | 0.46 | 0.36 (0.79) | 0.36 (0.78) | 0.31 (0.67) | 0.21 (0.45) | 7.0 (15) | 6.1 (13) |
| Robertson | 1e-5 | 0.68 | 2.6 (3.7) | 2.6 (3.8) | 0.31 (0.46) | 0.21 (0.30) | 20 (30) | 8.9 (13) |
| Robertson | 1e-7 | 1.9 | - | - | 0.39 (0.20) | 0.32 (0.17) | 46 (24) | 14 (7.4) |
| HIRES | 1e-3 | 0.60 | 16 (27) | 53 (87) | 5.9 (9.8) | 2.1 (3.4) | 36 (60) | 19 (31) |
| HIRES | 1e-5 | 1.4 | 198 (146) | 200 (148) | 5.9 (4.3) | 5.2 (3.9) | 50 (37) | 46 (34) |
| HIRES | 1e-7 | 4.3 | - | - | 8.1 (1.9) | 8.9 (2.1) | 110 (25) | 71 (16) |
| van der Pol | 1e-3 | 1.9 | 31 (17) | 30 (16) | 9.5 (5.1) | 5.3 (2.9) | 133 (72) | 70 (38) |
| van der Pol | 1e-5 | 3.5 | 156 (45) | 922 (264) | 9.5 (2.7) | 5.3 (1.5) | 282 (81) | 162 (46) |
| van der Pol | 1e-7 | 7.0 | - | - | 10 (1.4) | 7.8 (1.1) | - | 284 (41) |
| Brusselator | 1e-3 | 13 | 358 (28) | 283 (22) | 180 (14) | 69 (5.4) | 31 (2.4) | 35 (2.7) |
| Brusselator | 1e-5 | 29 | 966 (33) | 897 (31) | 180 (6.2) | 100 (3.4) | 55 (1.9) | 46 (1.6) |
| Brusselator | 1e-7 | 62 | 6278 (101) | 23377 (378) | 360 (5.8) | 229 (3.7) | 173 (2.8) | 96 (1.6) |

**Work per run at rtol 1e-6** (accepted steps, Jacobian builds, LU factorizations, right-hand-side evaluations):

| Problem | rodas5p | repo-radau5 tier-a | scipy-bdf | scipy-radau |
|---|---|---|---|---|
| Robertson | 43, 45, 45, 360 | 70, 72, 72, 1269 | 144, 4, 33, 366 | 78, 18, 100, 647 |
| HIRES | 204, 210, 210, 1680 | 216, 275, 275, 6657 | 327, 25, 85, 911 | 210, 75, 232, 1931 |
| van der Pol | 351, 469, 469, 3752 | 702, 1206, 1206, 27295 | 847, 57, 198, 2620 | 616, 122, 430, 5167 |
| Brusselator | 82, 92, 92, 736 | 113, 118, 118, 2029 | 198, 2, 37, 538 | 124, 23, 90, 948 |

(The repository Radau factorizes the real 3n x 3n stage system; SciPy Radau counts one real and one complex n x n
factorization as two.)

**Reading, by the preregistered rule and with its caveats:**

- **Within the repository (one language, one LU and one right-hand side):**
  - Against `repo-bdf2`, RODAS5P was cheaper at every matched target on HIRES, van der Pol and Brusselator (17x to
    378x). On Robertson it was cheaper at 1e-5 (3.7x) and slightly more expensive at 1e-3 (0.79x).
  - Against the better repository Radau arm (tier-a), RODAS5P was cheaper on HIRES, van der Pol and Brusselator
    (1.1x to 5.4x).
  - On Robertson the repository Radau was 2.2x to 5.9x cheaper. Its error is far below the tolerance: 6.6e-7 at
    rtol 1e-3, against the requested 1e-3. Its loosest run therefore already meets every target.
  - Both repository comparators are reference implementations. The BDF has order at most 2, and its error is not
    monotone in rtol (HIRES: 2.7e-6 at 1e-7, but 1.5e-5 at 1e-8). The Radau factorizes the 3n real system. These
    ratios therefore do not show that RODAS5P beats production BDF or Radau.
- **Against production algorithms (SciPy), work counters are the comparable measure:**
  - RODAS5P builds a Jacobian and factorizes once per attempt, with 8 right-hand sides per step.
  - SciPy BDF reuses its Jacobian and LU across steps. It used fewer factorizations on all four problems at rtol
    1e-6 (33 vs 45, 85 vs 210, 198 vs 469, 37 vs 92) and 2 to 57 Jacobians against 45 to 469.
  - RODAS5P used fewer right-hand sides than SciPy BDF on Robertson, comparable numbers on HIRES, and more on
    Brusselator.
  - Per accepted step RODAS5P is more expensive, but it takes fewer and longer steps.
- **Wall time against SciPy is not a fair speed comparison.** The SciPy arms evaluate the right-hand side and the
  Jacobian in Python. On the small problems (n = 2 to 8) Python per-call overhead dominates, and RODAS5P was 7x to
  81x faster. On the 100-component Brusselator, where factorizations weigh more, the gap was only 1.6x to 2.8x. For
  large systems, a native BDF with LU reuse could plausibly match or beat RODAS5P. This benchmark does not measure
  that.
- **Tolerance proportionality:**
  - RODAS5P's error stayed within a factor of about 0.1 to 3.5 of rtol on every problem.
  - The repository Radau's error ran far below rtol on stiff transients.
  - SciPy BDF ran up to 13x above rtol (HIRES).
