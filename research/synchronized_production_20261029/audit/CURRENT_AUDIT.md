# PR98 이후 상태와 병목 감사

검사일은 2026-10-29 KST다. 최신 공개 작업은 PR101 `bc5ddbe42b24b1a6364bf653c3e2d6d938ca4087`이며 main은 `8d0c79184e09efb5bdadc24a6315c60a71a44264`다. main은 PR101의 조상이고 PR101은 930 commits ahead / 0 behind다. PR98에서 PR101까지는 18 commits ahead / 0 behind다. 이번 감사는 GitHub의 실제 PR·비교·Actions 결과와 고정 source 파일을 읽었다. 사용자 로컬 디렉터리, 비공개 대화 전체, 모든 과거 파일의 전수 열람을 주장하지 않는다.

## 새로 확인된 결과

PR101은 dense/banded FD scratch와 비자율 time-FD scratch 재사용을 구현했다. 실제 실행 source는 `a682f609a1f2db1da714bea6da671b362f56baf4`, 비교 baseline은 `80bd038a13a3623e6220b9725d5c1ad4f7611fcf`다. 최종 delivery 문서 commit과 실행 commit을 구분한다. 18개 반환 checkpoint/Stats/callback pair는 정확히 일치했고, whole-solve allocation은 dense 337→324, banded 334→318, time-JVP 401→386, autonomous control 389→389였다. 전체 내부 accepted states나 dense interior를 관측한 결과는 아니다.

PR101의 F6 manufactured trace whole-path certificate는 source `299310290c2e254149ee30f4a9834c1123066815`에서 독립 Fraction oracle의 992 slabs를 통과했다. 정확도 cap은 256 subdivisions에서 처음 만족했다. 8개 등록 suite의 실행 source는 `a1a459f2be186d9f67ff86bf38c19ae24b769768`이며 release 결과 2055 passed / 76 ignored는 해당 source와 범위의 결과다. 이를 latest FD source의 전체 재검증으로 바꾸어 부르지 않는다.

PR100 `8615b365c2812a126574f579328b2056ce5b03c1`은 PR98에 별도로 쌓인 scalar affine physical-event Fraction oracle다. 원래 방정식 defect, Bernstein hull, 두 rational envelope, event transversality를 이용한다. Rust dispatcher 통합과 실제 nonlinear client certificate는 남아 있다. PR99 `e46704cbc5a1356bd8fd199de09ad26b046acfd4`는 PR95를 base로 둔 별도 chemical rate-box 연구다. uniform Gamma*rho²와 별도 deltaJ*rho를 재사용할 수 있으나 actual rate/metric/corridor/owner capsule 입력이 필요하다. 두 PR의 과학적 입력은 명시적 source binding으로 활용하며, 현재 PR101 ancestry에 이미 들어있다고 간주하지 않는다.

## 원인별 분리

| 관측 | 원인 분류 | 재사용할 아이디어와 다음 근거 |
|---|---|---|
| time-JVP r12 process Ir 중 MGS24.22%, Arnoldi7.36%, QR5.60% | 알고리즘·수치 계산 비용 | PR93 fused backward Clenshaw, PR94 target-specific polynomial compiler, 기존 signed-adjoint certificate, warm coefficient cache. 동일 target·provider·rounding 계약을 먼저 증명해야 한다. |
| ensure_pool396 allocator calls /12 solves=33/solve; dense workspace192/12=16/solve | workspace·adapter 구현 | PR-N04 owned-session continuation, 기존 exact time/state/callback identity와 epoch invalidation. 메모리 재사용과 과거 수학 정보 재사용을 구분한다. |
| strict/RHS work1.428x, guest-instruction slope2.420x | switcher·탐색 비용과 후보 알고리즘 비용의 결합 | PR-N06 residence/payback theorem. 과거 측정만으로 미래 이득을 승인하지 않고 같은 목표의 비용 상하계·variation 조건·모든 실패 탐색비를 포함한다. |
| cubic benchmark unequal checksum과 미완료 finite-step planner | 알고리즘·정확도 계획 | PR93 chord-integral constant1/12, O(logN) rational decay sums, root/update/clock prefix budgets. 동일 achieved accuracy를 확인한 후 비교한다. |
| trace/cyclic closure physical scope 제한 | 구조 provider·인증 입력 의존성 | PR97/98 multiplicity-preserving trace, PR101 F6 certificate, PR100 event-sign proof를 source별로 연결한다. selected-real branch와 all-root aggregate는 별개 target이다. |

이 표의 profile 수치는 guest instruction 관측이다. 미계측 잔여 시간을 “수학적으로 필수”라고 배정하지 않는다. Timing의 adverse blocks와 산포도 보존한다. PR101 dense-FD median1.060의 block range는0.609–1.384, autonomous control median0.979다. 수학적 계산 감소, allocation 감소, instruction 감소, wall-time 개선은 서로 다른 주장이다.

## 현재 CI가 새로 알려준 문제

최신 PR101 head의 Actions를 직접 조회하고, 실패 job 로그를 읽었다. 원문 실패 문맥은 `evidence/CI_FAILURE_EXCERPTS.json`, job/step 목록은 `evidence/CI_RUNS.json`에 있다.

