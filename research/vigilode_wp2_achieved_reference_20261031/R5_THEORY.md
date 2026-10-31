# VigilODE WP2 | Continuous-time reference-defect enclosure for matched achieved work–precision

Date: 2026-10-31 KST. Parent `cosmosapjw-quantum/vigilode` Draft PR #158, immutable HEAD `72bd52d6936b923bd110222f36df79172b9ced49`. Protected original coupled nonlinear RHS-only inverse-Picard source and production RODAS5P. **Standalone research**; official VigilODE DAG **7/23**, successor admitted 0/12, distinct-author C9 0/1 NOT_RUN; no production merge/default/tolerance change.

## R1–R3: target, source, literature boundary

The four frozen WP1 native cohorts are `asymmetric_2`, `asymmetric_2_refined`, `coupled_3_cubic`, and `linear_exchange_control`, with 5 source-owned caller-error budgets per arm and 40 previously executed binary64 Rust traces. Original `SWEEP.log` SHA256 `9dae90079893170edddcf5cf8af193e53b43ab5bcf0c436cb81ce7aa0f8790b0`, previous **selected best pairs** frozen by original `WP_SUMMARY.json` SHA256 `56746487ac5c5c8702b53ef0ffd47694c040ab4e0adcfea1ec25aa3b8e9ba909`. New calculations MUST NOT re-select the four historical pairs to claim they were preregistered. Raw timings must remain the frozen WP1 measurements, not new WP2 timings.

Source ODE, treating every represented binary64 coefficient as an exact dyadic real, is

\[
\dot y_i=f_i(y)=-\kappa_i y_i-g_i y_i^3+\sum_{j\ne i}c_{ij}(y_j-y_i),\qquad \kappa_i>0,\ g_i\ge0,\ c_{ij}\ge0.
\]

The initial state satisfies \(0\le y_i(0)\le M=\max_i y_i(0)\). Continuum clock is exact \(t_n=n h\), where \(h\) is the original represented binary64 coarse step; separately account for represented output-label clock displacement. Positive cooperative source and inward boundary derivative make \([0,M]^d\) forward invariant. This is the **same source** as the WP1/PR156 coupled cubic model; no new physical source, closure, timestep, integrator, or callback.

A posteriori defect-based bounds for contractive ODEs have an established literature (see SIAM DOI `10.1137/0732001`, *A Posteriori Error Bounds and Global Error Control for Approximation of Ordinary Differential Equations*, and Hundsdorfer 1994, DOI `10.1093/imanum/14.3.363`). The following degree-9 exact-binary-rational Hermite/Bernstein construction and numerical result are **derived and computed for this scoped source**, not literature novelty claims.

## R4–R5 Theorem 1: one-sided max-norm dissipativity and continuous defect

Let \(u,v\in\mathbb R^d\), \(z=u-v\), and \(i\) realize \(|z_i|=\|z\|_\infty\). Since the real cubic is monotonically increasing on all of \(\mathbb R\),

\[
\operatorname{sgn}(z_i)[f_i(u)-f_i(v)]
\le -\kappa_i |z_i|+\sum_{j\ne i}c_{ij}(|z_j|-|z_i|)
\le -\kappa_{\min}\|z\|_\infty.
\]

For any *continuous, piecewise \(C^1\)* numerical interpolant \(X(t)\), and its source-original continuous-time defect \(r(t)=\dot X(t)-f(X(t))\), max-norm Dini differential inequality yields

\[
D^+\|X(t)-y(t)\|_\infty
\le-\kappa_{\min}\|X(t)-y(t)\|_\infty+\|r(t)\|_\infty.
\]

Consequently, if \(\|r(t)\|_\infty\le R_n\) on \([t_n,t_{n+1}]\), 

\[
 e_{n+1}\le e^{-\kappa_{\min}h}e_n+
 \frac{1-e^{-\kappa_{\min}h}}{\kappa_{\min}}R_n.
\]

Using the exact inequalities \(e^{-x}\le(1+x)^{-1}\) and \((1-e^{-x})/\kappa\le h\) for \(x=\kappa h\ge0\) gives the rigorously conservative but fully rational recurrence

