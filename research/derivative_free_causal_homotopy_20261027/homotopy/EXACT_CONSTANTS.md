# Proof-derived constants and a physical-accuracy consumer

## Additive implementation and exact scope

The user's new requirement replaces empirical scientific margins with constants
derived from numerical analysis, algebra and analysis. The previous heuristic
arms remain unchanged controls. The additive native implementation is in
`rhs_homotopy.rs`:

- `RhsAcceleration::ResidualDominance` uses the actual residual dominance
  condition with eta=1. It ignores the coefficient-magnitude cap, observed
  ratio threshold, gain margin and rejected-trial count. Finite arithmetic and
  domain membership still determine whether a trial can be evaluated.
- `CubicDecayAccuracyPlan::new` receives the physical absolute error budget,
  polynomial model parameters, horizon and caller resource limits. It computes
  the time grid and algebraic budgets from analytic inequalities.
- `integrate_cubic_decay_accuracy` returns a model-specific outward physical
  error bound for every returned grid observation. It evaluates the original
  polynomial stage residual in outward intervals and includes state-addition
  and observation-time rounding displacement.

The model is the represented polynomial `y'=-k y-g y^3`, `k>0,g>=0,y0>=0`.
No black-box RHS oracle is assigned an unproved global Lipschitz constant.
The physical result belongs to this sealed model and the documented directed
binary64 arithmetic assumptions, not to the generic fixed-point interface.

## Sharp domain constants

Write `Phi(y)=k y+g y^3`, and let `Y=y0`. The true positive solution decreases
inside `[0,Y]`. On this interval the exact extrema are

\[
M_1=\Phi(Y),\qquad L=\Phi'(Y)=k+3gY^2,\qquad
M_2=L\Phi(Y),
\]
\[
M_3=\Phi''(Y)\Phi(Y)^2+\Phi'(Y)^2\Phi(Y).
\]

They follow directly from `y'=-Phi`, `y''=Phi' Phi`, and
`y'''=-Phi'' Phi^2-Phi'^2 Phi`, whose nonnegative magnitudes increase with y.

The implicit-midpoint stage residual has the global inverse denominator

\[
D=1+hk/2,\qquad
|K-K^*|\le |F(K)|/D,
\]

because `F'(K)=1+h[k+3g(y+K/2)^2]/2>=D`. The lower bound is attained at
midpoint state zero. For stage increment K in the exact bracket between
`-2 yhat` and zero, the direct-map Lipschitz factor is exactly

\[
q(\widehat y)=h[k+3g\widehat y^2]/2.
\]

The plan uses `B=Y+epsilon` and the upward bound
`q_B=h[k+3gB^2]/2`. The forthcoming global error induction proves
`|yhat|<=B` at accepted nodes. When `q_B<1`, the direct map preserves its
increment bracket, since `h(k+g yhat^2)<=2`. This analytically justified
domain belongs only to the new planned consumer; it is not imposed on the
older noncontractive control, which is a different root-solver experiment.

## A finite-step truncation bound, with its coefficient proved

Let y0,y1 denote an exact ODE step, let `ym=y(h/2)`, and
`mc=(y0+y1)/2`. The exact increment's midpoint residual is `A-Bq`, where

\[
A=h[\Phi(m_c)-\Phi(y_m)]\ge0,\quad
B_q=\int_0^h\Phi(y(t))\,dt-h\Phi(y_m)\ge0.
\]

The first inequality uses convexity of the exact trajectory; the second uses
convexity of `Phi(y(t))`. The endpoint-average interpolation error has integral
remainder constant 1/8, and midpoint quadrature has constant 1/24. Therefore

\[
0\le A\le h^3 L M_2/8,
\qquad0\le B_q\le h^3 M_3/24.
\]

For the cubic polynomial,

\[
2\Phi'(Y)^2-\Phi''(Y)\Phi(Y)
=2k^2+6kgY^2+12g^2Y^4\ge0.
\]

Thus `M3<=3 L M2`. Since both defects are nonnegative, the magnitude of their
difference is bounded by the larger upper bound, giving

\[
|F(K_{\rm exact\ ODE})|\le C h^3,
\qquad C=\frac{L^2\Phi(Y)}8.
\]

This is a proved analytic upper bound for finite steps, not a fitted factor
or a claim that C is the optimal truncation constant. The corresponding
ideal midpoint local error is at most `C h^3/D`.

## Global propagation and deriving every scientific budget

Let `Psi_h(y)` be the exact implicit-midpoint step. Its midpoint variable m
satisfies `m+(h/2)Phi(m)=y`, hence

\[
\Psi_h'(y)=\frac{1-(h/2)\Phi'(m)}{1+(h/2)\Phi'(m)},
\qquad |\Psi_h'(y)|\le1.
\]

This nonexpansivity is global, including negative approximate states. Only
the true exact ODE orbit is used in the truncation constant. If a returned
binary64 step differs from `Psi_h(yhat_n)` by at most alpha_n, induction gives

\[
|\widehat y_n-y(nh)|
\le n C h^3/D+\sum_{j=1}^n\alpha_j.
\]

The plan chooses the **smallest dyadic** N satisfying both

