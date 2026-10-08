**Probe B1: the integrated Tier-1 stack, closed loop**

This is an exploratory pilot from a Python replica, meant to shape future preregistered Rust nodes. It is not ledger authority. The worktree `/home/user/wt-speed` (a49f7e4) was only read, and no Rust was built.

## Decision
- **PROMOTE stack S to one preregistered Rust node, T1-A + T1-B + T1-C combined.**
  - S is: duplicate residual removed + in-cycle projected stop (proj) + A1's coupled target + the predictive controller with post-rejection cap (PRED+cap) + maxit 2000 with stagnation guard.
  - S leaves INO out.
  - **Accuracy:** S passes the solve-contamination gates. Its error versus the exact-solve twin under the same controller (LUP) is at most 1.37x, and contamination versus LUP is at most 0.454.
  - **Speed at matched accuracy:** S is 0.26x base JVPs on Bruss-50 (0.10x flops), 0.43x on Bruss-160, 0.71x on HIRES and 0.62x on vdP.
- **CLOSE INO-FORCE-ABS as a stacked layer.**
  - On top of A1's target it saves 0–2% JVPs (Bruss 0.98–1.00, small problems 0.98).
  - It breaks the accuracy gate:
    - θ = 0.001: HIRES 3.2e-10 at 3.06x base and HIRES 1e-10 at 1.86x.
    - θ = 0.002: six HIRES cells, up to 8.70x.
  - A1's error-per-unit-step (EPUS) target already does its job. This trips the judge's kill rule, "incremental factor > 0.9 on both Brusselators".
- **PRED: promote only under a matched-accuracy gate.**
  - S fails the equal-rtol 1.5x gate in 8 cells.
  - Exact solves with the same controller (LUP) reproduce every one of those failures, so they come from the controller, not from stage contamination.
- **maxit 2000 + guard: promote as a robustness layer.**
  - It is bit-identical wherever the 200-column budget never binds (all ladder cells except Bruss-160 at loose rtol and Bruss-300).
  - It removes all linear failures on Bruss-300.
  - The guard never fired in any run.
- **Finite-difference JVPs: HOLD.**
  - As calibrated, every arm livelocks with FD JVPs, production included.
  - A 1e-8 floor fixes it at rtol ≥ 1e-9.
  - At rtol ≤ 3e-10 every FD arm is 4–8x base.

## Question
Does the Tier-1 stack deliver a realistic combined factor once it runs closed loop, counts admitted cost, and is measured at matched accuracy against the strongest cheap rivals? Which layer contributes what? Does PRED misbehave with loose stage targets? Does any cell break the 1.5x error gate?

## Method
**Replica.** `stack.py` is A1's MGS2 replica (`rep_a1.py`, unchanged) extended with:
- the PRED+cap and I725 controllers, copied from `probe/ctrl/core.py`;
- stacked WRMS targets;
- maxit and the stagnation guard, with an uncounted shadow run that classifies false aborts;
- FD JVPs with σ = √ε(1+‖y‖₂)/‖v‖₂ and cached f0 (the formula in `semilinear_f033_ablation.rs`);
- a noise diagnostic that runs an uncounted exact-LU stage chain on every attempt.

The loop is fully closed: stage right-hand sides come from the inexact solves, linear failures and non-finite stages become rejections with h × 0.2, and every solve is accepted on a true residual.

**Fidelity against Rust (SPD07 BASE.json, arm 0):**
- 12 of 14 cells match exactly in attempts, accepted, rejected, JVPs, iterations, matvecs, orthogonalization dots and RHS.
- Robertson JVPs are 1523 vs 1528 (1e-6) and 3622 vs 3626 (1e-8); step counts are exact.
- Rust's `orthogonalization_vector_updates` equals the replica's orthogonalization updates plus true residuals exactly. Example: HIRES 1e-6 gives 93464 + 1669 = 95133.
- Arm 1 has exactly the same trajectory as arm 0, at 8.0 fewer JVPs per attempt (checked on HIRES and Bruss-50 at 1e-6).

**Ladders:**
- HIRES, Robertson, vdP, PR-forced, quad-4, Bruss-50: rtol 1e-3 to 1e-10 in half-decades (15 rungs).
- Bruss-160: 1e-4 to 1e-8 (9 rungs). Bruss-300: 1e-4 to 1e-7 (7 rungs).
- h0 = 1e-6. The controller comparison also uses h0 = 1e-5 and 1e-4 on HIRES, Robertson, vdP and Bruss-50.

