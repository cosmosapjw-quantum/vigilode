# Implementation plan — production wave 3

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** close the P0 protected-tier gaps that block gate G-A (dense output on the fast driver, vector atol,
events, typed refusals, CI), open the third B1 card with an explicit preconditioner-staleness bound, and
re-measure E1 with VigilODE stepping freely and interpolating like its comparators.

**Architecture:** six nodes on disjoint files, developed in parallel in separate worktrees (stage 1). One
small API node runs on the integration tip after the facade nodes merge (stage 2). The E1 measurement and
the full suites run on the owner's workstation through handoff documents. Task 8 integrates.

**Tech stack:** Rust (stable 1.97 in the cloud; pinned 1.94.1 on the workstation and in CI), faer, serde;
the vigilode-bench comparators (gfortran 13.3.0, SUNDIALS 6.4.1, Diffsol 0.16.1).

**Spec:** `docs/plan/PRODUCTION_PLAN_20261020.md` (§N, T1–T9). Earlier records: `docs/plan/IMPLEMENTATION_WAVE1.md`,
`docs/plan/IMPLEMENTATION_WAVE2.md`, `docs/plan/nodes/*.md`, `.superpowers/sdd/IMPLEMENTATION_WAVE{1,2}/progress.md`.

**Base:** `production/wave2-integration` after the wave-2 final fix wave (`production/w2-final-fix`) merges
(Task 0). The controller fills the exact SHA into every worktree. All symbols cited below as existing were
checked against `production/wave2-integration` at `00973c7` (worktree `/home/claude/wt/integration2`).
Symbols marked **new** are defined by the task that creates them.

## Global Constraints

- Every wave-1 and wave-2 Global Constraint applies. That covers: `--locked`; opt-in new behaviour;
  bit-identical defaults unless a task says otherwise; type-distinct statuses (estimate, receipt,
  certificate); counted failures (attempted vs completed); fmt and clippy clean in touched files; and a node
  report at `docs/plan/nodes/<node-id>.md`.
- Build environment for every cloud cargo call:
  `export RUSTUP_TOOLCHAIN=stable CARGO_TARGET_DIR=/home/claude/targets/<node-id> CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`,
  then `cargo ... -j 1 --locked` (`-j 2` only when no other node is building). One target directory per node.
  Never `cargo clean`. Never run `cargo test --workspace` in the cloud: full suites go to the workstation
  handoff (Task 8).
- Disk: 13 GB was free on 2026-10-21. Before dispatch the controller decides whether to delete stale target
  directories of finished work (`/home/claude/targets/integration` 8.2 GB, `finalreview2*` 1.5 GB). A node
  checks `df -h /home/claude` before each large build, and stops and reports below 4 GB.
- Commits use `git -c user.name="Jiwon Park" -c user.email="cosmosapjw@gmail.com" commit`. The wave-2
  p0-facade identity slip must not recur. Messages end with the session trailers of the wave-1 constraints.
  A node pushes only its own branch `production/<node-id>`: no force-push, no merges into other branches.
- No new external crates in the root workspace. `crates/vigilode-bench` stays its own workspace.
- Ledger: `research/LEDGER.jsonl` is append-only (`docs/RESEARCH_LEDGER.md`) and is checked by
  `python3 tools/check-research-node.py --base <wave-3 base SHA>`. L-0140 is taken by the wave-2 fix.
  Wave-3 ids are pre-assigned: **L-0141 → b1c-staleness**, **L-0142 → e1b-dense**. The order is fixed so that
  ids increase in merge order, because B1c lands before the E1 measurement returns. Row date is the
  measurement date.
- Rulings that bind:
  - `wrms` stays strict and `step_error_wrms` carries the zero-scale rule.
  - Preregistered kill gates are never re-scored or loosened.
  - **`Tier::Performance` routes no lagged driver in wave 3, whatever B1c's verdict.** A PASS opens gates 3–4
    of plan §8 in wave 4, and routing is decided only after the holdout.
- Heavy runs (full suites, the E1 benchmark) run on the owner's workstation (Ryzen 9 5900X, 12C/24T, 96 GB)
  through `docs/plan/handoff/LOCAL_*_W3.md`. The documents follow the pattern of
  `docs/plan/handoff/LOCAL_TEST_HANDOFF_W2.md`: CODE_TASK, Setup, Commands, SCIENTIFIC_CONTRACT,
  VALIDATION_MATRIX, SUMMARY.json and Return path. Results come back on `results/local-*` branches. A
  handoff document is committed and pushed before the controller hands it over, and the local session
  edits no code.

## Review Focus

1. A vector atol with one component `atol_i = 0` that stays exactly zero (and one that does not). The A1
   zero-scale rule must hold per component: 0 contributes 0, nonzero gives +inf. Owner: Task 4, step 1 (d).
2. `Solution::at(t)` exactly at an accepted step boundary on the new fast dense route. It returns the stored
   state bit for bit, never the interpolant evaluated at θ = 1. Owner: Task 3, step 1 (b).
3. An event function that is zero exactly at `t0`, or exactly at a step endpoint. There is no event at `t0`,
   and an event at a shared endpoint is reported exactly once. Owner: Task 5, step 1 (e, f).
4. A failed run (fatal RHS) under fast dense output. The returned segments are a bit-identical prefix of the
   clean run, and `at(t)` past the last accepted step is `None`. Owner: Task 3, step 1 (f).
