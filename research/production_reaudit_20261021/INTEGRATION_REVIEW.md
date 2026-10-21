# 최종 통합 검토: N01–N08과 실행용 bound clients

**판정은 “명시된 범위의 코드와 주장 정합성 수용, 최종 통합 실행·발행 대기”다. 범용 production·임의 RHS 전역 정확도·일반 속도 우위는 HOLD를 유지한다.** 이번에는 하나의 노드만 이식한 것이 아니라 기존 두 줄기의 보존 작업 위에 N01–N08의 구현, 실제 비교 프로그램, 기계 판정 경로를 연결했다. 구현 완료라는 표현은 각 노드의 `delivered_scope`에 한정한다.

이 검토는 현재 소스와 노드별 보고서를 대조했다. 이미 발표된 증명과 과거 scientific campaign은 다시 실행하지 않았다. 최종 native matrix와 push는 부모 작업의 책임이며, **`EXECUTION.json`, `SOURCE_MANIFEST.json`, `EVIDENCE_MANIFEST.json`이 동적인 최종 권위**다. 검토 시점의 앞선 실행은 각 명령 성공에도 source 변경을 발견해 FAIL로 보존되어 있다. 이를 최종 PASS로 대체해 읽으면 안 된다. 노드별 중복 시험 횟수도 합산하지 않았다.

검토자 소유 구현은 N06/N07과 `reaudit_bound_clients`다. 이 부분의 독립성 근거는 부모의 소스 검토와 기록된 native contracts/smoke이고, 본 문서 자체를 또 하나의 독립 코드 감사로 세지 않는다. 다른 담당자가 구현한 API의 수학·소유권 경계와 전체 보고서의 일관성은 별도로 교차 검토했다.

## 노드별 구현과 닫히지 않은 경계

| 노드 | 현재 소스에서 확인한 구현 | 검증과 한계 |
|---|---|---|
| N01 | facade 소유 record, exhaustive 변환, 실제 지원 조합 질의 | v4 JSON 유지와 384개 조합 검사는 기존 receipt를 재사용한다. Rust 타입 identity 변경에 따른 downstream migration과 안정 API 검토가 남는다. 지원 질의는 전역 인증을 부여하지 않는다. |
| N02 | build-time source closure, source/corpus/binary 실행 전후 검사, RADAU5/CVODE 전체 refinement ladder | standalone 및 실제 harness+명시적 mock adapter 검증이다. exact dependency/native toolchain 부족으로 전체 bench typecheck/link는 미실행이다. grid agreement는 연속시간 상계가 아니다. |
| N03 | private model/run identity에 묶인 고정 L2 global replay, 초기오차·jump·gain·query rounding 계상 | 원래 source absolute budget보다 큰 요청은 거부한다. sealed provider의 유효한 tube를 재사용하는 post-run 계약이다. 임의 callback이나 online controller로 확대하지 않는다. |
| N04 | outward 정수 배분 검증, 자원 한계, 같은 분모의 항 묶기와 안전한 fallback | 실수 seed는 제안이고 마지막 outward validation이 권위다. 입력 A_j/c_j를 실제 ODE의 오차·비용 법칙으로 증명하는 일은 별도다. |
| N05 | 소유된 선형 matrix/norm에 대한 Cayley/Bernstein target 계획·후보 평가, 실제 Diagonal staleness 경로 | output/embedded estimator와 반환 중심 비용은 다룬다. ODE truncation·nonlinear drift 인증은 아니다. **계획기는 현재 일반 driver의 target으로 연결되지 않았고 `used_by_driver:false`다.** |
| N06 | Krylov 및 polynomial-priority routing의 동일 IVP/budget endpoint 비교 | `protected` arm은 명시적으로 analytic diagonal exponential baseline이며 protected RODAS5P가 아니다. Laguerre 등의 이름은 fallback 가능한 우선순위이고 실제 backend 기록이 해석의 기준이다. |
| N07 | direct λ=1 대 continuation, nonlinear endpoint와 actual-K stage-functional 별도 비교 | endpoint 계약과 stage 계약을 섞지 않는다. 실제 wall time을 certified parallel span으로 바꾸지 않는다. 독립 window scheduling의 보장은 후속 범위다. |
| N08 | source-bound preflight·검증·resume·고정 campaign과 별도 typed 결과 consumer | 139 Python PASS/3 dependency SKIP receipt를 재사용한다. NCP 실측은 아직 없다. consumer는 완전한 raw/환경 결속을 요구하고 broad production을 자동 승격하지 않는다. |

