# U03 PPSW1-6b 독립 qualification — 2026-11-15

판정(후속 갱신): **U03 COMPLETE_SCOPED** — 계약의 완료 조건(독립 계수 비교, 원시 실패를 포함한 참조 기반 오차 곡선)을 선형·비선형 calibration에서 충족했고 거부 영역을 기록했다. 후보의 경쟁력·production 자격은 아니다. production **HOLD**, G0 변경 없음. §1–7은 선형, §8은 비선형이다. 원시 근거는 `logs/u03_green.log`(QUAL 행), `RESULTS.json`, `theta_oracle_results.json`, `coefficients_oracle_summary.json`이다.

## 1. 변경 (C1, `CHANGE_CONTRACT.json`에 사전 기록)
`solve_stages`는 fallible `LuFactorization::solve` 전에 `linear_solves/direct_solve_calls += 6`을 선부과했다. 순차 실행은 첫 실패에서 멈추므로 1회 진입에 6회가 부과됐고, Rayon `collect::<Result>`는 형제 job 취소가 비결정적이었다.
- RED: `logs/u03_red.log`, `logs/u03_red2.log` — 오버플로 RHS(J=f64::MAX)와 정확히 특이한 W에서 순차 `(6,6)` 부과, 기대 `(1,1)`.
- 수정: 순차는 진입 직전 1회씩 부과 후 첫 오류에서 중단, 병렬은 6개 job을 모두 Ok envelope로 join하여 6회 부과 후 stage 순서 첫 오류 반환. 성공 경로 값/카운터 불변. 계수·식·상수 불변.
- GREEN: 신규 13/13, 기존 library 123(+1 ignored)/Peer 37/병렬 5/경계 6 = 171 통과(`logs/u03_regression.log`). fmt 통과. 진단 clippy exit 0, 경고 8개는 모두 미변경 `source_owned_graph_schur.rs`(기존과 동일), 변경 파일 0. strict `-D warnings`는 같은 기존 8개 때문에 여전히 실패로 간주한다.
- 추가 분류: NaN Jacobian은 Jacobian 생성 단계에서 NonFinite로 거절된다(Jacobian 1, LU 0, solve 0, Rejected). 실패 시 state/controller 보존.

## 2. 계수: 독립 경로 R1
`oracle_coefficients.py`는 논문 식 (14) Gamma(6), (23) beta, (4) A=beta-Gamma, (15) AB(6)를 mpmath 60자리로 직접 푼다. 구현이 쓰는 닫힌형 (16)/(21)은 쓰지 않는다. 기존 `ppsw1_6b_contracts.rs` oracle은 같은 닫힌형(route 2)이라 구현과 독립적인 비교가 아니었다.
- gamma: Table 1 다항식 근 0.619411975066250516310763374002905109124…, 구현 38자리 상수와 f64 동일.
- sigma 1e-9..1.1의 15점에서 B/A/Gamma/beta 최대 차이 ≤1.4e-16 scale-eps(사실상 bit 일치). 조건 잔차 ≤4e-25(sigma=1e-9), ≤1e-58(sigma≥0.5). k=6 잔차는 0이 아니어서 변보폭 차수는 정확히 5다.
- V 색인 규약(PW04 UNRESOLVED-1)은 (23)에만 들어가며 node-row 규약을 유지했다. 인쇄본 자체의 모호성은 문헌상 미해결이다.

## 3. 실제 구현 단계 실행
- 다항식 정확성(q02, 비정규 L, sigma에 1e-3 포함): 5차 오차 1.8e-12(스케일 1.1), 6차 1.0e-4. W-방법 차수조건이 실제 stepping 경로에서 성립한다.
- Prothero–Robinson(q03, 정확 시작, N=16..128): 균일 보폭 lambda=-1 5.93/5.98(초수렴, 이후 바닥), -1e4 6.05/6.05/6.13, -1e6 6.04/6.02/6.37. lambda=-100 균일은 2.3→4.6→5.3 preasymptotic. 변보폭 lambda=-1 5.7/6.1/7.5, stiff 변보폭은 1e-11 이하 바닥에서 기울기 1.8–10.5로 흔들려 asymptotic 판정 불가.
- LIN3 비정규(q04): 독립 numpy backend(theta=1)와 N=12,24에서 0.988–1.0003배 일치, N≥48은 roundoff 바닥(1e-10~1e-11) 차이.
- sigma 패턴(q05, 0.5..1.1): 모든 패턴 유한, lambda=-1 첫 refinement 쌍 관측 차수 5.5–6.1(이후 바닥 근처 4.3–9.2). final clipping 0.3..1e-12: 오차 증가 최대 1.001배. 극소 비율 계수는 유계(최대 |coef|≈1). **extreme ratio 검증은 이 선형 문제 범위뿐**이다.

