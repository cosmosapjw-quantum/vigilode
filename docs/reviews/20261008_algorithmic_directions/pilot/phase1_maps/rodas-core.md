# R1: RODAS5P method mathematics as implemented (worktree `/home/user/wt-speed`, HEAD a49f7e4)

Each fact carries a label:
- **[READ]**: taken from a file at the line given.
- **[LEDGER]**: taken from `research/LEDGER.jsonl`.
- **[COMPUTED]**: computed by me from the repo fixture with a read-only script. The scripts are in `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/`: `tab.py`, `worder.py`, `prdae.py`, `ctrl.py`, `ctrl2.py`, `stalej.py`.
- **[INFERRED]**: my reasoning, not measured.

Nothing in the worktree was modified.

---

## 1. Tableau

### Where it comes from
- **Data file [READ].** The tableau is in `fixtures/rodas5p_coefficients_snapshot.json`. Its SHA-256 is pinned at `coefficients.rs:15-16` and checked when the file is parsed (`coefficients.rs:149-155`).
- **Origin [READ].** OrdinaryDiffEq.jl, author commit `000230a3`, file `src/tableaus/rosenbrock_tableaus.jl`. The literals are "official ordinary decimal literals; not original exact rationals". No higher-precision values are public.
- **`docs/CONSTANTS_PROVENANCE.toml` has no tableau rows [READ].** It lists only policy and atlas constants (V25, V29, V36, V37, TIER_L/N), at lines 22-140. So the tableau's provenance is the snapshot plus `coefficients.rs`.

### How the code derives the method [READ] `coefficients.rs:187-197`
- Γ = (I/γ − C)⁻¹
- α = AΓ
- β = α + Γ
- b = Γᵀ b_code
- btilde = Γ[s−1,:]
- γ_i = row sums of Γ
- dense_d = HΓ

### Basic facts
- **Stages and nodes [READ/COMPUTED].** s = 8 and γ = 0.21193756319429014. c = [0, .6358, .4096, .9769, .4288, 1, 1, 1].
- **Stage-wise γ_i [COMPUTED; E-07 note].** γ_i = [0.212, −0.424, −0.338, 1.805, 2.326, ~1e-15, ~1e-15, ~1e-15]. Analytically 0 for stages 6-8.
- **Weight norms [COMPUTED].** ‖b‖₁ = 4.9455, ‖btilde‖₁ = 1.4288, ‖b_code‖₁ = 24.905.
- **Coefficient sizes [COMPUTED].** max|A| = 11.63, max|C| = 165.4, max|γC| = 35.06.
- **Conditioning [COMPUTED].** cond₁(I/γ − C) = 4.07e4.

### Stiffly accurate, and so is the embedded method
- **[READ]** E-07: `research/adversarial_audit_20260927/experiments/E-07/results.json`.
- **[COMPUTED]** Row 8 of A equals the first 7 entries of b_code. b = β[8,:]. The embedded weights b̂ = b − btilde = α[8,:] = β[7,:].
- **[COMPUTED]** R(∞) = R̂(∞) = −1.3e-49.
- The error vector is the last stage increment U₈.

### Order
- **Main method [READ, E-07].** All 17 rooted-tree Rosenbrock conditions through order 5 hold. The largest residual is 1.2e-15 on the fixture and 2.0e-15 on the f64 coefficients the crate uses.
- **Embedded method [READ, E-07].** All 8 conditions through order 4 hold. All 9 order-5 conditions fail (residuals 1.2e-4 to 1.5e-2). So the pair is 5(4).
- **Rounding of b [READ, E-07].** f64 rounding is up to about 807 ulp in b. It does not degrade the order.
- These are conditions for an exact Jacobian. Nothing in the repo imposes W-method conditions.

### Order when the matrix in W is not the exact Jacobian
[COMPUTED] One step on a 3-D nonlinear autonomous test problem, 60-digit arithmetic, h = 1/8 to 1/128.

