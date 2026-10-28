# Local Codex / native 실행 인계

아래 내용을 새 실행 세션의 입력으로 사용할 수 있다. 이 문서는 실행 계약이며 native 구현 완료 영수증이 아니다.

~~~text
PROJECT=VigilODE
TASK=LOOP12_PACKET_NATIVE_PORT
SOURCE_BRANCH=research/promotion-theory-20261011
SOURCE_BASE_COMMIT=2e944afc36a20fd20ce33438b2a1d4f4e0a6a5c4
PACKAGE=research/promotion_loop12_20261011
MODE=IMPLEMENT_AND_VALIDATE_SELECTED_PACKET_ROUTE

먼저 원격 source branch의 실제 HEAD/tree와 로컬 변경을 확인하라.
기존 사용자 변경을 보존하고 현재 AGENTS.md가 있다면 읽어라.
이 통합 패키지 이후 source drift가 있으면 관련 source anchor만 다시 결박하라.

읽을 순서:
1. docs/reviews/20261011_loop12_upgrade/REPORT_KO.md
2. research/promotion_loop12_20261011/DECISION.json
3. docs/reviews/20261011_loop12_upgrade/PORTING_KO.md
4. research/promotion_loop12_20261011/{PORTING_DAG,SOURCE_BINDINGS,THEOREMS}.json
5. 선택한 경로의 DAMPING_WINDOWS_KO/RANK_GEOMETRY_KO/COST_CORRECTIONS_KO 증명
6. input/IMPORT_INDEX.json 및 필요한 원본 reference

전체 원본 ZIP은 input/rvj_independent_loop12_20261011.zip에 포함되어 있으므로
추가 첨부를 요구하지 말라. ZIP SHA256:
2735bf3ea1c52abae2e97a3310714eb4a9b47e4e08e786c7c0fc32831d97835c
기존 원본 캠페인을 일괄 재실행하지 말고 새 native 계약을 판별할 fixture만 선택하라.

즉시 실행할 첫 경로는 LP01 -> LP02다.
LP01: 생성형 frequency/weight/initial/cubic 모델 descriptor와 실제 RHS를 같은
      데이터에서 생성한다. 기존 OdeProblem::new의 Jacobian/JVP 요구를 약화하거나
      dummy JVP를 넣지 말고 별도 opt-in API를 만든다.
LP02: kappa=0 single-window moment, waveform, 모든 mode 복원과 원모델 total
      certificate를 native outward arithmetic으로 구현한다.
      causal_weights.py는 표 컴파일의 exact reference이지 Rust 구현은 아니다.
      cached beta weights의 O(pD) 저장 비용과 출력마다 발생하는 pD 준비를 센다.

이후 선택된 주장에 필요한 노드만 이어간다.
- common damping: LP03, 감쇠를 지수함수로 유지하고 scalar moment 오차를 인증한다.
- finite global trajectory: LP04, 모든 초기 mode enclosure와 weighted moments를 이월한다.
- rank-r/multiple clusters: LP05, 실제 r/cluster 수/폭과 reconstruction 비용을 센다.
- 실제 nonlinear 고객 모델에 적용 주장: LP06으로 원 모델의 구조 identity를 먼저 증명한다.
- 속도 주장: LP09의 동일 정확도·동일 출력·실패/준비/인증 포함 비교를 수행한다.
- 각 완료된 주장: LP10에서 후보 작성자가 아닌 독립 구현 reviewer가 판정한다.
LP07 implicit closure와 LP08 FFT는 필요한 모델/성능 근거가 있을 때만 진행한다.
이 명시적 packet 경로를 관련 없는 모든 과거 TP 노드 완료까지 막지 말라.

반드시 보존할 경계:
- packet은 RVJ5와 다른 적분맵이며 same-tableau라고 등록하지 않는다.
- shared_shift_jet의 R0W-W는 이미 구현되어 있으므로 새 개선으로 재계상하지 않는다.
- joint_phi_action의 symmetric-negative authority를 skew/비정규 packet에 전이하지 않는다.
- 기존 scalar_phi_enclosure는 phi0..4 및 [-600,0] 범위다. 다른 범위는 새 근거가 필요하다.
- continuous Volterra factorial convergence가 finite projection 오차를 자동 제거하지 않는다.
- 초기값/모드 순서/epoch/time/norm/phase/roundoff/kernel tail을 certificate에 결박한다.
- rejected trial은 accepted state를 바꾸지 않고 실제 작업량은 반드시 누적한다.
- 이전 K-form leakage 및 production HOLD는 packet 이론의 승인으로 해제되지 않는다.

검증은 새 변경의 위험을 판별하도록 수행하라.
Native fixture에 대해 exact-reference enclosure, kappa->0, 비대칭 초기 odd moment,
tail 누락, stale certificate, wrong mass, overflow/underflow, 복수 window의 inherited
error 누락을 검사한다. 모든 Rust 검사 결과에는 command/exit/raw log/source SHA를 남긴다.
count-work, span, bit/precision 비용, elapsed time을 구분하고 이미 hoist한 baseline에도
n배 이득을 다시 주장하지 않는다. 실제 기준법이 없으면 그 비교만 미수행으로 표시한다.

반환할 파일:
RETURN_HANDOFF_KO.md, RETURN_STATUS.json, SOURCE_PROVENANCE.json,
실제 diff/commit/tree, commands와 raw logs, 테스트별 실제 결과,
ORIGINAL_MODEL_ERROR_LEDGER.json, WORK_LEDGER.json, INDEPENDENT_DECISION.json.
증명 PROMOTE와 native/global/speed 승격을 분리하고 미실행 항목을 PASS로 쓰지 말라.
~~~
