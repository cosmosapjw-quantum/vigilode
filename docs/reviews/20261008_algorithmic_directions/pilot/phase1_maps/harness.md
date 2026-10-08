# R5: Problem corpus and measurement harness for method-level comparisons (VigilODE @ a49f7e4)

All repository access was read-only. The only files written are in the scratch directory `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/`:
- `r5_replica.py`: a NumPy replica of the RODAS5P fast driver, checked against the Rust counters.
- `r5_spectra.py`: Jacobian spectra of the benchmark problems.
- `sb_fast.json`, `age_smoke.json`, `atlas_vdp_cal.json`, `spd07_base_rerun.json`: CLI outputs used to check the harness.

The same directory also holds files from sibling stages, including `ctrl.py`, which is another RODAS5P replica. I did not change them.

## 0. Key facts for hypothesis work

1. **A Python replica matches the Rust driver exactly in 11 of 12 cases.** The replica is about 60 lines (`scratchpad/algo/r5_replica.py`), and §7 gives its formulas. Compared with `stiff-profile-run --arm rodas5p-fast`, it reproduces attempts, accepted and rejected steps, RHS and Jacobian counts, and final errors to 4 digits on robertson, hires, vdp-mu1000 and brusselator-1d-50 at rtol 1e-3, 1e-5 and 1e-7. The one exception is van der Pol at rtol 1e-3: 180 attempts and 66 rejections in Python, against 181 and 67 in Rust. That is a rounding-sensitive rejection cascade.
2. **RODAS5P rejects far more steps than Hairer's RODAS on the same problems.** Data: `research/stiff_native_benchmark_20261001/ANALYSIS.json` (L-0029).

   | problem | rtol | RODAS5P rejected/attempts | Hairer RODAS rejected/attempts |
   |---|---|---|---|
   | vdp-mu1000 | 1e-3 / 1e-5 / 1e-7 | 67/181 (37%), 121/349 (35%), 95/654 (15%) | 23/159 (14%), 15/365 (4%), 9/1085 (0.8%) |
   | hires | 1e-3 / 1e-4 / 1e-5 | 11/52, 24/82 (29%), 18/123 | 3/57, 2/94, 1/175 |
   | brusselator-1d-50 | 1e-3 / 1e-5 / 1e-7 | 8/41, 12/69, 7/130 | 3/35, 3/80, 0/221 |

   The step-size controller explains part of this:
   - The benchmark uses the default `ControllerKind::Integral`: fac = 0.9·err^(-1/5), clamped to [0.2, 5] on accept and [0.2, 0.9] on reject (`crates/rodas5p-integrators/src/adaptive.rs:97-113, 264-306`).
   - The estimator order is 5 (`adaptive.rs:27-33`).
   - INFERRED: Hairer's RODAS uses the Gustafsson predictive controller by default.

   RODAS5P needs fewer accepted steps (vdp at 1e-5: 228 against 350). Its error-to-rtol ratio on vdp is 1.3 to 4.4 across the ladder.
3. **The cost per attempt is fixed by the method.**
   - Sequential `rodas5p` arm: 8 RHS evaluations, 1 Jacobian and 1 LU per attempt; the Jacobian is rebuilt even after a rejection.
   - Fast arms: 7 new RHS evaluations after a rejection, because J, f(t,y) and f_t are reused (`rodas5p_fast.rs:26-27, 704-720`).
   - Hairer RODAS: 6 RHS per attempt.
4. **Matrix-free Krylov cost is dominated by unpreconditioned GMRES on the Brusselator.** Data: `research/spd07_mf_step_warm_start_20261007/BASE.json` (L-0084). Settings: linear rtol fixed at 1e-10, atol 1e-14, restart 40, no preconditioner, and no inner forcing (`rodas5p_matrix_free_fast.rs:35-36`).

   | problem | n | GMRES iterations per stage solve |
   |---|---|---|
   | brusselator-1d-50 | 100 | 39.7 (≈333 JVPs per attempt) |
   | brusselator-1d-160 | 320 | 74.5 at rtol 1e-6; 48.5 at rtol 1e-8 |
   | hires | 8 | 7.0 |
   | robertson | 3 | 2.3 |
   | vdp | 2 | 2.0 |
