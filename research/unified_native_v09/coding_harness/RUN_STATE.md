# RUN_STATE.md

DATE: 2026-08-08

CURRENT_LAYER: closeout

TASK: Native fixed-step BDF1/BDF2 and Radau IIA1/3 anchors after sequential math and coding research loops.

CURRENT_HYPOTHESIS: Complete-integrator methods require their own scientific gate and an external global-error/total-cost comparison authority; they must not be ranked inside the RODAS stage-equation screen.

REPRODUCTION_OR_ACCEPTANCE: 82 release tests pass; four native anchors pass order/stiff/mass gates; repeated native JSON is byte-identical; unified 1/4-thread scientific checksum matches.

BLOCKERS: production variable-step/adaptive implementations and matched-global-error Pareto harness remain deferred.

LAST_VALIDATION: full release workspace tests exit 0; affected strict Clippy and release build pass.

NEXT_MINIMAL_ACTION: implement method-independent global-error target and total-cost report before adding variable-order BDF.
