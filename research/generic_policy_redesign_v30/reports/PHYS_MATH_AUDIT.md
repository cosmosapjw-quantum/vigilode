# PHYS–MATH AUDIT — v3.0 Stage-Trajectory Geometry Discovery

## Verdict

**NEGATIVE DISCOVERY / FEATURE FAMILY NOT PROMOTED.**

The reconstructed v3.0 contract predeclared three tolerance-scaled nonlinear-defect magnitudes and the two log-slopes

\[
s_{23}=\frac{\log(\rho_3/\rho_2)}{\log(c_3/c_2)},\qquad
s_{34}=\frac{\log(\rho_4/\rho_3)}{\log(c_4/c_3)},
\]

with

\[
\kappa_{234}=s_{34}-s_{23},\qquad
(c_2,c_3,c_4)=(1/4,1/2,9/10).
\]

No numerical threshold was selected on the N=96/256 discovery replay.

## Definitions / dimensions

- Each \(\rho_i\) is a WRMS-like tolerance-scaled defect and is dimensionless under the existing compatible-state scaling contract.
- The ratios \(\rho_3/\rho_2\) and \(\rho_4/\rho_3\) are dimensionless.
- The log-slopes and \(\kappa_{234}\) are therefore dimensionless.
- The shape quantities are undefined when any \(\rho_i\le0\) or is nonfinite; the implementation returns `null` rather than imputing a value.

## Known limit

If, locally in stage fraction \(c\),

\[
\rho(c)=Kc^p(1+o(1)),
\]

then

\[
s_{23}\to p,\qquad s_{34}\to p,\qquad \kappa_{234}\to0.
\]

Thus \(\kappa_{234}\) measures a failure of a single local power-law description across the three stage fractions. This is a shape diagnostic, not a local-error theorem.

## Discovery data

- 48 scored events, 5 audit-E-inadmissible.
- All 48 had finite positive \(\rho_2,\rho_3,\rho_4\) and finite shape quantities.
- \(\kappa_{234}\) pooled orientation-free ROC AUC: 0.665116.
- Cross-dimension orientation-free AUC: 0.791667 at N=96 (only one unsafe event), 0.684211 at N=256.
- Leave-one-family-out minimum orientation-free AUC: 0.533333 when the nonautonomous family is removed.
- Unsafe \(\kappa_{234}\) values occur on both sides of zero; the safe range overlaps the unsafe range.

## Load-bearing counterexample

The five unsafe events have \(\kappa_{234}\) spanning negative and positive values. Therefore neither sign of curvature nor a one-sided monotone \(\kappa_{234}\) rule is a safety certificate on this discovery corpus. The safe cloud also spans both signs and contains values beyond multiple unsafe values.

## Interpretation

The three scalar norms contain some ranking information, but the family-removal and dimension split show that the information is not robust enough to open independent numerical calibration. The next missing quantity is plausibly **directional geometry of the nonlinear remainders** rather than another scalar transformation of their norms.

The scalar norm sequence discards whether \(D_2,D_3,D_4\) remain aligned, rotate, or partially cancel. Two trajectories can therefore have similar norm slopes while carrying different vector information into the endpoint combination.

## Claim ceiling

Allowed:
- three-stage scalar geometry is measurable with zero extra solver work beyond the already-purchased prefix/audit path;
- \(\kappa_{234}\) carries weak-to-moderate ranking information on the pooled discovery replay;
- the predeclared scalar feature family fails the family-robust discovery gate.

Not allowed:
- \(\kappa_{234}\) is a generic safety witness;
- any numerical threshold on \(s_{23},s_{34},\kappa_{234}\);
- opening N=192 calibration from this feature family;
- active switching.