5. B1c on a Jacobian with positive log norm (`h_P γ μ_S(J_P) ≥ 1`). The bound is +inf, so the driver refreshes
   every attempt and reproduces the fast driver within 1e-12. Owner: Task 2, step 1 (c).

## Decisions taken in this plan

- **A2 W-filtered dense error control: park the retune and keep the mode.** The wave-1 card failed its
  preregistered gate (2.14x the `Off` step count against the limit of 1.3x; interior error 0.35 tolerance
  units). Wave 3 opens no retune card, for three reasons.
  1. The gate compares with `Off`, which leaves 321 tolerance units of interior error on Prothero–Robinson
     λ = −1e5 in 7 steps. Any mode that meets ≤ 2 units has to cut the interior error 160-fold, and the gate
     leaves it 9 steps to do so.
  2. The only retune that T3 justifies is a safety factor κ: accept an estimate up to κ = 2 tolerance units.
     That is admissible because Lemma 3 gives the estimate ≥ the true interior error (R ≥ 1; the upper bound
     is 1/(2γ) = 2.359). But it moves the acceptance threshold toward the gate's own 2-unit criterion, and it
     would be tuned on the problem that failed. Plan §8 forbids exactly that (one mechanism; the holdout is
     never used for tuning).
  3. A cadence change (checking fewer steps) would void the "`at(t)` anywhere" contract that the facade
     makes under dense output.

  `WFilteredEnforce` stays the facade's dense-output default. That is the P0 interior-error contract (§3), not
  a performance promotion. E1b (Task 1) records interior error and step ratios against `Off` on the corpus.
  These are the evidence for the A2 acceptance ("≤ 2 tolerance units on the stiff corpus, ≤ 1.2x steps"). A
  κ = 2 card may open in wave 4, only on a preregistered witness set that excludes PR λ = −1e5.
- **Dense output moves to the fast driver** (Task 3). The wave-2 dense route uses the sequential driver,
  which allocates per step and holds per-step LU clones, so it cannot meet the P0 zero-allocation promise. It
  is also not the driver that §9 names as the default dense path. Mass matrices and `InteriorError::Enforce`
  stay on the sequential driver. Results under `dense_output = true` change bits between wave 2 and wave 3.
  The facade records this through `STATS_SCHEMA = "vigilode.stats.v3"` and the new driver ids.
- **`#[non_exhaustive]` (wave-1 final-review item 8): adopt it now, in the facade only** (Task 7).
  - Rationale: adding it is a breaking change, and the cheapest moment is before the first published 0.x.
  - Mark: every public facade enum except `Atol`, which is closed by design, and the output-only structs
    (`Stats`, `Solution`, `RouteRecord`, `StopRecord`, `CallbackWork`, `FiniteDifferenceWork`,
    `KrylovStartRecord`, and the new `EventRecord`).
  - Do not mark: `Options`. Its documented construction is `Options { rtol, ..Default::default() }`, and
    `#[non_exhaustive]` forbids struct-update syntax outside the crate. Adding an `Options` field stays a
    minor-version break before 1.0.
  - Do not mark: the internal-crate enums (`BandedKernel`, `DenseErrorControl`), which become internal
    modules (§9).
  - Facade types that leak internal types through `Stats` (final-review item 9) are deferred to wave 4.

## Carry-over map

| Carry-over | Task |
| --- | --- |
| E1 grid-constrained rows (98–99 clipped steps per row) | 1 (cloud harness + workstation run), ledger L-0142 |
| Final-review minor 6: stale fidelity note at `arms.rs:151` | 1 |
| B1/B1b kill-gate FAIL, no h/h_P staleness bound | 2, ledger L-0141 |
| A2 kill-gate FAIL | Decision above; evidence columns in 1 |
| `#[non_exhaustive]` deferred | Decision above; implemented in 7 |
| Wave-2 final-review residuals (`production/w2-final-fix`) | 0 |
| Parent-plan P0 items scheduled next (§3 P0, §6 A1, G-A): vector atol, events, dense output on the protected fast path with zero hot-loop allocation, typed refusals, docs/examples, CI/MSRV/semver gate | 3, 4, 5, 6, 7 |
| Deferred to wave 4 | A3 controller hardening (vdP rejections ≤ 20%); A4 small-N/sparse colouring routes; B2 JVP-only tier; vector atol on the sequential route; in-loop event stop; facade-owned Stats types; corpus v2 problems; §7 rule-1 reference (RADAU5 and CVODE agreeing to 1e-10) |

## Dependency table

| Task | Node id / branch / worktree | Stage | Depends on | Tier hint | Est. wall clock |
| --- | --- | --- | --- | --- | --- |
| 0 | controller, `production/w2-final-fix` → `production/wave2-integration` | pre | wave-2 final review | — | controller |
| 1 | `e1b-dense` / `production/e1b-dense` / `/home/claude/wt/e1b-dense` | 1a cloud; 1b workstation | 1a: 0; 1b: 3, 4, 5, 7 merged | standard | 4 h cloud + 2 h local + 1 h ingest |
| 2 | `b1c-staleness` / `production/b1c-staleness` / `/home/claude/wt/b1c-staleness` | 1 | 0 | most capable | 8–10 h |
| 3 | `p0-fastdense` / `production/p0-fastdense` / `/home/claude/wt/p0-fastdense` | 1 | 0 | most capable | 12–16 h |
| 4 | `p0-atol` / `production/p0-atol` / `/home/claude/wt/p0-atol` | 1 | 0 | standard | 5–6 h |
| 5 | `p0-events` / `production/p0-events` / `/home/claude/wt/p0-events` | 1 | 0 | standard | 5–6 h |
| 6 | `ci-docs` / `production/ci-docs` / `/home/claude/wt/ci-docs` | 1 | 0 | cheap | 2–3 h |
| 7 | `p0-api` / `production/p0-api` / `/home/claude/wt/p0-api` | 2 | 3, 4, 5 merged into `production/wave3-integration` | standard | 3 h |
| 8 | controller, `production/wave3-integration` | final | all | — | controller + 3 h local |

