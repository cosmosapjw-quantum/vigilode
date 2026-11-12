# PW01 controller/tableau 상수 복원: Podhaisky 2002 박사논문 1차 열람 결과

**작성일:** 2026-11-11 · **모드:** LITERATURE + DOCUMENTATION ONLY (Rust/git 미실행, 기존 파일 미수정, PDF 미복사)
**소스:** 소유자 제공 `/home/cosmosapjw/vigilode/Podhaisky_2002.pdf` (76 pp., sha256 `4a3cfa7f...edf0`). 이전 `PW01_CONTROLLER_CONSTANTS_RECOVERY_KO.md`의 UNRESOLVED #1(원문 미열람)을 해소하는 문서.
**쪽수 표기:** "p.N" = 논문 인쇄 쪽수, PDF 쪽수 = N + 4 (예: p.42 = PDF 46).
**이미지 검증:** 아래 3쪽을 `pdftoppm -r 300`으로 렌더링해 Read로 직접 확인: p.36 (Tabelle 4.1, PDF 40), p.41 (식 4.7, PDF 45), p.42 (controller, PDF 46). 그 외 쪽은 `pdftotext -layout` 텍스트만으로 읽음(상수가 아닌 서술; 이미지 미확인이라고 표기).

## 0. 결론

1. **controller의 모든 상수가 원문 p.41-42에 인쇄되어 있다.** SECONDARY_ONLY 상태였던 C1, C2, C3, C4, C5, C6, C7이 SOURCE_PRINTED_THESIS로 승격된다. 인쇄 형태(p.42):
   `σ* = min{2, max{0.5, Err_emb^(-1/s) · 0.85}}`, `h* = h_m` (σ*>1.05 이고 어떤 j∈{0,...,s-1}에 대해 σ_{m-j}>1), `h* = h_m` (σ*∈[0.95,1.05]), `h* = h_m σ*` (그 외). 여기서 **s = 스테이지 수**(2,3,4 모두 같은 식), embedded 방법 차수는 s-1 (p.41).
2. **hysteresis(C3/C4)는 PTSW2B 전용이 아니라 논문의 모든 PTSW 방법(PTSW2C 포함)에 공통인 하나의 규칙**이다. 방법별 상수 표는 없다. 이전 D5 blocker("PTSW2C에 C3/C4가 전사되지 않음")는 원문 기준으로 해소: PTSW2C는 s=2이므로 exponent -1/2, history s=2.
3. **PTSW2A/B/C 매핑은 RESOLVED** (Tabelle 4.1, p.36 + §3.2/§3.3.3/§3.3.6): 논문 이름 PTSW2B = 저장소 PTSW2B, PTSW2C = 저장소 PTSW2C. (§2 참조)
4. **PW02 코드(`ptsw2b.rs`)의 SAFETY=0.85, RATIO_CLIP=(0.5,2), HYSTERESIS_BAND=(0.95,1.05), HYSTERESIS_HISTORY=2, exponent -0.5, RMS norm은 논문과 일치.** 불일치는 비상수 항목(starter, h0, ceiling 1.10 등)뿐이다 (§7).
5. **repo certificate의 PTSW2B/PTSW2C 계수는 논문 정의(식 2.14, 4.7)로부터 sympy 정확 재계산과 전부 일치(diff = 0).** 논문은 γ, c만 8자리로 인쇄하고 A, Γ, v, b_e, v_e는 인쇄하지 않는다(PTSW2A만 Fortran으로 인쇄, p.69). 따라서 certificate의 A/Γ/v/v_e는 "DERIVED from printed (c, γ, b-rule) via printed eq (2.14)/(4.7)"이지 SOURCE_PRINTED가 아니다.
6. 논문에 **없는** 것: 초기 step h0 선택 규칙, hmin/hmax, 연속 거부 상한, 변동 step 안정성 증명, σ 하한 안정성 근거, 관측 차수 표. (§5, §6, §8)

## 1. Step-size control (Task 1), 원문 위치 p.41-42 (PDF 45-46, 이미지 검증)

