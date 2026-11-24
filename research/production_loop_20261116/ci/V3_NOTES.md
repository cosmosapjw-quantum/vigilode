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
  The fix round adds the calls that create, remove, rename or change paths (R32).
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
  whenever the diff is not empty (rule 5 for packages). A selected run unites the inputs it
  records with the carried-over entry instead of replacing it: one `cargo test -p` per package
  means a library may be built under another package's feature unification, and that must not
  drop a feature-gated input (no workspace package enables features of another today, so the sets
  agree; the union is a guard). A full run starts from an empty map, so a removed input stays at
  most until the next full run (over-selection only).
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
  whitespace and comments that are not doc comments changed, and no two punctuation characters
  split apart or joined: R29), rule 3 counts: the `read` set of every
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
  `github.event.pull_request.base.sha`; since the fix round the recorded `pr_base` is the first
  parent of the checked-out merge commit, R26); a difference from the baseline's values is a full
  run (an empty string equals a missing value). The pull-request checkout stays GitHub's merge
  commit; with the base fixed, the merge trees of two heads differ only where the heads differ, so
  the head diff is sound for the merge commit (the owner chose this over checking out the head,
  F8/F-04). Which earlier run is the baseline: R24, R25.

## Mechanics

- **R16 Concurrency.** The workflow-level group of production.yml is removed (it cancels the whole
  run, test job included). Every job has its own group; `test` and `ignored-tests` queue
  (`cancel-in-progress: false`), the short jobs cancel on non-main refs as before.
- **R17 Baseline lookup.** No `status=completed` filter: a run whose test job has uploaded its
  outcome is usable while other jobs of that run still run (upload-artifact v4 artifacts are
  available at once). Only runs with a smaller run id than the current one count. An outcome whose
  head is not an ancestor of the head is skipped and the search continues (v2 fell back to a full
  run). A failed run listing has its own note ("the run listing failed ..."); a failed download other
  than "no valid artifacts found" is listed in `errors` and counted in the note (F-09). Since the
  fix round only runs of the same event and repository qualify, and a run on the same runner image
  is preferred (R24, R25).
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
  strace install or a schedule fails the job. The fix round adds the checks of R27, R28, R33, R34
  and R35.
- **R21 Branch and expressions through env.** `BRANCH`, `HEAD_SHA`, `PR_BASE` and `FULL` are job
  environment variables; the scripts use `"$BRANCH"` etc. (F-12). `--branch` is URL-encoded in the
  REST query.
- **R22 Parallel exports.** Coverage exports and trace parsing run after the tests in a thread pool
  (`--jobs`, default: the CPUs available to the process); locally `--jobs 2` (memory).
- **R23 Workflow file names.** Artifact names (`ci-test-outcome-test`,
  `ci-test-outcome-ignored-tests`), the full-mode inputs, the time limits (300, 330 min) and the
  coverage instrumentation are unchanged. No schedule (OD-L1-21 point 4). The fix round adds the
  job `test-cli-fair-ab` with the artifact `ci-test-outcome-test-cli-fair-ab` (R27) and removes the
  path filters of the two test workflows (R28).

## Fix round (two adversarial reviews of b0d4bc97)

Reviewers: soundness of the selection (findings S-N1 to S-N6, earlier findings S-F1 to S-F14) and
CI mechanics (M-N1 to M-N8, earlier M-F-01 to M-F-12). The task text carried the mechanics review
only in part (it was cut at 14000 characters); its full JSON was read from the workflow journal of
this run. Every finding below was reproduced before it was fixed; the logs are the `fix_*` files
in `v3_logs/`.

- **R24 Baseline on the same runner image (M-N1, major).** Confirmed: the set-up logs of run
  45358324432 show job `fmt` on image version 20261116.88.1 and job `clippy` on 20261116.202.1,
  both started at 08:41:48Z (read with `gh api .../jobs/<id>/logs`); the reviewer counted 7 of 39
  ubuntu-latest jobs on .88.1. With an exact comparison, about 3 runs in 10 would be full without a
  code change. The owner's rule (a change of the runner image since the baseline means a full run)
  stays literal, with both `ImageOS` and `ImageVersion`; what changes is which run is the baseline:
  the newest qualifying run (complete, at an ancestor of the head) on the same image. Runs on other
  images are passed over; when none on this image qualifies (the first run on a new image), the
  newest qualifying run on any image is the baseline and rule 1 runs the whole suite, as before.
  Sound because any complete earlier run at an ancestor is a valid baseline: the diff from it holds
  every change since, and its outcome and map describe its own tree; the runs of one image form a
  chain of their own, and a failure seen only on the other image is selected again by the next run
  on that image (rule 6 of its chain). Not adopted: comparing `ImageOS` only (the reviewer's first
  option), because a new image version can change system libraries (libm) that bit-exact anchors
  depend on (F14). Cost: one artifact download per passed-over run, at most the 100 listed runs,
  and the long scan only on the first run on a new image, which is full anyway.
