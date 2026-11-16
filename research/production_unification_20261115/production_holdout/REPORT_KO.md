# U11 production holdout — 2026-11-16

판정: **HOLD**. 사전등록(`PREREGISTRATION.json`, commit 745ffafb)은 어떤 holdout 실행보다 먼저 push했다. runner와 참조는 891044d7에서 고정했다. 실행은 이 commit의 깨끗한 detached worktree에서 새 target dir로 빌드해 진행했다(`logs/host_identity.txt`, `logs/runner_binary.sha256`). U11 상태는 **PARTIAL**이다: CVODE(SUNDIALS 미설치), Julia 참조 RODAS5P, 독립 검토자 재실행이 NOT_RUN이다. 같은 에이전트가 깨끗한 checkout에서 돌린 것이며, 사람 검토자가 독립적으로 재현한 것은 아니다.

## 설정 (사전등록 그대로, 조정 없음)
- 문제: HIRES(8), Oregonator(3), Brusselator 1D MOL N=32(64). U03/U04 calibration에 쓰지 않은 문제다.
- tol 1e-4..1e-8, atol = 1e-2·tol. arm: production RODAS5P(`Rodas5p::solve`, dense J), PPSW1(`PeerOde`, 1 thread / 6 thread).
- 참조: SciPy Radau rtol 1e-13, U_ref = |Radau13−Radau12| + |Radau13−DOP853_13| ≈ 1e-13(3문제 모두). 모든 점이 admissible이다.
- 시간: arm 순서를 회전하며 interleave, 2 warmup + 7 반복의 median. 공유 호스트이므로 진단용이다.
- SciPy BDF/Radau는 count만 기록했다(`reference.json`, Python이라 wall은 비교하지 않음).

## 결과 (`EVALUATION.json`)
PPSW1은 45개 실행 중 실패 0이다. 동일 달성오차(겹침 구간, 로그-로그 보간):

| 문제 | step 배수 (PPSW1/RODAS5P) | 속도 (RODAS5P wall / PPSW1 6thr wall) |
|---|---|---|
| Brusselator MOL32 | 1.9–5.2 | 0.049–0.14 |
| HIRES | 3.4–5.8 | 0.0049–0.0082 |
| Oregonator | 2.5–7.1 | 0.0023–0.0061 |

6 thread는 이 작은 문제들에서 1 thread보다 느리다(thread overhead).

## G0 기준 (각각 그리고 결합)
| 기준 | 결과 | 비고 |
|---|---|---|
| order ≥ 4.8 | 형식상 통과 (median 10.4) | 오차 대 step 기울기다. F2(tol에 둔감한 step 수) 때문에 과대 추정되며 단독으로 의미가 없다 |
| steps ≤ 1.2× | **실패** (최대 7.1×) | |
| median speed ≥ 1.15× | **실패** (0.006×) | |
| fast explicit-J = 0 | **실패** (최대 5188) | PPSW1 dense는 step마다 J를 쓴다 |
| fast Newton = 0 | **실패** (최대 7) | starter의 simplified Newton |
| depth p95 ≤ 6 / max ≤ 8 | 통과 (stage depth 1) | 내부 span: step당 LU 1회 + solve phase 1회 |
| fallback ≤ 0.1 | 통과 (0) | fallback 없음 |

결합: **실패 → HOLD**. 기본 라우팅·G0·허용오차·main은 변경하지 않았다. release는 어떤 경우에도 소유자 승인이 필요하다.
