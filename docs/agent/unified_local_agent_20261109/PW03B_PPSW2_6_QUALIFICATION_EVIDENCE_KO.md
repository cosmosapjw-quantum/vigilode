# PW03B: PPSW2-6 자격 증거 (문헌 + 수치 재현, 2026-11-11)

범위: 문헌/정확-수치 재현만. Rust/git 없음, 기존 파일 미수정, `python3 -I`. 기계판독본 `PW03B_PPSW2_6_QUALIFICATION_EVIDENCE.json`(스크립트 전문, stdout 포함). 모든 수치는 NUMERICALLY_CHECKED이며 증명이 아님. 합성 문제 프로토타입은 qualification이 아님.

## 0. 판정

**QUALIFICATION_STATUS = CANDIDATE_FOR_PW04 (조건부)**. G0 게이트는 전부 NOT_RUN(실제 IVP 배터리 없음): global order>=4.8 (합성 프로토타입 ~5는 지지 증거일 뿐), step ratio<=1.2, wall speedup>=1.15, depth p95<=6, fallback<=0.1, fast lane Newton/explicit-J 0. 이는 PW06 이후에만 획득 가능.

선결 조건: (1) **sigma_max=1.2는 적대적 열에서 불안정**(아래 1절) -> sigma_max<=~1.15로 제한하고 긴 단어/실제 컨트롤러 열로 재검증; (2) 시작값 절차 확정(2절); (3) T/J 허용 오차(5절); (4) 오차추정기 스케일/컨트롤러(타 에이전트); (5) PW06 full-IVP matched-error 벤치.

## 1. (i) 스텝비 영역과 가변스텝 zero-stability

문헌(APNUM 48 pp.432-434): Thm 3.2는 strongly stable이면 [sigma_a,sigma_b], sigma_a<1<sigma_b 존재만 증명(증명: B(1)을 Jordan 변환 T로 `||T^-1 B(sigma) T||_1 = 1` 이 sigma 근방에서 성립; 연속성 논증). 수치 구간은 인쇄되지 않음("bounds ... can be rather close to one"). Fig.1은 Maple Jordan 형태로 계산한 노름(3.10) vs sigma (PPSW2-3,4,5만; **2-6은 없음**), Fig.2는 무작위 열(sigma=0.1+2r를 [2-smax,smax]로 클립, 10000 스텝)에서 곱 원소 최대값 bnorm vs smax. 구현상 한도 sigma_max=1.5 (Super2, PPSW2-3,4), 그 외 1.2, 하한 없음(p.434). **그림 자체는 이번에도 열람하지 않았음(텍스트만): 그림 수치 UNRESOLVED.** 즉 1.2는 PPSW2-5/6에 대해 "Fig.2에 맞춰 경험적으로 선택"이며 증명된 값이 아님. 또한 Jordan 논증은 (3.10) 노름 조건이 ||.||_1 기준 sufficient일 뿐 sigma_b의 크기를 주지 않음.

재구성 tableau 수치(z1, z2; float64 + mpmath 50자리 확인):
- 점별: B(sigma)의 비-1 고유치 최대 모듈러스 <1 인 연속 구간 [0.05,1.53] (sigma=1: 0.8936, 1.2: 0.9486, 1.53 이후 >1). 단, 이는 고정 sigma 조건.
- 무작위 균일/정점 열(6000 스텝): [0.8,1.2]에서 곱 최대원소 ~1.5e2 (유계처럼 보임). 그러나 **교번 열 sigma=0.8,1.2 는 8.8e3, [0.8,1.3]은 7.5e285**.
- **발산 열 발견**: 스텝이 h, r h, h, r h ... 로 진동(sigma = r, 1/r 교대)할 때 두 스텝 곱의 비-1 스펙트럼 반경^(1/2): r=1.10: 0.934, 1.15: 0.971, 1.18: 0.994, **1.19: 1.002, 1.20: 1.0105** (mpmath 50자리도 1.01048154501664 로 일치). 일반 쌍 최악값(격자): smax=1.16: 0.989, 1.18: 1.008, 1.20: 1.028 (sigma_a 0.1~0.8 무관, 쌍은 약 (1.2, 0.92)). 길이 3,4 단어(sigma_a=0.8): smax=1.15에서 0.980 이내, 1.2에서 1.027. 길이<=7, 5문자 알파벳에서 [0.8,1.1]은 최대 0.9474(<1), [0.8,1.2]은 1.027(>1).
- 해석: 인쇄 sigma_max=1.2는 sigma 열 {1.2, ~0.92} 반복에서 성장률 ~1.03/step(수백 스텝 시 큰 증폭) -> **(2.6)이 [., 1.2]에서 성립하지 않음(NUMERICALLY_CHECKED, 증명은 아님: 해당 단어 2개로 반례 구성은 사실이나 격자 한정)**. 실제 컨트롤러 열이 이를 밟는지는 미검증. 고정 sigma=1.2의 alpha=32.7도, B 원소 ~32(PW03A와 정합; sigma=1.33에서 44.5, 1.5에서 66).
- 권고 안전 한도: sigma_max<=1.15 (쌍/짧은 단어 ρ<1, 긴 단어 미검증). 하한은 문제없음(sigma->0에서 고유치 감소; 쌍 검사 0.1~0.5 포함).

