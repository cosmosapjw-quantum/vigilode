# 독립 최종 판정: ACCEPT_NATIVE_RESEARCH

검토자: `/root/integration_decision`. 검토 source: `699a7adc6dba5fc45d4da01a5456f10f25dcf210`.
사전등록: `3e784982104274312027490d481de4929e4f3cf6`.

**현재 real dense shared-shift 연구 primitive와 좁은 chart 입력 경계 수정을 수용한다.**
기본 solver 활성화, generic RVJ order, coupled Fourier native client, 실제 parallel dispatch,
wall-clock speedup 및 기존 enclosure 합성 proof obligation은 승격하지 않는다.

검토자는 후보 코드와 최초 검증 설계를 작성하지 않았다. 실제 완성 source, 새 예제 출력,
정확 oracle의 원시 결과, RED/GREEN과 최종 테스트·명령 로그, 보고서와 machine-readable
계약을 직접 읽었다. 과거 science나 새 Cargo 캠페인을 다시 실행하지 않았다.
별도 계산은 결과 집계 일치와 DAG의 ID·의존관계·비순환성 검사에 한정했다.

## 수학·구현 판단

`A_gamma=A0(I-zT)`, `T=R0-I`이므로 정규화 recurrence와 Horner의 부호가 올바르다.
현재 입력의 symmetric-part outward row bound가 dissipativity의 충분조건을 증명하고,
positive real shift의 inverse Euclidean gain≤1을 사용해 원 target 잔차 L1 upper를
각 RHS의 Euclidean solution error upper로 바꾼다. LU/recurrence/ratio/Horner의
유한 정밀도 오차는 저장 후보에 대한 별도 원 target residual에 포함된다.

고정 borrow된 J와 현재 h/gamma/B를 같은 호출에서 사용하며, 모든 RHS column을
보존한다. 증명서/후보/report는 private fields와 읽기 accessor를 제공하고 Deserialize를
구현하지 않는다. degree와 크기 곱·합·예산은 할당 전에 제한된다. Gershgorin 충분조건을
통과하지 못하는 실제 dissipative 입력도 있을 수 있다는 보수적 범위는 명시되어 있다.

정확 oracle은 native LU/jet/interval 구현을 공유하지 않는 Fraction Gauss–Jordan이다.
현재 exact binary target을 직접 풀고 exact residual L1와 squared Euclidean error를
각 reported upper에 비교한다. 입력과 결과의 IEEE bits 및 ordered requested gamma를
결속하므로 다른 target의 수치를 대신 검증하는 틈도 닫았다.

## 실제 증거

- 새 15 cases, 591 target candidates: 559 Certified, 32 Rejected.
- RHS×target 1,341 rows: 1,309 Certified, 32 의도적 under-resolution Rejected.
  모든 reported upper가 exact error와 residual을 포함하며 실패 0이다.
- 11 negative controls 전부 거절. 고정 holdout, zero J/h, 2^±400 center scaling을 포함한다.
- 새 core 6, 새 chart 9, 관련 기존 8: fresh unique Rust tests 23 PASS.
- source에 결속된 최종 root 명령 8개 모두 exit 0; fmt와 affected clippy 포함.
- RESULTS/FINAL_VERIFICATION/raw evidence 집계가 일치한다. 21-node 개발 DAG는 비순환이다.

이 범위를 repository-wide regression PASS로 읽을 수 없다. 기존 모든 연구 캠페인을
다시 실행하지 않았으며, 과거 저자의 PASS를 이번 fresh test 합계에 더하지 않았다.

## 이 검토에서 발견하고 닫은 사항

`REVIEW-STORAGE-01`: gamma와 certificate tolerance의 `2*s` 명시적 scalar slots가
예산식에서 빠져 있었다. 최종 `3*s` 식과 392/391 경계 테스트를 확인했다.
이 예산은 faer workspace, vector metadata/capacity, RSS를 포함하지 않는다.

`REVIEW-ORACLE-BINDING-01`: 원래 verifier의 candidate count 검사에 ordered requested
shift 검사를 추가하도록 요청했고, 최종 equality/bit checks를 확인했다.

Chart 변경은 NaN의 거짓 Converged, 잘못된 shape의 panic, 비유한 설정과
`t+h==t` 상태 갱신을 닫는 범위다. 기존 chart/nonnormal 함수의 exp·시간 곱·midpoint
방향성 의무를 이 경계 테스트로 해결했다고 주장하지 않는다.

Wolfram 보조 출력은 identity와 degree 0–5 residual checks 모두 True를 반환했지만
`Symbol::undefined2` 및 `General::messages` 경고도 반환했다. 경고는 보존되어 있으며
이 출력은 native 인증의 판단 권위가 아니다.

## 후속 개발 및 출판

AC07과 최종 metadata는 이 독립 판정을 연결할 수 있다. 현재 claim ceiling을 유지하면
추가 역사 재실행이나 리뷰의 재귀적 반복은 필요 없다. 다음 개발은 명시적 GCRODR 정책
연결, local directed composition, 실제 client와 그 원 target certificate, domain에 맞는
baseline 비교와 비용 gate로 이어져야 한다. 생산·속도 승격은 그 별도 증거가 생길 때까지 HOLD다.
