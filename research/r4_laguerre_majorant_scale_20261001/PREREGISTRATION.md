# Preregistration: Laguerre recurrence majorant and continuous scale (R4-POLY-DEV-03, R4-POLY-DEV-05)

Written and committed before the run. Results are appended below the line at the end after the run.

## Questions

1. **POLY-DEV-03.** Does the scalar recurrence majorant (`laguerre_majorant_total`) enclose the observed error, and how
   often does it reject at the requested budget? Laguerre stays EstimateOnly whatever the answer.
2. **POLY-DEV-05.** Does the continuous scale `L*(m) = min(16, 2(m+1) - h rho)` (`laguerre_scale_for_degree`) select a
   degree no worse than the fixed grid {1, 2, 4, 8, 16}, with an equally good majorant and accuracy?

## Source and command

- Source commit: `96639dbf9b6be7c288028534570674d4decfcfcb`, rustc 1.94.1.
- Command:
  `target/release/rodas5p r4-study --study laguerre --output research/r4_laguerre_majorant_scale_20261001/STUDY.json`

## Design

- Diagonal operators with verified enclosures:
  - `diag24-wide`: eigenvalues `-geomspace(0.1, 100, 24)`;
  - `diag24-narrow`: `-geomspace(1, 4, 24)`;
  - `diag8-stiff`: `-geomspace(0.01, 1000, 8)`.
- Steps h in {1e-3, 1e-2, 0.1, 1} and budgets in {1e-6, 1e-10}, which gives 24 points.
- Inputs are the deterministic cosine vectors. The reference uses the scalar phi enclosures per eigenvalue.
- Arms: the grid Laguerre action, the continuous-scale Laguerre action (smallest m whose f64 tail estimate meets the
  budget), and certified Chebyshev for context.

## Gates

- **POLY-DEV-03 PASS** if the majorant total is at least the observed error at every evaluated Laguerre point (both
  arms), and the fraction with majorant total above the budget is at most 0.5. Otherwise **FAIL**, which means the
  looseness is documented and the candidate certificate is rejected.
- **POLY-DEV-05 PASS** (retain the candidate) requires all four of the following on every point where both arms run.
  Otherwise **FAIL** (reject the candidate).
  - The continuous degree is at most the grid degree.
  - The continuous degree is strictly smaller on at least a quarter of the points.
  - The continuous majorant is at most 10 times the grid one.
  - The continuous observed error is at most `10 * budget + 1e-13`.

The rules are applied as stated. Wall seconds are not measured and no speed is claimed.

## Prior information

The contract tests ran the majorant on six amplitude cases at h = 0.1 only. This grid has not been run before.

## Addendum before the rerun (process deviation, disclosed)

The first run of the preregistered command at `95cb66e` aborted before writing any output. The reference evaluation
calls the scalar phi enclosure, which supports `-600 <= z <= 0`. The `diag8-stiff` family at `h = 1` reaches
`z = -1000`. The fix is a study-code change only. For `z < -600` the reference is computed in binary64 by the
cancellation-free recurrence `phi_k(z) = (phi_(k-1)(z) - 1/(k-1)!)/z`, where `e^z < 1e-260` is taken as 0. The
grid, the arms and both gates are unchanged. The output of the aborted run was an error message only, so nothing was
seen.

---

## Results (appended after the run at `d700572`)

Output: `STUDY.json`. The run took 15 s; wall time is not an outcome. The grid has 24 points. Both Laguerre arms fail
with a typed non-finite product at `diag8-stiff`, `h = 1`, where the coefficient series leaves the binary64 range.

**POLY-DEV-03: PASS.**

- The majorant total enclosed the observed error at all 44 evaluated Laguerre points (both arms).
- It exceeded the budget at 8 of the 44 points, a rate of 0.18 against the 0.5 limit.
- Tightness (majorant over observed error): median 162. The maximum is 3.5e65, at degrees 152 and 214, where the
  scalar recurrence majorant grows like `(1+sqrt 2)^n`. Those points reach majorant totals of 1e33 to 6e52 against
  observed errors of 1e-10 to 1e-13.
- The useful certified region is therefore low degree. The baseline is too loose to certify high-degree Laguerre
  actions.
- Laguerre stays EstimateOnly until the majorant is independently reviewed.

**POLY-DEV-05: FAIL (the candidate is rejected).**

- On all 22 points where both arms ran, the continuous scale `L*(m)` chose the same degree as the fixed grid. It was
  never smaller (0 of 22, where the gate needs at least a quarter).
- The majorant was within 10x and accuracy held at all 22 points. Where the continuous scale differed from the grid
  (h <= 0.01 with narrow or moderate spectra), it lowered the majorant by up to 27x at equal degree.
- That is not the preregistered gain, so the fixed grid {1, 2, 4, 8, 16} stays. The scale cap is not lifted.

## Corrections after the second independent review

The numbers in `STUDY.json` are unchanged. This section corrects how they were described above. The text above is
left as first published. Ledger rows L-0020 and L-0021 supersede L-0013 and L-0014 with input hashes taken at the
execution commit `d700572`, where this file is the preregistration and its addendum, before any results.

- **What "enclosed" means.** The observed error is measured against an f64 reference (the midpoint of the scalar
  enclosure, or the recurrence for `z < -600`). Enclosing it at every grid point is an empirical check. It does not
  certify a region, and the words "certified region" above overstate it.
- **Distinct evaluations.** 13 of the 22 compared rows run both arms at scale 16, so the 44 points are 31 distinct
  evaluations, and the 8 budget rejections are 4 distinct cases.
- **Median.** The median tightness is 156.8. The 162 above is the upper median.
- **Where the continuous scale differed.** It lowered the majorant at h = 0.001 on `diag24-wide` (4.5x and 21x) and
  at h = 0.001 to 0.1 on `diag24-narrow` (8.6x to 27x). The largest, 27x, is at `diag24-narrow`, h = 0.1, budget
  1e-6 (scale 13.6 against 4), not at h <= 0.01.
- **Contract count.** The POLY-DEV-03 contract checks the seven fixture actions at L in {1, 2, 4, 8, 16}, not six
  as the prior-information paragraph says.
