# N01 — facade Stats 경계와 실행 가능한 지원 계약

**현재 상태: 구현 완료. Facade native 85개, doc test 2개, warning-denied rustdoc, 실제 예제 4개 모두 PASS.** 기존 감사의 event/FD/O-01 수정과 검증은 재사용했다. 이번 변경은 공개 결과 타입의 소유권과 지원 조합 조회를 마무리한다. 새로운 수치 경로의 production 승격이나 전역 오차 보증을 뜻하지 않는다.

## 구현 내용

[D] `records.rs`에 22개 facade 소유 record/enum을 추가했다. `Stats`, `RouteRecord`, `StopRecord`, `DriverWork`의 공개 필드는 이 타입과 기존 facade/표준 타입만 포함한다. 내부 dense extension 저장소는 비공개로 유지된다. 실제 `WorkCounters`는 61개 필드이며, 계획 문서의 62라는 개수는 현 소스와 다르다. 61개 모두 이름과 순서를 유지한다.

[D] source struct를 `..` 없이 분해하고 source enum을 wildcard 없이 변환한다. 내부 필드/variant가 추가되면 컴파일러가 해당 변환의 갱신을 요구한다. 모든 numeric field를 서로 다른 nonzero 값으로 채운 fixture와 모든 enum variant의 JSON 비교를 추가했다. router 결과는 consuming 변환을 사용하여 band 검증의 `relative_mismatch` Vec를 복사하지 않는다.

[D] `STATS_SCHEMA`는 `vigilode.stats.v4`를 유지한다. 변환 적용 전에 실제 Robertson dense+event 실행에서 pretty JSON을 채집했다. 원본 snapshot source, 변환 전 source hash, golden hash, 성공/실패 로그를 함께 보존한다. 새 golden을 변환 후 출력으로 덮어쓰지 않는다.

[D] `Rodas5p::capabilities(&problem, &options)`는 user callback과 solver workspace 생성 없이 선택될 route, 실제 tier, local-error 계약, event 후처리 여부를 반환한다. 지원 불가 사유는 `solve`가 사용하는 동일한 두 단계 predicate를 공유하므로 기존 입력 거부 순서를 보존한다. 수치 입력 유효성, band 실측 검증, 실제 solver 성공 여부까지 보증하는 조회는 아니다.

## 지원 행렬과 정확도 의미

384개 조합은 provider 4종 × band 선언 2종 × mass 2종 × dense 2종 × vector-atol 2종 × event 2종 × interior 3종이다. 각 조회에서 RHS/Jacobian/JVP/event 호출이 0인지 검사하고, 빈 span의 실제 `solve` 결과와 typed refusal 또는 driver ID를 비교한다. 6개 driver family와 7개 Unsupported variant를 모두 포함해야 통과한다. 384조합 실제 실행은 PASS이며 원자료는 `evidence/n01/CAPABILITY_MATRIX.json`에 저장했다.

| 계약 | 명시한 의미 |
| --- | --- |
| endpoint only | endpoint local estimate를 사용한다 |
| dense, interior Off | dense interior가 제어되지 않는다 |
| dense, interior enforce | endpoint와 midpoint local estimates를 사용한다 |
| global error | facade certificate 없음 |
| mass/DAE | consistent initial data는 caller 책임 |
| Performance tier | 여전히 Protected를 실행한다 |
| events | accepted segment의 후처리이며 terminal 이후 적분 비용도 기록한다 |

## 호환성 및 적용 범위

[I] v4 JSON의 필드명·순서·태그를 보존하면서 내부 타입과 공개 결과 구조의 결합을 줄인다. 이는 Rust 타입 자체가 이전과 동일하다는 의미는 아니다. 안정 semver baseline 이전에 output field의 타입은 facade 소유 타입으로 변경된다. `From<&Source>`와 직접 비교용 `PartialEq` bridge를 제공하지만 enum pattern, `Option`, tuple 또는 명시적 타입 annotation은 새 이름으로 바꿔야 할 수 있으며, 내부 `WorkCounters`의 메서드를 호출하던 코드는 별도 이행이 필요하다. 이 저장소의 해당 테스트는 함께 갱신했다.

[D] sequential/matrix-free vector atol, JVP mass/dense output, JVP band, dense Jacobian+declared band의 mass/dense 조합은 여전히 typed refusal이다. 이들을 지원한다고 표시하거나 미검증 수치 구현을 노출하지 않는다. in-loop event 종료, tangent/multiple-root 완전 검출도 이번 범위에 포함하지 않는다.

## 실제 연구/코딩 루프

