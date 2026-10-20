# Node b1c-staleness: lagged LU with the T2 staleness bound and an a-posteriori stage-error stop

Production wave 3, Task 2 (`docs/plan/IMPLEMENTATION_WAVE3.md`; plan §5 T1, T2, T9, §6 B1).
Branch `production/b1c-staleness`. Ledger row **L-0141**, verdict **FAIL**.
Preregistration and results: `research/b1c_staleness_20261021/PREREGISTRATION.md`. Raw output:
`research/b1c_staleness_20261021/results.json`.

The lagged driver stays unrouted from `Tier::Performance`. This node does not change routing, and the
`rodas5p_fast.rs` and facade code are untouched.

## What was built

All of it is in `crates/rodas5p-integrators/src/rodas5p_lagged.rs`, plus exports in `lib.rs`.

- `StalenessRule { Off (default), Bounded }` and the new field `Rodas5pLaggedOptions::staleness`. The
  options struct stays `Copy + Eq + Default`.
- `RODAS5P_LAGGED_STALENESS_Q_MAX = 0.5`.
- `rodas5p_staleness_bound(h, h_p, gamma, j, j_p, s)`, Lemma B1c's q_pred. It returns +∞ in these cases:
  - h_Pγμ_S(J_P) ≥ 1;
  - a zero or non-finite weight (ruling B2d);
  - a non-positive step, mismatched shapes, or non-finite entries.

  It never returns NaN.
- The check runs before the stage solves, only when P is valid and no Initial or Forced refresh is due
  (ruling B2c). q_pred > 0.5 triggers a refresh through the private `Refresh::Staleness`. Otherwise the
  stage stops on ‖P⁻¹r_k‖_S ≤ (1 − q_pred) ρ_i, on both Richardson and GMRES. The contraction and
  progress refreshes keep their WRMS tests.
- J_P is a dense copy taken at every refresh. It is allocated once at setup, and only with `Bounded`.
- `StalenessWork { checks, staleness_refreshes, unbounded_checks, q_pred_max }` is carried as
  `Rodas5pLaggedResult::staleness`. `q_pred_max` is the largest *finite* q_pred.
- New driver ids: `rodas5p-lagged-gmres-bounded-v1` and `rodas5p-lagged-richardson-bounded-v1`.
- The `LaggedWork::preconditioner_refreshes` doc now names the fifth reason (B2e).

## Contracts (`crates/rodas5p-integrators/tests/b1c_staleness_contracts.rs`)

- **(a)** For Robertson and HIRES, at three states with J_P from an earlier state, h ∈ {1e-5, 1e-3, 1e-1}
  and h_P/h ∈ {0.5, 0.8, 1, 1.25, 2}: ‖I − P⁻¹W‖_S, formed by n LU solves, is ≤ q_pred (1 + 1e-12)
  whenever q_pred is finite.
- **(b)** For n = 1 and λ ∈ {−1e6, −1, 0}, the bound equals |1 − h_P/h|/(1 − h_Pγλ) within 1 ulp. A zero
  weight gives +∞, and h_Pγμ = 2 gives +∞.
- **(c)** Ruling FP1 replaced y′ = +y with J = [[−1, 1e8], [0, −1]] and y0 = (0, 1e-8), with
  atol = rtol ∈ {1e-4, 1e-6}. Every check is unbounded and refreshes, and the outputs equal the fast
  driver within 1e-12, with equal step and factorization counts. This holds on both stage solvers.
- **(d)** `Off` reproduces the endpoint bits and counters on Robertson, HIRES and Brusselator for both arms:
  - Richardson against the existing pins;
  - GMRES against pins printed at base `75a0c6c` before any change (ruling B2b; FP5).
- **(e)** `refresh_every_attempt` + `Bounded` on both arms: no check is made (`StalenessWork` is zero). On
  Robertson and HIRES at rtol {1e-4, 1e-6, 1e-8}, the result equals the fast driver within 1e-12, with
  equal factorizations.
- **(f)** refreshes = initial + forced + contraction + budget + staleness = `direct_factorizations`, on
  three problems and both arms.

## Measurement (preregistered; rtol 1e-6; reference = fast at 1e-11; release, `-j 1`)

