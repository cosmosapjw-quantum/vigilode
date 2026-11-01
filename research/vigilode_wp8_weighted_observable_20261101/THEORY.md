# WP8 source-specific weighted scalar contraction and exact error certification

### Exact mathematical model
Fix original source bits from WP1 and WP2. For n=2 or n=3 with k_i>0 and g_i,c_ij>=0,
y'_i = -k_i y_i - g_i y_i^3 + sum_{j != i} c_ij (y_j-y_i).
Let Q_ij=c_ij (i!=j) and Q_ii=-sum_j c_ij. Q has row sums zero but, in general, COLUMN sums are not zero. A strictly positive normalized left stationary vector pi solves pi^T Q=0, sum_i pi_i=1.

Exact source values:
- asymmetric_2 and asymmetric_2_refined: pi=(2/5,3/5)
- coupled_3_cubic: pi=(185/671,286/671,200/671)
- linear_exchange_control: pi=(1/2,1/2).

For W=pi^T y the exchange terms cancel EXACTLY:
W'=-sum_i pi_i (k_i y_i + g_i y_i^3).
The positive invariant cube [0,M0]^n (M0=max_i y_i(0)) yields 0<=W(t)<=W(0) exp(-k_min*t) and |W'|<=sum_i pi_i (k_i M0 + g_i M0^3). These equations are source-specific diagnostic physics, not a claim that W is conserved (it has sinks).

### Rigorous a posteriori weighted error
Let X be arbitrary piecewise cubic Hermite approximate trajectory interpreted as an exact dyadic real function on WP2's source grid. Define r=X' - f(X) and a_i=|X_i-y_i|. By global monotonicity of the cubic difference and the Dini derivative estimate:
D^+ a_i <= -(k_i + sum_j c_ij)a_i + sum_j c_ij a_j + |r_i|.
For E=sum_i pi_i a_i, pi^T Q=0 implies
D^+E <= -k_min E + sum_i pi_i |r_i|.
No assumption that numerical predictor X is of a particular formal order is used.

On a panel [t_n,t_n+h] with u=(t-t_n)/h, the exact residual r_i(u) has degree <=9. Let b_i,l be its exact degree-9 Bernstein coefficients in dyadic rational arithmetic. Set R_i(u)=sum_l |b_i,l| binom(9,l)u^l(1-u)^(9-l) so |r_i|<=R_i(u) for ALL u in [0,1].

For q=k_min h in [0,1], choose the even M=8 Taylor polynomial T_M(z)=sum_{j=0}^M (-z)^j/j!. Because exp(-z)<=T_M(z) for z>=0, the exact weighted bound obeys:
E_(n+1) <= T_M(q) E_n
            + h sum_i pi_i sum_l |b_i,l| sum_{j=0}^M (-q)^j/j! beta(10-l+j,l+1) binom(9,l).
Here binom(9,l)*beta(10-l+j,l+1)=binom(9,l)*l!*(9-l+j)!/(10+j)! is exactly rational.
Rounding the whole RHS upward to the least binary64 >= the exact Fraction at EACH micro panel yields a certified E_n for all coarse source grid labels.

### Clock and output interval claims
The user-observed solver time label bits can differ from the exact source grid t_i=i*h. Define dt=max_{all original WP1 outputs}|t_reported-t_i| as an EXACT dyadic fraction, and B_W=sum_i pi_i(k_i M0+g_i M0^3). A true weighted physical solution therefore changes by at most B_W*dt. Add this exact clock bound to the maximum certified coarse-grid E before upward binary64 conversion; never use round-to-nearest for an upper endpoint. To bound the maximum true achieved weighted error for each original solver trace Y, compute d=max_i |pi^T (Y_i-X_ref(t_i))| EXACTLY. Then [max(0,d-delta),d+delta] encloses the true max-in-time weighted output error over original solver output labels, with lower endpoint rounded DOWN and upper UP. Intersect the two independent micro32 and micro64 intervals.

### Validity and adversarial counterexamples
Uniform pi=(1/2,1/2) fails for asymmetric Q=[[-12,12],[8,-8]] because (1/2,1/2)Q=(-2,2) != 0. Replacing k_min by an unconditional weighted arithmetic mean of k is false: with k=(4,7), error e=(1,0), the instantaneous rate is -4, not -5.5. These candidates are REFUTED, not unresolved. If Q is reducible and no strictly positive stationary pi exists, this certificate refuses to treat uniform weights as a substitute. A compiler or code verification claim does not follow from a scalar algebra check.

### Actual evidence and limitations
Current source-matched run executed 43,008/43,008 WP2 reference panels (original micro32 and micro64 from byte-locked files) and generated all 56 weighted observable output traces from existing BE baseline 20, BE fast-first 20, and original native RODAS5P 16 (no new owner IVPs). Same-author 12/12 adversarial tests check source hashes, stationarity, beta positivity, clock/outward rounding, two-reference interval consistency, and negative weights/kappa-mean examples. Full binary/raw science outputs are in the separate WP8 archive; the compact Git RESULTS.json is only a digest-indexed summary. Newly claimed whole-ODE HOL kernel proof NOT_RUN; distinct-author C9 NOT_RUN. Different norms require careful interpretation: a smaller pi-weighted bound than an older infinity bound is not a direct statement of smaller infinity error.

References: Blanes, Iserles & MacNamara (2022), ESAIM M2AN 56:1843-1870 doi:10.1051/m2an/2022042; Akrivis, Makridakis & Nochetto (2009), Numer. Math.114:133-160 doi:10.1007/s00211-009-0254-2; Corless & Corliss (1991), Rationale for Guaranteed ODE Defect Control. Current construction needs no additional observational data.

Official VigilODE DAG 7/23; successor admitted 0/12; production HOLD, NO_MERGE / NO_DEFAULT_CHANGE / NO_TOLERANCE_RELAXATION / AUTO_NEXT_NODE=false. This is source-scoped theory/data, not a new production solver.
