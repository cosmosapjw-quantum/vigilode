"""Human-readable parts of each task section (rendered by build.py)."""

WHY = {}
NOTES = {}
RED_REASON = {}
SPEC = {}  # full body for governance and spec-only tasks

WHY["G0"] = (
    "Every later check compares against the state at the design commit. G0 proves that this state builds, "
    "that the skeleton helpers reproduce the registered counters (rotation 892 attempts / 5,371 RHS / 355 "
    "discarded JVPs; stiff-forced 66 / 456), and that the gate itself validates."
)
SPEC["G0"] = """**Steps.**

1. `python3 tools/agent_gate.py validate` must print `OK: 23 tasks, anchors verified at 4339facc`.
2. `python3 tools/agent_gate.py start G0`.
3. Run the commands of the card in order. Record each with its exit code in `self_audit.commands_run`.
4. Write `restated_goal`, `spec_steps_done` (the numbers of the steps above), `invariants_checked` (`INV-01`: the skeleton test result; `INV-15`: "held: PROGRESS entry written"), `no_unrequested_changes: true`. `symbols_verified` may stay empty (governance task).
5. Append the first PROGRESS.md entry, including the toolchain used and the wall time of the full suite.
6. `finish G0`, commit (only PROGRESS.md changes).

**Expected.** All commands exit 0. If the pinned toolchain cannot be installed, use `RUSTUP_TOOLCHAIN=stable` for every command and for `finish`, and record it.
"""

WHY["G1"] = (
    "Ledger row L-0206 (WorkModel v2, PASS) cites commit d164d116, which changed six hand-typed \"measured\" "
    "values and moved the admission threshold from `< 200` to `<= 300` after the preregistered run failed "
    "(audit A-1). Rerunning the preregistered inputs of aafc2e0b gives FAIL (maximum relative error 0.5015, "
    "transition-39 gate failed). The row must be superseded by an INVALID row; the as-preregistered rerun is "
    "kept as numeric evidence."
)
SPEC["G1"] = """**Human step before `start`.** A human reads this section and runs `python3 tools/agent_gate.py approve G1 <name>`.

**Step 1 — reproduction script.** Create `tools/agent_reproduce_e1_prereg.py` with exactly this content:

```python
{{G1_SCRIPT}}
```

**Step 2 — run it.** `python3 -I tools/agent_reproduce_e1_prereg.py` must print `verdict FAIL -> research/switcher_work_model_v2_20261103/RESULTS_AS_PREREGISTERED.json`. In the written file, the case `transition39_n2_mf` must have `relative_error` 0.5015094339622641. Any other verdict or value: abort.

**Step 3 — INVALIDATION.md.** Create `research/switcher_work_model_v2_20261103/INVALIDATION.md` with these sections and nothing else: `# L-0206 invalidation`; `## Finding` (the paragraph "Why" above, verbatim); `## Reproduction` (the command of step 2 and the printed line); `## Consequence` ("WorkModel v2 is not admitted. `route_registry` uses WorkModel V1 (task F3). A new calibration needs measured callgrind data under a frozen script (task X11).").

**Step 4 — ledger row.** Append one row with the script of section 13.2 and these values:

| field | value |
|---|---|
| id | one more than the last row (L-0207 if the ledger still ends at L-0206) |
| date | the day of the run |
| commit | `4339facc62258b5fb9a2469f16a3af8b93763e24` (on the remote branch; the reproduction itself lands in the G1 commit) |
| command | `UNRECORDED: python3 -I tools/agent_reproduce_e1_prereg.py lands in the G1 commit; rerun after that commit is pushed` |
| profile | `VIGILODE_SWITCHER_V2_E1_INVALIDATION` |
| inputs_sha256 | `research/switcher_work_model_v2_20261103/PREREGISTRATION.md` and `research/switcher_work_model_v2_20261103/eval_work_model.py`, both hashed at the commit above (`sha(path, rev=...)`) |
| outputs_sha256 | `research/switcher_work_model_v2_20261103/RESULTS_AS_PREREGISTERED.json` (`sha(path)`) |
| claim | `L-0206 is invalid: its measured Ir values are literals in eval_work_model.py, six of which and the admission threshold (< 200 to <= 300) changed in d164d116 after the preregistered run; rerunning the preregistered aafc2e0b inputs gives FAIL (max relative error 0.5015, transition-39 gate failed).` |
| verdict | `INVALID` |
| supersedes | `L-0206` |

**Step 5.** Run the card commands, fill the self-audit, `finish G1`, commit.

**Human step after the commit.** Push the branch, then append a FAIL row for the as-preregistered rerun whose `commit` is the pushed G1 commit and whose `command` is the real command. Agents do not write that row (INV-13: its commit is not on a remote until the human pushes).
"""

