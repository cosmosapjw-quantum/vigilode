# PHYS–MATH AUDIT — actual common-W RHS telemetry v0.15

## Conventions and dimensions

- Metric/GR convention은 이 ODE solver task에 직접 등장하지 않는다.
- \(h\) has time dimension, \(J\) has inverse-time dimension, and \(W=M-h\gamma J\) is dimensionally consistent with the mass matrix convention.
- Numerical-rank telemetry는 RHS vectors의 span을 본다. Raw singular values는 physical units와 variable scaling에 의존한다.

## Load-bearing checks

| Check | Result | Qualification |
|---|---|---|
| 8-stage lower-triangular path unchanged | PASS | observer-only hook; 162/162 outcomes/counters match |
| Exact rank under invertible \(W^{-1}\) | PASS mathematically | numerical rank may change through scaling/conditioning |
| Unit-column directional analysis | PASS | removes inter-RHS magnitude dominance, not component-unit dependence |
| Principal-angle meaning | PASS after fix | only identical phase and propagation level are compared |
| Rank threshold invariance | FAIL | 22.6–38.8% classifications change under tested threshold mutations |
| High-risk fallback rationale | PASS as conservative experiment policy | current risk labels are manufactured metadata, not deployable observables |
| Severe q2 path failure | CONFIRMED | NaN/Inf is path-level; linear backend cannot repair it |

## P0–P3 ledger

- **P0:** none found in telemetry-only path after behavior-equivalence gate.
- **P1:** active dispatch cannot rely on a single relative-rank threshold.
- **P1:** production scaling must use left-preconditioned and/or WRMS-scaled quantities.
- **P1:** path-level nonfinite failure needs protected sequential fallback independently of backend dispatch.
- **P2:** low-dimensional cases structurally constrain rank and must not dominate general claims.
- **P2:** p90 drift of block-recommended batches is large; median-only policy is unsafe.
- **P3:** full SVD is acceptable offline but too costly as an online diagnostic.

## Surviving claim

Actual SABR/homotopy common-\(W\) batches frequently contain low-dimensional directional structure, but only a guarded per-batch policy with drift/risk/true-residual checks is scientifically defensible.
