# Candidate-free stage-certificate claim ledger

## Frozen input state

- Stack base: Draft PR #40 head
  `426d37ce3c0f4e5b7843b163eaf772b8e55bfa87`, tree
  `84ab302b6e7ec1318022753e9f31a669bdca4704`.
- Candidate executions in this node: exactly zero.
- Holdout access: `NOT_OPENED_OR_EXECUTED`.
- Workspace-pruned unpublished commits: not recovered and not treated as
  evidence.
- Execution authority: a new clean local Codex job at the published handoff PR
  head.

## Permitted conclusions

| Claim | Maximum disposition |
|---|---|
| The checked-in synthetic schema recomputes its stage majorant and safe accept/reject intervals. | `SYNTHETIC_SCHEMA_CONSISTENCY_ONLY` after tests pass. |
| A receipt preserves the frozen plan, trace, identities, work fields, and partial failure state. | `SYNTHETIC_SCHEMA_CONSISTENCY_ONLY` after negative and failure-path tests pass. |
| F01--F05 proof sources compile on the recorded local toolchains. | `LOCAL_FORMAL_REPLAY_VERIFIED` only if every mandatory backend exits zero and the compact receipt binds source and output hashes. |
| Formal sources agree on their declared exact algebraic identities. | `LOCAL_FORMAL_CROSS_CHECK_ONLY`; this is not empirical or production authority. |

The API and result vocabulary must use `SyntheticConsistentAccept`,
`SyntheticConsistentReject`, and `SyntheticSchemaConsistencyOnly`. It must not
use names that imply certified real-client, production, or external authority.

## Explicit nonclaims

The node does not establish or execute:

- a Bateman or other real-client candidate result;
- an Oregonator or other holdout result;
- production/default solver dispatch or production preconditioning;
- speed, amortization, scalability, or Krylov-basis reuse;
- arbitrary stale-Jacobian or approximate-W fifth-order accuracy (M09);
- field-of-values or pseudospectral GMRES convergence (M11);
- observable accuracy, dense output, events, or whole-integration
  transactionality;
- a cryptographic, remote, or third-party formal attestation;
- merge, tag, release, PM-7/K0 closure, or claim-ceiling promotion.

## Ceiling

For every local success, failure, or unavailable backend, the ceiling remains:

> `EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE`
