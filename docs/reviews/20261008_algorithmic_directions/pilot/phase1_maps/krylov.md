# R2 map: matrix-free linear algebra and inner–outer coupling (`/home/user/wt-speed`, HEAD a49f7e4)

This map was read-only. Nothing in the worktree was changed. Three throw-away probe scripts are in `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/` (`bruss_gmres_probe.py`, `bruss_pc_probe.py`, `bruss_lognorm.py`, `bruss_deflation.py`). Their results are marked **PROBE (INFERRED)** below: they use a frozen W at y0, run in numpy, and have no ledger entry.

## 0. Headline facts

1. **The production GMRES never stops inside a restart cycle.** Every cycle builds exactly `min(restart, n, budget left)` columns unless it hits a happy breakdown (`gmres.rs:110-186`, `gmres_into.rs:237-262`). Convergence is checked only between cycles, on the true residual (`gmres.rs:350-366`). GCRO-DR (`gcrodr.rs:989-1072`) and LGMRES behave the same way.
   - On 320 frozen Brusselator-50 stage solves, cold GMRES used 40/40 columns on every solve that iterated. It finished 10^0.1 to 10^15 below the threshold, typically 4–5 decades below (L-0046 data, `rnext03…/RESULTS.json`).
2. **The strict matrix-free paths use no preconditioner.**
3. **The fast U-form matrix-free driver has no forcing term.** It applies a fixed relative tolerance (1e-10 or 1e-11) to the unweighted ‖rhs_U‖₂.
4. **The 344 LGMRES failures in L-0061 were induced on purpose.** They all come from the "failure paths" set (`max_outer = 1`, rtol 1e-14). The natural sequences had 0 failures.
5. **On these Brusselator systems, recycling pays nothing once it is made correct.** The recorded GCRO-DR failure mechanism is a broken invariant `M⁻¹AU = C` that is reused unchecked. Refreshing C fixes it (L-0059) at 1.08–1.32x the work of cold GCRO-DR.
6. **For n ≤ 8, every stage solve fills the whole Krylov space.** It costs n columns plus 2–3 true-residual matvecs, so about 8(n+2..3) JVPs per attempt.

## 1. Krylov kernels (`crates/rodas5p-krylov`)

**Common contract** (`common.rs`, `core/operator.rs:352-367`)
- Preconditioning is left only (`apply_left_with_raw` = pc ∘ op, `common.rs:93-109`). The trait says implementations are not assumed nonsingular.
- Every kernel certifies on the unpreconditioned true residual.
- Threshold = `max(atol, rtol·‖b‖)` (`common.rs:40-48`). It is measured in L2, or in WRMS when a `residual_scale` is passed (`common.rs:50-55`). It is relative to ‖b‖, not ‖r₀‖.
- Tolerances are validated (F-034, `common.rs:30-38`).

