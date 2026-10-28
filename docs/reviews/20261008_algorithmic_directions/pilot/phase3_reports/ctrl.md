**PROBE B5 report: step-size controller adoption (CTRL-PRED-CAP), closure of CTRL-EXPANSIVE-GATE, check of CTRL-NULLS**

These are exploratory replica pilots in Python/numpy, meant to inform future preregistered nodes. They are not ledger authority. The worktree `/home/user/wt-speed` was only read, and no Rust was built. All files are in `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl/`.

## Question

1. Should VigilODE adopt the predictive controller (Hairer/Gustafsson min form), with or without the no-growth-after-rejection cap?
2. Is the gain from the predictive term or from the set-point?
3. Should CTRL-EXPANSIVE-GATE be closed?
4. Do the CTRL-NULLS items (a) to (g) and the catastrophe abort hold up?

## Method

**Replicas.** All runs are closed loop.
- **Dense fast driver** (`core.py: dense_integrate`): U form, exact J, one LU per attempt, J/f0 reused after a rejection, retry clipped below the rejected step, landing on tf.
- **Matrix-free U form** (`core.py: mf_integrate`):
  - Uses the mfrep GMRES(40) with x0 = 0 and maxit 200.
  - Stage right-hand sides come from the inexact solves; every exit is certified by a true residual; linear failures become rejections (×0.2) that the controller sees.
  - Modes:
    - `proj`: proj-stop at the base L2 target max(γ·1e-14, 1e-10·‖b‖).
    - `tf1`/`tf2`: the INO-FORCE-ABS uncertified absolute WRMS targets, θ = 0.001/0.002, err_prev^1.2 scaling, U8 floor.
    - `rel4`: uniform relative forcing η = 1e-4 (negative control).
    - `base`: production full cycles, used for fidelity only.

**Admitted cost.**
- Dense: attempts = LU factorizations; RHS calls (8 on a fresh attempt, 7 on a retry); Jacobian builds; flops = attempts·(2n³/3 + n² + 16n²) + RHS·c_rhs·n + 3n·J.
- Matrix-free: JVPs (columns plus true-residual matvecs, plus the diagnostic residual in base mode); orthogonalization inner products and vector updates, counted as production two-pass MGS counts them (2(j+1) of each per column); other n-vector ops; RHS calls. There are no preconditioner applications and no LU.
- Flop models, stated per JVP:
  - **flops10**: 10 flops per state component per JVP. That is 1,000 flops/JVP for Bruss-50 (n = 100), 3,200 for Bruss-160 (n = 320) and 80 for HIRES.
  - **flops100**: 100 flops per component.
  - In both, an inner product, a vector update or another vector op costs 2n; an RHS costs 7n (Brusselator) or 6n (HIRES); the GMRES least-squares O(j) term is ignored.

**Ladders and seeds.**
- Quarter-decade rtol ladders: 1e-3 to 1e-7 (17 points) for vdP μ = 1000, Bruss-50 dense and Bruss-50/160 matrix-free; 1e-3 to 1e-10 (29 points) for HIRES and Robertson.
- h0 seeds: {1e-6, 1e-4, 1e-2, Hairer automatic h0}. The automatic h0 is HNW HINIT with iord 5 and costs one extra RHS.
- The noise arm uses 2 seeds (1e-6 and auto).
- A secondary dense problem, the corpus v2 rotating-nonnormal family (n = 96, [0, 1], atol = 0.01·rtol, rtol 1e-4 to 1e-8), was added for the gate's firing test.
- In total there are 8,340 dense runs and 2,543 matrix-free runs. The only unfinished runs are 39 HIRES `rel4` negative-control runs at the 6,000-attempt cap.

