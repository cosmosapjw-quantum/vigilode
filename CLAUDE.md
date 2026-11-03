# VigilODE — instructions for coding agents

Rust workspace of ODE integrators (`crates/rodas5p-*`) and the facade crate `crates/vigilode`.

## Current work: switcher v2 remediation
- Tasks: `docs/agent/switcher_v2_remediation/TASKS.json`. Design: `DESIGN.md` in the same folder.
- Start every session with `python3 tools/agent_gate.py status` and `python3 tools/agent_gate.py next`,
  then read only the DESIGN.md section that `next` names. One task per session.
- Hooks in `.claude/settings.json` run `tools/agent_gate.py`. A `BLOCKED` message is a rule:
  do not work around it; run `python3 tools/agent_gate.py abort <ID> "<reason>"` if you cannot proceed.

## Commands
- One test file: `cargo test --locked -p vigilode --release --test <file_stem>`
- One test: append `-- <test_name> --exact`
- vigilode suite (10–20 min): `cargo test --locked -p vigilode --release --no-fail-fast`
- Lint: `cargo clippy --locked -p vigilode --all-targets --release --no-deps -- -D warnings`
- Format check: `cargo fmt -p vigilode -- --check`
- The toolchain is pinned (1.94.1). If it cannot be installed, prefix cargo and `agent_gate.py finish`
  with `RUSTUP_TOOLCHAIN=stable` and record that in PROGRESS.md.

## Facts the code does not tell you
- `research/LEDGER.jsonl` is append-only and carries authority; rows only via the script in DESIGN.md 13.2.
- WorkModel V2 costs are instruction units (Ir); guard ceilings are callback units.
- Without `--no-fail-fast`, cargo stops at the first failing test binary and hides later failures.
- `.agent/STATE.json` is the gate's state: edit only your task's `self_audit` block, never `status`.
