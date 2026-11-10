# PW01 / PW02 사전 구현 준비도 감사 (PTSW2C order-3, PTSW2B L-stable order-2)

**작성일:** 2026-11-10 · **모드:** DOCUMENTATION/REPLAY ONLY (Rust/Cargo/git 미실행, 기존 파일 미수정)
**작업트리:** `/media/cosmosapjw/5CAA5033AA500BC0/vigilode-unified-t02-20261110`
**증거 규칙:** 실행한 것만 PASS. 실행하지 않은 것은 NOT_RUN, 미확정은 UNRESOLVED.

## 1. 결론 요약

| 항목 | 판정 | 증거 등급 |
|---|---|---|
| (a) PTSW2C 차수 3 | PASS (정확 유리수) | EXACT: C(2),Γ(2),B(3)=0, B(4)≠0 |
| (a) PTSW2B 차수 2 | PASS (정확 대수) | EXACT: C(2),Γ(2),B(2)=0, B(3)≠0 |
| (b) PTSW2B M(∞) 멱영(index 3), ρ=0 | PASS | EXACT |
| (b) PTSW2B L(90) 섹터 | UNRESOLVED (source-reported) | 수치 샘플만, 증명 아님 |
| (c) PTSW2C A(α) 각도 ≈ 81.17° | PASS_NUMERIC_CORROBORATION_ONLY | 부동소수 + 40자리 정밀 refine, 구간증명 아님 |
| (c) PTSW2C NOT L-stable | PASS | EXACT: det M∞=667/2592 ≠ 0, ρ∞=0.67958 |
| (d) 현재 step stage 독립성 | PASS_STRUCTURAL | 식 구조(Jacobian 비대각 0) |
| PW01 G1 준비도 | **NOT_READY_FOR_G1** | 아래 UNRESOLVED 항목 |
| PW02 G1 준비도 | **CONDITIONALLY_READY_FOR_RED_TESTS** | 아래 UNRESOLVED 항목 |

준비도는 문서/산술에 한정된다. Rust RED test는 하나도 실행되지 않았다. 자격(qualification) 주장이 아니다.

## 2. 명령 영수증 (R1-R4)

| id | 명령 | exit | stdout sha256 | stderr sha256 |
|---|---|---|---|---|
| R1_verify_ptsw2b | `python3 -I verify_ptsw2b.py` (cwd `/media/cosmosapjw/5CAA5033AA500BC0/vigilode-unified-t02-20261110/docs/agent/solver_consolidation_20261108`) | 0 | `80c254377c533433ec36a2b25b0660a2598daa043208e166a8dc0fd602ea9943` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| R2_verify_ptsw2c_stability | `python3 -I verify_ptsw2c_stability.py` (cwd `/media/cosmosapjw/5CAA5033AA500BC0/vigilode-unified-t02-20261110/docs/agent/solver_consolidation_20261108`) | 0 | `ffb6d42adfccefc794ef87ef2364251c5a80e2150a9f1ce41aa3ad748f32dced` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| R3_pw_check | `python3 -I <scratchpad>/pw_check.py` (cwd `/`) | 0 | `f5ff2b99a274cc0202fa70d87c6f57ed4e28698855598ddf6f09935a215060af` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| R4_pw_check2 | `python3 -I <scratchpad>/pw_check2.py` (cwd `/`) | 0 | `fb3f84a59135671cca34987c46b9f6cd288f4866b786d2a9ea4c023859162fd0` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

- stderr sha256 `e3b0c442...b855`는 빈 문자열의 해시이다(stderr 0 바이트).
- 환경: Python 3.12.3, sympy 1.12, numpy 1.26.4, scipy 1.11.4. 스크립트 sha256은 JSON `receipts[].script_sha256`.
- 정적 확인: `verify_ptsw2b.py`는 sympy만, 파일/네트워크 I/O 없음, 실행함. `verify_ptsw2c_stability.py`는 numpy+scipy+sympy를 요구하며(지시의 stdlib/sympy-only 가정과 다름, D7) 출력만 하고 파일/네트워크 I/O 없음을 읽어 확인한 뒤 실행함. BLOCKED 아님.
- R1 출력: 6줄 모두 PASS/LIMIT(체크 완료). R2 출력: 7개 PASS + LIMITATION. 원문은 JSON `receipts[].stdout_text`.
- 기존 certificate의 수치(81.16974941°, ρ∞=0.6795805635, 82° 증인 ρ=1.0105069736, 샘플 최대 0.999999845290)는 R2에서 그대로 재현되었다.

