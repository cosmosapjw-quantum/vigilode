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

