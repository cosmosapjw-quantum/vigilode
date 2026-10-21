# Task 3 review — B1-lagged (9b5d083..16f6631)

Reviewed from the review package `/home/claude/wt/b1-lagged/.superpowers/sdd/IMPLEMENTATION_WAVE1/review-9b5d083..16f6631.diff` (read in two passes, lines 1-1101 and 1101-1477; nothing cut off mid-function). No tests re-run. Line numbers below are from the diff file (D:n) or from the new files.

### Spec Compliance

- ✅ Spec compliant. All brief items are present:
  - New driver `integrate_rodas5p_lagged_observed(problem, span, y0, adaptive, output, options)` in `rodas5p_lagged.rs` (D:686-837), exported from `lib.rs` (D:31, D:53-58).
  - The fast driver's `Workspace`, `factor`, `solve`, `choose_lu` and `failure_kind` are reused. `rodas5p_fast.rs` changes are visibility only (D:84-252): no arithmetic line changed.
  - J₀ is built at every new state (D:569-583). Only the LU is lagged.
  - Richardson starts from x = P⁻¹c (D:631-634), then x ← x + P⁻¹(c − Wx) (D:519-530).
  - Stopping rule is ρ_i = θ/(8·max(a_i,1)), θ = 0.1. The a_i are constants with a T1 doc comment (D:337-359).
  - Refresh happens on q > 0.5 (D:539-544), on the 6-iteration budget (D:547-548), and before the first step (D:584-585). Each refresh is counted in `direct_factorizations` (via `factor`) and in `LaggedWork` by reason (D:437-458).
  - Abandoned iterations stay counted (D:524, D:542, D:547).
  - Equivalence test: forced refresh vs the fast driver on Robertson and HIRES at 1e-12 relative (D:1088-1140).
  - Measurement test is `#[ignore]`d and labelled diagnostic (D:1213-1339).
- ⚠️ Cannot verify from the diff alone:
  - The claim that outputs are bit-identical under forced refresh. The committed test asserts 1e-12 only. The one-off `assert_eq!` run is not in the repository.
  - The trace behind the "why it fails" diagnosis. It was uncommitted and is labelled as interpretation.
  - Whether other nodes' results in the shared target dir were linked against a stale `rodas5p-integrators`. This is the implementer's concern 1, and the controller should check it.

### Named-risk checks

1. **Unit factor (scaled W vs U-form error scale).** I checked the algebra in the module doc (D:303-321).
   - r = c − Wx has units of f. W and P = I/(h_Pγ) − J_P both have units of 1/time, so P⁻¹W is dimensionless. Then P⁻¹r_k = P⁻¹W(W⁻¹c − x_k) = −(I−T)e_k, which is a state increment of the U form. The factor is 1.
   - Plan normalisation: P_T1⁻¹ r_T1 = (h_Pγ P)⁻¹(hγ r) = (h/h_P) P⁻¹r, i.e. P⁻¹r = (h_P/h) P_T1⁻¹ r_T1, as written.
   - It also matches T9's stopping quantity ‖W⁻¹r‖, because W_T1⁻¹ r_T1 = W⁻¹r exactly.
   - Verdict: the factor is correct, and the dimensional argument is right.
2. **Residual uses current J₀, never J_P.** I checked `apply_w` (D:461-491) and `factor` (rodas5p_fast.rs:205-225).
   - JVP path: `jvp(t, y, x, wx)` is called at the attempt's own (t, y). `JvpFn` writes into `out` (problem.rs:27), and `wx` is zeroed first anyway.
   - Matvec path: it uses `ws.jacobian`, which holds J₀ of the current state. It is rebuilt when `fresh`, and kept across a retry from the same state.
   - `factor` writes `w` or `w_dense`, never `jacobian`, so P never overwrites J₀.
   - `p_generation` is bumped before the Jacobian build (D:572), so a stored P is never treated as exact at a new state.
   - Verdict: correct.
