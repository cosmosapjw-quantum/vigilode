# VigilODE PR128: 적대적 블라인드 방식 C9 기술 심사

**작성일:** 2026-10-30 KST. **심사 대상:** `cosmosapjw-quantum/vigilode`, Draft PR128, immutable subject head `b06cd9b7bf9ad9b103d43e89ca83ea31a71505a3`. **심사 성격:** 같은 ChatGPT assistant가 수행한 *source-first blind-style hostile review*. 실제 제3의 인간/별도 subagent의 독립 심사가 아니며, reviewer identity를 속이지 않는다. 별도 저자 C9 0/1 NOT_RUN. 심사 대상 PR128은 수정하지 않았다.

## 판정: REQUEST_CHANGES for any physical / production admission; scoped research PASS remains intact

새 검증 경로는 동일한 **선택된 owner**와 그 자체의 opaque trial이 일치함을 검사한다. 하지만 그 owner가 실제로 적분 중인 `OdeProblem` RHS/Jacobian callback에 결합된 owner인지 검사하지 않는다. Native factory의 유효한 중간 stage probe는 현행 epoch 정의로는 시작 trial을 무효화한다. 두 사실은 수학적으로 엄밀한 Neumann·causal stage error inequality와 무관한 provenance/시스템 통합 문제다. PR128 자체가 test-only/prototype 범위를 명시한다면 그 제한적 결과는 계속 유효하다. 생산 코드 및 전체 사용자 physics model으로의 승격만 허용하지 않는다.

### H1. [MAJOR for source-authenticated physical admission] Owner substitution

`crates/vigilode-coupled/tests/n01_df_c8_work_precision.rs::source_authenticated_raw_u_bound`는 자유롭게 넘겨받는 `source: &N01MassActionOwner`의 `check_raw_u_stage_trial()`과 `value_only_stage_inputs()`를 사용한다. 실제 `OdeProblem`에 대한 신뢰된 callback-owner provenance, `Arc::ptr_eq` 또는 sealed problem identity를 받지 않는다. `crates/rodas5p-core/src/n01_mass_action_owner.rs::SourceStamp`의 owner/epoch/clock/state bit identity는 호출자가 전달한 다른 owner 내부에서만 유효하다.

적대적 TEST (`blind_c9_owner_substitution_still_certifies_a_different_problem`)은 실제 `n01_mass_action_owned_problem(k=(1/2,10,200))`에서 `eval_rhs_into()`를 호출하고, 독립적으로 만든 fake owner `k=(0,0,0)`+fake owner가 발행한 정상 trial을 검증 함수로 넘긴다. fake residual0, `Some(0.0)`와 `PROVED_CONDITIONAL_TUBE`가 반환되는 동안 실제 native RHS는 정확 dyadic `(-7/32,-93/32,25/8)`로 비영이므로 동일한 8개 영 stage는 실제 방정식의 첫 raw-U stage가 아니다. 독립 Python Fraction seed20261158에서 1024/1024 양수 물리율 계통에서 fake0 / real residual 비영·질량중립을 확인했다. 정확 tau=1/64 예에서 실제 original residual `(7/2048,93/2048,-25/512)`, infinity norm25/512; Wolfram CAS 독립 대조. **이는 wrong-source certification이지 Neumann bound 수학적 반례가 아니다.**

GitHub-hosted Rust에서 `h=1e-4`, 원래 RODAS gamma일 때 actual native first raw-U zero-stage residual ℓ∞=`6.62304884982156818e-5`, fake bound0을 기록했다. 과거 F01의 `1.32460976996431364e-4`은 세 성분을 더하는 **ℓ¹ norm**이므로 여기의 infinity norm과 정확히 두 배 차이이고 모순이 아니다.

**최소 수정안:** 인증에 필요한 `OdeProblem`, source owner handle, initial `(t0,y0)`, `h`, frozen 8-stage tableau identity, physical units/weights를 하나의 non-user-forgeable step context로 소유시켜라. `source_authenticated_raw_u_bound`가 caller-constructible 임의 owner를 받는 API는 production-admission authority로 쓰지 말 것. `n01_mass_action_owned_problem`의 동일한 source Arc 핸들과 연결된 typed sealed handle 또는 trusted factory가 필요하다. 콜백을 한 점에서만 재계산해 같은 값을 얻는 것만으로 모든 stage source 동일성을 증명할 수 없다.

### H2. [MAJOR for native integration] Transient stage query increments model_epoch

`n01_mass_action_owned_problem()`의 RHS/Jac/JVP callback은 `guard.advance(t,state)`를 호출한다. `advance()`는 물리 모델의 rates가 고정되어 있어도 평가 시각 또는 상태가 달라지면 `model_epoch`을 증가시킨다. 새로운 native TEST (`blind_c9_actual_odeproblem_probes_invalidate_frozen_step_trial`)에서 같은 상태 `y`, 다른 평가 시간 `t0=0`과 `h/2`만으로 epoch `0 -> 1`이 발생하고 pre-step frozen trial이 stale 처리되는 것을 확인했다. 이것은 현재 코드의 의도된 보수적 정책을 정확히 반영한다. 그래서 PR128의 positive comparator에서 epoch가 고정된 `ref_problem` 사용을 **실제 factory/adapter 통합 검증**으로 승격해서는 안 된다.

