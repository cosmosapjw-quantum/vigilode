# PW03A: PPSW2-6 tableau 유도 (3대 1차 문헌 기반, 2026-11-10)

범위: 문서 + 정확/고정밀 산술 재현만. Rust/git 없음, 기존 파일 수정 없음, `python3 -I`. 논문 PDF는 복사하지 않았고 식/표 짧은 인용만 사용. 기계 판독본: `PW03A_PPSW2_6_DERIVATION.json`.

## 0. 판정

**PPSW2-6_TABLEAU_DERIVED_AND_ORDER_CHECKED**

- PPSW2-6의 B(sigma), A(sigma), Gamma(sigma), G(sigma)는 인쇄된 c, gamma0, gamma1(9자리)만으로 **완전히 결정**된다(식 (3.2),(3.3),(3.6),(3.8)). 자유도 잔여 없음.
- 차수 조건 잔차: k<=5 (Gamma(5..6), AB(6)) 모두 <= 1e-77 (80자리 산술, sigma=0.3..2). 유도가 정확하므로 "9자리 반올림 때문에 ~1e-9 잔차"는 **발생하지 않는다**. 반올림은 잔차가 아니라 방법 족 내 다른 원소를 고르는 것이다(아래 6절 감도).
- 독립 유도 3경로(행렬식 (3.2)(3.3) / tau-변수 직접 해 / BIT (3.1) 다항 규칙)가 B에서 1e-77 수준으로 일치.
- sigma=1에서 B(1) 고유치 {1, 0.8936, 0.5192(x2), 0.4630(x2)}(모듈러스): strongly stable. L(alpha) 수치 alpha = 81.609 deg (인쇄 81 deg와 정합; 인쇄는 정수 절사로 보임).
- 순차 의존 없음: 같은 스텝 stage 간 의존 행렬 = 대각(단위) 행렬.
- **소스 불일치 1건**: APNUM에 인쇄된 PPSW2-5의 gamma0/gamma1(PPSW2-6의 앞 4개와 동일)은 해당 c에서 B(1)이 고유치 5.894를 가져 zero-stable이 아님(alpha=0). BIT 2005 p.213의 PPSW2-5 gamma-bar 행은 alpha=86.4 deg로 인쇄 86 deg와 일치. 따라서 APNUM의 PPSW2-5 gamma 행은 오식(복사 오류 추정). PPSW2-6 행에는 영향 없음(검증됨). 이 범위에 한해 CONTRADICTS_SOURCE.
- **주의 사항(소유자 결정)**: (i) 계수 정밀도 정책: 9자리 십진수를 방법의 정의로 취급하고 B,A,Gamma는 런타임에 고정밀로 재계산할 것을 권고(소유자 결정). (ii) sigma 고정 stiff 안정성이 sigma=1.2에서 alpha=32.7 deg, 1.5에서 0 deg로 급락하며 |B| 항목은 최대 약 20: 인쇄된 sigmamax=1.2 사용의 근거는 Fig.2(미열람)뿐.

## A. 문헌에서 추출한 사실 (쪽은 저널 쪽번호)

