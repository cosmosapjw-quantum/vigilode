# Push 이후 상태와 로컬 검증 우선순위

기준일: **2026-10-28 KST**. 원격 관측 시각은 `CI_SNAPSHOT.json`에 기록했다.
PR #92의 수치 구현·원자료·21쪽 REVTeX 보고서는 공개 commit
`78bb8775bf553ad2b514575a0ef4a025446f5d28`에 고정되어 있다. 이 부록은 그 뒤에
완료된 CI 결과와 검토를 추가한다. 이번 부록에서 수치 코드를 바꾸거나 native
실험을 다시 실행하지 않았다. 재개 시 PR #92 이후 추가 PR은 없었다.

**Production 설정과 제한된 경로의 구현은 완료되었지만, 전체 repository CI 및
실제 배포 환경의 release 승인은 완료되지 않았다.** 기존 post-repair native
464 passed / 0 failed / 4 ignored는 해당 범위의 실제 기록이며 전체 CI 통과를 뜻하지 않는다.

## 현재 CI와 원인

| 관측 | 분류와 필요한 조치 |
|---|---|
| Production fmt, facade/core Clippy, docs/examples/serialized smoke 통과 | 실제 원격 job 결과다. Integrator 전체 Clippy/전체 테스트로 확대하지 않는다. |
| Production test와 A2/A3 Givens 실패 | RHS1e14에서 candidate `Ok(3640)`, 역사적 expected-failure assertion이 외부 잔차 확인 전에 실패. 검토한10파일은 PR91과 동일하다. 정확한 입력 비트·profile·실제 잔차로 원인을 분리해야 한다. False convergence는 이 로그로 입증되지 않았다. |
| A1 parity의 fair-AB 실패 | `apply_counted`의 시도 수 A와 budget 허가 수 E의 혼동. 관측은 A2/E1/R1/B1이며 올바른 관계는 A=E+R, E<=B다. 관련11파일은 PR91과 동일하며 Lagged driver를 호출하지 않는다. Debug assertion 삭제나 budget 증가는 해결이 아니다. |
| Bench `mpi.h` 실패 | Ubuntu26.04 / libsundials-dev7.1.1 / MPI development package가 설치된 환경에서 hardcoded gcc가 include/link 탐색을 하지 않는다. 기존 bridge의 선언 target은6.4.1이다. 단일 include 경로나 `CC=mpicc`만 추가해 완전한 해결로 표시하지 않는다. |
| E4/Audit2의 integrator Clippy 실패 | 고유23진단 위치 중21은 PR91 byte-identical,2는 검증된 rustfmt 변화뿐이다.17lib/22libtest는 중복 집계이므로 합산하지 않는다. |
| Semver 실패 | 이번에 추가한 `SwitchingConfig`의 두 public field가 exhaustive literal을 깨는 실제 source break다. `continue-on-error`는 호환성을 뜻하지 않는다. API 배치 또는 명시적 버전·migration 결정을 release 전에 검증한다. |
| Ignored suite W-filtered kill gate 실패 | 소스가 이미 기록한2026-10-21의 음성 결과다:15/7>1.3. Off의 interior error311.467과 WFiltered0.350은 정확도가 다르다. 기존 실패·기준을 유지하면서 expected-negative regression의 실행 의미를 검토하고 나머지 ignored 검사를 실행한다. |

각 job ID, 원문 로그 hash, 소스 비교와 구체적 수리 계약은 같은 디렉터리의
JSON/MD 검토 파일에 있다. `PUBLIC_LOGS.tar.xz`는 decoded GitHub 로그와 실제
정책 검사 stdout/stderr를 보존한다. GitHub 원래 zip의 byte identity를 주장하지 않는다.

## 사후 원장 등록

기존169행·468008바이트를 그대로 유지하고 **L-0173 / INCONCLUSIVE**를 추가했다.
`PREREGISTRATION.md`는 요구되는 진입 파일명이지만 내용은 명시적으로 사후 등록이다.
이 캠페인은 aggregate preregistration을 실행 전에 commit하지 않았고 unopened holdout도
없으므로 `docs/RESEARCH_LEDGER.md`와 timing contract의 사전 등록 요건을 소급 충족하지
못한다. 기록된 수치 관측과 대수적 증명은 남아 있지만 wall-time ratio는
**diagnostic_only**이며 프로젝트의 사전 등록 성능 승인으로 인용하지 않는다.

수정된 연구 노드 검사는170행을 인식하며 통과했다. 새 정책 문서의 authority 검사는
6citations/2reachable commits로 통과했다. 전체 authority 검사는 shallow checkout에서
과거 object/ref가 부족해 실패했으며 통과로 보고하지 않았다. 전체 history를 가진
환경에서 실제 미게시 commit인지 확인해야 한다. Checker나 allowlist를 변경하지 않았다.

## 로컬 개발 순서

`LOCAL_DEVELOPMENT_PLAN.json`이 기존16-node/20-edge 계획에 관측된7개 qualification
과제만 더한 실행 계획이다. 수치 연구를 위한 선택적 PR-N01~06은 그대로 보존했다.

1. 정확한 최종 전달 SHA에서 별도 worktree를 만들고 전체 과거 refs와 dependency를 준비한다.
2. Q-GIVENS/Q-FAIR-AB/Q-LINT/Q-NEGATIVE의 독립 진단·증명·코드 수리는 병렬로 진행하되
   native 실행은 공통 lock으로 직렬화한다. Q-LEDGER/Q-BENCH/Q-API도 독립 분담할 수 있다.
3. 수정된 코드의 `L-RELEASE`를 새 target에서 실행한다. 벤치마킹은 그 뒤 L-HARDWARE에서
   같은 실제 정확도 조건으로 수행한다. 새로운 binary에 이전 timing을 붙이지 않는다.
4. 실제 client의 장시간 적분/restart/resource exhaustion까지 기록하고 R-QUALIFY를 판단한다.
   full repository CI를 요구하는 release 판단에서는 나머지 qualification 문제도 닫아야 한다.
5. PR-N01~06은 각 이론 가정과 승인 조건을 충족하는 별도 확장으로 진행한다. 이를 전부
   끝내야 현재 제한된 RHS-only route를 사용할 수 있다는 식으로 과제 범위를 늘리지 않는다.

과제당 수정·재실험은 최대3회다. 완료된 structural3/3, homotopy1/3, adapter3/3와
별도 lagged accounting1/3을 재명명해서 초기화하지 않는다. 이번7개는 새로 관측된
구체적인 repository qualification 문제이며 기존 수치 최적화의 네 번째 반복이 아니다.

## API migration

기존 exhaustive `SwitchingConfig` literal은 새 필드
`strict_return_work_improvement: false`, `reuse_observed_entry_step: false`를 명시하거나,
의도적으로 `..SwitchingConfig::default()`를 사용해야 한다. 이는 호출자 코드의 예시이며
release version policy를 대신하지 않는다. ProductionConfig의 selective supplied-JVP
template은 명시적인 엄격한 entry/return 정책을 쓴다. 새 public field의 legacy default와
선택적 template 정책을 혼동하지 않는다.

## 보존된 증거의 사용

`../MANIFEST.json`, `../SOURCE_MANIFEST.json`, `../evidence/RAW_MANIFEST.json`은 원래
과학적 snapshot의 manifest다. `SUPPLEMENT_MANIFEST.json`은 이 부록과 원장 추가를
별도로 결속한다. `python3 .../post_push/verify_supplement.py`는 두 범위의 identity를
확인한다. 파일/그래프/hash 검사는 수학 정리나 native test의 대체물이 아니다.