**Error and references.**
- Endpoint error uses the stiff-benchmark metric max_i |y_i − r_i| / max(|r_i|, 1e-10), against the NATIVE.json references.
- A Radau rtol-1e-13 check differs from NATIVE by at most 4.4e-13 (HIRES 1.8e-13), so the 1e-10 cells are usable.
- Bruss-160 has no stored reference. I computed one with SciPy Radau (sparse J) at rtol 1e-13, with a 1e-12-vs-1e-13 uncertainty of 1.25e-13.
- Rotating-nonnormal uses the normwise error against the exact φ.

**Scoring.** Every scoring rule is applied per seed.
- **R (regression frontier):** an OLS fit of log work against log error over the whole ladder, evaluated at E.
- **Rw:** the same fit restricted to a ±1-decade window around E.
- **C (cheapest run):** the harness rule, the cheapest run reaching E.
- **Cp:** the cheapest run pooled over seeds.
- Ratios are reported as median [min–max] over seeds.
- **x** marks an E outside either arm's measured error range (extrapolated); those points are excluded from geometric means ("gm"). gm values use half-decade in-range E grids.

## Arms

- **I:** production. Factor 0.9·err^(−1/5), clamped to [0.2, 5] on accept and [0.2, 0.9] on reject.
- **I725:** safety 0.725 on accepted proposals only; the set-point rival.
- **Set-point sweep:** I85, I80, I65; PRED85 (run as PRED+cap85, since the cap is neutral in dense), PRED80, PRED725, PRED65.
- **PIc:** PI as coded in adaptive.rs.
- **H211b+cap, PI34+cap:** at the matched set-point θ = 0.9^5.
- **PRED:** min(0.9·err^(−1/5), 0.9·(h/h_acc)·(err_acc/err²)^(1/5)), each term clamped to [0.2, 5], with err_acc = max(1e-2, err_prev).
- **PRED+cap**, **PRED+gate**, **PRED+cap+gate** (ρ1 > 1 → proposal ×(0.2/0.59)^(1/5)), **I+gate**.
- **I+kobs, PRED+kobs.**
- **PRED+cc12/cc20:** a bounded Krylov column cap, used for null (c).

## Results

### R0. Replica fidelity against recorded Rust counters

- **Dense vs `sb_fast.json`** (rodas5p-fast, I controller, h0 1e-6, rtol 1e-3 to 1e-9 on Robertson, HIRES, vdP and Bruss-50): 27 of 28 cells match exactly in attempts, accepted, rejected, RHS, Jacobian builds and LU, and in final error to 4 digits. The exception is vdP at 1e-3: 180/114/66 against Rust's 181/114/67.
- **Matrix-free base vs SPD07 `BASE.json` (gmres_into_zero):**
  - Bruss-50, Bruss-160 and vdP at 1e-6 and 1e-8 match exactly in attempts, accepted, rejected, JVP, iterations, inner products and RHS. For example, Bruss-160 at 1e-6 gives 92/82/10, 56,906 JVPs and 2,246,800 inner products.
  - HIRES is high: JVPs +6.7%/+7.7% and inner products +17%/+20%.
  - Robertson JVPs are +0.46%/+0.06%.
- No Rust counters exist for any non-I arm.

### R1. Rejection fractions

Dense, pooled over the ladder and 4 seeds; brackets give the seed range.

| arm | vdP | HIRES | Bruss-50 | Robertson | rot-nn96 |
|---|---|---|---|---|---|
| I | 28.6% [28.1–29.1] | 1.5% | 13.8% | 3.5% | 2.43% |
| I725 | 9.2% | 0.4% | 2.4% | 1.8% | 0.01% |
| PIc | 15.9% | 0.3% | 2.4% | 1.1% | |
| H211b+cap | 29.5% | 1.8% | 9.8% | 2.2% | |
| PI34+cap | 26.9% | 1.4% | 7.3% | 1.7% | |
| PRED | 7.2% [6.7–7.8] | 0.7% | 6.0% | 3.2% | 0.60% |
| PRED+cap | 7.1% | 0.7% | 6.0% | 3.1% | 0.53% |
| PRED725 | 2.9% | 0.3% | 1.7% | 1.5% | 0.01% |
| PRED+cap+gate | 3.9% | 0.7% | 3.5% | 2.8% | 0.28% |

