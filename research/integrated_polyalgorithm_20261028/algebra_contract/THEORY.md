# From a declared operator algebra to a certified raw-U endpoint

## Status, scope and reuse

This is a constructive R3/R5/R9 contribution to the integrated PR90--93
research, against source `384dba091a3d9e1aac0ecddec264958298239253`.
It supplies two extensions: an **output-specific quotient stage compiler**
with an explicit full-target defect, and a **precision-aware refinement
admission rule**. They are mathematical constructions and proposed interfaces,
not native implementations or measured improvements. The elementary proofs
below do not assert literature novelty.

Skill context used: `physmath-research-loop` 1.3.0; the actual stage definitions
and policies supplied in `../SKILL_CONTEXT.md` and the objective in
`../RESUME.md` were read. This worker performs no publication, recursive
delegation, solver build, native timing or execution of its prospective checker.

### Existing results to reuse, rather than relabel as new

| Current source | Reused result | New connection here |
|---|---|---|
| `crates/rodas5p-integrators/src/conditioned_polynomial.rs` | Conditioned chart, polynomial inverse, stored-coefficient defect, exact operator/norm/epoch identity | Compile all causal affine stages in that same quotient algebra; certify the actual stored stage polynomials |
| `crates/rodas5p-integrators/src/polynomial_resolvent.rs` | Approximate quotient refinement and original-residual-controlled commit | Derive the finite-precision defect floor and least sufficient correction count |
| `crates/vigilode-coupled/src/causal.rs` | One common projected factorization, strictly causal forward/Picard execution, exact-input/prefix reuse | Eliminate all virtual stage-vector actions for an admitted endpoint-only affine request |
| `crates/vigilode-coupled/src/structural_mean_field.rs` | Two-channel algebra, original-space stage enclosure, charged failure work | A general declared finite algebra, with the two-channel implementation retained as a stronger specialized comparator |
| `crates/vigilode-coupled/src/residual_first.rs` | Full nonlinear remainder and original-target residual semantics | Bound a declared near-affine remainder before omitting stage RHS calls |
| `crates/rodas5p-core/src/polynomial_router.rs` | Same-request fused-action admission, current dense symmetric provider, complete cost boundaries | Separate endpoint-stage request type; do not reuse a phi-action token for it |
| PR93 `polynomial/THEORY.md`, P1--P4 | Fused Clenshaw, exact defect transport, SPD degree selection, near-inverse geometric correction | Use these as execution/certification leaves for compiled output polynomials |
| PR91--93 causal/geometry theory | Finite nilpotent residual propagation and two-channel adjoints | Transfer the compiler's explicit full residual to the original stage endpoint |

The inspection also found current refinement defaults
`maximum_relation_ratio=0.5` and `stagnation_ratio=0.95`. Their comments classify
them as work/selection policies. They are not mathematical convergence constants.
The second theorem gives a proof-based planning alternative; explicit resource
caps and the original-space residual gate remain useful controls.

## A. One output polynomial for an entire declared affine stage system

### A1. Target, chart and domain

Let there be s strictly causal raw-U stages (s=8 in the current consumer),
with the actual represented tableau coefficients a_ij, c_ij, gamma and endpoint
weights beta_i. Let tau=h gamma, h>0, gamma>0, and let J0 be one fixed,
declared linear carrier for this attempted step. The target is

\[
 W U_i=\tau f\!\left(y+\sum_{j<i}a_{ij}U_j\right)
       +\gamma\sum_{j<i}c_{ij}U_j,
 \qquad W=I-\tau J_0.                                      \tag{A1}
\]

This is the autonomous/frozen-time raw-U target currently used by the causal
consumer. A different nonautonomous target, mass matrix, stage time, or extra
partial-time term requires its own matching equation. The construction does
not silently change (A1).

Suppose the full-space provider declares

\[
 Z=\tau J_0=cI+\sigma C,\qquad \sigma>0,
 \qquad \|C\|\le M,\qquad \|\pi(C)\|\le\rho,                \tag{A2}
\]

in one fixed induced norm. The polynomial pi has degree d>=1 and nonzero
leading coefficient. These are whole-space declarations, not a relation
inferred from a small set of sample vectors. The chart, norm, operator and
provider/model epoch are part of the immutable plan identity.

Write z(x)=c+sigma*x, w0(x)=1-z(x), and
zeta=(1-c)/sigma. If pi(zeta)!=0, the polynomial