\[
E_{\rm trunc}=N C(T/N)^3/[1+kT/(2N)]<\epsilon,
\qquad q_B<1.
\]

The dyadic class is an explicit representable-grid design constraint, not an
empirically tuned safety factor. The code checks that represented division
and multiplication enclose the same exact h and `Nh=T`; unsupported
underflow/overflow is refused. It does not claim minimality over all integer
grids or globally optimal total work.

The remaining budget is computed by directed rounding:

\[
\epsilon_{\rm alg}=\epsilon-E_{\rm trunc},\qquad
\alpha_{\rm budget}=\epsilon_{\rm alg}/N,
\qquad \tau_F=D\alpha_{\rm budget}.
\]

There is no guessed epsilon/2 allocation, convergence ratio, gain threshold,
or station spacing. The generic candidate initially targets the derived
tau_F. Then an outward interval of the original represented polynomial gives

\[
\alpha_n\le \operatorname{mag}(\boldsymbol F(K_n))/D_{\rm lower}
+|\operatorname{fl}(\widehat y_n+K_n)-(\widehat y_n+K_n)|.
\]

Only this latter bound authorizes the physical step. If needed, monotone
derivative-free bisection polishes the root in its analytic bracket. If the
represented interval residual cannot fit the remaining budget, the attempt
returns an explicit rounding-floor failure rather than enlarging tolerance.
The exact initial point and all previously accepted points remain available.

The code sums actual outward alpha values and checks the requested physical
cap at every output. Rounded time labels contribute the additional bound
`Phi(Y)*|fl(nh)-nh|`, calculated from an interval for the exact time. For the
executed T=1 dyadic cases all time labels are exact.

All elementary enclosures use `rodas5p_core::directed`. Their contract assumes
correct IEEE binary64 operations, correctly rounded FMA, gradual underflow,
and absence of reassociation that invalidates error-free transforms. The
candidate's ordinary residual need not itself prove contraction or root
accuracy; the final original-polynomial enclosure is the physical authority.

## Continuation choice

This consumer directly solves lambda=1. For the anchor homotopy,
`q_lambda=lambda q_1`; intermediate contraction cannot change the final
factor. Here the analytically selected h already supplies `q_1<1`, so extra
stations have no role in the convergence guarantee and their extra work is
avoided. Other continuation families remain available through the generic
API but require their own constants and branch conditions.

## Actual execution and results

`EXACT_CONSTANTS_PREREGISTRATION.md` preceded the run. Sixteen contracts passed
on the extended module. One additional focused rounding-floor contract then
passed, covering the previously unexercised interval polish/refusal path:
**17 distinct native contracts PASS**. The original eleven controls are included
in the sixteen-test run; they must not be counted twice.

All 12 new physical-accuracy arms completed. The same derived plan is used
for Picard/ResidualDominance and zero/warm starts at each physical tolerance.

| Physical cap | Derived N | Derived residual target | Zero/Picard RHS + verifier | Warm/ResidualDominance RHS + verifier |
|---:|---:|---:|---:|---:|
| 1e-3 | 64 | 1.12767219543e-5 | 192 + 64 = 256 | 134 + 64 = 198 |
| 1e-4 | 128 | 2.25508213043e-7 | 430 + 128 = 558 | 334 + 128 = 462 |
| 1e-5 | 512 | 1.08191743493e-8 | 1580 + 512 = 2092 | 1147 + 512 = 1659 |

The total number of polynomial evaluations decreases by 22.66%, 17.20%, and
20.70%, respectively. Ordinary RHS and interval evaluations have different
costs, so these counts are retained separately and are not translated into
wall-clock speed. No bisection polish was needed in these 12 positive arms.

For the strictest cap, the warm/ResidualDominance maximum observed grid error
is approximately `7.08647025e-7`, and the final outward physical bound is
`6.09514438e-6`, both below `1e-5`. Decimal80 independently checked every grid
point against the exact continuous solution, all derived constants and
remaining-budget inequalities, and minimality within the dyadic class.

On these rapidly contractive derived grids, ResidualDominance and Picard use
the same number of map calls for each seed choice. The savings in the table
come from accepted-stage reuse under the proof-derived physical budget; the
data do not attribute them to Anderson extrapolation. The earlier slow-stage
controls retain their separate evidence for acceleration.

## Review, provenance and resumption

The theory agent independently checked the induction, exact domain constants,
finite-step truncation derivation and outward validation structure. The parent
also reviewed the interval code and nonexpansivity argument. These are
complementary native-agent reviews with inherited context, not a claim of
independent model families.

Evidence: `EXACT_CONSTANTS_EXECUTION.json`, `EXACT_CONSTANTS_RESULTS.json`,
`EXACT_CONSTANTS_SUMMARY.json`, `ROUNDING_FLOOR_TEST.json` and their raw logs.
`analyze_exact_constants.py` is the independent retained checker. The new
example is `cubic_accuracy_bench_20261027.rs`. The older `RESULTS.json` and its
source-bound receipts remain historical controls; no old performance campaign
was silently rerun or re-labelled as this new physical-accuracy comparison.

Resource limits (16384 steps and 1,000,000 polynomial evaluations in the
example) are caller-owned execution bounds and have no role in the scientific
constants. No production default or generic whole-ODE certificate was changed.
