# Nonlinear window client: constructive extension of PT2

Status: directly derived theory, not a native implementation or measured speed claim. Read-only source baseline `ac89a9ac` (`vigilode-readonly`); existing `research/cloud_completion_20261018/PARALLEL_THEORY.md` PT1/PT2 and `collective_model.rs` were read. No repository mutation or publication. This note derives only from that model; no literature lookup.

## 1. Domain, exact fine maps, and the inexpensive coarse map

Let there be N complex modes, positive stored weights w_n, S=sum w_n (do not silently set S=1), and weighted Hilbert norm |z|_w^2=sum w_n |z_n|^2. On each window consider

    y' = L y + lambda B(y),
    (Ly)_n=(-kappa+i omega_n)y_n,
    B(y)_n=i chi |m(y)|^2 m(y),  m(y)=sum w_n y_n,
    0<=lambda<=1, kappa>=0.

Here lambda is a homotopy parameter; it is held fixed through an IVP. It is not physical time. `carrier=0` is the currently admitted implementation route. Frequencies may be arbitrarily large finite real values in this mathematical result. Time has its original unit; kappa and |chi|S^2 R^2 have inverse-time units.

The exact coarse map is A_h=exp(hL), diagonal in complex coordinates. Its weighted norm is a_h=exp(-kappa h). The fine map F_{lambda,h} is initially the exact ODE flow; a repeated exact Strang map is treated separately below, with its ODE discretization error separately certified.

Set c=|chi|S^2 R^2 for an input energy ball |y(0)|_w<=R. The identities and bounds are

    |m(y)| <= sqrt(S)|y|_w,
    |B(y)|_w <= c R,
    ||DB(y)||_w <= 3c,
    mu_w(DB(y)) <= c,
    d/dt |y|_w^2 = -2 kappa |y|_w^2.

The energy identity follows because the nonlinear contribution is
2 Re[i chi |m|^2 conjugate(m)m]=0, independently of S. For the sharper logarithmic norm, the collective two-dimensional real Jacobian of i|m|^2m has symmetric eigenvalues +/-|m|^2; the averaging/replication pair contributes S in the normalized collective direction. Thus mu_w(DB)<=|chi|S|m|^2<=c. The factor 3 in the operator norm must not be substituted for this factor 1, nor vice versa.

Define d_kappa(t)=(1-exp(-2 kappa t))/(2 kappa), with d_0(t)=t. Energy decay and the logarithmic-norm differential inequality imply

    Lip(F_{lambda,h}) <= exp[-kappa h + lambda c d_kappa(h)].             (1)

No assumption that this is <1 is required below.

## 2. The coarse/fine defect is small without a small oscillation frequency

Let D_{lambda,h}=F_{lambda,h}-A_h. Variation of constants gives

    D(x)-D(y) = integral_0^h A_{h-s} lambda[B(F_s x)-B(F_s y)] ds.

Both exact trajectories have radius <=R exp(-kappa s), so the nonlinear Lipschitz constant at time s is <=3 lambda c exp(-2 kappa s). Their separation is bounded by (1) with time s. Therefore

    Lip(D_{lambda,h})
      <= 3 exp(-kappa h) [exp(lambda c d_kappa(h))-1] =: d.              (2)

Indeed the integrand is
3 lambda c exp(-kappa h) exp(-2 kappa s) exp(lambda c d_kappa(s)),
whose integral is the displayed exponential difference. Also

    sup_{|x|<=R} |D_{lambda,h}(x)|_w
      <= lambda c R exp(-kappa h) d_kappa(h).                           (3)

These bounds do not contain max |omega_n|. They isolate weak nonlinearity from rapid rotations. The bound (2) is a defect bound, not Lip(F), and not a tolerance certificate.

For an exact Strang step S_tau=A_{tau/2} B^flow_{lambda,tau} A_{tau/2}, repeated m times with tau=h/m, the same conservative defect bound (2) holds. Proof: let

    C_l=lambda c tau exp(-kappa tau) sum_{r=0}^{l-1} exp(-2 kappa r tau).

Energy decay of each split step gives the right intermediate radii. The telescoping identity

    S_tau^m-A_tau^m
       = sum_{l=0}^{m-1} A_tau^(m-1-l) (S_tau-A_tau) S_tau^l

has Lipschitz contribution bounded by
3 exp(-kappa h) exp(C_l)[exp(C_{l+1}-C_l)-1]. Summing telescopes to
3 exp(-kappa h)[exp(C_m)-1]. For kappa>0,

    C_m=lambda c d_kappa(h) [kappa tau/sinh(kappa tau)]
        <=lambda c d_kappa(h),