\[
 v(x)=\frac{\pi(\zeta)-\pi(x)}
            {\sigma\pi(\zeta)(\zeta-x)}                    \tag{A3}
\]

has degree d-1 and satisfies

\[
 w_0(x)v(x)=1-\frac{\pi(x)}{\pi(\zeta)}.                    \tag{A4}
\]

This is the already implemented conditioned-resolvent identity. In particular,
if rho=0 then v(C)=W^-1. If

\[
 \theta_\pi=\rho/|\pi(\zeta)|<1,
\]

then W is invertible and a valid inverse bound is

\[
 \omega=\frac{\|v(C)\|}{1-\theta_\pi}
 \le\frac{\sum_{k<d}|v_k|M^k}{1-\theta_\pi}.                \tag{A5}
\]

For stored binary64 coefficients, use the existing *complete stored-polynomial*
defect in place of theta_pi. The exact-real formula (A5) is not an excuse to
discard preparation error. If c/sigma or pi(zeta) is ill-conditioned, an
outward preparation can refuse this route without changing the target.

### Theorem A — exact quotient stage compilation

Assume rho=0 and f(y+u)=f0+J0 u throughout the stage domain, with f0=f(y).
In the polynomial quotient by pi, compute degree<d representatives

\[
 r_i=\operatorname{rem}_\pi\left[
 v\left(1+\sum_{j<i}B_{ij}r_j\right)\right],
 \qquad B_{ij}(x)=a_{ij}z(x)+\gamma c_{ij}.                   \tag{A6}
\]

Then the unique solution of (A1) is

\[
 U_i=\tau r_i(C)f_0,
\]

and the requested endpoint is

\[
 y_+=y+\tau p_\beta(C)f_0,
 \qquad p_\beta=\sum_i\beta_i r_i,\qquad \deg p_\beta<d.     \tag{A7}
\]

**Proof.** Equation (A4) makes v the inverse of w0 modulo pi.
Thus (A6) gives w0*r_i=1+sum B_ij*r_j modulo pi. Evaluating at C
annihilates the multiple of pi because pi(C)=0. Multiplication by tau*f0
gives (A1), using the affine identity for f. W is invertible, and strict
causality gives uniqueness inductively. Summing the stage outputs proves
(A7). No diagonalization, normality or contractivity assumption is used.

**Work and output contract.** Once its scalar plan is prepared, one endpoint
requires at most d-1 full-space C actions by Horner evaluation, plus vector
assembly and f0 if it is not already available. It does not require s separate
degree-(d-1) stage actions or s-1 new stage RHS calls. Straightforward scalar
preparation costs O(s*d^2+s^2*d) with dense coefficient arithmetic and can be
reused at the identical chart/h/tableau epoch. This is an operation count,
not an elapsed-time result.

The represented-time grid, tableau and exact original stage target are
unchanged. The output API *does* matter: an endpoint-only caller can omit stage
vectors; a dense-output or diagnostics consumer demanding those vectors cannot
pretend they were produced. Embedded error and fixed dense-output linear
functionals each have their own polynomial p_beta. Several such outputs can
share the power sequence f0,Cf0,...,C^(d-1)f0, using d-1 actions total and
O(number_of_outputs*n) accumulator storage. That power-basis evaluation can
be numerically worse than separate conditioned/Clenshaw recurrences; the
complete arithmetic bound decides admission.

The existing mean-field direct scalar/projector calculation may be cheaper
than a general polynomial evaluator and remains a required comparator.
Similarly, this construction does not beat the shared projected factorization
automatically when d is large or scalar preparation dominates.

## B. Near-structure and stored coefficients: an explicit full-target defect

Theorem A would be unsafe if a small pi(C) were silently replaced by zero.
The following exact identity gives a constructive extension. It also permits
any actually stored degree<d coefficient vectors, even when scalar compilation
has rounding error.

For the stored r_i, form the polynomial

\[
 s_i(x)=w_0(x)r_i(x)-1-\sum_{j<i}B_{ij}(x)r_j(x).             \tag{B1}
\]

Its degree is at most d. Division by pi therefore has a **scalar quotient**:

\[
 s_i(x)=\alpha_i\pi(x)+e_i(x),\qquad\deg e_i<d.              \tag{B2}
\]

For exact (A6), e_i=0. For stored coefficients, an outward coefficient
calculation bounds every coefficient of e_i. Define

\[
 E_i(M)=\sum_{k<d}|e_{ik}|M^k.                               \tag{B3}
\]

One explicit value of the scalar quotient is

