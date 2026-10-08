# Preregistration: tolerance-coupled stage targets with in-cycle exit in the matrix-free U-form driver (ALG01)

Algorithmic-directions cycle, Tier 1 node N1 (`docs/reviews/20261008_algorithmic_directions/ALGORITHMIC_DIRECTIONS.md`,
sections 2.1, 2.4 and 4). Branch `audit/rvj-algorithmic-directions-20261008`, base `a793ecd`.

**Claim boundary.** The node measures counted work (WorkCounters: JVPs, Krylov iterations, matvecs,
orthogonalization inner products and vector updates, RHS) and endpoint accuracy. It makes no instruction-count or
wall-time claim; the timing authority stays on HOLD. The matrix-free U-form driver is a research driver, and every
change here is opt-in: the default arm must reproduce its recorded base export bit for bit.

## Question

The U-form driver solves each stage to an L2-relative 1e-10 with absolute floor gamma * 1e-14. Restarted GMRES never
stops inside a cycle, so it over-solves that target by 4-5 decades. The exploratory pilots (probe A1, B1 and the
critic, `docs/reviews/20261008_algorithmic_directions/pilot/`) found two things:

- With an in-cycle exit at the production target, accuracy collapses at tight tolerances. HIRES reaches 4.2 x rtol at
  1e-10.
- A stage target measured in the outer WRMS metric and tied to the tolerance per unit step ("coupled target") keeps
  accuracy within 1.15x of base and matches direct LU on fixed-step ladders. It also cuts JVPs per accepted step to
  0.26-0.31x on Brusselator-50.

Does the Rust implementation reproduce this? It must keep accuracy on every gated cell, including the strongly
nonnormal and badly scaled stress cells where the pilot's first version failed.

## Change (opt-in; the default path is unchanged)

1. **`rodas5p-krylov`: a staged GMRES solver** (new file). Restarted GMRES with two-pass MGS and incremental Givens QR,
   zero start.
   - **Exit and confirmation.** In-cycle exit when the projected residual is at most the threshold. Each exit is
     confirmed by one true residual. If the confirmation fails, the cycle continues and the gap to the next check
     doubles. No separate final diagnostic residual is computed.
   - **Threshold.** `max(atol, rtol ||b||)` as today, so the production target is available as an arm.
   - **Optional stall rule.** After a restart cycle that cuts the true residual by less than 4x, accept when
     `||r|| <= 1024 eps (||b|| + ||x|| + ||x - A x||)`.
   - **Optional nonnormality guard.** At the first projected exit, compute `nu = 1 / sigma_min(R_j)` of the Givens
     triangular factor. If `nu > 1`, divide the threshold by `nu` and require that threshold of the true residual too.
   - **Optional fallback test.** A caller-supplied closure that may accept the iterate when the budget is exhausted,
     given the iterate and its true residual.
   - **Report.** How the solve ended: converged, stall-accepted, fallback-accepted or failed. Also cycles, checks,
     the largest nu, and the confirmed true-residual norm.
2. **`rodas5p_matrix_free_fast.rs`: a stage-target policy on the workspace.** The default is `Legacy`: today's solver
   and arithmetic, bit for bit. The other policies use the staged solver:
   - **`ProjL2`**: the production L2 target (rtol 1e-10, atol from `raw_absolute_residual_budget`) with in-cycle exit.
   - **`L2Coupled`**: `rtol_lin = min(1e-10, 1e-3 rtol)` and `atol_lin = gamma * 1e-3 * atol`, with in-cycle exit.
     This is the judge's cheap rival.
   - **`Coupled`**: the coupled target. GMRES runs on the scaled system `D W D^-1 z = D b`, with
     `D = diag(1 / (atol + rtol |y_n|))` and `x = D^-1 z`. Each stage gets the absolute target
     `eps_i = theta_n / (8 max(tau_y,i, tau_e,i))`.
     - `theta_n = Theta (h_n / T) min(1, e_hat / 0.5)^(6/5)`, with Theta = 0.2.
     - T is the integration span, passed explicitly by the driver.
     - `e_hat` is the last accepted error, 1e-6 before the first acceptance.
     - `eps_8 <= 0.1 (0.9/5)^5`.
     - The scaled-system threshold is `max(eps_i sqrt(n), 16 eps ||D b||)`, with the stall rule.
     - The tau constants are the tableau's residual-to-output transfer constants. They are computed at workspace
       construction from the coefficient snapshot (pilot `phase2_code/tau_table.py`) and checked against the pilot
       table in a unit test.
   - **`CoupledGuarded`**: `Coupled` plus the nonnormality guard plus a production fallback. On budget exhaustion the
     iterate is accepted if its unscaled true residual satisfies the production rule
     `||b - W x||_2 <= max(gamma 1e-14, 1e-10 ||b||_2)`. This is the stack S' target and the gated arm.

