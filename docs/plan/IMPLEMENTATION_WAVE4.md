# Implementation plan — production wave 4

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** wave 4 has five aims:
- close the last P0 interior-error gap on the sequential route;
- find out by measurement why the dense route's global error outgrows its tolerance, and open a controller card only if the theorems support it;
- open B1d, a staleness bound that needs no weighted log norm;
- harden the allocation contract tests;
- give the facade its own `Stats` types before any semver baseline.

**Architecture:**
- Stage 1: four nodes on disjoint files, each in its own worktree.
- Stage 2: two nodes that depend on stage-1 results. The `Stats` node waits for the sequential fix, because both edit the facade. The controller card waits for the study's verdict.
- The final task integrates. The full suites go to the owner's workstation through a handoff.
- Counter-only measurements (no timing) run in the cloud.

**Tech stack:** Rust (stable 1.97 in the cloud; pinned 1.94.1 on the workstation and in CI), faer, serde; the
`vigilode-bench` workspace (gfortran 13.3.0, SUNDIALS 6.4.1, Diffsol 0.16.1).

**Spec:** `docs/plan/PRODUCTION_PLAN_20261020.md` (§3 P0, §5 T1–T5, §6 A2/B1, §7, §8, §9). Records:
- `docs/plan/WAVE3_REPORT.md`: the facts this plan argues from;
- `docs/plan/OPEN_ITEMS.md`: the item ids `O-nn` cited below;
- `docs/plan/IMPLEMENTATION_WAVE3.md`;
- `docs/plan/sdd/WAVE{1,2,3}/progress.md`.

**Base:**
- `production/wave3-integration` at `f2f20c1` (worktree `/home/claude/wt/integration3`), plus the plan-branch merge of Task 0.
- Every symbol cited below as existing was checked at `f2f20c1`.
- Symbols marked **new** are defined by the task that creates them.

## Global Constraints

- **Earlier constraints.** Every wave-1, wave-2 and wave-3 Global Constraint applies:
  - `--locked`;
  - opt-in new behaviour;
  - bit-identical defaults unless a task says otherwise;
  - type-distinct statuses (estimate, receipt, certificate);
  - counted failures (attempted vs completed);
  - fmt and clippy clean in touched files;
  - a node report at `docs/plan/nodes/<node-id>.md` (**enforced in wave 4**, see O-37).
- **Cloud host:** 2 cores and about 18 GB free disk.
  - Before every cargo call, set `export RUSTUP_TOOLCHAIN=stable CARGO_TARGET_DIR=/home/claude/targets/<node-id> CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`.
  - Then run `cargo ... -j 1 --locked`; use `-j 2` only when no other node is building.
  - Use one target directory per node. At most **three** node builds exist at once (about 4.5 GB each; the bench target is larger).
  - A node checks `df -h /home/claude` before each large build, and stops and reports below 4 GB.
  - The controller deletes a node's target directory after that node merges. Never `cargo clean`.
- **Heavy runs go to the workstation.** The cloud runs targeted tests and counter-only measurements. `cargo test --workspace`, full package suites and anything timed run on the owner's workstation through `docs/plan/handoff/LOCAL_*_W4.md`. The W2/W3 pattern applies:
  - sections CODE_TASK, Setup, Commands, SCIENTIFIC_CONTRACT, VALIDATION_MATRIX, SUMMARY.json and Return path;
  - results come back on `results/local-*` branches;
  - the controller pushes the handoff document before handing it over;
  - the local session edits no code.
- **Commits.** Use `git -c user.name="Jiwon Park" -c user.email="cosmosapjw@gmail.com" commit`, with the session trailers. A node pushes only its own branch `production/<node-id>`. No force-push, and no merges into other branches.
- **Dependencies.** No new external crates in the root workspace. `crates/vigilode-bench` stays its own workspace.
- **Ledger.** `research/LEDGER.jsonl` is append-only, and `python3 tools/check-research-node.py --base <wave-4 base SHA>` checks it.
  - Wave-4 ids are pre-assigned in merge order: **L-0143 → g1-global**, **L-0144 → b1d-resolvent**, **L-0145 → a5-epus** (only if that card opens).
  - An id whose node ends without a row stays unused and is never reassigned (wave-3 ruling R-C6).
  - A node appends its row only on a branch that already contains every lower wave-4 id that was assigned.
  - The row date is the measurement date.
- **Rulings that bind:**
  - `wrms` stays strict, and `step_error_wrms` carries the zero-scale rule.
  - Preregistered kill gates are never re-scored or loosened. B1d is a new card, not a re-score of B1c.
  - **`Tier::Performance` routes no lagged driver in wave 4, whatever B1d's verdict.** A PASS opens gates 3–4 of plan §8 in wave 5.
  - The facade gains no option for the a5-epus mechanism in wave 4. Exposure waits for gates 3–4.
  - One `STATS_SCHEMA` bump in wave 4: `"vigilode.stats.v4"`, made by Task 1. Task 5 keeps the serialized bytes of the post-Task-1 tip.
- **Claim discipline.** Node reports tag each statement D (measured or recorded), I (interpretation) or H (hypothesis). No speed claim is made from a cloud or loaded-host run.

## Review Focus

1. A recoverable RHS failure exactly at the midpoint of the **first** attempt from `t0`, on the sequential route under `WFilteredEnforce`. Expected: the attempt is rejected, the retry from `t0` with a smaller step succeeds, the run reaches `t1`, and `at(t)` is unaffected for every kept step. The same run under `WFilteredReport` must stay bit-identical to the clean run. Owner: Task 1, step 1 (a, c).
2. A zero weight `s_i = 0` (atol = 0 with a component that is exactly 0) under the new `StalenessRule::Resolvent`. Expected: the bound is `+inf` and P is refreshed. The result is never NaN, and the run equals the fast driver within 1e-12. Owner: Task 3, step 1 (g).
3. A study row whose global error never reaches one tolerance unit. Expected: no ratio is formed on near-zero errors, the class is `ok`, and no `NaN` or `null` appears in a numeric field of the JSON. Owner: Task 2, step 1 (c).
4. The a5-epus scaling on an empty span (`t1 == t0`) and on a single-step run (h = T). Expected: no attempt on the empty span, and a factor of exactly 1 when h = T. A default run stays bit-identical. Owner: Task 6, step 1 (c).
5. A `WorkCounters` value with every field nonzero, serialized through the new facade mirror. Expected: byte-identical JSON to the internal type, so that a field added to the internal type and forgotten in the mirror fails a test. Owner: Task 5, step 1 (b).

## Decisions taken in this plan

- **Highest-value set.** Wave 4 takes O-01, O-02, O-03, O-05/O-06 and O-26 (`OPEN_ITEMS.md`), plus the hygiene items that share their files (O-09, O-11, O-12, O-13, O-14, O-17, O-29, O-46, O-47) and the process items of Task 7 (O-07, O-36).
  - A3, A4, B2, the in-loop event stop and vector atol on the sequential route move to wave 5 (O-21–O-25).
  - Vector atol on the sequential route needs a vector form of the public `finish_step(.., atol: f64, ..)` (`sequential.rs:826`) and of three `&[atol]` estimate sites in `dense_output_v2.rs` (l. 1162, 1194, 1245). That is a separate plumbing node, and it is not on the G-A critical path.
- **Global error before any controller change** (Tasks 2 and 6).
  - The E1b facts are in `WAVE3_REPORT.md` §3.2: 20/30 dense rows are above one unit, Brusselator reaches 9.0 with 0 interior rejections, and the slope is about 0.78. They do not say *why*.
  - Plan §5 gives one controller change with a theorem behind it, the error-per-unit-step criterion of T4 under μ ≤ 0. It helps only if the local errors are within tolerance and the flow does not amplify them.
  - So the study comes first (gate-1/2 style, counters and errors only), and its verdict decides whether the controller card opens. This is §8's "measure before you write" rule.
