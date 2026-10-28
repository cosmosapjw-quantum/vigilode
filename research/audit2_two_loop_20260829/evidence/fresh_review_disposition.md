# Fresh-context review disposition

## Review identity

- Reviewed range: `93fe348ce36859dd5f78b31267d771ea9c054677..25e086f86819577978e0710d2dab9c352555c4cc`
- Review count: one fresh-context review
- Review decision: `REQUEST_CHANGES`
- Final repaired HEAD/tree: supplied by the live Draft PR receipt
- Second fresh-context review: not performed and not claimed

This review is a fresh-context code review of the bounded pre-repair diff. It is not a production certification, external scientific replication, or review of the eventual publication head.

## Confirmed items

- Residual convention is `R = lhs - rhs`; solving `J_R z = R` implies the Newton update `K <- K - z`.
- Frozen/common W and strict-lower coupling are the assumptions that make block-forward substitution applicable.
- No production integration, gate, campaign, or dispatch activated the candidate.

## Findings and dispositions

| Severity | Finding | Disposition in continuation working tree | Verification state |
|---|---|---|---|
| P1 | The official converted decimal coefficients contain nonzero forbidden entries, invalidating the literal claim that the unprojected target is exactly strict lower/common diagonal. | Added a separate research-target projection with the fixed tolerance `64*f64::EPSILON = 1.4210854715202004e-14`. Observed maxima are alpha forbidden `5.577737968635803e-16`, Gamma upper `4.994632140352628e-16`, and Gamma diagonal error `8.881784197001252e-16`. Values above the fixed tolerance fail. Exactness is claimed only after projection; production coefficients and residual are unchanged. | Final structured suite: `11 PASS` |
| P1 | Stored reference uncertainty had no explicit authority, so an empirical estimate could be treated as a bound. | Added mandatory `ReferenceUncertaintyTreatment`. `EstimateOnly` cannot yield `WithinBudget` or `OutsideBudget`; missing B remains `BudgetNotSpecified`. | Updated accuracy suite: `9 PASS` |
| P1 | Error returns erased attempted work and partial correction progress. | Added typed phase, attempt/completion counters, underlying work counters, and retained partial corrections to research outcomes. Missing/failing JVP and failed/overflowed solves have explicit accounting assertions. | Final structured suite: `11 PASS` |
| P2 | Zero RHS could make a secondary relative state comparison evaluate 0/0. | Uses exact absolute-zero residual/correction criteria; the undefined relative ratio is represented as absent rather than NaN or a fabricated equality. | Final structured suite: `11 PASS` |
| P3 | Formatting did not satisfy the repository formatter. | Formatted the repaired source/tests. | `cargo fmt --all -- --check`: PASS |

## Fixed rules independent of results

- The six saved trajectories are the metadata slice containing all six families at dimension96 and rtol1e-8. That post-campaign extraction rule was fixed independently of their numerical outcomes but was not a preregistered holdout.
- The correction test matrix and its backward-error/state/invariant thresholds were fixed before observing the test results.
- The coefficient projection tolerance and entry pattern are independent of correction residuals, state differences, historical statuses, and campaign outcomes.
- No numerical accuracy budget B was chosen from the historical54 results.

## Review ceiling

The review permits only an `EXPLORATORY_NONAUTHORITATIVE` research diagnostic claim for the explicitly projected target. It does not admit unprojected exactness, a nonlinear certificate, historical accuracy PASS, timing/ranking/speedup, BDF/CVODE comparison, production use, holdout/freeze validity, or scientific-publication claims.