\[
 \delta_0=0,\qquad
 \boxed{\delta_{n+1}=\operatorname{up}\!\left(
     \frac{\delta_n}{1+\kappa_{\min}h}+hR_n\right)}.
\]

The division/combined expression is evaluated using exact `Fraction` arithmetic over represented inputs; `up` returns the smallest finite binary64 ≥ that exact result, with a tested `nextafter` correction. A separate rational upper for \((1+\kappa_{\min}h)^{-1}\) is used; hence every stored \(\delta_n\) is a mathematically valid upward enclosure. This bound is independent of whether the predictor was RK4, DOP853, midpoint, or garbage values. A deliberately degraded predictor must yield a correspondingly larger \(R_n\) or fail closed.

## R5 Theorem 2: exact rational degree-nine Bernstein residual

On a fine exact dyadic panel \([t_j,t_{j+1}]\) of exact length \(h_f=h/2^m\), let \(s=(t-t_j)/h_f\in[0,1]\), and fix arbitrary *represented* binary64 reference endpoint vectors \(x_0,x_1\) (interpreted as exact rationals). Define \(f_0=f(x_0),f_1=f(x_1)\) by exact source-rational arithmetic, and cubic Hermite interpolant

\[
P_i(s)=x_{0i}+(h_ff_{0i})s+
 (-3x_{0i}-2h_ff_{0i}+3x_{1i}-h_ff_{1i})s^2+
 (2x_{0i}+h_ff_{0i}-2x_{1i}+h_ff_{1i})s^3.
\]

