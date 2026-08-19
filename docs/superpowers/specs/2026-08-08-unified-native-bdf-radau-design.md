# Unified Native BDF/Radau Anchors Design Specification

Date: 2026-08-08

## Goal

Add mathematically exact fixed-step BDF1/BDF2 and Radau IIA one-/three-stage anchors to the existing pure-Rust candidate harness, using one transparent dense Newton layer and preserving every existing RODAS5P/SABR/homotopy behavior.

## Architecture

### Common nonlinear layer

`nonlinear.rs` owns a small dense full-Newton reference solver. It receives residual and Jacobian closures, uses `DenseMatrix`/`LuFactorization`, checks finite values and an external scaled residual, records all nonlinear work, and commits no external state. It is a correctness anchor, not a production JFNK implementation.

### Jacobian materialization

`OdeProblem::dense_jacobian` returns the explicit Jacobian when available or builds it column-by-column from JVPs. This makes the anchor work for the existing problem abstraction while charging every JVP vector.

### BDF module

`bdf.rs` defines `BdfOrder::{One,Two}`, `BdfHistory`, and fixed-step `bdf_step`. BDF2 requires the previous state; the trajectory driver starts with BDF1 and reports it. The nonlinear residual is written in mass-matrix form and uses a method-specific extrapolation predictor.

### Radau module

`radau.rs` contains exact Radau IIA coefficients. `RadauIiaStages::One` cross-checks backward Euler. `Three` solves the coupled stage-increment system with a `3n x 3n` Newton Jacobian. The last-stage/stiff-accuracy identity is tested exactly.

### Candidate and gate integration

The catalog adds executable `bdf1-fixed`, `bdf2-fixed`, `radau-iia1-fixed`, and `radau-iia3-fixed`, while keeping production variable-order/adaptive entries deferred. A separate `native_gates.rs` evaluates global order, scalar stiff damping, mass-matrix accuracy and total work. These candidates are not inserted into the RODAS-stage one-step certificate screen because they solve different complete-integrator equations.

## Error handling

- Newton returns typed nonconvergence, nonfinite and singular-solve errors.
- BDF history is copied before a step and committed only after success.
- Radau stage state is local until success.
- A deferred or unsupported candidate reports `NOT_APPLICABLE`/deferred, never a numerical failure.

## Testing

Tests are written first for Newton, then BDF, then Radau, then registry/gates. Exact symbolic data are duplicated only in tests/constant definitions and checked by identities rather than decimal snapshots alone.

## Scientific limitations

This cycle does not implement error estimators or adaptive controllers for BDF/Radau. Fixed-step wall time is not a production-solver ranking. DAE support is excluded beyond nonsingular mass matrices.
