# PW01 controller 상수 복원 조사 (PTSW2C / PTSW2B)

**작성일:** 2026-11-11 · **모드:** LITERATURE + DOCUMENTATION ONLY (Rust/git 미실행, 기존 파일 미수정, 스크립트 미실행)
**증거 규칙:** 실행/열람한 것만 기술. 열람하지 못한 것은 UNRESOLVED. 실행한 PASS는 이 문서에 없음(해당 없음).
**작업트리:** `/media/cosmosapjw/5CAA5033AA500BC0/vigilode-unified-t02-20261110`

## 0. 결론 (먼저)

1. 소유자가 제공한 세 PDF(BIT 2005, ANM 2004, SIAM 2004)는 **PTSW2B/PTSW2C의 controller 상수(0.85, [0.5,2], hysteresis [0.95,1.05])를 인쇄하지 않는다.** 세 PDF를 `pdftotext -layout`으로 추출해 `hysteres|0.95|1.05|dead|safety` 를 검색했으나 controller 맥락의 hit 없음(hit는 모두 무관: 주소의 "Halle", 노드 c=(0.85,0.95,1), rktol).
2. 해당 상수의 유일한 출처는 repository certificate가 인용하는 **Podhaisky 2002 박사논문 (Parallele Zweischritt-W-Methoden, DOI 10.25673/3017)** 이다. 이 문서는 Halle 저장소(opendata.uni-halle.de)에 있으나 **Anubis bot-check에 막혀 이번 조사에서 열람하지 못했다(우회하지 않음).** 따라서 0.85/[0.5,2]/hysteresis는 현재 **SECONDARY_ONLY (certificate 전사; 원문 미확인)** 이다.
3. **PTSW2B certificate(§5)만 hysteresis를 전사**했고, **PTSW2C certificate(PEER_W §2.3)는 "additional history/hysteresis"라고만 쓰고 상수를 적지 않았다.** PTSW2B와 같은 규칙이 PTSW2C에도 적용되는지는 어떤 열람 가능 문서에도 없다. = D5 blocker의 정확한 원인.
4. Hysteresis는 **차수/안정성 정확성에 필요하지 않은 순수 heuristic**으로 판단한다(근거 §5). 따라서 PW01은 hysteresis 상수를 "OWNER_CHOICE, 비문헌"으로 명시 라벨하고 insensitivity 테스트(§5.3)를 붙이는 조건으로 진행 가능하다. 단 이는 소유자 결정 사항이며 이 문서가 결정하지 않는다.

## 1. 필요한 controller 상수 정의 (certificate/test 읽기 결과)

근거 파일: `solver_consolidation_20261108/PTSW2B_EXACT_TABLEAU_CERTIFICATE_20261109.md` §5 (line 226-266 부근), `PEER_W_TABLEAU_CERTIFICATES_20261109.md` §2.3 (line 110), `PEERW_FIRST_IMPLEMENTATION_PLAN_KO.md` line 81, `PW02_SCAN_AND_RED_TEST_KO.md` §4 (상수 이름: SAFETY, RATIO_CLIP, HYSTERESIS_BAND, HYSTERESIS_HISTORY, ACCEPTED_STEP_CEILING_VS_RODAS5P).