HIRES fractions are lower than the earlier 8% because this ladder runs to 1e-10.

Matrix-free, proj mode, 4 seeds:
- Bruss-50: I 13.8%, I725 2.4%, PRED 6.0%, PRED+cap 6.0%, PRED725 1.7%.
- Bruss-160: I 17.6% (345 linear failures), I725 6.3% (218), PRED 12.4% (351), PRED+cap 9.3% (201), PRED725 5.4% (191).

Ladder totals at h0 = 1e-6 and equal rtol (not matched accuracy), as LU count:

| problem | I | I725 | PRED | PRED+cap |
|---|---|---|---|---|
| vdP | 6,337 | 5,662 | 5,006 | 5,001 |
| HIRES | 19,120 | 25,458 | 19,161 | — |
| Bruss-50 | 1,269 | 1,309 | 1,183 | — |
| Robertson | 2,921 | 3,977 | 2,931 | — |

Median endpoint error/rtol, I against PRED: vdP 1.71 vs 1.69, HIRES 0.13 vs 0.10, Bruss-50 1.04 vs 1.25, Robertson 0.55 vs 0.59.

### R2. Dense matched accuracy (work = attempts = LU)

Each cell is the median [min–max] over seeds. Dense flops ratios agree with these within 2%.

**vdP, ratio to I:**

| arm | rule | 1e-3 | 1e-4 | 1e-5 | 1e-6 | 3e-7 |
|---|---|---|---|---|---|---|
| PRED | R | 0.68 [0.58–0.72] | 0.72 [0.65–0.75] | 0.76 | 0.81 | 0.84 |
| PRED | C | 0.75 | 0.67 [0.53–0.85] | 0.74 | 0.94 | 0.81 |
| I725 | R | 0.86 | 0.86 | 0.86 | 0.86 | 0.86 |
| I725 | C | 0.88 | 0.81 | 0.82 | 0.93 | 0.93 |
| PRED vs I725 | R | 0.76 | 0.82 | 0.88 | 0.95 | 0.99 |

**HIRES, ratio to I:**

| arm | rule | 1e-4 | 1e-5 | 1e-6 | 1e-7 | 1e-8 | 1e-9 | 1e-10 |
|---|---|---|---|---|---|---|---|---|
| PRED | R | 0.88 [0.85–0.92] | 0.89 | 0.91 | 0.93 | 0.95 | 0.97 | 0.99 [0.97–1.01] |
| PRED | C | 0.85 | 0.95 | 0.87 | 1.02 | 0.94 | 1.01 | 1.00 |
| I725 | R | 1.00 | 1.01 | 1.03 | 1.05 | 1.06 | 1.08 | 1.09 |
| PRED vs I725 | R | 0.88–0.90 at every E | | | | | | |

**Bruss-50, ratio to I:**

| arm | rule | 3e-5 | 1e-5 | 3e-6 | 1e-6 | 3e-7 |
|---|---|---|---|---|---|---|
| PRED | R | 0.95 | 0.94 | 0.93 | 0.93 | 0.91 |
| PRED | C | 0.88 | 0.89 | 0.94 | 0.97 | 0.99 |
| I725 | R | 0.96 | 0.95 | 0.93 | 0.92 | 0.90 |
| PRED725 | R | 0.87 | 0.87 | 0.87 | 0.87 | 0.87 |

PRED vs I725 on Bruss-50: R 0.98–1.02, C 0.92–1.10.

**Robertson, ratio to I:**

