# R3 map: exponential, polynomial, homotopy and certificate paths, and their recorded outcomes

Read-only map of `/home/user/wt-speed` at HEAD `a49f7e4`. Nothing in the worktree was modified. Citations are `crate/src/file.rs:line` (integrators = `crates/rodas5p-integrators/src`, core = `crates/rodas5p-core/src`). Ledger IDs refer to `research/LEDGER.jsonl`. I mark as INFERRED anything I derived rather than read.

## 0. Summary

- **Exponential Rosenbrock exists but has never been compared with RODAS5P end to end at matched accuracy in the ledger.**
  - The only native evidence is: order screens (G2), method-labelled negative controls (L-0040), and the v3.5–3.7 *shadow* polyalgorithm (L-0001..L-0003).
  - In the shadow, at the 64 recommended events, a full PEXPRB54S4 step used **0.10–0.38x the JVPs** of the RODAS5P matrix-free attempt at the same (t, h). Its *self-estimated* error was comparable: median ratio of E error to R error 1.07; E was smaller in 31 of 64 events.
  - Switching was never activated, and the safety holdout is statistically empty (F-023 erratum).
- **Every certificate is dense and small-n, and none drives control.**
  - Coverage: nonnormal exp up to n=32, Chebyshev/Laguerre on symmetric nonpositive operators, stage-target certificates only for diagonal J or n≤2, complex shift with H=I.
  - The matrix-free phi path reports only a residual estimate, which on a nonnormal operator can hide large errors (VIG-A02: estimate 6e-15, true error 7e-2).
  - No controller reads a log-norm, spectral or certificate quantity. The "stiffness/nonnormality proxies" in the regime code are declared case parameters.
- **Homotopy/transactional parallel stages lost decisively on work:**
  - about 4.6–5.8x RHS, 6–8x JVP and 5.6–6.4x linear solves;
  - speedup point 0.04–0.17 (L-0008);
  - net certificate margins are negative for n≤8 (L-0050).
- **Polynomial bases:**
  - **Chebyshev** wins only in a frozen-operator, warm-coefficient regime (L-0009, 3.17x).
  - **Laguerre** is closed (L-0014/L-0021, L-0071, L-0077).
  - **Leja** is estimate-only and median 1.15x the products of Chebyshev (L-0080).

---

## 1. Exponential Rosenbrock candidates

### 1.1 Methods implemented

| Method | Source | Stages / phi functions | Order (embedded) | Notes |
|---|---|---|---|---|
| EXPRB2 (exponential Rosenbrock–Euler) | integrators/exponential.rs:2244 | 1 action of φ1(hJ)f0 | 2 (none) | no error estimate |
| EXPRB43 | exponential.rs:2279 | U2 at c=1/2 (φ1), U3 (φ1 plus φ1 on D2); update `16φ3D2 − 48φ4D2 − 2φ3D3 + 12φ4D3` (exponential.rs:2325); embedded `16φ3D2 − 2φ3D3` (:2336) | 4 (3) | nonlinear remainder `D_i = f(U_i) − f0 − J(U_i − y)` (exponential.rs:2106–2125), one JVP per stage |
| PEXPRB54S4 (Luan–Ostermann 2016, parallel; g2_exponential_gate.rs:168) | tableau exponential.rs:907–930; step :2359 | c = 1/4, 1/2, 9/10; φ1, φ3, φ4; U3 and U4 independent (critical depth 3) | 5 (4) | unfused = 15 Krylov actions per step; fused = 5 augmented actions (U2, U3, U4, main, embedded; g3 reports `legacy_to_fused_phi_action_ratio 15/5`) |

Scope restrictions:
- **Autonomous problems with identity mass only.** `validate_problem` refuses anything else (exponential.rs:2048–2064). INFERRED: the regime-atlas families run the nonautonomous case by autonomization, because they carry `excluded_trailing_components`.
- **Order evidence** (g2_exponential_gate.rs:670–676; tests/exponential_order_contracts.rs:4–8):
  - The scalar y'=y² screen cannot detect order reduction.
  - On a stiff non-commuting problem the observed global slopes were 4.19, 4.39 and 4.92 for |λ|h from 100 down to 6.25. At λ=−1e4 the order is about 4.1 (an exact-phi replica gives 4.15/4.09/4.08).
  - So the stiff order collapses from 5 to about 4. A Krylov dimension cap lowers it further.

