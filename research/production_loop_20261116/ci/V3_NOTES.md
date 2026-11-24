# CI test selection v3: rulings (task ci-v3, owner decisions OD-L1-19, OD-L1-20, OD-L1-21)

Implementation: `tools/ci/select_tests.py` (v3), `.github/workflows/production.yml`,
`.github/workflows/ignored-tests.yml`, `.github/workflows/a1-inner-tolerance-parity.yml`,
`tools/check_ignored_tests_in_ci.py`. Verification logs: `v3_logs/`. Korean summary: `TRIAGE_KO.md`
section 11.

Each ruling below settles a point that the approved design (OD-L1-21 and the task text) leaves open,
or records where the implementation reads the design in a particular way. The reason follows each.

## Map, outcome and schemas

- **R1 Schemas.** `vigilode.ci.coverage-map.v1` becomes `.v2` (name kept, version bumped),
  `vigilode.ci.test-outcome.v1` becomes `.v2`, `vigilode.ci.test-selection.v2` becomes `.v3`. A map
  entry is `{compile, executed, read, dirs, absent, head}`; the map also holds `packages: {name:
  {compile, head}}` (R4). The baseline lookup accepts only v2 outcomes, so the first v3 run of each
  job is full (it is full anyway: `tools/ci/` is a global path).
- **R2 Map keys.** `<pkg>::lib`, `<pkg>::bin:<name>`, `<pkg>::<test>`, `<pkg>::example:<name>`
  (compile set only) and `<pkg>::<target>#<test>` for a negative control (R11). Entries of targets
  that are no longer current (deleted, or required features not enabled) or outside the job's scope
  are pruned at every run.
- **R3 A fifth set, `absent`, and more traced calls.** The design names `read` and `dirs` from
  `open/openat/openat2` with `-e status=successful`. The implementation also traces `creat, execve,
  execveat, stat, lstat, newfstatat, statx, access, faccessat, faccessat2, readlink, readlinkat` and
  keeps failed calls: a failed probe with `ENOENT` or `ENOTDIR` is recorded in `absent`, any other
  observation in `read`. Reason: `Path::exists()`, `is_dir()`, `metadata()` and a tolerated failed
  open decide behaviour without a successful `open`; when such a file is added, only `absent`
  connects the change to the test (rule 3, added path). `chdir/fchdir` are traced to keep the cwd of
  calls without a directory descriptor. `-y` prints the cwd of `AT_FDCWD` and the path of every
  descriptor, so relative and descriptor-relative paths resolve. Command (one value of
  `CARGO_TARGET_<HOST>_RUNNER`, set per target run):
  `strace -ff --seccomp-bpf -qq -y -o <work>/<target>/trace/t -e trace=<the list above>`, where the
  legacy and newer names carry strace's `?` qualifier (an unknown name is not an error: the legacy
  calls do not exist on aarch64). `-e status` keeps its default (successful and failed calls).
  A successful open is recorded under both its lexical path and the resolved path from `-y`
  (a symlink and its target both count).
- **R4 Package inputs.** The dep-info of a test artifact lists only that crate's files; a test that
  links a library does not list the library's sources or the files the library `include_str!`s
  (`fixtures/rodas5p_coefficients_snapshot.json` for core,
  `tools/reference_v2/generate_references_v2.py` for fair-ab). The map therefore records, per
  package, the dep-info of the non-test library and binaries as other crates link them. They are read
  from every cargo invocation of the run: the build step, each target run and each doc-test run
  (all with `--message-format=json-diagnostic-rendered-ansi`; a doc-test run builds the non-test
  library). These files are package sources of that package (rule 4): a non-Rust file among them is a
  structural change (its bytes are a constant of the crate), a `.rs` file is classified. A package
  in scope, or a dependency of one, whose inputs are not recorded counts as structurally changed
  whenever the diff is not empty (rule 5 for packages).
