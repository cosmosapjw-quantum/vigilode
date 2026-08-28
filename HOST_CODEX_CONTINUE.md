# Host Codex continuation — no external bundle required

This is the controlling entry when `START_CONTINUATION.sh` is absent from the host filesystem.

The package branch is used only to discover a candidate commit. Before executing it, validate the bound semantic authority from that same commit; `START_CONTINUATION.sh` then verifies ancestry, allowed path scope, the preserved local commit/tree, semantic self-tests, and the exact merge topology.

```bash
set -euo pipefail
REPO=/tmp/vigilode-k0-stage-telemetry.kAguIL/tree
PACKAGE_REF=origin/docs/k0-codex-execution-package-20260827
AUTH=docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_SEMANTIC_CONTINUATION_AUTHORITY.json

cd "$REPO"
test "$(git branch --show-current)" = research/k0-stage-telemetry-integration-20260827
test "$(git rev-parse HEAD)" = f6208a104d2f341157d900294aa30d8edb4446c0
test "$(git rev-parse HEAD^{tree})" = 19c393ca5a1ebb6c440130c9c3155e5625c85ce3
test -z "$(git status --porcelain=v1)"

git fetch --prune origin docs/k0-codex-execution-package-20260827
K0_PACKAGE_SHA=$(git rev-parse "$PACKAGE_REF")
export K0_PACKAGE_SHA

# Validate the semantic authority before extracting executable code.
git show "$K0_PACKAGE_SHA:$AUTH" | python3 -c '
import json, sys
x=json.load(sys.stdin)
assert x["schema"] == "vigilode-k0-wu05-semantic-continuation/v1"
assert x["status"] == "BOUND"
assert x["finding"] == "WU05-NEW-P0-001"
assert x["prior_package_sha"] == "c6ec0121be11f76b86afc21f8ae7a304d35c6d83"
assert x["preserved_prepared_local"]["head"] == "f6208a104d2f341157d900294aa30d8edb4446c0"
assert x["campaign_rerun"] == "FORBIDDEN_FOR_REPRESENTATION_ONLY_CHANGE"
'

ENTRY=$(mktemp "${TMPDIR:-/tmp}/k0-semantic-continuation.XXXXXX")
trap 'rm -f -- "$ENTRY"' EXIT
git show "$K0_PACKAGE_SHA:START_CONTINUATION.sh" > "$ENTRY"
bash "$ENTRY" --repo-root "$REPO" --package-sha "$K0_PACKAGE_SHA"
```

Required output:

```text
EVIDENCE_V3_PASS
LOCAL_WU05_AUTHORITY_READY
CONTINUATION_RECEIPT=...
```

Then read `CODEX_START_HERE.md` and `WU05_SEMANTIC_REPAIR_HANDOFF.md` from the prepared tree and continue WU-05 immediately. Do not search for or unpack an external continuation ZIP.
