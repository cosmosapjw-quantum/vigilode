# Production facade 재감사와 구현

기준은 `cc2e1a0`의 계획과 `f2f20c1`의 실제 wave-3 코드를 합친 `4a5fc656730fe8243f06b6e603cb8aa758b00b2f`이다. 담당 범위는 `crates/vigilode`: 공개 입력·라우팅·실패·연속출력·event·finite difference 계약이다. 이전 wave-3의 15개 event 계약이 이 환경에서도 그대로 통과함을 먼저 기록했다. 이전에 닫힌 RODAS5P 차수나 coefficient 정리를 다시 증명하지 않았다.

## 판정과 수정

1. **F-FAC-01 / O-01 — enforcing sequential 경로의 검증 실패 후 수용.** 기존 mass 또는 `InteriorError::Enforce` 경로는 내부오차 추정이 실패해도 `unavailable`만 늘리고 step을 수용했다. 따라서 같은 공개 옵션의 fast 경로와 계약이 달랐다. `dense_transaction_audit`가 integrator의 reject/retry와 상태 rollback을 수정했고, 이 노드는 `DriverWork::SequentialDense::estimate_failures`, `vigilode.stats.v4`, 문서, golden record, fast/mass/Enforce 세 facade 경로의 실제 recoverable-midpoint 시험을 연결했다. `unavailable`는 report-only이다. 이 수정은 추정 계산의 실패를 성공으로 처리하던 문제를 닫으며, 추정량을 전역오차 certificate로 바꾸지는 않는다.
2. **F-FAC-02 / O-09 — event 상태의 잘못된 실패 보고.** `extension=None`이면 조용히 event 검사를 건너뛰고, root에서 `at(t)`가 없으면 다른 시간의 endpoint 상태를 event에 붙였으며, Illinois 내부의 `at()` 실패는 사용자 함수의 NaN으로 보고했다. 정상 solver가 생성하는 자료에서 이 세 경로가 실제로 발생했다는 주장은 하지 않는다. 코드상 실패 처리 결함을 fault injection으로 검사했다. 이제 `EventInterpolationFailed { t }`를 반환하고 해당 step 전체의 event 결과를 폐기한다. 사용자 함수의 비유한 값은 기존 `EventFunctionNonFinite`를 유지하며 이미 존재하는 driver failure의 우선순위도 유지한다.
3. **F-FAC-03 — 유한 상태에서 FD perturbation의 불필요한 overflow.** `y_j=f64::MAX`에서 `y_j + sqrt(eps)*|y_j|`가 무한대여서 유한한 smooth RHS도 평가할 수 없었다. forward shift가 overflow하는 경우에만 방향을 반전하고 실제 signed increment로 나눈다. 정상 범위의 연산은 bitwise 동일하다. dense/banded 두 구현에서 비유한 derivative entry도 factorization 전에 명시적으로 거절한다. 독립 oracle는 `f_j(y)=2^-1023 y_j`의 정확한 대각 미분 `2^-1023`이며, `±f64::MAX`에서 두 경로 모두 이를 재현하는지 검사한다. 이는 FD helper의 유효범위 개선이며, 극단 상태의 전체 ODE 적분 정확도를 증명한 것은 아니다.
4. **F-FAC-04 — 공개 band routing 문서.** dense output이면 항상 dense LU라는 오래된 설명을 실제 fast-banded-dense 경로에 맞게 고쳤다. `Jacobian::Dense`의 band 선언 거절은 `StructureRefused`가 아니라 실제 `Unsupported`로 정정했다. singular mass가 자동으로 index-1이라는 표현도 고쳤다. index-1과 consistent initial data는 호출자의 수학적 전제이며 현재 facade는 index 판정·consistent initialization을 수행하지 않는다.

## 구성적 정리와 계산 비용

### T-FAC-1: signed FD increment의 정확도 차수 보존

스칼라 성분의 해당 방향 제한을 `g(s)=f_i(y+s e_j)`라 하자. 유한한 비영 `δ`와 선분 `[0,δ]`에서 `g`가 두 번 미분 가능하고 `|g''|≤M`이면 적분 Taylor식으로

\[
\frac{g(\delta)-g(0)}{\delta}-g'(0)
=\frac{1}{\delta}\int_0^\delta(\delta-s)g''(s)\,ds,
\qquad |\cdot|\le \frac M2|\delta|.
\]

음의 `δ`에도 절댓값 치환을 하면 같은 bound가 성립한다. 따라서 overflow 시 방향 반전은 forward difference의 일차 truncation order를 유지한다. 실제 계산은 `(shifted-y_j)`를 분모로 써서 입력의 표현 가능한 perturbation과 일치시킨다. callback 오차와 부동소수점 차분오차는 별도 항이며 이 정리는 이들을 생략한 exact-arithmetic truncation 정리이다. signed fallback은 전체 구간의 smoothness를 자동 보장하지 않는다. domain 밖의 callback은 기존 recoverable error로 처리된다.

비용은 dense의 `n+1`, bandwidth `w=lower+upper+1`인 grouped band의 `min(w,n)+1` RHS call로 그대로다. 입력 방향만 바뀌므로 callback 추가 호출이 없다. column support가 겹치지 않는 기존 grouped FD 논증도 유지된다.