| 항목 | 원문 인쇄 | 쪽 |
|---|---|---|
| embedded 방법 | 차수 s-1. 가중치 `b_e^T = 0.95 b^T`, `v_e^T = ((1^T + ε e_s^T) S D^{-1} - b_e^T V0 S) V1^{-1}` (4.7); B(s)에서 σ와 무관한 residual ε := 0.1 | 41-42 |
| 오차 | `Err_emb = sqrt( (1/n) Σ_i ( (u_{m+1,i} - ũ_{m+1,i}) / (ATOL + RTOL·|u_{m+1,i}|) )^2 )`, `ũ_{m+1} = u_m + h_m Σ_i (b_{e,i} k_{m,i} + v_{e,i} k_{m-1,i})` | 42 |
| tolerance | 실험에서 항상 `ATOL = RTOL = 10^{-i}, i = 2,...,8` (일반 규칙이 아니라 실험 설정) | 42 |
| step 비율 | `σ* = min{2, max{0.5, Err_emb^{-1/s} · 0.85}}` (즉 안전계수 0.85는 clip 안쪽에서 적용) | 42 |
| hysteresis | `h*=h_m` if `σ*>1.05` 그리고 `∃ j∈{0,...,s-1}: σ_{m-j}>1`; `h*=h_m` if `σ*∈[0.95,1.05]`; `h*=h_m σ*` otherwise. "häufige Schrittweitenwechsel zu vermeiden" | 42 |
| accept/reject | `Err_emb ≤ 1` accept, `h_{m+1}:=h*`, `σ_{m+1}:=h_{m+1}/h_m`. 거부 시 `h_m := h*`, `σ_m := h_m/h_{m-1}` 로 같은 step 반복. 별도 reject factor 없음 | 42 |
| 부가 효과 | "s 연속 step에서 최대 한 번만 증가", 오래된 LU 재사용 가능 (§4.4 참조) | 42 |
| NaN | `Err_emb ≠ Err_emb` 이면 `h_m := h_m/2`, reject (Bemerkung 4.6) | 42 |
| 계수 재계산 | σ 변경 시 식 (2.14)로 A, Γ, v 재계산: s×s 행렬곱 2회 + S=diag(1,σ,...,σ^{s-1}) 스케일링, 비용 무시 가능 | 41 |
| 노름 | RMS (L2/√n), 가중치는 **새 해** u_{m+1} 기준 | 42 |

해석 확정 사항:
- 이전 문서의 모호성("σ*∈(1,1.05] 처리", "last s ratios의 의미")은 인쇄 식으로 해소된다. 세 경우는 서로 배타적이다 (σ*>1.05&growth / σ*∈[0.95,1.05] / 나머지). 나머지에는 σ*<0.95와 "σ*>1.05이지만 최근 s개 ratio 중 증가 없음"이 모두 포함되어 후자는 `h_m σ*` 로 성장한다. σ_{m-j}는 j=0에서 **현재 시도 중인 step의 비율 σ_m**을 포함한다(j=0..s-1). 논문은 "시도 vs 승인" 구분을 하지 않으나, 거부 시 σ_m이 h*/h_{m-1}로 갱신된다는 서술(p.42)이 있고 σ*>1.05인 거부는 불가능(σ*≤0.85)하므로 실질 차이 없음(DERIVED).
- exponent: 인쇄값 -1/s (s=스테이지 수). embedded 차수 p_e=s-1 이므로 -1/(p_e+1)과 일치(DERIVED, 일관). PTSW2B/PTSW2C 모두 s=2 → -1/2. 이전 문서의 "PTSW2C p_e 미계산(NOT_RUN)"은 이번에 계산: 아래 §3 스크립트에서 PTSW2B, PTSW2C 모두 embedded B(1) residual 0, B(2) residual σ/20 ≠ 0 → 차수 1 = s-1.
- 논문에서 σmax는 두 곳에만 나온다: p.8 "계수의 균등 유계성은 step 증가가 제한될 때(σ_m<σ_max ∀m) 얻는다", 그리고 controller의 clip 2. 별도의 안정성 근거 표는 없다.

## 2. 방법 이름 매핑 (Task 1 후반) — RESOLVED

