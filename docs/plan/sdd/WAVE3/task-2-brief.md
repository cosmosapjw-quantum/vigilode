## Task 2: B1c-staleness — lagged LU with a T2 staleness bound and an a-posteriori stage-error test (node id `b1c-staleness`)

Plan §5 T1, T2, T9; §6 B1. This is a new card. The B1 (Richardson) and B1b (GMRES) cards FAILED. Their
recorded method-level cause is that the refresh rule had no h/h_P term, so the stopping test on ‖P⁻¹r‖
under-reported ‖W⁻¹r‖ once P was stale (B1b: Brusselator 980 against 82 accepted steps, error 11.9x).

**Mathematics (preregistered; T-status in exact arithmetic, rounding ignored).** The driver's scaled form is
W = I/(hγ) − J and P = I/(h_P γ) − J_P, with γ = 0.21193756319429014. J is the current step's Jacobian,
evaluated every step. J_P and h_P are the Jacobian and step size at P's factorization. The weights are
s_i = atol + rtol |y_n,i| > 0, the lagged driver's start-state weights; the lagged driver refuses zero
weights. ‖x‖_S = max_i |x_i| / s_i. The induced norm is ‖A‖_S = max_i Σ_j |A_ij| s_j / s_i, and the
logarithmic norm is μ_S(A) = max_i (A_ii + Σ_{j≠i} |A_ij| s_j / s_i).

*Lemma B1c.* If h, h_P > 0 and h_P γ μ_S(J_P) < 1, then P is nonsingular and

```latex
T := I - P^{-1}W = (I - h_P\gamma J_P)^{-1}\big[(1 - h_P/h)\,I + h_P\gamma\,(J - J_P)\big],\qquad
\|T\|_S \le q_{\rm pred} := \frac{|1 - h_P/h| + h_P\gamma\,\|J - J_P\|_S}{1 - h_P\gamma\,\mu_S(J_P)} .
```

*Proof.* P = (h_Pγ)⁻¹(I − h_PγJ_P). The log-norm resolvent bound ‖(I − aA)⁻¹‖ ≤ 1/(1 − aμ(A)) holds for
a ≥ 0 with aμ(A) < 1; this is Lemma 1 in the norm ‖·‖_S. Next,
P − W = (1/(h_Pγ) − 1/(hγ)) I + (J − J_P) = (h_Pγ)⁻¹[(1 − h_P/h) I + h_Pγ(J − J_P)], and
T = P⁻¹(P − W). ∎

*Corollary (a-posteriori stage error).* If q_pred < 1, then for any iterate x with residual r = c − Wx,
‖x − W⁻¹c‖_S = ‖(I − T)⁻¹P⁻¹r‖_S ≤ ‖P⁻¹r‖_S / (1 − q_pred), by T2's Neumann bound. WRMS(v) ≤ ‖v‖_S, so

  ‖P⁻¹r_k‖_S ≤ (1 − q_pred) ρ_i  implies  WRMS(x_k − W⁻¹c) ≤ ρ_i,

where ρ_i is the existing T9 target (`rodas5p_lagged_stage_targets()`, θ = 0.1), in the units the driver
already uses (the module docs of `rodas5p_lagged.rs`).

*Notes.*
- With J = J_P and μ_S ≤ 0, q_pred ≤ |1 − h_P/h|. This is tighter than T2 Corollary 2a's 2|1 − h/h_old|,
  because the scaled P differs from the unscaled one by the factor h/h_P. GMRES is invariant to that factor
  and Richardson is not.
- The "Jacobian age" is measured by the drift h_Pγ‖J − J_P‖_S, not by a step count. A count cap would be an
  untuned constant that T2 does not supply.
- If h_Pγμ_S(J_P) ≥ 1, the lemma gives no bound and q_pred = +∞.

**Rules (constants fixed now, never tuned):**
- Refresh before an attempt's stage solves iff q_pred > q_max = 0.5. That is T2's production rule q_max.
  With J = J_P and μ_S ≤ 0, the rule means refreshing when h/h_P ∉ [2/3, 2].
- Stage stop: ‖P⁻¹r_k‖_S ≤ (1 − q_pred) ρ_i. This replaces WRMS(P⁻¹r_k) ≤ ρ_i in the stopping test only.
- The GMRES arm is otherwise unchanged:
  - restart `RODAS5P_LAGGED_GMRES_RESTART` = 10;
  - column cap `RODAS5P_LAGGED_MAX_ITERATIONS` = 6;
  - progress refresh (ν_k/ν_0)^{1/k} > `RODAS5P_LAGGED_CONTRACTION_MAX` = 0.5, on WRMS as in B1b.
- q_pred costs O(n²) per attempt (a dense J_P copy kept from the last refresh, allocated once at setup).

**Kill gate (§6 B1, the same as B1/B1b):** HIRES and Brusselator 1D n = 100 (`tests/b1_problems/mod.rs`),
rtol 1e-6, reference = the fast driver at rtol 1e-11, arm `StageSolver::Gmres` + `StalenessRule::Bounded`.
Per problem, all three must hold:
- factorizations per accepted step ≤ 0.4;
- accepted steps ≤ 1.05 × fast;
- endpoint error ≤ 1.5 × fast's (tolerance units).