Pre-flight shared surfaces:
- Tasks 3 and 4 both edit `rodas5p_fast.rs`: the step loop for 3, the error-norm hunk and new entry points
  for 4. Neither may change the signature of a `pub(crate)` item that `rodas5p_lagged.rs` uses (Task 2).
- Tasks 3, 4 and 5 all edit `crates/vigilode/src/{solve,solution,options}.rs`. Task 5 touches the segment
  store only through `Solution::truncate_after` (new). The cross-node seams are explicit steps of Task 8.
- Tasks 2 and 1 each append one ledger row (L-0141, L-0142, in that order).

---

## Task 0: absorb wave-2 final-review residuals (controller; placeholder)

Input: branch `production/w2-final-fix`, the fix wave of the wave-2 final review, which is in flight.
Include `production/h-checker` (4f92ec3), the ledger-checker change the user approved (2026-10-21), if the
fix wave has not already merged it. Wave 3 branches from `production/wave2-integration` after both land.
The controller records the resulting SHA as the **wave-3 base** in `.superpowers/sdd/IMPLEMENTATION_WAVE3/progress.md`.

- [ ] Residual items: _to be filled by the controller from the wave-2 final review._
- [ ] `python3 tools/check-research-node.py --base <wave-2 base a5a48f7>` exits 0 on the wave-3 base (record it).
- [ ] Wave-3 base SHA recorded; worktrees created from it.

## Task 1: E1b-dense — re-run E1 with free stepping and interpolated output (node id `e1b-dense`)

Plan §7; wave-2 ruling "no solver ranking is admitted until the facade's dense output is benchmarked".

**Two parts:**
- 1a (cloud): harness changes, a preregistration and a handoff document. 1a starts at once.
- 1b (workstation): the measurement, run after Tasks 3, 4, 5 and 7 have merged into
  `production/wave3-integration`, so the shipping dense route is the one measured. The node branch merges
  that integration tip before writing the handoff SHA.

**Files:**
- Modify: `crates/vigilode-bench/src/arms.rs` (new arms; the `deviations()` and `output_interface()` text),
  `src/harness.rs` (endpoint-only check), `src/report.rs` (two new sections), `tests/adapters.rs`.
- Create: `research/bench_wave3_YYYYMMDD/{PREREGISTRATION.md, corpus_v2.json}`, where YYYYMMDD is the
  preregistration date.
- Create: `docs/plan/handoff/LOCAL_E1B_HANDOFF_W3.md`.
- After 1b: `research/bench_wave3_YYYYMMDD/{results_v2.json, REPORT.md, run.log, host.json}`, ledger row
  **L-0142**, and `docs/plan/nodes/e1b-dense.md`.

**Interfaces:**
- Consumes: `vigilode::{Problem, Jacobian, Options, Atol, InteriorError, Rodas5p, Solution}`;
  `Solution::at(t) -> Option<Vec<f64>>`; `Stats::{output_clipped_steps, accepted_steps, route.driver_id}`.
- Produces: `Arm::{VigilodeDense, VigilodeDenseOff, VigilodeBandedDense, VigilodeBandedDenseOff}` (new), with
  kebab ids `vigilode-dense`, `vigilode-dense-off`, `vigilode-banded-dense`, `vigilode-banded-dense-off`.

**Exact values:**
- Corpus v2 = corpus v1 with the same four problems, `rtols` {1e-4, 1e-6, 1e-8}, `error_levels`,
  reference rule, atol scales, 101-point grid, warm-ups and repetitions. The arm list is v1's 13 plus the 4
  new arms.
- The new arms call `Rodas5p::solve` with `Options { rtol, atol: Atol::Scalar(rtol × atol_scale),
  dense_output: true, interior_error: WFilteredEnforce | Off, ..Default::default() }`. The analytic Jacobian
  comes through `Jacobian::dense` or `Jacobian::banded(lower, upper, ..)`. The grid values are
  `sol.at(t_k)`. Scalar atol keeps the run comparable with v1, so vector atol is not used.
- Endpoint-only stepping: a dense row with `stats.output_clipped_steps != 0`, or with any `at(t_k) == None`
  for `t_k` in the span, gets the status `protocol-violation`. Such a row is kept, never dropped.
- The v1 arms `vigilode` and `vigilode-banded` stay as continuity arms, labelled `grid-clipped`. The
  matched-ratio headline uses only the dense arms with interior `WFilteredEnforce`, which is the product
  default.
- Minor 6: `deviations()` and `output_interface()` no longer cite "the facade at a5a48f7". The text is
  generated from the facade route each arm uses, and each row records `stats.route.driver_id`.

