# Bordered-band Schur route for phase-space/matter systems

## Result and scope

**Derived:** if the radiation block is banded with fixed lower/upper bandwidth and the matter border has fixed size, an admitted bordered system can be solved with linear work in the number of radiation degrees of freedom. The new implementation `rodas5p_core::bordered::BorderedBandedPreconditioner` exposes this factorization as an immutable right-preconditioner with reusable numeric storage. This is a new core implementation; the private SP03 `BandState` was inspected but not extracted or claimed as reused. Production ODE driver wiring remains a separate dependency.

Let `n` be radiation degrees of freedom, `m` matter degrees of freedom, and `K=I-gamma*h*J` a dimensionless stage matrix, with `gamma*h` having time units and `J` inverse-time units after consistent state scaling. Partition

\[
K=\begin{pmatrix}A&B\\C&D\end{pmatrix},\quad
W=A^{-1}B,\quad S=D-CW.
\]

The radiation block must be nonsingular; the new kernel additionally requires strict **row** diagonal dominance to support no-pivot band elimination. Nonsingularity of `S` is separately required; positivity or dominance of `A` alone says nothing about this border condition. Mixed physical units must be scaled before applying dimensionless residual tests. The implementation never assumes a physical model from matrix shape.

## T1: exact block solve and arithmetic reduction

Given right-hand side `(f,g)`, solve

\[
z=A^{-1}f,\quad v=S^{-1}(g-Cz),\quad u=z-Wv.
\]

Then `Au+Bv=f` and `Cu+Dv=Cz+(D-CW)v=g`; hence `(u,v)=K^{-1}(f,g)`. This derivation also proves that `K` is nonsingular exactly when `S` is, given nonsingular `A`.

For lower/upper bandwidths `l,u`, band LU uses `O(n*l*u)` arithmetic and `O(n*(l+u+1))` storage. Computing `W` uses `m` band solves, `O(n*m*(l+u+1))`. Assembling `S` uses `O(n*m^2)` and dense pivoted factorization uses `O(m^3)`. A new right-hand side costs

\[
O(n(l+u+1)+nm+m^2).
\]

The constructor stores exactly `nmax*(l+u+1) + 2*nmax*mmax + mmax^2 + nmax` f64 slots and `mmax` usize pivots, apart from object metadata. Repeated refactors/applies within capacity allocate no successful-path heap storage. They recompute factors after changed dimensions; slicing alone cannot preserve numeric factors, recycling identities, or ODE error weights.

**Consequence:** for fixed `l,u,m`, both setup and each solve are `O(n)`, compared with `O((n+m)^3)` dense factorization and `O((n+m)^2)` dense solve. This is an arithmetic/storage guarantee under the structural assumptions, not a measured wall-time crossover. In an energy-angle mesh bandwidth can scale with the angular block or mesh ordering; it must be included in the complexity rather than silently treated as constant. Dense angular scattering belongs in a suitable local block/low-rank representation, not automatically a scalar narrow band.

The implementation does not store the full dense radiation Jacobian. The new `bordered_probe` creates the full residual through independent block arithmetic and a known manufactured solution; it is a linear-algebra probe, not a BASS/reionization trajectory benchmark.

## T2: exact triangular block preconditioning

For theoretical comparison, take the exact lower triangular block factor

\[
P_L=\begin{pmatrix}A&0\\C&S\end{pmatrix}.
\]

Then

\[
P_L^{-1}K=\begin{pmatrix}I&W\\0&I\end{pmatrix}=I+N,\qquad N^2=0.
\]

The residual polynomial `p(z)=(1-z)^2` annihilates this matrix and has `p(0)=1`. Therefore unrestarted GMRES terminates in at most two iterations in exact arithmetic (and one when `W=0`). For an exact **upper** block factor `P_U=[[A,B],[0,S]]`, the corresponding right-preconditioned matrix is `K P_U^{-1}=[[I,0],[CA^{-1},I]]`, again degree two. These results require exact block inverses and exact Schur complement. The implemented full Schur inverse solves the admitted matrix directly; if used as its own preconditioner its exact-arithmetic preconditioned matrix is identity. No benefit follows from wrapping an exact solve in unnecessary Krylov iterations.

## T3: controlled approximate preconditioning

Let `M` be the admitted approximation represented by the new factorization and `K=M+Delta` the actual operator. For fixed **right** preconditioning,

\[
K M^{-1}=I+E,\qquad E=\Delta M^{-1}.
\]

If `||E||_2 <= q < 1`, ordinary GMRES with the true Euclidean residual satisfies

\[
\|r_k\|_2\le q^k\|r_0\|_2,
\]

because the admissible polynomial `(1-z)^k` gives `(-E)^k r0` and GMRES minimizes over all such polynomials. For `0<q<1` and `0<epsilon<1`, a relative residual target `epsilon` is attained within `ceil(log(epsilon)/log(q))` iterations in exact arithmetic. For `q=0` the preconditioned matrix is identity and at most one step suffices (zero steps for an already-zero residual); `epsilon>=1` already admits the initial residual. The same contraction applies cyclewise to restarted GMRES while the preconditioner remains fixed and the mathematical assumptions hold. Weighted residuals require the norm of the correspondingly scaled `E`, not the unscaled bound. Finite-precision orthogonality and applications still require actual true-residual checks.