WHY["G2"] = (
    "L-0205 claims \"confirmatory admission of post-audit adapter hardening\". The campaign behind it only ran "
    "`SwitchingConfig::evidence_driven`, so it shows default-path neutrality, not admission of the opt-in "
    "strict, seed, sentinel and dominated-comparator flags (audit A-2). A correction row narrows the claim; the "
    "data stay valid."
)
SPEC["G2"] = """**Human step before `start`.** `approve G2 <name>`.

**Step 1.** Read the L-0205 row (`grep '"L-0205"' research/LEDGER.jsonl`; that row is written without spaces after the separators). Copy its `commit`, `command`, `profile`, `inputs_sha256` and `outputs_sha256` unchanged.

**Step 2.** Check that every `outputs_sha256` entry of L-0205 still matches the file in the tree (`sha(path)`). If one does not match, abort.

**Step 3.** Append a row with the script of section 13.2: the copied fields; `id` = next id; `date` = today; `verdict` = `PASS`; `supersedes` = `L-0205`; `claim` = `Re-scope of L-0205: default-path neutrality of the post-audit adapter hardening (9 direct cells bit-identical, switching cells with identical counters and decisions between 596dcdfa and a4b08c59 under SwitchingConfig::evidence_driven); the opt-in strict, seed, sentinel and dominated-comparator flags were not exercised and are not admitted by this row.`

**Step 4.** Run the card command, fill the self-audit, `finish G2`, commit.
"""

WHY["F1"] = (
    "`SwitchingConfig::adaptive_evidence` silently started to build `EntryProbeAdaptation { loss_margin_backoff: "
    "true, .. }`, and `AutomaticOde::solve_adaptive` calls this factory, so the change reached the automatic "
    "default path without an admission node (audit A-4). F1 restores `EntryProbeAdaptation::default()` "
    "(backoff off). `route_registry` keeps the backoff as its opt-in."
)
RED_REASON["F1"] = "at base the factory sets `loss_margin_backoff: true`, so `assert!(!adaptation.loss_margin_backoff)` fails."
NOTES["F1"] = [
    "The existing contract `b1_loss_margin_backoff_contracts` asserted the A-4 behaviour; its one assertion is inverted (listed under \"Existing tests that change\"). The rest of that test sets the flag explicitly and stays as it is.",
    "Expected counters after F1 (adaptive_evidence): rotation (892, 5371, 355), stiff-forced (58, 394). Before: rotation (882, 5299, 119). The loss of the backoff benefit on rotation is intended (erratum E-4).",
    "Do not touch `route_registry`; its `loss_margin_backoff: true` is the second occurrence of that text in the file and must remain.",
]

