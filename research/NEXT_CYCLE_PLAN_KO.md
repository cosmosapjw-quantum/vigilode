# 다음 연구 브랜치 — homotopy 기반 RODAS5P stage-coupling

현재 `rust-adaptive-v0.2` 최적화 작업은 `rust-pre-homotopy-v0.2.0-alpha1`에서 닫는다.
다음 구현은 이 tag에서 `homotopy-stage-coupling-v0.3` 브랜치로 분기한다.

우선순위는 다음과 같다.

1. predictor-anchored coupling homotopy의 residual/path API와 exact sequential oracle 연결;
2. `p=0,1,2` truncated nilpotent-coupling propagation의 TDD 구현;
3. stage-batched RHS와 common-`W` multiple-RHS solve의 비용 장부;
4. original RODAS residual + LTE-relative certificate + transactional fallback;
5. affine, Prothero–Robinson, nonnormal/mass-matrix 문제의 과학 검증;
6. SABR5P, sequential RODAS5P, homotopy-only, homotopy+Anderson의 동일 Rust A/B;
7. 그 뒤에만 deferred GCRO-DR `(m,k)` tuning과 adaptive admission을 합쳐 비교.

상세 handoff는 `research/HOMOTOPY_BRANCH_HANDOFF_KO.md`를 따른다.
