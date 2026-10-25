# VigilODE 현재 native 연결과 다음 실제 누락 조사

관측일: 2026-10-25 UTC. 과업: 새 full theory/coding loop의 R1–R2/C1–C2 읽기 전용 source intake. 이 문서는 새로운 solver 결과가 아니다.

## 1. 첫 행동의 결과: 공개 source는 고정됐고, live 담당 연결은 미해결이다

| 연결 항목 | 실제 관측 | 판정 |
|---|---|---|
| D4 branch / PR87 | `research/d4-effect-journal-20261025`, HEAD `3c8877ea0409495752dcfab42426e679e3ba92d3` | 공개 완료 snapshot 그대로 |
| Native frontier | `research/local-frontier-20261024`, HEAD `aaf1f834d4c589e734ca7851a56c11bb44f7047a`, tree `03b12c682c688155e0f8b08c52508e04738acb93` | 공개 source unchanged |
| 현재 native 담당자와 작업 예약 | 최신 중앙 PR222 전체 issue comments 및 현재 관련 PR body에 새로운 owner reservation receipt 없음 | UNRESOLVED_INPUT_DEPENDENCY |
| 담당자의 현재 worktree HEAD / tracked·untracked dirty | live host를 관측하지 않음 | UNOBSERVED |
| 현재 담당 source와 실제 완료 로그의 결속 | 공개 frontier 결과는 존재하나 현재 live 담당을 연결하는 receipt 없음 | NOT_ESTABLISHED |
| Public commit author | frontier commit author가 게시돼 있음 | live 담당/예약/종료의 대체 근거로 쓰지 않음 |

