# Original A09: represented K target and H5 closure

This module addresses the original represented-stage obligation in addition to the window successor. It does not import exact causal claims from a lower-triangular ideal tableau, and it does not change or bless the legacy `homotopy_step` linearized diagnostic.

The sealed source is a `CollectiveModel` in the zero-carrier frame, its represented initial vector y, positive represented h, all loaded binary64 alpha/beta/gamma/b weights, and a frozen represented Jacobian J obtained by the model's JVP on basis vectors. The returned immutable object owns these exact coefficients and J; the theorem treats those stored entries as exact real constants. It certifies the nonlinear stage equation with the actual collective polynomial f. It does not claim every arbitrary floating callback equals that polynomial or that a dynamically reevaluated JVP is an exact linear map. Such callback operations only generate the candidate.

With W=I-h gamma J define, for 0<=lambda<=1,

  Phi_lambda(K)_i = W^{-1}[h f(y)+h J sum_j(beta_ij-gamma delta_ij)K_j
                   +lambda h {f(y+sum_j alpha_ij K_j)-f(y)-J sum_j alpha_ij K_j}].

This is theta=1 continuation of the nonlinear stage defect. At lambda=1 its fixed-point equation is exactly

  K_i-h J sum_j beta_ij K_j
      =h f(y+sum_j alpha_ij K_j)-h J sum_j alpha_ij K_j.

Every index j runs over ALL stages. All upper and diagonal entries are retained; stored `l` need not be substituted for the exact difference of stored beta and gamma. Autonomous f_t is zero. The lambda<1 family uses the same source polynomial; its root need not be computed accurately before proceeding because the final certificate encloses the entire parameter family on one common tube.

An approximate inverse V of the nearest represented W is only a candidate. Interval replay of the exact products defining W yields rho>=||I-VW||_infinity. If rho<1, W is nonsingular and

  ||W^{-1}||_infinity <= ||V||_infinity/(1-rho)=:g.

The certificate never treats the approximate inverse V as exact. All numerator/denominator operations in this bound are directed outward.

On the candidate-centered product of component boxes with declared stage radius R, interval evaluation of the actual cubic derivative gives

  L_i >= sup_{tube} ||Df(y+alpha_i K)-J||_infinity.

The real polynomial derivative is evaluated using the actual source weights and mean intervals, including every mixed term. The common-family Lipschitz majorant is

  A_ij = g h [ ||J||_infinity |beta_ij-gamma delta_ij| + L_i |alpha_ij| ].

This is uniform for lambda in [0,1]. It includes nonlinear dependence on upper/diagonal alpha leakage, not just an additive tableau perturbation.

For the common candidate c, interval residuals at lambda=0 and lambda=1 give

  d_i >= max_lambda ||Phi_lambda(c)_i-c_i||_infinity,

because each residual component is affine in lambda and its absolute value reaches its maximum on an endpoint. Residual evaluation includes the exact-source f(y), f(y+alpha c), J, W, and all coefficient products; finite candidate arithmetic therefore cannot create a false certificate.

Split A=B+E where B is strictly lower triangular and E contains the complete diagonal/upper part. Forward positive substitution constructs an upper enclosure of S=(I-B)^-1. Positive multiplication constructs T>=S E and q>=||S E||_infinity. If q<1, then

  (I-A)^-1 = (I-S E)^-1 S >=0.

For positive slack delta, compute nine positive terms for (I-T)^-1 S(d+delta 1), append the uniform geometric tail q^9 ||S(d+delta 1)||/(1-q), and obtain proposed component stage radii r_i. Final admission explicitly rechecks BOTH r_i<=R and d+A r<=r with directed operations. This recheck is essential: an upper enclosure of an algebraic inverse is not automatically a self-mapping radius after rounding.

Each Phi_lambda maps the resulting closed convex product box to itself, so Brouwer gives a root. For two roots, component distances e satisfy e<=Be+Ee, hence e<=S E e; q<1 implies equality only at e=0. The same bounds hold on one common tube for every lambda in [0,1]. The derivative I-DPhi_lambda is nonsingular by the same absolute-majorant argument; the implicit function theorem and uniqueness connect the roots along the full parameter interval. The returned stage radii bound the unique endpoint root relative to the actual returned K centers.

The output is y+sum_i b_i K_i. Its radius is sum_i |b_i|r_i plus the interval-enclosed displacement of the returned binary64 output evaluation. This is a certified distance to the exact REPRESENTED STAGE TARGET output. It is not a certificate of the RODAS discretization error relative to the continuous ODE; that obligation belongs to a trajectory verifier such as the independent packet route in the window client.

Cost: one dense inverse candidate and O(n^3) inverse verification; stage RHS evaluations run in parallel within each candidate round; candidate rounds/lambda stages remain sequential. Stage tube derivatives and residuals use dense interval matrix work. H5 has eight stage blocks, so the S/T/positive-series work is independent of mode dimension after constructing the majorant. The implementation makes no matched-time, allocator, or general high-dimensional efficiency claim.
