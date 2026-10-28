# Affine 축약 상태의 물리 공간 오차 제어

2026-10-23. 대상은 고정 affine 축약을 사용하는 비강성 ODE의 DOPRI5 경로다. 기존 DOPRI5의 5(4) 차수 조건과 FSAL 증명은 `research/nonstiff_explicit_20261023/THEORY_KO.md`의 검증된 결과를 재사용한다. 여기서는 새 오차 함수, 구현 비용, 적용 조건을 유도한다. 독립적인 전역 오차 추정기를 추가한 것으로 해석하지 않는다.

## 1. 해결할 문제와 계산 계약

물리 상태 \(v\in\mathbb R^m\)가 축약 상태 \(u\in\mathbb R^n\)로부터

\[
v=\phi(u)=d+Lu
\]

로 복원된다고 하자. \(L\)과 \(d\)는 한 번의 적분 동안 고정한다. 예를 들어 원소별 보존식을 이용해 일부 종의 밀도를 제거했다면, 제거된 성분의 오차는 남아 있는 여러 성분 오차의 합일 수 있다. 그러므로 축약 성분 각각의 크기만 검사하면 원래 물리 상태에서의 큰 오차를 놓친다.

한 시도의 시작점, 5차 끝점, embedded 차이를 \(u_0,u_1,e\)라 하자. 원래 물리 변수의 허용오차 \(a_j\ge0\), \(r\ge0\)에 대해 다음 값을 검사한다.

\[
v_0=d+Lu_0,\qquad v_1=d+Lu_1,\qquad \epsilon=Le,
\]
\[
s_j=a_j+r\max\{|(v_0)_j|,|(v_1)_j|\},\qquad
\mathcal E(u_0,u_1,e)=
\left[\frac1m\sum_{j=1}^m\left(\frac{\epsilon_j}{s_j}\right)^2\right]^{1/2}.
\tag{1}
\]

오프셋 \(d\)는 두 상태의 scale에는 들어가지만 오차 \(Le\)에는 들어가지 않는다. 분모의 평균 차원은 \(n\)이 아닌 \(m\)이다. \(s_j=0\)이면 \(\epsilon_j=0\)만 0을 기여하며, 그 외에는 무한대여서 시도를 거절한다. 모든 scale이 양수인 경우

\[
\mathcal E^2=\frac1m e^TL^TD^{-2}Le,\qquad D=\operatorname{diag}(s_1,\ldots,s_m).
\tag{2}
\]

이 행렬을 조립할 필요는 없다. 식 (1)을 직접 계산하면 상관항을 보존하면서 조밀한 \(n\times n\) Gram 행렬의 저장과 곱셈을 피한다. 이 계량은 \(L\)이 full column rank일 때 축약 오차에 대한 norm이고, 그 외에는 seminorm이다. 후자의 경우 물리 상태에 보이지 않는 nullspace 오차를 제한하지 않는다.

SUNDIALS의 WRMS 설명은 성분별 tolerance와 local-error 검사의 일반적인 의미를 뒷받침한다. 식 (1)의 두 끝점 최대값은 본 구현의 구체적인 계약이며 CVODE와 동일한 분모라고 주장하지 않는다. 문헌의 local-error 안내와 마찬가지로, 성공 종료가 전역 오차를 tolerance 아래로 보증하지는 않는다. [S3, S4]

## 2. 구성적 정확도 정리

**정리 1 — affine 관련 RHS의 명시적 Runge–Kutta 교환성.**
축약 RHS \(g(t,u)\), 물리 RHS \(f(t,v)\)가 해당 계산 영역에서

\[
f(t,d+Lu)=Lg(t,u)
\tag{3}
\]

를 만족한다고 하자. 같은 명시적 Runge–Kutta tableau, 같은 시작 시각과 step을 사용하면, 물리 공간의 각 stage와 끝점은 축약 계산 결과의 affine lift와 일치한다. Embedded 차이도 정확히 \(Le\)다. 이 명제는 exact arithmetic에서 성립한다.

*증명.* 첫 stage에서 식 (3)이 바로 성립한다. 앞선 stage의 물리 기울기가 \(LK_j\)라고 가정하면 다음 stage 상태는

