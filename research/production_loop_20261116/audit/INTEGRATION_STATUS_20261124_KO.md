# 연구 아이디어의 production 경로 결합 상태 (2026-11-24)

- 대상 독자: 외부 감사자.
- 기준: 브랜치 `research/production-loop-20261116`, commit `a8d196b4`. 이 문서를 추가한 commit은 코드를 바꾸지 않는다.
- 작성: 연구 루프 L1 orchestrator 세션, 2026-11-24 19:50 KST. 소유자 질문에 대한 구두 보고(19:40 KST)를 근거와 함께 기록했다. 소유자의 질문(원문): "이 solver에서 adaptive하게 상황에 따라 결합하고자 했던 다양한 아이디어들(causality/jacobian-free/derivative-free/matrix-free/newton-free/homotopy-based root finder/parallelized rosenbrock 8-stages 등 야라 시도들 전부)의 현재 production lane에의 결합 상태는 어떻게돼?"
- 판정의 성격: 기존 기록과 정적 코드 참조를 모은 것이다. 이 문서를 위해 새로 실행한 수치 실험은 없다. 확인 방법과 한계는 6절에 적었다.

## 1. 결합 수준의 정의

| 수준 | 뜻 |
|---|---|
| L0 기본 경로 | `vigilode::Rodas5p::solve`가 선언된 구조에서 고르는 보호 driver 계열(`FacadeRoute`: DenseFast, BandedFast, MatrixFree, FastDense, FastBandedDense, SequentialDense, `crates/vigilode/src/capabilities.rs:11-24`)과 `GeneralOde`의 기본 선택. 사용자가 방법을 고르지 않아도 실행된다. |
| L1 facade opt-in | `vigilode` 공개 API에 있으나 사용자가 명시적으로 골라야 실행된다(`MethodRegistry::route/solve`, `VigilODE::solve`, `SwitchingSolver`, `PeerOde`, `LawsonDopri5`, `Dopri5Ensemble` 등). |
| L2 integrator crate 전용 | `crates/rodas5p-integrators`에 코드와 테스트가 있으나 facade 소스가 부르지 않는다. |
| L3 별도 crate·연구 기록 | `crates/vigilode-coupled`(facade의 의존성이 아님: `crates/vigilode/Cargo.toml`의 의존성은 `rodas5p-core`, `rodas5p-integrators`뿐) 또는 연구 문서·스크립트에만 있다. |
| L4 미이식·설계만 | 다른 브랜치에만 있거나 코드가 없다. |
| 음성 | 사전등록된 판별 실험에서 FAIL·KILL을 받았다. |

## 2. 요약

1. L0에 들어간 것은 보호 RODAS5P 계열과, 이번 루프의 V22 하나다. V22는 반폭 6 이상의 banded 경로에서 같은 driver의 Slices kernel을 쓰는 변경이고, 결과 비트는 바뀌지 않는다(L-0334, `crates/vigilode/src/solve.rs:186`).
2. 실행 중에 상황에 따라 방법을 바꾸는 자동 결합은 DOPRI5와 RODAS5P 두 방법 사이에만 있다(L1: `SwitchingSolver`, unified router의 Auto). 나머지 방법은 사용자가 고르거나(L1), 코드만 있거나(L2), 기록만 있다(L3).
3. RODAS5P 8단계의 병렬화는 시도한 방식이 모두 음성이거나 판정 불가였다. L0의 RODAS5P는 완전 순차다. facade에서 병렬로 실행되는 것은 `Dopri5Ensemble`(궤적 묶음)과 `PeerOde`(peer RHS 병렬)뿐이고, 둘 다 L1이다.
4. 보호 RODAS5P 기본 경로 밖에서 production 자격을 얻은 경로는 없다. G0는 HOLD이고(U11), 릴리스 게이트는 충족되지 않았다(L-0301).

## 3. 소유자가 열거한 아이디어