WHY["F2"] = (
    "`integer_work_v2` returns instruction units (callbacks × κ + overhead). The pilot guard compares its ceiling "
    "with callback units. Under WorkModel V2 the ceiling was therefore about κ = 50 times too large and losing "
    "pilots ran to their attempt budget (audit A-5): on rotation 1,509 discarded RODAS JVPs under V2 against 149 "
    "under V1. F2 converts the V2 cost to a callback ceiling with rounding that keeps the comparison "
    "decision-preserving: a pilot whose charged callbacks exceed `floor((E−1)/κ)` cannot be strictly cheaper than "
    "the incumbent E, because its V2 cost is at least κ·c ≥ E."
)
RED_REASON["F2"] = "at base the V2 run discards 1,509 RODAS JVPs against 149 for V1 (bound: V1 + 50)."
NOTES["F2"] = [
    "`RhsCost::ir_cost()` already exists and returns κ (Cheap 50, Moderate 500, Expensive 5000, or the declared value). Grep it before use (R-03).",
    "Keep the V1 arms bit-identical: entry ceiling `E − 1`, return ceiling `E`.",
    "`kappa == 0` returns `None` (no ceiling armed). Do not map it to 0: a zero ceiling would stop every pilot.",
    "Edit 4 inserts three unit tests in front of an existing unit test in the `#[cfg(test)]` module of `switching.rs`; the existing `#[test]` attribute line above the anchor now belongs to the first new test, and the edit adds a fresh `#[test]` line for the existing one.",
]

WHY["F3"] = (
    "WorkModel v2 rests on the invalid row L-0206 (task G1). Until X11 re-runs the calibration with measured data, "
    "the `route_registry` factory uses WorkModel V1. With F2 and F3, `route_registry` on stiff-forced goes from "
    "319 attempts / 1,968 RHS calls (base) to 59 / 400."
)
RED_REASON["F3"] = "at base the factory sets `WorkModelVersion::V2`."
NOTES["F3"] = [
    "The existing contract `route_registry_factory_defaults_contract` pins `V2`; that one line changes to `V1` (listed under \"Existing tests that change\").",
    "Only the `work_model` line of `route_registry` changes; the anchor includes the next line (`progress_bound: true,`) to make it unique.",
]

WHY["F4"] = (
    "The progress screen discards an entry pilot that cannot reach the window end before its ceiling. It assumed "
    "16 (17 non-autonomous) callbacks per accepted RODAS5P attempt. Under V2 the ceiling was in Ir, so the screen "
    "never fired; with callback ceilings (F2) the constant 16 is unsound, because an accepted RODAS5P attempt "
    "costs at least 7 callbacks (stages 2–8 evaluate the RHS once each; stage 1 may reuse f(y_n)) (audit A-6). "
    "F4 introduces `RODAS5P_MIN_CALLBACKS_PER_ACCEPTED_ATTEMPT = 7` and a pure, tested `progress_reach_upper_bound`. "
    "A smaller c_min allows more attempts, so the bound grows and the screen fires less often: it can keep pilots "
    "that the old screen discarded, and never discards a pilot that could win."
)
RED_REASON["F4"] = "`progress_reach_upper_bound` does not exist at base, so the test file does not compile (compile-red allowed for this task)."
NOTES["F4"] = [
    "Growth factor: `rodas5p_integrators::AdaptiveStepConfig::default().max_factor` (5.0), the value the old literal used; grep it in `crates/rodas5p-integrators/src/adaptive.rs` before use.",
    "The bound returns `next_up()` of the computed reach so that rounding never makes it smaller than the true bound.",
    "pexprb is out of scope (erratum E-3): leave `PexprbWindowDriver`'s `c_min = 12` and its `min_work_per_attempt: 12` unchanged.",
    "The existing contract `b8_progress_bound_contracts` pins `min_work_per_attempt` 16 and 17; both become 7. Its positive control (ceiling 16, h 1e-4) still triggers `ProgressBound`: m = 2, reach = 6e-4 < 1.",
]

WHY["F5"] = (
    "Audit A-7 found that the B7 rule `budget = budget.max(k_d0_min)` can only lengthen the first explicit batch "
    "and proposed `min`. The dry run refuted that proposal (erratum E-2): with `min`, stiff-forced under "
    "`route_registry` needs 298 attempts and 1,840 RHS calls instead of 59 and 400. F5 keeps the code, adds the "
    "explanation as a comment, and pins the current first-batch length so that any later change is deliberate. "
    "The comparison is preregistered as X10."
)
NOTES["F5"] = [
    "This task changes a comment and adds a characterisation test; it is bitwise-safe and needs no red check.",
    "Do not change `budget.max(k_d0_min)` itself.",
]

