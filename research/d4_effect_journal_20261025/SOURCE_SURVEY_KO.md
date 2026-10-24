# 최신 native source·예약·완료 근거 조사

조사일: 2026-10-25 KST (원 API observation UTC 2026-10-24T18:xx).
분담: physmath-research-loop 1.0.0 R1–R2/C1–C2. 원자료·실제 source의 읽기 전용 조사다.

## 확인한 현재 identity

| 대상 | 현재 readback | 상태 |
|---|---|---|
| D3 PR86 | 02060cc9dc4551dc5cf6ec86ad9b53b697a2c3ce | 기존 완료 pin 유지, Draft |
| D2 PR85 | 1c829f206ab67585191231be7f5135fd0d4c804e | 기존 완료 pin 유지, Draft |
| native research/local-frontier-20261024 | aaf1f834d4c589e734ca7851a56c11bb44f7047a | 기존 공개 pin 유지 |
| candidate-reconsideration | 94db44d1e7487a3b5e4f8b97fea9504b2c2e396c | 기존 공개 pin 유지 |
| general-ode-selection | 14171ed0c4f7dc453f47870cf6c1399f41afc4f2 | 기존 공개 pin 유지 |
| 중앙 REI PR222 | 407381b302d8b98f251b54d277823997bc4fca9f | 다른 연구에서 새 update, VigilODE owner receipt는 추가되지 않음 |

VigilODE branch metadata 전체는 205개, 페이지별 100+100+5+0.
전체 PR metadata 86개, 페이지별 86+0, open56개.
중앙 PR222 issue comments는 8개+다음page0, 마지막 VigilODE 전달은6384170816이다.
목록은 접근 가능한 GitHub 공개 API 범위의 전수다. 모든 branch의 모든 source나 사용자의 모든 대화를 열람했다는 뜻은 아니다.

root 및 research AGENTS는 D3 pin에서404. frontier root/research/crates/crates/rodas5p-integrators의 AGENTS도404. frontier crates 재귀 tree795entries(truncated=false)에 AGENTS 없음. 이 기록은 해당 원격 pin의 관측이며 live native host 파일의 부재 판정이 아니다.

## native 담당 연결 결과

공개 source와 기존 source-bound 결과는 확인했다. 다음은 여전히 UNRESOLVED_INPUT_DEPENDENCY다.

- 현 native 담당자의 실행 중 예약/lease와 종료 여부.
- 그 담당자의 현재 worktree HEAD, dirty tracked/untracked 목록 및 source hash.
- 현재 변경에 대응하는 native RODAS full33 hook/FP provider/actual BASS 완료 receipt.
- 공개 branch 이후의 아직 게시되지 않은 실제 수정.

Desktop Commander 호출0. 월 quota 제한을 재시도하거나 다른 경로로 우회하지 않았다.
기존 공개 source나 역사적 dirty-development receipt를 현재 live owner의 종료 증거로 치환하지 않는다.

## 이미 존재하는 hook: 재사용할 범위

| 코드 | 실제 연결 | 판정 |
|---|---|---|
| transaction.rs | Transaction<S:Clone>::begin은 committed.clone(); Trial::commit/rollback/Drop, work ledger, pre-reserve Budget | 기존 owned-state transaction·work accounting 재사용 |
| generic_verified.rs | immutable model capability, max_attempts, min_step, valid represented clock, all pending dense outputs prepared before accepted state/radius/cursor/history commit | 유한 attempt와 scientific-prefix 보존 재발명 불필요 |
| integrate.rs | max_attempts loop, minimum step/clock stop; retry 시 Krylov+history snapshots 복구; attempted work 유지 | 해당 state 범위의 rollback 재사용 |
| output.rs | prepare_dense_interval_with_radii, commit_prepared_dense_interval | internal output prefix에 대한 prepare/commit 존재; commit에서 allocation/callback 없음 |
| problem.rs | RHS/Jacobian/JVP/partial_t는 Arc<dyn Fn ...>; callback 전 attempt counter 증가; model epoch 계약 | 입력/attempt accounting 재사용. Clone만으로 callback capture deep-isolation 보장 안 됨 |
| affine_error.rs + quotient_flow.rs | AffineErrorNorm 및 integrate_*lawson_dopri5_observed_with_norm | 기존 affine API 존재. native RODAS 전체/현재 owner 완료와 동등하지 않음 |
| integrators lib.rs | 첫 줄 #![forbid(unsafe_code)] | 유지 |
| core directed.rs | TwoSum/FMA/next_up/down outward arithmetic | D3 기존 blob과 동일; FP 환경 검증 대체물 아님 |

