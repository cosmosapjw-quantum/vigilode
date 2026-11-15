# Parallel-stage research checkpoint, 2026-11-15

Base: `bee5563c51264a7da19c519295bdda2fb317caef` (production unification research branch).
This unit extends the prior R6/eight-stage and Peer-W work; it does not rediscover those results or replace the current production path.

## Current-source findings and three routes

1. **Preserve the Rodas5P method with structured algebra.** The original black-box stage-task DAG has eight levels. Existing R6 proves exact finite propagation after eight simultaneous sweeps for a deterministic nonlinear strictly lower stage map with fixed invertible W; this does not need contraction. Existing affine-scan, graph-Schur and nilpotent kernels address special algebraic forms. For N^8=0, `(I-N)^-1=(I+N)(I+N^2)(I+N^4)` is exact, but computing powers through a black-box operator still consumes 1+2+4 sequential operator applications. Three written factors are not three expensive operator layers. Keep original-residual certification, expensive-operation span, setup, work, memory and full-IVP wall time separate. No new universal impossibility claim follows from the stage DAG.
2. **Parallel Peer-W as a distinct method.** `ppsw1.rs` already implements PPSW1-6b. Six solves share W and are mapped in parallel; stage-input construction and new RHS evaluations are serial. It uses a dense Jacobian; its startup uses Newton. Qualify its stiff order/controller/startup and make stage-local RHS work parallel with local counters, then evaluate matrix-free common-W methods. Parallel Peer-W and its global error control are prior art; novelty requires a new certified algorithm or a measured advance on a defined problem class.
3. **Diagonal SDC with an explicit restricted operator contract.** This unit executes a low-dimensional discriminator and derives a residual bound. It is a separate collocation iteration, not a parallel execution of the original Rodas5P tableau. It is macro-step causal but not strictly causal in internal quadrature time: an earlier node can use the previous iterate at a later node. It is admissible only if that causality distinction is allowed by the application. The two-sweep fixed-depth candidate fails the forced-stiff accuracy discriminator. Residual stopping recovers accuracy at increased depth. It remains a research comparator.

Parallel implicit extrapolation is an additional established comparison family. Its independent extrapolation rows are attractive for small/moderate stiff systems, but implicit row solvers, linearizations and total work must be charged. Published timing improvements on different hardware are not VigilODE evidence.

## Executed route: two-node diagonal SDC

Take dimensionless Radau IIA nodes `c=(1/3,1)` and

\[
Q=\begin{pmatrix}5/12&-1/12\\3/4&1/4\end{pmatrix},\quad
B=\operatorname{diag}(3/4,1/4),\quad
D_1=\operatorname{diag}(1/3,1),\quad
D_2=\operatorname{diag}(1/6,1/2).
\]

For `y'=Ly+g(t,y)`, constant L and h>0, define one sweep

\[
(I-hD_k\otimes L)U^{k+1}
=\mathbf1\otimes y_n+h(Q-D_k)\otimes L\,U^k+h(Q\otimes I)G(U^k).
\]

Start with `U^0=1⊗y_n` and alternate D1,D2. Each sweep contains two independent shifted solves and stage-local RHS evaluations. Coupling is only through the frozen previous sweep. This script uses vectorized scalar arithmetic, not threads. With a user-declared linear operator L and RHS-only g, the formula contains neither a Jacobian approximation nor Newton iteration. This is a restricted semilinear contract; setting L=0 gives an explicit iteration and cannot capture arbitrary stiffness in g.

### Exact scalar stability calculation

For g=0 and z=h lambda, two sweeps give

\[
R_2(z)=\frac{-6+5z+2z^2}{(z-3)(z-2)(z-1)}
=1+z+\tfrac12z^2+\tfrac1{12}z^3+O(z^4).
\]

Thus the finite two-sweep method has order two on the scalar linear problem. All poles lie on the positive real axis, R tends to zero at infinity, and

\[
1-|R_2(ix)|^2=
\frac{x^4(10+x^2)}{(1+x^2)(4+x^2)(9+x^2)}\ge0.
\]