논문 §4.2 명명 규칙(p.35): PTSWsA = L(α)-stable, 스티프 정확, 0<ρ(M(∞))<1 (s=2,3은 B*(s+1)); PTSWsB = L(α)-stable, 스티프 정확, ρ(M(∞))=0, Satz 3.1로 구성; PTSWsC = s=2,3는 A(α)-stable, B(s+1), v^T=0 (s=4는 L(α), ρ(M∞)=0). Tabelle 4.1 (p.36, 이미지 검증):

| 이름 | 논문 인쇄 | 저장소 대응 |
|---|---|---|
| PTSW2A | s=2, L(90), B*(s+1), \|C4\|≈0.18, γ=4/5, c=(23/9, 1), ρ(M∞)=0.8799 | PTSW2A |
| PTSW2B | s=2, L(90), ρ(M∞)=0, \|C3\|≈0.25, γ=0.63397459, c: 아래 주의, ρ=0 | PTSW2B (cert: γ=(3-√3)/2, c=(3-2√3, 1)) |
| PTSW2C | s=2, A(81.1), B(s+1), \|C4\|≈0.12, γ=0.48, c=(-1, 5/9), ρ(M∞)=0.6796 | PTSW2C (cert: γ=12/25, c=(-1,5/9), b=(1/28,27/28), v=0) |

**인쇄 이상 (플래그, 조용히 고치지 않음):** Tabelle 4.1 PTSW2B 행의 Knoten 열에는 **3개 값**(-0.46410161, 0.63397459, 1)이 인쇄되어 있으나 s=2다 (s=3,4 방법은 정확히 s개). 가운데 값 0.63397459는 γ 값과 같다. 본문 §3.2 (p.17)는 `γ = 0.63397459, c1 = -0.4641016, c2 = 1 (PTSW2B)` 로 두 노드만 준다. 또 φ*(c,γ)=c²+(2-4γ)c-3+4γ 의 근이 정확히 {-0.46410..., 1}임을 계산으로 확인(0.634는 근이 아님). 결론: 표의 가운데 값은 조판 오류로 판단(DERIVED), 올바른 노드는 p.17의 두 개. **이미지에서 3개 값이 실제로 인쇄된 것을 확인.**
**순서 차이:** PTSW2C는 Tabelle 4.1에서 c=(-1, 5/9)이고 §3.3.6(p.28)에서는 c1=5/9, c2=-1. 논문 Bemerkung 3.1(p.15)에 의해 방법은 노드 permutation에 불변이고 (4.7)의 e_s는 monomial 지수 l=s를 고르므로 stage 순서와 무관. certificate는 Tabelle 4.1 순서를 사용하며 일관됨.

## 3. 계수 비교 (Task 2)

### 3.1 논문이 직접 인쇄한 것
- Tabelle 4.1: γ, c (8자리 이하, 절단 표기): PTSW2B γ=0.63397459, c1=-0.46410161; PTSW2C γ=0.48, c=(-1, 5/9); PTSW2A γ=4/5, c=(23/9,1).
- §3.2 s=2 (p.17): π(γ) = -γ²+3γ-3/2; C3 = 2γ²-7/2γ+7/6; φ*(c,γ)=c²+(2-4γ)c-3+4γ. 해: γ=0.63397459, c1=-0.4641016, c2=1 (PTSW2B); 두 번째 해 γ=2.36602540, c1=6.4641016 (거부).
- 식 (2.14) (p.11): `A = C V0 S D^{-1} P_s V0^{-1}`, `Γ = -γ V0 S P_s V0^{-1}`, `v^T = [1^T S D^{-1} - b^T V0 S] P_s V0^{-1}`; 스티프 정확 (2.22) (p.12): `c_s=1, b^T=γ e_s^T, v^T = e_s^T β`; v^T=0 방법은 B(s)로 b 결정(§3.3.6).
- 식 (4.7) (p.41): `b_e^T = 0.95 b^T`, `v_e^T = ((1^T+ε e_s^T) S D^{-1} - b_e^T V0 S) V1^{-1}`, ε=0.1.
- **A, Γ, v, b_e, v_e의 수치/폐형식 tableau는 PTSW2B, PTSW2C에 대해 인쇄되어 있지 않다.** 유일한 인쇄 계수 예는 부록 A.5 (p.68-69)의 PTSW2A Fortran 블록 (PDF 72-73; 텍스트 열람, 이미지 미확인).
- PTSW2C의 b는 인쇄되지 않음. v^T=0이고 B(3) 조건에서 b1+b2=1, -b1+(5/9)b2=1/2 → b=(1/28, 27/28) (DERIVED; b·c²=1/3 확인).