확인한 source와 문서23개를 원격 Git blob identity와 실제 저장 byte로 대조해23/23일치했다.
SOURCE_PINS.json(19개)와 SOURCE_PINS_EXTRA.json(4개)에 SHA256/byte/Git blob을 보존한다.
로컬 수신 시 apply_patch가 붙인 추가 final newline은 원 Git blob에 맞게 교정했으며 원격 source를 변경하지 않았다.

주요 exact source:

- transaction.rs blob1cfb7ef5dba52edafc2221dc4a074e3379f2f9fe; SHA256 eb4ade404b3e34d1a4c9e781eaa727b1b77736a22a5ec83b78518ec2b1827adf.
- problem.rs blob8f5926440d99204fc271553b3c0907c3880747f1; SHA256 890aa2218931a12ec7fcea7c72a0fb25eb588e7984a04ad2c5d89b4d5e95e066.
- generic_verified.rs blob4d8e906f687dae338f3cec031e59e458d1b417b9; SHA256 b8944a94d0909e4746f080abebc8aa33d43c850d704c4c13c2f4ed0f9fcffb1a.
- output.rs blob4d75334eabcc4dc89f62cdbec0e4d58afcb1fab0; SHA256 81bdf698f36ae0b450af77e36631e27da319fb079d648658c794ad948978a10c.
- directed.rs blob89e2b3122b08db1849e4e56cc94f1f58c473b735; SHA2562adadb6a5d77073e0a7d3d60aedfbf37e124d15a586a880d7d5a8c8d5307a814.

## 실제 남은 차이와 다음 연구 선택

1. **owned callback effect journal / single pending delivery와 명시적 acknowledgement 계약**이 유용한 다음 연구 단위다.
   기존 Transaction과 Session의 accepted-prefix/attempt bound를 재사용한다.
   callback이 deep-owned trial context와 bounded journal에만 효과를 기록하고,
   scientific accept 후 journal을 pending delivery로 넘긴다.
   실패 또는 누락 ack는 다음 attempt를 막는 명시적 상태로 다룬다.
   arbitrary Arc/Mutex 캡처, 파일/네트워크/시스템 외부효과는 이 owned-journal 계약과 별도다.
2. **native integration**은 실제 owner source/예약/dirty/완료 receipt가 공급된 후 hook diff만 이식한다.
   지금 source absence를 근거로 owner의 구현을 복제할 수 없다.
3. **일반 Jacobian·domain·dense/BASS/FP provider**는 각각 source-bound gate다.
   첫 후보의 state/effect protocol 검증을 물리적 IVP 또는 whole-solver 생산 승인으로 승격하지 않는다.

유한 종료의 정당한 재사용 범위는 callback/내부 solve가 반환하고 각 attempt의 내부 연산이 유한한 조건 아래,
existing max_attempts가 driver attempt 횟수를 제한한다는 명제다.
이미 존재하는 cap을 새 알고리즘 성과로 재포장하지 않는다.

## 실행·보존

새 solver/test/benchmark/NCP 실행0. 과거 D2/D3/V3 harmonic/benchmark 재실행0.
생산 source/branch/dirty tree·closed budget·archive 변동0.
원자료는 INVENTORY.json, FRONTIER_CRATES_TREE.json, RESEARCH_TREES.json,
CENTRAL_COMMENTS.json, AGENTS_READBACK.json, SOURCE_PINS*.json 및 source/에 보존한다.
큰 최상위 recursive tree 요청1회는 Transport closed로 실패했고, 관측 가능한 crates/relevant research subtree로 축소해 완료했다.
이 환경 실패를 source/과학 실패로 해석하지 않았다.