\[
v_0+h\sum_{j<i}a_{ij}LK_j
=d+L\left(u_0+h\sum_{j<i}a_{ij}K_j\right).
\]

식 (3)을 다시 적용하면 해당 기울기도 \(LK_i\)다. 삼각 stage 구조에 따라 귀납하면 모든 stage가 일치한다. 끝점의 선형 결합에도 같은 논리를 적용한다. 두 endpoint formula를 빼면 \(d\)와 \(u_0\)가 상쇄되어 embedded 차이는 \(Le\)가 된다. 따라서 식 (1)은 원래 물리 상태를 직접 적분한 시도의 embedded WRMS와 같다. □

이는 affine-equivariant RK 이론의 이 구현에 필요한 직접적인 특수화다. 일반적 배경은 Verdier의 affine-related vector field 논의에서 확인할 수 있다. [S1] 비선형 lift나 시간에 따라 변하는 \(L,d\)에는 식 (3)과 위 귀납을 그대로 적용할 수 없다.

**정리 2 — 물리 오차 계량과 adaptive 판정의 좌표 불변성.**
가역인 \(S\)에 대해 \(q=Su+c\), \(L_q=LS^{-1}\), \(d_q=d-LS^{-1}c\)로 좌표를 바꾸자. 두 표현에서 같은 물리 tolerance를 쓰면

\[
\mathcal E_L(u_0,u_1,e)=
\mathcal E_{L_q}(Su_0+c,Su_1+c,Se).
\tag{4}
\]

*증명.* \(d_q+L_q(Su+c)=d+Lu\), \(L_qSe=Le\)이므로 식 (1)의 모든 분자와 분모가 동일하다. 따라서 평균과 수락·거절 결과도 같다. 같은 초기 step, 같은 결정적 controller, 같은 출력 정책까지 사용하면 시도된 step의 순서도 귀납적으로 같다. □

차원이 다른 중복 축약 표현이라도 시작/끝 물리 상태와 lift된 embedded 차이가 동일하면 같은 결론이 성립한다. 단, 물리 성분 자체를 임의로 복제하여 \(m\)과 평균 가중을 바꾸는 것은 동일한 계량이 아니다. 유한정밀도에서는 곱셈 순서와 conditioning 때문에 bitwise 동일성을 보장하지 않는다.

**정리 3 — 보존식으로 제거한 변수도 포함하는 오차 검출.**
\(n=30,m=33\)으로 두고 \(L\)의 앞 30행을 단위행렬, 마지막 세 행을 각각 연속된 10개 성분의 음의 합으로 두자. \(a_j=1,r=0,e_i=2/5\)이면

\[
\mathcal E_{\rm reduced}=2/5,\qquad
\mathcal E_{\rm physical}
=\sqrt{\frac{30(2/5)^2+3(10\cdot2/5)^2}{33}}
=2\sqrt{2/5}>1.
\tag{5}
\]

따라서 reduced WRMS가 수락하는 이 embedded 오차를 식 (1)은 거절한다. 이 예는 기존 norm이 원래 물리 기준과 동등하지 않다는 구체적인 조건부 반례다. 특정 실제 화학 궤적에서 반드시 같은 오차가 발생한다거나, 새 norm이 모든 문제에서 step 수를 줄인다는 주장은 아니다.

Wolfram Language에서 식 (5)의 정확한 값, \(L\)의 nonzero 개수 60, 별도의 2차원 좌표변환에서 식 (4)의 제곱 norm 차이 0과 scale 차이 \((0,0)\)을 확인했다. `AFFINE_SYMBOLIC_CHECK.json`에 코드, 첫 호출의 진단 메시지, 진단을 제거한 후속 호출의 원출력을 모두 보존했다.

## 3. 무엇이 전역적으로 보장되는가

**정리 4 — 조건부 전역 오차 상계.**
고정된 물리 norm에서 정확한 flow가 \(\|\Phi_h(x)-\Phi_h(y)\|\le e^{\lambda h}\|x-y\|\)를 만족한다고 하자. 수락된 \(k\)번째 수치 step의 해당 exact-flow에 대한 물리 defect가 \(C h_k^6+\rho_k\) 이하이고 \(0<h_k\le H\), \(\sum h_k=T\)라 하자. 그러면