## 4. 시작·추정기·적응 driver (실패 포함)
- 시작(q06): Radau IIA starter Newton은 선형 문제에서 2회, node 오차 ≤1.2e-11. 끝점은 정확히 `t_init+2h0`.
- **결함 F1 (시작 오차 미제어)**: h0 규칙 `min(h_max, 1/||J||)`은 tolerance를 보지 않는다. h_max=t_end/2, lambda=-1이면 starter 한 번이 [0,1] 전체를 덮어 tol=1e-9에서 달성 오차 7.4e-8(tol의 74배). 호출자 h_max가 사실상 정확도 계약이다.
- **결함 F2 (성장 제한)**: cap 1.10 + 6-step hysteresis로 성장률은 약 1.1^(1/7)/step이다. lambda=-1e4는 tol 1e-3..1e-9에서 동일 359 step, 달성 오차 8.3e-12(tol 1e-3의 8e-9배). lambda=-1(h0=0.01)도 tol 1e-3..1e-7에서 동일 61 step. 느슨한 tol에서 작업 과다.
- **결함 F3 (추정기 바닥)**: tol=1e-12에서 세 문제 모두 h가 1e-19까지 붕괴하고 typed `InvalidInput("representably advancing time")`로 끝난다(무한루프 아님). error_norm은 h와 무관하게 0.2–1.0을 유지했고 rounding-floor 경고는 0회였다. 경고 모델 `sum|w| eps |Y|`(sigma=1에서 상대 5.6e-14)는 stage 값 잡음 증폭을 포함하지 않는다. q07: 추정기/참 국소오차 비 14–132, h가 줄수록 증가(추정기 ~h^5, 참 오차 ~h^6) — 보수적이다.
- 병렬 4 thread는 순차와 오차·counter가 동일(q08).
- 거부(q09): mass matrix는 callback 전에 InvalidInput, counter 0. 이벤트·dense output API는 없다(미지원).

## 5. approximate-J theta 축 (sigma와 분리, 독립 backend)
`theta_oracle.py`: T = theta J + DeltaT, R1 계수, numpy 실행. 생산 exact-J 코드는 건드리지 않았다.
- scalar 안정성 M(z,theta): theta≠1이면 M(∞)=-(beta+(theta-1)Gamma)/(gamma theta)가 멱영이 아니어서 L-안정성이 깨진다. sigma=1: theta≥0.95는 scan 범위에서 ρ≤1, theta=0.9/0.85/0.8은 z≲-4.5/-2.8/-2.2에서 ρ>1, ρ(-1e8)=1.35/1.70/2.05. sigma=1.1 상수 비율 행렬은 theta=1에서도 z∈[-4.9,-1.5]에서 ρ=1.04(상수 성장 수열의 1-step 지표이며 가변 곱 판정은 아님).
- 비정규 stiff PR(고유값 -1,-50,-1000): theta∈{0.8,0.85} 발산, theta=0.9는 z가 불안정 창에 들어가는 N=96에서 오차 증가(eps=1e-2 행만 수렴: 비단조, 미해결). theta∈[0.95,1.1], ||DeltaT||≤1e-2||J||는 수렴(차수≈6, 바닥 1e-10). 비가환 ||DeltaT||=0.1||J||는 모든 theta에서 coarse 격자 오차 ≥1e-2(theta≤0.9는 1e5 이상)로 불안정/미해결.
- 결론(이 문제 한정): 지원 창 theta∈[0.95,1.1], 비가환 perturbation ≤1e-2. J 과소추정이 위험하며 PW04의 0.8 발산을 재확인했다. 구현은 step 시작의 exact J/JVP를 쓰므로 비선형 문제의 J 노후화가 이 축에 해당한다(비선형 NOT_RUN).

