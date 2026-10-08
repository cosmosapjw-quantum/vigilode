**Critic probe report (exploratory): checks on the eight probe reports, stress tests of the recommended stack S, and the remaining gaps**

Every number here comes from Python replicas. These are pilots for future preregistered nodes and carry no ledger authority. I only read `/home/user/wt-speed` at a49f7e4 and built no Rust.

**Decision: HOLD on S as specified. PROMOTE an amended S′ to the Rust node.** S = proj-stop + A1 coupled target + PRED+cap + maxit 2000 + stagnation guard (B1 `ARMS['S']`). It wins large and safely in most new regimes. It fails badly in two, though: a strongly nonnormal operator and a long-span badly scaled run. In two more it has robustness or accuracy problems, and it relies on an unchecked assumption that the true span T is known.

## Question
1. Do the eight reports' headline numbers reproduce from their own files, and does any report break the mandatory discipline?
2. Does S lose accuracy (more than 1.5x base error), lose robustness (new failures) or cost more than base in regimes no probe covered?
3. Which untested areas could overturn a recommendation?

## Method
- **Replica:** a copy of B1's `stack.py`. Before any change it reproduces 8 B1 cells bit for bit: attempts, JVPs, dot products, RHS, flops and final state (Δy = 0). The cells are Bruss-50 at 1e-6 (A0/A3/S/LUP) and Robertson at 1e-10 (A0/S/LU/LUP).
- **Changes to the copy:**
  - an optional vectorised CGS2 orthogonalisation for speed (counts are charged as MGS2). Against MGS2, counts agree within 0.05% on Bruss-50, Bruss-160 and HIRES;
  - a misdeclared-T option;
  - a wall-clock cap per run;
  - two post-hoc remedies, described under "Rule I recommend".
- **Loop:** fully closed. Stage RHS come from the inexact solves, every solve is checked by a true residual, and failures go back to the controller.
- **Cost:** every count is admitted:
  - JVPs (or finite-difference RHS), dot products, axpys, norms, scalings, RHS and f_t evaluations, LU factorisations and LU solves;
  - flops with F_jvp = 2·nnzJ + 2n (+2n for the scaled operator), or RHS + 6n for finite-difference JVPs.
- **Arms:**
  - A0 = production;
  - Rbig = production with maxit 2000, the failure-free base;
  - R3 = the judge's cheap L2-coupling rival;
  - C3G = S without PRED;
  - LU / LUP = exact-solve twins under the I / PRED+cap controllers.
