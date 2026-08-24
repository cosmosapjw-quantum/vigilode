# PM-4 Task-1 Codex recovery handoff

This branch contains the machine-checkable handoff for repairing the R4 Cargo-vendor gate and publishing the already-sealed PM-4 Task-1 source patch to draft PR #11.

## Decode the package

```bash
git fetch origin handoff/pm4-task1-publication-recovery-20260824
git show origin/handoff/pm4-task1-publication-recovery-20260824:handoff/pm4-task1-publication-recovery-20260824/VIGILODE_PM4_CODEX_HANDOFF_20260824.zip.b64 \
  | base64 -d > VIGILODE_PM4_CODEX_HANDOFF_20260824.zip

git show origin/handoff/pm4-task1-publication-recovery-20260824:handoff/pm4-task1-publication-recovery-20260824/VIGILODE_PM4_CODEX_HANDOFF_20260824.zip.sha256 \
  > VIGILODE_PM4_CODEX_HANDOFF_20260824.zip.sha256

sha256sum -c VIGILODE_PM4_CODEX_HANDOFF_20260824.zip.sha256
unzip VIGILODE_PM4_CODEX_HANDOFF_20260824.zip
```

Then read:

```text
VIGILODE_PM4_CODEX_HANDOFF_20260824/AGENTS.md
VIGILODE_PM4_CODEX_HANDOFF_20260824/README_FIRST.md
VIGILODE_PM4_CODEX_HANDOFF_20260824/AUDIT_COMPILED_EXEC_PLAN.yaml
VIGILODE_PM4_CODEX_HANDOFF_20260824/IMPLEMENTER_PROMPT.md
```

## Boundaries

- target PR: #11
- canonical main: `140f6b5c078c3d8fcd5b6c52310c063ee233dc12`
- expected feature head before Task-1 publication: `b2d5ec41cb147e01aadbc9c42928da8abfa75c58`
- no force push, merge, wall timing, candidate ranking, or PM-4 Task 2
- the Task-1 source patch is immutable

This handoff branch is documentation/transport only. Do not merge it into `main` as part of the Task-1 publication transaction.
