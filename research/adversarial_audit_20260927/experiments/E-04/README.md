# E-04 — Observed order of RODAS5P under inexact (Krylov) stage solves (H2)

**Status: EVIDENCE_ONLY — H2 SUPPORTED.** Under the PLAN rule (REFUTED only if the eta<=1e-9 arms *and* the production forcing rule all keep slope>=4.5 for >=4 halvings), no problem/arm combination other than `direct` and `gmres:1e-12` on P2 reaches 4 halvings. The production inner-forcing rule (`g4_s5b0_inner_tolerance.rs:37-72`, called from `sequential.rs:365-380`) yields an h-independent error floor at ~0.1-0.3*rtol (P1PR) / 0.01-0.8*rtol (P2) because eta grows like h^-1.9 (stiff P1/P1PR, rhs_wrms-dominated) or h^-1.1 (P2, flow_wrms-dominated) as h shrinks; the 0.5 clamp was never reached (eta_max<=1e-5). At tight tolerance the rule aborts at step 0 (`roundoff floor exceeds the stage-residual heuristic allocation`) for whole ranges of legal h (P1 rtol=1e-10: every h in the ladder; P1 rtol=1e-8: h>=1/256; P1PR rtol=1e-10: h>=1/32).

Additional finding: on the pure Prothero-Robinson problem P1PR the **direct LU arm itself shows order ~4, not 5** (slopes 4.01, 4.03, 4.03, 3.63, 3.42) — classical ROW order reduction; the fixed-eta GMRES error term is C_eta*eta with C_eta ~ 1e2-1e3 (not O(1)) because the stage right-hand side contains h*J*sum Gamma_ij K_j = O(h*lambda)*|K| (`sequential.rs:352-363`), so a *relative* residual tolerance becomes an absolute stage error ~ eta*|h lambda|*|K|; crossover vs direct at h*=1/32 already for eta=1e-12.

## Slope table (max-norm; local slope per halving, k=4..10; floor; crossover h* where error > 3x direct; failed h; d log eta / d log h)
| problem | arm | slopes k=4..10 | floor (k, failed rows excluded) | h* | failed at h | eta exponent |
|---|---|---|---|---|---|---|
| p2 | direct | 5.33, 5.27, 5.15, 5.05, 4.27, 0.41, -0.14 | 1.25e-15 (k=9) | - | - |  |
| p2 | gmres:1e-12 | 5.33, 5.27, 5.13, 4.94, 4.33, -3.34, 3.58 | 1.47e-15 (k=10) | - | - |  |
| p2 | gmres:1e-9 | 5.27, 3.14, 2.97, 1.27, -1.47, 5.00, 7.94 | 3.19e-15 (k=10) | h*=1/32 | - |  |
| p2 | gmres:1e-6 | 0.24, 2.50, 1.38, 0.27, 4.44, 0.90, 16.32 | 3.19e-15 (k=10) | h*=1/8 | - |  |
| p2 | gmres:1e-3 | 0.96, 1.25, 1.52, 0.49, -3.66, 1.00, 1.03 | 1.22e-05 (k=7) | h*=1/8 | - |  |
| p2 | forcing:1e-4 | -0.12, -0.17, -0.26, 5.56, -6.50, -3.39, 1.03 | 8.12e-08 (k=7) | h*=1/8 | - | -1.10 |
| p2 | forcing:1e-6 | 0.29, 0.80, 1.09, 0.28, 3.82, -0.05, -3.65 | 7.47e-10 (k=8) | h*=1/16 | - | -1.10 |
| p2 | forcing:1e-8 | 5.12, 2.41, 0.73, 3.59, -0.93, 1.57, 11.63 | 3.19e-15 (k=10) | h*=1/32 | - | -1.10 |
| p2 | forcing:1e-10 | 5.33, 5.15, 3.75, -1.47, 0.82, 2.89, 7.65 | 3.19e-15 (k=10) | h*=1/128 | - | -1.10 |
| p1pr | direct | 4.01, 4.03, 4.03, 3.63, 3.42, 1.97, 1.37 | 6.55e-15 (k=10) | - | - |  |
| p1pr | gmres:1e-12 | 3.14, 1.72, 1.46, 0.78, 1.72, 1.11, 1.59 | 1.36e-11 (k=10) | h*=1/32 | - |  |
| p1pr | gmres:1e-9 | 1.07, 1.41, 1.52, 1.18, 6.81, -3.02, 3.32 | 1.33e-09 (k=10) | h*=1/8 | - |  |
| p1pr | gmres:1e-6 | 1.26, 1.67, 1.45, 2.56, -0.86, 2.14, 1.36 | 7.22e-06 (k=10) | h*=1/8 | - |  |
| p1pr | gmres:1e-3 | -0.07, 1.26, 1.21, 2.41, 3.15, 2.20, -1.66 | 7.91e-04 (k=9) | h*=1/8 | - |  |
| p1pr | forcing:1e-4 | -0.43, -0.24, -0.40, -0.08, -0.27, 1.15, -0.42 | 1.20e-05 (k=3) | h*=1/8 | - | -1.86 |
| p1pr | forcing:1e-6 | -0.25, -0.43, -0.38, 0.09, 0.83, 0.64, -1.68 | 9.96e-08 (k=9) | h*=1/8 | - | -1.86 |
| p1pr | forcing:1e-8 | 3.60, 0.49, -0.30, -0.02, -0.88, -1.37, 2.75 | 1.98e-09 (k=10) | h*=1/32 | - | -1.86 |
| p1pr | forcing:1e-10 | nan, nan, nan, -0.20, -0.29, 1.92, -2.77 | 1.08e-11 (k=9) | h*=1/64 | 1/8, 1/16, 1/32 | -1.94 |
| p1 | direct | 0.19, 0.04, -0.38, 0.05, 0.69, 0.15, 0.22 | 4.70e-02 (k=10) | - | - |  |
| p1 | gmres:1e-12 | 0.19, 0.04, -0.38, 0.05, 0.69, 0.15, 0.22 | 4.70e-02 (k=10) | - | - |  |
| p1 | gmres:1e-9 | 0.19, 0.04, -0.38, 0.05, 0.69, 0.15, 0.22 | 4.70e-02 (k=10) | - | - |  |
| p1 | gmres:1e-6 | 0.80, 0.58, -0.38, 0.17, 0.74, 0.13, 0.20 | 4.74e-02 (k=10) | - | - |  |
| p1 | gmres:1e-3 | 0.81, 0.15, -1.11, 0.60, 0.88, 1.45, 1.06 | 2.03e-01 (k=10) | h*=1/8 | - |  |
| p1 | forcing:1e-4 | 0.19, 0.04, -0.38, 0.05, 0.69, 0.15, 0.22 | 4.70e-02 (k=10) | - | - | -1.88 |
| p1 | forcing:1e-6 | 0.19, 0.04, -0.38, 0.05, 0.69, 0.15, 0.22 | 4.70e-02 (k=10) | - | - | -1.88 |
| p1 | forcing:1e-8 | nan, nan, nan, nan, nan, nan, 0.22 | 4.70e-02 (k=10) | - | 1/8, 1/16, 1/32, 1/64, 1/128, 1/256 | -1.97 |
| p1 | forcing:1e-10 | nan, nan, nan, nan, nan, nan, nan | 0.00e+00 (k=3) | - | 1/8, 1/16, 1/32, 1/64, 1/128, 1/256, 1/512, 1/1024 |  |

