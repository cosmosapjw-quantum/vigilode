# Codex prompt — VigilODE K0 WU-05 bounded repair

Resume only after the orchestrator emitted:

```text
LOCAL_WU05_AUTHORITY_READY
```

Do not create, switch, merge, rebase, reset, cherry-pick, amend, or force-update branches.

Preserve without rewriting or rerunning:

```text
WU-00        183e24feb39fd7581450ae4380bd8afe09249451
WU-01        faa759de5c54848bb60d4cb8af4b06b6bcbbe514
WU-02        2badcec35b51d23fcd2938d1e15c9e0875a0f9df
WU-03        321c63ee8ca0f216001bf41b30d58c1858a4781a
WU-04        c7a5393a2cb1cf6f6095c6390348dd21fb45efe9
fresh review e95ce1e58a603306cb665a6ab91cfe02d279972f
```

Read in order:

1. `WU05_LOCAL_REPAIR_SUPPLEMENT.json`
2. `WU05_LOCAL_REPAIR_HANDOFF.md`
3. `PUBLIC_BRIDGE_CONTRACT_V2.md`
4. `evidence/EVIDENCE_V3_CANONICALIZATION.json`
5. `schemas/stage-receipt-v3.schema.json`
6. `schemas/cell-receipt-v3.schema.json`
7. `research/k0_stage_telemetry_20260827/review/fresh_review_findings.yaml`

Repair exactly the five original findings and every `SR-K0-*` supplement finding. Reproduce each finding and write RED tests before implementation.

Use only these public-at-the-language-level research bridges:

```text
rodas5p_krylov::k0_research_bridge
rodas5p_integrators::k0_research_bridge
```

Every bridge module and direct export is K0-specific and `#[doc(hidden)]`. `pub use` is forbidden. Call sites are limited by `PUBLIC_BRIDGE_CONTRACT_V2.md`.

Do not modify any Cargo manifest, `Cargo.lock`, existing production signature, ordinary production path, solver equation, tolerance, convergence authority, output, recycle transaction, semi-Jacobian-free homotopy certificate, or historical receipt.

Preserve all twelve raw WU-04 cells byte-for-byte. Build evidence-v3 wrappers mechanically from the raw cells. Do not rerun the campaign unless equations, tolerance, routing, convergence, stage work, or numerical payload bytes changed.

Required closure markers:

```text
PUBLIC_BRIDGE_SOURCE_PASS
EVIDENCE_V3_PASS
AGGREGATE_ERROR_GUARD_PASS
SIGNED_RESIDUAL_GUARD_PASS
```

After targeted and workspace tests, run exactly one read-only fresh repair review using the existing supplement review prompt, then the existing final differential audit. Repair only reproduced P0/P1 findings.

Push and open one draft stacked implementation PR only after:

- all original findings are `CLOSED`;
- all supplement findings are closed;
- no new P0/P1 remains;
- raw cell SHA-256 and numerical/stage payload digests are preserved;
- the required closure markers pass;
- the worktree is clean.

Do not merge, activate, time, rank, tag, or release.