For example, `q<=0.1` implies five iterations suffice for relative residual `1e-5`; `q<=0.01` implies three suffice for `1e-6`. Such `q` bounds must be established for the physical operator and parameters. Merely identifying an arrowhead structure establishes neither bound nor a universal 3–5-iteration claim. If `M` changes within a Krylov solve, ordinary fixed-preconditioner GMRES's proof no longer applies; use a flexible algorithm or freeze the factors for that solve.

## T4: quantitative forward-error witness

For exact row dominance margin

\[
\delta=\min_i(|a_{ii}|-\sum_{j\ne i}|a_{ij}|)>0,
\]

choose `i` where `|x_i|=||x||inf`. Then `|(Ax)_i| >= delta*||x||inf`, hence `||A^-1||inf <= mu=1/delta`. Let `beta=||B||inf`, `chi=||C||inf`, and a valid `nu>=||S^-1||inf`. The block solve yields

\[
\|K^{-1}\|_\infty\le
\max\{\mu+\mu\beta\nu(1+\chi\mu),\ \nu(1+\chi\mu)\}=H.
\]

For any approximate result `xhat`, the true residual `r=b-K*xhat` then bounds the forward error by `||xhat-x||inf <= H*||r||inf`. A rigorous computed witness requires outward-enclosed residual arithmetic and an enclosed Schur inverse bound; the present kernel does **not** supply those certificates. Its dominance admission uses upward-rounded off-diagonal row sums, while the solve is ordinary f64 and must be assessed by true residuals. Root report must keep this derived conditional certificate distinct from implementation verification.

## T5: finite-rank radiation couplings through an augmented border

For a radiation block `A+U V^T` with `U,V` of size `n*r`, introduce `z=V^T u`. The exactly equivalent enlarged system is

\[
\begin{pmatrix}A&B&U\\ C&D&0\\-V^T&0&I_r\end{pmatrix}
\begin{pmatrix}u\\v\\z\end{pmatrix}
=\begin{pmatrix}f\\g\\0\end{pmatrix}.
\]

The last block equation imposes `z=V^T u`; substitution into the first restores `(A+UV^T)u+Bv=f`, and the matter equation is unchanged. The new kernel can represent this system with the same phase block and enlarged border `m+r`, without dropping scattering or periodic-wrap entries. The complexity formulas above apply after replacing `m` by `m+r`. Linear-in-n cost holds when this total border remains bounded.

This is a useful option for a genuinely small number of global moments or cyclic wrap corrections. For energy-resolved Thomson scattering, rank two **at each energy** gives global rank `2*N_E`; it is not a fixed rank-two radiation operator. Grouping angular cells within energy blocks can instead keep these couplings local, at a bandwidth/block-size cost that must be counted. The current scalar-band API supports the resulting declared bandwidth but does not yet implement a specialized low-rank block-band factorization.

## Code contract and failure behavior

- `new(max_phase,max_matter,lower,upper)` uses checked sizes/byte bounds and fallible reservation.
- `refactor(n,m,A,B,C,D)` accepts packed band A and row-major dense borders, invalidates previous factors before checking input, checks finite values/dominance, and refuses zero/nonfinite pivots or overflow.
- `Preconditioner::apply` is immutable and `Send+Sync`, right-sided, and allocation-free on successful calls. Invalid state/shape/nonfinite input refuses before modifying output; arithmetic overflow may modify output, which must then be discarded.
- The default unknown `exact_identity()` conservatively prevents claiming recycle identity. Future driver hookup must bind matrix/mesh epoch before reusing recycle images.
- Border count `m=0` is supported. Active dimensions can shrink and grow within capacity while fixed declared bandwidth remains in force.
- Failure is a typed `CoreError`; the caller must choose the existing protected fallback rather than weakening the admission condition.

## Literature relation

The elementary block identities and polynomial proof above are reproduced here to make the assumptions executable. They are established numerical-linear-algebra mechanisms, not claimed as novel mathematical discoveries.

- Murphy, Golub and Wathen (2000), *A Note on Preconditioning for Indefinite Linear Systems*, SIAM J. Sci. Comput. 21, 1969–1972, DOI https://doi.org/10.1137/S1064827599355153. Primary full text: https://ora.ox.ac.uk/objects/uuid:36c706dd-45d0-4882-9ff3-cae603b964be/files/mdedd5b22c9f398e41fa1990b1936a2eb . Its saddle-point setting supplies related exact block-preconditioning mechanisms; the general nonzero-D border identity is explicitly derived above.
- Southworth, Sivas and Rhebergen, *On fixed-point, Krylov, and 2x2 block preconditioners for nonsymmetric problems*, https://arxiv.org/abs/1911.02664 . The exact diagonal-block assumptions and relation to Schur convergence delimit approximate-use claims.

The new contribution to this repository is a capacity-owned general bordered-band implementation plus failure, residual, variable-dimension, and allocation contracts suitable for subsequent FVM client integration.