| 항목 | 내용 | 출처 |
|---|---|---|
| 방법 형태 | `(I - gamma_i h_m T_m) Y_mi = sum_j (b_ij I + h_m gamma_ij T_m) Y_(m-1,j) + h_m sum_j a_ij f(t_(m-1,j),Y_(m-1,j))` (2.1); 행렬형 (2.2) | APNUM p.427; BIT (1.3) p.198; SINUM (3),(5) pp.266 |
| stage 병렬성 | "Obviously, with s processors all s values Y_mi can be computed in parallel." 좌변 `I - h G (x) T` (G 대각), 우변은 Y_(m-1)만 사용 | APNUM p.427; BIT p.197-198 |
| W 성질 | T_m은 "stability reasons only", 정확도는 임의 T_m(W 의미) | APNUM p.427; SINUM p.266 |
| 잔차/차수조건 | (2.3) 잔차; Gamma(p): `gamma_i c_i^k + sum_j gamma_ij (c_j-1)^k sigma^-k = 0`, AB(p): `c_i^k - sum_j b_ij (c_j-1)^k sigma^-k - k sum_j a_ij (c_j-1)^(k-1) sigma^(1-k) = 0`, k=0..p-1 (2.4),(2.5); Lemma 2.1: Gamma(p)+AB(p+1) => 잔차 O(h^p) | APNUM p.427; SINUM Lemma 2.3 p.270 |
| 수렴 | Thm 2.1: AB(p+1), Gamma(p), stable(2.6), 유계 계수(sigma<=sigma*), 시작값 오차 O(h0^p) => 차수 p | APNUM p.428 |
| stage order/superconv | 초록: order, stage order s-1 또는 s; Thm 3.1: (3.5) `phi(1+sigma c_i) = gamma_i sigma phi'(1+sigma c_i)` 이면 stage i 잔차 O(H^s); AB(s+1)는 모든 i에 대해 (3.5) 요구 | APNUM p.425, p.430 |
| stiff accuracy | Def 2.3: beta = A+Gamma = 0; M(inf) = -G^-1 beta | APNUM p.428 |
| 안정행렬 | M(z) = (I - zG)^-1 (B + z beta) (2.7); L(alpha): rho(M(z))<1 for abs(arg z - pi)<alpha 및 rho(M(inf))=0 | APNUM p.428 (Def 2.2) |
| zero/strong stability | Def 2.1 (2.6) 곱 유계; Def 2.4 strongly stable: sigma=1에서 B 고유치 lambda1=1, abs(lambda_i)<1 | APNUM pp.428-429 |
| PPSW2 정의 | beta=0, G=diag(gamma_i) 다중 암시 | APNUM p.429 |
| Gamma, B | `Gamma = -G Theta`, `Theta = V S_m P V^-1` (3.2); `B = Theta - sigma_m G Theta V D F0^T V^-1` (3.3); V=(c_i^(j-1)), P=(C(j-1,i-1)), S=diag(1,sigma,..,sigma^(s-1)), D=diag(1..s), F0=(delta_(i-1,j)); A=-Gamma | APNUM p.429-430; BIT (2.4),(2.5) p.200; SINUM (16),(20) p.270 |
| gamma_s | (3.6): `1/gamma_i = sigma phi'(1+sigma c_i)/phi(1+sigma c_i) = sigma sum_j 1/(1+sigma c_i - c_j)`, PPSW2-s에서는 i=s 만 사용 (BIT (2.12) p.201과 동일) | APNUM p.430-431 |
| gamma_i, i<s | (3.8): `gamma_i = gamma0_i/(1+sigma gamma1_i)`, i=1..s-1 | APNUM p.431 |
| PPSW2-6 | s=6, c=(-1.702,-0.51,-0.195797762,0.155193392,0.755672784,1), gamma0=(0.214789562,0.249182503,0.237832565,0.245204742,0.353848357), gamma1=(0.566334104,0.496228487,0.518584061,0.498857638,0.502412430), alpha=81 deg (300 dpi 이미지로 숫자 재확인) | APNUM p.432 |
| PPSW2-5 (APNUM) | c=(-1.09,-0.2290,0.51,0.63,1), gamma0/gamma1 = 위 PPSW2-6의 앞 4개와 동일, alpha=86 | APNUM p.431 |
| PPSW2-5 (BIT) | c=(-1.09,-0.229,0.51,0.63,1), gamma-bar0=(0.2066,0.1544,0.32,0.3304), gamma-bar1=(0.5811,0.4917,0.5887,0.4688), gamma_s from (2.12) | BIT p.213 |
| 인쇄되지 않은 것 | B,A,Gamma,Theta 수치, sigma_a/sigma_b 수치(Thm 3.2는 존재만), B(1) 고유치, 행렬 노름 | (해당 없음) |
| sigma 사용 상한 | PPSW2-3,4: 1.5; 나머지(PPSW2-5,6 포함) sigmamax=1.2, 하한 없음 | APNUM p.434 (Fig.1,2 근거; 그림 미열람) |
| 안정구간 정리 | Thm 3.2: strongly stable이면 [sigma_a,sigma_b], sigma_a<1<sigma_b 존재 (3.9),(3.10); "bounds can be rather close to one" | APNUM p.432-433 |
| 오차 추정 | (3.7): `Y_ms - sum_j theta_sj Y_(m-1,j)` (stage s는 (3.6)에 의해 order s; 아래 C에서 재확인) | APNUM p.431, 434 |
| 시작값 | 실험에서 ROWMAP (APNUM p.434), SINUM은 RODAS (p.277). c<0 노드의 시작값 산출 절차는 미인쇄 | |
| 비교용 SINUM | 단일 gamma(PPSW1) 논문. Gamma(q),AB(q+1) 및 (16),(20) 동일 | SINUM pp.269-270 |

식 번호 대응(저널 -> 사용자 표기): Halle (4.3)/(4.5)에 해당하는 내용은 저널 (3.6),(3.8)에, (3.9),(3.10)은 저널 Thm 3.2의 (3.9),(3.10)에 대응하는 것으로 **내용 기준 추정**(프리프린트는 이번에 재확인하지 않음, UNRESOLVED).

문헌 내 표기 불일치(관찰): BIT (3.1)의 Gamma(q) 줄은 `sum_j gamma_ij p(c_j) = gamma_i p(1+sigma c_i)`로 인쇄되어 BIT (2.4) Gamma=-G Theta와 **부호가 반대**. 본 유도에서 BIT (3.1) 경로의 B는 일치했고 Gamma만 부호 반대(표 참조). APNUM/SINUM 부호(Gamma=-G Theta, A=-Gamma=G Theta)를 사용.

## B. 유도 (재현 절차)

입력: 위 인쇄 십진수만(문자열 -> mpmath mpf, 80자리). sigma마다
1. `gamma_i(sigma)` = (3.8) (i<6), `gamma_6` = (3.6)에서 c_s=1.
2. 경로 1: 행렬식 (3.2),(3.3) 그대로.
3. 경로 2: tau_j=(c_j-1)/sigma; `sum_j theta_ij tau_j^k = c_i^k` (k<s)를 Vandermonde 전치로 풀어 Theta, A=G Theta, Gamma=-A; AB(s)를 `sum_j b_ij tau_j^k = c_i^k - k sum_j a_ij tau_j^(k-1)`로 풀어 B. (V^-1, Pascal, 시프트 행렬 미사용)
4. 경로 3: BIT (3.1) 다항 규칙 `sum_j b_ij p(c_j) = p(1+sigma c_i) - gamma_i sigma p'(1+sigma c_i)` 를 c-단항식에서 풀어 B (Gamma는 부호 반대로 인쇄됨).

경로 일치(최대 절대차):

