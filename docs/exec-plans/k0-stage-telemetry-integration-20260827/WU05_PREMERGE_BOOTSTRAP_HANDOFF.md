# VigilODE K0 WU-05 pre-merge bootstrap handoff

## Why this bootstrap exists

The preserved implementation branch is correctly still at the fresh-review commit and therefore cannot execute validators that exist only in the package branch. The old ordering was impossible:

```text
run package validator -> merge package
```

The controlling ordering is now:

```text
fetch exact package SHA
-> extract bootstrap script with git show
-> validate package in detached temporary worktree
-> merge exact package SHA as second parent
-> run post-merge authority gate
-> start Codex repair
```

No package file is expected to exist in the preserved branch before the merge.

## Frozen local identity

```text
branch  research/k0-stage-telemetry-integration-20260827
HEAD    e95ce1e58a603306cb665a6ab91cfe02d279972f
tree    e3621a370297a76907e97730ebd18c5c1e0fb83e
status  clean
```

## Local orchestrator command

Use the exact package SHA supplied in the final publication receipt. A branch name is discovery only.

```bash
set -euo pipefail

REPO=/tmp/vigilode-k0-stage-telemetry.kAguIL/tree
PACKAGE_SHA=<EXACT_PACKAGE_SHA_FROM_PUBLICATION_RECEIPT>
BOOT=/tmp/k0-wu05-premerge-bootstrap.sh

cd "$REPO"
git fetch --prune origin docs/k0-codex-execution-package-20260827

test "$(git rev-parse origin/docs/k0-codex-execution-package-20260827)" = "$PACKAGE_SHA"
git show "$PACKAGE_SHA:tools/k0-wu05-premerge-bootstrap.sh" > "$BOOT"
chmod 700 "$BOOT"

"$BOOT" --repo-root "$REPO" --package-sha "$PACKAGE_SHA"
```

The script first validates the package in a detached temporary worktree. It does not merge if that validation fails. It then performs the exact two-parent merge and runs the post-merge gate.

Required final marker:

```text
LOCAL_WU05_AUTHORITY_READY
```

Only after that marker should Codex receive `WU05_PREMERGE_CODEX_PROMPT.md`.

## Failure handling

- Wrong branch, HEAD, tree, package SHA, or dirty state: `BLOCKED_BY_AUTHORITY_DRIFT`.
- Package bootstrap or detached-worktree validation failure: `STOP_INVALID` with no local merge.
- Merge conflict: the script aborts the merge and reports `BLOCKED_BY_AUTHORITY_DRIFT`.
- Post-merge gate failure: preserve the clean merge commit and report `STOP_INVALID`; do not mutate source.
