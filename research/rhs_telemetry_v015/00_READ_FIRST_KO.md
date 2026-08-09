# RODAS5P actual RHS telemetry v0.15 — 먼저 읽기

이번 checkpoint는 actual SABR5P와 partial-coupling homotopy q=0/1/2가 생성하는 8개 common-W RHS batch의 numerical rank, collinearity, energy rank, principal-angle drift를 **읽기 전용**으로 계측한다.

핵심 결과:

- canonical cases: 9
- telemetry rows: 1,215
- standard/observed behavior comparisons: 162
- behavior mismatches: 0
- hostile path failures: 2, partial rows preserved
- advisory recommendations:
  - seeded shared GMRES: 237
  - block GMRES: 437
  - independent Rayon GMRES: 541
- rank-threshold mutation under tested tolerances: 22.6–38.8% rows change classification

판정:

```text
TELEMETRY_GATE=PASS
DISPATCHER_ACTIVE=false
DISPATCH_ACTIVATION=HOLD
```

다음 branch에서는 full SVD를 cheap 8x8 Gram/QR diagnostic으로 교체하고, protected independent result만 commit하는 shadow dispatcher를 먼저 검증한다.

Start with:

- `reports/RHS_TELEMETRY_CYCLE_REPORT_KO.md`
- `tables/RHS_TELEMETRY_ANALYSIS.json`
- `reports/PHYS_MATH_AUDIT.md`
- `reports/PHYS_MATH_CODE_AUDIT.md`
- `handoff/NEXT_CYCLE_PLAN_KO.md`