| arm | rule | 1e-5 | 1e-6 | 1e-7 | 1e-8 | 1e-9 | 3e-10 |
|---|---|---|---|---|---|---|---|
| PRED | R | 1.00 | 1.01 | 1.01 | 1.02 | 1.03 | 1.03 [1.01–1.06] |
| PRED | C | 1.00–1.02 | | | | | |
| I725 | R | 0.93 | 0.98 | 1.02 | 1.07 | 1.13 | 1.17 |

**Rotating-nonnormal:** PRED R 0.94, 0.97, 1.01, 1.05 at E = 1e-6, 1e-7, 1e-8, 1e-9; gm R 0.98, C 0.97; the worst C is 1.10 at 1e-8.

### R3. Predictive term or set-point? Factorial over controller and safety

Values are dense gm R / gm C against I.

| safety | 0.9 | 0.85 | 0.80 | 0.725 | 0.65 |
|---|---|---|---|---|---|
| vdP I | 1 | 0.915/0.921 | 0.859/0.886 | 0.861/0.852 | 0.809/0.793 |
| vdP PRED | **0.750/0.767** | 0.785/0.780 | 0.737/0.731 | 0.789/0.773 | 0.746/0.762 |
| HIRES I | 1 | 1.013/1.066 | 1.034/1.052 | 1.046/1.087 | 1.049/1.122 |
| HIRES PRED | **0.932/0.944** | 0.940/0.967 | 0.942/0.964 | 0.975/1.024 | 0.972/0.989 |
| Bruss-50 I | 1 | 0.966/0.956 | 0.949/0.939 | 0.931/0.903 | 0.913/0.882 |
| Bruss-50 PRED | **0.931/0.932** | 0.912/0.902 | 0.896/0.888 | 0.870/0.885 | 0.858/0.867 |
| Robertson I | 1 | 1.032/0.987 | 1.012/0.984 | 1.025/0.980 | 1.051/0.966 |
| Robertson PRED | **1.014/1.004** | 1.014/0.995 | 1.007/0.990 | 1.000/0.970 | 1.064/0.960 |

Worst single-E R ratio for Robertson: PRED 1.033, PRED80 1.069, PRED725 1.100, I725 1.167, I65 1.261. For HIRES: PRED 0.986, PRED725 1.077, I725 1.095.

How the gain splits:
- **The predictive term is the robust part.** It gives −25% on vdP, −7% on HIRES, −7% on Bruss and ±1% on Robertson, and is never above 1.033 at any E.
- **The set-point depends on the problem.**
  - vdP: it captures about half of the log-gain (I725 0.861 against PRED 0.750) and converges with PRED only at E ≤ 1e-6, so the judge's "most of the vdP gain" holds only at tight E.
  - Bruss: it captures all of the gain (I725 = PRED = 0.931).
  - HIRES: it captures none, and makes HIRES worse (1.01–1.09).
  - Robertson: it costs up to 1.17–1.26 at tight E.
- **Combining them** (PRED80) adds 2–4% on vdP and Bruss, costs 1–2% on HIRES, and takes the Robertson worst case to 1.07.

### R4. Matrix-free Bruss-50/160, proj at L2 1e-10 (4 seeds, gm R/C, in-range E)

| comparison | Bruss-50 JVP | Bruss-50 flops10 | Bruss-50 flops100 | Bruss-160 JVP | Bruss-160 flops10 | Bruss-160 flops100 |
|---|---|---|---|---|---|---|
| PRED / I | 0.913/0.918 | 0.904/0.978 | 0.908/0.937 | 0.934/0.921 | 0.933/0.923 | 0.933/0.922 |
| PRED+cap / I | 0.913/0.919 | | | 0.908/0.909 | 0.905/0.912 | 0.906/0.910 |
| I725 / I | 0.895/0.887 | 0.867/0.948 | 0.881/0.903 | 0.913/0.889 | 0.906/0.894 | 0.908/0.891 |
| PRED725 / I | 0.863/0.877 | 0.862/0.947 | 0.864/0.897 | 0.896/0.883 | | |
| PRED / I725 | 1.022/1.030 | 1.043/1.032 | 1.032/1.034 | 1.024/1.038 | | |
| PRED+cap / PRED | | | | 0.980/1.000 (range 0.94–1.02) | | |
| PRED725 / I725 | 0.964/0.988 | | | 0.983/0.992 | | |

