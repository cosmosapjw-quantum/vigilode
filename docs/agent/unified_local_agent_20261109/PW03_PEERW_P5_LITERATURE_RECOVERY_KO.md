# PW03 Peer-W p>=5 문헌 회수 보고 (문헌 전용, 2026-11-10)

범위: 문헌 조사만 수행. 기존 파일 수정 없음, git 없음, Rust 없음, 무료 LLM offload 미사용.
기계 판독본: `PW03_PEERW_P5_LITERATURE_RECOVERY.json` (같은 폴더).

## 0. 전체 판정 (정직한 결론)

- **PARTIAL**: 읽을 수 있는 1차 문헌에서 p>=5(가변 스텝)이며 완전 stage-parallel인 Peer-W 방법은 **PPSW2-6** (s=6, p=s-1=5) 하나만 확인됨.
- 그러나 논문에 인쇄된 것은 노드 `c`와 `gamma0`, `gamma1`(최대 소수 9자리)뿐이며 **B/A/G 행렬 전체 tableau는 인쇄되어 있지 않음**. 식 (3.9), (3.10), (4.3), (4.5)로 유도해야 하며 이 유도는 검증하지 않음.
- 읽은 것은 저널본이 아니라 **Halle 프리프린트**(Report 03-02)임. 저널본(APNUM 48, 2004)의 값이 동일한지는 UNRESOLVED.
- 완전 stage-parallel이 아니면서 p>=5인 표(Rosenbrock-type peer s=6, Lang-Schmitt IP4o5)는 존재하지만 기준 불충족.
- 따라서 "p>=5 완전 병렬 Peer-W의 완전한 tableau를 1차 문헌에서 읽었다"는 주장은 **하지 못함**.

## 1. 후보 목록과 실제 열람 범위

| ID | 서지 | DOI/URL | 열람 범위 |
|---|---|---|---|
| C2 | Weiner, Schmitt, Podhaisky, "Parallel 'peer' two-step W-methods and their application to MOL-systems", Appl. Numer. Math. 48 (2004) 425-439 | 10.1016/j.apnum.2003.10.005 ; 프리프린트 https://sim.mathematik.uni-halle.de/reports/sources/2003/03-02report.ps | **프리프린트 전문 17쪽** (텍스트층 + 10~11쪽 이미지). 저널본 미열람 |
| C1 | Schmitt, Weiner, "Parallel two-step W-methods with peer variables", SIAM J. Numer. Anal. 42 (2004) 265-282 | 10.1137/S0036142902411057 | **초록만** (Crossref). 저자 프리프린트 URL은 404, 출판사 본문은 유료 |
| C3 | Podhaisky, Weiner, Schmitt, "Rosenbrock-type 'peer' two-step methods", Appl. Numer. Math. 53 (2005) 409-420 | 10.1016/j.apnum.2004.08.021 ; https://www.math.auckland.ac.nz/Research/Reports/Series/504.ps | **프리프린트 전문 13쪽** (텍스트층, 수식 일부 깨짐) |
| C4 | Schmitt, Weiner, Podhaisky, "Multi-implicit peer two-step W-methods for parallel time integration", BIT 45 (2005) 197-217 | 10.1007/s10543-005-2635-y | **미열람** (프리프린트 링크 404, 집계 사이트 초록 일부만) |
| C5 | Podhaisky, Schmitt, Weiner, "Design, analysis and testing of some parallel two-step W-methods for stiff systems", APNUM 42 (2002) 381-395 | (미확인) | **2차 인용만** (C2 Sec 2) |
| C6 | Gerisch, Lang, Podhaisky, Weiner, "High-order linearly implicit two-step peer - finite element methods for time-dependent PDEs", APNUM 59 (2009) 624-638 | 10.1016/j.apnum.2008.03.017 | **미열람** (Halle Report 2006/13 다운로드가 PDF가 아닌 HTML 반환) |
| C7 | Jebens, Knoth, Weiner, "Linearly implicit peer methods for the compressible Euler equations", APNUM 62 (2012) 1380-1392 | 10.1016/j.apnum.2012.06.013 | **미열람**. 요청문의 2011은 부정확, Crossref는 2012 |
| C8 | Kulikov, Weiner, "Global error estimation and control in linearly-implicit parallel two-step peer W-methods", JCAM 236 (2011) 1226-1239 | 10.1016/j.cam.2011.08.006 | **미열람** (403) |
| C9 | Jackiewicz, Podhaisky, Weiner, "Construction of highly stable two-step W-methods...", JCAM 167 (2004) 389-403 | 10.1016/S0377-0427(03)00928-2 | **미열람** (403). Peer 아님 |
| C10 | Lang, Schmitt, "Stiff Norm Estimates for Variable-Stepsize Peer Two-Step Methods", arXiv:2608.26913v1 (27 Aug 2026) | 10.48550/arXiv.2608.26913 | **전문 36쪽** |

