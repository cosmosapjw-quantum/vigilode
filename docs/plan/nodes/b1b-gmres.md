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