| # | 상수/정의 | PTSW2B (certificate 값) | PTSW2C (certificate 값) |
|---|---|---|---|
| C1 | safety factor | 0.85 | 0.85 (PEER_W §2.3 "safety factor 0.85") |
| C2 | ratio clip [σmin,σmax] | [0.5, 2] | [0.5, 2] |
| C3 | hysteresis band | [0.95, 1.05] keep h | **미전사** |
| C4 | ratio history 사용 | σ*>1.05 이고 최근 s개 ratio 중 >1 이 있으면 h 유지 (s=2) | **미전사** |
| C5 | error norm | RMS_i[(u_{m+1,i}-ũ_{m+1,i})/(ATOL+RTOL·|u_{m+1,i}|)] | 동일 ("normalized RMS") |
| C6 | tolerance scaling | ATOL+RTOL|u_{m+1,i}| (새 해 기준) | 동일 |
| C7 | estimator order exponent | σ*=0.85·Err^{-1/s}, s=2 | "E^{-1/s}" 로만 기술; PEER_PLAN line 81은 E^{-1/2} |
| C8 | accept/reject, reject factor | Err≤1 accept; reject 시 마지막 committed history에서 더 작은 h*로 반복 (별도 reject factor 없음, σ* 공식 그대로) | 동일(전사 수준) |
| C9 | initial step h0 | certificate에 없음 (startup은 h1=h0, σ1=1만 규정) | 없음 |
| C10 | accepted-step ceiling vs RODAS5P | 1.10 (B cert §6) 대 1.20 (PLAN §2) 불일치 D2 | 동일 불일치 |

참고: PTSW2B의 estimator (b_e=0.95 b, ε=0.1)는 "source eq.(4.7)"로 전사되었으나 이 식도 위 박사논문 안의 식이며 supplied PDF에는 없음.
**혼합 서술 주의:** PEER_W §2.3은 이 controller를 "source recipe"로 부르며 PTSW2C 맥락에서 0.85, [0.5,2]를 적었으므로 C1, C2, C5-C7은 PTSW2C에도 동일 출처(박사논문)로 보이나 **C3, C4는 PTSW2C에 대해 전사된 적이 없다.**

## 2. 세 PDF에서의 step-size control (Task 2)

| PDF | 위치 | 내용 (짧은 인용/요약) | PTSW2B/C에 대한 효력 |
|---|---|---|---|
| ANM 48 (2004) 425-439, Weiner-Schmitt-Podhaisky | §4, p.434 | "The new step-size is computed by standard rules, see, e.g., [6]" ([6]=Podhaisky-Schmitt-Weiner, ANM 42 (2002) 381-395). "we limit the stepsize increase to σmax = 1.5 for Super2, PPSW2-3,4 and to σmax = 1.2 for the other methods. No lower bound on σm is enforced." 오차추정: PPSW2는 Y_ms − Σθ_sj Y_{m−1,j} (식 3.7), PPSW1은 O(h^{s−2}) embedded. Krylov 정지 residual ≤ rktol·atol, rktol=0.1 (order 2,3), 0.001 (order 4,5). Fig.2 (p.434, bnorm)이 σmax 선택 근거. 시작값은 ROWMAP. | **PPSW(peer-variable) 방법용**. PTSW 상수 아님. σmax는 안정성 상수 norm(식 3.10) 도표 기반으로 선택됨 = 방법별 안정성 근거가 있는 상한. 하한 없음. |
| SIAM J. Numer. Anal. 42(1) (2004) 265-282, Schmitt-Weiner | §4, p.277 | "the new stepsize was computed in a standard manner; cf. [16]" ([16]=Podhaisky-Schmitt-Weiner ANM 2002). "The maximally allowed increase of the stepsize was set to σmax = 1.5 for all PPSW-methods." 추정: Method 4a는 superconsistent stage의 K_ms (order s 추정), s≥5는 order s−2 polynomial predictor와 비교. 시작값 Y1은 RODAS. | PPSW용. 안전계수/하한/hysteresis 인쇄 없음. |
| BIT 45 (2005) 197-217, Schmitt-Weiner-Podhaisky | §7, p.212 | "the stepsize control is performed in a standard way with absolute and relative tolerances atol and rtol." 추정: s≥4는 order s−2 predictor와 비교, 저차는 superconsistent stage (조건 2.12)의 Δ_ms=O(h^s) 사용. Krylov 정지 (7.1) residual ≤ rktol·atol (atol은 "global integration error"의 절대 허용오차), Table 7.1(p.214)에 σ̄=2,1.4 등 방법별 ratio 상한과 rktol. 거부 시 f(Y_{m−1}) 재사용 가능 (식 6.1, p.212). | multi-implicit peer용. 안전계수/하한/hysteresis 인쇄 없음. |