| Job / run | 실제 첫 실패 | 해석 |
|---|---|---|
| E4 40781615906 / Audit2 40781616016 | `time_fd_workspace_contracts.rs:112`의 `b.clone()`, WorkCounters Copy | 새 테스트의 lint 오류. `*b`로 교정하고 실제 대상 lint 재검증 가능. |
| A2/A3 40781616049 / Production test40781615964 | Givens RHS1e14에서 candidate `Ok(3640)`, pinned Err 기대 | 기존 profile-sensitive 결과가 다시 나타남. 실패는 exact residual assertion 이전이다. 실제 input/output bits와 독립 원래 residual 검증이 선행되어야 한다. |
| Production semver40781615964 | main workspace에서 package vigilode manifest를 못 찾음 | CI setup 실패. 별도 downstream fixture가 재현한 E0063/E0004 API break와 구별한다. |
| Production bench40781615964 | SUNDIALS7.1.1 헤더의 `mpi.h` discovery 실패 | hardcoded gcc와 설치환경 include/link 계약 문제. 로컬7adapter PASS가 이 환경의 성공을 뜻하지 않는다. |
| Ignored40781616237 | WFiltered15/7=2.143>1.3 | 보존된 과학적 cost gate 실패. Off interior311.467, WF0.350으로 정확도도 다르다. |
| Research40781615865 | main-base40 inherited diagnostics,184ledger rows |22개 기존 node의 누락. authority 단계는 skipped였으며 PASS가 아니다. |

Production docs/examples/deployment smoke, fmt, narrow facade/core Clippy는 해당 job에서 통과했다. A1 inner-tolerance parity는 감사 조회 시 in_progress였으므로 최종 결과를 추정하지 않는다. 최신 open standalone issue 검색은0건이지만 repository gate와 연구 의무는 여전히 존재한다.

## 보존·예산·규칙 해석

23-node/34-edge DAG,8suites,73paths와10interactions를 보존한다.10interactions는 PR101 SUMMARY 기준 모두 PENDING_NOT_RUN이다. Q-API는 compatibility failure가 재현되었고, Q-LEDGER의 마지막 full authority는31missing/16repository-unavailable/11invalid-repository/4unreachable로 FAIL이다. 최신 CI에서는 authority가 실행되지 않았다.

현재 inherited counters는 CURRENT_STATE.json에 source 그대로 복사했다. 특히 Q-LEDGER와 PR-N05는3/3, R-STRUCT와 R-ADAPT도3/3이다. PR-N01/N02/N06은0/3, PR-N04는1/3, PR-N03은2/3, Q-API/Q-LINT는1/3이다. 이번 사용자의 “최대3회”는 새 synchronized campaign의 global correction ceiling으로 기록한다. 기존 task counter를 초기화하거나 이름을 바꾸어 추가 허용을 만들지 않는다. 실제 repair는 controlling task와 global episode에 함께 기록하고, 같은 원인의 coherent prepublication repair를 여러 통계 단위로 쪼개지 않는다.

`docs/RESEARCH_LEDGER.md`의 “proof/CAS는 numerical holdout 통과 뒤” 규칙은 현재 사용자의 정밀 이론 우선 지시와 충돌한다. 최소 교정은 이번 연구에서 proof/CAS를 코딩 전에 수행하도록 순서를 바꾸는 것이다. pre-run source·protocol publication, append-only ledger, 실패 보존, timing authority 규칙은 유지한다. 과거 retrospective registration을 prospective로 바꾸지 않는다.

감사 당시 production 판정은 **HOLD**다. 이 감사는 구현을 수정하거나 solver 실험을 실행하지 않았다. 새 이론·코딩 성과와 통합 실행은 이 감사 이후 별도 source/receipt로 기록한다.

## 범위가 고정된 재사용 권고

1. PR68/69 계수 machinery·pool lifetime과 후속 scaled/guarded polynomial path를 다시 만드는 대신 정확한 epoch를 가진 provider에 연결한다. PR93의 다섯 distinct-vector fused Clenshaw는 operator products5m→m의 대수적 기회다. backward rounding defect certificate가 구현 admission 의무다.
2. PR94 error graph의 original target와 anchor를 기준으로 contraction transport, physical norm, observation clock을 한 번만 계상한다. q-step 출력과 exact-flow 출력의 차이를 통합 예산에서 숨기지 않는다.
3. PR98 controller word-preserving ownership relation을 PR-N04 workspace 재사용에 발전시키되 FSAL/history/pending/fatal state 및 callbacks의 관측열을 포함한다. allocator pool 재사용이 stale operator cache 재사용을 허가하지 않는다.
4. PR100 sign certificate는 controller symbol과 physical itinerary를 연결하는 좁은 증명 입력이다. 일반 nonlinear extension은 tube containment·log norm·event Lipschitz·strict directional derivative의 실제 provider를 요구한다.
5. PR99 chemical remainder는 실제 source binding을 가진 경우의 nonlinear defect budget 입력이다. H–Li catalog의 존재로 부족한 reaction-rate cell을 채우거나 전체 physical client를 승인하지 않는다.

## 근거 위치

- `CURRENT_STATE.json`: source pins, 새 CI 문제, 분류, inherited counters, global cap 해석.
- `evidence/PRS_CURRENT.json`: 실제 PR metadata 조회.
- `evidence/MAIN_ANCESTRY.json`, `PR98_TO_PR101_DIFF.json`: 실제 compare 응답.
- `evidence/CI_RUNS.json`, `CI_FAILURE_EXCERPTS.json`: 실제 Actions 응답과 실패 원문 문맥.
- `evidence/PR100_THEORY_SOURCE.md`, `PR99_REMAINDER_CONTRACT.json`: 별도 연구선의 읽은 입력. 저장본은 audit copy이며 원본 Git blob identity는 CURRENT_STATE에 별도 기록한다.

