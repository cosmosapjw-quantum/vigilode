# D3: represented dense curves for the guarded Riccati consumer

## Scope and provenance

This continues the existing D3 S0 contract at vigilode@d702012e06eeafda5ce30b7c017e51b2c9bcbe4c. D2 stage generation, full33 admission and Session remain byte-identical to the published parent. The existing native dense_output_v2.rs, generic_verified.rs and packet paths already contain residual transport, inherited radius and prepare-before-commit principles. The new work connects those principles to the fixed D2 manufactured diagonal model and its cubic/quintic endpoint Hermite curves. It does not qualify the reserved native driver, arbitrary callbacks or actual BASS.

The original S0 archive and its complete bytes were restored. No S1 implementation or execution record was present in that durable checkpoint. Current native owner/reservation/live dirty state remains unresolved. The research subtree can be developed independently under the user's request; native source and D2 source are preserved. A same-branch expected-SHA update will detect published concurrent changes; unpublished work remains unknown.

## R1–R3: fixed model, data and claim

All manufactured coordinates are normalized. Physical time in this fixture is normalized, so rates have inverse-time units and alpha has inverse-(state*time) units before normalization. For j=0,...,29,

    z'_j = -lambda_j z_j - alpha z_j^2,
    lambda_j in {1,20,300}, alpha in {0,4}.

The initial condition is exactly the represented binary64 array returned by the pinned trajectory::initial. This defines the IVP; uncertainty in a different physical/model initial condition is outside this fixture. Its positive exact solution is decreasing and remains in the nonnegative orthant. The three conservation complements in the affine lift are

    Y23=1-z23, Y25=0.5-z24-z25, Y28=T-z26-z27-z28,

where T is the exact real value represented by the binary64 literal 0.2. The other full33 entries copy the corresponding compact coordinates. Since compact coordinates decrease, initial complement positivity persists for the exact IVP. Actual BASS units and nonlinear coupled invariants are separate obligations.

The target is a represented polynomial p on each stored interval [t0,t1], with a uniform full33 absolute error bound not exceeding the frozen budget, plus checked returned queries. Endpoint generator accuracy is not used as a proof of global dense accuracy. The certificate uses the original field defect of the actual stored curve and the inherited initial error radius.

## R4–R5: construction of immutable curves

Let Delta=t1-t0 denote the exact difference of stored binary64 endpoint times. For theta=(t-t0)/Delta,

    p(theta) = sum_{i=0}^n b_i B_i^n(theta),
    B_i^n(theta)=C(n,i) theta^i (1-theta)^(n-i), n=3 or 5.

All b_i are the actually stored binary64 controls. Once constructed, controls, model alpha, degree and time endpoints are immutable. End controls are exact copies of the accepted old state and trial candidate. Constructor arithmetic may round: certification refers to the resulting represented polynomial. Derivatives and queries enclose the exact Delta, rather than assuming the rounded proposal h equals it.

Cubic construction uses b0=z0, b1=fl(z0+h f0/3), b2=fl(z1-h f1/3), b3=z1. Quintic construction uses

    b0=z0, b1=fl(z0+h f0/5),
    b2=fl(z0+2 h f0/5+h^2 g0/20),
    b3=fl(z1-2 h f1/5+h^2 g1/20),
    b4=fl(z1-h f1/5), b5=z1,

with f(z)=-lambda*z-alpha*z^2 and g=Jf=(-lambda-2alpha*z)f. These are endpoint Hermite formulas; no stage refitting or exact-solution fallback is used. The exact unrounded formulas interpolate endpoint value/first derivative, and the quintic also interpolates second derivative. Wolfram verifies those algebraic identities. The actual rounded curve is separately certified by its defect.

### Bernstein arithmetic and domain

The derivative has degree n-1 controls n(b_{i+1}-b_i)/Delta. Degree m controls a_i elevate to degree N by

    a_tilde_k = sum_i [C(m,i) C(N-m,k-i)/C(N,k)] a_i.

For degree-n polynomials with controls a_i,b_j, their product has degree-2n controls

    c_k = sum_{i+j=k} [C(n,i) C(n,j)/C(2n,k)] a_i b_j.

All binomial integers in this unit are exactly representable. Ratios and arithmetic are enclosed with the existing directed::Interval operations. No rounded binomial ratio is used as an exact real coefficient.

For theta in [0,1], the Bernstein basis is nonnegative and sums to one. Therefore nonnegative controls certify p_j(theta)>=0. The affine lift preserves the same degree, so interval-enclosed complement controls with lower bound >=0 certify the full33 curve domain. This is a sufficient test: a negative control or a lower bound below zero causes DomainNotCertified, not a theorem that the curve is negative. No subdivision is required for the frozen cases; a failed sufficient certificate is a recoverable attempt refusal.