- **R5 Build scripts.** `build.rs` stays structural. The `rerun-if-changed` inputs of build scripts
  are not recorded: the integrators build script depends on itself only; the fair-ab build script
  reruns on every tracked file but embeds only the HEAD revision and the dirty flag, which its tests
  use for run-time self-consistency (checked by the v1 reviewer). Generated files under `OUT_DIR` lie
  in the target directory and are dropped.
- **R6 Dropped paths.** Outside the repository, under `.git`, under the cargo target directory and
  under the measurement directory (`--work-dir`, `$RUNNER_TEMP` in CI). Dep-info entries are joined
  to the repository root (rustc runs there and writes member paths relative to it), never to the
  Python cwd (F13).

## Selection rules

- **R7 Directory rule.** For an added or deleted path, the listing that changes is the one of the
  nearest ancestor that exists on both sides (`git cat-file --batch-check <commit>:<dir>`); every
  directory that the change creates or removes is checked against `absent` (added) and `read`/`dirs`
  (deleted). This is a superset of the stated rule (parent directory in `dirs`): a file added in a
  new directory changes the listing of the first existing ancestor, not of its (new) parent.
- **R8 Noop rule, exactly.** For a modified package source F whose tokens are unchanged (only
  whitespace and comments that are not doc comments changed), rule 3 counts: the `read` set of every
  target; the `compile` set of every target except the unit-test targets (`lib`, `bin:*`) of the
  packages that own F; no `executed` set; no package rule. Why this is sound for text consumers:
  rustc records every file opened by `include_str!`/`include_bytes!` in the dep-info of the crate
  that expands it, and strace records every file a test opens at run time, so every consumer that
  sees the bytes of F is in a `compile` set of another crate or test crate or in a `read` set, and
  runs. Consumers that compile F as Rust code (module, `#[path]`, `include!`) see the same token
  stream; those in other crates or test crates are selected anyway (as the design asks), the owning
  crate's own unit tests are not. The owning crate would consume F as text only if it
  `include_str!`ed its own source file: no member does (no `include_str!`/`include_bytes!` of a
  `.rs` path under any member's `src/`). Residual: spans (`line!()`, panic locations,
  `Location::caller()`), see the gaps in TRIAGE section 11.
- **R9 Body rule, literal.** A body change applies rule 3 with all three sets, so the owning
  package's lib (and bin) unit tests run for every body change in its sources: their dep-info lists
  every source of the crate. The alternative, exempting them as in R8 and relying on their coverage,
  would skip the `cfg(test)` compilation of the changed body; it is not adopted because rule 3 as
  approved counts compile sets. Cost: one more target per body change; the integrators lib unit
  tests took 14.9 s locally (141 tests, debug, coverage and strace, host load about 190;
  `v3_logs/e2e_run.log`). The doc tests of the package and its dependents run as in v2.
- **R10 Doc tests.** They run whenever their package runs whole or has any selected target (the
  design: "they always run for a selected package"), and for body changes of the package or a
  dependency (v2). They are not traced: cargo 1.94.1 passes the target runner to doc tests
  (checked on a probe crate), so the runner variable is removed from their environment. A package
  without a library has no doc tests and never gets a `--doc` command.
- **R11 Negative controls: measured, and merged into the outcome.** Each control step runs
  `select_tests.py control`, which runs the one test (`--ignored --exact`) with the same measurement
  as a target and writes a map fragment for `<pkg>::<target>#<test>`; the run step merges the
  fragments. Reason: the selector's own run of these two targets skips the control tests, and each
  target has no other ignored test, so the target's entry measured nothing the control executes or
  reads; without this, a body change in code that only the control executes would never select it.
  The select step declares the controls (`--control`), and a control without an entry selects its
  target (rule 5). For "failures must not let the baseline advance" the chosen option is to merge:
  a failed control (test passed, wrong signature, did not build, or selector error) appends
  `<pkg>:<target>` to `failed-controls.txt`, the run step writes it to `outcome.failed` (flagged
  `negative_control`), and rule 6 selects the target again at the next run. The other option (upload
  only when no control failed) would discard the measurement of the whole run. The steps have ids
  (`kill-gate`, `stop-diff`).