| Matrix used in place of J | Local-error slope | Method order | Estimate slope |
|---|---|---|---|
| exact J | 6.0 | 5 | 5.0 |
| J + O(h²) | 5.0 | 4 | 4.0 |
| J + O(h) (e.g. J from the previous step) | 4.0 | 3 | 3.0 |
| J + O(1) (W-method) | 2.9 | 2 | 1.9 |
| zero matrix | 3.0 | 2 | — |

### Linear stability
- **A-stability [COMPUTED].** On a grid of the imaginary axis and left half-plane, max|R| = 1 and max|R̂| = 1.
- **The small excess above 1 comes from the literals [COMPUTED].** |R(iy)| − 1 is +3.9e-21 at y = 1e-3 and +3.9e-19 at y = 1e-2, even when the decimal literals are evaluated at 50 digits. So L-0051's |R(iy)| = 1 + 1.3e-19 is already present in the literals themselves, not only after f64 rounding [INFERRED].
- **Stiff decay [COMPUTED].** R(z) ≈ 12.5/z as z → −∞. Values: R(−10) = −0.040, R(−30) = −0.123 (the largest |R| on (−∞, −5]), R(−100) = −0.086, R(−1e3) = −0.012.
- **Internal stages are not damped [COMPUTED].** Stage stability functions at z = −1e8 are R_i = [1, −2.00, −0.93, 3.27, −1.01, 3.41, 1e-7, −2e-9]. Stages 2-6 amplify stiff components by up to 3.4×; only stages 7 and 8 damp them.

### Estimator behaviour on the stiff real axis [COMPUTED]
- For z ≤ −30, |R − R̂| ≈ |R − e^z| (the ratio is 0.98-1.2). The estimate is not blind to stiff modes.
- On the forced Prothero-Robinson problem it over-estimates by about 4.7×:
  - [LEDGER L-0040] quintic PR, effectivity 11, 6.2, 4.6, 4.6 at z = −10, −100, −1e4, −1e6.
  - [COMPUTED] sin-forced PR, effectivity 4.66-5.0 at λ = −1e6 and 4.7-14.6 at λ = −1e4.

---

## 2. How each driver forms and uses the Jacobian

### Sequential K form (reference / protected path)
- **Per attempt [READ].** `build_step_context` (`sequential.rs:168-204`) evaluates f₀, f_t (`:185`) and J = `problem.linearize` (`:186`) on every call. `sequential_step` calls it once per attempt (`:787`), so the Jacobian is rebuilt even after a rejection.
- **Stage equation [READ].** The stage loop is at `:389`. Each stage i ≥ 1 needs one JVP for gmix = Σ_j Γ_ij K_j (`:412`), so 7 per attempt. The right-hand side is h f_i + hJ·gmix + h²γ_i f_t (`:414-419`).
- **Factorization [READ].** With a direct solve there is one LU per attempt (`:375-382`).
- **Strict matrix-free variant [READ].** `build_step_context_matrix_free` (`:211-253`) never materialises J.

### Matrix-free U form (`rodas5p_matrix_free_fast.rs`)
- **Stage equation [READ] (`:1-37`, `:419-470`).** (I − hγJ)U_i = hγ f_i + γ Σ_j C_ij U_j + h²γγ_i f_t. J appears only inside the Krylov solver.
- **Reuse after a rejection [READ].** f₀, f_t and the JVP operator are kept only for a retry from the same bits (`:389-410`). The flag is set at `:822`.
- **Preconditioning [READ].** Identity only (`:156`, `:250`). `validate_strict` (`:188-211`) refuses Direct and Jacobi preconditioners and any mass matrix.
- **Linear tolerance [READ].** There is no inner forcing (`:35`). The Krylov tolerance is the fixed `config.rtol`; the default is 1e-11 (`rodas5p-core/src/solver_types.rs:54-69`), and the SPD07 runs used 1e-10. The absolute tolerance is mapped to |γ|·atol (`raw_stage_target.rs:292-304`).

