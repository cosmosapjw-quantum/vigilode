# Peer-W error certificate across inexact solves and method transitions (PW07)

**Status:** conditional derivation complete; a general VigilODE global-error theorem remains **CONJECTURAL / UNRESOLVED**. Research only, no Rust implementation claim.

## Stage-level parallelism

For common-shift PTSW-W,
\[
Y_m=\mathbf1\otimes u_m+h_m(A_m\otimes I)K_{m-1},\quad
(I_s\otimes W_m)K_m=F(t_m\mathbf1+h_mc,Y_m)+h_m(\Gamma_m\otimes T_m)K_{m-1},\quad W_m=I-h_m\gamma T_m.
\]
The right-hand side uses committed previous-step data. If the selected tableau has no same-step off-diagonal coupling and all stages share the same shift/operator, current stage solves are independent. The next time step remains causal. A Peer label alone does not prove this dependency graph.

## Original-W residual and operator perturbation

Let \(WK=b\), while the implementation uses \(\widetilde W,\widetilde K\). Define the original-equation residual \(r=b-\widetilde W\widetilde K\). Then
\[
W(\widetilde K-K)=(W-\widetilde W)\widetilde K-r,
\quad
\|\widetilde K-K\|\le\|W^{-1}\|(\|W-\widetilde W\|\|\widetilde K\|+\|r\|).
\tag{1}
\]
A small residual against \(\widetilde W\) is not sufficient if the operator perturbation is unbounded. For solution plus new derivative history \(q_m=h_mK_m\), the stage contribution in the Euclidean product norm is bounded by
\[
\delta_m^{stage}\le h_m\sqrt{1+\|b\|_2^2}\,\|W_m^{-1}\|
(\|W_m-\widetilde W_m\|\|\widetilde K_m\|+\|r_m\|).
\tag{2}
\]
This excludes truncation, startup/history, roundoff, dense output and nonlinear-root defects; these must be added separately.

## RHS-only homotopy is a separate method identity

If \(G:D\to D\) maps a closed invariant domain into itself and is a source-attested q-contraction, \(q<1\), then
\[
\|x-x^*\|\le\frac{\|G(x)-x\|}{1-q}
\le\frac{\|\widehat G(x)-x\|+\epsilon_{eval}}{1-q}.
\tag{3}
\]
The residual must be for the original target map and include evaluation-rounding allowance. Folds/singular endpoints or absent invariant-domain witnesses are NO-GO unless a separate validated branch-tracking theorem applies. A JVP-based W operator is Jacobian-free, not derivative-free; a RHS-only implicit Peer root worker has separate order/stability.

## Conditional portfolio transition theorem

Let \(E_m\) bound the complete committed state and method history. At a committed boundary \(a\to b\), suppose the history-transfer map has validated Lipschitz gain \(\chi_m\) and exact-data transfer defect \(\eta_m\):
\[
E_m^+\le\chi_mE_m^-+\eta_m.
\tag{4}
\]
For the subsequent step suppose method b has a validated propagation gain \(\kappa_m\) in the chosen norm and total new defect \(\delta_m\). Then
\[
E_{m+1}\le\kappa_mE_m^++\delta_m=g_mE_m+d_m,\quad
g_m=\kappa_m\chi_m,\quad d_m=\kappa_m\eta_m+\delta_m.
\]
Induction gives
\[
E_N\le\left(\prod_{j=0}^{N-1}g_j\right)E_0+
\sum_{i=0}^{N-1}\left(\prod_{j=i+1}^{N-1}g_j\right)d_i.
\tag{5}
\]
The proof is repeated substitution; no commutativity is assumed. This is **conditional**, not yet a general VigilODE theorem. The missing inputs are source/tableau/domain-specific \(\kappa\), switch/history \(\chi,\eta\), outward bounds for residual/rounding/output defects, and a complete defect inventory.

## Associative certificate aggregation

Each bound update is the affine map \(E\mapsto gE+d\), represented by \(\begin{pmatrix}g&d\\0&1\end{pmatrix}\). Earlier \((g_1,d_1)\) followed by later \((g_2,d_2)\) composes as \((g_2g_1,g_2d_1+d_2)\), associative but noncommutative. An ordered Blelloch scan has O(N) work/O(log N) ideal span once valid constants exist. The scan is known prior art and does not parallelize the causal IVP trajectory.

## Nonnormal counterexample and fail-closed rule

\(A=\begin{pmatrix}1/2&1\\0&1/2\end{pmatrix}\), \(B=\begin{pmatrix}1/2&0\\1&1/2\end{pmatrix}\) each have \(\rho=1/2\), but \(\rho(BA)=(3+2\sqrt2)/4>1\). Per-step spectral radius is not a product bound. PTSW-specific finite mock fixtures also show transient product-norm amplification; they do not prove asymptotic instability.

**NO-GO:** no switch/accept if propagation, transfer, original residual, operator perturbation, roundoff or output bounds are missing.

## Prior-art boundary

Parallel Peer-W and its local/global control are established (2002/2004/2011); true scaled global-error control for an SDI Peer triple exists (2015); coupled GL solution/defect estimation exists (2017); adaptive multistep error models and variable-step Peer norm estimates also exist (2021/2026). The remaining candidate is a source-attested heterogeneous transactional switch/history-transfer certificate tied to original-equation residuals. Novelty is not established.

Sources: https://doi.org/10.1016/S0168-9274(01)00162-3 ; https://doi.org/10.1137/S0036142902411057 ; https://doi.org/10.1016/j.cam.2011.08.006 ; https://doi.org/10.1137/140979952 ; https://doi.org/10.1016/j.cam.2017.05.012 ; https://doi.org/10.1007/s11075-020-00900-1 ; https://arxiv.org/abs/2608.26913 ; https://doi.org/10.1145/31830.31844