WHY["F6"] = (
    "The replay-safety audit called the RHS and JVP callbacks directly, outside the counters, and on a mismatch "
    "dropped the explicit batch it had just executed (`continue 'windows`). Because the audited flag was already "
    "set, later pilots then ran with the non-deterministic callbacks anyway (audit A-9). Plan section 10.4 asks "
    "for: charge the audit work, commit the explicit batch, and disable competitive trials for the rest of the "
    "solve. F6 moves the audit in front of the pilot decision, charges 2 RHS calls (and 2 JVP vectors when a JVP "
    "is supplied), sets `competitive_trials_disabled` on a mismatch, adds that flag to the pilot condition, and "
    "labels the retained explicit batch."
)
RED_REASON["F6"] = (
    "at base a mismatch drops the batch and later pilots still run (RODAS probe segments exist), and the audit "
    "calls are not counted, so `stats.rhs_calls != segment sum + 2`."
)
NOTES["F6"] = [
    "`audit_replay_safety` defaults to `false`; only configurations that set it change behaviour.",
    "The audit now runs at the first pass through the pilot block when `implicit_pilot_attempt_budget > 0`, also in windows where no pilot would have run. That is intended: the audit is a property of the callbacks, not of a pilot.",
    "Edit 3 deletes the old 35-line block entirely (replace with nothing). After the edit, `batches_since_entry_pilot = 0;` is followed directly by `local.h0 = entry_pilot_checkpoint_seed(`.",
    "The mismatch decision string keeps the old wording as a prefix, so existing `contains` checks still match.",
    "The let-chain `if replay_ok && let Jacobian::Jvp(jvp) = &problem.jacobian` needs edition 2024, which the crate uses.",
    "The two JVP tests exist because the first mutation run showed that a dense-Jacobian test alone leaves the JVP accounting lines untested.",
]

WHY["F7"] = (
    "Epoch digests used `std::collections::hash_map::DefaultHasher`, whose output is not specified across Rust "
    "releases, so serialised epochs are not reproducible across toolchains. `run()` also built the window epoch "
    "with index 0 while the drive loop used the segment index, giving two epochs for one window (audit A-11). F7 "
    "adds a crate-private FNV-1a 64 hasher (little-endian integer feed), maps `InteriorError` to explicit tags "
    "instead of `mem::discriminant`, and makes the window epoch depend on the endpoints only. The public "
    "signature of `compute_window_epoch` is unchanged; the index parameter is ignored."
)
RED_REASON["F7"] = "at base the digests come from SipHash and the window epoch depends on the index, so the golden values differ."
NOTES["F7"] = [
    "Golden values were computed independently in Python from the byte sequence (rtol bits LE, tag byte, atol bits LE, interior tag) and match the Rust implementation.",
    "Dry run: counters and final states of all three factories on both registered models are bit-identical before and after F7 (bitwise_safe).",
    "Edit 1 replaces three occurrences at once (count 3). Edit 3 then finds the window-epoch function in its already edited form.",
]

WHY["F8"] = (
    "`entry_policy` only fed `route_registry_active`; a configuration could declare `AdaptiveEvidence` while its "
    "fields implemented pressure-only entry (audit A-10). F8 adds `SwitchingConfig::effective_entry_policy()` and "
    "refuses, before any callback, a declared policy that differs from it. Production needs no change "
    "(erratum E-1)."
)
RED_REASON["F8"] = "`effective_entry_policy` does not exist at base, so the test file does not compile (compile-red allowed)."
NOTES["F8"] = [
    "Mapping: budget 0 → `PressureOnly`; otherwise adaptation present → `AdaptiveEvidence`; otherwise `Evidence`.",
    "A config with adaptation and budget 0 was already refused by the adaptive-entry check; the new check runs first, so such a config now reports the policy mismatch when it declares a policy.",
    "`route_registry` declares `AdaptiveEvidence`, has an adaptation and a positive budget, and stays valid.",
]