| Kernel | Orthogonalization | Restart / budget defaults | Small problem | Stop test | Breakdown handling |
|---|---|---|---|---|---|
| GMRES `gmres.rs:327-430`; `gmres_into.rs` is bitwise the same | Two-pass MGS (`kernels.rs:51-80`) | restart 40, max_arnoldi 200, rtol 1e-11, atol 1e-13 (`gmres.rs:17-34`). Drivers pass `max(maxiter, restart)` | faer column-pivoted QR LS (`small.rs:4-15`): once per column in the legacy path, once per cycle in `_into`/`ls_once` | True residual between cycles only; final diagnostic true residual (`gmres.rs:407-422`) | Happy breakdown if `h_next ≤ 100ε·‖[h_col; h_next]‖`, scale-invariant (`gmres.rs:60-79`); the cycle ends and the solver restarts. `beta ≤ MIN_POSITIVE` is an error. Budget exhaustion returns `LinearSolve`, which the driver turns into a rejected step (`rodas5p_matrix_free_fast.rs:827-860`) |
| GMRES-Givens `gmres_givens.rs` (research only, never wired; header lines 1-17) | MGS×2 | as GMRES | Incremental Givens QR | Projected residual ≤ `threshold·β/‖r_true‖` (`:376-382`) triggers a true-residual check; after a rejected check the gap between checks doubles (`:432-453`) | Rejected happy breakdown fails closed; triangular diagonal ≤ 100ε·max fails closed (`:232`) |
| `GmresPrefixSession` / `solve_gmres_incremental` `gmres.rs:546-800` (G4 gate only) | MGS×2 | restart-free | LS every column | Small residual checked every column, then true residual (`:727`) | Geometric prediction from the last 2 ratios (`:505-540`) |
| GCRO-DR `gcrodr.rs:717-1240` | Arnoldi vector projected once against C (twice with REV-01b), then MGS×2 against V | restart 40, recycle k = 8 (Arnoldi columns per cycle = 40 − k), rank_tol 1e-12 floored at √ε (`:141`) | LS on the augmented relation G, size (k+p+1)×(k+p) (`:1092`) | True residual after projection and after each cycle (Diagnostic), plus loop-top and final checks | Happy breakdown measured against the full projection norm including the C coupling (`:1046-1051`). Optional stagnation reset (`:1171`) |
| LGMRES `lgmres.rs` | MGS×2 in the shared augmented Arnoldi (`gmres.rs:84-187`) | inner_m 30, outer_k 8, max_outer 20. **Both drivers pass max_outer = maxiter = 200 cycles** (`sequential.rs` LGMRES branch, `rodas5p_matrix_free_fast.rs:536-546`), so the budget is about 6000 matvecs against GMRES's 200 | LS per column (legacy) | True residual per outer cycle; failure after max_outer | Residual-breakdown error |
| Block GMRES / seeded GMRES `block_gmres.rs:278, 468` | Two-pass, rank-deflated | max_basis 80; seeded: shared_basis 12, then per-RHS GMRES refinement | faer LS | True residual per RHS | Deflation at rank_tol. **Only `common_w_gate.rs:404-419` (synthetic) calls these, never a RODAS5P stage path** |

**GCRO-DR recycle mathematics**
- Augmented relation `A[U V_p] = [C V_{p+1}] G` (`augmented_relation` `:337-385`). U columns are normalized with `1/‖u‖` on the diagonal of G.
- Update: harmonic Ritz vectors solving `GᵀG z = θ GᵀŴᵀV̂ z`, keeping the **smallest |θ|** (`real_harmonic_subspace` `:223-296`, sort at `:242`).
- `orthonormalize_pair` (`:99-187`): column-pivoted QR of BY gives C = Q and U = Y R₁₁⁻¹. The rank floor is `max(rank_tol, √ε)·|R₀₀|` together with an absolute floor of `100ε·max col‖G‖` (`:326`).
- Reuse for "same system" (exact identity of operator and preconditioner) happens without a check (`:761-819`).
- Cross-operator reuse refreshes the images with k charged matvecs and re-orthonormalizes (`:821-846`).
- Options: `verify_reuse` (`:775-818`), `orthogonalize_start` (`:976`), `reorthogonalize_recycle` (`:1028`), `refresh_after_update` (`:1141`).
- The driver offers three policies: `GcrodrRecyclePolicy::{Legacy (default), RefreshAfterUpdate, Cold}` (`rodas5p_matrix_free_fast.rs:58-127`).

**LGMRES augmentation**
- Error-approximation directions z (normalized corrections) are carried across solves (`lgmres.rs:155-170`).
- The image of each new z is computed by an explicit charged matvec (`:156-164`), so `Az = image` holds to roundoff for each vector and nothing drifts.
- Cached images are reused only for the same system identity. They are cleared on a system change and recomputed (k Refresh matvecs, `:129-141`).
- Augmentation columns with cached images cost no matvec (`gmres.rs:118-131`).

## 2. Preconditioning

There is none in any matrix-free path.
- `PreconditionerKind` has only `None | Jacobi | Direct` (`solver_types.rs:11-15`).
- `make_pc` (`sequential.rs:265-287`) builds Jacobi or Direct only from an explicit W.
- The strict MF U-form driver refuses both (`rodas5p_matrix_free_fast.rs:188-213`) and hard-wires `IdentityPreconditioner` (`:250`).
- The sequential MF steps refuse Direct (`sequential.rs:834, 888`). Jacobi needs `shifted.explicit()`, which MF contexts do not have.
- The only other implementations are research and synthetic code: `AnalyticDiagonalPreconditioner` in `common_w_gate.rs:215-246`, `Audit2VerifiedDiagonalPreconditioner` in `audit2_reusable_transaction_research.rs:147-175`, a Bateman analytic diagonal, block Identity/Direct/Jacobi in `block.rs:666-735`, and test doubles.
- A full search found no ILU, right preconditioning, lagged-Jacobian preconditioner, multigrid, ADI or operator-split preconditioner, polynomial preconditioner, or sparsity coloring.
- A stated repo principle supports adding one: "keep the current JVP target; reuse only the basis or preconditioner" and "fast geometric inverse → structural preconditioner plus a full current-J residual check" (`docs/reviews/thread_transfer_20261002/REVIEW_KO.md:357-366`).

