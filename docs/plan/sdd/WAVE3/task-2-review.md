# Task 2 review: b1c-staleness (75a0c6c..129aa46)

Reviewer: task reviewer (spec + quality), single pass over the review package, with focused checks named below.
Paths are under `/home/claude/wt/b1c-staleness/`. Code line numbers are as of `129aa46`.

### Spec Compliance

- ✅ Spec compliant. The implementation matches the brief as amended by rulings B2a–B2d, FP1, FP5 and R-C6. The
  measured FAIL verdict is trustworthy: it is reproducible, its arithmetic is correct, and the commit order is correct.
- ⚠️ Cannot verify from the diff: the Robertson row of `GMRES_BASELINE`
  (`crates/rodas5p-integrators/tests/b1c_staleness_contracts.rs:302`) has no independent wave-2 record. Its provenance
  rests on its presence in the RED commit `21ed134`, which predates any source change. The HIRES and Brusselator rows
  do match the wave-2 node record. To close this, the controller can run test (d) once at `75a0c6c` plus the new test
  file. This reviewer did not do so, because it would need a separate base build.

#### 1. Bound against Lemma B1c (`src/rodas5p_lagged.rs`)

- **q_pred formula.** `src/rodas5p_lagged.rs:313-357` computes the formula exactly as the lemma states it:
  - a = h_P·γ;
  - the numerator is |1 − h_P/h| + a·max_i Σ_k |J − J_P|_ik s_k/s_i, which is the induced ‖·‖_S, row-sum form with
    diagonal weights;
  - the denominator is 1 − a·μ_S(J_P), with μ_S = max_i (J_P,ii + Σ_{k≠i} |J_P,ik| s_k/s_i) (`:342-349`). This is the
    correct log norm for ‖x‖_S = max |x_i|/s_i, the max-norm conjugated by S = diag(s).
- **γ and P.** γ comes from `ws.gamma`, the coefficient snapshot value that the test pins as `GAMMA`. P is formed in
  `Workspace::factor` as −J + I/(hγ) (`rodas5p_fast.rs:796-815`, which is not modified). This matches the brief's
  scaled form.
- **J_P copy.** The copy is taken after `ws.factor` succeeds (`:414-418`). `factor` writes into `w`/`w_dense`/band
  factors, never into `ws.jacobian`, so J_P is the exact Jacobian of the stored P. I checked `rodas5p_fast.rs:802-812`
  for this named risk.
- **J at the check.** At the check, `ws.jacobian` is the current step's Jacobian: it is rebuilt when `fresh` (`:690-694`)
  and is the same state's Jacobian on a rejection retry.
- **h_Pγμ_S ≥ 1 gives +∞.** `denominator <= 0.0 || NaN` returns `+∞` (`:352`), and a non-finite q returns `+∞` (`:357`).
- **B2d (s_i = 0 gives +∞).** The guard at `:325` returns `+∞` for any weight that is not finite and positive, before
  any division. Test (b) asserts this at `tests/b1c_staleness_contracts.rs:167-170`. With `Bounded`, a zero weight
  therefore forces a refresh. That makes P equal W at h, so every stage of that attempt is direct, and the S-norm stop
  (`weighted_max`, `:360`) is never evaluated on a zero weight.
- **Refresh rule.** `q > RODAS5P_LAGGED_STALENESS_Q_MAX` (= 0.5) refreshes through `Refresh::Staleness` (`:732`). The
  inequality is strict, as specified: the endpoint h/h_P = 2/3 is not a refresh. Otherwise `stop_scale = 1 − q` (`:735`).
- **Stop criterion.** `stops()` (`:468-474`) tests `weighted_max(P⁻¹r, s) <= (1 − q_pred)·ρ_i` on all three stop sites:
  - GMRES at k = 0 (`:570`);
  - the GMRES monitor (`:594`), where `pres` is the explicit P⁻¹(c − W x_k);
  - Richardson (`:657`), where `ws.stage_rhs` is P⁻¹r_k after the solve.

  The weights are the same start-state s that the bound uses. `weighted_rms` (`:832`) uses the same weights with a
  1/n mean, so WRMS ≤ ‖·‖_S holds and the corollary gives WRMS(x_k − W⁻¹c) ≤ ρ_i. On Richardson, the iterate kept is
  x_{k+1} = x_k + P⁻¹r_k, whose error is T e_k with ‖T‖_S ≤ q < 1, so the guarantee carries over. The contraction and
  progress tests stay on WRMS (`:598`, `:661`), as the brief requires.