5. **Brusselator step counts do not depend on the mesh.** At rtol 1e-6 there are 92 attempts (82 accepted, 10 rejected) for N = 50, 160, 200 and 500. Stiffness grows like c = (N+1)²/50: min Re λ ≈ −211 at N = 50 and −3235 at N = 200, from `r5_spectra.py`.
6. **Replicated-block atlas families are easy for Krylov because few eigenvalues are distinct (INFERRED).** On the R-JF path for `van-der-pol-ramped-n128`, every stage solve took exactly 14 GMRES iterations (7728/552), 142 JVPs per attempt. The atlas multiplier `1 + 0.01·(block % 7)` (`g4_s5b0_regime_atlas.rs:1044`) gives 7 block types, so at most 2 × 7 = 14 distinct eigenvalues. Corpus v2 uses radical-inverse diversity (`scientific_corpus_v2.rs:293`), which makes every block distinct, so the same family should need much more Krylov work there.
7. **Output-grid accuracy is dominated by the dense interpolant.** All 54 canonical v2 calibration rows were `output-policy-dominated` (L-0004). The clipped/dense gap reaches 1.3e5 to 4e5 WRMS on the semilinear family at rtol 1e-4 (`research/scientific_validity_v2_20260829/external_reaudit_bundle/rust/calibration_all_cases_compact.json`). `docs/DENSE_OUTPUT_ERROR.md` says RODAS5P dense output is order 4 on nonstiff problems and about order 3 on stiff ones. On Prothero-Robinson with λ = −1e5 at rtol 1e-6, the interior error is 312 tolerance units. Method comparisons should use step landing on the grid ("clipped") or final-time error, or should state the interpolant as a separate variable.
8. **The stability function overshoots on the imaginary axis.** With binary64 coefficients, |R(iy)| = 1 + 1.3e-19 near the origin (L-0051, `tools/rnext08_perturbation_stability.py`). This matters for long oscillatory runs.

## 1. Problem corpus with Python-transcribable definitions

### 1A. Stiff work-precision benchmark (`crates/rodas5p-cli/src/stiff_benchmark.rs`)

Benchmark rules:
- Tolerance ladder: `TOLERANCES` = 1e-3 … 1e-9 (`:31`).
- atol = rtol·atol_scale (`adaptive_config`, `:473-483`).
- Initial step 1e-6 (`:234`), min_step 1e-14, max_step = tf − t0, max_attempts 1e6 (`:235`).
- Output only at {t0, tf}. Default I-controller (§0.2).

| id | n | definition | y0 | t-span | atol_scale |
|---|---|---|---|---|---|
| `robertson` | 3 | `problems.rs:630-698`: y1' = −0.04y1 + 1e4·y2·y3; y2' = 0.04y1 − 1e4·y2·y3 − 3e7·y2²; y3' = 3e7·y2² | [1, 0, 0] | [0, 40] | 1e-4 |
| `hires` | 8 | `stiff_benchmark.rs:246-312` (standard HIRES; q = 280·y6·y8; 7th equation q − 1.81y7; 8th −q + 1.81y7; constant 0.0007 in the 1st) | [1, 0, 0, 0, 0, 0, 0, 0.0057] | [0, 321.8122] | 1e-4 |
| `van-der-pol-mu1000` | 2 | `problems.rs:571-628`: y1' = y2; y2' = μ(1 − y1²)y2 − y1, μ = 1000 (unscaled form) | [2, 0] | [0, 2000] | 1.0 |
| `brusselator-1d-N` | 2N | `stiff_benchmark.rs:315-385`, interleaved u1, v1, u2, …; c = (N+1)²/50; u' = 1 + u²v − 4u + c(u_{i−1} − 2u + u_{i+1}); v' = 3u − u²v + c(v_{i−1} − 2v + v_{i+1}); boundary values u = 1, v = 3 | u_i = 1 + sin(2πx_i), v_i = 3, x_i = i/(N+1), i = 1…N | [0, 10] | 1.0 |

Notes:
- N ∈ {50, 200} are available in `stiff-benchmark` (`benchmark_problems`, `:397`).
- N ∈ {30, 40, 500} are available only in `stiff-profile-run` (`profile_problems`, `:417`).
- The Brusselator band fill is at `:69-95`.
- Jacobian character at y0 and y(tf), from `r5_spectra.py`:
  - Robertson: λ_min = −0.04 at y0 and −3.4e3 at tf; departure from normality 0.65 to 0.71.
  - VdP: λ_min ≈ −3.0e3 at y0 and −1.9e3 at tf; real spectrum.
  - HIRES: λ_min ≈ −10.5 at both endpoints, mildly stiff. INFERRED: stiffer mid-trajectory.
  - Brusselator: Im λ < 1.2 (weakly oscillatory); μ₂(J) = +6.97 at y0, so the problem is locally expansive early.