세 PDF 모두 PTSW(2002 ANM [6], [15], [17] 박사논문)의 controller는 **참조만** 하고 상수를 인쇄하지 않는다. 즉 PDF 내용만으로는 C1-C4, C7 어느 것도 확정할 수 없다. 공식 "σ*=0.85·Err^{-1/s}"는 PDF에 없다.

## 3. 외부 문헌 조사 (Task 3; 열람 상태는 §8)

| 출처 | 얻은 사실 (WebFetch 요약 결과; 열람 도구가 모델 요약이므로 중요한 수치는 원문 재확인 권장) | 판정 |
|---|---|---|
| Podhaisky 2002 박사논문 (opendata.uni-halle.de/.../9802/1/prom.pdf) | Anubis bot-check, 열람 불가 | BLOCKED, 우회 안 함 |
| Klinge 2020 박사논문 (opendata.uni-halle.de/.../41452/...) | Anubis bot-check | BLOCKED |
| Marburg EPPEER 코드 `ivpepp.f90` (Schmitt/Weiner, explicit parallel peer) | 오차 max-norm: max_i |rc|/(atol+rtol|y_i|) (코드 `rer`); `hn=exp(-log(1.D-8+rer)/order)`, `hn=phs*max(0.2D0,min(0.95D0*hn,sigx))`; sigx = 1.8 (EPP3f4), 1.6 (EPP4y3), 1.5 (EPP5f3/6j1/7f4), 1.4 (EPP8d/9f2); reject if rer>1; hmin=1e-10; 기본 atol=rtol=1e-5; **hysteresis/dead band 없음**; 초기 h는 Hairer-Wanner식 `inisteps` | explicit peer 코드 (implicit W 아님). 안전계수 0.95, 하한 0.2 → 문헌 연령 내에서도 값이 코드마다 다름 |
| Klinge-Weiner(-Podhaisky?) IMEX peer, arXiv:1902.01161 §2.5 식 (71),(75),(76) (ar5iv) | Δt_new = min(1.2, max(0.8, 0.9·err^{-1/s}))·Δt_n; err = max_i |est_i|/(atol+rtol·|w|) ; 안전 0.9, 지수 −1/s, clip [0.8,1.2], 종점 맞춤; atol=rtol 테스트 | **다른 peer 방법**이지만 같은 그룹. hysteresis 없음. clip이 PTSW2B의 [0.5,2]와 크게 다름 = 이 상수들은 방법/코드별 선택임을 보이는 증거 |
| Kulikov-Weiner JCAM 236 (2012) 1226-1239 (DOI 10.1016/j.cam.2011.08.006) | 검색은 초록/서지만 확인, 전문 미열람. 전역오차 추정이 목적(국소 controller 상수 확인 못함) | UNRESOLVED |
| Podhaisky-Schmitt-Weiner ANM 42 (2002) 381-395 (=ANM 2004 ref [6]; PTSW 설계논문) | 초록 수준만 확인 (step-size control 서술 여부 미확인) | UNRESOLVED (출판사 유료, 우회 안 함) |
| Weiner-Schmitt-Podhaisky "Rosenbrock-type peer" ANM 53 (2005), Jebens-Knoth-Weiner (2011), Beck et al. | 전문 미열람 | UNRESOLVED |
| Arxiv:2002.12028 (Rosenbrock-Wanner survey) | PDF 바이너리 압축으로 텍스트 미추출; controller 상수 확인 못함 | UNRESOLVED |
| EPPEER 매뉴얼 `man_epp.pdf` | 압축 PDF 미추출 (stepcon 스위치 존재 사실만 검색 요약에서 확인) | UNRESOLVED |

