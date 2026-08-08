# RUN_STATE.md

DATE: 2026-08-08

CURRENT_LAYER: validate

TASK: Build and validate a method-independent global-error / vector-cost Pareto authority for five fixed-step native anchors.

CURRENT_HYPOTHESIS: Exact-reference common-grid error plus complete vector cost prevents timestep-count and internal-tolerance comparisons from producing false solver rankings.

REPRODUCTION_OR_ACCEPTANCE: 70 canonical records, zero failures, T1/T1-rerun/T4 scientific checksum identity, repeated calibrated timing, exact reference provenance, full validation.

LIKELY_EDIT_LOCATIONS: `rodas5p-fair-ab::global_error`, CLI command/tests, research closeout artifacts.

BLOCKERS: common output observer without every-step storage; adaptive grid landing; larger corpus; non-analytic reference hierarchy.

LAST_VALIDATION: targeted fair-ab and CLI tests pass; canonical scientific checksum `72cb6ba257685f2bd73f16976717a98eefeab6d0821851e29fa335147c8f706e`.

NEXT_MINIMAL_ACTION: run exact-HEAD full validation and package/tag the cycle.