- Small fixed-dimension variants for the `rodas5p-fast-small*` arms: `SmallVanDerPol` (`:831`), `SmallRobertson` (`:873`), `SmallHires` (`:893`).
- Ensemble: `stiff-ensemble-run` uses μ_k = 1000(1 + k/members) (`:1059`).
- An existing Python transcription with analytic Jacobians is in `tools/stiff_benchmark_scipy.py:50-148` (`PROBLEMS`).

### 1B. Manufactured and analytic problems (`crates/rodas5p-integrators/src/problems.rs`)

All have an exact solution unless noted.
- `scalar_linear_problem(λ, y0)` (`:5`): y' = λy.
- `prothero_robinson_problem(λ, μ, t0)` (`:37`): y' = λd + cos t + μd², with d = y − sin t. Exact y = sin t. Non-autonomous, with analytic f_t.
- `oscillatory_prothero_robinson_problem(λ, μ, ω, t0)` (`:504`): the same with g = sin(ωt).
- `manufactured_vector_problem(n, s, m, η, t0)` (`:86`), the nonnormal one: A is tridiagonal with diagonal −0.5s, subdiagonal 0.25s and superdiagonal (0.25 + η)s. The equation is y' = A(y − φ) + φ' + m(y − φ)³, with φ_i = sin(πx_i)·sin t + 0.35·sin(2πx_i)·cos(t/2) and x_i = i/(n+1).
- `constant_affine_mass_problem` (`:245`): M = [[2, 1], [0, 3]], J = [[−4, 1], [2, −5]], quadratic forcing. No exact solution.
- `manufactured_mass_nonlinear_problem(s, m, η, t0)` (`:305`).
- `complex_dahlquist_problem(blocks, damping σ, frequency ω, t0)` (`:435`): 2×2 blocks [[−σ, −ω], [ω, −σ]]. Exact solution e^{−σt}(cos, sin)(0.17·block + ωt).
- `semilinear_advection_diffusion_problem(n, D, a, r, nl, t0)` (`:700`), 1-D advection-diffusion-reaction with upwind advection:
  - Operator diagonal −2D/dx² + r − a/dx; lower D/dx² + a/dx; upper D/dx²; dx = 1/(n+1).
  - y' = A(y − φ) + φ' + nl(y − φ)³, with φ_i = e^{−t}·sin(πi·dx).

The adaptive-global-error parameters for these problems (`crates/rodas5p-fair-ab/src/adaptive_global_error.rs:359-430`):
- Smoke: scalar linear (λ = −2, y0 = 1) and Prothero-Robinson (λ = −20, μ = 1), both on [0, 0.2].
- Canonical adds: manufactured vector (n = 4, s = 20, m = 1, η = 0.2) on [0, 0.2], and manufactured mass (20, 1, 0.2) on [0, 0.08].
- G1 adds, each on [0, 0.02]: complex Dahlquist (16 blocks, σ = 120, ω = 180); oscillatory Prothero-Robinson (λ = −1e4, μ = 1e3, ω = 140); manufactured vector (n = 32, s = 1000, m = 100, η = 0.5); advection-diffusion (n = 32, D = 0.01, a = 5, r = −1, nl = 10).

### 1C. Matrix-free trajectory set (`crates/rodas5p-integrators/tests/rnext_common/mod.rs:346-436`, and `spd07_mf_step_warm_start.rs:33-57`)

| case | details |
|---|---|
| robertson | as in 1A |
| van-der-pol-mu1000 | as in 1A |
| hires | as in 1A |
| brusselator-1d-50 | as in 1A |
| brusselator-1d-160 | as in 1A, N = 160 |
| prothero-robinson-forced | λ = −1e4, μ = 0, [0, 2], atol_scale 1, exact sin t |
| quadratic-4 | y_i' = a_i·y_i + q_i·y_i², a_i = −1 − i, q_i = −0.05(1 + i mod 3), y0_i = 1 + 0.1i, [0, 0.5]; exact y_i = a·z·e^{at} / (a − q·z(e^{at} − 1)) |

The configuration is `adaptive()` (`:134-144`): initial step 1e-6, max 5000 attempts, rtol ∈ {1e-6, 1e-8}. The error measure is `relative_error` = max|y − ref| / max|ref| (`:146`).

### 1D. Scientific corpus v2.1 (`crates/rodas5p-integrators/src/scientific_corpus_v2.rs`)