의의: 열람 가능한 peer 코드/논문 3종(EPPEER, Klinge arXiv, 본 PDF 세 편)에서 **hysteresis(dead band)를 쓴 것은 없다.** PTSW2B 박사논문의 [0.95,1.05] 유지 규칙은 이 그룹 안에서도 예외적이며(전사 기준), 이는 이 규칙이 방법 정확성의 일부가 아니라 구현 선택임을 간접 지지한다(증명 아님).

## 4. 상수별 판정표

범례: SOURCE_PRINTED(열람한 1차 문헌에 인쇄) / SECONDARY_ONLY(repo certificate 전사, 1차 원문 미확인) / DERIVED_FROM_THEORY / NOT_IN_LITERATURE → OWNER_CHOICE / UNRESOLVED.

| 상수 | 판정 | 출처/근거 | 비고 |
|---|---|---|---|
| C1 safety 0.85 (PTSW2B) | SECONDARY_ONLY | B cert §5, 인용 원문=Podhaisky 2002 (열람 불가). 대조: Klinge arXiv 0.9, EPPEER 0.95 (다른 방법) | 문헌 전반의 관행 범위 0.8-0.95 |
| C1 safety (PTSW2C) | SECONDARY_ONLY (PEER_W §2.3) | 같은 원문이라는 가정에 의존 | PTSW2C 전용 상수가 따로 있는지는 미확인 |
| C2 clip [0.5,2] | SECONDARY_ONLY | B cert §5; PEER_W §2.3 | 이론상 σ 구간은 계수의 유계성만 요구(B cert line 152). 안정성은 증명 안 됨 (UNRESOLVED) |
| C3 hysteresis band [0.95,1.05] | PTSW2B: SECONDARY_ONLY; PTSW2C: **UNRESOLVED** | B cert §5만 | §5: 정확성에 불필요, heuristic |
| C4 ratio history (s개) | PTSW2B: SECONDARY_ONLY; PTSW2C: **UNRESOLVED** | B cert §5 | "any of last s ratios >1" 의 s=스테이지수 2 (전사). 의미 모호성 §6 |
| C5 error norm (RMS) | SECONDARY_ONLY (cert) ; BIT/ANM/SIAM은 norm 미명시("standard") | 대조: EPPEER, Klinge = max-norm | 노름 선택은 OWNER_CHOICE에 가까움: 같은 estimator라도 RMS≤max |
| C6 tolerance scaling ATOL+RTOL·|u_{m+1,i}| | SECONDARY_ONLY | BIT p.212 "atol and rtol" 사용만 확인, 형태 미인쇄 | |
| C7 지수 1/s, s=2 | PTSW2B: SECONDARY_ONLY. 이론: DERIVED_FROM_THEORY = 1/(p_e+1), p_e=embedded 해의 차수 | PTSW2C는 p_e를 **계산하지 않았음 (NOT_RUN)**; 따라서 "s=2"가 p_e+1=2 에 해당하는지 미검증 | p=3인 PTSW2C에서 p_e=2이면 이론 지수는 1/3. UNRESOLVED |
| C8 reject 처리 | SECONDARY_ONLY | B cert §5: 마지막 committed history에서 새 h*로 재시도. σ_m=h_m/h_{m−1} 이 바뀌므로 A,Γ,v,v_e 재계산 (audit R4; BIT p.212, SIAM p.277: 거부 시 f(Y_{m−1}) 및 T_m 재사용 가능) | 별도 "reject factor" 상수는 문헌에 없음; 공식 σ*에 Err>1을 넣은 값 |
| C9 초기 h0 | NOT_IN_LITERATURE (열람 범위) | EPPEER는 Hairer-Wanner식 inisteps; ANM p.434/SIAM p.277은 시작값만 RODAS/ROWMAP | OWNER_CHOICE: 호출자 제공 h0 (프로젝트 기존 convention 따름) |
| C10 ceiling 1.10 vs 1.20 | UNRESOLVED (D2) | 임계값은 owner 승인 class | 이 조사 범위 밖 |