### 3.2 certificate 대조 (`python3 -I verify_thesis.py`, sympy 1.12, 정확 대수; 영수증 §9)
| 대상 | certificate 값 | 논문 식에서 재계산 | 결과 |
|---|---|---|---|
| PTSW2B γ, c1 | (3-√3)/2 = 0.6339745962, 3-2√3 = -0.4641016151 | 논문 p.17/표: 0.63397459, -0.46410161 (8자리 절단: 둘 다 1e-8 이내, 마지막 자리는 반올림이 아니라 절단 0.63397460→"...59") | 일치 (폐형식은 π, φ*의 해이므로 DERIVED) |
| PTSW2B A(σ) (cert 2.1) | 식 (2.1) | `A=C V0 S D^-1 P V0^-1` | diff = 0 |
| PTSW2B Γ(σ) (cert 2.2) | 식 (2.2) | `-γ V0 S P V0^-1` | diff = 0 |
| PTSW2B b, v(σ) (cert 2.3) | b=(0,γ), v=(√3-1)/8 (σ, 4-σ) | (2.22), (2.14) | v diff = 0, b 일치 |
| PTSW2B b_e, v_e (cert 5.1) | b_e=(0,19γ/20), v_e=((8√3-11)σ/80, (11-8√3)σ/80+(19√3-17)/40) | (4.7) | v_e diff = 0 |
| PTSW2B C3 | -0.24839684 | p.17 식 + Tabelle 3.1 p.19 "0.2483", Tabelle 4.1 "0.25" | 일치 (직접 대입 -0.24839) |
| PTSW2C A(σ) (cert 3) | 식 (3) | (2.14) with c=(-1,5/9) | diff = 0 |
| PTSW2C Γ(σ) (cert 4) | 식 (4) | (2.14) | diff = 0 |
| PTSW2C b, v | b=(1/28,27/28), v=0 | B(1..3) residual 0 (정확) | 일치 (b는 DERIVED) |
| PTSW2C b_e | (19/560, 513/560) | 0.95 b | 일치 |
| PTSW2C v_e (cert 7) | (-(8+27σ)/560, 9(4+3σ)/560) | (4.7) | diff = 0 |
| PTSW2A (스크립트 자체 검증) | 논문 A.5 Fortran: a11=529/252 σ, a12=23/9-529/252 σ, ..., v=(-27/140 σ, 1/5+27/140 σ), ve=(-27/200 σ, 6/25+27/200 σ), be=(0,19/25) | 같은 스크립트로 재계산 | 전부 일치 (스크립트가 올바르게 (2.14),(4.7)을 구현했다는 독립 증거) |
| Embedded 차수 | cert: "order s-1=1" | PTSW2B/C 모두 B(1) residual 0, B(2) residual σ/20 | 차수 1 확인 |
- **불일치: 없음.** 논문 소수 자릿수와 certificate 폐형식의 차이는 절단 표기(≤1e-8)뿐. 이전 certificate에서 "source eq.(4.7)"이라 한 것은 정확하다 (p.41 인쇄 확인).
- 직접 대조 불가(논문 미인쇄)로 남는 것: certificate의 A/Γ/v 폐형식과 PTSW2C의 b=(1/28,27/28)는 논문 인쇄가 아니라 인쇄된 공식으로부터의 DERIVED이다.

## 4. Startup (Task 3), 원문 §4.3 p.36-37 (PDF 40-41, 텍스트 열람)