It exactly matches both endpoint states and their **exact-source** derivatives. Since \(f\) is cubic, each component of original defect \(r_i(s)=P_i'(s)/h_f-f_i(P(s))\) is an exact rational polynomial of degree ≤9, \(r_i(s)=\sum_{j=0}^{9}a_{ij}s^j\). Convert, in **exact `Fraction` rational arithmetic**, to degree-nine Bernstein coefficients

\[
 b_{ik}=\sum_{j=0}^{k}a_{ij}\frac{\binom{k}{j}}{\binom{9}{j}},\qquad 0\le k\le 9.
\]

The Bernstein basis is nonnegative and sums to one on \([0,1]\), so the exact rational

\[
\boxed{R_j=\max_{i,k}|b_{ik}|\ge\sup_{s\in[0,1]}\|r(s)\|_\infty}
\]

rigorously encloses the **entire continuous panel** without a point sampling heuristic, and without assuming RK4 truncation order. Independent exact-evaluation tests check equality at rational \(s\in\{0,1/7,1/2,5/7,1\}\), including intentionally false zero-defect and false parity tests. Each panel's source state, clock and coefficient bits determine its mathematical object.

## R5 Theorem 3: achieved-error interval and frozen-pair admission

For each candidate native Rust trajectory \(A_n\), the true achieved peak max-norm error is \(e_A=\max_n\|A_n-y(t_n^{\rm label})\|_\infty\). Let \(\widehat e_A=\max_n\|A_n-X(t_n)\|_\infty\), computed **exactly** as a maximum of dyadic differences of original candidate and represented-reference states. The rational reference-defect recursion supplies a rigorous bound \(\delta=\max_n\delta_{n2^m}\) at all coarse grid nodes. Since original labels differ from mathematical \(t_n=nh\), include

\[
 \delta_{\rm clock}=\max_n|t_n^{\rm label}-nh|\,F_{\max},\qquad
 F_{\max}=\max_i[\kappa_iM+g_iM^3+2M\sum_{j\ne i}c_{ij}],
\]

valid on the proven positive invariant box, and define \(\epsilon_{\rm ref}=\delta+\delta_{\rm clock}\). Then

\[
\boxed{\max(0,\widehat e_A-\epsilon_{\rm ref})\le e_A\le\widehat e_A+\epsilon_{\rm ref}.}
\]

The original WP1-frozen pair \((A,B)\) is **rigorously within 1% in true peak achieved error** if both lower endpoints are strictly positive and

\[
 \boxed{\max\left(\frac{u_A}{\ell_B},\frac{u_B}{\ell_A}\right)\le1.01}.
\]

All comparisons are exact rational; printed decimals are presentation-only. The original PR156 source's BE physical error certificate remains independent and unchanged. This conditional conclusion is *true achieved peak global error*, not simply the same requested tolerance. It is scoped to the frozen source and comparison grid; neither RODAS8 production nor untested interpolation beyond the panels is certified.

## Original WP1-frozen-pair result, native source reused without mutation

Original GitHub 40 traces are pinned by SHA above. The four *prior-selected* alpha IDs and derived source calls are not re-optimized:

| Cohort | WP1 frozen alpha IDs (baseline, fast) | Source calls | Defect panels (micro=32) | Rigorous reference delta grid | Worst possible ratio upper | Rigorous 1% verdict |
|---|---|---|---:|---:|---:|---|
| asymmetric_2 | 1,1 | 396→297 | 2048 | 7.8005609e-13 | 1.000003429774 | PASS |
| asymmetric_2_refined | 0,0 | 810→602 | 4096 | 1.1570015e-13 | 1.000011941479 | PASS |
| coupled_3_cubic | 1,2 | 784→546 | 4096 | 4.3589693e-13 | 1.000015041309 | PASS |
| linear_exchange_control | 0,0 | 530→402 | 4096 | 2.5505123e-14 | 1.000000001015 | PASS |

The max clock terms are separately enclosed (≤4.0e-17 for these four), included in the above ratio bounds. Exact full **micro=32** total panel count is **14,336**, not 16,384. The previous live progress update stated 16,384 by arithmetic error; this document corrects that explicitly. Additional independently generated **micro=64** reference validation is performed for each cohort as a sensitivity and full mathematical certification repetition, rather than using a numerical grid-difference as a proof. An observed decrease of delta on refinement is an empirical feature, not a mathematical prerequisite for any of these certificates.

## R6 same-author adversarial findings and scope decisions

1. **BLOCK-NOW if missing:** source/clock provenance: retain exact WP1 executed SHA256 for `SWEEP.log` and prior frozen pair summary SHA256, including original root/point bits and exact represented clock shift. Do not reselect four winners with a new reference.
2. **No false zero defect:** cubic Hermite endpoint construction enforces \(r(0)=r(1)=0\). A falsification test must modify the interior (for example \(s=1/2\)); an earlier self-test incorrectly demanded a nonzero endpoint and was corrected before any claim.
3. **Exact rational arithmetic vs floating candidate:** all original polynomial coefficients and the Bernstein bound are exact dyadic rationals. Float RK4 only selects candidate reference states; it does not certify error. Decimal/Double precision refinement alone is insufficient.
4. **Conservative recurrence:** do not replace \(hR\) by \(hR/(1+\kappa h)\), which is in general too small. The chosen recurrence uses exact \(1/(1+\kappa h)\) contraction upper and **unscaled** \(hR\).
5. **Separate metrics:** identical achieved *peak-global* error within 1% does not mean identical pointwise error trajectories, terminal error, work-precision curves, or equal-cost calibration. Reference computation/validation and previous 100-pair sweep are not charged into a chosen one-run original timing ratio. Preserve all WP1 negative alpha=1 linear-control outcomes.
6. **Independence:** same-author mathematical review and Wolfram symbolic checks are not distinct-author C9. No biological/physical application beyond the original selected source.

## Admission/stop

Mathematical status `DERIVED`; exact-binary-rational script results `NUMERICALLY_CHECKED` and, once linked to a particular CI/artifact, `IMPLEMENTATION_VERIFIED_SCOPED`; public theorem literature-supported for residual/dissipative analysis only. No changing production source, code defaults, official DAG or publication gate; no first-party RODAS8 matched work-precision result. The next genuine scientific step is independently source-owned original eight-stage RODAS5P work–precision with matched achieved error and actual production-stage error budget before promoting an algorithm claim.

`OFFICIAL_VIGILODE_DAG=7/23; CENTRAL_REI_DAG=28/88_INFO_ONLY; SUCCESSOR_ADMITTED=0/12; C9_DISTINCT_AUTHOR=0/1_NOT_RUN; C10_NOT_STARTED; MERGE_READY=false; PRODUCTION=HOLD; NO_DEFAULT_CHANGE=true; NO_TOLERANCE_RELAXATION=true.`
