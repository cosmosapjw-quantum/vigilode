# U10 전환 오차 전달과 신규성 — 2026-11-16

판정: **U10 COMPLETE_SCOPED, 신규성 주장은 성립하지 않음(NOVELTY_NOT_SUPPORTED)**. 계약의 failure_action에 따라 통합 공학 결과만 남긴다. 원자료는 `transport_results.json`, 실행은 `python3 -I transport.py`(독립 Python backend, U03 route-R1 계수)다.

## 1. 전달 재귀 (WP7 확장: 결함 envelope + 양의 majorant)
선형 PR 시험 `y' = λ(y−φ) + φ'`, T = λ에서 Peer stage-이력 오차는 정확히 `e_{n+1} = M_n e_n + δ_n`, `M_n = (B + z(A+Γ))/(1−γz)`를 따른다. 따라서 성분별로 `|e_{n+1}| ≤ |M_n||e_n| + |δ_n|`(G_n = |M_n| ≥ 0)이다.
전환 사상:
- T_PR(Peer → Radau IIA(2) one-step): stage 6 선택, 이득 1, 결함 0.
- T_RP(one-step → Peer): PPSW1 Radau IIA(5) starter. y에 대해 affine이며, 이득 a와 결함 d_T를 정확한 자료에서 측정했다.
- one-step 구간: `|e| ≤ |R(z)||e| + |δ|`.

## 2. 결과 (λ = −1, −50, −1e4; h = 0.01; Peer 40 → one-step 20 → Peer 40)
- **위반 0건**: 모든 step에서 실제 오차 ≤ 상계.
- **상계는 쓸모없을 만큼 느슨하다**: bound/error = 8.7e175, 1.2e164, 3.2e15. q = ‖|M_n|‖∞가 nodal 기저에서 9–429, monomial 기저에서 1.8–12로 **항상 ≥ 1**이다(q ≥ 1 사례를 모두 보존).
- **비정규 증폭**: σ=1에서 ρ(M)=0.99(z=−0.01)인데 ‖M‖∞=429다. 가변 σ 30 step 곱에서 실제 ‖ΠM‖∞ = 56 / 6.6 / 0.0039인데, 스펙트럼 반경 곱은 0.77 / 0.085 / 0.00047(과소), majorant 노름 곱은 1e67 / 1e65 / 1e61(과대)이다. 고유값만으로는 증폭을 놓치고, step별 majorant 곱은 쓸 수 없다.
- **창 majorant** ‖|M^k|‖∞ < 1이 되는 첫 창: z ≤ −100에서 k=5(= s−1, Schmitt–Weiner Lemma 3.2의 rank-1 구조와 일치), z=−10에서 10, z=−1에서 40, z=−0.1에서 80, z=−0.01은 80 안에 없음. 블록 인증서는 stiff mode에서만 수축한다.

## 3. 선행연구와의 정확한 차이
| 구성요소 | 이번 근거 | 가장 가까운 선행연구 | 차이 |
|---|---|---|---|
| PPSW1-6b 계수·안정성 | U03 독립 재구성 | Schmitt & Weiner, SINUM 42(1) 2004 (Table 1, Thm 3.1, Lemma 3.2) | 없음(재현) |
| 병렬 stage RHS/solve | U01, U04 | Peer-W의 본래 설계 목적(s개 프로세서 병렬) | 없음 |
| matrix-free Krylov W | U04(crossover 없음) | Krylov-W/Rosenbrock–Krylov 계열(peer 변형 포함; 정확한 서지는 확인 필요) | 새 방법 아님, 이 구현에서 비용 우위 없음 |
| 병렬 SDC | U05 | parallel-across-the-method SDC, MIN-SR 계열 | 정확도 회복에 8–13회 반복, 우위 없음 |
| 전환 오차 전달 | 이 문서 | W-방법·다단계 안정성 이론, LSODA류 stiffness 전환(Petzold 1983) | 성분별 majorant은 표준이며 이 사례에서 쓸모없이 느슨함 |
| 구조 인증(trace) | U08 | 프로젝트 내부 기존 기제(PR 97/98) | 이식만 함 |

## 4. 소거(ablation)와 비용 (모두 동일 달성오차 또는 Dense 대비)
- U03: PPSW1은 RODAS5P 대비 총 RHS·LU가 1.4–9.2배다. 이상적 병렬 임계경로는 모델로만 0.47–0.89배(VdP)이고 측정이 아니다.
- U04: matrix-free W는 n ≤ 400에서 crossover가 없고, n=800에서 실패하며, peak 메모리 이득도 없다.
- U07: 전환 인증서가 없어 전환 API는 typed 거부다.
holdout은 쓰지 않았고 재조정도 하지 않았다. 인증서 구성 비용(행렬 곱)은 전환 실험에서 Peer step당 6×6 행렬 곱 1회이며 ODE 크기와 무관하다.

## 5. 결론
정리나 알고리즘 차원의 새 기여는 입증되지 않았다. 성분별 majorant 전달은 유효하지만 1e15배 이상 느슨하다. 창 majorant은 stiff mode에서만 수축하고 선행 Peer 안정성 이론과 같은 구조다. 측정된 비용 우위도 없다. 남는 것은 범위가 제한된 공학 결과다: 독립 qualification, 정직한 권한·회계, 음성 crossover 측정. 후보 신규성은 "인증된 영역 + 측정된 완전 비용 crossover"인데 둘 다 성립하지 않았다.