PASS iff both problems pass. Richardson + Bounded is measured and reported as context only, not gated.

**Files:**
- Modify: `crates/rodas5p-integrators/src/rodas5p_lagged.rs` only. `lib.rs` exports only if needed.
- Test: `crates/rodas5p-integrators/tests/b1c_staleness_contracts.rs` (new), reusing `tests/b1_problems/mod.rs`.
- Create: `research/b1c_staleness_YYYYMMDD/{PREREGISTRATION.md, results.json}`, ledger row **L-0141**,
  `docs/plan/nodes/b1c-staleness.md`.

**Interfaces (all new):**
- `pub enum StalenessRule { #[default] Off, Bounded }`.
- `Rodas5pLaggedOptions::staleness: StalenessRule`. The struct stays `Copy + Eq + Default`; `Off` is the
  wave-2 behaviour bit for bit.
- `pub const RODAS5P_LAGGED_STALENESS_Q_MAX: f64 = 0.5`.
- `pub fn rodas5p_staleness_bound(h: f64, h_p: f64, gamma: f64, j: &DenseMatrix, j_p: &DenseMatrix, s: &[f64]) -> f64`,
  which returns q_pred (+∞ when unbounded).
- `#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)] pub struct StalenessWork { pub checks: u64, pub staleness_refreshes: u64, pub unbounded_checks: u64, pub q_pred_max: f64 }`,
  carried as `Rodas5pLaggedResult::staleness: StalenessWork`. It is a separate struct, because `LaggedWork`
  derives `Eq` and cannot carry an `f64`.
- Driver ids: `"rodas5p-lagged-gmres-bounded-v1"` and `"rodas5p-lagged-richardson-bounded-v1"`.
- Constraints: no edit to `rodas5p_fast.rs`, and no facade edit.

- [ ] **Step 1: Write failing tests** in `b1c_staleness_contracts.rs`:
  - (a) `staleness_bound_dominates_explicit_t_norm`: for Robertson and HIRES Jacobians at three states,
    h_P/h ∈ {0.5, 0.8, 1, 1.25, 2} and J_P from an earlier state, form T = I − P⁻¹W explicitly (n LU
    solves) and assert ‖T‖_S ≤ `rodas5p_staleness_bound(..)` × (1 + 1e-12) whenever the bound is finite.
  - (b) `bound_is_exact_for_scalar_step_change`: n = 1, J = J_P = λ ∈ {−1e6, −1, 0}; the bound equals
    |1 − h_P/h| / (1 − h_Pγλ) within 1 ulp.
  - (c) `unbounded_log_norm_refreshes_every_attempt`: y' = +y (μ > 0), with h large enough that
    h_Pγμ_S ≥ 1 after the first step. `unbounded_checks` equals `checks`, and outputs equal
    `integrate_rodas5p_fast_observed` within 1e-12 relative.
  - (d) `bounded_off_is_wave2_bitwise`: `StalenessRule::Off` reproduces the endpoint bits and counters
    pinned by `b1b_gmres_contracts` on Robertson, HIRES and Brusselator.
  - (e) `forced_refresh_bounded_equals_fast`: `refresh_every_attempt` + `Bounded`, within 1e-12 relative on
    Robertson and HIRES at rtol {1e-4, 1e-6, 1e-8}.
  - (f) `staleness_ledger_identity`: refreshes = initial + forced + contraction + budget + staleness.
- [ ] **Step 2: Run** `cargo test -p rodas5p-integrators --locked -j 1 --test b1c_staleness_contracts`; expect
  a compile failure.
- [ ] **Step 3: Implement** the bound, the refresh check before each attempt's stage loop (a new private
  `Refresh::Staleness` variant), and the corrected stopping test on both stage solvers.
- [ ] **Step 4: Run** the new tests plus `b1b_gmres_contracts`, `b1_lagged_contracts` and
  `a1_zero_tolerance_scale`; all pass.
- [ ] **Step 5: Preregister.** Commit and push `PREREGISTRATION.md` before any measurement. It holds this
  task's math, rules, kill gate, command and the verdict rule (PASS/FAIL only; a run that cannot complete is
  reported, and no row is written). A PASS/FAIL row is also what keeps the ledger stall rule satisfied after
  two INCONCLUSIVE bench nodes.
- [ ] **Step 6: Measure.** Run the `#[ignore]`d `b1c_kill_gate_measurement` (debug; counters deterministic;
  wall time diagnostic). It writes `results.json` when `B1C_RESULTS_OUT` is set, with every counter of fast,
  Gmres+Bounded and Richardson+Bounded, `StalenessWork`, and PASS/FAIL per problem. Commit the numeric file
  first.
- [ ] **Step 7: Ledger L-0141** (verdict PASS or FAIL as measured, `supersedes: null`), the node report, and
  the checker exit code. On PASS the report says that routing waits for wave-4 gates 3–4; on FAIL it says
  "B1 family closed until a new mechanism".