### Dense / banded / small-n fast drivers (`rodas5p_fast.rs`, `rodas5p_fast_small.rs`)
- **Formulation [READ].** U form with W = I/(hγ) − J (`rodas5p_fast.rs:1-33`).
- **Per attempt [READ].** `attempt` (`:690-781`) builds J, f₀ and f_t only when `fresh` (`:702-718`). It factors W on every attempt (`:720`).
- **Reuse [READ].** `fresh_state` is set to false after every acceptance (`:1371`). So the Jacobian is rebuilt every accepted step and reused only after a rejection from the same state (L-0032: "Jacobian reuse after rejection").
- **Mass matrices [READ].** Refused (`:1294-1298`).
- **Small-n [READ].** Same semantics, autonomous problems only (`rodas5p_fast_small.rs:1-37`).

### Time derivative f_t [READ] `problem.rs:253-279`
- Autonomous problems get zeros.
- Otherwise the user's `partial_t` callback is used if given.
- Otherwise a central finite difference with step ε = √eps·max(|t|, 1), costing 2 extra right-hand sides.
- `time_augmented_clone` (`problem.rs:453`) requires an explicit `partial_t`.
- **Sensitivity [COMPUTED].** A unit error in f_t changes y₁ by O(h³) when λ = −1. At λ = −1e4 the change is bounded at about 3e-9.
- **[READ, E-09].** The finite-difference f_t error dominates from h ≤ 1/256, with an error floor proportional to |λ| (1.6e-13 at λ = −1e4).

### Stale or approximate Jacobian
- **No implementation or analysis in the repo [READ].** No W-method is implemented and no stale-J analysis is run.
- **Audit position [READ].** `VIGILODE_ADVERSARIAL_AUDIT_20260927.md:737-740` (F-027) treats the question as answered by existing theory. What remains is an "exact J vs stale J vs Krylov-projected J" campaign. There is no ledger row for that campaign [INFERRED: not run].
- **The finite-difference JVP arm does not change the global error [READ].** In the fixed-mesh ablation (`research/scientific_validity_v2_20260829/addendum_20260929_two_arm_v3/README.md`) the finite-difference JVP arm gives the same global error as the exact arm. Its GMRES tolerance had to be relaxed to 1e-8.
- **Reusing J across steps is badly negative [COMPUTED, `stalej.py`].** This used a Python replica of the fast driver: J reused for 2 or 4 steps, refreshed on rejection. The table shows reuse every 2 steps against the baseline:

| Problem, rtol | LU factorizations, baseline | LU factorizations, reuse every 2 steps | Ratio | Jacobians |
|---|---|---|---|---|
| HIRES 1e-8 | 784 | 4534 | 5.8× | 778 → 2267 |
| van der Pol 1e-6 | 469 | 1164 | 2.5× | 351 → 582 |
| Robertson 1e-6 | 45 | 126 | 2.8× | 43 → 63 |

Even the Jacobian count goes up.

---

## 3. Error estimation and step-size control

### Error vector and acceptance [READ]
- K form: e = Σ btilde_i K_i (`sequential.rs:734-770`).
- U forms: e = U₈ (`rodas5p_fast.rs:771`; `rodas5p_matrix_free_fast.rs:597`).
- Scale: atol + rtol·max(|y|, |y_new|), weighted RMS over n.
- A step is accepted when err ≤ 1 (`sequential.rs:752`; `rodas5p_matrix_free_fast.rs:825`).

### Controller [READ] `adaptive.rs`
- **Defaults (`:97-116`).** Safety 0.9; step factor clamped to [0.2, 5] after an acceptance; at most 0.9 after a rejection; Integral controller.
- **Exponent (`:27-33`).** The estimator order is set to 5, so the exponent is 1/5.
- **Formulas (`:264-306`).**
  - Integral: 0.9·err^(−1/5).
  - PI: 0.9·err^(−0.7/5)·prev^(0.4/5). Used only after an acceptance that has a previous accepted error (`:290`).
