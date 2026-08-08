# Phase 3 — Claim/source audit

| Claim | Status | Evidence / limitation |
|---|---|---|
| Equal internal tolerances are a fair cross-family ranking rule | UNSUPPORTED | Estimators and controllers differ; benchmark practice matches external error instead. |
| External exact-reference error can be authoritative on the current manufactured corpus | SUPPORTED | Every selected problem has an analytic exact solution. |
| Endpoint error alone captures transient error | CONTESTED | It can miss interior excursions; retain max-grid and RMS-grid errors. |
| A weighted sum of RHS, Jacobian, factorization and memory is method independent | UNSUPPORTED | Weights depend on hardware and problem structure; preserve a Pareto vector. |
| Fixed-step anchors can establish a production solver ranking | UNSUPPORTED | They establish correctness and the comparison authority only. |
| Case-level Rayon parallelism may be used without changing scientific results | HYPOTHESIS | Requires exact one-/four-thread checksum equality. |
| Common-grid sampling without interpolation is possible in P0 | SUPPORTED | Use nested step ladders and a grid equal to the coarsest step. |
