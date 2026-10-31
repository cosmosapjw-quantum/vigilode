# VigilODE ↔ REI Bianchi IGM 연구과정 적대적 메타감사

**기준일 2026-11-01 (KST); 분류: same-context adversarial, independent distinct-author review 아님.**

## Evidence and authority

- `vigilode/main` README는 generic stiff/oscillatory ODE polyalgorithm을 1차 임무로 정한다. 범용 방법 연구의 *자체 가치*는 본 감사가 취소하지 않는다.
- `vigilode` PR #162 `02f16fac...`, #165 `848bb269...`, #166 `9ddf8dd2...`: owner-source/frozen-source 물리량, 0 owner IVPs, 각자 Draft/HOLD, 원본 서로 다른 incoming fields. #165는 원본 full Volterra I/Q local collision 193,000 rows, #166은 PR130 first-gain Q 기반 27,000 complex spin-2 E/B rows. 서로 같은 full-source E/B가 아니다. GitHub: https://github.com/cosmosapjw-quantum/vigilode/pull/165 https://github.com/cosmosapjw-quantum/vigilode/pull/166
- `rei_bianchi/main` `PROJECT_STATE.json`과 `docs/science/current_00_READ_FIRST.md`는 한 4-site, 3-lane FLRW thermochemistry microstep을 인증했고 전체 첫 canonical interval, Bianchi feedback/production chemistry는 미승인으로 명시한다. Main 상태 기록 생성일은 2026-08-11. 최신 연구 PR (예: #317)은 별도 이력이며 자동으로 main의 canonical 상태를 대체하지 않는다. https://github.com/cosmosapjw-quantum/rei_bianchi
- `rei_bianchi/main/docs/roadmap/D0_PR_ROADMAP.md`는 원래 end-to-end FLRW, Bianchi geometry/tilt, Thomson, observables와 all-type verification을 PR3~10의 상위 의존성으로 둔다.
- 2026-11-01 REST 열린 PR 수: VigilODE 136 (Draft 134), rei_bianchi 302 (Draft 296). 이는 실제 open PR의 총량이지만 독립적인 과학 실험 수는 아니다. 저장소에서 열린 계층형 PR이 많이 중첩된 점에 주의한다.
- VigilODE PR165 global CI `41210052312`: `Check research nodes and the append-only ledger` FAILED; 다음 `Check authority ...` SKIPPED. PR162 global CI `41205681693` FAILED. 2026-10-27 정책 기록 `research/PRODUCTION_SOLVER_CLOSURE_POLICY_20261027.json`에는 다른 시기의 local authority scan missing 689/795 distinct objects가 기록돼 있으나, 그것이 현 PR165 CI 실패의 직접 원인이라는 증거는 없다. 두 failure를 혼동하지 않는다.
- **PR165 CI의 구체적 재현 가능 원인:** 해당 PR이 새 `research/bass21_owner_fullpolar_20261031/`를 추가했으나 파일 목록에는 `PREREGISTRATION.md`와 새 `research/LEDGER.jsonl` 행이 없다. 현재 `tools/check-research-node.py`는 새 `research/<node>/`에 대해 두 요소를 요구하므로 이 상태는 자체 계약상 불합격이다. 실제 job 로그를 확보하지 못했으므로 이것이 *유일한* CI 실패 사유인지까지는 입증되지 않았다. PR166의 `exploratory/` 비정규 구분은 별도의 소급 등록 우회가 아니라 과학 권위의 범위를 낮춘 분류이며, canonical 승인으로 해석하면 안 된다.
- 원본 solver DAG: `research/all_dag_completion_20261018/ORIGINAL_DAG.json`는 A01–A12, original scope immutable. 반복 보고된 VigilODE 7/23 및 REI 28/88/33/88은 다른 연구계층의 count다. 7/23 및 28/88은 PR166이 보고한 수치이며 본 감사가 전 노드 재계산한 값이 아니다.

## Critical findings: finding → evidence → impact → confidence → response

1. **F01 / Critical / 목표 대리화와 승인 진전 0:** PR162→165→166은 owner-frozen source 위의 진짜 수치/물리 결과를 산출했지만 새로운 coupled EoR IVP 또는 canonical REI 관측량 결과는 0이다. `7/23`, `28/88` 보고값 불변. **영향:** 같은 병목과 무관한 계산을 늘리며 중심 물리 결과를 지연. **신뢰도 High** (해당 PR 명시 상태), 실제 전체 연구 달성률 추정은 별도. **대응:** mission DAG M0→M5와 critical-path 우선순위 명시, PR lineage 숫자를 science credit으로 환산 금지.
2. **F02 / Critical / 서로 다른 authority·source 구분 실패 위험:** PR165 full Volterra I/Q vs PR166 PR130 first-gain Q는 다른 물리 근사/입력. 그 차이를 동일 Bianchi E/B의 완결 증거로 누적하면 거짓 완결. **영향:** 잘못된 물리적 추론. **신뢰도 High.** **대응:** 모델/클로저/전자속도/각도/주파수/스크린을 matched comparison 전까지 분리.
3. **F03 / Critical / 전역 과학 승격 불가:** #165 research-ledger CI 실패, authority check skipped. 각자의 scoped PASS와 구분해야 한다. **영향:** repo-wide admission/production 불가. **신뢰도 High.** **대응:** CI를 무력화하지 않고 실패 상태 보존, 원인 분석은 별도 finite-scope 과제; 이 governance 문서가 문제를 해결했다고 주장하지 않음.

## Watch findings

4. **F04 / Watch / 대량 병렬 PR과 검증 세분화:** 136/302 열린 PR, 대부분 Draft; 중첩 PR과 단계별 R/C 체크포인트가 많아 실제 scientist attention 및 dependency resolution 비용을 증가시킬 수 있다. **영향:** WIP·중복 감사·review 부담. **신뢰도 High** (현황), 시간낭비 비율은 측정되지 않아 inferred. **대응:** 임무 WIP 1 통합+1 blocker, 2 unit 정체 시 continue/pivot/park/kill. 기존 PR은 무단 폐쇄하지 않음.
5. **F05 / Watch / DAG 계층 착시:** Solver A01–A12, VigilODE 7/23, REI 28/88/33/88, D0 10–12 PR roadmap, 2026-08-11 canonical main은 서로 다른 측정공간. **영향:** 진행률의 착시와 status laundering. **신뢰도 High.** **대응:** 비정규 overlay + owner/counter/source type 명시, 공식 카운터 수정 0.
6. **F06 / Watch / 백업-검증의 목표 역전:** #165 바이너리 local/Library 재생성은 실질적 재현성 성과지만 #165/#166의 신규 Google Drive + Dropbox 쌍방 full-binary 확인은 안 되어 있다. 파일 백업 성공을 물리적 진전으로 합산하면 오류. **영향:** 배포/복원 위험 및 science attention 전도. **신뢰도 High** (#165/#166 설명과 로컬 재생성 기록), 최근 백업 제공자가 추가됐는지는 별도 확인 필요. **대응:** 최소 provenance 기록, scientific endpoint와 보존을 별도 필드로 유지.
7. **F07 / Watch / author review의 독립성 부재:** 다수 source-cond PR은 same-context review만 있고 distinct-author C9 `NOT_RUN`. **영향:** source ownership, screen gauge, physical closure 오판 위험. **신뢰도 High.** **대응:** 최종 canonical admit 시에만 independent referee 요구; 매 탐색 루프의 독립 감사 의례로 만들지 않음.

## Successful constraints to preserve

- Original source ZIP byte identity와 clean replay, 고유한 physical rows 및 synthetic controls, 실패 사례 보존, PR Draft/HOLD는 모두 실질적인 안전장치다. 폐기하지 않는다.
- 수치적 성공은 source-conditional 결과의 타당성을 지지하나 관측가능한 동적 진화나 원본 REI canonical 달성을 의미하지 않는다.
- 생산 허용오차와 원본 SHA, 독립 owner 권한, `NO_MERGE`, `NO_FORCE_PUSH`, `AUTO_NEXT_NODE=false`는 변경하지 않는다.

## Corrective action and falsification of this audit

이번 변경은 비정규 mission DAG, 작업선택 방침, 얕은 구조 검사만 포함한다. **의도적으로 새로운 6개 canonical 노드를 만들지 않는다.** 정책 적용의 진짜 성공 기준은 다음 실제 주기에서 (a) 추가 데이터/코드가 M1 또는 다른 named blocker를 줄였는지, (b) source-owned matched FLRW/Bianchi observable 출력을 만들었는지, (c) 기존 실패가 정확히 보존됐는지의 세 증거다. 이 중 하나라도 없으면 새 policy 파일의 존재만으로 개선을 주장할 수 없다.

**자기 적대적 반론:** REI의 2026-08 main baseline은 여러 최신 Draft PR이 만든 10월 과학적 축적을 과소대표한다. 따라서 이 감사는 10월 각 PR의 진짜 과학적 새 결과를 부정하지 않는다. 다만 canonical admission, coupled physical evolution 및 matched observables와 구분할 뿐이다. 대량 PR 수는 병목의 증거가 아니라 병목 *가능성*에 대한 운영 신호다. DAG overlay 자체가 새 control-plane drift가 되는 것을 막기 위해 validator를 schema/acyclicity/claim-level만으로 제한한다.

**미수행:** full `rei_bianchi` runnable source의 전체 fresh clone science/CI 실행; 모든 302 PR의 독립 콘텐츠 리뷰; 외부 distinct-author referee; binary cloud dual-backup; original global authority CI remediation. 어느 것도 PASS로 주장하지 않는다.