WHY["F9"] = (
    "`Rodas5pWindowDriver::for_problem` maps only the four legacy routes; `Rodas5pJvpAssembled` and "
    "`PexprbWindowDriver` are reachable only in their own tests (audit A-8). F9 states this in the rustdoc, pins "
    "the route selection, and pins the counters of all three factories after F1–F8, so that later tasks cannot "
    "change them without failing a test."
)
NOTES["F9"] = [
    "The counter test is the regression oracle for every later task. If it fails after F1–F8 were done as written, do not edit the numbers: abort (R-11).",
    "pexprb records still carry `SwitchingMethod::Rodas5p` as their method label (A-8); fixing that needs a new enum variant and belongs to a pexprb admission node.",
]

# ----------------------------------------------------------------- research tasks
WHY["X1"] = (
    "Theory T1 (shift invariance of polynomial Krylov spaces): all eight RODAS5P stage systems "
    "`(I/(hγ) − J) U_i = b_i` share one basis Q with Z = JQ, because every stage right-hand side except the "
    "nonlinear remainder lies in range Q + range Z. The prototype reduced JVPs per step 2.6–6× at matched steps "
    "(flop speedup about 1.3–5× depending on JVP cost). X1 builds the solver as a pure, unused module."
)
SPEC["X1"] = """**Spec only.** No dry-run-verified Rust exists. Reference prototype: `rodas_shared_krylov_wp.py` (delivered with the re-audit).

**File** `crates/rodas5p-krylov/src/shared_basis.rs`; register it in `crates/rodas5p-krylov/src/lib.rs` by inserting `pub mod shared_basis;` on the line after the anchor `mod gmres_into;` (the existing modules are private and re-exported with `pub use`; the new one is public as a module).

**API (exact names).**

```rust
/// Configuration of a shared per-step Krylov basis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SharedBasisConfig {
    /// Maximum basis dimension (prototype: 60).
    pub max_dim: usize,
    /// Residual budget: stop when h_gamma * ||r||_w <= theta (prototype: 1e-3).
    pub theta: f64,
}

/// Outcome of one shifted solve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SharedSolveStats {
    pub new_jvps: usize,
    pub basis_dim: usize,
    pub weighted_residual: f64,
    pub converged: bool,
}

pub struct SharedBasis { /* n, cfg, q: Vec<f64> (column-major n x k), z: Vec<f64> (n x k), k */ }

impl SharedBasis {
    /// Err when n == 0, max_dim == 0 or theta is not finite and positive.
    pub fn new(n: usize, cfg: SharedBasisConfig) -> Result<Self, String>;
    /// Forget the basis (call once per step: J changes between steps).
    pub fn reset(&mut self);
    pub fn dim(&self) -> usize;
    /// Approximately solve (I / h_gamma - J) x = b with x in range Q.
    /// `weights[i]` = 1 / (atol_i + rtol * |y_i|); `jvp(v, out)` computes out = J v.
    pub fn solve_shifted<E>(
        &mut self,
        h_gamma: f64,
        b: &[f64],
        weights: &[f64],
        jvp: &mut dyn FnMut(&[f64], &mut [f64]) -> Result<(), E>,
        x: &mut [f64],
    ) -> Result<SharedSolveStats, E>;
}
```

**Algorithm of `solve_shifted`** (n = dimension, `||v||_w = sqrt(mean((v_i * w_i)^2))`):

1. Check `b.len() == weights.len() == x.len() == n` and that `h_gamma` is finite and positive; otherwise zero `x` and return stats with `converged: false` (no panic, no JVP).
2. If `k == 0`: if `||b||_2 == 0`, set x = 0 and return converged. Else q₁ = b / ||b||₂, z₁ = J q₁ (one JVP), k = 1.
3. Loop:
   a. Form A = Q / h_gamma − Z (n × k). Solve min_y ||b − A y||₂ with a Householder QR of A (k ≤ max_dim; recompute each pass, O(n k²)).
   b. x = Q y; r = b − A y.
   c. If `h_gamma * ||r||_w <= theta`: return converged.
   d. If k == max_dim: return `converged: false` (the caller falls back to per-stage GMRES).
   e. v = r − Q (Qᵀ r); repeat once (two-pass Gram–Schmidt). If ||v||₂ ≤ 1e-14 ||r||₂: return `converged: false` (breakdown).
   f. q_{k+1} = v / ||v||₂; z_{k+1} = J q_{k+1} (one JVP; propagate its error); k += 1.
4. `new_jvps` counts the JVPs made in this call; `basis_dim` = k at return.

**Tests** (`crates/rodas5p-krylov/tests/shared_basis_contracts.rs`; names fixed by the card):

- `shared_basis_meets_budget_on_shifted_diagonal`: J = diag(−10, −20, …, −320) (n = 32), h_gamma = 0.01, weights all 1, b_i = 1 + i/32, theta = 1e-8. Recompute the true residual b − (x / h_gamma − J x) and assert `h_gamma * ||r||_w <= theta` and `converged`.
- `shared_basis_reuses_directions_across_stages`: same J; solve b₁, then b₂ = 0.5 b₁ + 0.1 (x₁ / h_gamma) (a vector in range Q + range Z). Assert the second call reports `new_jvps <= 1` and converges, and that a fresh basis solving b₂ alone needs more JVPs.
- `shared_basis_refuses_invalid_input`: `new` with n = 0, max_dim = 0, theta = 0 and theta = NaN returns Err; `solve_shifted` with a wrong slice length returns `converged: false` without calling the JVP (count calls in the closure).

**Not allowed:** wiring into any solver (that is X2), new dependencies, `unsafe`.
"""

