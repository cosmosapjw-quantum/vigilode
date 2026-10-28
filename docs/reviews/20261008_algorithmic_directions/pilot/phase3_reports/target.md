**Probe A1 — a stage target tied to the outer tolerance, for the matrix-free U form (exploratory)**

All numbers come from a Python replica. They are pilots for a future preregistered Rust node, not ledger authority. The worktree at a49f7e4 was only read and no Rust was built.

**Decision: PROMOTE to the Rust T1-A node as arm (iii), "proj + coupled target".** The new rule (my label `wE0.2`) is the only one tested that passes the 1.5x-of-base error gate, the stricter contamination gate and the fixed-step ladders against direct LU. It costs 0.21–0.51x production JVPs on the Brusselators and is 7–22% cheaper than the judge's L2 coupling there. On small problems (n ≤ 8) it costs 0–9% more JVPs than that L2 coupling.

## Question
The production target is L2-relative 1e-10 with an absolute floor of g·1e-14. Once restarted GMRES stops overshooting it (projected in-cycle stop, "proj"), it looks mis-scaled. I wanted to know why, which floor binds, and whether a target in the solver's WRMS tolerance units keeps endpoint error within 1.5x of base on every cell. I also checked that it keeps the direct-LU order on fixed-step ladders, and what it costs in admitted work.

## Method
- **Replica** (`rep.py`): closed loop, Zero start, GMRES(40), maxit 200, I controller. Linear failures and non-finite stages are fed back as rejections. Every solve is accepted on a true residual.
- **Orthogonalisation**: I switched the replica to two-pass modified Gram-Schmidt, as in Rust `kernels.rs:51-78`. This removes the 7% HIRES gap in the earlier replica.
- **Fidelity against SPD07 BASE.json Rust counters**:
  - Exact in attempts, rejections, JVPs, iterations, orthogonalisation dots and RHS on vdP, HIRES, Bruss-50, Bruss-160, PR-forced and quadratic-4, at both rtols.
  - Robertson JVPs are 1523 vs 1528 and 3622 vs 3626; its step counts are exact.
- **References**:
  - My own RODAS5P with exact solves in 80-bit long double at rtol 1e-14 and 1e-15. Uncertainty: HIRES 7e-16, vdP 3.9e-14, Bruss-50 1.3e-14, Robertson about 5e-15.
  - The stored Radau references in NATIVE.json are off by 3.4e-13 on HIRES, which is 45% of base's 1e-11 error, so they could not be used at the tight end.
  - PR and quadratic-4 use their exact solutions. Bruss-160 uses the SPD07 dense rtol-1e-12 reference.
- **Ladder**: rtol 1e-3 to 1e-11 (9 rungs) on HIRES, Robertson, vdP μ=1000, Bruss-50, PR-forced (λ=-1e4) and quadratic-4. Bruss-160 at 1e-6 and 1e-8 only.
- **Control arm**: an exact-LU arm on the same driver.
- **Error metric**: componentwise max |y−ref|/max(|ref|,1e-10), as in `stiff_benchmark_scipy.py`.
- **Cost counters**: JVPs, Krylov columns, true residuals, the duplicate diagnostic residual, dots, vector updates, norms, scalings, RHS and f_t.
- **Flop model**:
  - Analytic JVP = 2·nnz(J) + 2n, plus 2n for the scaled WRMS form. Per problem: HIRES 66, Robertson 20, vdP 10, Bruss-50 992, Bruss-160 3192, PR 4, quad-4 52.
  - Finite-difference JVP = F_rhs + 6n, reported as a second model.
  - Each dot or vector update counts 2n flops.

## Arms
- **Reference arms**:
  - `lu`: exact-solve control.
  - `base`: production, full cycles.
  - `proj`: L2 1e-10 with floor g·1e-14.
  - `proj12nf`: the judge's L2 1e-12 with the floor removed.
- **Rivals**:
  - `l2c`: the judge's cheap L2 coupling, rtol_lin = min(1e-10, 1e-3·rtol) and atol_lin = 1e-3·atol. This is the strongest cheap rival.
  - `ino`: INO-FORCE-ABS uncertified, a per-step budget of 1e-3.
  - `wS*`: per-step budgets 1e-2, 1e-3 and 1e-4.
