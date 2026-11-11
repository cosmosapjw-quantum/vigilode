# PW04: mipeer6(BIT 2005 multi-implicit Peer-W) 재구성 + W-tolerance / G0 fast-lane 분석 (2026-11-11)

범위: 문헌 + 정확/수치 재현만. Rust/git 없음, 기존 파일 미수정, 모든 스크립트 `python3 -I`(numpy 1.26.4, scipy 1.11.4, sympy 1.12, mpmath 1.2.1). 기계판독본 `PW04_MIPEER6_AND_W_TOLERANCE.json`에 스크립트 전문과 영수증(cmd, exit, sha256 stdout/stderr)이 있다. 프로토타입은 합성 문제 + 정확 시작값 한정이며 qualification이 아니다. 모든 G0 게이트는 NOT_RUN.

## 0. 판정 (요약)

- **mipeer6 = CANDIDATE_FOR_PW04 (조건부, 1순위).** 근거: Thm 2.2가 sigma<=sigma_bar(<sigma_sup=1.3299)에서 가변스텝 zero-stability를 **증명**(삼각 B~)하고, W-허용 창이 PPSW2-6보다 넓으며, 차수조건 Gamma(6),AB(6) 가변스텝 잔차가 1e-76 이하로 0임을 80자리로 확인. 선결 조건은 11절.
- **PPSW2-6 = NOT_PREFERRED (조건부 fallback만).** zero-stability 증명 없음(PW03B: 교번 열 sigma~1.19에서 ρ>1), W-허용 창이 매우 좁음(sigma=1.1에서 kappa in [0.99,1.12], sigma=1.2에서 [1.00,1.03], sigma=1.3에서 없음). 사용하려면 sigma_max<=1.1 이하 + 정확 T + 자체 starter.
- **전제 정정(중요).** 사용자 지시문의 "common-shift PPSW2 family"는 문헌과 다르다. APNUM 48 (p.425 이하 소개부, Sec. 3 제목 "Multi-implicit stiffly accurate PPSW-methods")은 **PPSW1=singly-implicit(SINUM 2004), PPSW2=multi-implicit(gamma_i 상이)**로 정의한다. 따라서 PPSW2-6도 6개의 서로 다른 gamma_i를 쓴다(PW03B 재구성: gamma_i=g0_i/(1+sigma g1_i), i<6, gamma_6는 (3.6)). common-shift는 PTSW2C 등 다른 계열이다. mipeer6와 PPSW2-6의 factorization 수는 동일(6)하다.
- 소유자 결정 (a)(b) 반영: mipeer 계열을 우선 검사했고(통과), G0(J-build 0, Newton 0)는 correction form + matrix-free Krylov로 설계(10절), 위반 시 `W-TOLERANCE-FALLBACK` 라벨.

## 1. 인쇄된 것 vs 재구성된 것 (BIT 45, 2005, pp.197-217; pdftotext + 300 dpi 이미지 확인)

| 항목 | 상태 | 근거 |
|---|---|---|
| 방법 정의 (1.3), (2.7)-(2.9), Gamma(q)/AB(q) (2.2)(2.3), Theta/Gamma/B 행렬식 (2.4)(2.5), gamma_i=g0+g1 c_i (2.14), B~ 식 (2.15), 고유값 (2.16) | 인쇄 (LITERATURE_SUPPORTED) | pp.199-202 |
| Thm 2.2 (zero-stability, 증명 있음), Remark 2.1 (sigma_sup 다항식, (1-g1)sigma_sup=1) | 인쇄 | pp.202-203 |
| 표 (2.17): s=3..8의 sigma_sup, g1 | 인쇄, **s=6: 1.330, 0.2481** | p.203; 내 계산 1.329917398..., 0.248073601...과 4자리 반올림 일치 (m1) |
| 노드 (5.8) stretched Chebyshev, 표 5.1 K_C | 인쇄, s=6: K_C=63.641 | p.210-211; 내 cond_F(V)=63.64115 일치 |
| Table 7.1 (p.214): **mipeer3,4,5만 인쇄** (c, sigma_bar, g1, g0, alpha, rktol). misup3 포함 | 인쇄 | 이미지 확인 |
| **mipeer6: 표 없음.** 본문 p.213: "mipeer6 has shown no further improvement ... we omit its results" | **미인쇄 -> 공식으로만 재구성** | g1: Remark 2.1, g0: (4.7), c: (5.8) |
| mipeer6의 sigma_bar, alpha, rktol, 오차추정 상수, 시작값 절차 | **미인쇄 (UNRESOLVED)** | — |
| 3 수치 실험 그림 (Figs 7.1-7.3)은 mipeer3,4,5와 PPSW2-5 | mipeer6 결과 없음 | p.213-216 |