\[
 \alpha_i=-\frac{\sigma}{\pi_d}
       \left(r_{i,d-1}+\sum_{j<i}a_{ij}r_{j,d-1}\right).     \tag{B4}
\]

This low-degree fact is useful: certifying the logical stage residuals does
not require forming a high-degree remainder polynomial for each stage.

### Theorem B — compiled endpoint defect at the numerical input

Let hat(f0) be the supplied base RHS with
||hat(f0)-f(y)||<=epsilon0. Define virtual stages

\[
 V_i=\tau r_i(C)\widehat f_0,
 \qquad q_i=\sum_{j<i}a_{ij}V_j.                            \tag{B5}
\]

Suppose a declared nonlinear remainder bound at these virtual stage inputs is

\[
 \|f(y+q_i)-f(y)-J_0q_i\|\le\mu_i.                          \tag{B6}
\]

Then their *original*, full-space residuals for (A1) obey

\[
 \|R_i\|\le b_i:=|\tau|\left[
 (|\alpha_i|\rho+E_i(M))\|\widehat f_0\|
 +\epsilon_0+\mu_i\right].                                 \tag{B7}
\]

Assume ||W^-1||<=omega, and f is L_i-Lipschitz on a set containing the
line segment joining the virtual and exact target stage arguments. Define
the nonnegative strictly lower triangular matrix

\[
 K_{ij}=\omega\left(|\tau|L_i|a_{ij}|+|\gamma c_{ij}|\right)
 \quad(j<i),\qquad
 H=\omega\sum_{\ell=0}^{s-1}K^\ell.                         \tag{B8}
\]

If the actual computed endpoint differs from
y+sum beta_i V_i by at most epsilon_eval, then

\[
 \|\widehat y_+-\Psi_h(y)\|
 \le |\beta|^T H b+\epsilon_{\rm eval}.                     \tag{B9}
\]

Here Psi_h(y) is the endpoint of the **exact original raw-U stage solve at
the same actual numerical input y**. Equation (B9) is an algebraic consistency
certificate; it contains no physical time-discretization or clock estimate.
Use it as the local algebraic leaf of the sibling physical-error composition
theorem. Do not reinterpret it as a bound on y_+-Phi_h(y).

**Proof.** Substituting (B5) into the original residual and using (B1) gives

\[
 R_i=\tau\left[
 (\alpha_i\pi(C)+e_i(C))\widehat f_0
 +(\widehat f_0-f(y))
 -(f(y+q_i)-f(y)-J_0q_i)\right].                            \tag{B10}
\]

Polynomial evaluation commutes with pi(C), and the induced norm and (A2)
give (B7). Subtracting the exact and virtual stage equations gives
componentwise error <=omega*b+K*error. Nilpotency K^s=0 yields
error<=H*b without requiring ||K||<1. Weight by |beta| and add the actual
endpoint evaluation defect. This last step reuses the established causal
majorant, not a new claim about finite propagation.

**What must be enclosed.** epsilon_eval covers output-polynomial coefficient
assembly, conversion to a chosen polynomial basis, operator-action defects,
backward recurrence defects, the factor tau, and the final addition to y.
PR93's fused Clenshaw defect transfer supplies one way to bound this term.
Terms already inside e_i or epsilon0 must not be counted again in the same
role. Independently recomputing a looser bound is allowed; omitting one role
is not. Every magnitude and comparison used for admission needs outward
arithmetic in a future native implementation.

### B1. When omitted RHS calls are justified

For an affine declared model, mu_i=0. For a near-affine model a full-domain
analytic remainder can be cheaper than evaluating every stage RHS.
One possible radius for the virtual argument is

\[
 \|q_i\|\le |\tau|\sum_{j<i}|a_{ij}|
       \left(\sum_{k<d}|r_{jk}|M^k\right)\|\widehat f_0\|.   \tag{B11}
\]

That radius can be passed to a truthful model-specific remainder provider.
For example, for scalar f(x)=-kx-gx^3 and the declared carrier J0=-k,
the remainder at y+q has magnitude at most

\[
 |g|\left(3|y|^2R+3|y|R^2+R^3\right),\qquad |q|\le R.      \tag{B12}
\]

This uses only given model coefficients; it introduces no derivative callback.
It may be too conservative to omit a stage. In that case compute the original
RHS/residual or use the existing causal/homotopy path, with all work charged.
L_i also requires a domain covering the exact target argument, not merely
the virtual one; establish that domain by a sequential causal enclosure or
an existing global model bound. A few successful stage probes do not provide
the required whole-domain witness.

