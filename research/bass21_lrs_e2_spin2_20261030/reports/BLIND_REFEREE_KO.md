# 자체 적대적 검산 | BASS21 source-owned E2/spin-2 (동일 모델, 독립 저자 아님)

판정: **SCOPED MODEL PASS / FULL BASS E2=UNRESOLVED / PRODUCTION HOLD**.

1. **연산자 부호/정규화 (PASS)**: 원본 BASS `lrs_boltzmann.py` SHA-1 `e15e1875fba7fcc4a42e257e291c54d9b540a3f0`의 문서화된 K2 항 `-Γ(9F2/10+√6 E2/10)`과 PR132의 `-g(9j2/10+3B_Q/40)`를 비교하여 `E2cand=3B_Q/(4√6)`를 도출. 실제 원본 RHS는 *E2=0*만 사용. 이것은 **충돌항의 사상**, 전체 source-native E2 hierarchy 복원 아님.
2. **고차 모멘트 차단 반례 (PASS)**: `P_4^2=(15/2)(7μ²−1)(1−μ²)` 및 직교성으로 `C_Q=B_Q/7+32c4/21`; 따라서 전단항 `b'(10 E2/7+4√6 c4/7)`. `b'≠0`, `c4≠0`이면 E2-only closure는 정확하지 않다. 실측 s04 `max|c4|=1.0928253414095219e-4`, 끝점 중앙값 `|c4/c2|≈0.02377`; 비영 잔차를 숨기지 않았다.
3. **충돌 감쇠와 극한 (PASS)**: 동차 등방 b'=0, (j2,E2) 선형 충돌행렬 `[[-9/10,-√6/10],[-√6/10,-2/5]]`의 고유값 `-1,-3/10`. 이 행렬은 기존 무편광 배경 J2의 외력을 제외한 동역학적 동차부에 한함. 원본 자유 전자 밀도가 변하면 시간 의존 계수임.
4. **수치의 한계 (CONCERN, NONBLOCKING for scoped data)**: `max|dE2/dx - RHS|≈8.8063e-6` (저장 grid에서 이산 미분), `max|E2-∫RHS dx|≈9.0324e-9` (sampled Simpson); 직접 고차 편광 IVP나 원본 BDF 전역오차 인증이 아님. GL64/96 각도 차이 `6.07e-18`은 **각도 판별량**만 검증.
5. **물리 주장 반증 위험 (HOLD)**: frozen 원본 62-state의 Jℓ은 무편광 collision을 사용했으므로, E2 되먹임을 충분히 포괄한 완전 photon+Einstein 해라고 주장하면 오류. 실제 관측 CMB E/B 파워스펙트럼, 스펙트럼별 온도 변환, 비대각 Bianchi, 소스 단일 사건이 아닌 KM2F 연결은 NOT_RUN.
6. **독립성 분류 (HONEST)**: Wolfram/SymPy 기호 교차검산과 자체 적대적 재분석은 근거를 강화하지만 다른 저자의 독립 R6/C9가 아님. Native subagent NOT_RUN.
7. **CI 위험 (FAIL HISTORICAL)**: 기존 `check-authority-refs.py` 결과를 우회하거나 허용 목록을 남발하지 않는다. 이전 실행에서 historical 330 failures: 257 missing Git objects/trees, 54 unavailable third-party repository checkouts, 19 invalid repository annotations. 이 노드의 **분리 scoped CI가 통과하더라도 전체 Research ledger and authority references CI의 실패는 별도로 남음**.

추후 최소 교정: source-owner의 정확한 spin-2 harmonic/PSTF 정의와 photon `Jℓ` 결합 collision, 원본 시점·electron/temperature 엄밀 보존, 필요시 실제 prospective 편광-광자 IVP 승인. 새로운 계산량과 DAG 증분을 혼동 금지.