## 3. Inner tolerance and forcing: how linear accuracy is tied to the outer tolerance

**A. Fast U-form MF driver.** Stage equation, `M = I` (`rodas5p_matrix_free_fast.rs:12-15`):

`(I − hγJ)U_i = hγ f(t+c_i h, y + Σ A_ij U_j) + γ Σ C_ij U_j + h²γγ_i f_t`

- The linear tolerance is a fixed `config.rtol` on ‖rhs_U‖₂, with no WRMS. `atol` becomes `|γ|·atol` (`:411-416, 475-480`; `raw_stage_target.rs:292-304`).
- There is no forcing (`:35`). The embedded error is `U_{s−1}` (`:597`).
- The research runs used linear rtol 1e-10 with atol 1e-14 (SPD07) and 1e-11 (L-0038, L-0046), against outer rtol 1e-6 or 1e-8.
- RODAS5P: γ = 0.21194 and Σ|b_code| = 24.9 (`fixtures/rodas5p_coefficients_snapshot.json`). Computed for this report: ‖(I−γC)⁻¹‖_∞ = 63.6, abs-sum of `b_codeᵀ(I−γC)⁻¹` = 23.3, of `e_sᵀ(I−γC)⁻¹` = 6.7 (non-stiff limit). |C_ij| reaches 165. **INFERRED:** ‖rhs_U‖ can greatly exceed ‖U_i‖, which loosens a relative criterion. This is the U-form analogue of F-031.

**B. Sequential K-form, unprotected.** Uses `config.rtol` in L2. The rhs contains `h J Σγ_ij K_j`, at one extra JVP per stage, 7 per attempt (`sequential.rs:405-418`). F-031 / E-04 measured `rhs_wrms/flow_wrms = 40` and `rhs_wrms` up to 5e11, so a relative criterion becomes an absolute stage error of about `η·|hλ|·|K|`.

**C. Protected K-form with WRMS forcing (WU-3, `4d3a7c9`; RA-05, `6197a84`)**
- Scale: `error_scale(y, Y_i, atol, rtol)`; `flow = |h|·wrms(f_i)`; `rhs_wrms` (`sequential.rs:421-423`).
- `η = clamp(0.1/(‖b‖₁·max(flow, rhs)), 64ε, 0.5)` and `τ = max(η·rhs_wrms, 64ε)`. A floor flag replaces the old step-0 abort (`g4_s5b0_inner_tolerance.rs:75-114`). GMRES receives rtol = η and atol = 64ε, with the WRMS residual scale.
- The cap 0.1/‖b‖₁ = 0.02022 implies ‖b‖₁ ≈ 4.95.
- Refinement (`sequential.rs:677-725`): after each pass, compute the embedded err and the limit `0.02022·min(1, err)^{6/5}` (`:54-73`). The exponent (p+1)/(q+1) is meant to make residuals scale like h⁶. If any stage residual exceeds `max(limit, floor)`, all 8 stages are re-solved from the previous pass. At most 3 passes; each result is labelled `SelfConsistent` or `Underresolved`.
- Declared claim scope: `StageResidualHeuristicRequiresResolventBound` (`:13-29`). A residual-to-error map needs ‖W⁻¹‖, which the matrix-free interface does not have.
- Measured:
  - E-04 (pre-fix) found an h-independent error floor of 0.1–0.3·rtol (P1PR) and 0.01–0.8·rtol (P2), and step-0 aborts at rtol ≤ 1e-8.
  - Post-fix ladder contracts hold the inexact error within 3x of direct LU on halving ladders (`tests/inner_forcing_fixed_step_ladder_contracts.rs:164-193`).
  - RA-05: 1512/1512 steps self-consistent (semilinear n = 64, fixed h).
  - A refinement pass costs JVPs: the shadow target went from 106 to 141 (+33%, WU-3 commit).
  - L-0038: protected versus sequential JVP per attempt is +1% (van der Pol), +1% (HIRES), +4% (Brusselator), +52% (Prothero-Robinson), +57% (quadratic-4). **INFERRED** from diagnostic matvecs per attempt of 8.7–14.8: about 1.1–1.85 passes per attempt.
  - Direct LU itself shows order about 4 on Prothero-Robinson (E-04), so not all order loss comes from the linear solve.

