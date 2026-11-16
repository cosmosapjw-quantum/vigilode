# U06 통합 method capability registry — 2026-11-16

판정: **U06 COMPLETE_SCOPED**. 순수 registry와 계약 테스트다. 기본 라우팅·`GeneralOde::select`·`Rodas5p::capabilities`는 바꾸지 않았다. 사전 계약은 `CHANGE_CONTRACT.json`.

## 구현 (`crates/vigilode/src/selection.rs`에 추가, glob export, `lib.rs` 무변경)
- `CapabilitySet {provided_jvp, finite_difference, automatic_differentiation, explicit_jacobian, dense_matrix, newton}`: 요청에서는 허용, descriptor에서는 단계별 필요다. 서로 독립인 집합이다.
- `MethodDescriptor`: method id·version·tableau identity·stages·order·history·startup, 요구 variant(per_step·startup·fallback, variant별 mass/events/dense output 지원), qualification(Production / ScopedCalibration / ImplementedUnqualified / ReferenceImplementationOnly), access(GeneralOdeSelectable / ExplicitApiOnly), 실제 export route.
- `MethodRegistry::query`는 순수 함수다. variant 하나가 세 단계 모두 허용 집합 안에 있고 요청 기능을 지원하며, 자격이 Production·ScopedCalibration이거나 `admit_unqualified`일 때만 Admitted다. 거절에는 typed 이유(단계·variant 포함)를 단다.
- `CapabilitySet::from_problem_and_policy`는 기존 `Problem`의 Jacobian 형태와 `GeneralOdePolicy`를 callback 없이 매핑한다. Newton은 결코 암시하지 않는다(fail-closed).
- `MethodRequest`/`CapabilitySet`은 serde `deny_unknown_fields`, 기본값 없음이다.

## 등록된 method (코드에서 확인한 요구)
| method | per-step | startup | 자격 | 접근 |
|---|---|---|---|---|
| dopri5 | RHS만 | 없음 | Production | GeneralOde |
| rodas5p | 명시 J+dense / FD+dense / JVP(matrix-free, mass·dense output 없음) | 없음 | Production | GeneralOde |
| ppsw1-6b | (명시 J 또는 JVP 열)+dense | 같은 것+Newton | ScopedCalibration(U03) | 명시 API |
| ppsw1-6b matrix-free | JVP | dense+Newton starter | ImplementedUnqualified(U04 PARTIAL) | 명시 API |
| bdf1-bdf2 | (명시 J 또는 JVP 열)+dense+Newton | 같음 | ImplementedUnqualified | 명시 API |
| radau-iia3 | (명시 J 또는 JVP 열)+dense+Newton | 없음 | ReferenceImplementationOnly | 명시 API |

자동 fallback이 있는 method는 없다. 전체 descriptor는 `DESCRIPTORS.json`.

## 검사
- `unified_method_capabilities.rs` 6/6: 2^6 허용 × 2^3 기능 × 자격 플래그 = 1024 요청을 계약에서 다시 적은 독립 표와 일치시켰다. 요청 생성과 query 동안 callback 0회다. policy 매핑은 집합이고, policy에서는 Newton이 없어 PPSW1이 startup 단계에서 거절된다. serde round trip과 미지·누락 필드 거절, 모든 route가 컴파일되는 export인지 확인, Peer mass/events는 작업 전에 거절된다.
- 호환성: vigilode 전체 suite 325 통과/0 실패(기존 319 + 신규 6), strict `clippy -D warnings` 통과, fmt 통과.

## 한계
registry는 선언적 기록이며 정확도 인증이 아니다. BDF/Radau의 자격은 코드와 CLI 표기에서 읽은 것이며 별도 qualification이 없다. 비정규·대형 문제에서의 경제성은 기록하지 않는다. U07에서 Session adapter가 이 registry를 소비해야 실제 선택 경로가 된다.
