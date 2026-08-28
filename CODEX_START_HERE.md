# Host Codex — continue VigilODE WU-05 without an external bundle

Use the exact `K0_PACKAGE_SHA` supplied with this handoff. Do not infer it from a stale ZIP, an old sidecar, or an unpinned branch.

## Role 1 — semantic continuation orchestrator

The preserved worktree is expected at `/tmp/vigilode-k0-stage-telemetry.kAguIL/tree`; if it moved, locate the exact branch with read-only `git -C ~/vigilode worktree list --porcelain`.

Require:

```text
branch   research/k0-stage-telemetry-integration-20260827
HEAD     f6208a104d2f341157d900294aa30d8edb4446c0
clean    true
```

Run:

```bash
set -euo pipefail
: "${K0_PACKAGE_SHA:?exact package SHA required}"
REPO=/tmp/vigilode-k0-stage-telemetry.kAguIL/tree
cd "$REPO"
test "$(git branch --show-current)" = research/k0-stage-telemetry-integration-20260827
test -z "$(git status --porcelain=v1)"
git fetch --prune origin docs/k0-codex-execution-package-20260827
test "$(git rev-parse origin/docs/k0-codex-execution-package-20260827)" = "$K0_PACKAGE_SHA"
ENTRY=$(mktemp "${TMPDIR:-/tmp}/k0-semantic-continuation.XXXXXX")
trap 'rm -f -- "$ENTRY"' EXIT
git show "$K0_PACKAGE_SHA:START_CONTINUATION.sh" > "$ENTRY"
bash "$ENTRY" --repo-root "$REPO" --package-sha "$K0_PACKAGE_SHA"
```

Do not use or search for a downloaded continuation ZIP. The exact Git commit contains the entry script, handoff, policies, schemas, and validator.

Stop without source mutation unless all appear:

```text
EVIDENCE_V3_PASS
LOCAL_WU05_AUTHORITY_READY
CONTINUATION_RECEIPT=...
```

No reset, rebase, stash, amend, force update, branch replacement, or manual conflict resolution is permitted.

## Role 2 — WU-05 implementer

After readiness, read `WU05_SEMANTIC_REPAIR_HANDOFF.md` and the semantic authority/policy files it lists. Then continue the existing WU-05 repair; do not restart WU-00 through WU-04.

`WU05-NEW-P0-001` is closed only at the representation/specification layer. Use `tools/verify-k0-wu05-semantic-evidence.py` on the actual 12 raw receipts and generate wrappers mechanically. It must preserve each raw byte SHA-256 while deriving numerical payload identity independently.

Mandatory semantics:

- no required raw top-level `status` or `tolerance_arm` label;
- exact numerical tolerance `rtol=1e-10`, `atol=1e-12`;
- source identity from raw outer `scientific_execution_head_sha/tree`;
- top-level `error: null` means no error;
- absent historical signed digest remains null with `LEGACY_NOT_RECORDED`;
- fabricated historical digest is a hard failure;
- wrapper/archive/source metadata do not enter the numerical digest;
- genuine numerical/work/gate/audit mismatches remain fail-closed.

Then repair the five original findings already recorded in `fresh_review_findings.yaml`. Preserve raw WU-04 bytes. Do not rerun the campaign for a representation-only repair. Execute actual targeted negative/mutation tests, source-derived public-bridge audit, aggregate-error guard, the current signed-residual mutation test, workspace regressions, the existing single fresh repair review, and the final differential audit.

Publish only a draft stacked implementation PR after all P0/P1 findings are closed. Update/read back Jira PM-7 and Confluence 15499267. An actual integration outage is `ATLAS_SYNC_PENDING`.

Do not merge PRs, activate production, alter equations/tolerances/routes/homotopy certification, time/rank methods, tag, or release.
