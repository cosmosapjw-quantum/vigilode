# PW04 대안 탐색: 공통 shift(common-shift) W-peer, 차수 >= 5

**작성:** 2026-11-12 · **모드:** LITERATURE + EXACT/NUMERIC CHECK ONLY (Rust 미실행, git 미실행, 기존 파일 미수정, PDF 미복사, 유료/우회 접근 없음)
**판정:** COMMON_W_ORDER5_CANDIDATE_FOUND — 후보 PPSW1-6b, PPSW1-6c (Schmitt & Weiner, SIAM J. Numer. Anal. 42 (2004) 265-282). 단, 아래 "전제 정정"과 "조건"을 반드시 읽을 것.

## 0. 소유자 해석 (보고 명시)

소유자 요청 "W-peer (loop-free)"를 다음 T1-T6으로 정의하여 평가한다.

| 항목 | 정의 |
|---|---|
| T1 | 선형 내재(linearly implicit) W형 방법. 한 스텝 내부에 Newton/고정점 반복 루프 없음 |
| T2 | 같은 스텝의 stage들이 서로 독립(peer 구조, 병렬 stage) |
| T3 | 공통 shift: 모든 stage가 같은 W = I - h*gamma*T 사용 (스텝당 factorization 1회, 또는 s에 무관한 상수 회) |
| T4 | 가변 스텝 차수 >= 5 (local 또는 global, 어느 것인지 명시) |
| T5 | zero-stability 및 A/L-stability 보고 |
| T6 | 인쇄된 tableau 또는 정확히 재현 가능한 tableau |

## 1. 전제 정정 (중요)

- 사용자 맥락의 "PPSW2-6 (APNUM 2004)는 공통 shift"는 **문헌과 다르다.** APNUM 48 (2004) 425-439의 서론(p.426 부근, 쪽수는 텍스트 기준 근사치로 UNRESOLVED 표시)은 "singly-implicit PPSW-methods [SINUM]"과 "multi-implicit PPSW-methods (Sec.3)"를 구분하며, 본 문서는 **PPSW1 = singly-implicit (SINUM 2004), PPSW2 = multi-implicit (gamma_i 상이)**로 본다. 따라서 PPSW2-6는 6개 factorization이 맞다.
- **공통 shift 차수 5 후보는 PPSW2가 아니라 PPSW1-6b/6c (SINUM 2004)이다.** 기존 repo 문서(PW03B §5-6)는 SINUM PPSW1의 A 구조를 "재구성 UNRESOLVED"로 두었고, 그때 시도한 A = gamma*Theta는 무효였다 (alpha = 0이 인쇄값 57.5/87.9와 불일치). 본 탐색에서 SINUM 식 (23)을 사용한 A = beta + gamma*Theta로 재구성하였고, 인쇄된 alpha 값과 일치함을 확인했다 (§4). 이것으로 해당 UNRESOLVED는 **재구성 측면에서 해소**되었으나, 저자 원본 tableau가 인쇄되지 않아 "동일성"은 간접 증거에 의존한다.

## 2. 문헌 점검 (Task 1) — 소유자 PDF (로컬, pdftotext -layout, 복사 없음)

sha256 (앞 16자): BIT 2005 `e9f8ef8ca7a788df` (21 pp.), APNUM 2004 `7e8d17deeecf896b` (15 pp.), SINUM 2004 `da9cd10e45c53907` (18 pp.), Podhaisky 2002 thesis `4a3cfa7f03a3f004` (76 pp.).