General rules:
- rtol ∈ {1e-4, 1e-6, 1e-8} (`:16`), atol = 0.01·rtol (`:339`).
- Calibration dimensions {96, 384, 1536} (`:15`).
- Output grid of 101 uniform points plus mandatory breakpoints (`:17, 365`).
- ramp(t; c, w) = ½(1 + tanh((t − c)/w)) (`:515`).
- div(k) = 0.9 + 0.2·(base-2 radical inverse of k+1) (`:293`).
- Calibration spans: Robertson [0, 0.1]; all other calibration families [0, 1] (`:307`).

| family | lines | definition |
|---|---|---|
| robertson-ramped | `:527` | n/3 Robertson blocks; k_i = (0.04, 1e4·act, 3e7·act)·div(block), act = 0.05 + 0.95·ramp(t; 0.045, 0.010); padding −20·act·y |
| hires-ramped | `:606` | HIRES blocks scaled by div(block); q = 280·act·y6·y8, act = 0.1 + 0.9·ramp(0.45, 0.08); padding −(2 + 20·act)·y |
| van-der-pol-ramped | `:694` | μ = (10 + 490·ramp(0.5, 0.08))·div(block); y0 = (2, 0) per block; padding −(5 + μ)·y |
| rotating-nonnormal | `:756-893` | y' = R(θ)ᵀ·A·R(θ)(y − φ) + φ' + 40·ramp(0.6, 0.06)(y² − φ²), with A = [[−s, ηs], [0, −0.35s]], s = (20 + 480·ramp)·div, η = 0.1 + 0.8·ramp, θ = (8t + 0.4 sin 4t)·div; exact φ_i = 0.4 sin(k_i t) + 0.2 cos(k_i t/2), k_i = (1 + i mod 7)·div(i/2) |
| nonautonomous-stiff-forcing | `:895` | y_i' = −div(i)·s·d + div(i)·Ω·cos(Ωt + p_i) + 20·ramp·d², d = y_i − sin(Ωt + p_i), s = 30 + 470·ramp(0.45, 0.07), Ω = 2 + 28·ramp, p_i = 0.17(i mod 11). The forcing uses Ω rather than the true derivative, so there is no exact solution |
| semilinear-advection-diffusion-ramped (2-D) | `:965-1082` | grids 8×12, 16×24, 32×48; D = 0.002, five-point stencil, zero Dirichlet; backward upwind with a = 0.5 + 3.5·ramp(0.5, 0.08); reaction −1; nl = 2 + 48·ramp; exact φ = e^{−t}·sin πx·sin πy |
| oregonator (holdout) | `:1084` | n = 3, s = 77.27, q = 8.375e-6, w = 0.161, y0 = (1, 2, 3), [0, 360] |
| pollution (holdout) | `:1116-1234` | n = 20, 25 reactions, constants `POLLUTION_K`, [0, 60] |
| medical-akzo (holdout) | `:1236-1309` | n = 400; split at t = 5 |
| brusselator-2d (holdout) | `:1311-1426` | 16×16 periodic grid, n = 512, A = 3.4, B = 1, α = 10, h = 1/15; forcing switched on at t = 1.1 (mandatory split); [0, 11.5] |

RHS/JVP oracles for checking a transcription: `fixtures/scientific_corpus_v2_1_calibration_oracle.json` and `fixtures/scientific_corpus_v2_1_semilinear_oracle.json` (mpmath, 80 digits, at t = 0.37, y = y0 + 0.01·cos(0.17(p+1)), v = sin(0.23(p+1))).

Holdout hygiene (`docs/HOLDOUT_HYGIENE.md`): holdout rows may be read only by a committed verdict script. **Probes must not use the holdouts or the atlas holdout profiles** (canonical, holdout-512, 320, 384) for calibration.

### 1E. G4/S5B0 regime atlas, legacy (`crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs`)

The families are the same as in v2 but without radical-inverse diversity:
- Robertson multiplier 1 + 0.02(block % 5) (`:861`).
- VdP local μ multiplier 1 + 0.01(block % 7) (`:1044`).
- HIRES and rotating: no per-block scale (`:939, :1133-1153`).
- 1-D semilinear on dx = 1/(n+1) (`:1324-1418`).

Profile settings:
- Profiles and dimensions: `:28-104`. (atol, rtol) per profile: `:121-131`. Smoke is (1e-6, 1e-4); canonical, calibration-128 and holdout-512 are (1e-7, 1e-5).
- Adaptive configuration (`:1503-1519`): PI controller with safety 0.9, factors [0.2, 4], reject cap 0.8; initial step span/20; max step span/5.
- R-JF linear solves use `G4S5B0InnerTolerancePolicy` (WRMS inner forcing, `:1521-1532`).