- **R25 Same event, same repository (M-N3, M-N5).** The run listing asks for `event=<the current
  event>` and keeps only runs with that event and with `head_repository.full_name` equal to
  `GITHUB_REPOSITORY`. A dispatch run (branch tree, no base) is no longer the baseline of a
  pull-request run (merge tree), which forced a full run through the base trigger, and the reverse;
  a fork's run with the same branch name never steers the selection. Self-test: a fake listing with
  a dispatch run and a fork run, both complete on the current image, neither chosen.
- **R26 Tested pull-request base (S-F8 and M-F-04 residual).** The recorded `pr_base` is the first
  parent of the checked-out commit when its second parent is the pull-request head (GitHub's merge
  commit), else the payload's `base.sha`; the payload value is kept as `pr_base_payload` and a
  difference gets a note. This removes the assumption that the payload base equals the base of the
  tested merge commit.
- **R27 rodas5p-cli and rodas5p-fair-ab get a test job (M-N2, major).** Confirmed: the removed A1
  step (`cargo test --workspace --all-targets --locked`) was the only non-ignored test run of
  rodas5p-cli (2 binaries, 14 test targets) and rodas5p-fair-ab (lib, 20 test targets, doc tests);
  besides it, A1 runs 2 cli targets and `tools/check-audit2-readiness.sh` 2 fair-ab targets. In the
  hosted A1 run 45358324393 those tests took 08:55-09:06 (cli, `triage_cli` 438 s) and 09:07-09:10
  (fair-ab), debug without coverage. OD-L1-21(3) removed the step on the premise (v1 review F-05)
  that the production and ignored-tests jobs cover the workspace; this makes the premise true. The
  new job `test-cli-fair-ab` in production.yml selects from these two packages with the same
  selector, measurement and rules, its own outcome artifact `ci-test-outcome-test-cli-fair-ab`, a
  queued concurrency group and a 120-minute limit. Not adopted: adding the two packages to the
  scope of the `test` job (the first version of this fix, commit 1288945b), because the full run
  of that job already runs longer than 157 minutes on hosted runners without strace (R36) and its
  limit of 300 minutes is fixed; a job of its own also keeps the production gate to the production
  crates. Cost: a second coverage build of the workspace on another runner when the job selects
  anything; with an empty selection it ends after the selection step. `check_workflows` asserts
  that the selecting jobs of each workflow together cover every workspace member and that each
  job looks up the artifact it uploads, under a name no other job uses. For the owner: these two
  research crates now have a change-based CI test job; the alternative is to accept that their
  tests run only in the full suite before a release.
- **R28 No path filter on the test workflows (S-N2, M-N4, S-F14 in part).** Confirmed: the filters
  (`production.yml` and `ignored-tests.yml`, `crates/**`, manifests, toolchain, `tools/ci/**`, the
  workflow) left out measured inputs: `docs/CONSTANTS_PROVENANCE.toml` (compile set of
  `constants_provenance_contracts`), research files (read sets), `fixtures/rodas5p_coefficients_snapshot.json`
  (core library input), `tools/reference_v2/generate_references_v2.py` (fair-ab library input). Both
  workflows now trigger on every pull request and every push to main. Not adopted: a longer list
  of directories, because a measured input can be any tracked file and the map changes at every
  run. Cost: a pull request that touches no measured input runs these jobs up to the selection step
  (toolchain, strace, self-test, baseline, select: a few minutes), and the other jobs of
  production.yml run too. The path filter of a pull request compared the whole pull-request diff,
  so a pull request that had touched `crates/**` already ran at every later push; the new runs are
  data-only pull requests and pushes to main (always full; the last push to main is from
  2026-08-26). `check_workflows` rejects a path filter on these triggers. The periodic full run
  stays out (OD-L1-21 point 4).
