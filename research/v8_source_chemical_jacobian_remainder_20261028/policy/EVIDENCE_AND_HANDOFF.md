# 근거·상태·인계 계약

현재 프로젝트의 유효한 기록을 우선 재사용하라. 이 문서의 필드를 기존 기록에 대응시키고 필요한 차이만 추가하라. 새 JSON이 유용할 때 `scripts/run_record.py`로 생성·검사하라.

## 서로 다른 상태

| 축 | 값과 사용 |
|---|---|
| 근거 상태 | established / literature-supported / derived / numerically checked / implementation-verified / conjectural / unresolved / blocked. 한 주장에 여러 근거를 연결할 수 있다. |
| 출처 관계 | supports / contradicts / limits / contextual / missing. 원전과 주장 사이의 관계로 사용하라. |
| 검사 결과 | pass_scoped / partial / fail / not_run / not_applicable / blocked. 검사 대상·범위·근거를 붙여라. |
| 실제 실행 | not_run / running / completed / failed / blocked. 명령·환경·종료 상태·출력을 연결하라. |
| 단계 상태 | pending / in_progress / complete / reused / blocked / not_applicable. 근거·잔여 의무와 함께 관리하라. |
| 사용 결정 | 채택·후속 작업·보류·교정 등 현재 범위의 결정과 이유를 기록하라. |
| 저장 근거 | provider acknowledgement, commit/tree/object identity, local bytes, restore 근거를 실제 확인한 범위로 기록하라. |

예를 들어 해석적 유도가 완결되고 수치 시험이 남으면 주장에는 derived, 해당 실행에는 not_run과 정확한 다음 명령을 기록하라. 단계 complete는 해당 단계의 요청 의무 완료를 뜻한다. 근거의 타당성·실행·저장·전체 요청의 완료를 각각 확인하라.

실제 실패는 이론 / 수치 / 구현 / 환경 / 권한·정책 / 입력 의존성으로 분류하라. 값·로그·반례·관련 버전을 보존하고 교정의 영향을 추적하라.

## 간결한 기록

한 연구 단위에는 목표·scope·완료 기준, 전제·입력·출처·현재 code identity, 실제 결과, 근거, 남은 의무, 다음 행동을 남겨라. 이미 있는 문서나 로그를 참조하라. 승인 범위의 후속 단위를 이어갈 때 현재 체크포인트를 보존하라.

기계 기록의 핵심 계약은 다음과 같다. 빈 배열은 아직 입력할 내용이 없음을 나타낸다. 새 기록은 pending으로 시작한다.

- 최상위: `schema_version: 1`, `workflow_version: "1.0.0"`, `task: research|coding|both`, `unit_id`, `objective`, `scope`, `status`.
- `dependencies`: `id, identity, description`. 입력·가정·식·code·환경 등 실제 의존 대상과 버전/내용 식별자를 기록하라.
- `evidence`: `id, kind, locator, scope, dependency_ids`. kind는 WORKFLOW.json의 목록을 사용하고 locator에는 실제 문서·식·로그·commit·provider 식별자를 넣어라.
- `stages`: 선택한 track의 정확한 10개 `id`와 `status, evidence_refs, dependency_ids, reason, reuse`.
- `claims`: `id, statement, scope, basis, evidence_refs, obligations`; basis는 위 근거 상태의 배열이다.
- `checks`: `id, domain, result, evidence_refs, reason, execution_ref`. domain은 math·physics·numerics·implementation·packaging·storage 중 하나다.
- `executions`: `id, status, command, environment, evidence_refs, exit_code, reason`.
- `decisions`, `overrides`, `failures`, `next_actions`: 실제로 발생한 내용을 기록하라. 실패에는 `class, evidence_refs, next_action`을 붙여라.

단계 complete/reused에는 실제 evidence_refs를 요구하라. reused의 dependency_ids에는 연결한 근거의 의존성을 포함하라. `reuse.dependency_identities`로 해당 단계의 dependency_ids 전체를 저장하고 현재 identity와 비교하라. 변경된 snapshot은 재검토 대상으로 처리하라. not_applicable·blocked에는 reason을 적어라.

