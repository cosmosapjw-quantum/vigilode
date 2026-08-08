# RUN_STATE.md

DATE: 2026-08-08

CURRENT_LAYER: closeout

TASK: Freeze the method-independent global-error / vector-cost Pareto authority for five fixed-step native anchors.

CURRENT_HYPOTHESIS: Exact-reference common-grid error plus complete vector cost prevents timestep-count and internal-tolerance comparisons from producing false solver rankings.

REPRODUCTION_OR_ACCEPTANCE: 70 canonical records, zero failures, T1/T1-rerun/T4 scientific checksum identity, calibrated repeated timing, exact reference provenance, complete exact-HEAD validation, and a clean-source rebuild.

LIKELY_EDIT_LOCATIONS: none in the frozen v0.10 branch; successor work belongs on `unified-adaptive-output-v0.11`.

BLOCKERS_TRANSFERRED: common output observer without every-step storage; adaptive grid landing; larger stiff/nonnormal/sparse corpus; non-analytic reference hierarchy; adaptive BDF/Radau.

LAST_VALIDATION: workspace and doc tests pass; exact scientific checksum `72cb6ba257685f2bd73f16976717a98eefeab6d0821851e29fa335147c8f706e`; T1/T4 scientific identity; harness validation pass; no external BLAS/LAPACK dynamic dependency.

NEXT_MINIMAL_ACTION: on `unified-adaptive-output-v0.11`, implement a common output observer with step clipping before adding variable-order BDF and adaptive Radau.