- **Equilibrium errors [INFERRED from the formulas].** The I controller settles at err ≈ 0.59 and the PI controller at err ≈ 0.17.
- **What is not there.** No predictive (Gustafsson) controller, no PID or digital-filter controller, and no rule that stops the step from growing right after a rejection.
- **Non-finite error.** The step is multiplied by 0.2 (`:444`).
- **Retry after a rejection.** The next attempt from the same time must be strictly shorter (`output.rs:244-256`).

### Clipped-landing controller (WU-6b, F-006) [READ]
Commits cf79ec5 and 73dde4a; code at `adaptive.rs:329-447`.
- A sliver landing (trial < 0.5 × request) returns the remembered request and leaves the history untouched.
- An informative clipped acceptance predicts err·ratio⁻⁵. It lowers the request only if that prediction exceeds 1, and never below the trial step; otherwise it may only raise the request.

### How inexact linear solves enter
- **Protected matrix-free path: forcing target [READ] `g4_s5b0_inner_tolerance.rs:54-115`.**
  - η = 0.1 / (‖b‖₁ · max(flow, rhs)), clamped to [64 eps, 0.5]; τ = η·rhs_wrms. The allocation is 0.1/4.9455 = 0.0202.
  - The claim scope is a heuristic that would need a resolvent bound to become a guarantee (`:12-29`).
- **Refinement [READ] `sequential.rs:677-722`.** Up to 3 passes, with the residual limit 0.0202·min(1, err)^(6/5).
- **A stage that misses its forcing target [READ].** It returns a linear-solve error (`sequential.rs:580`), so the attempt is rejected with the 0.2 factor.
- **The resolved / under-resolved flag is never used for acceptance [READ].** It is computed (`SelfConsistent` / `Underresolved`), but `integrate.rs:819` keeps only `.step`, and grep finds no other use. An under-resolved step can be accepted on its embedded estimate alone.
- **Matrix-free fast driver [READ].** The error estimate is simply the inexact U₈.
- **One-sided rule err + B ≤ 1 (REV-03) [READ/LEDGER].** It exists only offline: `tools/rnext01_residual_output.py`, `tools/rev03_one_sided.py`, `tests/rev03_export.rs`.
  - B propagates bounds through the stages: each stage bound d_i combines ‖W⁻¹‖, the stage residual and an interval bound of the Jacobian on a box.
  - **L-0062 PASS:** 68 of 68 acceptances were resolved, at step sizes not used before.
  - Six rejections were unresolved, all Robertson at h = 3e-2 (B ≈ 2e246).
  - **L-0049 FAIL:** Robertson at h = 1e-2 gave B = 2e122 with exact stage states near 1e117.
  - No driver uses this rule.
- **Homotopy path [READ].** It accepts when the embedded error plus a linearized output correction is ≤ 1 (`homotopy.rs:161`, `:1239`). The code labels that correction as not a bound (`:63-94`).
- **SABR path [READ].** It accepts on err + fixed_point_error (`integrate.rs:30-32`).
- **`path_controller.rs` [READ].** This is a research screen of homotopy schedules, not a step-size controller.
  - `run_schedule` (`:550-645`): "algebraic accepted" means the linearized output WRMS is ≤ 0.1; "full-step accepted" means the combined error is ≤ 1.
  - It also flags false accepts against an oracle.

### Rejection rates against Hairer's RODAS [READ]
Same benchmark, initial step 1e-6. Data: `research/stiff_rodas5p_fast_20261002/RUST.json` and `NATIVE.json`.

