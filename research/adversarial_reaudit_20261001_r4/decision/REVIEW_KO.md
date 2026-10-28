# R4 독립 decision 검토

검토자는 후보 설계·튜닝을 하지 않았다. 고정 source, 이전 R3 findings/claims/DAG, 최신 closure, 관련 implementation, 네 lane의 사전 계약·유도·raw 결과를 읽었다. 원 source는 `1c54194123ee6abc6daa512e8574922f510b4e2c`, 실행 전 사전등록은 `b2914f3e3c03eda60a8547619db6284aac8250a2`다. 최종 상태의 SSOT는 `FINAL_REVIEW.json`이다.

기술 판정은 **REWORK**다. 9개 finding(8 P2, 1 P3)은 명시한 좁은 범위에서 재현됐으며, 원 R3 반례가 닫혔다는 증거를 취소하지 않는다. known BDF floor의 실제 오차는 인정된 잔여 한계이며, Radau unequal-half mismatch는 updated path에서 확인했지만 과거 동일 입력이 올바르게 통과했다는 증거는 없다. 연구용 transform helper, mutable/serde witness 및 generic s>8 target의 오류를 기본 RODAS 생산 경로 전체로 확대하지 않는다.

## 독립 검증의 실제 범위

- 같은 고정 native 실행파일 네 개를 reviewer가 별도 프로세스로 실행했다. 36+78+9+21=144개 JSON 행 모두 owner 관측과 의미적으로 같고 exit 0이었다. 이것은 실행 재현이며 독립 solver 구현은 아니다.
- 통계 후보의 32개 design은 후보 함수를 import하지 않고 공정 coin 분포를 Fraction convolution으로 직접 만들어 k/coverage/무한 구간 여부를 확인했다.
- 추가 36개 exact spotcheck에서는 원 binary64 bits를 Fraction으로 읽어 nilpotent phi 변환 오차, candidate norm·역팩토리얼, 첫 stage exact solve, s=9/16 chain, BDF 실제 격자 recurrence, Radau 14/9·5/9·4/9·5/4를 독립 계산했다. transform false bound 네 개를 똑같이 확인했다.
- 실제 runtime stdout과 모든 named test inventory를 한 번 대조했다. 149 harness, 735 named test 중 671 PASS의 terminal `test ... ok`가 모두 존재한다. 4 ignored, 20 timeout 미확정, 40 예산 미실행이다. doctest와 full suite PASS는 주장할 수 없다.

추가 spotcheck script는 사전등록된 독립 검토 단계의 **사후 reviewer 구현**이다. 사전등록 candidate script인 것처럼 표기하지 않는다. 최초 실행은 큰 Fraction을 문자열로 쓰다가 Python의 4300-digit 제한으로 exit 1이었다. 첫 script와 stderr를 보존했고, 큰 분수의 출력 표현만 줄여 exact 비교를 그대로 유지한 재실행은 exit 0이다. 이는 serialization incident이며 solver·수학 결과 실패가 아니다.

## 허용하는 연구 결론

Chebyshev의 spectral-domain 전제, coefficient enclosure와 Green-U propagation, Laguerre의 exact-arithmetic tail, 유효 inverse witness에 대한 삼각 stage majorant, nilpotent finite path-sum 및 diagonal component 분해의 수학은 명시 전제에서 일관된다. Fraction candidate와 finite oracle sweep은 그 구현의 제한된 지지 증거다. 독립 세션 interval은 iid complete session vectors 및 fixed corpus 아래 정확한 조합확률/union-bound 보장을 갖지만 기존 pooled-pair estimand와 일반적으로 같지 않다.

Laguerre scale optimum은 미분식으로 독립 확인했다. 고정 r=m+1, s=hρ에서 2r>s이면 β*=ρ/(2r−s), L*=2r−s이고, L≤16 제약에서는 min(16,2r−s)다. 2r≤s이면 이 tail envelope의 infimum은 β→∞에서 W다. 이는 exact truncation envelope의 분석이며 실제 recurrence의 roundoff 또는 실행시간 최적값이 아니다.

이 결과들은 **조건부 연구 결과로 채택**한다. 생산 backend, general ODE certificate, 비정규 행렬, 성능 향상의 승격은 HOLD다. 기존 POLY03 raw Promote는 statistical authority HOLD를 함께 유지하며, 이미 FAIL한 coverage나 HOM06를 새 발견으로 중복 집계하지 않는다. 같은 결과에 추가 감사의 감사를 반복하지 않고, 보고서 DAG에 따라 국소 수정과 정해진 acceptance로 넘어가는 것이 적절하다.

## 통합 보고서의 한 번의 검토

통합 한국어 본문을 한 번 읽어 범위·숫자·claim ceiling을 검토했다. Radau 항목의 signed error와 magnitude 표기만 맞추도록 요청했고, 과거 source에서의 올바른 동작을 확인하지 않은 회귀 단정을 제거하도록 요청했다. 두 수정 모두 반영됐음을 선택 확인했다. 기계 판독 bundle은 9 findings, 이전 21 closure node, 15 claims, 26 development node로 일치하며 참조 파일이 존재하고 dependency 순서가 비순환이다. `FINAL_REVIEW.json`의 최종 보고서 판정은 PASS_FOR_AUDIT_REPORT_ONLY이고, 감사한 source의 기술 판정은 REWORK, production/speed 승격은 HOLD다. 추가 재귀 검토는 필요하지 않다.