- [ ] **Step 1: Write failing adapter tests** in `crates/vigilode-bench/tests/adapters.rs`:
  `dense_arms_equal_facade_solve` (each dense arm's grid values are bit-identical to `Rodas5p::solve` + `at`
  on Robertson and HIRES at rtol 1e-6); `dense_arms_take_no_clipped_steps` (`output_clipped_steps == 0`,
  `accepted_steps` equals the facade's); `fidelity_notes_name_no_stale_commit` (no `deviations()` string
  contains `a5a48f7`).
- [ ] **Step 2: Run them** (`cargo test -j 1 --locked --test adapters` in `crates/vigilode-bench`); expect a
  compile failure (the arms are absent).
- [ ] **Step 3: Implement** the four arms, the protocol check, and two report sections:
  - "Endpoint-only VigilODE (dense output)": the matched-error ratios of the dense arms.
  - "Interior control evidence (A2)": per problem × rtol, `error_own` and accepted steps for
    `WFilteredEnforce` against `Off`, the ratio, and a column `≤ 2 tol units` (yes/no), all generated from
    the JSON.
- [ ] **Step 4: Run** `cargo test -j 1 --locked` (all 11 + 3 new pass; the report test still proves that
  every Markdown number is in the JSON) and `cargo clippy -j 1 --locked --all-targets -- -D warnings`.
- [ ] **Step 5: Preregister.** Commit and push `PREREGISTRATION.md` and `corpus_v2.json` before any
  measurement. The preregistration states:
  - The question: VigilODE versus comparators with every arm interpolating.
  - The command and inputs.
  - The host: the workstation, `timing_authority = "diagnostic_only"` (no pinned frequency, in-process
    repetitions), recorded in `host.json`.
  - The rule that ratios across hosts (v1 against v2) are never compared.
  - The verdict, fixed in advance as INCONCLUSIVE because timing is diagnostic. The row exists for
    provenance and for the interior-error evidence.
  - That L-0142 supersedes **L-0140**, the head of the L-0139 chain, for the VigilODE-relative ratios.
  - That the comparator rows are re-measured, not inherited.
- [ ] **Step 6: Write `docs/plan/handoff/LOCAL_E1B_HANDOFF_W3.md`** with the W2 pattern:
  - Setup: clone, fetch `production/e1b-dense`, detach at the SHA under test. Install gfortran and
    libsundials-dev (6.4.1) if missing, and record their versions.
  - Commands: `cargo build -j 12 --locked --profile measurement` in `crates/vigilode-bench`, then
    `OPENBLAS_NUM_THREADS=1 taskset -c 2 vigilode-bench wp --corpus research/bench_wave3_YYYYMMDD/corpus_v2.json --out $R/results_v2.json 2> $R/run.log`,
    then `vigilode-bench report --in $R/results_v2.json --out $R/REPORT.md`, plus
    `{ lscpu; uname -a; rustc -V; gfortran --version; date -Is; } > $R/host.json.txt`.
    The run is pinned to one core, with no other heavy job on the host during it.
  - VALIDATION_MATRIX:

    | Check | Expected |
    | --- | --- |
    | wp exit code | 0 |
    | rows | 4 problems × 3 rtols × applicable arms, none dropped |
    | `protocol-violation` rows | 0 |
    | references undisputed | yes |
    | `binary sha256` | recorded |

  - Return branch: `results/local-e1b-<sha>`.
  - SCIENTIFIC_CONTRACT: no flag changes, no reruns to improve numbers, the measurement binary only.
- [ ] **Step 7 (after 1b returns): ingest.** Copy the four result files into the node directory unchanged,
  and verify their sha256 against `SUMMARY.json`. Append the results section to PREREGISTRATION.md and add
  ledger row **L-0142**:
  - `commit`: the pushed SHA that was measured.
  - `command`: the exact local commands.
  - `profile`: names the workstation and `taskset`.
  - `outputs_sha256`: covers results_v2.json, REPORT.md and run.log.
  - `verdict`: INCONCLUSIVE.
  - `supersedes`: "L-0140".

  Run `tools/check-research-node.py --base <wave-3 base>` and record its exit code (expected 0).

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

## Task 3: P0-fastdense — continuous extension and W-filtered interior control on the fast drivers (node id `p0-fastdense`)

Plan §3 P0 (interior error, zero allocation per step), §5 T3, §9 (`fast` becomes the default dense path).

**Files:**
- Create: `crates/rodas5p-integrators/src/rodas5p_fast_dense.rs`, plus a `lib.rs` export.
- Modify: `rodas5p_fast.rs`: a per-step sink hook, additive only. No signature change of any `pub(crate)`
  item that `rodas5p_lagged.rs` uses (`Workspace::new`, `factor`, `solve`, `solve_dense_into`,
  `after_loop`, `failure_kind`). Add a banded-capable `pub(crate) fn solve_into(&self, b: &mut [f64])`.
- Modify: `dense_output_v2.rs`: make `rodas5p_dense_output_parts` `pub(crate)` (visibility only).
- Modify: `crates/vigilode/src/{solve.rs, solution.rs, lib.rs}`.
- Test: `crates/rodas5p-integrators/tests/fast_dense_contracts.rs`,
  `crates/rodas5p-integrators/tests/fast_dense_allocations.rs` (counting allocator, following
  `rodas5p_fast_allocations.rs`), and `crates/vigilode/tests/p0_contracts.rs` (updated).