| Problem, rtol | RODAS5P accepted/rejected (rejection %) | Hairer RODAS accepted/rejected |
|---|---|---|
| van der Pol μ=1000, 1e-3 | 114/67 (37%) | 136/23 |
| van der Pol, 1e-4 | 155/87 (36%) | 210/18 |
| van der Pol, 1e-5 | 228/121 (35%) | 350/15 |
| van der Pol, 1e-6 | 351/118 (25%) | 608/12 |
| van der Pol, 1e-7 | 559/95 (15%) | 1076/9 |
| HIRES 1e-3 | 41/11 (21%) | 54/3 |
| HIRES 1e-4 | 58/24 (29%) | 92/2 |
| HIRES 1e-5 | 105/18 (15%) | 174/1 |
| Brusselator-50, 1e-3 | 33/8 (20%) | 32/3 |
| Brusselator-50, 1e-5 | 57/12 (17%) | 77/3 |
| HIRES 1e-9 | 1498/4 | 5209/0 |
| Robertson 1e-9 | 191/4 | 688/1 |

At tight tolerances RODAS5P takes 3.5× fewer steps than RODAS. At loose tolerances it rejects 3-10× more often.

### Controller replica [COMPUTED, `ctrl.py` / `ctrl2.py`]
- **Fidelity.** The Python replica reproduces the Rust accepted/rejected counts exactly on Robertson (5 of 5 tolerances), HIRES (5 of 5) and van der Pol (4 of 5; at 1e-3 it gives 114/66 against 114/67).
- **Predictive controller.** I added Hairer's Gustafsson rule plus "no growth right after a rejection". Errors are the final-state WRMS in tolerance units, against the SciPy Radau references stored in `research/stiff_native_benchmark_20261001/NATIVE.json`.

| Problem, rtol | Baseline LU count | Predictive LU count | Ratio | Rejections | Final error (tol. units) |
|---|---|---|---|---|---|
| van der Pol, 1e-3 | 180 | 141 | 0.78× | 66 → 28 | 0.745 → 0.664 |
| van der Pol, 1e-4 | 242 | 175 | 0.72× | 87 → 21 | 0.28 → 0.32 |
| van der Pol, 1e-5 | 349 | 252 | 0.72× | 121 → 19 | 0.45 → 0.47 |
| van der Pol, 1e-6 | 469 | 381 | 0.81× | 118 → 16 | 0.34 → 0.37 |
| van der Pol, 1e-7 | 654 | 598 | 0.91× | 95 → 12 | 0.44 → 0.30 |
| HIRES, 1e-3 | 52 | 45 | 0.87× | 11 → 3 | — |
| HIRES, 1e-4 | 82 | 68 | 0.83× | 24 → 7 | — |
| HIRES, 1e-5 | 123 | 115 | 0.93× | 18 → 5 | — |
| HIRES, 1e-6 | 210 | 215 | 1.02× | 6 → 2 | — |
| HIRES, 1e-7 | 406 | 409 | 1.01× | 4 → 1 | — |

- On Robertson the predictive rule makes no difference.
- **PI as currently coded.** It gives 0 rejections but 1.5-1.8× more accepted steps at lower error (Robertson at 1e-3: 38 against 21 steps). That is over-resolution, not a matched-accuracy gain.

---

## 4. Dense output

- **Formula [READ] `dense_output_v2.rs:66-121`.** u(θ) = (1−θ)y₀ + θ(y₁ + (1−θ)(d₀ + θ(d₁ + θd₂))), with d_r = Σ_j D_rj K_j and D = HΓ. It is quartic in θ and needs no extra right-hand-side evaluations.
- **Documented order [READ].** `docs/DENSE_OUTPUT_ERROR.md:5-11` states order 4 on nonstiff problems and about 3 on stiff ones. With the default `Off` mode the interior is not tolerance-controlled.
- **Measured order [COMPUTED].** The local interpolant error at θ = 0.5 and 0.3 scales as h⁵ (slopes 4.99), so order 4 on nonstiff problems.
- **Control modes [READ] `dense_output_v2.rs:186-269`, `:662`.**
  - `ReportDefect`: one right-hand side; unfiltered.
  - `Report` / `Enforce`: one right-hand side, one Jacobian and one LU per sampled step. The filter solves (M − (h/2)J)e = (h/2)d at θ = 1/2.