and the kappa=0 limit is equality. The pointwise startup bound also extends to the split map: each nonlinear kick changes its state by at most lambda c R times its effective kick duration, and energy is preserved at the kick; linear damped halves telescope the accumulated bound to <=lambda c R exp(-kappa h)d_kappa(h). In particular the total undamped coarse-orbit discrepancy is <=lambda c R t_j, as used in (7). This is a statement about exact split subflows; floating-point subflow errors and the split-vs-ODE error are additional terms.

## 3. Full-prefix projection gives a globally defined bounded client

Take M windows. Their exact fine maps F_j map energy balls X_j into X_{j+1}; use radii R_j=R_0 exp(-kappa t_j), or the conservative common radius R_0. Initial z_0=y_0 is fixed. Let P_j be the nearest-point projection onto X_j in the weighted Hilbert norm. It is nonexpansive and fixes the true fine orbit.

One correction round from z^k consists of:

1. Independently for every window, compute b_j^k=F_j(z_j^k)-A_j z_j^k.
2. Solve the affine recurrence exactly by a vector prefix:

       u_0=y_0,
       u_{j+1}=A_j u_j+b_j^k.

3. Only after the complete affine recurrence, set z_{j+1}^{k+1}=P_{j+1}(u_{j+1}).

Projection is NOT inserted inside the recurrence. Inserting it there destroys the affine prefix representation. Product projection after the scan keeps every next-round fine input in the assumed invariant ball.

The true fine orbit y_j is a fixed point. It is the unique fixed point even when the client is not a contraction: the first equation fixes z_1=F_0(y_0), then induction fixes z_2,...,z_M. A spurious homotopy branch cannot occur for this causal initial-value shooting system.

## 4. Finite propagation and factorial convergence without q<1

Write e_j^k=|z_j^k-y_j|_w, with e_0^k=0. If ||A_j||<=a and Lip(D_j)<=d, the unprojected prefix error obeys

    |u_{j+1}-y_{j+1}| <= a |u_j-y_j| + d e_j^k.

Nonexpansiveness of the final P_j yields the componentwise vector inequality

    e^{k+1} <= B e^k,
    B_{ji}=d a^(j-1-i) for 1<=i<j<=M, and 0 otherwise.                 (4)

B is strictly lower triangular, so B^M=0. Thus exact arithmetic gives the exact first k fine-window boundaries after k rounds, and the full fine orbit after at most M rounds, independent of any contraction bound. This remains true with product projection, because already correct boundaries lie inside their balls and projection fixes them.

For k>=1, direct multiplication or induction gives

    (B^k)_{ji}=d^k binom(j-i-1,k-1) a^(j-i-k),  j-i>=k.                (5)

For a<=1 and e_j^0<=E_0,

    e_j^k <= E_0 d^k binom(j-1,k)
             <= E_0 (M d)^k/k!.                                      (6)

This is useful even if q=(M-1)d>1: factorial growth eventually beats every fixed (Md)^k. It does not assert low round count for every parameter or speed improvement when the required count is comparable to M.

A sharper startup bound uses z_j^0=A_{t_j}y_0. In the undamped case (3) over total time t_j gives e_j^0<=R c h j for equal window size h. Equation (4), the binomial convolution identity, and a<=1 then give

    e_j^k <= R c h d^k binom(j,k+1),
    max_j e_j^k <= R c H (M d)^k/(k+1)!, H=Mh.                         (7)

Proof of the last convolution: sum_{i=1}^{j-1} binom(i,k+1)=binom(j,k+2). The initial case is binom(j,1)=j. With damping the conservative bound e_j^0<=Rc t_j still suffices, so (7) remains valid using global c and d.

For undamped weak nonlinearity theta=cH,

    d=3 expm1(theta/M), beta=M d,
    beta <=3 theta exp(theta/M).

Thus the number of rounds sufficient for a fixed error target can be independent of M and N when theta is fixed. This is the constructive regime for parallel-in-time benefit.

## 5. Optional contraction and blocking

The exact row sum in (4) gives

    q = d sum_{r=0}^{M-2} a^r,

with q=0 when M=1. If q<1, the projected map is a contraction in the maximum weighted norm, and the usual q^k bound is available as an alternative to (6)-(7). For undamped blocks of length H with cH<=1/12,

    q <= 3(M-1) expm1(1/(12M)) < 0.251

for all M>=2 (the looser 3 exp(1/24)/12<0.261 also suffices without optimizing constants). The uniform contraction is independent of mode count and frequency spread. A long trajectory can be divided into sequential blocks obeying this weak-nonlinearity condition, with independent windows within each block. Boundary uncertainty must be carried from block to block; blocking does not reset physical error.

There is no need to force kappa>c or claim the undamped flow itself is contractive. The linear coarse map has removed the dominant rotations and only the nonlinear defect drives the iteration.

## 6. Finite precision does not erase the constructive theorem