WHY["X2"] = "Use the X1 basis inside the matrix-free RODAS5P stage loop behind an opt-in flag, with bit-identical behaviour when the flag is off."
SPEC["X2"] = """**Spec only; tier L (human approval).**

**Integration point.** `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs`, `Rodas5pMfFastWorkspace::attempt_inner`, stage loop `for i in 0..s`. After the `if staged { ... continue; }` branch, the code builds `GmresConfig { restart, max_arnoldi, rtol, atol: linear_atol }` and, when `self.gmres_into && self.config.method == LinearMethod::Gmres`, calls `solve_gmres_into_with_accounting(&shifted, &self.preconditioner, &self.stage_rhs, x0, &gmres_config, None, stage, &mut self.gmres, ...)`. The operator there is `I − hγJ` with the right-hand side scaled by hγ; `(I/(hγ) − J) U = b` is the same system.

**Changes.**

1. Field `shared_basis: Option<rodas5p_krylov::shared_basis::SharedBasis>` (default `None`) and setter `set_shared_basis(&mut self, cfg: Option<SharedBasisConfig>)`, following the existing `set_gmres_into(on)` pattern.
2. At the start of each attempt: `reset()` the basis.
3. In the stage loop, before the GMRES branch, if the basis is set and the stage is not staged: call `solve_shifted` with the JVP routed through `apply_counted(op, x, y, counters, ApplyCategory::Krylov)` so that every JVP is charged. If `converged`, use x as the stage solution and skip GMRES; otherwise fall through to the unchanged GMRES call.
4. `weights` = the error weights the step controller already uses for this attempt (grep the existing weight vector; do not compute a new norm).

**Tests** (`crates/rodas5p-integrators/tests/shared_basis_mf_contracts.rs`):

- `shared_basis_off_is_bit_identical`: on the 1-D heat equation (n = 64) and on a 2-component rotation, outputs and counters with no setter call equal those with `set_shared_basis(None)`, bit for bit.
- `shared_basis_on_heat_uses_fewer_jvps_at_matched_steps`: 1-D heat, n = 64, rtol 1e-6, atol 1e-8, theta 1e-3: accepted steps within ±1 of the GMRES run, final error within 10× of the GMRES run's error, total JVPs at most half.
"""

