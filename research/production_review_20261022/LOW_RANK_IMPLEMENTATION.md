# Original-dimension low-rank bordered inverse

Baseline: `555e71e03fe31e6c9b21961c8e2d263b1794eb6f`. The public core module `low_rank_bordered` adds a reusable inverse/preconditioner for

\[
M=\begin{pmatrix}A+UV^T&B\\C&D\end{pmatrix},\qquad
Q=\begin{pmatrix}A&B&U\\C&D&0\\-V^T&0&I_r\end{pmatrix}.
\]

Here A is a packed n-by-n band matrix, U and V are row-major n-by-r arrays, and the matter border has dimension m. The implementation forms the border arrays of Q, delegates their factorization to the existing checked band/Schur implementation, embeds `(f,g)` as `(f,g,0)`, and returns the first n+m solution components. The public Krylov dimension remains n+m. The zero auxiliary right-hand side is rewritten on every application, including after active rank changes.

The last block equation gives z=V^T u. Substitution into the first block equation proves that the restricted solution solves the original matrix M. This is an algebraic identity, not a floating-point forward-error certificate. The existing core requires A to be strictly row diagonally dominant as a sufficient condition for no-pivot band LU; the complete updated block A+UV^T need not be diagonally dominant. Singular Schur complements are refused. Near singularity still requires original-operator residual and conditioning analysis.

For s=m+r, lower and upper bandwidth bounded by b, setup costs O(n b²+n b s+n s²+s³) and application costs O(n b+n s+s²). Storage is O(n b+n s+s²). The wrapper stores assembled border arrays in addition to the underlying factors and two augmented scratch vectors; it does not form a dense n-by-n phase block. Linear dependence on n holds with fixed b,m,r. An angular scattering representation whose global rank grows with the number of energy bins must retain that rank in this estimate.

`LowRankBorderedPreconditioner::new` checks dimensions and uses fallible reservations. A successful `refactor` or `apply` performs no allocation. A failed refactor invalidates the wrapper even if the old private factorization is still present. All application failures leave the caller output unchanged, because the complete augmented solve occurs in private scratch before output is copied. Inputs, shape, capacity, nonfinite values and arithmetic overflow are checked. The represented factors remain valid after an application error.

The `Preconditioner` interface requires `&self` and `Send + Sync`; its convenience adapter therefore protects one internal scratch workspace by a mutex. Simultaneous calls through this interface are serialized. `apply_with_workspace` accepts one caller-owned `LowRankApplyWorkspace` per parallel caller and avoids that lock. This tradeoff is explicit; it is not a claim of parallel solver speedup. Unknown exact-preconditioner identity is retained so recycled Krylov images are refreshed through the existing contract.

The change is a reusable core capability. The current specialized LRS finite-volume example is not automatically routed through it. Before such integration, use the original physical operator for residual evaluation, include setup work and allocation in telemetry, and compare at matched solution accuracy. No wall-time improvement over existing solvers is inferred from this implementation alone.

Verification sources are `low_rank/core_tests.log`, `low_rank/krylov_tests.log`, their exit files, and the corresponding new tests. Core tests compare against an independently assembled dense solve for changing n,m,r; exercise zero rank, no matter border, exact singularity, nonfinite input, overflow, capacity failure, stale-factor refusal and concurrent workspace use. A separate counting allocator tests both successful apply paths and refactor over changing dimensions. The Krylov integration test uses the true original matrix for exact and perturbed preconditioners and fixes workspace capacity to n+m rather than n+m+r.

For an identity-mass Rosenbrock stage matrix W=I−αJ, α=γh, and a Jacobian phase block Jpp=Jlocal+UJ VJᵀ, the exact input map is A=I−αJlocal, U=−αUJ, V=VJ, B=−αJpm, C=−αJmp and D=I−αJmm. Scaling both U and V by −α would produce the wrong quadratic factor α². This mapping should be introduced in a production stage factory with an explicit fallback when A fails its band factorization assumptions. A change of h or Jacobian requires refactorization for an exact inverse; reuse as an approximate preconditioner must retain true residual checks. Multiple caller-owned workspaces make concurrent inverse applications safe, but they do not remove causal dependencies between Rosenbrock stages.

Final targeted verification: **8 tests passed, 0 failed** (6 core numerical/error contracts, 1 counting-allocator contract, 1 original-dimension GMRES integration). Both Cargo commands exited 0 under Rust 1.94.1 with two build jobs and debug information disabled. The earlier two attempts were interrupted while awaiting the shared build directory lock (exit 130); their logs are preserved as environmental execution history. Source formatting checks passed. No wall-time benchmark was run for this new wrapper.
