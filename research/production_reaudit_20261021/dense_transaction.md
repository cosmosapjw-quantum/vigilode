# Dense output·global accuracy·causal transaction 재감사

기준은 최신 계획 `cc2e1a0`과 실행 코드 `f2f20c1`의 합성 `4a5fc656730fe8243f06b6e603cb8aa758b00b2f`이다. 담당 범위는 sequential dense enforcement, T3/T4/T8의 적용 범위, causal transaction 계정이다. 원래 A05/A09 등의 확정된 증명은 별도 source port 담당자의 동일성 검사와 표적 회귀를 재사용한다. 새 수학적 불가능성 주장은 하지 않는다.

## 판정과 수정

**DENSE-01 / High / 수정 완료.** `DenseErrorControl::Enforce`와 `WFilteredEnforce`에서 midpoint estimate가 오류를 반환해도 `unavailable`만 증가시키고 step을 승인하던 O-01을 재현했다. 새로운 실패 주입 시험에서 enforcing 거절과 fatal-error 전파가 모두 실패했으며 reporting 모드는 통과했다. 이제 `LinearSolve`, `NonlinearSolve`, `NonFinite` 실패는 실제 종류를 보존하여 거절하고 기존 controller로 step을 줄인다. `recycle`과 `history`는 기존 rejected 분기에서 복원되며 collector, state, dense segment는 commit되지 않는다. 계산한 estimate work는 거절해도 run에 합산한다. 다른 입력/구조 오류는 typed error로 반환한다. 이 plain `Err` API에는 partial/work receipt가 없으므로, fatal/unclassified 경로까지 소비한 work를 호출자에게 전달한다는 주장은 하지 않는다. recoverable 거절과 반환된 run의 계수 보존이 이번에 닫힌 범위다. `DenseErrorReport::estimate_failures`는 enforcing 거절, `unavailable`은 reporting 모드만 센다. Facade의 schema/field 연결은 별도 facade 담당자가 구현했다.

**TX-01 / Medium / 수정 완료.** finite한 개별 cost를 반복해서 기록하면 `protected_base` 또는 `net_overhead`가 무한대로 overflow할 수 있었다. 이후 guard가 `inf <= inf`를 참으로 판단하는 등 유한 비용 계정의 전제가 무너진다. prospective `BinState` 복사본의 모든 누적 실수 계정이 finite인지 검사한 후에만 반영한다. overflow는 typed refusal이고 기존 bin은 바뀌지 않는다. prior·quantile 계산 자체가 표현 불가능한 config도 거절한다. 실제 소비한 work는 별도의 `TransactionLedger`에 보존해야 한다는 계약을 명시했다. 이 수정은 candidate를 production facade에 새로 routing하지 않는다.

**DENSE-T3 / 이론 범위 교정.** 정상행렬이라는 조건만으로 모든 좌반평면 spectrum에 real-negative scalar의 2.36 factor를 적용할 수 없다. `z=4πi`에서 상수 defect의 exact half-step error는 0이나 W-filter는 0이 아니다. 유효한 정리는 nonpositive real spectrum과 constant-defect 조건이다. 소스 API 문서도 이 조건과 일반 time-dependent defect에 대한 한계를 명시한다. 실제 complex 또는 nonnormal route의 정확도를 판단할 때는 T4 continuous defect를 사용한다. 독립 연구 감사자가 exact Bernstein sensitivity 정리와 T3/T4/T8 교정의 상세본을 작성했다.

**DENSE-T4 / 구성적 확장.** `DENSE_GLOBAL_BUDGET_THEORY.md`의 G1은 미래 positive log norm으로 가중한 실제 continuous defect budget에서 전 시간 오차 한계를 증명한다. fixed norm, full tube, 초기 반경, 조각 사이 jump와 query-center rounding을 함께 다룬다. G2는 block별 `C_j h^(p+1)` local bound와 step work ceiling이 있을 때 Hölder 최적화와 integer rounding으로 `W < W* + sum c_j`를 증명한다. 정확한 유리수 예제에서 같은 `epsilon=1/10`의 certified cost ceiling은 1640→1000이며, step cost가 정확히 1이면 실제 counted work도 39.024% 줄어든다. 이것은 현재 black-box facade의 global certificate나 wall-time speedup을 주장하는 결과가 아니다.

## 코드 감사 범위와 남은 전제

