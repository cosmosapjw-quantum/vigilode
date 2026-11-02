# R4 | Candidate constructions: ODE dynamics × graph structure × symbolic switching

## A. Ordered affine path operators

For one directed path v_0,...,v_(m-1), the path inverse action is
\[
x_0=z_0,\qquad x_i=w_i x_{i-1}+z_i.
\]
Represent vertex i by F_i=(w_i,z_i), with F_0=(0,z_0). For an earlier segment L and later segment R define
\[
L\star R=R\circ L
=(a_Ra_L,a_Rb_L+b_R).
\]
This is an associative noncommutative monoid with identity (1,0). The desired inclusive prefix at i is the second coordinate of F_0 star ... star F_i.

Candidate algorithms:
1. Sequential recurrence: O(m) work/O(m) span, no scratch allocations; best baseline for one short/medium path.
2. Existing Hillis-Steele: O(log m) rounds, O(m log m) pair compositions, plus per-stage vector allocation/copy in current C5. High span advantage, poor total work.
3. Buffered Hillis-Steele: same O(m log m) work and O(log m) rounds but reuses two buffers; separates algorithmic work from avoidable allocation.
4. Blelloch upsweep/downsweep: O(m) work/O(log m) ideal span, preserving noncommutative order; scratch O(next_power_of_two(m)). Floating-point reassociation changes rounding, so validate original W residual and a q-dependent error bound.
5. Parallel over disjoint chains, sequential inside each: O(n) total work; span is max chain length divided by available cores up to scheduling overhead. This can dominate intra-chain scans on forests with many components.

For R9's nonnegative path coefficients with q=||T_AA||_inf<1, a path recurrence residual r satisfies the exact-arithmetic estimate
\[
\|e\|_\infty\le \frac{\|r\|_\infty}{1-q},
\]
because e_i=w_i e_(i-1)+r_i and max_i |w_i|<=q. The native C5 row-margin estimate rho/m remains the authoritative source-specific screen; neither computed f64 residual is an outward-rounded enclosure.

## B. Graph / algebraic eligibility

For the original cooperative cubic source,
\[
W=D(I-T),\quad D_i=(h\gamma)^{-1}+k_i+\sum_j c_{ij}+3g_i y_i^2,\quad T_{ij}=c_{ij}/D_i.
\]
Eligibility is a semialgebraic/sign contract:
\[
(h\gamma)^{-1}>0,\ k_i>0,\ g_i\ge0,\ c_{ij}\ge0,\ c_{ii}=0,\quad
q_\infty=\max_i\sum_jT_{ij}<1,
\]
plus a source-authenticated feedback set B (|B|<=3) whose complement is a disjoint path forest. The row margin is
\[
m=\min_i\left((h\gamma)^{-1}+k_i+3g_i y_i^2\right)>0.
\]
For a low-dimensional polynomial source family, offline CAD/sign-invariant decomposition can locate determinant, pivot and margin boundaries. Runtime should use inexpensive source-bound sign/margin checks, not execute a general CAD algorithm per step.

Graph structure descriptors: number of components, path coverage, max/mean path length, feedback-core size, branch/merge count, bandwidth/profile, elimination fill estimate, and reuse count of the same W factorization across stage RHSs.

## C. Nonnormal dynamical observer

At accepted state y_n, use a sampled Jacobian J_n (or source-owned JVP operator) and compute:
- spectral abscissa alpha(J)=max Re lambda(J) when a cheap eigen solve is available;
- Euclidean logarithmic norm / numerical abscissa mu_2(J)=lambda_max((J+J^T)/2);
- nonnormality proxy nu=||J^T J-J J^T||_F/(||J||_F^2+epsilon);
- short-horizon gain estimate G_tau approx max_{v in V} ||exp(tau J)v||_2/||v||_2, using a small Krylov probe rather than dense exponential on every step;
- damping time scale from logarithmic norms and the ratio between that scale and requested h;
- graph/ordering/fill and residual-margin features.

Interpretation: alpha<0 with mu_2>0 is a transient-growth warning. It is not by itself a proof of nonlinear instability; it triggers smaller trust regions, stronger residual checks or a robust implicit route. Oscillatory low-damping dynamics are separated from strongly damped stiffness.

## D. Finite-state symbolic solver router

Alphabet of observed regimes:
- P: source-positive cooperative path/core graph eligible;
- N: nonnormal transient warning (e.g. alpha<0, mu_2>0 or large G_tau);
- D: strongly dissipative/stiff;
- O: weakly damped oscillatory;
- S: sparse irregular / high fill uncertainty;
- C: changing or reentrant regime / feature drift.

Solver modes are deliberately split by mathematical layer:
- stage backend: GRAPH_SEQ, GRAPH_PAR_SCAN, BAND_DIRECT, SPARSE_KRYLOV;
- whole-IVP method: RODAS5P, BDF, EXPRB. Switching whole-IVP methods is a different IVP method and requires history/error-state restart and same-achieved-error validation.

Router state is (current mode, last regime word, accepted-step residence counter, source/model epoch, cached workspace epoch). A mode transition requires:
1. source and epoch identity unchanged;
2. two consecutive accepted observations satisfying an enter threshold;
3. exit threshold separated by hysteresis margin;
4. minimum residence steps, except emergency residual/nonfinite failure;
5. explicit work accounting for detector, rejected attempts, discarded cache and transition startup;
6. stage-backend switch retains the same original W; any failed graph candidate falls back to the existing full-band LU on that same W.

A regular-language/dwell-time constraint over regime symbols prevents one-step chattering. For a switched numerical method, a formal stability claim would require a common or multiple Lyapunov condition for the actual error-transfer maps across allowed transitions. The control-theory symbolic-dynamics result is inspiration for constrained switching, not a plug-in numerical stability certificate.

## E. Cost selector and stop rule

Estimate per-step cost:
\[
C_G=C_{setup}+N_{rhs}(C_{path}+C_{core}+C_{resid})+C_{failed}+C_{fallback},
\]
\[
C_D=C_{symbolic}+C_{factor}(fill,ordering)+N_{rhs}C_{triangular}.
\]
Graph route is considered only when eligibility is true and a conservative lower confidence bound on expected saved work exceeds a switching reserve. Otherwise use direct. C_setup includes graph extraction, source witness and detector cost. Repeated RHSs and factorization reuse can move the crossover; small n and short path are expected direct-favorable.

This is a candidate selector contract, not implemented production routing.

## F. Representative nonnormal example

Take
\[
A=\begin{pmatrix}-1&10\\0&-2\end{pmatrix}.
\]
Its eigenvalues are -1 and -2, so the spectral abscissa is -1. The Euclidean numerical abscissa is
\[
\mu_2(A)=\lambda_{\max}\begin{pmatrix}-1&5\\5&-2\end{pmatrix}
=\frac{-3+\sqrt{101}}2>0.
\]
The propagator is
\[
e^{tA}=\begin{pmatrix}e^{-t}&10(e^{-t}-e^{-2t})\\0&e^{-2t}\end{pmatrix}.
\]
Thus the frozen linear system is asymptotically stable while the norm initially grows. This is a concrete eigenvalue-only classifier counterexample. The numerical abscissa proves positive initial growth rate, not the maximum finite-time gain; a Krylov/vector probe estimates the latter.

## Scope

R4 retains all five algorithmic candidates and the router as a research design. No stage or IVP method is promoted.
