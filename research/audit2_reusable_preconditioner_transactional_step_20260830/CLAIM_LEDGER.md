# REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP claim ledger

## Authority

```text
stacked base head  17fcd447c1dadcea978f241ff3ba94635f9c2bd4
stacked base tree  1152e0c74235afd7ae30c3b6de6315634fa49a59
base PR            #35 OPEN / DRAFT / UNMERGED
archive SHA-256    c112309cab3e431ca563dd11dc1f67d95df0bfa85c8081251c33bea16ca44cfb
archive status     recovered snapshot; incomplete transcript
final head         PENDING_PARENT_FILL
final tree         PENDING_PARENT_FILL
draft stacked PR   PENDING_PARENT_FILL
remote checks      PENDING_PARENT_FILL
```

The archive constrains roadmap continuity but is not executable evidence. PR
#35 and its receipts remain the authority for the matrix-free substrate at
`17fcd447`; this node does not retroactively change their observations.

## Candidate claims

| Claim | Required evidence | Disposition |
|---|---|---|
| A preconditioner setup can be reused only for the same frozen W and declared map. | Semantic W SHA-256 and runtime exact operator identity both match; provider/revision/configuration and exact inverse-diagonal bits match; changed bindings or a drifted provider rebuild. | `IMPLEMENTED_IN_DECLARED_CONTRACTS; HOST_RECEIPT_PENDING` |
| The reusable map used by the matrix-free candidate is exact, diagonal, finite, and nonidentity. | Setup classifies the returned bits rather than trusting `is_identity`; the cache owns the frozen diagonal and exposes provider output only after bitwise map verification. | `IMPLEMENTED_IN_DECLARED_CONTRACTS; HOST_RECEIPT_PENDING` |
| Candidate state and its pending preconditioner lease commit atomically. | The admitted-candidate contract requires all external-output, embedded, original-target, and outer-step gates before state plus cache commit. | `IMPLEMENTED_IN_DECLARED_CONTRACTS; HOST_RECEIPT_PENDING` |
| Candidate rejection or late preconditioner failure cannot contaminate the protected fallback. | Pending lease rolls back; fallback uses the unchanged `StepContext` and its own identity-preconditioned sequential-JF path; partial candidate work remains visible. | `IMPLEMENTED_IN_DECLARED_CONTRACTS; HOST_RECEIPT_PENDING` |
| Terminal rejection preserves numerical state without erasing work. | No selected step is exposed; committed state equals the base state; solve, failure, fallback, and rejection counters remain monotone. | `IMPLEMENTED_IN_DECLARED_CONTRACTS; HOST_RECEIPT_PENDING` |
| Failed setup preserves the prior committed binding and partial setup work. | No pending lease remains; setup attempt/failure and work snapshot are retained; the old binding can still be reused. | `IMPLEMENTED_IN_DECLARED_CONTRACTS; HOST_RECEIPT_PENDING` |
| The prior P2 duplicate snapshot/RHS preparation is closed. | A stateful batched-RHS contract permits exactly one sample; the residual is derived from the same prepared RHS used for stage linearization. | `IMPLEMENTED_IN_THIS_NODE; PR35_DEFERRAL_REMAINS_HISTORICAL` |

None of these claims becomes host-verified until the actual command receipt and
immutable source identities replace the placeholders in this directory.

## Explicit nonclaims

| Claim | Status and reason |
|---|---|
| Speedup or timing improvement | `FORBIDDEN`: no timing protocol, paired baseline, or noise model is in this node. |
| Scalability | `FORBIDDEN`: only small declared contracts exist; no dimension or concurrency campaign was run. |
| Krylov basis/subspace reuse | `FORBIDDEN`: setup/workspace reuse is not basis reuse; every RHS still performs a fresh solve. |
| General or production preconditioner | `FORBIDDEN`: only an exact diagonal nonidentity map is admitted by this substrate. |
| Production/default dispatcher activation | `FORBIDDEN`: the API remains behind `audit2-research` and is not wired into the default solver. |
| Real-client or original observable accuracy | `UNESTABLISHED`: manufactured references exercise admission wiring but do not substitute for a predeclared real-client budget and uncertainty authority. |
| End-to-end whole-integration transaction | `UNESTABLISHED`: the transaction covers one prepared attempt, not controller history, restart state, or a complete integration. |
| Dense output correctness | `FORBIDDEN`: candidate/fallback selection does not validate or commit a dense-output object. |
| General event handling | `FORBIDDEN`: no root location, event restart, simultaneous event, or discontinuity contract is included. |
| Holdout, comparator ranking, PM-7/K0 closure, merge, tag, or release | `NOT_PERFORMED`. |

## Validity separation

- `RESULT_VALIDITY`: limited to the dedicated contracts and whatever exact host
  replay is recorded in `evidence/VERIFICATION.md`.
- `PROVENANCE_VALIDITY`: exact stacking on PR #35 plus final GitHub head/tree
  readback; remote values are pending until publication.
- `PACKAGING_VALIDITY`: this directory is a source-bound handoff. The recovered
  archive is checksum-bound context but incomplete and non-executable.
- `REAL_CLIENT_VALIDITY`: absent until the local-only protocol in
  `CODEX_START_HERE.md` is completed with a frozen client, independent
  reference, declared uncertainty, and precommitted budgets.

## Deferred work

- Execute the local-only real-client protocol without modifying remote state.
- Decide a scientifically justified production preconditioner family only in a
  later node.
- Extend transactionality to controller/restart/dense-output/event state only
  under separate contracts.
- Preserve PR #35's P3 stale historical test-count note as history; do not edit
  old receipts merely to normalize wording.

Claim ceiling:

> `EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE`

This ceiling describes a narrow reusable exact-diagonal setup and one
candidate/fallback attempt transaction. It does not admit performance,
production, generality, or real-client accuracy.
