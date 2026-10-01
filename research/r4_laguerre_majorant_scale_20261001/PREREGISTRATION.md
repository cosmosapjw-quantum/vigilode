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