The budget stays at 200 columns, the production default, in this node. ALG03 measures a larger budget. The step-size
controller is unchanged: Integral.

## Cells

All runs use GMRES, restart 40 and maxit 200, with the SPD07 adaptive configuration (`rnext_common::adaptive`:
initial step 1e-6, atol = rtol * atol_scale, max 5,000 attempts).

1. **C1, the SPD07 set.** Robertson, van-der-pol-mu1000, HIRES, prothero-robinson-forced, quadratic-4,
   brusselator-1d-50 and brusselator-1d-160, at rtol 1e-6 and 1e-8 (14 cells).
2. **C2, tight cells.**
   - HIRES and Robertson at 1e-9, 1e-10 and 1e-11.
   - Robertson 1e-11 is reported only, because the pilot found even direct LU at the round-off floor there.
3. **C3, stress cells** (definitions from the pilot `phase3_code/critic/sprobs.py`), at rtol 1e-4, 1e-6 and 1e-8:
   - `vigb-k10` and `vigb-k20` (32 blocks `lam_j [[-2, 2^k], [2^-k, -2]]`, `lam_j` log-spaced in [1, 1e4], smooth
     forcing, n = 64);
   - `vig1b-k20` (one block, `lam` = 1e3);
   - `e05-s0` and `e05-s1` (the E-05 operator, n = 256, forced as in the pilot; the construction is that of the audit's
     E-05 experiment, reproduced in Rust).
4. **C4, fixed-step ladders.**
   - `diagpr128` (diagonal Prothero-Robinson, n = 128, lambda to -1e6), k = 3..5.
   - `semilin64` (`semilinear_advection_diffusion_problem(64, 0.05, 0.5, -1, 0.5)`), k = 3..8, with stage targets
     from rtol 1e-4 and 1e-6.
   - `semilin128` (`(128, 0.02, 3, -1, 10)`), k = 3..5.
   - Step h = span / 2^k, every step accepted.
5. **C5, reported ladders** for matched-accuracy frontiers (not gated): Brusselator-50 and HIRES at rtol 1e-3 to 1e-10
   in half decades.

**References.**
- The exact solution where one exists.
- Otherwise the dense fast driver at rtol 1e-13, with the 1e-12 vs 1e-13 difference reported as the reference
  uncertainty. A cell whose uncertainty exceeds 0.1x the twin's error is excluded from gate 2 and reported.

**Twins.**
- **Adaptive cells.** The direct-solve twin is the dense fast driver (`integrate_rodas5p_fast_observed`) on the
  explicit-Jacobian problem, with the same adaptive configuration.
- **Ladders.** The twin is the same U-form arithmetic with dense LU stage solves, written in the test.

**Error metric.** Componentwise `max_i |y_i - ref_i| / max(|ref_i|, 1e-10)` at the end point.

## Base export (before any source change)

A test-only commit adds the export binary. Its `export_base` runs the `Legacy` policy on every cell of C1-C5 on the
unmodified solver source and writes `BASE.json`. The C1 rows must equal SPD07's `BASE.json` `gmres_into` `Zero` rows.