By the maximum-modulus argument on left half-discs, it is A-stable; together with the zero limit it is L-stable on the scalar linear test equation. SymPy and an independent Wolfram Language evaluation verified the identities. This is a scalar stability result, not a nonlinear or uniform stiff-order theorem.

For commuting scalar split parts z=h lambda_L and w=h lambda_g, the actual two-sweep amplification is

\[
R_2(z,w)=\frac{2w^2z-3w^2+wz^2+5wz-6w+2z^2+5z-6}
{(z-3)(z-2)(z-1)}.
\]

In particular `R2(0,w)=1+w+w²/2`; unrestricted stiffness in g is excluded.

### Residual-to-algebraic-endpoint-error theorem (exact real arithmetic)

Let the physical state be finite-dimensional Hilbert space H. Assume L is self-adjoint and negative semidefinite in a **fixed** physical inner product, g(t,·) is globally Lipschitz with constant ell in that norm, and `h q ell<1`, where

\[
q=\|Q\|_B=\sqrt{(4+\sqrt7)/18}\simeq0.6076252185107651.
\]

The product norm is `||U||_B²=sum_i B_ii ||U_i||_H²`. Direct calculation gives

\[
BQ+Q^TB=\begin{pmatrix}5/8&1/8\\1/8&1/8\end{pmatrix}>0.
\]

Diagonalize the self-adjoint L in an orthonormal physical basis. Each nonpositive eigenvalue lambda reduces `A=I-hQ⊗L` to `I+xQ`, x=-h lambda>=0. The displayed weighted accretivity gives
`Re <v,(I+xQ)v>_B >= ||v||_B²`, hence by Cauchy-Schwarz `||(I+xQ)v||_B>=||v||_B`. Therefore A is invertible and `||A^-1||_B<=1`.

The collocation fixed-point map `T(U)=A^-1(1⊗y_n+h(Q⊗I)G(U))` has Lipschitz constant at most `h q ell<1`; Banach's theorem gives a unique stage root U*. For any candidate U with **original collocation residual**

\[
r=U-\mathbf1\otimes y_n-h(Q\otimes I)[(I\otimes L)U+G(U)],
\]

subtract the root equation and apply A^-1:
`||U-U*||_B <= ||r||_B+h q ell ||U-U*||_B`. Since `B_22=1/4`,

\[
\boxed{\|U_2-U^*_2\|_H\le\frac{2\|r\|_B}{1-hq\ell}.}
\]

This controls error to the collocation endpoint on one step. It does not bound the collocation truncation error, accumulated global error, spatial discretization error or floating-point roundoff. Inexact shifted solves are allowed in the algebraic theorem because their effect is included in the recomputed original residual. Production use needs certified residual/roundoff evaluation or an explicit numerical safety policy; the current binary64 script is not an interval certificate.

For a constant SPD mass matrix M and symmetric negative semidefinite K, `M y'=K y+M g` fits the theorem with physical M norm and L=M^-1K. The shifted systems are `M-h d_i K`, so the formula need not form M^-1K. This maps to conservative/self-adjoint diffusion discretizations in 2D/3D or on the sphere once their quadrature/mass inner product and boundary treatment are checked. Advection, nonsymmetric stencils, moving mass matrices and remaps require separate resolvent or logarithmic-norm bounds. Merely negative eigenvalues are insufficient: the executable negative control L=[[-1,100],[0,-1]] gives weighted inverse norm about 26.797 at h=1, disproving reuse of the unit-resolvent bound for that class.

## Numerical discriminator and decision

The manufactured problem is

\[
y'=-\kappa y+(\kappa-1)e^{-t}
+0.2[\tanh y-\tanh(e^{-t})],\qquad y(0)=1,
\]

with exact solution e^-t, t in [0,1]. Here g has global Lipschitz constant 0.2 independent of kappa. Test kappa=1 and 1000 at N=20,40,80,160,320 fixed macro steps. Fixed two sweeps are compared with stopping when the derived endpoint algebraic bound is <=1e-9 h, maximum 32 sweeps. No fallback occurs implicitly.