| 아이디어 | 수준 | 근거 |
|---|---|---|
| Newton-free (선형 암시) | L0 | RODAS5P는 Rosenbrock(선형 암시) 방법이라 Newton 반복이 없다. 기본 경로의 본체다. |
| Jacobian-free / matrix-free (JVP + GMRES) | L0 (JVP를 선언한 경우) | `FacadeRoute::MatrixFree`가 보호 matrix-free driver(`rodas5p_matrix_free_fast.rs`)를 부른다. 개선안은 L2에 있다. per-stage GMRES client는 보호 driver 대비 명령어 0.216~0.430배로 PASS했다(L-0318, V17). adaptive 정책 `MfStageSolvePolicy`는 예산 규칙으로 FAIL했고 opt-in으로만 병합됐다(L-0335, V24). 기본 전환 여부는 V25가 재시험 중이다. matrix-free peer W(PPSW1-MF)는 end-to-end driver가 없어 registry에 Unavailable로 등록돼 있다(`selection/frontend.rs:292-304`). |
| derivative-free (사용자가 f만 제공) | L0 | Jacobian을 선언하지 않으면 FD Jacobian(n+1회 RHS)을 쓴다(`crates/vigilode/src/unified/mod.rs:39-41`). sparse JVP를 선언하면 CPR 색칠로 Jacobian을 복원한다(`solve.rs:669, 1075-1096`, `coloring.rs`). |
| 병렬·색칠 FD Jacobian 조립 | L2 | `AssemblyStageClient`는 순차 참조와 비트 단위로 같다(PASS, L-0330). 시간 이득은 RHS가 step 비용을 지배하는 영역에서만 있었다(dense n=400, worker 2/4/6에서 1.82/3.13/4.16배). 엄격 규칙으로는 FAIL이다(L-0331). V26(병렬·벡터화·multi-colouring 검사)이 진행 중이다. |
| causality (strict causal stage-error budget) | L1 API만 | `causal_budget.rs`는 facade에서 공개·재수출만 되고(`lib.rs:189, 220`), solve 경로는 쓰지 않는다. integrator의 `causal_majorant`와 transactional q1/q2는 L2다. transactional 경로는 4.75배 느렸다(음성, `docs/agent/solver_consolidation_20261108/NEGATIVE_EVIDENCE_INDEX.json` R7_TRANSACTIONAL). |
| derivative-free causal homotopy 계열 (2026-10-27) | L3 | `research/derivative_free_causal_homotopy_20261027/RESULTS.json`의 status는 `pass_scoped`(범위 한정 통과)다. facade 참조는 없다(6절 Part 1). |
| homotopy 기반 root finder | L2와 L3 | 관련 모듈 7개(`homotopy`, `homotopy_policy`, `homotopy_order_policy`, `homotopy_experiments`, `nonlinear_homotopy`, `actual_k_homotopy`, `fixed_point_continuation`)가 integrator crate에 있고, `vigilode-coupled`에 `rhs_homotopy.rs`가 있다. facade 참조는 0건이다. 연구 단계의 상태는 표 아래에 정리했다. |
| N01 derivative-free coupled, coupled BE, residual-first, trace quotient, cyclic closure, SwitchState | L3 (`vigilode-coupled`) | facade의 의존성이 아니다. N01은 owner-substitution 반례로 음성 판정을 받은 뒤 소스를 고쳤다(NEGATIVE_EVIDENCE_INDEX N01_OWNER_SUBSTITUTION, HISTORY_MATRIX lineage N01_C9). |

homotopy root finder의 연구 단계 상태:
- R5 Newton-free algebraic homotopy: native 8-stage 실험과 외부 심사가 NOT_RUN이고, production 코드 변경이 없다(`docs/vigilode_r5_newtonfree_algebraic_homotopy_20261101/RESULTS.json`의 `full_native_8stage_and_same_error`, `external_referee`, `no_production_write`).
- preconditioned fixed-point homotopy: PORTED_BLOCKED(`port_plan/port_contracts.json` P14).
- homotopy 출력 인증서: 선형화 보정일 뿐이라 해결되지 않았다(NEGATIVE_EVIDENCE_INDEX HOMOTOPY_TAIL).

## 4. RODAS5P 8단계 병렬화 시도

| 시도 | 판정 | 근거 |
|---|---|---|
| R6 동시 sweep | 음성 | q2는 W solve 21회 대 보호 경로 8회였고, 끝점 오차가 0.0038~0.020이어서 채택되지 않았다(NEGATIVE_EVIDENCE_INDEX R6_Q2). 정확해지려면 sweep이 8회 필요했다(PLAN_KO.md 1.4). |
| R7 transactional q2 | 음성 | 4.75배 느렸다(R7_TRANSACTIONAL). |
| C5 graph-Schur | 음성 | wall 비 중앙값 2.24, 12개 중 12개가 1보다 컸다(C5_SLOWER). source 한정 opt-in이고 HOLD다. |
| 대각 SDC (U05) | 음성 | sweep이 8~13회 필요했다(PLAN_KO.md 1.4). |
| lane batch | 음성 | 0.93배였다(PLAN_KO.md 1.4). |
| lagged LU | 음성 (kill gate) | HIRES에서 실패했다(`selection/frontend.rs:365-366`). registry의 명시 선택으로만 남아 있다. |
| PPSW1 parallel peer W | HOLD | U11 holdout에서 G0를 통과하지 못했다. L1 `PeerOde`로만 남아 있다. |
| V05 이론 | 구조적 이유 | RHS 의존 깊이가 정확히 8이고, 결합 행렬의 멱영 지수는 7이다(L-0303). sweep을 끊어 쓰는 방식은 강성 성분 오차를 20~40배 키운다(`theory/THEORY.md`). |
| V07 투기적 Jacobian | 음성 | L-0316 |
| V14 Schwarz stage chain | 음성 | 60개 구성 중 17개만 통과했고, 통과한 것은 모두 Gray-Scott였다(L-0304). |
| V16 깊이 5·6 tableau | 음성 | tableau는 존재하지만 matched error에서 이득이 없었다(L-0319). |
| V06 공유 Krylov basis | 음성 | L-0315 |
| V08 병렬 FD 조립 | 정확성 PASS, 시간 FAIL | L-0330, L-0331. RHS가 지배하는 영역에서만 이득이 있었다. L2. |
| V18 dense LU 재사용·병렬 | 재사용 identity PASS, 시간 판정 불가 | L-0332, L-0333. faer 병렬 arm은 보호 manifest를 바꿔야 해서 실행하지 않았다(NOT_RUN). |
| V19 지연 한계 banded kernel | 음성 (KILL) | L-0320 |
| V21/V22 banded kernel | PASS, V22는 L0 | 병렬화가 아니라 순차 kernel의 개선이다(L-0321~L-0323, L-0334). |

