# PM-4 Task-1 CWD-independent surgical handoff

This clean branch supersedes the accumulated forensic handoff in PR #12. It contains the RED regression contracts in the first commit and the verified canonical transport package in the second commit.

## Production boundary

```text
canonical main
140f6b5c078c3d8fcd5b6c52310c063ee233dc12

target feature before Task-1 publication
b2d5ec41cb147e01aadbc9c42928da8abfa75c58

target PR
#11 — OPEN / DRAFT / UNMERGED / ZERO FILE DIFF
```

No solver code, Task-1 source, Cargo configuration, lockfile, dependency, timing campaign, candidate ranking, merge, or Task 2 is changed by this branch.

## Canonical package

```text
VIGILODE_PM4_TASK1_CWD_SURGICAL_HANDOFF_20260824.tar.gz
SHA-256 5ddb0f19be010d53187bac00d468c110c49a54b3cf168894207584bee04f1694
```

Retrieve it without checking out or merging this branch:

```bash
git fetch origin handoff/pm4-task1-r5-cwd-surgical-repair-20260824

rm -f VIGILODE_PM4_TASK1_CWD_SURGICAL_HANDOFF_20260824.tar.gz.b64.part-*
for part in 00 01 02 03 04 05 06; do
  git show \
    origin/handoff/pm4-task1-r5-cwd-surgical-repair-20260824:\
handoff/pm4-task1-r5-cwd-surgical-repair-20260824/VIGILODE_PM4_TASK1_CWD_SURGICAL_HANDOFF_20260824.tar.gz.b64.part-$part \
    > VIGILODE_PM4_TASK1_CWD_SURGICAL_HANDOFF_20260824.tar.gz.b64.part-$part
done

cat VIGILODE_PM4_TASK1_CWD_SURGICAL_HANDOFF_20260824.tar.gz.b64.part-* \
  | base64 -d \
  > VIGILODE_PM4_TASK1_CWD_SURGICAL_HANDOFF_20260824.tar.gz

git show \
  origin/handoff/pm4-task1-r5-cwd-surgical-repair-20260824:\
handoff/pm4-task1-r5-cwd-surgical-repair-20260824/VIGILODE_PM4_TASK1_CWD_SURGICAL_HANDOFF_20260824.tar.gz.sha256 \
  > VIGILODE_PM4_TASK1_CWD_SURGICAL_HANDOFF_20260824.tar.gz.sha256

sha256sum -c VIGILODE_PM4_TASK1_CWD_SURGICAL_HANDOFF_20260824.tar.gz.sha256
tar -xzf VIGILODE_PM4_TASK1_CWD_SURGICAL_HANDOFF_20260824.tar.gz
```

Then read `README_FIRST.md`, `AGENTS.md`, `AUDIT_COMPILED_EXEC_PLAN.yaml`, and `IMPLEMENTER_PROMPT.md` inside the extracted package.

## Repair invariant

The outer R4 archive gate is now CWD-independent. Load-bearing instructions call one absolute-path Python authority validator through `acceptance/run_control_plane_preflight.sh`; they do not invoke the outer sidecar through a raw `sha256sum -c` command whose relative filename semantics depend on the caller's current directory.

The package verifies the same authority from the handoff root, repository root, and an unrelated temporary directory, and rejects wrong hash, withdrawn hash, wrong archive basename, multiple active sidecar records, and missing archive cases.

## External static audit boundary

The attached static audit opens later DAG nodes for E4 reproducibility and A1/A2/A3 fairness. Those findings are retained in `EXTERNAL_STATIC_AUDIT_DAG.md` but must not be mixed into the exact four-file Task-1 publication transaction.