### 1.2 How the phi actions are computed (error status in brackets)

1. **Unfused Arnoldi `krylov_phi_action`** (exponential.rs:983). Full MGS, dimension 4..24 in steps of 2, rtol 1e-11 (exponential.rs:15–35). [estimate]
2. **Fused augmented-matrix Krylov `fused_phi_action` / `fused_phi_linear_combination`** (exponential.rs:1514, :1565, :1657).
   - Builds the time-normalized augmented operator with weights `w_k = τ^k b_k`, formed on binary exponents (exponential.rs:1408).
   - Substepping takes exp(M̂) as exp(M̂/m)^m; m doubles on failure up to 16 (`phi_restarts`, exponential.rs:1647–1648).
   - Per substep, `krylov_exponential_once` (exponential.rs:1255) checks convergence at checkpoints with the KIOPS/Saad first-term residual estimate against `atol + rtol·max(‖current‖, ‖v‖)` (exponential.rs:1346).
   - Arnoldi stops early only on an exactly zero residual (exponential.rs:947). The comment there records VIG-A02: A=[[−2, 2^46], [2^−46, −2]], residual estimate 6e-15, true error 7e-2.
   - The status enum `PhiConvergenceBasis` (exponential.rs:145–200) is `ResidualEstimate` unless the space is invariant or full, so for nonnormal operators the result is **never a bound**.
   - `PhiTransformStatus` (exponential.rs:209) flags weights that underflowed.
   - [estimate]
3. **Dense oracle**: Padé (13,13) with scaling and squaring on projected Hessenberg or augmented matrices (core/matrix_functions.rs:62–67, :172, :247). [reference only]
4. **Scaled Taylor `taylor_phi_action`** (core/taylor_phi.rs:1–33), after Al-Mohy–Higham.
   - Exact-arithmetic tail bound `s·e^{(s−1)ν} r_m ‖v‖₁`, with ‖M‖₁ ≤ 64 (`TAYLOR_NORM_LIMIT`, :46) and degree ≤ 60.
   - [EstimateOnly: the rounding of s·m products is not bounded]
   - Its total-target certificate is `taylor_phi_total` (core/taylor_phi_total.rs:15–27): a triangle inequality of (candidate distance) + (REV-02 stepped certificate of exp(M̃)v) + (perturbation ‖Δ‖e^ω‖v‖). [certificate, PP11/L-0078]
5. **Chebyshev/Laguerre `joint_phi_action`** (core/polynomial_action.rs:1–56, :1360).
   - Dense exactly-symmetric A with spectrum enclosed in [−ρ, −λ], Gershgorin-verified or declared.
   - Coefficients use directed-rounding series (Bessel/Kummer for Chebyshev, Pfaff/₂F₁ for Laguerre).
   - Truncation bounds: Chernoff for Chebyshev; `e^{L'/2} q^{m+1}` for Laguerre.
   - The total is `Certified` only for Chebyshev with a verified enclosure. Laguerre scales are fixed at {1, 2, 4, 8, 16} with cap 16 (:93–95). Laguerre gets a total only through the signed adjoint (item 7) when degree ≤ 128.
   - A router admits the basis with fewer products (core/polynomial_action/router.rs:1–12).
6. **Newton–Leja** (core/leja_action.rs:1–73). Real discrete Leja points on a grid of 2^15 points; divided differences via Opitz exp(Z) with no cancellation; degree cap 128 (:94). [always EstimateOnly: `LEJA_TOTAL_NOT_CERTIFIED`]
7. **Laguerre signed output adjoint** (core/laguerre_adjoint.rs:1–32).
   - Exact summation-by-parts identity `Σ c_n(t̂_n − t_n) = Σ z_j(X)δ_j`.
   - β_j ≥ sup|z_j| from interval Bernstein envelopes; degree ≤ 128 (:41); depth ≤ 12 (:47).
   - [one proved component]
