# Controlling host Codex entry — no bundle, no premature evidence claim

Use this file. It supersedes both earlier `HOST_CODEX_CONTINUE*` entries.

```bash
set -euo pipefail
REPO=/tmp/vigilode-k0-stage-telemetry.kAguIL/tree
REF=origin/docs/k0-codex-execution-package-20260827
AUTH=docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_SEMANTIC_CONTINUATION_AUTHORITY.json
cd "$REPO"
test "$(git branch --show-current)" = research/k0-stage-telemetry-integration-20260827
test "$(git rev-parse HEAD)" = f6208a104d2f341157d900294aa30d8edb4446c0
test "$(git rev-parse HEAD^{tree})" = 19c393ca5a1ebb6c440130c9c3155e5625c85ce3
test -z "$(git status --porcelain=v1)"
git fetch --prune origin docs/k0-codex-execution-package-20260827
K0_PACKAGE_SHA=$(git rev-parse "$REF"); export K0_PACKAGE_SHA
git show "$K0_PACKAGE_SHA:$AUTH" | python3 -c '
import json,sys
x=json.load(sys.stdin)
assert x["schema"]=="vigilode-k0-wu05-semantic-continuation/v1"
assert x["status"]=="BOUND"
assert x["finding"]=="WU05-NEW-P0-001"
assert x["prior_package_sha"]=="c6ec0121be11f76b86afc21f8ae7a304d35c6d83"
assert x["preserved_prepared_local"]["head"]=="f6208a104d2f341157d900294aa30d8edb4446c0"
assert x["campaign_rerun"]=="FORBIDDEN_FOR_REPRESENTATION_ONLY_CHANGE"
'
ENTRY=$(mktemp "${TMPDIR:-/tmp}/k0-semantic-r2.XXXXXX"); trap 'rm -f -- "$ENTRY"' EXIT
git show "$K0_PACKAGE_SHA:START_CONTINUATION_R2.sh" > "$ENTRY"
bash "$ENTRY" --repo-root "$REPO" --package-sha "$K0_PACKAGE_SHA"
```

Preparation succeeds only with:

```text
EVIDENCE_V3_SEMANTIC_SELF_TEST_PASS
LOCAL_WU05_AUTHORITY_READY
CONTINUATION_RECEIPT=...
```

`EVIDENCE_V3_SEMANTIC_SELF_TEST_PASS` means only that the migration specification and regressions are executable. It is **not** the actual 12-cell evidence result.

After readiness, continue WU-05 immediately. Run `tools/verify-k0-wu05-semantic-evidence.py` on the actual twelve preserved raw receipts. Only that actual run may emit the closure marker:

```text
EVIDENCE_V3_PASS
```

Preserve all twelve raw SHA-256 values. Do not invent historical signed-residual digests, replay WU-00–04, or rerun WU-04 for a representation-only change. Then close the original five findings, run actual targeted/workspace tests, the existing single fresh repair review, and the final differential audit. Publish only a draft stacked implementation PR after P0/P1 closure.