### 1F. Linear-algebra-only operators (no ODE trajectory)

- `crates/rodas5p-fair-ab/src/scenarios.rs:65-170`: synthetic stage-system sequences for Krylov A/B tests.
  - Base matrix: diagonal 1 + s(0.01 + x²); superdiagonal η·√(d_i·d_{i+1}); subdiagonal 0.01·√(…).
  - Sequence kinds fixed, slow-drift, abrupt and rotating, built from Givens similarities.
  - Right-hand sides use a Pcg64Mcg RNG with 1e-4 noise, so a Python version is not bitwise; the noise is negligible (INFERRED).
  - CLI: `trace`, then `benchmark`.
- `tests/rnext_common/mod.rs:86`: 1-D convection-diffusion matrix I + τ(…) with a Péclet parameter.
- `crates/rodas5p-core/tests/int05_nonnormal_metric_bound.rs:28-100`:
  - VIG-A02: [[−2, 2^k], [2^−k, −2]] for k ∈ {0, 10, 20, 46}.
  - Jordan-8: diagonal −1, superdiagonal μ ∈ {1, 10, 100}.
  - Central convection-diffusion-32 with Pe ∈ {10, 50}, τ = 1e-3.

## 2. Reference solutions and matched-accuracy rules

| harness | error metric | reference | matched-accuracy rule |
|---|---|---|---|
| stiff-benchmark (L-0028, L-0029) | final time only: max_i \|y_i − r_i\| / max(\|r_i\|, 1e-10) (`tools/stiff_benchmark_scipy.py:156`, `ERROR_FLOOR` at `:42`) | SciPy Radau, rtol 1e-13, atol = 1e-14·atol_scale; uncertainty measured against rtol 1e-12, LSODA cross-check (`:181-196`) | targets {1e-3, 1e-5, 1e-7} (`:43`); per arm, the cheapest run with err ≤ target; a target below 10× the reference uncertainty is "reference-limited" (`:235-280`) |
| corpus v2 and holdouts | WRMS per grid point, weights 1e-10 + 1e-8·\|ref_i\|, divided by √n; max over grid = `max_grid_wrms` (`crates/rodas5p-fair-ab/src/global_error.rs:124-144, 306-380`) | SciPy Radau ladder: L0 (1e-8/1e-10), L1 (1e-10/1e-12), L2 (1e-12/1e-14), plus LSODA (3e-14/3e-16) (`numerical_reference.rs:609-642`); uncertainty = D1·q/(1−q) + method disagreement (`:954-1006`) | a row is invalid if uncertainty > 0.1 × measured max-grid WRMS (`:379-398`); budget verdicts in `output_accuracy.rs:44-78` |
| adaptive-global-error | same WRMS norm; endpoint, max-grid and rms-grid values for clipped and dense output | analytic, uncertainty 0 (`adaptive_global_error.rs:317-357`) | ladder {1e-4, 1e-6} for smoke, plus 1e-8 for canonical (`:432-437`); atol = 0.01·rtol; initial step = output spacing (`:439-453`) |
| matrix-free tests | max\|y − ref\| / max\|ref\| | dense fast driver at rtol 1e-12, or exact | none (counted work only) |

Stored references:
- Final states for robertson, hires, vdp, bruss-50 and bruss-200: `research/stiff_native_benchmark_20261001/NATIVE.json["references"]`. Uncertainties are 1.2e-15 to 3.8e-13. There are no stored references for bruss-30, -40 or -500.
- Corpus v2 n = 96 calibration references: `research/scientific_validity_v2_20260829/external_reaudit_bundle/reference/selected_raw/*.json`. The n = 384 and 1536 artifacts are missing (18 of the 22 manifest paths).
- Holdout references: `tools/reference_v2/artifacts/` (Oregonator uncertainty 0.0106 WRMS, Brusselator-2D 0.0081).
- `tools/reference_v2/artifacts_v2/reference_manifest_v2.json` has status `not-run`.
- The generator requires Python 3.12. `/usr/bin/python3.12` exists but has no NumPy. The default `python3` is 3.11.15 with NumPy 2.4.6, SciPy 1.17.1 and mpmath 1.3.0.

Error estimator inside the solver:
- Scale atol + rtol·max(\|y\|, \|y_new\|) and WRMS (`crates/rodas5p-core/src/norms.rs:30-83`).
- The embedded error is the last U stage (`rodas5p_fast.rs:770-785`).