### B2. Preserve correlated structure defects when profitable

With exact coefficients and an exactly affine model, all stage residuals are
scalar multiples of the same vector:

\[
 R_i=\tau\alpha_i\pi(C)f_0.                                \tag{B13}
\]

Thus (B9) deliberately loses some cancellation. If C is self-adjoint in the
declared Hilbert norm and sigma(C) lies in a certified interval I on which
w0(x)!=0, let the scalar lower triangular matrix L(x) have diagonal w0(x)
and lower entries -B_ij(x). Then the exact signed output identity is

\[
 \sum_i\beta_i(V_i-U_i)
 =\tau g_\beta(C)\pi(C)f_0,
 \qquad g_\beta(x)=\beta^T L(x)^{-1}\alpha.                  \tag{B14}
\]

Consequently its norm is at most

\[
 |\tau|\rho\|f_0\|\sup_{x\in I}|g_\beta(x)|.               \tag{B15}
\]

**Proof.** The affine error equation is L(C)(V-U)=tau*alpha*pi(C)f0.
All its blocks are rational functions of one fixed C and commute. Apply
the endpoint functional and then the spectral theorem. This signed bound
is an alternative to the majorant under its stronger self-adjoint assumptions.
It does not follow from eigenvalue enclosure alone for a nonnormal C.

The scalar rational envelope can reuse interval/Bernstein infrastructure
only after proving a nonzero denominator on I. Its setup cost must be charged.
For the existing two-projector model it reduces to the already established
two scalar adjoints and sharp block norm; do not replace that cheaper formula
with a generic interval engine.

## C. Precision-aware near-inverse correction without a fitted contraction rule

PR93 establishes the exact identity AQ_r=I-(I-AP)^r. Current native refinement
also monitors the original residual but uses operational relation/stagnation
thresholds. The following bound includes error in the residual, correction
and update, so planning can separate mathematical feasibility from work caps.

### Theorem C — inexact correction and finite-precision floor

Let A and P be fixed square operators on the same normed space with

\[
 \|I-AP\|\le\theta<1,\qquad \|A\|\le a.
\]

At an iterate x_k, define the true residual r_k=f-Ax_k. Suppose

\[
 \widehat r_k=r_k+e_k,\quad\|e_k\|\le\eta_k,
 \qquad d_k=P\widehat r_k+z_k,\quad\|z_k\|\le\zeta_k,
\]

and the represented update is x_(k+1)=x_k+d_k+u_k with
||u_k||<=upsilon_k. Then

\[
 r_{k+1}=(I-AP)r_k-APe_k-A(z_k+u_k),                         \tag{C1}
\]

and a valid recurrence is

\[
 R_{k+1}=\theta R_k+\kappa_k,
 \qquad\kappa_k=(1+\theta)\eta_k+a(\zeta_k+\upsilon_k).     \tag{C2}
\]

One can replace 1+theta by any independently sharper bound on ||AP||.
For uniform kappa_k<=kappa,

\[
 R_m\le\theta^mR_0+
             \kappa\frac{1-\theta^m}{1-\theta}.             \tag{C3}
\]

**Proof.** Substitute the represented residual, correction and update into
f-Ax_(k+1), then use ||AP||=||I-(I-AP)||<=1+theta. Unroll the nonnegative
scalar recurrence. No assumption that A and P commute is used.

If 0<theta<1, R0>epsilon and epsilon>F:=kappa/(1-theta), the least correction
count certified by this *uniform bound* is the least integer m satisfying

\[
 \theta^m\le\frac{\epsilon-F}{R_0-F}.                       \tag{C4}
\]

Determine that integer with outward powers/inequalities; rounded logarithms
alone cannot decide admission. If 0<theta<1, epsilon<=F and R0>epsilon, this uniform
recurrence cannot certify a finite success. This is a limitation of the bound,
not proof that the actual solve cannot converge. Sharpen the actual enclosures,
change precision/provider or use another admitted route. If theta=0, one
correction is bounded by kappa and can be certified when epsilon>=kappa,
including equality epsilon=F=kappa. If R0<=epsilon, no correction is needed.

Example for exact supporting checks: theta=1/4, eta=1/10000,
zeta=upsilon=1/20000, a=1 gives kappa=9/40000 and F=3/10000.
For R0=1 and epsilon=1/1000, the least sufficient count is **6**.
The checker will verify the integer inequalities and a scalar construction
that attains the bound at every step. These are exact finite checks, not
compiled roundoff measurements.

