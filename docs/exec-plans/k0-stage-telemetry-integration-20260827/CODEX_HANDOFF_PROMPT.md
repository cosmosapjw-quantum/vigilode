# Codex Handoff Prompt — VigilODE K0 Stage Telemetry

You are the implementation agent. Work in the orchestrator-prepared branch `research/k0-stage-telemetry-integration-20260827`.

## Absolute authority

- Repository: `https://github.com/cosmosapjw-quantum/vigilode`
- Canonical main: `8d0c79184e09efb5bdadc24a6315c60a71a44264`, tree `acd94364cf69f19d782619fc6c75554cb0754208`
- Required implementation base: draft PR #20, branch `research/a1-post-a2a3-kernel-rerun`, head `e1124586a4029f86669e7489278c61ef676d61aa`, tree `adbb933cf3bf3d401d652c8a6d9df661d8500a2b`
- Machine plan: `docs/exec-plans/k0-stage-telemetry-integration-20260827/plan.json`
- Jira: `PM-7`
- Confluence page: `15499267`
- Claim class: `EXPLORATORY/NONAUTHORITATIVE`

Do not ask me to run terminal commands. Inspect the machine and repository and perform the bounded work yourself.

**DO NOT ASK USER QUESTIONS.**
**DO NOT GUESS ACROSS A SPECIFICATION BOUNDARY.**

If repository evidence remains ambiguous and observable semantics would change, stop with `BLOCKED_BY_UNRESOLVED_SPEC`. If Git identity moved, stop with `BLOCKED_BY_AUTHORITY_DRIFT`. Preserve unrelated dirty work; never reset it.

## First commands

```bash
git fetch --prune origin main research/a1-post-a2a3-kernel-rerun docs/k0-codex-execution-package-20260827
printf 'main=%s\nbase=%s\nbranch=%s\n' \
  "$(git rev-parse origin/main)" \
  "$(git rev-parse origin/research/a1-post-a2a3-kernel-rerun)" \
  "$(git branch --show-current)"
git status --porcelain=v1
python tools/verify-k0-stage-telemetry-plan.py --repo-root . --check-package
```

Expected: main `8d0c79184e09efb5bdadc24a6315c60a71a44264`, base `e1124586a4029f86669e7489278c61ef676d61aa`, branch `research/k0-stage-telemetry-integration-20260827`, clean worktree, validator PASS.

## Execute exactly in order

1. `WU-00-authority-intake.json`
2. `WU-01-schema-and-observation-types.json`
3. `WU-02-solver-observation-hooks.json`
4. `WU-03-stage-receipts-and-aggregate.json`
5. `WU-04-frozen-six-family-replay.json`
6. `WU-05-review-audit-and-atlassian-sync.json`

For every work unit:

- write the specified RED test first and run it;
- implement the minimum change;
- run targeted, negative/regression, and affected integration checks named in the JSON;
- commit a coherent completed unit;
- do not proceed on P0/P1 or unresolved semantics;
- do not weaken tests, tolerances, families, expected outputs, or claim boundaries.

## Hard boundaries

- Public production remains legacy.
- Projected residual is advisory; unpreconditioned true residual is authority.
- Count linear and diagnostic applies separately and in checked total.
- Preserve every failed stage and matrix cell.
- Do not modify BDF, Radau, homotopy certification, adaptive-global-error/dense-output work, or historical authority receipts.
- Do not implement shared Krylov, new predictors, timing, speedup, activation, tag, release, or merge.
- Do not modify or merge PR #20.

## GitHub publication

After WU-04 and review closure, push `research/k0-stage-telemetry-integration-20260827` and open one **draft, unmerged, stacked PR** with base `research/a1-post-a2a3-kernel-rerun`. The PR body must state exact base/head/tree, twelve cell states, verification commands, failures, claim boundary, PM-7, and Confluence page.

Do not retarget to `main` unless PR #20 is merged and the package is mechanically regenerated against the new authority.

## Atlassian integration

Use Atlassian MCP/Rovo after a durable GitHub commit/PR exists.

- Jira `PM-7` owns work state and blockers.
- Confluence page `15499267` owns navigation and claim boundary.
- Write exact implementation branch, final commit/tree, draft PR, evidence manifest, final state, blockers, and review verdict.
- Read both back.
- If writes/readback are unavailable, create `research/k0_stage_telemetry_20260827/handback/ATLAS_SYNC_PENDING.json`, keep Jira non-Done, and report `ATLAS_SYNC_PENDING`.
- Mismatched identifiers: `BLOCKED_BY_ATLASSIAN_AUTHORITY_DRIFT`.
- Never create duplicate issue/page.

## Handback

Return and persist `final_handback.json` matching `schemas/final-handback.schema.json`. Include exact commands and exit codes, all skipped checks with reasons, P0/P1 findings and repairs, twelve cell states, final Git SHA/tree, draft PR, Jira status, Confluence version, and blockers.

Stop after publishing and verifying the draft stacked PR and synchronized control-plane records. Do not merge or start K2/K3.
