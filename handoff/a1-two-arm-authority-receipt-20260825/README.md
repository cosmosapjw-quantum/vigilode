# A1 Two-Arm Authority Receipt Handoff

## Objective

Complete the next admissible node of VigilODE PR #18:

```text
A1-TWO-ARM-AUTHORITY-RECEIPT
```

The compile/trace closure is already GREEN at implementation head
`7952bf96bfd9fb604e87bce41bd9b918cc9b93f4`. The next task is not to redesign
GMRES or to activate the candidate. It is to generate a complete, deterministic,
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

Each cell and the aggregate must retain enough evidence to reconstruct:

- exact source/head/tree and toolchain identity;
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

## Decision classes

```text
ADMISSIBLE_AND_DISCRIMINATING
ADMISSIBLE_BUT_NONDISCRIMINATING
NOT_ADMISSIBLE
```

Only `ADMISSIBLE_AND_DISCRIMINATING` can make the candidate eligible for a
separate committed-arm switch. Do not switch automatically inside the replay
runner. The decision must be committed as evidence and independently reviewed.

## Evidence ownership

- GitHub source bytes and Actions artifacts are canonical execution evidence.
- Jira PM-4 mirrors task state and blockers.
- The canonical Confluence page mirrors the DAG and claim boundaries.
- This handoff branch is navigation/control material only and must never be merged.
