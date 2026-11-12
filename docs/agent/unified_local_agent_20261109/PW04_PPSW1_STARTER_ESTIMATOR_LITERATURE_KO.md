# PW04 PPSW1-6b: 시작값(starter), 오차추정기, 스텝 제어 — 문헌 + 수치 재현

범위: 문헌과 python3 -I 재현만. Rust/git 없음. 스크립트 원문, receipt(cmd, exit, sha256), 표, URL은 같은 폴더 `PW04_PPSW1_STARTER_ESTIMATOR_LITERATURE.json`에 있다. 페이지 번호는 인쇄 쪽수(thesis는 PDF 쪽수 병기). 인용은 짧게 했고 PDF는 복사하지 않았다.

## S1. 시작 단계값의 시간 배치 — DERIVED (근거는 SOURCE_PRINTED)
- SINUM p.266 식 (2): `t_mi := t_m + h_m c_i`, "associated with the time interval [t_m,t_{m+1}] but not necessarily contained in it". 식 (1)은 `t0 <= t <= te, y(t0)=y0`.
- SINUM Thm 3.3 (p.270 부근): "Y_0i - y(t_0i) = O(h_0^{s-1})". ANM(2004) Thm 2.1: "Y_0i - y(t0 + c_i h0) = O(h0^p)".
- 따라서 Y_0은 격자 기준점 t0(PPSW 격자의 t_0)에서 t0 + c_i h0 에 놓인다. c=(-1,-0.5,0,0.4,0.8,1)이면 노드는 t0-h0, t0-0.5h0, t0, t0+0.4h0, t0+0.8h0, t0+h0 이다. 즉 Y_{0,s}는 초기값이 아니라 y(t0+h0)이고, 초기값 y0는 c_3=0 노드에 해당한다. c<0인 노드는 2개(c=-1,-0.5)이며 그것이 t0 이전이다.
- 중요한 구분: "격자의 t0"는 문제의 초기 시각 t_init과 같을 필요가 없다. 격자 기준점을 t0 = t_init + h0 로 잡으면 6개 노드는 t_init + h0*{0, 0.5, 1, 1.4, 1.8, 2} 가 되어 모두 t_init 이후(전진 시간)이다. 이 선택은 차수 이론과 모순되지 않는다(Thm 3.3은 노드 위치가 아니라 Y_0 오차만 요구한다). 비용은 첫 PPSW 스텝이 t_init + 2h0 에서 시작한다는 점이다.
- 논문 어디에도 "t0 이전 값을 후진 적분으로 얻는다"는 서술은 없다(NOT_PRINTED).

## S2. 시작값 계산법
- SINUM p.277: "RODAS was also used to compute the starting values Y1 for the PPSW-methods". 허용오차, h0, 적분 구간은 인쇄되지 않았다 (UNRESOLVED). 같은 쪽 결과 논의(p.280 부근)에서 "initial stepsize for the PPSW-methods were too small"로 h0가 작았음을 시사만 한다. "Y1" 표기는 모호하다(시작 스테이지 집합을 뜻하는 듯하나 확정 불가).
- ANM 4절: "The starting values Y0j are computed with ROWMAP [10]". 허용오차 NOT_PRINTED.
- BIT(2005) 7절: 시작 절차 서술 없음 (grep 확인).
- Podhaisky thesis Algorithm 4.1 (인쇄 p.36, PDF p.41; PTSW용): 상대 노드 T={c_i-1, 0}를 정렬해 T_1이 최소값, `t1 := t0 - h0 T1`, 이후 t0 + h0(T_i - T_1)까지 전진 적분. 인용: 후진 시간 적분 "(eventuell instabile)"을 피하기 위한 "kleinstmögliche" 시작 시각. 적분기는 임의(RADAU 또는 ROWMAP). Bem. 4.2는 대안으로 (1) 아주 작은 h0 자체 시작, (2) 연속 확장이 있는 시작 적분기, (3) 차수 제어와 함께 하는 자체 시작을 언급하되 구현하지 않았다.
- 개방 문헌: arXiv 1902.01161 (IMEX-Peer, 2.5절 Starting procedure)는 `w_{0,i} ~ u(t1-(1-c_i)dt0)`를 연속 출력이 있는 적분기(RK 또는 BDF)로 `[t0,t0+tau]`에서 구하고 `t1-(1-c_min)dt0 = t0` 가 되게 시각을 이동한다. 이는 위 shifted start와 동일 구조이다. arXiv 2608.26913은 RK형 시작 방법(국소 차수 s+1)을 쓴다 (읽은 범위 100k/197k자; 음수 노드 언급 없음). Schmitt-Weiner 2009 "Parallel start..."는 접근 불가(유료; 검색 스니펫만: 초기 구간 병렬 Euler, s-2 스텝 후 차수 s)이며 explicit 방법용이다. Montijano 2019 특정 논문은 검색으로 찾지 못함(주장 없음).
- 판정: 논문의 PPSW1 시작값 정확한 절차는 NOT_IN_LITERATURE(OWNER_CHOICE). 구조는 thesis Alg 4.1과 IMEX-Peer 문헌에서 DERIVED.