- 알고리즘 4.1: 입력 t0, h0, u0=y(t0), u0'=y'(t0). T={c1-1,...,cs-1,0} 정렬(#T는 s 또는 s+1개), `t1 = t0 - h0 T1`, 각 T_i에 대해 `t0 + h0(T_i - T1)` 까지 임의의 표준 integrator로 적분. T_i=0이면 `u1:=u(T_i)`, `T_i = c_j-1` 이면 `k_{0,j} := f(t,u(T_i))`. 마지막에 `h1 = h0`.
- 사용된 starter: "Der Integrator ist beliebig". 논문은 LU 문제에 RADAU, Krylov 문제에 ROWMAP를 사용. 순서/비용: 논문은 starter의 차수 요건을 이 절에서 명시하지 않고, 수렴 정리 Satz 2.1 (p.9)이 `ε0=O(h^p)`, `ν0=O(h^q)` 를 요구한다(PTSW2B: p=q=2). 비용은 "Zeitaufwand des Startens ... vernachlässigbar gering"(Bemerkung 4.2, p.37)이며 정량 수치는 없다; 병렬 실험에서는 sequential startup이 speedup을 제한한다고 언급(§5.5, p.56).
- 대안 (Bemerkung 4.2): ① 매우 작은 h0 + 거친 근사 k0,i=u0', u1=u0 (부정확, 많은 step 증가 → 비추천), ② dense output starter, ③ order control 자기시동. 논문은 알고리즘 4.1로 충분하다고 판단.
- 노드 c_min<0이면 backward 적분을 피하도록 t1을 잡는다; c_max>1이면 t_e+h(c_max-1)까지 해의 존재 가정 필요 (Bemerkung 4.1, p.36).
- PTSW2B에서: T={2-2√3, 0}, t1=t0+(2√3-2)h0 (cert 4절과 일치; 계산 일치 확인).
- **h0 선택 규칙은 논문에 없다**: h0는 사용자 입력("Gegeben sei t0, h0"). 고정 step 실험에서는 `h0 = (t_e - t0)/(N_step - T1)`로 종점을 맞춘다(p.37, 1b)).

## 5. 안정성 / step 비율 (Task 4a)

| 서술 | 내용 | 쪽 | 비고 |
|---|---|---|---|
| 안정성 행렬 | M(z) (2.16), 안정 영역 S={ρ(M(z))<1}; A(α), L(α) 정의 | 12 | **상수 step** 정의 |
| L-안정성 | (2.17) ⇔ b^T1=γ, v^Tγ=b^Tβ (Lemma 2.3); 고차+고단계차수면 steif-genau (Satz 2.3) | 12 | |
| PTSW2B | L(90), ρ(M(∞))=0 (Satz 3.1), Tabelle 3.1 s=2: γ=0.6339, \|C3\|=0.2483, α=90.0 | 17-19, 36 | 나머지 γ=2.3660 은 C 큼 |
| PTSW2C | A(81.1°) (L-안정 아님), ρ(M∞)=0.6796; 비교 변형 γ=1.2는 α=87.7°, ρ=0.9438 이지만 덜 효율적 | 28, 36 | 별도 repo audit가 81.17°, 0.67958 로 수치 재현 (이번에 재실행 안 함) |
| zero-stability | Bemerkung 3.2: z=0에서 λ=1 (단순) + s중 0 고유치 → "Eigenwertentwicklung sichert die Nullstabilität" | 15 | 상수 step만 |
| 변동 step | 계수의 균등 유계성만 요구, σ_m<σmax 이면 성립 (p.8). 안정성 증명은 **없음**; §5.5는 s=4 방법이 σ_m 의존성에 매우 민감하다고 실험적으로 보고 | 8, 56 | "UNRESOLVED: 변동 step 곱 안정성" 유지 |
| Satz 3.3 | 스티프 정확 + B(s),C(s),Γ(s) 방법은 변동 step에서 B(s+1) 불가 → 차수 s. v^T=0 방법은 σ가 B(r)에서 소거되어 차수 감소 없음 (Bemerkung 3.5) | 21 | PTSW2B 차수 2, PTSW2C 차수 3 근거 |
| 구체적 σ 경계 | controller clip [0.5,2] 외의 σ 상한/하한 정리는 없음 | - | |
| 실험적 견해 | 스티프 정확 L(α) 방법이 A(α) v^T=0 방법보다 우수 (PLATE, FEKETE, DIFFU2). PTSW2C는 FEKETE(DAE index 2)에서 안정성 문제 | 50-56 | |