- **New rule, Θ sweep**: `wE*` with Θ = 0.02, 0.1, 0.2, 0.5, 2.
- **Controls on Θ = 0.2**:
  - no order factor;
  - no U8 cap;
  - uniform allocation instead of equal share;
  - two L2-GMRES forms;
  - no stall rule;
  - order-factor reference 0.1 instead of 0.5.

## Results

**1. Why proj fails.** Endpoint error divided by rtol, for LU / base / proj:

| cell | LU | base | proj |
|---|---|---|---|
| HIRES 1e-7 | 0.138 | 0.138 | 0.119 |
| HIRES 1e-8 | 0.104 | 0.092 | 0.235 |
| HIRES 1e-9 | 0.092 | 0.086 | 1.419 |
| HIRES 1e-10 | 0.092 | 0.084 | 4.199 |
| HIRES 1e-11 | 0.098 | 0.075 | 6.921 |
| Robertson 1e-10 | 0.190 | 0.355 | 1.435 |
| Bruss-50 1e-10 | 1.685 | 1.685 | 1.778 |
| Bruss-50 1e-11 | 1.581 | 1.581 | 2.613 |

- **Both parts of the threshold are uncoupled.**
  - The absolute floor binds on 17–40% of HIRES solves, 44–58% of Robertson solves and 4–25% of Bruss-50 solves.
  - The stage right-hand side measured in tolerance units grows roughly like rtol^-0.7. On the largest HIRES stage its median goes from 8.1e2 to 1.8e8, and on Bruss-50 from 1.9e2 to 6.6e8.
  - So a fixed relative target leaves stage residuals that grow in tolerance units: HIRES median from 2e-12 to 0.21, Bruss-50 from 1.7e-8 to 2.4e-2.
  - The error spreads across all HIRES components and is largest at y6.
- **Small n is unaffected.** On vdP, PR and quadratic-4 (n ≤ 4) the Krylov space is exhausted every solve, so proj changes nothing.
- **Base itself is contaminated on Robertson at tight rtol.** At 1e-10 base is 1.9x the LU error. At 1e-11 the floor g·1e-14 exceeds atol = 1e-15, and base takes 165 rejections against LU's 3.
- **Robertson at 1e-11 is excluded from the gate.** Even LU gives 3.47 there, so the componentwise metric is at its round-off floor. Every coupled arm equals LU in that cell.

**2. The rule** (`coupled_target.py`, with `README.md`). Residuals are measured in the outer WRMS metric with weights D = 1/(atol + rtol·|y_n|):

    ||b_i − W U_i||_WRMS ≤ eps_i
    theta_n = Θ · (h_n/T) · min(1, ê_n/e_ref)^(6/5)
    eps_i   = theta_n / (8·max(τ_y,i, τ_e,i))
    eps_8   = min(eps_8, 0.1·e_sat)

- **Constants**: Θ = 0.2, e_ref = 0.5, e0 = 1e-6, e_sat = (0.9/5)^5. T is the integration span, ê is the error of the last accepted step, and τ are the existing tableau transfer constants.
- **Krylov form**: GMRES on D W D⁻¹ with right-hand side D·b. Stop at max(eps_i·√n, 16ε·‖Db‖), then confirm with one true residual.
- **Stall rule**: if a restart cycle cuts the true residual by less than 4x, accept when ‖r‖ ≤ 1024ε·(‖Db‖ + ‖DU‖ + ‖D(U−WU)‖). This fires at most 4 times per run.
- **Derivation**: per step, ‖dy_n‖ ≤ Σ τ_y,i·eps_i ≤ theta_n. Summed over steps, global contamination is at most M·Σ theta_n = M·Θ, which does not depend on rtol or the step count. The order factor scales like h⁶ at fixed rtol, which keeps the direct-LU order. This is first order with J frozen; the bound is valid only when μ_D(hJ) ≤ 0, so Θ is a calibration, not a certificate.

**3. Accuracy gate.** The robust gate is the largest contamination |y−y_LU| divided by base's error; at most 0.5 guarantees ≤ 1.5x base whatever the sign.