8. **Complex-shift resolvent certificate** (core/complex_shift_jet.rs:1–18). For (J+Jᵀ)/2 ≤ 0 (verified row bound) and Re γ > 0, `‖(I − γhJ)^{-1}‖₂ ≤ |γ|/Re γ`. Accepts a candidate through the recomputed outward residual. Covers partial-fraction outputs. H = I only. [certificate]
9. **Shared real-shift jet** (core/shared_shift_jet.rs:1–14) and the 4-method policy (core/shared_shift_policy.rs:1–15): per-target LU, common-shift LU, Hessenberg reuse, jet. [certificate through the residual gate]
10. **Fourier/contour client** (integrators/fourier_path_certificate.rs:1–24; FFT predictor fourier_path_candidate.rs:1–11).
    - Model-specific: two complex amplitudes on the invariant leaf |a|² + 2|b|² = 9/16.
    - Paths `Σ c[k,j] τ^j e^{ikωhτ}`, certified by a directed noncyclic product.
    - [certificate for that model only]

### 1.3 Adaptive exponential driver

`integrate_pexprb54s4_fused_adaptive_observed` (integrators/adaptive_exponential.rs):
- `total_error = max(time_error, phi_error)` (:290).
- The phi error proxy converts the Euclidean Krylov estimates to WRMS with `1/(√n · min w_i)` (:82–100, audit F-043). This is conservative.
- Acceptance is `total_error ≤ 1` and a finite state (:308).
- The controller is the standard one with order exponent 5 (:340, :348). There are only Integral and PI controllers (integrators/adaptive.rs:8–10, :290–292).
- No stiffness detection, spectral or log-norm input.
- Phi tolerances are tied to the integration rtol in the gates: g3 uses `0.03·rtol`, `3e-4·rtol` (g3_fused_adaptive_gate.rs:322–333). The atlas uses `inner_tolerance_policy(rtol)` (g4_s5b0_inner_tolerance.rs:296–305), with dimension up to n+4 ≤ 32 and 16 substeps.

### 1.4 Recorded outcomes for the exponential family

- **G2 gate** (g2_exponential_gate.rs:575–668): order gates are ≥1.8, ≥3.7 and ≥4.7, and the structural gate requires 0 Jacobians, factorizations and Newton iterations. `performance_promotion_authorized: false` (:663). No G2 or G3 output file is committed, and no ledger row records an exponential-vs-RODAS5P work-precision run (I grepped ledger claims: none).
- **G3 gate status** (g3_fused_adaptive_gate.rs:215–226) checks only success and structure, not speed.
- **L-0040 (PASS, negative controls)**, observed order at κ=8 and estimator effectivity at z = −10 … −1e6:

  | Method | Observed order at κ=8 | Effectivity, z = −10 … −1e6 |
  |---|---|---|
  | RODAS5P | 5.00 | 11 → 4.6 |
  | PEXPRB54S4 | 4.99–5.01 | 11.7 → 3.1 |
  | EXPRB43 | 4.0 | 29 → 1.4e6 (its estimator over-estimates by 1.4e6) |

  Source: research/thread_transfer_negative_controls_20261002/PREREGISTRATION.md:79–90. INFERRED: the EXPRB43 overestimate means rejections that are not needed.
