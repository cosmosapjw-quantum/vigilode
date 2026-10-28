# VigilODE: 2026-10-29 물리 이벤트의 기호 검증을 위한 연구 노드

## 대상과 최신 상황

기준 저장소 `cosmosapjw-quantum/vigilode`. 원격 main `8d0c79184e09efb5bdadc24a6315c60a71a44264`; PR95 `36104bfc`, PR96 `13f3298b`, PR97 `87465835`, PR98 `bf308589`, PR99 `e46704cb`. PR90–94는 PR96→97→98 누적 연구선의 선조이고 PR95→99는 별도 actual-source H–Li 이론선이다. 후자를 무작정 PR98에 병합하지 않는다. 현재 main은 8월 26일 기준이며 최신 승인 production line이라고 간주하지 않는다.

원래 PR97 23-node/34-edge local plan에 기록된 7 `complete_scoped`, 15 `pending_local`, 1 authority 부분 진행을 그대로 유지한다. **원본 DAG 7/23 scoped, 증가 0.** PR98 evidence에 따르면 375 integration tests and 70 library tests scoped PASS가 있으나, authority 56 missing, main-base registration 40 omissions, strict coupled all-targets 26 diagnostics FAIL, Q-API/Q-GIVENS/10 interactions/real-client/release 등이 열린 상태이다. 실패들은 소스·프로파일별로 보존해야 하며 전부 같은 검증이 아니다.

별도 Drive·Dropbox에서 10월 28일 source chemical, matrix, norm, photo, 10월 29일 tau-row H–Li 백업 metadata/내용을 조사했다. 최신 tau row는 reduced30 indices23..28 chemical, tau29, 20 geometry columns null; full actual ODE enclosure가 아니며 REI 중앙 28/88은 VigilODE의 7/23과 별개다. 다른 대화 내용은 최신 사용자 문맥으로 확인했지만 전체 스레드·227개 이상 branch의 완전 원문 열람을 주장하지 않는다. 실제 오프라인 PC의 unpublished worktree는 관측하지 못했다.

## 이번 이론 루프 R1–R10

기존 Dini/lognorm residual 비교와 Bernstein convex-hull, 두 rational endpoint-envelope를 입력으로, **연속 물리 상태의 event g(y) sign**을 보수적으로 인증하는 수학 계약을 유도했다. `THEORY_KO.md`는 일반 nonlinear sufficient conditions를 별도 명시한다. PR98의 finite controller words와 원래 연속 ODE symbolic itinerary를 구별하며, 근거 없는 approx-bisimilarity는 주장하지 않는다. Wolfram 직접 평가로 두 rational envelope의 차 항등식이 0으로 확인되었다.

## 이번 코딩 루프 C1–C10

`symbolic_oracle.py`와 `test_symbolic_oracle.py`: scalar affine model `y'=-λy+b`, λ>0, `p(t)` rational coefficients, exact `Fraction` operations, whole-slab original equation defect, error radius와 이벤트 sign/unique crossing. Python 8/8 PASS, 고정밀 Decimal 918 point checks PASS(이는 수학적 인증이 아닌 진단). 오라클에서는 float input을 자동 변환하지 않으며 임계값 접촉·판정 불가·음의 λ를 보수적으로 처리한다. 다음 표는 **유리수로 확인한 값**이다.

| Observable | Value |
| --- | --- |
| Original residual | `r(t)=t²/2` |
| Whole-slab residual cap | `1/128` |
| Whole-slab state error cap | `1/1088` |
| `g(y(1/16))` | `POS` |
| `g(y(1/15))` | `NEG` |
| `g(y(33/512))` | `UNRESOLVED` |
| Certified crossing | `UNIQUE_CROSSING` under strict monotonicity |

원래 그림에서 defect `[state/time]`와 state error `[state]`를 동일 y축에 나타낸 오류를 발견했다. 해당 `diagnostic_mixed_units_REJECTED.png`는 실패로 남기고 `residual_only.png`, `state_error_only.png`, `event_certificate.png`를 다시 생성해 직접 읽었다. 그림은 검증 계약을 시각적으로 설명하며 strict certificate는 Python exact rational 값이다.

