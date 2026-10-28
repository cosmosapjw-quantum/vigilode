# Robertson endpoint 독립 참조와 기존 실행 재판정

## 범위와 source identity

- 원본 local solver: `aaf1f834d4c589e734ca7851a56c11bb44f7047a`, tree `03b12c682c688155e0f8b08c52508e04738acb93`.
- 이 비교의 실제 측정 runner: `9b31da59b6d30a092af52357d3bc257b13ce043f`, tree `38d350b73ab2d809c1b6a0deb6abb7173810b45b`. 이후 브랜치 commit을 측정 당시 commit으로 소급하지 않는다.
- 원시 자료: `/tmp/vigilode-local-run-20261024.geLVCp9Q/native_diffsol_followup/comparison_{smoke,standard}/` 및 각 `IDENTITY.json`/`COMPARISON_RECEIPT.json`. 원본은 수정하지 않았다.
- 추가 재판정: `/tmp/vigilode-local-run-20261024.geLVCp9Q/robertson_reference/REASSESSMENT_V2.json` (SHA256 `15a377923a660be0ab7295e3eabc9ddd716ab599b97ac2483fe9036dcd282de1`). 원시 영수증 72개의 개별 SHA256과 방법별 timed 5회 통계가 있다.

## 독립 endpoint

Robertson 계수 `k1=0.04, k2=10^4, k3=3×10^7`, 초기 상태 `(1,0,0)`과 `t=40` 구간은 [Hairer testset RHS](https://www.unige.ch/~hairer/testset/stiff/rober/equation.f) 및 [Ramos–Rufai 2023, §6.3](https://link.springer.com/article/10.1007/s10910-022-01427-7)의 문제와 일치한다. 후자는 `t=40`에서 `(0.7158270687194135, 9.185534764558135e-6, 0.28416374574582)`를 선행 testset의 reference로 제시한다.

Wolfram `NDSolveValue`에 계수를 정확한 유리수로 입력하고, `Method -> {"BDF"}`, `MaxSteps -> Infinity`로 두 단계 계산했다. `WorkingPrecision/AccuracyGoal/PrecisionGoal`은 각각 `(35,20,20)`과 `(50,28,28)`이다. 높은 정밀도의 endpoint:

| 시각 | y1 | y2 | y3 | 두 정밀도 간 최대 성분 차이 |
| --- | ---: | ---: | ---: | ---: |
| 0.01 | 0.9996006826882932010120243312 | 0.00003645047887844242490793657 | 0.00036286683282835656306773224 | 3.63e-20 |
| 40 | 0.7158270687194050904744763316 | 9.185534764557763903899313e-6 | 0.2841637457458303517616197691 | 1.20e-17 |

`t=0.01`은 별도 `Method -> "StiffnessSwitching"`, `(50,28,28)` 계산과도 일치한다. `t=40` 문헌값과 최대 성분 차이는 약 `1.03e-14`다. 두 계산에서 질량 합은 표시 정밀도에서 1이다. 이것은 **수치적 교차검증**이며 외향 반올림을 사용한 엄밀한 참오차 인증은 아니다. 벤치마크 재판정에는 보수적인 수치 참조 불확실도 추정 `1e-12`를 사용했다. 이는 endpoint 오차 cap `1e-4`의 10%인 `1e-5`보다 작다.

첫 Wolfram `StiffnessSwitching` 장구간 계산은 기본 `MaxSteps=10000`에 도달해 `t≈8.99/9.29`에서 멈췄다. 그때의 `t=40` 외삽 출력은 폐기했고 reference나 성공 표본에 넣지 않았다. 전체 실패와 최종 BDF 입력·출력은 run root의 별도 영수증에 적었다.

## 기존 비교군 재판정

`smoke t=0.01`, `standard t=40` 각각 6개 방법 × warmup 1 + timed 5의 기존 72개 원시 영수증만 읽어 `max_i |y_i-y_i^ref| + 1e-12 <= 1e-4`로 판정했다. **72/72 `PASS_SCOPED`**. `standard`에서 방법별 최대 실제 endpoint 오차는 diffsol BDF `2.95e-8`, VigilODE DOPRI5 `5.42e-9`, native DVODE Adams `1.23e-6`, native DVODE BDF `8.96e-7`, VigilODE RODAS5P MF `4.55e-7`, VigilODE switching `4.99e-9`이다. 두 코호트 모두 재실행하거나 원시 `UNVALIDATED_REFERENCE` 상태를 덮어쓰지 않았다. 시간은 계측되지 않은 solve timer의 진단값으로만 유지하며, 이 문서는 전체 궤적 정확도나 production 속도 우위를 주장하지 않는다.

## 완료 판정

Robertson의 **이 2개 endpoint 비교만 닫혔다**. 전체 standard 1539개 작업은 모두 시도되었지만 기존 결과에는 timeout 209, 독립 참조 없는 표본 805, 정확도 실패 146, solver 실패 57이 남는다. Julia는 고정 Project/Manifest가 없어 `NOT_RUN`; 장시간 위상 및 별도 물리 client 검증도 미완료다. 따라서 local 측정 실행 완료와 전체 qualification 완료는 구분해야 한다. Broad production은 `HOLD`이며, 이 Robertson 재판정만으로 원격 push 완료 조건을 충족했다고 보지 않는다.