- W-filter는 현재 midpoint의 estimate다. step approval이 전체 continuous residual·global WRMS 오차를 증명하지 않는다. 기존 E1b의 20/30 global-tolerance 초과 기록은 그대로 보존한다.
- `Transaction<S>`는 `S: Clone`의 복사본으로 state를 격리한다. 공유 `Arc<Mutex<_>>` 같은 외부 interior-mutable 자원과 callback 부작용까지 deep rollback한다는 계약은 없다. driver가 복원 대상 state를 완전하게 소유해야 한다.
- atomic `Budget`의 정수 예약 cap은 checked addition과 compare-and-swap으로 구현된다. 반면 `AdmissionRule`의 비용 계정은 binary64이며, 이상적인 실수 계산의 10% 부등식을 roundoff까지 엄밀하게 보장하려면 정수 비용 단위 또는 outward cost account가 필요하다. 이번 수정은 nonfinite 누적으로 guard가 붕괴하는 경로를 닫는다.
- `AdmissionRule`의 Wilson pseudo-count 수치는 Beta posterior quantile의 엄밀한 upper bound가 아니며 소스는 이를 밝힌다. fixed-z optimistic policy의 UCB1 regret 정리는 별도 탐색·confidence-time 조건 없이 따라오지 않는다. 유용한 보장은 measured/estimated cost 차이를 명시한 deterministic overhead guard다.
- O-01 fail-closed 경로가 적용되면 완성되지 않은 estimate로 결과를 통과시키는 경로가 닫힌다. production 승격에는 모델·출력·horizon·자원 범위와 해당 정확도/성능 acceptance를 별도로 충족해야 한다.

## 실행 근거

- `evidence/dense_transaction/red.log`: 실제 기존 동작에서 3개 시험 중 2개 실패, reporting 1개 통과.
- `pre_fix_pins.log`: 변경 전 PR 및 Robertson, 두 enforcing mode의 endpoint bits, attempts, samples, rejected counts를 저장. 새 regression test에서 동일 값을 그대로 assert한다.
- `green.log`: O-01 4개 시험 통과.
- `regression.log`: all-DAG dense port와 union 후 32개 통과, 기존 ignored 1개 유지.
- `admission_red.log`: 누적 protected·attempt·saved cost와 config의 4개 hostile 시험이 기존 코드에서 모두 실패.
- `admission_artifact_failure.log`: 수정 후 공유 target에서 library compile 다음 E0463가 발생. 원인을 확정하지 않았으며 scientific failure로 보지 않는다. 격리된 target에서 동일 소스 재빌드로 판별한다.
- `test_compile_failure.log`, `admission_test_compile_failure.log`: 새 test 작성 중 API 사용 실수의 최초 compile failure도 보존했다. 제품 결함으로 세지 않는다.
- `GLOBAL_BUDGET_PROOF.json`: G1 finite propagation 예제 3개와 G2 exact witness 및 근방 feasible allocation 211개를 유리수 연산으로 검사했다. oracle는 theorem 자체의 대체물이 아니다.
- 최종 source-bound 시험 결과와 hashes는 `dense_transaction.json`에 기록한다.

## 독립 검토와 연구 루프

Facade 담당자가 O-01의 failure-kind 보존, attempted-work, outer counter reconciliation, state/history/recycle rollback 및 C1 prospective-account validation을 별도로 읽고 PASS로 판단했다. 연구 감사 담당자가 G1/G2의 Hölder와 gain placement를 검토했고, 절대연속성·jump charge 및 양의 실수 relaxation 범위를 명시하도록 지적했다. 모두 반영했다. Root는 G2의 비용 ceiling과 실제 비용의 차이를 명시하도록 지적했고 반영했다.

R1/C1 기준 SHA와 목적 → R2/C2 기존 계획·실행·source 대응 → R3/C3 scalar/normal/continuous/local/global의 가정 정렬 → R4/C4 fail-closed 후보와 병렬 담당 분담 → R5/C5 G1/G2 실제 증명·O01/TX01 코드 → R6/C6 독립 리뷰와 정확 유리수 oracle → R7/C7 hostile native tests → R8/C8 failed와 passed 로그 및 원래 tolerance 보존 → R9/C9 코드/증명/JSON 대응 → R10/C10 통합 보고서와 다음 개발 계약으로 이어졌다.

다음 개발은 model-bound `GlobalBudget`의 미래 gain envelope와 uniform density부터 시작한다. 기존 프리레지스터 G1 study로 generic facade의 local underestimation과 propagation을 구별한 뒤에만 controller card를 열어야 한다. arbitrary black-box RHS에서 point sample을 certified residual로 승격하지 않는다.

최종 격리 target 실행은 **56 native PASS, 0 FAIL, 1 기존 ignored**로 종료했다. ignored는 이미 실패 판정된 PR step-count kill gate이며 다시 채점하지 않았다. 동일 최신 source에서 C1 20개 기존 계약과 4개 새 overflow 계약이 모두 통과했다.
