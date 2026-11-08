# VigilODE unified solver portfolio: design contract

## Core decision
Use one public package/frontend, with distinct mathematically valid time integrators behind common contracts. Extend existing crates/vigilode. Do not create another facade or collapse everything into one algorithm.

## Mathematical problem interface
Start with dy/dt=f(t,y), y(t0)=y0. Future M(t,y)dy/dt=f(t,y) support is a separate capability; singular mass/DAE needs consistent initialization and a distinct qualification.
Problem owns vector space/norm, state dimension, time span/initial state, RHS and callback epoch, mass operator capability, analytic J, exact AD/user JVP, FD-JVP policy, optional f_t, matrix-free operator, preconditioner, replay safety and spatial domain descriptor.

## Jacobian and solver semantics
Analytic Jacobian is a materialized J. Exact JVP computes Jv by AD/user/analytic action without assembling J. FD-JVP approximates [f(y+sigma v)-f(y)]/sigma and is not mathematically derivative-free. Matrix-free Krylov avoids full matrix storage but commonly still needs preconditioning. JFNK is Newton outer iteration plus Krylov inner linear solves. Newton-free fixed-point/homotopy is separate and requires explicit contraction/inverse-action hypotheses.

## RODAS5P bottleneck
RODAS5P remains an 8-stage 5(4) Rosenbrock-W method with lower-triangular dependent stages. Transformations, fused kernels, cached shifted operators and matrix-free Jv can reduce work but do not remove the eight tableau stages.
Two separate workstreams: (A) optimize RODAS5P while preserving tableau/order; (B) derive new Rosenbrock-K/BOROK with Krylov subspace embedded in order-condition theory. B is a new method with new coefficients, stability and estimator requirements, not a RODAS5P refactor. C5 graph-Schur is a negative result, 2.242x slower than fast-direct in tested scope.

## Plugin lifecycle
Each plugin exposes immutable descriptor, callback-free capability query, initialize, attempt_step, commit, rollback, checkpoint/restart, optional dense segment and WorkDelta. StepAttempt records accept/reject/fail, candidate state, estimator metadata, next h, work and typed failure. Rejected step cannot mutate committed state. Dense output/events/mass/backward are optional capabilities, not forced methods.

## One frontend
Extend GeneralOde/existing facade. Concepts: Problem/Builder, MethodPreference {Auto, ExplicitOnly, StiffOnly, NamedMethod, Portfolio}, SolveOptions, tagged method config, Capabilities, SelectionRecord, Solution, Stats, WorkLedger, FailureReason, optional IntegrationSession. Preserve existing Rodas5p API.

## Routing and fallback
Hard capability filter before callbacks. Explicit method choice outranks heuristics. Auto may use tested stiffness evidence; speculative pilot requires replay-safe callback and finite budget. Switch only after accepted committed step. Invalidate DOPRI FSAL, BDF/Nordsieck history and Krylov/W/preconditioner caches on identity changes. Fallback restores last accepted state and retries same physical interval; preserve tolerances/budget and charge candidate plus fallback work.

## Spatial and parallel layers
Method-of-lines discretization creates the ODE; geometry remains in backend providers. Sphere: harmonics/spin-weighted harmonics, sphere finite volume or cubed sphere; test area measure, weighted norm, Laplace-Beltrami spectrum, chart poles/overlap. 2D/3D: anisotropic diffusion/advection/reaction, refinement, conservation and preconditioner scaling.
Parallel axes: independent IVPs, block/multivector operators, spatial SIMD/GPU/distributed vector, or a newly derived stage-parallel method. RODAS5P stages are not automatically independent. Full-IVP timing includes setup, preconditioner, orthogonalization, communication, rejected attempts and output.

## Promotion
Require source-bound order/stability/estimator evidence, capability/failure tests, independent references, matched achieved-output-error benchmarks, complete work accounting, clean clone/workspace CI and one adversarial review. Default remains HOLD.
