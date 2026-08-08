# Scientific contract — homotopy numerical experiments v0.5

## User-visible goal

Run the first nonlinear, fixed-schedule partial-coupling homotopy experiments in Rust and compare
them under identical backend conditions with protected sequential RODAS5P and SABR5P.

## Preserved behavior

- Official RODAS5P tableau and coefficient transform.
- Protected sequential step and adaptive-controller semantics.
- Existing SABR5P behavior.
- External true-residual and finite-value failure rules.
- Same Rust 1.94.1 / faer backend.

## Scientific invariants

- `theta` and `lambda` are dimensionless and lie in `[0,1]`.
- `q` is an integer in `0..8`.
- Endpoint `lambda=1` is the original RODAS5P structured stage system.
- `D-C` signs match `StructuredBlockSystem::apply`.
- Original output defect, not homotopy residual, controls acceptance.
- A failed speculative path never commits external solver state.
- `q=7` is a correctness oracle, not a fast-path candidate.

## Initial validity range

Regular ODEs with finite explicit Jacobians and nonsingular common shifted operator
`W=M-h gamma J_n`. Index-1 DAE, JVP-only exact certificates, sparse/MPI/GPU performance, and
production use are outside this cycle.