- On Bruss-50, H211b+cap is 1.000 and PI34+cap 0.989 against I in JVPs, but both are 1.08–1.09 against PRED+cap.
- Sample absolute counters, rtol 1e-6, seed 1e-6, Bruss-50:

| arm | attempts | rejected | JVP | inner products | flops10 |
|---|---|---|---|---|---|
| I | 92 | 10 | 14,331 | 307,112 | 1.463e8 |
| PRED | 86 | 3 | 13,343 | 286,130 | 1.363e8 |
| I725 | 97 | 0 | 13,785 | 265,328 | 1.287e8 |

- Bruss-160 at 1e-4:

| arm | attempts | rejected | linear failures | JVP | flops10 |
|---|---|---|---|---|---|
| I | 73 | 21 | 13 | 51,654 | 2.70e9 |
| PRED | 74 | 20 | 15 | 52,780 | — |
| PRED+cap | 67 | 12 | 8 | 44,104 | 2.29e9 |
| I725 | 52 | 5 | 1 | 33,683 | — |

- **The matrix-free frontier is almost flat.** For Bruss-50 proj with I, d log(work)/d log(err) is −0.055 for JVPs and +0.044 for flops10: flops10 falls from 1.70e8 at rtol 1e-3 to 1.34e8 at 1e-7. So matched-accuracy matrix-free ratios are close to level comparisons, and the cheapest-run rule in flops picks the tightest run.

### R5. Noise arm (2 seeds; PRED/I gm R/C in JVPs)

| problem | proj | tf1 | tf2 | rel4 (accuracy broken) |
|---|---|---|---|---|
| Bruss-50 | 0.911/0.916 | 0.916/0.921 | 0.916/0.921 | 0.959/0.833 (3 E; err/rtol median 5.1, max 508) |
| Bruss-160 | 0.935/0.920 | 0.925/0.924 | 0.916/0.918 | 0.931/0.879 (2 E; err/rtol max 278) |
| HIRES | 0.905/0.911 (max err/rtol 5.6–8.3) | 0.921/0.923 (max err/rtol 1.15–1.32) | 0.935/0.954 (worst E 1.014) | both fail at rtol ≤ 1e-6 (6,000-attempt cap, err/rtol 1e8–1e13) |

HIRES rel4 at rtol 1e-5: PRED 2,942 attempts (err/rtol 86) against I 2,599 (err/rtol 42).

- **Measured noise in the controller's error estimate**, |log(err_inexact/err_exact)| on attempts with 0.2 < err < 5, 90th percentile:
  - tf1 ≤ 1.8e-5 and tf2 ≤ 5.2e-5, with 0 accept/reject flips in about 8,300 attempts each.
  - proj: median 1.1e-6.
  - rel4: about 1.1–1.5, with 1,984 flips in 37,175 attempts.
