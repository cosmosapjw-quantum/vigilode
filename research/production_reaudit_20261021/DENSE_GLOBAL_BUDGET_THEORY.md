# Constructive global accuracy and work allocation

This note adds conditional, implementable results to T4 using the standard differential-inequality and Hölder arguments; it makes no priority claim. Related primary residual/global-control work is Lang and Verwer, residual-based global error control, DOI https://doi.org/10.1137/050646950 (bibliographic details and retrieved source are recorded by the root audit). It does not interpret the midpoint estimator as a rigorous continuous residual bound. All norms below are one fixed norm (for WRMS, its positive scale vector is frozen over the claimed interval). The facade's time-varying tolerance weights require explicit norm conversion before this theorem applies.

## G1: horizon-weighted defect budget

Let `u` be the actual returned absolutely continuous curve on `[0,T]`, with initial error at most `r0`. Assume a tube containing `u` and the true solution, a one-sided Lipschitz bound `mu(t)`, and an integrable *verified* bound `||u'-f(t,u)|| <= delta(t)`. Define

\[
A(s)=\exp\!\left(\int_s^T\max\{\mu(v),0\}\,dv\right).
\]

If

\[
A(0)r_0+\int_0^T A(s)\delta(s)\,ds\le\varepsilon,
\]

then `||y(t)-u(t)|| <= epsilon` for **every** `t in [0,T]`.

**Proof.** The already-audited T4 comparison inequality bounds the error at `t` by `exp(integral_0^t mu) r0 + integral_0^t exp(integral_s^t mu) delta(s) ds`. For every `s <= t <= T`, its propagation factor is at most `A(s)`: replacing `mu` by its positive part increases the exponent, and extending the interval to `T` increases it again. Extending a nonnegative integral from `[0,t]` to `[0,T]` proves the result. Tube validity is established by T4's same first-exit argument, with an outward-rounded strict margin when needed. ∎

**Implementable online allocation.** Let `B=epsilon-A(0)r0 > 0`, and choose any nonnegative integrable density `w` with `integral_0^T w=1`. Accept only a reconstructed curve whose true defect satisfies `delta(s) <= B*w(s)/A(s)` on each subinterval. For constant density this is `delta <= B/(T*A)`. In the contractive case `mu<=0`, `A=1`, so the budget is simply an error-per-unit-time bound. This is a complete conditional construction, and it does not require guessing how many adaptive steps remain. Coefficient, callback, initial-radius and query-center rounding must be inside `delta`/`r0` or added to the same budget with their propagation factors.

If the piecewise returned curve has jumps `J_i=||u(t_i+)-u(t_i-)||`, add `sum_i A(t_i) J_i` to the left-hand budget. This follows by applying the same comparison on each continuous piece and charging each jump by the triangle inequality. A query evaluation whose binary64 center is displaced from that curve by a certified `q(t)` needs a further pointwise charge `q(t)`; reserve a uniform query margin or check it against remaining budget at each query. An a.e. residual integral alone cannot detect jumps.

An endpoint version follows for a mesh `t_i`: if a verified local flow defect is `ell_i` and the later flow gain is bounded by `A(t_{i+1})`, then `A(0)r0 + sum_i A(t_{i+1}) ell_i <= epsilon` bounds all mesh endpoints. A bound for all intermediate times additionally needs each local *continuous* defect; endpoint tests alone do not supply it.

## G2: optimal integer step allocation with a proved work bound

Partition the horizon into `m` fixed blocks. On block `j` of length `T_j`, suppose every equal step of length `h <= T_j` admits a local error bound `C_j*h^(p+1)`, valid uniformly on the required tube. Let `a_j` bound propagation from every local step in that block to any later requested endpoint. Let each step cost at most `c_j > 0` counted work units, including its certificate. Suppose construction/admission succeeds under these uniform bounds, so no additional unaccounted failed trials are needed.

Using `n_j >= 1` equal steps gives total propagated error at most

\[
E(n)=\sum_j A_j/n_j^p,\qquad A_j=a_j C_j T_j^{p+1},
\]

and counted work at most `W(n)=sum_j c_j n_j`. For positive `A_j`, write

\[
S=\sum_j A_j^{1/(p+1)}c_j^{p/(p+1)},\qquad
n_j^*=(A_j/c_j)^{1/(p+1)}(S/\varepsilon)^{1/p}.
\]

The positive-real relaxation (dropping the lower constraints `n_j >= 1` and allowing all `n_j > 0`) has optimum

\[
W_* = S^{(p+1)/p}\varepsilon^{-1/p}.
\]

The integer construction `n_j=max(1,ceil(n_j*))` satisfies `E(n)<=epsilon` and

\[
W(n) < W_*+\sum_j c_j.
\]

**Proof.** Hölder's inequality gives
`S = sum (A_j/n_j^p)^(1/(p+1)) (c_j*n_j)^(p/(p+1)) <= E(n)^(1/(p+1))*W(n)^(p/(p+1))`.
Thus any feasible real allocation has `W>=W_*`. Substitution of `n_j*` attains both the error equality and this work lower bound. Rounding each positive `n_j*` up decreases every error contribution and increases work by less than `c_j`. ∎

For `A_j=0`, one step covers the block at zero modeled error; add its cost separately. Floating computation of the ceiling must be followed by an outward evaluation of `E(n)`, increasing counts until the original budget is enclosed. An adaptive run with rejected attempts adds their actual work explicitly; the theorem never erases them.

**Exact cost-reduction witness.** Take `p=1`, two unit-length blocks, `c1=c2=1`, `a1=a2=1`, `C1=1`, `C2=81`, and `epsilon=1/10`. The common-step-count strategy needs `n1=n2=ceil((A1+A2)/epsilon)=820`: a certified work ceiling of 1640 units. G2 yields `(n1,n2)=(100,900)`: exactly the same `1/10` error budget with a 1000-unit work ceiling, a **39.024% reduction of the certified cost bound**. When every step actually costs exactly `c_j=1` (as in this algebraic witness), it is also a 39.024% counted-work reduction; a ceiling alone does not prove that ratio for actual work. This is an algebraic witness, not a timing result or a claim about the current facade benchmarks. The asymptotic order remains `epsilon^(-1/p)`; the sharp allocation constant improves.

## Code and next execution

- Existing model-bound continuous providers and `integrate_verified_residual` supply the right authority boundary. Reuse their actual represented-curve, outward residual and full-tube machinery.
- Add a `GlobalBudget` owner storing fixed norm/scales, total horizon, initial charge, future positive-log-norm envelope, and remaining weighted defect budget. An attempt reserves no scientific budget permanently until acceptance; rejected *work* is permanent.
- Offer G1 constant-density scheduling first. A prototype must check exact linear contractive/expansive witnesses and nonzero initial radii before running nonlinear cases.
- Add G2 only where certified blockwise `C_j` and full per-step work ceilings exist. For generic black-box callbacks, the same expression can select a candidate step as a heuristic but cannot certify global error.
- Preserve the currently preregistered G1 study and B1c failures. No midpoint threshold is loosened to create a production promotion.

The exact oracle `verify_global_budget.py` checks the finite propagation comparison and the cost witness using rational arithmetic. It supports the algebra and detects implementation drift; the continuous theorem is proved above.
