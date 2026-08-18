# NEXT NODE — v3.1 Vector Nonlinear-Remainder Geometry Discovery

## Parent

Use the final committed v3.0 negative-discovery HEAD only.

## Why this node

v2.9 rejected a one-interval scalar stage-growth witness. v3.0 retained the full three-stage scalar norm shape, but the best predeclared curvature feature remains family-sensitive and does not robustly separate safety.

The remaining information loss is directional: \(\rho_i\) retains only the norm of each nonlinear remainder and discards how \(D_2,D_3,D_4\) rotate or cancel in state space.

## Predeclare before any new output

Using the same physical-component prefix used by the tolerance-scaled telemetry, compute read-only vector geometry from already-existing \(D_2,D_3,D_4\):

\[
\chi_{23}=\frac{D_2\cdot D_3}{\|D_2\|\|D_3\|},\quad
\chi_{34}=\frac{D_3\cdot D_4}{\|D_3\|\|D_4\|},\quad
\chi_{24}=\frac{D_2\cdot D_4}{\|D_2\|\|D_4\|}.
\]

Also predeclare at most two derived low-complexity directional witnesses, for example

\[
q_{34,\perp}=\sqrt{\max(0,1-\chi_{34}^2)}
\]

and a signed turn-change

\[
\Delta\chi=\chi_{34}-\chi_{23}.
\]

Do not add a learned classifier or a family identifier.

## Data role

- N=96/256: already-consumed feature-discovery replay only.
- N=192: remains sealed until one vector witness form survives discovery robustness checks.
- N=384: final safety holdout remains sealed.
- N=2048: remains sealed.

## Kill questions

1. Does vector-direction information materially rank the five unsafe events beyond scalar \(\kappa_{234}\)?
2. Does the ranking survive leave-one-family-out, especially removing the nonautonomous family?
3. Is the direction consistent between N=96 and N=256?
4. Is the telemetry exactly numerical/work neutral?

## Forbidden

- N=192/384/2048 execution before a witness form survives discovery;
- threshold fitting on N=96/256;
- high-capacity classifiers;
- family-specific whitelists/blacklists;
- active switching or runtime E-K activation.