- **R12 Literal matcher: removed** (task rule C8). Every reviewer link that the literal matcher
  found (F1-F4, F6) is in a measured set, and F5 and F7 were never literal. The note for a changed
  path that selects nothing comes from the map: `no mapped target compiles, executes or reads
  <path>`. Doc tests are the only unmeasured consumers, and literals in doc comments would not be a
  sound guard either; they run with their package (R10).
- **R13 Unmapped targets (rule 5).** Every current target in scope without an entry (new, measurement
  lost, new example) runs or is compiled whenever the diff is not empty; with an empty diff only
  baseline failures run.
- **R14 Required features.** A target whose `required-features` are not all in the closure of the
  package's `default` feature is never named: not in whole-package units, not by rule 5, and a
  baseline failure of such a target (or of a deleted target) is dropped with a note (F10, F-02).
- **R15 Image and base (rule 1).** The selection and the outcome record `environment: {image_os,
  image_version, pr_base}` (`ImageOS`, `ImageVersion` of the runner, `--pr-base` from
  `github.event.pull_request.base.sha`); a difference from the baseline's values is a full run (an
  empty string equals a missing value). The pull-request checkout stays GitHub's merge commit; with
  the base fixed, the merge trees of two heads differ only where the heads differ, so the head diff
  is sound for the merge commit (the owner chose this over checking out the head, F8/F-04).

## Mechanics

- **R16 Concurrency.** The workflow-level group of production.yml is removed (it cancels the whole
  run, test job included). Every job has its own group; `test` and `ignored-tests` queue
  (`cancel-in-progress: false`), the short jobs cancel on non-main refs as before.
- **R17 Baseline lookup.** No `status=completed` filter: a run whose test job has uploaded its
  outcome is usable while other jobs of that run still run (upload-artifact v4 artifacts are
  available at once). Only runs with a smaller run id than the current one count. An outcome whose
  head is not an ancestor of the head is skipped and the search continues (v2 fell back to a full
  run). A failed run listing has its own note ("the run listing failed ..."); a failed download other
  than "no valid artifacts found" is listed in `errors` and counted in the note (F-09).
- **R18 Measurement failure.** The run step probes strace first (`strace ... true`); when it cannot
  trace (missing, ptrace refused), the tests run without the runner and every target loses its entry
  (rule 5 runs them next time), and the outcome says why. A target without dep-info, profile or
  trace output, or whose command did not run its tests, loses its entry with the reason in
  `outcome.measurement.lost`. The outcome also records the strace version and, per target, the
  number of trace files and traced calls.
- **R19 Examples.** `cargo test --no-run --example <name>` builds a named example with the test
  harness (checked on a probe crate); a whole-package build compiles examples normally. Both are
  compile checks and give dep-info. Examples are never run.
- **R20 Self-test in CI.** Both test jobs run `select_tests.py self-test` before the baseline step
  (git only, about a second). It includes text checks of the workflow mechanics that the review asked
  for (`check_workflows`), so a later workflow edit that reintroduces a cancelling group, a selector
  step without `-u`, an upload without `overwrite`, a head_ref expansion in a script, a missing
  strace install or a schedule fails the job.
- **R21 Branch and expressions through env.** `BRANCH`, `HEAD_SHA`, `PR_BASE` and `FULL` are job
  environment variables; the scripts use `"$BRANCH"` etc. (F-12). `--branch` is URL-encoded in the
  REST query.
- **R22 Parallel exports.** Coverage exports and trace parsing run after the tests in a thread pool
  (`--jobs`, default: the CPUs available to the process); locally `--jobs 2` (memory).
- **R23 Workflow file names.** Artifact names (`ci-test-outcome-test`,
  `ci-test-outcome-ignored-tests`), the full-mode inputs, the time limits (300, 330 min) and the
  coverage instrumentation are unchanged. No schedule (OD-L1-21 point 4).
