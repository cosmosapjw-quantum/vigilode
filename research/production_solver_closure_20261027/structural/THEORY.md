# Full-space verification from two scalar channels, with represented-stage rounding

## Scope and preserved target

Let `n>=1`, `P=11^T/n`, `Q=I-P`, `J=-d I+kappa P=-d Q-(d-kappa)P`, with represented finite `d>=kappa>=0`. The physical RHS is the affine map `f(y)=b+Jy`. All represented input entries are interpreted as exact real numbers. Tableau entries, `h`, and `gamma` are similarly interpreted as real values of their stored binary64 representations. Thus `h gamma` in the target is a real product, enclosed in the new screen; the construction still uses the existing rounded product.

The raw-U equations are

\[
\mathcal R_i(U)=(I-h\gamma J)U_i-h\gamma f(y)
 -\sum_{j<i}(\gamma c_{ij}I+h\gamma a_{ij}J)U_j.
\]

The previous implementation explicitly evaluated these equations through eight full-state stage RHS calls. Its operational residual is a floating-point measurement. The new enclosure bounds the exact real value of these same equations on the actual returned, represented stages. Temporal truncation and endpoint-addition error remain separate terms in a physical trajectory bound.

## Theorem S1: exact factorized residual identity

Write `a=h gamma` in the following formulas. Let `x` be any represented vector used to construct the stages, `mu` any scalar, and `t_i,c_i` any scalar transfers. Define

\[
U_i=t_i(x-\mu\mathbf1)+c_i\mu\mathbf1+\Delta_i,
\qquad \varepsilon_f=x-f(y),\qquad m=\operatorname{mean}(x).
\]

No assumption that `mu=m` or that the transfers solve their scalar equations is made. Set

\[
\rho_i(r;v)=(1+ar)v_i
  -\sum_{j<i}(\gamma c_{ij}-ar a_{ij})v_j-a,
\quad\rho_{ti}=\rho_i(d;t),\quad
\rho_{ci}=\rho_i(d-\kappa;c),
\]

and `ell_i=t_i+sum_{j<i}a_ij t_j`. Then

\[
\boxed{\begin{split}
\mathcal R_i(U)={}&\rho_{ti}(x-\mu\mathbf1)
 +\rho_{ci}\mu\mathbf1
 +a\kappa\ell_i(\mu-m)\mathbf1+a\varepsilon_f\\
 &+(I-aJ)\Delta_i
 -\sum_{j<i}(\gamma c_{ij}I+a a_{ij}J)\Delta_j .
\end{split}}
\]

**Proof.** Substitute the definition of `U_i` and use `Jx=-d x+kappa m 1` and `J1=-(d-kappa)1`. The coefficient of `x-mu 1` is `rho_ti`, that of the collective transfer is `rho_ci`, and the remaining discrepancy between the represented `mu` and exact mean is `a kappa ell_i(mu-m)1`. Linearity gives the last line and `a epsilon_f`. This is an equality for arbitrary defects, including full-dimensional lift errors; no residual component is discarded. The explicit three-component symbolic expansion in the Wolfram receipt is an independent substitution check, not the proof for arbitrary dimension.

## Theorem S2: dimension-dependent full-stage norm enclosure

For `T=t Q+c P`, its exact induced infinity norm is

\[
\nu_n(t,c)=\left|t+\frac{c-t}{n}\right|
 +(n-1)\left|\frac{c-t}{n}\right|.
\]

Every row has the same diagonal and off-diagonal values, so this follows by summing absolute row entries. The old sharp-norm theorem from PR91 is reused with identical projectors and domain.

Define block weights

\[
w_{ii}=\nu_n(1+ad,1+a(d-\kappa)),
\]

\[
w_{ij}=\nu_n(ad a_{ij}-\gamma c_{ij},
 a(d-\kappa)a_{ij}-\gamma c_{ij})\quad(j<i).
\]