**최소 수정안:** 진짜 물리 모델 generation (`rates`, closure, table, data version)이 변경된 경우에만 증가하는 `model_epoch`, 그리고 temporary stage eval clock/state identity를 별도 typed snapshot으로 관리한다. 불변 source step session을 만들고 callback들은 stage eval 시 epoch를 변형하지 않으며, abort/accept에서만 실제 interior 모델 snapshot 갱신. 허용되는 동시성 경계·단일 owner identity를 명시한다. 이 단계는 C9 findings에 대한 **수정 제안**이지 본 심사에서 구현/검증 완료한 사항이 아니다.

### H3. [MODERATE claim-boundary] Algebraic certificate is not an end-to-end admission decision

연구용 `certified_raw_stage_bound`의 반환은 `(residual_upper, Option<algebraic_error_upper>, status)`이며 기본 물리 허용오차·전역 time truncation·이벤트·step-size consistency에 대한 최종 output commit gate가 아니다. `PROVED_CONDITIONAL_TUBE`는 주어진 represented coefficients/source와 positivity tube 전제하의 **이산 stage algebraic inequality**일 뿐이다. Source owner 쪽 trial guard도 h와 exact native `OdeProblem` callback provenance를 결합하지 않는다. 이를 `production physically accepted step`과 동일시해서는 안 된다.

**긍정적 결과:** 현재의 올바른 source에 대한 original raw-U residual, 조건부 Neumann κ, strict causal finite transfer 및 endpoint algebraic bound를 별개 수학적 대상으로 받아들일 수 있다. PR128 검증된 source-matched 8-stage direct reference의 small result는 계속 유효하지만, full BASS physical33/autonomous 이외 source나 전체 IVP/속도 주장으로 일반화할 근거는 없다. 2009년 Ortner의 a posteriori 존재성 논의 및 verified Runge–Kutta local truncation literature도 정확한 residual과 ODE global error가 다른 문제임을 뒷받침한다.

### H4. [BACKLOG] Test-only fallible interval arithmetic unwraps

C8 연구용 `interval_jac`/residual interval chain에 남은 `.unwrap()`은 유효하지 않은 raw 입력에서 typed refusal 대신 panic이 날 가능성을 가진다. 이번 심사의 유효한 source-bound native 시나리오에서 해당 panic을 재현하지 않았으므로 production blocker라고 새로 부풀리지 않는다. 함수가 production API로 노출될 때 `?`와 명시적 refusal로 바꿀 것.

## 실행 증거 및 심사 독립성

현재 turn Python3.13.5 실제 실행 확인 `PYTHON_PASS`; `independent_exact_reference.py` 1024/1024 exact rational PASS. Rust local compiler availability was not used for results; native work delegated to real GitHub-hosted Rust1.94.1 source checkout. GUI/connector subagent creation tool was absent; **native_subagent NOT_RUN**. The test author and this report writer are the SAME assistant; do not mark formal C9 different-author `1/1`.

Native tested source head and job IDs are in `REVIEW_VERDICT.json`. Source-target PR128 head must remain `b06cd9b7bf9ad9b103d43e89ca83ea31a71505a3`; hostile test-only diff is published on separately isolated reviewer-style branch, not on PR128. Preserved initial native 46/46 PASS and Clippy PASS with rustfmt failure as `NATIVE_RED.json`, followed by purely formatting changes and rerun.

## Correct next action

1. Separate actual human or distinct hosted agent independently reviews exact PR128 and this standalone negative-test PR, records real author identity + source SHA + commands/exits + conclusions.
2. Construct a trusted native `OdeProblem` owner/step-context bridge, with independent model generation vs stage clock/probe state and atomic acceptance; repeat H1/H2 negatives. Never erase negative examples or move source mismatch into generic JVP-first fallback.
3. Preserve C6/C7/C8 scoped evidence; C9 independent-author `0/1 NOT_RUN`, C10 NOT_STARTED, production HOLD. Official VigilODE 7/23; REI 28/88; successor admitted 0/12. No main merge, force push, tolerance relaxation, or wall-speed claims.

## Primary outside methodology references

- C. Ortner, *A Posteriori Existence in Numerical Computations*, SIAM J. Numer. Anal. 47 (2009), DOI 10.1137/060668183 (nonlinear existence/stability/residual assumptions).
- J. Alexandre dit Sandretto and A. Chapoutot, *Validated explicit and implicit Runge-Kutta methods* (2015), DOI/publication provenance to be checked before citing full technical details; highlighted here for local truncation versus stage residual distinction, not as an execution receipt.
