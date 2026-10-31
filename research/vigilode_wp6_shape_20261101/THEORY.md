# WP6 mathematical derivation: time-shaped continuous defect rather than constant source maximum

## Physical/source contract
Original exact represented dyadic cooperative cubic source:
`y_i'=-k_i*y_i-g_i*y_i^3+sum_{j!=i}c_ij*(y_j-y_i)`,
`k_i>0,g_i,c_ij>=0`. Let `X(t)` be the original WP2 source-locked piecewise cubic Hermite reference (no assumption on how it was numerically generated). Let `e_i=|X_i-y_i|`.

A previous WP5 derivation gives the (real, source-specific) componentwise differential comparison
`D^+e <= A_n e + |r|`, `r=X'-f(X)`, for a piecewise constant Metzler `A_n` whose off-diagonal elements are `c_ij` and whose diagonal subtracts original k, total C and source-proved cubic dissipation. `A_n` is substochastic stable because its row sums are negative.

## New exact envelope
For `u=(t-t_n)/h` in [0,1], the exact polynomial residual on a panel has degree <=9 and Bernstein expansion
`r_i(u)=sum_{l=0}^9 b_{i,l} B_{l,9}(u)`, `B_{l,9}=binom(9,l)u^l(1-u)^(9-l)>=0`.
Set `R_i(u)=sum_{l=0}^9 |b_{i,l}| B_{l,9}(u)`. Then for every u,
`|r_i(u)| <= R_i(u) <= max_l |b_{i,l}|`. This is a rigorous, generally strictly tighter forcing envelope.

For lambda >= max_i(-A_ii), `P=I+A/lambda` is entrywise nonnegative with `P*1<=1`. For `q=lambda*h<=1` the time-shaped forcing contribution to the error at t+h is bounded by
`h*sum_{j=0}^N (q^j/j!) P^j ( sum_{k=0}^M (-q)^k/k! * B_{j,k} ) + h*tail*max_i,max_l |b_i,l| *1`
where vector `(B_{j,k})_i=sum_l |b_i,l|*binom(9,l)*Beta(l+1,10-l+j+k)`.
The beta factor is exactly
`binom(9,l)*l!*(9-l+j+k)!/(10+j+k)!`.
`M` is even so `T_M(z)>=exp(-z)` for z in [0,q]. The omitted positive uniformization mass is bounded by
`tail = (q^(N+1)/(N+1)!)/(1-q/(N+2))`.
The initial-error term separately uses the same WP5 upper bound on `exp(A*h)*e(t)`. Every matrix and scalar in this formula is exact rational (source dyadics). The extra time-shaped source term never requires accepting a sampled residual as proof.

For optional depth d, split the original degree-nine Bernstein control polygon into `2^d` exact de Casteljau subpanels, then apply this formula on each of length `h/2^d`. Recompute local source-valid cubic minima from exact degree-three Bernstein coefficients; outward round after each subpanel. If `q>1`, non-Metzler A, or invalid source identity, reject rather than relax.

## Proof status
- DERIVED: convex-hull/triangle envelope; beta moment identity; positive matrix comparison and Poisson tail bound.
- NUMERICALLY_CHECKED: 4 frozen cohorts, 96 micro32 original reference panels each; 384/384 exact rational pilot; 8/8 same-context adversarial tests.
- ABLATION: shape forcing vs constant forcing at the **same subpanel partition**, asymmetric_2 1.1311062605085834e-13 -> 6.488679612260851e-14, linear control 7.818160336532192e-16 -> 4.587141366439368e-16.
- CAS: Mathematica exact beta for (l,j,k)=(4,2,3): 6/715 by direct definite integration, equals factorial formula.
- HISTORICAL HOL: WP5 established four scalar cubic algebra lemmas in original archived HOL Light logs; **new WP6 beta-integral/Metzler comparison not HOL kernel proved**.
- NOT MEASURED: full 43,008 panels under new scheme, native new IVPs, same-run BE/RODAS5P speedups.
- NO NEW NOVELTY CLAIM: compare Corless-Corliss guaranteed defect control, Auzinger et al. stiff defect correction, other positive-system validated schemes before claiming priority.

## Referee findings
1. Constant panel residual is a valid but loose upper bound; fair isolation requires same exact d,N,M,q for ablation.
2. Keep direction of P^j and the `(1-u)^j` factor. Using `u^j` is a genuine forcing-integration error.
3. Do not replace exact beta by midpoint samples, or misinterpret `T_M` for odd M as an upper.
4. Reject general componentwise claim `exp(Ah)<=(I-hA)^-1`; original WP5 has an explicit Metzler counterexample.
5. Certificate tightening is not reduced physical forward error, and no official DAG count is incremented.

Official VigilODE DAG 7/23, successor 0/12, production HOLD, no merge/default/tolerance changes.
