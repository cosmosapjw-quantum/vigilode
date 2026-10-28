## Global Constraints

- Toolchain: the repository pins Rust 1.94.1 in `rust-toolchain.toml`, which cannot be downloaded in this
  environment. Every cargo command runs with `RUSTUP_TOOLCHAIN=stable` (cargo 1.97.0). Do not edit
  `rust-toolchain.toml`. Record the toolchain in the node report.
- Every cargo invocation uses `--locked`. No new external crates. Path dependencies between workspace crates are allowed.
- Each node works only in its own worktree `/home/claude/wt/<node-id>` on branch `production/<node-id>`
  created from the tip of `production/vigilode-plan-20261021`, with
  `CARGO_TARGET_DIR=/home/claude/targets/<node-id>`. The machine has 2 cores and 7 GB RAM: run
  `cargo` with `-j 1`, and restrict test runs to the crates and test targets the node touches
  (`-p <crate> --test <name>` or `--lib`); the full workspace suite runs once, in Task 6.
- Default behaviour of every existing public entry point stays bit-for-bit unchanged unless the task says
  otherwise. New behaviour is opt-in through a new option, enum variant or function.
- Statuses are type-distinct (§4 idea 3): an estimate is never labelled a certificate; a wall time on this
  shared 2-core host is diagnostic only and is reported as such.
- Failed attempts and their work stay counted (attempted vs completed).
- Commit messages end with
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01HyDTYiTaAHRs6wHVKDkVKU`.
  Use `git -c user.name="Jiwon Park" -c user.email="cosmosapjw@gmail.com" commit`.
- Push after every commit group: `git push -u origin production/<node-id>`. Never push to any other branch,
  never force-push, never merge into another branch.
- Each node writes `docs/plan/nodes/<node-id>.md`: what was done, commands run with exit codes and test
  counts, measured numbers with their status, deviations from the brief.
- `cargo fmt` the touched files; `cargo clippy -p <crate> --all-targets --locked` must add no new warnings in touched files.


## Build environment (controller ruling, supersedes the per-node CARGO_TARGET_DIR line above)
Disk allows ~27 GB and a full debug workspace build exceeds it. Every cargo command uses exactly:

    export RUSTUP_TOOLCHAIN=stable CARGO_TARGET_DIR=/home/claude/targets/shared \
           CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0

The target dir is shared by all five nodes (cargo serializes on its lock; "Blocking waiting for file lock" is
normal — wait). Use `-j 1`. Build and test only the crates/test targets you touch. Never run
`cargo test --workspace` or `cargo build --workspace`. Never `cargo clean`. Check `df -h /home/claude`
before any build that adds many test binaries; stop and report if free space < 4 GB.