| sigma | B(1 vs 2) | A(1 vs 2) | Gamma(1 vs 2) | B(1 vs 3) | max abs B |
|---|---|---|---|---|---|
| 0.3 | 1.12e-79 | 3.37e-80 | 3.37e-80 | 7.64e-80 | 2.20637 |
| 0.5 | 1.62e-79 | 2.45e-79 | 2.45e-79 | 6.54e-80 | 4.1385 |
| 0.8 | 3.49e-78 | 6.41e-79 | 6.41e-79 | 1.52e-79 | 11.6106 |
| 1.0 | 3.74e-78 | 1.35e-78 | 1.35e-78 | 1.27e-79 | 20.0781 |
| 1.1 | 7.46e-78 | 1.96e-78 | 1.96e-78 | 2.02e-79 | 25.6983 |
| 1.2 | 6.68e-78 | 9.45e-79 | 9.45e-79 | 5.4e-79 | 32.4197 |
| 1.5 | 2.87e-77 | 4.45e-78 | 4.45e-78 | 1.08e-78 | 66.3326 |
| 2.0 | 2.73e-77 | 1.16e-77 | 1.16e-77 | 1.89e-78 | 308.85 |

(경로 3의 Gamma 차이 13.5~210은 BIT (3.1) 부호 때문이며 오류가 아님: Gamma_r3 = +G Theta.)

### sigma=1 결과 (경로 1, 40자리는 JSON)

G(1) = diag(0.137128829316, 0.16654040821, 0.156614685422, 0.163594417364, 0.235520120797, 0.288213064091) (gamma_6 = 0.288213064091166067).

B(1) (행 i=1..6, 열 j=1..6, 10 유효숫자):

| 0.02165382106 | 3.319944221 | -4.130263954 | 2.217192745 | -0.6820126744 | 0.2534858419 |
| -0.001470500533 | 0.1628703634 | -0.6384695055 | 1.359374882 | 0.2032150797 | -0.08552031897 |
| -0.0007116915291 | 0.06127623578 | -0.1988581665 | 0.2512051135 | 1.176220714 | -0.2891322056 |
| 0.005332743935 | -0.4049177688 | 1.221722753 | -1.341105373 | 1.135604678 | 0.3833629668 |
| -0.01956496906 | 2.002761434 | -6.697144191 | 8.406010295 | -10.57794925 | 7.885886681 |
| -0.04448388147 | 4.500101348 | -14.8140937 | 18.03197325 | -20.07805215 | 13.40455514 |

A(1) = G Theta = -Gamma(1):

| 0.001280048411 | 0.3100411936 | -0.2962692607 | 0.1503200883 | -0.04477974582 | 0.01653650554 |
| -0.0002339919268 | 0.02385259627 | -0.08762532056 | 0.1542115294 | 0.09844861962 | -0.0221130246 |
| 5.732345406e-05 | -0.005083706815 | 0.01683188381 | -0.02228267317 | 0.1509587649 | 0.01613309321 |
| -0.00103055695 | 0.08223206352 | -0.2553544248 | 0.2964019628 | -0.375826779 | 0.4171721519 |
| -0.05687808854 | 4.036694944 | -11.80731483 | 12.36938878 | -10.02868889 | 5.722318216 |
| -0.1646793292 | 11.29525438 | -32.5287538 | 33.26550341 | -24.98366673 | 13.40455514 |

A+Gamma = 0 (최대 절대값 0.0) => stiffly accurate, M(inf)=0. B(1)의 행합 - 1 = ~1e-79 (AB(1)).
임의 sigma의 전체 행렬은 `python3 -I` 스크립트(아래)로 재생성. 일부 B 항목이 최대 약 20(sigma=1), 66(sigma=1.5)로 큼; Vandermonde 조건수(2-norm) 400.63197.

## C. 검증

### C(i) 차수 조건 잔차 (max over rows, 80자리)

Gamma(k) k=0..5 최대 / AB(k) k=5 최대 / AB k=6 (= AB(7) 마지막 조건):

| sigma | Gamma k=0..5 | AB k=5 | AB k=6 |
|---|---|---|---|
| 0.3 | 2.11e-81/2.74e-80/1.6e-79/1.09e-78/6.33e-78/3.23e-77 | 7.75e-78 | 184.0 (rows1-5 min 23.1, row6 4.24e-77) |
| 0.5 | 4.22e-81/1.9e-80/6.11e-80/3.62e-79/1.14e-78/4.76e-78 | 4.23e-78 | 12.1 (rows1-5 min 1.23, row6 2.35e-77) |
| 0.8 | 1.37e-80/2.32e-80/3.27e-80/2.3e-79/4.17e-79/1.13e-78 | 2.2e-78 | 0.924 (rows1-5 min 0.0196, row6 7.25e-78) |
| 1.0 | 3.0e-80/3.58e-80/4.8e-80/1.61e-79/2.96e-79/6.65e-79 | 1.79e-78 | 0.451 (rows1-5 min 0.0408, row6 4.39e-78) |
| 1.1 | 2.37e-80/4.38e-80/1.1e-79/2.44e-79/3.3e-79/6.3e-79 | 9.7e-79 | 0.629 (rows1-5 min 0.0258, row6 2.66e-78) |
| 1.2 | 3.06e-80/1.13e-79/1.76e-79/2.13e-79/3.69e-79/5.76e-79 | 1.0e-78 | 0.668 (rows1-5 min 0.00753, row6 2.37e-78) |
| 1.5 | 1.61e-80/1.08e-79/3.96e-79/4.64e-79/5.48e-79/5.12e-79 | 2.43e-78 | 0.237 (rows1-5 min 0.00328, row6 2.81e-78) |
| 2.0 | 3.74e-80/4.01e-79/1.26e-79/1.81e-79/1.76e-79/1.88e-79 | 9.11e-79 | 2.13 (rows1-5 min 0.000287, row6 1.13e-78) |

