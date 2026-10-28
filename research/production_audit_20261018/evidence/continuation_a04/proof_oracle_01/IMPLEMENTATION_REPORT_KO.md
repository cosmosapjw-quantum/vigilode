# A04-PROOF-ORACLE-01 구현·실행 보고

새 Rust 통합 테스트와 독립 Fraction oracle을 구현했다. 기존 frozen manifest의
12 action 및 4 coefficient table 입력 bits, budget, degree cap, 기대 분기와
고정 oracle 순서 64→128→256→512→1024를 그대로 사용한다. production Leja,
Cargo manifest/lock, 허용오차, 다른 작업자의 A05 변경은 수정하지 않았다.

최종 `final_v3_run_record.json`의 scoped Rust test는 exit 0, 1 test 통과다.
`fraction_final_v3_run_record.json`의 Fraction 실행도 exit 0이며 218개 exact
포함성 비교가 모두 M=64에서 PASS다. 상·하 squared distance로 PASS /
FALSIFIED / INCONCLUSIVE를 구분하며, coefficient x_i는 개별 bits의 exact
rational c+gamma*xi로 구성한다. 실제 finite center/bound bits를 비교한다.
급수 및 remainder는 production Opitz/recurrence와 독립된 표현이다.
각 비교의 exact rational sign, numerator/denominator hex encoding의 hash와
bit length를 보존하므로 입력·스크립트로 재계산할 수 있다.

Allocation 양성 대조군은 같은 thread/allocator에서 3개 Vec clone 할당을
검출했다. 그 이후 같은 capacity/cache의 F02/F03/F04 성공 재실행에서 각각
0개를 관측했다. 거절 경로의 allocation 주장은 없다. Calibration 미검출 시에는 native receipt와
Fraction execution classifier가 ALLOCATION_UNAVAILABLE를 남기고 allocation
승인만 보류한다. 이 계측 부재를 solver 수치 실패나 일반 test panic으로
바꾸지 않으며, 보존한 native 결과의 수치 oracle은 계속 실행할 수 있다. 모든 refusal은 같은
workspace의 직전 성공 report를 byte serialization 기준으로 보존했으며,
WorkCounters checked_delta와 setup receipt, 마지막 certification work를
각 시도 전후에 보존한다. F09의 범위는 초기 L2 norm 중 finite-input
arithmetic overflow 거절이며 matvec overflow 검증으로 표현하지 않는다.

첫 실패는 `first_test.log`에 보존한 Rust API 형태의 compile 오류(exit 101)다.
Distinct가 `[&Vec;5]` 대신 `&[Vec;5]`를 요구하는 문제를 새 test adapter에서만
고쳤다. 이후 원시 성공 결과도 각 버전 파일에 별도로 보존했다. 후속 수정은
report authority와 byte preservation assertion, setup receipt, 일반 cargo test의
환경변수 선택 처리다. native 초기·최종 출력과 Fraction 결과는 덮어쓰지 않았다.

계수 추가 helper query의 의미를 native receipt에 적었다. Newton은 실제
c/gamma/사용 node prefix의 public helper query이며, scalar는 내부 scalar helper와
같은 c/gamma/node0에서 public helper를 별도 조회한다. zero branch는 실제
coefficient table을 사용하지 않으므로 그 별도 query를 사용 table이라고 주장하지
않는다. fused reference는 8개 실제 성공 반환 중심 모두를 검사한다.

`REVIEWED_INPUT_BINDINGS.json`은 독립 계약 검토의 6개 source/contract hash가
실제 파일과 일치함을 기록한다. 전달 지시에 나온 SOURCE_BINDINGS.json은
존재하지 않아 실제 CONTRACT_INDEPENDENT_REVIEW_01.json의 source identity를
사용했다. 최종 시험 source는 `FINAL_V3_SOURCE_IDENTITY.json`에 결속한다.

현재 결과는 bounded fixture 실행 근거다. 중요한 diff와 수식/실행 결과의 독립
검토는 미실행이며 full A04는 EstimateOnly/HOLD다. production repair를 유발하는
FALSIFIED 결과는 없었다. 전체 workspace/feature 조합/benchmark, commit/push도
이 실행 단위에서는 수행하지 않았다. 다음은 controller의 독립 검토 및 DAG 전이다.
