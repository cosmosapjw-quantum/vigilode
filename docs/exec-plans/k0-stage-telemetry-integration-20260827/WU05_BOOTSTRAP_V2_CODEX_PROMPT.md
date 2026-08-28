# Host Codex: prepare first, then repair WU-05

Use the exact `K0_PACKAGE_SHA` supplied with this prompt. Do not derive authority from an unpinned moving branch.

You have TWO sequential roles in this same host session:

1. HOST_CODEX_ORCHESTRATOR: run the exact pinned bootstrap-v2 script.
2. WU-05 IMPLEMENTER: only after `LOCAL_WU05_AUTHORITY_READY`, perform the existing bounded fresh-review repair.

The previous broad prohibition on Codex branch preparation is overridden ONLY for executing this pinned, tested bootstrap script. Do not perform manual merge resolution/reset/rebase/stash/amend/force-update. Do not run package-only validators from the unprepared implementation tree.

Expected preserved local state:

```text
branch research/k0-stage-telemetry-integration-20260827
head   e95ce1e58a603306cb665a6ab91cfe02d279972f
tree   e3621a370297a76907e97730ebd18c5c1e0fb83e
clean  true
```

Locate `/tmp/vigilode-k0-stage-telemetry.kAguIL/tree`; if moved, inspect `git -C ~/vigilode worktree list --porcelain` without switching branches. Keep every unpushed WU-00–04/review commit.

Fetch `docs/k0-codex-execution-package-20260827` and require its remote-tracking SHA to equal the supplied pin. Use `git show "$K0_PACKAGE_SHA:tools/k0-wu05-bootstrap-v2.sh"` to extract the runner to a `mktemp` file outside the worktree. Execute it with `bash ... --repo-root ... --package-sha "$K0_PACKAGE_SHA"`.

The script accepts either the original clean review or its exact clean two-parent merge with this package; it never creates a second merge on a retry. It validates real package dependencies in a detached worktree before mutation, requires all structured PASS markers, preserves logs, and rejects non-control source changes.

After readiness, read `WU05_BOOTSTRAP_V2_HANDOFF.md`, then `WU05_LOCAL_CODEX_PROMPT.md` and its seven required source/contracts. The old instruction that an external orchestrator has ALREADY prepared the branch is superseded by your actual bootstrap receipt. Do not rerun WU-00–04.

Repair only the five recorded fresh-review findings and already-bound supplement cases. Preserve raw campaign receipts; build evidence-v3 wrappers mechanically; make no numerical claims from package tests. Keep Cargo manifests, tolerance, equations, production signatures/routes, and homotopy certification unchanged.

Run actual targeted regression/mutation tests, source-derived bridge audit, evidence checks, the existing single read-only fresh review, and final differential audit. Retain original P3 lints as nonblocking unless your delta adds a new substantive defect.

Do not push source or open the implementation PR until required P0/P1 closure gates pass. Then publish only the draft stacked implementation PR; no PR merge, activation, timing, ranking, tag, or release.

Record the bootstrap receipt path, exact package/prepared/final SHAs and trees, all actually executed commands, five finding dispositions, raw evidence preservation, skipped work and reasons, and GitHub/Jira PM-7/Confluence 15499267 synchronization. Report `ATLAS_SYNC_PENDING` on an actual integration outage.