\[
\|v_N-v(T)\|
\le e^{\lambda_+T}\left(
\|v_0-v(0)\|+CTH^5+\sum_{k=0}^{N-1}\rho_k\right),
\quad \lambda_+=\max(\lambda,0).
\tag{6}
\]

*증명.* 한 step 오차는 \(E_{k+1}\le e^{\lambda h_k}E_k+Ch_k^6+\rho_k\)다. 반복해서 펼친 뒤 모든 전달 인자를 \(e^{\lambda_+T}\)로 제한한다. \(\sum h_k^6\le H^5\sum h_k=TH^5\)를 쓰면 식 (6)을 얻는다. □

식 (6)은 smoothness와 stability로부터 local defect의 상계가 확보된 상황의 전역 5차 수렴 정리다. 변하는 scale의 embedded norm이 1 이하라는 사실만으로 이 defect 가정을 증명한 것은 아니다. Embedded difference의 leading coefficient가 소멸하거나, phase error가 장시간 누적되거나, floating-point 오차가 지배하면 별도의 분석이 필요하다. 물리 norm을 도입해도 이 구분은 유지된다. SciSpace에서 확인한 Calvo–Higham–Montijano의 연구는 adaptive RK의 전역 오차 추정이 별도의 계산과 가정을 필요로 하는 관련 문헌이며, 해당 방법은 이번 코드에 이식하지 않았다. [S6]

식 (1)의 수락으로 직접 얻는 성분별 결과는

\[
|(Le)_j|\le\sqrt m\,s_j
\tag{7}
\]

이다. WRMS 수락을 모든 성분에 대해 \(|(Le)_j|\le s_j\)라고 바꾸어 해석해서는 안 된다. 그 기준이 필요한 사용자는 max norm 또는 특정 관측량의 별도 허용오차를 구현해야 한다.

## 4. 계산 복잡도와 Rust 구현

**정리 5 — 동일한 물리 계량의 희소 계산 비용.**
\(k=\operatorname{nnz}(L)\)라 하자. 고정 lift를 CSR에 저장하면 식 (1)은 \(O(k+m+n)\) 연산, 성공 경로에서 추가 heap allocation 0회, \(O(1)\) 임시 scratch로 계산할 수 있다. 저장된 lift와 offset·tolerance의 공간은 \(O(k+m)\)다.

*증명.* 먼저 입력 세 벡터의 길이와 유한성을 \(O(n)\)에 검사한다. 각 nonzero에 대해 시작/끝 상태와 오차의 세 곱을 누적한다. 각 물리 행의 scale과 scaled sum-of-squares를 한 번씩 갱신한다. 행 순회는 \(O(k+m)\), 누적기 개수는 상수다. 모든 배열은 생성자에서 완성되므로 evaluate의 성공 경로에는 allocation이 없다. □

현재 생성자는 사용하기 쉬운 row-major 입력을 받고 이를 CSR로 압축한다. 따라서 최초 입력·전처리는 \(O(mn)\) 시간과 peak 공간을 쓴다. 생성자까지 sparse complexity라고 주장하지 않는다. 향후 매우 큰 lift에는 별도 sparse constructor가 유용하다.

식 (5)에서는 조밀한 lift 990개 중 60개만 저장한다. 한 번의 norm에서 lift 곱의 개수는 조밀한 세 matvec의 2,970개에서 180개로 줄어든다. 이 16.5배는 **곱셈 수의 비율**이다. 전체 solver 시간이나 norm wall time의 16.5배 가속은 아니며, 보상합·index 접근·오류 검사의 비용을 따로 측정해야 한다. 이미 올바른 sparse full-state 계산을 하는 구현보다 항상 빠르다는 보장도 없다.

