# Wave-3 implementer contract (common to every node)

- You are one implementer of one node. Never dispatch subagents of any kind, including reviewers or helpers. The controller reviews your work after you report.
- Work only in your node worktree, on your node branch. Commit with the repository's configured git identity and do not change git config. End each commit message with:

  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01HyDTYiTaAHRs6wHVKDkVKU

- Push your branch after every green commit (`git push -u origin <branch>`). Pushing is the backup. Never force-push, never rewrite history, never touch another branch. Do not merge other branches unless your dispatch tells you to.
- Base: production/wave3-integration 75a0c6c (wave-2 integration 08fd052 plus plan docs).
- Requirements: your task brief is the source. The controller's rulings in `preflight-scan.md` (same directory as this file) override the brief where they name your task. Read §(b) and §(c) for rows that mention your task number or node id.
- Use TDD: write the failing test first, see it fail for the right reason, then implement.
- Build environment (2-core machine, limited disk, other nodes building in parallel):

  RUSTUP_TOOLCHAIN=stable CARGO_TARGET_DIR=/home/claude/targets/<node-id> CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo ... -j 1 --locked

  Use -j 1. Three nodes share two cores.
- Run only the targeted tests for the crates you touch. Full workspace suites and benchmark corpora run on the owner's workstation by handoff. If your brief asks for a full-suite or heavy run, write the exact commands into your report under "Handoff commands" instead of running them.
- Disk: if `df -h /home/claude` shows under 4 GB free, stop building and report BLOCKED.
- Never use bare `git stash`. The stash stack is shared across worktrees.
- `cargo fmt --all -- --check` already fails at base in `crates/rodas5p-integrators/tests/production_audit_packet_curve.rs`. Ignore that file. Format only what you touch (`rustfmt --edition 2024 <files>`).
- Ledger: `research/LEDGER.jsonl` is append-only (docs/RESEARCH_LEDGER.md). Run `python3 tools/check-research-node.py --base a5a48f77e7ac7a80b8ff86cd897110790066f7ee`; it must print PASS when you finish.
- Report: write the full report to your report path. Include:
  - commits
  - each requirement mapped to file:line
  - test evidence (RED then GREEN)
  - deviations, each with its reason
  - the handoff commands
  - open questions

  Return only: status (DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED), commit shas, a one-line test summary, and concerns.