Finite samples are insufficient. The polynomial q(theta)=(theta-1/10)(theta-3/20) is positive at {0,1/4,1/2,3/4,1}, with values {3/200,3/200,7/50,39/100,153/200}, but q(1/8)=-1/1600. This elementary discriminator is a test of sampling versus continuous certification, not a new no-go result.

### Original-field defect and inherited error

Compute the degree-2n residual polynomial

    r_j(t)=p'_j(t)+lambda_j p_j(t)+alpha p_j(t)^2.

Derivative elevation, linear elevation and the product formula give outward intervals for every residual coefficient. Let rho_j be the maximum of their absolute endpoint bounds. Convexity yields |r_j(t)|<=rho_j throughout the stored time interval.

Let e_j=z_j-p_j. Exact subtraction gives

    e'_j = -[lambda_j+alpha(z_j+p_j)] e_j-r_j.

Exact z_j is nonnegative by the IVP invariant; p_j is nonnegative by the independent polynomial domain test. Hence the coefficient in brackets is >=lambda_j. The integrating-factor variation-of-constants formula gives, for 0<=s<=Delta,

    |e_j(t0+s)| <= exp(-lambda_j s) E0_j
                      + [(1-exp(-lambda_j s))/lambda_j] rho_j.

This does not require the uncertainty tube p±E to stay positive, so there is no circular use of the desired error estimate. The incoming E0 must already enclose this same nonnegative exact IVP; arbitrary signed initial balls are not covered.

The comparison solution is monotone toward rho_j/lambda_j. Its maximum on a segment occurs at an endpoint. With outward endpoint bound E1_j, uniform U_j=max(E0_j,E1_j). The native directed::exp_interval implementation encloses exponential values by range reduction and an interval Taylor remainder. The existing signed contracting-field transport is reused: upper gain at the upper exponent, lower exponential at the lower exponent, response <=min(Delta,(1-exp_lower)/lambda). E1 is the upward sum of transported incoming radius and defect response. Generic libm exp is not an enclosure provider.

At the initial time E0=0 exactly. After accepted commit E0_next is copied bit-for-bit from E1. An ignored initial radius is demonstrably wrong even for alpha=0 and zero-defect p: a perturbed initial value produces E0 exp(-lambda*s). Rejection leaves accepted state, E0, query cursor and saved scientific history unchanged.

### Full33 error and actual query values

For retained rows, the error radius is U_j. Complement rows use upward sums U23, U24+U25 and U26+U27+U28. Constant affine offsets contribute no error between the exact trajectory and exact represented curve.

For each represented query time tq, an interval encloses exact theta=(tq-t0)/Delta. Interval de Casteljau evaluates the exact stored curve at that theta, and the affine lift is performed in intervals. A separate binary64 de Casteljau/lift produces the returned center Yhat. For each full33 component, qround bounds the displacement between Yhat and the exact represented curve. The returned radius is upward(U_full+qround), and the query is refused unless it meets the original absolute budget and the returned represented state passes the existing domain check.

The full curve U_full is checked before commit. Every requested query falling in the candidate interval and one internal query are prepared and checked before Session::finish can commit. A subsequent query also performs its own displacement/budget check and may refuse. No query time is made a mandatory integration landing point.

## R6–R9: implementation obligations and evidence

New dense.rs forbids unsafe and reuses unchanged directed.rs and domain.rs. A separate crate-level forbid build checks this safe core. The research runner still uses D2 admission::environment, whose MXCSR asm stays in the inherited research boundary. This work does not close the production FP-provider gate. The bound assumes the checked round-to-nearest, gradual-underflow environment and compiler settings are maintained throughout certificate/query arithmetic; the immutable model introduces no external callback into that path.

The frozen 8 new runs are alpha0/4 × cubic/quintic × budget2e-6/2e-8, t0=0,tf=.25,h0=.004,attempt cap20000, rtol=budget,atol=budget*1e-3. D2 proposal cap1.8/max(lambda+2alpha*z) remains a nonlinear heuristic. Exact solution values are used only by the saved-output reviewer, never by candidate construction, controller or acceptance.

Validation uses focused new Rust contracts, an independent Fraction power-basis residual reconstruction, Decimal80 exact-solution query checks, preserved failed attempts and independent source/math review. Existing D2 campaigns and prior unsafe-boundary tests are reused, not rerun. Work counters are compared as workloads; no wall-time speed claim is made.

## R10: limits and next dependency

Claims are conditional mathematical derivation plus scoped implementation verification for this fixed model, curve construction, environment and frozen corpus. Degree-elevation, interval arithmetic and dissipative defect transport are established prior methods, reused here. No novelty or strong no-go claim is submitted. Actual BASS, native owner clearance/integration, general callback/Jacobian domain, external callback side effects and general finite retry termination remain independent gates. The current D3 branch is a research continuation, not a takeover of the reserved native checkout.