WHY["X3"] = "Decide K1 (shared basis) on measured work, not on prototype JVP counts."
SPEC["X3"] = """**Tier H; preregistered node.** Directory `research/k1_shared_krylov_basis_<YYYYMMDD>/`. Write PREREGISTRATION.md (template 13.3) with:

- arms: per-stage GMRES (current), per-stage GMRES with the existing preconditioner option, shared basis (X2), all on the MF RODAS5P route;
- cells: 1-D heat and 1-D Brusselator (n = 256), 2-D heat and 2-D Brusselator (n = 10⁴), rtol ∈ {1e-4, 1e-6, 1e-8};
- metric: callgrind Ir per solve at matched achieved error (work–precision curves), deterministic JVP and step counters;
- PASS: Ir per solve lower by ≥ 1.3× on ≥ 3 of 4 families against the per-stage arm, and no family worse than the preconditioned per-stage arm;
- KILL: Ir not lower than the preconditioned per-stage arm on any family, or any accounting gap (JVPs not charged).

Ledger profile name: `K1_SHARED_KRYLOV_BASIS`. Commit and push the preregistration before the first run.
"""

WHY["X4"] = (
    "Theory T3: DOPRI5's stages 6 and 7 are both at t + h, so the secant g = k7 − k6 along d = Y7 − Y6 gives a "
    "Rayleigh-quotient estimate q of the dominant real eigenvalue at no RHS cost. On a dissipative mode the step "
    "controller settles at h|λ| ≈ 0.8–0.97·x*, x* = 3.306568 (real-axis stability boundary), so "
    "h·q ≤ −0.8·x* marks a stability-limited step. X4 adds the estimate as a pure function."
)
NOTES["X4"] = [
    "`DOPRI5_Y7_MINUS_Y6` = last tableau row minus row 6, in exact rationals (checked with Python fractions); the first unit test checks it against the tableau.",
    "Stage layout `k_j = stages[j*n..(j+1)*n]` matches `Dopri5Workspace::trial` in `dopri5.rs` (grep it before relying on it in X5).",
    "Under stable 1.97, clippy of `rodas5p-integrators` fails on pre-existing code in `source_owned_graph_schur.rs` even with `--no-deps`; use the pinned toolchain for this task or escalate.",
]

WHY["X5"] = "Use the X4 estimate as an entry-pilot gate (pilot at once in flagged windows, maximal backoff elsewhere)."
SPEC["X5"] = """**Spec only; tier H.** Opt-in field `SwitchingConfig::stiffness_gate_fraction: Option<f64>` (default `None`, valid range (0, 1]). DOPRI5 batches report the minimum `dopri5_secant_h_lambda` over their accepted steps (new telemetry field, `None` when unavailable). In a window whose batch reports `h_lambda <= -fraction * DOPRI5_REAL_AXIS_BOUNDARY`, an eligible pilot runs without waiting for `adaptive_due`; in other windows the adaptive backoff uses its maximum.

Preregistered node D1 (`research/d1_stiffness_gate_<YYYYMMDD>/`): arms adaptive factory vs adaptive + gate (fraction 0.8). PASS: identical entry decisions on all registered cells; discarded pilot work on rotation and gyro lower by ≥ 90%; agreement of the gate with an intervention test ("the step does not grow when rtol is loosened 10×") ≥ 0.9 on ≥ 3 nonlinear problems with n ≥ 50. KILL: any lost registered entry, or agreement < 0.9 off the registry (the prototype saw 0.58 on the Brusselator at rtol 1e-6).
"""

WHY["X6"] = (
    "Theory T4 (Curtis–Powell–Reid): for a declared sparsity pattern, columns that share no row can share one JVP "
    "probe, so J is assembled exactly from as many JVPs as there are colours (≥ Δ+1; 3 for a tridiagonal pattern, "
    "4 for the interleaved 1-D Brusselator, 5 for the 2-D 5-point stencil with the (i+2j) mod 5 colouring). X6 "
    "adds a deterministic greedy colouring, the probe vectors and the recovery map as a pure module."
)
NOTES["X6"] = [
    "Greedy colouring in natural column order is deterministic and valid but not optimal (the 2-D 5-point pattern may get more than 5 colours). The test bounds it between Δ+1 and Δ(G²)+1.",
    "A dense row forces n colours with forward JVPs; that is the expected result of `dense_row_forces_n_colours`.",
]