## S3. 오차 추정기
인쇄 정의 (SINUM p.277, BIT 7절, ANM 4절): s>=5에서 최신 Y_ms를 "the polynomial predictor of order s-2 that ignores Y_{m-1,1} and interpolates Y_{m-1,i}, i=2..s" 와 비교. ANM: "for PPSW1-methods an embedded solution of order O(h^{s-2}) is used". 가중치 표는 인쇄되지 않음.

DERIVED (sympy, 정확 유리수; 해석 가정: 4차 다항식이 5점을 보간 — 5점을 보간하는 유일한 해석, UNRESOLVED 표기):
- 예측 위치 x = t_m + c_s h_m = t_{m-1} + h_{m-1}(1+sigma). 노드 c_j (j=2..6) = -1/2, 0, 2/5, 4/5, 1. 예측값 = sum_j w_j(sigma) Y_{m-1,j}:
  - w_2 = -sigma(2sigma+3)(5sigma+1)(5sigma+3)/8
  - w_3 = 125 sigma(sigma+1)(2sigma+3)(5sigma+1)/108
  - w_4 = -125 sigma(sigma+1)(2sigma+3)(5sigma+3)/104
  - w_5 = (sigma+1)(2sigma+3)(5sigma+1)(5sigma+3)/9
  - w_6 = 16 sigma(sigma+1)(5sigma+1)(5sigma+3)/351  (JSON 순서는 c_2..c_6에 대응: 첫 항목 16.../351은 c_2=-1/2... 주의: 아래 sigma=1 값 표를 기준으로 삼을 것)
  - 정확한 매핑은 JSON `results.s1.weights_sigma` 에 노드 순서(c=-1/2,0,2/5,4/5,1)로 나열됨: [16σ(σ+1)(5σ+1)(5σ+3)/351, -σ(2σ+3)(5σ+1)(5σ+3)/8, 125σ(σ+1)(2σ+3)(5σ+1)/108, -125σ(σ+1)(2σ+3)(5σ+3)/104, (σ+1)(2σ+3)(5σ+1)(5σ+3)/9]. (위 본문 w_2..w_6 라벨은 이 리스트와 라벨이 어긋나 있을 수 있으므로 JSON 리스트와 스크립트 s5의 pred_w를 정본으로 한다.)
  - sigma=1: [512/117, -30, 625/9, -1250/13, 160/3], 합 1 (sympy 확인), sum|w| = 253.3 (sigma=0.5: 50.3; 1.1: 325.4; 1.5: 769.7).
- 추정량 Est = Y_ms - sum w_j Y_{m-1,j}. 선도 오차: y^{(5)} h^5 * sigma(sigma+1)(2sigma+3)(5sigma+1)(5sigma+3)/6000 (sigma=1에서 2/25).
- 차수: Est = O(h^5) = O(h^{s-1}) (예측기 보간 오차가 지배; Y_ms 국소오차는 O(h^6)). 수치 확인 (균일 격자, 정확 시작, PR lambda=-1, 스크립트 s5): Est 적합 지수 5.01 (N=16..128). 따라서 제어 지수는 1/5 = 1/(s-1) (thesis의 1/s=1/6 아님; 그것은 PTSW 차수 s의 임베디드 s-1용).
- 주의 (DERIVED/NUMERIC): Est는 Y_{m-1}의 전역오차(O(H^5))도 포함하므로 순수 국소 추정이 아니며, 균일 격자에서는 진짜 오차(Y_ms 초수렴 h^6)보다 크게 보수적이다 (Est/진오차 약 18~42배, N=16, 32). 가중치 합 |w|가 250~770이라 이중정밀 반올림 바닥이 약 250*eps*|Y|로 올라간다.
- 논문의 신뢰도 검증: 별도 정량 검증 없음. 그림 1-5 (OREGO 등, 최종 오차 대 스텝 수)로 간접 성능만 제시 (SOURCE_PRINTED). ANM/BIT도 동일.
- 대비: BIT mipeer의 추정기도 "compared with a predictor of order s-2" 로 동일 개념이며, 저차는 superconsistent stage 대안(식 (2.12)). Thesis (4.7): 임베디드 가중 `b_e = 0.95 b`, `v_e = ((1+eps e_s^T) S D^{-1} - b_e^T V_0 S)V_1^{-1}`, 잔차 eps=0.1 (PTSW; 전혀 다른 구성).