**Interfaces (new):**
- `pub enum FastInteriorControl { Off, WFilteredEnforce }`.
- `pub struct FastDenseSegments` (flat storage: `t: Vec<f64>`, states `(m+1)·n`, stage rows `m·8·n`) with
  `len()`, `t_old(k)`, `t_new(k)` and
  `pub fn evaluate_into(&self, t: f64, out: &mut [f64]) -> CoreResult<()>`. `evaluate_into` returns the
  stored state bitwise at a step endpoint and otherwise computes θ exactly as `Rodas5pDenseSegment::evaluate`
  does.
- `pub struct FastInteriorWork { pub samples: u64, pub interior_rejections: u64, pub rhs_calls: u64, pub back_solves: u64, pub max_estimate: f64 }`.
- `pub struct Rodas5pFastDenseResult { pub fast: Rodas5pFastResult, pub segments: FastDenseSegments, pub interior: FastInteriorWork, pub partial: Option<DriverPartial> }`.
- `pub fn integrate_rodas5p_fast_dense_observed(problem: &OdeProblem, t_span: (f64, f64), y0: &[f64], adaptive: &AdaptiveStepConfig, interior: FastInteriorControl) -> CoreResult<Rodas5pFastDenseResult>`,
  and `integrate_rodas5p_fast_banded_dense_observed(problem, band: &BandedJacobian, t_span, y0, adaptive, interior)`
  with the same return type.
- `pub const RODAS5P_FAST_DENSE_DRIVER_ID: &str = "rodas5p-fast-dense-v1"`,
  `RODAS5P_FAST_BANDED_DENSE_DRIVER_ID = "rodas5p-fast-banded-dense-v1"`.
- Facade: `DriverWork::FastDense { banded: bool, interior: FastInteriorWork, segments: usize }`;
  `STATS_SCHEMA = "vigilode.stats.v3"`.

**Exact rules:**
- The output is `[t0, t1]` only (no grid clipping).
- On every attempt whose embedded error is ≤ 1 (the `InteriorSampling::EveryStep` rule), compute
  E = (I − hγJ₀)⁻¹(h/2)d(1/2) = W⁻¹((h/2)d)/(hγ), using the workspace's own LU of W = I/(hγ) − J. The cost
  is one RHS at the midpoint and one back-solve. There is no Jacobian and no factorization.
- Use the midpoint derivative formula of `scaled_midpoint_defect` (`(y_new − y_old + (d1 + d2)/4)/h`).
- Take the norm with `scaled_error_component` on the step's scale.
- Under `WFilteredEnforce` the controller sees `max(err_emb, est)` for acceptance and for the step proposal;
  `Off` skips the estimate entirely.
- The no-op sink is compiled out, so the existing entry points stay bit-identical.
- Facade routing:

  | Request | Route |
  | --- | --- |
  | `dense_output` with no mass matrix, interior `WFilteredEnforce` or `Off` | fast dense; banded twin for `Jacobian::Banded` or an FD band |
  | `Enforce`, or a mass matrix | sequential (unchanged) |
  | JVP-only with dense output | still refused |

- Facade dense Jacobian provider: switch `build()` in `solve.rs` from an allocating `JacobianFn` to
  `OdeProblem::with_jacobian_into` (removes the per-call `DenseMatrix` allocation E1 found, 8.7–18.5 MB per
  Brusselator run).

- [ ] **Step 1: Write failing tests** (`fast_dense_contracts.rs` unless noted):
  - (a) `off_matches_fast_driver_bitwise`: Robertson and HIRES at rtol {1e-4, 1e-6, 1e-8}. Endpoint bits,
    attempts, accepted, rejected and counters equal `integrate_rodas5p_fast_observed` with
    `OutputSchedule::new(vec![t0, t1])`; the same holds for the banded twin on Brusselator n = 100.
  - (b) `segment_endpoints_are_stored_states`: `evaluate_into(t_k)` gives the stored state bitwise for
    every k.
  - (c) `stage_rows_match_sequential_stages`: one step from the same (t, y, h) on Robertson; the fast `u`
    rows equal the sequential stage rows used by `Rodas5pDenseSegment` within 1e-12 relative (a lib unit
    test with `pub(crate)` access). If the coordinates differ, the test documents and applies the transform.
  - (d) `wfiltered_pr_interior_within_two_units`: Prothero–Robinson λ = −1e5, [0, 2], rtol 1e-6, atol 1e-8,
    201 evaluation points. The max interior error is ≤ 2 tolerance units; record the accepted steps next to
    the sequential values 15 (WFilteredEnforce) and 7 (Off), measured and not gated.
  - (e) `interior_rejection_counter_sum`: `accepted + rejected == attempts`, and `interior_rejections > 0`
    on PR at rtol 1e-6.
  - (f) `fatal_rhs_partial_prefix`: under fast dense output, the segments of a run with a fatal RHS after
    ≥ 3 steps are a bit-identical prefix of the clean run.
  - (g) `fast_dense_allocations_after_setup` (`fast_dense_allocations.rs`): allocations during the run are
    ≤ those of `integrate_rodas5p_fast_observed` on the same problem plus 3·⌈log₂(accepted + 1)⌉, which
    covers segment-store growth only.
  - (h) facade `p0_contracts.rs`: `dense_output_routes_to_fast_dense` (driver id, schema v3);
    `dense_output_bit_identical_to_direct_fast_dense`; `enforce_and_mass_stay_sequential`;
    `at_none_outside_span_and_after_failure`; `empty_span_dense_output` (t1 == t0: `at(t0) == y0`).
    Update the wave-2 dense-output tests to the new route; the PR λ = −1e5 test keeps "≤ 2 tolerance units"
    and updates the step count it records.