Require first that every represented next-round center belongs to the declared input ball: implement a certified inward projection with rounding slack, or project into a strictly smaller ball inside a fixed enlarged analysis ball. Merely approximating P within eta does not prove membership, and is insufficient for reusing its next-round Lipschitz constants. Under this membership condition, suppose each implemented correction map is within weighted max-norm eta_k of its ideal projected map. Include errors in the fine outputs, A_j evaluation/application, defect subtraction, vector prefix, and projection. Then

    e^{k+1} <= B e^k+eta_k 1.

For uniform eta and a<=1, (6) implies

    ||e^k||_inf <= E_0 beta^k/k! + eta sum_{r=0}^{k-1} beta^r/r!
                 <= E_0 beta^k/k!+eta exp(beta).                       (8)

Use (7) for the first term when its startup hypothesis applies. Exact finite termination becomes a precision floor, not an exact floating-point claim. If q<1, eta/(1-q) is another valid floor.

A fine-map error delta does not enter eta unamplified: the affine prefix contributes at most Lambda_A delta, where Lambda_A=sum_{r=0}^{M-1}a^r. For undamped windows Lambda_A=M. A valid budget is

    eta <= Lambda_A delta + sigma_vector_scan + sigma_projection,

with A/defect arithmetic errors included in delta or separately. Errors in A applied during the vector prefix must be bounded on the actual unprojected prefix states, which can exceed the input-ball radius; interval replay or an independent prefix-state bound is required. This prevents a false M-fold precision saving. Fine tolerance must be selected before the cost comparison; its cost can grow when M grows.

## 7. A final certificate independent of iterative contraction

At final candidate centers z_j, compute one independent fine evaluation (or reuse one only if its exact input identity matches the final center) with a verified physical-flow enclosure

    |F_j^ODE(z_j)-Fhat_j(z_j)| <= delta_j.

Define a rigorous defect bound

    r_j >= |z_{j+1}-Fhat_j(z_j)|+delta_j,

including norm/subtraction rounding. Then true ODE endpoint errors satisfy

    E_0 >= |z_0-y_0|,
    E_{j+1} = L_j E_j + r_j,
    L_j=exp[-kappa h_j+c_j d_kappa(h_j)], c_j=|chi|S^2 R_j^2.            (9)

All L_j,r_j must be fixed certified quantities on the declared energy balls before parallel composition. PT1 scalar outward affine prefix then gives a valid complete endpoint certificate. No denominator 1-q, no Banach hypothesis, and no trust in a residual-only stopping test are needed. If r_j includes a split-map-to-ODE bound, the certificate is about the ODE; otherwise it only certifies the discrete fine orbit.

For a continuous local curve v_j(s), separately bound |v_j(s)-F_s^ODE(z_j)|<=delta_j(s), including returned-center displacement. Then

    |v_j(s)-y(t_j+s)| <= L_j(s) E_j+delta_j(s).

Mean errors are <=sqrt(S) times the weighted norm bound; an individual mode error is <=E/sqrt(w_n). This must be mapped to the unchanged API tolerances. A uniform mesh endpoint certificate is not automatically a continuous-trajectory certificate.

## 8. The vector prefix is a new primitive, not PT1 reused by renaming

Each A_j is complex diagonal. Represent a vector affine map by (q_j,b_j), one complex q and translation b per mode. The composition is

    (q_2,b_2) after (q_1,b_1)=(q_2*q_1, q_2*b_1+b_2),

componentwise in mode. Work is O(MN), storage O(MN), and no dense 2N-by-2N matrix is formed. A balanced tree has O(log M) affine-composition depth, but each composition is O(N) if modes are serial: native span is then O(N log M). Parallel mode lanes can reduce this arithmetic depth, with their scheduling/reduction costs counted. Existing scalar `AffinePrefixWorkspace` certifies nonnegative scalar bounds; it does not implement this signed complex vector prefix.

A fixed binary tree and outward vector enclosures can bound rounding without assuming floating-point associativity. Directly summing transformed translations using A_j^{-1} needs an additional overflow analysis under damping and is not the default proposal.

## 9. Full work/span and a nonempty speedup regime

Let C_F^par be an upper time cost of one parallel fine window at the chosen accuracy, C_F^seq a lower time cost of the matched serial fine window, and C_V an upper final physical-flow verification cost per window, C_scan(M,N,p) the vector-prefix wall-time bound per round, C_proj the all-window projection bound, C_setup initialization/coefficient/workspace cost, C_cert the final scalar-prefix and outputs cost, and C_sync communication/scheduling cost. With p window workers,

    T_parallel <= C_setup
       + K[ceil(M/p) C_F^par+C_scan+C_proj+C_sync]
       + ceil(M/p) C_V+C_cert.                                       (10)

