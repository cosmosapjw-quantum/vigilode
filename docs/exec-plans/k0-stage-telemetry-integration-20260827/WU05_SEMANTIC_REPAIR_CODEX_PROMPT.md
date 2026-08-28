# Codex prompt — upgrade prepared WU-05 and finish the real repair

Use the exact `K0_PACKAGE_SHA` supplied with this prompt. Do not search the host for an earlier ZIP and do not infer authority from a moving branch.

You are authorized first as `HOST_CODEX_ORCHESTRATOR` for the exact Git-materialized entry script, then as WU-05 implementer after `LOCAL_WU05_AUTHORITY_READY`.

Preserve exactly:

```text
branch        research/k0-stage-telemetry-integration-20260827
prepared      f6208a104d2f341157d900294aa30d8edb4446c0
prepared tree 19c393ca5a1ebb6c440130c9c3155e5625c85ce3
parents       e95ce1e58a603306cb665a6ab91cfe02d279972f
              c6ec0121be11f76b86afc21f8ae7a304d35c6d83
```

Keep WU-00 through WU-04, fresh review, and all twelve raw WU-04 files. Do not reset, rebase, stash, amend, force-update, or create a replacement worktree. Do not ask the user to materialize a bundle.

## Exact Git entry

```bash
set -euo pipefail
: "${K0_PACKAGE_SHA:?exact package SHA required}"
REPO=/tmp/vigilode-k0-stage-telemetry.kAguIL/tree
cd "$REPO"
test "$(git branch --show-current)" = research/k0-stage-telemetry-integration-20260827
test "$(git rev-parse HEAD)" = f6208a104d2f341157d900294aa30d8edb4446c0
test "$(git rev-parse HEAD^{tree})" = 19c393ca5a1ebb6c440130c9c3155e5625c85ce3
test -z "$(git status --porcelain=v1)"
git fetch --prune origin docs/k0-codex-execution-package-20260827
test "$(git rev-parse origin/docs/k0-codex-execution-package-20260827)" = "$K0_PACKAGE_SHA"
START=$(mktemp "${TMPDIR:-/tmp}/k0-start-continuation.XXXXXX")
trap 'rm -f -- "$START"' EXIT
git show "$K0_PACKAGE_SHA:docs/exec-plans/k0-stage-telemetry-integration-20260827/START_CONTINUATION.sh" > "$START"
chmod 700 "$START"
"$START" --repo-root "$REPO" --package-sha "$K0_PACKAGE_SHA"
```

The new package closes `WU05-NEW-P0-001` as a representation false-fail, not a scientific-data failure:

- raw top-level status and tolerance-arm labels are not authority;
- exact numerical tolerance values establish LegacyFixed;
- source SHA/tree come from the unique raw outer envelope;
- raw `error:null` is not ERROR;
- historical absent signed-residual digest remains null and is labeled `LEGACY_NOT_RECORDED`;
- never fabricate or backfill that digest;
- numerical payload identity ignores packaging labels/transport metadata and is validated by counts, gates, work, residual/audit content and canonical projection;
- raw byte SHA remains immutable provenance only.

After readiness, do not add another contract/review cycle and do not rerun WU-04 solely because wrapper bytes changed. Resume the existing five findings immediately:

```text
FR-K0-P0-001
FR-K0-P0-002
FR-K0-P1-001
FR-K0-P1-002
FR-K0-P1-003
```

Use only the two authorized doc-hidden K0 public bridges. Keep Cargo manifests, equations, tolerances, convergence authority, production signatures/routes, output semantics and homotopy certification unchanged.

Generate cell/stage v3 wrappers mechanically from the immutable raw cells. Use the repaired validator; preserve the twelve raw SHA values and their actual fields. Current source signed-residual correctness is proven by the targeted mutation test, not by inventing historical digests.

Execute actual targeted RED/GREEN tests, source-derived bridge validation, evidence-v3 validation, aggregate ERROR and signed-residual guards, affected regressions and workspace tests. Reuse one existing fresh repair review and the final differential audit only. P2/P3 remain nonblocking unless they genuinely threaten the requested result.

Only after P0=P1=0, push the implementation branch and open one draft stacked implementation PR. Update/read back GitHub, Jira PM-7 and Confluence 15499267. Use `ATLAS_SYNC_PENDING` for a real integration outage. No merge, activation, timing/ranking/speedup, tag or release.

Return the bootstrap receipt, actual prepared/final SHAs and trees, all executed commands, the twelve raw SHA preservation result, finding dispositions, tests/reviews, skipped work with reasons, and cross-plane readbacks. Do not treat the rejected Qwen candidate as evidence.

After `LOCAL_WU05_AUTHORITY_READY`, continue the actual five-finding repair immediately. Do not create another bundle-audit cycle.