### T-FAC-2: event 스캔의 scratch allocation 감소와 결과 보존

`E`개의 event와 `N≥1`개의 완전히 검사한 accepted step을 가정하자. 기존 `scan_step`은 event-step마다 길이 2의 `Vec`을 만들었다. 현재 구현은 고정 길이 2의 stack array와 유효 길이를 사용한다. sampling 순서 `(endpoint, midpoint)`, 두 half의 순서, Illinois의 입력과 연산은 같다. 따라서 동일 callback과 정상 extension에서 callback 호출열과 root 결과는 bitwise 동일하다. allocator 요청을 코드 수준에서 정확히 `E N`회 제거한다. step마다 새로 만들던 `next`를 두 buffer의 swap으로 재사용하므로 별도로 `N-1`개의 allocation 요청을 제거한다. crossings buffer도 재사용한다.

이는 결과를 바꾸지 않는 구성적 비용 감소이다. 총 work의 점근 차수 `O(E N + root evaluations)`는 같고, `Solution::at`의 state vector allocation과 발견한 event 저장은 남는다. compiler가 allocation을 제거할 수 있으므로 코드 수준 allocator 요청의 감소를 실제 wall-clock speedup으로 환산하지 않는다. 현재 기록에는 matched-accuracy wall-clock 성능 주장이 없다.

## 지원 범위와 남은 개발

- 일반 ODE의 공개 Protected route는 local embedded/interior **estimate**를 사용하는 solver이다. `Status::Success`가 requested global error bound를 뜻하지 않는 기존 의미를 유지한다. `Tier::Performance`는 아직 Protected와 같은 경로이며 연구 후보를 몰래 활성화하지 않는다.
- mass 경로는 constant mass와 일관된 index-1 입력, 직접 Jacobian 또는 FD를 필요로 한다. vector atol은 이 sequential 경로에서 여전히 명시적으로 Unsupported이다. 이것을 자동 scalar로 바꾸지 않았다.
- event는 계속 post-processing이며 terminal event 이후 계산 비용도 모두 남는다. three samples 사이 짝수 개 교차나 접선 접촉을 놓칠 수 있다는 공개 계약은 기존 시험과 함께 유지한다. 이 한계는 새로운 no-go 결과가 아니다.
- 다음 구현 우선순위: (1) in-loop event transaction으로 terminal 이후 불필요한 적분을 없애되 step acceptance·callback failure·work accounting을 함께 보존, (2) caller-scratch `Solution::at_into`로 root iteration의 `O(n)` allocation 제거, (3) facade-owned Stats mirror와 schema fixture 확대로 internal type에 대한 semver 결합 축소, (4) sequential vector-atol plumbing. 각각 독립된 API/성능 card로 시행해야 한다.

## R1–R10 / C1–C10 실행 연결

R1/C1은 위 기준 SHA·범위, R2/C2는 공개 함수→route→driver 및 기존 15 event 계약, R3/C3은 local estimate·DAE·event/FD 전제의 구분, R4/C4는 failure typing·signed increment·scratch 재사용의 후보 선택과 integrator 담당자와의 병행 분업이다. R5/C5는 T-FAC-1/2 유도와 실제 코드, R6/C6은 exact power-of-two oracle 및 내부 fault injection, R7/C7은 외부 facade 세 경로의 midpoint 실패와 analytic decay, R8/C8은 source-bound 시험 기록과 한정된 비용 주장, R9/C9는 이 문서·JSON·독립 검토 요청, R10/C10은 통합 보고서와 재실행 명령에 연결한다. 독립 검토 상태와 실제 시험 종료값은 `facade.json`에 별도 기록한다.

## 실제 실행 결과

`evidence/facade/targeted_retry.log`의 최종 결과는 **81 native tests PASS**(unit 13, events 15, facade 20, typed API 9, P0 24)이다. doc test 1개와 compile-fail doc test 1개도 통과했다. `cargo clippy -p vigilode --all-targets --locked --offline --no-deps -j 1 -- -D warnings`는 0으로 종료했다. dependency의 기존 warning까지 해결했다는 뜻은 아니다.

첫 표적 compile은 E0463(`can't find crate for vigilode`)와 연쇄 type-inference 오류로 101 종료했다. 원인은 확정하지 않았다. 원본 로그를 유지하고 해당 package만 `cargo clean -p vigilode` 후 `-j 1`로 재빌드했으며 위 통과를 얻었다. 전체 target 또는 cache를 삭제하지 않았다. 이 실패는 수학적 반례나 FD/event 회귀로 분류하지 않는다. 소스와 로그 SHA256은 `facade.json`에 기록했다. 독립 reviewer의 최종 판정은 통합 검토 기록에 연결한다.

독립 검토자 `dense_transaction_audit`는 event/FD diff, signed fallback의 exact oracle, event fault injection, allocation 감소의 산식을 직접 검토하여 **PASS_SCOPED**로 판정했다. blocking finding은 없으며 `evidence/facade/facade_peer_review.json`에 검토 범위와 해당 소스 hash를 기록했다. 이 노드는 별도로 상대 담당자의 O-01 상태 rollback/실패 work accounting과 C1 prospective overflow refusal을 검토했고 `dense_peer_review.json`에 한정된 PASS를 남겼다.
