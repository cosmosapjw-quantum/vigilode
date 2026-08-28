# Codex prompt — resume VigilODE K0 at WU-05 only

The orchestrator has already completed the exact package merge. Do not create, switch, merge, rebase, reset, cherry-pick, amend, or force-update branches.

Preserve these completed commits without rewriting or rerunning them:

```text
WU-00        183e24feb39fd7581450ae4380bd8afe09249451
WU-01        faa759de5c54848bb60d4cb8af4b06b6bcbbe514
WU-02        2badcec35b51d23fcd2938d1e15c9e0875a0f9df
WU-03        321c63ee8ca0f216001bf41b30d58c1858a4781a
WU-04        c7a5393a2cb1cf6f6095c6390348dd21fb45efe9
fresh review e95ce1e58a603306cb665a6ab91cfe02d279972f
```

Before mutation, require the orchestrator output marker:

```text
LOCAL_WU05_AUTHORITY_READY
```

Then read in order:

1. `docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_LOCAL_REPAIR_SUPPLEMENT.json`
2. `docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_LOCAL_REPAIR_HANDOFF.md`
3. `docs/exec-plans/k0-stage-telemetry-integration-20260827/PUBLIC_BRIDGE_CONTRACT_V2.md`
4. `docs/exec-plans/k0-stage-telemetry-integration-20260827/evidence/EVIDENCE_V3_CANONICALIZATION.json`
5. `docs/exec-plans/k0-stage-telemetry-integration-20260827/schemas/stage-receipt-v3.schema.json`
6. `docs/exec-plans/k0-stage-telemetry-integration-20260827/schemas/cell-receipt-v3.schema.json`
7. `research/k0_stage_telemetry_20260827/review/fresh_review_findings.yaml`

Repair exactly the five original findings and the supplement failure classes. Write RED reproducers before implementation. Use only the two authorized `#[doc(hidden)]` research bridge modules:

```text
rodas5p_krylov::k0_research_bridge
rodas5p_integrators::k0_research_bridge
```

Do not modify any Cargo manifest, `Cargo.lock`, existing production signature, production route, equation, tolerance, convergence authority, output, recycle transaction, homotopy certificate, or historical receipt.

Preserve all twelve raw WU-04 receipts byte-for-byte. Generate evidence-v3 wrappers mechanically. Do not rerun the campaign unless solver semantics or numerical payload bytes change.

Required closure markers:

```text
PUBLIC_BRIDGE_SOURCE_PASS
EVIDENCE_V3_PASS
AGGREGATE_ERROR_GUARD_PASS
SIGNED_RESIDUAL_GUARD_PASS
```

Then run one read-only fresh repair review and the existing final differential audit. Push/open one draft stacked implementation PR only when all original and supplement P0/P1 findings are closed and no new P0/P1 exists.

Do not merge, activate, time, rank, tag, or release.