- Gamma(6)(k<=5)과 AB(6)(k<=5): 전 sigma에서 <=1e-77 => Lemma 2.1 (p=5): 모든 stage 국소 잔차 O(h^5).
- AB(7) (k=6): stage 1..5에서 O(1e-2..1e2) **비영** => 차수 6 조건 불성립(국소 잔차 정확히 5차). stage 6만 ~1e-77 (슈퍼정합, (3.6)). Gamma k=6은 비영(차수 6 불가 추가 증거).
- 한계: 이는 잔차 차수이며 전역 차수 5(정확히)는 Thm 2.1의 곱 안정성 + 실측 필요. NUMERICALLY_CHECKED.
- 추정자 (3.7) 정당화: stage 6 행의 AB(7) 잔차 0 확인(위 표 row6) => 국소오차 O(h^6)의 추정에 사용 가능.

### C(ii) zero-stability, (iii) L(alpha), 안정구간

| sigma | abs eig B(sigma) (내림차순) | min gamma_i | alpha(frozen sigma) deg |
|---|---|---|---|
| 0.3 | 1.0, 0.359625828249, 0.114711748702, 0.0175018946055, 0.0175018946055, 0.00144856838486 | 0.183596 | - |
| 0.5 | 1.0, 0.46307279903, 0.240161163884, 0.0901669109502, 0.0901669109502, 0.0171075735088 | 0.16739 | - |
| 0.8 | 1.0, 0.712060289874, 0.484861184981, 0.288727693129, 0.288727693129, 0.166285415008 | 0.147818 | 90.0 |
| 1.0 | 1.0, 0.893553324218, 0.519213308576, 0.519213308576, 0.463041204765, 0.463041204765 | 0.137129 | - |
| 1.1 | 1.0, 0.947401252392, 0.635934094769, 0.635934094769, 0.567364419123, 0.567364419123 | 0.132344 | 65.104 |
| 1.2 | 1.0, 0.948639715437, 0.742895803819, 0.742895803819, 0.673402739373, 0.673402739373 | 0.127881 | 32.7 |
| 1.5 | 1.0, 0.952821433793, 0.952821433793, 0.778968523426, 0.778968523426, 0.504846502666 | 0.116134 | 0.0 |
| 2.0 | 9.66037487563, 6.66127666259, 2.22808967639, 2.22808967639, 1.28091911251, 1.0 | 0.100714 | - |

- sigma=1: lambda1=1, 나머지 <= 0.8936 => Def 2.4 strongly stable (모듈러스 기준).
- B(sigma) pointwise 고유치 조건(|lambda2|<1, gamma>0)을 만족하는 sigma 격자(간격 0.025, 0.05~3.05): 연속 구간 [0.050, 1.550]. 이는 **고유치 필요조건**일 뿐 (2.6) 곱 안정성의 증명이 아님. sigma=2에서 abs eig 9.66로 불안정.
- alpha(sigma 고정): 1.0에서 81.609 deg; 0.8-0.9에서 90; 1.1에서 65.104; 1.2에서 32.7; 1.5에서 0. 인쇄 alpha=81 deg는 sigma=1 값으로 해석하면 정합. (L(alpha) 정의상 rho(M(inf))=0 이므로 L; alpha는 r in [1e-5,1e9] 로그 500점, phi 0.125 deg 격자 + 이분법. 증명 아님.)
- 무작위 sigma 열(APNUM Fig.2 방식: sigma=0.1+2r를 [2-smax, smax]로 클립; 3열x10000스텝) 곱 최대 원소: smax=1.05: 226, smax=1.1: 144, smax=1.15: 205, smax=1.2: 273, smax=1.3: 439, smax=1.5: 2528. (정의 해석 불확실: 원문 'L(m)' 정의가 텍스트에서 깨져 곱 최대원소로 해석. 인쇄 값이 없어 비교 불가.)
- 파이프라인 검증(동일 코드로 인쇄 alpha 재현):

| 방법 | 인쇄 alpha | 수치 alpha(sigma=1) | abs eig B(1) 상위2 |
|---|---|---|---|
| PPSW2-6 | 81 | 81.609 | 1.0 / 0.893553324218 |
| PPSW2-5(APNUM) | 86 | 0.0 | 5.8943676016 / 1.0 |
| PPSW2-5(BIT-gbar) | None | 86.437 | 1.0 / 0.448360603156 |
| PPSW2-4 | 83 | 83.645 | 1.0 / 0.61823475697 |
| PPSW2-3 | 90 | 90.0 | 1.0 / 0.222167524 |

PPSW2-3 (90), PPSW2-4 (83.6 vs 83), PPSW2-6 (81.6 vs 81), PPSW2-5(BIT) (86.4 vs 86)는 일치. PPSW2-5(APNUM 인쇄 gamma)만 불일치/불안정.

### C(iv) 같은 스텝 stage 독립

LHS 행렬 `I - h_m G (x) T_m`은 G 대각이므로 블록 대각, RHS는 Y_(m-1), f(Y_(m-1))만 사용 => 같은 스텝 stage 의존 행렬 = 단위행렬(i!=j 항 0). 구조적(코드 아닌 정의) 확인. 의존은 이전 스텝의 전 stage(밀집 B, A, Gamma)에만 존재.

