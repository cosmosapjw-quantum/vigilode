# D3 dense-curve certification: independent derivation and primary-source review

## Scope and result

Review date: 2026-10-24 (Asia/Seoul).
Pinned D3 contract: vigilode@d702012e06eeafda5ce30b7c017e51b2c9bcbe4c,
research/d3_dense_certificate_20261024/CONTRACT.md, Git blob
9672dafc2a98a0b61e03a4c387b81dc0ea8f9a33.

The contract is S0 only and freezes eight new manufactured runs:
alpha in {0,4}, cubic/quintic dense curves, full33 absolute curve budgets
{2e-6,2e-8}; tf=.25, h0=.004, attempts cap20000. Existing D2 endpoint
solver and existing dense_output_v2/provider machinery are to be reused.
No experiment, solver, test, compiler, benchmark, NCP, or Wolfram execution
was performed by this reviewer. This report independently derives and
checks the mathematical contract; implementation status is NOT_REVIEWED.

Conclusion: the proposed dissipative defect enclosure is DERIVED under the
conditions below. It supports continuation of the existing D3 node.
It does not close the native ownership, safe FP-provider, general callback,
Jacobian, dense BASS, or arbitrary side-effect gates.

## 1. Precise scalar theorem

Let h>0, lambda>=0, alpha>=0 be fixed real parameters. The field is

f(y)=-lambda*y-alpha*y^2.

Let y solve y'=f(y) on [t0,t0+h], with its actual y(t0)>=0.
Let p be a C1 represented polynomial curve on that interval, with p>=0.
Let |y(t0)-p(t0)|<=E0 and a certified D bound satisfy

|r(t)| <= D,  r(t)=p'(t)+lambda*p(t)+alpha*p(t)^2.

Then, for 0<=s<=h,

|y(t0+s)-p(t0+s)| <= exp(-lambda*s)*E0
                        + D*(1-exp(-lambda*s))/lambda,   lambda>0,

and <=E0+s*D when lambda=0.

Independent derivation: write e=y-p. Exact algebra gives

e' = -[lambda+alpha*(y+p)]*e-r.

The exact solution remains nonnegative: zero is an equilibrium and local
uniqueness prevents crossing it; on the nonnegative half-line y'<=0, so y
remains bounded and extends for all finite forward times. Hence
a(s)=lambda+alpha*(y+p)>=lambda. The integrating-factor identity is

e(s)=exp(-integral_0^s a)*e(0)
     -integral_0^s exp(-integral_u^s a)*r(u) du.

Taking absolute values gives the asserted bound. This proof avoids
differentiating |e| at zero. For alpha=0 the algebra itself does not require
nonnegativity, but D3 can retain one uniform domain contract.

No circularity: y>=0 follows from the actual IVP and p>=0 is certified
separately from the represented curve. Positivity of the entire tube
[p-E,p+E] is not needed to establish this scalar estimate. A later domain
admission requiring p-E>=0 is a separate, stronger output requirement.
If uncertain initial data are intended, the supported family is restricted
to y(t0)>=0; an error ball extending below zero does not by itself establish
the sign of every member.

The coefficients have dimensions [lambda]=T^-1 and
[alpha]=Y^-1 T^-1; [r]=Y/T, [D]=Y/T, and [E0]=[E]=Y.
For the dimensionless manufactured model these same relations reduce to
dimensionless variables and time. The lambda->0 limit is E0+sD.
For D=0 only inherited initial error remains.
For E0=0 the error approaches D/lambda from below when lambda>0.

The enclosure is a standard dissipative a posteriori estimate specialized
to this quadratic scalar field, not a novelty claim. Neumaier [N] gives the
one-sided Lipschitz/logarithmic-norm framework, local-to-global propagation,
and computable enclosing tubes.

## 2. Bernstein implementation form

Define theta=(t-t0)/h and

p(theta)=sum_{i=0}^n b_i B_i^n(theta),  n=3 or 5.

Since B_i^n>=0 and sum_i B_i^n=1 on [0,1],

min_i b_i <= p(theta) <= max_i b_i.

For interval coefficients [l_i,u_i] containing the exact coefficients,
min_i l_i and max_i u_i give enclosing bounds. Nonnegative lower controls
are a sufficient whole-segment domain certificate. A negative control
is inconclusive about the polynomial's actual sign. Subdivision may
tighten the bound; any subdivision depth budget must return UNKNOWN or
a typed undecided result when it is exhausted, rather than claim a sign.

Derivative coefficients in theta are n*(b_{i+1}-b_i). Physical time
derivatives require division by h. The square p^2 has degree 2n and
coefficients

c_k = sum_{i+j=k} b_i*b_j
      * binom(n,i)*binom(n,j)/binom(2n,k).

For degree elevation from n to m>=n, controls become