- **Shadow polyalgorithm v3.5–3.7 (L-0001 INCONCLUSIVE, L-0002 PASS, L-0003 PASS):**
  - The committed method is the matrix-free RODAS5P (R-JF).
  - At trigger events a speculative PEXPRB54S4 prefix (levels 1+2) runs under a JVP budget `min(80, ⌊0.25·R_k⌋ − S_k)` (g4_s5b0_regime_atlas.rs:3074–3083).
  - It recommends E when `zeta34 ≤ 13.397` (:433, :450–461).
  - v3.6 economics: all 127 events used 3,799 shadow JVPs = **0.98%** of the 388,999 committed R-JF JVPs. The 64 recommended target attempts used 2,586 vs 13,043, i.e. **19.8%** (research/generic_frozen_full_e_shadow_v36/reports/RESULT.md).
  - Paired wall ratio of shadow to R-only had median 1.0245 (that is overhead, since R-JF is always committed), with N=384 noise up to 13.8x.
  - v3.7 continuation: 62 of 64 completed under an 80-JVP cap; 2 exhaustions on semilinear advection–diffusion.

  Per-family aggregation from the v36 runtime shards (my one-off read of `results/runtime/*/*.json`; errors are **self-estimates**, with E = max(time, phi) WRMS and R = RODAS5P embedded):

  | Family | Recommended / events | Median JVP of E / R-JF target | E JVP vs R JVP (median) | E err / R err (median, range) |
  |---|---|---|---|---|
  | hires-ramped | 7/12 | 0.384 | 33 vs 86 | 2.47 (1.84–5.20) |
  | nonautonomous-stiff-forcing | 4/53 | 0.229 | 27 vs 118 | 0.75 |
  | robertson-ramped | 9/9 | 0.280 | 33 vs 118 | 7.46 (3.5–93.7) |
  | rotating-nonnormal | 27/28 | 0.232 | 33 vs 142 | 0.99 |
  | semilinear-adv-diff-ramped | 6/8 | 0.112 | 90 vs 880 | 0.14 |
  | van-der-pol-ramped | 11/17 | 0.218 | 31 vs 142 | 0.67 (0.02–39.7) |

  All 64 E steps were self-admissible (total ≤ 1). E was evaluated only at the R-JF step size h, never at a larger h.
- **Safety statistics:**
  - The F-023 erratum (research/generic_enforced_prefix_budget_v35/reports/ERRATUM_20260929_F023.md) shows the N=320 holdout has 1 positive (HIRES, zeta34 = 14.32, q_E = 1.135). The pass is decided only by the one-rank backoff, so the result is `INCONCLUSIVE_INSUFFICIENT_POSITIVES`.
  - Labels are self-estimates, and the reference-anchored labels from F-045 were never run on this holdout.

### 1.5 Where the exponential family lost or has not won

- **Robustness:**
  - stiff order about 4 instead of 5;
  - estimate-only phi convergence for nonnormal J (VIG-A02);
  - autonomous and identity-mass only;
  - EXPRB43 estimator pessimism of 1.4e6;
  - Robertson events show E error 3.5–94x R error at the same h (self-estimates).
- **Cost:** no matched-accuracy trajectory comparison exists. The shadow JVP advantage is per attempt at R's h, and its error parity is mixed.
- **Process:** switching never activated; holdout non-discriminating; timing authority on HOLD (L-0007/L-0010 single-case coverage fails).

---

## 2. Certificates: what they bound and at what cost