## 6. 수치 실험 (Task 4b), p.47-56 (텍스트 열람; 그림은 이미지 미확인, 정량 데이터 표는 없음, 결과는 Rechenzeit-Fehler 그림)

- 환경: HP/Convex SPP2000 (PA8000), FORTRAN-77, 기준 코드 RADAU(5), RODAS, ROWMAP, VODPK(표준 설정); 오차 척도 Err=sqrt((1/n)Σ((ũ_i-u_ref,i)/(1+|u_ref,i|))²) at t_e (5.1); reference는 RADAU/VODPK, ATOL=RTOL=1e-12 (p.49). 모든 PTSW 실행: ATOL=RTOL=10^{-i}, i=2..8 (p.42).
- LU 문제 (p.49-52): PLATE (n=80), MEDAKZO (n=400), CUSP (n=96), FEKETE (n=160, DAE idx 2). 결과: s=2에서 PTSW2A가 가장 효율적, PLATE에서 PTSW2C는 거친 tol, PTSW2B는 엄격 tol에서 느림; CUSP에서 PTSW2A,2C가 PTSW2B보다 정확; FEKETE에서 PTSW2C 불안정 징후. PTSW2A,3A는 RADAU와 경쟁, RODAS에는 미달 (MEDAKZO). 병렬 speedup은 FEKETE만: PTSW2A 약 1.5.
- Krylov 문제 (p.52-56): NILIDI (n=10000), DIFFU2 (n=10000), BRUSSELATOR (n=20000). 요약(p.56, §5.5): ρ(M∞)=0인 PTSW2B/3B가 Krylov residual 허용치에 더 낮은 비용; PTSW3B가 최고; DIFFU2에서 순차 PTSW2B는 거친 tol에서 최고; PTSW2B-PAR/3B-PAR가 모든 정밀도에서 VODPK보다 효율적; speedup 약 1.5/2.0/2.5 (s=2/3/4, DIFFU2), 결론부 "최대 1.6/2.5 (2/3 프로세서)" (p.60 근방).
- **관측 차수 표, 고정 step 수렴표: 이 논문에 없음.** 수치 차수 확인은 인용 문헌([37] report)과 PTSW4B-FIX 효율 비교만; 5단계 시험방법에서 "차수 5 수치 확인" 문장이 p.29에 있음.
- 비교 해석 주의: 효율은 HP 2000년대 하드웨어의 CPU time 기반(논문 자체가 플랫폼 의존성 경고, Beispiel 5.1, p.48). 현대 CPU 성능 주장의 근거가 될 수 없다.

## 7. 상수별 판정 (Task 5) 및 PW02 모듈 대조

범례: SOURCE_PRINTED_THESIS(쪽) / DERIVED / NOT_IN_THESIS(OWNER_CHOICE). 코드 열은 `crates/rodas5p-integrators/src/ptsw2b.rs` (읽기 전용).

