# R4 evidence digest: VigilODE (HEAD a49f7e4), robustness and cost facts at the method level

This is a read-only digest. I wrote no files and changed nothing in the worktree.

**Conventions**
- `[L-xxxx]` is a ledger row in `research/LEDGER.jsonl`. `path:N` is a line in that file.
- **COMPUTED** means I recomputed the value from committed JSON with a read-only python one-liner (`stiff_native_benchmark_20261001/ANALYSIS.json`, `spd07_mf_step_warm_start_20261007/BASE.json`).
- **INFERRED** marks my own interpretation, not something a document states.
- Wall times are diagnostics only. Timing authority is on HOLD (`SPEED_RESEARCH_STATUS.md:7-12`; L-0007/L-0010 FAIL; L-0018/L-0025 held).

**Driver map**

| Driver | Status | Linear solves | Inner forcing | Krylov preconditioner |
|---|---|---|---|---|
| Library default (`README.md:52-58`) | default | adaptive matrix-free sequential RODAS5P in K-form | yes | — |
| Fast driver, `rodas5p_fast*` | research | dense LU, plus small-n and banded variants | — | — |
| U-form matrix-free fast driver, `rodas5p_matrix_free_fast.rs` | research | Krylov | none (`rodas5p_matrix_free_fast.rs:35-36`) | none |

- `LinearSolverConfig::default` uses `method: Direct`, `rtol 1e-11`, `restart 40`, `maxiter 200`, `preconditioner: None`, `x0: Previous` (`crates/rodas5p-core/src/solver_types.rs:54-69`).
- The only preconditioner kinds are None, Jacobi and Direct. Direct refactors the matrix on every apply and is unused (audit F-083).
- Step controller (`crates/rodas5p-integrators/src/adaptive.rs`):
  - integral controller by default (`:98-112`), with safety 0.9, min_factor 0.2, max_factor 5.0, reject_max_factor 0.9;
  - estimator order 5, exponent −1/5 (`:27-34`, `:288-294`);
  - PI is opt-in (`:290`);
  - after a rejection, the first accepted step may grow by up to 5x (`:396-417`). `output.rs:244-257` only forbids retrying at a size at or above the rejected trial.

---

## A. Robustness failures and near-failures

### A1. Accuracy and step control of the core RODAS5P method

