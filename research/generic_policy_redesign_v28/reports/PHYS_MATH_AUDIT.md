# PHYS–MATH AUDIT — v2.8 staged pexprb54s4 level-2 safety probe

## Scope and authority

This audit covers only the read-only dependency-level-2 prefix of fused `pexprb54s4` on the frozen v2.8 campaign. It does **not** authorize active switching, a production threshold, or a full-E safety claim.

The implemented stage data are

\[
U_3=y_n+h\left[c_3\varphi_1(c_3hJ_n)F_n+a_{32}\varphi_3(c_3hJ_n)D_2\right],
\]
\[
U_4=y_n+h\left[c_4\varphi_1(c_4hJ_n)F_n+a_{42}\varphi_3(c_4hJ_n)D_2\right],
\]
with
\[
(c_3,c_4)=\left(\frac12,\frac9{10}\right),\qquad
(a_{32},a_{42})=\left(4,\frac{2916}{125}\right),
\]
and
\[
D_i=F(U_i)-F_n-J_n(U_i-y_n),\qquad i=3,4.
\]

The campaign diagnostics are the tolerance-scaled nonlinear defects
\[
\rho_i=
\left[\frac1m\sum_{a=1}^{m}
\left(\frac{|h|D_{i,a}}
{\mathrm{atol}+\mathrm{rtol}\max(|y_{n,a}|,|U_{i,a}|)}\right)^2\right]^{1/2}.
\]

## Dimensions and scaling

For compatible component units,
\([D_i]=[y]/[t]\), so \(|h|D_i\) and the tolerance scale both have units of state. Hence every component contribution to \(\rho_i\) is dimensionless.

As in v2.4/v2.7, this does **not** make the statistic invariant under arbitrary component-wise reparameterization when scalar `atol` is used. That remains a claim ceiling.

## Proposition 1 — small-h stage-amplification baseline

Assume:

1. \(F\in C^3\) in a neighborhood of the accepted state \(y\);
2. \(F_0=F(y)\ne0\);
3. the tolerance scale vector has a finite, componentwise positive limit \(s(h)=s_0+O(h)\);
4. the leading quadratic remainder vector \(B=F''(y)[F_0,F_0]\) is nonzero in the weighted norm.

Then for the pexprb54s4 level-2 stages,
\[
U_i-y=c_i h F_0+O(h^2),\qquad i=3,4,
\]
and Taylor's theorem gives
\[
D_i=\frac12c_i^2h^2B+O(h^3).
\]
Therefore
\[
\rho_i=Cc_i^2|h|^3+O(|h|^4)
\]
for a common \(C>0\), and
\[
\boxed{\frac{\rho_4}{\rho_3}
=\left(\frac{c_4}{c_3}\right)^2+O(h)
=3.24+O(h).}
\]

For the raw Euclidean defect ratio
\[
\eta_i=|h|\frac{\|D_i\|}{\|U_i-y\|},
\]
the analogous expansion is
\[
\boxed{\frac{\eta_4}{\eta_3}=\frac{c_4}{c_3}+O(h)=1.8+O(h).}
\]

### Interpretation

These are **asymptotic baselines, not admissibility theorems**. A ratio substantially above the baseline can indicate later-stage growth of nonlinear remainder relative to the near-quadratic small-step regime, but neither baseline supplies a certified threshold for full-step local error.

## Data confrontation

Frozen campaign size: 56 level-2 events, 54 historically full-E-admissible and 2 historically inadmissible rows. The two inadmissible rows are the same structural HIRES event reproduced at N=128 and N=512 and must not be counted as two independent failure geometries.

For the **predeclared** feature \(A_{34}=\rho_4/\rho_3\):

- safe min = 2.2088417922;
- safe median = 3.1306066594;
- safe max = 3.6121326423;
- unsafe N=128 = 4.2802538589;
- unsafe N=512 = 4.2802540852;
- observed gap = 0.6681212166.

Normalized by the asymptotic baseline 3.24, safe median is 0.9662, safe max 1.1149, whereas the repeated unsafe event is 1.3211.

The raw ratio \(\eta_4/\eta_3\), computed only as a **post-hoc consistency check** because it was not predeclared, has safe median 1.80085, strikingly close to its 1.8 small-h baseline; the repeated unsafe event is 2.45693. This supports the stage-growth interpretation but may not be promoted to a policy feature from this campaign.

## HIRES counterexample and what changed from level 1

The repeated inadmissible HIRES event has
\[
\rho_2\simeq2.2712,\quad
\rho_3\simeq10.6800,\quad
\rho_4\simeq45.7131,
\]
with historical full-E error \(q_E\simeq1.135>1\).

Safe HIRES neighbors include both a much larger level-1 defect
\(\rho_2\simeq39.45\) and a smaller one \(\rho_2\simeq1.86\). Thus absolute level-1 magnitude was genuinely ambiguous. The later-stage growth ratio, not the absolute defect magnitude, is the new information exposed by level 2.

## Counterexamples / red-team

1. **Same-campaign separation:** strict separation on 56 rows does not validate a universal threshold. The unsafe set contains one structural geometry replicated across dimensions.
2. **Direction-change / cancellation:** Taylor's leading vector \(F''[F_0,F_0]\) can vanish or rotate with stage direction; then the asymptotic ratio proof does not provide a stable finite-h discriminator.
3. **Tolerance-scale drift:** \(s_i\) can change materially between U3 and U4 for strongly varying components, altering \(\rho_4/\rho_3\).
4. **Non-smooth or highly nonnormal regimes:** the C3 local expansion may be a poor finite-step model even though the code remains numerically defined.
5. **Family concentration:** the observed unsafe event is HIRES-specific; no unsafe semilinear/PR-type level-2 event appears in this exact corpus.
6. **Economics:** safety information is not free. Median cumulative level1+2 work is 19 JVP vectors and the N=512 maximum is 61.

## Verdict

- Level-2 mathematical construction: **PASS**.
- Small-h stage-growth proposition: **PASS under stated assumptions**.
- Claim that level-2 contains information absent at level 1 on the observed corpus: **PASS**.
- Claim that \(\rho_4/\rho_3\) is already a safety certificate or production threshold: **REJECTED / NOT TESTED**.
- Correct next move: independent calibration of a normalized stage-growth witness, followed by a separately sealed safety holdout; no active switching yet.
