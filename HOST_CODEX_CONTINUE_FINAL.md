# Final host Codex entry — Git-native, no external bundle

This file supersedes `HOST_CODEX_CONTINUE.md` and every instruction requiring a host-side continuation ZIP.

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

git show "$K0_PACKAGE_SHA:$AUTH" | python3 -c '
import json,sys
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
git show "$K0_PACKAGE_SHA:START_CONTINUATION_FINAL.sh" > "$ENTRY"
bash "$ENTRY" --repo-root "$REPO" --package-sha "$K0_PACKAGE_SHA"
```

Require:

```text
EVIDENCE_V3_PASS
LOCAL_WU05_AUTHORITY_READY
CONTINUATION_RECEIPT=...
```

After readiness, read `CODEX_START_HERE.md` and `WU05_SEMANTIC_REPAIR_HANDOFF.md`, then continue the existing WU-05 source/evidence repair immediately. Do not search for a ZIP, replay WU-00–04, or rerun WU-04 merely because wrapper/package bytes changed.

Raw SHA-256 remains provenance; scientific validity is determined by the canonical numerical/work/gate/audit projection. Missing historical signed-residual telemetry remains null with `LEGACY_NOT_RECORDED`; current source sign correctness remains a separate executed mutation test.
