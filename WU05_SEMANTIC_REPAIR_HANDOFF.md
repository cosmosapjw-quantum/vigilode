# VigilODE K0 WU-05 semantic continuation handoff

## Current preserved local authority

```text
branch   research/k0-stage-telemetry-integration-20260827
HEAD     f6208a104d2f341157d900294aa30d8edb4446c0
tree     19c393ca5a1ebb6c440130c9c3155e5625c85ce3
parents  e95ce1e58a603306cb665a6ab91cfe02d279972f
         c6ec0121be11f76b86afc21f8ae7a304d35c6d83
clean    true
```

The earlier preparation and all seven pre-repair markers succeeded. Do not replay that preparation, reset this merge, or replace it with the remote prepared branch.

## Why the new stop was valid

`WU05-NEW-P0-001` is a validator/schema incompatibility with preserved historical raw evidence:

- raw has no required top-level `status` or `tolerance_arm` labels;
- source head/tree are in the outer raw envelope, not the discovered receipt mapping;
- historical COMPLETE stages did not record signed-residual digests;
- `error: null` means no error;
- the old numerical digest mixed representation labels with scientific content.

Inventing any missing value is forbidden. The 12 raw files and their SHA-256 values remain immutable.

## Controlling repair

This package adds a semantic continuation rather than altering the historical package in place.

- Exact raw SHA-256 remains provenance authority.
- Numerical identity is a canonical projection of exact tolerance values, arm/family, attempt/step/work counters, trace, events/recommendations, hard gates, audit outcomes, and stage scientific/work fields.
- Source commit/tree and wrapper/archive metadata are excluded from the numerical digest but remain provenance.
- Missing historical signed-residual telemetry migrates as `null` and `LEGACY_NOT_RECORDED`.
- Current sign correctness remains covered by the required vector-aware mutation test.
- No WU-04 rerun is authorized by this representation-only repair.

Read:

1. `docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_SEMANTIC_CONTINUATION_AUTHORITY.json`
2. `docs/exec-plans/k0-stage-telemetry-integration-20260827/policy/BYTE_VS_SEMANTIC_IDENTITY.md`
3. `docs/exec-plans/k0-stage-telemetry-integration-20260827/policy/EXECUTION_TRANSITION_ANTI_BUREAUCRACY.md`
4. `docs/exec-plans/k0-stage-telemetry-integration-20260827/evidence/EVIDENCE_V3_SEMANTIC_AUTHORITY.json`
5. the two `*-semantic.schema.json` files
6. `CODEX_START_HERE.md`

## Exact continuation

Use the exact new package SHA supplied externally. Extract `START_CONTINUATION.sh` with `git show`; no downloaded ZIP is required.

The script accepts only the exact clean `f6208a10...` prepared commit or its exact `[f6208a10..., new-package]` merge. It validates the new semantic files in a detached worktree, rejects any package delta outside the named control files, creates one ordered upgrade merge, and is idempotent. It never resets, rebases, stashes, amends, force-updates, or resolves a conflict.

Required output:

```text
EVIDENCE_V3_PASS
LOCAL_WU05_AUTHORITY_READY
CONTINUATION_RECEIPT=...
```

## After readiness

Resume the existing WU-05 repair immediately. Do not create another review layer.

1. Preserve/reproduce the five original findings.
2. Use the new semantic validator to derive the 12 wrappers from raw evidence.
3. Record all 12 exact raw SHA-256 values separately from numerical payload digests.
4. Close the aggregate structured-error and public-bridge findings.
5. Execute the current vector-aware signed-residual mutation test.
6. Run the existing single fresh repair review and final differential audit.
7. Publish only a draft stacked implementation PR after P0/P1 closure.
8. Synchronize/read back GitHub, Jira PM-7, and Confluence 15499267.

Do not rerun the campaign unless a distinct equation, tolerance, route, convergence, work, or numerical-output change is demonstrated. Do not merge, activate, time, rank, tag, or release.