- **B2c (check only with a valid P and no Initial or Forced refresh due).** The check is the third arm of
  `if !valid … else if refresh_every_attempt … else if bounded()` (`:710-737`), so it runs only with a valid P and no
  Initial or Forced refresh due. Test (e) asserts `StalenessWork::default()` under forced refresh and the fast
  driver's `direct_factorizations` (`tests/…:387`).
- **Weight hoist.** The weight computation moved above the refresh decision. In `Off` mode no refresh reads the weights,
  and test (d) confirms that `Off` is bit-identical on both arms.

#### 2. Measurement

- **Commit order.** The order is RED `21ed134`, then GREEN `5f5eecf`, then PREREGISTRATION `5572d7d` (08:52:01), then
  results.json `cbcffff` (08:55:56), then ledger, node report and appended results `129aa46`. `git diff 5f5eecf 129aa46
  -- crates/` is empty, so the code that ran is the preregistered code. The `129aa46` diff of `PREREGISTRATION.md` only
  replaces the placeholder "(appended after the run)" below "## Results". Nothing above that line changed.
- **Gate arithmetic, recomputed from `research/b1c_staleness_20261021/results.json`:**

  | problem | fact/acc | step ratio | error ratio | verdict |
  |---|---|---|---|---|
  | HIRES Gmres+Bounded | 120/204 = 0.588235 | 204/204 = 1.000 | 0.2597531/0.2586579 = 1.004234 | FAIL (criterion 1) |
  | Brusselator Gmres+Bounded | 20/82 = 0.243902 | 1.000 | 0.998129 | PASS |

  - Overall: FAIL, which agrees with the file's `verdict` and the per-problem `gate` fields.
  - Refresh identity: refreshes = initial + forced + contraction + budget + staleness = `direct_factorizations` holds on
    all four lagged arms (HIRES GMRES: 1 + 119 = 120).
  - Reference: `integrate_rodas5p_fast_observed` at rtol 1e-11 (HIRES 5301 steps, Brusselator 737). The error metric is
    max_i |y − y_ref|/(atol + rtol|y_ref|) (`tests/b1_problems/mod.rs:235`), which matches the preregistration.
- **Release and debug agree.** I re-ran `b1c_kill_gate_measurement` in debug with the prescribed environment, writing to
  scratch (not to the repo). The output JSON, with `wall_ms_diagnostic` removed, is identical to the committed release
  `results.json` field for field, including every float. The implementer's claim holds.
- **Ledger L-0141.** `python3 tools/check-research-node.py --base a5a48f77e7ac7a80b8ff86cd897110790066f7ee` printed
  `PASS (141 ledger rows …)`, exit 0. The checker verifies `outputs_sha256` only, so I checked every digest separately:
  - `results.json` matches HEAD.
  - The `rodas5p_lagged.rs`, `b1c_staleness_contracts.rs`, `b1_problems/mod.rs` and `Cargo.lock` inputs match both HEAD
    and the row's `commit` (`5572d7d`).
  - The `PREREGISTRATION.md` input matches the tree at `5572d7d` (the pre-results text), not HEAD. This is correct under
    `docs/RESEARCH_LEDGER.md:38` ("inputs … as they are in the tree of `commit`"), and it matches precedent: L-0139's
    preregistration digest also differs from HEAD after its results were appended. It is not a defect.
  - Other fields: `verdict: FAIL`, `supersedes: null`, id L-0141, date 2026-10-21. R-C6 is satisfied.

#### 3. Test validity

- **Test (c), FP1 matrix.** It uses J = [[−1, 1e8], [0, −1]], y0 = (0, 1e-8), atol = rtol
  (`tests/…:182-212`). The assertions are real: `checks == attempts − 1`, `unbounded_checks == checks`,
  `staleness_refreshes == checks` and `lagged_stage_solves == 0`. It also requires fast equality of t, step and
  factorization counts, and of outputs within 1e-12 relative. All of this runs on both solvers at rtol 1e-4 and 1e-6
  (`:215-255`).