## 5. 그 밖의 시도

| 항목 | 수준 | 근거 |
|---|---|---|
| DOPRI5↔RODAS5P 전환 | L1 | `SwitchingSolver`는 `stiffness_gate_fraction`이 설정되면 DOPRI5 secant 강성 추정(`dopri5_stability_limited`)으로 진입을 판단한다(`switching.rs:2263-2271`). unified router의 Auto는 V03b에서 PASS했다(L-0329). V27이 이 Auto를 다시 설계하고 있으며 소유자 승인을 기다린다(`router_hyperparameter_free/DESIGN_RECOMMENDED.md`). |
| registry 명시 선택 | L1 | BDF, Radau, PEXPRB54s4, lagged RODAS5P, Lawson, PeerOde는 직접 호출과 비트 단위로 같다(L-0317). 방법 자체의 자격 심사는 없다(카탈로그 `qualification: ImplementedUnqualified`). |
| hybrid route automaton | facade 모듈이지만 경로에 연결되지 않음 | `switching.rs:935`가 오프라인 검증 모델이라고 밝힌다. 실시간 경로에서 쓰지 않는다. |
| nonnormal observer, secant preconditioner, Chebyshev/Leja | L2 | 코드만 있고 측정하지 않았다(PORTED_NOT_MEASURED, port_contracts P06, P08, P13). 어떤 경로의 입력도 아니다. |
| PTSW2B, SABR, small-batch RODAS5P, collective, packet(RVJ), graph-Schur, stage batch | L2 | facade 참조 0건(6절 Part 1). |
| stage client(BandedTightStageLu, DenseLuReuseStageClient, AssemblyStageClient), MfStageSolvePolicy | L2 | facade 참조 0건. 각 노드의 판정은 4절에 있다. |
| N02 triggered FGMRES, N03 interval certificate, affine event certificate, RVJ envelope | L4 미이식 | port_contracts P09, P10, P11, P15 |
| Blelloch scan, guarded transition words | L4 설계만 | port_contracts P12, P16 |
| 구면·2D/3D 격자 문제 | test 안에만 있거나 workspace 밖 | P03은 PORTED_NOT_WIRED(`vigilode-bench`는 workspace 밖이고 rust-version 1.99.0을 선언한다), P04는 TEST_ONLY. 담당 노드 V11은 일시정지 상태다. |

## 6. 확인 방법과 한계

- 정적 참조 검사: 저장소 루트에서 `python3 -I research/production_loop_20261116/audit/integration_reach_scan.py`를 실행한다. 출력은 `integration_reach_scan.txt`에 있다.
  - Part 1은 facade 소스(`crates/vigilode/src`)의 주석이 아닌 줄에서 연구 구성요소의 고유 식별자를 센다. 0이면 facade가 그 구성요소를 이름으로 부르지 않는다는 뜻이다.
  - Part 2는 integrator 모듈 단위의 거친 스캔이다. 일반적인 이름 때문에 거짓 양성이 생기므로 단서로만 쓴다.
- 이 문서가 확인하지 않은 것:
  - 동적 실행 추적: 기본 경로가 실제로 어떤 함수를 실행하는지는 golden oracle(L-0302)과 V03G 기록이 다룬다.
  - 각 기록의 수치를 다시 실행하지 않았다.
  - port_contracts의 상태는 2026-11-17 시점 값이다. 그 뒤 노드(V06, V08, V17, V18, V21, V22, V24)의 결과는 ledger에서 읽었다.
- ledger 중복 주의: L-0324~L-0327은 각각 L-0317~L-0320을 대체한 행이다(`supersedes` 필드). 원래 행에 형식 오류가 있던 시기에 추가한 것이고, 주장 내용은 같다(소유자 결정 OD-L1-13 이후 원래 행은 제자리에서 정정됐다).
- 자격 상태: G0 HOLD(U11, `research/production_unification_20261115/production_holdout/REPORT_KO.md`), 릴리스 게이트 미충족(L-0301, `audit/AUDIT_KO.md`).