N01/N02를 “미구현”으로 남겼던 DECISION의 표현은 현재 수정되어 있다. `REPORT_KO.md` 6–7절과 `NEXT_STEPS.json`은 이번 구현 범위와 앞으로의 연구 범위를 구별한다. N01의 Rust 타입 migration, N02의 native 미실행, N05의 planner 미연결은 실제 사용자가 알아야 할 한계이며 삭제해서는 안 된다.

## 실행 가능한 세 bound client의 닫힌 범위

| client | 동일 입력의 arm | 허용되는 결론 | 허용되지 않는 결론 |
|---|---|---|---|
| diagonal | Bounded / Resolvent / Diagonal | 원래 1e-7 endpoint budget, 동일 출력 oracle, fallback 0. Resolvent 추가 inverse RHS 32회에서 Diagonal 0회 | Bounded도 0회이므로 전체 최속 정책이라는 주장. endpoint 검사를 full-curve 인증으로 승격 |
| global | sealed provider / global replay | 동일 PR101 곡선·원래 1e-4 budget, private run binding, full horizon 및 저장된 query의 상계 | replay가 원래 radius를 줄인다거나 모든 RHS가 인증된다는 주장 |
| allocation | uniform [20,20] / optimized [8,24] | A=[1,9], c=[1,1], p=1, 동일 오류 상계 0.5에서 work ceiling 40→32 | ODE 32단계를 실행했다거나 wall time이 20% 빨라졌다는 주장 |

7개 native smoke 행과 typed NCP 소비 검사는 기존 raw receipt에서 확인했다. `REQUESTED_ODE_ENDPOINTS_ONLY`, `BOUND_MODEL_FULL_CURVE`, `ALGEBRAIC_ALLOCATION_ONLY`를 서로 교환하지 않는다. 실제 full-curve true는 complete sealed global case에만 허용한다. 비교 실행기는 solver transaction 시간을 기록하고 NCP가 process 시작·pool 구성 등을 포함한 바깥 시간을 별도로 기록한다. operation counter를 완전한 FLOP 수로, 구조적 round 수를 wall-clock span으로 취급하지 않는다.

구성적 성과는 수치로도 분명하다. 대각 fixture에서 세 arm의 endpoint 상계는 약 3.1333e-15로 같고 Diagonal은 15번의 검사와 15,360 matrix-entry 스캔을 지불하면서 32개의 추가 inverse RHS solve를 제거한다. allocator의 40→32는 명시된 대수 모델에서 원래 budget을 보존하는 20% 작업 상계 개선이다. global replay는 radius가 약 8.392e-12에서 9.495e-12로 더 보수적이지만, 초기오차·jump·query rounding을 포함한 horizon 전체의 권위와 run identity를 하나의 계약으로 묶는다. 더 작은 radius나 실측 가속을 주장할 필요가 없는 실제 기능 개선이다.

Cayley/Bernstein 결과도 코드에 연결되어 있으나 좌표를 구분해야 한다. N05 normalized output 전달 다항식의 bound와 이전 preconditioned residual 이론의 23.35를 임의로 바꿔 쓰면 안 된다. 동일한 norm, tableau, residual normalization, embedded-estimator 오염 비용 및 반환 중심 반올림을 유지한 `LinearStageTargetPlan::assess`가 후보별 판단 경계다.

## 브랜치와 과거 근거의 보존

전체 branch delta 검토는 `INDEPENDENT_BRANCH_REVIEW.json`을 재사용한다. 그 checkpoint에서 incoming 변경 1,133개 중 1,109개가 byte-identical, 24개가 명시적 적응, 누락은 0이었다. 전체 incoming 6,451경로와 production 6,344경로의 존재도 확인했다. 788개 파일을 수용했고, 충돌부는 namespace 분리·export union·기존 API 보존으로 처리했다. 이후 phase2 코드 변경이 있으므로 이 수치를 최종 tree의 byte-identity 주장으로 사용하지 않는다.

