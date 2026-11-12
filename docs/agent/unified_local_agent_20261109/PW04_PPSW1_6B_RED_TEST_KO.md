# PW04 PPSW1-6b RED 테스트 (opt-in, Schmitt-Weiner SINUM 2004, s=6, gamma=0.619411975066...)

**상태: RED 확정, oracle GREEN 확인 (27 passed, 0 failed, 1 ignored).** 테스트 파일 1개만 작성했다: `crates/rodas5p-integrators/tests/ppsw1_6b_contracts.rs` (sha256 `291a9247ec801f13cdf5c6f81741085948a05b05ecdb140361dc0b55e6927ba6`, 1745줄). src 파일/기존 파일/Cargo 파일은 건드리지 않았다. 기계 판독본: `PW04_PPSW1_6B_RED_TEST.json`.

## 1. 근거
계수와 사실은 `PW04_PPSW1_6BC_VERIFICATION_KO.md/.json`(두 경로 약 87자리 일치)과 SINUM 논문에서 가져왔다. 패턴은 PW02(`ptsw2b_contracts.rs`)를 따랐다: 구현과 독립된 정확 산술 oracle, ptsw2b식 Session/work-counter 규약, protected sha256 guard, catalog 불변. 시작기와 오차 추정기는 설계 전이므로 수치를 고정하지 않았다(`#[ignore]` 플레이스홀더에 UNRESOLVED로만 기록).

## 2. 제안 API (`rodas5p_integrators::ppsw1`, GREEN 단계 = 신규 `src/ppsw1.rs` + `lib.rs`에 `pub mod ppsw1;` 한 줄)
- 상수: `METHOD_ID`("ppsw1"과 "6b" 포함, ptsw/mipeer/ppsw2/rodas/6c 불포함), `STAGES=6`, `FORMAL_ORDER=5`(변보폭 정확 차수), `ACCEPTED_STEP_SIGMA_MAX=1.10`(소유자 규칙), `PAPER_STEP_RATIO_BOUND=1.5`(논문 값, 별도 기록).
- 계수: `gamma() -> f64`, `nodes() -> [f64;6]`, `tableau(sigma) -> CoreResult<Tableau>`; `Tableau{gamma, nodes, b_matrix(B), a_matrix(A), gamma_matrix(Gamma=-gamma*Theta), beta(=A+Gamma)}`, 모두 `[[f64;6];6]`. 유한 양수 sigma이면 항상 성공(1.5 포함, cap은 step 단계에서 적용), 아니면 `CoreError::InvalidInput`.
- 단계식 (PW04 s05의 단계형 (3)/(7)): `(I - gamma h T) Y_i = explicit_i + h T transfer_i`, `explicit_i = sum_j B_ij Y_prev_j + h sum_j A_ij F_prev_j`, `transfer_i = sum_j Gamma_ij Y_prev_j`, `T = J(t_m, u_m)`, `u_m = Y_prev_5`.
- 타입: `State{t, y:[Vec<f64>;6], f:[Vec<f64>;6], h_prev}`(Clone, Debug, PartialEq), `StageInput{t, explicit, transfer}`, `StageSolve{y:[Vec;6]}`, `StepOutput{next:State}`, `Config{step_ratio_cap}`(Default=1.10), `ControllerState{accepted_ratios}`, `Attempt`(`next_state()`), `Session`, `Identity`, `DerivativeClass::ExactJacobianOrJvpFrozenAtStepStart`.
- 함수: `identity()`, `stage_inputs(&State, h) -> CoreResult<[StageInput;6]>`, `solve_stages(&problem, &State, h, &inputs, &exec, &mut counters) -> CoreResult<StageSolve>`, `advance(&problem, &State, h, &exec, &mut counters) -> CoreResult<StepOutput>`.
- `Session::{new(problem, state, Config, exec), state(), controller(), counters(), attempt(h) -> CoreResult<Attempt>, accept(Attempt) -> CoreResult<()>, reject(Attempt) -> CoreResult<()>}`. attempt는 시도 작업을 즉시 charge하되 아무것도 commit하지 않는다. 수락/기각 결정은 호출자(미설계 estimator) 몫이므로 estimator 수치를 고정하지 않고도 lifecycle을 시험할 수 있다. 오래된 Attempt(다른 commit 이후)는 `InvalidInput`.
- `Identity` 필드: method_id, order, stages, accepted_step_sigma_max, paper_step_ratio_bound, derivative_class, default_routed, reuses_rodas5p_{starter,estimator,controller}, claims_{shared_krylov_recurrence,g0_qualified,derivative_free}(모두 false).
- sigma > 1.10 정책(선택, 문서화): clamp하지 않고 `CoreError::InvalidInput`로 거부(panic 없음, work charge 없음, 상태 불변). 정확히 1.10은 허용. `Config.step_ratio_cap`은 1.10 이하로만 조일 수 있고 1.10 초과(예: 1.5)는 `Session::new`가 거부.
- charge 규약(sequential.rs): solve_stages = jacobian_builds+1(또는 JVP면 jvp_calls+n), direct_factorizations+1, linear_solves+6, direct_solve_calls+6, rhs_calls+0; advance는 거기에 새 단계 f 평가 rhs_calls+6.

