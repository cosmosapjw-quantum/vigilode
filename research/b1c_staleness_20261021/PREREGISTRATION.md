# B1c-staleness: preregistration

Node `b1c-staleness` (production wave 3, Task 2 of `docs/plan/IMPLEMENTATION_WAVE3.md`; plan §5 T1, T2,
T9 and §6 B1). Ledger row **L-0141**. This file was written, committed and pushed before the measurement
run. The results are appended below the line "Results" after the run, and nothing above that line changes.

Code under test: branch `production/b1c-staleness` at `5f5eecf26482e41a0ac6cf746f6e1a4ef324d287`
(`crates/rodas5p-integrators/src/rodas5p_lagged.rs`, `StalenessRule::Bounded`).

## Question

The B1 (Richardson) and B1b (GMRES) cards FAILED. Their refresh rule had no h/h_P term, so the stopping
test on ‖P⁻¹r‖ under-reported ‖W⁻¹r‖ once P was stale (B1b: Brusselator 980 against 82 accepted steps,
error 11.9x). Does a refresh rule driven by Lemma B1c's bound, with the stage stop corrected by
(1 − q_pred), pass the same B1 kill gate?

## Mathematics (T-status in exact arithmetic, rounding ignored)

The driver's scaled form is W = I/(hγ) − J and P = I/(h_P γ) − J_P, with γ = 0.21193756319429014. J is
the current step's Jacobian, evaluated every step. J_P and h_P are the Jacobian and the step size at P's
factorization. The weights are s_i = atol + rtol |y_n,i|. ‖x‖_S = max_i |x_i| / s_i. The induced norm is
‖A‖_S = max_i Σ_j |A_ij| s_j / s_i. The logarithmic norm is μ_S(A) = max_i (A_ii + Σ_{j≠i} |A_ij| s_j / s_i).

**Lemma B1c.** If h, h_P > 0 and h_P γ μ_S(J_P) < 1, then P is nonsingular and

    T := I − P⁻¹W = (I − h_PγJ_P)⁻¹ [(1 − h_P/h) I + h_Pγ (J − J_P)],
    ‖T‖_S ≤ q_pred := (|1 − h_P/h| + h_Pγ ‖J − J_P‖_S) / (1 − h_Pγ μ_S(J_P)).

*Proof.* P = (h_Pγ)⁻¹(I − h_PγJ_P). The resolvent bound ‖(I − aA)⁻¹‖ ≤ 1/(1 − aμ(A)) holds for a ≥ 0
with aμ(A) < 1 (Lemma 1, in the norm ‖·‖_S). Next, P − W = (h_Pγ)⁻¹[(1 − h_P/h) I + h_Pγ(J − J_P)], and
T = P⁻¹(P − W). ∎

**Corollary (a-posteriori stage error).** If q_pred < 1, then for any iterate x with r = c − Wx,
‖x − W⁻¹c‖_S ≤ ‖P⁻¹r‖_S / (1 − q_pred). Since WRMS(v) ≤ ‖v‖_S, the stop
‖P⁻¹r_k‖_S ≤ (1 − q_pred) ρ_i implies WRMS(x_k − W⁻¹c) ≤ ρ_i, where ρ_i is the T9 target
`rodas5p_lagged_stage_targets()` (θ = 0.1).

If h_Pγμ_S(J_P) ≥ 1, the lemma gives no bound and q_pred = +∞. A zero weight s_i = 0 also gives
q_pred = +∞ (controller ruling B2d).

## Rules (constants fixed now, never tuned)

- The check runs before an attempt's stage solves, only when P is valid and no Initial or Forced refresh
  is due on that attempt (ruling B2c). The attempt refreshes P iff q_pred > q_max = 0.5
  (`RODAS5P_LAGGED_STALENESS_Q_MAX`). With J = J_P and μ_S ≤ 0 this means refreshing when
  h/h_P ∉ [2/3, 2].
- Stage stop: ‖P⁻¹r_k‖_S ≤ (1 − q_pred) ρ_i, on both stage solvers. It replaces WRMS(P⁻¹r_k) ≤ ρ_i in the
  stopping test only.
- The GMRES arm is otherwise unchanged: restart 10 (`RODAS5P_LAGGED_GMRES_RESTART`), column cap 6
  (`RODAS5P_LAGGED_MAX_ITERATIONS`), and the progress refresh (ν_k/ν_0)^{1/k} > 0.5
  (`RODAS5P_LAGGED_CONTRACTION_MAX`), on WRMS as in B1b.
- J_P is a dense copy taken at every refresh and allocated once at setup. q_pred costs O(n²) per check.

