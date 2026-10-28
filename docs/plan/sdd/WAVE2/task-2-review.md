# Task 2 review: B1b-gmres (a5a48f7..c8b7e11)

Reviewed from the review package diff (single pass), plus four focused out-of-diff reads named below.

### Spec Compliance

- ✅ Spec compliant. `StageSolver::{Richardson (default), Gmres}` (rodas5p_lagged.rs diff 219-239), right-preconditioned
  by the stored LU (`StoredLu::side() = Right`, diff 669-671), restart 10 (diff 196, 428), stopping on explicit
  `WRMS(P^-1 r_k)` at k = 0 and after every column (diff 414-457, documented in module docs diff 149-153), refresh on
  contraction > 0.5 or a 7th column (diff 449-456), preregistration, forced-refresh = fast at 1e-12 on Robertson and
  HIRES (test diff 1173-1219), JVP path with iterations > 0 (diff 1275-1291).
- Deviation (disclosed, accepted): the brief named `gmres_into`; a new monitored entry point was added instead
  (gmres_into.rs diff 1666-1848). `gmres_into` cannot test between columns, and the brief's fallback (test at each
  exit) would need re-runs that are not one GMRES. Justified; see Important 1 for how it was done.
- ⚠️ Cannot verify from the local checkout: that 0879a7e was *pushed* before the measurement run (no remote-tracking
  ref in the worktree). Commit order and timestamps are right (0879a7e 06:45:11 < 8677bb5 06:56:34). Controller: check
  the remote branch's push log/reflog.

### Named-risk checks

1. Preregistration precedes measurement; constants used are preregistered. Checked `git show 0879a7e:docs/plan/nodes/b1b-gmres.md`
   against code: θ = 0.1, aᵢ (incl. 1.02), ρᵢ formula, restart 10, budget 6, q_max 0.5 are identical (diff 196-216,
   1905-1914); kill-gate thresholds 0.4/1.05/1.5 in the measurement test (diff 1590-1592) match; `git diff 0879a7e c8b7e11`
   on the node doc is append-only (the preregistration section is unchanged). PASS (push timing ⚠️ above).
2. Error growth: method-level, not an implementation defect, on the code evidence.
   - Operator: `StageOperator::apply` (diff 596-619) is the same arithmetic as Richardson's `apply_w`
     (rodas5p_lagged.rs:283-313, read): `x/(hγ) − J₀x` with the current `ws.jacobian` (rebuilt on every fresh attempt,
     rodas5p_lagged.rs:527-531) and the current h. No stale J₀ in the operator.
   - Preconditioner side and arithmetic: right; Arnoldi uses `W P⁻¹ v` (diff 1729-1741), x_k = x_cycle + P⁻¹V y
     (diff 1796-1811), bitwise equal to `solve_gmres_into` at a full cycle (test diff 1328-1381).
   - Residual units: r = c − Wx has units of y/(hγ); the tested quantity P⁻¹r has units of y and is WRMS-weighted by
     atol + rtol|yₙ|, exactly as on the Richardson path (diff 552-577 vs the wave-1 Richardson test). The correction
     x ← x_k + P⁻¹r_k uses `pres` from the last monitor call, which is computed on the accepted iterate (diff 445, 1812-1827).
   - So both arms test the same quantity with the same P and W; the only difference is which (P, iterate) pairs get
     accepted. Richardson's contraction rule bounds q = ‖I − P⁻¹W‖ ≲ 0.5, hence ‖(P⁻¹W)⁻¹‖ ≲ 2. GMRES progress does not
     bound q, and the refresh rule has no h/h_P term (`exact` only when p_h == h, rodas5p_lagged.rs:547; `refresh` at
     259-280). With P = I/(h_Pγ) − J_P and h ≫ h_P (Brusselator: 9 factorizations over 1,839 attempts from h₀ = 1e-6),
     P⁻¹W ≈ (h_P/h) on nonstiff components, so ‖W⁻¹r‖ ≈ (h/h_P)‖P⁻¹r‖ can exceed ρᵢ by orders of magnitude. The
     implementer's explanation is correct in form; the h/h_P scaling is the likely dominant term. Status I (not measured):
     a diagnostic logging h/h_P and ‖W⁻¹r‖/‖P⁻¹r‖ at each accepted lagged stage would confirm it (not run; read-only).