## Commands

    ALG01_BASE=research/alg01_coupled_stage_target_20261008/BASE.json cargo test --release -p rodas5p-integrators --locked --test alg01_coupled_stage_target -- --ignored --nocapture --test-threads=1 export_base
    ALG01_RUNS=research/alg01_coupled_stage_target_20261008/RUNS.json cargo test --release -p rodas5p-integrators --locked --test alg01_coupled_stage_target -- --ignored --nocapture --test-threads=1 export_runs
    python3 tools/alg01_coupled_target_check.py --base .../BASE.json --runs .../RUNS.json --output research/alg01_coupled_stage_target_20261008/RESULTS.json

## Gate (arm `CoupledGuarded`)

**PASS** if all of the following hold.

1. **Base reproduction.** `Legacy` in RUNS reproduces BASE bit for bit (states, attempts, counters), and BASE's C1 rows
   equal SPD07's.
2. **Accuracy.** In every C1, C2 (except Robertson 1e-11) and C3 cell where the twin succeeds and the reference is
   admissible: `err(arm) <= 1.5 err(twin)`.
3. **Ladders.**
   - At every rung of `diagpr128` and `semilin128`: `|y_arm - y_ref| <= 3 |y_LU - y_ref| + 1e-13` (relative
     max-norm).
   - On `semilin64` at rtol 1e-6: two consecutive observed order slopes of at least 4.5.
4. **Robustness.** In every cell the arm succeeds wherever `Legacy` succeeds, and has no more linear-solve failures
   than `Legacy`.
5. **Work at equal rtol.** JVPs per accepted step:
   - Brusselator-1d-50: at most 0.40x `Legacy` at both rtols, with orthogonalization inner products at most 0.35x.
   - Brusselator-1d-160: at most 0.60x.
   - HIRES, Robertson and van der Pol: at most 0.85x, at 1e-6 and 1e-8.

Everything else is **FAIL**, with every ratio preserved.

**Reported, not gated.**
- `ProjL2`, `L2Coupled` and `Coupled` on every cell.
- The C5 frontiers (regression and cheapest-run).
- Stall and fallback acceptances, nu statistics.
- A finite-difference-JVP Brusselator-50 variant of `CoupledGuarded`, if the problem type allows it.

**Predictions** (pilot, equal rtol).
- JVPs per accepted step, `Coupled` against `Legacy`:
  - Brusselator-50: 0.31 (1e-6) and 0.26 (1e-8).
  - Brusselator-160: 0.45 and 0.51.
  - HIRES 0.74; Robertson 0.55-0.72; van der Pol about 0.75.
- `ProjL2` fails gate 2 on HIRES at 1e-9 and 1e-10.
- `Coupled` without the guard fails gate 2 on `vigb-k20` (pilot: 40-1,600x).
- `CoupledGuarded` costs at most 1.03x `Coupled` outside the VIG cells.
- `L2Coupled` fails the `semilin64` slope item.

## Prior information

- Pilot evidence: `docs/reviews/20261008_algorithmic_directions/pilot/phase3_reports/{target,stack,critic}.md`.
- SPD07 (L-0084) and its `BASE.json`.
- L-0045 (GMRES into); the Givens research kernel `gmres_givens.rs` (never wired; not reused bit for bit).
- No code of this node exists before this commit.

## Results (appended after the recorded run; the registered text above is unchanged)

