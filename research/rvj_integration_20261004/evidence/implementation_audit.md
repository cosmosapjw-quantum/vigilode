# 최신 구현 브랜치 소스 감사 (intake)

기준은 `audit/rvj-native-followup-20261003@826fa05cc1fdaf7b1a0cf7ac45116596b4ddff65`다. `wu25-stiff-benchmark@5a8d7fe`는 이 커밋의 조상이다. 독립 구현 가지를 잘못 택한 상황이 아니다. 이 작업은 소스와 게시 결과의 1회 독해이며 과거 수치 실험/전체 suite를 재실행하지 않았다. 새 발견의 실행 재현은 owner의 사전등록 이후에만 한다.

## 유지할 게시 결론

- REV-01c: recycle invariant 복구가 기존 실패를 제거했지만 비용은 cold GCRODR 대비 1.08–1.32배 제품 수다. 반복된 이전 가설 두 개는 FAIL로 남긴다. 현재 데이터에서 recycle의 가속을 주장하지 않는다.
- REV-02: directed exponential, 자동 Osborne metric, certified stepping을 구현했다. 절대 오차의 개선과 강한 decay/nonnormality에서의 상대 bound 실패를 구분한다. REV-02 전체 verdict는 FAIL이다.
- REV-03: 새 h에서 68 acceptance가 one-sided budget으로 resolved. unresolved 6건은 안전 방향의 rejection. 범용 acceptance theorem의 완결은 아니다.
- REV-04: LGMRES solve_into가 1,216 solve에서 기존 구현과 bitwise parity를 보였고 344 failed solve rollback을 포함한다. allocation 비는 0.94–0.99이며 속도 향상으로 확대하지 않는다.
- INT-04 stage-chart adapter의 gate는 FAIL이다. 정규 chart가 Newton을 악화시킬 수 있었고 실제 RVJ predictor를 이식한 실험이 아니다.
- q=2 기본 활성화 및 timing 승격은 HOLD. directed 연산과 plain floating 연산을 동일 비용으로 나누었던 +0.63 margin을 결정 근거로 재사용하지 않는다.

## 실제 코드 경계와 권장 이식 위치

| 기능 | 현재 위치 | 이식 시 지켜야 할 경계 |
|---|---|---|
| 모델별 chart ODE | `crates/rodas5p-integrators/src/chart_transport.rs` | `audit2-research` opt-in; 모델과 branch/time identity; 원 좌표 bound |
| stage coordinate candidate | `stage_chart_candidate.rs::StageChart` | quadratic stage residual용; 일반 ODE chart/RVJ solver로 오인 금지; acceptance 없음 |
| Laguerre/Chebyshev certificate | `crates/rodas5p-core/src/polynomial_action.rs`, `laguerre_adjoint.rs` | verified symmetric nonpositive domain; signed cancellation과 admission bound 유지 |
| nonnormal exponential action | `nonnormal_certificate.rs` | Crouzeix–Palencia + numerical-range box; 범용 phi/RVJ 인증이 아님 |
| homotopy q1/q2 | `transactional_q1_q2.rs`, `outward_certificate.rs` | 원 stage target와 독립 certificate; fallback·실패·증명 비용 전체 계상 |
| fast MF driver | `rodas5p_matrix_free_fast.rs` | raw/U form; current-J solve; protected native path와 구별 |
| GCRODR repair | `gcrodr.rs::GcrodrSolveOptions` | refresh는 선택 옵션이며 driver에 자동 연결되지 않음 |

실제 Rust RVJ 모듈은 아직 없다. `research/thread_transfer_negative_controls_20261002/thread_loop3/proofs/reference_rvj.py`는 독립 연구 기준 코드이며 native production API가 아니다. 새로운 RVJ 이식은 research-gated 별도 module에서 시작하고, 후보 생성·도메인 자격·출력 bound를 분리해야 한다. 기존 StageChart가 근처에 있다는 이유만으로 그 잔차를 RVJ 흐름과 동일시하면 안 된다.

## 새 경계 결함과 이식 부채

### INTAKE-NATIVE-01 (P2) — Stage chart can report Converged for a NaN residual

`crates/rodas5p-integrators/src/stage_chart_candidate.rs`. check_shapes validates only dimensions; inf_norm folds f64::max, which ignores NaN; the convergence guard checks the resulting norm and K, not every residual entry. Identity chart, finite K and problem.q=[NaN] produce all-NaN R with computed inf_norm=0 and ChartStatus::Converged.

영향 제한: False convergence diagnostic for public research candidate API; the candidate carries no acceptance, so this does not establish false admission by the native certificate or default solver.

다음 작업: Reject nonfinite target/problem values and every nonfinite residual/JVP before convergence; add focused regression.

### INTAKE-NATIVE-02 (P2) — Chart integration advances physical state without representable time progress

`crates/rodas5p-integrators/src/chart_transport.rs`. The loop computes tau=min(h,target-state.t), calls advance, and accepts/stores point.t=state.t+tau without requiring a strict representable increase. With t0=2^53, target=t0+2 and h=max_step=1, t0+1 rounds to t0. Every accepted iteration changes x,w and global enclosure at an unchanged time; no attempt cap exists.

영향 제한: Opt-in research chart path can return physically advanced states labelled with unchanged time, or perform a very large number of iterations. No default RODAS5P path impact inferred.

다음 작업: Refuse nonprogress before advance and ensure finite times/configuration/output schedule; use bounded test x0=-1,kappa=40,eps=0,large finite tolerance,denominator_min=0.01 to limit accepted attempts.