R1 범위: N01의 타입/지원 계약으로 한정. R2 기존 wave4 Task5와 facade 감사 근거 재사용. R3 v4 wire equality와 총 work 보존을 불변식으로 설정. R4 recursive source→owned 변환을 구성. R5 선언 순서와 serde tag의 구조적 대응으로 직렬화 보존을 유도. R6 모든 source field/variant를 빠짐없이 요구하는 컴파일러 의무를 둠. R7 nonzero fixture·기존 golden·새 dense-event golden을 반례 탐색 oracle로 사용. R8 지원 조합 384개를 유한 열거. R9 exact conversion/query 범위만 주장. R10 별도 JSON evidence와 소스 hash로 handoff.

C1 기존 타입과 route 조사. C2 변환 전 실제 golden 채집. C3 record/query 계약 테스트 작성. C4 exhaustive mirrors와 consuming Vec 이동 구현. C5 공개 결과 변환 및 validation predicate 공유. C6 모든 callback 무호출/지원-실행 일치 검증. C7 기존 facade 회귀와 docs/examples 검증. C8 독립 source review. C9 실패 원자료와 정확한 build identity 보존. C10 로컬의 추가 코딩 의무가 아닌 pinned verification/등록 benchmark 작업으로 인계. C6–C9의 실행 상태는 아래 검증 결과와 JSON을 따른다. 인프라 compile 실패를 semantic RED 증거로 세지 않는다. 실제 첫 record 실행에서는 12개 conditional zero-counter serde 속성 누락으로 두 JSON 테스트가 실패했다. 원본 golden을 유지한 채 속성을 복원했으며, default JSON key 열거 대신 61개 필드를 직접 nonzero로 초기화하는 fixture로 강화했다. all-zero 비교도 추가했다. 이후 모두 PASS했다.

[도출] 유한 record tree에서 각 leaf의 값·serde representation을 보존하고 각 struct field 및 enum variant 순서/태그를 동일하게 유지하면, 해당 tree의 직렬화는 구조적 귀납법으로 같다. 이 명제는 새 수치 정확도나 속도 정리가 아니다. consuming router 변환은 기존 Vec storage를 이동하므로 추가 vector allocation/copy가 필요 없으며 pointer identity를 테스트한다. wall time 향상은 주장하지 않는다.

## 검증 상태와 남은 실행

PASS: record 3개, capability 384조합, facade 전체 native 85개, doc test 2개. `RUSTDOCFLAGS=-D warnings cargo doc --no-deps`와 실제 예제 4개도 PASS했다. 모든 성공 receipt의 source-before/after가 동일하다. 최초 독립 source review가 놓친 serde 속성 누락은 native 테스트에서 발견했다. 수정 후 독립 재검토도 PASS했으며 `peer_review_final.json`이 초기 검토를 대체한다. 누락과 실패 로그를 숨기지 않고 보존했다. 원래 snapshot을 생성한 final-target 실행은 PASS이며 오래된 shared target의 rlib 문제로 실패한 최초 로그는 보존했다.

로컬에는 추가 N01 코딩 작업을 넘기지 않는다. 통합된 source에서 pinned toolchain 통합 검증·semver baseline 정책 검사와 등록된 native comparator/NCP benchmark를 실행해야 하며, 원래 tolerance와 실패 row를 보존한다. production 판정은 그 외 검증 및 root의 통합 보고서와 함께 판단한다.


## 재현 evidence

| 항목 | 증거 |
| --- | --- |
| field/enum/golden 및 384조합 | `evidence/n01/contracts_03.{log,json}` |
| facade 85개 | `evidence/n01/facade_all_02.{log,json}` |
| doctest 2개 | `evidence/n01/facade_docs.{log,json}` |
| warning-denied rustdoc | `evidence/n01/facade_rustdoc.{log,json}` |
| 실제 예제 4개 | `evidence/n01/example_{robertson,dense_output,dae_mass,banded_fd}.{log,json}` |
| API 공개 field 조사 | `evidence/n01/public_api_audit.json` |
| 기계 판독 지원 행렬 | `evidence/n01/CAPABILITY_MATRIX.json` |
| 보존한 실패 | `contracts_01.log` 문법, `contracts_02.log` 실제 JSON 회귀, `facade_all.log` 기존 test 타입 이행 |

공개 field 직접 조사 명령은 `rg -n 'pub .*(rodas5p_core|rodas5p_integrators)::|pub use rodas5p_' crates/vigilode/src`이며 출력은 없고 exit 1(일치 없음)이다. `solution.rs`와 `records.rs`의 alias/payload를 수동으로 재귀 추적했다. 계획에서 요구한 `From`/호환 비교 trait impl 자체에는 명시적으로 source 타입이 등장하므로, 정확한 주장은 공개 record field의 분리이지 모든 공개 trait signature에서 내부 타입을 없앴다는 주장이 아니다.