- **Numbers [READ], PR with λ = −1e5.**
  - True interior error is 3.2, 312 and 209 tolerance units at rtol 1e-4, 1e-6, 1e-8.
  - With `Enforce` it is 0.92, 1.05 and 1.38.
  - `ReportDefect` overstates the stiff case: 1.3e7 against a true 312.

---

## 5. Order reduction, DAE and Prothero-Robinson analysis

### Already in the repo [READ]
- **E-04.** On pure PR (λ in [−1e6, −1]), even the direct-LU arm has global slopes 4.01, 4.03, 4.03, 3.63, 3.42.
- **E-09.**
  - PR at λ = −1e4: slopes 2.95-3.25.
  - Stiff diagonal mass matrix (ε = 1e-3): 3.22-4.78.
  - Constant mass matrix: 5.18-5.83.
- **Contract tests.** `tests/fixed_step_order_contracts.rs:140-165` fixes the stiff PR band at 2.7-3.6 and the mildly stiff band at 3.8-5.2.
- **Forcing ladder.** `tests/inner_forcing_fixed_step_ladder_contracts.rs` requires the inexact arm to stay within 3× of the direct arm.

### Ledger rows [LEDGER]
- **L-0040:** RODAS5P stays on the slow manifold (two-step error / h³ ≤ 1.7e-11) and has order 5.00 at κ = 8.
- **L-0051:** on the semilinear problem RODAS5P's error falls from 1.1e-4 to 3.9e-11 as K grows from 1e2 to 1e12.
- **Two-arm addendum (F-033):** every accepted local error is ≤ 1.36 tolerance units and their sum is 1.37-6.83, but the global error is 17-207 units. The flow amplifies the local errors.

### Prothero-Robinson [COMPUTED, `prdae.py`]
- **Fixed z = hλ (−1 to −1e6):** the local error scales as h⁴ (slope 4.0).
- **Error constant:** it peaks near z ≈ −10 and then decays like 1/|z|.
- **Fixed λ (−1e4, −1e6):** the local slope is 3.0-3.5.
- This matches the repo's bands: the PR order is about 4 at fixed z and about 3 at fixed λ.

### Index-1 DAE
- **Not tested in the repo [READ].** Only constant and diagonal mass matrices are tested.
- **[COMPUTED]** M = diag(1, 0), y' = −w, 0 = w + w³ − y:
  - differential component: local error h⁶, so order 5;
  - algebraic component: local slope rising to 4.9, approaching order 5;
  - the estimate scales as about h⁴.
- **Fast drivers [READ].** They refuse mass matrices entirely.

---

## Mathematical levers