| # | 상수 | 판정 | 논문 값/출처 | 코드 현재값 | 일치? |
|---|---|---|---|---|---|
| C1 | safety | SOURCE_PRINTED_THESIS p.42 | 0.85 (clip 안쪽 곱) | `SAFETY=0.85` | 일치 |
| C2 | ratio clip | SOURCE_PRINTED_THESIS p.42 | [0.5, 2] | `RATIO_CLIP=(0.5,2.0)` | 일치 |
| C3 | hysteresis band | SOURCE_PRINTED_THESIS p.42, **모든 PTSW 방법 공통** | σ*∈[0.95,1.05] → h*=h_m (닫힌 구간) | `HYSTERESIS_BAND=(0.95,1.05)`, `contains` 닫힌 구간 | 일치 |
| C4 | growth history | SOURCE_PRINTED_THESIS p.42 | σ*>1.05 & ∃ j∈{0..s-1}: σ_{m-j}>1 → h_m 유지 (s=스테이지) | `HYSTERESIS_HISTORY=2`, 현재 σ 포함한 최근 2개 `any(>1)`, band 상단 초과일 때만 | 일치 (현재 시도 σ를 tail에 push한 뒤 호출: j=0 포함) |
| C5 | error norm | SOURCE_PRINTED_THESIS p.42 | RMS, 분모 ATOL+RTOL\|u_{m+1,i}\| | `error_norm` 동일 | 일치 |
| C6 | tolerance 형태 | SOURCE_PRINTED_THESIS p.42 | atol+rtol·\|u_{m+1}\| (새 해) | 동일 | 일치 |
| C7 | exponent | SOURCE_PRINTED_THESIS p.42 (-1/s) + DERIVED(=−1/(p_e+1), p_e=s−1) | s=2 → -1/2 | `err.powf(-0.5)` | 일치 |
| C8 | accept rule | SOURCE_PRINTED_THESIS p.42 | Err ≤ 1 | `accepts: err<=1.0` | 일치 |
| C8b | reject 처리 | SOURCE_PRINTED_THESIS p.42 | 같은 step을 h_m:=h*, σ_m:=h*/h_{m-1} 로 반복, 별도 factor 없음 | `Rejected{next_ratio=propose_ratio}`, 상태 불변 | 일치 (σ*≤0.85이므로 hysteresis 무관) |
| C8c | NaN | SOURCE_PRINTED_THESIS p.42 (Bem. 4.6) | h:=h/2, reject | NaN→`RATIO_CLIP.0`=0.5, `accepts(NaN)`=false | 일치 |
| C8d | Err=0 | NOT_IN_THESIS (극한: Err^{-1/s}→∞ → clip 2) | - | `err<=0 → 2.0` | 수학적 극한과 일치, 인쇄는 아님 |
| C9 | 초기 step h0 | NOT_IN_THESIS(OWNER_CHOICE) | 사용자 입력 (Alg. 4.1 "Gegeben ... h0") | 호출자 제공 | 일치(규칙 없음) |
| C10 | starter | 논문: 임의 integrator (RADAU/ROWMAP) p.37 | ROS2 one-step starter (owner option S5), 비논문 | 문서화된 owner 선택 | 불일치 아님(허용 범위, 단 O(h^2) 요건 충족 여부는 코드 테스트에 의존; 논문 요건 Satz 2.1 p.9) |
| C11 | ceiling 1.10 vs RODAS5P | NOT_IN_THESIS(OWNER_CHOICE) | - | `ACCEPTED_STEP_CEILING_VS_RODAS5P=1.10` | 해당 없음 (D2 별도) |
| C12 | hmin/hmax, 연속 거부 상한, σ 하한 제어 | NOT_IN_THESIS(OWNER_CHOICE) | 없음 | 코드 상수 없음 | 해당 없음 |
| C13 | estimator b_e,v_e | DERIVED from printed (4.7) p.41 | b_e=0.95b, ε=0.1 | `b_embedded`, `v_embedded` (cert 5.1) | cert와 일치 (§3.2); 코드 값 자체는 이번에 재검증하지 않음 (Rust 미실행) |
| C14 | PTSW2C controller | SOURCE_PRINTED_THESIS p.42 (동일 규칙, s=2) | 0.85, [0.5,2], band [0.95,1.05], history 2, exp -1/2 | PW02는 PTSW2B 전용 | PTSW2C 구현 시 같은 상수 사용 가능 (OWNER 승인 하에); embedded 차수 1 확인 |

