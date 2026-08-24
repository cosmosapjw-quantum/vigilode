# PM-4 Task-1 CWD-independent surgical handoff

This clean branch supersedes PR #12 and the corrupted archive transport in PR #13. It contains:

1. a RED-test commit encoding the CWD-independent archive-authority contract;
2. one package commit containing the exact verified handoff archive as checksum-addressed base64 parts;
3. one transport-verification commit fixing the retrieval command and publishing the exact Git-blob manifest.

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

Retrieve and reconstruct without merging the handoff branch:

```bash
git fetch origin handoff/pm4-task1-r5-cwd-surgical-repair-final-20260824

git worktree add --detach \
  ../vigilode-pm4-cwd-surgical-handoff \
  origin/handoff/pm4-task1-r5-cwd-surgical-repair-final-20260824

cd ../vigilode-pm4-cwd-surgical-handoff/handoff/pm4-task1-r5-cwd-surgical-repair-20260824

./RECONSTRUCT_AND_VERIFY.sh

tar -xzf VIGILODE_PM4_TASK1_CWD_SURGICAL_HANDOFF_20260824.tar.gz
```

Then read, inside the extracted package:

```text
README_FIRST.md
AGENTS.md
AUDIT_COMPILED_EXEC_PLAN.yaml
IMPLEMENTER_PROMPT.md
```

`TRANSPORT_BLOB_MANIFEST.tsv` records the exact byte size and Git blob SHA for every part used in the committed tree.

## Repair invariant

The outer R4 archive gate is CWD-independent. Load-bearing instructions invoke one absolute-path Python authority validator through `acceptance/run_control_plane_preflight.sh`; they do not invoke the outer sidecar through a raw `sha256sum -c` command whose relative filename semantics depend on the caller's current directory.

The package verifies the same authority from the handoff root, repository root, and an unrelated temporary directory, and rejects wrong hash, withdrawn hash, wrong archive basename, multiple active sidecar records, and missing archive cases.

## External static audit boundary

The external static audit opens later DAG nodes for E4 reproducibility and A1/A2/A3 fairness. Those findings are retained in `EXTERNAL_STATIC_AUDIT_DAG.md` but must not be mixed into the exact four-file Task-1 publication transaction.