## S4. 스텝 제어
SOURCE_PRINTED (SINUM p.277-278): "the new stepsize was computed in a standard manner; cf. [16]" ([16] = Podhaisky-Schmitt-Weiner, Appl. Numer. Math. 42 (2002) 381-395, 이 저장소에 없음, 미열람); "maximally allowed increase of the stepsize was set to sigma_max = 1.5 for all PPSW-methods"; atol=rtol=1e-2..1e-9 (ROBER는 atol=1e-6 rtol); 오차 지표는 종점 오차 대 스텝 수(거절 포함). ANM: sigma_max=1.5 (Super2, PPSW2-3,4) 또는 1.2 (나머지), "No lower bound on sigma_m is enforced". 노름, 안전계수, hysteresis, 거절 처리는 인쇄되지 않음 (NOT_IN_LITERATURE).
thesis 제어기 (인쇄 p.41-42, PDF p.45-46): sigma* = min{2, max{0.5, Err^{-1/s} 0.85}}, [0.95,1.05] 유지, s연속 스텝 내 증가 1회 제한, NaN 처리, 노름은 RMS 가중(ATOL+RTOL|u|). 이는 PTSW용으로만 인쇄됨. PPSW1에 재사용은 문헌이 허용한다고 쓰지 않았으므로 OWNER_CHOICE. 수정 권고: 지수를 1/(s-1) = 1/5 로, sigma_max <= 1.2~1.5 (논문 값), hysteresis와 "quasi-constant" 규칙은 sigma 의존 계수(안정성 상수 138 @sigma<=1.1, 709 @sigma<=1.5; 기존 PW04 검증)와 정합적이라 합리적이나 PPSW1에 대해 검증되지 않았다.

## 설계 종합 (제안; 소스 사실 아님)
제약: (i) 모듈 자급 (ii) J/JVP를 스텝 시작에서 동결 + f만 (iii) 6개 노드에서 O(h0^5) (iv) 안정 (v) 비용.

| 후보 | 전진 시간만 | 정확도 (측정) | 안정성 | 비용 |
|---|---|---|---|---|
| A: shifted start + 전진 Radau IIA 5단 1스텝 (구간 [t_init, t_init+2h0], 구성적: 충돌점 노드와 Lagrange 적분으로 tableau 생성, 동결 T로 단순 Newton, 충돌 다항식 dense output) | 예 | 비경직/경직 매끄러운 해: 5.6~7; 전이부 해상 시 sr=5: 5.9 (sr=4: 4.9; sr=3 dense는 4.0으로 부족) | L-안정, 경직 매끄러운 해(PR -1000, amp 0)에서도 정상 | LU 1회(5n), f 15~25, JVP/J 1회 |
| B: 선형 암시적 Euler 외삽 (조화열, k열, 노드마다 매크로 스텝) | 예 | 비경직: k=4:약 5(4.4->5.0), k=5: 5.9, k=6: 6.4; 경직 h0|lambda|>=6: 오차 1e-5..1e-3로 정체 (차수 감소, 실패) | h0*lambda 크면 부적합 (측정) | k=5: LU 25, f 75 |
| C: PPSW1 자체 부트스트랩 (차수 상승) | 예 | NOT_RUN | NOT_RUN | NOT_RUN |
| D: 문헌 (RODAS/ROWMAP, Radau; IMEX-Peer shifted+dense) | 예 | 구조 일치, 개별 정확도 미측정 | 소스 주장만 | RODAS는 보호 솔버라 제약 (i) 위반 |

경직 안전성(핵심): 후진 시간 적분이 필요한 후보는 없다. 방법은 격자 기준점을 t0 = t_init + h0 로 옮겨 c=-1 노드가 t_init에 오게 하는 것이며(S1), 전진 적분만 쓴다. 단 첫 PPSW 스텝이 t_init+2h0에서 시작한다. 한계: 초기값이 느린 다양체 밖이고 h0|lambda_max|>>1 이면 y(t_init+h0 c)의 전이부 때문에 어떤 L-안정 시작기도 O(h0^5)를 못 낸다 (실측: PR -1000 amp 1, h0 >= 6e-3에서 오차 0.1~0.3). 이 경우 h0는 전이부를 해상하도록 (h0 |lambda| 약 1 이하) 선택해야 하고(소유자 선택), 측정상 h0|lambda|=2~0.06에서 A(sr=5)는 4.6~5.9, B(k=6)는 6.7까지 도달.