Total fine work is K M fine-window evaluations (plus M final verifications), not reduced relative to serial. Parallelization reduces critical path, subject to (10). Serial baseline must solve the same model at the same certified error, and its lower comparator is M C_F^seq. A parallel upper time cost is not a lower serial time cost. The gate is the right-hand side of (10) <M C_F^seq.

Concrete nonvacuous sufficient regime: kappa=0, M=64, p=32, theta=cH=1/12, target iteration error epsilon=1e-8 R, and linear coarse initialization. Then

    d = 0.003908794235659535,
    beta = 0.2501628310822102,
    q = 0.24625403684655067,
    K=7,
    R theta beta^7/8! = 1.267236614567818e-10 R.

If eta<=epsilon/(4 exp(beta))=1.9466849510526445e-9 R, (8) adds at most epsilon/4. Allocate the remaining budget to ODE discretization/representation and final certification; those are not free. In an ideal work-cost example let a single matched fine-window cost be C_F (exact cost, not merely an upper serial time bound). Equivalently for wall time require C_F^par<=C_F<=C_F^seq. If additionally C_V<=C_F and all non-fine/non-verification costs in (10) total <=8 C_F, then

    T_parallel <= 14 C_F+2 C_F+8 C_F=24 C_F <64 C_F,

a sufficient speedup of at least 8/3 relative to that matched serial baseline. This is an algebraic conditional guarantee, not a measured machine result. The inequality is realizable in a work/span model when each fine window has enough substeps to dominate O(N) scan/projection/setup; actual hardware cost bounds must be measured before a production speed promotion. The ODE-verification cost condition must also be checked, not assumed because split subflows are explicit.

The floating numbers above were computed by a small Python evaluation of expm1/binomial bounds; their role is illustrating the theorem's nonempty parameter regime. They are not interval-certified binary constants for a native admission gate.

## 10. When a homotopy path earns its additional cost

For lambda-dependent fine maps, use d_lambda=3 a expm1(lambda c d_kappa(h)). The unique fine orbit is causal for every lambda; continuation is not needed to avoid multiple IVP branches. Direct lambda=1 correction is the default if (7), (9), and (10) already meet the target.

A previous certified lambda state with error e_prev and orbit sensitivity bound S_lambda supplies next-stage initial error <=e_prev+S_lambda Delta_lambda. A first-order predictor can reduce this to e_prev+Delta_lambda e_tangent+(V/2)Delta_lambda^2 if a certified tangent and second-derivative bound V are available. Projection cannot increase this error because the true orbit remains in the ball.

For the undamped physical flow from a lambda-independent initial condition, u=partial_lambda y and v=partial_lambda^2 y solve

    u'=(L+lambda DB(y))u+B(y), u(0)=0,
    v'=(L+lambda DB(y))v+2 DB(y)u+lambda D^2B(y)[u,u], v(0)=0.

Using ||D^2B||<=6c/R and mu<=lambda c gives

    |u(t)| <= R [exp(lambda c t)-1]/lambda,
    |v(t)| <= (6R/lambda^2) exp(lambda c t)
                         [exp(lambda c t)-1-lambda c t],              (11)

with lambda=0 limits Rct and 3Rc^2t^2. Derivation of v: insert the u bound into
integral exp(lambda c(t-s))[6c|u|+6lambda c|u|^2/R] ds; the integrand simplifies to
(6cR/lambda)exp(lambda ct)[exp(lambda cs)-1]. The displayed result follows by elementary integration. These conservative finite bounds provide an actual predictor remainder, rather than naming a homotopy technique without a gate.

To select a path, calculate each stage's K_l from (7)/(8) or q_l, including its inherited state and tangent errors. Charge all low-lambda solves and predictor work. Admit a continuation schedule only if

    sum_l T_stage(lambda_l,K_l)+T_predictors < T_direct(lambda=1,K_direct)

under comparable certified error and cost bounds, or if it opens an explicitly demonstrated admissible domain that direct target correction cannot yet reach. A smaller final warm-start error alone is not evidence of a net speed gain. Sensitivities already required by a parameter study may be reused with provenance; they are not assigned zero cost merely for this comparison.

## Implementation status and next scope

New theory: defect bound for exact flow and exact repeated Strang maps; projected vector-prefix client; nilpotent finite-front/factorial error theorem without q<1; explicit finite-precision floor; final noncontractive causal ODE certificate; conditional full-cost speedup example; parameter predictor remainder.

Existing implementation: scalar outward error prefix, represented model, and previously reviewed scoped packet/structured drivers. Not implemented by this note: exact split nonlinear-flow primitive with outward transcendental evaluation, signed complex vector affine prefix, product weighted projection, full nonlinear client, its ODE split error certificate, or runtime costs. A scalar radius scan does not fill these missing components.
