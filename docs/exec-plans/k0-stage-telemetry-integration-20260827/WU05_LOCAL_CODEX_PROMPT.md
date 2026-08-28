# Codex prompt — VigilODE K0 WU-05 bounded local repair

Resume only the preserved WU-05 repair. Do not redo WU-00 through WU-04.

## Frozen local authority

```text
branch        research/k0-stage-telemetry-integration-20260827
WU-00         183e24feb39fd7581450ae4380bd8afe09249451
WU-01         faa759de5c54848bb60d4cb8af4b06b6bcbbe514
WU-02         2badcec35b51d23fcd2938d1e15c9e0875a0f9df
WU-03         321c63ee8ca0f216001bf41b30d58c1858a4781a
WU-04         c7a5393a2cb1cf6f6095c6390348dd21fb45efe9
fresh review  e95ce1e58a603306cb665a6ab91cfe02d279972f
review tree   e3621a370297a76907e97730ebd18c5c1e0fb83e
```

The orchestrator has already merged the exact externally pinned package SHA. Codex must not create, switch, merge, rebase, reset, cherry-pick, amend, or force-update branches.

## Required intake

Read in this order:

1. `docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_LOCAL_REPAIR_SUPPLEMENT.json`
2. `docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_LOCAL_REPAIR_HANDOFF.md`
3. `docs/exec-plans/k0-stage-telemetry-integration-20260827/PUBLIC_BRIDGE_CONTRACT_V2.md`
4. `docs/exec-plans/k0-stage-telemetry-integration-20260827/evidence/EVIDENCE_V3_CANONICALIZATION.json`
5. `docs/exec-plans/k0-stage-telemetry-integration-20260827/schemas/stage-receipt-v3.schema.json`
6. `docs/exec-plans/k0-stage-telemetry-integration-20260827/schemas/cell-receipt-v3.schema.json`
7. `research/k0_stage_telemetry_20260827/review/fresh_review_findings.yaml`

Require all pre-repair markers:

```text
PACKAGE_CONTRACT_PASS
WU05_SUPPLEMENT_MANIFEST_PASS
LEGACY_REPAIR_BLOBS_PASS
EXTERNAL_PACKAGE_PIN_PASS
WU05_SUPPLEMENT_AUTHORITY_PASS
WU05_REPAIR_MERGE_PASS
HOSTILE_FIXTURES_PASS
```

If any marker is absent, stop. Do not infer success.

The validator payload is transparent: `python tools/verify-k0-wu05-supplement.py --dump-source` emits the exact source bound by the supplement manifest.

## Repair exactly these findings

```text
FR-K0-P0-001
FR-K0-P0-002
FR-K0-P1-001
FR-K0-P1-002
FR-K0-P1-003
```

Also close the six supplement failure classes `SR-K0-*` mechanically. Do not ask user questions and do not guess across a new semantic boundary.

### API boundary

Use only:

```text
rodas5p_krylov::k0_research_bridge
rodas5p_integrators::k0_research_bridge
```

Follow `PUBLIC_BRIDGE_CONTRACT_V2.md` literally. Every module declaration and direct export is K0-specific and `#[doc(hidden)]`; `pub use` is forbidden. Do not modify any Cargo manifest, `Cargo.lock`, existing production signature, or ordinary production call path. `public_bridge_surface.json` must be generated from the actual source-derived inventory, not used as authority.

### Evidence boundary

- Raw WU-04 receipts are immutable.
- Build **cell/stage v3** wrappers mechanically from raw cells.
- Recompute raw SHA-256, canonical numerical payload SHA-256, raw stage payload SHA-256, the exact twelve named hard gates, campaign counts, and source head/tree from raw evidence.
- Never type a digest or hard-gate summary by hand.
- A failed cell preserves its actual partial stage array, count, and canonical digest.
- Do not rerun the 12-cell campaign unless equations, tolerance, routing, convergence, stage work, or numerical payload bytes changed.

### Test order

1. Preserve/reproduce the five original RED cases.
2. Add RED cases for the six supplement failures.
3. Apply the smallest bounded repair.
4. Run targeted tests.
5. Run source-derived bridge validation.
6. Build and validate evidence-v3 wrappers.
7. Execute the aggregate-error and signed-residual tests through the supplement validator.
8. Run affected regressions and workspace tests.
9. Run one read-only fresh repair review using `FRESH_REPAIR_SUPPLEMENT_REVIEW_PROMPT.md`.
10. Repair only reproduced P0/P1, then run the existing final differential audit.

Required closure markers:

```text
PUBLIC_BRIDGE_SOURCE_PASS
EVIDENCE_V3_PASS
AGGREGATE_ERROR_GUARD_PASS
SIGNED_RESIDUAL_GUARD_PASS
```

Pass also requires original findings 5/5 `CLOSED`, all supplement findings closed, no new P0/P1, immutable raw receipt SHA-256 values, and unchanged numerical/stage payload digests.

Do not push or open the stacked implementation PR before all closure conditions pass. Do not merge, activate, time, rank, tag, or release.