- **Ladders:** rtol 1e-4 to 1e-10 by decades.
- **Scoring:** the regression frontier R and the harness cheapest-run rule C. An "x" marks an extrapolated point.
- **Error metric:**
  - synthetic problems: endpoint WRMS against the exact solution, in tolerance units;
  - benchmark-derived problems: componentwise relative error divided by rtol (B1's metric).
- **References:** exact solutions, or Radau at rtol 1e-12 and 1e-13 (Robertson to 4e10: uncertainty 9.4e-13; vdP to 2e4: 2.0e-12; Bruss to t = 100: 3.7e-13; switched Brusselator: 3.8e-15).

## 1. Report checks and discipline flags

All headline numbers reproduce. The flags below are about how the reports interpret or gate their numbers.

| report | spot-check | result | discipline flags |
|---|---|---|---|
| **A1** target | B1's A3 is identical to `wE0.2`; HIRES 1e-10 gives 137,980 JVPs in both probes' files. Tables B, C and D in `tables.txt` match the report: HIRES 1e-10 is 72.1 JVP per accepted step for base and 48.4 for `wE0.2`. | reproduces | The headline "0–9% more JVPs than L2 coupling for n ≤ 8" contradicts A1's own matched table: PR up to 1.30, quad-4 1.55 (frontier) / 1.60 (cheapest run). Θ was calibrated and validated on the same problems. The "robust gate" leaves out cells whose step sequence differs from LU (2 of 53 for `wE0.2`); for proj it reports 1.15 while proj's endpoint error is 92x base. One h0 seed. |
| **A2** corpus | `corpus_v2.py` self-test: SELF-TEST PASS in 0.8 s. Semilinear rows in `results.json` are 17.0 / 53.2 / 207. | reproduces | Tooling only, no method claim. Its "dense arm" replica solves stages exactly, while the Rust campaign arm was matrix-free and preconditioned. B2 and B6 inherit that baseline. |
| **B5** controller | Re-ran dense vdP and HIRES at 1e-5 and 1e-8 under I, PRED+cap and I725: 10/10 cells identical in attempts, rejections and error. `factorial.txt` and `judge_eval.txt` match. | reproduces | "Error/rtol shift within 0.77–1.20x" is a median over the ladder. B5's own `dense_runs.jsonl` has per-cell PRED+cap/I ratios from 0.34 to 9.48: Robertson 1e-10 9.48x, HIRES 1e-7 4.14x. Its matrix-free replica is 7% high on HIRES (it did not adopt MGS2). JVP cost is assumed at 10 or 100 flops per component. Endpoint error only. |
| **B4** pclag | Re-ran bruss2d-32 at 1e-6, direct and lin: 63/3 attempts/rejections, 3.222e8 and 5.801e8 flops (ratio 1.80), identical. | reproduces | HOLD is consistent with the data. The time model halves the gains. One seed. The 2-D Brusselator is off-contract. |
| **B1** stack | 8 cells bit-identical (above). | reproduces | Its proposed gate (error ÷ exact twin under the same controller ≤ 1.5) cannot detect a controller-induced calibration shift, by construction. In B1's own seed data, C3P (equal to S where the budget never binds) fails the base gate at Robertson 1e-10 for **all 3 h0 seeds**: 5.07 / 1.99 / 3.75x, with LUP/LU at 9.48 / 4.41 / 1.91. Only the h0 = 1e-6 cell was reported. "Guard never fired" was a property of the test set (see robL below). The FD claim "valid at rtol ≥ 1e-9" rests on Bruss-50 alone and fails elsewhere (§2). |
| **B2** reg | Re-read `res/analysis.txt`; the semi-96 table and swd ratios match. | reproduces | The promoted swd fails B2's own proposed gate (misroute ≤ 1.25 on every corpus family) in its own data: vdp-96 E4 1.52/1.43, semi-96 E4 1.48/1.38, rot-96 E4 1.47 (cheapest run). Where swd fires on rot and vdp it loses to direct (1.03–1.52). The on-contract evidence is the semilinear family alone; the rest is off-contract bruss2d. Several rules were added after seeing data (B2 says so). Endpoint error only. |
| **B3** expo | Re-ran rot-96 at 1e-6: E gives 146/3 attempts/rejections, 56.517 Mflop, error 5.029, identical; mf gives 76.499 Mflop at 3.429, identical. | reproduces | PROMOTE is borderline. The judge's PASS line (≤ 0.6x) is met only at n ≥ 384 on the frontier, not at n = 96 (0.55–0.71 frontier, 0.62–0.90 cheapest run) and not on the cheapest-run rule (0.55–0.64). Endpoint error only, while the corpus contract is the max over the 101-point grid. |
| **B6** geamp | `summary.json` confusion matrices (7/0/0/32 and 7/0/20/12) and the `analyze.log` attribution match. | reproduces | The threshold was chosen in-sample (B6 flags this). The exact-map separation margin is narrow: negatives up to 2.78, positives from 5.78, and one gray-zone false positive. |

One flag applies across all eight probes: none measured callgrind Ir, which is the repo's admissible unit. All flops are hand-count models, and only B4 timed kernels.

## 2. Stress tests of S

Shorthand used below:
- **lf**: linear failures.
- **equal rtol**: arm and base run at the same rtol.
- **matched**: compared at matched accuracy. Pairs are frontier R / cheapest run C unless marked JVP or flops.

### Stiff-oscillatory regimes
`stosc` has complex blocks with eigenvalues −50 ± iω_k, n = 128, a smooth forcing, and the solution on the slow manifold. `stoscx` adds an excited, damped carrier that must be resolved.

| cell | S robustness | accuracy | cost |
|---|---|---|---|
| stosc ω = 1e2 | — | S/A0 0.99–1.01 | matched S/A0 0.41 (JVP) / 0.24 (flops) |
| stosc ω = 1e3 | A0 has 22 / 5 / 2 lf at 1e-4 to 1e-6; S has 0 | S/Rbig 1.00 | matched S/Rbig 0.79 / 0.75 |
| **stosc ω = 1e4** | A0 has **637 lf at every rtol** (1,281 attempts against S's 11–87); S has 0 | S/Rbig 0.91–1.11 | **S costs 0.97–1.11x Rbig JVPs and 0.95–1.09x its flops at equal rtol; it costs more at 4 of 7 rungs.** Matched S/Rbig 1.03 / 1.05 (JVP), 1.01 / 1.02 (flops). The gain over the maxit rival is zero. |
| stoscx ω = 1e2 / 1e3 | — | S/A0 0.91–1.04 | matched 0.18 / 0.29 JVP, 0.06 / 0.16 flops |

Against R3: 1.00–1.02 JVP on stosc ω = 1e2 and 1e3, and 1.09 JVP / 1.13 flops on stoscx ω = 1e2. The coupled target gives no advantage over the L2-coupling rival here.

### Forced oscillator with resolved carrier (L-0079-type, Prothero-Robinson)
`oscpr`: n = 64, λ from −1e2 to −1e6, ω = 1e2 to 1e4.
- S/LUP is 1.00 in every cell.
- The equal-rtol shift comes from PRED (LUP/LU gives the same ratios): up to 2.44x at ω = 1e4, rtol 1e-9; 1.78 and 1.58 at 1e-8 and 1e-10; 1.33 at ω = 1e3, rtol 1e-10. All errors are at most 0.09 tolerance units.
- Matched S/A0, JVP / flops: 0.86 / 0.79, 0.67 / 0.51, 0.40 / 0.19 for ω = 1e2, 1e3, 1e4.
- Matched S/R3 (JVP): 1.09, 1.02, 1.05.

### Strongly nonnormal operators
**E-05 replica.** A = Q(D+N)Qᵀ, n = 256, built with E-05's construction and seed (LCG + Householder), with s = 0, 1, 10. The time-0 ‖W⁻¹‖ in the error-weighted (WRMS) norm, with h = 0.05·span, is 12–18.
- A0 has 53 / 33 / 1 linear failures (s = 0) and 31 / 41 / 2 (s = 1) at loose rtol. S has none, and the guard never fired.
- S/LUP ≤ 1.22 in every cell.
- Matched S/A0: 0.57 / 0.60 / 0.43 (JVP). Matched S/Rbig: 0.70 / 0.69 / 0.43.
- Dense LU is 5–8x cheaper in flops at n = 256.

**VIG-A02 blocks, k = 20** (n = 64; ‖W⁻¹‖ in that norm is 4.9e5):
- Accuracy: **S is 40–1,598x base error**, with S/LUP 41–962. C3G (no PRED) is just as bad: C3G/LU 27–4,487. **The coupled target is the cause.**
- Cost: attempts 2–4x base, **JVPs 1.4–3.3x base**. At matched accuracy S/A0 is 8.8 (JVP) / 4.1 (flops).
- Other arms:
  - A2 (proj) and R3 hit the 20,000-attempt cap with errors around 2.6e6;
  - production survives only because it over-solves.

**Other VIG cells:**
- **Single 2×2 block, k = 20:** S/LUP up to 97x, JVPs 1.2–1.55x base, matched 1.21 (JVP) / 1.56 (flops).
- **k = 10:** S is fine (matched 0.68). Production itself is 10–22x worse than LU at rtol 1e-5 and 1e-6.
- **k = 46:** ill-posed in binary64; every arm, LU included, hits the attempt cap.

### Undeclared discontinuous forcing
**heatsw:** 1-D heat equation, n = 128, with a source switched on and off at five undeclared times. Exact reference.
- Robustness: A0 has 4–25 linear failures at 1e-4 to 1e-8; S has 0.
- Errors are chaotic for every arm: per cell, LU/A0 ranges from 0.013 to 44.
- Pooled over 3 h0 seeds × 7 rtols (21 cells):

| arm | geometric-mean error (tolerance units) |
|---|---|
| S | 0.547 |
| A0 | 0.407 |
| LU | 0.41 |
| LUP | 0.37 |

| ratio | geometric mean | cells over 1.5x |
|---|---|---|
| S/A0 | 1.34 | 11/21 |
| S/LUP | 1.48 | 9/21 |
| LUP/LU (noise control) | 0.90 | 8/21 |
| LU/A0 (noise control) | 1.01 | 10/21 |

  This suggests about 1.4x contamination, which this sample cannot separate from noise.
- Matched S/A0 0.57 (JVP) / 0.43 (flops), but **S/R3 1.22 (JVP) / 1.25 (flops): the R3 rival is cheaper.**

**brusw:** Bruss-50 with a switched source.
- Errors are 15–100 rtol for every arm.
- Pooled S/LUP geometric mean 1.14 against LUP/LU 1.08, so no contamination is detectable.
- Matched S/A0 0.21 (JVP) / 0.08 (flops); S/R3 1.02 / 1.00.

### Long-time integration
- **vdP μ = 1000 to t = 2e4 (12 periods):** S is fine; matched 0.62 (JVP) / 0.80 (flops). Every arm hits the attempt cap at 1e-10.
- **Bruss-50 to t = 100:** S is fine; matched 0.29 / 0.12.
- **Robertson to t = 4e10 (robL), new failures:**
  - **S and C3G livelock at all 8 rtols from 1e-4 to 1e-11.** They have 2,549–6,935 linear failures, and every run is capped at t ≤ 1.3e9.
  - A0 and LU finish in 103 to about 2,600 attempts with 0 failures.
  - Mechanism: the guard's q ≥ 0.98 abort, with **50–80% false aborts** (the shadow continuation would have converged).
  - At the aborts the residual stagnates at 5e-13 to 6.5e-12·‖Db‖. That is 136–1,840x the target and 1–14x the stall-rule threshold: the attainable-accuracy floor RHO_FL = 16ε‖Db‖ underestimates the round-off floor of the badly scaled operator.
  - Span T = 4e7 works; 4e9 fails.

### Misdeclared span T
T is the span the EPUS target uses for its per-step share.
- **Declared span = T/100** (for example, a driver called once per output interval):
  - HIRES error rises 1.6–69x over S: 4.93 against 0.177 rtol at 1e-6, 6.15 against 0.085 at 1e-9, 5.49 against 0.0795 at 1e-10.
  - That is up to **66x base** in exchange for a 15–17% JVP saving.
  - Bruss-50 is unaffected.
- **Declared span = 100T:** accuracy equals S; JVPs rise 7–11% (HIRES) and 30–39% (Bruss-50) over S.

### Finite-difference JVPs
Compared: B1's FD-aware variant **Sfg8** against the production-form FD rival **A0f8s** (rtol_lin 1e-8 plus a stall backstop).

| problem | Sfg8 lf | A0f8s lf | other |
|---|---|---|---|
| HIRES | 31 / 15 / 4 | 0 / 0 / 0 | 21 / 9 / 4 false aborts; true/projected > 10 on 261–545 exits, which trips the >1% FD kill rule |
| Robertson | 23 / 28 | 0 / 0 | — |
| vdP | 27 / 21 / 9 | 2 / 0 / 0 | at 1e-8 the error is 9.11 against 2.15 rtol (4.2x) |
| heatsw | 285 / 338 / 124 | 426 / 657 / 133 | both arms fail |

- **e05s1, rtol 1e-6:** Sfg8 error is 2.6x A0f8s.
- **Sfg8 cheaper:** 0.13–0.93x of A0f8s FD flops on E-05, stosc ω = 1e3, vdP and HIRES at 1e-6 and 1e-8.
- **Sfg8 more expensive:** HIRES 1e-4 (1.18x) and Robertson 1e-4 / 1e-6 (1.58x / 1.33x).
- **As-calibrated arms:** S with FD JVPs and production with FD JVPs livelock or fail on every problem tested.

### Tight, badly scaled cells
- Robertson 1e-10: S fails the equal-rtol gate at all 3 seeds (from B1's data, above). This is PRED.
- Robertson to 4e10, rtol 1e-5: LUP/LU = 9.8.
- HIRES 1e-9 and 1e-10 across 3 seeds: 0.81–1.21, fine.
- Robertson 1e-11: S gives 0.54 rtol against A0's 2.67 (A0's absolute floor exceeds atol).

## 3. Untested areas and which recommendations they could overturn

| gap | recommendation at risk |
|---|---|
| **DAE / mass matrix** (the matrix-free U form refuses M) | A1's τ constants and the EPUS derivation assume M = I and an ODE. B5's PRED evidence: algebraic-variable error estimates are rougher. B2 and B3 (ROCK4 and EXPRB cannot integrate DAEs, so the switch must be disabled). B6's transport needs M. All of B1. |
| **2-D / 3-D grids** | B1's factors come only from 1-D Bruss and n ≤ 8. In B4, unpreconditioned matrix-free GMRES needs 9.6–32.9 columns per stage, up to 425, so the guard and maxit layer becomes active and its false-abort rate matters. In 3-D, sparse-LU fill grows, which could flip B4's HOLD on lagged preconditioning and B2/B3's "direct dominates". |
| **Expensive JVPs** | B1's flop ratios (Bruss-50 0.10) move toward its JVP ratios (0.26). B5's null (c) lead disappears. B3's EXPRB gains, since it needs about 30 operator applications per attempt against 45–200 JVPs. B6's exact-map overhead (+8 JVPs per step) grows. The SVD charge of the ν-guard becomes negligible. |
| **GPU / parallel** | Reductions in orthogonalisation dominate, so every flop weighting is wrong. ROCK4 has no inner products and gains. Sparse LU scales poorly, which could overturn "direct dominates" in B2, B3 and B4. CGS2 against MGS2 matters. |
| **Finite-difference JVPs** | Sfg8 is not robust (§2). Keep FD on HOLD. |
| **Dense output / max-grid error** | B1, B2, B3 and B5 scored endpoint error only. PRED's larger steps and contamination at interior points are unmeasured against the corpus max-grid contract. |
| **Events and declared breakpoints** | Per-cell equal-rtol gates are ill-posed under discontinuities (noise is about 10x per cell); gate on distributions instead. |
| **S with a preconditioner** | How the ν-guard, PRED and the guard interact with B4's right preconditioning is untested. |
| **Unknown T (open-ended integration)** | EPUS needs T. Per-interval T loses up to 66x on HIRES. |
| **Rust / Ir; holdout families** | No new arm exists in Rust. The holdout families have never been integrated. |

## Rule I recommend (amended stack S′)
These remedies were added after seeing the stress data. They are exploratory pilots, not certified.
1. **Production fallback at the guard (arm Sfb).**
   - Before declaring a linear failure on a guard abort (q ≥ 0.98 or predicted overrun) or on reaching maxit, accept the iterate if the unscaled true residual meets production's own rule: ‖b − Wx‖₂ ≤ max(γ·1e-14, 1e-10‖b‖₂), with 1e-8 for FD JVPs. Otherwise reject with h × 0.2.
   - On robL: 0 failures at all 8 rtols, accuracy equal to the exact twin. Cost against base at matched accuracy is 0.99–1.04 in JVPs and 1.22–1.29 in flops.
   - On Bruss-50, HIRES and Robertson the counts are identical to S.
2. **Nonnormality guard (arm Snu).**
   - When the projected residual first meets thr_i, compute σ_min(R_j) from the Givens-triangularised Hessenberg factor. Keep ν̂ = max over the solve of 1/σ_min. If ν̂ > 1, set thr_i = thr_i⁰/ν̂ and require the same threshold of the true residual.
   - Keep the stall rule. Without it, the ν-guard livelocks on VIG k = 20 (7,228 linear failures).
   - On VIG k = 20 it restores accuracy (Snu/LUP 0.26–1.03), but costs 1.08–1.55x base JVPs and 1.26–1.78x base flops at matched accuracy. On k = 10 it costs 0.87x JVPs and 1.13x flops.
   - Elsewhere it is close to inert: +0–0.4% JVPs on Bruss-50, HIRES and Robertson, and +3% on E-05 s = 1. The flops figures charge a dense SVD (4j³ flops per check), which is pessimistic: +6–35% on HIRES, +5–11% on Bruss-50.
3. **Span contract.** Use EPUS only with the true remaining span. If T is unknown, fall back to A1's per-step budget of 1e-4 (`wS1e-4`), which passed A1's gates.
4. **Gates for the Rust node:**
   - PRED needs a separate equal-rtol calibration gate, multi-seed, including the Robertson 1e-10 cell.
   - Add these cells as mandatory: VIG k = 20 (n = 64 and n = 2), Robertson to 4e10, stosc ω = 1e4 (with Rbig as rival), heatsw (gate on pooled distributions), and HIRES with T/100.
   - Count guard false aborts.
   - R3 must be a rival everywhere: S is 0.95–1.26x R3 on the new problems.
5. **FD stays HOLD.** With fallback at 1e-8, failures drop on HIRES (31 → 9) and Robertson (23 → 2), but not on vdP (27 / 21 / 9 unchanged).

## Threats to validity
- **Replica, not Rust.** The stress runs use CGS2. Its counts match MGS2 within 0.05% on 3 cells, but round-off differs on nonnormal problems.
- **Remedies were designed after seeing the data.** The ν-guard's cost is charged pessimistically.
- **Problems are mostly linear and synthetic, with one h0 seed** (three for the discontinuous cases). The E-05 operator was rebuilt from its construction and seed, not checked bitwise against Rust (`e05_matrices.json` is absent).
- **Some tight cells are capped for every arm** (vdP to 2e4 at 1e-10; VIG k = 20 at 1e-9 and 1e-10) and are excluded.
- **Per-cell gates are noise-dominated** on the switched problems.
- **Error metrics are endpoint-only. Flops are hand-count models, with no Ir or wall time.**

Everything is in `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/critic/`:
- Code: `stack.py` (B1 replica plus options), `crun.py`, `sprobs.py`, `e05mat.py`, `ref.py`, `cana.py`, `dist.py`, `fdtab.py`
- Tables and summaries: `tables.txt`, `tables_snu.txt`, `tables_sfbnu.txt`, `fd_table.txt`, `dist_discontinuous.txt`
- Raw runs: `res/*.jsonl`
- References: `refs/*.json`
- Spot checks: `spot/` (`b1_seeds_gate.py`, the B4/B3/B5 copies)
- Logs: `logs/` (`spot_b1_a.log`, `spot_b4.log`, `spot_b5.log`, `cgs_check.log`, `b1_seeds_gate.log`, `opdiag.log`, `q*.log`)