- **B1d mechanism (Task 3): the resolvent norm, not a row-scaled norm.** Two candidates were considered.
  - *Row-scaled norm* ‖x‖_D = max|x_i|/d_i with D chosen to shrink μ_D(J_P). Rejected. The stopping test must certify WRMS in the S-weights, and the conversion ‖v‖_S ≤ max_i(d_i/s_i)·‖v‖_D, with ‖v‖_D ≤ max_i(s_i/d_i)·‖v‖_S on the residual side, costs the condition number κ(D, S) = max(d/s)·max(s/d). That reintroduces the same weight ratios (≈ 1e4 on HIRES) that sank B1c.
  - *Exact resolvent norm* ‖R‖_S with R = (I − h_Pγ J_P)⁻¹. Adopted. R is formed once per refresh by n back-solves with the LU just computed. The bound needs no log-norm hypothesis, is finite whenever P is nonsingular, and is never larger than B1c's bound (Lemma B1d). It does not remove the weight ratios from ‖R‖_S itself. Whether that is enough on HIRES is the open question the kill gate answers.
- **Facade-owned `Stats` types now** (Task 5). The semver job becomes blocking at G-A. A first 0.x baseline that exposes eleven internal types would freeze them. Mirrors keep the serialized bytes, and the public field names stay the same, so `crates/vigilode-bench` compiles unchanged.
- **Wave-3 node reports (O-37).** The six missing wave-3 node reports are served by `docs/plan/sdd/WAVE3/task-N-report.md`. Task 0 records this ruling. No backfill files are written.

## Dependency table

| Task | Node id / branch / worktree | Stage | Depends on | Tier hint | Est. wall clock |
| --- | --- | --- | --- | --- | --- |
| 0 | controller, `production/wave4-integration` | pre | — | — | controller, 0.5 h |
| 1 | `p0-seqdense` / `production/p0-seqdense` / `/home/claude/wt/p0-seqdense` | 1 | 0 | standard | 4–5 h |
| 2 | `g1-global` / `production/g1-global` / `/home/claude/wt/g1-global` | 1 | 0 | most capable | 6–8 h (build + run, cloud) |
| 3 | `b1d-resolvent` / `production/b1d-resolvent` / `/home/claude/wt/b1d-resolvent` | 1 | 0 | most capable | 8–10 h |
| 4 | `h-alloc` / `production/h-alloc` / `/home/claude/wt/h-alloc` | 1 | 0 | cheap | 3–4 h |
| 5 | `p0-stats` / `production/p0-stats` / `/home/claude/wt/p0-stats` | 2 | 1 merged | standard | 6–8 h |
| 6 | `a5-epus` / `production/a5-epus` / `/home/claude/wt/a5-epus` | 2 (conditional) | 2 merged with verdict PASS; 3 merged | most capable | 6–8 h |
| 7 | controller, `production/wave4-integration` | final | all | — | controller + 3 h local |

Dispatch order on 2 cores and about 18 GB:
- **Batch 1:** Task 3 (long pole), Task 1 and Task 2.
- **When the first of these merges:** Task 4.
- **Task 5** branches from the integration tip after Task 1 merges.
- **Task 6** branches after Tasks 2 and 3 merge, and only on a g1 PASS.

## Pre-flight conflict scan

Shared files and interfaces, checked at `f2f20c1`:

| # | Pair | Shared surface | Conflict | Sequencing / ruling |
| --- | --- | --- | --- | --- |
| P1 | T1 × T5 | `crates/vigilode/src/{solution.rs, solve.rs, lib.rs}`, `tests/p0_contracts.rs`, `tests/golden/robertson_stats.json` (schema string) | Both edit `DriverWork::SequentialDense`, `STATS_SCHEMA` and the golden. | T5 branches from the tip that holds T1. T1 owns the only schema bump (v4). T5's byte-identity baseline is the post-T1 tip. |
| P2 | T2 × T6 | `crates/vigilode-bench/src/{global.rs (new), main.rs}` | T6 adds an arm to T2's study harness. | T6 starts only after T2 merges (it is conditional on T2 anyway). |
| P3 | T3 × T6 | `crates/rodas5p-integrators/src/lib.rs` export block | Adjacent `pub use` hunks; trivial. | T6 after T3. T3 must not edit `rodas5p_fast.rs`: it uses the existing `pub(crate) fn solve_dense_into(&self, b: &mut [f64])` (`rodas5p_fast.rs:907`). |
| P4 | T6 × all users of `Rodas5pFastOptions` | 20 struct literals, 9 without `..Default` (12 in `rodas5p-cli/src/stiff_benchmark.rs`) | Adding a field would break them. | **Ruling:** T6 adds **no** field to `Rodas5pFastOptions`. The mechanism enters through a provided method of the private `StepSink` trait (`rodas5p_fast.rs:1487`) and a new entry point. |
| P5 | T4 × T1/T3 | `rodas5p-integrators` formatting | `cargo fmt -p rodas5p-integrators` could reformat other nodes' files. | T4 formats the one file with `rustfmt --edition 2024 crates/rodas5p-integrators/tests/production_audit_packet_curve.rs`. Every node keeps its own files fmt-clean. |
| P6 | T4 × T7 | `.github/workflows/production.yml` | T4 edits the fmt and clippy jobs. T7 adds nothing to the file. | No conflict. |
| P7 | T2 × T3 × T6 | `research/LEDGER.jsonl` EOF | Three appends. | Ids L-0143/0144/0145 in this merge order. T3 merges its code only after L-0143 is on the integration tip, then appends L-0144. |
| P8 | T1 × T4 | `crates/rodas5p-integrators/tests/` | T1 creates `sequential_estimate_failure_contracts.rs`; T4 edits `fast_dense_allocations.rs`. | Disjoint files. |
| P9 | T5 × bench | `crates/vigilode-bench/src/arms.rs` reads `st.counters.{rhs_evaluations, jacobian_builds, direct_factorizations, direct_solve_calls}`, and `DriverWork::FastDense { interior }` / `SequentialDense { .. }` fields (l. 738–800) | Mirror types must keep these field names. | T5 step 4 builds the bench (`cargo check --manifest-path crates/vigilode-bench/Cargo.toml --all-targets --locked`). Bench source edits are not expected and need a recorded deviation if made. |

False premises checked: none found in the tasks below. Every existing symbol they cite is listed in their **Interfaces** with its file and line.

---

## Task 0: base and plan merge (controller)

- [ ] Create `production/wave4-integration` = `f2f20c1` + merge of `production/vigilode-plan-20261021` (docs only). This is the wave-3 B0 lesson: without the merge, no worktree contains this plan. Record the merge SHA as the **wave-4 base** in `docs/plan/sdd/WAVE4/progress.md`.
- [ ] Run `python3 tools/check-research-node.py --base <wave-4 base>` and `--base a5a48f77e7ac7a80b8ff86cd897110790066f7ee`. Both are expected to exit 0 with 142 rows. Record the exit codes.
- [ ] Record the ruling for O-37 (wave-3 node reports are the archived task reports) in `progress.md`.
- [ ] Delete the stale target directories of wave-3 nodes, then record `df -h /home/claude`.
- [ ] Create worktrees for Tasks 1–4 from the wave-4 base.

## Task 1: P0-seqdense — the sequential route rejects an attempt whose interior estimate fails (node id `p0-seqdense`)

Plan §3 P0 ("endpoint and interior error controlled"), §5 T3. Closes O-01.

