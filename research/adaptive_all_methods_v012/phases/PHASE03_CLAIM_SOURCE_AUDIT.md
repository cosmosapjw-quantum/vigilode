# Phase 3 — Claim–Source Audit

| Claim | Status | Audit note |
|---|---|---|
| CVODE uses variable-step, variable-order BDF 1–5 with local-error tests and step/order selection | SUPPORTED | Official SUNDIALS mathematics documentation states this directly. |
| RADAU5 uses adaptive step control and a Gustafsson predictive controller | SUPPORTED | Directly visible in the Hairer–Wanner Fortran source. |
| RODAS5P's embedded difference can drive a native controller | SUPPORTED | Method is explicitly 5(4); exact controller constants remain implementation choices. |
| One common raw tolerance makes all method families comparable | UNSUPPORTED | Internal estimators have different semantics; v0.10/v0.11 external-error authority is required. |
| Step doubling is an efficient production estimator for all methods | CONTESTED | Method-independent and useful as an oracle, but triples nominal step work and complicates multistep history. |
| Variable-step BDF2 requires \(r<1+\sqrt2\) for all notions of stability | CONTESTED | Older zero-stability bounds exist; newer analysis proves stability on arbitrary grids under a different stability framework. A conservative ratio cap is an engineering choice, not a universal theorem. |
| First homotopy output correction is a guaranteed local-error bound | UNSUPPORTED | Prior numerical evidence includes severe underestimation. |
| Cost-aware control is relevant only to exponential methods | PARTIALLY_SUPPORTED | It is especially explicit there, but also relevant to Krylov/Newton/homotopy methods whose iteration cost changes with \(h\). |
