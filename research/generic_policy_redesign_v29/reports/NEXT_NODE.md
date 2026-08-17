# NEXT NODE — v3.0 Stage-Trajectory Safety Geometry Discovery

## Parent

Use the final committed v2.9 negative-calibration HEAD only.

## Status inherited from v2.9

- A34 scalar monotone safety rule: **REJECTED**.
- N=384 / rtol=7e-6 safety holdout: **UNOPENED / SEALED**.
- N=2048: **SEALED**.
- k=3 upstream R-side event: fixed development condition, not promoted.
- active switching: forbidden.

## Objective

Determine whether the *shape* of retained nonlinear-defect growth across c2=1/4, c3=1/2, c4=9/10 contains safety information that A34 discards.

## Predeclared candidate feature family

Retain raw/scaled stage quantities before any new solver output:

\[
\rho_2,\rho_3,\rho_4,
\]

\[
s_{23}=\frac{\log(\rho_3/\rho_2)}{\log(c_3/c_2)},
\qquad
s_{34}=\frac{\log(\rho_4/\rho_3)}{\log(c_4/c_3)},
\]

\[
\kappa_{234}=s_{34}-s_{23}.
\]

Also retain R-side target error, h, family, dimension and prefix work as audit covariates, not automatic authority features.

## Data roles

1. Re-run the already-consumed v2.9 N=96/256 profiles only as **feature-discovery replay** with richer retained telemetry. They may not calibrate the final safety rule.
2. Freeze one low-complexity witness family from that discovery replay.
3. Use a newly predeclared calibration profile not previously run to choose any numerical threshold.
4. Preserve N=384 as the final sealed safety holdout.
5. Keep N=2048 sealed.

## Kill questions

1. Does kappa234 separate or materially rank the five v2.9 unsafe events without merely encoding family identity?
2. Does the signal survive leave-one-family-out and cross-dimension checks?
3. Can a low-complexity witness obtain nontrivial coverage with zero calibration unsafe after multiplicity correction?
4. Does the pathwise prefix budget still hold?

## Forbidden

- fitting a high-capacity classifier on 48 events;
- using N=384 during feature discovery or threshold calibration;
- active switching or forced-switch order tests;
- family-specific whitelist/blacklist as a substitute for a physics witness;
- claiming conformal/formal coverage without validating sampling assumptions.

## Promotion

Only after a newly calibrated low-complexity stage-trajectory witness survives the still-sealed N=384 holdout may the project open a frozen read-only full-E shadow policy node.