### INTAKE-NATIVE-03 (P2) — Published GCRODR recycle repair is not connected to the fast driver

`crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs`. REV-01c review states recycle updates break M^-1 A U=C and refresh repairs 85 prior failures; GcrodrSolveOptions derives Default, leaving refresh_after_update=false; solve_gcrodr_with_workspace reaches this default, and the fast driver calls it.

영향 제한: Opt-in recycled GCRODR fast-driver use still inherits published convergence failures and wasted work. True residual acceptance remains present; no false converged solution is alleged.

다음 작업: Expose explicit safe recycle policy in the opt-in driver or select cold GMRES/GCRODR. Keep old algorithm identifier for comparison. Validate only policy wiring and charged work; do not rerun old attribution campaigns.

## 추가 proof obligation (확정 false certificate와 구분)

- INTAKE-NATIVE-O1: StageChart::jvp output is not shape-checked before residual_jacobian_action indexes it; a short Ok(Vec) from a third-party provider panics instead of returning CoreError::Dimension. Add boundary validation together with NATIVE-01 if this adapter is reused.
- INTAKE-NATIVE-O2: exp_neg_enclosure is evaluated only at mul_up(kappa,tau); because exp(-x) decreases, the exact-product upper endpoint requires the lower product endpoint. Current Taylor interval slack might cover tested inputs, but the compositional proof should pass a product interval or evaluate both directed endpoints. Do not label a false enclosure without a counterexample.
- INTAKE-NATIVE-O3: In stepped decay, mul_up(count,h) is then multiplied by possibly negative re_hi; direction reverses. Interval midpoint radius is based on half-width rather than distance to the actually rounded midpoint. Existing safety factors may dominate particular errors; repair local enclosures before strengthening claims. No historical numerical campaign rerun authorized by this observation alone.

## 구체적인 다음 이식 절차

1. offline 연구의 최종 survivor, 실패한 arm, 정확한 입력 제약을 각각 native 기능에 연결하는 integration matrix를 만든다. 과거 결과를 새 PASS로 바꾸지 않는다.
2. 모델이 제한된 native RVJ reference kernel 하나를 opt-in으로 이식한다. signed/weighted physical norm, 초기층, interval/domain refusal, output contract를 먼저 고정한다.
3. archive Python 구현과의 parity는 port fidelity, 독립 고정밀/해석 해는 numerical correctness로 별도 판정한다. 동일식의 binary64 재포장만 독립 oracle로 세지 않는다.
4. 새 source boundary 두 건은 bounded native reproduction → 최소 수정 → 해당 regression으로 닫는다. 모든 과거 chart/RVJ 캠페인을 재실행할 필요는 없다.
5. homotopy, polynomial backend와의 결합은 원 target의 독립 residual certificate를 통과한 뒤 counter 기반 비용부터 비교한다. 메서드가 달라진 경우 같은 이름의 기존 speedup을 상속하지 않는다.

## 근거

`docs/reviews/20261003_integrated_plan/{INTEGRATED_EXECUTION_STATUS,CRITICAL_REVIEW,REVIEW_DAG_STATUS}.md`, `docs/reviews/20261003_native_reaudit/REVIEW_REMAINING_KO.md`; 해당 ref의 소스·계약 tests. Harness `research/START_HERE.md`, `PROJECT_INSTRUCTIONS.md`, `coding/START_HERE.md`, `AGENTS.md`, `SCIENTIFIC_CONTRACT.md`를 읽었다. 저장소 자체에는 AGENTS.md/CLAUDE.md가 없다.

## 사전등록 이후 이식 경계 수정 완료

사전등록 remote commit `3e784982104274312027490d481de4929e4f3cf6` 이후 새로운 경계 테스트를 먼저 실행했다. 최초 RED는 8건 중 6건 실패/2건 통과이며 종료코드 101이다. NaN convergence, 잘못된 target/JVP의 panic, nonfinite JVP의 잘못된 상태 분류, nonprogress 시간 advance, infinity configuration 수락을 직접 재현했다.

`stage_chart_candidate.rs`는 사용되는 target/problem/stage의 shape·finiteness, JVP shape·finiteness, 산술 중 생긴 비유한 residual/Jacobian action을 검사한다. NaN을 inf_norm의 f64::max가 삼켜 Converged가 되는 경로를 닫았고, 실제 실행된 JVP의 카운트를 보존했다. `chart_transport.rs`는 유한 time/config/output을 요구하고 표현 가능한 시간이 전진하지 않으면 물리 상태·enclosure를 바꾸기 전에 refusal을 돌려준다. 이 시간 수정은 stagnation 경계를 닫는 범위이며 임의의 floating timestamp에 대한 완전한 exact-time theorem을 새로 주장하지 않는다.

최소 수정 뒤 새 테스트 8건과 기존 stage contract 3건/transport contract 5건이 모두 통과했다. 입력 검증만으로 잡을 수 없는 finite coefficient의 overflow→NaN residual을 별도 추가하여 최종 새 테스트 9건 모두 통과했다. 원래 published chart/RVJ research campaign은 재실행하지 않았다. 로그는 `research/rvj_integration_20261004/evidence/chart_boundaries_{red,green,final}.log`; 실제 명령과 exit는 `CHART_COMMANDS.jsonl`에 기록했다. 최종 합본 fmt/clippy 및 독립 판단은 owner가 수행한다.

GCRODR 연결 부채와 exponential-product/midpoint proof obligation은 소스 변경 없이 남겼다. 게시된 GCRODR 수렴 실패를 새 실험으로 다시 증명하지 않았다.