- Under the INO targets, PRED's 2/k amplification acts on a perturbation of about 1e-5, which is inert. With INO, PRED's Bruss-160 rejection fraction falls from 12.4% to 7.3% (linear failures 179 → 33).
- Uniform relative forcing breaks accuracy for both controllers. There, PRED loses its rejection advantage on Bruss-50 (12.3% against I's 10.7%).
- So PRED needs tolerance-coupled stage targets, as the merged candidate's assumption (ii) states; the noise arm confirms this rather than refuting PRED.

### R6. The judge's PASS and KILL lines for PRED (both PRED and PRED+cap pass)

- **(i) vdP ≤ 0.90 at ≥ 3 of 5 E:** R 5/5 (0.675, 0.716, 0.759, 0.813, 0.857, with 1e-7 extrapolated); C 3/5 (0.747, 0.671, 0.738, 0.941, n/a). PASS.
- **(ii) No problem > 1.06 (R) or > 1.10 (C).** PASS. The worst cases are Robertson R 1.033 at 3e-10 and C 1.05; rotating-nonnormal R 1.048 and C 1.097; every matrix-free Bruss cell is ≤ 0.96.
- **(iii) Beats I725 by ≥ 3% on HIRES or Bruss.** PASS on HIRES only (0.885/0.873). Bruss does not meet it: dense 0.99/1.01, matrix-free 1.02–1.04.
- **KILL lines:** none fire.
- The merged candidate's matrix-free predictions also hold: Bruss-160 ≤ 0.94 (gm 0.934) and Bruss-50 ≤ 0.96 (gm 0.913).

### R7. CTRL-EXPANSIVE-GATE (base PRED+cap): fails all four keep criteria

1. vdP gain is 0.958/0.938 (needs ≤ 0.92).
2. Against the best uniform arm, PRED+cap80, it is 1.004/0.981 (needs ≤ 0.97).
3. It is not within ±4% elsewhere: Robertson 1.054 (up to 1.086) and rotating-nonnormal 1.085 (up to 1.120). HIRES 0.994 and Bruss 0.967 are inside the band.
4. It fires on 41.2% of accepted rotating-nonnormal steps (per run 24–47%; limit 30%) and adds 9% attempts there.

In matrix-free Bruss-50 the gate equals I725 (0.992 against I725).

### R8. CTRL-NULLS

- **(a) Same-state exponent adaptation: confirmed.** I+kobs/I gm is 0.989, 1.000, 1.000, 1.003 (best single E 0.965); PRED+kobs/PRED is the same.
- **(b) Set-point-matched filters: confirmed.**
  - H211b+cap/I gm: 1.017 (vdP), 1.082 (HIRES), 1.057 (Bruss), 1.038 (Robertson).
  - PI34+cap/I gm: 0.908 vdP (best E 0.865), but 1.10/1.08/1.05 on the others, so the kill rule is not met.
  - Against PRED, both filters are 1.02–1.31 worse. PIc (as coded) is 0.895/1.176/1.103/1.066.
- **(c) Krylov-cost-aware step reduction: confirmed at the 10% rule, with a recorded lead.**
  - Bruss-160 at rtol 1e-6, 8 states, proj, cost per unit time when h → s·h:

| s | 0.35 | 0.5 | 0.7 | 1.4 |
|---|---|---|---|---|
| JVP | 1.37 | 1.23 | 1.11 | 0.90 |
| flops10 | 1.08 | 1.08 | 1.06 | 0.92 |

  - Under tf1 the JVP rate is flat (1.12 at s = 0.35), but flops10 per unit time falls to 0.64/0.78/0.89 at s = 0.35/0.5/0.7, because orthogonalization grows with columns squared when JVPs are cheap.
  - A closed-loop bounded column cap (h ≥ 0.5·h_acc, tf1, seed 1e-6) gains only a little at matched accuracy:
    - Bruss-50: flops10 0.984/0.968 and JVP 0.942/0.974 (cc12/cc20).
    - Bruss-160: flops10 0.978/0.996 and JVP 0.971/0.979.
    - The worst E is 1.04–1.06.
  - An unbounded cap ran away (Bruss-160 at 1e-7: 4,616 attempts, error 4e-14); those runs were discarded and kept in `mf_runs_colcap_unbounded.jsonl`.
- **(d) Early attempt abort: confirmed.** Under PRED, the oracle ceiling (every rejection aborted after stage 2, no false aborts) is 5.4% of attempts on vdP, 4.5% on Bruss, 2.4% on Robertson and 0.5% on HIRES. A late-stage abort (stage 7) saves at most 0.9%.
- **(e) Global-error feedback on F-033 (copied gfb.py): confirmed.** Ratios against uniform tightening are 1.00–1.25 under I (reproduced) and 0.97–1.20 under PRED.
- **(f) Stage-1 retry reuse: confirmed.** The retry share of JVPs is 0.69% on Bruss-50 and 1.29% on Bruss-160 under PRED, and at most 1.82% under I.
- **(g) Initial step: refuted as a "< 5%" null under PRED, but it is a harness choice.**
  - Equal-rtol savings reach −20% at rtol 1e-3 (Bruss-50 0.80 under I, 0.84 under PRED).
  - Matched-frontier ratios with PRED + automatic h0 against h0 = 1e-6: Bruss 0.876–0.928, Robertson 0.919–0.976, HIRES 0.931–0.956, vdP 0.931–0.988. That meets the kill rule.
  - With I, vdP reaches 1.092, so the rule is not met.
- **Catastrophe-only abort (|Y_i|∞ > 1e6·max(1, |y|∞)): confirmed as a D5 guard.**
  - On the Rust-exported U-driver stages it catches L-0049 (Robertson h = 1e-2) 3 of 3 at stage 5 (growth 7.5e59) and L-0062 (h = 3e-2) 3 of 3 at stage 4 (growth 5.2e121), and touches 0 of 64 accepted attempts.
  - It leaves the err ≈ 1e6, growth-97 rejections at h = 3e-3 alone; the error estimate already rejects those.
  - In the adaptive ladders: 8,021 attempts, 0 false aborts (largest stage growth on an accepted attempt 2.3), 13 aborts, all on rejected attempts.
  - It is trajectory-neutral, since the step factor is the 0.2 clamp either way.

## Decision

- **CTRL-PRED-CAP: PROMOTE** to the Rust node (T1-B). The replica meets every PASS line and no KILL line, on dense and matrix-free problems and under INO noise.
  - Keep the cap. It is neutral in dense runs (gm 0.99–1.00) and worth 0–6% of JVPs on budget-failing Bruss-160 matrix-free cells, where it cuts linear failures 351 → 201 and rejections 12.4% → 9.3%.
- **Predictive term or set-point:** both, depending on the problem; the predictive term is the robust component. Do not lower the default set-point. A PRED retune to safety 0.80 is HOLD.
- **CTRL-EXPANSIVE-GATE: CLOSE.**
- **CTRL-NULLS:**
  - (a), (b), (d), (e), (f) and the catastrophe abort: confirmed.
  - (c): confirmed in closed loop; the flops-per-time lead under INO targets is recorded.
  - (g): refuted as a < 5% null under PRED at loose tolerances; report it as a harness or initialization lever, not as a method gain.
  - All of these remain replica nulls, not C-list closures.

## Recommended rule (ControllerKind::Predictive)

**On an accepted step** (err ≤ 1, embedded WRMS, k = 5):
- f_I = clamp(0.9·err^(−1/5), 0.2, 5).
- If a previous accepted step exists, f_P = clamp(0.9·(h/h_acc)·(err_acc/err²)^(1/5), 0.2, 5), with err_acc = max(1e-2, err of the previous accepted step), and f = min(f_I, f_P). Otherwise f = f_I.
- If the previous attempt was rejected or failed in the linear solve, f = min(f, 1).
- Then set h_acc ← h and err_acc ← max(1e-2, err).
- If err = 0, f = 5.
- On a clipped output-landing sample, update (h_acc, err_acc) only for informative samples (trial ≥ 0.5 of the request), mirroring the PI history rule.

**On a rejected step:** keep the production rule clamp(0.9·err^(−1/5), 0.2, 0.9). A non-finite result or a linear-solve failure gives 0.2. Do not update h_acc or err_acc.

**Safety stays 0.9** (set-point 0.59). PRED requires tolerance-coupled stage targets (INO θ ≤ 0.002, or proj with a coupled target). Never pair it with uniform relative forcing.

## Proposed Rust node predictions

Arms: I, I725, PRED and PRED+cap, plus H211b+cap, PI34+cap and the automatic h0. Ladders: 17 points to 1e-7, and to 1e-10 for HIRES and Robertson. Use 4 seeds and both rules.

Replica predictions, as attempt ratios:

| problem | rule | 1e-3 | 1e-4 | 1e-5 | 1e-6 | 3e-7 |
|---|---|---|---|---|---|---|
| vdP | R | 0.68 | 0.72 | 0.76 | 0.81 | 0.84 |

| problem | gm | worst E |
|---|---|---|
| HIRES | 0.93 | ≤ 0.99 |
| Bruss-50 | 0.93 | |
| Robertson | 1.01 | ≤ 1.06 |
| Bruss-160 matrix-free JVP (PRED+cap) | 0.91 | |

Rejection fraction: vdP 28.6% → 7.2%, Bruss-50 13.8% → 6.0%.

Error/rtol shift within 0.77–1.20×: vdP 1.71 → 1.69, HIRES 0.13 → 0.10, Bruss-50 1.04 → 1.25, Robertson 0.55 → 0.59.

## Threats to validity

1. The evidence is replica-only.
   - The I baseline is faithful (27 of 28 dense cells exact; matrix-free exact on Bruss and vdP), but the PRED, I725 and other arms have no Rust counters.
   - The matrix-free replica uses a dense W·v product and CGS2 rather than the production JVP and MGS2; the counts are the same but the round-off is not.
   - The HIRES matrix-free replica is 7% high in JVPs (17–20% in inner products).
2. Only endpoint error was measured, not max-grid or dense-output error. vdP phase error could change the picture under a max-grid metric.
3. Matched accuracy is protocol-sensitive.
   - A global linear frontier over 4–7 decades is crude at the ends; the windowed fit differs by up to ±10% at the extreme E for HIRES and Robertson.
   - The cheapest-run rule shows single-ladder outliers, with seed ranges up to 0.53–0.85.
   - Matrix-free frontiers are nearly flat, which makes matrix-free ratios level comparisons.
   - E points at ladder edges are extrapolated (flagged x).
4. The h0 seeds are a sensitivity band, not independent replicates. The noise arm and the column-cap test used 2 seeds and 1 seed.
5. The problem set is narrow: four benchmarks, rotating-nonnormal n = 96 (dense only), and Bruss-50/160 matrix-free. There are no DAE, long oscillatory or nonnormal budget-failing matrix-free cases, and corpus semilinear rows were not run.
6. The cap's matrix-free value comes from maxit-200 budget failures. With maxit 2000 (KRY-BUDGET-PREDICT) it may vanish.
7. The flop models are assumptions (10 or 100 flops per component per JVP). The (c) lead depends on cheap JVPs. No wall time was measured.
8. INO here is the uncertified form with err_prev scaling, and PRED shifts the err_prev distribution. The interaction was tested closed loop only on Bruss-50/160 and HIRES.
9. Robertson's worst E for PRED (1.03; seed maximum 1.06–1.07) sits near the judge's 1.06 line and needs a preregistered tight-E cell.

## Key files

All in `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl/`:
- **Code:** `core.py`, `run_dense.py`, `run_mf.py`, `analyze.py`.
- **Raw runs:** `dense_runs.jsonl`, `mf_runs.jsonl`.
- **Fidelity:** `fidelity_dense.json`, `fidelity_mf.json`.
- **Reports:** `report_dense_att.txt`, `report_dense_flops.txt`, `compact.txt`, `factorial.txt`, `report_mf_b50proj.txt`, `report_mf_b160proj.txt`, `report_noise.txt`, `noise_diag.log`, `report_rot.txt`, `judge_eval.txt`, `h0_null.log`, `catastrophe.log`, `kcost_proj_160.log`, `nulls/gfb_I.log`, `nulls/gfb_PRED.log`.
- **Reference:** `ref_bruss160.npy`.