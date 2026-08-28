# VigilODE K0 WU-05 bootstrap v2 handoff

This is the controlling local handoff. It supersedes every earlier pre-merge/bootstrap handoff.

## Preserved local state

```text
branch  research/k0-stage-telemetry-integration-20260827
HEAD    e95ce1e58a603306cb665a6ab91cfe02d279972f
tree    e3621a370297a76907e97730ebd18c5c1e0fb83e
status  clean
```

Do not reset, rebase, squash, amend, cherry-pick over, or replace the completed WU-00 through WU-04 commits or the fresh-review commit.

## Exact local action

Read the package SHA from the delivered publication receipt. Do not substitute a moving branch name for that SHA.

```bash
set -euo pipefail

REPO=/tmp/vigilode-k0-stage-telemetry.kAguIL/tree
PACKAGE_SHA=<EXACT_40_HEX_FROM_PUBLICATION_RECEIPT>
RUNNER=/tmp/k0-wu05-bootstrap-v2.sh

cd "$REPO"
git fetch --prune origin docs/k0-codex-execution-package-20260827
test "$(git rev-parse origin/docs/k0-codex-execution-package-20260827)" = "$PACKAGE_SHA"

git show "$PACKAGE_SHA:tools/k0-wu05-bootstrap-v2.sh" > "$RUNNER"
chmod 700 "$RUNNER"

"$RUNNER" --repo-root "$REPO" --package-sha "$PACKAGE_SHA"
```

The runner performs these operations in order:

1. Verify local branch, HEAD, tree, and cleanliness.
2. Verify the package commit and all required bootstrap/WU-05 files directly from Git objects.
3. Create a detached temporary worktree at the exact package commit.
4. Run package and WU-05 supplement validation there before changing local history.
5. Merge the exact package commit as second parent of the preserved fresh-review head.
6. Run post-merge and repair-merge authority checks.
7. Confirm a clean worktree and emit `LOCAL_WU05_AUTHORITY_READY`.

No package-resident command is invoked from the preserved branch before the merge.

## Required markers

```text
WU05_BOOTSTRAP_V2_PREMERGE_PASS
PACKAGE_CONTRACT_PASS
WU05_SUPPLEMENT_MANIFEST_PASS
LEGACY_REPAIR_BLOBS_PASS
EXTERNAL_PACKAGE_PIN_PASS
WU05_SUPPLEMENT_AUTHORITY_PASS
HOSTILE_FIXTURES_PASS
WU05_BOOTSTRAP_V2_POSTMERGE_PASS
WU05_REPAIR_MERGE_PASS
LOCAL_WU05_AUTHORITY_READY
```

## Failure semantics

- Identity or branch drift: `BLOCKED_BY_AUTHORITY_DRIFT`.
- Package or detached-worktree validation failure: `STOP_INVALID`, before merge.
- Merge conflict: merge is aborted and the state is `BLOCKED_BY_AUTHORITY_DRIFT`.
- Post-merge authority failure: preserve the clean merge commit, perform no source mutation, and report `STOP_INVALID`.

After `LOCAL_WU05_AUTHORITY_READY`, paste `WU05_BOOTSTRAP_V2_CODEX_PROMPT.md` into Codex.