## PHYS-MATH → PHYS-MATH-CODE 독립 감시

- PHYS-MATH: 단위, 부호, exact original residual, lognorm/Dini 조건, 전 구간 covariance가 아닌 **deterministic scalar** 오차, endpoint vs whole-slab, threshold strictness, transversality 증명. PASS_SCOPED.
- PHYS-MATH-CODE: 코드 변환, exact Fraction, sign/negative path, Decimal 진단, GitHub blob byte parity. PASS_SCOPED_TOY.
- 외부 independent subagent: 현재 세션에 별도 subagent dispatch API가 노출되지 않아 `NOT_RUN`. Exa/SciSpace/Wolfram 독립 서비스와 adversarial tests는 이를 대체하는 다중 에이전트 심사로 격상하지 않는다.
- Native Rust: 원격 source tree의 Rust integration 검사 및 새로운 actual client ODE 실행은 `NOT_RUN`. 해당 테스트 수치를 이전 PR의 성공으로 대체하지 않는다. 현재 Python `PYTHON_PASS`; Rust toolchain은 이 컨테이너에서 실행하지 않았다.

## GitHub 게시/백업

새 격리 브랜치 `research/physical-symbolic-defect-oracle-20261029`, base PR98 exact head `bf308589e21b524d0ddc6e571b260f5e94a853f0`. 이론 소스는 commit `17a38e674b8842d19985d6b08d61155612a7026c` (Git blob `36d6c57...`)에서 readback identity 확인. Native 소스 아닌 Python oracle commit `58fadf00644d606747a98128e240c4e34320dd80` (blob `22a1d66...`), tests commit `ad3493345a02b4bc7a28852207307ff12a12b330` (blob `77575fd...`), 내용·blob hash match. main 또는 PR98 source 자체를 수정하거나 merge하지 않았다. 이후 manifest/판정/실행 handoff의 GitHub 게시 여부 및 두 cloud backup ACK는 실제 도구 응답으로만 갱신한다.

## Gates / 다음 최소 작업

P0/P1: 실제 native library로의 adapter 연결, `f(t,y)`/dense curve의 source·FP/clock·domain/epoch identity, outward rounding과 Jacobian/lognorm tube certificate가 없다. 그러므로 solver의 event optimizer로 사용하거나 root finding을 skip시키면 안 된다. P1: 이벤트 transversality/grazing, nonlinear mode changes, discontinuities, callback side effects. P2: total-work speedup은 아직 미측정. P3: doc/API cleanup은 보류.

**다음 하나의 DAG-admissible 단계:** 원본 PR98 source를 고정한 분리된 worktree에서 `VIGILODE-SYM-02` (native event-provider/adapter 계약)를 먼저 구체화한다. 실제 dense-output polynomial의 clock/source/rounding·event callback identity를 연구하고 변경 전후 원래 ODE residual oracle, sign/refusal coverage와 exact source identity를 검증한다. `VIGILODE-SYM-01`를 production default로 연결하지 않는다. 기존 Q-LEDGER/authority와 Q-GIVENS 등 production gate는 별도로 유지한다.

## 출처

- PR98 code/theory: https://github.com/cosmosapjw-quantum/vigilode/blob/bf308589e21b524d0ddc6e571b260f5e94a853f0/research/dynamics_symbolic_validation_20261028/theory/DYNAMICS.md
- PR97 closure: https://github.com/cosmosapjw-quantum/vigilode/blob/87465835d9d21298d235917f036580b0b53ab645/research/production_rvj_nag_20261028/theory/CLOSURE.md
- Higueras, Söderlind (2002): DOI 10.1023/A:1021956621531
- Pola, Girard, Tabuada (2008): DOI 10.1016/j.automatica.2008.02.021

`production:HOLD`. 분모가 서로 다른 original VigilODE 7/23 scoped, auxiliary 1/1 scoped, REI 28/88은 혼동하지 않는다.