| ID | Problem / regime | Mechanism | Status | Source |
|---|---|---|---|---|
| F-033 | Semilinear advection-diffusion, n=96: dense-arm global error 17 / 53.2 / 207 case-tolerance units at rtol 1e-4 / 1e-6 / 1e-8. Across n=96/384/1536: 17–261 units with only 60–66 accepted steps. 12/18 n=96 rows exceed tolerance. Against SciPy Radau at equal tolerance: median per-row error ratio 5.28, max 438.3 | Not the inexact solves (R_inner 0.80/1.00/1.00). Not fixed-h order reduction (slopes 5.01, 4.98, 4.99, 4.96). Not the W construction or the FD-JVP (ablation 16.7–16.8 / 53.1–53.3 / 200.5–207.4). Not the interpolant. Local error control holds: true local error ≤0.28/0.48/1.36 units, true/estimate ≤0.90/1.13/1.60. The global error is propagation: the sum of local errors (1.37/2.67/6.83) is 12–30x below the global error | **OPEN.** "Choosing a global-error control for error-amplifying problems. This is a method decision for the owner." | `research/scientific_validity_v2_20260829/ADDENDUM_20260928_OUTPUT_POLICY_AND_GLOBAL_ERROR.md:53-90`; `.../addendum_20260929_two_arm_v3/README.md:52-117`; audit `VIGILODE_ADVERSARIAL_AUDIT_20260927.md:214` |
| F-018 | Prothero-Robinson λ=−1e4, fixed step, direct LU: observed order 2.95/3.03/3.11/3.25. Also P1PR 4.0, diagmass ε=1e-3 3.2–4.8. λ=−1e2: 4.03/4.69. Nonstiff λ∈[−10,−1]: 5.4–5.9 | Classical ROW stiff order reduction (the tableau satisfies all 17 order-5 conditions, E-07). Audit: "PI/I controller exponent 1/5 … mis-scaled on stiff problems (efficiency)" | **ACCEPTED.** Locked as a test band of 2.7–3.6 with err(1/64)=1.75e-12 | `crates/rodas5p-integrators/tests/fixed_step_order_contracts.rs:139-166`; audit ledger F-018 |
| F-002 | Dense output: order 4 nonstiff, about 3 stiff. PR λ=−1e5: true interior maximum 3.2 / 312 / 209 tolerance units at rtol 1e-4/1e-6/1e-8 | Acceptance tests only the endpoint embedded estimate | **ACCEPTED as default (Off).** Opt-in `Enforce` gives 0.92/1.05/1.38, at 1 RHS + 1 Jacobian + 1 LU per sampled step | `docs/DENSE_OUTPUT_ERROR.md:5-50`; commit f06afd5 |
| F-078 | Every adaptive lane | No growth cap after a rejected step: factor up to max_factor=5 on the next acceptance. Hairer uses facmax=1 after a rejection | **OPEN.** No fix commit; code unchanged (`adaptive.rs:396-417`) | audit md:559 |
| Rejection rates (COMPUTED) | RODAS5P rejected/attempted at rtol 1e-3, 1e-4, 1e-5, 1e-6, 1e-7: **VdP μ=1e3 37/36/35/25/15%**; HIRES 21/29/15/3/1%; Brusselator 20/19/17/11/4–5%. Same tolerances: Hairer RODAS 14/8/4/2/1% (VdP), 5/2/1/0/0% (HIRES); RADAU5 6/4/3/3/2% (VdP); CVODE 7–11% (VdP) | INFERRED: controller-driven. Same estimator order, integral controller, no post-reject cap (F-078) | **OPEN.** CTRL-PREDICTIVE is listed as contested because it "changes results and needs a matched-accuracy design" | `research/stiff_native_benchmark_20261001/ANALYSIS.json` rows; `SPEED_RESEARCH_STATUS.md:66` |
| F-079 | SABR, transactional and homotopy lanes | `fixed_point_error`, which is not O(h⁵), is added to the embedded error before the ^(−1/5) exponent | **OPEN** (`integrate.rs:31`); affects efficiency only | audit md:560 |
| F-006 | Clipped (output-observed) lanes | Controller froze the requested step, which made the arm fixed-step at span/100 | **FIXED** (cf79ec5, 73dde4a) | audit md:168 |
| F-008 / F-009 / F-031 | Matrix-free K-form inner forcing. Error floor 0.10–0.32·rtol (P1PR) and 0.18–0.29·rtol (adaptive P1). Stage-0 abort at rtol ≤1e-8, \|λ\|≈1e6 | h-independent absolute residual budget, measured against a K-form RHS inflated by O(h\|λ\|) | **FIXED** (WU-3): budget scaled by min(1,err)^{6/5} (`g4_s5b0_inner_tolerance.rs:39-75`). Fixed-outer-rtol slopes 4.95/4.94 (`fixed_step_order_contracts.rs:338-355`). **The U-form MF fast driver has no inner forcing**; its studies use fixed linear rtol 1e-10 | `research/adversarial_audit_20260927/fixes/G2_inner_forcing_rule.json`; `rodas5p_matrix_free_fast.rs:35-36`; `spd07.../PREREGISTRATION.md:30-31` |
| Robertson, fixed h=1e-2 | One step diverges in both drivers (states about 1e117, error norm 1e6). The residual-to-output budget is 2e122 (h=1e-2) and 2e246 (h=3e-2) | Mean-value budget over a blown-up box | **ACCEPTED, safe side.** All 68 acceptances resolve under the one-sided rule; the unresolved cases are rejections | [L-0049], [L-0062]; `thread_transfer_mf_workspace.../PREREGISTRATION.md:110-113` |
| Oscillatory leaf (Fourier client model) | ω=1e4: RODAS5P final error 9.3e-7 at tol 1e-8 (about 93x tolerance, INFERRED from the numbers) and 4.2e-10 at tol 1e-10. That takes 6,315 attempts, 49,177 RHS and 4,972 Jacobians at 1e-10 | Resolves the carrier with steps. Never reached the certified client's level (2.2e-12) | **OPEN.** Not gated | [L-0079]; `research/pp15_fourier_comparator_20261004/PREREGISTRATION.md:52-66,79-96` |
| Linear stability of the binary64 tableau | \|R(iy)\| = 1 + 1.3e-19 at y=0.01. Semilinear two-mode error falls 1.1e-4 → 3.9e-11 as K goes 1e2 → 1e12 | Coefficient rounding at about 1e-16 | **ACCEPTED** | [L-0051]; `rnext08.../PREREGISTRATION.md:77-90` |
| Estimator effectivity | Quintic PR: RODAS5P 11 / 6.2 / 4.6 / 4.6 at z=−10…−1e6 (over-estimate). EXPRB43 up to 1.4e6. PEXPRB54S4 3.1–11.7 | Pessimistic estimate, which causes extra steps (INFERRED) | **ACCEPTED** | [L-0040]; `thread_transfer_negative_controls.../PREREGISTRATION.md:81-93` |

