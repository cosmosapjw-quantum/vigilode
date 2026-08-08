# RUN_STATE.md

DATE: 2026-08-08

CURRENT_LAYER: design

TASK: Implement native fixed-step BDF1/BDF2 and Radau IIA1/3 anchors after closing the math research loop.

CURRENT_HYPOTHESIS: A common dense Newton and work ledger can express both multistep and fully implicit stage methods without weakening existing RODAS5P semantics.

REPRODUCTION_OR_ACCEPTANCE: baseline release suite passes; new anchors meet exact definitions, order/stiff/mass tests, and registry/gate contracts.

LIKELY_EDIT_LOCATIONS: `rodas5p-core/src/work.rs`; `rodas5p-integrators/src/{nonlinear,bdf,radau,native_gates,candidates,integrate,lib}.rs`; integration tests.

BLOCKERS: no common nonlinear counter fields; no dense Jacobian extraction from JVP; no multistep history type; deferred candidate execution variants.

LAST_VALIDATION: baseline background release test exited 0.

NEXT_MINIMAL_ACTION: write failing tests for common dense Newton counters and failure rollback.