**D. G4/S5B0 lane policy.** The committed arm is `LegacyFixed`: linear rtol 1e-10, atol 1e-12, restart 32, maxiter 256 (`g4_s5b0_inner_tolerance.rs:10-11, 179, 285-294`). The outer-scaled arm is pending replay.

**E. Stage certificate.** `audit2_stage_certificate_research.rs:288-400` (synthetic, explicit W).
- Per stage: `q_i = ‖x_i‖ + κ·‖r_i‖`, with κ verified ≥ ‖W⁻¹‖₂ through an explicit inverse witness.
- Propagation through `strict_lower` (forward majorant), then endpoint and estimator weights give θ.
- Decision: accept if `ê + θ ≤ 1`, reject if `ê − θ > 1`, otherwise inconclusive.
- It is not usable matrix-free because it needs κ.
- PROBE (INFERRED): for the Brusselator, ‖W⁻¹‖₂ is 1.015, 1.16 and 1.59 at h = 0.01, 0.1 and 0.3. The log-norm bound `1/(1 − hγμ₂(J))` with μ₂ = 6.97 gives 1.015, 1.17 and 1.80. A verified upper bound on μ exists only for explicit matrices (`nonnormal_certificate.rs` `symmetric_part_upper`, PP12 / L-0075/76).

## 4. Initial guesses

- **K-form:** `Previous` gives `x0 = K_{i−1}` (`sequential.rs:454-466`). Refinement passes start from the previous pass's stages. Step-indexed starts are refused (`:350-355`).
- **U-form:** `Previous` gives `U_{i−1}`; `PreviousStep` and `PreviousStepScaled` give `u_i` of the last accepted step, the scaled one times h/h_prev (`rodas5p_matrix_free_fast.rs:460-506`).
- **LGMRES and GCRO-DR with `x0 = None`** fall back to `state.previous_solution` only for the same system. A new attempt is a new W, so it is dropped.
- **A nonzero x0 costs one true-residual matvec.** It saves work only if it saves whole cycles, because the threshold is relative to ‖b‖ and cycles are full.
- **L-0084 (SPD07; GMRES-into, restart 40, linear rtol 1e-10, 7 problems at rtol 1e-6 and 1e-8):**
  - PreviousStep / Previous linear matvecs = 0.963–1.001 on the Brusselators (gate ≤ 0.85) and 1.03–1.05 on Robertson and van der Pol.
  - Brusselator-160 at 1e-8: iterations per solve fall from 50.6 to 48.5.
  - **Previous / Zero = 1.02–1.57 in 14/14 cases, so Zero is cheapest everywhere:** Brusselators 1.024–1.066, HIRES 1.11, Robertson, van der Pol and quadratic-4 1.22–1.29, Prothero-Robinson 1.51–1.57.
  - Per-stage ‖r₀‖/‖b‖ was not recorded.
  - Earlier oracle-seeded warm starts gave 0.905x and 0.94x (F-013, a different setting).

## 5. Recorded failure modes

**Recycled GCRO-DR** (L-0038 → L-0046 → L-0052 → L-0057 → L-0058 → L-0059 → L-0066)
- *L-0038:* unpreconditioned recycled GCRO-DR never completes the Brusselator in any driver ("Arnoldi budget exhausted"; NaN least squares at rtol 1e-12).
- *L-0046:* 21/336 frozen solves fail (13 trajectory, 8 one-step). Cold GMRES and cold GCRO-DR fail on none. All failures carry R2 (recycle-induced), R3 (residual gap up to 1.4e22) and R4 (`max|CᵀV|` up to 0.9999997 while CᵀC and VᵀV stay orthonormal to 1e-15); 7 carry R5 (NaN least squares). The true residual grows within a cycle, for example 6.5e-4 → 6.4e13 → 4e31.
  - Post-hoc: some recycle updates return pairs that break `AU = C` (3.4e-5 → 2.59). With the operator unchanged, they are reused unchecked.
  - A stagnation reset (q = 0.5) gives 0 failures.