If `||Delta_j||_infty<=delta_j`, `||epsilon_f||_infty<=epsilon`, `|mu-m|<=zeta`, and `||x-mu 1||_infty<=X`, then

\[
\|\mathcal R_i(U)\|_\infty\le
 |\rho_{ti}|X+|\rho_{ci}|\,|\mu|
 +|a\kappa\ell_i|\zeta+a\epsilon
 +\sum_{j\le i}w_{ij}\delta_j.
\]

**Proof.** Apply the triangle inequality and induced-norm inequality to S1. Scalar plan coefficients and block norms are computed with outward interval arithmetic, and the nonnegative final sum uses upward-rounded additions/products. Interval dependency can widen the result but cannot reduce it. Each bound covers all `n` components of that original stage. Computing the eight maxima and rounding envelopes costs `O(sn)`; the triangular scalar weights and their multiplication cost `O(s^2)` and are independent of `n`.

These are certified upper bounds. Only each individual block norm `nu_n` is asserted sharp; the sum including rounded lift envelopes is generally conservative.

## Theorem S3: constructive rounding envelopes

Assume IEEE binary64 round-to-nearest with gradual underflow and no unsafe reassociation. For finite rounded result `z`, let `r(z)` be half the spacing to the next greater-magnitude value in its binade. If this half-spacing is not representable, use the least positive subnormal. At the largest binade use its finite spacing. Then every correctly rounded addition, subtraction, multiplication or division with rounded result `z` has absolute rounding error at most `r(z)`. The exponent-bit implementation computes this value without fitted constants; `1/2` is the round-to-nearest radius.

### Compensated mean

The implementation computes `s=fl(1/n)`, then `v_i=fl(x_i s)`, and accumulates the same Neumaier sum as PR91. For each running sum `S_i=fl(S_(i-1)+v_i)`, the magnitude-ordered FastTwoSum expression obtains the exact local residual `e_i=S_(i-1)+v_i-S_i`. The correction is `C_i=fl(C_(i-1)+e_i)` and the returned value is `mu=fl(S_n+C_n)`. All intermediate values must be finite.

Let upward-computed bounds be

\[
A\ge\sum_i|v_i|,\quad E_p\ge\sum_i r(v_i),\quad
E_c\ge\sum_i r(C_i),\quad\theta\ge r(s)/s.
\]

Then

\[
\boxed{|\mu-\operatorname{mean}(x)|
\le E_p+\theta(A+E_p)+E_c+r(\mu).}
\]

**Proof.** Telescoping FastTwoSum identities gives `sum v_i=S_n+sum e_i`; accumulating the corrections contributes at most `E_c`, and the final addition at most `r(mu)`. Also `sum |x_i s|<=A+E_p`, and `|s-1/n|<=r(s)`, giving the scaling error bound `theta(A+E_p)`. Add the product errors `E_p`. The dimension must have an exact binary64 representation; the public enclosure plan restricts it to the positive integers at most `2^53`.

### RHS envelope

The actual RHS operations are `p_k=fl(d y_k)`, `v_k=fl(b_k-p_k)`, `shared=fl(kappa mu_y)`, `x_k=fl(v_k+shared)`. If the mean envelope is `zeta_y`, then

\[
\|x-f(y)\|_\infty\le
 r(\max_k|p_k|)+r(\max_k|v_k|)+r(\max_k|x_k|)
 +r(\mathrm{shared})+\kappa\zeta_y.
\]

The radius is monotone in absolute magnitude, so maxima suffice. Every term is evaluated upward in the implementation.

### Lift envelope

The actual lift uses `v_k=fl(x_k-mu)`, `p_k=fl(t_i v_k)`, `q=fl(c_i mu)`, and `U_ik=fl(p_k+q)`. Hence

\[
\boxed{\|\Delta_i\|_\infty\le
 r(\max_k|U_{ik}|)+r(\max_k|p_k|)+r(q)
 +|t_i|r(\max_k|v_k|).}
\]