e_j = sum_i b_i*binom(n,i)*binom(m-n,j-i)/binom(m,j),

where binomial coefficients outside their natural ranges vanish.
Elevate p and dp/dtheta to degree 2n. Build physical-defect controls

d_k = (1/h)*Elevate(dp/dtheta)_k
      +lambda*Elevate(p)_k+alpha*c_k.

Exact rational arithmetic or a proven outward enclosure of each d_k
yields D>=max_k |d_k| over the entire segment. Equivalently build the
normalized defect R=dp/dtheta+h*lambda*p+h*alpha*p^2 and use D_R=h*D.
The normalized endpoint bound is

E1 <= exp(-x)*E0 + phi1(-x)*D_R,
x=lambda*h, phi1(-x)=(1-exp(-x))/x, phi1(0)=1.

Garloff [G] and Narkawicz et al. [B] supply primary Bernstein range,
degree-elevation, and subdivision results. This report gives the exact
scalar specialization above independently.

## 3. Cubic and quintic endpoint controls

These formulas describe the ideal algebraic construction; the certificate
must be computed for the controls actually stored.

Cubic Hermite controls:
b0=y0,
b1=y0+h*f0/3,
b2=y1-h*f1/3,
b3=y1.

Quintic Hermite controls, with g0=y''(t0), g1=y''(t1):
b0=y0,
b1=y0+h*f0/5,
b2=y0+2*h*f0/5+h^2*g0/20,
b3=y1-2*h*f1/5+h^2*g1/20,
b4=y1-h*f1/5,
b5=y1.

For the autonomous quadratic model,

g=Jf=(-lambda-2*alpha*y)*(-lambda*y-alpha*y^2).

The endpoint first- and second-derivative identities follow from
p_theta'(0)=n(b1-b0),
p_theta'(1)=n(b_n-b_{n-1}),
p_theta''(0)=n(n-1)(b2-2b1+b0), and its right endpoint analog.
A rounded control set defines its own exact polynomial; its residual
must be recomputed from that set. One must not certify the ideal controls
and then evaluate different rounded controls.

## 4. Global inheritance, full33 mapping, and query output

For contiguous segments with the next p(0) equal to the previous stored
endpoint, E0_next must enclose E1_previous. If there is a representation
mismatch delta between these endpoints, add |delta|.

A decisive analytic failure case for resetting E0 is alpha=0 and p itself
an exact solution started from a perturbed initial value. Then r=0, while
the error is E0*exp(-lambda*s). Resetting E0 to zero would falsely certify
zero error. This is a local counterexample to that bookkeeping rule, not
a new no-go result.

For an exact affine lift x=A*z+b, componentwise reduced bounds E_i imply

|x_j-xhat_j| <= sum_i |A_ji|*E_i.

Lift coefficient uncertainty and actual floating evaluation error must be
included where applicable. The full33 budget norm and denominator must
follow the existing contract exactly. A reduced norm alone does not
certify an asserted full33 output budget.

The theorem bounds the mathematical represented p. A returned floating
value q at requested time t needs

|y(t)-q| <= E(t)+eta_query,

where eta_query encloses basis evaluation, time normalization, and lift
rounding. Certifying p(theta_float) is insufficient if the claimed time
is a different exact t. Outward interval evaluation or exact rational
evaluation of the stored binary floats can establish eta_query.

Time identity is material: if t1=fl(t0+h_proposed), then t1-t0 need not
equal h_proposed as real numbers. Either define h as the exact difference
of stored endpoints and use that same h in theta and the physical defect,
or enclose the mismatch explicitly.

## 5. Numerical proof obligations for code review

1. All parameters, endpoints and stored controls used by certificate and
   query evaluator refer to the same represented data.
2. Nonfinite inputs and unsupported signs are typed failures.
3. The derivative factor 1/h is consistent with represented time.
4. Domain bounds and defect coefficients are exact or outward-enclosed.
5. The inherited radius is preserved across acceptance, rejection and yield.
6. The accepted curve and certificate commit atomically; rejection cannot
   replace the accepted prefix/radius.
7. Query arithmetic/lift error is added to the solution-error certificate.
8. Transcendental evaluation has a proved enclosure if exponential
   contraction is used. Ordinary libm exp/expm1 plus an arbitrary one-ULP
   adjustment is not automatically a rigorous enclosure.
9. Safe conservative alternative under lambda>=0: E<=E0+sD, computable
   with exact/outward arithmetic. This may be wider and must retain the
   original tolerance if it rejects more.
10. For constant D and lambda>0, the exact majorant
    E(s)=D/lambda+(E0-D/lambda)*exp(-lambda*s) is monotone, so its maximum
    over a segment is max(E0,E(h)). This statement requires enclosing
    the endpoint evaluation correctly.
