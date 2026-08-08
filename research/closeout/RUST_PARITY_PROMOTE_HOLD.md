# Rust parity alpha checkpoint — PROMOTE/HOLD closeout

- benchmark code HEAD: `d34dee5f55106bfda9f7a4c6667dbdce531b8622`

## PROMOTE

- Rust workspace와 release CLI를 새 연구 정본으로 승격
- Rust GMRES, LGMRES, fixed-left GCRO-DR
- protected sequential RODAS5P와 SABR5P fallback
- 공통 true-residual certificate, operator budget, trace identity, state-lifetime 계약

## HOLD

- 모든 default recycling policy
- GCRO-DR 성능 승격
- production, sparse, MPI, GPU, DAE, BDF/Radau 순위, homotopy 주장

## 정량 gate

사전에 고정한 default-backend gate인 median wall speedup `>=1.15`를 GCRO-DR가 통과하지 못했다.
Core suite에서 GCRO-DR/PERSISTENT의 GMRES 대비 wall ratio 중앙값은 `1.565`, operator ratio
중앙값은 `1.095`다.

다음에 허용된 성능 작업은 allocation 제거, `(m,k)` calibration/holdout, frozen Rust trace 위의
adaptive admission/reset이다.