검사 pass_scoped/partial/fail에는 근거를 연결하라. numerics·implementation 실행 검사에는 완료되거나 실패한 실제 execution_ref를 연결하라. 실행 completed에는 종료 코드 0과 출력 근거, failed에는 실패 이유·실행 근거를 기록하라. 결과 정확도는 검사 결과에서 판단하라.

새 no-go 후보는 claim에 `strong_no_go: {status: "candidate", criteria: {...}}`를 붙일 수 있다. criteria는 NG1–NG7 각각의 `status: met|open`과 `evidence_refs`다. `admitted`에는 일곱 항목 모두의 실제 근거를 갖추고 공통 정책의 의미론적 검토를 마쳐라.

작업 status complete에는 objective·scope와 모든 단계의 의무 처리(complete/reused/not_applicable)를 기록하라. 전체 사용자 요청의 후속 단위는 별도로 이어가라. JSON checker는 자료형·참조·근거 연결·snapshot을 검사한다. 증명과 문헌 신규성은 실제 내용을 읽고 평가하라.

## 재개와 저장

중단 뒤에는 재개에 필요한 실제 파일·크기·mtime·해시·git/state·관련 실행 프로세스를 확인하라. 확인된 원본과 완료 근거부터 이어가고, 사라진 산출물에 의존하는 작업만 재생성하라. 대화의 완료 문구는 해당 산출물을 찾는 단서로 사용하라.

정상 저장에는 provider 응답과 해당 commit/tree/object identity·크기/checksum·provenance가 충분하면 그 근거를 사용하라. 실제 다운로드·복원을 수행했을 때 그 복원 근거를 별도로 기록하라. 저장 확인 때문에 동일 자료를 즉시 다시 내려받고 모든 과학 계산을 반복하지 말라.

## 외부 실행 인계

현재 환경에서 수행 가능한 승인된 유도·코드·작은 검증을 먼저 완성하라. 실행자 선택은 실제 계산량·환경과 프로젝트 지시로 판단하라. 미완성 구현을 외부 실행 요청으로 넘기기 전에 현재 scope의 코드·설정·명령·검증 준비를 완료하라.

인계에는 다음 내용을 연결하라.

1. 목표·campaign/단위, source commit·관련 patch와 input·가정·closure·IC·단위 identity.
2. 의존성·build/compiler·실행 환경, 정확한 명령과 필요한 입력.
3. 원래 수용 기준, CPU·wall·memory·누적 비용과 남은 예산(관련 시), 중단·재개 조건.
4. 체크포인트·출력·로그·종료 코드·실패·자원 측정의 반환 위치와 형식.
5. 결과를 받아 확인할 identity·근거와 곧바로 이어갈 다음 작업.

제출·대기·실행·과학적 수용의 상태를 구별하라. 실행 비용과 자원 사양은 현재 확인한 근거를 사용하라.

## 구버전 기록 이관

하네스 버전, state schema, 이 스킬의 내장 규약 버전을 각각 보존하라. 기존 파일은 원본으로 유지하고 새 절차와의 대응을 기록하라.

| 기존 이론 phase의 실제 내용 | 내장 단계 |
|---|---|
| 1 계약 / 2 근거 / 3 출처·주장 / 4 가설 | R1 / R2 / R3 / R4 |
| 5 adversarial review | R6 |
| 6 물리·수학 validation의 유도 / 검토 | R5 / R6 |
| 7 검증 설계와 실행 | R7에 설계·실행 근거를 각각 연결 |
| 8 scoped decision | R8 |
| 9 formalization | R9, 유용한 잠정 정의·유도는 R4/R5에서도 재사용 |
| 10 closeout | R10 |

기존 코딩 phase 0은 공통 진입, phase 1–10은 C1–C10에 대응한다. 기존 PROMOTE는 당시 범위의 사용 결정으로 읽어라. NOT_TESTED·NOT_EVALUATED·검사표의 NOT_RUN은 실제 문맥으로 해소하고 실행의 NOT_RUN은 별도로 유지하라. NA에는 적용 제외 이유를 확인하라. 템플릿과 빈 상태표는 초기 양식으로 취급하라.