WHY["X7"] = "Assemble J from coloured JVP probes and factor it directly when the pattern allows it."
SPEC["X7"] = """**Spec only; tier H.** New constructor `Jacobian::sparse_jvp(pattern: SparsityPattern, jvp)` (additive; existing variants unchanged), a route that assembles J with `column_coloring` + `recover_values` (one JVP per colour, each charged as a JVP vector) and factors it with the banded or a sparse direct path when the pattern is banded or has a small elimination width, and falls back to the MF route otherwise.

Preregistered node C1 (`research/c1_coloured_assembly_<YYYYMMDD>/`): arms MF (current), shared basis (X2), coloured assembly. PASS: JVPs per step ≤ colours + 1; Ir lower than X2 on banded and tree-like families; states identical to the dense-Jacobian route up to rounding. KILL: assembled J differs from a finite-difference reference beyond rounding.
"""

WHY["X8"] = (
    "Theory T5: with strict comparisons on both sides (`route_registry`) the hysteresis band has width zero and "
    "switching near break-even is unbounded; with entry ratio ρ_e and return ratio ρ_r, ρ_e·ρ_r < 1, the number "
    "of switches is at most 1 + TV(ln Φ̂)/w with w = −ln ρ_e − ln ρ_r. X8 adds integer helpers for margin-shifted "
    "comparisons and their decision-preserving callback caps; with ratio 1/1 they reduce to the current strict "
    "rules (tested)."
)
NOTES["X8"] = [
    "All arithmetic is checked u128; overflow returns `None`, never a wrapped value.",
    "The helpers are not wired into any comparison; X9 does that after a bias measurement.",
]

WHY["X9"] = "Use the X8 band in `route_registry` only after measuring the bias of the work-ratio estimate."
SPEC["X9"] = """**Spec only; tier H.** Opt-in fields `entry_margin: Option<MarginRatio>` and `return_margin: Option<MarginRatio>` on `SwitchingConfig` (default `None`, which keeps the strict rules); validation requires `band_is_open`. With V1 the caps are `margin_entry_cap` / `margin_return_cap`; with V2 they are divided by κ as in F2.

Preregistered node H1 (`research/h1_hysteresis_band_<YYYYMMDD>/`): first measure the bias b of ln Φ̂ on the transition-39/79 models (measured incumbent against the challenger's observed prefix). Choose ρ only if |b| < w/2 for the chosen band. PASS: switches on a preregistered break-even family ≤ the Theorem-H bound; no registered decision lost. KILL: |b| ≥ w/2 for every candidate band, or no reduction of switches.
"""

WHY["X10"] = "Decide the B7 first-batch length on measured cells (erratum E-2)."
SPEC["X10"] = """**Spec only; tier H.** Preregistered node `research/b7_first_batch_<YYYYMMDD>/`. Arms: `max(budget, k_d0)` (current), `min(budget, k_d0)`, `min(budget, max(k_d0, 24))`. Cells: stiff-forced, transition-39, transition-79, dense n ∈ {64, 256} with `route_registry`. Metric: total callbacks (RHS + JVP + n·Jacobian builds) and entry time. PASS: an arm lowers total callbacks by ≥ 10% on ≥ 3 of 5 cells and raises none by more than 5%, with identical entry decisions. KILL: every `min` arm raises stiff-forced work (the dry run measured 400 → 1,840 RHS calls for plain `min`).
"""

WHY["X11"] = "Re-run E1 (WorkModel v2 calibration) with measured callgrind instruction counts under a frozen script."
SPEC["X11"] = """**Spec only; tier H.** New node `research/e1r_work_model_v2_<YYYYMMDD>/`. The preregistration reuses the gate text of the aafc2e0b preregistration verbatim (maximum relative error ≤ 25% per case, Spearman ρ ≥ 0.90, transition-39 admission threshold as originally written). The measurement script runs `valgrind --tool=callgrind` on a frozen example binary per case and reads Ir from the callgrind output; no measured value may appear as a literal in any script. WorkModel v2 may return to a factory only through a PASS row of this node.
"""
