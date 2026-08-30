# REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP

This directory records a feature-gated Audit-2 research node stacked directly
on the published matrix-free common-W substrate. The implementation remains
outside the default and production solver paths.

## Authority and recovery boundary

The exact stack base is the head of draft PR #35:

```text
head  17fcd447c1dadcea978f241ff3ba94635f9c2bd4
tree  1152e0c74235afd7ae30c3b6de6315634fa49a59
```

The recovered VigilODE thread archive used to reconstruct the roadmap has
SHA-256:

```text
c112309cab3e431ca563dd11dc1f67d95df0bfa85c8081251c33bea16ca44cfb
```

That archive is a recovery snapshot, not a complete transcript. It supports
the ordering of this node after PR #35, the requirement for reusable W setup,
transactional fallback, and later real-client validation. It does not supply a
missing exact contract or authorize a broader claim. Historical exploratory
branches are non-authoritative design input only.

Remote publication receipts are filled after immutable GitHub readback:

```text
scientific branch head  PENDING_PARENT_FILL
scientific branch tree  PENDING_PARENT_FILL
draft stacked PR        PENDING_PARENT_FILL
remote checks           PENDING_PARENT_FILL
```

## Implemented research boundary

The node adds:

- a reusable-preconditioner cache with a caller-supplied semantic frozen-W
  SHA-256 identity and a mandatory runtime `ExactOperatorIdentity` match;
- a declared preconditioner identity checked against the exact identity of the
  returned map;
- a cache-owned immutable diagonal map; the setup provider is revalidated
  before reuse and every provider apply must match the frozen map bit-for-bit
  before caller output is exposed;
- setup reuse only while the operator, semantic W identity, and
  preconditioner identity all match;
- copy-on-write pending leases with explicit commit or rollback;
- one supported reusable preconditioner family: an exact, finite,
  nonidentity Jacobi/diagonal map;
- a whole-attempt candidate using the PR #35 matrix-free common-W correction,
  followed by external-output, embedded-error, and original-target gates;
- an isolated protected sequential-JF identity-preconditioned fallback that
  starts from the immutable pre-attempt step context;
- receipts for candidate admission, candidate rejection, setup failure, late
  preconditioner-apply failure, fallback acceptance, and terminal rejection;
- immutable numerical state on rejection while all attempted work remains in a
  monotone `WorkCounters` ledger.

The PR #35 review's P2 follow-up is closed in this node: target preparation now
samples the batched RHS once, and the projected residual is constructed from
that single prepared snapshot. The prior PR #35 receipt remains an accurate
historical record of what was deferred at that earlier head; it is not rewritten
or backdated.

## Transaction semantics

The provisional diagonal preconditioner and candidate state share one
transactional disposition:

1. Validate the configuration, independent budget, external reference, and
   trial shape before setup.
2. Compute the original-target diagnostic for the unchanged trial.
3. Reuse or build an exact diagonal preconditioner under a pending cache lease.
4. Run the matrix-free candidate and retain all partial work on failure.
5. Commit the candidate state and lease only if the step and every independent
   gate accept it.
6. Otherwise roll back the lease and run the protected fallback from the
   original `StepContext`, without the provisional preconditioner.
7. Commit only an accepted fallback. If it rejects, expose no selected step and
   preserve the original state. Spent work is never rolled back.

This is attempt-level research plumbing. It is not a production controller or
an end-to-end integration transaction.

## Changed paths

```text
README.md
crates/rodas5p-integrators/src/audit2_matrix_free_research.rs
crates/rodas5p-integrators/src/audit2_reusable_transaction_research.rs
crates/rodas5p-integrators/src/lib.rs
crates/rodas5p-integrators/tests/audit2_reusable_preconditioner_transaction_contracts.rs
tools/check-audit2-readiness.sh
tools/test_a1_receipt_ci_scope.py
research/audit2_reusable_preconditioner_transactional_step_20260830/**
```

No default feature or production dispatcher is activated by these paths.

## Host contract expectations

The dedicated contract contains 13 tests. They cover exact same-binding reuse,
changed-W/preconditioner invalidation, mathematical-identity rejection despite
a false provider self-report, mutable-provider invalidation, terminal setup
failure, setup-work retention, validation before work, single-snapshot RHS
preparation, atomic candidate commit, late apply failure with isolated fallback,
nonfinite admission arithmetic, and full rejection with base-state preservation.
Some tests protect more than one of those properties.

The host verification must also retain the PR #35 6-test matrix-free suite and
the exact-base 15-test original-target bridge suite, then run the full readiness
script, clippy with `-D warnings`, formatting, and `git diff --check`. Actual
counts and hashes belong in the post-run receipt, not in this pre-run scope
statement.

## Claim ceiling

> `EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE`

Only the declared feature-gated probes and failure contracts are in scope. This
node establishes no speed, scalability, Krylov-basis reuse, production
preconditioner, production dispatcher, dense-output, general event-handling,
or real-client accuracy claim. The remaining local-only task is a frozen,
independently budgeted real-client validation described in
`CODEX_START_HERE.md`.
