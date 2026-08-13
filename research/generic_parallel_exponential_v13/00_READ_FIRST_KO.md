# G3 Fused Adaptive Parallel Exponential — 먼저 읽기

Date: 2026-08-13
Branch: `generic-parallel-exp-adaptive-v1.3`
Base: `generic-parallel-exponential-foundation-v1.2.0-alpha1`

## 한 문장 결과

여러 `phi_k(hJ)v` 항을 fused augmented Krylov action으로 묶어 pexprb54s4의 phi-action work를 크게 줄였고, full-MGS primary path는 explicit Jacobian과 Newton 없이 일부 stiff/oscillatory·semilinear regime에서 protected JF RODAS5P보다 빠르다. 그러나 12개 same-error target 중 11개만 덮고, oscillatory nonlinear Prothero–Robinson에서 step economy와 wall time이 붕괴하므로 generic solver 승격은 HOLD다.

## 정본 수치

- canonical campaigns: 7
- scientific checksum: `a921e2ce3cf6f2fe752b916bc965c44ba1131aece81054ae3d4eb4ee470a804a`
- fused phi median wall speedup: Full-MGS `4.096x`, IOP2 `4.124x`
- Full-MGS pexprb54s4 same-error coverage: `11/12`
- covered-target median speedup: `1.519x`
- median step ratio versus RODAS5P: `1.10`
- maximum step ratio: `8.333`
- explicit Jacobian builds: `0`
- Newton iterations: `0`
- within-step T1/T4 wall ratio median: `0.525` (four-thread path slower)

## 판정

```text
PROMOTE
  fused phi-combination infrastructure
  full-MGS matrix-free adaptive Krylov reference
  adaptive fused EXPRB2/EXPRB43/pexprb54s4 infrastructure

PROMOTE TO EXPANDED SEALED HOLDOUT
  full-MGS pexprb54s4 scientific candidate

HOLD
  generic accelerated solver claim
  incomplete-orthogonalization default
  small-state within-step Rayon default
  universal same-error superiority
```

다음 node는 current pexprb54s4 정책을 동결한 expanded sealed holdout과 EPIRK-K/W sibling comparator 준비다.