## 3. CLI commands, outputs and binaries

### Binaries (no build needed)

- `/home/user/target-ens/measurement/rodas5p` (12.6 MB, built Oct 7 14:23, measurement profile: lto=false, codegen-units 16). INFERRED from timestamps: built from c087a59. `git diff c087a59 a49f7e4 -- crates` is empty.
- `/home/user/target-speed/release/rodas5p` (45.7 MB, fat LTO, built Oct 7 12:04). It predates c26a235, which changed only the opt-in matrix-free warm start. Both binaries gave identical counters in my checks.
- Prebuilt test binaries are in `/home/user/target-ens/measurement/deps/`, for example `spd07_mf_step_warm_start-f505c8264b67ebeb`, `safe_recycle_policy-b19807ee40627445` and `rnext0*`.

### Rebuilding

Rebuilding from the worktree is possible: `cargo` is installed and `Cargo.toml:31-49` defines the profiles.
- Logged measurement-profile builds took 2 min 34 s and 4 min 15 s. Fat-LTO release build time is unknown (INFERRED: longer).
- The disk has about 20 GB free. An earlier rerun failed with "No space left on device" (`docs/reviews/20261005_speed_research/SPEED_RESEARCH_STATUS.md:245`).
- Building into the worktree is not allowed in this stage; use `CARGO_TARGET_DIR` outside it.

### Commands

Always set `RAYON_NUM_THREADS=1`. If you pipe stdout to `head`, the binary panics with a broken pipe; redirect to a file instead.

| purpose | command | output | runtime |
|---|---|---|---|
| one arm, one problem, one rtol | `rodas5p stiff-profile-run --problem <id> --arm <arm> --rtol <r> [--repetitions k]` | JSON on stdout: attempts, accepted_steps, rejected_steps, counters{rhs_evaluations, jacobian_builds, direct_factorizations, linear_solves, direct_solve_calls}, final_state, deterministic, and banded_work for banded arms | ≤ 0.03 s |
| full rtol ladder | `rodas5p stiff-benchmark --problems robertson,hires,van-der-pol-mu1000,brusselator-1d-50,brusselator-1d-200 --arms rodas5p-fast,rodas5p-fast-banded --repetitions 1 --warmups 0 --output <scratch>.json` | rows with final_state, steps, counters, wall time, and parity_samples; refuses to overwrite its output | 0.85 s |
| errors and SciPy/native comparison | `python3 tools/stiff_benchmark_scipy.py --rust X --scipy-output Y --analysis-output Z` | recomputes references and SciPy arms, then the matched-error analysis | INFERRED: minutes. Faster: compute `final_error` against the stored NATIVE.json references |
| matrix-free U-form trajectories (GMRES or GCRO-DR) | `SPD07_BASE=<abs scratch path> RAYON_NUM_THREADS=1 <deps>/spd07_mf_step_warm_start-f505c8264b67ebeb --ignored --nocapture --test-threads=1 --exact export_base` | 7 problems × 2 rtols; every WorkCounters field (jvp_vectors, linear_iterations, linear_matvecs, orthogonalization …) | 17 s; reproduced BASE.json bit for bit |
| analytic-reference global error, all families | `rodas5p adaptive-global-error --profile smoke\|canonical --output X` | 10 candidates (direct, GMRES, LGMRES and GCRO-DR sequential RODAS5P; SABR; homotopy; BDF1/2; Radau 1/3) × clipped and dense; errors{endpoint, max_grid, rms_grid}_{l2, wrms}; full counters; diagnostics.accepted_step_sizes | smoke 0.03 s. Smoke is not discriminating: RODAS5P takes 2-3 steps |
| R-JF matrix-free atlas | `rodas5p generic-policy-redesign-atlas --profile calibration --family <robertson\|hires\|van-der-pol\|rotating-nonnormal\|nonautonomous-forcing\|semilinear> --output X` | the calibration profile is n = 128, rtol 1e-5; per-trajectory attempt_work.{total, accepted, rejected}; per-step rows with rodas_jvp_vectors, linear_matvecs, h, embedded error | 0.09 s (vdp) |
| van der Pol ensemble | `rodas5p stiff-ensemble-run --arm rodas5p-fast-small --members 64 --rtol 1e-6` | aggregate work | fast |

