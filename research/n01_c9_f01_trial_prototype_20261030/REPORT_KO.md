# C9 F01 원래 raw-U source trial 결합: 제한적 구현·검증 결과 (2026-10-30 KST)

## 실제 대상과 변경
- 원래 프로젝트 목표: **causal, parallelized, RHS-only derivative-free-first Rosenbrock/Runge–Kutta augmented by accelerated Newton-free homotopy**. JVP는 조건부 보조, dense 3×3은 독립 작은 기준용. PR91 경로·기본 dispatcher·RODAS5P tableau/허용오차 **미변경**.
- 고정 출처: cosmosapjw-quantum/vigilode PR126, `2688a59e0b8ab88120105a6949db68d61a378e71`. C9 F01 독립 branch `research/n01-c9-f01-source-trial-prototype-20261030`, Draft PR128.
- 변경 1 (opt-in): `N01MassActionOwner::check_raw_u_stage_trial(trial,time,carrier)`는 원래 opaque trial의 owner·epoch·clock·state stamp를 검사하고, caller clock/carrier의 **binary64 bit identity**를 실제 owner snapshot에 대조한다. read-only; `advance()` 없음.
- 변경 2 (research test-only): `source_authenticated_raw_u_bound`는 owner/trial 검사를 거쳐 `value_only_stage_inputs(trial)`의 실제 rate·carrier로 기존 `certified_raw_stage_bound`를 호출하고 마지막에 trial을 재검증한다. 불변 객체 자체를 소유하지 않는 임의 `OdeProblem`을 증명한 것은 아니다. 원래 자유 입력 C8 함수는 **음성 대조군으로 그대로 유지**.
- 변경 3: forgery, 다른 owner, stale epoch, ±0.0 clock, 다른 carrier, NaN/Inf, stage shape refusal 및 실제 native RODAS5P 8-stage 양성 대조군을 검증.

## 원래 방정식에 대한 반례와 양성 수치 결과
자율 Robertson (f=(-k_1a+k_2bc, k_1a-k_2bc-k_3b^2, k_3b^2)), identity mass, (	au=hgamma).
원래 첫 raw-U stage (r_0=(I-	au J_0)U_0-	au f(y)). For (U_0=0), 실제 source에 대한 잔차는 (-	au f(y))이고, (k_3b^2>0,	au>0)이면 세 번째 성분은 정확히 (-	au k_3b^2<0). 따라서 `k=0`으로 바꾼 가짜 source에 대한 (r_0=0)은 실제 source에 대한 인증이 아니다.
실제 C9 F01 `h=0.0001`, (y=(.75,.125,.125)), (k=(.5,10,200)):
- 잘못된 외부 (k=0), 여덟 영 stage: fake (r=0) 및 bound Some(0).
- 같은 후보를 true owner trial에 대조: original residual `1.32460976996431364e-4`, **POSITIVE_TUBE_REFUSED**, output bound None. **거절이 올바른 결과**.
- 동일 true source로 원래 RODAS5P 8-stage 3×3 direct 작은 reference 계산: 원래 raw-U full residual `5.73949525059513910e-18`, conditional discrete algebraic endpoint upper `3.0599993684558566e-15`, **PROVED_CONDITIONAL_TUBE**, 8회 reference linear solves. 3×3 direct는 production derivative-free 적분기가 아님.
- Wolfram 별도 기호/유리수 검사: `y=(3/4,1/8,1/8)`, `k=(1/2,10,200)`, (	au=1/64)에서 가짜 원잔차 0, 실제 ((7/2048,93/2048,-25/512)), 질량합 0.

## 실제 컴파일/검산
[GitHub hosted run 40989982863](https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40989982863), job `123849770277`, tested source HEAD `1b98ec58353e80e553d03df24d2afaf12aeaef39`, Rust 1.94.1 on Ubuntu 24.04:
- `cargo test --locked --offline -p vigilode-coupled --test n01_df_c8_work_precision -- --nocapture` **6/6 PASS** (original C8 2, original C9 wrong-source 1, F01 new 3).
- `cargo test --locked --offline -p vigilode-coupled --test n01_df_value_stage_contract --test n01_df_c7_original_raw_u_contract` **9/9 PASS**.
- `cargo test --locked --offline -p vigilode-coupled --test rhs_homotopy_contracts --test causal_contracts --test causal_reuse_contracts` **29/29 PASS**.
- `cargo clippy --locked --offline -p rodas5p-core -p vigilode-coupled --test n01_df_c8_work_precision -- -D warnings` **PASS**, source-only `rustfmt --check` core+test **PASS**.
- 합계 **44/44 실제 선택 native tests PASS**. 초기 RED [40989229211](https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40989229211): 물리적으로 invalid 영 stage를 긍정 인증할 것이라는 잘못된 테스트 기대. 두 번째 RED [40989578203](https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40989578203): 과학 44/44 + strict Clippy PASS, test-only rustfmt 불일치. 두 실패를 로그/수령증으로 보존하고 각각 진단 후 교정; 허용오차/모델 변경 없음.

## 다음 미해결 항목과 판정
- 새 `N01MassActionOwner` checker는 **한 실제 owner/trial**을 잠그지만 임의 `OdeProblem` 콜백이 이 owner를 실제 사용했다는 증거, 원래 RODAS5P 단계별 time/state clock 동기화, `n01_mass_action_owned_problem`의 ephemeral `advance(t,y)`가 생성하는 epoch churn, 실제 BASS physical33 네트워크까지 증명하지 않는다. 따라서 F01 **일반 source physical admission BLOCK_NOW remains**.
- 원래 C8의 source-matched 수치결과 1/1 scoped 및 C9의 부정반례 유지. 새 F01 **owner-wrapper research prototype 1/1 native scoped**는 C9 동일 assistant 보완 실험이지 **독립 저자 C9 심사는 0/1 NOT_RUN**, C10 미착수, production HOLD.
- 공식 DAG VigilODE **7/23**, REI **28/88**, successor **0/12 admitted**, C6/C7/C8 각 1/1 scoped.
- Native 별도 subagent 도구 없음: `native_subagent=NOT_RUN`. GitHub reviewer 기록 없음. 사용자 요청한 실질적인 독립 심사자 작업이 수행되기 전까지 C9 공식 노드를 닫지 않는다.