| problem | arm | accepted | factorizations | fact/acc | checks | staleness refr. | unbounded | err (tol units) |
|---|---|---|---|---|---|---|---|---|
| HIRES | fast | 204 | 210 | 1.029 | – | – | – | 0.2587 |
| HIRES | gmres-bounded (gated) | 204 | 120 | **0.588** | 210 | 119 | 75 | 0.2598 |
| HIRES | richardson-bounded (context) | 204 | 161 | 0.789 | 209 | 101 | 75 | 0.2578 |
| Brusselator n=100 | fast | 82 | 92 | 1.122 | – | – | – | 1.686 |
| Brusselator n=100 | gmres-bounded (gated) | 82 | 20 | 0.244 | 91 | 19 | 0 | 1.683 |
| Brusselator n=100 | richardson-bounded (context) | 82 | 73 | 0.890 | 91 | 8 | 0 | 1.686 |

Gate for `Gmres + Bounded`:

- **HIRES FAIL.** fact/acc is 0.588 > 0.4. Steps are 1.000x and error is 1.004x, both within their limits.
- **Brusselator PASS.** fact/acc 0.244, steps 1.000x, error 0.998x.

**Overall FAIL.** This verdict is final and is not re-scored. **B1 family closed until a new mechanism.**

## Observations (D = measured, I = interpretation, not tested here)

- **D.** The corrected stop removes the B1b failure mode. On Brusselator, B1b GMRES took 980 accepted
  steps with error 11.9x. Here it takes 82 steps, the fast driver's count, with error 0.998x. On both
  problems the step counts equal fast's.
- **D.** On HIRES, 75 of the 210 checks are unbounded (h_Pγμ_S(J_P) ≥ 1). Those alone are 0.37
  refreshes per accepted step. The other 44 staleness refreshes come from finite q_pred > 0.5.
- **D.** The GMRES arm needed no contraction or budget refresh on either problem.
- **I.** On HIRES the bound is the binding cost, not the iteration. μ_S is inflated by the weight ratios
  s_j/s_i. At y0 the ratio s_0/s_1 ≈ 1e4 enters row 1, and the preflight scan found μ_S ≈ 1.7e4 there. As a result the lemma's hypothesis fails at accepted steps, where GMRES would still have
  converged. A mechanism with a sharper or differently normed bound would be a new card, not a re-score
  of this one.
- The preflight scipy-Radau proxy predicted 0.39 against the 0.4 gate. The measured RODAS5P value is 0.588.

## Commands

Environment:

    RUSTUP_TOOLCHAIN=stable CARGO_TARGET_DIR=/home/claude/targets/b1c-staleness CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0

Commands:

    cargo test -p rodas5p-integrators --locked -j 1 --test b1c_staleness_contracts
    cargo test -p rodas5p-integrators --locked -j 1 --test b1b_gmres_contracts
    cargo test -p rodas5p-integrators --locked -j 1 --test b1_lagged_contracts
    cargo test -p rodas5p-integrators --locked -j 1 --test a1_zero_tolerance_scale
    B1C_RESULTS_OUT=$PWD/research/b1c_staleness_20261021/results.json \
      cargo test -p rodas5p-integrators --release --locked -j 1 --test b1c_staleness_contracts \
      b1c_kill_gate_measurement -- --ignored --nocapture
    python3 tools/check-research-node.py --base a5a48f77e7ac7a80b8ff86cd897110790066f7ee   # PASS, exit 0

## Deviations

- **Measured in release, not debug.** The controller's dispatch allowed release mode with `-j 1`. A
  debug re-run printed identical counters, steps and errors, so only the diagnostic wall time differs.
- **Test (c) construction.** Test (c) uses the non-normal Jacobian of ruling FP1, not y′ = +y.
- **Extra file edited.** `tests/b1b_gmres_contracts.rs:156` gained `..Rodas5pLaggedOptions::default()`
  (ruling B2a).
- **Exports.** The new driver-id constants (`RODAS5P_LAGGED_{RICHARDSON,GMRES}_BOUNDED_DRIVER_ID`) and the
  new API are exported from `lib.rs`.