요청문에 있던 "Beck-Weiner-Podhaisky-Schmitt Parallel peer W" 는 실제로는 "Implicit peer methods for large stiff ODE systems", J. Appl. Math. Comput. 38 (2012) 389-406 이며 열람하지 않음(Beck 발표 초록만 확인, 계수 없음). "Podhaisky-Weiner-Schmitt Rosenbrock-type peer"는 서지 일치(2005, APNUM 53). 요청문의 "Jebens-Knoth-Weiner 2011"은 2012가 맞음.

## 2. 후보별 상세 (C2, C3, C10 중심)

### C2 PPSW (Halle 프리프린트) — 유일한 stage-parallel Peer-W

- 형태 (3.1): `(I - gamma_i h_m T_m) Y_mi = sum_j (b_ij I + h_m d_ij T_m) Y_{m-1,j} + h_m sum_j a_ij f(t_{m-1,j}, Y_{m-1,j})`
- stage 병렬성 원문: "Obviously, with s processors all s values Y_mi can be computed in parallel." (3절, 식 3.1 직후)
- W 성질 원문: "The matrix T_m should be an approximation to the Jacobian f_y ... for stability reasons only. In fact, the accuracy of these methods is derived for arbitrary T_m in the sense of W-methods."
- 차수: PPSW1, PPSW2 모두 가변 스텝 p=s-1 (Thm 3.1, Thm 4.2 및 (4.5) 가정). Super2~4는 p=s이지만 s<=4. PPSW1은 상수 스텝에서만 p=s.
- 안정성: L(alpha). PPSW2-5 alpha=86도, PPSW2-6 alpha=81도. PPSW2는 stiffly accurate (Def 3.3, 기호가 텍스트층에서 소실되어 정확한 문구는 UNRESOLVED).
- 스텝비: Thm 4.2는 strongly stable이면 구간 [sigma_a, sigma_b] (sigma_a<1<sigma_b) 존재만 증명. **수치 구간은 인쇄되지 않음**. 실험에서 sigma_max=1.5 (PPSW2-3,4, Super2), **그 외(PPSW2-5, 2-6 포함) 1.2**. "For larger s the bounds can be rather close to one."
- zero-stability: 열거된 PPSW2 방법을 "strongly stable"이라 서술. B(1) 고유값은 인쇄되지 않아 재계산 필요 (UNRESOLVED). Fig 4.2의 bnorm 곡선(값 미열람).
- 시작값: 실험에서 ROWMAP 사용. Thm 3.1은 시작값 오차 O(h0^p) 요구.
- 오차 추정: (4.4) `Y_ms - sum_j theta_sj Y_{m-1,j}` (단일 superconsistent stage 사용). PPSW1은 O(h^{s-2}) embedded.
- 수치 실험의 실제 관측 차수(Fig 5.3~5.6)는 읽지 않음. global order >=4.8 달성 여부는 UNRESOLVED.

### C3 Rosenbrock-type peer — W 허용이지만 병렬 아님

- 원문: "an additional sequential coupling between the stages is allowed, i.e. g_ij != 0 for i > j" → **stage-parallel 아님**.
- W: T는 Jacobian 근사, 차수 조건은 T와 무관, 오래된 Jacobian/LU 재사용 시 차수 감소 없음 (4절).
- s=4..8, 가변 스텝 차수 s-1(s=6이면 5), 상수 스텝 차수 s. "unconditionally stable, i.e. stable for all stepsize patterns". 시험 sigma는 [0,1.5].
- Table 1 (단일 gamma): s=5 0.26036881402181763 (89.3도), s=6 0.19942255252118767 (85.4도), s=7 0.24256912841054176 (89.4도). 행렬은 식 (8), (17), (11)로 정의되며 식 텍스트가 깨져 옮기지 않음.
- 오차 추정: 차수 s-2 embedded. 시작: RADAU.

### C10 IP4o5 (Lang-Schmitt 2026) — 표는 읽히나 기준 불충족