### A2. Krylov and linear algebra

| ID | Problem / regime | Mechanism | Status | Source |
|---|---|---|---|---|
| L-0046/52/57/58/59 | Recycled GCRO-DR fails on 21/336 frozen Brusselator solves; cold GMRES and cold GCRO-DR fail on none | Recycle updates break M⁻¹AU = C | **FIXED, opt-in** (`refresh_after_update`). The reuse check (19/21), start projection (56 left) and second pass (28 vs 12) were refuted | [L-0046], [L-0052], [L-0057], [L-0058], [L-0059]; `REVIEW_DAG_STATUS.md:18-22` |
| L-0066 (driver level) | Legacy recycling in the U-form driver: bruss-50 hits the 5,000-attempt cap with 2,453 / 2,475 linear failures. Bruss-160 completes but needs 320 / 420 attempts (102 failures) vs 92 / 193 with refresh or cold | Linear failure → rejected step. Cost and robustness are coupled | Refresh and cold both reach 0 failures, but **the default is still `Legacy`** (`rodas5p_matrix_free_fast.rs:65-71`) | [L-0066]; `safe_recycle_policy.../PREREGISTRATION.md:103-111` |
| L-0038 | Unpreconditioned GCRO-DR (rtol 1e-11) on Brusselator n=100 never completes within 5,000 attempts in the sequential, protected and U-form drivers | Alternates failed and accepted attempts ("Arnoldi budget exhausted"). A faer `generalized_eigen` scratch panic was fixed separately | **OPEN** (a property of that configuration) | [L-0038]; `thread_transfer_mf_workspace.../PREREGISTRATION.md:87,116-121` |
| F-038 / E-05 | Nonnormal Krylov stress: 54/96 rows fail (all fail closed). For s=1, GMRES+Jacobi succeeds or fails depending only on RHS scale (3,200 / >4,000 / 3,440 iterations). For s=10 and s=100, every solver exhausts 4,000 vectors | No stagnation detection in restarted GMRES(40) | **OPEN for GMRES.** Only traced GCRO-DR has a reset (`gcrodr.rs:507`) | audit md:562; `experiments/E-05/README.md` |
| E-05 stale suite / L-0044 | LGMRES fails closed in 3/6 preconditioner swaps. GCRO-DR with a mutated same-token operator burns 4,126 matvecs. Without an epoch, a mutated model mixes stale and live data, and the GCRO-DR least-squares step goes NaN | Stale recycle and operator identity | **FIXED opt-in** (the client promises an epoch). There is no automatic detection | [L-0044] |
| F-010, F-034, F-037 | GCRO-DR panic on a dimension change; NaN/Inf tolerances accepted; degenerate seeded GMRES "certified" | Input and state validation | **FIXED** (WU-1; 06052bf; `solver_types.rs:73-76`) | audit md:36 |

### A3. Exponential and φ actions, polynomial actions, directed arithmetic