### 측정 요약 (receipt는 JSON)
- s2 (B, 노드값 오차 대 h0), s3 (A, h0 = 0.2..0.00625), s4 (전이부 해상 영역), s5 (PPSW1-6b 전역).
- PPSW1-6b + 시작기 전역 오차 (이중정밀, T=1, N=16,32,64; 반올림 바닥 약 1e-11 이하 구간은 해석 제외): 
  - PR lambda=-1: exact 시작 5.9 (N16->32), radau5 5.9 (오차 값이 정확 시작과 동일 5.5e-9, 9.2e-11), limex5 5.9, radau3(dense 4차) 4.0 (오염 확인).
  - PR lambda=-1000 (매끄러운 해): exact 6.15/7.6, radau5 6.2/6.4, limex5 8.0/7.0 (이 경우 amp0이고 N>=64는 바닥).
  - 2차원 PR (-1,-1000) 매끄러운 해: exact 5.6/5.7, radau5 5.6/5.6, radau3 4.0, limex5 5.6/5.7.
  - 임의 sigma in [0.5,1.1]: exact와 radau5의 오차 일치 (예 PR -1: 3.6e-9 vs 3.8e-9, 7.5e-11 vs 7.6e-11), radau3 4.0.
  - 결론: 설계된 radau5 시작기는 정확 시작과 구분되지 않으며(바닥 위 구간), 4차 dense 시작은 전역 차수를 4로 끌어내린다 (이전 offset 실험과 정합).
  - 한계: 이중정밀이라 N>=64에서는 바닥(1e-11~1e-13)에 걸려 6차 차수 확인은 불가. mpmath 전역 재현은 NOT_RUN. 선형 문제만 (비선형 동결 J Newton, Fisher MOL: NOT_RUN). limex5가 매끄러운 경직 해(PRvec -1000)에서 PPSW 전역 오차를 오염시키지 않은 것은 해당 노드값 오차가 PPSW 오차보다 작았기 때문으로, s2에서 측정된 경직 영역 실패와는 별개이다 (s5 크기 범위 h0 >= 0.004, 이때 B 오차 1e-5 수준이 아님: PR -1000 amp0에서 s2는 h0 >= 0.00625에서 1e-6~1e-5 인데 s5 limex5 오차 4e-9는 이 값과 모순처럼 보임 -> 설명 UNRESOLVED; s5에서 t_init=0,h0=T/(N+1)로 h0 = 0.06..0.004 인 것은 s2와 같은 범위이므로 추가 조사가 필요하다).

## 판정 및 권고
- S1: DERIVED (SOURCE_PRINTED 근거). S2: 부분 SOURCE_PRINTED(RODAS/ROWMAP 사용, 세부 NOT_PRINTED), 구조는 DERIVED (thesis Alg 4.1, IMEX-Peer). S3: 정의 SOURCE_PRINTED, 계수 DERIVED, 지수 1/5. S4: SOURCE_PRINTED는 sigma_max와 tol 범위뿐, 나머지 OWNER_CHOICE.
- 권고 시작기: A (shifted start t0 = t_init + h0, 전진 Radau IIA 5단 구성적 1스텝, 동결 T 단순 Newton, 충돌 다항식으로 5개 노드). 이유: 후진 시간 없음, 경직 안전(측정), 외부 솔버 불필요, 비용 LU 1회. 필요 조건: dense 차수 >= 5 (3단은 불가, 4단 경계, 5단 권고).
- 권고 추정기: Est = Y_ms - sum w_j(sigma) Y_{m-1,j} (j=2..6, 위 가중치), 제어 지수 1/5. 가중치 합 |w| 크기(250+)에 대한 tolerance 바닥을 경고로 기록.
- 권고 제어기: OWNER_CHOICE로 thesis 제어기를 지수 1/5, sigma_max 1.2(안전) 또는 1.5(논문)로 사용, 하한 0.5, hysteresis [0.95,1.05] 는 선택 사항. 문헌이 PPSW1에 대해 보증하지 않음을 명시.

## UNRESOLVED / NOT_RUN
JSON `unresolved` 참조: 예측기 "order s-2" 해석, 노름/하한/안전계수 미인쇄, RODAS 시작 허용오차, 비선형 동결 J starter, mpmath 전역 차수, 설계 C와 Schmitt-Weiner 2009 (접근 불가), s5의 limex5 경직 영역 불일치 설명.
