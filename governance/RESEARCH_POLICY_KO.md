# REI Bianchi IGM 정렬 연구·개발 방침 (제안, 비정규 DAG 오버레이)

**기준:** 2026-11-01. 범위: `cosmosapjw-quantum/vigilode`의 REI-IGM 연계 연구. 독립적 범용 ODE 연구 자체의 정당성을 부정하거나 기존 소스·원장·DAG를 소급 변경하지 않는다. 이 파일은 `research/LEDGER.jsonl`, `research/all_dag_completion_20261018/ORIGINAL_DAG.json`, `rei_bianchi`의 `PROJECT_STATE.json`을 대체하지 않는다. 아래의 상태는 **승인된 정책이 아니라 Draft 변경안**이며 원본 연구 정책과 충돌하면 원본 승인 규칙이 우선한다.

## 임무, 책임자, 관측량

중앙 문제는 Bianchi 배경에서 동적 광자/화학/열 상태를 결합하여 동일한 원본 조건의 FLRW와 비교하는 것이다. 필요 최소 관측량은 물리적 의미가 고정된 `x_HII, x_HeII, x_HeIII, T_gas, tau`와 그 차이 및 전파 오차이다. 편광·E/B는 완전한 수송·화면계·소스 소유권이 있을 때에만 추가한다. 임의 속도 boost로 생긴 국소 U, first-gain spin-2 B는 관측된 B-mode가 아니다. `rei_bianchi`는 EoR 결합계의 canonical owner이고, `bass`는 배경/광자 입력, `rec_bianchi`는 재결합, `vigilode`는 방법론 공급자다. 별도 owner 동의 없이 물리 source로 전용하지 않는다.

## 두 연구 트랙을 분리한다

1. **Mission-critical (REI IGM):** 명명된 미해결 의존성 하나를 대상으로 원본 소유자/부호/단위/정규화/clock, 실제 입력, 반증 가능한 계산 결과, 대응되는 비교 또는 불확실성 감소를 제시한다. 한 단위가 중앙 DAG를 직접 승인받지 못해도 *어떤 특정 blocker를 얼마나 제거했는지* 기록한다. 지식/정의만 정제했다면 `methodological` 또는 `blocker_resolution`, 중앙 +0으로 분류한다.
2. **Independent VigilODE/Exploratory:** 일반 ODE 알고리즘·sourcefree/conditional 물리 이론은 계속 가능하지만 `VigilODE 7/23`, `REI 28/88`의 증가로 광고할 수 없다. `exploratory/` 위치도 과학적 권위를 자동 획득하지 않는다. 이미 존재한 failed/negative 연구는 보존한다.

## 작업 선택과 종료 규칙 (gate inflation 금지)

- 첫째 기준은 *최단 경로로 중요한 과학적 불확실성을 제거하는가*이다. 새 코드, 10개 단계 기록, 물리 행 수, 테스트 개수, PR 수는 과학적 endpoint의 대리 지표가 아니다.
- 원본 `rei_bianchi/main`이 인증한 것은 46,080-node 문제의 첫 전체 interval이 아니라 4-site, 3-lane **단일 microstep**이다. 연구용 Working DAG 숫자와 main SSOT를 동일시하지 않는다. 이 미해결 의존성을 REI 담당자에게 우선 전달하고, VigilODE에는 필요한 모듈/오류 예산이 특정된 경우에만 solver 변경을 요청한다.
- REI 미션에서 동시에 열어둘 적극적 신규 범위는 원칙적으로 **한 가지 통합 물리 실험 + 한 가지 독립 blocker 해소**로 한다. 현재의 다수 열린 Draft PR을 임의로 닫거나 수정하지 않고 기존 결과의 인덱스로 취급한다. 병렬성이 실제로 서로 다른 critical dependency를 해소한다면 예외 사유를 기록한다.
- **동일 병목을 두 bounded research unit 동안 해소하지 못하면** 후손 PR 추가 이전에 `continue / pivot / park / kill` 중 하나를 명시적으로 선택한다. 반복 감사나 백업을 새 science unit으로 계산하지 않는다. 계산이 구조적으로 2회 이상의 단위를 요구하면 누적 물리 증거를 명시해 이어간다.
- 하나의 연구 단위는 별도의 R1–R10·C1–C10 의례가 아니라 **질문 → 원본 입력 → 계산/반증 → 물리적 delta 또는 정확한 blocker → 보존/판정**으로 끝낸다. 원래 외부 R/C 요구 조건이 실제로 필요한 경우에만 수행한다. 부정적 결과와 미달 계산은 지우지 않는다.
- 소스/closure 부재가 원인이라면 수치 optimizer, PR 갯수 증가, 가짜 full field 추정으로 보완하지 않는다. 원본 획득·정규화·rate/temperature/source-moment closure 중 특정 결손을 추적한다.