3. **Mid-stage refresh restart and counters.** I checked D:635-648 and D:437-458.
   - After a refresh, `stage_rhs` is restored from `self.rhs` (= c), the direct solve runs, and `exact = true` holds for the remaining stages. Earlier stages keep their converged lagged values, which is legitimate.
   - `preconditioner_refreshes` is charged before `factor`, so a failed factorization is still counted, and `direct_factorizations` is charged inside `factor`. The identity refreshes = Σ reasons = direct_factorizations is asserted (D:1160-1164).
   - exact + lagged + refreshed = 8·attempts is asserted (D:1174-1177). Abandoned iterations are added as k (contraction) or 6 (budget), and they were already in `richardson_iterations`.
   - A failed attempt sets `valid = false` (D:770), which forces an Initial refresh on the next attempt.
   - Verdict: consistent.
4. **Kill-gate faithfulness.**
   - The constants 0.1, 0.5, 6 and the a_i are exported and asserted (D:1077-1081).
   - Gate thresholds are 0.4, 1.05 and 1.5 (D:1327-1329).
   - The configuration matches `stiff_benchmark.rs`: `adaptive_config` at :795-806 with initial_step 1e-6 and min_step 1e-14, the HIRES span and atol_scale 1e-4 at :775-776, and `brusselator_problem(50)` with atol_scale 1 at :762, 789-790.
   - Recomputed from the recorded numbers:
     - HIRES: 134/205 = 0.654 > 0.4, and 0.410/0.259 ≈ 1.58 > 1.5, so FAIL.
     - Brusselator: 70/82 = 0.854 > 0.4, so FAIL.
   - Verdict: both FAIL verdicts follow from the recorded numbers, and nothing was tuned.

### Strengths

- The unit-factor derivation is short, correct, and placed in the module doc.
- Counting is thorough: refreshes are split by reason, and abandoned iterations are kept. The identities are asserted in tests, not only printed.
- Using the Jacobian generation to decide "P = W exactly" avoids a subtle false-exact case at a new state with the same h.
- The forced-refresh arm makes the fast-driver equivalence a structural property (no iteration ever runs), and the test asserts zero Richardson work.
- The node doc records the FAIL honestly, with a diagnosis labelled as interpretation, and lists untested follow-ups without trying them.

### Issues

#### Critical
None.

#### Important
None.

#### Minor
1. **WRMS weights differ from the brief** (rodas5p_lagged.rs, D:590-592).
   - The brief says "WRMS uses the step's error scale". The code uses atol + rtol|y_n|, which is stricter than the step scale atol + rtol·max(|y_n|,|y_new|).
   - The deviation is disclosed. It can only add iterations, so it biases toward FAIL. Its size per step is about |y_new|/|y_n|, far smaller than the 0.654 vs 0.4 margin, so the verdict is unaffected.
   - Suggested fix: note in the node doc that the deviation is verdict-conservative and does not change it.
2. **The convergence test lags the last correction by one** (D:527-538).
   - The test is on P⁻¹r of the pre-correction iterate. The 6th correction is applied but never credited, so the effective budget is about one factor q short of "6 iterations of credited reduction".
   - This is a standard convention, and the diagnosis (needed reduction ~1e-6, q^6 ≈ 5e-6) already reflects it.
   - Suggested fix: state the convention explicitly in the doc next to `RODAS5P_LAGGED_MAX_ITERATIONS`.
3. **The JVP-path assertion can pass vacuously** (b1_lagged_contracts.rs, D:1181-1185).
   - `jvp_applications == operator_applications` holds trivially when zero iterations run.
   - Suggested fix: add `assert!(w.richardson_iterations > 0)` for the default-option run.
4. **The contraction estimate ignores the x₀ = P⁻¹c norm** (D:518, D:539).
   - The first ratio is formed only at k = 2.
   - This matches the brief's q_k definition and is not a defect, but it is undocumented.
5. **`Rodas5pLaggedResult` has no doc comment** (D:402-408), unlike the other public items. It is also not `Serialize`, while `LaggedWork` is.
6. **Process: the recorded runs used an extra cargo `--config`** (`profile.dev.package.rodas5p-integrators.codegen-units=255`).
   - It is disclosed. It does not change FP semantics, but it is outside the controller's exact command set.
   - The underlying shared-target-dir cross-linking hazard (report concern 1) is a controller-level issue for every node's results.

### Assessment

**Task quality:** Approved

**Reasoning:** The driver implements T1/T2/T9 as briefed. The unit factor (1) is correctly derived, the residual uses J₀ only, and refresh/restart accounting is consistent and asserted. The preregistered FAIL on both problems follows directly from the recorded counts. Remaining items are documentation and test-tightening polish.
