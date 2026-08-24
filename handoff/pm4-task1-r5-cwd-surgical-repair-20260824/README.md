# PM-4 Task-1 CWD-independent surgical handoff

This clean branch supersedes the accumulated forensic handoff in PR #12. It
contains the CWD-independent control-plane contracts, the checksum-addressed
complete handoff package, and repository-local launch/reconstruction entry
points. No ChatGPT sandbox file is required after this branch is fetched.

## Production boundary

```text
canonical main
140f6b5c078c3d8fcd5b6c52310c063ee233dc12

target feature before Task-1 publication
b2d5ec41cb147e01aadbc9c42928da8abfa75c58

target PR
#11 — OPEN / DRAFT / UNMERGED / ZERO FILE DIFF
```

No solver code, Task-1 source, Cargo configuration, lockfile, dependency,
timing campaign, candidate ranking, merge, or Task 2 is changed by this branch.

## Preferred fetched-worktree path

After adding the detached handoff worktree, define:

```bash
HANDOFF_ROOT="$HOME/vigilode-pm4-cwd-surgical-handoff/handoff/pm4-task1-r5-cwd-surgical-repair-20260824"
test -d "$HANDOFF_ROOT"
```

The complete, repository-local launcher is:

```bash
cat "$HANDOFF_ROOT/CODEX_LAUNCHER.md"
```

The complete package reconstruction and verification entry point is:

```bash
bash "$HANDOFF_ROOT/RECONSTRUCT_AND_VERIFY.sh" \
  --output-dir "$HOME/vigilode-pm4-cwd-surgical-runtime"
```

This is the preferred path. It reconstructs the archive from the checked-in
base64 parts, verifies the exact archive SHA-256 and internal package manifest,
requires all executable prompt/contract files, and writes
`PACKAGE_ROOT.txt` plus a reconstruction receipt.

## Canonical package

```text
VIGILODE_PM4_TASK1_CWD_SURGICAL_HANDOFF_20260824.tar.gz
SHA-256 5ddb0f19be010d53187bac00d468c110c49a54b3cf168894207584bee04f1694
```

The archive is stored as parts `00..06` under this handoff directory. The
reconstruction script concatenates them in explicit order and fails closed on
any missing part, base64 error, hash mismatch, invalid gzip/tar structure,
missing required file, package-manifest mismatch, Python compilation error, or
shell-syntax error.

A manual fallback remains possible by following `CODEX_LAUNCHER.md`, but no
manual reconstruction is needed in the normal workflow.

## Repair invariant

The outer R4 archive gate is CWD-independent. Load-bearing instructions call
one absolute-path Python authority validator through
`acceptance/run_control_plane_preflight.sh`; they do not invoke the outer
sidecar through a raw `sha256sum -c` command whose relative filename semantics
depend on the caller's current directory.

The control-plane package verifies the same authority from the handoff root,
repository root, and an unrelated temporary directory, and rejects wrong hash,
withdrawn hash, wrong archive basename, multiple active sidecar records, and
missing archive cases.

## External static audit boundary

The attached static audit opens later DAG nodes for E4 reproducibility and
A1/A2/A3 fairness. Those findings are retained in
`EXTERNAL_STATIC_AUDIT_DAG.md` but must not be mixed into the exact four-file
Task-1 publication transaction.