3. `solve_gmres_into_right_monitored`: justified (per-column stopping on a caller quantity) and correct on inspection
   (cycle-start true residual, same column ops, monitor at every column, output written only on Accept, left-pc refused,
   abandon/exhaust tested). Duplication is real: Important 1.
4. Richardson default bit-identical: `iterate` changes are a pure extraction (`check_weights` same predicate and message,
   diff 330-347 vs removed 484-497; `weighted_rms` same arithmetic, diff 547-550 vs removed 321-326); dispatch adds one
   match (diff 521-524). Pinned by endpoint bits/counts on three problems (diff 1099-1171); measured Richardson rows
   reproduce wave-1 (134/70 factorizations). PASS.

### Strengths

- Clean ledger: every W·x and LU application is charged whether a stage converges, refreshes or errors (diff 379-387);
  `operator_applications = gmres_iterations + residual_applications` asserted (diff 1248-1251).
- Bitwise equivalence test of the monitored cycle against `solve_gmres_into` is the right guard for a re-implemented loop.
- Kill gate applied without re-scoring; the interpretation is labelled as untested.

### Issues

#### Critical
None.

#### Important
1. crates/rodas5p-krylov/src/gmres_into.rs diff 1725-1794: the Arnoldi column loop and the least-squares/`linear_combination_into`
   match block are copied from `cycle_right` (the LS block is now in three places: gmres_into.rs:411-423, 492-504, and the
   new function). Drift between the copies would silently break the "bitwise the same as gmres_into" claim, which only one
   test (full cycle, n = 7) guards. Fix: factor the column step and the LS-on-prefix solve into shared helpers used by
   `cycle_right` and the monitored path (the self-imposed "existing functions untouched" rule does not outweigh this; the
   existing bitwise tests would cover the refactor).

#### Minor
1. gmres_into.rs diff 1817-1840: after a happy breakdown with a `Continue` decision the outer loop restarts, the near-zero
   true residual fails the `beta > MIN_POSITIVE` check and returns `Err(LinearSolve)` instead of reporting an exhausted or
   converged solve; in the driver that error aborts the attempt via `?` (diff 388). Return `accepted: false` (or hand the
   breakdown to the monitor) instead.
2. rodas5p_lagged.rs diff 388-389: on a krylov error mid-stage the columns are not added to `gmres_iterations`
   (report lost), so the ledger identity breaks on those runs (disclosed as concern 3). Return the partial count in the error path.
3. tests diff 1173-1219: the forced-refresh test never enters GMRES (asserts `gmres_iterations == 0`); as the brief asked,
   but no test checks a lagged GMRES stage solution against the direct solve on a stale P (e.g. ‖W x − c‖ and ‖x − W⁻¹c‖
   vs ρᵢ). Such a stage-level test would also have exposed the ‖W⁻¹r‖ vs ‖P⁻¹r‖ gap directly.
4. Per column the GMRES arm applies the LU three times and W twice (x_k formation + explicit residual), giving the
   70x/230x wall-time ratio; diagnostic only here, but any successor card should form P⁻¹r_k from the Arnoldi relation
   instead of an explicit residual.

### Assessment

**Task quality:** Needs fixes (Important 1 only; the measurement and FAIL verdict stand independently of it).

**Reasoning:** The GMRES arm builds the right operator, preconditioner side and residual units, the default is
bit-identical, and the preregistered gate was applied as written; the error growth follows from a refresh rule that no
longer bounds ‖(P⁻¹W)⁻¹‖ (dominantly h/h_P), not from a code defect. The monitored entry point is justified but
copies the Arnoldi/LS logic a third time.