**Current behaviour [V]:** in `integrate_adaptive_dense_impl` (`dense_output_v2.rs:1719`), when the interior estimate of an attempt that the embedded estimate accepted returns `Err`, the code does `Err(_) => dense_report.unavailable += 1` (l. 1954), and the step is **accepted** under every mode. The fast dense route rejects such an attempt like a stage error (`rodas5p_fast_dense.rs` module doc, l. 17–21).

**Rule (new):**
- Under `DenseErrorControl::Enforce` and `DenseErrorControl::WFilteredEnforce`, an estimate error `e` is handled as follows:
  - If `adaptive_failure_kind(&e)` is `Some(kind)`, reject the attempt as the stage-error path does (l. 1857–1883):
    - error = +∞;
    - failure = `kind`, recorded with `record_adaptive_work_failure` and `diagnostics.record_with_failure(.., false, Some(kind))`;
    - the next step comes from `rodas_next_step_after_attempt(.., f64::INFINITY, false, ..)`;
    - `recycle` and `history` are restored from their snapshots.
  - Otherwise return `Err(e.into())`, as l. 1885 does.
  - `unavailable` is not incremented in either case.
- Under `Report`, `ReportDefect` and `WFilteredReport`: unchanged. The error is counted in `unavailable`, and the step sequence is never changed.
- The estimate's work counters are accumulated exactly as now (l. 1956–1962).

**Files:**
- Modify: `crates/rodas5p-integrators/src/dense_output_v2.rs`: the estimate `match` at l. 1928–1955, and `DenseErrorReport` (l. 1117–1127) gains a field.
- Modify: `crates/vigilode/src/{solution.rs, solve.rs, options.rs, lib.rs}`: `DriverWork::SequentialDense`, the fill at `solve.rs:437-451`, the `STATS_SCHEMA` note, and the `InteriorError` docs. The routing table in `lib.rs` is unchanged.
- Modify: `crates/vigilode/tests/golden/robertson_stats.json` (the schema string only).
- Test: `crates/rodas5p-integrators/tests/sequential_estimate_failure_contracts.rs` (new); `crates/vigilode/tests/p0_contracts.rs`.

**Interfaces:**
- Consumes: `integrate_adaptive_dense_observed_with_segments(problem, t_span, y0, method, linear_config, sabr_config, adaptive, sampling, control, interior) -> DenseOutputResult<AdaptiveDenseSegmentRun>` (`dense_output_v2.rs:1647`); `InteriorSampling::EveryStep`; `prothero_robinson_problem(lambda, mu, t0)`, `robertson_problem()`, `scalar_linear_problem(lambda, y0)` (`problems.rs`).
- Produces:
  - `DenseErrorReport::estimate_failures: usize` (**new**): attempts rejected because the estimate failed under an enforcing mode.
  - `DriverWork::SequentialDense { .., estimate_failures: usize }` (**new** field).
  - `STATS_SCHEMA = "vigilode.stats.v4"`. Its note: "v4 (wave 4): under `Enforce`/`WFilteredEnforce` on the sequential route an estimate error rejects the attempt; `unavailable` counts report-mode samples only; new `estimate_failures`."

- [ ] **Step 1: Write failing tests** (`sequential_estimate_failure_contracts.rs` unless noted):
  - Probe design: run the clean problem once to get the first accepted step `[t_a, t_b]`. Then rerun with an RHS that returns `CoreError::NonFinite` on its first call with `t == t_a + 0.5 * (t_b - t_a)` (bitwise, one-shot latch, as in `fast_dense_contracts.rs` test (f)). Use the problem `scalar_linear_problem(-1.0, 1.0)` on `[0, 2]` at rtol 1e-6, atol 1e-8, with `InteriorSampling::EveryStep`. No RODAS5P stage abscissa equals 1/2, so only the interior estimate hits that time.
  - (a) `estimate_failure_rejects_under_enforcing_modes`: for `[Enforce, WFilteredEnforce]`:
    - `report.estimate_failures == 1`, `report.unavailable == 0`;
    - `rejected_steps == clean.rejected_steps + 1`;
    - `nonfinite_step_failures == clean + 1`;
    - the run succeeds at `t = 2`;
    - every kept segment's `evaluate(t_new)` equals its stored `y_new` bitwise.
  - (b) `unclassified_estimate_error_stops_the_run`: the same probe, but the RHS returns `CoreError::InvalidInput` at the midpoint. `adaptive_failure_kind` (`dense_output_v2.rs:3253`) classifies only `LinearSolve`, `NonlinearSolve` and `NonFinite`, so the driver call must return `Err` under both enforcing modes. It never returns an accepted run.
  - (c) `report_modes_still_count_unavailable`: for `[Report, WFilteredReport]`, the same single failure gives `unavailable == 1`, `estimate_failures == 0`, and `t`, `y` and counters bit-identical to the clean run.
  - (d) `runs_without_estimate_errors_are_unchanged`: pins taken **in the RED commit from the base code**. PR λ = −1e5 on `[0, 2]` and Robertson on `[0, 40]`, under Enforce and WFilteredEnforce at rtol 1e-6. The pins are endpoint bits, `attempts`, `samples.len()` and `rejected_steps`.
  - (e) facade, `p0_contracts.rs` `a_recoverable_midpoint_failure_rejects_on_both_dense_routes`: the decay problem with a `CallbackError::Recoverable` at the first midpoint, run twice:
    - with `.mass(vec![1.0])` (sequential route): `SequentialDense { estimate_failures: 1, unavailable: 0, .. }`;
    - without mass (fast route): `FastDense` and `rejected_steps == clean + 1`.

    Both runs give `callbacks.recoverable_errors == 1` and are successes.
  - (f) facade `stats_schema_is_v4`: `sol.stats.schema == "vigilode.stats.v4"`.
- [ ] **Step 2: Run** `cargo test -p rodas5p-integrators --locked -j 1 --test sequential_estimate_failure_contracts` and `cargo test -p vigilode --locked -j 1 --test p0_contracts`. Expect a compile failure (the field is missing) and failures in (a), (b) and (e).
- [ ] **Step 3: Implement** the rule, the new field, the facade fill, the schema bump, the golden schema line, and the docs. The docs are `InteriorError::{Enforce, WFilteredEnforce}` in `options.rs` and the `DriverWork::SequentialDense::unavailable` doc, which now reads "report modes only".
- [ ] **Step 4: Run** the new tests and the regression set. All pass.
  - `dense_wfiltered_contracts`, `p0_partial_contracts`, `fast_dense_contracts`, `a1_zero_tolerance_scale`;
  - `cargo test -p vigilode --locked -j 1` (all targets, including `p0_api_contracts::stats_json_unchanged` against the updated golden);
  - `cargo clippy -p vigilode --all-targets --locked --no-deps -- -D warnings`;
  - `cargo check --manifest-path crates/vigilode-bench/Cargo.toml --all-targets --locked`.
- [ ] **Step 5: Node report** `docs/plan/nodes/p0-seqdense.md`, with the pins of (d), the counts of (a)–(c), and the D/I/H tags.

## Task 2: G1-global — global-error and tolerance-proportionality study (node id `g1-global`, ledger L-0143)

Plan §2 "Global error" (F-033), §5 T4, §7 rule 2. Closes O-02 (study part) and O-17. This is a measurement node with a preregistered PASS/FAIL hypothesis. It changes no solver code.

**Question:**
- For the dense route that the facade ships (`vigilode-dense`, `WFilteredEnforce`), is the global error above 2 tolerance units on Brusselator and van der Pol (WAVE3_REPORT §3.2) the accumulation of local errors that are each within tolerance (**H1**)?
- Or does the estimator under-report the local error, or the flow amplify it (**H2**)?

