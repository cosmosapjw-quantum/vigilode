# REAL_CLIENT_AUTHORITY_CONSTRUCTION claim ledger

## Authority

```text
base PR                  #38 OPEN / DRAFT / UNMERGED
remote base head         f954e39130e5141256731d0745666a872c0267ea
remote base tree         4314da2f9e1533737d4169526ebd2d84515ab19d
equivalent local parent  5cf4189be67458cfd998eceba39363462d59ca6d
equivalent local tree    4314da2f9e1533737d4169526ebd2d84515ab19d
implementation head      cac7d1b7337a6dff25a60072009658f6ddf155d9
implementation tree      c23abbee0d47e2dbe002e01516bf34e2481bc333
final branch/tree        RECORDED_IN_POST_COMMIT_PR_RECEIPT
draft stacked PR         RECORDED_IN_POST_COMMIT_PR_RECEIPT
remote checks            RECORDED_IN_POST_COMMIT_PR_RECEIPT
```

## Candidate claims

| Claim | Evidence required | Current disposition |
|---|---|---|
| One real nuclear-decay ODE client is exactly specified and coupled to the reusable transaction substrate. | Canonical manifest, exact binary64 fields, feature-gated `OdeProblem`, JVP, operator binding, and manifest-equality contracts. | `HOST_CANDIDATE_FREE_FINAL_PASS; LOCAL_RESULT_PENDING` |
| The two stored Bateman references have a declared L2 error upper bound of `1e-15`. | Stdlib-only exact-rational Taylor--Lagrange verifier over `Fraction.from_float` inputs; endpoint-distance squared-L2 proof. | `VERIFIED_WITHOUT_CANDIDATE_EXECUTION` |
| Frozen-W and analytic Jacobi identities are canonical for both admitted operator cases. | Domain-separated big-endian payload recomputation, exact SHA-256 equality, exact inverse-diagonal bits, candidate-free live-context W/PC basis rebinding. | `HOST_CANDIDATE_FREE_FINAL_PASS` |
| A declared reference uncertainty is consumed inside the output budget rather than added as tolerance. | Nonzero-uncertainty killing test; conservative f64 path outward-bounds candidate/reference error and `+u`, inward-bounds `B`, and checks `E_reference_upper + u <= B_lower`; estimate-only uncertainty rejects. | `P1_AND_FRESH_REVIEW_P2_REPAIRED_BEFORE_CANDIDATE_EXECUTION; HOST_FINAL_PASS` |
| The exact six local scenarios satisfy their structural and numerical contracts. | Exact published head/tree; verifier and readiness PASS; unmodified fixed runner; complete receipt, logs, and hashes; independent receipt review. | `NOT_RUN_LOCAL_EXECUTION_PENDING` |
| Same-live-context setup reuse and changed-W invalidation occur for this exact Bateman package. | Scenario receipts with exact cache transitions and complete work counters. | `NOT_OBSERVED` |
| Candidate/fallback/rejection state and work invariants hold for the Bateman package. | Nominal, strict-budget, late-failure, and terminal-rejection receipts. | `NOT_OBSERVED` |

Construction evidence is not a candidate result. A passing verifier proves the
manifest/reference/digest/preconditioner authority construction; it does not
predict or certify the transactional candidate output.

## Explicit nonclaims

| Claim | Status and reason |
|---|---|
| Bateman candidate accuracy | `UNESTABLISHED`: no hosted Bateman candidate was executed; local six-case receipt is pending. |
| General real-client accuracy | `FORBIDDEN`: one frozen Bateman client cannot establish a client class. |
| Speed or timing improvement | `FORBIDDEN`: no timing protocol or paired performance campaign exists. |
| Scalability | `FORBIDDEN`: no dimension or concurrency campaign exists. |
| Krylov basis/subspace reuse | `FORBIDDEN`: setup reuse is not basis reuse. |
| General or production preconditioner | `FORBIDDEN`: the admitted map is one exact analytic Jacobi inverse-multiply. |
| Production/default dispatcher activation | `FORBIDDEN`: the surface remains behind a non-default research feature. |
| Dense-output correctness, general event handling, whole-integration transactionality | `NOT_ESTABLISHED`: the package covers one-step research attempts only. |
| Oregonator holdout, comparator ranking, PM-7/K0 closure, merge, tag, or release | `NOT_PERFORMED_OR_AUTHORIZED`. |

## Validity separation

- `AUTHORITY_VALIDITY`: limited to the exact manifest, reference proof,
  canonical W serialization, two declared PC maps, and fixed scenario list.
- `IMPLEMENTATION_VALIDITY`: candidate-free host verification passed; immutable
  remote implementation head/tree are fixed above, while the later docs head
  and remote checks remain post-commit receipt data.
- `RESULT_VALIDITY`: absent for the Bateman six-case suite until the exact
  local runner is executed and its receipt is reviewed.
- `PROVENANCE_VALIDITY`: PR #38 remote identity, byte-identical local parent
  tree, and byte-identical remote implementation anchor are known; the final
  docs head/PR/check readback remains pending.
- `HOLDOUT_VALIDITY`: none is claimed; the holdout was not opened or run.

## Ceiling and possible later review

Current ceiling:

> `EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE`

A clean local receipt does not automatically rewrite this ledger or promote a
claim. A later independent review may admit only a bounded Bateman-specific
statement for the exact published source, authority manifest, operator cases,
budgets, and six scenarios. Every broader claim remains forbidden.