11. No domain inference from finitely sampled queries alone.
12. Independent exact-flow references are audit-only as required by S0.

## 6. Primary-source review and scope

[N] Arnold Neumaier, Global, rigorous and realistic bounds for the
solution of dissipative differential equations. Part I: Theory,
Computing 52 (1994), 315–336, DOI 10.1007/BF02276880.
Author full text: https://arnold-neumaier.at/ms/ode.pdf
Read: §§2.6–2.9 (contraction and inherited-error propagation), 3.3–3.5
(comparison/tube theorem), 4.9 (polynomial approximation), and example4.11.
Supports the dissipative residual-error framework. The exact quadratic
formula in §1 was independently derived above.
An older author-domain URL failed with CRAWL_NOT_FOUND; the current
author-domain PDF succeeded. This was an access failure, not a theorem
or implementation failure.

[G] Jürgen Garloff, The Bernstein Algorithm, Interval Computations
No.2 (1993), pp.154–168 (PDF copyright line 1994).
https://interval.louisiana.edu/reliable-computing-journal/1993/interval-computations-1993-2-pp-154-168.pdf
Read: primary range enclosure, degree-elevation and subdivision discussion.
Supports whole-interval coefficient bounds.

[B] Anthony Narkawicz et al., Bounding the Range of a Rational Function
over a Box, Reliable Computing 17(1) (2012), pp.34–39.
https://kops.uni-konstanz.de/bitstreams/b007dfd0-8a25-4c1f-b255-5fa65bc7cc22/download
Read: §2 equations(1)–(5), §3 remarks4–5.
Supports the polynomial Bernstein enclosure and derivative coefficients.
The rational-function extension is not needed for D3.

[F] Rida T. Farouki, The Bernstein polynomial basis: a centennial
retrospective, author-hosted presentation.
https://faculty.engineering.ucdavis.edu/farouki/wp-content/uploads/sites/51/2021/07/Bernstein-polynomial-basis.pdf
Read: de Casteljau triangular construction and subsegment controls.
Used as author exposition; the primary algorithm claims are grounded in G/B.

[P] Stephan Nüßlein, Hendrik Ranocha, David I. Ketcheson,
Positivity-preserving adaptive Runge–Kutta methods,
CAMCoS 16 (2021), 155ff., DOI 10.2140/CAMCOS.2021.16.155.
Author preprint https://arxiv.org/html/2005.06268
Read: §§1–2 and formulae2.3–2.6.
It changes RK weights after computing stage derivatives to enforce bounds.
It is a distinct approach and is not adopted because D3 freezes the
tableau and endpoint candidate. Its endpoint correction also does not
supply D3's represented continuous-curve error certificate.

SciSpace returned Neumaier, rigorous defect-control work, and other
a-posteriori papers. Only the primary documents actually read above
support the adopted mathematical statements. Search hits by themselves
are not verification.

Search accounting:
- Exa: 4 queries, requested 5 each = 20 result slots (not 20 distinct
  verified primary sources).
- SciSpace: 2 natural-language queries, 10 records each.
- Exa fetch: 3 batches of 3 URLs = 9 URL requests, including one failed
  outdated Neumaier URL; successful readable documents were preserved
  as scratch source receipts.
- This is a targeted mathematical contract review, not an exhaustive
  literature survey or novelty adjudication.

## 7. Callback literature retained as separate open gate

The original task also requested recoverable callback failure contracts.
Official SUNDIALS sources were searched/read before D3 was identified:

https://sundials.readthedocs.io/en/latest/cvode/Usage/index.html
https://sundials.readthedocs.io/en/latest/cvode/Mathematics_link.html
https://computing.llnl.gov/projects/sundials/usage-notes

These provide established recoverable RHS/Jacobian return semantics,
separate error/convergence/repeated-RHS exhaustion, and constraints.
The LLNL usage notes explicitly caution against modifying the input state
inside the RHS callback. No general external I/O/RNG rollback guarantee
was established. These findings remain contextual for a separate native
callback gate; they do not change the D3 S0 scope.

## 8. Review disposition

- Scalar comparison theorem: PASS_SCOPED, DERIVED.
- Bernstein/defect computational identities: PASS_SCOPED, DERIVED plus
  primary literature support.
- Cubic/quintic endpoint identities: PASS_SCOPED, directly checked algebraically.
- Global inheritance/time/query obligations: explicit acceptance conditions.
- New implementation/test/campaign status: NOT_RUN by this reviewer.
- Native owner/dirty completion: UNRESOLVED_INPUT_DEPENDENCY.
- Strong no-go/new-method novelty: no such claim submitted.
- Production/global completion: inherited HOLD/null.

Scratch raw fetch files contain third-party full texts for internal
inspection. The public deliverable should use this synthesis, source URLs,
and receipt metadata/hashes rather than re-publish whole third-party texts.

