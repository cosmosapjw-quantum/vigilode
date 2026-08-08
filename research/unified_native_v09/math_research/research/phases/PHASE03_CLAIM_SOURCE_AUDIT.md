# Phase 3 — Claim–Source Audit

| Claim | Statement | Sources | Status | Auditor note |
|---|---|---|---|---|
| C-001 | A universal rigorous nonlinear root/output certificate cannot be obtained from a single residual/correction without local derivative-variation assumptions. | S-010,S-011,S-012 | SUPPORTED | The cited theorem families explicitly require interval/derivative bounds; previous finite corpus supplies a counterexample to the weaker claim. |
| C-002 | Cross-family authoritative ranking must use externally measured global error versus cost, not equal input tolerance alone. | S-001,S-002,S-003,S-013 | SUPPORTED | Official packages expose different estimators/controllers; a posteriori RK analysis separates true error from input tolerance. |
| C-003 | BDF1/BDF2 are the smallest native multistep anchors; production BDF requires history, predictor, nonlinear solve and order/step control. | S-001,S-002,S-015 | SUPPORTED | Official implementation and stability analysis. |
| C-004 | Three-stage Radau IIA is a fifth-order stiffly accurate L-stable anchor. | WOLFRAM-001,S-008,S-016 | SUPPORTED | Exact tableau/stability calculation plus established Radau lineage. |
| C-005 | ROK and BOROK belong to a large-system matrix-free capability tier, not the same internal-solver tier as RODAS5P+GMRES. | S-004,S-017 | SUPPORTED | ROK order theory co-designs projection and time discretization. |
| C-006 | Parallel SDC and Peer/W should be compared as complete integrators. | S-005,S-006 | SUPPORTED | Both define their own propagation and order/stability properties. |
| C-007 | An adjoint/output projection can reduce certification to the quantity of interest, but nonlinear remainder control remains necessary. | S-013,S-014 | PARTIALLY_SUPPORTED | Linear/output frameworks are strong; specialized nonlinear RODAS bound is an inference requiring derivation. |
| C-008 | The current homotopy family is disproved in all possible future forms. | NONE | UNSUPPORTED | Only current fixed schedules/certificates were tested; remove this claim. |

## Required corrections

- Do not call first- or second-correction diagnostics rigorous bounds.
- Do not place deferred solver names in numerical rankings before native execution.
- Do not merge ODE/mass-matrix success with index-1 DAE support.
- Do not interpret a native dense anchor as production equivalence to mature BDF/RADAU codes.