Full per-row table (errors, endpoint slopes, rejections, rhs/jvp/linear_iterations, eta/tau): `table.md`; machine-readable: `analysis.json`, `results.json`. Plots: `e04_p2.png`, `e04_p1pr.png`, `e04_p1.png`, `e04_p1ns.png` (red stars = free adaptive integrator at rtol 1e-4..1e-10).

## Free adaptive integrator (production path, rtol 1e-4..1e-10, atol=1e-2 rtol)
| problem | rtol | err_max | err/rtol | accepted/rejected | linear_solve_failures | GMRES iterations |
|---|---|---|---|---|---|---|
| p1 | 1e-4 | 1.85e-05 | 0.18 | 95/6 | 0 | 136768 |
| p1 | 1e-6 | 2.06e-07 | 0.21 | 140/6 | 0 | 229920 |
| p1 | 1e-8 | 2.66e-09 | 0.27 | 257/6 | 0 | 354784 |
| p1 | 1e-10 | 2.90e-11 | 0.29 | 578/7 | 3 | 565120 |
| p2 | 1e-4 | 3.55e-06 | 0.04 | 67/0 | 0 | 19264 |
| p2 | 1e-6 | 1.28e-08 | 0.01 | 66/0 | 0 | 28320 |
| p2 | 1e-8 | 2.12e-10 | 0.02 | 66/0 | 0 | 41792 |
| p2 | 1e-10 | 3.02e-12 | 0.03 | 66/0 | 0 | 55680 |

## Problems
* **P1** (PLAN spec): n=256, A=Q^T diag(lambda) Q, lambda log-spaced in [-1e6,-1] (Q from seeded Gaussian + 2x MGS, orthogonality defect 7.8e-16), g_i=sin(t+phi_i), y' = A(y-g)+g', y0=g(0)+1 (transient in every mode), exact y = g(t)+Q^T e^{lambda t} Q 1. Its max-norm error is 5e-2..1e-1 for every arm and every h (first-step transient max_z|R(z)-e^z| over 256 modes) — unusable for order; endpoint error shown in table.md.
* **P1PR**: same with y0=g(0) (pure Prothero-Robinson) — added after seeing P1 (documented in results.json notes).
* **P1NS**: lambda in [-10,-1]; used by E-07 (main ~5, embedded ~4).
* **P2**: `semilinear_advection_diffusion_problem(512, 0.02, 3.0, -1.0, 10.0, 0.0)` with its manufactured exact solution (spectral bound ~2.4e4).

## Arms
`direct` = `sequential_step` with `LinearSolverConfig::default()` (LU, 1 factorization/step). `gmres:<eta>` = `sequential_matrix_free_step`, GMRES restart 32, maxiter 20000, rtol=eta, atol=1e-2 eta, no preconditioner, x0=previous stage. `forcing:<rtol>` = `sequential_matrix_free_step_with_inner_forcing` (the step called by `integrate_sequential_matrix_free_adaptive_observed`, integrate.rs:774) with outer (atol,rtol)=(1e-2 rtol, rtol), force_accept=true, would-be rejections counted. `adaptive:<rtol>` = `integrate_sequential_matrix_free_adaptive_observed`, h0=1e-3, Integral controller, output grid 1/64.

Harness: `harness/src/bin/e04_order_krylov.rs` (predecessor's, + `p1pr` variant), binary `cargo-target/measurement/e04_order_krylov`, RAYON_NUM_THREADS=1, measurement profile. Raw outputs `p*_<arm>.jsonl` / `.stderr`.
