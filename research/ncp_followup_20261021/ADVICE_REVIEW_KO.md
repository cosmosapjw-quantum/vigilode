# 첨부 설계 조언의 판정과 구현 가능한 교정

근거 기준은 게시된 NCP 원자료, 현재 코드와 아래 식이다. NCP bundle의 원래 source는 `de4c10a9`; 이번 변경의 base는 `7a63876a`. 과거 실행과 새 구현의 성능을 합치지 않는다.

| 조언 | 검토 결과 | 채택할 구현 계약 |
|---|---|---|
| 작은 dense-fast가 SP02보다 3–5배 빠르다 | 해당 NCP 캠페인은 native rodas5p-fast를 실행했으며 SP02 비교 근거가 없다 | 같은 문제·출력·실제 오차의 직접 비교를 새 campaign에서 실행 |
| N≈1600이면 GMRES가 반드시 유리 | dense LU 비용은 증가하나 crossover는 bandwidth, conditioning, iteration, 구현에 의존 | dense/band/bordered/matrix-free를 동일 정확도로 비교 |
| SP03 band LU는 O(N) | 고정 bandwidth에서만 성립 | 반폭 b에 대해 setup O(N b²), solve O(N b) 기록 |
| 인접 이류·각도 산란은 banded | 이류 stencil은 sparse, Thomson 각도 적분은 비국소 구조 | 이류·흡수 band/sparse + 산란 low-rank + matter border 분리 |
| RODAS5P 6개 stage | 현재 tableau와 fast driver는 8개 stage | `coefficients.stages()`를 SSOT로 사용 |
| remap 직후 자동으로 5차와 큰 h 유지 | history 재보간 부담은 줄지만 projection defect·정칙성·새 stiffness가 남음 | remap 오차 budget, mesh generation, cache invalidation, 새 h admission |
| minmod 대신 smooth limiter면 5차 회복 | regularization 폭·positivity·stiff order가 별도 조건 | smooth-region 시간 수렴과 limiter-active 정확도/양성성을 분리 검증 |
| slope weight를 step 동안 동결하면 안전 | 실제 RHS의 상태 의존성까지 동결하면 다른 ODE를 적분 | 실제 nonlinear RHS는 stage마다 재평가; Jacobian/preconditioner 동결과 구분 |
| Nmax workspace slice면 AMR 완료 | Krylov scratch 외 state/tolerance/output/recycle/geometry도 바뀜 | accepted-step transaction에서 모든 dimension/identity 갱신 |
| (E,theta) 격자로 일반 Bianchi | 축대칭 제한에서만 충분 | 첫 단계는 명시적 LRS Bianchi I; 일반형은 S² angular DOF 및 tetrad/screen 처리 |

## 1. 실제 비용 감소를 주는 블록 분해

선형 stage 행렬을 M=[[A,B],[C,D]]라 하고 A의 차원을 n, 물질 border를 m, band 반폭을 b라 하자. A와 S=D-C A^-1 B가 가역이면

    W=A^-1 B, u=A^-1 r_r,
    z=S^-1(r_m-Cu), x_r=u-Wz

는 Mx=r의 정확한 대수적 해다. 고정 b,m에서 setup과 application이 n에 선형인 경로가 된다. 실제 code는 row diagonal dominance를 검사하여 no-pivot band factorization을 허용하고 Schur에는 pivoting을 쓴다. 이는 새 `rodas5p_core::bordered` 구현이며 기존 private SP03를 그대로 공개한 것이 아니다. 비대각 우세 A는 명시적으로 거절하므로 sparse pivoted backend 또는 GMRES fallback을 호출자가 선택해야 한다. 실제 부동소수점 정확도는 full residual로 검증한다.

근사 전처리 P에 대해 ||I-M P^-1||_2≤q<1이면, 정확 산술의 right-preconditioned GMRES 잔차는 ||r_k||_2≤q^k||r_0||_2다. 증명은 최소잔차 다항식 중 p_k(z)=(1-z)^k를 선택하는 것이다. 0<q<1, 0<ε<1에서 목표 상대잔차 ε에는 k≥ceil(log ε/log q)가 충분하다. q=0이면 전처리된 행렬이 항등이므로 한 번의 비자명 iteration이면 충분하다. 따라서 n에 독립적인 q 상계가 확보될 때 차원에 독립적인 반복수 보장이 생긴다. '항상 3–5회'를 주장할 대신 이 q 조건과 실제 true residual을 기준으로 삼는다. P를 iteration마다 바꾸면 고정 다항식 증명을 그대로 적용할 수 없어 flexible Krylov 계약이 필요하다.

정확한 block triangular P=[[A,B],[0,S]]에서는 M P^-1=I+K, K²=0이므로 unrestarted GMRES가 최대 2 iteration에 끝난다. 이는 정확한 A,S inverse를 쓰는 대수 결과다. 이번 full Schur inverse는 해당 모델 행렬을 직접 풀 수 있으므로 항상 GMRES를 겹칠 이유는 없다. 모델 행렬과 실제 JVP가 다를 때 전처리기로 사용한다.