**Definitions (exact, preregistered).**
- The run is the facade's `Rodas5p::solve` with `arms::dense_options(rtol, atol, InteriorError::WFilteredEnforce, None)` and the problem `arms::facade_problem(&model, None)`. Here atol = rtol · atol_scale (per component, as the corpus defines it). The accepted step ends are `t_0 < t_1 < … < t_N` (`sol.t`), with states `y_k`.
- **Reference:** the facade dense solve at rtol 1e-12 with the problem's `reference_atol`, read through `at(t_k)`; for `linear-exact`, `Model::exact(t_k)`.
- **Local error:** ℓ_k = y_{k+1} − φ_k, where φ_k is the endpoint of the facade solve from `(t_k, y_k)` over `[t_k, t_{k+1}]` at rtol 1e-12, reference atol, `dense_output = false`.
  - Its size in the controller's units is L_k = WRMS(ℓ_k; atol_i + rtol·max(|y_k,i|, |y_{k+1},i|)).
- **Global error at step ends:** E_K = WRMS(y_K − y_ref(t_K); w_K), with w_K,i = atol_i + rtol·|y_ref,i(t_K)|. These are the E1b `error_own` weights.
- **Accumulation ratio:** A_K = E_K / Σ_{k<K} WRMS(ℓ_k; w_K). It is formed only where E_K ≥ 1. One fixed norm per K makes this the triangle-inequality test of T4 with μ ≤ 0 (Lemma A5 below, Task 6).
- **Grid error:** E_grid = max over the corpus grid (101 points, `at`) of WRMS with the E1b weights. This is exactly E1b's `error_own` definition (`metrics::max_wrms`).
- **Context, not gated:** M = max_k h_k·μ_w(J(y_k)), with w = the controller weights at k and μ_w(A) = max_i (A_ii + Σ_{j≠i}|A_ij| w_j/w_i), using `Model::jac_dense_row_major`. Tolerance slope p: the least-squares slope of log10(E_grid·rtol) on log10(rtol) over the three rtols.
- **Row classes:**
  - `ok`: max_K E_K ≤ 2;
  - `acc`: max_K E_K > 2, max_k L_k ≤ 1 and max A_K ≤ 1;
  - `under`: max_k L_k > 1;
  - `amp`: max_k L_k ≤ 1 and max A_K > 1.

**Corpus** (`research/g1_global_error_YYYYMMDD/corpus_g1.json`, schema `vigilode-bench.corpus.v1`):
- The four problems of `research/bench_wave3_20261021/corpus_v2.json` with the same `atol_scale`, `reference_atol` and `t_end`;
- plus `{"id": "linear-exact", "kind": "linear-exact", "n": 2, "t_end": 10.0, "reference_atol": 1e-12, "banded": false, "atol_scale": [1.0, 1.0]}`;
- rtols {1e-4, 1e-6, 1e-8}.

**Hypothesis and verdict (preregistered):**
- **H1 PASS** iff every row of `brusselator-1d-n100` and `van-der-pol-mu1000` with max_K E_K > 2 has class `acc`, and at least one such row exists.
- Otherwise **FAIL**. The verdict names the class of each failing row.
- **Validation (reported, not part of the verdict):**
  - E_grid of the 15 `WFilteredEnforce` dense rows reproduces E1b's `error_own` (REPORT.md "Interior control evidence") within 1e-9 relative. A mismatch means that the code or the reference moved since `33e60cf`; it is reported as such.
  - For `linear-exact`, the facade reference agrees with `Model::exact` within 1e-10 WRMS units.
- **Consequence fixed now:** Task 6 opens **only on PASS**. On FAIL, wave 4 opens no controller card, and the classes go to wave 5 (A3 or estimator work).

**Files:**
- Create: `crates/vigilode-bench/src/global.rs`.
- Modify: `src/main.rs` (subcommand `global`), `src/lib.rs` (`pub mod global;`), `tests/adapters.rs` (O-17 test).
- Create: `research/g1_global_error_YYYYMMDD/{PREREGISTRATION.md, corpus_g1.json, results_g1.json}`, ledger row **L-0143**, `docs/plan/nodes/g1-global.md`.

**Interfaces:**
- Consumes (existing):
  - `vigilode_bench::arms::{facade_problem(model: &Model, band: Option<(usize, usize)>) -> Problem, dense_options(rtol: f64, atol: f64, interior: InteriorError, max_steps: Option<usize>) -> Options}` (`arms.rs:665, 685`);
  - `model::{Model, Kind}` with `Model::{new, y0, exact, jac_dense_row_major}`;
  - `metrics::max_wrms`;
  - `corpus::Corpus::from_json`;
  - `vigilode::{Rodas5p, Options, Atol, InteriorError, Solution}`.
- Produces:
  - `pub fn run_global(c: &Corpus, log: Log) -> Result<serde_json::Value, String>` (**new**), using the same `Log` type as `harness::run_corpus`;
  - CLI `vigilode-bench global --corpus <file> --out <json>` (**new**);
  - JSON rows `{problem, rtol, accepted, e_grid, e_max, l_max, a_max, m_context, class, steps: [{t, h, l_units, e_units, a}]}` plus `{slope_p}` per problem and `{verdict, validation}` at the top.
  - Every number is finite, and absent values are omitted (Review Focus 3).

- [ ] **Step 1: Write failing tests** (`src/global.rs` unit tests and `tests/adapters.rs`):
  - (a) `local_error_of_exact_step_is_zero_to_reference_precision`: on `linear-exact` at rtol 1e-6, with φ_k replaced by `Model::exact` propagated from `y_k`, a tolerance of 1e-6 units matches the facade-reference L_k. This validates the per-step reference.
  - (b) `class_rules`: unit tests of the class function on hand-made `(e_max, l_max, a_max)` triples, one per class, including the boundaries 2.0 (`ok`) and 1.0 (`acc`).
  - (c) `tiny_errors_form_no_ratio`: a row with every E_K < 1 has no `a` values, class `ok`, and serializes without `NaN` or `null` in numeric fields.
  - (d) O-17 `protocol_violation_status_is_reachable` in `tests/adapters.rs`: drive the status logic of `vigilode_dense` with a grid time outside the span. `at` returns `None`, so the status is `ProtocolViolation`.
- [ ] **Step 2: Run** `cargo test -j 1 --locked` in `crates/vigilode-bench`. Expect a compile failure.
- [ ] **Step 3: Implement** `global.rs` and the subcommand. One facade solve per step for φ_k; no parallelism.
- [ ] **Step 4: Run** `cargo test -j 1 --locked` (all pass) and `cargo clippy -j 1 --locked --all-targets -- -D warnings`.
- [ ] **Step 5: Preregister.** Commit and push `PREREGISTRATION.md` and `corpus_g1.json` before any measurement. The preregistration contains:
  - the definitions, the classes and the verdict rule above;
  - the command;
  - "counters and errors only, no timing";
  - the Task 6 consequence.
- [ ] **Step 6: Measure (cloud).**
  - Build with `cargo build -j 1 --locked --profile measurement`.
  - Run `vigilode-bench global --corpus research/g1_global_error_YYYYMMDD/corpus_g1.json --out research/g1_global_error_YYYYMMDD/results_g1.json 2> run.log`.
  - Commit the numeric file first.
  - If the run exceeds 90 min wall time, stop it, and the controller moves it to `LOCAL_G1_HANDOFF_W4.md`. Same command; only the host changes.
- [ ] **Step 7: Ledger L-0143** with the verdict PASS or FAIL as measured, `supersedes: null`, `inputs_sha256` (corpus, preregistration) and `outputs_sha256` (results, run.log). Then write the node report and run `check-research-node.py --base <wave-4 base>` (expected exit 0).