This follows by expanding the four local rounding errors. Also `X<=max_k|v_k|+r(max_k|v_k|)`. The constructed stage rows are therefore retained exactly, while their full-dimensional discrepancy from the two scalar channels is explicitly covered.

## Charged failure contract

`step_charged` and `sequential_step_charged` return `StructuralFailure {error, work}`. Pre-attempt invalid inputs retain zero work. Numerical RHS/resolvent attempts are charged before invocation. Scheduler closures return successful containers carrying each row's result and work, so a row failure never short-circuits the scheduled batch. All row work is summed, then the first error in stage order is returned. The legacy `CoreResult` wrappers retain their existing signatures and intentionally discard only the additional work payload; adaptive clients should use the charged variants.

`MeanFieldEnclosurePlan::step` follows the same failure contract. It declines if its outward bound exceeds the requested residual limit. It never widens the limit or silently replaces the certificate with the operational screen.

## Alternatives and attribution

The fused direct screen retains `O(s^2 n)` arithmetic and the original per-component accumulation order, while reducing row temporaries from four `n`-vectors to two. The factorized enclosure changes the verification algorithm and adds a stronger rounding-aware contract; it is not an assertion that fewer scalar instructions always mean less wall time. The same-binary benchmark and instruction profile decide that question for each measured case. A scalar-only residual without the rounding terms in S1 is excluded from the candidate set because it does not verify the returned full-state stages.

The implementation-specific factorization and error-budget composition above are derived here. General error-free-transform and verified-bound methods are established numerical-analysis tools, for example Ogita–Rump–Oishi, *Accurate Sum and Dot Product*, SIAM J. Sci. Comput. 26 (2005), 1955–1988, DOI 10.1137/030601818, and Rump, *Accurate Solution of Dense Linear Systems, Part II: Algorithms Using Directed Rounding*, original author source https://www.tuhh.de/ti3/paper/rump/Ru12a.pdf. No broad impossibility or novelty claim is made.

## Theorem S4: exact scalar degeneration removes per-step triangular enclosure work

For `n=1`, `P=I`, `Q=0`, and the existing compensated mean returns the input exactly as a real number (its signed-zero convention is retained). Therefore `mu=x` and the represented stage construction has the real value

\[
U_i=\operatorname{fl}(c_i x),
\]

because the transverse product and addition contribute only exact zeros. For a finite rounded product under round-to-nearest with gradual underflow,

\[
|\operatorname{fl}(z)-z|\le u|z|+\eta/2,
\quad u=2^{-53},\quad\eta=2^{-1074}.
\]

`eta/2` has no positive binary64 representation; the implementation encloses it upward by `eta`. Thus S2 specializes to

\[
\|\mathcal R_i(U)\|_\infty\le
 \left(|\rho_{ci}|+u\sum_{j\le i}w_{ij}|c_j|\right)|x|
 +\eta\sum_{j\le i}w_{ij}+h\gamma\varepsilon_f.
\]

The two sums depend only on the existing immutable plan and are precomputed upward. Per-step enclosure work is consequently `O(s)`. All eight represented stages are still constructed with the original expression and checked finite; no stage or component is omitted. `n=1` is the exact projector degeneration, not an empirically chosen crossover size. The RHS and endpoint use the same expressions as the general path.

This specialization was introduced in correction pass 3 after the scalar adverse timing exposed the general enclosure's constant cost. Its measured results are explicitly post-diagnostic. No dimension cutoff or fitted performance parameter was introduced.

### Exact aggregation of scheduled work

The direct screen schedules all eight rows. Every row constructs `i*n` state contributions and attempts its RHS before it can return a numerical error. Hence the batch contributes exactly 8 RHS attempts and `sum_i i*n=28n` state accumulations. A row records only whether its linear action began, how many scalar components reached the mix, and how many reached the residual. The remaining counters are reconstructed as `i * mixed_components` and `residual_components`. The quotient lift constructs all eight complete vectors before finite checks, so it always charges `8n` constructed components after joining the batch, including failed rows. These identities replace full per-worker counter payloads without dropping failed work.
