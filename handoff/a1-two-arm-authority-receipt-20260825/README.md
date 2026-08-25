# A1 Audit Full-E Evidence Closure Handoff

## Objective

Complete the only admissible next node of VigilODE PR #18:

```text
A1-AUDIT-FULL-E-EVIDENCE-CLOSURE
```

The previous twelve-cell run completed computationally but could not support a scientific decision because the atomic cells conflated runtime recommendation shadows with independent audit full-E evidence.

## Current authority boundary

```text
committed arm
legacy-fixed

candidate arm
outer-scaled-numeric-parity

frozen tau
13.39706618860016

invalidated run
32906175896
```

The candidate shares the preserved phi tolerance numbers for the same outer `rtol`. This remains a numerical-parameter experiment, not a claim of equal forward/backward error, equal dimensions, or equal outer-error contribution.

## Two distinct full-E channels

### Runtime shadow channel

This channel follows the recommendation policy. An unrecommended event normally has no runtime shadow execution. Its absence is expected and says nothing about safety.

### Independent audit full-E channel

This channel is read-only, arm-specific, and executed for the audit-eligible event population regardless of recommendation. It must retain completion, error, admissibility, failure, and work evidence without affecting recommendation, budget, controller, or committed runtime behavior.

## Valid classification rule

A scientific decision is forbidden until the aggregate proves audit-evidence completeness. Missing audit evidence produces `STOP_INVALID`, not `ADMISSIBLE_BUT_NONDISCRIMINATING`.

Only after complete evidence may the aggregate emit:

```text
ADMISSIBLE_AND_DISCRIMINATING
ADMISSIBLE_BUT_NONDISCRIMINATING
NOT_ADMISSIBLE
```

The ordinary committed arm remains `legacy-fixed` throughout this node. Candidate activation is a separate explicitly approved commit after a complete receipt and fresh review.

## Evidence ownership

- GitHub source bytes and Actions artifacts are canonical execution evidence.
- Jira PM-4 mirrors task state and blockers.
- The canonical Confluence page mirrors the DAG and claim boundaries.
- This handoff branch is navigation/control material only and must never be merged.