| ID | Problem / regime | Mechanism | Status | Source |
|---|---|---|---|---|
| F-011 | Arnoldi happy breakdown at 64√ε reported error 0 while the true relative error was 3.9e-7 | Loose breakdown test | **FIXED** (f12c335) | audit md:136 |
| VIG-A02 | A = [[−2, 2^k], [2^-k, −2]]: an Arnoldi residual of 6e-15 hid a true error of 7e-2. Certified lower bound 7.35e-2 | Nonnormal; a residual is not an output error | **FIXED** as a label and certificate. Osborne balancing finds the metric automatically (k=46 certified at 2.2e-15) | [L-0056], [L-0060]; `int05.../PREREGISTRATION.md:6` |
| F-040 / F-042 | φ relative error 1.58e-8 at ‖v‖=1e8. Padé-13 overscaling: 7.5e-15 → 9.0e-12 as h goes 0.1 → 1e-4 | No input normalization; 1-norm squaring count | **FIXED** (f12c335, e93abca) | audit md:264,284 |
| Strongly nonnormal exp-action bounds | Random and Jordan matrices stay at 1e9–1e19 relative. Convection-diffusion decay is invisible to a Gershgorin box (3/6 cases fail) | Crouzeix–Palencia over a Gershgorin box | Tridiagonal convection-diffusion **FIXED** by the chain symmetrizer (6/6 within 1e-8; worst 3.0e-11). **Random and Jordan remain OPEN** | [L-0060], [L-0075], [L-0076]; `DAG_EXECUTION_STATUS.md:55-57` |
| Laguerre, stiff region | ρ=400, h=0.1: true error 6e-14–3e-13 but bound 9.5e6–2.1e8, so the budget rejects it. Majorant up to 3.5e65x loose | e^{L'/2}/max E_n = 4.196, so totals stay ≥2.3e6 against a 1e-6 budget | **OPEN** (likely the recurrence-adjoint term; the component fields are not exported) | [L-0047], [L-0077], [L-0020]; `POLYNOMIAL_PARALLEL_KO.md:16`; `DAG_EXECUTION_STATUS.md:58-60` |
| Taylor fused φ total bound | Unbounded for w ≈ 1e100 (e^ω overflows); useless for subnormal w (floor about 1e-15) | — | **ACCEPTED** limits | [L-0078] |
| Directed exp floor / compositions | `exp_interval` returned [0, 2^-1020] for x < −707, which is false on (−707.0101, −707). Base compositions under-covered: chart decay 2/20,200, stepped time direction 676/20,000, midpoint radius 2,660/20,000 | Rounding direction | **FIXED** (2^-1019; 40,511 points, 0 violations) | [L-0064], [L-0065]; `INDEPENDENT_REVIEW_DAG.md:16` |
| R4 transform false bounds | Example: A = [[0,.25],[0,0]], h=1000.1 gave a bound of 5.8e-11 against an exact 1.507e-9 | ExpBound ordering | **FIXED** (ARITH-DEV-01) | [L-0011]; `REAUDIT_R4_CLOSURE_20261001.md:24` |
| Complex shifts | With gain 1, 16/80 random shifts have error above ‖r‖₂ (up to 2.16x) | Resolvent gain is \|γ\|/Re γ, not 1 | **FIXED by design** (worst bound/actual 1.17) | [L-0070] |
| Dissipativity witness | The row-Gershgorin witness fails at all 28 Fourier step starts (row bound 0.056–0.115) | Sufficient condition only | **ACCEPTED** limit; the jet is not admissible there | [L-0074] |

### A4. Homotopy, transactional q1/q2 and charts

| ID | Problem / regime | Mechanism | Status | Source |
|---|---|---|---|---|
| L-0017/24 | Doubling certificate does not close at n=8 and 16 in 6 attempts | Radius cap too small (closes at attempt 7 or 8) | Diagnosed [L-0034]. The residual-seeded radius 2·max B E(0) closes in one preflight plus one check [L-0036]. **The old gate stays FAIL** | `thread_transfer_20261002/REVIEW_KO.md:197-216` |
| Common-radius obstruction | k_i = 1 + k_{i−1}²: no common radius closes | Single scalar D | **FIXED opt-in** (causal box) | [L-0036]; `REVIEW_KO.md:218-228` |
| A-RADIUS-01/02 | Negative α gave a false closure; mutable backing arrays | Validation | **FIXED** | `20261003_native_reaudit/COMPLETED_FINDINGS.json` |
| HOM-05 model check | A cubic ODE declared quadratic passed (agreement checked only at f(y) and one JVP) | Sampled agreement | **FIXED** (agreement now checked at every stage state) | `REAUDIT_R3_CLOSURE_20261001.md:76` |
| L-0055 | Triangular-chart Newton diverges on 3/8 cases | Chart without globalization | **CLOSED** (stopped) | `CRITICAL_REVIEW.md:45-48` |
| L-0003 / L-0001 | v3.7: 2 charged 80-JVP cap exhaustions out of 64 recommendations. v3.5: 1 exhaustion (provenance INCONCLUSIVE) | Budget cap | **ACCEPTED** (transaction semantics) | [L-0003], [L-0001] |
| HOM-06 coupled-linear-6 | pf = 1.000: q2 always falls back | No witness | **ACCEPTED** | [L-0008] |

### A5. Comparators (context)

- Repository BDF2 failed at rtol 1e-9 on Robertson and van der Pol ("maximum step count or minimum step reached") [L-0028] (`stiff_bdf_radau_benchmark.../PREREGISTRATION.md:104-105`).
- CVODE did not reach 1e-7 on van der Pol within the ladder [L-0029].
- Tolerance proportionality:
  - RODAS5P error/rtol stays within about 0.1–3.5 (`stiff_native_benchmark.../PREREGISTRATION.md:188-190`); COMPUTED maximum 4.4 (VdP, rtol 1e-8).
  - CVODE up to 14x (HIRES 1e-3) and 230x (VdP 1e-6) per the document; COMPUTED up to 1.8e3 (VdP, rtol 1e-8).

---

## B. Method-level cost facts

### B1. Work per attempt at rtol 1e-6 (COMPUTED from native ANALYSIS.json rows)

| Arm | RHS per attempt | LU per attempt | Jacobians per attempt | Notes |
|---|---|---|---|---|
| RODAS5P | **8.00** | **1.00** | **1.00** | 8 sequential solves per attempt (176 solves = 8 × 22 attempts). Per accepted step: 8.24–10.69 RHS and 1.03–1.34 LU (VdP worst, 118/469 attempts rejected) |
| Hairer RODAS, 6 stages, order 4(3) | 5.98–6.00 | 1.00 | 0.98–1.00 | — |
| Hairer RADAU5 | 6.90–9.58 | 0.87–1.00 (real + complex pairs) | 0.61–0.89 | — |
| CVODE | 1.16–1.48 | **0.12–0.20** | **0.02** | LU reused across steps |

- In the fast driver, Jacobian reuse after a rejection cuts Jacobians from 210 / 469 / 92 to 204 / 351 / 82, but every attempt still factorizes [L-0032] (`stiff_rodas5p_fast.../PREREGISTRATION.md:115-116`).
- RODAS5P has 8 stages against RODAS's 6 (`:149-150`).

### B2. Matched-error counts [L-0029]

Each cell is RHS / LU (Jacobians), taken from the cheapest run with error ≤ E (`ANALYSIS.json matched_error`).

| Problem, E | RODAS5P | RADAU5 | Hairer RODAS | CVODE |
|---|---|---|---|---|
| Robertson 1e-3 | 176 / 22 | 155 / 21 | 143 / 24 | 139 / 20 (3) |
| Robertson 1e-7 | 824 / 103 (needs rtol 1e-8) | 464 / 64 | 815 / 136 | 554 / 79 (8) |
| HIRES 1e-5 | 984 / 123 | 1140 / 103 | 1049 / 175 | 826 / 112 (12) |
| HIRES 1e-7 | 3248 / **406** | 2050 / **167** | 5153 / **859** | 1627 / 163 (23) |
| VdP 1e-3 | 1936 / 242 (needs rtol 1e-4) | 1739 / 213 | 1350 / 228 | 779 / 117 (19) |
| VdP 1e-7 | 7576 / **947** | 5539 / **594** | 11916 / **1987** | not reached |
| Brusselator-50 1e-5 | 736 / 92 (needs rtol 1e-6) | 370 / 52 | 477 / 80 | 236 / 24 (4) |
| Brusselator-50 1e-7 | 1544 / 193 (needs rtol 1e-8) | 697 / 99 | 1326 / 221 | 622 / 48 (10) |

- **Reading (INFERRED):**
  - RODAS5P needs about 1.0–2.4x RADAU5's factorizations.
  - It needs fewer than the order-4 RODAS only at tight tolerances (406 vs 859, 947 vs 1987, which the document also states at `:176-177`).
  - It needs 3.8–4x CVODE's factorizations on the Brusselator.
  - On the Brusselator, error/rtol is about 2, which forces a one-decade tighter rtol at E = 1e-5 and 1e-7.
- **Wall-time ratios at matched error (diagnostic):**
  - Hairer RADAU5 and RODAS took 0.04–0.25x RODAS5P's time; CVODE took 0.19–1.34x [L-0029].
  - Lean driver [L-0032]: 3–11x faster than the sequential path, and 1–3.7x behind Hairer (it was 4–25x behind).
  - v2 driver [L-0033]: every native arm took 1.03–4.0x v2's time on both Brusselators; on the small problems Hairer stays at 0.41–1.9x of it.
  - CVODE with OpenBLAS is still 0.53–0.98x on the Brusselator because it reuses one factorization across many steps (`stiff_rodas5p_fast.../PREREGISTRATION.md:142-146`).
- **LU implementation, n=400 banded:** faer 3.0 ms vs DECSOL 0.31 ms vs OpenBLAS 2.1 ms. CVODE needs 4–10 Jacobians and 15–49 factorizations per run, against 41–298 for RODAS5P (`stiff_native_benchmark.../PREREGISTRATION.md:158-187`).

### B3. SciPy comparison at rtol 1e-6 [L-0028]

Each cell is accepted steps, Jacobians, LU, RHS (`stiff_bdf_radau_benchmark.../PREREGISTRATION.md:124-131`).

| Problem | RODAS5P | SciPy BDF | SciPy Radau |
|---|---|---|---|
| Robertson | 43, 45, 45, 360 | 144, 4, 33, 366 | 78, 18, 100, 647 |
| HIRES | 204, 210, 210, 1680 | 327, 25, 85, 911 | 210, 75, 232, 1931 |
| VdP | 351, 469, 469, 3752 | 847, 57, 198, 2620 | 616, 122, 430, 5167 |
| Brusselator n=100 | 82, 92, 92, 736 | 198, 2, 37, 538 | 124, 23, 90, 948 |

- The document's reading: RODAS5P "takes fewer and longer steps" but is more expensive per step (`:149-154`).
- Against the repository's own reference BDF2 and Radau, RODAS5P was 1.1–378x cheaper on HIRES, VdP and Brusselator, and 2.2–5.9x costlier than the repository Radau on Robertson. Those comparators are reference implementations, so the document draws no ranking from them (`:138-147`).

### B4. Matrix-free U-form Krylov work (COMPUTED from SPD07 `BASE.json`)

Configuration: GMRES-into, restart 40, linear rtol 1e-10, atol 1e-14, no preconditioner.

| Problem (n) | rtol | Iterations per stage solve | Linear matvecs per accepted step (Zero / Previous) | JVPs per accepted step |
|---|---|---|---|---|
| Robertson (3) | 1e-6 | 2.2–2.3 | 27.2 / 33.9 | 35.5–42.3 |
| VdP (2) | 1e-6 | 2.0 | 32.0 / 41.4 | 42.7–52.1 |
| HIRES (8) | 1e-6 | 7.0 | 65.5 / 72.9 | 73.7–81.1 |
| PR (1) | 1e-6 | 0.9 | 13.8 / 21.7 | 21.8–29.7 |
| Brusselator-50 (100) | 1e-6 / 1e-8 | 39.7–40.4 / 39.8–39.9 | 365 / 379 (1e-6) | 374–388 |
| Brusselator-160 (320) | 1e-6 | **74.5–78.5** | 685 / 730 | 694–739 |
| Brusselator-160 (320) | 1e-8 | 48.5–50.6 | 404 / 428 | 412–437 |

- INFERRED: iterations per stage grow with n and with h, which is the conditioning of W = I − hγJ without a preconditioner.
- The default `Previous` start costs 1.02–1.57x the matvecs of a zero start in 14/14 cases [L-0084] (`spd07.../PREREGISTRATION.md:96-101`).
- A stage-indexed warm start from the previous step saves at most 4% (50.6 → 48.5) [L-0084].
- The K-form needs 7 more RHS-assembly JVPs per attempt than the U-form. On Brusselator n=100 the U-form took 328 GMRES iterations per attempt against 315 for the K-form, so total JVPs were 351 against 345 [L-0038] (`thread_transfer_mf_workspace.../PREREGISTRATION.md:103-105`).

### B5. Krylov recycling cost

- GCRO-DR refresh costs 1.08–1.32x cold GCRO-DR's operator products on the Brusselator and 0.98x on CDR; "recycling saves nothing here" [L-0059].
- On the CDR family, recycling used 0.80x cold GCRO-DR's products [L-0052].
- Driver level [L-0066]: on the five small problems, cold costs 1.3–2.6x more than refresh (HIRES 1e-8, legacy/refresh/cold: 23,515 / 23,522 / 61,896). On the Brusselator, cold is 4–23% cheaper than refresh.

### B6. Homotopy and transactional parallel stages

- HOM-06 [L-0008]: about 5x the baseline's RHS and 6–8x its JVP vectors (e.g. diagonal-quadratic-8: 3,600 vs 649 RHS). Speedup point 0.174 / 0.072 / 0.048 / 0.037 at P = 1/2/4/8.
- Serial certificate: 425–26,000 operations, the cheapest at every n. The idealized budget at n=16, P=8 is 7.38 against 8 solves [L-0024].
- Cost model at P ≥ 8: the margin is 1 + p1 − 8·pf W-solves. Recorded margins 0 / 1.25 / 0.30 / 1.0714 / 0.6190 (`thread_transfer_20261002/REVIEW_KO.md:258-270`; `20261002_thread_transfer/REVIEW_KO.md:187`).
- Action-first certificate costs 0.565x the operations [L-0035]. The structured certificate costs 27,200 against 349,760 at n=64 [L-0053].
- Q2 net margin with the structured certificate (derived): −20.5 / −10.0 / −3.2 / −0.33 / **+0.63** solve units per attempt at n = 1/2/4/8/16. Charging each directed operation as k plain operations gives +0.63 / +0.26 / **−0.11** / −0.49 for k = 1..4 (`Q2_ACTIVATION_DECISION.md:24-30`; `CRITICAL_REVIEW.md:10-21`).

### B7. Polynomial and φ actions on a frozen operator

- POLY-03 [L-0009]: warm Chebyshev 3.17x faster than Arnoldi (interval 1.17–13.41); cold Chebyshev 0.57 (Inconclusive); cold Laguerre 0.12 (Blocked); warm Laguerre 1.18. Statistical-authority hold applies.
- A cached eigensystem (124,416 setup + 4,056 per action) beats Chebyshev (60,480 per action) from the third action. The certified arm's enclosure work costs 2.4–22.8x the warm wall time [L-0027].
- Laguerre is never cheaper (1.0–1.57x the vector products) and is routed 0/79 times [L-0071].
- Leja/Chebyshev product ratio is 0.83–2.0 (median 1.15); Leja uses fewer only in 8 cases (ρ=200, h=0.2) [L-0080].
- Laguerre envelope setup costs 2.3e4–7.5e6 interval operations for m = 16..128, which pays off after 11–460 actions at n=8 [L-0042].
- Shared-shift jet: positive counted-flop margin in 49/90 configurations (up to 12.6x), none for equal shifts [L-0067]. RHS compression reduces LU solves to k(d+1), e.g. 16 against 256 [L-0068].

### B8. Instruction counts (context only; programming level, closed by two speed cycles)

Instructions per attempt, best driver against Hairer RODAS: 2,348 vs 2,947 (VdP), 8,224 vs 9,157 (HIRES), 0.98M (banded) vs 8.83M (Brusselator n=400) (`SPEED_RESEARCH_STATUS.md:164-169`).

---

## C. Closed directions (do not re-propose)

1. **Generic no-chart RVJ5 replacing RODAS5P** [L-0051, L-0040]. One-step error and step-map derivative grow like K² (1.1e-3 → 1.3e17). Two-step error/h³ tends to 4/3. Effectivity is 5.0e-6 at z=−1e6. Stopped (`INTEGRATED_PLAN.md:31`).
2. **Default native q=2 transactional or parallel-stage activation** [L-0008, L-0050]. 5x RHS, speedup 0.04–0.17, margins negative for n ≤ 8, and the n=16 margin disappears at k ≥ 3. Stays opt-in; ACTIVATE and PP13 are BLOCKED (`Q2_ACTIVATION_DECISION.md:37-44`; `DAG_EXECUTION_STATUS.md:33-35`).
3. **Six-attempt ×4 radius schedule** [L-0017, L-0024]. The cap was too small; superseded by the residual-seeded radius [L-0036].
4. **Static stage-coordinate chart adapter** [L-0055]. The target is block lower triangular and is solved exactly by one sequential pass; Newton diverges on 3/8 (`CRITICAL_REVIEW.md:45-48`).
5. **GCRO-DR recycle repairs other than refresh.** Absolute-defect reuse check [L-0052], start projection [L-0057] and second projection pass [L-0058] were all refuted. Refresh works but saves nothing on the Brusselator [L-0059]. Recommendation: cold GCRO-DR or GMRES (`REVIEW_DAG_STATUS.md:18-22`).
6. **Continuous Laguerre scale L\*(m)** [L-0021]. 0/22 degree reductions.
7. **Laguerre as a cheaper backend** [L-0071, L-0009]. Never cheaper; cold Laguerre Blocked at 0.12.
8. **Leja as certified or cheaper** [L-0080]. Always EstimateOnly; median 1.15x Chebyshev's products.
9. **Stage-indexed warm start from the previous step** [L-0084]. At most 4% fewer iterations.
10. **Selective second Gram-Schmidt pass** (KRY-MGS-SELECTIVE). The DGKS skip test fires on 97–100% of columns (`SPEED_RESEARCH_STATUS.md:50-54`).
11. **Shared-shift jet for common-γ RODAS stages.** No new LU saving. The Fourier client has no shifted solves [L-0067, L-0074] (`POLYNOMIAL_PARALLEL_KO.md:7`).
12. **Pseudo-arclength or fold handling for stage homotopy.** det D_K F = det(W)^s, so the stage system has no fold (`20261002_thread_transfer/REVIEW_KO.md:65-75`). Hensel root loops on triangular stages are flagged as unnecessary (`thread_transfer_20261002/REVIEW_KO.md:362`).
13. **Hand-chosen or theory-chosen metrics and Gershgorin-box bounds for decaying convection-diffusion.** Exponential weights make transport worse (up to 2.4e9) [L-0056]; Gershgorin cannot see decay [L-0060]; power-of-two Osborne cannot symmetrize the convection-dominated cases [L-0075]. Superseded by the chain symmetrizer [L-0076].
14. **"Inexact stage solves", "interpolant" and "fixed-h order reduction" as causes of F-033.** All refuted by the fixed-mesh ablation.
15. **Arnoldi terminal residual or sampled residual as an output-error certificate.** Closed by VIG-A02 and TF-09.
16. **Approximate-W substitution** treated as safe for stiff accuracy, and **generic low-depth W5 via variable γ**. Forbidden extrapolations: at fixed relative W error, RJ/RVJ amplification grows like z²/z⁴; a strict W-method needs depth D ≥ p (`REVIEW_KO.md (rvj):79-83`; `thread_transfer_20261002/REVIEW_KO.md:357-358`).
17. **The 0.1 output-policy rule** [L-0004, F-007]. Unattainable by construction; SciPy Radau also fails it in 17/18 rows.
18. **Single-case paired timing** [L-0007, L-0010]. Coverage only 0.78–0.93. Programming-level closures (SPD02 at n ≤ 8, the slices kernel, lane batching at N=2) are excluded here.

---

## D. Method-level open questions the documents name

1. **Global-error control for error-amplifying problems (F-033).** The documents call it an owner decision (`addendum_20260929_two_arm_v3/README.md:116-117`).
2. **Stiff order reduction and controller exponent.** Order is 3–4 on stiff PR, and the audit says the 1/5 exponent is "mis-scaled on stiff problems (efficiency)" (audit F-018).
3. **Controller.** Growth cap after rejection (F-078, open). Fixed-point error inside the exponent (F-079, open). CTRL-PREDICTIVE needs a matched-accuracy design (`SPEED_RESEARCH_STATUS.md:66`).
4. **Matrix-free driver.** Zero start as default (`SPEED_RESEARCH_STATUS.md:192`). KRY-STAGE-PROJ is contested (`:66`). No inner forcing is ported (`rodas5p_matrix_free_fast.rs:35-36`). GMRES stagnation detection and an FOV/pseudospectral probe (F-038). Default recycle policy still `Legacy` (L-0066 note). A residual-relative reuse rule (`INTEGRATED_EXECUTION_STATUS.md:51`).
5. **Decision rule for blown-up candidates** in the residual-to-output budget (`INTEGRATED_EXECUTION_STATUS.md:52`; `INTEGRATED_PLAN.md:30`).
6. **Nonnormal certificates.** Scaling and squaring when τ\|W\| is large (`INTEGRATED_EXECUTION_STATUS.md:54`). A verified numerical-abscissa bound and a range-minimizing metric (`REVIEW_DAG_STATUS.md:34`). Strongly nonnormal random and Jordan cases (`DAG_EXECUTION_STATUS.md:55-57`).
7. **Structural witness for non-diagonal J.** Needs a decay-bound witness; a dense \|W⁻¹\| is the obstacle (`CRITICAL_REVIEW.md:95`).
8. **Laguerre stiff bound.** Locate the recurrence-adjoint loss; component fields must be exported first (`DAG_EXECUTION_STATUS.md:58-60`). A tighter transfer bound (`REAUDIT_R4_CLOSURE_20261001.md:202-204`).
9. **q=2 at n ≥ 16.** Needs a measured directed/plain operation cost and timing authority (`Q2_ACTIVATION_DECISION.md:41-44`). For homotopy: candidate quality, residual preflight and measured fallback rate in one cost model (`thread_transfer_20261002/REVIEW_KO.md:272`).
10. **A real coupled client with genuine shift families.** Compare against Schur reuse, block multishift Krylov and rational/partial-fraction actions. For oscillatory and skew problems: complex Chebyshev and ellipse Leja/Faber (`REVIEW_KO.md (rvj):210-218`; `POLYNOMIAL_PARALLEL_KO.md:42-44`). General Fourier closure q(t) (PP14, DEFERRED).
11. **Time or stage parallelism** (waveform relaxation, Parareal, SDC) with stability and iteration-depth checks (`REVIEW_KO.md (rvj):183-187`).
12. **Stiff-order conditions and derivative-light correction or peer classes.** Only as a scoped exploration; cites Roberts–Shirokoff–Biswas–Seibold, arXiv:2505.15099 (`REVIEW_REMAINING_KO.md:83-89`).
13. **LU and Jacobian reuse economics.** "A native BDF with LU reuse could plausibly match or beat RODAS5P" for large systems (`stiff_bdf_radau_benchmark.../PREREGISTRATION.md:156-159`). INFERRED: for a ROW method this needs a W-method or preconditioner-reuse route (see C16).
14. **Smaller open items.**
    - Default interior error control for dense output; Enforce costs 1 Jacobian + 1 LU per sampled step (`DENSE_OUTPUT_ERROR.md:29-31`).
    - Directed budget check `error_upper ≤ budget_lower` (`REAUDIT_R2_CLOSURE_20260930.md:44`).
    - Banded gains for problems without a declared band (`SPEED_RESEARCH_STATUS.md:74`).