### C1. Backend changes and the actual decision contract

For standalone refinement, P_k may vary if every iteration supplies its own
theta_k and defect enclosures. The scalar bound is then

\[
 R_m\le\left(\prod_{j<m}\theta_j\right)R_0+
 \sum_{k<m}\left(\prod_{j=k+1}^{m-1}\theta_j\right)\kappa_k. \tag{C5}
\]

This does not justify changing a preconditioner inside a standard fixed-linear
Krylov method. Freeze it there or choose a separately justified flexible method.

The proposed planner admits a candidate when its derived correction count fits
the caller's explicit action/memory/time budget and the resulting complete
bound meets the requested residual tolerance. It can compare a theta=0.7
candidate with a theta=0.3 candidate by their certified work, without treating
the present 0.5 cutoff as a theorem. Work costs include residual evaluations,
setup, failed attempts and fallback. Measured elapsed-time coefficients stay
measurements; they are not analytic constants.

The actual original residual plus its evaluation allowance still controls
output commit. A tighter observed residual is legitimate a posteriori evidence;
an observed residual ratio is not automatically a future contraction witness.

## D. Integration contracts and exact failure cases

1. **Request identity:** (A/B) returns a raw-U endpoint or specified stage-linear
   functional. It is not a joint-phi action, arbitrary nonlinear root, or
   physical-flow certificate. (C) returns a linear-system residual certificate.
   Keep those result types distinct under one orchestration layer.
2. **Derivative scope:** a truthful analytic/model-declared C action can be fully
   derivative-free. Current polynomial routing is dense symmetric; the general
   coupled basis still uses frozen supplied JVPs. These implementations are not
   silently reclassified. Nonlinear finite differences require action defects
   and do not define an exact immutable C automatically.
3. **Plan lifetime:** bind h/gamma/tableau, all output weights, chart/annihilator,
   norm, domain, provider/model epoch, precision and arithmetic policy. For a
   changed h, rebuild the scalar plan; a fixed declared J relation can sometimes
   be transformed analytically, but old coefficients do not survive by name.
4. **No fabricated stages:** endpoint-only consumers get their actual result.
   Dense output, stage logging, derivative-dependent callbacks or effectful
   stage callbacks can require the original stage execution. Omitting calls
   requires the provider's mathematical and callback-effect contract to permit it.
5. **Adverse control:** C=[[delta,1],[0,-delta]], pi(x)=x^2 has
   pi(C)=delta^2 I. A plan compiled as if pi(C)=0 has nonzero original residual
   for delta!=0. B7 covers it; using a zero structure budget is false.
6. **Conditioning/refusal:** pi(zeta)=0 gives no polynomial inverse of w0 in
   that quotient; even small but nonzero pi(zeta) can make the proof/cost
   unfavorable. This refuses this plan, not every solver for that problem.
7. **Nonlinear omission:** a near-affine candidate must pay mu_i. Treating an
   affine carrier as the full nonlinear RHS changes the target; B10 states
   exactly the term that would be lost.
8. **Precision floor:** a contraction witness theta<1 alone does not guarantee
   a binary64 target below F. Current original-residual acceptance remains the
   safeguard while the new outward implementation is developed.

## E. Future implementation and discriminating validation

The minimal first implementation is an opt-in endpoint-only compiler on a
declared low-degree affine model, with exact-rational coefficient oracles.
Keep the two-channel scalar path, the shared causal factorization and direct
forward stage path as comparators. Add near-affine omission only after model
remainder/domain providers and coefficient/evaluation enclosures are available.

Test axes: d=1 and d>1; nonnormal exact annihilator; nonzero relation defect;
stored-coefficient perturbation; endpoint versus embedded/dense outputs;
zero/small denominator; h/norm/epoch mismatch; full-space versus projected
relation; near-affine remainder; cold/warm preparation; refused plan plus
fallback; residual/update precision floor; callback effects and failures.
Measure scalar preparation, operator/RHS actions, residual certification,
allocations and whole-call time separately. A cheaper certificate can be worth
more than a saved product; neither is demonstrated by this proof alone.

`CHECK.py` is prospective source, intentionally unexecuted by this worker.
It independently solves small full-space rational stage systems, compares
the compiled output and original residual, checks the nonzero near-structure
defect and nonlinear remainder, and verifies inexact correction identities.
It does not prove arbitrary floating-point enclosure, generic nonlinear
temporal order, or production performance. Parent registration must precede
its recorded scientific execution.
