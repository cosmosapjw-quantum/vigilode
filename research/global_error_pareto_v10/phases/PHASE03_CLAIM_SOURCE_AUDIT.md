# Phase 3 — Claim/source audit

| Claim | Status | Basis |
|---|---|---|
| Cross-family equal internal tolerance is fair | UNSUPPORTED | Controllers and estimators differ by family. |
| Work-precision against a trusted reference is an accepted comparison pattern | SUPPORTED | SciML benchmark documentation and examples. |
| A WRMS threshold of one has a standard absolute/relative tolerance interpretation | SUPPORTED | PETSc weighted error norm documentation. |
| High-accuracy numerical reference is automatically exact | CONTESTED | It requires provenance and an uncertainty budget. |
| Endpoint-only error is enough | UNSUPPORTED | It can miss transient or interpolation error. |
| A single weighted total-cost score is objective | UNSUPPORTED | Weights encode user-specific economics. |
| Pareto fronts per cost metric avoid arbitrary scalarization | INFERENCE | Follows from multiobjective dominance. |
