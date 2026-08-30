# REAL_CLIENT_AUTHORITY_CONSTRUCTION

This directory records the fail-closed construction of one feature-gated,
non-holdout Bateman real-client authority package above draft PR #38. It was
started after the local validation verdict
`REAL_CLIENT_AUTHORITY_UNAVAILABLE`. No Bateman transactional candidate or
six-case scientific result was executed while constructing this node.

## Source boundary

The published stack base is PR #38:

```text
remote head  f954e39130e5141256731d0745666a872c0267ea
remote tree  4314da2f9e1533737d4169526ebd2d84515ab19d
state        OPEN / DRAFT / UNMERGED
```

The construction worktree began at a locally equivalent commit:

```text
local parent  5cf4189be67458cfd998eceba39363462d59ca6d
local tree    4314da2f9e1533737d4169526ebd2d84515ab19d
```

The different parent commit IDs have the same tree. Remote publication must
rebuild the new commits with `f954e391...` as their exact parent and must use a
non-force update. Final head/tree, Draft PR URL, and check readback belong in
the immutable post-publication PR receipt because a commit cannot contain its
own identity.

## Constructed authority

The exact manifest freezes a four-state, two-timescale Bateman decay client:

\[
\dot y_0=-1000y_0,\quad \dot y_1=1000y_0,\quad
\dot y_2=-y_2,\quad \dot y_3=y_2,
\]

with binary64 initial state `(0.5, 0, 0.5, 0)`. It binds:

- nominal `h=0.001` and changed-W `h=0.0005` operator cases;
- the exact RODAS5P coefficient gamma bits;
- a domain-separated canonical frozen-W byte serialization and SHA-256 for
  each case;
- an exact finite, nonidentity analytic Jacobi inverse-multiply identity;
- analytic Bateman references with a declared L2 error upper bound of
  `1e-15`, verified using exact-rational Taylor--Lagrange enclosures;
- output, embedded-error, original-target-residual, and contraction budgets
  frozen before any candidate result;
- six fixed local scenarios with no caller-supplied replacement thresholds,
  reference, digest, preconditioner, or case order.

The authority API and contracts are behind the non-default
`audit2-bateman-authority` feature. Construction and manifest verification do
not call the transactional candidate. The local example admits only the exact
checked-in manifest, exact-verifier source, and candidate-free verification
receipt bytes, then consumes the opaque authority in one fixed suite call.

## Reference-uncertainty repair

A pre-execution review found a P1 in the PR #38 admission arithmetic. A
declared reference-error upper bound `u` had been added to the allowed budget.
The repaired pre-commit rule is:

\[
E_{\mathrm{reference}}+u\le
B_{\mathrm{absolute}}+B_{\mathrm{relative}}\lVert y_{\mathrm{reference}}\rVert_2.
\]

An estimate-only uncertainty can never categorically admit a candidate. The
receipt now separates the independent budget from the resulting true-error
upper bound. A nonzero-uncertainty killing contract distinguishes the repaired
rule from the old `E_reference <= B + u` rule.

A fresh arithmetic review then found a P2: ordinary nearest-even subtraction,
L2 accumulation, addition, and budget formation are not automatically
conservative. Before candidate execution, the admitted declared-upper-bound
path was tightened to outward-bound the candidate/reference difference and L2
norm, outward-round the uncertainty sum, and inward-round the allowed output
budget. The `output_error_upper_l2` interpretation is valid only for
`DECLARED_UPPER_BOUND` under that conservative path; estimate-only data remains
non-admitting.

## Execution boundary

The exact-binary verifier and non-executing authority contracts are eligible
for hosted verification. The six fixed candidate scenarios remain local-only
and are specified in `CODEX_START_HERE.md`. The local executor must use the
exact published implementation identity, must not use a local LLM, must not
open or run a holdout, and must not change a frozen threshold after observing a
result.

## Expected changed paths

```text
crates/rodas5p-integrators/Cargo.toml
crates/rodas5p-integrators/examples/audit2_bateman_local_six_case.rs
crates/rodas5p-integrators/src/audit2_bateman_real_client_research.rs
crates/rodas5p-integrators/src/audit2_reusable_transaction_research.rs
crates/rodas5p-integrators/src/lib.rs
crates/rodas5p-integrators/tests/audit2_real_client_authority_contracts.rs
crates/rodas5p-integrators/tests/audit2_reusable_preconditioner_transaction_contracts.rs
tools/check-audit2-readiness.sh
tools/test_a1_receipt_ci_scope.py
tools/test_audit2_bateman_local_receipt.py
tools/test_audit2_real_client_authority.py
research/audit2_real_client_authority_construction_20260830/**
```

The local-only example is
`crates/rodas5p-integrators/examples/audit2_bateman_local_six_case.rs` and is
feature-gated. No default feature, production dispatcher, protected fixture,
`rodas5p-core`, or `Cargo.lock` change is authorized.

## Claim ceiling

Until the local six-case receipt is executed and independently reviewed, the
ceiling remains:

> `EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE`

Even a clean local receipt could support only a bounded observation for the
exact Bateman client, manifest, two operator cases, six scenarios, and declared
budgets. It would not establish speedup, scalability, Krylov-basis reuse, a
general or production preconditioner, production dispatch, dense output,
general event handling, or end-to-end integration transactionality.