- s=4, 차수 5, alpha=71.2도, sigma_bar=1.05 (sigma in {20/21,1,21/20}). 부록 A.3 (34쪽)에 K(sigma) 인쇄 (약 15~16자리).
- 식 (2): "A_n is a nonsingular lower triangular matrix" → 순차 대각 암시적. Peer-W 아님(Newton형). Jacobian 허용성은 UNRESOLVED.
- 두 텍스트 추출(layout/raw)에서 A.3의 소수 39개 완전 일치. 상수 항 일부는 JSON에 기록. 행렬 A0는 추출이 깨져 옮기지 않음.

### C1, C4~C9

C1 초록 원문: "s independent linear stage equations which may be solved in parallel. Convergence of order s-1 ... Promising methods up to order 7 are identified". 계수와 본문은 읽지 못함. 나머지(C4~C9)는 미열람이므로 모든 세부는 UNRESOLVED.

## 3. p>=5 후보의 계수 전사 (PPSW2-6, 프리프린트 11쪽, 식 (4.5) 직후)

인쇄 정밀도: c는 3~9자리, gamma0/gamma1은 소수 9자리, alpha는 정수 도. `gamma_i = gamma0_i / (1 + sigma*gamma1_i)` (i=1..s-1), `gamma_s`는 (4.3)으로 결정.

Pass A (렌더 이미지 육안 전사):
- c = (-1.702, -0.51, -0.195797762, 0.155193392, 0.755672784, 1)
- gamma0 = (0.214789562, 0.249182503, 0.237832565, 0.245204742, 0.353848357)
- gamma1 = (0.566334104, 0.496228487, 0.518584061, 0.498857638, 0.502412430)
- alpha = 81 도

Pass B (PS 텍스트층 숫자 자동 추출 + 300dpi 크롭 부호 재확인):
- c 절대값 = (1.702, 0.51, 0.195797762, 0.155193392, 0.755672784, 1), 부호 (-,-,-,+,+,+)
- gamma0, gamma1 = Pass A와 동일

**자릿수 단위 일치: 예 (c, gamma0, gamma1 모두).** 단서: 숫자는 텍스트층과 이미지로 독립 확인했으나 부호는 같은 렌더링의 이미지 판독 두 번뿐임.

참고 PPSW2-5 (p=4, 10쪽): c=(-1.09, -0.2290, 0.51, 0.63, 1), gamma0=(0.214789562, 0.249182503, 0.237832565, 0.245204742), gamma1=(0.566334104, 0.496228487, 0.518584061, 0.498857638), alpha=86도. 두 pass 일치.

주의: 노드가 음수이므로 Lemma 4.1(gamma_i>0 보장)은 적용되지 않음. gamma_s의 부호와 B(sigma) 조건수는 재계산 필요.

## 4. 후보별 판정

| 후보 | 판정 |
|---|---|
| C2 PPSW2-6 | SOURCE_FOUND_TABLE_NOT_READABLE (엄밀히는 "파라미터 읽음, 행렬 미인쇄"; 프리프린트만) |
| C1 SINUM 2004 | SOURCE_FOUND_TABLE_NOT_READABLE |
| C3 Rosenbrock-type peer | NO_P5_SOURCE_FOUND (p=5 존재하나 병렬 아님, 기준 제외) |
| C4 BIT 2005 | SOURCE_FOUND_TABLE_NOT_READABLE |
| C5, C6, C7, C8, C9 | SOURCE_FOUND_TABLE_NOT_READABLE (미열람) |
| C10 IP4o5 | NO_P5_SOURCE_FOUND (표는 읽었으나 병렬/Peer-W 아님) |

SOURCE_AVAILABLE_TABLE_READ 로 판정되는 후보는 **없음**.

## 5. UNRESOLVED 목록

1. PPSW2-6 저널본 값이 프리프린트와 같은지.
2. B, A, G, Theta 행렬은 인쇄되지 않음. 식 (3.9)(3.10)(4.3)(4.5)로의 유도는 미검증.
3. sigma_a, sigma_b 수치 구간과 B(1) 고유값 (strongly stable 실제 여부).
4. PPSW2-6의 실측 global order (그림 곡선 미열람). 4.8 목표는 측정 필요.
5. SINUM 2004, BIT 2005, APNUM 2009/2012, JCAM 2011 본문 전부.

## 6. 접근 제약

유료 출판사 미러 접근 시도는 권한 시스템이 차단했으며 우회하지 않았음. 열람은 공개 저자/기관 프리프린트와 arXiv로 한정.
