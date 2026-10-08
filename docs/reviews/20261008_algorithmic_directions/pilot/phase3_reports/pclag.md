**Probe B4 (PC-LAGGED-HWINDOW on 2-D grids): HOLD.** The candidate as specified, arm (c), is never the cheapest arm in any cell. The linear-part variant, arm (d), is the real niche and is worth a preregistered node. These are exploratory replica pilots for future preregistered nodes, not ledger authority. No Rust was built and the worktree was not touched.

## Question
Does a declared-structure preconditioner beat sparse direct factorization of W (arm a) and unpreconditioned matrix-free GMRES (arm b) in total admitted flops at matched accuracy on 2-D grids? Two preconditioners were tested: the lagged full-Jacobian one, (c) PC-LAGGED-HWINDOW, and a linear-part-only one, (d). If either wins, where, by how much as N grows, and is it robust?

## Method
- **Driver.** Closed-loop adaptive RODAS5P in the U form, I controller (from `probe/target/rep.py`). Stage right-hand sides come from the inexact solves. Linear failures feed back as a rejection with h×0.2.
- **Stage target.** Every Krylov arm uses probe A1's coupled WRMS absolute target (`coupled_target.py`: Theta 0.2, error-per-unit-step, U8 cap, RHO_FL guard, RHO_STALL stall rule). GMRES runs on D W D⁻¹ with restart 40 and maxit 2000 (the judge's strongest cheap setting), with an in-cycle projected stop. Every exit is confirmed by one true residual of b − W x on the exact, unpreconditioned operator.
- **Problems** (N×N grids, N ∈ {32, 64, 128}, plus N = 256 beyond the ask):
  - `bruss2d-N`: a 2-D extension of the harness Brusselator (Dirichlet u=1, v=3; α=0.02; n = 2N², up to 131072). It is my own construction and off-contract; the corpus brusselator-2d holdout was not used.
  - `bruss2d-N-a0.1`: the stiffer α=0.1 variant.
  - `semilin2d-N`: the corpus v2 semilinear advection-diffusion family with upwinding (nonnormal), exact solution, atol = 0.01·rtol, n = N².
  - `semilin2d-32x48`: the on-contract corpus n = 1536 grid (the judge's probe (i)).
  - `semilin2d-64-s4`: a badly scaled cell, atol = 1e-4·rtol.
- **Rtol ladder.** 1e-3 to 1e-10 for N ≤ 128 (tight cells 1e-9 and 1e-10 included); 1e-4 to 1e-8 for N = 256 and the α=0.1 variant. HIRES and Robertson (n ≤ 8) are outside the preconditioner question and were not rerun.
- **References.** semilin2d uses its exact solution. bruss2d uses the direct arm at rtol = atol = 1e-13 (N ≤ 128; 1e-12 for N=256 and α=0.1 at N=128).
  - The 1e-12 and 1e-13 references differ by 8.6–8.8e-13.
  - An independent check at N=32 against SciPy Radau (rtol 1e-12) differs from the 1e-13 reference by 1.1e-13.
- **Cost model (all counted).**
  - Krylov JVP: 2·nnzJ + 4n flops, i.e. 15.9n for Brusselator and 13.9n for semilinear. nnzJ/n is 5.97 and 4.97.
  - Coloured JVP for building J: 2·nnzJ. Greedy Curtis-Powell-Reid colouring needs 8 colours for Brusselator and 5 for semilinear.
  - A finite-difference JVP model (F_rhs + 4n) gives the same Brusselator ratios.
  - Orthogonalization (MGS2-equivalent counts): each dot product, axpy and norm costs 2n; scaling costs n.
  - RHS: 23N² flops (Brusselator); 28n plus 38n for f_t (semilinear).
  - Stage assembly is counted. W/P assembly costs nnzJ + n.
  - LU flops = Σ_k (l_k + 2·l_k·u_k) from the actual SuperLU factors (formula checked against a dense LU). Each solve costs 2·nnz(L+U) + n.
  - Ordering is MMD_AT_PLUS_A in symmetric mode, the best available. My geometric nested dissection was 30–50% worse in fill at every N.
- **Scoring.** Both the log-log regression frontier (x marks extrapolation) and the harness cheapest-run-reaching-E rule.
- **Second cost model (T, wall time).** Single-thread kernel times, minimum of two runs, with the ordering reused.

## Arms
| arm | definition |
|---|---|
| a direct | sparse LU of W every attempt; J from 8 or 5 coloured JVPs, rebuilt per new step |
| b mf | unpreconditioned matrix-free GMRES with the coupled target |
| c lag | right preconditioner LU(I − h_P·γ·J_P); refreshed when ρ = h/h_P leaves [½, 2] or the previous attempt had a stage over 10 columns |
| d lin | right preconditioner LU(I − h_P·γ·A), A = declared linear part; same window rule, iteration trigger only when A depends on t |
| d-dst | Brusselator only: fast DST solve of I − hγA at the current h, no factorization; favourable model 2.5·N·log₂N per 1-D DST |

Control arms: `lag-rho` (iteration trigger off) and `lag-w4` / `lin-w4` (window [¼, 4]).

## Replica fidelity
- **SPD07 BASE.json** (gmres_into_zero, Bruss-1d-50 and -160 at 1e-6 and 1e-8): attempts/accepted/rejected, jvp_vectors, inner products and rhs_evaluations are exact in all 4 cells. For example Bruss-1d-50 at 1e-6 gives 92/82/10, 30666 JVPs, 1197200 inner products, 726 RHS. vector_updates differ by +2.6% because my count also includes the `used` axpys of the solution update.
- **Corpus semilinear n=1536** (Rust v2 dense arm): attempts/rejected 36/8 and 69/3 are exact at 1e-6 and 1e-8. At 1e-4 I get 24/7 against Rust's 23/6, the same offset as probe A2's replica.
- **No Rust counters exist for N×N grids or the 2-D Brusselator.**

## LU structure (W, MMD ordering)
κ = factor flops / solve flops.

| problem | N | nnz(L+U)/row | F_LU(W) | S(W) | κ_flop | κ_time | linear part F / S |
|---|---|---|---|---|---|---|---|
| bruss2d | 32 / 64 / 128 / 256 | 39 / 54 / 72 / 95 | 3.17e6 / 3.27e7 / 3.16e8 / 3.32e9 | 1.58e5 / 8.81e5 / 4.70e6 / 2.48e7 | 20 / 37 / 67 / 134 | 29 / 32 / 48 / 73 | 8.6e6/4.8e5 (N=64), 8.3e7/2.6e6 (128), 8.1e8/1.3e7 (256) |
| semilin2d | 32 / 64 / 128 / 256 | 22 / 30 / 40 / 51 | 4.0e5 / 4.3e6 / 4.2e7 / 4.0e8 | 4.3e4 / 2.4e5 / 1.28e6 / 6.6e6 | 9 / 18 / 33 / 61 | 28 / 38 / 37 / 48 | same as W |

SuperLU's factorization speed rises from 0.3 to 4.4 GF/s with N while triangular solves stay at 0.9–2.4 GF/s. So κ_time does not grow like κ_flop.

## Matched accuracy, ratio arm / direct
Frontier R / cheapest-run C at E = 1e-6 (E = 1e-8 for α=0.1 and the badly scaled cell). "-" means not run at that size.

| problem | mf F | lag F | lin F | dst F | lag T | lin T |
|---|---|---|---|---|---|---|
| bruss2d-32 | 3.49/3.28 | 2.20/2.28 | 1.82/1.80 | 1.79/1.77 | 1.52 | 1.55 |
| bruss2d-64 | 4.49/4.56 | 1.45/1.45 | 1.09/1.09 | 0.95/0.94 | 1.38 | 1.14 |
| bruss2d-128 | 5.26/5.26 | **0.94/0.93** | **0.64/0.63** | 0.48/0.48 | 1.06 | 0.88 |
| bruss2d-256 | - | **0.575/0.571** | **0.333/0.330** | 0.216/0.215 | 0.81 | 0.60 |
| bruss2d-64-a0.1 | 10.0/8.6 | 1.36/1.44 | 1.01/1.04 | 0.79/0.78 | 1.36 | 1.25 |
| bruss2d-128-a0.1 | 10.9/9.4 | 0.99/0.96 | 0.62/0.61 | 0.40/0.40 | 1.05 | 0.87 |
| semilin2d-32x48 (corpus) | 21.6/18.2 | 3.50/4.37 | 5.15/5.07 | - | 1.62 | 2.23 |
| semilin2d-32 | 17.7/16.4 | 3.75/4.23 | 5.20/5.16 | - | 1.78 | 2.28 |
| semilin2d-64 | 21.4/18.6 | 2.69/2.71 | 3.85/3.81 | - | 1.45 | 2.08 |
| semilin2d-128 | 23.4/20.0 | 1.92/1.87 | 2.64/2.54 | - | 1.46 | 1.91 |
| semilin2d-256 | - | 1.23/1.20 | 1.63/1.56 | - | 1.26 | 1.74 |
| semilin2d-64-s4 | 9.17/8.74 | 2.95/2.79 | 3.41/3.48 | - | - | - |

- Ratios are flat across E = 1e-4 to 1e-10. For example bruss2d-128 lag goes 0.929 to 0.958 and lin 0.628 to 0.662.
- The DST arm's time-model numbers are dominated by scipy's DST slowness at lengths whose transform size is prime (N+1 = 257 for N=256), so dst conclusions are flop-only.
- Absolute Gflop at rtol 1e-6:

| problem | direct | mf | lag | lin | dst |
|---|---|---|---|---|---|
| bruss2d-128 | 22.87 | 120.35 | 21.21 | 14.40 | 10.96 |
| bruss2d-256 | 221.2 | - | 126.4 | 73.1 | 47.6 |
| semilin2d-128 | 2.13 | 46.9 | 3.98 | 5.40 | - |
| semilin2d-256 | 17.0 | - | 20.3 | 26.5 | - |

- **Per-attempt counters, bruss2d-128 at 1e-6:**
  - direct: 1.05 LU per accepted step, 8 solves per attempt, 8 coloured JVPs per new step.
  - lag: 42.5 JVPs and 42.5 preconditioner solves per attempt, 0.233 LU per accepted step, 4.3 columns per stage.
  - lin: 50.5 / 50.5, 0.233 LU per accepted step, 5.3 columns per stage.
  - dst: 45.3 / 45.3, no LU.
  - mf: 369 JVPs per attempt, 44.5 columns per stage, orthogonalization 89% of its flops.
- **Fitted scaling (flops).** Per doubling of N the ratio falls ×0.63 for lag, ×0.57 for lin and ×0.50 for dst on Brusselator; ×0.69 and ×0.67 for lag and lin on semilinear. Fitted break-even is N≈112 (lag) and N≈70 (lin) on Brusselator. On semilinear it is N≈400 (lag) and N≈600 (lin), both extrapolated.
- **Cheapest arm per cell (flops):** direct in 50 cells and dst in 31; lin in 20 when dst is excluded. lag is never cheapest. In the time model lag is also never cheapest; mf wins 6 small-N tight cells.

## Robustness and accuracy
- **Failures.** 467 runs, 0 linear failures, 0 stall acceptances, 0 non-finite results.
- **Columns per stage are independent of N for the preconditioned arms:**

| arm | bruss2d (N = 32 / 64 / 128 / 256) | semilin2d (N = 32 / 64 / 128 / 256) |
|---|---|---|
| lag | 4.9 / 5.2 / 5.1 / 4.7 | 5.3 / 5.4 / 5.7 / 5.3 |
| lin | 5.4 / 5.6 / 5.8 / 5.5 | 6.4 / 6.4 / 6.5 / 6.8 |
| dst | 4.7 at every N | - |

- **mf grows with N:** 9.6 / 16.9 / 32.9 (Brusselator), 14.6 / 22.5 / 36.8 (semilinear), 37 / 80 (α=0.1). Its maximum is 425 columns in one stage, which would fail at production maxit 200.
- **Iterations vs ρ (lag, bruss2d-128):** 6.7 at ρ∈[0.5, 0.7), 5.0 at [0.7, 0.9), 2.2 at refresh, 4.5 at [1, 1.2), 5.9 at [1.2, 1.6), 7.2 at [1.6, 2]. There is no growth with h·γ·λ_max up to about 1e3.
- **Accuracy.** At equal rtol, Krylov-arm error / direct error is 0.986–1.011 at 1e-8 to 1e-10, including the badly scaled cell, and within 0.83–1.15 at 1e-4.

## Attribution (control arms)
- **Iteration trigger.** It is inert on Brusselator (lag-rho = lag). It is essential on the nonnormal semilinear problem: without it, columns per stage grow 8.2 / 10.0 / 11.5 with N and cost is 2–3.5× higher.
- **Window width.** The wider [¼, 4] window is worse on Brusselator (1.01 vs 0.93 at N=128) and mixed on semilinear. [½, 2] stays.
- **lin vs lag.** Dropping the u-v coupling from P wins at every N on Brusselator (0.64 vs 0.94 at 128; 0.33 vs 0.57 at 256). The cost of applying P, not its quality, decides the trade.
- **dst vs lin.** Using the exact current h instead of a lagged window cuts columns per stage from 5.6 to 4.7.

## Decision: HOLD
- **The judge's lines (flops; Ir not measured):**
  - (i) corpus n=1536: loses 3.5–4.0×.
  - (ii) 2-D grid of at least 128² with sparse-LU fill: lag gives 0.93–0.96 at N=128 and 0.52–0.63 at N=256, but only 0.80–0.83 at 256 in the time model.
  - (iii) problem declaring only its linear part: lin gives 0.63–0.67 at N=128 and 0.33–0.34 at 256, which is 0.59–0.62 in time.
- **Arm (c) as specified: do not promote.** In every cell it is beaten by direct or by (d). It never wins on the scalar advection-diffusion family up to N=256 (1.2×). Its only conceivable use is a problem that declares a sparsity pattern but no linear split, multi-species, N ≳ 200 in flops (and above 256 in time).
- **Arm (d), declared linear part: PROMOTE to a preregistered Rust node, low priority.**
  - Scope: multi-component reaction-diffusion on 2-D grids with N ≥ 128.
  - Prerequisites: a sparse-LU kernel (the repo has banded only) and a right-preconditioned Krylov path (production preconditioning is left-only today).
  - Coverage gap: no current corpus row is in this niche; the n=512 holdout is far too small.
  - Interaction: adopting it voids the shift-invariant Krylov reuses (KRY-BUDGET item 3, EXPRB phi actions, Ritz gates).
  - Gate: Ir ≤ 0.7× direct at N=256 and ≤ 0.9× at N=128.

## Recommended rule
1. **Linear-part preconditioner (d).** Factor P = I − h_P·γ·A once per window, reusing the ordering. Refresh when ρ ∉ [½, 2]; if A depends on t, also when the previous attempt's maximum stage column count exceeded 10.
2. **Stage solves.** Right-preconditioned GMRES(40), zero start, on the exact operator. Use the A1 coupled WRMS target with projected stop and one true-residual check, maxit 2000 with the stall guard.
3. **Switch.** Choose (d) over direct when C_d < 0.8·C_a. Measured per-attempt models:
   - C_d ≈ r·F_P + p·(S_P + 2.8·F_op) + rest
   - C_a ≈ F_W + 8·S_W + (colours)·F_cj + rest
   - p ≈ 45–55 preconditioner applications per attempt (5–6 columns per stage); r ≈ 0.1–0.25 refreshes per attempt; the 2.8·F_op term is the JVP plus orthogonalization per application.
   - This model reproduces the measured bruss2d-128 costs: 229 vs 228.5 MF per attempt for lin, 332 vs 337 for lag.
4. **What the switch means for (c).** With S_P = S_W, the break-even is κ_W > [(p − 8) + 2.8·p·F_op/S_W] / (1 − r), about 60 on Brusselator (measured 55–66) and 79–96 on semilinear. Read κ from timed kernels (κ_time), not flops. SuperLU's κ_time stays at about 30–73, so (c) never clears its break-even at N ≤ 256 (time ratio 0.81 at 256); (d) does, because S_P ≈ 0.5·S_W and F_P ≈ F_W/4.

## Threats to validity
1. **Flops vs Ir.** Supernodal LU is more efficient per flop than triangular solves, so flop ratios favour the preconditioned arms. The time model roughly halves the gains (lin 0.64 → 0.88 at N=128).
2. **Ordering.** METIS-quality nested dissection was not available; MMD was the best I had. A better ordering would cut F_W and help direct most.
3. **DST cost.** The DST flop model is favourable. Real scipy DST was slower than the LU solve.
4. **Problem class.** Only two families: smooth, mesh-independent step counts, cheap exact sparse JVPs. No 3-D, no stiff reaction outside the linear part (where (d) would degrade), no finite-difference JVP noise.
5. **Construction and seeds.** The 2-D Brusselator is my own construction. One initial-step seed per ladder. Accept/reject sequences are almost identical across arms, so the ratios are mostly per-attempt cost ratios.
6. **Extrapolation.** N=256 ladders are shorter (1e-4 to 1e-8). Crossover N values are fits on 3–4 points. Extrapolated frontier points are flagged in `summary.txt`.
7. **Timing noise.** Individual kernel timings vary by ±10%, with outliers up to 1.6× on shared CPUs; the minimum of two runs is used.
8. **References.** The N=256 and α=0.1 N=128 references are rtol 1e-12 only. Their uncertainty is about 9e-13, against measured errors of at least 4e-11.

## Files
All under `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/pclag/`:
- Code: `driver.py`, `probs2d.py`, `lucost.py`, `run.py`, `ref.py`, `analyze.py`, `robust.py`, `scaling.py`, `kappa.py`, `summary.py`, `microbench.py`, `fid_spd07.py`, `fid_corpus.py`; copies of `coupled_target.py`, `rep.py`, `corpus_v2_copy.py`.
- Raw runs and references: `res/*.jsonl`, `refs/*.npz`.
- Tables: `tables.txt`, `summary.txt`, `robust.txt`, `kappa.txt`, `scaling_F.txt`, `scaling_T.txt`, `scaling_Ffd.txt`.
- Timing and fidelity: `microbench.json` (with `_rep1` and `_rep2`), `fid_spd07.json`, `fid_corpus.json`, `lu256.json`.
- Logs: `logs/`.