## 2. Frozen limiter의 정확한 구분

고정 Jacobian을 쓰는 Rosenbrock tableau의 order condition과, RHS F(y)=L(y)y를 F_n(y)=L(y_n)y로 바꾸는 것은 별개다. 간단한 y'=y², y(0)=y0에서 frozen coefficient 식의 정확해는 y0 exp(y0 h), 실제 해는 y0/(1-y0 h)다. 차이는

    -y0³ h²/2 - 5 y0⁴ h³/6 + O(h⁴).

Wolfram의 유리식/급수 검산은 `WOLFRAM_CHECK.json`에 있다. 이는 smooth coefficient에서도 발생하는 local consistency defect이므로 높은 order의 내부 solver만으로 제거되지 않는다. 이 사례는 특정 frozen-RHS 처방을 반박하는 기존 수준의 국소 예제이며 새로운 일반 no-go 정리로 주장하지 않는다.

실전 경로는 (a) reconstruction과 physical flux를 각 stage 상태에서 평가, (b) 선형화·전처리기는 step 내 고정, (c) branch/active-set가 바뀌면 Jacobian mismatch를 감시하고 필요시 h를 줄여 재시도다. smooth limiter의 regularization은 mesh에 따른 consistency를 검사한다. minmod의 active branch만 고정하더라도 branch 밖 stage에서 원래 flux와 달라질 수 있으므로 별도 defect를 측정해야 한다. positivity는 고차 implicit 시간적분의 자동 성질로 두지 않는다.

## 3. Remap 후 정확도와 transaction

exact-arithmetic overlap map의 positivity·질량보존·L1 contraction 증명과 구현은 `REMAP_THEORY.md`에 있다. 시간구간마다 local 시간오차 d_k, projection defect δ_k가 있고 안정 증폭계수 a_k를 상계할 수 있으면

    e_(k+1) ≤ a_k e_k + d_k + δ_k,
    e_K ≤ (product a_k)e_0 + sum_j (product_(k>j) a_k)(d_j+δ_j).

계속 5차 시간 수렴을 요구하는 fixed spatial problem에서 매 step remap한다면 충분조건의 하나는 δ_k=O(h_k^6)이고 누적 공간·부동소수점 오차도 따로 제어해야 한다. 유한 회수 remap 또는 다른 budget 배분은 다른 충분조건을 쓸 수 있다. piecewise-constant positive overlap만으로 이 조건을 충족했다고 보지 않는다.

accepted-step hook의 atomic commit 단위는 `(t,y,mesh,cell_measure,tolerances,mesh_generation,error_budget)`다. 후보 mesh의 양의 cell volume, remap 보존/양성성, domain boundary flux, projection budget을 모두 검사한 후 commit한다. 실패하면 수락 전파 결과의 old mesh를 유지하고 remesh만 거절한다. 성공 후 J/PC, recycler, old stage start, dense interpolation, event bracket의 dimension/geometry identity를 무효화한다. 이미 출력한 old interval은 old mesh identity와 함께 보존한다. 새 cell에서 error weight는 같은 물리 오차 norm으로 재구성한다. 재분할 전후 동일 수치 atol을 맹목적으로 복사하지 않는다.

`ConservativeRemapWorkspace`는 이 transaction에 사용할 kernel이다. 기존 adaptive driver에 완성된 accepted-step AMR가 연결되었다고 표시하지 않는다.

## 4. One-step/AMR에 대한 문헌 대응

Lang & Verwer (2001), *ROS3P—An Accurate Third-Order Rosenbrock Solver Designed for Parabolic Problems*, BIT41,731–738, [원문](https://ir.cwi.nl/pub/1102/1102D.pdf): one-step 방식의 adaptive mesh 이점을 설명하면서 semi-discrete PDE의 order reduction과 추가 조건도 다룬다. 이번 선택은 그 이점과 조건을 함께 적용한다.

Murphy, Golub & Wathen (2000), *A Note on Preconditioning for Indefinite Linear Systems*, DOI[10.1137/S1064827599355153](https://doi.org/10.1137/S1064827599355153): exact Schur 기반의 낮은 minimal polynomial을 재사용한다. 위 일반 D 블록식은 여기서 직접 전개했다. Southworth, Sivas & Rhebergen, [arXiv:1911.02664](https://arxiv.org/abs/1911.02664)는 한 diagonal block을 정확하게 역산하는 조건에서 Schur 수렴과 block Krylov 수렴의 관계를 분석한다. 해당 조건을 근사 inverse 전체에 무조건 확대하지 않는다.

조회일 2026-10-21. 검색은 Exa/SciSpace로 수행했고 핵심 원문은 원저자 저장소/원논문으로 확인했다. 이 문서의 계산량·잔차·remap 식은 독립 검토와 구현 검증 파일로 연결된다.