## 2. (ii) 시작값 (c1=-1.702, c2=-0.51)

문헌에서 확인된 사실만: 방법은 Y_mi ≈ y(t_m + h_m c_i), t_m = t_{m-1}+h_{m-1} (APNUM p.427 및 (3.2)의 서브그리드 설명, (3.5)의 1+sigma c_i)이므로 Y_0j는 t_0+c_j h_0 의 값. Thm 2.1은 시작값 오차 O(h_0^p) 요구(p.428). 실험에서 시작값은 APNUM p.434 "computed with ROWMAP [10]", SINUM p.277 "RODAS was also used to compute the starting values Y1". **c<0 노드 처리, 시작기의 차수/허용오차/비용은 세 논문 어디에도 인쇄되지 않음(UNRESOLVED).** BIT는 시작 절차를 언급하지 않음(grep 결과 없음).
추론(문헌 아님): 노드가 모두 t_0+c_j h_0 이므로, IVP 시각 t_ivp = t_0 + c_1 h_0 로 두고 전방 적분하여 구간 (1-c_1)h_0 = 2.702 h_0 에서 s개 시점 값을 얻는 것이 자연스러움. 시작기 오차는 B^m->B^inf(고유치 1) 때문에 비소멸 전파되므로 O(h^5) 이하 필요: 프로토타입(Riccati, 균일격자, 시작값에 h^q 섭동)은 q=3 -> 관측 3.4~3.9, q=4 -> 4.5~5.2 (z4). 비용은 sigma_start 이전 ~3 스텝 분량의 고차 원스텝 솔버(예 RODAS5P) 호출 한 번 + s-1개 dense 값; 인쇄 근거 없음.

## 3. (iii) 논문들의 수치 실험 (추출 가능한 사실)

- **APNUM 48** (Figs 3-6, p.436-437): 정밀도-시간 선도(그림)만. DIFFU(n=10^4, t<=1), NILIDI(n=10^4), FOOD(n=2x10^4, t<=3), BRUSSELATOR(n=2x10^4, t<=10); 8 코어 SunFire에서 s 프로세서, FOM Krylov rktol=0.1(차수 2,3)/0.001(4,5), 비교: TSW3B, VODPK. **관측 차수 수치, 표, RADAU5/RODAS 비교는 없음**; 본문 결론: PPSW2가 PPSW1/순차 방법보다 효율적, 초consistent(order s)는 안정성 때문에 비효율. PPSW2-6이 어느 곡선인지는 텍스트로 확인 불가(그림에 PPSW2-3,4,5 위주; 4번 항목 포함 여부 UNRESOLVED).
- **SINUM 42** (Figs 1-5, p.277-281): OREGO(te=360), ROBER(te=1e8), VDPOL(te=11), PLATE, KREISS; atol=rtol=1e-2..1e-9; 로그 종단오차 vs 스텝 수(기각 포함); 기준해 RADAU5, 비교 RODAS(스텝 수 비교; "RODAS per-step work ~ six-stage PPSW 순차"). 결론 텍스트: RODAS는 느슨한 허용오차에서 명백히 우월(스텝 증가가 빠름), 중간/엄격에서는 고차 PPSW(6단 포함)가 종종 우월. 방법 6b(alpha 57.5), 6c(87.9) 등 단일-gamma(PPSW1) 방법이며 PPSW2-6이 아님. **관측 차수 수치는 인쇄되지 않음**(그림 곡선 기울기 미추출).
- **BIT 45** (Figs 7.1-7.3): DIFFU, RADIATION; mipeer3,4,5(s-1 차)와 VODPK/ROWMAP/RKC 비교; "speed-up near optimal s". mipeer6는 "no further improvement"라서 결과 생략(p.213). 관측 차수 표 없음.
- 결론: 3논문 어디에도 PPSW2-6의 **측정된 global order 수치는 인쇄되지 않음**. 따라서 >=4.8은 문헌으로 닫히지 않음(프로토타입 5절 참조).

## 4. (iv) 오차추정기 (3.7)

