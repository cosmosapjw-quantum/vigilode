# A1 Two-Arm Authority Receipt Handoff

## Objective

Complete the next admissible node of VigilODE PR #18:

```text
A1-TWO-ARM-AUTHORITY-RECEIPT
```

The compile/trace closure is already GREEN at implementation head
`7952bf96bfd9fb604e87bce41bd9b918cc9b93f4`. The next task is not to redesign
GMRES or activate the candidate. It is to generate a complete, deterministic,
read-only scientific receipt for:

```text
2 tolerance arms × 6 G4/S5B0 families = 12 cells
profile = EnforcedBudgetHoldout320
```

## Current authority boundary

```text
committed arm
legacy-fixed

candidate arm
outer-scaled-numeric-parity

frozen tau
13.39706618860016
```

The candidate shares the preserved phi tolerance **numbers** for the same outer
`rtol`. This is not a claim of equal forward/backward error, equal dimensions,
or equal outer-error contribution.

## Required receipt content

Each cell and aggregate retain enough evidence to reconstruct:

- frozen scientific execution head/tree and tested execution merge tree;
- base, toolchain, execution workflow run, and artifact content manifest;
- arm, family, profile, outer tolerance, linear tolerance, and phi tolerance;
- attempts and accepted/rejected steps;
- committed RHS/JVP/linear-matvec work;
- canonical wall-excluding trace digest;
- event keys and event counts;
- every finite `zeta34` value and signed margin `zeta34 - tau`;
- recommendation keys and counts;
- unsafe recommendation keys and count;
- audit unsafe-event keys and count;
- Hires positive-control status;
- all hard gates and limitations;
- a predeclared final classification.

## Cycle-free evidence lifecycle

```text
scientific execution head
  -> twelve-cell workflow and artifacts
  -> later receipt commit
  -> external exact-head closure
  -> fresh review
```

The committed receipt binds the earlier scientific execution identity. It does
not contain its own later commit/tree or post-receipt workflow IDs. Those
late-bound identities are recorded externally in GitHub/Atlassian and the
completion report. Any load-bearing code change after scientific execution
invalidates the artifacts and requires a new execution head.

## Decision classes

```text
ADMISSIBLE_AND_DISCRIMINATING
ADMISSIBLE_BUT_NONDISCRIMINATING
NOT_ADMISSIBLE
```

Only `ADMISSIBLE_AND_DISCRIMINATING` can make the candidate eligible for a
separate, explicitly approved activation commit. No activation occurs in this
receipt node.

## Evidence ownership

- GitHub source bytes and Actions artifacts are canonical execution evidence.
- The committed receipt is durable scientific interpretation of the frozen
  execution artifacts.
- PR checks/comments carry late-bound receipt-commit and closure-run identity.
- Jira PM-4 mirrors task state and blockers.
- The canonical Confluence page mirrors the DAG and claim boundaries.
- This handoff branch is navigation/control material only and must never be
  merged.