- [ ] **Step 2: Run** `cargo test -p rodas5p-integrators --locked -j 1 --test fast_dense_contracts --test fast_dense_allocations`
  and `cargo test -p vigilode --locked -j 1`; expect compile failures.
- [ ] **Step 3: Implement** the module, the sink hook, `solve_into`, the facade route and `with_jacobian_into`.
- [ ] **Step 4: Run** the new tests and the regression set:
  - `rodas5p_fast_contracts`, `rodas5p_fast_allocations`, `int03_banded_fast_contracts`,
    `p0_partial_contracts`, `dense_wfiltered_contracts`, `b1_lagged_contracts`, `b1b_gmres_contracts`;
  - `cargo test -p vigilode --locked -j 1` (all targets).

  All pass.
- [ ] **Step 5: Node report** with the PR and Robertson interior and step numbers (measured, deterministic)
  and the allocation counts.

## Task 4: P0-atol — per-component absolute tolerance on the fast routes (node id `p0-atol`)

Plan §3 P0 ("vector atol"); E1 deviation 2 (the corpus atol vectors had to be uniform).

**Files:**
- Modify: `crates/rodas5p-integrators/src/rodas5p_fast.rs`: the error-norm hunk of `Workspace::attempt` and
  new entry points. The existing `attempt(.., atol: f64, ..)` signature is unchanged.
- Modify: `crates/vigilode/src/{solve.rs, options.rs}`.
- Test: `crates/rodas5p-integrators/tests/fast_vector_atol_contracts.rs` (new),
  `crates/vigilode/tests/facade_contracts.rs`.

**Interfaces (new):**
- `Workspace` gains `pub(crate) atol_vector: Vec<f64>`. Empty means the scalar path, unchanged bit for bit.
  With entries, component i uses `atol_vector[i] + rtol·max(|y_i|, |y_new,i|)` through
  `scaled_error_component`. Every other use of `adaptive.atol` in the fast loop follows the vector when one
  is given.
- `pub fn integrate_rodas5p_fast_observed_with_atol(problem: &OdeProblem, t_span: (f64, f64), y0: &[f64], adaptive: &AdaptiveStepConfig, output: &OutputSchedule, atol: &[f64]) -> CoreResult<(Rodas5pFastResult, Option<DriverPartial>)>`.
- `pub fn integrate_rodas5p_fast_banded_observed_with_atol(problem: &OdeProblem, band: &BandedJacobian, t_span: (f64, f64), y0: &[f64], adaptive: &AdaptiveStepConfig, output: &OutputSchedule, atol: &[f64]) -> CoreResult<Rodas5pFastBandedResult>`.
- Validation: length n, finite, ≥ 0, otherwise `CoreError::InvalidInput`. `adaptive.atol` is still validated
  but is unused when a vector is given.
- Facade: `Atol::Vector` is accepted on the dense fast route (no declared band; direct call recorded in
  `RouteRecord::entry_point`) and on the banded fast route. It stays refused with `Unsupported` on the
  matrix-free route (the Krylov weights take a scalar), on the sequential route (mass or `Enforce`) and on
  dense + declared band (router). Task 8 extends it to fast dense output. Update the `Atol::Vector` docs.

- [ ] **Step 1: Write failing tests:**
  - (a) `uniform_vector_equals_scalar_bitwise`: Robertson dense and Brusselator banded at rtol 1e-6; endpoint
    bits and counters are equal.
  - (b) `robertson_vector_atol_accuracy`: atol = [1e-8, 1e-14, 1e-8] at rtol 1e-6. The endpoint error
    against the fast driver at rtol 1e-11, measured in the same vector-weighted WRMS, is ≤ 2 tolerance units.
  - (c) `invalid_vector_rejected` (wrong length, NaN, negative).
  - (d) `zero_component_follows_zero_scale_rule`: one identically-zero component with atol_i = 0 succeeds,
    and a component with atol_i = 0 and a nonzero error is rejected (+inf).
  - (e) facade: `vector_atol_accepted_on_fast_routes`, `vector_atol_refused_on_mf_sequential_and_declared_band`.
- [ ] **Step 2: Run** `cargo test -p rodas5p-integrators --locked -j 1 --test fast_vector_atol_contracts`; expect a
  compile failure.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests plus `rodas5p_fast_contracts`, `int03_banded_fast_contracts`,
  `a1_zero_tolerance_scale`, `rodas5p_fast_allocations`, `b1_lagged_contracts` and
  `cargo test -p vigilode --locked -j 1`; all pass.

## Task 5: P0-events — event location with root finding on the dense extension (node id `p0-events`)

Plan §3 P0 ("events with root finding"), §3 API (`.event(|t, y| y[0] - 1.0, Direction::Down)`).

**Files:**
- Create: `crates/vigilode/src/events.rs`.
- Modify: `crates/vigilode/src/{problem.rs, solution.rs, solve.rs, lib.rs}`.
- Test: `crates/vigilode/tests/events_contracts.rs`.