| 수학 계약 | 구현 위치·정책 |
|---|---|
| 고정 affine lift, 물리 tolerance | `AffineErrorNorm::new`, scalar 또는 물리 차원 길이의 `atol` |
| \(d+Lu_0,d+Lu_1,Le\) | CSR 세 row product, 오차에는 offset 없음 |
| 취소 오차 완화 | Neumaier compensated sum |
| 제곱의 overflow 방지 | streaming scaled sum-of-squares |
| 잘못된 shape/NaN/Inf | typed `CoreError`, 수락 상태를 commit하지 않음 |
| zero scale | 기존 `scaled_error_component`와 동일한 규칙 |
| finite lift 또는 scale의 overflow | 명시적 거절; 임의정밀도 계산으로 우회하지 않음 |
| 런타임 계량 확인 | reduced/physical 차원, physical tolerances, nonzero 수 getter |

이는 시스템 Jacobian이나 Newton 행렬의 계산을 요구하지 않는다. 저장된 \(L\)은 문제 정의에 속하는 고정 좌표 복원 연산자다. Dense 출력의 관측점 오차를 별도로 제어하거나, event를 찾거나, time-varying remeshing을 수행하는 기능은 이 norm에서 제공하지 않는다.

## 5. 검증 항목과 연구 루프 판정

`affine_error_contracts.rs`는 30→33 correlated-error 판정, affine offset, 좌표변환, 중복 축약좌표, cancellation, 큰·작은 RMS, zero scale, malformed 입력, heap allocation을 확인한다. Driver 수준에서는 full norm이 실제 step 판정에 연결되는지, 실패 후 last accepted state가 유지되는지, 기존 norm 없는 경로가 유지되는지를 별도 확인해야 한다. 테스트의 실제 실행 상태는 상위 보고서의 고정 소스 receipt를 따른다.

이론 루프는 (1) 물리 오차 누락 문제 정의, (2) affine-related RHS 가정, (3) norm 유도, (4) 기존 RK 이론 재사용, (5) 교환성·불변성 증명, (6) exact counterexample 계산, (7) sparse complexity 증명, (8) floating-point failure 분석, (9) 코드 계약 전달, (10) 실행 결과와 범위 판정으로 이어진다. 코딩 루프는 생성자 검증 → sparse 표현 → compensated row 계산 → scaled RMS → driver hook → public telemetry → 정확도 회귀 → allocation 검사 → 독립 감사 → 재현 문서 순서로 진행한다.

실제 BASS/REI 모델 채택에는 client가 제공한 \(L,d\), 물리 단위, 제거된 성분의 순서, full-space tolerance를 저장하고 식 (3)을 모델별로 확인해야 한다. 또한 실제 full33/reduced30 궤적, 보존량과 관심 관측량을 비교해야 한다. 여기의 30→33 fixture만으로 해당 물리 모델의 production 자격까지 승인하지 않는다. Native stiff RODAS 경로에 같은 계량을 연결하는 작업도 별도 잔여 항목이다.

## 참고 자료

- [S1] Olivier Verdier, *Full affine equivariance and weak natural transformations in numerical analysis—the case of B-Series* (2016), <https://arxiv.org/html/1605.07344v1>. Abstract 및 affine-related field 관련 검색 추출 구절을 확인했다. 위 명제는 본문에서 직접 증명했다.
- [S2] McLachlan, Modin, Munthe-Kaas, Verdier, *B-series methods are exactly the affine equivariant methods*, DOI: <https://doi.org/10.1007/s00211-015-0753-2>. 서지·초록 수준의 배경 확인.
- [S3] SUNDIALS CVODE, *Mathematical Considerations*, <https://sundials.readthedocs.io/en/latest/cvode/Mathematics_link.html>.
- [S4] SUNDIALS, *Frequently Asked Questions*, tolerance selection, <https://sundials.readthedocs.io/en/latest/FAQ_link.html>.
- [S5] L. F. Shampine, *Conservation laws and the numerical solution of ODEs, II* (1999), DOI: <https://doi.org/10.1016/S0898-1221(99)00183-2>. SciSpace discovery abstract만 확인; 해당 projection 정리를 구현 근거로 채택하지 않았다.
- [S6] Calvo, Higham, Montijano, *Global error estimation with adaptive explicit Runge-Kutta methods* (1996), DOI: <https://doi.org/10.1093/imanum/16.1.47>. SciSpace discovery abstract만 확인.

원전의 확인 범위, 조회 도구와 query는 `AFFINE_SOURCES.json`, exact symbolic 계산은 `AFFINE_SYMBOLIC_CHECK.json`에 기록했다.
