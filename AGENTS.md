# VigilODE Agent Map

Read this file before changing the repository. It is a map, not a complete process manual.

## Current Codex execution package

- Canonical machine contract: `docs/exec-plans/k0-stage-telemetry-integration-20260827/plan.json`
- Current handoff prompt: `docs/exec-plans/k0-stage-telemetry-integration-20260827/CODEX_HANDOFF_PROMPT.md`
- Launcher/orchestrator preparation: `docs/exec-plans/k0-stage-telemetry-integration-20260827/CODEX_LAUNCHER.md`
- K0 invariants: `docs/invariants/K0_STAGE_TELEMETRY.md`
- P0/P1 policy: `docs/quality/P0_P1_POLICY.md`
- Package validator: `python tools/verify-k0-stage-telemetry-plan.py --repo-root . --check-package`
- Jira owner: `PM-7`
- Confluence control page: `BOOTSTRAP_PENDING`

## Required working policy

1. Do not create, switch, rebase, or force-update branches inside a Codex task. The orchestrator prepares the branch.
2. Inspect `git status --porcelain=v1` before mutation. Preserve unrelated work; never reset it.
3. Do not guess across a scientific or numerical specification boundary. Use `BLOCKED_BY_UNRESOLVED_SPEC`.
4. For K0, production routing, solver equations, tolerance, acceptance, requested outputs, and historical evidence are immutable.
5. Run the tests required by the active work-unit JSON after the final edit. Report every skipped check and its reason.
6. Commit only completed work. Leave the worktree clean and report the final SHA/tree.
7. Update Jira/Confluence only after a durable GitHub commit/PR exists. If Atlassian is unavailable, use `ATLAS_SYNC_PENDING`.

The package was compiled for `main@8d0c79184e09efb5bdadc24a6315c60a71a44264` and the separate stacked implementation base `PR #20@e1124586a4029f86669e7489278c61ef676d61aa`. Re-read both before execution.