**Interfaces (new):**
- `pub enum Direction { Up, Down, Either }`.
- `Problem::event<G>(self, g: G, direction: Direction) -> Self` and `Problem::terminal_event<G>(self, g: G, direction: Direction) -> Self`,
  with `G: Fn(f64, &[f64]) -> f64 + Send + Sync + 'static`.
- `pub struct EventRecord { pub index: usize, pub t: f64, pub y: Vec<f64>, pub direction: Direction }`.
  `index` is the declaration order, and `direction` is the observed Up or Down.
- `Solution::events: Vec<EventRecord>` (time order). `Stats::terminal_event: Option<usize>` (an index into
  `events`) and `Stats::event_evaluations: u64`.
- `FailureReason::EventFunctionNonFinite`.
- `pub(crate) fn Solution::truncate_after(&mut self, t: f64)`, the only access to the segment store.

**Exact rules (post-processing over the accepted steps; the integration is unchanged):**
- Events require `dense_output = true`; otherwise the solve is refused with `Unsupported`.
- On each accepted step [t_k, t_{k+1}], evaluate g at t_k, at the midpoint (via `at`) and at t_{k+1}. A
  crossing on a half-interval [a, b]: Up when g(a) < 0 ≤ g(b), Down when g(a) > 0 ≥ g(b).
- Localize by Illinois (modified regula falsi) on g(at(t)) until the bracket width is
  ≤ 4ε·max(|t|, 1) or 60 iterations. Report the bracket end on the after-crossing side.
- g(t0) = 0 is not an event. A crossing whose b is a shared step endpoint is reported once.
- A non-finite g value gives `Status::Failed` with `EventFunctionNonFinite`. The solution is truncated at
  that step.
- The first terminal event in time truncates `t`, `y` and the segments after it, and appends `(t_e, y_e)`.
  The status is `Success` even if the driver failed later, with `Stats::stop` kept. Work after the event
  stays charged in the counters, and the docs say so. An in-loop stop is a wave-4 item.

- [ ] **Step 1: Write failing tests:**
  - (a) `exponential_decay_half_life`: y' = −y, g = y − 0.5 Down, rtol 1e-8; |t − ln 2| ≤ 1e-6.
  - (b) `oscillator_crossings`: y'' = −y as a 2-system on [0, 10], g = y₀, rtol 1e-8. Either gives 3 events
    at π/2, 3π/2 and 5π/2 within 1e-6; Up gives only 3π/2.
  - (c) `terminal_event_truncates`: (a) as terminal. `t.last() == t_e`, `at(1.0) == None`, status
    `Success`, `terminal_event == Some(0)`.
  - (d) `events_without_dense_output_refused`.
  - (e) `zero_at_t0_is_not_an_event`.
  - (f) `crossing_at_step_endpoint_reported_once`: run once, take an interior accepted t_k, rerun with
    g = t − t_k; exactly one event.
  - (g) `nonfinite_event_function_fails_typed`.
  - (h) `events_do_not_change_integration`: `t`, `y` and counters are bit-identical with and without a
    non-terminal event.
