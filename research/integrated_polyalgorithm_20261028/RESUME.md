# Integrated VigilODE research — 2026-10-28 KST

## Objective and boundary

Synthesize this conversation's PR90--93 into one constructive solver design,
extend the mathematics where it connects existing paths, and provide an executable
handoff for a separate PR that validates the complete cumulative implementation.
Selected physmath-research-loop 1.3.0 stages: R3 (align claims/assumptions),
R5 (constructive derivation), R9 (formal results and implementation connection),
with supporting source retrieval/checks and the requested C4/C10 handoff design.
This is not a claim that unexecuted native validation has passed.

## Source anchor

- Source PR93: 384dba091a3d9e1aac0ecddec264958298239253.
- PR92: 7eb2dd0ffda39afb2871d76700a720986ff09fe0.
- PR91: 206298b4068d923bb47c7160b111b7439098660c.
- PR90: 68a28357d960c4c2743c299770f0a33ef63245b2.
- Pre-conversation baseline: 3064d453091af1dec7f99f0c8afabd98b447ea34.
- Observed main: 8d0c79184e09efb5bdadc24a6315c60a71a44264.
- Remote survey at start found no PR after #93. All four remain open drafts/PRs;
  exact remote metadata, ancestry and CI will be recorded rather than assumed.

## Scientific invariants

Preserve original target, full-state physical error and original residual;
derivative-free first, selective explicitly supplied JVP/AD only; distinguish
matrix-free from derivative-free. Derive numerical constants, preserve old
negative results, failed CI and existing correction budgets. Keep expensive
native validation in the requested later independent PR. Checkpoints and
supporting exact computations must not become a new solver-performance claim.

## Progress

PR93 checkout restored from the remote after scratch maintenance. Primary PR
metadata and current main fetched. PR93 presently has two failed remote workflows;
their concrete causes will enter the integration validation plan.

## Next action

Dispatch disjoint derivation/assumption and cumulative-validation work, integrate
one theorem spine with a small executable validation manifest, check it once,
then publish the research package and exact-SHA handoff.