Arms:
- `rodas5p`: sequential, dense direct, J rebuilt on rejection.
- `rodas5p-fast`: U form, J reused after rejection.
- `rodas5p-fast-small` and `-small-static`: robertson, hires and vdp only.
- `rodas5p-fast-banded` and `-banded-slices`: Brusselators only.
- `repo-bdf2[-tier-a]`, `repo-radau5[-tier-a]`.
- Option arms `-val`, `-land`, `-ovh`, `-colext[64]`: `stiff_benchmark.rs:139-234`, `run_arm` `:485-588`.

There is no CLI arm for the matrix-free U-form driver; use the prebuilt test binary.

Not useful for trajectory-level method comparisons:
- `r3-campaign*`: hom06 and poly03 are scalar or quadratic stage problems.
- `r4-study`: phi actions and homotopy certificate cost (`crates/rodas5p-cli/src/r4_studies.rs`).
- `global-error-pareto`: fixed-step anchors.
- `native-integrator-gates`.

## 4. Existing Python tooling

- **Problem transcriptions with dense Jacobians (NumPy):** `tools/stiff_benchmark_scipy.py:50-148` covers robertson, hires, vdp and Brusselator(N). It also has a parity check against the Rust `parity_samples` (`:161`).
- **Native comparators:** `tools/stiff_benchmark_native.py` runs CVODE, Hairer RADAU5 and RODAS. It needs `tools/native_stiff/build.sh`; the driver is not prebuilt.
- **RODAS5P single-step code:**
  - `tools/rnext08_perturbation_stability.py:51-63`: K form, mpmath.
  - `tools/audit2_output_policy_research.py:76-97`: `ScalarRodas`, including dense output with D = H·Γ.
  - `tools/rnext01_residual_output.py:172-196`: U and K forms; problems robertson, vdp, hires, brusselator, PR, quadratic at `:47-134`.
- **No adaptive RODAS5P integrator exists in the repository.** `scratchpad/algo/r5_replica.py` (mine) and `scratchpad/algo/ctrl.py` (sibling) fill that gap.
- **Linear-algebra probes (`tools/pp*_check.py`):** pp10 (Leja), pp11 (Taylor), pp12 (log-norm), pp16 (complex shift), and others. They work on operators, not trajectories.

## 5. Probe kit

### 5.1 RODAS5P replica in U form, matching `rodas5p_fast.rs`

Coefficients come from `fixtures/rodas5p_coefficients_snapshot.json`: gamma, A (8×8), C (8×8), c (8), b_code (8), H (3×8), stored as decimal strings. Derived quantities follow `crates/rodas5p-core/src/coefficients.rs:188-196`:
- Γ = (I/γ − C)⁻¹
- gamma_rows_i = Σ_j Γ_ij

One attempt from (t, y, h):
1. Form W = I/(hγ) − J(t, y) and factor it once.
2. For each stage i = 0…7, solve
   W·u_i = f(t + c_i h, y + Σ_{j<i} A_ij u_j) + Σ_{j<i} (C_ij/h)·u_j + gamma_rows_i·h·f_t.
   Stage 0 reuses f(t, y).
3. y_new = y + Σ_j b_code_j·u_j.
4. err = WRMS(u_7 / (atol + rtol·max(|y|, |y_new|))).
5. Accept if err ≤ 1.
6. Step-size update:
   - accept: h ← h·clamp(0.9·err^(−1/5), 0.2, 5)
   - reject: h ← h·clamp(0.9·err^(−1/5), 0.2, 0.9)
   - If the next trial from the same t is not shorter than the rejected one, clip it to just below the rejected trial (`output.rs:226-258`).
   - The last step is clipped to land on tf.

Dense output in U form: d = H·u, and y(θ) = (1−θ)y + θ(y_new + (1−θ)(d₀ + θ(d₁ + θd₂))).

Matrix-free variant (`rodas5p_matrix_free_fast.rs:1-36`): (I − hγJ)U_i = hγ·f(…) + γΣ C_ij U_j + h²γ·γ_i·f_t. The embedded error is U_(s−1). Krylov stopping is max(atol, rtol·‖b‖) with linear rtol 1e-10, atol 1e-14, restart 40 and maxiter 200 (`solver_types.rs:54-69`), x0 = previous stage, and no preconditioner.

Validation: the replica's output matches the Rust counters in the table below, which also serves as the baseline for any modified controller, estimator or tableau.