- **R29 Operator spacing (S-N1).** Confirmed with the reviewer's script: `success && collector` ->
  `success & &collector` in `integrate.rs` was noop and selected only the three text readers. The
  tokenizer now puts a `('j', '')` marker between two punctuation tokens with nothing between them,
  so whitespace or a comment that splits or joins `&&`, `=>`, `..`, `->`, `+=` changes the token
  stream: such an edit is body or structural, never noop (`fix_tokenizer_probe.log`: the
  integrate.rs edit is body and selects cli `alg02_predictive_controller`, which executes the file).
  Over-selection only: `Vec<Vec<u8>>` and `Vec<Vec<u8> >` are the same program and are no longer
  noop.
- **R30 Directories an added path creates (S-N3).** For an added path, every directory it creates
  (missing on the head trees) is also checked against `read` and `dirs`, as for deletions. The
  directory rule still decides existence on the head trees; a directory missing there but present
  in the tested merge tree (through the base) is among the created directories, so a test that
  listed or observed it is selected. Self-test and selection case S-N3.
- **R31 `.gitattributes` is global (S-N4).** A `.gitattributes` at any depth is a global file (rule
  1): it changes the checked-out bytes (eol, filters, `ident`, `working-tree-encoding`) of files
  whose blobs did not change. Confirmed with the reviewer's scratch repository
  (`review-soundness/gitattr_probe_1`). Selection case S-N4: full.
- **R32 Calls that change paths are traced (S-N5).** The runner also traces `mkdir, mkdirat,
  rename, renameat, renameat2, link, linkat, symlink, symlinkat, unlink, unlinkat, rmdir, mknod,
  mknodat, utimensat, utime, utimes, futimesat, chmod, fchmodat, fchmodat2, truncate, statfs,
  getxattr, lgetxattr, listxattr, llistxattr` (`?` where an architecture may lack the call). Every
  path argument counts: both paths of rename and link, the new path of symlink (its target is the
  link's content). Success counts as `read`, ENOENT or ENOTDIR as `absent`, another error as `read`;
  a failed `chdir` is recorded as `absent` and leaves the cwd. Arguments are split at top-level
  commas with quoted strings and `<...>` annotations kept whole. Probe with strace 6.8:
  `fix_strace_calls_probe.log` (non-destructive: every remove, rename, link, chmod and truncate
  targets a missing path). Cost: these calls stop the tracee as well; the traced-call counts of
  the measured run are in `fix_e2e_outcome.json`.
- **R33 The source facts of R8 and R10 are checked (S-N6).** `check_sources` (part of the
  self-test, so of both CI test jobs) scans the package sources (`src/**/*.rs` of every member)
  and fails on `line!()`, `column!()`, `Location::caller`, `track_caller` (a comment-only change
  shifts the positions they report), `include_str!`/`include_bytes!` of a `.rs` path (a crate that
  reads its own source as text), and file access (`fs::`, `File::`, `include_*!`, `read_to_string`,
  `read_dir`, `current_dir`) inside a code block of a `///` or `//!` doc comment (doc tests are not
  measured). Today it finds nothing in 264 files. When a later change trips it, the test jobs fail
  at the self-test, and the remedy is the code or the rule (classify the noop changes of such a
  file as body). Limits: patterns on single lines; block doc comments are not scanned.
- **R34 strace is optional at installation (M-N6).** Both install steps have
  `continue-on-error: true`; without strace the selector runs every selected test, says "strace is
  not installed" in the outcome and leaves those targets unmapped (R18), so the next run measures
  them again. Checked by `check_workflows`.
- **R35 The A1 check is conditional (M-N8).** `check_workflows` checks the A1 workflow only while
  the file exists, so renaming or removing that workflow cannot fail the test jobs.
- **R36 Hosted duration (M-N7): recorded, limits unchanged.** The live v2 full runs (coverage, one
  command per target, no strace), test job of run 45378440406 and ignored-tests job of run
  45378440571, started 10:49:59Z and 10:49:42Z; at 13:26:48Z both were still in their run step
  (157 minutes), longer than every completed v1 full run (test 92-109, ignored 94-97 minutes). The
  limits stay 300 and 330 minutes (design). If the first v3 full run uses more than about two
  thirds of its limit, the next step is to split the full mode across jobs (an owner decision);
  a job that times out leaves no outcome, so the next run is full again.