| # | Lever | Code location | Robustness side | Cost side | Evidence |
|---|---|---|---|---|---|
| L1 | **Predictive controller plus no growth after a rejection** | `adaptive.rs:264-306`, `:396-447` | 3-10× fewer rejections | 0.72-0.91× LU count on van der Pol, 0.83-0.93× on HIRES at loose tolerance; ≈1.0× at tight | Rejection table and replica in §3. SPEED_RESEARCH_STATUS.md:66 lists CTRL-PREDICTIVE as "contested, needs a matched-accuracy design". HIRES costs up to 1.02× at 1e-6 to 1e-7. The current PI over-resolves (target err ≈ 0.17). |
| L2 | **Jacobian reuse rule** | `rodas5p_fast.rs:1346-1372`; tableau fixture | Naive reuse collapses the order to 3 (stale J) or 2 (W-method) | Replica: 2.5-5.8× more LUs | [COMPUTED] W-order table in §1 and §2. Reuse needs a W-method tableau (a different family) [INFERRED], or must be confined to the preconditioner (L3). |
| L3 | **Exact-JVP operator with a lagged or structured preconditioner (matrix-free U form)** | `rodas5p_matrix_free_fast.rs:156`, `:188-211` | Order unaffected, because the operator stays exact [INFERRED] | Unpreconditioned GMRES needs 40 iterations per solve (Brusselator n=100) and 49-78 (n=320); 325-650 matvecs per attempt (SPD07 BASE.json) | Risk: E-05 shows LGMRES failing after a preconditioner change, so Krylov state must be keyed to the preconditioner. L-0046/L-0059/L-0066 cover GCRO-DR recycling. |
| L4 | **Couple the matrix-free Krylov tolerance to the error estimate (forcing in the fast driver)** | `rodas5p_matrix_free_fast.rs:452-457`, `:35`; `sequential.rs:677-722`; `integrate.rs:819` | The protected rule exists, but the under-resolved flag is ignored at acceptance | Iterations per solve are flat in outer rtol (Brusselator-50: 39.7-40.4 at both 1e-6 and 1e-8 with linear rtol 1e-10) | E-04: GMRES error is about C_η·η with C_η ≈ 1e2-1e3 in K form. REV-03 err + B ≤ 1 is a safe acceptance guard (L-0062) but is too loose on stiff blow-ups (L-0049). |
| L5 | **Step-size choice that accounts for Krylov cost** | Controller as in L1; driver loop `rodas5p_matrix_free_fast.rs:760-860` | — | Brusselator-160: 73-78 iterations per solve at rtol 1e-6 against 49-50 at 1e-8 | Larger hγ‖J‖ makes W harder [INFERRED]. Maximising h for accuracy is not cost-optimal for unpreconditioned Krylov [INFERRED]. |
| L6 | **Calibrate the stiff estimator** | `coefficients.rs:195`; `rodas5p_fast.rs:771`; `adaptive.rs:27-33` | Must not become under-conservative on nonstiff modes | Over-estimation of 4.6-5× on stiff PR (L-0040, [COMPUTED]) costs about 4.7^(1/5) ≈ 1.36× in step size [INFERRED] | The estimate is accurate on homogeneous stiff modes (ratio ≈ 0.98). |
| L7 | **A tableau with higher Prothero-Robinson / stage order** (a method-family change) | Fixture plus `fixed_step_order_contracts.rs:140-165` bands | Removes order reduction from 5 to 3-4 on stiff forcing and stiff mass matrices | Larger admissible steps at tight tolerance on such problems | E-04, E-09, my PR analysis. Literature claims for Rodas4P/4P2-type methods are unverified [INFERRED]. |
| L8 | **Monitor internal stage states and abort early** | Stage loops `sequential.rs:389`, `rodas5p_matrix_free_fast.rs:423`, `rodas5p_fast.rs:721` | Catches the stiff-limit amplification of up to 3.41× in stages 2-6 before NaN or 1e117 states | A failed attempt can stop at stage k instead of running all 8 [INFERRED] | Replica overflow on van der Pol; Robertson stage blow-up in L-0049/L-0062. An atlas `StageGrowthBudgetMode::Predictive` exists (`g4_s5b0_regime_atlas.rs:3083`; V29_STAGE_GROWTH_BASELINE = 3.24, unsourced). |
| L9 | **Cheaper filtered dense-output estimate** | `dense_output_v2.rs:192-207` | Interior error control (currently uncontrolled; up to 312 units at λ = −1e5) | Today one J + one LU per sampled step; reusing the step's own factors (shift hγ instead of h/2) would avoid that [INFERRED] | DENSE_OUTPUT_ERROR.md tables. |
| L10 | **Global error control** | Controller | Local control does not bound global error (17-207 units against local ≤ 1.36) | Costs extra work | Two-arm addendum; the README there calls this "a method decision for the owner". |
| L11 | **Replace finite-difference f_t** | `problem.rs:253-279`, `:453` | Removes the finite-difference floor ∝ |λ| (E-09) | Saves 2 RHS per step on nonautonomous problems | Effect on y₁ is O(h³δ) nonstiff [COMPUTED]; minor. |
| L12 | **Stage predictors** (effectively closed) | `rodas5p_matrix_free_fast.rs:470-490` | — | Zero start is cheapest | L-0084: `Previous` costs 1.02-1.57× zero's matvecs; step-indexed starts 0.96-1.05×. In U form GMRES from zero already contains rhs_i [INFERRED]. |