- *L-0052:* `verify_reuse = 1e-8` removes 19/21. Two remain at a defect of 2.2e-9: projection removes almost all of r, so a defect δ enters as `δ·‖Cᵀr‖`, which is comparable to the projected residual. An absolute tolerance is the wrong scale; the defect must be small relative to `‖r − CCᵀr‖/‖Cᵀr‖`. Post-hoc tolerances of 1e-10 and 1e-12 remove all failures (chosen after seeing them).
- *L-0057:* start projection alone gives 56 failures; `CᵀV` reaches 0.71–0.999 during Arnoldi.
- *L-0058:* adding reorthogonalization gives 28 failures on Brusselator-120 (plain recycling: 12), so H2 is refuted. A near-breakdown explanation is not supported either: min ‖next‖/scale = 3.9e-7.
- *L-0059 (PASS):* refreshing `C = M⁻¹AU` after every update removes all 16 (primary) and 69 (seen) failures. A large CᵀV persists without failures, so it was not the cause. Cost: 1.08–1.32x cold GCRO-DR on the Brusselators, 0.98x on CDR (unrepaired: 0.80x).
- *L-0066:* in the driver, linear-solve failures drop from 2453/2475 per 5000 attempts (Brusselator-50, attempt cap hit, about 1.36M operator applications) to 0 with refresh or cold. Cold is 4–23% cheaper than refresh on the Brusselators. Refresh is far cheaper on small n (HIRES at 1e-8: 23,522 against 61,896 for cold).
- **INFERRED amplification path:** U = Y·R₁₁⁻¹ with diagonal entries allowed down to √ε·|R₀₀| can amplify the relation defect by up to about 1/√ε ≈ 6.7e7.

**LGMRES.** L-0061's 344/1,216 failures are 6×48 CDR plus 56 Brusselator solves from set 3 (`max_outer = 1`, rtol 1e-14), all final-residual failures, all rolled back. Natural sequences had 0 failures, and LGMRES completed every L-0038 run.

**GMRES stagnation without preconditioning**
- R-NEXT-03 held-out CDR (n = 200, I − τ(D2 − Pe·D1)): every control including cold GMRES fails 256/384 at rtol 1e-11 within 200 products (all τ = 1e-2 and all Pe = 100). The successes use 206 products, right at the budget.
- E-05: A = Q(D+N)Qᵀ, n = 256, D down to −1e4. At s = 0 GMRES needs 2120–2280 iterations; at s = 10 and 100 every solver fails at 4000. Jacobi is no help.
- E-04 P1 (λ to −1e6, n = 256, restart 32): about 1575 GMRES iterations per attempt (≈197 per stage) at rtol 1e-6, and 3 linear-solve failures at rtol 1e-10.

**Nonnormal W.** Only E-05 is a solve-level stress test; INT-05, REV-02 and PP12 are exponential bounds. PROBE: the Brusselator W is nearly normal (Henrici departure 3e-2 at N = 50, 4e-3 at N = 160), so cond(W) is what limits convergence.

## 6. Quantities (JVPs; every MF JVP is a W application)

**SPD07, U-form driver, GMRES-into, Zero / Previous start** (per stage = linear + diagnostic over solves):

| Problem (n), outer rtol | Attempts / accepted | JVP per stage | JVP per attempt | JVP per accepted step |
|---|---|---|---|---|
| Robertson (3), 1e-6 | 45 / 43 | 4.24 / 5.05 | 34.0 / 40.4 | 35.5 / 42.3 |
| van der Pol (2), 1e-6 | 469 / 351 (25% rejected) | 4.00 / 4.87 | 32.0 / 39.0 | 42.7 / 52.1 |
| HIRES (8), 1e-6 | 210 / 204 | 8.95 / 9.85 | 71.6 / 78.8 | 73.7 / 81.1 |
| Brusselator-50 (100), 1e-6 | 92 / 82 | 41.7 / 43.3 | 333 / 346 | 374 / 388 |
| Brusselator-50 (100), 1e-8 | 193 / 189 | 41.8 | 334 | 342 |
| Brusselator-160 (320), 1e-6 | 92 / 82 | 77.3 / 82.4 | 618 / 659 | 694 / 739 |
| Brusselator-160 (320), 1e-8 | 193 / 190 | 50.7 / 53.7 | 405 / 430 | 412 / 437 |