**불일치 목록 (정밀):**
- 상수 수준 불일치: **없음** (SAFETY, RATIO_CLIP, HYSTERESIS_BAND, HYSTERESIS_HISTORY, exponent, norm, NaN 처리 모두 일치).
- 비상수 차이 (정보, 수정 요구 아님):
  1. starter: 논문 RADAU/ROWMAP 임의 integrator vs 코드 ROS2 1-step (owner S5). 논문은 starter 차수 요건을 알고리즘 절에 쓰지 않고 정리 요건(O(h^p))만 둔다.
  2. 문서 `PW02_SCAN_AND_RED_TEST_KO.md`는 위 4개 상수 이름과 값을 직접 서술하지 않음(grep 결과 해당 상수명은 19행의 목록과 39행의 UNRESOLVED 항목뿐); 값은 `ptsw2b.rs` 주석 "CERT sec. 5"로 추적되며 certificate §5 = 논문 p.42 와 일치.
  3. 코드 주석은 hysteresis를 "CERT sec. 5"로만 귀속; 이제 1차 출처 "Podhaisky 2002 p.42"를 추가할 수 있다 (코드 수정 요청 아님).
  4. 이전 문서 `PW01_CONTROLLER_CONSTANTS_RECOVERY_KO.md` §4-§7의 "SECONDARY_ONLY / OWNER_CHOICE 비문헌" 라벨 중 C1-C7, C3/C4(PTSW2C 포함)는 본 문서로 대체해야 한다. "hysteresis는 차수/안정성에 필요 없는 heuristic"이라는 이전 구조적 추론은 논문이 목적을 "häufige Schrittweitenwechsel vermeiden, Robustheit, LU 재사용"으로 서술하는 것과 일치한다 (p.41-42). 논문은 이를 안정성 증명으로 주장하지 않으며, 변동 step 민감성을 완화하는 휴리스틱으로 사용한다.

## 8. UNRESOLVED (이 논문으로 해결 불가)

1. 변동 step 곱 안정성 증명, σ 허용 구간에 대한 정량 정리 (논문: σ<σmax 균등 유계성만 p.8).
2. 초기 step h0 선택 알고리즘, hmin/hmax, 연속 거부 횟수 상한 (논문에 없음 → OWNER_CHOICE).
3. PTSW2A/B/C의 관측 차수 표, work-precision 수치표 (논문은 그림만; 그림은 이미지 미검토, 수치 추출 안 함).
4. PTSW2B/C A(α), L(α)의 완전 sector 증명 (논문은 수치 계산 α, 이번에도 재계산 안 함. 별도 PTSW2C audit만 존재).
5. starter 필요 정확도의 논문 내 명시 (Satz 2.1의 ε0=O(h^p), ν0=O(h^q) 외 없음; 코드 쪽 starter가 O(h^2)/O(h^3)을 만족하는지는 Rust 테스트 영역, 이번 미실행).
6. Table 4.1 PTSW2B 노드 열의 3번째 값(0.63397459)이 조판 오류라는 판단은 §3.2 p.17과 φ* 근 계산에 근거한 DERIVED; 저자 확인 전까지 "표 이상"으로 표시.
7. Kulikov/Weiner 등 외부 문헌, 논문 [37] report (수렴 차수 수치 확인) 미열람.

## 9. 실행 영수증

| 항목 | 명령 | 결과 |
|---|---|---|
| 텍스트 추출 | `pdftotext -layout Podhaisky_2002.pdf <scratchpad>/thesis/t.txt` | 3850줄, 76쪽 |
| 이미지 | `pdftoppm -r 300 -png -f {40,45,46}` | pg-40/45/46.png; Read로 눈 검증 (sha256: 40 `0d5c6ece...`, 45 `f469d3d7...`, 46 `02f3ab7c...`) |
| 정확 대수 | `python3 -I <scratchpad>/verify_thesis.py` (sympy 1.12), sha256 `dc2bffbd...45cf` | 출력: PTSW2B A/Γ/v/v_e diff 모두 0; PTSW2B embedded B(1)=0, B(2)=σ/20; PTSW2C A/Γ/v_e diff 0, b·B(1..3) residual 0, embedded B(1)=0,B(2)=σ/20; PTSW2A가 논문 A.5 Fortran과 일치 |
| 실행하지 않은 것 | Rust build/test, ρ(M∞)/α 재계산, 그림 수치 추출, 외부 문헌 | NOT_RUN |
스크립트는 scratchpad에 있고 저장소/PDF에는 복사하지 않았다. PASS 판정은 위 sympy 항등 검사에 한정되며 그 외 항목에 PASS를 쓰지 않았다.