| 방법 (출처) | s | 차수 (local/global, 가변/상수) | shift 구조 | 인쇄 tableau | 쪽/식 | T1-T3 | T4 | 비고 |
|---|---|---|---|---|---|---|---|---|
| **PPSW1-6b, PPSW1-6c** (SINUM 2004) | 6 | s-1 = 5 (가변 스텝, Thm 3.3); 상수 스텝 초수렴 global s (Table 1 주석) | **단일 gamma** (singly-implicit) | c, gamma-hat 인쇄; A,B,Gamma 미인쇄(식으로 재구성) | Table 1 p.276, Table 2 p.277; Thm 3.1 p.271; Thm 3.3 p.272; (14)-(16) p.269-270; (20)-(24) p.270-272 | T1 yes, T2 yes, T3 yes | yes (가변, 5) | alpha: 6b 57.5 deg, 6c 87.9 deg (Table 1) |
| PPSW1-7b/c, 8c (SINUM) | 7,8 | s-1 | 단일 gamma | c, gamma-hat 인쇄 | Table 1-2 | yes | s-1 | 6-stage 이상 참고 |
| PPSW1-4a/4b, 5b (SINUM) | 4,5 | s-1 | 단일 gamma | 인쇄 | Table 1-2 | yes | s-1 | 4a는 superconsistent stage 포함 |
| PPSW2-s (APNUM 2004), mipeer s (BIT 2005) | s | s-1 | **다중 gamma_i** | 일부 | BIT Table 7.1 (s=3,4,5만) | yes/yes/**no (s factorizations)** | yes | T3 위반 (6개) |
| PTSW2B (thesis Podhaisky 2002; = SINUM PPSW1-2 같은 gamma) | 2 | order 2 (B(2)) | 단일 gamma | 인쇄 (Tab 4.1, 3 node 값 인쇄 오류 플래그) | thesis Tab 4.1 p.36 | yes | no (2) | 중복 아님: thesis PTSW는 v항 포함 2-step 형태 |
| PTSW2C (thesis) | 2 | order 3 (B(3), v=0, A(81.1)) | 단일 gamma | 인쇄 | thesis Tab 4.1 p.36, Sec 3.3.6 | yes | no (3) | 소유자 맥락과 일치 |
| PTSW3C (thesis Sec 3.3.7, p.28) | 3 | order 4 (B(4), v^T=0, A(alpha)) | 단일 gamma | 계수식 (c3 = (3-4c1+6c1c2-4c2)/(2(6c1c2-3c1+2-3c2))), 수치 최적화는 Abb. 3.10-3.11 | p.28 | yes | no (4) | thesis 자체 결론: s=3까지 |
| 4-stage A(alpha), 5 free params, "Ordnung fuenf numerisch bestaetigt" (thesis Sec 3.3.8, p.29) | 4 | order 5 **numerisch 주장만**; B(5) 강제하지 않음 | 단일 gamma | gamma=0.7535833817, c=(-3.3397654896, -1.4870830356, 0.5362026478, 3.5569894699) 인쇄; rho=0.9262701231, alpha=79.2 | p.29 | yes | 미증명 | 저자 스스로 "sehr empfindlich", 이후 미사용 -> UNRESOLVED |
| PTSW4B/4C (thesis, Satz 3.1 구성) | 4 | order 4 | 단일 gamma | 인쇄 | Tab 4.1 | yes | no (4) | |

**Task 1 결론:** 인쇄된 tableau를 가진 단일-gamma 방법 중 **order >= 5를 증명된 형태로 갖는 것은 SINUM PPSW1-6b/6c 뿐이다** (thesis의 order 5 4-stage 방법은 B(5) 미강제, 수치 확인만). thesis는 s >= 4에서 "s=8, p=4" 같은 완화를 제안하고 (thesis p.59), s>3에서 stage order를 포기해야 한다고 결론한다 (thesis p.59, "um effiziente PTSW-Verfahren mit s > 3 zu konstruieren ... auf die hohe Stufenordnung s verzichten").

## 3. 이론 점검 (Task 2) — 공통 shift 장벽

- **문헌 장벽:** SINUM p.267 부근 (Sec.1): "For the singly implicit methods in the form (7) it is unlikely that this can be achieved by improving all local errors by one order." 이는 **정리가 아닌 서술**이며, order s (global) 이상에 대한 증명된 상한은 찾지 못했다 -> **UNRESOLVED (장벽 증명 없음).**
- **Order conditions (SINUM (14),(15), p.269-270):** Gamma(q): gamma c_i^k + sum_j gamma_ij (c_j-1)^k sigma^{-k} = 0 (k=0..q-1); AB(q): c_i^k - sum_j b_ij ((c_j-1)/sigma)^k - k sum_j a_ij ((c_j-1)/sigma)^{k-1} = 0 (k=0..q-1). Lemma 2.3 (p.270): Gamma(q) and AB(q+1) => local order q.
- **Parameter counting (s=6, order 5, 공통 gamma):** 미지수 (A, B, Gamma: 3 s^2 = 108) + c (s=6) + gamma (1). 조건: Gamma(5) (5 s eqs, 학습 식 기준 s*q)과 AB(6) (6 s eqs) 등. 그러나 식 (16),(20),(23)이 Gamma(s), AB(s)를 정확히 해로 푸므로 **구조적으로 잉여가 없고**, 자유도는 c(6) + gamma(1)에 있다. 이 계산은 정성적이며 장벽 증명이 아니다.
- **feasibility (NUMERICALLY_CHECKED):** 아래 §4의 구성이 Gamma(6), AB(6)를 mp 60자리에서 잔차 ~1e-58로 만족 (sigma = 0.8, 1, 1.2, 1.5). 따라서 **s=6, 공통 gamma, 가변 스텝 order 5는 실현 가능함을 수치로 확인**. 장벽 주장은 하지 않는다.
- AB(7) (global superconvergence 조건)은 sigma=1에서 0이 아니다 (잔차 36.1, 85.8). 이는 SINUM Remark (p.273-274)가 말하듯 superconvergence가 local이 아니라 **global error constant의 소거**(ṽ_{s+1}(gamma)=0)임과 일치한다. 해당 global order 6 주장은 **재실행하지 않음 (NOT_RUN)**.

## 4. 재구성 및 수치 검증 (정확 mp 60 digits)

재구성 (SINUM 식 (16), (20), (22)-(23); c_s = 1, G = gamma I; scheme (5)):
- V_ij = c_i^{j-1}, P_ij = C(j-1,i-1), S = diag(sigma^{k-1}), D = diag(k), F0 = (delta_{i-1,j}).
- Theta = V S P V^{-1} (extrapolation, eq (16)); Gamma = -gamma Theta.
- beta = V F0 S D^{-1} V^{-1} (eq (23): beta~ = F0 S D^{-1}); A = beta - Gamma.
- B = Theta - sigma A V D F0^T V^{-1} (eq (20)).
- 스킴: (I - gamma h T) Y_m = (B (x) I + h Gamma (x) T) Y_{m-1} + h (A (x) I) f(Y_{m-1}).

| 점검 | 결과 | 상태 |
|---|---|---|
| s=2, c=(3-2sqrt3, 1), gamma=(3-sqrt3)/2: Gamma(2), AB(2) 잔차 | ~1e-61 | NUMERICALLY_CHECKED |
| s=2: AB(3) 잔차 (superconvergence 조건 아님) | 1.05 (기대대로 0 아님, 전역 상수 조건) | NUMERICALLY_CHECKED |
| s=6, c=(-1,-0.5,0,0.4,0.8,1) (Table 2 6b,c): v_7(gamma) 근 | 0.61941197506625 (인쇄 0.619411), 1.08708024064 (인쇄 1.087080) | NUMERICALLY_CHECKED (6자리 일치) |
| 6b/6c, sigma=0.8, 1, 1.2, 1.5: Gamma(6), AB(6) 잔차 | <= 3e-58 | NUMERICALLY_CHECKED |
| beta^6 (nilpotent) | ~1e-63 | NUMERICALLY_CHECKED |
| B의 변환 기저 V^{-1} B V: 1열 = e1, 대각 = (1,0,..0), 하삼각 ~1e-58 | 확인 (식 (24) 구조) | NUMERICALLY_CHECKED |
| 5개 B_m 곱 (sigma in [0.5,1.5] 난수, mp 정밀도): s2/s1 | 2.7e-58 (rank one, Lemma 3.2) | NUMERICALLY_CHECKED |
| 같은 곱, double precision (cond(V) 증폭): s2/s1 | 3e-3 — **부정확한 반올림 아티팩트**, mp 결과로 대체 | 기록 |
| A(alpha) 스캔 (double, 0.1도 격자, r in [1e-3,1e6]): 6b | **57.5 deg** (인쇄 57.5) | NUMERICALLY_CHECKED |
| 6c | **87.9 deg** (인쇄 87.9) | NUMERICALLY_CHECKED |
| rho(M(-1e8)) (M(z) = (I-gamma z)^{-1}(B+z beta)): 6b / 6c | 3.9e-2 / 2.6e-2 (-> 0, L-stable at infinity 방향) | NUMERICALLY_CHECKED |
| 전역 수렴 차수 (실제 적분 실험) | **NOT_RUN** | — |
| 시작 단계 (accurate starting, O(h^5)) 구현 | **NOT_RUN** (요구 조건만 확인) | — |

재구성 식의 정확성 근거: (a) 인쇄된 조건 AB(6), Gamma(6)를 정확히 만족, (b) 인쇄된 alpha 57.5/87.9 재현, (c) 인쇄된 gamma-hat 근 재현. 그러나 저자가 실제로 쓴 tableau 자체는 인쇄되지 않았으므로 **"동일 tableau"는 간접 증거**이다 (UNRESOLVED-U1).

## 5. 후보 평가: T1-T6 표

| 항목 | PPSW1-6b (gamma=0.619411975066) | PPSW1-6c (gamma=1.08708024064) |
|---|---|---|
| T1 선형 내재 | yes (stage 방정식 선형, 루프 없음) | yes |
| T2 stage 독립 | yes (모든 stage는 Y_{m-1}만 사용) | yes |
| T3 공통 shift | yes: 스텝당 W = I - h gamma T 1개 factorization (직접법); 거부 시 재사용 가능 | yes |
| T4 차수 >= 5 | **order 5 (s-1), 가변 스텝 (local 조건 Gamma(5)/AB(6) 수치 확인; 전역 차수는 논문 Thm 3.3 주장, 본 탐색에서 미실행)**. 상수 스텝 superconvergence global 6 주장은 미검증 | 동일 (order 5 가변; global 6 at sigma=1 주장) |
| T5 안정성 | zero-stable: B-e1 1^T nilpotent (논문 Thm 3.1), rank-one 곱 확인. L(alpha): beta nilpotent -> L-stable 주장; A(alpha) alpha=57.5 deg (6b) | zero-stable 동일. **L(alpha), alpha = 87.9 deg (A-stable 아님; 엄밀한 A-stability 아님)** |
| T6 tableau | c 인쇄 (Table 2), gamma 인쇄 (Table 1, 6자리). A,B,Gamma,beta 미인쇄, 식 (16),(20),(23)로 **정확 재구성 가능** (§4) | 동일 |

Factorization: 1 (T3 충족). Krylov 구현 시 stage당 선형 해 6회 (스텝당), 각 해는 내부 반복 필요 -> 루프-프리 여부는 "비선형 루프 없음" 기준으로 충족, "내부 Krylov 반복 없음"은 **미충족** (direct LU 구현에서만 내부 반복 없음). thesis PTSW2B/3B (Krylov)와 동일 성격.

**전제 조건:** (a) gamma 고정 (sigma-무관), (b) sigma <= sigma_bar (논문 실험 sigma_max = 1.5), (c) 시작 값 O(h^{s-1}) 정확 (Thm 3.3 조건: 시작에 Y_{0,i} 오차 O(h_0^{5})), (d) T는 J의 근사 (안정성 위해), (e) 스텝당 sigma-의존 계수 재계산 O(s^3) (논문 p.273).

## 6. Task 3 — 웹 탐색 (공개 접근만)

| 항목 | 내용 | 접근 |
|---|---|---|
| 검색 1 | "parallel peer W-methods linearly implicit common Jacobian order 5 Weiner Schmitt Podhaisky" | 검색 결과 (스니펫) |
| Marburg peer 페이지 https://www.mathematik.uni-marburg.de/~schmitt/peer | 2004-2023 목록 (약 80편); 대부분 order 명시 없음; EPPEER 명시 방법 order 3..9 (explicit) | 열람 성공 (공개) |
| Beck, Weiner, Podhaisky, Schmitt, J. Appl. Math. Comput. 38 (2012) 389-406, "Implicit peer methods for large stiff ODE systems" | 검색 스니펫: singly-implicit p=s-1 (arbitrary steps, s<=5), multi-implicit p=s (본문 주장과 차이 있음) | **본문 미열람**: Springer 리다이렉트/로그인 게이트 (link.springer.com, mirror 모두) -> UNRESOLVED |
| Montijano, Podhaisky, Randez, Calvo, BIT 59 (2019), "L-stable singly implicit peer methods" (검색 스니펫) | 검색 스니펫에만 존재; 정확한 DOI 추정 404 | **미열람** -> UNRESOLVED (order 미확인) |
| arXiv 2608.26913 "Stiff norm estimates for variable-stepsize peer two-step methods" | 식별자가 조회 시점 범위 밖, 내용 미열람 | 미열람 |
| Klinge (2021, Halle 학위논문 opendata.uni-halle.de) | 검색 결과에만 존재 | 미열람 (PDF 본문 미확인) |
| Weiner, Kulikov, Beck, Bruder (2017, JCAM 316) "third- and fourth-order" | 제목 기준 | 제목만 |
| Gerisch-Lang-Podhaisky-Weiner (2009), Jebens-Knoth-Weiner (2011, 2012), Kulikov-Weiner (2011), Schmitt-Weiner 2016 등 | 목록에 존재 | 본문 미열람; order 불명 |
| Schmitt-Weiner 2004, Weiner-Schmitt-Podhaisky 2004 | 소유자 PDF로 확인 (§2) | 로컬 |

**Task 3 결론:** 2004 이후 공개 접근 가능한 문헌에서 **공통 shift + 차수 >= 5 + 인쇄 tableau**를 갖는 방법은 확인하지 못했다. 2012 Beck 등과 2019 Montijano 등은 **본문 미열람**이므로 "없다"고 단정하지 않는다 (UNRESOLVED-U2, U3).

**URLs 접근 상태 (요약):**
- https://www.mathematik.uni-marburg.de/~schmitt/peer — 열람 성공
- https://link.springer.com/article/10.1007/s12190-011-0485-0 — 로그인 리다이렉트, 미열람
- https://springerlink.bibliotecabuap.elogim.com/doi/10.1007/s12190-011-0485-0 — 로그인 게이트 (기관 mirror), 미열람 (우회하지 않음)
- https://doi.org/10.1007/s10543-019-00751-5 — 404 (추정 DOI 오류, 미열람)

## 7. Task 4 — loop-free 판단

- 비선형 루프: T1 후보 전부 없음.
- 내부 선형 해: PPSW1-6b/c, 스텝당 stage 6개 선형 해 (동일 W). 직접 LU: 내부 반복 0. Krylov: stage당 내부 반복 (6회 독립 Krylov). thesis PTSW2B/3B (Krylov로 실험, p.59) 동일 성격.
- PPSW2-6 / mipeer6: W_i 6개 -> 스텝당 6 factorization (직접) 또는 6 Krylov 시스템 (T3 위반).

## 8. 판정 (Task 5)

**COMMON_W_ORDER5_CANDIDATE_FOUND.**
- 후보: **PPSW1-6b** (gamma = 0.619411975066...) 및 **PPSW1-6c** (gamma = 1.087080240637...), knots c = (-1, -0.5, 0, 0.4, 0.8, 1), 출처 SINUM 2004 Table 1-2.
- T1 yes, T2 yes, T3 yes (1 factorization), T4 order 5 가변 스텝 (local 수치 확인, global 실행 NOT_RUN), T5 zero-stable, L(alpha) (6b 57.5 deg, 6c 87.9 deg; A-stable 아님), T6 c/gamma 인쇄 + A,B,Gamma,beta 식 재구성 (간접 동일성).
- 전제: §5 전제 조건 (a)-(e). 특히 시작 값 O(h^5) 정확성과 sigma <= 1.5 범위.
- mipeer6 / PPSW2-6 폴백은 **적용하지 않는다** (공통 shift 후보 존재). 단, 전역 차수 5의 실측 확인 전까지 "order 5 달성"은 NOT_RUN 상태다.

## 9. 발견된 불일치 및 정정

- PW03B 문서의 "SINUM PPSW1 A 재구성 무효/UNRESOLVED" 항목은 A = gamma*Theta 시도에 기인. 올바른 식은 A = beta + gamma*Theta, beta = V F0 S D^{-1} V^{-1} (SINUM (23)). 해당 repo 항목의 갱신을 권고 (본 탐색은 repo 파일을 수정하지 않음).
- thesis Sec 3.3.8의 4-stage order 5 방법은 B(5)를 강제하지 않았으므로 order 5 증명 없음.

## 10. 영수증 (receipts)

스크립트 위치 (반입하지 않음, 임시 디렉터리):
- `/tmp/claude-1000/-home-cosmosapjw-vigilode/d3cab3fe-917d-4d13-8122-642eb1b07ae5/scratchpad/code/ppsw1_check.py` sha256 `de3735865934b8c2568b...` (exit 0)
- `.../code/ppsw1_stab.py` sha256 `534371a9dd941bc2bdbc...` (exit 0)
- `.../code/ppsw1_rank.py` sha256 `25ea39dcfcb700989d18...` (exit 0)
- stdout: `out_check.txt` `bc5d75869a835f67cea1...`, `out_stab.txt` `de674d1b2a15af19fb8b...`, `out_rank.txt` `a7d76f4b9ccaf70c6d45...`; stderr 세 개 모두 0 byte (`e3b0c44298fc1c149afb...`).
- 실행: `python3 -I <script>` (numpy 1.26.4, mpmath 1.2.1, sympy 1.12, scipy 1.11.4 가용).
- PDF sha256 (앞 16자): BIT `e9f8ef8ca7a788df`, APNUM `7e8d17deeecf896b`, SINUM `da9cd10e45c53907`, thesis `4a3cfa7f03a3f004`.

**핵심 코드 (build, mpmath):**
```
Theta = V*S*P*inv(V);  Gam = -gamma*Theta
beta  = V*(F0*S*inv(D))*inv(V)            # SINUM (23)
A     = beta - Gam
B     = Theta - sigma*A*V*D*F0T*inv(V)    # SINUM (20)
```
잔차 점검: Gamma(q) = gamma c_i^k + sum_j Gam_ij (c_j-1)^k sigma^{-k}; AB(q) = c_i^k - sum_j B_ij ((c_j-1)/sigma)^k - k sum_j A_ij ((c_j-1)/sigma)^{k-1}.

## 11. UNRESOLVED

- U1: 저자의 실제 A, B, Gamma 수치 tableau 미인쇄. 재구성의 동일성은 간접 증거 (order 조건, alpha, gamma-hat 재현)에 의존.
- U2: Beck, Weiner, Podhaisky, Schmitt 2012 본문 미열람 (공개 접근 차단).
- U3: Montijano, Podhaisky, Randez, Calvo 2019 BIT 본문 미열람 (정확 DOI 미확인).
- U4: 전역 수렴 차수 5 (PPSW1-6b/c) 실측 NOT_RUN; 시작 단계 구현 NOT_RUN.
- U5: 공통 shift order s 이상 장벽: 증명 없음 (SINUM 서술 "unlikely"만).
- U6: thesis 4-stage order-5 방법(p.29)의 B(5) 충족 여부 미확인 (논문도 미확인).
- U7: double precision 곱 rank-one 점검 (3e-3)과 mp 결과 (2.7e-58)의 차이: mp 결과를 채택, double은 cond(V) 증폭 아티팩트로 판단 (NUMERICALLY_CHECKED 간접).
