# VigilODE solver portfolio: 단기 실행 DAG

**현재 판정: HOLD_NOT_RELEASE_QUALIFIED**

이 패키지는 기존 PLAN_BUNDLE 및 parallel peer research addendum을 대체하지 않는다. 2024-2026년 문헌과 최신 소스 인벤토리를 반영해 후속 여러 턴을 실행할 수 있도록 추가한 실행 계획이다. 기존 negative evidence, coefficients, tolerances, method defaults와 release gates는 변경하지 않는다.

## 기준점

- 작업 브랜치: research/solver-portfolio-shortdag-20261109
- 문서 기준 브랜치/HEAD: docs/solver-consolidation-plan-20261108 / becbc75438354d7bc8ae3fea362ce943d4d2272a
- 코드 기준 브랜치/commit/tree: research/vigilode-c5-native-graph-schur-20261102 / 569ce02629498fec133db9ce368eb303ba943b69 / a4b2f09521b197b37612ff7dc66a13fbb74a0bae
- 기존 문서 브랜치는 code source lock 위에 39개 문서 커밋이 추가된 상태이며 비교된 27개 변경 파일은 모두 문서다.
- 정확한 blob identity는 SOURCE_LOCK_20261109.json을 기준으로 한다. 소스가 바뀌면 lock부터 다시 수행한다.

## 연구 방향의 결정

원본 RODAS5P의 8개 stage는 8단계 causal lower-triangular dependency를 가진다. 잠긴 transformed tableau에서는 28개의 가능한 strict-lower coupling이 모두 사용되므로, 일반적인 한-step scheduler만으로 이 의존성을 없앨 수 없다. 따라서 세 접근을 구분한다.

1. **원본 RODAS5P 보존 최적화**: RHS/JVP/operator/Krylov/spatial rows 및 독립 IVP ensemble의 CPU 병렬화. 이 방법을 stage-parallel이라고 부르지 않는다.
2. **Certified speculation**: 여러 stage 후보를 batched round로 계산하되 원래 RODAS residual과 output/LTE budget을 검증한다. 거짓 accept가 없어야 하며 실패 시 이전 accepted state로 순차 RODAS5P fallback한다.
3. **Genuine stage-parallel method**: common-shift 또는 multi-implicit two-step Rosenbrock-W/peer-W처럼 새로운 method identity를 도입한다. 자체 coefficient, variable-step proof, startup, estimator, peer history와 restart가 필요하다.

2024-2026 문헌은 세 번째 경로를 보강한다. SEPP는 TASE-stabilized peer, 2026 reused-stage peer는 MOL 비용/stability comparator, 2026 variable-step L(alpha)-stable peer는 고차 stiff-norm comparator다. Stage reuse나 diagonal implicitness만으로 stage independence를 주장하지 않는다. 상세 제한은 LITERATURE_DELTA_20261109.json에 있다.

## 여러 턴의 DAG

| 턴 | 작업 | 완료 산출물 |
|---|---|---|
| T00 | source lock, 최신 문헌 델타, method inventory, 실행 DAG, prompt | 현재 커밋에 백업 |
| T01 | PR/branch/commit/Library 계보와 28-entry source inventory 복원 | history matrix, lineage DAG, negative evidence index |
| T02 | peer-W/SEPP/reused-stage/stiff peer coefficient·stability·startup·estimator 심사 | exact certificates, method selection |
| T03 | RODAS causal graph, P1/P2/P3 CPU 병렬, P4 speculative homotopy, P5 nilpotent lift 결정 | experiments, residual/order/economic gates |
| T04 | 공통 Problem, StepperPlugin, MethodDescriptor, StepAttempt, WorkLedger, capability routing | schemas와 migration contract |
| T05 | 선택된 peer-W method를 opt-in plugin으로 수직 구현 | source-backed method + startup/history/rollback tests |
| T06 | 기존 2D/sphere client 재사용, 3D와 일반 sphere MOL operator 구현 | geometry-aware contracts와 convergence tests |
| T07 | matched achieved-error full-IVP benchmark, clean-clone CI, 독립 심사, production verdict | release decision |

실행 순서와 acceptance tests의 권위본은 SHORT_TERM_DAG_20261109.json이다.

## 이미 있는 공간 코드와 아직 없는 것

- **2D**: periodic Gray-Scott RHS/JVP 및 source-specific logarithmic-norm provider가 구현돼 있다.
- **Sphere**: P06 full-sphere photon finite-volume client, geometry/layout, banded Jacobian, oracle parity contracts가 구현돼 있다. P06는 unpolarized/non-tilted scope에서 PASS_SCOPED이나 production은 HOLD다.
- **Spherical research**: remap, information geometry, spectral tools 연구 패키지가 있다. ODE solver certification과 구분한다.
- **3D/general geometry**: 범용 3D MOL API와 일반 spherical surface operator portfolio는 아직 qualification되지 않았다.

## Mutation/backup 규칙

- 매 턴 마지막에 실행 영수증을 추가하고 별도 커밋으로 push한 뒤 원격 readback한다.
- 각 구현 작업은 별도 worktree에서 수행하고 수정 allowlist를 지킨다.
- main, protected tableau, method defaults, thresholds, tolerances, historical receipts를 변경하지 않는다.
- 이 단기 DAG는 CPU-only다. GPU code/dependency/benchmark는 금지한다.
- 빈 CI 결과, 실행하지 않은 명령, 누락 로그는 NOT_RUN이다.
- 두 턴 연속 재현 가능한 진전이 없으면 반복 감사로 시간을 소비하지 말고 정확한 blocker와 stop decision을 남긴다.

## 실행 프롬프트

다음 local agent invocation은 NEXT_TURN_AGENT_PROMPT_KO.txt를 읽고 T01만 수행한다. 각 턴의 acceptance gate를 닫고 새 evidence/receipt를 push한 뒤에만 다음 턴으로 넘어간다.