## 5. Hysteresis는 정확성에 필요한가? (Task 4 핵심)

### 5.1 판단
**Heuristic이다. 차수/안정성 조건의 일부가 아니다.**
- 차수 조건 C(2), Γ(2), B(3) (PTSW2C)은 σ에 대해 항등적(audit §3 (a), EXACT)이며 σ를 어떻게 고르는지에 의존하지 않는다. 수렴 정리(인용)는 σ-계수의 유계성과 시작오차 조건만 요구(B cert line 147, 152).
- 안정성: 정상 step 안정성 판정(A(81.1°), M(∞))은 σ=1 상수 step 조건이다. 변동 step 곱 안정성은 증명 안 된 상태(audit D, UNRESOLVED)이며 hysteresis도 이를 증명하지 않는다. 다만 hysteresis의 "keep h"는 σ=1을 더 자주 만들어 증명된 상수-step 영역에 가깝게 한다 = 안정성 **증거를 늘리는 것이 아님**, 이를 안정성 근거로 주장하면 안 된다.
- 목적: 계수 재계산 빈도, factorization 재사용(W_m=I−hγT_m은 h가 바뀌면 재구성), step 진동 억제 — 효율 heuristic. (이 해석은 문헌 인쇄가 아니라 구조적 추론)

### 5.2 주장 규칙
Accuracy claim은 tolerance가 아니라 **matched achieved global error**(fair-ab 규칙)로만 한다. hysteresis 상수는 achieved accuracy claim, 차수 claim, 안정성 claim의 근거가 될 수 없다.

### 5.3 insensitivity를 보일 테스트 (제안, NOT_RUN)
1. 고정 step 차수 테스트(controller 미사용): 관측 차수 p=3 (PTSW2C), 2 (PTSW2B). hysteresis와 무관함을 구조적으로 확인.
2. Controller 활성 sweep: 밴드 ∈ {없음, [0.95,1.05], [0.9,1.1]} × history ∈ {없음, s=2} × tol 격자(≥4 decade) × 문제(스칼라 선형/비선형, stiff 선형, Prothero-Robinson류, MOL 하나). 측정: 달성 전역오차(참값/고정밀 reference 대비), accepted/rejected steps, factorization 수, RHS 호출. 합격 기준은 **사전 등록 필요**(owner): 예) 같은 tol에서 달성 오차 비 범위, matched-error에서 work 비. 이 문서는 임계값을 정하지 않는다.
3. 기대(가설, 미확인): 달성 오차는 tol 비례 곡선상에서 band에 약하게 의존, work만 변동.
4. 거부 처리 테스트: Err>1 일 때 σ*<0.85 이므로 [0.95,1.05] 밴드 안에 절대 들어오지 않음(공식으로 자명), 재시도 시 A,Γ,v,v_e 재계산 확인.

## 6. 모호성/주의
- B cert 문장 "keeps h*=h_m when σ*>1.05 and any of the last s ratios was greater than one": 성장 억제 규칙이며 "그 외 σ*(≤1.05 또는 최근 성장 없음)면 h*=h_mσ*"로 읽힌다. 단 σ*∈(1,1.05]이며 [0.95,1.05] 밴드가 우선하는지, "last s ratios"가 acceptance 기준인지 시도 기준인지는 원문 없이 확정 불가 → 구현 전 소유자/원문 확인 필요 (UNRESOLVED).
- 안전계수 0.85, 지수 1/s로 Err≤1에서 σ*=0.85 Err^{-1/2}∈(0, 0.85] on reject; accept 시 Err가 매우 작으면 σ*→2 (clip).
- PTSW2C estimator식 (7)은 박사논문 eq (4.7)의 이관 구성과 일치(audit D4)할 뿐 원문 확인은 아님.