- [ ] **Step 2: Run** `cargo test -p vigilode --locked -j 1 --test events_contracts`; expect a compile failure.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -p vigilode --locked -j 1`; all pass.

## Task 6: CI-docs — production CI gate, MSRV, examples (node id `ci-docs`)

Plan §1 G-A (`cargo semver-checks` clean, docs build), §6 E2; wave-1 ruling (MSRV 1.94.1 unverified in the
cloud). The repository has research workflows only; no workflow builds or tests the production crates.

**Files:**
- Create: `.github/workflows/production.yml`.
- Create: `crates/vigilode/examples/{robertson.rs, dense_output.rs, dae_mass.rs, banded_fd.rs}`.
- No edit to any `src/` file (other nodes own them).

**Exact content:**
- Triggers: `pull_request` and `push` to `main`, on paths `crates/**`, `Cargo.toml`, `Cargo.lock`,
  `rust-toolchain.toml` and the workflow file. The toolchain is the pinned 1.94.1 from
  `rust-toolchain.toml`, which makes this the MSRV check.
- Jobs:
  1. `fmt`: `cargo fmt --check -p vigilode -p rodas5p-core -p rodas5p-krylov`. Integrators is excluded:
     `tests/production_audit_packet_curve.rs` is unformatted from before.
  2. `clippy`: `cargo clippy -p vigilode --all-targets --locked -- -D warnings`.
  3. `test`: `cargo test --locked -p vigilode -p rodas5p-core -p rodas5p-krylov -p vigilode-coupled`, and
     `cargo test --locked -p rodas5p-integrators --lib` plus the `--test` list of production contract tests
     named in Tasks 2–4 of waves 1–3.
  4. `docs`: `RUSTDOCFLAGS="-D warnings" cargo doc -p vigilode --no-deps --locked`,
     `cargo test -p vigilode --doc --locked`, `cargo build -p vigilode --examples --locked`, and each example
     run with `cargo run --example`.
  5. `semver`: `cargo-semver-checks` installed at an exact pinned release (`cargo install --locked --version <X>`;
     the implementer records X), run with `--baseline-rev ${{ github.event.pull_request.base.sha }} -p vigilode`.
     It has `continue-on-error: true` until the first published 0.x baseline; G-A makes it blocking.
  6. `bench`: `apt-get install gfortran libsundials-dev`, then
     `cargo test --manifest-path crates/vigilode-bench/Cargo.toml --locked`.
- Examples, each asserting one check and exiting 0:
  - Robertson with an analytic dense Jacobian; mass conserved to 1e-8.
  - Prothero–Robinson with dense output and `at(t)`; interior ≤ 2 tolerance units.
  - Robertson as an index-1 DAE with mass diag(1, 1, 0); constraint ≤ 1e-12.
  - Brusselator 1D n = 100 with `Jacobian::FiniteDifference` and `.banded(2, 2)`.

- [ ] **Step 1:** Write the examples. Run `cargo build -p vigilode --examples --locked -j 1` and
  `cargo run -p vigilode --example <each> --locked`; all exit 0 on the base.
- [ ] **Step 2:** Write the workflow. Validate it with
  `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/production.yml'))"`. Run jobs 1–4
  locally with the stable toolchain and record their exit codes; job 6 runs in Task 8's handoff.
- [ ] **Step 3:** Node report listing which jobs were run locally and which can only run on GitHub.

## Task 7: P0-api — typed refusals and `#[non_exhaustive]` (node id `p0-api`, stage 2)

Branches from `production/wave3-integration` after Tasks 3, 4 and 5 merge, so every refusal site exists.
Implements the decision above. No arithmetic change.

**Files:** `crates/vigilode/src/{error.rs, solve.rs, options.rs, problem.rs, solution.rs, events.rs, lib.rs}`,
`crates/vigilode/tests/*.rs`.

**Interfaces (new):**
- `SolveError::Unsupported(Unsupported)`, replacing `Unsupported(String)`.
- `#[non_exhaustive] pub enum Unsupported`, with one variant per distinct refusal site in `solve.rs`
  (enumerated at implementation time from `grep -n Unsupported`). `Display` equals the current message text.
- `#[non_exhaustive]` on `Tier`, `InteriorError`, `Jacobian`, `FailureReason`, `Status`, `SolveError`,
  `CallbackError`, `Callback`, `JacobianRecord`, `DriverWork`, `Direction` and `Unsupported`, and on the
  output structs `Stats`, `Solution`, `RouteRecord`, `StopRecord`, `CallbackWork`, `FiniteDifferenceWork`,
  `KrylovStartRecord` and `EventRecord`. Not on `Atol` or `Options`.

- [ ] **Step 1: Write failing tests:**
  - every existing refusal assertion changes from string matching to
    `matches!(e, SolveError::Unsupported(Unsupported::<Variant> { .. }))`, one test per variant;
  - a `compile_fail` doctest showing that an exhaustive `match` on `FailureReason` outside the crate does not
    compile;
  - `stats_json_unchanged`: the serde JSON of a Robertson run's `Stats` is byte-identical before and after.
- [ ] **Step 2: Run** `cargo test -p vigilode --locked -j 1`; expect failures.
- [ ] **Step 3: Implement**, and record the decision text in the crate docs (`lib.rs` "Stability" section).
- [ ] **Step 4: Run** `cargo test -p vigilode --locked -j 1`, the clippy job of Task 6, and
  `cargo build --manifest-path crates/vigilode-bench/Cargo.toml --locked -j 1` (the bench matches on facade
  types); all pass.

## Task 8: Integration (controller)

Branch `production/wave3-integration` from the wave-3 base.

- [ ] **Merge stage 1** in this order:
  - [ ] `ci-docs`.
  - [ ] `p0-atol`.
  - [ ] `p0-events`.
  - [ ] `p0-fastdense`: resolve `solve.rs`/`solution.rs`.
  - [ ] `b1c-staleness`: ledger L-0141.
- [ ] **Cross-node seam steps**, each with a test in `crates/vigilode/tests/p0_contracts.rs`:
  - (i) `Atol::Vector` on the fast dense output route: the W-filtered estimate and the segment scale use
    `Workspace::atol_vector`. Test: the uniform vector equals the scalar bitwise under dense output.
  - (ii) `Solution::truncate_after` implemented over `FastDenseSegments`. Test: Task 5's terminal-event
    tests pass on the fast dense route.
  - (iii) the `production.yml` `--test` list includes the wave-3 test files.
- [ ] **Dispatch Task 7** on the integration tip; merge it.
- [ ] **Task 1b:** `e1b-dense` merges the integration tip into its branch and pushes. Hand over
  `LOCAL_E1B_HANDOFF_W3.md`, ingest the results (Task 1 step 7), and merge `e1b-dense` (ledger L-0142 after
  L-0141).
- [ ] **Full suites as a local handoff.** Write `docs/plan/handoff/LOCAL_TEST_HANDOFF_W3.md` by the W2
  pattern, pinned to the integration SHA. It runs:
  - `cargo test -j 12 --workspace --locked --no-fail-fast`;
  - `vigilode-hybrid`;
  - `vigilode-bench`, with `OPENBLAS_NUM_THREADS=1`;
  - `tools/test_check_research_node.py`;
  - `check-research-node.py --base <wave-3 base>` (expected exit 0);
  - every `production.yml` job command with the pinned 1.94.1, including semver (informational).

  VALIDATION_MATRIX expectations:
  - workspace failed = 0;
  - ignored ≈ the wave-2 local count plus the new `#[ignore]`d measurement tests (b1c 1);
  - checker exit 0.

  Results return on `results/local-wave3-<sha>`.
- [ ] Push; append the wave-3 rows to `docs/plan/EXECUTION_LOG.md`; whole-branch final review; one fix wave.
