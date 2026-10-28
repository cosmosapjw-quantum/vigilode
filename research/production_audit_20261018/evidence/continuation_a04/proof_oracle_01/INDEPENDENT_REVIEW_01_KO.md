# A04-PROOF-ORACLE-01 독립 검토

판정은 **READY**다. 적용 범위는 동결된 proof-oracle fixture 구현, harness 및
기록된 실행이다. 차단 finding이나 실제 under-enclosure는 발견하지 못했다.
Full A04의 권한과 판정은 **EstimateOnly / HOLD**를 유지한다.

이 문서와 인접 JSON의 검토 내용은 `a04_oracle_independent_review`가 작성했다.
검토자는 읽기 전용으로 작업했으며 파일 저장은 controller가 담당한다.
요청된 역할은 COSTROUTE01 research-review이고 실제 모델 runtime identity는
`NOT_INDEPENDENTLY_OBSERVED`다.

## 확인한 근거

검토 checkout은 `/home/cosmosapjw/vigilode`, branch는
`audit/production-readiness-20261018`, HEAD는
`7e2873887ed6038b47b667c976435810851c5dba`다.

최종화 직전에 `FINAL_V3_SOURCE_IDENTITY.json`의 모든 source binding과
`ARTIFACT_SHA256.json`의 35개 항목을 다시 읽어 확인했다. 불일치는 없었다.
동결 계약의 derivation, preregistration, decision 해시도 일치한다.
개별 검토 파일의 정확한 SHA-256은 인접 `INDEPENDENT_REVIEW_01.json`에 기록했다.

12개 action과 4개 standalone coefficient fixture의 입력 bits, budget,
degree cap 및 기대 분기가 동결 계약과 일치한다. Action은 성공 반환 8개와
`CoreError::NonFinite` 거절 4개다. 예산을 충족하지 못한 F07/F08은
`Ok(report)`와 `converged=false`를 유지한다.

계수 oracle은 complete homogeneous polynomial의 exact Fraction 급수와
엄밀한 꼬리를 사용한다. Fused oracle은 represented matrix와 vector에 대한
exact matrix-phi Taylor 급수를 사용한다. Coefficient의 `M+2>R`, 각 active
fused k의 `M+2+k>B` 조건, squared-distance 상·하계와
PASS/FALSIFIED/INCONCLUSIVE 판정이 계약과 일치한다.

218개 exact 비교가 모두 M=64에서 PASS다. 검토자가 보존된 native receipt와
검토한 oracle 함수를 사용해 메모리 안에서 결과를 재계산한 결과도 12개 수치
행의 비교 결과 및 rational digest와 일치했다. 이 재계산은 exit 0이며 파일을
쓰지 않았다. 새 독립 알고리즘을 추가한 실행이라는 의미는 아니다.

Allocation 양성 대조군은 같은 thread와 allocator 계측에서 3회를 검출했다.
그 이후 F02/F03/F04의 동일 capacity/cache 성공 호출은 각각 0회를 관측했다.
따라서 이 세 warmed 성공 호출에 한해 `ZERO_ALLOCATION_PASS_SCOPED`를 인정한다.
Calibration 실패 시 `ALLOCATION_UNAVAILABLE`로 allocation 승인을 보류하고
수치 oracle 입력을 유지하는 코드도 확인했다.

네 거절 모두 직전 성공 report를 보존했다. WorkCounters의 before/after와
기록된 delta가 일치한다. F09는 coefficient setup 1회 및
`last_certification_ops=26`을 남긴다.

최초 compile 실패(exit 101)는 `JointPhiInput::Distinct`의 `&[Vec<f64>;5]`
API에 맞지 않는 새 test adapter에서 발생했다. 첫 실패 로그와 후속 실행 기록이
보존되어 있다. 최종 Rust test와 Fraction 실행 기록은 각각 exit 0이다.
Production source와 Cargo.lock은 동결 identity와 일치한다.

## 적용 범위와 한계

- F09는 초기 outward L2 norm에서 finite-input overflow를 거절한다. Matvec
  overflow 실행 근거로 확대하지 않는다.
- 계수 비교는 별도로 조회한 public helper 표를 검증한다. 실제 내부 cache 표
  자체를 직접 추출하지 않았다. 실제 반환 fused 중심의 포함성은 별도로 검사했다.
- 실행된 oracle 차수는 M=64다. M>64 refinement와 cap exhaustion은 코드 검토
  범위이며 이번 fixture에서 실행되지 않았다.
- Calibration은 실제 실행에서 성공했다. Calibration 실패 처리는 코드 검토
  근거이며 실패 경로를 실행했다는 주장은 하지 않는다.
- 검토자는 기존 Rust 실행 로그를 읽었으며 Rust test를 재실행하지 않았다.
  Source 수정, 새 test 추가, commit/push도 수행하지 않았다.
- 이 결과는 모든 입력의 Leja 산술 인증, 전체 workspace나 feature 조합,
  성능 우위의 근거가 아니다.

이 bounded proof-oracle 노드를 닫고 다음 dependency-safe DAG 작업으로 진행할
수 있다. Full Leja authority는 남은 source-matched 증명 및 admission 의무를
충족할 때까지 EstimateOnly/HOLD다.
