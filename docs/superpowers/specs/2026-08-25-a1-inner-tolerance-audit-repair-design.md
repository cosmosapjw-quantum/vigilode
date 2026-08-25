# A1 Inner-Tolerance Audit-Repair Design

## Status

Approved repair design for PR #18 after the independent static audit of `67ec3ad77d0a88f3ff9c096b309d3a12da72b600`.

## Problem

PR #18 replaced the protected R-JF/GMRES fixed tolerances `1.0e-10 / 1.0e-12` with the numerical values already used by the exponential phi-Krylov lane:

```text
relative = max(3.0e-2 * outer_rtol, 1.0e-12)
absolute = max(3.0e-4 * outer_rtol, 1.0e-14)
```

For the frozen G4/S5B0 profiles this relaxes the protected linear solve by factors between 2,100 and 30,000. The change is scientifically load-bearing because it can alter the committed R-JF step sequence, persistence events, quadratic-drift distribution, recommendations, and the applicability of the frozen `V36_FROZEN_ZETA34_TAU` receipt.

The original PR did not contain a two-arm replay receipt, an explicit invalidation ledger, or an external frozen-reference regression. Its CI checked same-build self-consistency rather than drift against an independent fixture.

## Accepted findings

The repair accepts the following findings as blockers:

- the protected trajectory change requires a receipt and explicit claim/invalidation record;
- the frozen zeta34 threshold and downstream consumed receipts cannot be presumed valid under the new GMRES policy;
- same-build lane parity is not an external behavioral regression gate;
- fallible tolerance construction must propagate `CoreResult` rather than abort through `expect`;
- source-text scanning is not an acceptable proof that runtime lanes consume a policy;
- CI must compile and test the downstream workspace, including the frozen-full-E CLI contract.

## Qualified findings

### Numerical parity is not semantic error equivalence

The exponential lane controls a forward-error estimate for a phi action, whereas GMRES controls a residual/backward-error quantity. Their scales, dimensions, and threshold algebra differ. Sharing the same numerical coefficients therefore does **not** prove equal contribution to the outer local error.

This repair will:

- remove the phrase `one outer-error contract`;
- represent phi and linear tolerances as separate named fields;
- label the new arm `outer-scaled-numeric-parity`;
- state explicitly that it is an experimental fairness arm, not a derived forward/backward-error equivalence theorem.

Deriving an `h*gamma`-aware outer-error contribution bound is a separate numerical-analysis node and is not silently introduced here.

### Other gates and GMRES restart-cycle convergence

The analogous fixed-GMRES asymmetry in G1/G3 and the absence of within-cycle GMRES convergence remain real. They are not expanded into this bounded repair because doing so would mix a G4/S5B0 authority replay with independent solver-algorithm changes. The two-arm receipt is therefore safety/provenance evidence only and cannot authorize timing or work-efficiency claims.

## Architecture

### Explicit tolerance arms

Introduce a sealed enum:

```text
legacy-fixed
outer-scaled-numeric-parity
```

For both arms, the phi-Krylov configuration remains bitwise equal to the pre-A1 expression. Only GMRES differs:

```text
legacy-fixed:
  linear rtol = 1.0e-10
  linear atol = 1.0e-12

outer-scaled-numeric-parity:
  linear rtol = max(3.0e-2 * outer_rtol, 1.0e-12)
  linear atol = max(3.0e-4 * outer_rtol, 1.0e-14)
```

The committed production arm remains explicit in one constant. It may be changed from `legacy-fixed` to `outer-scaled-numeric-parity` only after the new arm independently satisfies the replay gates and the receipt is committed.

### Typed lane wiring

Represent the six G4/S5B0 trajectory lanes with an enum and construct a typed lane configuration by value. Every lane receives its configuration through a fallible constructor. Tests iterate the enum and compare the resulting values; no test scans Rust source text.

### Deterministic trace digest

Add a canonical SHA-256 digest for R-JF attempt/accepted traces excluding wall-clock fields. The byte encoding includes schema/version tags, identities, integer fields, booleans, optional values, and all load-bearing floating-point fields via `to_bits()`.

A focused frozen fixture must fail if the committed trace drifts. The fixture is independent of same-build comparisons.

### Two-arm replay receipt

Add a dedicated read-only runner for `EnforcedBudgetHoldout320` that accepts an explicit tolerance arm and family. It emits a deterministic summary containing:

- attempts, accepted/rejected steps and committed work counters;
- canonical R-JF trace digest;
- event keys and event count;
- finite zeta34 values and signed margins `zeta34 - tau`;
- recommendation keys and unsafe recommendation count;
- the Hires discriminating event status;
- hard-gate status and explicit limitations.

GitHub Actions runs the Cartesian product of two arms and six families, uploads each family summary, validates/aggregates all twelve outputs, and publishes one machine-readable receipt plus a human-readable comparison.

### Authority decision

The aggregate receipt classifies the new arm as one of:

- `ADMISSIBLE_AND_DISCRIMINATING`: hard gates pass, zero unsafe recommendations, and at least one unsafe full-E event remains correctly unrecommended;
- `ADMISSIBLE_BUT_NONDISCRIMINATING`: hard gates pass but the positive control disappears;
- `NOT_ADMISSIBLE`: any hard safety/provenance gate fails.

Only the first class can support switching the committed arm in this PR. The second leaves A1 experimental; the third requires reverting the production change.

## CI

The A1 workflow must run on relevant pull-request changes, relevant pushes to `main`, and manual dispatch. It must:

- run A1 unit/contract tests;
- run the frozen trace fixture;
- run the existing G4/S5B0 behavioral contracts;
- run the downstream frozen-full-E CLI contract;
- compile and test the workspace with `--all-targets`;
- run workspace Clippy with warnings denied;
- run formatting, diff, and tracked-source stability checks.

The expensive twelve-cell two-arm replay is a separate manual/explicit workflow and is not treated as a timing benchmark.

## Non-goals

- no GMRES Givens/in-cycle convergence implementation;
- no QR-complexity change;
- no G1/G3 tolerance-policy migration;
- no threshold, persistence, prefix-budget, or continuation-budget retuning;
- no timing, speedup, ranking, switching, tag, release, or inference claim;
- no claim that equal numeric tolerances imply equal outer-error contributions.

## Completion boundary

The repair is complete only when:

1. the exact head contains this design, an implementation plan, and an invalidation/claim ledger;
2. fallible typed lane wiring replaces `expect` and source-text scanning;
3. a frozen external trace fixture is green;
4. workspace/downstream CI is green;
5. a twelve-cell two-arm receipt is committed and its authority decision is explicit;
6. the PR body names superseded/conditional receipts and the remaining A2/A3/M1/M2 limitations;
7. the PR remains draft and unmerged pending fresh-context review and explicit user approval.