## 3. 테스트 매트릭스 (27개 + ignore 1개)
| 테스트 | 고정하는 사실 | 근거 |
|---|---|---|
| a1 | identity 구별(ptsw/mipeer/ppsw2/rodas/6c 아님), opt-in, s=6, 차수 5, 자체 시작기/estimator/controller | 소유자 지시, PW04 2절 |
| a2 | derivative class 상수 | 소유자 승인 |
| a3 | 상한 1.10과 논문 1.5가 별개 상수 | 소유자 규칙, PW04 3절 |
| b1 | 정확 유리수 oracle: B*1=1, Gamma(k) 및 AB(k) k=0..5는 gamma^0/gamma^1 부분 모두 정확히 0, k=6은 0 아님, beta^6=0 이고 beta^5!=0 (sigma 0.5,0.8,1,1.05,1.1) | PW04 1-2절 (노드행 V), 논문 (14)-(16),(21),(23) |
| b2 | gamma는 인쇄된 6차 다항식(Table 1, s=6)의 근: 60자리 문자열에 대해 P(N-1)<0<P(N+1)을 큰 정수로 정확 판정, PW04 JSON 40자리 접두사 일치, 인쇄값 0.619411975066과 3e-13 이내, 모듈 gamma 1e-15 | PW04 1절 |
| b3 | 모듈 tableau(sigma)가 double-double oracle와 1e-13*(1+|x|) 이내 일치 (sigma 0.5,0.8,1.0,1.05,1.1; B,A,Gamma,beta,nodes, A+Gamma=beta) | PW04 1절 |
| b4 | oracle과 모듈이 PW04 JSON의 sigma=1 B, A 24자리 인쇄값과 일치 | PW04 JSON |
| b5 | tableau는 sigma>0 유한이면 성공(1.5 포함), 아니면 typed error | API 선택 |
| c1 | 모듈 계수로 AB(6)/Gamma(6) (k=0..5) 잔차 <= 1e-12*(항 크기 합), sigma 0.5,0.8,1.0,1.1 | PW04 2절 |
| c2 | k=6 잔차 최대값 > 1 (정확 차수 5; PW04 범위 30-153) | PW04 2절 |
| c3 | 정확한 단계 데이터로 y'=-y 1스텝 오차 지수 5.5-6.6 (국소 h^6), sigma 0.8, 1.1 | 변보폭 정확 차수 5 |
| d1 | stage_inputs는 이전 step 데이터만 사용, explicit/transfer가 oracle 식과 일치 | 단계식 |
| d2 | 한 stage 입력 섭동이 다른 5개 현재 stage 출력을 비트 단위로 바꾸지 않고 자기 stage는 바꿈 (6개 victim, 2개 필드) | stage 독립 |
| d3 | 6 solve가 인수분해 1회 공유, charge 1/6/6/0, (I-h gamma T)Y=explicit+hT transfer 잔차 | PW04 5절 |
| d4 | advance charge 1 fact, 6 solve, 6 rhs, 새 history의 f=f(t+c_i h, y_i) | PW04 5절 |
| d5 | 직렬 vs rayon(2) stage solve와 전체 step이 비트 동일, 작업 ledger 동일 | PW02 d4 패턴 |
| d6 | JVP-only 문제 == 명시 Jacobian 문제, jvp_calls=n 이 charge됨 | PW02 d5 패턴 |
| e1 | 기각: history/controller 비트 불변, accepted 불변, rejected+1, 실패 작업 charge 유지, 이후 사용 가능 | PW02 승인 의미 |
| e2 | 수락: 독립 재계산(oracle 계수+2x2 Cramer)과 일치, commit==제안 상태, controller에 h/h_prev 기록 | |
| e3 | stale Attempt 거부, 부작용 없음 | API 선택 |
| e4 | sigma 1.10 허용, 1.1000001/1.2/1.5/3.0은 typed error, work 0, 상태 불변; Config cap>1.10 거부, 1.05 cap 동작 | 소유자 규칙 |
| f1 | 모듈 beta^5!=0(크기 sigma^10 기준), beta^6=0 (rho=0) | 정리 3.1, PW04 3절 |
| f2 | z=-1e10에서 M(z)=(I-gamma z)^-1(B+z beta) -> -beta/gamma, (-beta/gamma)^6=0 | PW04 3절 |
| f3 | A(alpha) 각도 수치 스캔(0.02도 간격, 복소 특성다항식 + Aberth)이 57.54도 +-0.05, 45도 안정 | PW04 3절 (57.54, 인쇄 57.5) |
| g1 | 기본 catalog 불변, PeerW Deferred 유지, default_routed=false | PW02 h1 |
| g2 | derivative-free 라벨 없음 | 소유자 지시 |
| g3 | protected sha256 14개(PW02 h2의 12개 + ptsw2b.rs + ptsw2b_contracts.rs); lib.rs는 `pub mod ppsw1;\n` 한 줄을 제거하면 현재 sha256과 일치해야 함(최대 1줄) | 소유자 allowlist |
| unresolved_items_are_listed_not_tested (#[ignore]) | 시작기, estimator, 강경직 전역 차수, W 허용 구간, Vandermonde 규약 | PW04 UNRESOLVED |

## 4. RED 영수증
- 명령: `CARGO_TARGET_DIR=/mnt/extdrive/vigilode-t02-20261110/target cargo test --locked --release -p rodas5p-integrators --test ppsw1_6b_contracts`
- exit 101, stdout sha256 `e3b0c442...b855`(빈 출력), stderr sha256 `5fb246dd85de5e96b8fcc5a490927ef1217c87a418adc2445933ba8641a0538b`. 유일한 에러: `E0432 unresolved imports rodas5p_integrators::ppsw1` (`use rodas5p_integrators::ppsw1::{self, ...}`, 29행). 판정: **PASS-as-RED**.
- 과정 중 첫 RED들은 E0282 "type annotations needed" 2건이 함께 있었다(미해결 import로 타입을 모르는 값에 대한 추론; 테스트 버그). 타입 주석/구조만 바꿔 제거했으며 assertion은 바꾸지 않았다. 최종 파일로 재실행해 위 결과를 얻었다.
- rustfmt `--edition 2024 --check`: exit 0.

## 5. Oracle GREEN 확인 (worktree 밖, 구현 미전달)
scratch `/tmp/claude-1000/-home-cosmosapjw-vigilode/d3cab3fe-917d-4d13-8122-642eb1b07ae5/scratchpad/oracle6b/`(fixtures 포함 복사, target `/mnt/extdrive/vigilode-t02-20261110/target-oracle`)에 폐기용 `src/ppsw1.rs`(double-double Gauss-Jordan으로 tableau, 위 API)와 `pub mod ppsw1;`를 둔 사본에서 실행: **27 passed; 0 failed; 1 ignored**, exit 0, 총 0.5초. f3 각도 스캔 포함 전부 통과.
- 1차 실행 3건 실패는 모두 TEST 버그였고 assertion 완화는 없다: (i) d1이 sigma=1.25(cap 초과)를 사용 -> 입력을 sigma=1.05로 수정; (ii) f1의 beta^5 하한 1e-3이 sigma^10 스케일을 무시 -> `1e-4*sigma^10`으로 수정(beta^5=sigma^10/120 * V e6 e1^T V^-1이므로 스케일 보정이지 완화가 아님); (iii) b1의 i128 overflow -> Rat 덧셈/곱셈을 최소공배수/교차약분 형태로 바꿈(결과는 동일, 검사는 그대로).
- clippy(`--no-deps --release --test ppsw1_6b_contracts`, -D 없음): 이 테스트 파일 진단 0건(처음에 `suspicious_arithmetic_impl` 1건 -> Div 구현을 직접 정수 연산으로 수정). worktree에서는 RED라 clippy가 컴파일되지 않으므로 oracle 사본에서만 실행했다. `--all-targets`는 scratch에 research 픽스처가 없어 무관한 다른 테스트에서 실패하므로 사용하지 않았다.
- 이 oracle은 계약 테스트의 일관성만 증명한다(구현 증거 아님).

## 6. GREEN 구현자 참고
- 1e-13 계수 일치는 f64 Vandermonde 역행렬로는 어렵다(조건수 약 1e4). 정확 유리수(분모 작음) 또는 double-double 계산 후 f64로 반올림하는 방식이 필요하다. 모든 계수는 gamma에 대해 아핀: B=B0+gamma B1, A=beta+gamma Theta, Gamma=-gamma Theta (B0,B1,beta,Theta는 sigma의 유리 함수).
- 테스트 경로: tests는 `State.y[5]`를 u_m으로 보고 `T=J(t, y[5])` 확인.

## 7. UNRESOLVED / NOT_RUN
1. 시작기(Y_0는 t0 이전 점 c_1=-1을 요구, 논문은 시작 절차 없음, 자체 시작기 필요, RODAS 금지).
2. 오차 추정기(논문은 산문만; 계수 없음) 및 수락/기각 규칙, 변보폭 controller.
3. 강경직 전역 차수(PR lambda=-1000, LIN3 NOT_CONCLUSIVE). 상수 보폭 6차 초수렴은 정수 보폭 한정 논문 주장이며 테스트하지 않음.
4. W 허용 구간 0.8-1.0 NOT_RUN (T=cJ: c=1.0, 1.2 수렴, 0.8 발산).
5. Vandermonde 색인 규약: 인쇄본(c_j^(i-1))은 B*1=1과 모순, 노드행 V_ij=c_i^(j-1) 채택(b1에서 B*1=1 정확 확인).
6. 출력 stage=Y_m^s(c_s=1) 가정. t_end<t1 처리, event, matrix-free, mass matrix(모듈은 identity mass만 가정; 시험 안 함), 실제 문제 벤치마크, vigilode 기본 라우팅 NOT_RUN.
7. 작업 중 읽기 전용 `git status`를 한 번 잘못 실행했다(출력 없음, 아무것도 변경 안 함).