**References:**
- HIRES, Robertson, vdP, Bruss-50: A1's 80-bit RODAS5P at rtol 1e-15.
- Bruss-160 and Bruss-300: scipy Radau with rtol = atol = 1e-13. The 1e-12 vs 1e-13 difference is 5.2e-13 and 3.5e-13. On Bruss-50, Radau 1e-13 is within 2.2e-14 of the 80-bit reference.
- PR-forced and quad-4: exact solutions.

**Error metric:** componentwise max |y−ref|/max(|ref|, 1e-10).

**Matched accuracy:**
- Regression frontier: log work against log error over the ladder, evaluated at decade values of E (half-decades on Bruss-160/300). "x" marks an extrapolated point.
- Harness cheapest-run rule: the cheapest run whose error is at most E.

**Flop model.** Each operator application costs, in flops:

| problem | n | analytic JVP | scaled form (D W D⁻¹) | FD JVP | FD scaled | RHS |
|---|---|---|---|---|---|---|
| Bruss-50 | 100 | 992 | 1192 | 1600 | 1800 | 1000 |
| Bruss-160 | 320 | 3192 | 3832 | 5120 | 5760 | 3200 |
| Bruss-300 | 600 | 5992 | 7192 | 9600 | 10800 | 6000 |
| HIRES | 8 | 66 | 82 | 93 | 109 | 45 |
| Robertson | 3 | 20 | 26 | 34 | 40 | 16 |
| vdP | 2 | 10 | 14 | 19 | 23 | 7 |
| PR-forced | 1 | 4 | 6 | 31 | 33 | 25 |
| quad-4 | 4 | 52 | 60 | 40 | 48 | 16 |

- Analytic JVP is 2·nnzJ + 2n. FD JVP is RHS + 6n. The scaled form adds 2n per application.
- Each dot, axpy (vector update) and norm costs 2n; each scaling costs n; each stage-assembly axpy costs 2n.
- Dense LU costs 2n³/3 and each LU solve 2n². Preconditioner applications and LU factorizations are 0 in every matrix-free arm.

## Arms
**The stack, cumulative:**

