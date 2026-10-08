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
