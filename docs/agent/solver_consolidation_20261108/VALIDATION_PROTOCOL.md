# Validation and production qualification protocol

Evidence has independent axes: claim status (ESTABLISHED, LITERATURE_SUPPORTED, DERIVED, NUMERICALLY_CHECKED, IMPLEMENTATION_VERIFIED, CONJECTURAL, UNRESOLVED, BLOCKED), execution status (PASS, FAIL, CONCERN, NOT_RUN, NOT_APPLICABLE, BLOCKED), and failure class (THEORY, NUMERICAL, IMPLEMENTATION, RUNTIME_ENVIRONMENT, PROVENANCE, AUTHORITY, UNKNOWN).

## Source and lineage
Pin repository, branch, commit, tree, toolchain and blobs. Verify every Git reference; never recreate missing SHAs. Keep the original 23-node DAG immutable. Distinguish public APIs from internal modules and preserve the scope of historic receipts.

## Mathematical and numerical conformance
For each method record exact equations/tableau and coefficients, then trace equation -> callback -> operator -> linear/nonlinear solve -> estimator -> acceptance -> metric. Test order on exact nonautonomous IVPs, stiff decay, Prothero-Robinson order reduction, nonnormal transients, diffusion/reaction MOL systems, rejection and step underflow. Verify mass/DAE only within declared index and consistent-initialization scope. Test dense output independently.
RODAS5P: preserve eight-stage 5(4) identities. Fewer stages require a different method and proof.
BDF: verify variable-step/order/history startup, nonlinear solve, Jacobian refresh, estimator and restart. Internal BDF1/2 remains reference-only.
Radau: verify collocation stage equations, stiff accuracy, estimator, dense extension and coupled linear solves. Internal Radau IIA1/3 remains reference-only.
Rosenbrock-K: verify Krylov subspace dimension and order-condition theorem, stability, basis construction, orthogonalization, preconditioner and full-stage work. A fixed basis does not imply universal stability.

## Derivatives, operators and callbacks
Compare JVP with independently assembled J on small systems; test finite-difference scale/cancellation; AD/user JVP correctness and replay; preconditioner setup/apply/refresh and injected failure; no hidden dense materialization; row apply versus scalar apply; refuse speculative replay for effectful callbacks.

## Router and fallback
Capability query must not call user code. Exclude reference-only methods from Auto. Inject RHS/JVP/preconditioner/Krylov/interpolation/event failures. Verify rollback to last accepted state, all work retained, tolerances unchanged, cache/history invalidation on identity changes, and no switching during a trial step.

## Spatial geometry
Sphere: constant/harmonic modes, area-weighted norm, Laplace-Beltrami spectrum, pole/chart behavior and refinement. 2D/3D: anisotropic diffusion/advection/reaction, mesh refinement, conservation and size scaling. Record unknowns, sparsity, spectrum, memory, preconditioner setup and iterations.

## CI blockers found in the audited PR head
1. allocation_contracts shifted_operator_apply: 2 allocations / 144 bytes, expected zero.
2. clippy -D warnings: items-after-test-module and other lints in n01_mass_action_owner.rs and source_owned_graph_schur.rs.
3. CVODE bridge compile: missing mpi.h in runner.
4. authority-ref checker: 19 missing commit references (plus other category counts in PLAN_BUNDLE.json).
5. Other failed research/ignored jobs remain to be individually classified.

Run clean clone, locked metadata, workspace all-target compile/tests, ignored tests, clippy, docs, examples, benchmark build and provenance checker. Retain raw logs and source identity. One independent reviewer receives the frozen packet; allow at most one bounded correction cycle. No review-of-review.