| arm | definition |
|---|---|
| A0 | SPD07 base |
| A1 | A0 with the duplicate residual removed (derived from A0's counters) |
| A2 | proj-stop at the L2 target max(g·1e-14, 1e-10‖b‖) |
| A3 | A2 + A1's coupled target (wE0.2) |
| A4a / A4b | A3 + INO as a per-step floor: θₙ = max(0.2·h/T, θ)·min(1, ê/0.5)^1.2, θ = 0.001 / 0.002; same allocation and U8 cap |
| A5a / A5b | A4 + PRED+cap |
| A6 | A5a + maxit 2000 + guard. The guard aborts at q = ‖r_k‖/‖r_{k−1}‖ ≥ 0.98, or when the predicted columns exceed 2000. |
| S | A6 without INO (the recommended stack) |

**Rivals:**
- R3: the L2 coupling `l2c`.
- R5 / R3U / SU: uniform safety factor 0.725 on the I controller (I725), applied to A4a / A3 / S.
- Rbig: base with maxit 2000.

**Attribution controls:**
- C4a / C4b: INO alone.
- C0P / C2P / C3P: PRED added to A0 / A2 / A3.
- C0G / C3G: the guard added to A0 / A3.
- C6ng: A6 without the guard.
- LU / LUP: exact-solve twins under I / PRED+cap.
- X02I/P (per-step 0.02) and XrI/P (relative 1e-4): deliberately loose targets.

**FD (arm 7, Bruss-50):**
- As calibrated: A0f, A2f, A3f, A5af.
- With an FD-aware floor (in-cycle floor 1e-8‖Db‖, stall acceptance at 4√ε of the backward-error scale): A3fg, A5afg, Sfg8 (floor 1e-9: Sfg9; stall only: Sfs).
- A5ag: the same floor with exact JVPs.
- Production form with a stall backstop: A0f8s and A2f8s (rtol_lin 1e-8).

## Results

### 1. Equal rtol: JVP / err÷rtol
SPD07 cells plus the extra cells.

| cell | A0 | A1 | A2 | A3 | A4a | A4b | A5a | A6 | S | R3 |
|---|---|---|---|---|---|---|---|---|---|---|
| Bruss-50 1e-6 | 30666 / 2.14 | 29930 / 2.14 | 14331 / 2.14 | 9542 / 2.14 | 9515 / 2.14 | 9427 / 2.14 | 8911 / 1.81 | 8911 / 1.81 | 8937 / 1.81 | 10909 / 2.14 |
| Bruss-50 1e-8 | 64561 / 2.16 | 63017 / 2.16 | 19086 / 2.16 | 16673 / 2.16 | 16490 / 2.16 | 15966 / 2.16 | 16334 / 1.77 | 16334 / 1.77 | 16530 / 1.77 | 17898 / 2.16 |
| Bruss-160 1e-6 | 56906 / 2.14 | 56170 / 2.14 | 41593 / 2.14 | 25481 / 2.13 | 25444 / 2.13 | 25205 / 2.13 | 23988 / 1.83 | 23988 / 1.83 | 24027 / 1.83 | 31561 / 2.14 |
| Bruss-160 1e-8 | 78255 / 2.17 | 76711 / 2.17 | 48393 / 2.17 | 40042 / 2.16 | 39405 / 2.16 | 38171 / 2.16 | 39159 / 1.78 | 39159 / 1.78 | 39789 / 1.78 | 45382 / 2.17 |
| HIRES 1e-6 | 15032 / .263 | 13352 / .263 | 11368 / .264 | 11147 / .251 | 10637 / .256 | 10518 / .252 | 10897 / .193 | 10897 / .193 | 11420 / .185 | 11066 / .264 |
| HIRES 1e-8 | 56376 / .0915 | 50104 / .0915 | 33747 / .235 | 39974 / .100 | 37308 / .114 | 36354 / .237 | 37594 / .0871 | 37594 / .0871 | 40302 / .0766 | 37583 / .105 |
| Robertson 1e-6 | 1523 / 1.10 | 1163 / 1.10 | 1045 / 1.10 | 1073 / 1.10 | 1053 / 1.10 | 1053 / 1.10 | 1074 / 1.30 | 1074 / 1.30 | 1096 / 1.30 | 1034 / 1.10 |
| Robertson 1e-8 | 3622 / .608 | 2798 / .608 | 2378 / .608 | 2464 / .622 | 2414 / .622 | 2409 / .622 | 2364 / .631 | 2364 / .631 | 2415 / .631 | 2397 / .622 |
| vdP 1e-6 | 14996 / 1.56 | 11244 / 1.56 | 11183 / 1.56 | 11269 / 1.56 | 11152 / 1.56 | 11142 / 1.56 | 9025 / 1.78 | 9025 / 1.78 | 9146 / 1.77 | 11003 / 1.56 |
| vdP 1e-8 | 30292 / 4.43 | 22716 / 4.43 | 22525 / 4.43 | 22672 / 4.43 | 22417 / 4.44 | 22408 / 4.44 | 22803 / 2.47 | 22803 / 2.47 | 23060 / 2.46 | 22148 / 1.99 |
| PR-forced 1e-6 | 262 / .006 | 166 / .006 | 166 / .006 | 190 / .006 | 186 / .006 | 186 / .006 | 186 / .006 | 186 / .006 | 190 / .006 | 156 / .006 |
| PR-forced 1e-8 | 430 / .104 | 278 / .104 | 278 / .104 | 304 / .104 | 300 / .104 | 300 / .104 | 316 / .0008 | 316 / .0008 | 320 / .0008 | 272 / .104 |
| quad-4 1e-6 | 579 / .160 | 475 / .160 | 412 / .160 | 459 / .155 | 450 / .155 | 450 / .155 | 450 / .155 | 450 / .155 | 459 / .155 | 353 / .161 |
| quad-4 1e-8 | 963 / .219 | 795 / .219 | 708 / .219 | 734 / .205 | 728 / .204 | 728 / .204 | 728 / .204 | 728 / .204 | 734 / .205 | 659 / .219 |
| HIRES 1e-9 | 108216 / .0855 | 96184 / .0855 | 57062 / 1.42 | 74478 / .081 | 68040 / .0977 | 66224 / .659 | 68319 / .0992 | 68319 / .0992 | 74801 / .0853 | 69221 / .0934 |
| HIRES 1e-10 | 205408 / .0835 | 182576 / .0835 | 99516 / 4.20 | 137980 / .0861 | 123259 / .155 | 120454 / .615 | 123472 / .156 | 123472 / .156 | 138253 / .0791 | 126828 / .094 |
| Robertson 1e-9 | 7131 / .395 | 5571 / .395 | 4527 / .395 | 4671 / .399 | 4585 / .399 | 4578 / .399 | 4609 / .438 | 4609 / .438 | 4695 / .438 | 4575 / .399 |
| Robertson 1e-10 | 16927 / .355 | 13367 / .355 | 10359 / 1.43 | 10634 / .190 | 10515 / .190 | 10504 / .190 | 10585 / 1.80 | 10585 / 1.80 | 10705 / 1.80 | 10510 / .190 |
| Bruss-160 1e-4 | 63722 / .262 | 63181 / .262 | 51657 / .262 | 18732 / .421 | 18694 / .421 | 18670 / .421 | 16463 / .616 | 16463 / .616 | 16499 / .616 | 23991 / .426 |
| Bruss-300 1e-4 | 218341 / .0576 | 216685 / .0576 | 187356 / .0576 | 53587 / .156 | 53528 / .156 | 53334 / .156 | 45103 / .158 | 31496 / .613 | 31562 / .613 | 53980 / .134 |
| Bruss-300 1e-6 | 180076 / .952 | 178637 / .952 | 151584 / .952 | 47652 / 2.12 | 47553 / 2.12 | 47055 / 2.12 | 44764 / 1.83 | 44764 / 1.83 | 44864 / 1.83 | 60964 / 2.14 |

**Attempts / accepted / rejected / linear failures:**
- Every cell except vdP and the budget-binding cells changes by at most ±4 attempts.
- vdP 1e-6: A0 469/351/118/0, S 381/365/16/0. vdP 1e-8: 947/915/32/0 against 963/952/11/0.
- Bruss-160 1e-4: A0 73/52/21/13, A3 52/42/10/0, S 48/43/5/0.
- Bruss-300 1e-4: A0 264/142/122/118, A3 109/70/39/36, A5a 83/65/18/15, A6 and S 48/43/5/0, Rbig 52/42/10/0.
- Bruss-300 1e-6: A0 195/129/66/57; A3 and S have 0 failures.

**RHS / dots / axpys, A0 → S:**

| cell | A0 | S |
|---|---|---|
| Bruss-50 1e-6 | 726 / 1,197,200 / 1,228,596 | 685 / 120,956 / 130,581 |
| Bruss-160 1e-6 | 726 / 2,246,800 / 2,305,076 | 700 / 751,282 / 776,209 |
| HIRES 1e-10 | 22,826 / 1,278,032 / 1,506,262 | 22,868 / 772,100 / 933,227 |
| Bruss-300 1e-4 | 1,652 / 8,667,400 / 8,891,026 | 379 / 1,149,072 / 1,181,592 |

**Mflop, A0 → S:**
- Bruss-50: 1e-6 527 → 65.6; 1e-8 1110 → 104.
- Bruss-160: 1e-6 3160 → 1100; 1e-8 4320 → 1620.
- HIRES: 1e-8 18.5 → 13.8; 1e-10 67.4 → 46.4.
- vdP 1e-6: 0.655 → 0.514.
- Bruss-300: 1e-4 22,800 → 3,090; 1e-6 18,800 → 4,300.

All arms and cells are in `report_tables.md` and `tables.txt`, table T1.

### 2. Combined factor at matched accuracy
Each cell is the frontier geometric mean [range over E] / cheapest-run geometric mean. "(kx)" means k extrapolated E points were excluded.

| problem | S/A0 JVP | S/A0 flops | S/A1 JVP | S/A2 JVP | S/A2 flops | S vs strongest rival, JVP |
|---|---|---|---|---|---|---|
| Bruss-50 | 0.26 [0.23–0.29] / 0.27 | 0.10 [0.08–0.14] / 0.10 | 0.27 / 0.27 | 0.75 [0.55–1.04] / 0.80 | 0.64 [0.37–1.12] / 0.74 | R3: 0.88 / 0.90; SU: 1.00 / 1.05 |
| Bruss-160 | 0.43 [0.36–0.52] / 0.46 | 0.35 / 0.37 | 0.44 / 0.47 | 0.61 [0.46–0.82] / 0.66 | 0.56 / 0.61 | Rbig: 0.44 / 0.46; R3: 0.78 / 0.81; SU: 1.01 / 1.06 |
| Bruss-300 | 0.29 [0.20–0.43] / 0.49* | 0.27 / 0.45* | 0.30 / 0.50 | 0.35 / 0.61 | 0.34 / 0.60 | Rbig: 0.49 / 0.51 (flops 0.45 / 0.46); R3: 0.74 / 0.78; SU: 1.02 / 1.06 |
| HIRES | 0.71 [0.69–0.73] / 0.73 | 0.74 / 0.77 | 0.80 / 0.82 | 0.88 / 0.91 (2x) | 0.95 / 0.99 | R3: 1.00 / 1.00; SU: 0.94 / 0.97 |
| Robertson | 0.69 / 0.65 | 0.84 / 0.78 | 0.90 / 0.84 | 1.02 / 1.07 (1x) | 1.09 / 1.14 | R3: 1.07 / 1.05; SU: 0.98 / 0.91 |
| vdP | 0.62 [0.53–0.73] / 0.64 | 0.80 / 0.83 | 0.83 / 0.86 | 0.83 / 0.86 | 0.89 / 0.93 | R3: 0.88 / 0.91; SU: 0.88 / 0.88 |
| PR-forced | 0.74 / 0.71 (2x) | 1.03 / 0.97 | 1.16 / 1.11 | 1.16 / 1.11 | 1.14 / 1.08 | R3: 1.24 / 1.17 |
| quad-4 | 0.78 / 0.77 | 0.88 / 0.87 | 0.94 / 0.94 | 1.08 / 1.07 | 1.13 / 1.11 | R3: 1.26 / 1.22 |

\* On Bruss-300 the A0 frontier is not valid. A0's work falls as rtol tightens (218k JVPs at 1e-4, 102k at 1e-7) because of its failure cascades. Use the cheapest-run rule and the Rbig rival there.

**Realistic factor per class:**
- **PDE-like, matrix-free, no declared structure:**
  - Versus base: Bruss-50 0.26x JVP / 0.10x flops; Bruss-160 0.43x / 0.35x; Bruss-300 0.49x / 0.45x on the cheapest-run rule.
  - Versus A2 (arm 2): 0.75 / 0.61 / 0.61 in JVP; 0.64 / 0.56 / 0.60 in flops.
- **Small n (HIRES, Robertson, vdP, PR-forced, quad-4):**
  - Versus base: 0.62–0.78x JVP and 0.74–1.03x flops.
  - Versus A2: 0.83–1.16x JVP; HIRES, vdP and Robertson come to 0.88, 0.83 and 1.02.
  - Net of the duplicate fix (S/A1): 0.80–1.16x.
  - A2 itself is inadmissible on HIRES and Robertson at tight rtol (section 4).
- **These numbers match the judge's forecast:**
  - At equal rtol, S is Bruss-50 0.291x and 0.256x at 1e-6/1e-8 (forecast 0.30–0.33 and 0.24–0.26).
  - Bruss-160 is 0.42 / 0.51 (forecast 0.45–0.50).
  - vdP 0.62, Robertson 0.69 and HIRES 0.71 are at or slightly above the top of the forecast ranges.
- **Context that limits the scope:**
  - Where the band is declared, banded direct beats S by 15–27x (Bruss-50), 72–132x (Bruss-160) and 215–273x (Bruss-300) in flops.
  - For n ≤ 8, dense LU is 2–7x cheaper than S.
  - The matrix-free stack therefore applies only where no structure is declared.

### 3. Attribution
Geometric mean of matched-accuracy frontier ratios, JVP. The flops ratio follows in parentheses where it differs materially.

| problem | A1/A0 duplicate | A2/A1 proj | A3/A2 coupled target | A4a/A3 INO | A5a/A4a PRED | A6/A5a guard | total S/A0 |
|---|---|---|---|---|---|---|---|
| Bruss-50 | 0.98 (1.00) | 0.35 (0.16) | 0.79 (0.67) | 0.98 | 0.95 | 1.00 | 0.26 (0.10) |
| Bruss-160 | 0.99 | 0.72 (0.61) | 0.65 (0.60) | 0.99 | 0.94 | 1.00 | 0.43 (0.35) |
| Bruss-300 | 0.99 | 0.84 | 0.38 | 1.00 | 0.94 | 0.98 | 0.29* |
| HIRES | 0.89 (0.97) | 0.91 (0.82) | 0.92 (1.00) | 0.98 | 0.96 | 1.00 | 0.71 (0.74) |
| Robertson | 0.77 (0.90) | 0.88 | 1.00 (1.07) | 0.98 | 1.03 | 1.00 | 0.69 (0.84) |
| vdP | 0.75 (0.90) | 0.99 | 1.01 (1.08) | 0.99 | 0.83 | 1.00 | 0.62 (0.80) |
| PR-forced | 0.64 (0.90) | 1.00 | 1.10 | 0.99 | 1.04 | 1.00 | 0.74 (1.03) |
| quad-4 | 0.82 (0.91) | 0.87 | 1.08 (1.13) | 0.99 | 1.00 | 1.00 | 0.78 (0.88) |

- **Duplicate fix:** saves exactly 8.0 JVPs per attempt.
- **Coupled target versus the R3 rival (A3/R3):** Bruss-50 0.92, Bruss-160 0.84, Bruss-300 0.81. On small problems R3 is cheaper (HIRES 1.03, PR-forced 1.14, quad-4 1.26), but A1 showed R3 fails the fixed-step ladders.
- **INO alone over proj (C4a/A2):** Bruss-50 0.82, Bruss-160 0.70. It is dominated by A3 on every problem.
- **The budget layer overlaps fully with the coupled target:**
  - A3 alone removes all 13 Bruss-160 1e-4 failures (18,732 JVPs against Rbig's 48,304).
  - On Bruss-300 1e-4, A3 still has 36 failures. Adding the guard leaves 0 (C3G: 35,859 JVPs; A6: 31,496).
  - At matched accuracy the guard is worth only 0.98–1.00, because failure cascades over-resolve the solution.
  - Across all ladders the guard fired 0 times on q, 0 times on overrun, with 0 false aborts.
- **PRED versus the uniform set-point rival (C3P/R3U), median over 3 h0 seeds, frontier / cheapest:**
  - HIRES 0.89 / 0.91, vdP 0.88 / 0.88, Robertson 0.98 / 0.91.
  - Bruss-50 1.00 / 1.05 against PRED. Bruss-160 and Bruss-300 (S/SU, single seed): 1.01 / 1.06 and 1.02 / 1.06.
- **PRED on top of A3 (C3P/A3) across seeds:** HIRES 0.96, Robertson 1.02, vdP 0.82, Bruss-50 0.95.
- **Rejections over the ladder (A3 → C3P):** vdP 8.6% → 1.9–2.1%, HIRES 1.5% → 0.6%, Bruss-50 4.1% → 1.7%.

### 4. Accuracy gate
All ladder rungs, 106 cells per arm.

| arm | error ÷ failure-free base (Rbig, else A0): max, cells > 1.5x | error ÷ exact-solve twin with the same controller: max, cells > 1.5x | contamination ÷ error of A0 (cells with the same step sequence) |
|---|---|---|---|
| A2 | 50.27, 6 (HIRES 1e-8 to 1e-10; Robertson 1e-10 4.04) | 45.5, 7 | 11.4 |
| A3 | 1.16, 0 | 1.18, 0 | 0.349 |
| R3 | 1.14, 0 | 1.09, 0 | 0.138 |
| A4a | 3.06, 2 (HIRES 3.2e-10, 1e-10) | 3.02, 2 | 2.59 |
| A4b | 8.70, 6 (HIRES 3.2e-8 to 1e-10) | 8.60, 6 | 8.23 |
| A5a / A6 | 101.8, 10 | 3.09, 3 (HIRES 3.2e-10 / 1e-10, vdP 3.2e-4 1.58) | 2.61 |
| S | 101.8, 8 | **1.37, 0** | **0.454** |
| LUP | 101.8, 8 (identical cells to S) | 1.00 | 0 |

- **Which S cells fail the equal-rtol gate:**
  - Bruss-50 1e-3: 2.16. HIRES 1e-3: 1.70. HIRES 1e-7: 4.22 (0.58 rtol against 0.138).
  - Robertson 1e-10: 5.07 (1.80 rtol against 0.355).
  - Four PR-forced cells, 8.6–101.8x. There the base error is only 0.0002–0.02 rtol, so the ratio is meaningless.
  - All are reproduced by LUP. They are controller set-point or trajectory effects, so PRED has to be gated at matched accuracy.
- **Budget-binding cells:**
  - Measured against A0, every failure-free arm, including exact LU, exceeds 1.5x: LU is 1.62x on Bruss-160 1e-4 and 2.2–7.4x on Bruss-300 1e-4 to 1e-6.
  - Base's failure cascades over-resolve the solution, so the equal-rtol gate is ill-posed there. Measured against Rbig, A3 is at most 1.00.
- **The base arm itself:**
  - A0 is 1.87x LU on Robertson 1e-10 and 1.64x on 3.2e-10.
  - At 1e-10 the absolute floor g·1e-14 exceeds the outer atol of 1e-14.
- **Fixed-step contract ladders** (`ladders_b1.py`, against direct LU):
  - A3, A4a, A4b and C4a are all within 1.00x of LU on diagpr128 (k = 3–5) and semilin128 (k = 3–5).
  - On semilin64 they are within 0.70–1.17x, where the k = 7–8 rungs sit at the round-off floor (LU error 6.6e-15 and 4.5e-15).

### 5. Does PRED interact badly with loose targets?
Noise is |err/err_exact − 1| on attempts with err_exact ≥ 0.05; "flips" counts accept/reject decisions that change.

**Absolute WRMS targets: no.**
- For A3, A4b and even X02 (per-step budget 0.02), maximum noise is ≤ 3.0e-3 on all 48 cells: HIRES 1e-4 to 1e-10, Robertson 1e-8/1e-10, vdP 1e-4 to 1e-8, Bruss-50 1e-4 to 1e-8.
- Flips are 0. PRED's assumption needs noise below 10%.
- PRED's gain is unchanged on a loose absolute target. X02P/X02I is vdP 0.81, Bruss-50 0.95, HIRES 0.99, matching C3P/A3.

**Production and proj targets at Robertson 1e-10 (floor-dominated): yes.**

| arm | rejections | attempts with noise > 10% |
|---|---|---|
| A0 / A2 (I controller) | 4 | 0.2% |
| C0P / C2P (PRED) | 42 | 7.4% |
| A3 / C3P | 3 / 4 | 0% (max noise 5.6e-6) |

**Relative forcing 1e-4 (negative control): yes.**
- Noise is O(1) and decisions flip: HIRES 1e-4 46/42 flips (XrI/XrP), Bruss-50 1e-6 25/18.
- HIRES at rtol ≤ 1e-6 hits the attempt cap.
- PRED is worse here: the XrP/XrI frontier ratio on HIRES is 1.25.

### 6. Finite-difference JVPs (arm 7, Bruss-50)

**As calibrated, every FD arm livelocks.** At the 3000-attempt cap:

| arm | 1e-6 | 1e-8 |
|---|---|---|
| A0f | 1498 linear failures, 1503 accepted | same |
| A2f | 1498, 1503 | same |
| A3f | 1499, 1502 | same |
| A5af | 1080, 1921 | 1079, 1922 |

- **Why:** FD stagnates GMRES at about 1.5e-9·hg‖J‖ relative to ‖Db‖. Measured: 4.3e-11 at hg‖J‖ = 0.045, up to 1.7e-8 at 13. That is far above A1's 16ε guard, its 1024ε stall threshold and production's 1e-10 target.
- **The FD "true residual" stops certifying anything in A5af:**
  - exact residual ÷ FD residual: median 875, max 1.9e5;
  - true ÷ projected > 10 on 3.3% of exits, which trips the judge's FD kill rule (more than 1%).

**With the FD-aware floor (Sfg8: in-cycle floor max(eps_i√n, 1e-8‖Db‖), stall 4√ε):**
- 0 failures.
- True ÷ projected > 10 on 0% of exits (max 8.7). Exact ÷ FD residual at most 1.39.
- Errors: 1e-3 0.173 (= LUP), 1e-6 1.80, 1e-8 1.57 rtol.
- At matched accuracy in the FD flop model: 0.07 / 0.07 of A0f8s and 0.71 / 0.78 of A2f8s.

**All FD arms fail at rtol ≤ 3e-10, whatever the floor:**

| arm | err÷rtol at 1e-10 |
|---|---|
| Sfg8 | 8.39 |
| Sfg9 | 7.99 |
| Sfs | 8.08 |
| A0f8s | 8.01 |
| A2f8s | 9.36 |
| exact S | 1.57 |

- At 3.2e-10, Sfg8 is 2.08x base and Sfg9 1.68x.
- Without any floor (Sfs), true ÷ projected exceeds 10 on 27–72% of exits.

## The rule I recommend: stack S
`run.py` ARMS['S'], from `stack.py` plus A1's `coupled_target.py`.

1. **Stage solves:** GMRES(40) with zero start and MGS2, on D W D⁻¹ with right-hand side D·b, where D = 1/(atol + rtol·|yₙ|).
2. **Stop and confirm:** stop inside the cycle when the Givens residual is ≤ thr_i = max(eps_i·√n, 16ε·‖Db‖₂). Confirm with one true residual ‖D(b − W U)‖₂ ≤ thr_i.
3. **No duplicate final residual.**
4. **Stage targets:**
   - eps_i = θₙ / (8·max(τ_y,i, τ_e,i)).
   - θₙ = 0.2·(hₙ/T)·min(1, ê/0.5)^1.2, where ê is the last accepted err (1e-6 before the first acceptance).
   - eps_8 ≤ 0.1·(0.9/5)^5.
5. **Stall rule:** after a cycle with ‖r_k‖ > 0.25·‖r_{k−1}‖, accept if ‖r_k‖ ≤ 1024ε·(‖Db‖ + ‖DU‖ + ‖D(U − WU)‖).
6. **Budget:** maxit 2000 columns. At each restart, abort as a linear failure (reject, h × 0.2) if q ≥ 0.98, or if total + 40·⌈log(thr/‖r_k‖)/log q⌉ > 2000.
7. **Controller, PRED+cap:**
   - h₊ = h·min{0.9e^(−1/5), 0.9·(h/h_prev)·(max(1e-2, e_prev)/e²)^(1/5)}, clamped to [0.2, 5].
   - The factor is ≤ 1 on the first acceptance after a rejection.
   - On rejection, h × clamp(0.9e^(−1/5), 0.2, 0.9). On linear failure, h × 0.2.
8. **FD-JVP variant (exploratory):** raise the 16ε floor to 1e-8 and the stall constant to 4√ε. It is valid at rtol ≥ 1e-9.

**Gates for the Rust node:**
- matched-accuracy frontier **and** cheapest-run rule;
- solve-contamination against an exact-solve twin with the same controller, at most 0.5;
- error ÷ exact-solve twin with the same controller at most 1.5 in every cell;
- the fixed-step ladders against direct LU;
- rivals: R3 for the target, I725 for PRED with multiple h0 seeds, Rbig for the budget, and banded direct wherever a band is declared.

## Threats to validity
1. **Replica, not Rust.** Base is exact on 12 of 14 SPD07 cells, and Robertson is within 0.3%. The scaled operator, PRED, the guard and FD have no Rust counterpart yet. Every JVP is exact except in the FD arms.
2. **Narrow evidence.** Large n means 1-D Brusselator only, where banded direct wins by 15–270x in flops. There are no 2-D, nonnormal-advection, DAE or corpus-v2 cells.
3. **The guard was never exercised.** It fired 0 times, so false aborts on solves that plateau and then converge remain unmeasured.
4. **Matched-accuracy protocol limits:**
   - Bruss-300's base frontier is invalid (failure cascades).
   - PR-forced errors are noise-dominated (0.0002–0.2 rtol).
   - A few E values are extrapolated and flagged.
   - Bruss-160 and Bruss-300 use one h0 seed; the controller seeds cover only 4 problems.
5. **INO was implemented as a per-step floor**, max(Θh/T, θ). A min() combination would only tighten A3's targets, so it cannot save work over A3 by construction.
6. **Flop model omissions:** Givens rotations, small least-squares solves and memory traffic are not counted. The FD JVP cost is RHS + 6n; the in-repo example would cost 2 RHS per JVP.
7. **References:** Radau at 1e-13 (difference between the two Radau runs ≤ 5.2e-13) and A1's 80-bit runs. The SPD07 Bruss-160 reference is 1.5e-12 from Radau, which is harmless here.

Everything is in `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/stack/`:
- `README.md` — how the pieces fit
- `tables.txt` — all tables
- `report_tables.md` — every arm and cell
- `seeds.txt` — the h0-seed comparison
- `res/fidelity.json` — fidelity against Rust
- `res/noise.jsonl` — noise diagnostic
- `res/ladders_b1.jsonl` — fixed-step ladders
- `res/<problem>.jsonl` — raw runs
- `stack.py`, `run.py`, `ana.py`, `tables.py`, `pred_noise.py`, `fdfloor.py` — code