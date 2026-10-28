# VigilODE 클라우드 구현·증명 완료 및 로컬 실행 인계

2026-10-18. 기준 source는 `c513f478f952068b91e3da368fb2cb156d942b28`이다.
이전 감사 `6c067105…` 이후의 35개 커밋을 확인한 뒤, 변경되지 않은 증거를
재사용하고 남은 구현을 연결했다. 최종 source hashes와 독립 검토는
`SOURCE_IDENTITY.json`, `REVIEW.json`에 묶었다. 이 보고서의 판정은 명시한
새 API 범위에 적용한다. 일반 production solver 전체의 판정은 계속 HOLD다.

## 실제로 구현한 결과

| 경로 | 완성된 기능 | 수학·실행 근거 | 판정 |
|---|---|---|---|
| Packet A01/A02 | 요청 mesh별 immutable 다중 window 출력, weighted-ball 전달 | 같은 curve의 연속 residual·sharp log norm; unequal-frequency Fraction oracle 120 포함 검사 | 무감쇠·zero-carrier native 범위 PROMOTE |
| Leja A04 | 명시적 enclosed API, 실제 fused 출력 전체 오차 | coefficient, operator, recurrence, accumulation, scale/fusion의 외향 replay; exact Fraction 18 사례 | dense symmetric represented action 범위 PROMOTE |
| PR101 A05 | 연속 검증→commit driver, 전역 출력 반경 | signed gain, inherited error, same-curve output; 원래 101-output witness 및 비영 nonlinear orbit | 봉인된 PR101 모델 범위 PROMOTE |
| Structured A06 | native K-coordinate 전체 8-stage Woodbury 및 adaptive observed driver | 단계별 dense 비교 60 조합, 정확해·오류 처리 | 수치 구현·O(N) 복잡도 범위 PROMOTE |
| Parallel transport | reusable outward affine prefix | interval composition 귀납법, 214 prefix + 6 initial의 220개 유리수 검사, 1/4 thread 일치 | 연산 kernel 범위 PROMOTE |

Packet은 평가한 binary64 중심과 정확한 저장 curve 사이의 변위를 다음 window의
weighted radius에 포함한다. component disks로 매번 되돌아가는 방법과 달리,
불필요한 `sqrt(N)` 인자가 window마다 곱해지는 현상을 없앴다. 실제 실행 예제
16 modes·32 windows에서 최대 mean radius는 약 `1.127e-15`, 최대 mode radius는
`3.891e-15`였다. 정규화된 가중치라는 근사 대신 저장된 실제 weight sum을 쓴다.
성공 결과는 모든 요청 시각의 원래 budgets를 통과했을 때만 반환한다.

Leja는 기존 legacy API의 EstimateOnly 상태를 유지하면서 별도
`leja_phi_action_enclosed_with_workspace`를 제공한다. 독립 검토에서 총오차
분해의 coefficient-radius × recurrence cross term, additive underflow,
Opitz/Taylor coefficient tail, 출력 fusion까지 확인했다. Certified bound가
유효하더라도 요청 budget을 못 맞춘 경우 `converged=false`가 된다. 이 구분을
회귀 검사했고 PY05 exporter도 실제 authority를 읽도록 수정했다.

PR101의 기존 기록에 언급된 provider source가 원격 tree에 없었다. 새 봉인된
모델과 검증 driver를 실제 소스로 구현했다. 정확해 callback을 solver에 주지 않고
native RODAS5P candidate의 연속 residual을 다항식으로 감싼다. 원래
lambda=-1e6, eta=10, [0,1], 101 outputs, atol=1e-11, rtol=1e-8에서 492 accepted,
292 rejected, 6,272 RHS calls를 얻었다. 모든 출력 반경은 atol 이하이고 최대
표본 오차는 `8.382183835919932e-15`다. 표본 오차와 별도로 연속 tube 증명이
전체 구간을 다룬다. 양의 nonlinear orbit 검토에서 조기 예산 소진이 드러나
시간에 따른 budget headroom을 예약하도록 고쳤고 첫 실패도 보존했다.

## 보장되는 계산량 개선

* Leja prefix-bound interval products: `1,073,280 → 16,512`, 이 부분 65배 감소.
  Truncation 및 coefficient-radius bookkeeping은 O(m²)→O(m). Dense replay의
  O(mN²), suffix transport의 O(m²)와 cold setup은 그대로 비용에 포함한다.
* PR101 polynomial square: 대칭성을 사용해 507→351 primitive calls. 전체
  provider는 795→639이며, 실제 784 attempts에서 122,304회를 줄여 총
  696,398→574,094가 됐다. mesh·오차·tolerance는 유지됐다.
* Collective Woodbury: 동일 frozen Jacobian의 8개 RHS를 O(N) work·storage로
  처리한다. 기존 dense factorization의 일반 O(N³)와 구별되는 구조적 개선이다.
* Affine prefix: 준비된 workspace에서 정확히 `3N-2` compositions, O(N) work와
  O(log N) fork-join span. 초기 구현의 level-wide scheduling과 serial reset을
  검토자가 지적해 실제 재귀 구조로 고쳤다. allocation convenience wrapper의
  serial O(N) setup은 별도로 명시했다.