## Kill gate (§6 B1, the same as B1 and B1b)

- Problems: HIRES (n = 8, span [0, 321.8122], atol = 1e-4 rtol) and Brusselator 1D with 50 cells
  (n = 100, span [0, 10], atol = rtol), from `crates/rodas5p-integrators/tests/b1_problems/mod.rs`.
- Tolerance: rtol 1e-6, initial step 1e-6, one output interval.
- Reference: `integrate_rodas5p_fast_observed` at rtol 1e-11. Baseline "fast" is the same driver at rtol 1e-6.
- Gated arm: `StageSolver::Gmres` + `StalenessRule::Bounded`.

Per problem, **all three** must hold:

1. factorizations per accepted step ≤ 0.4;
2. accepted steps ≤ 1.05 × fast's accepted steps;
3. endpoint error ≤ 1.5 × fast's endpoint error. The error is the max over components of
   |y − y_ref| / (atol + rtol |y_ref|).

**PASS iff both problems pass.** Otherwise the verdict is FAIL. `Richardson + Bounded` is measured and
reported as context only, and it is not gated. Wall time is diagnostic only.

## Verdict rule

- The verdict is PASS or FAIL as computed by the test from the counters. It is never re-scored and
  never loosened.
- A run that cannot complete is reported, and no ledger row is written for it.
- The lagged driver stays unrouted from `Tier::Performance` in wave 3, whatever the verdict.
  - A PASS opens gates 3–4 of plan §8 in wave 4, and routing waits for those gates and the holdout.
  - A FAIL closes the B1 family until a new mechanism.

## Command

Run from the worktree root, with
`RUSTUP_TOOLCHAIN=stable CARGO_TARGET_DIR=/home/claude/targets/b1c-staleness CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`:

    B1C_RESULTS_OUT=$PWD/research/b1c_staleness_20261021/results.json \
      cargo test -p rodas5p-integrators --release --locked -j 1 \
      --test b1c_staleness_contracts b1c_kill_gate_measurement -- --ignored --nocapture

The task brief says "debug". The controller's dispatch allows release mode with `-j 1` instead, for
wall-clock reasons. The counters, steps and endpoint bits do not depend on the profile: Rust does not
contract or reassociate floating-point operations in either profile. Only the diagnostic wall time differs.

## Holdout

None in this node. The kill gate is a falsification gate on two fixed problems. Any routing decision
requires the wave-4 gates 3–4 and their holdout.

---

## Results

Run on 2026-10-21 at `5572d7d` with the command above (release, `-j 1`). Raw output:
`results.json` (sha256 `beafb0a18f56489e01f2567e3de925989b8fbaae4bc08d5d8bb9cb65861524c6`, commit `cbcffff`).
The same test in debug printed identical counters, steps and errors; only wall time differed.

| problem | arm | accepted | rejected | factorizations | fact/acc | checks | staleness refreshes | unbounded checks | endpoint err (tol units) |
|---|---|---|---|---|---|---|---|---|---|
| hires | fast | 204 | 6 | 210 | 1.0294 | - | - | - | 2.5866e-01 |
| hires | lagged-gmres-bounded (gated) | 204 | 7 | 120 | 0.5882 | 210 | 119 | 75 | 2.5975e-01 |
| hires | lagged-richardson-bounded (context only) | 204 | 6 | 161 | 0.7892 | 209 | 101 | 75 | 2.5782e-01 |
| brusselator-1d-50 (n = 100) | fast | 82 | 10 | 92 | 1.1220 | - | - | - | 1.6858e+00 |
| brusselator-1d-50 (n = 100) | lagged-gmres-bounded (gated) | 82 | 10 | 20 | 0.2439 | 91 | 19 | 0 | 1.6827e+00 |
| brusselator-1d-50 (n = 100) | lagged-richardson-bounded (context only) | 82 | 10 | 73 | 0.8902 | 91 | 8 | 0 | 1.6857e+00 |

Kill gate, gated arm `Gmres + Bounded`:

| problem | fact/acc ≤ 0.4 | steps ≤ 1.05x | error ≤ 1.5x | verdict |
|---|---|---|---|---|
| hires | 0.5882 (FAIL) | 1.0000 (pass) | 1.0042 (pass) | **FAIL** |
| brusselator-1d-50 (n = 100) | 0.2439 (pass) | 1.0000 (pass) | 0.9981 (pass) | **PASS** |

**Verdict: FAIL.** HIRES fails criterion 1 (0.588 > 0.4). Brusselator n = 100 passes all three.
The verdict is recorded as measured and is not re-scored. The B1 family is closed until a new mechanism.