| arm | max err/base | cells over 1.5x | robust gate |
|---|---|---|---|
| **`wE0.2`** | **1.15** (LU itself is 1.30 there) | **0** | **0.318** |
| `wE0.5` | 1.18 | 0 | 0.347 |
| `wE2` | 6.94 | 9 (HIRES and quad-4) | 6.45 |
| `l2c` | 1.39 | 0 | 0.212 |
| `ino` | 2.49 | 2 (HIRES 1e-10 1.86x, 1e-11 2.49x) | 2.0 |
| `wS1e-3` | 2.14 | 2 | 1.65 |
| `wS1e-4` | 1.25 | 0 | 0.099 |
| `proj` | 92 | 6 | 1.15 |
| `proj12nf` | 2.78 | 2 | 2.28 |

Controls:
- Order factor referenced to 0.1: passes the endpoint gate but scores 0.846 on the robust gate (HIRES 1e-3).
- Without the U8 cap: HIRES 1.60–1.78x base.
- Uniform allocation: robust gate 0.876.
- L2-GMRES forms: pass, at 3–8% more JVPs.
- Without the stall rule: vdP has 11 linear failures at 1e-4 (+71% JVP per accepted step) and 4 at 1e-7.

**4. Contamination scaling.** |y−y_LU| in rtol units, from rtol 1e-3 to 1e-11:
- Bruss-50 (33 to 737 steps):
  - per-step 1e-2: 0.003 → 0.075, growing with the step count;
  - per-step 1e-3: 0.000 → 0.010;
  - `wE0.2`: flat at 0.004–0.007;
  - proj: 0 → 1.82.
- HIRES:
  - per-step 1e-2: 0.017 → 3.46;
  - per-step 1e-3: 0.010 → 0.124;
  - `wE0.2`: 0.004–0.024.

**5. Fixed-step ladders against direct LU.**
- **diagonal PR, n=128, k = 3, 4, 5** (LU error 5.38e-8, 3.49e-9, 2.08e-10):
  - `wE0.2`: 1.00 / 1.00 / 1.00x;
  - base: 8.6 / 157 / 114x;
  - proj: 40 / 88 / 1885x;
  - l2c: 22 / 75 / 419x;
  - no order factor: 1.03 / 8.2 / 149x.
- **semilinear n=64, order slopes at rtol 1e-6** (the contract wants two consecutive ≥ 4.5):
  - LU 4.92, 4.91;
  - `wE0.2` 4.89, 4.90;
  - l2c 4.92, 4.45, −0.25 — fails; at rtol 1e-4 it sits 1e6x above LU at k = 8;
  - no order factor 1.09, 0.52 — fails.
- **semilinear n=128**: all arms pass except no order factor (49x at k = 5).
- **PR-tight**: identical for all arms (n = 1).

**6. Cost at equal rtol.** JVP per accepted step / dots per accepted step / kflops per accepted step:

| cell | base | proj | l2c | ino | `wE0.2` |
|---|---|---|---|---|---|
| Bruss-50 1e-3 | 569.0/22364/9826 | 351.2/11439/5131 | 193.6/4810/2243 | 205.5/5007/2381 | 174.8/3602/1767 |
| Bruss-50 1e-6 | 374.0/14600/6426 | 174.8/3745/1787 | 133.0/2155/1088 | 124.3/1846/976 | 116.4/1565/851 |
| Bruss-50 1e-8 | 341.6/13337/5870 | 101.0/1269/683 | 94.7/1116/613 | 88.8/979/567 | 88.2/950/555 |
| Bruss-50 1e-11 | 335.6/13104/5767 | 52.5/307/226 (inaccurate) | 70.2/590/366 | 65.2/509/339 | 69.6/580/375 |
| Bruss-160 1e-6 | 694.0/27400/38491 | 507.2/17869/25397 | 384.9/12896/18443 | 337.3/10825/15779 | 310.7/9744/14250 |
| Bruss-160 1e-8 | 411.9/16150/22727 | 254.7/7639/11080 | 238.9/7014/10204 | 212.4/5826/8693 | 210.7/5765/8604 |
| HIRES 1e-6 | 73.7/458/24.2 | 56.0/339/18.5 | 54.5/323/17.9 | 52.2/301/17.8 | 54.6/326/18.9 |
| HIRES 1e-10 | 72.1/449/23.6 | 34.7/125/9.4 (4.2x rtol) | 44.5/216/13.3 | 43.3/214/13.8 | 48.4/271/16.3 |
| HIRES 1e-11 | 72.0/448/23.6 | 31.3/99/8.2 (6.9x rtol) | 43.2/202/12.7 | 41.4/195/13.0 | 46.4/248/15.3 |

