# A1 Inner-Tolerance Audit — Claim Scope and Invalidation Ledger

## Exact audit boundary

```text
repository: cosmosapjw-quantum/vigilode
pull request: #18
reviewed head: 67ec3ad77d0a88f3ff9c096b309d3a12da72b600
base: 4e3a75e5b2843dc1e135dcadba72edb1d09be94c
status at intake: draft / open / unmerged
```

## What changed at the reviewed head

The pre-A1 exponential phi-Krylov tolerance expression was preserved bitwise. The protected R-JF/GMRES lane changed from fixed `rtol=1.0e-10, atol=1.0e-12` to the same **numerical values** as the phi lane. Across the frozen profiles this relaxes GMRES by factors from 2,100 to 30,000.

This is not classified as a refactor. It is a protected trajectory-generator change.

## Historical artifacts affected

The following artifacts remain immutable evidence for the exact code identities under which they were generated, but they are **conditional and non-transplantable** to a different protected GMRES tolerance arm until replay:

- `research/generic_enforced_prefix_budget_v35/results/RESULT_SUMMARY.json`
  - `events=29`
  - `recommendations=13`
  - `unsafe_recommendations=0`
  - `audit_unsafe_events=1`
  - Hires positive control: `zeta34=14.320053508327359`
- `research/generic_enforced_prefix_budget_v35/results/CONSUMED_REPLAY_SUMMARY.json`
- `research/generic_enforced_prefix_budget_v35/results/consumed_replay/**`
- `research/generic_enforced_prefix_budget_v35/results/fresh_holdout320/**`
- downstream v3.7 continuation/timing authority receipts that consume the frozen event distribution;
- CLI contracts that assert `V36_FROZEN_ZETA34_TAU=13.39706618860016` without independently replaying the changed trajectory generator.

No historical file is deleted or rewritten. The issue is applicability to the current code, not falsification of the legacy run.

## Frozen threshold status

`V36_FROZEN_ZETA34_TAU` is not retuned in this repair. Its numerical value remains frozen. What is under audit is the distribution to which it is applied.

Before the two-arm replay, the following statements are **withheld** for the outer-scaled GMRES arm:

- the event set remains 29;
- the recommendation set remains 13;
- zero unsafe recommendations still holds;
- the Hires positive control remains above tau and unrecommended;
- consumed v3.5/v3.7 economics or timing receipts remain applicable;
- the new arm preserves the prior protected committed trace.

## Allowed pre-replay claim

Only this statement is allowed:

> PR #18 defines an experimental G4/S5B0 GMRES arm whose relative and absolute tolerance numbers equal the preserved phi-Krylov tolerance numbers for the same outer `rtol`.

This is numerical parameter parity. It is not a proof of equal forward/backward error, equal dimensions, equal outer-error contribution, equal work, equal timing, or unchanged scientific classification.

## Mandatory replay evidence

The exact `EnforcedBudgetHoldout320` two-arm replay must report, by family and in aggregate:

- attempts, accepted/rejected steps, and committed work;
- canonical trace digest excluding wall time;
- event keys and counts;
- all finite zeta34 values and signed margins from tau;
- recommendation keys and counts;
- unsafe recommendation count;
- Hires positive-control status;
- hard-gate status and limitations.

The two arms are:

```text
legacy-fixed
outer-scaled-numeric-parity
```

## Predeclared decision rule

- `ADMISSIBLE_AND_DISCRIMINATING`: all hard gates pass, zero unsafe recommendations, and at least one unsafe completed full-E event is correctly unrecommended.
- `ADMISSIBLE_BUT_NONDISCRIMINATING`: hard gates pass and unsafe recommendations are zero, but the positive control disappears.
- `NOT_ADMISSIBLE`: any hard safety/provenance gate fails or an unsafe recommendation appears.

The committed arm may change to `outer-scaled-numeric-parity` in PR #18 only under `ADMISSIBLE_AND_DISCRIMINATING`. No threshold or persistence retuning is allowed to obtain that class.

## Deferred nodes

The following findings are accepted as real but deliberately kept outside this bounded repair:

1. **Semantic inner-error contract:** derive and validate a common outer-error contribution bound rather than equating raw forward-error and residual tolerances.
2. **G1/G3 scope:** audit and repair analogous fixed-GMRES asymmetries in other comparator gates.
3. **A2/A3:** add within-cycle GMRES convergence and incremental Givens updates; current restart-cycle behavior limits work/timing resolution.
4. **Performance protocol:** warmup, sample duration, repetition count, `black_box`, codegen profile, and rank-decision rules remain separate blockers for timing authority.

## Current integration state

```text
A1 production authority: BLOCKED_PENDING_TWO_ARM_RECEIPT
wall-time/ranking authority: NOT_AUTHORIZED
active switching: NOT_AUTHORIZED
merge: NOT_AUTHORIZED
release/tag: NOT_AUTHORIZED
```