인쇄 내용: `Y_ms - sum_j theta_sj Y_{m-1,j}` (APNUM (3.7), p.431; 구현 p.434 "error estimation is done for Y_ms by (3.7) with PPSW2-methods"). 원문 주장: Theta 외삽의 마지막 성분은 O(h^{s-1}) 근사이고 stage s는 (3.6)으로 초일관(O(h^s))이므로 "higher order estimate for the local error". 인쇄되지 않은 것: 스케일 상수, 노름, 허용오차 정규화, 가변 sigma에서의 보정. SINUM의 유사 논의(p.277): K_m=Y_m-Theta Y_{m-1}은 일반 stage에서는 차수 s-1이고 한 stage만 order-s이면 "asymptotically correct order-s estimate"; s>=5 방법은 대신 차수 s-2 예측자(Y_{m-1,1} 무시) 사용(SINUM p.277), BIT도 s-2 예측자 또는 초일관 stage 언급.
수치(z7, Riccati, 정확 이전값, sigma=1과 1.2): 한 스텝 추정량 |E| 는 h^6로 감소(관측 지수 6.00), stage s 실제 국소오차는 h^7(6.96~6.99; 이는 일반 stage 국소 h^6 예상보다 높음: PW03A의 AB(7) k=6 잔차 비영과 모순처럼 보이나 원인 미규명, 이 문제의 구조 효과 가능성; UNRESOLVED), |E|/실제오차 = 9.5 -> 140 (h 감소에 따라 ~1/h로 증가). 즉 E는 order-5 예측자 오차 규모를 추정하는 보수적 추정량이며 Y_ms 자체의 오차의 점근적으로 올바른 추정량이 아님(이 합성 문제 한정).

## 5. (v) 재현 가능 실험 + 프로토타입 결과

프로토타입(합성, 정확 시작값, 컨트롤러 없음, mpmath 40자리, `z4/z5/z6.py`): 계수는 매 스텝 (3.2),(3.3),(3.6),(3.8)로 재계산, 방법 (2.1) 그대로(T_m=f_y(t_{m-1,s},Y_{m-1,s}) 또는 배율 kappa). t_0=-c_1 h_0, Y_0j = 정확해(VdP는 DOP853 rtol 1e-13). 격자: 균일, 또는 h_m ∝ 1+0.09u (u~U[-1,1], 최대 sigma 1.198).

주요 결과(오차=마지막 스텝 전 stage 최대; 관측차수=연속 N 배가 비):
| 문제 | 격자 | N | 관측차수 |
|---|---|---|---|
| PR lam=-1, T=lam | 균일 | 80,160,320,640 | 4.885, 4.96, 4.984, **4.993** |
| 같음 | 무작위 | 80..640 | 4.895, 5.044, 4.979, 4.963 |
| PR lam=-1, T=0.5 lam (W) | 균일 | 20..80 | 4.49, 4.74, 4.93 |
| PR lam=-1e4, T=lam | 균일 | 20..80 | 6.55, 6.27, 6.08 (매끄러운 PR 해에서의 초수렴, stage order 효과; 차수 보증 아님) |
| Riccati y'=-y^2, T 동결 | 균일 | 80,160,320 | 5.55, 5.33, 5.19 |
| VdP mu=1, T 동결 | 균일 | 160,320,640 | 6.96, 6.78, 6.55 (선점근; 감소 추세) |
비교용 BIT mipeer6 재구성: PR 균일 6.01(초수렴, 상수 스텝), 무작위 격자 5.5~5.9, VdP 6.2~6.8.
시작값 오염: 5.1 참고. 관찰: PPSW2-6 합성 균일 PR에서 차수 -> 4.99(예상 5). 이는 "합성+정확 시작" 한정.

**W 허용 오차 프로브(z5, PR lam=-1e4, N=40, h*lam=-250)**: T=kappa*lam, kappa=0.7: 오차 8e-8, 0.8: 8e-11, 0.9~1.5: 1e-10~4e-8 (안정), 그러나 **kappa=0.6은 발산**(N=20,40,80에서 오차 2e-4, 0.89, 1e11; PPSW2-6과 mipeer6 모두). 즉 T는 "정확도에는 무관하나 안정성에는 J 근사가 필요"하며 경험적 하한이 T/J~0.65 부근. 이는 G0의 "fast lane에 explicit-J 0"과 충돌 가능(J 근사/freeze 정책 필요), 1개 스칼라 사례 한정.

재현 계획(리포 밖): z4/z5/z6의 문제정의와 coeffs_mp를 그대로 사용, N을 2^k 배로 늘려 log2 오차비 기울기, 시작값은 (a) 정확해, (b) h^q 섭동(q=3,4,5), (c) RODAS5P 계열 시작기로 교체해 차이 확인. 전부 dps 40 이상.