- For n ≤ 8, iterations per solve equal n (2.00 for van der Pol, 6.97 for HIRES). With x0 = 0, each solve also pays one loop-top Krylov residual plus one final diagnostic residual on the same x. **INFERRED:** that last matvec is redundant (`gmres.rs:350-362` against `:407-416`). For van der Pol, half of all JVPs are residual checks.
- **L-0038, outer rtol 1e-6, linear rtol 1e-11, JVP per attempt (sequential K / protected / U-form):**
  - GMRES: Robertson 47.1 / 52.9 / 40.4; van der Pol 46.0 / 47.4 / 39.1; HIRES 85.3 / 86.5 / 78.8; Brusselator-50 345 / 358 / 351.
  - LGMRES on Brusselator-50: 289 / 293 / 308, which is **0.84x / 0.88x of GMRES at equal error**.
  - Recycled GCRO-DR on small n: 25–38 JVP per attempt with 0.0–0.3 iterations, because the recycle space spans Rⁿ.
- **R-NEXT-02 driver:** linear iterations per attempt 327.8 (Brusselator-50), 55.8 (HIRES), 17.4 (Robertson), 16.1 (van der Pol).
- **INT-01 CDR fresh family (n = 120, rtol 1e-9):** products per solve 164 (cold GMRES), 144 (cold GCRO-DR), 116 (recycled), 123 (verified), 141 (refreshed).
- **RA-06 (L-0005):** predictor plus two common-W corrections costs 1.8–2.1x the W applications and 2x the W solves, so it was rejected. A small correction RHS costs a full solve under a relative criterion.

**PROBE (INFERRED), Brusselator W at y0, full GMRES, rhs = hγf₀**

| Case | Columns to 1e-10 | Columns to 1e-6 |
|---|---|---|
| N=50, h = 1e-4 / 1e-2 / 0.05 / 0.1 / 0.3 | 3 / 8 / 16 / 23 / 41 | – |
| N=160, h = 0.05 / 0.1 / 0.3 | 45 / 66 / 122 | 23 / 35 / 68 |

- Production spends 40 columns per cycle regardless of need.
- Jacobi and pointwise 2×2-block preconditioners give equal or worse counts.
- Inverting the diffusion part alone (tridiagonal per species) needs 6–9 columns, independent of h and N.
- An exact W built from J at a state shifted by 10% needs 5–7 columns. W at 0.7h needs 7–10.
- Deflating the 8 smallest |λ(W)| shrinks the eigenvalue ratio by only 1.1–1.4x. The 8 largest give about 1.0x.

## 7. Mathematical levers

1. **Stop each cycle on the projected residual, then certify once on the true residual.**
   - Where: `gmres.rs:110-186`, `gmres_into.rs:237-262`, `gcrodr.rs:1006-1072`. Prototypes exist in `gmres_givens.rs:376-453` and `GmresPrefixSession` (`gmres.rs:681-730`).
   - Evidence: 40/40 columns used with 4–15 decades of overshoot (L-0046 data). PROBE: 16–25 columns suffice at typical h on Brusselator-50. RA-06 found small-RHS solves cost full cycles. There is no integrator-level measurement yet; the F-036 note says Givens matvecs are not comparable.
   - Expected effect (INFERRED): roughly 40–60% fewer Krylov JVPs on Brusselator-50, none for n ≤ 8.
   - Risk: certification is unchanged. Left-preconditioned scaling must use `β/‖r_true‖`.
   - **This also unblocks lever 2:** looser forcing saves nothing while cycles are full.
