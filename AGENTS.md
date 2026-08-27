# VigilODE Agent Map

Read this file before changing the repository. It is a map, not a complete process manual.

## Current Codex execution package

- Canonical machine contract: `docs/exec-plans/k0-stage-telemetry-integration-20260827/plan.json`
- Package-overlay contract: `docs/exec-plans/k0-stage-telemetry-integration-20260827/PACKAGE_OVERLAY_CONTRACT.md`
- Current handoff prompt: `docs/exec-plans/k0-stage-telemetry-integration-20260827/CODEX_HANDOFF_PROMPT.md`
- Launcher/orchestrator preparation: `docs/exec-plans/k0-stage-telemetry-integration-20260827/CODEX_LAUNCHER.md`
- K0 invariants: `docs/invariants/K0_STAGE_TELEMETRY.md`
- P0/P1 policy: `docs/quality/P0_P1_POLICY.md`
- Package validator: `python tools/verify-k0-stage-telemetry-plan.py --repo-root . --check-package --check-overlay-authority`
- Jira owner: `PM-7`
- Confluence control page: `15499267`

## Required working policy

1. Codex does not create, switch, merge, cherry-pick, rebase, reset, or force-update branches. The orchestrator prepares the two-parent overlay branch first.
2. Inspect `git status --porcelain=v1` before mutation. Preserve unrelated work; never reset it.
3. Do not guess across a scientific or numerical specification boundary. Use `BLOCKED_BY_UNRESOLVED_SPEC` only for unresolved observable semantics, not for the now-specified package overlay.
4. For K0, production routing, solver equations, tolerance, acceptance, requested outputs, semi-Jacobian-free homotopy certification, and historical evidence are immutable.
5. Run the tests required by the active work-unit JSON after the final edit. Report every skipped check and its reason.
6. Commit only completed work. Leave the worktree clean and report the final SHA/tree.
7. Update Jira/Confluence only after a durable GitHub commit/PR exists. If Atlassian is unavailable, use `ATLAS_SYNC_PENDING`.

The package was compiled for `main@8d0c79184e09efb5bdadc24a6315c60a71a44264` and source parent `PR #20@e1124586a4029f86669e7489278c61ef676d61aa`. The implementation branch must also contain the exact package branch tip as its second-parent ancestry. Re-read all three refs before execution.
