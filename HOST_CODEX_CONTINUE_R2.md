# Controlling host Codex entry — trusted-baseline transition repair

Use this file. It keeps `3f2f771…` as the finite trusted semantic-control
baseline and checks only the bounded successor repair. It does **not** re-run
the defective `c6…→package` positive allowlist.

```bash
set -euo pipefail

REPO=/tmp/vigilode-k0-stage-telemetry.kAguIL/tree
REF=origin/docs/k0-codex-execution-package-20260827
AUTH=docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_SEMANTIC_CONTINUATION_AUTHORITY.json

cd "$REPO"

test "$(git branch --show-current)" = \
  research/k0-stage-telemetry-integration-20260827
test "$(git rev-parse HEAD)" = \
  f6208a104d2f341157d900294aa30d8edb4446c0
test "$(git rev-parse HEAD^{tree})" = \
  19c393ca5a1ebb6c440130c9c3155e5625c85ce3
test -z "$(git status --porcelain=v1)"

git fetch --prune origin \
  docs/k0-codex-execution-package-20260827

K0_PACKAGE_SHA=$(git rev-parse "$REF")
export K0_PACKAGE_SHA

git show "$K0_PACKAGE_SHA:$AUTH" |
python3 -c '
import json
import sys

x = json.load(sys.stdin)
assert x["schema"] == \
    "vigilode-k0-wu05-semantic-continuation/v1"
assert x["status"] == "BOUND"
assert x["finding"] == "WU05-NEW-P0-001"
assert x["trusted_control_baseline"] == \
    "3f2f7712d04e12beb3291b8369e82bd3f8d92c45"
assert x["preserved_prepared_local"]["head"] == \
    "f6208a104d2f341157d900294aa30d8edb4446c0"
assert x["campaign_rerun"] == \
    "FORBIDDEN_FOR_REPRESENTATION_ONLY_CHANGE"
'

ENTRY=$(mktemp "${TMPDIR:-/tmp}/k0-semantic-transition.XXXXXX")
trap 'rm -f -- "$ENTRY"' EXIT

git show "$K0_PACKAGE_SHA:START_CONTINUATION_R2.sh" > "$ENTRY"

bash "$ENTRY" \
  --repo-root "$REPO" \
  --package-sha "$K0_PACKAGE_SHA"
```

Required preparation output:

```text
SEMANTIC_TRANSITION_SCOPE_PASS
EVIDENCE_V3_SEMANTIC_SELF_TEST_PASS
LOCAL_WU05_AUTHORITY_READY
CONTINUATION_RECEIPT=...
```

These are preparation/readiness markers only. They do not admit the historical
twelve-cell evidence.

After readiness, continue immediately:

1. locate all twelve immutable WU-04 raw receipts;
2. recompute and retain their exact SHA-256 values;
3. run the semantic evidence validator on the actual twelve files;
4. admit `EVIDENCE_V3_PASS` only for that real run;
5. report result, provenance, and packaging validity separately;
6. return directly to the five existing WU-05 findings;
7. run the actual aggregate ERROR regression, public-bridge source audit,
   signed-residual vector mutation, targeted/affected tests;
8. reuse the existing single fresh review and final differential audit;
9. publish only one draft stacked implementation PR after P0=P1=0;
10. synchronize/read back GitHub, Jira PM-7, and Confluence 15499267.

Do not create another package/schema/manifest/review layer. Do not rerun the
campaign for representation-only change. Stop only for an actual immutable raw,
scientific/numerical, source-repair, destructive-state, or attribution defect.