## Task 3: B1d-resolvent — a staleness bound from the exact resolvent norm (node id `b1d-resolvent`, ledger L-0144)

Plan §5 T1, T2, T9; §6 B1; §8 (new card, new mechanism). Closes O-03, O-12 (generator) and O-13.

**Mathematics (preregistered; T-status in exact arithmetic, rounding ignored).** The notation is that of Lemma B1c (`IMPLEMENTATION_WAVE3.md` Task 2):
- W = I/(hγ) − J and P = I/(h_Pγ) − J_P, with γ = 0.21193756319429014;
- weights s_i > 0;
- ‖x‖_S = max_i |x_i|/s_i and ‖A‖_S = max_i Σ_j |A_ij| s_j/s_i;
- R := (I − h_Pγ J_P)⁻¹ = P⁻¹/(h_Pγ).

*Lemma B1d.* If h, h_P > 0 and P is nonsingular, then

```latex
T := I - P^{-1}W = R\big[(1 - h_P/h)\,I + h_P\gamma\,(J - J_P)\big],\qquad
\|T\|_S \le q_R := \|R\|_S\,\big(|1 - h_P/h| + h_P\gamma\,\|J - J_P\|_S\big).
```

If moreover h_Pγ μ_S(J_P) < 1, then ‖R‖_S ≤ 1/(1 − h_Pγ μ_S(J_P)), and so q_R ≤ q_pred (the bound of Lemma B1c).

*Proof sketch.*
- The identity is the one in the proof of Lemma B1c: P − W = (h_Pγ)⁻¹[(1 − h_P/h)I + h_Pγ(J − J_P)], and P⁻¹(h_Pγ)⁻¹ = R.
- ‖·‖_S is an induced norm, so it is submultiplicative. Apply it to the product and use the triangle inequality on the bracket.
- The second claim is Lemma 1 (the resolvent bound through the log norm) in ‖·‖_S. ∎

*Corollary (stop).* If q_R < 1, then ‖x − W⁻¹c‖_S ≤ ‖P⁻¹r‖_S/(1 − q_R) by T2's Neumann bound. The stage stop ‖P⁻¹r_k‖_S ≤ (1 − q_R) ρ_i therefore implies WRMS(x_k − W⁻¹c) ≤ ρ_i, with ρ_i = `rodas5p_lagged_stage_targets()` (θ = 0.1).

*What changes against B1c, and what does not.*
- q_R is finite whenever P is nonsingular. On HIRES this removes the 75 `+inf` checks, which is the cause recorded in L-0141.
- ‖R‖_S still carries the ratios s_j/s_i. The lemma does not show that q_R ≤ 0.5 at the steps where B1c refreshed. That is exactly what the gate measures.
- **No prediction is registered.**

**Cost:** n back-solves with the fresh LU per refresh, so the back-solves per accepted step are n × (refreshes per accepted step). An O(n²) |R| buffer is allocated once at setup. Each check costs O(n²) with the attempt's current weights. The back-solves are charged as `direct_solve_calls` and in `StalenessWork::resolvent_solves`. The kill gate counts factorizations only, as all B1 cards have. A PASS still has to beat the protected path on matched time at gate 3, which charges these solves.

**Rules (constants fixed now, never tuned):**
- Refresh before an attempt's stage solves iff q_R > q_max = `RODAS5P_LAGGED_STALENESS_Q_MAX` = 0.5. Same ordering as B1c (ruling B2c: only with a valid P and no initial or forced refresh due).
- Stage stop ‖P⁻¹r_k‖_S ≤ (1 − q_R) ρ_i.
- A weight that is not finite and positive gives `+inf` (B2d).
- The GMRES arm is otherwise unchanged:
  - restart `RODAS5P_LAGGED_GMRES_RESTART` = 10;
  - column cap `RODAS5P_LAGGED_MAX_ITERATIONS` = 6;
  - contraction refresh on (ν_k/ν_0)^{1/k} > `RODAS5P_LAGGED_CONTRACTION_MAX` = 0.5.

**Kill gate (§6 B1, identical to B1, B1b and B1c):** HIRES and Brusselator 1D n = 100 (`tests/b1_problems/mod.rs`), rtol 1e-6, reference = the fast driver at rtol 1e-11, arm `StageSolver::Gmres` + `StalenessRule::Resolvent`. Per problem, all three must hold:
- factorizations per accepted step ≤ 0.4;
- accepted steps ≤ 1.05 × fast;
- endpoint error ≤ 1.5 × fast's (tolerance units).

PASS iff both problems pass. Richardson + Resolvent is measured as context only. Context columns are also recorded on the gated arm:
- `resolvent_solves` per accepted step;
- checks where B1c's q_pred = +∞ but q_R ≤ 0.5 (`rescued_checks`).

**Files:**
- Modify: `crates/rodas5p-integrators/src/rodas5p_lagged.rs` and the `lib.rs` export block. **No edit** to `rodas5p_fast.rs` and no facade edit.
- Test: `crates/rodas5p-integrators/tests/b1d_resolvent_contracts.rs` (new), reusing `tests/b1_problems/mod.rs`. Copy the ignored-measurement pattern of `b1c_staleness_contracts.rs:523-660`.
- Create: `research/b1d_resolvent_YYYYMMDD/{PREREGISTRATION.md, results.json}`, ledger row **L-0144**, `docs/plan/nodes/b1d-resolvent.md`.

**Interfaces:**
- Consumes (existing):
  - `StalenessRule` (`rodas5p_lagged.rs:203`), `Rodas5pLaggedOptions` (l. 214), `StalenessWork` (l. 274);
  - `rodas5p_staleness_bound(h, h_p, gamma, j, j_p, s) -> f64` (l. 313);
  - the private `Lagged::refresh` (l. 396) and the check at l. 715–739;
  - `Workspace::solve_dense_into(&self, b: &mut [f64]) -> CoreResult<()>` (`rodas5p_fast.rs:907`);
  - `integrate_rodas5p_lagged_observed(problem, t_span, y0, adaptive, output, options)` (l. 964).
- Produces:
  - `StalenessRule::Resolvent` (**new** variant; `Off` stays the default).
  - `pub fn rodas5p_resolvent_staleness_bound(h: f64, h_p: f64, gamma: f64, j: &DenseMatrix, j_p: &DenseMatrix, abs_r: &DenseMatrix, s: &[f64]) -> f64` (**new**). It returns q_R from |R| entries, `+inf` under the same guards as `rodas5p_staleness_bound`, and never NaN.
  - `StalenessWork::resolvent_solves: u64` and `StalenessWork::rescued_checks: u64` (**new**; zero unless `Resolvent`).
  - `pub const RODAS5P_LAGGED_GMRES_RESOLVENT_DRIVER_ID: &str = "rodas5p-lagged-gmres-resolvent-v1"` and `RODAS5P_LAGGED_RICHARDSON_RESOLVENT_DRIVER_ID = "rodas5p-lagged-richardson-resolvent-v1"` (**new**).