**Recorded commits.** Base export harness `168e951` (test only; the solver source is the registration commit's),
`BASE.json` committed in `61126ee`. Implementation: staged solver `d9a4b5b` (`rodas5p-krylov/src/gmres_staged.rs`),
stage-target policies, entry point `integrate_rodas5p_mf_fast_observed_with_stage_target` and the RUNS exports
`ea99664`. `RUNS.json` was produced on `ea99664` (clean tree), `RESULTS.json` by `tools/alg01_coupled_target_check.py`.

**Commands** (release, `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`, `CARGO_INCREMENTAL=0`): the registered
`export_base` and `export_runs` commands, then

    python3 tools/alg01_coupled_target_check.py --base research/alg01_coupled_stage_target_20261008/BASE.json --runs research/alg01_coupled_stage_target_20261008/RUNS.json --output research/alg01_coupled_stage_target_20261008/RESULTS.json

Base export 96 s, recorded run 169 s. Contract tests: `cargo test -p rodas5p-krylov -p rodas5p-integrators
--all-targets --locked` passes; clippy `-D warnings` clean with and without `audit2-research`.

**Gate (arm `CoupledGuarded`): FAIL.**

| Item | Result | Numbers |
|---|---|---|
| 1. Base reproduction | PASS | `Legacy` in RUNS equals BASE on all 65 adaptive cells and 18 ladder rungs (also at the pilot budget); BASE C1 equals SPD07 `gmres_into_zero` 14/14; the twins and LU rungs reproduce too |
| 2. Accuracy (<= 1.5x twin) | **FAIL** | 31 cells evaluated, 29 pass (C1 0.93-1.00x, C2 0.88-1.00x, E-05 0.02-1.04x, vigb-k10 1.00x, vigb-k20 0.55-1.01x). Fails: `vig1b-k20` 1e-4 **37.14x**, 1e-8 **4.36x** (1e-6: 0.50x). Excluded: HIRES 1e-10 and Robertson 1e-10 (reference uncertainty 3.7e-12 and 3.0e-12 > 0.1x twin error 1.1e-11 and 1.9e-11; the arm is 0.955x and 1.00x there), HIRES 1e-11 (twin fails at 5,000 attempts), Robertson 1e-11 (registered report-only) |
| 3. Ladders (budget 200) | **FAIL** | diagpr128 k = 3, 4, 5 and semilin128 k = 3: the stage solve exhausts the 200-column budget and the fallback does not accept (Legacy fails the same rungs). semilin128 k = 4, 5: 1.00x LU. semilin64 1e-6 observed slopes 5.15, 5.13 (pass) |
| 4. Robustness | PASS | 52 cells (C1, C2 without Robertson 1e-11, C3, C4 rungs): succeeds wherever Legacy succeeds, failures <= Legacy's everywhere (E-05 1e-4: 27 vs 53 and 30 vs 31) |
| 5. Work (JVP per accepted step vs Legacy) | PASS | Bruss-50 0.312 / 0.258 (inner products 0.108 / 0.071); Bruss-160 0.448 / 0.512; HIRES 0.741 / 0.711; Robertson 0.702 / 0.680; van der Pol 0.751 / 0.748 (1e-6 / 1e-8) |

**Reported, not gated.**
- Pilot ladder budget 20,000 (the pilot's `ladders.py` value): `CoupledGuarded` is 0.997-1.001x LU at every rung
  of diagpr128 and semilin128 and passes the semilin64 slope item; `Coupled` the same. Legacy is 18-330x LU on
  diagpr128, `ProjL2` 27-796x, `L2Coupled` 33-987x.
- `ProjL2` fails item 2 on HIRES 1e-9 (1.70x; 1e-10 is 101.8x but excluded by the reference rule), also HIRES 1e-8
  (2.48x), E-05 s = 0 1e-6, vigb-k10 (12-423x) and does not complete vigb-k20; it also fails item 5.
- `Coupled` (no guard) fails item 2 on vigb-k20 (97x, 205x, and fails to complete at 1e-8) and vig1b-k20; the
  nonnormality guard restores vigb-k20 (1.00x, 1.01x, 0.55x) but not the single-block vig1b-k20, where the
  pilot's unguarded coupled arm (`C3G`, critic `tables.txt`) shows the same 37.17x at 1e-4.
- `L2Coupled` fails the semilin64 slope item (slopes 5.18, 4.06, then -0.71, -1.24, -0.80) and item 2 on VIG cells.
- `CoupledGuarded` / `Coupled` JVPs outside the VIG cells: at most 1.032x (prediction <= 1.03x).
- C5 frontiers, JVP ratio against Legacy (regression frontier / cheapest-run rule): Bruss-50 `CoupledGuarded` 0.274 /
  0.283, `L2Coupled` 0.298 / 0.311, `ProjL2` 0.351 / 0.423; HIRES `CoupledGuarded` 0.734 / 0.806, `L2Coupled`
  0.712 / 0.894, `ProjL2` 0.799 / 0.886.
- Stage statistics over all cells, `CoupledGuarded`: 247,617 solves, 11,776 stall acceptances, 27 fallback
  acceptances, 57 failed solves (all budget exhaustion, all in the E-05 1e-4 cells; adaptive cells only), 208,051 solves with
  the threshold tightened by nu (max nu 2.05e6, on vigb-k20), 61,254 failed confirmations in 295,932.
- Finite-difference-JVP Brusselator-50: Legacy and `CoupledGuarded` both stop at 5,000 attempts with 2,498 failures
  at both rtols (the pilot's "every arm livelocks as calibrated").
- Predictions (Coupled vs Legacy JVP per accepted step): Bruss-50 0.311 / 0.258 (pred. 0.31 / 0.26), Bruss-160
  0.448 / 0.512 (0.45 / 0.51), HIRES 0.741 / 0.710 (0.74), Robertson 0.702 / 0.680 (0.55-0.72), van der Pol
  0.751 / 0.748 (about 0.75).

**Interpretations fixed before the recorded run, and deviations.**
- Ladders: run with the registered budget 200 ("All runs use ... maxit 200"); the pilot's budget 20,000 is
  recorded as a reported supplement only. Ladder atol = 1e-2 rtol (the contract ladders and the pilot), rtol 1e-6
  for diagpr128 and semilin128, span [0, 1], `e_hat` = the previous step's embedded error (pilot `rep.py`).
- Observed slope: `log2(e_k / e_(k+1))` of the relative max-norm error, counted only when `e_(k+1) > 1e-12` (the
  pilot's floor in `final.py`); "two consecutive" = two adjacent observed slopes.
- Item 4 covers C1, C2 (without Robertson 1e-11), C3 and the C4 rungs; C5 is registered as not gated. Item 5's
  inner-product ratio is per accepted step, like the JVP ratio.
- Nonnormality guard: evaluated whenever the projected test would pass (the first evaluation is at the first
  projected exit; later ones where an exit would otherwise be taken), running maximum per solve, as in the pilot;
  `sigma_min` from a dense SVD of the `j x j` Givens factor (faer), charged in the report as `4 j^3` flops.
- Confirmation gap: starts at 1 column, doubles after each failed confirmation of a solve, and is not reset at a
  restart (each new cycle may confirm from its first column).
- The production fallback uses the configured production rule `max(raw_absolute_residual_budget(1e-14, gamma),
  1e-10 ||b||_2)`, which is the registered `max(gamma 1e-14, 1e-10 ||b||_2)` (up to the directed rounding of
  `gamma 1e-14`). `ProjL2` and `L2Coupled` have no stall rule (pilot `make_arm`).
- The transfer constants are computed from the coefficient snapshot when a coupled policy is set on the workspace
  (before any attempt), not in `Rodas5pMfFastWorkspace::new`, so the default path runs no new code; they match the
  pilot table within 6e-4 (unit test).
- The E-05 exact endpoint is `phi(T) + expm(T A) e` with the Pade-13 scaling-and-squaring kernel of
  `rodas5p-core`; the dense driver at 1e-13 agrees to 4.7e-12 (s = 0) and 4.0e-13 (s = 1).
- The exports run only when their output variable is set, because the registered filter `export_base` (and
  `export_runs`) also matches `export_base_alg03` (`export_runs_alg03`).
