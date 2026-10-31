# VigilODE R5 | coupled source-owned inverse-Picard fast-first, 2026-10-31

**Authority:** parent PR #154, immutable HEAD `93b2f238f693033ee2d83efc270124c5fef6af84`. Scope is the native opt-in VigilODE **algorithm**. BASS/REI source, original RODAS5P eight-stage order, production defaults, source policies and tolerances remain untouched. This R5 is *not* an order upgrade for BE.

## Definition and positive domain

Let `d>=2` be a bounded number of modes, with represented binary64 coefficients interpreted as exact real source values. Require `k_i>0`, `g_i>=0`, `c_ij>=0 (i!=j)`, `c_ii=0`, initial data `Y_i>=0` with `M=max_i Y_i>0`. The actual genuinely coupled model is

\[
\dot y_i=-k_i y_i-g_i y_i^3+\sum_{j\ne i}c_{ij}(y_j-y_i),\qquad y_i(0)=Y_i.
\]

No time-dependent source ownership changes are permitted. For `u_i=0` the vector field is nonnegative; at any maximizing component it is at most `-k_min M`. Standard comparison hence gives `0<=u_i(t)<= M exp(-k_min t)` globally. Continuous invariance is exact-real; the numerical representation uses directed outward enclosures for original source residuals, not a claim about roundoff-free floats.

## Backward Euler stage and two candidate maps

At a represented step `h>0` and previous stage vector `v` in `[0,M]^d`, define `C_i=sum_j c_ij`, `a_i=1+h(k_i+C_i)`, `b_i=h g_i`, and `d_i(z)=v_i+h sum_j c_ij z_j`. The *original*, unmodified BE residual is

\[
 R_i(z)=a_i z_i+b_i z_i^3-d_i(z).
\]

A basic rational-Picard candidate is `G^rat_i(z)=d_i(z)/(a_i+b_i z_i^2)`. It preserves positivity but can be noncontractive when `b_i` is large. This is the explicit negative alternative, **not** the implemented accelerator.

Define instead `P_i(w)=a_i w+b_i w^3`. Since `P_i'(w)=a_i+3b_i w^2>=a_i>0` on `[0,infty)`, the unique source-owned inverse `psi_i=P_i^{-1}` exists. The **preconditioned value-only candidate** is

\[
 G_i(z)=\psi_i(d_i(z)).
\]

This treats the local stiff cubic exactly in the underlying map, retaining off-diagonal coupling as the sole fixed-point iteration. If `z in D=[0,M]^d`, then `0<=d_i(z)<=M(1+hC_i)`, and `0<=G_i(z)<=d_i(z)/a_i<=M`. Thus `G(D) subset D` for **every positive h**, independent of the stiffness of `g_i`. The source-driven `ell_infty` contraction constant is

\[
q=\max_i\frac{h C_i}{1+h(k_i+C_i)}<1.
\]

Indeed `d psi_i/d d=1/(a_i+3b_i psi_i(d)^2)<=1/a_i`, so `||G(z)-G(w)||_inf<=q||z-w||_inf`. The q factor uses only the frozen source, physical h and directed-up interval coefficients, never a measured iteration ratio, supplied JVP, finite difference, Jacobian or Newton solve. A true numerical q upper must be strictly less than one, else fail closed.

## Stable inverse candidate and its independent certificate

For `a>0,b>=0,d>=0`, let `beta=b*d^2/a^3`, `Z=(3/2)*sqrt(3*beta)`, `U=(Z+sqrt(1+Z^2))^(1/3)`, `s=3/(U^2+1+U^-2)`. The unique inverse is `w=(d/a)*s`, with `w=d/a` if `b=0` or `d=0`. Wolfram independently simplified `(a*w+b*w^3-d)` to 0 under `a>0,b>0,d>0` (2026-10-31). This inverse has no subtractive cancellation, but its **binary64 approximation is an untrusted predictor**. The original polynomial evaluation is recomputed in outward intervals at the represented candidate `w`:

`Sigma_i=a_i*w_i+b_i*w_i^3-d_i(z)`. From strong monotonicity, the mathematical exact map output obeys `|G_i(z)-w_i|<=mag([Sigma_i])/a_i_lower`. Thus `G_i(z)-z_i` has a rigorous interval enclosure from `point(w_i)-point(z_i)+[-delta_i,delta_i]`. The per-component inner cubic inversion is charged as algebraic-predictor work; no cube-root correct-rounding premise enters the certificate.

For the fixed point residual `r(z)=G(z)-z`, these interval enclosures produce `L(z)<=||r(z)||_inf<=U(z)`. The fast-first depth-one Anderson trial `z_A` is an operational candidate; if `U(z_A)<=mul_down(q_upper,L(z))`, it may skip the baseline-first Picard source call. The baseline-first comparator instead computes both and requires `U(z_A)<=L(G_float(z))` when admitting the speculative trial. A separate sufficient stop condition is the **original BE residual** `rho(z)=||R(z)||_inf` enclosed outward and compared to the same algebraic accuracy budget. Neither a small operational `float residual` nor the two methods' raw output equality is a mathematical error certificate.