- [ ] **Step 1: Write failing tests** in `b1d_resolvent_contracts.rs`:
  - (a) `resolvent_bound_dominates_explicit_t_norm`: the grid of B1c test (a) (Robertson and HIRES Jacobians at three states, h_P/h ∈ {0.5, 0.8, 1, 1.25, 2}, J_P from an earlier state). |R| comes from an explicit `LuFactorization` of I − h_PγJ_P and n solves. Assert ‖T‖_S ≤ q_R·(1 + 1e-9) at every grid point, and record the largest ratio.
  - (b) `resolvent_bound_never_exceeds_b1c_bound`: same grid; q_R ≤ q_pred·(1 + 1e-12) wherever q_pred is finite.
  - (c) `resolvent_bound_closed_form_where_b1c_is_unbounded`: J = J_P = [[−1, K], [0, −1]] with K = 1e8, s = (1, 1), a = h_Pγ = 1e-6, h_P/h = 0.8. Then q_pred = +∞ and q_R = 0.25·(1/(1+a) + aK/(1+a)²) within 1e-12 relative.
  - (d) `off_and_bounded_unchanged`: `Off` and `Bounded` reproduce the B1c pins of `b1c_staleness_contracts` (d) and the gate counters of `research/b1c_staleness_20261021/results.json` bitwise.
  - (e) `forced_refresh_resolvent_equals_fast`: `refresh_every_attempt` + `Resolvent` matches `integrate_rodas5p_fast_observed` within 1e-12 relative, on Robertson and HIRES at rtol {1e-4, 1e-6, 1e-8}.
  - (f) `resolvent_ledger_identity`:
    - refreshes = initial + forced + contraction + budget + staleness = `direct_factorizations`;
    - `resolvent_solves = n × (number of refreshes)`.
  - (g) O-13 / Review Focus 2, `zero_weight_refreshes_without_nan`: atol = 0 with an identically zero component, under `Bounded` and under `Resolvent`. Expected:
    - the bound is `+inf` on those checks;
    - the run completes and equals the fast driver within 1e-12;
    - `q_pred_max` is finite.
- [ ] **Step 2: Run** `cargo test -p rodas5p-integrators --locked -j 1 --test b1d_resolvent_contracts`. Expect a compile failure.
- [ ] **Step 3: Implement**:
  - the |R| buffer (allocated once at setup when the rule is `Resolvent`);
  - its fill after `ws.factor` in `refresh`: n solves of e_j with `solve_dense_into`, scaled by 1/(h_Pγ), absolute values stored, counted;
  - the check, which also computes q_pred for `rescued_checks`;
  - the stop on both stage solvers.
- [ ] **Step 4: Run** the new tests plus `b1c_staleness_contracts`, `b1b_gmres_contracts`, `b1_lagged_contracts` and `a1_zero_tolerance_scale`. All pass.
- [ ] **Step 5: Preregister.** Commit and push `PREREGISTRATION.md` before any measurement. It contains:
  - the lemma, the rules and the gate;
  - the command;
  - the verdict rule (PASS/FAIL only; a run that cannot complete is reported, and no row is written);
  - "no prediction registered".
- [ ] **Step 6: Measure.** Run the `#[ignore]`d `b1d_kill_gate_measurement` in release with `-j 1`. It writes `results.json` when `B1D_RESULTS_OUT` is set:
  - every counter of fast, Gmres+Resolvent and Richardson+Resolvent;
  - `StalenessWork`;
  - PASS/FAIL per problem.

  Context arms carry `"would_pass_gate"`, never `"verdict"` (O-12). Commit the numeric file first. Do this only after L-0143 is on the integration tip (P7).
- [ ] **Step 7: Ledger L-0144** with the verdict as measured and `supersedes: null`, the node report and the checker exit code.
  - On PASS: "routing waits for wave-5 gates 3–4 (matched-time A/B on the workstation, holdout)".
  - On FAIL: "B1 family closed until a new mechanism", naming the limb that failed.

## Task 4: H-alloc — allocation contract hardening and lint hygiene (node id `h-alloc`)

Plan §3 P0 (zero heap allocation per step after setup), G-A. Closes O-05, O-06, O-11, O-14, O-29 and O-46 (test side). No solver arithmetic changes.

**Files:**
- Modify: `crates/rodas5p-integrators/tests/fast_dense_allocations.rs`.
- Create: `crates/vigilode/tests/facade_allocations.rs` (counting allocator, per-thread, as `fast_dense_allocations.rs:13-56`).
- Modify (format only): `crates/rodas5p-integrators/tests/production_audit_packet_curve.rs`.
- Modify: `crates/rodas5p-core/src/damped_moments.rs:492` (one attribute) and `crates/rodas5p-integrators/src/output.rs` (unused items, l. 706–760).
- Modify: `.github/workflows/production.yml` (fmt and clippy jobs).

**Interfaces:**
- Consumes (existing):
  - `integrate_rodas5p_fast_observed`;
  - `integrate_rodas5p_fast_dense_observed(problem, t_span, y0, adaptive, interior)`;
  - `integrate_rodas5p_fast_banded_observed(problem, band, t_span, y0, adaptive, output)` (`rodas5p_fast.rs:1645`);
  - `integrate_rodas5p_fast_banded_dense_observed(problem, band, t_span, y0, adaptive, interior)`;
  - `integrate_rodas5p_fast_dense_observed_with_atol(.., atol: &[f64])`;
  - the `brusselator(cells) -> (OdeProblem, BandedJacobian, Vec<f64>)` helper of `fast_dense_contracts.rs:109` (copied into the test file).
- Produces: tests only, plus the CI edits.

- [ ] **Step 1: Write the tests.**
  - (a) Replace the absolute budget with a **growth form**: for a pair of runs whose accepted counts m₁ < m₂ differ by about 4× (Robertson at spans 10 and 40, or at rtol 1e-6 and 1e-9; record which), assert (dense₂ − fast₂) − (dense₁ − fast₁) ≤ 3·(⌈log₂(m₂+1)⌉ − ⌈log₂(m₁+1)⌉) + 1. Keep the old absolute assertion as a printed diagnostic only. This makes the test independent of the setup constant (O-46).
  - (b) `banded_fast_dense_allocations_grow_only_with_store`: Brusselator n = 20 (`brusselator(10)`) through the banded dense entry against `integrate_rodas5p_fast_banded_observed`, with the growth form of (a).
  - (c) `vector_atol_fast_dense_allocations_grow_only_with_store`: Robertson with `atol = [1e-10; 3]` through `..._with_atol`, with the growth form.
  - (d) `facade_dense_jacobian_allocates_nothing_per_step` (`facade_allocations.rs`): Robertson with `Jacobian::dense`, `dense_output = false`, at two spans with about a 4× step ratio. Facade allocations grow by at most 2 between the spans.
    - Measure first and record both counts in the test doc.
    - If the growth is per step, mark the test `#[ignore = "measured per-step allocation: <k>"]` and report it as a finding. The task does not change facade code.
  - (e) `b3d_zero_fill_masks_stale_entries` (`facade_allocations.rs` or `p0_contracts.rs`; pick `facade_allocations.rs` to stay off T1/T5 files). A dense Jacobian callback writes entry (0,1) only on its first call. The run is bitwise the run whose callback writes an explicit `0.0` at (0,1) after the first call.
- [ ] **Step 2: Run** `cargo test -p rodas5p-integrators --locked -j 1 --test fast_dense_allocations` and `cargo test -p vigilode --locked -j 1 --test facade_allocations`. All pass, or (d) is ignored with its count recorded.
- [ ] **Step 3: Lint hygiene.**
  - `rustfmt --edition 2024 crates/rodas5p-integrators/tests/production_audit_packet_curve.rs`. Then `cargo fmt --check -p rodas5p-integrators` exits 0.
  - Add `#[allow(clippy::too_many_arguments)]` on `query` at `damped_moments.rs:492`, the repository's convention.
  - In `output.rs`, remove the unused `prepared_*` fields, `DenseOutputPreparationWork` and `*_radii` items only if `grep -rn` finds no use in the workspace. Otherwise add a scoped `#[allow(dead_code)]` with a one-line reason.
