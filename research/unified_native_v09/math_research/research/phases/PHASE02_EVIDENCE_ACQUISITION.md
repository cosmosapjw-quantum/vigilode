# Phase 2 — Evidence Acquisition

## Literature lineage

### Production multistep lineage

SUNDIALS separates time integrators from common vector, matrix, nonlinear and linear solver abstractions. CVODE uses BDF for stiff ODEs and IDA uses BDF for DAEs. PETSc similarly exposes BDF in a common TS/SNES/KSP framework and documents selectable order plus extrapolated initial guesses. This supports building BDF anchors on a shared nonlinear-solver and work-ledger layer rather than as a standalone special case.

### Fully implicit collocation lineage

Three-stage Radau IIA is the minimal high-order, stiffly accurate, L-stable fully implicit control relevant here. PETSc's external RADAU5 wrapper is not an equal-backend comparator because it owns its dense solver and adaptivity, whereas a native implementation can share the present Rust operator, factorization and accounting layers. PETSc TSIRK demonstrates that Kronecker-structured fully implicit methods are a distinct scalable implementation path.

### Parallel-across-method lineage

Parallel SDC uses diagonal or nonstationary preconditioners to expose node concurrency; recent work constructs a nilpotent product of sweep error-propagation matrices. Peer/W methods instead build independent linear stages into the method definition. These are not drop-in internal solvers for RODAS5P and require complete-integrator comparisons.

### Large-scale Krylov/exponential lineage

Rosenbrock–Krylov co-designs order conditions with a low-dimensional Krylov approximation. BOROK changes the Krylov construction to biorthogonal short recurrences. Exponential Rosenbrock/Leja methods replace shifted linear solves by matrix-function actions. These families are capability-specific large-system candidates, not universal dense controls.

### Certification lineage

Kantorovich/Newton a posteriori bounds and Krawczyk/interval Newton methods can prove existence/uniqueness and error radii, but require inverse and derivative-variation information. Goal/output-oriented bounds use an adjoint and residual representation. No source supports a universal cheap rigorous certificate based only on one Newton correction. This directly explains the previous first-correction false-accept blocker.

## Conflict matrix

| Question | Evidence A | Evidence B / limitation | Resolution |
|---|---|---|---|
| Same input tolerance gives fair ranking? | Production solvers expose user tolerances | Estimator/controller mappings differ by family | Report same-tolerance views, but rank by external global error–cost Pareto |
| First correction estimates nonlinear root error? | Locally it is the leading Newton correction | Previous corpus shows underestimation up to hundreds; rigorous theory needs derivative drift bounds | Keep as operational diagnostic only |
| One universal solver suite? | Shared ODE/mass-matrix contract is possible | ROK/BOROK and Leja target matrix-free/semilinear regimes; DAEs need extra contracts | Use capability tiers plus a common core |
| BDF1/2 sufficient production control? | They are decisive low-order anchors | Production BDF needs variable order, step history and nonlinear forcing | Implement anchors first, then variable-order cycle |
| Radau5 can be imported as external control? | Mature RADAU5 exists | It owns a different dense/nonlinear backend | Use only as algorithmic cross-check; native Rust anchor is authoritative for fair backend comparison |

## Missing evidence

- A cheap, rigorous output certificate specialized to the RODAS lower-triangular stage Jacobian.
- Target-application derivative/Hessian bounds needed by Kantorovich/Krawczyk lanes.
- Real sparse/PDE traces that determine whether ROK/BOROK/Leja are economically relevant.
- A common residual-form index-1 DAE test suite with consistent initialization.
