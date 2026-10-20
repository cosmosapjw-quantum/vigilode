# Node b1b-gmres: lagged factorization with right-preconditioned GMRES stage solves

Production wave-2, Task 2. Plan: `docs/plan/PRODUCTION_PLAN_20261020.md` §5 T1, T2, T9; §6 B1.
Branch `production/b1b-gmres` from `a5a48f7`. Builds on wave-1 node `b1-lagged`
(`docs/plan/nodes/b1-lagged.md`, kill gate FAIL on both problems) and the wave-1 integration rules
(zero-scale end-of-step norm, zero-weight refusal on iterating stages).

## Preregistration (committed and pushed before the measurement run)

### Card

* **Mechanism.** Same driver as wave-1 B1 (exact `J₀ = J(tₙ, yₙ)` at every new state, only the LU of
  `P = I/(h_P γ) − J_P` lagged, same U-form stage equations `W uᵢ = cᵢ`, `W = I/(hγ) − J₀`, same
  controller, clock, error norm and output). The stage solver changes: Richardson
  `x ← x + P⁻¹(c − Wx)` is replaced by GMRES on `W x = c`, right-preconditioned with the same stored LU
  (`PreconditionerSide::Right`, `W P⁻¹ v = c`, `x = P⁻¹ v`). T2: the GMRES residual after k columns is at
  most the Richardson polynomial's, so with the same refresh rule fewer stages hit the budget.
* **Regime.** Wave-1 trace: q ≈ 0.10–0.20 per Richardson step against a reduction of about 1e-6, so
  stages hit the 6-iteration budget (126 of 134 refreshes on HIRES, 62 of 70 on Brusselator).
* **Theorem obligation.** T1 (exact target, residual tied to the tolerance), T2 (admissibility, refresh
  as a counted transaction), T9 (per-stage targets ρᵢ).

### Fixed constants (not to be changed after measurement)

| constant | value | source |
|---|---|---|
| θ | 0.1 | wave-1 `RODAS5P_LAGGED_THETA` |
| aᵢ | (0.58, 0.24, 6.25, 5.97, 3.73, 4.57, 1.02, 1.00) | wave-1 `RODAS5P_NONSTIFF_STAGE_SENSITIVITIES` (as at `a5a48f7`) |
| ρᵢ | θ / (8 max(aᵢ, 1)) | T9, wave-1 `rodas5p_lagged_stage_targets` |
| GMRES restart | 10 | brief |
| GMRES iteration budget per stage | 6 Arnoldi columns (refresh if a 7th would be needed) | brief, wave-1 `RODAS5P_LAGGED_MAX_ITERATIONS` |
| convergence-factor limit | 0.5: refresh when (νₖ/ν₀)^(1/k) > 0.5 | brief, wave-1 `RODAS5P_LAGGED_CONTRACTION_MAX` |

With a budget of 6 < restart 10 the restart never fires on these runs; it is configured as 10.

### Definitions fixed before measuring

* **Initial iterate** `x₀ = P⁻¹c`, as the Richardson path. The tested residual at iterate k is
  `νₖ = WRMS(P⁻¹ rₖ)`, `rₖ = c − W xₖ`, computed **explicitly** (one `W·x` and one LU application) at
  each check: k = 0 before GMRES starts and after every Arnoldi column k = 1, …, 6. Weights
  `atol + rtol |yₙ|` as wave-1 (a zero weight on an iterating stage is the typed `InvalidInput` refusal).
  `gmres_into` itself stops only on the true residual at cycle boundaries, so an opt-in monitored
  entry point is added to `rodas5p-krylov/src/gmres_into.rs`: the same right-preconditioned Arnoldi cycle,
  with the small least-squares problem solved after each column and the iterate `xₖ` handed to a caller
  monitor that accepts, continues or abandons. The existing `gmres_into` functions are not touched.
* **Order of tests at each check** (as wave-1): νₖ ≤ ρᵢ → converged; else (k ≥ 1) (νₖ/ν₀)^(1/k) > 0.5 →
  contraction refresh; else k = 6 → budget refresh. On convergence the already computed correction is
  applied, `x ← xₖ + P⁻¹rₖ` (as the Richardson path applies its last correction).
* **Refresh**: refactor at the current (h, J₀), restart the stage with the direct solve; charged to
  `direct_factorizations` and `LaggedWork` by reason; abandoned GMRES columns stay counted.