| problem | rtol | attempts | accepted | rejected | RHS (fast) | J | final error |
|---|---|---|---|---|---|---|---|
| robertson | 1e-3 / 1e-5 / 1e-7 | 22 / 33 / 66 | 21 / 32 / 63 | 1 / 1 / 3 | 175 / 263 / 525 | = accepted | 9.19e-5 / 4.87e-6 / 1.10e-7 |
| vdp-mu1000 | same | 181 / 349 / 654 | 114 / 228 / 559 | 67 / 121 / 95 | 1381 / 2671 / 5137 | | 3.49e-3 / 2.21e-5 / 2.29e-7 |
| hires | same | 52 / 123 / 406 | 41 / 105 / 402 | 11 / 18 / 4 | 405 / 966 / 3244 | | 1.71e-4 / 6.28e-6 / 1.38e-8 |
| bruss-1d-50 | same | 41 / 69 / 130 | 33 / 57 / 123 | 8 / 12 / 7 | 320 / 540 / 1033 | | 8.02e-5 / 1.04e-5 / 2.42e-7 |

The complete 1e-3…1e-9 ladders for RODAS5P, repo Radau5, SciPy BDF/Radau, CVODE and Hairer RODAS are in `research/stiff_bdf_radau_benchmark_20261001/ANALYSIS.json` and `research/stiff_native_benchmark_20261001/ANALYSIS.json`.

### 5.2 Per-problem setup

| problem | equations | y0 | span | atol | rtol ladder | Rust counters | Rust error |
|---|---|---|---|---|---|---|---|
| robertson | §1A | [1, 0, 0] | [0, 40] | 1e-4·rtol | 1e-3…1e-9 | `stiff-profile-run --problem robertson --arm rodas5p-fast --rtol R` | final_error against NATIVE.json reference [0.7158270687194065, 9.185534764557796e-06, 0.28416374574582975] |
| hires | §1A | e1 + 0.0057·e8 | [0, 321.8122] | 1e-4·rtol | same | `--problem hires` | NATIVE.json |
| vdp-mu1000 | §1A | [2, 0] | [0, 2000] | rtol | same | `--problem van-der-pol-mu1000` | reference [1.7061677321708557, −0.000892809701024391] |
| brusselator-1d-N | §1A, c = (N+1)²/50 | §1A | [0, 10] | rtol | same | `--problem brusselator-1d-{30,40,50,200,500} --arm rodas5p-fast` (or `-banded`) | NATIVE.json for N = 50 and 200; otherwise SciPy Radau with a banded Jacobian sparsity |
| Prothero-Robinson | §1B, λ ∈ {−20, −1e4, −1e5}, μ ∈ {0, 1} | sin t0 | [0, 0.2] or [0, 2] | 0.01·rtol (global-error harness) or rtol (matrix-free set) | 1e-4…1e-8 | `adaptive-global-error --profile smoke` (λ = −20) or the spd07 test export (λ = −1e4) | exact sin t |
| oscillatory PR / complex Dahlquist | §1B G1 parameters | exact(t0) | [0, 0.02] | 0.01·rtol | 1e-4, 1e-6, 1e-8 | `adaptive-global-error --profile canonical` (INFERRED: G1 additions run through `generic-q1q2-adaptive`; not checked) | exact |
| nonnormal manufactured vector | §1B (n, s, m, η) | φ(t0) | as above | 0.01·rtol | same | `adaptive-global-error --profile canonical` | exact |
| advection-diffusion 1-D / 2-D | §1B, §1D | φ(0) | [0, 1] (v2) | 0.01·rtol | 1e-4, 1e-6, 1e-8 | `generic-policy-redesign-atlas --family semilinear` (1-D atlas, n = 128) | v2 2-D n = 96 reference in selected_raw; exact φ |
| ramped corpus families | §1D, n = 96 | §1D | §1D | 0.01·rtol | §1D | atlas CLI (legacy variants), or `scientific-validity-v2-run-calibration` (needs the complete reference manifest; only n = 96 references are present) | selected_raw n96 + WRMS rule |
| matrix-free set (§1C) | §1C | §1C | §1C | rtol·scale | 1e-6, 1e-8 | spd07 test binary `export_base` | dense rtol-1e-12 reference or exact |

### 5.3 Caveats for fair comparisons

- Compare counted work: attempts, RHS evaluations, JVPs, linear iterations, LU factorizations. Wall time is diagnostic only: the timing authority is on hold, and SciPy RHS calls are interpreted Python.
- Use the same initial step (1e-6), clipping rules and atol scaling.
- Report rejections separately from accepted steps, and say whether J is reused after a rejection.
- Separate endpoint or clipped error from dense-output error (L-0004, F-002).
- Do not tune on holdouts.