## 6. 다른 병렬 peer-W 방법 조사

결론: **PPSW2-6 외에, 가변스텝 차수>=5 + 완전 stage 병렬 + tableau가 인쇄/재현 가능한 방법은 확인되지 않음.** 확인 목록은 JSON `survey`. 요약:
| 인용 | 차수 | 병렬 | W/Jacobian | 표 | 접근 |
|---|---|---|---|---|---|
| Schmitt-Weiner SINUM 42 (2004) PPSW1 | s-1 (6b,6c=5; 상수 스텝 초수렴 s) | yes(단일 행렬) | W | c(Table 2), gamma-hat(Table 1) 인쇄. A 구조는 Thm 3.1 필요; 내 A=gamma*Theta 재구성은 **무효**(alpha 0 vs 인쇄 57.5/87.9; 특성 beta=0이 아님) -> 미재구성 UNRESOLVED | 소유자 PDF |
| Schmitt-Weiner-Podhaisky BIT 45 (2005) mipeer s | s-1 (s=6 -> 5), 상수 스텝 초수렴 | yes | W | Table 7.1은 s=3,4,5만. **mipeer6는 공식으로 재현 가능**(아래) | 소유자 PDF |
| Podhaisky-Weiner-Schmitt APNUM 53 (2005) | s-1, s=4..8 | **no** | W | yes | 프리프린트(PW03) |
| Lang-Schmitt arXiv 2608.26913 IP4o5 | 5 (s=4) | no(하삼각) | ? | yes | arXiv(PW03) |
| Beck-Weiner-Podhaisky-Schmitt JAMC 38 (2012) | 검색 스니펫상 multi-implicit p=s, s<=5 (미검증) | yes | W | ? | 미열람(미러만 봄, 사용 안 함) |
| Kulikov-Weiner 2011; Jebens-Knoth-Weiner 2012; Gerisch et al. 2009; Klinge et al. 2021 | 불명 | peer-W | W/AMF | 불명 | 유료/미열람, Klinge 학위논문은 bot-check |
| Zaragoza TASE-peer | 2-4(TASE-peer), RK-TASE 최대 5 | stage 독립 | TASE(W 아님) | 불명 | 403/스니펫 |
| Marburg peer 페이지 | 선형 암시 order>=5 명시 없음(EPPEER 명시적 방법만 >=5) | | | | 공개 |

**mipeer6 (BIT 재구성, 수치 확인; z3)**: gamma_i=g0+g1 c_i (BIT (2.14)), 노드 (5.8), g1=1-1/sigma_sup=0.248074 (sigma_sup=1.32992, Remark 2.1 / (2.17) 표 1.330·0.2481 일치), g0는 초수렴 조건(BIT (4.7) 대응 수치식)의 근 0.279786, gamma_min=0.0317, alpha=89.58도, B(1) 고유치 {1, 0.752, 0.504,...}, max|B|=55.8. 재현 검증: s=3,4에서 g0=0.9057, 0.5443 (Table 7.1과 4자리 일치); s=5는 0.3771 vs 인쇄 0.3756(불일치, 원인 UNRESOLVED; 인쇄값을 쓰면 초수렴 잔차 -0.34). BIT Thm 2.2가 sigma<=sigma_bar에서 zero-stability(곱 유계)를 **증명**(삼각 B-tilde, 고유치 sigma^{i-1}(1-(i-1)g1)); 수치로 쌍 최악값 smax 1.3에서 0.978, 1.4에서 1.29(>1). 단 BIT는 mipeer6가 "no further improvement"이라 결과를 생략했고 노드가 Chebyshev(조건수 K~63.6)이며 PPSW2-6와 달리 tableau 미인쇄. 대체 후보로만 기록(고정 sigma 안정성 증명이라는 점이 장점).

## 7. 영수증 / UNRESOLVED

영수증(전체는 JSON): z1(23 s), z2(1.3 s), z3(9.6 s), z4(24 s), z5(26 s), z6(27 s), z7 모두 exit 0, stderr 0 byte 이외 없음. sha256는 JSON `receipts`. 세 PDF는 PW03A의 sha 사용(변경 없음). PDF 텍스트는 pdftotext -layout.
UNRESOLVED: APNUM Fig.1,2의 수치(그림 미열람); 시작기 절차(c<0); PPSW2-6 측정 차수(논문 미인쇄); SINUM PPSW1 A 구조 재구성; mipeer s=5 g0 불일치; Beck 2012 본문; 단일 stage 국소 h^7 vs AB(7) 불일치 원인; 실제 컨트롤러가 sigma 1.19+ 교번을 밟는지; 유료 논문 미접근(우회 없음).