* `P = W` (factored at this attempt's own (h, J₀)): direct solve, no GMRES.
* **Iterations counted** = Arnoldi columns. Operator applications (`W·x`) = columns + explicit stopping
  residuals + GMRES's cycle-start true residuals; all are counted and reported.
* Default `StageSolver::Richardson` stays the wave-1 driver bit for bit (driver id
  `rodas5p-lagged-richardson-v1`); the GMRES arm reports driver id `rodas5p-lagged-gmres-v1`.

### Measurement and kill gate (identical to wave-1 Task 3)

* Problems: HIRES (n = 8, t ∈ [0, 321.8122], atol = 1e-4 · rtol) and Brusselator 1D n = 100
  (`brusselator_problem(50)`, t ∈ [0, 10], atol = rtol), rtol = 1e-6, initial step 1e-6, min step 1e-14,
  `stiff_benchmark.rs::adaptive_config`. Dense storage; debug build as wave-1.
* Reference: fast driver at rtol 1e-11. Error: endpoint, max over components, in tolerance units
  |y − ref| / (atol + rtol |ref|).
* Comparator: `integrate_rodas5p_fast_observed`, same tolerances.
* **Kill gate per problem** (fail if any): factorizations per accepted step > 0.4; accepted steps > 1.05 ×
  fast; endpoint error (tolerance units) > 1.5 × fast. Verdict PASS/FAIL per problem with all counters.
  The constants above are never tuned after the run. Wall time is diagnostic only.
* Command: `cargo test -p rodas5p-integrators --locked -j 1 --test b1b_gmres_contracts --
  --ignored --nocapture b1b_gmres_measurement` (env per global constraints).

## Result

**Verdict: kill gate FAIL on both problems** (HIRES: endpoint error; Brusselator 1D n = 100: accepted steps and
endpoint error). The factorization criterion passes on both. Constants and definitions are those preregistered
in `0879a7e`; nothing was changed after the run.

Commits: `0879a7e` (this preregistration, pushed first), `5134cd2` (tests, RED: API absent), `8677bb5`
(implementation; measurement run on this commit).

### What was built

* `crates/rodas5p-integrators/src/rodas5p_lagged.rs`: `StageSolver::{Richardson (default), Gmres}` in
  `Rodas5pLaggedOptions::stage_solver`; `iterate_gmres` / `gmres_stage` with a `StageOperator` (`W x` by the
  JVP at `(tₙ, yₙ)` or the current dense Jacobian's matvec, the arithmetic of the Richardson `apply_w`) and
  `StoredLu` (right `Preconditioner`, the stored LU). Driver id `rodas5p-lagged-gmres-v1`; `LaggedWork` gains
  `gmres_iterations`, `gmres_cycles`, `residual_applications`. Module docs state which residual is tested.
* `crates/rodas5p-krylov/src/gmres_into.rs`: additive `solve_gmres_into_right_monitored` (+
  `GmresMonitorDecision`, `GmresMonitoredReport`, `GmresColumnMonitor`): the right-preconditioned cycle of
  `gmres_into` with the least-squares problem solved after every column and the iterate handed to a monitor.
  Needed because `gmres_into` tests only the true residual and only at cycle boundaries. Existing functions
  untouched; a test shows the monitored iterate at the last column of a full cycle is bitwise
  `solve_gmres_into`'s.
* `crates/rodas5p-integrators/src/rodas5p_fast.rs`: additive `Workspace::solve_dense_into(&self, b)`.

### Measurement (preregistered; rtol 1e-6, reference = fast at rtol 1e-11; debug build)

Command as preregistered, on `8677bb5`. Wall time is diagnostic only (shared 2-core host, debug build).
"iterations" = Richardson iterations or GMRES Arnoldi columns; "W·x" = all stage-operator products; "LU
applications" = `direct_solve_calls`.

**HIRES** (n = 8, atol 1e-10)

| arm | accepted | rejected | Jacobians | factorizations | fact/accepted | iterations | abandoned it. | W·x | LU applications | endpoint max abs err | endpoint err (tol units) | wall ms (diagnostic) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fast | 204 | 6 | 204 | 210 | 1.029 | 0 | 0 | 0 | 1680 | 1.640e-9 | 0.2587 | 7.68 |
| lagged-richardson (wave-1, comparison) | 205 | 8 | 205 | 134 | 0.654 | 4434 | 770 | 4434 | 6271 | 2.602e-9 | 0.4104 | 25.38 |
| lagged-gmres | 205 | 10 | 205 | 15 | 0.073 | 4868 | 20 | 13033 | 18011 | 6.147e-9 | 0.9697 | 565.29 |

GMRES arm: refreshes initial 1, contraction 13, budget 1; stages exact 47, lagged-converged 1659, refreshed
14; GMRES cycles 1624; non-column W·x 8165.

**Brusselator 1D, 50 cells (n = 100)** (atol 1e-6)

| arm | accepted | rejected | Jacobians | factorizations | fact/accepted | iterations | abandoned it. | W·x | LU applications | endpoint max abs err | endpoint err (tol units) | wall ms (diagnostic) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fast | 82 | 10 | 82 | 92 | 1.122 | 0 | 0 | 0 | 736 | 7.903e-6 | 1.686 | 49.60 |
| lagged-richardson (wave-1, comparison) | 82 | 10 | 82 | 70 | 0.854 | 1673 | 386 | 1673 | 2478 | 7.891e-6 | 1.683 | 619.98 |
| lagged-gmres | 980 | 859 | 980 | 9 | 0.009 | 11401 | 43 | 47678 | 63593 | 9.371e-5 | 19.99 | 11289.33 |

GMRES arm: refreshes initial 1, contraction 1, budget 7; stages exact 42, lagged-converged 14662, refreshed
8; GMRES cycles 10206; non-column W·x 36277.

The Richardson rows reproduce wave-1 exactly (the default is pinned bit for bit by a test).

### Kill gate per problem (GMRES arm)

| problem | fact/accepted (fail > 0.4) | accepted vs fast (fail > 1.05) | error vs fast (fail > 1.5) | verdict |
|---|---|---|---|---|
| HIRES | 0.073 — pass | 1.0049 — pass | 3.749 — fail | **FAIL** |
| Brusselator n = 100 | 0.009 — pass | 11.95 — fail | 11.86 — fail | **FAIL** |

### Why it fails (status: interpretation, not tested here)

The card's premise held: budget refreshes fell from 126 to 1 (HIRES) and from 62 to 7 (Brusselator), and
factorizations per accepted step fell to 0.07 and 0.009. What broke is accuracy. The stopping quantity
`P⁻¹rₖ = (P⁻¹W) eₖ` estimates the stage error `eₖ` only within `1 ± q`, `q = ‖I − P⁻¹W‖` (T2), and that is
where wave-1's Richardson was safe: Richardson diverges, and its contraction rule refreshes, as soon as q
approaches 1. GMRES converges for any nonsingular `P⁻¹W`, so the refresh rule (GMRES progress, not q) no
longer bounds the age of P; with 9 factorizations over 1,839 attempts on Brusselator, P was far from W, and
`‖P⁻¹r‖ ≤ ρ` then allows a stage error up to `‖(P⁻¹W)⁻¹‖ ρ`. The 859 rejections (local-error rejections of
inexact stages) and the 12x step count fit this. Untested next-card candidates (none tried, to keep this gate
unscored): bound P's age through an estimate of q (one Richardson ratio per stage, or `h/h_P` and the
Jacobian change), or stop on a residual whose error map does not depend on P (`W⁻¹`-weighted through a
fresh-P bound).

### Commands (env `RUSTUP_TOOLCHAIN=stable CARGO_TARGET_DIR=/home/claude/targets/b1b-gmres CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`, `--locked -j 1`)

| command | exit | result |
|---|---|---|
| `cargo test -p rodas5p-integrators --test b1b_gmres_contracts` | 0 | 7 passed, 1 ignored |
| `cargo test -p rodas5p-integrators --test b1b_gmres_contracts -- --ignored --nocapture b1b_gmres_measurement` | 0 | measurement above |
| `cargo test -p rodas5p-integrators --test b1_lagged_contracts` | 0 | 5 passed, 1 ignored |
| `cargo test -p rodas5p-integrators --test a1_zero_tolerance_scale` | 0 | 9 passed |
| `cargo test -p rodas5p-integrators --test rodas5p_fast_contracts` | 0 | 4 passed |
| `cargo test -p rodas5p-integrators --test rodas5p_fast_allocations` | 0 | 2 passed |
| `cargo test -p rodas5p-integrators --test int03_banded_fast_contracts` | 0 | 4 passed |
| `cargo test -p rodas5p-integrators --lib rodas5p_fast` | 0 | 17 passed |
| `cargo test -p rodas5p-krylov --test rnext02_gmres_into_contracts` | 0 | 5 passed |
| `cargo clippy -p rodas5p-integrators -p rodas5p-krylov --lib --test b1b_gmres_contracts` | 0 | no warnings in touched files |
| `cargo fmt -p rodas5p-integrators -p rodas5p-krylov -- --check` | — | touched files clean (pre-existing diff in `production_audit_packet_curve.rs` only) |

### Deviations and notes

* **Measurement invoked twice.** The first invocation's console output was piped through a filter that
  dropped the table rows (the kill-gate lines were kept). It was re-run once, unchanged, to capture the
  tables; the kill-gate and refresh lines of both runs are identical (counts are deterministic). No code or
  constant changed in between.
* `rodas5p-krylov` gained an additive entry point (above); the brief named `gmres_into`, which cannot stop
  between columns.
* Counter conventions of the GMRES arm: every stored-LU application adds to `direct_solve_calls`; an
  accepted GMRES solve adds 1 to `linear_solves` (the Krylov convention); `W·x` are charged through
  `apply_counted` (`linear_matvecs`, plus `jvp_calls`/`jvp_vectors` or `jacobian_matvecs`).
* Two existing test literals of `Rodas5pLaggedOptions` gained `..Rodas5pLaggedOptions::default()`.
* `Tier::Performance` still routes no lagged driver (wave-1 ruling; no B1 card has passed).