두 ledger 원본은 각각 142개 행을 유지한다. index는 published production f2f20c1 및 research ee65d4db의 commit-qualified 284개 identity를 사용하여 L-0136–L-0142의 7개 의미 충돌을 구분한다. 로컬에만 있던 merge SHA를 원본 이름 공간으로 쓰는 문제는 교정되어 있다. 원래 246개 component module의 포함 관계도 보존되며 phase1의 8개와 phase2의 2개를 더한 현재 registry는 256개다. 과거 254개 실행 receipt를 사후 수정하지 않는다.

발행 시에는 production f2f20c1, plan cc2e1a0, research ee65d4db가 실제 조상으로 남는지와 remote tree/최종 manifest가 같은지 확인해야 한다. 본 검토는 그 push가 이미 이루어졌다고 주장하지 않는다.

## 최종 승격과 실행 인계

현재 코드로 local의 다음 작업은 정해진 verification과 NCP 실행이다. source-clean checkout, exact locks/native dependencies, 원래 budget을 유지한다. 2 warmup·21 반복·1/2/4/8 threads, 3,221개 필수 결과 ID의 고정 campaign과 raw consumer가 준비되어 있다. missing dependency, 불완전 reference ladder, unknown RSS, timeout, budget 실패, 바뀐 binary/shared library는 HOLD로 남겨야 한다. logical affinity는 host 독점 증명이 아니고 관측 RSS는 전체 process tree의 수학적 peak 상계가 아니다.

범용 승격에는 여전히 unsupported route와 안정 API 검토, 실제 native comparator/workspace 검증, 독립 full-cost 실측, 모델별 연속오차 권위가 필요하다. 미래 연구는 G2의 A_j/c_j를 실제 accepted-step ODE 법칙에 묶는 일, nonlinear drift를 포함한 target 연결, band/low-rank resolvent, 독립 window homotopy의 전체 work/span 검증이다. 이들을 이미 완성했다고 표기하거나 저예산 실행 인계에 숨겨 둔 구현 과제로 바꾸지 않는다.

Machine-readable 판정과 검토한 소스 SHA-256은 `INTEGRATION_REVIEW.json`에 있다. 최종 matrix와 publication 상태는 해당 JSON의 `final_validation` 및 `publication`이 가리키는 권위를 따른다.

## 검토 이후의 단일 파일 lint 수정 기록

최종 Clippy가 `solve.rs`의 `single_match`를 지적하여, `Jacobian::Banded`와 빈 wildcard arm으로 구성된 match가 `if let`으로 바뀌었다. 현재 블록에서 기존 band 검증 조건·refusal 메시지·뒤따르는 capability 검증은 유지되어 있다. 이 변경의 별도 독립 검토는 `FACADE_FINAL_LINT_REVIEW`를 참조하며, 본 추가 기록 시점에는 담당 검토자의 receipt를 기다리고 있다.

`INTEGRATION_REVIEW.json`의 `reviewed_source_sha256` 및 40개 hash 일치 결과는 **2026-10-21T05:10:21.764473+00:00의 검토 스냅샷**이다. 이를 현재 소스의 일치 주장으로 사용하지 않는다. 당시 `solve.rs` hash `d05233df298e206bb096b7e763c280a9248e1ecb8da8cd0c8748b66af2d9f53d`와 수정 후 관측 hash `e1ad9ea19c0cd948b33fc67e67876b111262a3252ad807cc62af119e2e2331f8`를 모두 보존한다. 과거 proof/test receipt의 source를 새 hash로 소급 교체하지 않았다.

부모 작업은 동일 소스에 대한 일관된 최종 증거를 만들기 위해 final_04의 선택된 19-command matrix를 다시 실행하고 있다. 완료 여부와 결과는 `EXECUTION.json`이 권위이며, 이 문서는 미완료 실행을 PASS로 가정하지 않는다. 본 추가 검토에서는 코드 수정이나 native 시험을 수행하지 않았다.