## 6. capability 판정 (startup/fallback 포함)
| 성질 | per-step | startup | fallback |
|---|---|---|---|
| Jacobian-free | 아니오(explicit J 또는 n개 JVP로 dense J) | 아니오(J 1회) | fallback 없음 |
| derivative-free | 아니오(`OdeProblem::new`가 J/JVP 없으면 거부) | 아니오 | 없음 |
| matrix-free | 아니오(dense W LU) | 아니오(5n×5n dense LU) | 없음 |
| Newton-free | 예(선형 solve 6개) | 아니오(simplified Newton) | 없음 |

FD/AD 경로 없음. JVP는 model-supplied 도함수이며 추정이 아니다. 실패는 typed error 또는 Rejected로 반환, 자동 fallback 없음.

## 7. NOT_RUN / 열린 의무
비선형 문제(Fisher/Van der Pol 등)와 J 노후화 연동, T01 matched-error 출력격자·참조 불확실성 결합, RSS/CPU/wall time, F1–F3 교정(별도 근거 필요: h0의 tol 결합, 최소 tol fail-closed, 경고 모델 보정 등), theta 0.9 비단조 해석, 이벤트/dense output, callback panic/unwind 계약. 범위 제한 통과를 production 자격·신규성으로 승격하지 않는다.

## 8. 비선형 qualification (후속 턴)
사전 계약: `CHANGE_CONTRACT.json#turn2_nonlinear`. 원시 근거: `logs/u03_green.log`(n1–n5), `NONLINEAR_RESULTS.json`, `nonlinear_reference.json`.

### 8.1 참조와 오차 정의
- 문제: scaled Van der Pol eps=1e-1, 1e-3 (t∈[0,0.5]), Robertson (t∈[0,40]), Fisher MOL N=20 (t∈[0,1]).
- 참조: scipy Radau rtol 1e-13; U_ref = |Radau13−Radau12| + |Radau13−M2_13|. M2는 Fisher/VdP 0.1은 DOP853, VdP 1e-3/Robertson은 LSODA(보수적). Rust 실행 전 참조 진단에서 Fisher의 LSODA/BDF가 Radau·DOP853(서로 1e-10 일치)과 5e-8 다른 이상치여서 이렇게 선언했고, 모든 방법 간 차이를 기록했다. U_ref WRMS: 4.6e-14, 1.8e-11, 4.2e-12, 2.1e-10.
- E = 끝점 WRMS(가중치 |y_ref|+floor, solver tol과 무관) + U_ref. U_ref ≤ 0.1E인 점만 admissible. 출력 격자는 {t_end} 한 점이다(PPSW1에 library dense output 없음) — T01 protocol의 다점 격자·interpolant gate는 충족하지 않은 제한이다.

### 8.2 고정 보폭 차수 (n1, 실제 starter, h0=h/16)
- Fisher: 5.80/5.97/6.26 (깨끗한 asymptotic 6차 근접).
- VdP eps=0.1: 5.88 후 5e-11–9e-11 바닥. 선형 PR 바닥(3e-11)과 같은 크기로, 큰 B 계수(|B| 최대 134)에 의한 반올림 누적으로 분류한다(**F4 정확도 바닥**, 가설; 별도 증명 없음).
- VdP eps=1e-3: −0.07/4.35/2.34, preasymptotic stiff envelope, 차수 판정 불가.
- Robertson: 모든 N에서 starter 실패(아래 F5).

### 8.3 Jacobian 노후화 축 (n2, J를 k번째 호출마다 갱신, N=128)
k=1..32에서 VdP·Fisher 모두 발산 없음. VdP eps=1e-3은 k=4에서 오차 14배(8.5e-9 대 6.1e-10)로 비단조 악화. 선형 theta 축(§5)의 불안정은 이 비선형 문제들에서는 재현되지 않았다. 일반화하지 않는다.