| Module | Bounds | Assumptions / domain | Cost | Outcome |
|---|---|---|---|---|
| core/nonnormal_certificate.rs:1–19, :76 | ‖exp(τA)v − x‖₂ via Crouzeix–Palencia (1+√2)·sup over a Gershgorin numerical-range box, Taylor truncation, interval Horner | dense A, diagonal metric D | O(m n²) interval ops per Taylor step | L-0056 PASS (66 cases enclose; VIG-A02 Arnoldi candidate certified wrong by ≥ 7.35e-2; convection–diffusion transport up to 2.4e9) |
| stepped `certify_exp_action_stepped` (:340), `stepping_rule` N = pow2(⌈τR⌉) (:542), Osborne metric (:279), auto (:574) | stepped propagation e^{N h a_hi} | n ≤ 32 tested, degree 20 | N·20 dense interval matvecs (INFERRED) | L-0060 FAIL: absolute 9e-14..7e-13, but no decay (Gershgorin upper 0 vs true about −10); strongly nonnormal or Jordan 1e9–1e19 relative; Osborne fixes VIG-A02 (2.2e-15 in 4 steps vs 3.5e13 steps with identity) |
| verified log-norm `symmetric_part_upper` (:712, interval Cholesky with up to 40 tries); chain symmetrizer (:825); auto3 (:849) | e^{tμ} propagation | dense / tridiagonal | O(n³) per Cholesky try (INFERRED) | L-0075 FAIL (μ = −9.86 replaces 0; 3/6 F2 cases); L-0076 FAIL on a design defect, but all 6 F2 cases within 1e-8 relative (worst 3.0e-11, μ = −2/h²) and 10/12 holdout cases (the 2 failures have true values below the binary64 range) |
| core/transform_bound.rs:1–22, :506 | Σ C_k ‖δ_k‖ for rounded weights | only nilpotent, or dissipative by Gershgorin symmetric part; otherwise Unbounded | negligible | R3 closures (in R3/R4 closure docs; no dedicated ledger row) |
| core/directed.rs:1–11, `exp_interval` :298 | outward intervals | binary64, FMA | about 2–10x plain (INFERRED; PP13 needed the calibrated cost and is BLOCKED) | L-0064 FAIL (floor 2^-1020 false on (−707.0234, −707)); L-0065 PASS |
| core/laguerre_adjoint.rs | recurrence error component | symmetric X, spec in [0, L'], m ≤ 128 | setup 2.3e4–7.5e6 interval ops for m = 16..128 (exponent 2.78) | L-0039 PASS (7.35x–7.6e13x tighter than the majorant); L-0042 PASS (amortizes after 11–460 actions at n=8, 0.3–13 at n=64); L-0047 PASS (52 admitted, bound/error 7.4–1452, median 125) |
| integrators/outward_certificate.rs:1–31 (`certify_stage_target` :1155, doubling :1350, blocked :1551, action :1877) | componentwise \|K̂ − K*\| for the declared quadratic family stage equations | W⁻¹ witness: diagonal or dense n ≤ 2 (transactional_q1_q2.rs:504 "small witness needs n <= 2") | serial 425–26,000 ops (n = 1..16) | L-0017/L-0024 FAIL (doubling fails to close at n = 8, 16 in 6 attempts); L-0034 PASS (8 attempts); L-0035 PASS (action-first 0.565x ops); L-0053 FAIL on a threshold, but structured slope 1.00 (27,200 vs 349,760 ops at n = 64) |
| integrators/causal_majorant.rs:1–33 | path sum Σ H^k a; radius proposals | strictly lower nonnegative H | 1 preflight + 1 check | L-0036 PASS (residual-seeded radius closes in one check vs 4–8 attempts) |
| integrators/certified_budget.rs:1–15 | enclosure of the output budget | none | negligible | ARITH-04 closure (R3/R4 closure docs) |
| residual-to-output budget (research/rnext01, rev03) | \|err_X − err(X*)\| ≤ B from inexact stage residuals | interval Jacobian on boxes | 60-digit offline | L-0049 FAIL (Robertson h = 1e-2: B = 2e122); L-0062 PASS (68/68 fresh acceptances resolved; the 6 unresolved are safe-side rejections) |
| complex shift (PP16) / shared jet (PP04, PP03) | resolvent errors | (J+Jᵀ)/2 ≤ 0, H = I | jet wins counted flops in 49/90 configs (up to 12.6x), **none for all-equal shifts** | L-0070, L-0067, L-0068 PASS; L-0063 PASS (scoped); L-0069 FAIL (allocation) |
| Taylor total (PP11) | fused φ total | dense, ‖M‖ moderate | about 10x cost (L-0027: certified-arm warm wall 9.9x / 2.4x / 22.8x / 2.5x) | L-0078 PASS (bound 3.5–4.9e4x the error; Unbounded at w ~ 1e100) |

**Common structure.** Every certificate here is dense, a posteriori, and used only as an *admission filter*. It never feeds h, method or tolerance selection. INFERRED: none has a matrix-free form usable for n ≥ 100.

---

## 3. Homotopy, chart and stage-target candidates

- **Partial-coupling homotopy** (integrators/homotopy.rs):
  - `H(K, λ) = (block-diag W + η·coupling)K − base − λhR(K)` with η = θ + λ(1−θ) (homotopy.rs:23–54, :632–680). At λ = 0 the 8 RODAS5P stages decouple, all with the same W, so they can be solved in parallel. Continuation to λ = 1 recovers the sequential target.
  - Truncation depth q < 8 (:190–240); Euler or AB2 predictor.
  - Acceptance used a *linearized output correction*, explicitly **not a bound** (`is_error_bound() = false`, homotopy.rs:77–95). A counterexample reports 2.8e5 against a true error of 2.1e6.
  - Fallback is the full sequential step, with its work kept in the counters (homotopy.rs:1193–1350).
- **Transactional q1/q2** (transactional_q1_q2.rs):
  - Lanes are Q1Fast, Q2Escalated and SequentialFallback (:522).
  - q2 admission is either an operational diagnostic (an 8th W batch) or the native outward certificate on the sequential target. The certificate route saves the 8th batch: 7 batches instead of 8 (HOM-05).
  - The model must be the quadratic family, checked by sampled agreement (:47–120).
  - **HOM-06 (L-0008 FAIL, SPEEDUP_UNPROVEN)**, from VERIFY.json, certified arm vs sequential, per case:

    | Case | RHS | JVP | Notes |
    |---|---|---|---|
    | scalar-linear | 1745 vs 359 | 12,042 vs 1,539 | |
    | diagonal-quadratic-8 | 3600 vs 649 | 43,252 vs 7,196 | |
    | coupled-linear-6 | — | — | 41/41 fallback (no witness for n > 2 non-diagonal) |

    Accuracy matched. Speedup points: 0.17 at P=1 and 0.04–0.07 at P=2–8.
  - **L-0050 FAIL:** net margins are negative for n = 1..8; +0.65 solve units at n = 16 after post-hoc correction.
  - **Q2 decision** (docs/reviews/20261003_integrated_plan/Q2_ACTIVATION_DECISION.md): structured margin +0.63 solve units per attempt at n = 16 with 8 ideal workers and no dispatch cost. **Not activated.**
- **Raw U-form target** (integrators/raw_stage_target.rs:1–34):
  - Hairer–Wanner transformed stages U = ΓK with `r_U = γ r_K`.
  - L-0037 PASS: exact identity; discrepancy enclosures ≤ 1.4e-14.
  - L-0038 FAIL: **RHS-assembly JVPs drop 7 → 0 per attempt**, but GCRO-DR failures and Krylov noise break one-step agreement in 22/36 cases.
  - A related accounting issue: the K-target coefficient leakage of α (28 entries ≤ 5.6e-16) is recorded in integrators/stage_target.rs:1–27.
- **Stage chart candidate** (integrators/stage_chart_candidate.rs:1–14):
  - Newton on R(Ψ(Z)) with the static chain rule; the root is unchanged.
  - L-0055 FAIL: 3/8 cases fail to converge; certificates 3.5e5–5e5 WRMS; converged roots equal the sequential root to 1.4e-15. It buys nothing over direct stage solves.
- **Model-specific Darboux ratio chart** y = x²(w + 1/κ) (integrators/chart_transport.rs:1–14):
  - L-0043 PASS (SymPy identity; refusal conditions).
  - L-0048 PASS: 19,652 points enclosed; the controller accepts on the certified *local* bound. Without the cofactor forcing, 0/828 points are enclosed.
  - One model only.
- **Fourier client:**
  - L-0072 FAIL and L-0073 FAIL: alias controls were mis-designed; bounds still enclose everywhere.
  - L-0074 abstain: no shifted solves.
  - L-0079 FAIL: client 4 steps vs RODAS5P 11 / 72 / 6,315 attempts at ω = 1, 40, 1e4. No matched error was reached (RODAS5P 1.6x and 193x the client bound at ω = 40 and 1e4), and the work units are incomparable.

---

## 4. Polyalgorithm and regime machinery

- **Trigger (g4_s5b0_regime_atlas.rs:2030–2056):**
  - Feature: error curvature `−(log10 err_k − log10 err_{k−2})`, compared with `V25_ERROR_DROP_THRESHOLD = 0.01279` decades (:1988).
  - Fires on a `PersistenceLatch` of k = 3 (policy_redesign_v25.rs:58–98), i.e. three consecutive accepted R-JF steps with falling error.
  - Alternative features (JVP pressure, step contraction) are defined at policy_redesign_v25.rs:20–50.
- **Discriminator:** quadratic remainder drift `ζ34 = h·WRMS((D4/c4² − D3/c3²)/scale)` (exponential.rs:496–625), with τ = 13.397 frozen (atlas :433). This tests whether the nonlinear remainder is still about quadratic in c, i.e. whether the exponential method is in its asymptotic regime. The stage-growth features ρ2, ρ3, ρ4 and the slopes s23, s34 are at :3086–3105.
- **Budget:** prefix cap `min(80, ⌊0.25R⌋ − S)` (:3074); wall form `PrefixBudget` (policy_redesign_v25.rs:100–179).
- **Admitted switching:** none. `switching_active = false` in every shard; R-JF is always committed (v36 RESULT.md, v37 RESULT.md).
- **Common-W backend telemetry** (integrators/rhs_telemetry.rs:557–590): rank, energy rank and cosine of the stage RHS batch choose shared, block or independent GMRES. "Risk" comes from **declared** case parameters (`stiffness_proxy` and `nonnormality_proxy` literals in `build_cases`, :680–760), not from measurement. The common-W gate (integrators/common_w_gate.rs:685) is substrate only (research/audit2_matrix_free_common_w_20260830/CLAIM_LEDGER.md: no speedup, identity preconditioner, no recycle).
- **Unified gates and candidate catalog:** integrators/unified_gates.rs:18–23 (order floor 4.8, stiff floors). No recorded output is in the tree. integrators/candidates.rs:331–357 lists PeerW, ParallelSDC, Rosenbrock–Krylov, Borok, exponential-Leja, adaptive Radau and variable-order BDF as **Deferred**.

---

## 5. Outcomes by family

| Family | PASS | FAIL / INCONCLUSIVE | Quantitative reason it lost |
|---|---|---|---|
| Exponential / shadow | L-0002, L-0003, L-0040 | L-0001 INCONCLUSIVE | never activated; holdout has 1 positive (F-023); stiff order about 4; phi convergence is an estimate |
| Polynomial: Chebyshev | L-0009 (warm 3.17x vs Arnoldi, CI 1.17–13.41), L-0015/22, L-0016/23/27, L-0071 | — | gain only with frozen operator and cached coefficients; cold arm inconclusive; certified enclosure about 10x work |
| Polynomial: Laguerre | L-0039, L-0042, L-0047 | L-0013/L-0020 (majorant loose, median 157x, up to 3.5e65x), L-0014/L-0021 FAIL, L-0077 FAIL | continuous scale reduces degree at 0/22 points; never cheaper (ratio 1.0–1.57, routed 0/79); stiff totals ≥ 2.3e6 against budget 1e-6 because of the e^{L'/2} factor |
| Newton–Leja | L-0080 | — | estimate-only; products 0.83–2.0x Chebyshev (median 1.15), fewer only at ρ=200, h=0.2 |
| Taylor | L-0078 | — | bound 3.5–4.9e4x the error; Unbounded at large w |
| Nonnormal / log-norm | L-0056 | L-0060, L-0075, L-0076 FAIL | no decay with Gershgorin; strongly nonnormal 1e9–1e19; works only with a symmetrizable tridiagonal structure |
| Shifts | L-0063, L-0067, L-0068, L-0070, L-0074 (abstain) | L-0069 | jet useless for a single common γ, which is RODAS5P's case |
| Fourier | — | L-0072, L-0073, L-0079 | narrow model; error not matched |
| Homotopy / transactional | L-0034, L-0035, L-0036 | L-0008, L-0017/L-0024, L-0050, L-0053 | 5x RHS, 6–8x JVP; margin < 1 solve unit at n = 16 |
| Charts / stage target | L-0037, L-0043, L-0048 | L-0038, L-0055 | chart Newton non-convergence; U-form blocked by Krylov issues |
| Directed arithmetic | L-0065 | L-0064 | real local defects, repaired |

RODAS5P as the anchor:
- L-0051: perturbation error falls from 1.1e-4 to 3.9e-11 as K goes from 1e2 to 1e12. The binary64 tableau has |R(iy)| = 1 + 1.3e-19.
- L-0040: effectivity 4.6–11.
- L-0028/L-0029/L-0032: cost per step and Robertson against Radau are the remaining losses, not robustness.

---

## 6. Mathematical levers

### Still open, given this evidence

1. **Use certificates and spectral quantities to *drive* control, not only to verify.**
   - No controller reads them today (search: no spectral, log-norm or Ritz use in any controller; adaptive.rs offers only Integral and PI).
   - Candidates (INFERRED):
     - a verified μ[hJ] (L-0075/L-0076 show it is computable and tight for symmetrizable structure) to bound ‖W⁻¹‖ ≤ 1/(1 − hγμ);
     - feed that bound into the residual-to-output budget (L-0062) and set the Krylov forcing per stage (looser solves where the bound allows);
     - use `stepping_rule`-type τR estimates to size phi substeps and Krylov dimension a priori, instead of the doubling loop (exponential.rs:1647).
   - Barrier: the certificate cost is dense O(n³) or O(N m n²); a matrix-free or banded variant does not exist.
2. **Regime-switched exponential steps with real, larger steps.**
   - At 64 recommended events a PEXPRB54S4 step cost 0.10–0.38x the JVPs, and its self-estimated error was similar in 4 of 6 families (worse on HIRES and Robertson).
   - Untested: (a) E at its own, larger h; (b) reference-anchored safety on a fresh holdout with at least 5 positives; (c) a spectral or ζ-based trigger instead of the calibrated error-curvature latch.
   - Order reduction (about 4) and the autonomous-only limit must be priced in.
3. **Frozen-operator reuse across steps.**
   - L-0009 (warm Chebyshev 3.17x) and L-0023/L-0027 (a cached eigensystem beats Chebyshev from the 3rd action) point to exponential-W or EPIRK-type methods with a lagged J. These are analogues of RODAS5P Jacobian reuse (INFERRED).
   - Valid only on a verified symmetric-dissipative domain.
4. **JVP removal via the U-form target** (L-0037 PASS; L-0038 shows 7 → 0 JVPs per attempt). This is an algebraic saving. Its blockers are Krylov-side (GCRO-DR), not mathematical; it belongs to the R2 scope.
5. **Oscillatory carriers.** At ω = 1e4 RODAS5P needs 6,315 attempts, while a carrier-aware path needs 4 steps (L-0079, unmatched accuracy, one model). A general closure (PP14) and complex-domain polynomial or rational actions are unbuilt. The complex-shift certificate (L-0070) is the existing building block for rational/partial-fraction φ via RODAS5P-style shifted solves.

### Effectively closed (at current sizes and evidence)

- Homotopy/transactional parallel stages as a speed lever (L-0008, L-0050, Q2 decision); doubling certificates (L-0017/L-0024).
- Laguerre basis in any form (L-0014/L-0021, L-0071, L-0077).
- Leja as a cheaper certified alternative (L-0080).
- Generic stage charts (L-0055).
- Shared-shift jets for single-γ RODAS5P (L-0067: none for all-equal shifts; L-0074).
- Gershgorin or Crouzeix–Palencia certification of strongly nonnormal or Jordan operators (L-0060: 1e9–1e19).
- Fourier client generalization without a new closure (L-0072/L-0073/L-0079).

### Not yet supported by this evidence

- Any speed claim: timing authority is HOLD (L-0007/L-0010).
- Any claim that the shadow recommendations are safe: L-0001 is inconclusive and its labels are self-estimates.

No files were written; the scratch directory `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/` was created and left empty.