이들은 정의한 연산 모델의 보장이다. wall-clock speedup은 아래의 로컬 공정 비교로
측정한다. 작은 입력에서는 parallel scheduling 비용이 지배할 수 있다.

## Homotopy와 polynomial 확장

`PARALLEL_THEORY.md`의 PT2는 coarse/fine window maps가 공통 invariant product
ball에서 q<1 contraction일 때 homotopy parallel shooting의 유일한 고정점,
예측 오차 `Δλ Δ/(1-q)`, k회 후 `q^k D` 오차, map 오차 η에 의한
`η/(1-q)` floor와 실제 latency 개선의 충분조건을 증명한다. Common damping의
`κ>S²|χ|E`는 collective 모델의 fine flow에 대한 구체적인 contraction 조건이다.
Coarse map의 공통 invariant-ball 보존과 contraction은 별도로 확인해야 한다.
이는 실제 nonlinear homotopy client가 이미 실행됐다는 주장이 아니다.

Laguerre/PY02의 기존 ABSTAIN과 기존 HT01/HT02 결과는 그대로 보존했다. 새 Leja
인증은 다른 polynomial router의 전체 비용·정확도 결정을 대신하지 않는다.
Sparse/callback Leja, damped packet, 일반 OdeProblem continuous provider는
추가 개발 범위로 남는다. 이 인계에서 로컬 Codex에게 미완성 핵심 코드를 숨겨
넘기지 않고, 이번에 완성한 경로의 compile·실행·작은 호환 수정만 요청한다.

## 검증과 발견된 환경 문제

실제 원본 Rust 1.94.1과 제공 offline vendor를 복원해 컴파일했다. 처음에 잘린
core metadata를 원본 archive에서 복원했고, proc-macro E0463은 누락된 libstd
동적 loader 경로를 추가해 해결했다. dependencies와 Cargo.lock은 바꾸지 않았다.
최종 all-target compile, targeted contracts, Fraction oracles와 runnable examples의
실제 명령·exit code는 `VALIDATION.json`과 `evidence/runner_*`에 있다.

Allocation 양성 대조군은 **실패**했다. 세 개의 nonempty Vec를 clone하는 library
호출을 이 번들의 custom GlobalAlloc이 0회로 세었다. 독립 target 디렉터리에서도
재현됐다. 따라서 이 환경에서의 과거/신규 “0 allocation” test 성공은 계측의
타당성을 확보하지 못한다. 수치 정리에는 영향이 없고, source 수준의 buffer 재사용과
측정 기반 allocation 주장을 분리했다. 새 비교 runner는 calibration 실패 시
allocation 값을 null로 출력하며 별도 양성 대조 test는 의도대로 실패를 유지한다.
로컬의 정상 toolchain/profiler에서 재확인할 정확한 명령을 제공한다.

독립 검토에서 발견한 잘못된 span 표현, q=0 case, stale Schur getter,
native-failure counter 이중 분류, nonlinear headroom 문제를 수정했다. 증명과
수치 경로의 scoped 승격은 이러한 교정 후의 source에만 적용한다. 최초 실행
오류와 수정 후 로그를 함께 남겼다. 원시 로그에 포함된 이전 상태는 최종
`DECISION.json`으로 해석한다.

## 로컬에서 바로 할 일

`HANDOFF_CODEX_KO.md`를 Codex에 전달한다. `run_local.py`는 check/smoke/bench를
제공하며 source identity, compiler, lock hash, 명령·stdout/stderr·exit code를
저장한다. 성공한 check를 불필요하게 재연구하지 않고 같은 입력의 실제 optimized
실행으로 이어간다. 구조적 비교는 같은 equation/output/tolerance에서 setup을
포함한 시간과 두 해상도 독립 RK4 reference, WRMS, 실패 arm, calibration을 남긴다.

다음 승격은 scope별로 판단한다. 새 packet·Leja·PR101 경로의 성공을 broad generic
solver 또는 모든 문제에서의 speed 승격으로 합치지 않는다. 기존 Diffsol campaign의
accuracy 실패와 과거 보고서 불일치는 재실행으로 덮지 않았다.

## 게시 전 기록 검사

원격 소스 commit `b2379658a4eb0d8b4ddddcd58f754ef28e615c12`의 tree가 검토한 로컬 tree와 일치한다. Authority-reference 검사는 통과했다. 전체 research ledger 검사는 기존 L-0124의 `CONTINUATION_DAG.json` 및 `CONTINUATION_FINDINGS.json` 출력 해시 불일치 두 건으로 실패했다. 두 파일은 기준 commit과 바이트 단위로 같아 이번 변경에서 생긴 불일치가 아니다. 과거 ledger와 파일을 그대로 보존했으며, 새 노드·append-only 오류는 없었다. `evidence/publication_checks.json`에 실제 기대/관측 해시를 기록했다. 이 저장소 기록 문제를 신규 수치 검사 통과나 production 전체 통과로 바꾸어 해석하지 않는다.
