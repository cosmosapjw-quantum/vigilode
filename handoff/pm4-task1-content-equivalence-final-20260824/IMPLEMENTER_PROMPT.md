# Codex implementer prompt — PM-4 Task-1 final local run and push

You are the implementation agent. Work autonomously in `~/vigilode`. Your goal
is to finish the narrow Task-1 publication and return the project to scientific
work. Do not ask the user to run commands for you.

Your primary deliverable is not a narrative plan. Compile and execute the
repository-local contract so every foreseeable P0/P1 fails closed, while
non-scientific transport/packaging hash differences do not stall the work.

## Read first

Locate the fetched handoff worktree or fetch this branch yourself:

```text
handoff/pm4-task1-content-equivalence-final-20260824
```

Read `AGENTS.md` and its ordered files. Treat
`AUDIT_COMPILED_EXEC_PLAN.yaml` as the execution contract.

## Identity policy — mandatory

Never use an outer tar/zip/wheel SHA mismatch alone as a blocker. First classify
any mismatch as Git source, immutable input, deterministic generated evidence,
numerical output, packaging transport, or working-tree materialization.

For tracked source, Git commit/tree/blob identity and the actual diff are
primary. For numerical outputs, use invariants, residuals, units, limits, and
tolerances. Packaging metadata is soft unless byte-identical packaging is the
explicit deliverable. Record packaging differences, but continue when content
and code authority are established.

The Task-1 payload is directly tracked at:

```text
payload/PM4_TASK1_SCHEMA_BOUNDARY.patch
```

Do not reconstruct or authenticate an R4 archive. Do not compare any outer R4
archive SHA. The Git-tracked patch blob is the payload.

## Exact expected starting authority

```text
canonical main
140f6b5c078c3d8fcd5b6c52310c063ee233dc12

feature branch
research/v38d-exploratory-benchmark-substrate

expected feature head
b2d5ec41cb147e01aadbc9c42928da8abfa75c58

PR #11
OPEN / DRAFT / UNMERGED / ZERO FILE DIFF
```

Resolve current remote state yourself. If the expected feature has already been
fast-forwarded with the exact Task-1 content, verify it and report
`ALREADY_PUBLISHED_CONTENT_EQUIVALENT` rather than applying or pushing again.
If refs diverged in any other way, stop with `BLOCKED_BY_UNRESOLVED_SPEC` and
show ancestry/diff evidence.

## Environment flexibility

You may adapt non-scientific environment details when mechanically verified:

- create a fresh worktree instead of using a dirty checkout;
- locate Rust 1.94.1 from PATH, rustup, or an existing environment script;
- inspect `.cargo/config.toml` and locate the persistent offline vendor;
- create a temporary sibling symlink or use an untracked Cargo invocation
  override, provided tracked config/lockfiles do not change;
- ignore exact vendor directory counts and let structural checks plus
  `cargo metadata --frozen --format-version 1` prove closure;
- diagnose EOL/filter/LFS differences with Git attributes rather than assuming
  transport corruption.

Do not guess across scientific semantics, Git ancestry, allowed diff surface,
or dependency versions.

## Execute

1. Run the handoff preflight.
2. Capture exact main, feature, PR, patch blob OID, toolchain, attributes, and
   Cargo configuration.
3. Create a clean isolated worktree at the exact feature head.
4. Apply the tracked patch without editing it.
5. Delete only:
   `research/generic_v38d_high_entropy_performance_tournament/RECOVERY_START.md`.
6. Verify the exact M/A/A/D four-file diff before tests.
7. Establish offline Cargo closure with no tracked Cargo/config/lock change.
8. Run, retaining complete logs:

```bash
cargo metadata --frozen --format-version 1
cargo test -p rodas5p-integrators --test v38d_performance_probe_contracts --offline --locked
cargo test -p rodas5p-integrators --all-targets --no-run --offline --locked
TERM=dumb cargo clippy -p rodas5p-integrators --all-targets --offline --locked -- -D warnings
cargo fmt --all -- --check
git diff --check
git status --porcelain
```

9. Require exactly five focused tests and no fabricated benchmark/wall output.
10. Commit exactly one implementation commit with subject:

```text
feat: define v3.8-D exploratory probe schema
```

11. Immediately re-read remote main and feature refs. Push only if they still
    match the captured preconditions. Use ordinary non-force fast-forward
    semantics.
12. Verify remote ancestry, exact four-file diff/bytes, unchanged main, and PR
    #11 open/draft/unmerged.
13. Produce a local evidence directory containing command logs and a
    `COMPLETION_EVIDENCE.json`. Do not add evidence files to the PR diff.
14. Stop. Do not merge and do not begin Task 2.

## Failure policy

Emit `BLOCKED_BY_UNRESOLVED_SPEC` only for a real unresolved scientific,
dependency, Git-history, or source-content boundary. A packaging SHA mismatch,
absolute-path difference, or harmless numerical serialization difference alone
is not eligible.

On a blocker, preserve exact command, exit code, stdout/stderr, worktree, ref
state, classification, and why no safe mechanically verified substitute exists.

## Final response

Report:

- previous and final main/feature/tree SHAs;
- patch Git blob OID;
- exact name-status surface;
- identity-mismatch classifications encountered;
- Cargo closure strategy and metadata result;
- every required command and result;
- commit and normal push evidence;
- PR #11 final state;
- P0/P1/P2/P3 ledger;
- unresolved blockers;
- explicit confirmation that merge, wall timing, ranking, and Task 2 were not performed.