(4.1)의 인쇄 부호 점검: (4.1)은 `-sigma s sum gamma_ij c_j^{s-1}`로 인쇄되어 있으나, A=-Gamma일 때 Taylor 전개는 `-sigma s sum a_ij c_j^{s-1}` 즉 `+sigma s sum gamma_ij ...`가 되어야 한다. 80자리 확인(m3): (4.2)의 `phi(1+sigma c_i)-gamma_i sigma phi'(1+sigma c_i)`와 Taylor-correct 형태는 1e-79로 일치, 인쇄된 (4.1)은 불일치(차이 202.7). (4.1)은 오타로 보이며 (4.2)는 영향 없음 (NUMERICALLY_CHECKED).
또 (6.1)은 괄호 누락: 올바른 형태 `(I-hG(x)T)(Y_m - Theta Y_{m-1}) = G Theta (h f(Y_{m-1}) - sigma E Y_{m-1})`가 (2.8)과 대수적으로 동일함을 mp 60자리에서 확인(1e-58, 임의 T). E Theta = sigma Theta E, B=(I-GE)Theta (2.9)도 1e-59로 확인 (m6).

## 2. "s=5 g0 불일치" 해소 (m1, mpmath 80자리)

인쇄 Table 7.1 (이미지 300 dpi 재확인): mipeer5 g1=0.3075, **g0=0.3756**, alpha=89.8. 내 (4.7) 해는 **0.37709171145...** (s=3: 0.90569803977 vs 인쇄 0.9057, s=4: 0.54431912454 vs 0.5443: 일치). 두 독립 경로(a: (s+1)차 삼각 B~의 좌고유벡터 v~와 (4.7), b: 노달 B(1)의 좌고유벡터와 (4.5) w_i=phi(1+c_i)-gamma_i phi'(1+c_i))가 모든 근에서 1e-80 수준으로 일치하고, v~ 스케일 성분 (1, 1-g0, 2(1-g0)^2-g1, 6(1-g0)^3-9g1(1-g0)+2g1^2)이 인쇄식과 5e-82로 일치. 따라서 **z3 스크립트 버그가 아님.**

가설 검증: (H1) g1을 인쇄 반올림 0.3075로 사용 -> 0.377086 (불일치 유지). (H3) 비스케일 Chebyshev 노드 -> 0.37848. (H2) 0.3756을 주는 g1은 0.30876 (인쇄 0.3075와 상이; 반올림으로 설명 불가). 근 목록 {-0.0196, 0.3771, 0.8715, 1.483, 2.288}에 0.3756 근처 다른 근 없음. alpha: 인쇄 g0 -> 89.888도, 재구성 g0 -> 89.900도 (인쇄 89.8, 둘 다 불일치 +0.1도 내).
**결론: 0.3756은 오타 또는 저자들이 쓴 미공개 수치(예: 다른 반올림 경로)로 분류, 원인 UNRESOLVED.** 영향: g0는 Gamma(s),AB(s)에 들어가지 않으므로 차수/안정성에 영향 없음(Thm 2.2는 bounded g0 허용), 초수렴 잔차 (4.7)만 -0.225(기울기 150.8)만큼 어긋남. mipeer5 전역 오차를 인쇄 g0 vs 재구성 g0로 비교(PR 문제, N=8..64): 오차 차이 < 20%, 관측 차수 동일(4.3-5.5) -> 사전점근 구간에서 구분 불가 (m6). 내 선택: **재구성값(수식 (4.7)) 사용**, 인쇄값과의 차이는 기록.

## 3. 재구성 mipeer6 tableau (s=6; 세 독립 경로가 80자리에서 일치)

경로 R1: BIT (2.4)(2.5) 행렬식 `Theta=V S P V^-1, E=V D F0^T V^-1, B=Theta - sigma A E, A=-Gamma=G Theta`.
경로 R2: Lagrange 다항식 형태 (3.1): `Theta_ij=l_j(1+sigma c_i)`, `B_ij=l_j(x_i)-gamma_i sigma l_j'(x_i)` (Vandermonde 역행렬 없음).
경로 R3: (2.15) `B~=(I-g1 D^-g0 F0^T D^)S P`, `B=V B~ V^-1`.
sigma in {1, 1.2, 0.7, 0.1}에서 max|R1-R2|, |R1-R3|, |R2-R3| <= 1.4e-78 (m2).

- c = (-1, -(sqrt3-1), -(2-sqrt3), 2-sqrt3, sqrt3-1, 1) (정확, Q(sqrt3); 확인 1e-81). 노드 -1..1이므로 시작 구간이 [t0-h0,t0+h0].
- sigma_sup = 1.32991739848048029510781702205815... (정수 다항식 `4x^5-5x^4-1=0`의 근, 잔차 0), g1 = 1-1/sigma_sup = 0.248073601305940519731346336549792...
- g0 = 0.27978618386103127251510128768089728... ((4.7)의 6차 다항식의 첫 admissible 근; 전체 근 {-0.0210, 0.2798, 0.6526, 1.0994, 1.6420, 2.3472}; (2.18) g0>g1=0.2481 이므로 0.2798이 최소 admissible). 이전 PW03B의 0.27978618386094856(double 근)과 8e-14 차이로 정합.
- gamma_i = g0+g1 c_i = 0.031712582555090752783754951, 0.098183703688497822245239557, 0.213315062727624203053616681, 0.346257304994438341976585894, 0.461388664033564722784963017, 0.527859785166971792246447624 (gamma_min>0, max/min=16.6).
- sigma=1 tableau (12자리; 60자리는 JSON) B:
```
+0.046074425553 -0.173886698898 +0.695625017059 +0.548391918797 -0.159446634435 +0.043241971924
+0.020748657967 -0.071875459575 +0.183213285320 +0.986845877943 -0.154870091443 +0.035937729788
-0.045078776920 +0.145696897348 -0.291393794696 +0.627866366311 +0.854303102652 -0.291393794696
+0.861743223481 -2.637303151107 +4.543628295055 -6.455022042966 +7.042674669644 -2.355720994107
+5.056229267614 -14.974971257177 +23.845026666798 -28.967722211644 +24.354813142531 -8.313375608121
+10.659325519247 -31.160301636064 +48.193418892683 -55.752376225857 +44.363633740428 -15.303700290436
```
A=-Gamma=G Theta의 행 2,3은 단위벡터 (sigma=1에서 1+c_2=c_4, 1+c_3=c_5, 즉 단계 간 노드 겹침: stage 2,3이 Y_{m-1,4},Y_{m-1,5}의 f만 사용). max|B|=55.75, max|A|=53.89. 행합 B=1 (오차 5e-15 double). B(1) 고유값 {1, 0.75193, 0.50385, 0.25578, 0.00771, -0.24037} = (1-(i-1)g1) (식 (2.16), 수치와 1e-12 일치).

## 4. 가변스텝 차수 조건 (m3, mp 80자리, B는 R1)

잔차 (2.2)(2.3) 단계 최대값 k=0..9, sigma in {0.1,0.5,0.8,1,1.2,1.33,2}:
- k=0..5: Gamma, AB 모두 <= 1e-75 (전 sigma). 즉 **Gamma(6), AB(6) 성립**.
- k=6: AB 잔차 34.7(σ=1), 25.3(1.2), 52.8(0.8) 등, Gamma 21.5(σ=1) 등 **전 sigma에서 비영**; k=7..9도 비영.
- 용어 정리: 지시문의 "order 7 fails => exactly 6"은 **조건 k<=5(6개 조건)**를 의미하는 것으로 해석한다. BIT 정의로는 stage 국소잔차 h_m Delta = O(h^6) -> **전역 차수 s-1=5**(논문 초록 "order s-1"), 상수 스텝에서는 초수렴 s=6 (Thm 4.1). k=6 조건(AB(7), Gamma(7))은 실패하므로 정확히 이 차수.
- 선도 오차 벡터 (4.2) `phi(1+sigma c_i)-gamma_i sigma phi'(1+sigma c_i)`: sigma^6 * res_AB(6)_i 와 일치(3e-77). 전 stage 비영: min|W|=0.0227(σ=1), max 34.7. 이 부분군(g 선형)에서 모든 stage를 동시에 0으로 만드는 g0는 없음 (min_g0 max_i|W_i| = 0.613). (2.12)의 마지막 stage 초일관은 g0=0.0371이 필요하나(`gamma_s=0.2852 vs 현재 0.5279`), mipeer6는 초수렴 g0를 택했으므로 초일관 stage 없음.
- 비교: PPSW2-6도 k<=5 잔차 1e-78 이하, k=6 비영(AB 0.45 등, 더 작음).

## 5. Zero-stability (m4; 이론 + 수치)

이론(Thm 2.2 + Remark 2.1, 인쇄): B~(sigma)는 **모든 sigma에서 상삼각**이고 B=V B~ V^-1 (V는 노드만의 함수, sigma 무관). 따라서 임의 곱의 스펙트럼 = 대각 곱 `prod sigma_k^{i-1}(1-(i-1)g1)^n` = `(h_n/h_0)^{i-1}(1-(i-1)g1)^n` (수치 확인: 곱 스펙트럼 [1.0000000002, 0.1394704187, 0.00868231, 7.74e-5, 5.28e-5, ~0]가 예측값과 일치; 1e-6 미만 고유값은 반올림 지배라 비교 제외). g1=1-1/sigma_sup는 i=2와 i=6 조건을 동시에 속박: sigma_sup(1-g1)=1.0000, sigma_sup^5|1-5g1|=1.0000000000000009. 엔트리별 상계 B~_bar(sigma_bar)로 `||B_m...B_{m-k}|| <= cond(V) ||B_bar^{k+1}||`.
- **Theorem 2.2의 sigma_bar 값**: 정리는 존재만(sigma_bar>1, 상한은 Remark의 sigma_sup=1.3299). 인쇄된 sigma_bar는 s=3:2, 4:1.4, 5:1.3 (sigma_sup의 0.83, 0.83, 0.90배)이며 **s=6은 인쇄되지 않음 (UNRESOLVED)**.
- 상수 K(sigma_bar) (Thm 2.2 증명의 엔트리별 상계, 보수적): s=5 인쇄 sigma_bar=1.3에서 8.2e5, s=6: sigma_bar=1.05: 7.8e5, 1.1: 1.7e6, 1.2: 1.4e7, 1.25: 6.5e7, 1.3: 9.4e8, 1.3166: 6.3e9. s=5 인쇄값(8.2e5)과 같은 K가 되는 s=6 sigma_bar는 **약 1.05**, sigma_sup 비율 0.90을 따르면 1.197. **제안(자체 규칙, 인쇄 아님): sigma_max <= 1.10** (K는 s=5 인쇄값의 약 2배, alpha 79도, 아래 6절; 더 보수적 값은 1.05).
- 점별: B(sigma)의 비-1 고유값 최대 모듈러스 = max_i sigma^{i-1}|1-(i-1)g1| (수치와 sigma<=1.25에서 0.9399까지 일치; 그 이상은 내 스크립트가 정렬 2번째 고유값을 보아서 불일치하므로 공식값이 권위), sigma=1: 0.7519, 1.2: 0.9023, 1.3: 0.9775, 1.3299: 1.0.
- 곱 수치(double, 노달 B): 균등난수 sigma in [0.05,1.1/1.2/1.3/1.329/1.4/1.5]: max ||P||_inf 371/449/826/718/1878/1543 (1000스텝 x 20), [0.05,2.0]: 8.0e5; 대칭 [1/1.2,1.2]: 1372; [1/1.3,1.3]: 1914. 순 성장 0인 주기열 (sigma^r, sigma^-r): 모두 유계(1.5e3-1.1e4, 400주기 수렴), **PPSW2-6의 (1.2, 0.92) 같은 발산 열 없음.** 상수 sigma 지속: 1.2: 3473으로 포화, 1.3: 1.8e4 포화, 1.329: 1.6e5 증가 중(ρ=0.9993), **1.335: ρ=1.019으로 성장(6.9e5 at 400), 1.35: 5.6e15** (sigma_sup 초과 시 지속 성장; 정확히 이론 예측). 알파벳 {0.1,0.8,1,1.2,1.329} 길이<=5 전수 단어: 비-1 몫 ρ^(1/L) 최대 0.99931 (단어 sigma=1.329 반복) < 1.
- 해석: mipeer6 zero-stability는 **sigma_bar<sigma_sup인 모든 스텝비 열에 대해 증명됨**(NUMERICALLY_CHECKED로 재확인; 증명 자체는 문헌). 한계: K가 크고(실측 곱 노름 3.5e3 상수 sigma=1.2; 다수 열 1e3 수준) double 반올림 증폭이 커서 합성 PR 문제에서 전역 오차 바닥이 ~3e-10 (T=1, N=64-256)이다 (PPSW2-6도 유사). 허용오차 1e-10 이하는 이 방법의 double 구현에서 위험 (UNRESOLVED, 실제 RJF 대비 비교 필요).

## 6. 안정성 함수, L/A(alpha), stiff accuracy (m2)

- stiff accuracy: beta=A+Gamma=0 (BIT (2.7)), M(z)=(I-zG)^-1 B -> rho(M(-1e12))=9.4e-12 (M(infty)=0). 
- 주근 오차: 일정 스텝 sigma=1에서 |lambda_1(z)-e^z| ~ h^7.5, 7.3, 5.9(반올림 포화): 주근 차수 6 이상 (초수렴 s=6 정합).
- L(alpha) 각도 (z 평면 쐐기, 수치 스캔 r=1e-6..1e10; rho<=1+1e-13): sigma=0.5: 90.0, 0.8: 90.0, **1.0: 89.57**, **1.1: 79.4**, **1.2: 66.8**, **1.3: 48.7** (B(sigma)의 둘째 고유값 0.75 sigma가 1에 접근하므로 sigma 증가 시 감소). sigma=1에서 음의 실수축 전체 rho<1. 인쇄 mipeer3,4,5의 alpha(90,90,89.8)는 재구성 90,90,89.90과 일치(+-0.1도). **mipeer6의 인쇄 alpha는 없음.** 설계 결과: sigma_max를 키우면 A(alpha)가 급격히 좁아짐 -> sigma_max<=1.10 권장 근거.
- 시작 stage 구조: stage당 별도 W_i = I - h gamma_i T, gamma_i는 sigma 무관 상수 (g0,g1 고정) -> PPSW2-6 (gamma_i가 sigma 의존)보다 factor 재사용에 유리 (h 불변일 때).

## 7. 오차추정기와 시작값 (m6)

- **BIT 인쇄 내용**(Sec.7, p.212): 고차(s>=4)는 Y_ms를 **차수 s-2 예측자**와 비교, 또는 초일관 stage (2.12) (misup3만). 예측자의 구체적 계수/스케일/노름은 **미인쇄**. mipeer6는 초일관 stage 없음.
- 내 후보(자체, 인쇄 아님) 두 개를 단일 스텝, 정확 이전 데이터, 상수 sigma=1, 1.1, 0.8에서 측정: E1 = Y_ms - (Y_{m-1,2..6}로 보간한 차수 s-2 예측자) 은 O(h^5)(기울기 4.9, h=0.05), E3(노드 6 제외)도 O(h^5), 국소오차(stage s) 와 E2 = Y_ms - (Theta Y_{m-1})_s 는 둘 다 O(h^6)(기울기 5.9-6.0). **E2/국소오차 -> 해석 상수 `-gamma_s sigma phi'(x_s)/W_s` = 2.175 (sigma=1), 2.103 (1.1), 2.388 (0.8)**, 수치 2.32(h=0.05)->수렴 중. 즉 E2는 국소오차에 점근적으로 비례(상수 계산 가능)하나 인쇄된 방법이 아님. E1(BIT 문장에 가장 가까움)은 h^5이어서 실제 국소오차(h^6)보다 훨씬 보수적 -> 허용오차 비례성 손실 (PW03B의 PPSW2-6 관찰과 같은 경향).
- 시작값: Y_0j ~ y(t0 + c_j h0), c in [-1,1]. 시작값 오염 O(h^q) 실험(PR, N=12..96, kappa=1): q=3 -> 관측 3.3-3.5, q=4 -> 4.4-4.6, q>=5 -> 감소 없음(5.2-5.7). 즉 **시작 오차 O(h^5)=O(h^{s-1})이면 차수 5 유지(관측)**; BIT Thm 4.1은 초수렴에 X_0=O(h^s) 요구. 시작기(차수/허용오차/비용)는 BIT에 인쇄되지 않음 (UNRESOLVED; PW03B와 동일한 결론). 확장된 q 실험의 DIFF 문제는 반올림 바닥에 걸려 해석 불가.

## 8. 구조 적합성 (project goals)

- **multi-implicit = stage마다 별도 shifted matrix**: W_i = I - h gamma_i T_m, i=1..6, gamma_i 6개 모두 상이 (spread 16.6배). 직접법이면 **스텝당 6회 factorization**, singly-implicit(PPSW1, PTSW2C 계열)은 1회. PPSW2-6도 동일하게 6회(gamma_i도 sigma 의존이라 sigma 변화 시 재사용 불가). 즉 "one common W factor"는 **성립하지 않음**; 6 W 모두를 한 번에 병렬 factor해야 하며, common-W 가정에 기댄 설계(PTSW2C 식 단일 factor 재사용, block Krylov 공유)는 mipeer6/PPSW2-6에 쓸 수 없다.
- **동일 스텝 stage 독립성**: (1.3)/(2.8)에서 stage i의 우변은 이전 스텝 값 Y_{m-1,*}, f(Y_{m-1,*}), T_m만 사용하므로 **6개 stage가 같은 스텝 안에서 서로 독립**(문헌 명시: BIT p.197 "s stages which are totally independent within the actual time step"). 스텝 간에는 완전 의존(스텝 m은 스텝 m-1의 6개 stage 모두에 의존).
- critical depth (G0: p95<=6, abs<=8): **정의가 소스 문서에 명시되지 않음 (UNRESOLVED)**. 가정 정의 "한 accepted step 안에서 의존 연쇄의 최장 길이": 직접법 mipeer6 = {f 6개+J} -> {factor 6개 || 우변 조립} -> {solve 6개} = 3 레벨(순차 RODAS류 stage 6개 직렬에 비해 작음), 시작/출력/거절 제외. **matrix-free Krylov fast lane에서는 stage당 FOM 반복이 직렬이므로 깊이 = max_i k_i**: 측정값 correction form rtol=1e-2에서 stage solve당 평균 7.2-10.3회 (n=31, N=48) -> 8 한계(abs<=8)에 근접/초과 가능, 큰 n/나쁜 조건에서는 증가. 즉 **G0 "J-build 0"과 "depth<=6/8"이 정의에 따라 충돌할 수 있음** (소유자 결정 필요).
- 비용(스텝당, 직접): 6 factorization + 6 solve + 6 f + 1 J. Krylov: 6 solve x k_i matvec + 6 f. 완전 병렬 stage 가정에서 wall 효과는 6 worker 필요 (G0 speedup>=1.15는 코어 수/비용 모델에 의존, NOT_RUN).

## 9. W-tolerance 분석 (m5; 합성 문제, 정확 시작값, 프로토타입)

T를 바꿔도 order condition은 무영향(Gamma(s)로 T항 소거)이나 **안정성은 T~J가 필요**. 아래 모두 mipeer6 / PPSW2-6 비교. 문제: PR(lambda=-1,-1e4), LIN(n=24: 실수 -1..-1e4 + 복소쌍 -100+-400i), DIFF-semilinear(Fisher-KPP, n=31, J^~ -4e3), DIFF-quasilinear(kappa(u)=0.1+2u^2, J가 x18 변동). 모두 정확해 격자 존재(manufactured, 반이산 정확).

**9.1 기준 차수 (T=J 정확; N=12,24,48,96; 오차<2e-9는 반올림 바닥으로 제외)**
mipeer6: PR-1 6.1/5.9 (균등/난수), PR-1e4 6.4/6.1, LIN 5.6/5.7, DIFF-s 5.4/5.6(+초기 10), DIFF-q 5.4/4.9. PPSW2-6: PR-1 4.4-4.7, PR-1e4 6.5, LIN 5.9-6.7, DIFF-q 4.9/6.0. (사전점근, 관측 4.4-6.7; global order>=4.8 NOT_RUN 상태.) 난수 격자는 sigma in [0.91,1.09].

**9.2 선형 이론: T=kappa J의 창 (stage 안정성 sup over z<0, 음의 실수축과 135도 선)**
M(z;kappa)=(I-kappa z G)^-1 (B+z(1-kappa)A). kappa 창 (mipeer6 | PPSW2-6):
| sigma | mipeer6 실수축 | PPSW2-6 실수축 |
|---|---|---|
| 0.8 | [0.743, 1.755] | [0.819, 2.053] |
| 0.9 | [0.773, 1.514] | [0.909, 1.600] |
| 1.0 | [0.799, 1.380] | [0.964, 1.410] |
| 1.1 | [0.822, 1.295] | [0.989, 1.116] |
| 1.2 | [0.841, 1.236] | [0.998, 1.034] |
| 1.3 | [0.859, 1.192] | 없음 |
135도 선은 약간 더 좁음(mipeer6 sigma=1: [0.809,1.332]; PPSW2-6 sigma=1.2: 없음). z->-inf 극한만 보면 M->-((1-kappa)/kappa)Theta(sigma)이고 rho(Theta(sigma))=max(1,sigma^5)이므로 kappa>0.5 (sigma=1)이나, **중간 |z|에서 B의 큰 원소(55) 때문에 실제 창은 [0.80,...]로 훨씬 좁다** (PW03B의 kappa=0.6 발산, 0.7 오차 1e-3과 정합). 일반 T에 대해 z->inf 극한: M->-Theta (x) (T^-1 J - I)이므로 stiff 부공간에서 `rho(I-T^-1 J) < 1/rho(Theta(sigma))`가 필요조건 (유도, 수치 확인은 kappa 경우만).

**9.3 시간 적분 (T=kappa J, N=24,48; 성공= err48<1e-2 이고 err48<err24)**
성공 kappa 범위 (스캔 격자 0.3,0.4,...,5): mipeer6: PR-1e4 [0.8,1.6], LIN [0.8,1.3], DIFF-s/q [0.75,1.6] (난수 격자 동일). PPSW2-6: PR-1e4 [0.75,2.0], LIN [1.0,1.4] (난수 [1.0,1.3]), DIFF-s [0.9,1.6], DIFF-q [1.0,1.6]. **kappa<=0.7은 모두 발산, kappa>=2(5)는 발산.** (linear 창과 정합; 문제별 z 집합이 달라 창이 약간 넓어질 수 있음).

**9.4 T=J+delta (||E||_2=1, delta x ||J||_2) 및 원소별 J∘(1+delta R)**
- 스칼라 PR: 둘 다 |delta|<=0.5까지 안정 (kappa 창 내).
- LIN: 2노름 상대 섭동은 delta=0.01에서도 발산 (스펙트럼이 1..1e4라 ||J||_2 상대 섭동이 작은 고유 모드를 압도; **다중 스케일 문제에서는 행렬노름 상대 오차가 부적절한 척도**). 원소별 섭동도 0.01에서 발산.
- DIFF-s: 2노름 delta=0.01 통과, 원소별 mipeer6 0.05까지, PPSW2-6 0.01까지. DIFF-q: mipeer6 0.01(2노름)/0.05(원소별), PPSW2-6 0.0/0.01.
- 결론: 임의(비구조) 섭동에 대한 **실용 허용치는 delta ~ 0.01-0.05 수준**이며 mipeer6가 PPSW2-6보다 약간 넓다. ε_W = max|1-mu| (T^-1 J 고유값) 지표는 비정상 모드를 포함해 1을 넘는 값이 많아 예측력이 약함 (표는 JSON).

**9.5 lagged J (T=J_{m-k}) (DIFF-q, J 변동 x18)**
| 방법 | N=48에서 안정/성능 저하 | N=96 |
|---|---|---|
| mipeer6 | k<=5 오차 불변(1.0e-8), k=10 오차 1.6e-6(저하), 20: 1.1e-2, 40: 7.7e-2 | k<=10 불변, k=20: 2e-2, k=40: 발산 |
| PPSW2-6 | k<=5 불변, k=10: 3.6e-4, 20: 0.56 | k<=10 불변, k=20: 발산 |
해당 lag의 실측 rho(I-T^-1 J)(전 스텝 최대): mipeer6 안정 0.42 이하, 저하 0.65 이상; PPSW2-6 불안정 0.65 이상. DIFF-semilinear(J가 거의 상수)는 모든 lag(40까지)에서 안정(J 변동이 작음). 즉 **lag 허용치는 "J의 상대 변화 ~0.4 이하"**; 시간으로는 이 문제에서 약 0.1 단위(lag10, N=96).

**9.6 diag / 절대 행합 / T=0**: LIN, DIFF-s, DIFF-q 전부, 양 방법 모두 **발산** (diag: rho(I-T^-1J)=1.05-6.8; 절대행합 -sum|J_ij|: 0.998-1.06; 0은 명시적). 즉 diag/lumped/Jacobi형 T는 fast lane T로 허용 불가.

**9.7 Krylov (matrix-free, FOM, JVP만; n=31이므로 반복 상한 n+1)**
두 형태를 비교: (2.8) 형태(전체 우변을 Krylov로 풂)와 **correction form (6.1)** (우변이 T 비의존, O(h^s) 크기).
- (2.8) 형태: rtol 1e-6에서도 오차 1e-2..1e-5 수준으로 수렴 실패, rtol=1e-10에서 겨우 4.5/2.2차(바닥), 사실상 **사용 불가**.
- correction form (6.1): rtol=1e-2, 1e-3, 1e-4, 1e-6, 1e-10 모두 정확해법과 같은 오차(예: DIFF-q mipeer6 N=24,48,96에서 2.9e-7, 8.9e-9, 1.4e-10; 관측 5.0-5.4)로 **차수 유지**. **rtol=0.1은 발산**. 평균 stage solve당 반복: rtol=1e-2 -> 7.2-10.3, 1e-3 -> 9.1-12, 1e-6 -> 14-15.
- J의 base point를 lag해서 JVP (frozen base): mipeer6 DIFF-q correction form: lag 0,2: 오차 불변; lag 5: rtol=1e-2에서 N=48 오차 8.8e-8(약 10배 악화), 1e-6에서 불변; lag 10: rtol=1e-2 N=48 8.8e-3, 1e-6 2.0e-6 (저하). PPSW2-6 유사 (lag10: 6.2e-3/3.6e-4).
- 핵심: **fast lane에서는 base point를 매 스텝 (t_m, Y_{m-1,s})로 갱신해도 explicit J 빌드가 아니다** (JVP만 사용). 따라서 lag 허용치에 의존하지 않고 T=J(정확, JVP 오차 4e-11)를 유지할 수 있다. 남는 W-tolerance 변수는 Krylov 잔차 허용치 rtol (correction form 기준 <=1e-2)와 반복 한계.

## 10. G0 fast lane 정의 (제안) + W-TOLERANCE-FALLBACK 규칙

**FAST-LANE (제안)**
- 형태: correction form (6.1): Ytilde=Theta Y_{m-1}; rhs_i = gamma_i [Theta (h f(Y_{m-1}) - sigma E Y_{m-1})]_i; solve (I-h gamma_i T) d_i = rhs_i; Y_mi=Ytilde_i + d_i (또는 (2.8)과 동일 대수; 수치로 확인함).
- 허용 연산: f 평가; **JVP / operator application only** (기준점 (t_m, Y_{m-1,s}) 에서의 정확 AD 또는 중심차분 방향미분, 반복마다 새 base 가능); Krylov (FOM/GMRES) 반복 중 벡터 연산, 작은 Hessenberg 밀집 대수; 6x6 tableau 대수 (Theta, E, 계수는 sigma별 재계산 또는 캐시).
- 금지 연산: J의 explicit 조립 (밀집/희소/컬러링 FD), W_i의 LU/ILU factorization (preconditioner 구성 포함 여부는 정책 결정 필요: 조립 없는 matrix-free preconditioner만 허용), Newton/비선형 반복 (Peer-W는 stage가 선형이므로 Newton 반복은 구조상 0), 다중 JVP base를 스텝 내 상이하게 쓰기(스텝당 1 base).
- 운용 조건(실험 근거, 합성 문제 한정): sigma in (0, 1.10], Krylov correction-form 잔차 상대허용치 rtol<=1e-2 (rtol=0.1 발산), 반복 상한 k_max(정의 후보 8 = critical depth abs; 정책 결정 필요), 시작 오차 O(h^5) 이하.

**W-TOLERANCE-FALLBACK (규칙, 소유자 결정 (b))**
트리거(하나라도): (T1) 어떤 stage에서 Krylov가 rtol=1e-2에 k_max 반복 내 미도달; (T2) sigma_m > 1.10 (또는 A(alpha)<79도 영역); (T3) correction 크기 모니터 ||d_s||(=E2 후보 추정량) 가 이전 스텝 대비 급증(예: >10배) - 임계값 미검정 (UNRESOLVED); (T4) JVP 상대오차 프로브 > 1e-3(미검정); (T5) lag를 사용하는 모드에서 lag>5 스텝 (실측 한계).
동작: 해당 스텝을 폐기하고 **method-compatible T**로 재시도 = 신선하게 평가한 explicit J (T=J(t_m,Y_{m-1,s})) + 6개 W_i 직접 factorization (또는 정확 T를 쓰는 비-fast 경로). 영수증에 `W-TOLERANCE-FALLBACK`와 {trigger id, J_build 수, factorization 수, solve 수}를 기록하고 **fallback_fraction(<=0.1) 분자에 산입**. fast lane의 "explicit J=0"은 fallback 스텝을 제외한 스텝에서만 주장한다.
정당화: 9.2/9.3의 허용 창(kappa 하한 ~0.75-0.8, rho(I-T^-1J)<~0.25-0.4)은 T=J(정확, JVP)에서는 침해되지 않고, 침해 위험은 Krylov 비수렴/섭동 시에만 발생하므로 트리거를 Krylov 수렴과 sigma에 둔다.

## 11. 판정 (DAG evidence class별) / 선결조건 / UNRESOLVED

| 항목 | 상태 |
|---|---|
| mipeer6 계수 구성 (BIT 공식, 3경로 80자리 일치) | DERIVED, NUMERICALLY_CHECKED |
| 가변스텝 차수 조건 k<=5 성립, k=6 실패 (전역 5) | NUMERICALLY_CHECKED |
| zero-stability (sigma_bar<sigma_sup) | LITERATURE_SUPPORTED (Thm 2.2 증명) + NUMERICALLY_CHECKED (곱 수치, 상수 sigma 임계 1.329-1.335) |
| 정확한 sigma_bar(6) 값, mipeer6 alpha/rktol/오차추정/시작기 | UNRESOLVED (미인쇄); 자체 제안: sigma_max<=1.10, E2 후보 |
| s=5 g0 인쇄값 0.3756 | UNRESOLVED (오타/미공개 수치; 내 계산 0.37709) |
| global order>=4.8, step ratio<=1.2, wall speedup>=1.15, depth, fallback<=0.1, fast-lane J/Newton=0 (실제 IVP) | **NOT_RUN** |
| 합성 관측 차수 4.9-6.7 | NUMERICALLY_CHECKED (지지 증거일 뿐) |
| mipeer6 = PW04 후보 | **CANDIDATE_FOR_PW04 (조건부)** |
| PPSW2-6 | NOT_PREFERRED (조건부 fallback; sigma_max<=1.1, 정확 T, 자체 starter 선결) |

선결 조건 (mipeer6): (1) sigma_max를 K/alpha 근거로 확정(자체 제안 1.10; 소유자 승인 필요: G0 step ratio<=1.2와의 관계 - "step ratio"가 sigma_m 상한인지 accepted-step 비율인지 불명), (2) 오차추정기 선택·상수 결정 및 컨트롤러(PW01은 다른 에이전트), (3) 시작기: c_1=-1 노드이므로 t0 에서 시작해 [t0, t0+2h0]의 6개 값 제공(O(h^5) 이하), (4) double 반올림 바닥(~3e-10, 합성) 대비 목표 허용오차 확인 또는 확장정밀도 계수, (5) critical depth 정의 확정 후 Krylov 반복 깊이 포함 여부 결정, (6) 6-worker 병렬 비용 모델과 PW06 full-IVP 비교(mipeer6 성능은 BIT가 "no further improvement"로 생략했으므로 s=5 대비 이득 불명), (7) 실제 컨트롤러의 sigma 열 검증.

UNRESOLVED 목록: mipeer6 sigma_bar/alpha/rktol/오차추정/시작기(미인쇄); s=5 g0 0.3756; critical depth 정의; BIT (4.1) 부호(오타 추정); W 트리거 T3,T4 임계값; Krylov 반복수의 대규모 n 의존; double 반올림 증폭이 RJF 대비 실제 문제에서 갖는 영향; 비정상(non-normal) J, 복소 스펙트럼에서의 W 창(135도 선만 확인); sigma 변동 격자에서 lag/kappa 복합 효과; 유료 논문 미접근(우회 없음); Podhaisky 2002 미참조(타 에이전트 담당).

## 12. 영수증
m1..m6 각각 `OMP_NUM_THREADS=1 python3 -I mN_full.py > mN.out 2> mN.err`, 전부 exit 0. stderr: m1-m4, m6은 0 byte, m5는 175 byte(진행 로그 `A done ..` 줄, 오류 아님). 스크립트 전문과 sha256(script/stdout/stderr)은 JSON `receipts`. PDF sha256: BIT `e9f8ef8ca7a788df8046e6adcc1ed938cfa1d342cdf6cbfc74496c67f2236e0b`, APNUM `7e8d17deeecf896bf4128493d4eaec26b50cf062b8a9d10d1397acb6ad84d01d`, SINUM `da9cd10e45c539...f9a`(JSON 전체값). 이미지 확인: `pdftoppm -r 300` BIT p.7, 10, 11, 18 (Table 7.1). 인용은 최소 구절만, PDF는 repo에 복사하지 않음.
