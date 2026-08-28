# Codex prompt — upgrade prepared WU-05 and finish the real repair

You must first execute the exact package upgrade described in `WU05_LOCAL_REPAIR_HANDOFF.md`. You are authorized as HOST_CODEX_ORCHESTRATOR for that pinned runner only, then as WU-05 implementer after `LOCAL_WU05_AUTHORITY_READY`.

Preserve WU-00 through WU-04, fresh-review `e95ce1e…`, prepared merge `f6208a10…` / tree `19c393ca…`, and all twelve raw WU-04 files. Do not reset, rebase, stash, amend, force-update, or create a replacement worktree. Do not ask the user to manually merge the package.

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

## Exact Git materialization entry

Do not search the host filesystem for a previous semantic ZIP. Use the exact pinned package commit as the source:

```bash
git show "$K0_PACKAGE_SHA:docs/exec-plans/k0-stage-telemetry-integration-20260827/START_CONTINUATION.sh" > /tmp/k0-start-continuation.sh
chmod 700 /tmp/k0-start-continuation.sh
/tmp/k0-start-continuation.sh --repo-root /tmp/vigilode-k0-stage-telemetry.kAguIL/tree --package-sha "$K0_PACKAGE_SHA"
```

After `LOCAL_WU05_AUTHORITY_READY`, continue the actual five-finding repair immediately. Do not create another bundle-audit cycle.
