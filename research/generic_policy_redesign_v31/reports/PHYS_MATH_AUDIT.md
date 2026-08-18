# PHYS-MATH AUDIT — v3.1 Vector Nonlinear-Remainder Geometry

## Verdict

**Mechanism PASS; generic safety-witness claim REJECTED.**

The directional telemetry is mathematically well-defined on the declared physical-component prefix and adds no solver action. However, the five predeclared vector scalars do not survive cross-dimension/family robustness. In addition, raw Euclidean directional cosines are not invariant under arbitrary component-wise state rescaling, so they could not have become a generic policy authority without a second scale-aware formulation even if the discovery ranking had looked stronger.

## Definitions

For physical components only,

\[
\chi_{23}=\frac{D_2\cdot D_3}{\|D_2\|_2\|D_3\|_2},\quad
\chi_{34}=\frac{D_3\cdot D_4}{\|D_3\|_2\|D_4\|_2},\quad
\chi_{24}=\frac{D_2\cdot D_4}{\|D_2\|_2\|D_4\|_2}.
\]

The derived witnesses are

\[
q_{34,\perp}=\sqrt{\max(0,1-\chi_{34}^2)},\qquad
\Delta\chi=\chi_{34}-\chi_{23}.
\]

Zero norm or nonfinite input is fail-closed (`null`). Any augmented clock coordinate is excluded by reusing the same physical prefix as the tolerance-scaled defect telemetry.

## Theorem — leading Hessian direction degeneracy

Assume an autonomous smooth vector field `F` with `F in C^3` near `y_n`, and write

\[
D(u)=F(y_n+u)-F(y_n)-J_nu.
\]

If the pexprb54s4 internal stage increments satisfy

\[
u_i=U_i-y_n=c_i hF_n+O(h^2),
\]

and

\[
v=F''(y_n)[F_n,F_n]\neq0,
\]

then

\[
D_i=\frac12 c_i^2 h^2v+O(h^3).
\]

Consequently

\[
\widehat D_i=\widehat v+O(h),
\]

and for any two such stages

\[
\chi_{ij}=1+O(h^2),\qquad
q_{ij,\perp}=O(h).
\]

Thus raw directional cosines are *asymptotically degenerate near +1* in the ordinary nondegenerate small-step regime. They can only expose higher-order departures from the common Hessian direction, and those departures are intrinsically small.

### Proof sketch

Taylor expansion gives

\[
D(u)=\frac12F''(y_n)[u,u]+O(\|u\|^3).
\]

Substituting `u_i=c_i hF_n+O(h^2)` yields the stated `D_i`. Normalizing a nonzero vector perturbed by `O(h)` changes its direction by `O(h)` in the tangent space of the unit sphere; the cosine between two such unit directions differs from one only at second order.

## Data alignment

The theorem predicts the observed concentration:

- safe `chi23`: approximately `[0.9940, 1]`;
- unsafe `chi23`: approximately `[0.99978, 1]`;
- safe `chi34`: approximately `[0.99754, 1]`;
- unsafe `chi34`: approximately `[0.99944, 1]`.

The discovery data therefore do not contradict the asymptotic argument; they illustrate its information bottleneck.

## Scale-invariance caveat

For a diagonal component transformation `y -> S y`, Euclidean angles between `D_i` generally change unless `S` is a scalar multiple of an orthogonal matrix. Therefore raw `chi_ij` is not a coordinate-/unit-invariant generic ODE statistic under component-wise rescaling. A future generic witness should either use the solver's tolerance metric or an explicitly declared problem metric.

## Special cases / counterexamples

1. If `v=F''[F,F]=0` or is very small, higher-order terms can dominate the direction and the theorem's common-direction conclusion is not uniform.
2. `chi_ij≈1` does not imply local admissibility: the N=256 HIRES unsafe rows are essentially perfectly collinear.
3. Non-collinearity does not imply inadmissibility: safe rows span a wider directional range than the unsafe rows.
4. The observed orientation of `chi34` reverses by dimension: lower values rank the lone N=96 unsafe event, while higher values rank N=256 unsafe events.

## Claim ceiling

Allowed:
- direction telemetry is numerical-work-neutral;
- raw `D2,D3,D4` directions are highly collinear on this discovery corpus;
- direction alone does not repair the scalar safety-classification problem.

Not allowed:
- a generic `chi34`, `q_perp`, or `Delta chi` threshold;
- a family-independent vector-direction safety certificate;
- opening the independent N=192 calibration on the basis of this node.