## Whole-IVP physical error with full original residual

Let `k_min=min_i k_i>0`; denote the actual smooth ODE solution `u(t)` and the accepted represented BE candidate `z_(n+1)` at exact grid time `(n+1)h`. For positive `z,w`, the original nonlinear BE operator `T(z)=z-h f(z)` obeys an `ell_infty` inverse monotonicity bound

\[
||z-w||_\infty\le\frac{||T(z)-T(w)||_\infty}{1+h k_{min}}.
\]

Proof: take a component k attaining the maximum absolute difference `e_k=z_k-w_k`, multiply the kth residual difference by `sgn(e_k)`; the diagonal `1+h(k_k+C_k)` and monotonic cubic increment dominate offdiagonal `h sum c_kj e_j`, yielding at least `(1+h k_min)||e||_inf`. This proof does not use the conditional contraction bound `q`; BE stability remains valid even if the candidate map is judged inconclusive.

For `t_n=nh`, write `M_n=M/(1+h k_min)^n`, an upper bound on the exact continuous max over `[t_n,t_{n+1}]` since `exp(-h k_min)<=1/(1+h k_min)`. Put

\[
L_n=\max_i (k_i+3g_i M_n^2+2C_i),\quad
P_n=\max_i(k_i M_n+g_i M_n^3+2C_i M_n).
\]

By differentiating `u'=f(u)`, `||u''||_inf <= L_n P_n` in each interval. The exact-solution BE local defect is bounded by `tau_n=h^2 L_n P_n/2`. With the original source residual upper `rho_(n+1)`, the certified error recursion is

\[
E_{n+1}\le\frac{E_n+\tau_n+\rho_{n+1}}{1+h k_{min}},\quad E_0=0.
\]

The physical error at the *reported represented clock label* must additionally include `P_0*|label - exact n*h|` with directed outward arithmetic. All arithmetic bounds follow the frozen `rodas5p_core::directed::Interval` machinery in the PR154 parent, rather than trusting f64 integer/float comparisons.

A sufficient source-derived **per-stage original residual limit** for a requested `eps`: precompute the directed recurrence with `rho=0` and the worst observed clock term, giving `T_major<eps`. Allocate the whole remaining bound `B=eps-T_major` by `rho_limit=mul_down(B,h*k_min)` (positive only). This is sufficient since the geometric accumulation of constant `rho` is at most `rho/(h*k_min)`; it is not a fitted margin or a change to the caller tolerance. The returned state is committed only if the full accumulated directed physical bound is <=eps at every returned time.

## Scientific and computational limitations

* This is *actual* coupled nonlinear dynamics when two initial components differ and `c_ij>0`, not independent scalar replications. It is **first-order backward Euler**, not RODAS5P and not a universal nonlinear stiff IVP solver.
* Candidate `G` uses algebraic per-component scalar inverses and original-value residuals: derivative-free/JVP-free **at runtime**, but not free of `sqrt`/`cbrt` or interval arithmetic. Report predictor and interval work, not just source-evaluation counts.
* The source-specific global bound can be quite conservative for fast exchange because `L_n P_n` includes a generic `2*C_i` factor. Rejecting a tight tolerance in such regimes is a limitation of this enclosure, not evidence that the underlying BE solution is invalid.
* Uniform target eps is not the same as identical **achieved** physical error. For genuinely coupled nonlinear cohorts, compare independently solved Radau reference to the actual full trajectories and report both achieved errors; for the linear coupled symmetric limit an explicit mode-exponential solution is an independent oracle.
* The fast-first benefit is contingent. An accepted trial may cost more than a Picard evaluation; preserve all negative paired blocks, resource failures and optional C9 independent author status.

Literature context: Jerome, *Fully Discrete Stability and Invariant Rectangular Regions for Reaction-Diffusion Systems*, SIAM J. Numer. Anal. DOI 10.1137/0721065; Pollock & Rebholz, *Anderson acceleration for contractive and noncontractive operators*, IMA J. Numer. Anal. DOI 10.1093/imanum/draa095. The source-specific formulae above are direct derivations, not borrowed novelty claims.

## Scope, decision, and DAG (after R5)

- `OFFICIAL_VIGILODE_DAG=7/23`; unchanged and NOT increased for theoretical research.
- `CENTRAL_REI_DAG=28/88` informational; `SUCCESSOR_ADMITTED=0/12`.
- Distinct author C9 `0/1 NOT_RUN`, C10 `NOT_STARTED`, production `HOLD`, auto advancement false, no default/tolerance changes.
- R5 derived and symbolic identity Wolfram-checked; native implementation and paired elapsed measurements are **NOT RUN** at this checkpoint.