For kappa=1000, the fixed two-sweep endpoint error declines only from approximately 1.62e-4 at N=20 to 4.42e-5 at N=320. Residual stopping reduces it to approximately 9.30e-8 and 1.36e-10, respectively. The mean sweep count rises from 8 to 12.24 and the maximum from 8 to 13. The method therefore fails the desired reduction below eight expensive sequential rounds on this grid. This is a scoped accuracy/cost result; it does not exclude other node sets, preconditioners, problem classes or budgets.

The experiment also checks the bound against an independently iterated coupled 2x2 collocation reference at ten first-step cases, typed failure when only two sweeps are permitted, exact stability identities, and the nonnormal negative control. All raw rows and work/depth counts are in RESULTS.json. The comparison slack 5e-14 belongs only to a binary64 diagnostic assertion; it is not a production error tolerance.

**Decision:** retain diagonal SDC as a scoped research comparator. Do not route it automatically, do not advertise a speedup, and do not lower the residual budget to recover a preferred depth. The next useful discriminant is a registered work/error comparison of qualified Peer-W and preconditioned diagonal SDC on a real self-adjoint MOL operator, including shifted-solver iteration costs. First derive a practical computable residual bound for the actual operator; no spectral-only shortcut.

## Primary literature checked

- Čaklović, Lunet, Götschel, Ruprecht, *Improving Efficiency of Parallel Across the Method Spectral Deferred Corrections*, SIAM J. Sci. Comput. 47 (2025), A430–A453, DOI 10.1137/24M1649800; https://arxiv.org/abs/2403.18641 and full-text https://arxiv.org/html/2403.18641v2. Diagonal MIN-SR coefficient design and finite-sweep stability/order are prior art. Their cost model is not an empirical guarantee here.
- Utkarsh, Elrod, Ma, Rackauckas, *Parallelizing Explicit and Implicit Extrapolation Methods for Ordinary Differential Equations* (2022), https://arxiv.org/abs/2207.08135. Independent rows, shifted factorization costs and application-dependent threading gains.
- Tranquilli, Sandu, *Rosenbrock-Krylov Methods for Large Systems of Differential Equations*, https://arxiv.org/abs/1305.5481. A shared Krylov subspace is tied to method order conditions; it is not justified by merely substituting an approximate Jacobian into Rodas5P.
- Schmitt, Weiner, Podhaisky, *Parallel two-step W-methods*, ZAMM 81 S3 (2001), 749–752, https://onlinelibrary.wiley.com/doi/10.1002/zamm.200108115148. Original publisher abstract confirms stage-independent order-three/four W methods. The publisher web page's later online date is not the article year.
- Schmitt, Weiner, *Parallel Two-Step W-Methods with Peer Variables*, SIAM J. Numer. Anal. 42 (2004), 265–282, DOI 10.1137/S0036142902411057. Existing repo source lock retained; SciSpace discovery independently returned matching title/authors and independent-stage description. No fresh full-text theorem audit performed in this unit.
- Saupe et al., *Adaptive time step selection for spectral deferred correction*, Numerical Algorithms, https://link.springer.com/article/10.1007/s11075-024-01964-z. Adaptive iteration/time control and limitations of order-per-sweep assumptions; residual-based adaptivity remains distinct from method order.
- Ketcheson, Waheed, *A comparison of high-order explicit Runge–Kutta, extrapolation, and deferred correction methods in serial and parallel*, https://msp.org/camcos/2014/9-2/camcos-v9-n2-p01-s.pdf. Work/span methodology; this explicit-method paper is not evidence of stiff implicit stability.

Search access date: 2026-11-15. Exa, SciSpace and Parallel returned overlapping literature. An erroneous guessed SIAM DOI ending `1054` was queried, returned HTTP403 and was excluded; the correct existing source DOI ends `1057`. Wolfram semantic search returned no relevant theorem; only its explicit exact algebra evaluations were used. No claim of priority or mathematical novelty is adopted from these searches.