## 7. 추천 OWNER_CHOICE 기본값 (문헌 아님, 명시 라벨)

**전부 "OWNER_CHOICE / NON_LITERATURE_FOR_PTSW2C" 라벨을 코드 상수 문서화에 부착할 것.** 결정은 소유자.

| 상수 | 추천 | 라벨 | 이유 |
|---|---|---|---|
| safety | 0.85 | SECONDARY (B/C cert) | 두 certificate와 일치, 문헌 관행 0.8-0.95 범위 내 (Klinge 0.9, EPPEER 0.95: 다른 방법) |
| ratio clip | [0.5, 2] | SECONDARY | cert와 일치. 계수 유계 가정(B cert line 152)과 호환 |
| hysteresis band | **설정 가능 상수, 기본값 [0.95,1.05] "PTSW2B 규칙을 PTSW2C에 복사"** | **OWNER_CHOICE, PTSW2C용 비문헌** | 정확성에 영향 없는 heuristic; §5.3 테스트 후 확정. 대안: 기본 OFF(순수 I-controller) |
| ratio history | s=2 (PTSW2B 규칙 복사) | OWNER_CHOICE | 같은 이유 |
| error norm | RMS, ATOL+RTOL|u_{m+1}| | SECONDARY | cert |
| 지수 | 1/(p_e+1), p_e는 embedded 해 차수의 **계산 후** 확정. 계산 전 임시값 1/2 | DERIVED_FROM_THEORY(계산 필요) / 임시 SECONDARY | PTSW2C embedded 차수 미계산 |
| reject | σ* 공식 재사용, A,Γ,v,v_e 재계산, history 롤백 | SECONDARY + 구조적 | 별도 factor 없음 |
| h0 | 호출자 제공 (또는 프로젝트 기존 방식) | OWNER_CHOICE | 문헌 상수 없음 |

## 8. 방문 URL과 접근 상태
JSON `urls_visited` 참조. 요약: DOI 해석 OK(redirect), opendata.uni-halle.de PDF 2건 + repo.bibliothek.uni-halle.de 1건 = **Anubis bot-check로 BLOCKED(우회 시도 안 함; 한 번 curl로 PDF를 받으려다 HTML 차단 페이지를 받았고, 이후 UA를 바꾼 재시도는 하지 않았다)**. Marburg EPPEER 페이지/코드 OK, man_epp.pdf 압축 미추출, arXiv 2002.12028 압축 미추출, ar5iv 1902.01161 OK.
이 문서의 외부 사실은 WebFetch의 모델 요약에 의존하므로 수치(EPPEER sigx, Klinge 0.9/[0.8,1.2])는 원문 재확인 전까지 "보고된 값"이다.

## 9. UNRESOLVED 목록
1. Podhaisky 2002 박사논문 원문 (Chapter 4 step-size control)에서 C1-C4, C7 직접 확인 (소유자가 PDF를 제공하거나 Halle ULB 사이트에서 직접 내려받아야 함; 에이전트는 bot-check를 우회하지 않음).
2. PTSW2C에 PTSW2B와 같은 hysteresis가 적용되는지 (C3, C4).
3. PTSW2C embedded 해(b_e, v_e)의 차수 p_e 계산 → 지수 확정 (python3 -I 로 order residual 계산 가능, 미실행).
4. ANM 42 (2002) 381-395 (PTSW 설계논문)의 step-size control 서술; Kulikov-Weiner 2012, Jebens-Knoth-Weiner 2011, Rosenbrock-type peer 2005 원문.
5. EPPEER manual 및 정확한 Klinge arXiv 식 번호 원문 재확인.
6. "last s ratios" 의미 (시도 vs 승인 step).
7. 변동 step 곱 안정성 (audit 기존 UNRESOLVED), ceiling 1.10 vs 1.20 (D2).