- **Test (a).** It forms T explicitly by n LU solves and asserts ‖T‖_S ≤ q(1 + 1e-12) on every finite case, plus
  `finite > 0` (`:104-147`).
- **Test (b).** It checks the 1-ulp scalar identity, B2d, and the unbounded case.
- **B2b and FP5 (GMRES `Off` pins).** `GMRES_BASELINE` is present in the RED commit `21ed134`, before any source change.
  Its HIRES row (205/10/15/4868/20) and Brusselator row (980/859/9/11401/43) match the wave-2 record
  `docs/plan/nodes/b1b-gmres.md:107,118`, which corroborates base provenance. The Richardson pins are kept unchanged.
- **No `Tier::Performance` routing.** The diff touches no file under `crates/vigilode/` and does not touch
  `rodas5p_fast.rs`. The only extra file is `tests/b1b_gmres_contracts.rs:159`, under ruling B2a.

#### 4. HIRES failure mechanism (research record, not a defect)

- **D: the unbounded checks alone nearly use up the budget.** 75 of 210 checks are unbounded, which is 0.368
  refreshes per accepted step. The gate allows at most 81 factorizations for 204 accepted steps, so 80 refreshes after
  the initial one.
- **D: the iteration cost no factorizations.** The other 44 refreshes come from finite q_pred > 0.5; the largest finite
  q_pred was 14.27. GMRES made no contraction or budget refresh on either problem. On HIRES the factorization count is
  therefore 1 + staleness refreshes, entirely the refresh rule.
- **D: the corrected stop removes the B1b failure.** Steps equal the fast driver's on both problems, and Brusselator
  goes from 980 to 82 accepted steps.
- **I: the attribution to μ_S is reported honestly.** The node report labels the attribution to weight-ratio inflation
  of μ_S as I and cites μ_S ≈ 1.7e4 at y0 to the preflight scan (`docs/plan/nodes/b1c-staleness.md`, Observations). This
  run records no per-check h_Pγμ_S trace, so the attribution is reasonably labeled interpretation, not measurement.
- **The preflight proxy is reported, not hidden.** The proxy predicted 44 of 210 unbounded and 0.39 fact/acc; the
  measurement gives 75 of 210 and 0.588. The node report states the gap.

### Strengths

- The bound function is defensive. It returns +∞ for every degenerate input, never NaN, and computes μ_S and the drift
  in one O(n²) pass. Test (b) pins it to the ulp.
- B2c is enforced structurally by the `else if` chain, not by a flag, and test (e) checks it by counter equality with the
  fast driver.
- The measurement is reproducible bit for bit across build profiles, and it computes rather than asserts the verdict.
  The preregistration discipline is clean: code is frozen before the preregistration, and the results are appended
  only below the marker.
- The node report separates D from I and does not soften the FAIL.

### Issues

#### Critical (Must Fix)

None.

#### Important (Should Fix)

None.

#### Minor (Nice to Have)

- **M1. A "verdict" field on ungated arms.** `tests/b1c_staleness_contracts.rs:596-606` and `:622` write a `criteria`
  block with `"verdict": "FAIL"` onto the ungated Richardson+Bounded arms of `results.json`. The gate is the
  `problems[].gate` field, and the arm name says "context only". Still, a field named `verdict` on an ungated arm can be
  misread by a downstream consumer. If the file is regenerated in a later node, the field could be named
  `would_pass_gate` on context arms. Do not edit the committed results for this.
- **M2. Unpinned zero-weight driver path.** The driver-level effect of a zero weight under `Bounded` (refresh, then all
  stages direct, then no S-norm evaluation on a zero weight) is correct by construction (`src/rodas5p_lagged.rs:325`,
  `:710-737`). No contract exercises it end to end: test (b) covers only the bound function. A follow-up test could use
  `atol = 0` with an exactly zero component, as described in B2d.

### Assessment

**Task quality:** Approved

**Reasoning:** The bound, the refresh rule, the corrected stop and the B2c gating match Lemma B1c and the rulings line
for line. The FAIL verdict is reproduced exactly in debug, its gate arithmetic recomputes, and its ledger row is
schema-valid with digests bound correctly. The HIRES failure comes from the refresh rule (75 unbounded checks, μ_S
inflated by the weight ratios), not from a code defect, and it is reported as such.
