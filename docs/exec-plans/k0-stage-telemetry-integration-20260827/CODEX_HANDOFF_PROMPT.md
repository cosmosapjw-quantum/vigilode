# Codex Handoff Prompt — VigilODE K0 Stage Telemetry

You are the implementation agent. Work only in the orchestrator-prepared branch `research/k0-stage-telemetry-integration-20260827`.

## Absolute authority

- Repository: `https://github.com/cosmosapjw-quantum/vigilode`
- Canonical main: `8d0c79184e09efb5bdadc24a6315c60a71a44264`, tree `acd94364cf69f19d782619fc6c75554cb0754208`
- Source parent: draft PR #20, branch `research/a1-post-a2a3-kernel-rerun`, head `e1124586a4029f86669e7489278c61ef676d61aa`, tree `adbb933cf3bf3d401d652c8a6d9df661d8500a2b`
- Package parent: exact fetched tip of `origin/docs/k0-codex-execution-package-20260827`
- Prepared-branch topology: one orchestrator-created merge commit with ordered parents `source parent`, then `package parent`
- Machine plan: `docs/exec-plans/k0-stage-telemetry-integration-20260827/plan.json`
- Overlay contract: `docs/exec-plans/k0-stage-telemetry-integration-20260827/PACKAGE_OVERLAY_CONTRACT.md`
- Jira: `PM-7`
- Confluence page: `15499267`
- Claim class: `EXPLORATORY/NONAUTHORITATIVE`

Do not ask the user to run terminal commands. Inspect the machine and repository and perform the bounded work yourself.

**DO NOT ASK USER QUESTIONS.**
**DO NOT GUESS ACROSS A SPECIFICATION BOUNDARY.**

If repository evidence remains ambiguous and observable semantics would change, stop with `BLOCKED_BY_UNRESOLVED_SPEC`. If Git identity or overlay topology moved, stop with `BLOCKED_BY_AUTHORITY_DRIFT`. Preserve unrelated dirty work; never reset it.

## First commands

```bash
set -euo pipefail

git fetch --prune origin \
  main \
  research/a1-post-a2a3-kernel-rerun \
  docs/k0-codex-execution-package-20260827 \
  research/k0-stage-telemetry-integration-20260827

BASE=e1124586a4029f86669e7489278c61ef676d61aa
PACKAGE="$(git rev-parse origin/docs/k0-codex-execution-package-20260827)"

printf 'main=%s\nbase=%s\npackage=%s\nhead=%s\nbranch=%s\n' \
  "$(git rev-parse origin/main)" \
  "$(git rev-parse origin/research/a1-post-a2a3-kernel-rerun)" \
  "$PACKAGE" \
  "$(git rev-parse HEAD)" \
  "$(git branch --show-current)"

test "$(git rev-parse origin/main)" = "8d0c79184e09efb5bdadc24a6315c60a71a44264"
test "$(git rev-parse origin/research/a1-post-a2a3-kernel-rerun)" = "$BASE"
test "$(git branch --show-current)" = "research/k0-stage-telemetry-integration-20260827"
test -z "$(git status --porcelain=v1)"
test "$(git show -s --format='%P' HEAD)" = "$BASE $PACKAGE"
git merge-base --is-ancestor "$BASE" HEAD
git merge-base --is-ancestor "$PACKAGE" HEAD

python tools/verify-k0-stage-telemetry-plan.py \
  --repo-root . \
  --check-package \
  --check-overlay-authority
```

Expected: exact main and source parent above, current package tip as second parent of HEAD, exact implementation branch, clean worktree, `PACKAGE_CONTRACT_PASS`, and `PACKAGE_OVERLAY_PASS`.

Do not create, switch, merge, cherry-pick, rebase, reset, or force-update branches. The prior isolated local branch at the source parent was only a preserved pre-overlay state and was not WU-00 success.

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
- Do not modify or merge PR #20 or PR #21.

## GitHub publication

After WU-04 and review closure, push `research/k0-stage-telemetry-integration-20260827` and open one **draft, unmerged, stacked PR** with base `research/a1-post-a2a3-kernel-rerun`. The PR body must state exact source parent, package parent, prepared-start commit/tree, final head/tree, twelve cell states, verification commands, failures, claim boundary, PM-7, and Confluence page.

Do not retarget to `main` unless PR #20 is merged and the package is mechanically regenerated against the new authority.

## Atlassian integration

Use Atlassian MCP/Rovo after a durable GitHub commit/PR exists.

- Jira `PM-7` owns work state and blockers.
- Confluence page `15499267` owns navigation and claim boundary.
- Write exact source parent, package parent, prepared-start commit/tree, implementation final commit/tree, draft PR, evidence manifest, final state, blockers, and review verdict.
- Read both back.
- If writes/readback are unavailable, create `research/k0_stage_telemetry_20260827/handback/ATLAS_SYNC_PENDING.json`, keep Jira non-Done, and report `ATLAS_SYNC_PENDING`.
- Mismatched identifiers: `BLOCKED_BY_ATLASSIAN_AUTHORITY_DRIFT`.
- Never create a duplicate issue or page.

## Handback

Return and persist `final_handback.json` matching `schemas/final-handback.schema.json`. Include exact commands and exit codes, all skipped checks with reasons, source/package/prepared-start identities, P0/P1 findings and repairs, twelve cell states, final Git SHA/tree, draft PR, Jira status, Confluence version, and blockers.

Stop after publishing and verifying the draft stacked PR and synchronized control-plane records. Do not merge or start K2/K3.