중앙 [PR222 최신 댓글6390359885](https://github.com/cosmosapjw-quantum/rei_bianchi/pull/222#issuecomment-6390359885)는 D4 완료 전달이며, 그 댓글도 native owner/source/reservation/live dirty/completion을 미해결로 둔다. 이번 조회에서는 그 뒤 새 댓글이 없다. PR87의 issue comments는 0개다. 과거 D2/D3 반환 댓글을 다시 게시하지 않았다.

Native [SOURCE_IDENTITY.json](https://github.com/cosmosapjw-quantum/vigilode/blob/aaf1f834d4c589e734ca7851a56c11bb44f7047a/research/local_frontier_20261024/SOURCE_IDENTITY.json)의 `preserve_prior_untracked=true`는 과거 미추적 작업을 보존하라는 기록이다. 현재 dirty 여부가 clean이라는 증거가 아니다. [DEVELOPMENT_STATUS.json](https://github.com/cosmosapjw-quantum/vigilode/blob/aaf1f834d4c589e734ca7851a56c11bb44f7047a/research/local_frontier_20261024/DEVELOPMENT_STATUS.json)은 broad production HOLD, N7 full local campaign 미실행, C-LOCAL user_host_run=false를 유지한다.

Desktop Commander는 호출하지 않았다. 월간 quota 거절을 재시도·우회하지 않았고, 이 입력 미해결을 알고리즘 실패나 현재 담당자의 미구현 증명으로 바꾸지 않았다.

## 2. 실제 조사 범위와 AGENTS

VigilODE branch metadata는 100+100+6+0으로 pagination을 끝내 **206개**를 확인했다. 모든 상태 PR 목록은 87+0으로 끝나 **87개**이며 최신 번호는 PR87이다. 이 목록의 head/ref/title/body/update를 읽었다. 모든 branch의 모든 source나 모든 대화를 열람했다는 주장은 하지 않는다.

| 실제 범위 | 관측량 | 읽기 수준 |
|---|---:|---|
| 전체 branch metadata | 206 | 이름, HEAD, 보호 여부 |
| 전체 상태 PR metadata/body | 87 | source frontier 및 소유권 관련 텍스트 대조 |
| 중앙 PR222 issue comments | 9 + 다음 page 0 | 모든 반환 body와 날짜 읽기 |
| D4 PR87 issue comments | 0 + 다음 page 0 | 공개 새 논의 없음 |
| frontier crates recursive tree | 795 entries | truncated=false |
| frontier docs recursive tree | 549 entries | truncated=false |
| frontier local_frontier 연구 subtree | 609 entries | truncated=false |
| D4 source subtree | 11 entries | truncated=false |
| 실제 source·상태·handoff byte 수신 | 22 files | 주요 구현 구간 읽기 및 Git blob 22/22 일치 |

D4 pinned root/research/해당 연구 directory의 `AGENTS.md` 직접 조회는404다. Native pinned root/research/local_frontier의 직접 조회도404다. Native crates recursive tree에는 AGENTS가 없다. 이것은 해당 원격 pin에 적용할 파일의 관측이며, 사용자의 live host에 AGENTS가 없다는 판정이 아니다. 중앙 REI의 AGENTS를 VigilODE 편집 정책으로 자동 이식하지 않았다.

모든 byte 수신 파일의 원격 Git blob과 로컬 복사본을 비교해 22/22 일치했다. `SOURCE_PINS.json`에 repository/ref/path/git blob/SHA256/byte count를 기록했다. `source/`는 실제 수신 byte를 보관하고, `private/`는 원시 API 응답·댓글·payload를 보관한다. 공개할 요약은 이 문서와 pin/scope 기록으로 한정할 수 있다. 관련 repo·개인 대화·원격 백업의 더 넓은 조사는 다른 intake 과업의 결과와 합쳐야 한다.

## 3. 기존 hook을 재사용할 정확한 위치

Native 다음 네 blob은 이전 D4 source 조사 pin과 동일하다. 새 기능처럼 재계상하지 않는다.

| 파일과 Git blob | 실제 이미 있는 계약 | 재사용 경계 |
|---|---|---|
| `crates/rodas5p-integrators/src/transaction.rs` / `1cfb7ef5dba52edafc2221dc4a074e3379f2f9fe` | `Transaction<S: Clone>::begin`이 trial을 clone; `Trial::commit/rollback/Drop`; charge된 work가 rollback 후 남고 Budget은 실행 전 CAS reserve | owned 수치 상태와 work 원장. 공유 Arc capture의 deep isolation이나 외부 I/O 복구를 보장하지 않음 |
| `crates/rodas5p-integrators/src/problem.rs` / `8f5926440d99204fc271553b3c0907c3880747f1` | RHS/Jacobian/JVP/partial_t의 Arc callback, shape 검사, callback 전 attempted work charge, client model_epoch | pointer/epoch는 client 약속이다. 내부 가변 모델이 실제 불변임을 증명하지 않음 |
| `crates/rodas5p-integrators/src/generic_verified.rs` / `4d8e906f687dae338f3cec031e59e458d1b417b9` | private immutable `VerifiedModelProvider`, max_attempts/min_step/represented clock; 모든 dense 출력 검증 뒤 state/radius/cursor/history commit | 기존 whole-interval capability 경로를 재사용. 공개 임의 callback이 인증 권한을 발급할 수 없음 |
| `crates/rodas5p-integrators/src/output.rs` / `4d75334eabcc4dc89f62cdbec0e4d58afcb1fab0` | `prepare_dense_interval_with_radii`와 `commit_prepared_dense_interval`; preparation 실패가 committed prefix를 보존; 최종 commit에 allocation/callback 없음 | output commit을 새로 만들 필요 없음. 유한 중심·비음수 유한 radius 검사는 임의 물리 domain 전체 인증과 구별 |
| D4 `source/reused/admission.rs` / `0b1d4fd7be28e2d2f8b61a03ef6f5faf7dcebd0d` | Session cap/clock/lease, finish/abort, state epoch와 sticky fault | 완료 D2/D3 Session을 그대로 재사용. snapshot만으로 완전 복구는 불가능 |
| D4 `source/driver.rs` / `563e2692eea01da25943214bad06cb7eec7d3f46` | bounded owned journal/poison, accepted pending batch, delivery 후 같은 identity/payload 유지, exact ACK barrier | D4 완료 기능. 현재 ACK는 caller assertion이며 durable external receipt 자체가 아님 |
| `affine_error.rs` / `95efe19477aba8efe378a1f5e083c09b2cdffb69`, `quotient_flow.rs` / `fd7321b90adc4957fb9c8dcbe0f7f58a72680f11` | 기존 AffineErrorNorm 및 Lawson/DOPRI5 observed 경로 | 이미 존재하는 affine API. 실제 BASS/native RODAS 전체 완료로 승격하지 않음 |

[Native source](https://github.com/cosmosapjw-quantum/vigilode/tree/aaf1f834d4c589e734ca7851a56c11bb44f7047a/crates/rodas5p-integrators/src)와 [D4 source](https://github.com/cosmosapjw-quantum/vigilode/tree/3c8877ea0409495752dcfab42426e679e3ba92d3/research/d4_effect_journal_20261025/source)에서 위 hook의 실제 byte를 읽었다.

### Callback/Jacobian의 구체적 현재 경계

`problem.rs`의 일반 RHS 경로는 shape를 확인하고 user callback을 호출한 뒤 출력 finite를 검사한다. `dense_jacobian_into`는 shape 확인과 build charge 후 callback이 제공된 matrix를 직접 채우게 한다. 그 API의 계약은 이전 matrix 혹은 zeros와 고정 sparsity/명시 Jacobian 일치를 요구한다. Callback이 오류 전에 일부 칸을 썼을 때 caller buffer 자체를 원자적으로 복구하는 공통 wrapper는 이 함수에 없다. 이 사실만으로 모든 driver가 잘못된 상태를 accept한다고 결론내리지 않는다. 후속 driver의 폐기·재준비 규칙은 별도다.

`with_model_epoch`는 데이터가 바뀔 때 client가 epoch를 바꾸라는 약속이다. Arc pointer equality나 epoch 숫자만으로 함수 의미가 불변임을 입증하지 않는다. `request_continuous_residual_authority`는 일반 point callback에 `UnsupportedUnboundModel`을 반환한다. 따라서 일반 callback admission을 연구하려면 input domain, callback partial write, Jacobian consistency와 trusted whole-domain provider를 실제로 구분해야 한다.

### Unsafe / MXCSR 경계

Native integrators `lib.rs`의 첫 줄은 `#![forbid(unsafe_code)]`이며 core `lib.rs`도 같다. D4 새 `driver.rs`와 `stage_safe.rs`도 forbid를 선언한다. 상속 `admission::environment`의 read-only MXCSR assembly는 별도 `d3_fixture` dependency에 존재한다. `fixture.rs`는 그 경계를 명시한다. 이 turn에는 그 compile 검사를 다시 실행하지 않았다. 완료 D4의 safe-wrapper 검사와 direct-fixture 예상 실패를 재사용하며, 전체 dependency가 unsafe-free라는 주장을 하지 않는다. 생산 forbid를 완화하거나 MXCSR block을 생산 crate 안으로 이식하지 않았다.

## 4. 다음 연구에 적합한 실제 누락 후보

아래는 새 구현 승인이 아니라 root가 R3 이후 하나의 bounded 계약을 고를 수 있도록 한 후보 목록이다. 새로운 norm-only 노드를 제안하지 않는다.

| 후보 | 기존 hook과의 차이 | 새로 닫을 수 있는 의무 | 계속 별도로 남는 gate |
|---|---|---|---|
| 1. Payload-bound durable receiver receipt 또는 consistent restart cut | D4 live pending/exact ACK는 완료됐지만 durability/crash restore는 명시적으로 범위 밖 | 동일 batch ID·동일 payload의 중복 억제, 같은 ID·다른 payload 거부, 원자적 receiver commit 및 receipt binding; restart를 포함한다면 전체 authority-state 복구 | 세션 crash 복구를 구현하지 않은 sink-only 결과는 solver restart나 global exactly-once로 승격하지 않음 |
| 2. 일반 callback/Jacobian의 staging·domain 계약 | 현재 shape/counter hooks는 있고 generic typed domain/partial-write isolation은 보편적으로 제공되지 않음 | callback 진입 전 허용 domain, private output/Jacobian staging, 실패 시 accepted prefix와 후속 cache 의미, 잘못된 모델/epoch 거부의 좁은 consumer 계약 | 임의 공유 capture/I/O의 자동 rollback, nonreturning callback, 실제 BASS 및 현재 native 담당 integration |
| 3. Whole-domain dense provider binding | immutable model capability와 prepare/commit은 이미 있으나 일반 point callback에는 authority를 거부 | 새로 지원할 특정 모델의 유효 domain 전체에서 residual/Jacobian·tube bound를 model identity와 연결하고 실제 저장 curve를 검증 | 일반 point samples만으로 전체 domain 증명, 모든 모델 universal dense safety |

현재 native owner 입력이 없으므로 기존 native branch에 hook이 전혀 없다고 가정해 복제하거나 그 담당 작업을 이어받지 않는다. 위 후보는 이미 읽은 연구 consumer의 미해결 범위에서만 고를 수 있다.

### Restart를 선택할 때 보존해야 할 전체 상태

현재 Session의 `Accepted { t, y, epoch, last_norm }` snapshot은 충분한 checkpoint가 아니다. Session은 추가로 `tf, h, attempts, accepted, rejected, cap, pending lease, halt, domain policy, domain_rejected`를 가진다. D4 Driver는 `run_id, metric, journal limits, pending batch, last_ack, terminal, counters`를 추가한다.

Accepted state로 `Session::new`를 다시 호출하는 방식은 attempt 수·cap·다음 h·ACK 상태를 잃을 수 있다. Consistent restart 연구는 source/model/config identity, 중간 producer lease를 포함하는지 여부, 수치 accept와 outbox 사이 crash cut, 과거 attempt debit 및 pending/last_ack를 명시적으로 보존해야 한다. 현재 관측한 native crates/docs/frontier tree의 checkpoint/restore/durable/outbox/journal 파일명 검색에서는 구현 경로가 발견되지 않았다. 이는 모든 source의 내용 검색에 의한 부재 증명이 아니다. D4 실제 public API에는 complete export/import가 없다.

## 5. 완료 근거와 예산의 재사용

D2, D3, D4 완료 evidence는 새 실행으로 표시하지 않았다. D2의 507 forbidden RHS 대 guard 두 경로0, guard+반감1,032 attempts/5,323 RHS 대 guard+rate cap602/4,816은 고정 제조 모델 작업량이다. Scalar-linear [0,2] certificate와 비선형 rate cap heuristic을 구분한다. Native source intake는 이를 다시 계산하지 않았다.

기존 closed budget은 그대로다: D4 25.906088458010345 s, D3 35.897308696992695 s, D2 25.104395919000012 s, prior known 83.29566578299983 s, 별도 native boundary 14.951735806986107 s. older remaining=null, reset=false. 이 파일 작성/조회에 새 연구 budget을 선언하거나 기존 원장을 초기화하지 않았다.

이번 과업의 새 solver/test/benchmark/NCP/V3 harmonic 실행, 원격 mutation, 외부 메시지, Desktop Commander 호출은 모두0이다. 22개의 원본 Git blob identity 대조는 byte provenance 검사이며 과학 재실행이 아니다.

현재 전체 상태는 production=HOLD, global completion=null이다. D2 historical independent personnel/native_subagent=NOT_RUN은 덮어쓰지 않았다. 이 현재 분담 intake 과업 수행은 live native owner 입력의 해결이나 독립 인력 검증을 뜻하지 않는다.

## 6. skill_context_used와 인계

적용 package: `e1/skill-6aa0657100b08191b7d67482d1f97c1d`, physmath-research-loop 1.0.0. 이번 turn에서 RESEARCH_POLICY.md, THEORY_LOOP.md, CODING_LOOP.md, EVIDENCE_AND_HANDOFF.md를 읽었다. R1 실제 source/입력 경계, R2 provenance 및 재사용, C1 baseline/소유권 연결, C2 equation-data-code mapping을 적용했다. 완료 계산을 반복하는 audit loop를 만들지 않았으며 scope 밖 변경을 하지 않았다.

Root가 이어 쓸 실제 파일:

- `NATIVE_SURVEY_KO.md`: 공개 가능한 요약.
- `SOURCE_PINS.json`: 22개의 실제 byte/원격 blob 대응.
- `INTAKE_SCOPE.json`: pagination/조회범위/실행0/skill context.
- `AGENTS_READBACK.json`: 적용 경로 조회 결과.
- `source/frontier/`, `source/d4/`: 재사용할 정확한 source.
- `private/`: raw comments/metadata/payload. 별도 검토 없이 공개 묶음에 넣지 않는다.

이 intake는 다음 node의 과학 완료를 선언하지 않는다. Root의 다른 repo·대화·backup intake와 합친 뒤 실제 누락 하나의 계약 및 성공 기준을 정해야 한다.