- [ ] **Step 4: CI.**
  - Add `-p rodas5p-integrators` to the fmt job (`production.yml:64`).
  - Run `cargo clippy -p vigilode --all-targets --locked -- -D warnings` **without** `--no-deps`:
    - if it exits 0, drop `--no-deps` and its comment (l. 78–82);
    - otherwise keep the flag and list the remaining lints by crate in the node report.
  - Validate the YAML with `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/production.yml'))"`.
- [ ] **Step 5: Run** `cargo test -p rodas5p-integrators --locked -j 1 --test rodas5p_fast_allocations --test fast_dense_contracts` and `cargo build -p vigilode --examples --locked -j 1`. Then write the node report, including the faer-LU note: dense n above `RODAS5P_FAST_SMALL_LU_MAX` allocates per solve by design and is not covered by these tests.

## Task 5: P0-stats — facade-owned record types (node id `p0-stats`, stage 2)

Plan §3 (the API rules), §9, G-A (`cargo semver-checks`). Closes O-26, O-09 and O-47. Branches from the integration tip after Task 1 merges (P1).

**Rule:** no type from `rodas5p_core` or `rodas5p_integrators` is reachable from a public item of `vigilode`. Mirrors keep:
- field names and order;
- serde attributes (`rename_all`, `tag`, `content`) of their source;
- the public field visibility, so `crates/vigilode-bench` builds unchanged (P9).

A mirror whose source contains further internal types mirrors them recursively.

| Leaked type (`solution.rs:5-10`) | Where | Mirror (**new**, `crates/vigilode/src/records.rs`) |
| --- | --- | --- |
| `AdaptiveStepConfig` (+ `ControllerKind`, `MaxStepPolicy`) | `Stats::adaptive` | `AdaptiveRecord` (+ `ControllerRecord`, `MaxStepPolicyRecord`) |
| `WorkCounters` (62 fields) | `Stats::{counters, driver_counters}`, `SequentialDense::estimate_counters` | `WorkRecord` |
| `LinearSolverConfig` | `Stats::matrix_free_linear`, `SequentialDense::linear_solver` | `LinearSolverRecord` |
| `RoutingRecord` | `RouteRecord::routing` | `RoutingDecisionRecord` |
| `BandedWork` | `DriverWork::{Banded, FastDense}` | `BandedWorkRecord` |
| `DenseErrorControl`, `InteriorSampling` | `DriverWork::SequentialDense` | `InteriorControlRecord`, `InteriorSamplingRecord` |
| `FastInteriorWork` | `DriverWork::FastDense::interior` | `FastInteriorRecord` |
| `Rodas5pFastLu` | `DriverWork::{Dense, Banded, FastDense}::lu` | `LuRecord` |
| `DriverStopReason`, `AdaptiveFailureKind` | `StopRecord` | `StopReason`, `StepFailureKind` |

Each mirror has `impl From<&Source> for Mirror` inside the facade. All mirrors are `#[non_exhaustive]`, which follows the wave-3 decision for output types.

**Files:** create `crates/vigilode/src/records.rs`; modify `crates/vigilode/src/{solution.rs, solve.rs, lib.rs, events.rs}`; create `crates/vigilode/tests/records_contracts.rs` and `crates/vigilode/tests/golden/robertson_dense_event_stats.json`; update `crates/vigilode/tests/{p0_contracts.rs, facade_contracts.rs}` where they name internal types.

**Interfaces:**
- Consumes: the post-Task-1 `DriverWork::SequentialDense { .., estimate_failures }` and `STATS_SCHEMA = "vigilode.stats.v4"`.
- Produces: the mirror types above, re-exported from `lib.rs`, and `FailureReason::EventLocationFailed` (**new**; see step 3).

- [ ] **Step 1: Write failing tests** (`records_contracts.rs`):
  - (a) `stats_json_unchanged` stays as is (`p0_api_contracts.rs:220`, against the post-T1 golden). In the RED commit, also write `golden/robertson_dense_event_stats.json` from the **pre-change tip**: Robertson with dense output, `WFilteredEnforce`, one non-terminal event, `serde_json::to_string_pretty(&sol.stats)`. A new test `dense_event_stats_json_unchanged` compares it byte for byte (O-47).
  - (b) Review Focus 5, `every_mirror_serializes_like_its_source`: for each pair in the table, build a source value with **every numeric field nonzero** and every enum at a non-default variant. Assert `serde_json::to_string(&Mirror::from(&src)) == serde_json::to_string(&src)`.
  - (c) `no_internal_type_in_public_api`: a doc comment in the test records the check command. The node report records its empty output: `grep -nE 'pub .*(rodas5p_core|rodas5p_integrators)::|pub use rodas5p_' crates/vigilode/src/*.rs`, plus a manual read of every `pub` field type in `solution.rs`.
- [ ] **Step 2: Run** `cargo test -p vigilode --locked -j 1 --test records_contracts`. Expect a compile failure.
- [ ] **Step 3: Implement** the mirrors and switch `Stats`, `RouteRecord`, `StopRecord` and `DriverWork` to them. In `events.rs` (O-9):
  - add `debug_assert!` on the impossible paths at `locate` (`extension.is_none()`) and on the `unwrap_or_else` state;
  - report an `at()` miss inside the Illinois loop as `FailureReason::EventLocationFailed`, with a doc line;
  - leave `EventFunctionNonFinite` for non-finite `g` only.
- [ ] **Step 4: Run** `cargo test -p vigilode --locked -j 1` (all targets), `cargo test -p vigilode --doc --locked`, `RUSTDOCFLAGS="-D warnings" cargo doc -p vigilode --no-deps --locked`, the 4 examples, the clippy job as Task 4 left it, and `cargo check --manifest-path crates/vigilode-bench/Cargo.toml --all-targets --locked`. All pass. Then write the node report, with the grep output of (c).

## Task 6: A5-epus — error-per-unit-step acceptance on the fast drivers (node id `a5-epus`, stage 2, conditional; ledger L-0145)

**Opens only if L-0143 (Task 2) is PASS.** On FAIL, this task is not dispatched, L-0145 stays unused, and the card text remains here as the preregistration of record. Plan §5 T4. This is a new card, with one mechanism.

**Mathematics (preregistered).**

*Lemma A5 (T4 with zero defect between steps).* Let ‖·‖ be a fixed norm, and suppose the flow φ of y' = f is non-expansive on a tube containing the numerical states and the true solution. That is the hypothesis of T4 with μ ≤ 0, and it gives ‖φ_{t,s}(x) − φ_{t,s}(w)‖ ≤ ‖x − w‖. Let ℓ_k = y_{k+1} − φ_{t_{k+1},t_k}(y_k). Then ‖y_N − y(t_N)‖ ≤ Σ_{k<N} ‖ℓ_k‖. Hence, if ‖ℓ_k‖ ≤ tol·h_k/(t_end − t_0) for every k, then ‖y_K − y(t_K)‖ ≤ tol for every K.