## 증거와 승인 수준을 절대 혼동하지 않는다

각 결과는 `sourcefree / frozen-source-conditional / coupled-owner-physical / observable-validated` 중 하나로 표기한다. 명시적 새 Einstein–Boltzmann IVP가 없으면 `NEW_OWNER_IVPS=0`. `sourcefree`와 `frozen-source-conditional`은 canonical EoR 후속 노드 승인을 스스로 의미하지 않는다. PR165 full Volterra I/Q와 PR166 first-gain Q는 서로 **다른 incoming 모델**로 분리한다. PR157 등의 과거 E/B 결과 역시 같은 source/model/gauge가 증명되기 전까지 합산하거나 승격하지 않는다.

승인은 **원래 canonical owner가 기존 acceptance·source·물리 closure·관측량·독립 검증을 승인했을 때만** 가능하다. 새 overlay의 JSON 상태 변경이나 machine validator PASS는 인정되지 않는다. 전역 research-ledger/authority CI 실패는 계속 FAIL이며, scoped Python 재실행 성공으로 덮지 않는다. 독립 심사가 없다면 `NOT_RUN`이라고 쓴다. 기존 허용오차, physical sample, outlier, 음성 결과는 뒤집지 않는다.

## 중앙 이력의 provenance, 안전 경계

- 원본 branch/PR/commit/ZIP의 정확한 SHA와 사용한 본문을 기록한다. Git에 있는 witness 일부를 전체 바이너리 이중백업이라 부르지 않는다.
- source ownership, 전자 밀도/속도, `c`, `sigma_T`, tetrad screen, `g=c sigma_T n_e/H` 정규화, proper clock을 명시한다. 임의 beta는 control이자 counterfactual이다.
- 기존 `research/LEDGER.jsonl`과 A01–A12 DAG, 생산 솔버 dispatcher, 기존 PR 및 binary checkpoint는 편집하지 않는다. `NO_MERGE`, `NO_FORCE_PUSH`, `NO_MORE_IVPS`, `AUTO_NEXT_NODE=false`, `PRODUCTION=HOLD`를 해제하지 않는다.
- 본 변화는 감사 보고서+비정규 미션 오버레이+구조 검사만 포함하며 물리/수치 생산 코드는 변경하지 않는다.

## 최소 의사결정 기록 (별도 양식 요구 없음)

연구 단위 종료 시 5개 정보만 충분하다: (1) 중앙 의존성 식별자, (2) 원본 source/ref, (3) 실제 변경된 물리량 또는 미해결 blocker, (4) 독립/범위별 검증과 실패, (5) continue/pivot/park/kill 및 공식 중앙 Δ. 신규 대규모 gate·ledger 복제 없이 기존 PR 설명/리뷰에 기록하면 된다.

## 다음 실행 우선순위

REI owner의 **첫 canonical interval 전 구간** closure·보존·오차를 우선 닫는다. 병렬 가능한 Bianchi geometry/proper-clock source 계약은 분리하여 근거를 모은다. VigilODE의 다음 Thomson spin-2 확장은 REI에 실제로 필요한 관측량·closure 병목을 지정할 수 없으면 `PARK`한다. 재개하는 경우에도 PR165 full-Volterra I/Q와 PR166 first-gain Q의 동일 모델·동일 샘플 비교가 선행돼야 하며, 이것만으로 중앙 DAG를 늘리지 않는다.