### C(v) sigma 영역
위 표와 구간: 고유치/alpha/gamma>0 모두 만족하는 연속 영역은 대략 sigma<=1.1에서 alpha>=65 deg, sigma=1.2에서 alpha 32.7 deg. NUMERICALLY_CHECKED, not proven.

## 6. 9자리 반올림 감도 (소유자 결정 사항)

입력(c1..c5, gamma0, gamma1) 각각에 +-5e-10 균일 섭동 12회(sigma=1): max |dB| = 6.15e-08, max |d abs(lambda2)| = 4.25e-08, max d alpha = 0.011 deg, 섭동 입력에서도 차수 조건 잔차(k<=5) <= 1.1e-78.
해석: 차수는 어떤 입력에서도 구성상 정확. 9자리는 최적화 결과를 표현한 정밀도이며, 안정성 지표는 ~1e-8 수준에서 변화(위험 낮음). 프로덕션에서는 인쇄 십진수를 정확 유리수(예 -1.702 = -1702/1000)로 두고 계수를 런타임 재계산하는 방안이 합리적이나, 이는 소유자 결정.

## UNRESOLVED
- Halle-preprint equation numbers (3.9),(3.10),(4.3),(4.5) not re-read; mapping to journal numbering is by content only
- journal Figs 1-2 (norm (3.10), bnorm vs sigma) not read; no printed sigma_a,sigma_b
- global order 5 (empirical) not measured; Thm 2.1 needs product stability, which is only existential (Thm 3.2)
- startup values Y_0j at off-step (negative) nodes: printed only as ROWMAP (APNUM p.434) / RODAS (SINUM p.277); procedure for c_j<0 not printed
- error-estimator constant/scaling and step controller not printed for PPSW2-6 beyond (3.7)
- whether eigenvalue moduli pairs of B(1) are complex-conjugate pairs or Jordan blocks (moduli only reported)

## 영수증

- cmd: `sha256sum <3 PDFs in /home/cosmosapjw/vigilode>` exit 0 pdf_sha256={'BIT45_2005': 'e9f8ef8ca7a788df8046e6adcc1ed938cfa1d342cdf6cbfc74496c67f2236e0b', 'APNUM48_2004': '7e8d17deeecf896bf4128493d4eaec26b50cf062b8a9d10d1397acb6ad84d01d', 'SINUM42_2004': 'da9cd10e45c539076a3f817fb27738e1bb75c49817710ac2696a8a80e39f9993'}
- cmd: `pdftotext -layout <BIT pdf> bit.txt; pdftotext -layout <APNUM pdf> apnum.txt; pdftotext <SINUM pdf> sinum_raw.txt` exit 0 txt_sha256={'bit': 'ef8143e042fd89301c737765fb42c7bb2547cfe5bd49c9cb66ba9b44fba77e7b', 'apnum': 'eea6064fe5fe0441dc19829d748e7d6037690bea55cd7a347a28e8ab6ba74be1', 'sinum_raw': 'b08eaf1a55c42af1918de9bd47624e05352f327496f603c8237e8c8094a214b7'}
- cmd: `pdftoppm -r 300 -f 6 -l 8 -png apnum.pdf ap  (journal pp.430-432; viewed with Read; PPSW2-6 line on p.432 read from the image)` exit 0 
- cmd: `python3 -I pw03a_ppsw2_derive.py > out.json 2> err.txt (run twice)` exit 0 stdout_sha256=1f5b462f40c41edd102d332d8e840bc4865cf6187535c517afab0032640a4919, stdout_sha256_second_run=1f5b462f40c41edd102d332d8e840bc4865cf6187535c517afab0032640a4919, stderr_sha256=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855, stderr_bytes=0, script_sha256=e6d10631959bb7b5086e762193139a937492563cd6e462bf846bb736e5768ef9, wall_s=76

## 전체 유도 스크립트 (sha256 e6d10631959bb7b5086e762193139a937492563cd6e462bf846bb736e5768ef9)

