# Next Node — v2.7 Actual First-Proposal pexprb54s4 Level-1 Prefix

Parent: v2.6 final result commit.

## Objective
At event times only, execute the **actual pexprb54s4 dependency-level-1 U2/D2 prefix** at the controller's first next proposed `(t,y,h_trial)`. Measure its real work/wall and tolerance-scaled early defect. Do not continue to level 2/3 and do not activate switching.

## Policies
- exact frozen v2.5 k=1 event decisions: comparator only;
- k=3 persistence: development candidate;
- k=2: rejected as a primary candidate, may be retained only as analysis control.

## Hard constraints
- R-JF committed trajectory bitwise/exact parity with v2.6 attempt trace;
- E full continuation count = 0;
- explicit Jacobian builds = 0, Newton iterations = 0;
- prefix starts on the first runtime proposal after the event, even when that R proposal will reject;
- full prefix work is speculative and retained in a separate ledger;
- no threshold retuning from N=512;
- N=2048 remains sealed;
- do not reuse historical G4-S4 fused-Arnoldi prefix timing as a substitute for the actual U2/D2 prefix.

## TDD requirement
Factor level 1 from `pexprb54s4_fused_step_with_telemetry_config` into a prefix object/report, prove a full step resumed from that object has exact output/work parity with the existing monolithic path, then use prefix-only mode in the research runner.