## 3. 독립 검사 (R3, R4) — 사용한 조건

계수는 markdown certificate에서 직접 옮겼다(verify 스크립트에서 가져오지 않음). 계수는 수정하지 않았다.
변수 step 비 σ=h_m/h_(m-1), 형식 Y_i=u+hΣa_ij k'_j, (I-hγT)k_i=f(Y_i)+hTΣΓ_ij k'_j, u⁺=u+h(b·k+v·k'):

- C(q): Σ_j a_ij (c_j−1)^(ℓ−1) = σ^(ℓ−1) c_i^ℓ/ℓ
- Γ(q): Σ_j Γ_ij (c_j−1)^(ℓ−1) = −γ σ^(ℓ−1) c_i^(ℓ−1)
- B(p): Σ_j b_j σ^(ℓ−1) c_j^(ℓ−1) + Σ_j v_j (c_j−1)^(ℓ−1) = σ^(ℓ−1)/ℓ

차수 p* = min(q+1, p)는 PTSW 수렴 정리를 **인용**한 것이다(재증명 아님). 가정: 계수 유계(σ 구간 위 affine), 시작 오차 O(H^p), 시작 stage 도함수 오차 O(H^q), 매끄러운/Lipschitz RHS.

### (a) 차수
- **PTSW2C**: C(1),C(2),Γ(1),Γ(2),B(1..3) 잔차가 σ에 대해 항등적으로 0. B(4) 잔차 = −13σ³/108 ≠ 0(정확히 3차). C(3) 잔차 ≠ 0(stage order 정확히 2). 표의 A(σ),Γ(σ)는 C(2),Γ(2)로부터 유일하게 재유도한 값과 일치.
- **PTSW2B**: C(1),C(2),Γ(1),Γ(2),B(1),B(2) 항등적으로 0. B(3) 잔차 = σ(−3√3σ+7σ−30+18√3)/6 ≠ 0(정확히 2차). A,Γ 재유도 일치. v=e₂ᵀ(A+Γ)(stiff accuracy), c₂=1, b=γe₂. C₃=23/12−5√3/4 일치. RHS 변환 (2.5) W p = p − hTΣΓk' 항등식 정확 성립.

### (b) PTSW2B stiff limit (σ=1, 상수 step)
- M(∞)가 certificate (3.1)과 정확히 일치. 특성다항식 = λ³. M(∞)² ≠ 0 (certificate 값과 일치), M(∞)³ = 0 → 멱영 index 3, ρ=0.
- γ는 −γ²+3γ−3/2=0의 근, c₁,c₂ 모두 φ*(c,γ)=0.
- 좌반평면 격자(300×91점) 수치 샘플: max ρ = 1+1.6e-15 (|z|≈1.8e-5, 허수축 근처; 즉 영점안정 근 1). 이는 샘플일 뿐 **L(90) 증명이 아니다**.

### (c) PTSW2C 안정성
- 특성다항식 분자가 certificate (2)와 비례상수 1로 일치. M(∞)의 특성다항식 λ³+19/27λ²−13/54λ−667/2592 (certificate 행렬 (3)과 동일), det = 667/2592 ≠ 0 → 멱영 아님, ρ∞ = 0.6795805635 → **L-stable 아님(정확)**.
- 경계궤적: 40001점 스캔 후 40자리 황금분할 refine → α* = 81.1697494054° (문서값 81.16974941°). z*=−0.43730643+2.81499625i, ρ(M(z*))=1. 음의 실축 쪽 내부 점 ρ(M(−1))=0.452, ρ(M(−1e6))=0.67958 (<1). 81.1° 섹터 극좌표 스캔 최대 ρ=0.99999984529, 82° 증인 ρ=1.0105>1.
- 이는 부동소수 corroboration이다. 구간증명, 변수-step 곱 안정성, W 섭동(T≠J) 견고성은 UNRESOLVED.

### (d) 의존성 행렬 (한 step 내부)
현재 step 잔차 F_i = W k_i − f(Y_i) − hTΣ_jΓ_ij k'_j, Y_i = u + hΣ_j a_ij k'_j. ∂F_i/∂k_j (j≠i) ≡ 0 (두 방법 모두), 대각 = W_m = 1−hγT_m.

현재 step stage 간 의존 행렬 (행 i ← 열 j):

| | k_m,1 | k_m,2 |
|---|---|---|
| stage 1 | W (대각만) | 0 |
| stage 2 | 0 | W (대각만) |

이전 step 결합(둘 다 σ 일반값에서 밀집 2×2): stage i는 k_(m−1,1), k_(m−1,2)에 A_ij(σ), Γ_ij(σ)로 의존. 이전 step/step 시작 값에만 결합하는 것: u_m, k_(m−1,j), σ_m, h_m, 그리고 step m에서 선언한 T_m(stage 값과 무관, 두 stage가 공유).
- 출력: PTSW2C u_(m+1)=u_m+h(b·k_m), v=0. PTSW2B는 b=(0,γ), v(σ)≠0이므로 k_(m,1)은 u_(m+1)에 직접 들어가지 않고 **다음 step의 이력**으로만 영향.
- 계수가 특정 σ에서 0이 되는 항(PTSW2C: A₁₁ σ=8/9, A₁₂ σ=4, Γ₁₁ σ=4/9, Γ₁₂ σ=2; PTSW2B: A₁₂ σ≈6.31, Γ₁₂ σ≈3.15)이 있어도 값으로 의존성을 제거(prune)하면 안 된다.
- step m+1은 u_(m+1)(두 stage 해 모두)이 필요하므로 시간 전진은 순차적. 병렬화는 한 step 내부 stage에 한정.

## 4. 불일치/주의 (수정하지 않음)

- **D1 [LOW]** SOURCE_LOCK_20261109.json audit_addendum TESTS blob '2ddca4c399d632540334f43964ccdffe457151cd2' has 41 hex chars (typo). Worktree git blob of crates/rodas5p-fair-ab/tests/global_error_contracts.rs is 2ddca4c399d632540334f43964ccdffe457151cd = the 40-char prefix. Lock-file defect, not tree drift. Not modified.
- **D2 [MED]** Accepted-step ceiling is stated as <=1.10x (PTSW2B certificate sec.6) vs <=1.20x (PEERW plan sec.2, original G0 restored from PR #176). UNRESOLVED which applies to PW01/PW02; do not choose silently (owner-approval class: thresholds).
- **D3 [MED]** PTSW2B certificate calls PTSW2B 'the first source-certified candidate'; PEER_W certificates sec.9 and the DAG name PTSW2C as first implementation (PW01 priority 9, PW02 priority 10). DAG order governs; tension recorded.
- **D4 [MED]** PTSW2C embedded estimator v_e(sigma) (7) is not covered by verify_ptsw2c_stability.py or verify_ptsw2b.py. This audit (R4) shows equality with the generic construction transferred from PTSW2B (v_e=([1,1+eps]S D^-1 - b_e V0 S)V1^-1, eps=1/10, b_e=0.95 b); it does NOT confirm Podhaisky eq (4.7) itself (source PDF not read) -> estimator 'EXACT_BY_TRANSFERRED_CONSTRUCTION', source confirmation UNRESOLVED.
- **D5 [MED]** PTSW2C controller hysteresis constants are not transcribed in PEER_W_TABLEAU_CERTIFICATES sec.2.3 ('additional history/hysteresis'); PTSW2B certificate sec.5 gives them (keep h when sigma*>1.05 and any of last s ratios >1; keep when sigma* in [0.95,1.05]). Whether the same rule applies to PTSW2C is UNRESOLVED -> must be transcribed from source before PW01 controller code.
- **D6 [LOW]** PTSW2B certificate sec.2.1 has garbled LaTeX (lost backslashes, e.g. 'rac', 'ag{2.5}', 'Gamma_{ij}' text); equations were reconstructed by the verified identity, not by reading the rendered formula.
- **D7 [LOW]** verify_ptsw2c_stability.py needs numpy+scipy (task assumed stdlib/sympy-only). Read statically: no file writes/network; executed. Reproducibility on a machine without scipy is not guaranteed.
- **D8 [INFO]** SOURCE_LOCK absence_check says no PTSW/peer path; a deferred stub exists: crates/rodas5p-integrators/src/candidates.rs CandidateFamily::PeerW / CandidateSpec::deferred(PeerW,'peer-w',DEFERRED_REASON) and tests/unified_candidate_contracts.rs line 32. These two files are NOT in the source lock; their worktree blobs are recorded here.
- **D9 [INFO]** PTSW2C startup needs a forward integration to t1=t0+2h0 (derivative at t0+14/9 h0); PTSW2B to t1=t0+(2sqrt3-2)h0 ~ t0+1.4641h0. Behaviour when t_end < t1, when events lie inside the startup window, and cost charging are UNRESOLVED.

소스 락 대조: git blob sha1을 Python으로 계산(git 명령 미사용). 33/34 일치, 불일치 1건은 D1(락 파일의 41자 오타)로 실제 트리와는 일치.

## 5. G1 준비도 체크리스트

### 5.1 method_identity — RESOLVED_ON_PAPER
Two separate opt-in methods. PTSW2C: s=2, gamma=12/25, c=(-1,5/9), b=(1/28,27/28), v=0, A(sigma),Gamma(sigma) per cert (3),(4), p=3,q=2, A(81.1deg) source-reported (numerically corroborated 81.1697deg), NOT L-stable (rho_inf=0.67958). PTSW2B: s=2, gamma=(3-sqrt3)/2, c=(3-2sqrt3,1), b=(0,gamma), v=(sqrt3-1)(sigma,4-sigma)/8, p=2, q=2, stiffly accurate, rho(M_inf)=0 nilpotent index 3, L(90) source-reported. Separate tableau/startup/estimator/controller/identity per method; never share coefficients; claim ceilings from DAG: PW01 ORDER3_CANDIDATE_A(81.1_DEGREE)_NOT_L_STABLE_NOT_G0_QUALIFIED, PW02 ORDER2_L_STABLE_CONTROL_NOT_G0_QUALIFIED. Neither is a G0 (order>=4.8) candidate.

### 5.2 startup_requirement — PARTIAL_UNRESOLVED
Forward-only startup per Podhaisky Alg. 4.1 (cert). PTSW2C: T={-2,-4/9,0}, t1=t0+2h0, k0,1=f(t0,u0), k0,2=f(t0+14/9 h0,u(.)), need u1 error O(H^3), k0,i error O(H^2). PTSW2B: t1=t0+(2sqrt3-2)h0, k0,1=f(t0,u0), k0,2=f(t1,u1), u1 O(H^2). h1=h0 (sigma=1). UNRESOLVED: starter choice (DOPRI5 vs implicit/stiff starter; source used RADAU/ROWMAP; DOPRI5 on stiff problems unverified), starter dense-output accuracy, t_end<t1, events in window, startup cost ledger charging, whether startup ratio sigma_1 may differ.

### 5.3 estimator_source — RESOLVED_ON_PAPER_WITH_GAPS
Own method-specific embedded estimator only (never RODAS/DOPRI embedded pair or controller; do not call sequential.rs/rodas5p_* estimator/controller). PTSW2C: b_e=(19/560,513/560)=0.95 b, v_e(sigma)=(-(8+27 sigma)/560, 9(4+3 sigma)/560) [equal to transferred generic construction: R4]. PTSW2B: b_e=(0,19 gamma/20), v_e=((8sqrt3-11)sigma/80, (11-8sqrt3)sigma/80+(19sqrt3-17)/40) [verified by verify_ptsw2b.py R1]. Err_emb=scaled RMS(ATOL+RTOL|u|); sigma*=min{2,max{0.5,0.85 Err^{-1/s}}}, s=2; accept if Err<=1. UNRESOLVED: PTSW2C hysteresis constants (D5), source confirmation of eq (4.7) (D4), exponent 1/s with s=2 for the p=3 method, asymptotic estimator-vs-achieved-error test. Estimator is a step-control indicator, not a global-error enclosure.

### 5.4 history_rollback — DESIGN_ONLY_NOT_RUN
Committed state = {u_m, K_{m-1} (2 x n peer derivatives), h_{m-1}, last-s ratio history, controller state, (model,provider,norm,tolerance) epochs, T_m/W-factor generation}. Stages compute into scratch; commit (u_{m+1},K_m,h_m,ratio history) atomically only on accept. Reject: discard all scratch incl. stage solves, factor/preconditioner generation for the old h, restore committed state bit-exactly, recompute sigma-dependent A,Gamma,v,v_e for the new h. Event/discontinuity/epoch change => invalidate history and re-run startup. Existing crates/vigilode/src/session.rs checkpoint/epoch design is DOPRI5-specific (NativeSessionMethod::Dopri5): no peer history exists; new identity needed (UNRESOLVED how, since session.rs is a protected path).

### 5.5 derivative_permission_class — UNRESOLVED
Linearly implicit W-method: no Newton, but NOT derivative-free (plan sec.3/6, anti_drift: 'JVP is derivative action'). T_m must be a declared source: (i) explicit Jacobian Dense/Banded (explicit Jacobian build counted), (ii) Jacobian::Jvp matrix-free (Jacobian-free, not derivative-free; Krylov per stage; no shared-Krylov-iteration claim from common W), (iii) default forward-difference Jacobian = approximate derivative action with extra RHS calls, (iv) source-owned exact rank-r collective operator (Woodbury) only if it is exactly T_m. Strict 'derivative-free/zero-Jacobian fast lane' label NOT granted. UNRESOLVED: mapping to Problem Jacobian enum / DerivativeFallback in switching.rs, T_m refresh/freeze policy across steps, stability robustness when T_m != J (listed unresolved in PTSW2C audit), permission wording requiring owner approval.

### 5.6 stage_graph — PASS_STRUCTURAL
Current-step stage coupling matrix: diag only (W_m), zero off-diagonal for both methods. Both stages couple to previous step only via u_m, K_{m-1}, sigma_m, h_m. PTSW2B b_1=0 so k_{m,1} reaches u_{m+1} only through the next step's A,Gamma,v. Next step requires u_{m+1}: time marching remains sequential. Do not prune coefficient entries by value (A12 zero at sigma=4 for PTSW2C; Gamma11 zero at sigma=4/9; A11 zero at 8/9).

### 5.7 admissible_sigma_domain — UNRESOLVED
Source controller clips sigma to [0.5,2]; A,Gamma,v affine in sigma (bounded there). No proof of stability for arbitrary variable-step sequences; constant-step stability slice only. Controller must enforce the domain; the proof of variable-step stability is UNRESOLVED.

### 5.8 RED 테스트 후보 (제안명, 아직 존재하지 않음 / NOT_RUN)
1. `ptsw2c_tableau_exact_residuals_c2_g2_b3 (exact rational in sigma; also asserts B(4)!=0)`
2. `ptsw2b_tableau_exact_residuals_c2_g2_b2 (exact algebraic; B(3)!=0)`
3. `ptsw2b_stiff_limit_nilpotent_index3_and_ptsw2c_not_l_stable (M_inf charpoly)`
4. `ptsw2c_embedded_coefficients_exact_and_ptsw2b_embedded_exact`
5. `ptsw2b_rhs_transform_p_x_exact_parity (W x = f + p; k = x - p)`
6. `stage_graph_zero_same_step_dependency (perturb k_{m,j} -> no change in stage i inputs/RHS, j!=i)`
7. `shared_W_factor_reused_for_both_stage_rhs (factor count 1 per step; two solves)`
8. `serial_vs_parallel_bitwise_or_tolerance_parity (ParallelExecution::sequential vs rayon)`
9. `startup_forward_only_order (u1 O(H^3)/O(H^2) for PTSW2C; O(H^2) PTSW2B; k0 O(H^2)); no backward integration`
10. `step_ratio_recompute_after_reject (A,Gamma,v,v_e at new sigma)`
11. `reject_rollback_atomicity (u,K,h,ratio-history,controller,epochs,factor gen bit-identical after reject)`
12. `event_or_epoch_change_invalidates_history_and_restarts`
13. `estimator_not_rodas (compile-time/identity assertion; weights not inherited)`
14. `controller_clip_0.5_2_and_hysteresis (PTSW2B constants; PTSW2C once transcribed)`
15. `observed_order_p3_and_p2_on_scalar_linear/nonlinear with matched startup (fixed and variable ratio)`
16. `work_ledger_counts_startup_rejected_factor_solve_work (complete ledger, failed work included)`
17. `protected_paths_byte_identical_gate (blob compare of protected list)`
18. `no_default_selector_wiring (ProductionMethod/selection unchanged)`

### 5.9 통합 지점 (소스 락 blob 기준)
| 경로 | blob | 사용 방식 |
|---|---|---|
| `crates/rodas5p-integrators/src/parallel.rs` | `ffdfb1cef258ed43586db0323eb0e7f9de27dd53` | READ-ONLY reuse: ParallelExecution::{sequential,rayon,map_ordered,try_for_each_ordered_mut}; no second pool |
| `crates/rodas5p-integrators/src/stage_batch.rs` | `c2f44c327c53d7c6dfe7ba80b26cd43486d56b43` | READ-ONLY reference for stage-batch instrumentation; RODAS-specific; not a Peer stepper |
| `crates/rodas5p-integrators/src/common_w_gate.rs` | `NOT_IN_SOURCE_LOCK` | READ-ONLY reference for shared-W/multi-RHS Krylov gate (block_gmres usage); blob not locked -> UNRESOLVED provenance |
| `crates/rodas5p-core (LinearOperator, LinearSolverConfig, LinearSolveReport, WorkCounters)` | `NOT_IN_SOURCE_LOCK` | reuse for stage solves and work accounting; exact API UNRESOLVED (not read in this audit) |
| `crates/rodas5p-krylov (gmres, block_gmres, workspace)` | `NOT_IN_SOURCE_LOCK` | reuse independent per-stage GMRES; block-GMRES only after separate proof/measure |
| `crates/rodas5p-integrators/src/dopri5.rs (+ dopri5_dense.rs)` | `NOT_IN_SOURCE_LOCK` | candidate startup starter, read-only; accuracy for startup UNRESOLVED |
| `crates/rodas5p-integrators/src/candidates.rs` | `2a6c7a7179a34932938ee4233d48b31457e38cf1 (worktree, not in lock)` | deferred PeerW stub exists (CandidateFamily::PeerW); any edit is an allowlist/owner decision -> UNRESOLVED |
| `crates/rodas5p-integrators/src/lib.rs` | `9d42738178cb7c198668dde0a9aac3e1adb86963 (worktree, not in lock)` | would need one `mod`/`pub mod` line for a NEW module file; whether this single-line edit is allowed by the PW01/PW02 allowlist is UNRESOLVED |
| `crates/vigilode/src/problem.rs` | `f1b5ef43ec66d11585193b6be9d0ee0ce271f2ec` | READ-ONLY: Jacobian enum (Dense/Banded/Jvp/FD default) for T_m source |
| `crates/vigilode/src/session.rs` | `ce30e5703a8d47a6e258a6df829b19a553fe79f9` | READ-ONLY reference for checkpoint/epoch semantics; DOPRI5 only |
| `crates/rodas5p-fair-ab/src/{global_error,adaptive_global_error,numerical_reference,output_accuracy}.rs` | `see lock_check` | READ-ONLY reporting/authority for later matched-error comparison (not PW01/PW02 acceptance) |
| `NEW module file(s) in crates/rodas5p-integrators/src/ (names UNRESOLVED) + NEW test file(s)` | `n/a` | all PTSW tableau/startup/history/estimator code; DAG mutation class ISOLATED_OPT_IN_WORKTREE_NEW_MODULE_ONLY (PW01) / SEPARATE_METHOD (PW02) |

### 5.10 변경 금지 경로
- crates/rodas5p-integrators/src/sequential.rs
- crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs
- crates/rodas5p-integrators/src/collective_rodas5p.rs
- crates/rodas5p-integrators/src/rodas5p_fast*.rs and rodas5p_lagged.rs (RODAS5P family)
- RODAS5P tableau/coefficients (rodas5p-core load_rodas5p_coefficients, embedded weights), tolerances, controller constants
- crates/vigilode/src/{selection,automatic,switching,production,lib}.rs (default selection/ProductionMethod/switching); crates/vigilode/src/session.rs; problem.rs
- crates/rodas5p-integrators/src/{bdf,radau,exponential,dopri5*}.rs (reference-only)
- crates/rodas5p-integrators/src/{homotopy,actual_k_homotopy,source_owned_graph_schur,...}.rs and all other source-lock blobs (33 locked blobs byte-identical as checked)
- crates/rodas5p-fair-ab/** (authority; T02 replay)
- research/LEDGER.jsonl (append-only), negative-evidence files, official G0 DAG, VIGILODE_UNIFIED_DAG_20261109.json, SOURCE_LOCK_20261109.json
- existing tests (never weaken/delete); CLAUDE.md/.claude/settings.json/.agent/STATE.json rules where they apply

### 5.11 G1 선행 조건
- G1 mutation gate (LOCAL_AGENT_DEVELOPMENT_SPEC_KO sec.10): task-specific prerequisites, own worktree, explicit file allowlist, RED test first, opt-in module/API, unchanged protected path, rollback tests, exact hashes, receipt; never wire default selector.
- DAG dependency PW00_PEER_W_SOURCE_AND_API_AUDIT: status of PW00 in DAG not re-checked here -> UNRESOLVED (PW00 receipt exists in solver_consolidation_20261108/PEERW_EXECUTION_RECEIPT_PW00.json; not audited).
- Owner approval (G3) needed for: tableau/estimator/controller changes, thresholds (D2), default behaviour.

## 6. 최종 준비도
- **PW01:** NOT_READY_FOR_G1 (math/tableau READY; UNRESOLVED: D5 PTSW2C controller constants, starter/startup contract, derivative permission class, file allowlist, step ceiling D2)
- **PW02:** CONDITIONALLY_READY_FOR_RED_TESTS (tableau/embedded/controller constants all transcribed and verified; UNRESOLVED: starter, derivative class, allowlist, ceiling, L(90) sector proof only source-reported)
- READY refers to documentation/arithmetic only. No Rust/RED test executed. Not a qualification.

## 7. 실행하지 않은 것 (NOT_RUN)
- cargo build/test/clippy (forbidden by task)
- git commands (forbidden by task)
- any Rust RED test (none exist yet; no Rust written)
- interval-certified A(alpha) sector proof
- L(90) full sector proof for PTSW2B
- variable-step product-stability proof for either method
- startup order test with a real starter
- W-perturbation (T!=J) robustness
- matched-error/benchmark performance

## 8. 재현
검사 스크립트 전문(`pw_check.py`, `pw_check2.py`)과 모든 출력은 `PW01_PW02_READINESS_AUDIT.json`의 `check_script_texts`, `results`, `receipts`에 포함. 실행: `python3 -I <script>` (파일 쓰기 없음, 네트워크 없음).