```python
#!/usr/bin/env python3
"""PW03A: reconstruct PPSW2-s (Weiner-Schmitt-Podhaisky, APNUM 48 (2004) 425-439, eqs (3.2),(3.3),(3.6),(3.8)).
Run: python3 -I pw03a_ppsw2_derive.py > out.json     (needs numpy, mpmath)
Inputs are ONLY the printed nodes/gamma0/gamma1 (APNUM p.431-432). Everything else derived.
"""
import json, sys, math, random
import numpy as np
import mpmath as mp
from mpmath import mpf, matrix
mp.mp.dps = 80

METHODS = {
 "PPSW2-6": dict(c=["-1.702","-0.51","-0.195797762","0.155193392","0.755672784","1"],
                 g0=["0.214789562","0.249182503","0.237832565","0.245204742","0.353848357"],
                 g1=["0.566334104","0.496228487","0.518584061","0.498857638","0.502412430"], alpha_printed=81),
 "PPSW2-5(APNUM)": dict(c=["-1.09","-0.2290","0.51","0.63","1"],
                 g0=["0.214789562","0.249182503","0.237832565","0.245204742"],
                 g1=["0.566334104","0.496228487","0.518584061","0.498857638"], alpha_printed=86),
 "PPSW2-5(BIT-gbar)": dict(c=["-1.09","-0.229","0.51","0.63","1"],
                 g0=["0.2066","0.1544","0.32","0.3304"], g1=["0.5811","0.4917","0.5887","0.4688"], alpha_printed=None),
 "PPSW2-4": dict(c=["-0.38333","-0.25","0.825","1"],
                 g0=["0.584888524","0.536977593","0.467201038"],
                 g1=["0.472969203","0.506264075","0.496391355"], alpha_printed=83),
 "PPSW2-3": dict(c=["-0.1","0.7","1"], g0=["2.55","0.4005"], g1=["0.5","0.5"], alpha_printed=90),
}

def mk(m, pert=None):
    c=[mpf(x) for x in m["c"]]; g0=[mpf(x) for x in m["g0"]]; g1=[mpf(x) for x in m["g1"]]
    if pert is not None:
        rng, eps = pert
        c=[ci+(mpf(rng.uniform(-1,1))*eps if i<len(c)-1 else 0) for i,ci in enumerate(c)]
        g0=[x+mpf(rng.uniform(-1,1))*eps for x in g0]; g1=[x+mpf(rng.uniform(-1,1))*eps for x in g1]
    return c,g0,g1

def gammas(c,g0,g1,sig):
    s=len(c)
    g=[g0[i]/(1+sig*g1[i]) for i in range(s-1)]                      # (3.8)
    g.append(1/(sig*sum(1/(1+sig-cj) for cj in c)))                  # (3.6) with i=s, c_s=1
    return g

# ---------- Route 1: printed matrix formulas (3.2),(3.3) ----------
def route1(c,g,sig):
    s=len(c)
    V=matrix(s,s); P=matrix(s,s); S=mp.zeros(s,s); D=mp.zeros(s,s); F0=mp.zeros(s,s); G=mp.zeros(s,s)
    for i in range(s):
        for j in range(s):
            V[i,j]=c[i]**j                                   # V=(c_i^{j-1})
            P[i,j]=mp.binomial(j,i)                          # P=C(j-1,i-1)
        S[i,i]=sig**i; D[i,i]=i+1; G[i,i]=g[i]
        if i>=1: F0[i,i-1]=1                                 # F0=(delta_{i-1,j})
    Vi=V**-1
    Th=V*S*P*Vi
    Gam=-G*Th; A=G*Th
    B=Th-sig*G*Th*V*D*F0.T*Vi
    return B,A,Gam,Th

# ---------- Route 2: direct solve of order conditions in shifted variable tau_j=(c_j-1)/sig ----------
def route2(c,g,sig):
    s=len(c)
    tau=[(cj-1)/sig for cj in c]
    W=matrix(s,s)
    for j in range(s):
        for k in range(s): W[j,k]=tau[j]**k                   # W[j,k]=tau_j^k
    Wt=W.T                                                   # rows k, columns j
    Th=matrix(s,s); Gam=matrix(s,s); A=matrix(s,s); B=matrix(s,s)
    for i in range(s):
        rhs=matrix([c[i]**k for k in range(s)])              # Gamma(s): sum_j theta_ij tau_j^k = c_i^k
        th=mp.lu_solve(Wt,rhs)
        for j in range(s):
            Th[i,j]=th[j]; Gam[i,j]=-g[i]*th[j]; A[i,j]=g[i]*th[j]   # gamma_ij=-gamma_i theta_ij, A=-Gamma
        # AB(s): c_i^k = sum_j b_ij tau_j^k + k sum_j a_ij tau_j^{k-1}
        r2=matrix([c[i]**k - (k*sum(A[i,j]*tau[j]**(k-1) for j in range(s)) if k>0 else 0) for k in range(s)])
        b=mp.lu_solve(Wt,r2)
        for j in range(s): B[i,j]=b[j]
    return B,A,Gam,Th

# ---------- Route 3: BIT eq (3.1) polynomial-rule in monomials of c (no tau, no Pascal) ----------
def route3(c,g,sig):
    s=len(c)
    Vc=matrix(s,s)
    for j in range(s):
        for k in range(s): Vc[k,j]=c[j]**k                    # rows k, cols j
    B=matrix(s,s); Gam=matrix(s,s)
    for i in range(s):
        x=1+sig*c[i]
        rhsB=matrix([x**k - g[i]*sig*(k*x**(k-1) if k>0 else 0) for k in range(s)])   # AB(q) of BIT (3.1)
        rhsG=matrix([g[i]*x**k for k in range(s)])                                    # Gamma(q) of BIT (3.1)
        b=mp.lu_solve(Vc,rhsB); gm=mp.lu_solve(Vc,rhsG)
        for j in range(s): B[i,j]=b[j]; Gam[i,j]=gm[j]
    return B,-Gam,Gam

def maxabs(M): return max(abs(M[i,j]) for i in range(M.rows) for j in range(M.cols))

def residuals(c,g,sig,B,A,Gam,kmax=8):
    """Gamma(k) and AB(k) residual rows, k=0..kmax-1, from APNUM (2.4),(2.5)."""
    s=len(c); out={"Gamma":[], "AB":[], "AB_rows":[]}
    for k in range(kmax):
        rg=0; rb=0; rows=[]
        for i in range(s):
            e1=g[i]*c[i]**k+sum(Gam[i,j]*((c[j]-1)/sig)**k for j in range(s))
            e2=c[i]**k-sum(B[i,j]*((c[j]-1)/sig)**k for j in range(s))-(k*sum(A[i,j]*((c[j]-1)/sig)**(k-1) for j in range(s)) if k>0 else 0)
            rg=max(rg,abs(e1)); rb=max(rb,abs(e2)); rows.append(abs(e2))
        out["Gamma"].append(rg); out["AB"].append(rb); out["AB_rows"].append(rows)
    return out

def fmt(x,n=3): return mp.nstr(x,n)

def to_np(M): return np.array([[float(M[i,j]) for j in range(M.cols)] for i in range(M.rows)])

def stab_alpha(Bnp,gnp,nphi=721,nr=500,rlo=-5,rhi=9):
    """alpha = inf{phi: sup_r rho(M(z))>=1}, z=r exp(i(pi-phi)); M=(I-zG)^-1 B (beta=0)."""
    rs=np.logspace(rlo,rhi,nr); s=len(gnp)
    def f(phi_deg):
        z=rs*np.exp(1j*(np.pi-np.radians(phi_deg)))
        M=(1.0/(1.0-z[:,None]*gnp[None,:]))[:,:,None]*Bnp[None,:,:]
        return float(np.max(np.abs(np.linalg.eigvals(M))))
    phis=np.linspace(0.0,90.0,nphi); prev=0.0; a=90.0; fv=[]
    for p in phis:
        v=f(p); fv.append(v)
        if v>=1.0-1e-12:
            a=prev; lo,hi=prev,p
            for _ in range(40):
                mid=0.5*(lo+hi)
                if f(mid)>=1.0-1e-12: hi=mid
                else: lo=mid
            a=lo; break
        prev=p
    return a

def main():
    out={"env":{"python":sys.version.split()[0],"numpy":np.__version__,"mpmath":mp.__version__,"dps":mp.mp.dps}}
    sigs=[mpf(x) for x in ["0.3","0.5","0.8","1","1.1","1.2","1.5","2"]]
    allres={}
    for name,m in METHODS.items():
        c,g0,g1=mk(m); s=len(c); R={"s":s}
        R["c"]=[mp.nstr(x,12) for x in c]
        agree={}; ordtab={}; sigdata={}
        for sig in sigs:
            g=gammas(c,g0,g1,sig)
            B1,A1,G1,T1=route1(c,g,sig); B2,A2,G2,T2=route2(c,g,sig); B3,A3,G3=route3(c,g,sig)
            agree[mp.nstr(sig,3)]=dict(
              B_r1_r2=fmt(maxabs(B1-B2)),A_r1_r2=fmt(maxabs(A1-A2)),G_r1_r2=fmt(maxabs(G1-G2)),
              B_r1_r3=fmt(maxabs(B1-B3)),G_r1_r3=fmt(maxabs(G1-G3)),Theta_r1_r2=fmt(maxabs(T1-T2)),
              maxabsB=fmt(maxabs(B1),6),rowsum_B_minus_1=fmt(maxabs(B1*matrix([1]*s)-matrix([1]*s))))
            res=residuals(c,g,sig,B1,A1,G1)
            ordtab[mp.nstr(sig,3)]=dict(Gamma_max_by_k=[fmt(x) for x in res["Gamma"]],AB_max_by_k=[fmt(x) for x in res["AB"]],
                 AB_k_eq_s_rows=[fmt(x) for x in res["AB_rows"][s]], AB_k_eq_s_minus1_max=fmt(res["AB"][s-1]))
            ev=mp.eig(B1)[0]; evn=sorted([abs(e) for e in ev],reverse=True)
            sigdata[mp.nstr(sig,3)]=dict(gamma=[mp.nstr(x,15) for x in g],gamma_min=mp.nstr(min(g),6),
                 abs_eig_B=[mp.nstr(x,12) for x in evn])
        R["route_agreement"]=agree; R["order_residuals"]=ordtab; R["per_sigma"]=sigdata
        # matrices at sigma=1 and 1.2 (route 1, 40 digits)
        mats={}
        for sig in [mpf(1),mpf("1.2")]:
            g=gammas(c,g0,g1,sig); B1,A1,G1,T1=route1(c,g,sig)
            mats[mp.nstr(sig,3)]=dict(G=[mp.nstr(x,40) for x in g],
                B=[[mp.nstr(B1[i,j],40) for j in range(s)] for i in range(s)],
                A=[[mp.nstr(A1[i,j],40) for j in range(s)] for i in range(s)],
                Gamma=[[mp.nstr(G1[i,j],40) for j in range(s)] for i in range(s)])
        R["matrices_route1_40digits"]=mats
        # stiff accuracy / dependency: beta = A+Gamma
        sig=mpf(1); g=gammas(c,g0,g1,sig); B1,A1,G1,_=route1(c,g,sig)
        R["beta_max_abs(A+Gamma)"]=fmt(maxabs(A1+G1))
        R["same_step_dependency_matrix"]=[[1 if i==j else 0 for j in range(s)] for i in range(s)]  # LHS I-h G(x)T is block-diagonal since G=diag
        # stability numerics sigma=1
        gnp=np.array([float(x) for x in g]); Bnp=to_np(B1)
        R["alpha_deg_sigma1_numeric"]=round(stab_alpha(Bnp,gnp),3)
        R["alpha_printed"]=m["alpha_printed"]
        # alpha at other sigmas
        al={}
        for sg in ["0.8","0.9","1.1","1.2","1.5"]:
            sig=mpf(sg); gg=gammas(c,g0,g1,sig); Bq,_,_,_=route1(c,gg,sig)
            al[sg]=round(stab_alpha(to_np(Bq),np.array([float(x) for x in gg]),nphi=361,nr=300),3)
        R["alpha_deg_by_sigma"]=al
        allres[name]=R
    out["methods"]=allres

    # sigma domain for PPSW2-6: |lambda2(B(sigma))|<1 , gamma>0
    c,g0,g1=mk(METHODS["PPSW2-6"]); s=6
    dom=[]
    for k in range(0,121):
        sg=mpf("0.05")+mpf(k)*mpf("0.025")
        g=gammas(c,g0,g1,sg); Bq,_,_,_=route2(c,g,sg)
        e=np.linalg.eigvals(to_np(Bq)); ae=sorted(np.abs(e),reverse=True)
        dom.append((float(sg),float(ae[0]),float(ae[1]),float(min(g))))
    out["sigma_scan_PPSW2-6"]=dict(columns=["sigma","|lam1|","|lam2|","min_gamma"],rows=[[round(x,6) for x in r] for r in dom])
    ok=[r[0] for r in dom if r[2]<1-1e-12 and r[3]>0]
    # contiguous interval around 1
    lo=1.0;hi=1.0
    step=0.025
    xs={round(r[0],6):r for r in dom}
    sg=1.0
    while round(sg-step,6) in xs and xs[round(sg-step,6)][2]<1 and xs[round(sg-step,6)][3]>0: sg-=step
    lo=sg; sg=1.0
    while round(sg+step,6) in xs and xs[round(sg+step,6)][2]<1 and xs[round(sg+step,6)][3]>0: sg+=step
    hi=sg
    out["sigma_pointwise_interval_PPSW2-6"]=dict(lo=lo,hi=hi,note="pointwise |lambda2(B(sigma))|<1 & gamma_i>0 on grid step 0.025; NOT a proof of product stability")

    # product growth (Fig.2 style) for PPSW2-6
    rng=np.random.default_rng(12345); growth={}
    c,g0,g1=mk(METHODS["PPSW2-6"])
    cn=np.array([float(x) for x in c]); g0n=np.array([float(x) for x in g0]); g1n=np.array([float(x) for x in g1])
    def Bdouble(sg):
        tau=(cn-1)/sg; s=len(cn)
        gam=np.concatenate([g0n/(1+sg*g1n), [1/(sg*np.sum(1/(1+sg-cn)))]])
        Wt=np.vander(tau,s,increasing=True).T
        Th=np.linalg.solve(Wt, np.array([cn**k for k in range(s)])).T       # rows i: theta_i
        A=gam[:,None]*Th
        B=np.zeros((s,s))
        for i in range(s):
            r2=np.array([cn[i]**k-(k*np.dot(A[i],tau**(k-1)) if k>0 else 0) for k in range(s)])
            B[i]=np.linalg.solve(Wt,r2)
        return B
    for smax in [1.05,1.1,1.15,1.2,1.3,1.5]:
        worst=0.0
        for trial in range(3):
            L=np.eye(6)
            for m_ in range(10000):
                sg=0.1+2*rng.random(); sg=max(2-smax,min(sg,smax))
                L=Bdouble(sg)@L
                worst=max(worst,float(np.max(np.abs(L))))
        growth[str(smax)]=worst
    out["random_product_max_entry_PPSW2-6_3x10000"]=growth

    # sensitivity to 9-decimal rounding: perturb all printed numbers by uniform +-5e-10 (c_s=1 fixed)
    rnd=random.Random(2026); base_c,base_g0,base_g1=mk(METHODS["PPSW2-6"])
    gb=gammas(base_c,base_g0,base_g1,mpf(1)); Bb,_,_,_=route1(base_c,gb,mpf(1))
    ebase=sorted([abs(e) for e in mp.eig(Bb)[0]],reverse=True)
    abase=out["methods"]["PPSW2-6"]["alpha_deg_sigma1_numeric"]
    dl2=[];dal=[];dB=[];ordres=[]
    for t in range(12):
        c,g0,g1=mk(METHODS["PPSW2-6"],pert=(rnd,mpf("5e-10")))
        g=gammas(c,g0,g1,mpf(1)); Bp,Ap,Gp,_=route1(c,g,mpf(1))
        ep=sorted([abs(e) for e in mp.eig(Bp)[0]],reverse=True)
        dl2.append(float(abs(ep[1]-ebase[1]))); dB.append(float(maxabs(Bp-Bb)))
        dal.append(abs(stab_alpha(to_np(Bp),np.array([float(x) for x in g]),nphi=361,nr=300)-abase))
        r=residuals(c,g,mpf(1),Bp,Ap,Gp); ordres.append(float(max(max(r["Gamma"][:6]),max(r["AB"][:6]))))
    out["rounding_sensitivity_PPSW2-6"]=dict(perturbation="uniform +-5e-10 on c_1..c_{s-1}, gamma0, gamma1 (12 samples, sigma=1)",
        max_change_abs_lambda2=max(dl2),max_change_B_entries=max(dB),max_change_alpha_deg=max(dal),
        max_order_condition_residual_k_lt_6_perturbed_inputs=max(ordres),
        base_lambda_abs=[mp.nstr(x,12) for x in ebase])
    # gamma_s positivity / condition numbers
    for name in ["PPSW2-6"]:
        c,g0,g1=mk(METHODS[name]); Vc=matrix(6,6)
        for i in range(6):
            for j in range(6): Vc[i,j]=c[i]**j
        out["cond_Vandermonde_PPSW2-6_2norm"]=mp.nstr(mp.cond(Vc),8)
    json.dump(out,sys.stdout,indent=1)
main()

```
