# NEXT NODE — v2.8 staged level-2 safety probe

## Parent

Use the final v2.7 closeout commit as the only parent.

## Goal

Extend the retained actual `pexprb54s4` prefix through the next dependency level
needed to expose later nonlinear-remainder information, while still forbidding
final E-K endpoint completion and active switching.

## Why this node

v2.7 established all of the following simultaneously:

1. k=3 avoids the rejected-first-proposal causality failure on N=128/512;
2. actual level-1 prefix acquisition is cheap and resumable;
3. level-1 rho fails to certify the repeated HIRES full-E inadmissible event.

Therefore another trigger retune is lower information gain than measuring the
next actual stage remainder.

## Pre-output contract requirements

Before any v2.8 solver output:

- define exactly which pexprb dependency-level-2 states/actions are computed;
- expose raw and tolerance-scaled later-stage defects without the final endpoint;
- prove level1→level2→resume full-step parity against one-shot pexprb54s4;
- count every RHS/JVP/phi/Krylov action incrementally;
- retain R-JF as the sole committed trajectory;
- forbid controller/cache mutation by the E prefix;
- forbid full E continuation in the research campaign;
- keep k=1 as frozen negative comparator and k=3 as development-only;
- keep N=2048 sealed;
- do not tune a final safety threshold from the same campaign.

## Kill questions

1. Does level-2 telemetry separate the replicated HIRES inadmissible event from
   the two admissible HIRES events that bracket it in level-1 rho?
2. What additional vector/RHS/phi work is paid relative to level 1, target R-JF,
   and historical full E?
3. Does a staged abort rule dominate immediate full continuation on both safety
   information and tail-cost exposure?
4. Does any family acquire a new prefix failure or group-local pathology?

## Promotion rule

Even a successful v2.8 does **not** authorize active switching. It only opens a
fresh, independently calibrated safety-policy node. N=2048 remains sealed until
that policy is frozen before the holdout.