*Proof sketch.* Telescope y_N − y(t_N) = Σ_k [φ_{t_N,t_{k+1}}(y_{k+1}) − φ_{t_N,t_{k+1}}(φ_{t_{k+1},t_k}(y_k))] (Lady Windermere's fan). Each term is at most ‖ℓ_k‖ by non-expansiveness. T4 with r_n = ‖ℓ_k‖, δ = 0 and μ ≤ 0 is that statement. ∎

*Gap, stated, H-status.* The controller sees err_emb, the embedded (order-4) difference, not ‖ℓ_k‖ of the propagated order-5 solution. "err_emb ≥ ‖ℓ_k‖" holds only asymptotically (h → 0) and is a hypothesis. The weights also vary with y, while the lemma needs one norm. Task 2's `acc` class is the measured evidence that the local errors stay within tolerance on the target rows.

**Mechanism (one).**
- With span T = t_end − t_0 > 0, an attempt's embedded error is replaced by err_emb·T/h *before* the acceptance test and before the interior sink.
- Under `WFilteredEnforce`, the controller therefore sees max(err_emb·T/h, interior estimate). The interior estimate is not scaled.
- The step proposal uses the same number. The controller's exponent is not retuned.
- An empty span takes no attempt. h = T gives a factor of exactly 1.

**Kill gate (preregistered now):**
- *Gated rows:* the Task 2 rows of class `acc`.
- *Arm:* fast dense + `WFilteredEnforce` + `StepErrorTarget::PerUnitStep`, against the same with `PerStep`, in the Task 2 harness.
- *PASS iff all hold:*
  1. every gated row has max_K E_K ≤ 2.0 tolerance units;
  2. the median accepted-step ratio PerUnitStep/PerStep over gated rows is ≤ 2.0, and the maximum is ≤ 3.0;
  3. no gated row changes from success to failure.
- *Context:* the other corpus rows are reported, not gated.
- No facade exposure in wave 4.

**Files:**
- Modify: `crates/rodas5p-integrators/src/rodas5p_fast.rs`, the `StepSink` trait (l. 1487) and the application point at l. 1933–1949 (before the `error <= 1.0` arm).
- Modify: `crates/rodas5p-integrators/src/rodas5p_fast_dense.rs` (a new entry point; `run_dense` passes the target to the sink) and `lib.rs`.
- Modify: `crates/vigilode-bench/src/global.rs` (the arm switch).
- Test: `crates/rodas5p-integrators/tests/epus_contracts.rs` (new).
- Create: `research/a5_epus_YYYYMMDD/{PREREGISTRATION.md, results_epus.json}`, L-0145, `docs/plan/nodes/a5-epus.md`.

**Interfaces:**
- Consumes: Task 2's `run_global` and its class field; `integrate_fast` (`rodas5p_fast.rs:1855`); `run_dense` (`rodas5p_fast_dense.rs`, which passes `Rodas5pFastOptions::default()` at l. 444).
- Produces:
  - `pub enum StepErrorTarget { #[default] PerStep, PerUnitStep }` (**new**).
  - A provided method on the private trait, `fn embedded_error(&self, h: f64, error: f64) -> f64 { error }`. `NoSink` keeps it, so every existing entry point is unchanged bit for bit. `DenseSink` overrides it under `PerUnitStep`.
  - `pub fn integrate_rodas5p_fast_dense_observed_with_target(problem: &OdeProblem, t_span: (f64, f64), y0: &[f64], adaptive: &AdaptiveStepConfig, interior: FastInteriorControl, target: StepErrorTarget) -> CoreResult<Rodas5pFastDenseResult>` (**new**).
  - **No** new field on `Rodas5pFastOptions` (ruling P4).

- [ ] **Step 1: Write failing tests** (`epus_contracts.rs`):
  - (a) `per_step_target_is_bitwise_the_fast_dense_run`: Robertson and HIRES at rtol {1e-4, 1e-6, 1e-8}, `PerStep` against `integrate_rodas5p_fast_dense_observed`. Endpoint bits, counters and `FastInteriorWork` are equal.
  - (b) `per_unit_step_bounds_global_error_on_a_dissipative_scalar`: `scalar_linear_problem(-1.0, 1.0)` on `[0, 1]` (μ = −1, so the lemma's hypothesis holds exactly) at rtol {1e-4, 1e-6, 1e-8}. The error against e^{−1} is ≤ 1 tolerance unit, and accepted steps are ≥ those of `PerStep`. The test doc says that the assertion also rests on the H-status estimator hypothesis.
  - (c) Review Focus 4, `empty_and_single_step_spans`: `t1 == t0` gives 0 attempts and `y == y0`. A span equal to `max_step` with one accepted step gives an error factor of exactly 1, which is checked through the sink by comparing with `PerStep` on that attempt.
  - (d) `interior_estimate_not_scaled`: PR λ = −1e5, rtol 1e-6, 201 points. The maximum interior error is ≤ 2 tolerance units under `PerUnitStep`, and `FastInteriorWork::max_estimate` is ≤ 1.
- [ ] **Step 2: Run** `cargo test -p rodas5p-integrators --locked -j 1 --test epus_contracts`. Expect a compile failure.
- [ ] **Step 3: Implement** the enum, the trait method and the entry point. Add the arm to `global.rs` (CLI flag `--target per-unit-step`).
- [ ] **Step 4: Run** the new tests plus `fast_dense_contracts`, `fast_dense_allocations`, `rodas5p_fast_contracts`, `rodas5p_fast_allocations`, `b1_lagged_contracts` and `b1d_resolvent_contracts`. All pass.
- [ ] **Step 5: Preregister** (the lemma, the gap, the gate, the gated rows copied from `results_g1.json`, the command), then commit and push.
- [ ] **Step 6: Measure** in the cloud: `vigilode-bench global --target per-unit-step` on `corpus_g1.json`. Commit `results_epus.json` first.
- [ ] **Step 7: Ledger L-0145** with the verdict as measured, then the node report. On PASS, facade exposure becomes a wave-5 card with gates 3–4 (matched time on the workstation, holdout).

## Task 7: Integration (controller)

- [ ] **Merge stage 1** in this order:
  - [ ] `h-alloc`;
  - [ ] `p0-seqdense`;
  - [ ] `g1-global` (L-0143);
  - [ ] `b1d-resolvent`: rebase its ledger append onto the tip that holds L-0143 (L-0144).

  After each merge, run `check-research-node.py --base <wave-4 base>`.
- [ ] **Dispatch Task 5** on the tip after `p0-seqdense` merges, then merge it.
- [ ] **Dispatch Task 6** only if L-0143 is PASS; merge it (L-0145). Otherwise record "a5-epus not opened (L-0143 FAIL)" in `progress.md` and `EXECUTION_LOG.md`.
- [ ] **Cross-node seam tests** (`crates/vigilode/tests/p0_contracts.rs`):
  - (i) after Task 5, `SequentialDense::estimate_failures` is a field of the facade mirror path, and the Task 1 facade test (e) still passes;
  - (ii) after Task 4, the CI clippy job command (with or without `--no-deps`, as Task 4 left it) passes on the integration tip.
- [ ] **Workstation handoff** `docs/plan/handoff/LOCAL_TEST_HANDOFF_W4.md` (W2/W3 pattern), pinned to the integration SHA. Commands:
  - `cargo test -j 12 --workspace --locked --no-fail-fast`;
  - `vigilode-hybrid`;
  - `vigilode-bench` with `OPENBLAS_NUM_THREADS=1`;
  - `tools/test_check_research_node.py`;
  - `check-research-node.py --base <wave-4 base>`;
  - every `production.yml` job command with the pinned 1.94.1, including semver (informational);
  - **O-07:** `time cargo test --locked -p rodas5p-integrators` in the debug profile with `-j 2` (a 2-core runner proxy), reported against `timeout-minutes: 120`;
  - **O-36:** the two former flakes run 20× each under default parallel threads.

  VALIDATION_MATRIX:

  | Check | Expected |
  | --- | --- |
  | workspace failed | 0 |
  | ignored | wave-3 local count 111 + new `#[ignore]` measurement tests (b1d 1; a5 0) ± facade (d) if ignored |
  | leja / bound_collective 20× | 0 failures |
  | integrators debug `-j 2` wall | recorded; ruling on M6 by the controller (keep, split the job, or `--lib` + list) |
  | checker | exit 0 |

  Results return on `results/local-wave4-<sha>`.
- [ ] Push the integration branch, append the wave-4 rows to `docs/plan/EXECUTION_LOG.md`, run the whole-branch final review, and dispatch one fix wave.