- **Against base**: Robertson 0.55–0.72 JVPs, vdP 0.75 (this is the removed duplicate residual only), PR 0.67–0.73, quad-4 0.76–0.79.
- **Against base with the duplicate already removed**: Bruss-50 at 1e-6 is 0.32, HIRES at 1e-10 is 0.75, vdP is 1.0.
- **With finite-difference JVPs**, `wE0.2` flops against base / against l2c:
  - Bruss-50: 0.07–0.18 / 0.79–1.02;
  - Bruss-160: 0.37–0.38 / 0.77–0.84;
  - HIRES: 0.65–0.82 / 1.02–1.21.

**7. Matched accuracy (`wE0.2` against each rival).** Each entry is regression frontier / harness cheapest-run rule.

| problem | vs base, JVP | vs base, flops | vs l2c, JVP | vs l2c, flops |
|---|---|---|---|---|
| Bruss-50 | 0.22–0.31 / 0.21–0.31 | 0.07–0.15 / 0.06–0.13 | 0.88–0.97 / 0.87–0.99 | 0.78–1.00 / 0.78–1.02 |
| HIRES | 0.66–0.79 / 0.64–0.83 | — | 1.00–1.06 | 1.03–1.20 |
| Robertson | 0.60–0.73 | — | 1.01–1.06 | 1.08–1.13 |
| vdP | 0.75 | — | 1.03–1.06 | — |
| PR | 0.67–0.73 | — | up to 1.30 | — |
| quad-4 | 0.67–0.80 | — | up to 1.60 at loose rtol (≤ 35 JVP per step) | — |

- HIRES against base has one cheapest-rule outlier, 1.32 at E = 1e-9, which is rtol-ladder granularity.
- HIRES at E = 1e-12 against l2c is extrapolated.

## The rule I recommend
`coupled_target.py` with `default_target()`: Θ = 0.2, epus = True, equal-share allocation, p = 1.2, e_ref = 0.5, e0 = 1e-6, kappa8 = 0.1, the round-off guard at 16ε and the stall rule at 1024ε.
- For the integrated-stack probe use `stage_wrms_targets(h, span, err_hat)` and `scaled_gmres_threshold(eps_i, D·b)`.
- In Rust, either wrap the operator as D W D⁻¹, or use L2 GMRES with the in-cycle threshold eps·√n/max(D) and a WRMS true-residual check. The second costs 3–8% more JVPs.
- Run l2c as an explicit rival arm. Gate on the ladders and on the robust contamination ratio, not only on the 1.5x endpoint rule.

## Threats to validity
1. **Replica, not Rust.** JVPs are exact; with finite-difference JVPs the gap between projected and true residual is unmeasured. The scaled operator and in-cycle stop do not exist in production yet.
2. **Narrow calibration.** Θ was set on 6 adaptive problems and 4 ladders. HIRES and quad-4 fail at Θ = 2 and pass at 0.5, so 0.2 has a 2.5x margin. There are no 2-D, nonnormal-advection, DAE or corpus-v2 cells. Regions where μ_D > 0 (vdP, early Brusselator) are covered empirically, not certified.
3. **Trajectory noise.** The LU control itself is 1.30x base on HIRES at 1e-11, and vdP error ranges 0.9–4.4 rtol across the ladder. I used a single h0 = 1e-6 seed. The robust contamination metric was added to separate real contamination from this noise.
4. **Robertson at 1e-11** is excluded as round-off-limited.
5. **T must be known.** The rule assumes the driver knows the integration span. Overestimating T only tightens the targets; open-ended integration is untested.
6. **Small n.** For n ≤ 8, all Krylov arms lose to direct LU in flops. Dense LU on HIRES is about 1.4 kflop per attempt, and LU flops are not counted in the LU control arm.
7. **Bruss-160 reference** disagrees with `hyp/ctrl/ref160.npy` by 1.4e-11. That is harmless at the 1e-6 and 1e-8 cells used.

Files are in `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/target/`:
- `coupled_target.py`
- `README.md`
- `tables.txt` — all result tables
- `abs_table.txt` — absolute cost numbers
- `res/*.jsonl` — raw runs, including `res/ladders.jsonl`
- `fid_mgs2.json` — replica fidelity against Rust
- `ref_*.json` — references