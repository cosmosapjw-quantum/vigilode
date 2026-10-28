# Local Codex 인계: 검증과 전체 궤적 평가

아래 prompt를 그대로 전달할 수 있다. 구현·증명·첫 benchmark는 이미 이 브랜치에 있다.

```text
저장소: https://github.com/cosmosapjw-quantum/vigilode
브랜치: research/adaptive-causal-20261023
목표: 새 코드를 다시 설계하지 말고 local 컴파일과 실제 모델 검증을 수행한다.
NCP는 사용하지 않는다. 사용자의 기존 checkout, local 수정, receipt는 보존한다.

1. git status, 현재 HEAD, 원격 브랜치를 읽고 실행 identity를 기록한다.
   fetch 후 해당 원격 tip을 기준으로 새 임시 detached worktree를 만든다.
   기존 checkout을 reset/강제 checkout하지 않는다. PUBLISH_MAP.json의 원격
   native commit d2775a7a0c0e5784d190158c2a3159f0f89d4f72와 그 부모
   0aeacfff9e95dc7d349b7cf4d0cc1acec60c78a4가 tip의 조상이어야 한다.
   로컬 evidence commit d4e8623b/548526ad는 GitHub API 게시 과정에서 commit
   메타데이터가 달라졌다. 각각 전체 Git tree가 동일하다는 검증이 기록되어 있다.
   원래 로컬 hash의 remote ancestor 여부를 요구하지 않는다. 이후 문서 commit은 정상이다.
   예상 tip이 없으면 경로와 git 오류를 보고하고 기존 checkout을 그대로 둔다.

2. research/adaptive_causal_20261023/REPORT_KO.md, DECISIONS.json,
   NEXT_STEPS.json, REPRODUCE.md, BENCHMARK_PLAN.json을 읽는다.
   원고 전체와 이미 검증한 864/324/112/36개 기록을 다시 돌리지 않는다.
   소스/전제에 영향을 주는 변경 또는 다른 host 측정만 새 검증 대상으로 한다.
   앞선 구면 연구는 research/spherical_information_20261023/에 함께 있다.

3. 로컬 toolchain과 locked dependency cache를 확인한다. 먼저 offline 빌드를
   시도하고, 없는 dependency는 정확히 기록한다. cache 실패를 solver 실패로
   판정하지 않는다. 기존 vendor/cache와 프로젝트의 설치 정책을 따른다.
   REPRODUCE.md의 facade/kernel/coupled 시험을 실행한다.
   컴파일 수정이 필요하면 최소 수정만 하고 diff와 재시험 receipt를 남긴다.

4. 단일 스레드, CPU affinity 고정, timed 구간 동시 작업 없이 measurement profile로
   9-case × 3-tolerance × 9-route 캠페인을 새 디렉터리에 실행한다.
   warmup1+timed3이며 총972기록이다. 900초 예산을 지키고 timeout/failure도 보존한다.
   기존 결과를 덮어쓰지 않는다. source commit/executable SHA/CPU/OS/compiler/
   thread환경/관측시각/오차정의/RHS·JVP·factorization·solve·memory를 기록한다.
   같은 실제 오차 상한에서 비교하고 phase/amplitude도 검사한다.

5. 현재 default Forbidden adapter와 retained DOPRI의108쌍 parity는 정확히 일치했다.
   FD/JVP 허용 policy는 rapid oscillation에서 약32.5배 손해가 있었다.
   이 실패를 숨기거나 단일 threshold 변경만으로 production으로 바꾸지 않는다.
   NEXT_STEPS의 SW02 persistent authorized driver와 SW03 bounded pilot 작업은
   독립된 개발 단계다. 시간 산포가 크면 반복을 늘리고 raw sample을 보존한다.

6. 선언된 Lawson 경로에는 full RHS가 아니라 N(t,y)를 전달한다: y'=Ay+N.
   A는 정확히 선언된 상수 damping/rotation block이어야 한다.
   slow/noncommuting/stiff-forcing control도 반드시 포함한다.
   현재 stiff-forcing control에서 약12.16배 손해가 있었다.
   5179배 결과는 알려진 exact carrier의 특정사례이고 일반 ODE 배수가 아니다.

7. causal Forward의36행은 단계 root 계산 성능이며 물리 embedded acceptance는6행이다.
   전체 궤적 성능은 accepted adaptive integration으로 새로 비교한다.
   full-space residual/embedded error/원래 raw-U equation을 완화하지 않는다.
   현재-stage JVP=0이어도 frozen basis/validation JVP가 남아 있음을 기록한다.

8. 실제 모델과 BENCHMARK_PLAN의 VODE/DVODE·diffsol·Julia 비교를 진행할 수 있으면
   같은 관측·norm·실제 오차·스레드 조건을 맞춘다. 설치되지 않았거나 자원상
   실행하지 않은 비교군은 NOT_RUN으로 남긴다. 장시간 phase, nonnormal transient,
   event/restart, callback 실패, 작은/큰 차원을 포함한다.

9. 결과에는 새 machine-readable SUMMARY/DECISIONS, raw 실패, 재현명령과
   source identity를 포함한다. 현재 broad production은 HOLD다. 정확도·안정성·
   범위와 worst-case 비용 근거가 충족될 때만 상태를 재평가한다.
   실행 가능한 경로, 특수한 경우만 유리한 경로, 추가 최적화 경로를 분리한다.
   연구 보고서의 증명을 재작성하지 말고 달라진 사실만 보고한다.
```

안전한 worktree 준비 예시는 다음과 같다. 원래 저장소 안에서 실행한다.

```bash
git status --short
git fetch origin research/adaptive-causal-20261023
review_tip=$(git rev-parse FETCH_HEAD)
review_parent=$(mktemp -d -t vigilode-adaptive-review-XXXXXXXX)
git worktree add --detach "$review_parent/checkout" "$review_tip"
git -C "$review_parent/checkout" rev-parse HEAD
git -C "$review_parent/checkout" status --short
```

이 준비는 다른 checkout의 branch/HEAD나 파일을 변경하지 않는다. 원격에 새 결과를
게시할 때는 새 receipt의 source identity를 유지하고, 기존 연구 결과를 대체하지 않는다.