2. **A forcing term tied to the local error, with a resolvent bound.**
   - Where: the U-form fixed rtol (`rodas5p_matrix_free_fast.rs:475-480`) and the K-form heuristic (`g4_s5b0_inner_tolerance.rs:54-114`, `sequential.rs:677-725`).
   - Theory available in the repo: the stage-certificate algebra (`audit2_stage_certificate_research.rs:340-386`) needs only κ ≥ ‖W⁻¹‖. In a dissipative metric, `‖W⁻¹‖_D ≤ 1/(1 − hγμ_D)`. PROBE: ≤ 1.8 on the Brusselator.
   - U-form weights: b_code abs-sum 23.3, embedded 6.7.
   - PROBE: 1e-6 instead of 1e-10 cuts columns by 1.7–1.9x.
   - Risk: E-04's floors and order collapse under h-independent budgets, and F-033 (the embedded estimator cannot see solve error).
3. **A preconditioner built from available structure, with the exact current-J operator kept.**
   - Where: the PreconditionerKind gate (`rodas5p_matrix_free_fast.rs:188-213`; `sequential.rs:265-287`).
   - Options:
     - a lagged W factorization (J assembled every k steps by JVPs or coloring);
     - an operator-split, physics-based inverse such as the diffusion part;
     - for declared bands, coloring with about 5 JVPs for the 1-D Brusselator, then reuse the banded LU of INT-03 / L-0054 / L-0083.
   - Evidence: PROBE gives 5–9 columns, h- and N-robust, against 16–122. Unpreconditioned stagnation is recorded in the R-NEXT-03 CDR held-out set, E-05 and E-04 P1. The repo's own rule allows it ("keep the JVP target; reuse the preconditioner"). Jacobi is useless here (probe, E-05).
   - Robustness: the true-residual certificate is unchanged, so RODAS5P order is kept.
4. **Small n: assemble J once per step instead of solving 8 full-dimension Krylov problems.**
   - Cost today: 8(n+2..3) JVPs per attempt (34, 32 and 72 for Robertson, van der Pol and HIRES). J by n JVPs plus one LU would cost n (3, 2, 8).
   - Recycled GCRO-DR already approximates this at 25–30 JVPs per attempt (L-0038, L-0066).
   - INFERRED. The small-n direct drivers (L-0041, L-0085) already exist for explicit J.
5. **Share Krylov information across the 8 stages that use the same W.**
   - Evidence: LGMRES's exact-image augmentation gives 0.84–0.88x the JVPs of GMRES with 0 natural failures (L-0038). Image-drifting recycling (GCRO-DR legacy) is unsafe. Refreshed recycling is safe but pays nothing on the Brusselator (L-0059, L-0066). The **INFERRED** reason: harmonic Ritz targets the smallest |θ| ≈ 1 of a dissipative W, and deflating them changes the eigenvalue ratio by only 1.1–1.4x (PROBE).
   - **INFERRED, untested:** each certified stage solve already computes its image, `W·U_j = b_j − r_j`, in the final true-residual matvec. Stages 1…i−1 can therefore be used as a free augmentation space with exact images (minimal-residual projection onto previous solutions, Fischer-type).
   - SPD07 shows that using one such vector only as x0 does not help (≤ 4%).
   - Block and seeded GMRES (`block_gmres.rs:278, 468`) cannot apply directly, because the stage RHS depend sequentially and nonlinearly on earlier stages. RA-06 failed for the same reason (L-0005).
6. **A Krylov-projected Jacobian (Rosenbrock-Krylov / ROWMAP type).**
   - This would be a new method identity: K-method order conditions that RODAS5P does not satisfy. The registry lists it as deferred (`candidates.rs:347-351`). The review warns against treating an approximate W as safe (`REVIEW_KO.md:357-360`). F-027 notes the missing W-method / ROK literature.
   - Potential: about m JVPs per step for all stages. Risk: stiff order reduction. No in-repo evidence.
7. **Recycling geometry.** If recycling is kept, use LGMRES-style recomputed images, or `verify_reuse` with a residual-relative tolerance, or always refresh (L-0052, L-0059). Select deflation targets by spectral relevance; for a dissipative W, the smallest |θ| is the wrong end (PROBE).
8. **Failure-to-rejection economics.** A linear failure spends the whole budget before the driver cuts h (`rodas5p_matrix_free_fast.rs:838-860`): about 1.36M applications under Legacy against about 33k with refresh (L-0066). The prefix contraction predictor (`gmres.rs:505-540`) could end hopeless solves early. It has no integrator evidence.
   - Separately, the budgets are asymmetric: LGMRES gets about 6000 matvecs, GMRES and GCRO-DR 200. This affects robustness comparisons.