### 8.4 결함 수정 (RED → GREEN)
- **C2 cap 반올림**: 제어기는 성장 시 정확히 cap(1.10)을 제안하는데, 호출자가 h=h_prev·1.1을 만들면 fl(h/h_prev)=1.1000000000000003이 되어 try_step이 자기 제안을 InvalidInput으로 거부했다(VdP 1e-3, tol 1e-9 driver가 t=3.48e-2에서 중단). 검사를 h > fl(cap·h_prev)로 바꿨다: r≤cap인 모든 fl(h_prev·r)는 허용, next_up은 계속 거부. cap 값·제어기·계수 불변. RED `logs/u03_nonlinear_red.log`, n5 GREEN.
- **C3 starter 실패 회계**: starter Newton 중 RHS/solve 오류가 `?`로 빠져 nonlinear_failures가 0이었다. 이제 1회 부과한다. 오류 종류·성공 경로 불변. n4 GREEN.
- 회귀: library 123(+1 ignored)/Peer 37/병렬 5/경계 6 = 171 통과, 신규 18/18. fmt 통과, 진단 clippy 경고는 기존 8개뿐.

### 8.5 적응 실행과 매칭 오차 (n3/n4, PPSW1 test-local driver vs production RODAS5P Sequential/Direct)
같은 문제·y0·끝점·atol/rtol·h0(PPSW1 규칙, h_max=t_end/100 고정)로 tol 1e-3..1e-9 7점을 돌렸다. wall time은 단일 실행 diagnostic_only.
- **F2 재확인**: PPSW1은 VdP 0.1에서 tol 1e-3..1e-7 동일 61 step, Fisher 1e-3..1e-5 동일 234 step. tol에 둔감하다.
- **F6 (VdP eps=1e-3)**: tol을 조일수록 PPSW1 오차가 커진다(3.1e-10 → 1.1e-9, 거절 0→11). 원인 미해결.
- **F5 (Robertson)**: 문서화된 h0 규칙은 ||J(y0)||=0.08이라 h0=0.4를 주고, J(y0) 고정 simplified Newton이 발산한다(NonFinite). h0≥1e-2 실패, ≤1e-3 성공. RODAS5P는 같은 h0로 성공한다. 호출자가 h0=1e-6을 주면 모든 tol에서 968 step, E≈3.1e-10(과잉 정확).
- 매칭 오차(겹침 구간에서만, log-log 보간, 외삽 없음):

| 문제 | 겹침 E 구간 | RHS 비 (PPSW1/RODAS5P) | LU 비 | 임계경로 모델 비 |
|---|---|---|---|---|
| VdP eps=0.1 | 3.5e-10–1.6e-9 | 1.9–2.6 | 3.0–4.2 | 0.66–0.89 |
| VdP eps=1e-3 | 5.1e-10–1.1e-9 | 1.4–2.5 | 2.3–4.1 | 0.47–0.85 |
| Fisher N=20 | 2.0e-8–9.4e-7 | 3.3–5.6 | 5.4–9.2 | 1.1–1.9 |
| Robertson | 겹침 없음 | NOT_COMPARABLE | – | – |

  총 작업(RHS, LU)은 모든 겹침에서 PPSW1이 1.4–9.2배 많다. 임계경로 모델은 이상적 6-way stage 병렬을 가정한 계산값이며 측정이 아니다. LU 비용 차이, thread overhead, 메모리는 넣지 않았다. VdP에서만 모델상 0.47–0.89배다. 이것은 속도 주장이 아니다. 후보 arm은 opt-in이라 comparative reading은 diagnostic이다.

### 8.6 종합과 남은 의무
U03 계약의 완료 조건은 scoped로 충족됐다. 거부 영역은 F1(tol 무시 h0), F2(성장 제한), F3(추정기 바닥), F4(정확도 바닥 ~1e-11 상대), F5(Robertson류 starter 실패), F6(VdP 1e-3에서 tol을 조일수록 오차 증가), theta≤0.9이다. 다음은 U11/T02로 넘긴다: 다점 출력 격자 interpolant, paired wall-time protocol(≥2 warmup, ≥7 반복), RSS, 큰 MOL 문제, F1–F6 교정(근거 필요). 기본 라우팅·G0·허용오차는 바꾸지 않았다.
