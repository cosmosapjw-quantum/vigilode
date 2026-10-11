# VigilODE: 정확도와 계산량 승격을 위한 구성적 이론 연구

2026-10-11 · 기준 코드 `8ce9bda0d72d308facd615ac42ec137560fd68c6`

이번 연구는 **조건을 확인하면 정확도와 계산량 개선이 실제로 따라오는 정리**를 도출했다. 핵심은 더 높은 수학의 이름을 기존 알고리즘에 붙이는 데 있지 않다. 연산자의 대수적 관계로 실행해야 하는 recurrence 길이를 줄이고, stage의 인과구조로 병렬 계산의 오차를 감싸며, 최종 오차에 대한 영향도에 따라 선형계의 정확도를 배분하는 데 있다. 전체 증명은 아래 네 문서에 있다.

- [다항식·몫대수·finite jet 정리](POLYNOMIAL_KO.md)
- [stage에서 전역 오차까지의 정리와 최적 잔차 배분](GLOBAL_ACCURACY_KO.md)
- [homotopy 병렬화와 정확한 불변부분공간 축소](HOMOTOPY_KO.md)
- [매개변수 셀 인증과 구조 컴파일](CERTIFICATE_ATLAS_KO.md)
- [전체 공개 연구의 통합 지도](SOURCES_SYNTHESIS_KO.md)

최종 scope별 판정은 `research/promotion_theory_20261011/DECISION.json`에 기록한다. **조건부 수학 정리의 승격, 그 조건을 강제하는 native 구현의 승격, 특정 궤적의 전역 정확도 인증, 실측 시간의 속도 승격은 서로 다른 판단**이다. 이번 요청은 이론 연구이며 production 코드는 변경하지 않았다. 수학적 신규성이나 proof-assistant formalization도 주장하지 않는다.

## 1. 최신 결과를 반영한 출발점

103개 공개 연구 ledger 행과 supersession 관계를 통합했고, superseded되지 않은 행은91개다. 이전 Loops01–11의 보존 자료·감사 결과와 최신 native 연구를 함께 읽었다. 과거 과학 campaign의 재실행은0회다. `REUSED_RESULTS.json`은 각 행의 source와 결과 identity를 보존한다. 모든 historical raw payload가 현재 archive-index branch에 게시되어 있다고 주장하지 않는다.

이전 감사 이후 바뀐 점이 중요하다.

| 항목 | 이번에 채택한 최신 상태 | 연구에 주는 함의 |
|---|---|---|
| AS01/AS02/AS03 | fallback overflow, predictive underflow, malformed evidence PASS 경로 수정 | 과거 결함을 현재 미해결 결함으로 재기재하지 않음 |
| SP01 | duplicate residual 제거가 versioned opt-in에서 PASS | 이미 있는 절감을 새 이론 성과로 세지 않음 |
| SP03 | dense-reference 구조 검증 setup 때문에 gate FAIL | 검증된 local expression graph로 O(n²) setup을 O(nb)로 바꾸는 직접 연결점 |
| CT01 | fixed interior-grid corpus에서 PASS;644cells에서 PREDcap2=PREDcap | 정확도 증거는 넓어졌으나 새 controller 분기의 효과·전역 정리와 구분 |
| AS04/05 등14개 remaining nodes | preregistration만 존재 | 새 연구가 이들의 실행 결과를 만들어낸 것처럼 표시하지 않음 |
| Laguerre·homotopy·chart | signed adjoint, finite causal inverse, 특정 모델 global certificate 등 이미 존재 | 이 기반 위에서 실행 degree·총 비용·적용 모델 class를 확장 |

현재 일반 AS05 residual-gain route에 없는 것을 기존 repository 전체에 없다고 말해서도 안 된다. 예를 들어 `outward_certificate.rs::InverseWitness::approximate`는 이미 quadratic-stage 계약에서 Neumann inverse-defect witness를 구현한다. 이번 새 연결점은 **그 witness의 매개변수 셀 재사용과 일반 current-target binding**이다.

## 2. 얻은 긍정적 보장

| 정리 | 확인해야 하는 구조 | 보장되는 개선 | 비용과 적용 경계 |
|---|---|---|---|
| P1/P2 | full-space 소거관계 q(X)=0 또는 검증된 작은 defect | degree-m Laguerre action을 degree≤d−1로 압축; RHS당 products m→d−1 | scalar compilation O(md), witness·roundoff 비용 포함; Chebyshev 등에도 같은 engine 적용 가능 |
| P3 | A=−κI+N, N^d=0 또는 near-nilpotent bound | shifted solve가 d−1 N-products에서 끝남; 큰 nonnormal N도 허용 | gain이 크면 finite-precision accuracy는 어려울 수 있음; direct structured solve와 비교 |
| A1/A2 | 매개변수 box 전체에서 ||I−PB(θ)||≤q<1 | 동일 셀에서 inverse certificate 재계산을 membership으로 대체 | N(C_full−C_member)>C_compile+C_extra이면 총 modeled work 엄밀히 감소 |
| A4 | f/J가 같은 검증된 row-local expression graph에서 생성 | band 구조를 whole-domain identity로 보장하고 dense-reference setup O(n²)→O(nb) | local graph 총 길이 O(nb), primitive regularity·AD 계약 필요; 임의 callback에 적용 불가 |
| H1–H3 | invertible common W, strictly causal stage target, certified predictor/tube | 같은 homotopy branch의 독립 window 계산; s-stage target에서 m<s parallel waves | 모든 setup·barrier·인증·실패 비용 포함한 span inequality 필요; work가 늘 수도 있음 |
| H5 | causal majorant B와 leakage E에서 ||(I−B)^−1 E||<1 | 원래 계수를 유지한 actual-target 존재·유일성·오차 인증 | 원래 target의 B/E/tube를 검증해야 하며 exact s-round 종료는 주장하지 않음 |
| H4 | 정확한 affine invariant model와 직접 reduced callback | 원래 stage target을 보존하면서 n차원 factorization을 r차원으로 축소 | full↔reduced operator identity·물리 metric·lift·setup 비용 포함 |
| G1–G3 | 현재 W, stage tube, truncation, exact-flow stability, rounding 상계 | accepted stage residual을 실제 ODE의 전역오차 상계로 운반 | estimator나 projected ν로 전제를 대체할 수 없음 |
| G4/G5 | 검증된 contraction과 고정 영향도 bounds | 동일 global budget에서 최적 비균일 residual allocation; 특정 family의 Θ(mL)→O(L+m log m) | 같은 backend의 scheduled-work 비교; arbitrary GMRES/실측 시간에 대한 정리 아님 |

### 2.1 Laguerre를 실제로 더 싸게 만드는 방법

q는 현재 연산자의 full-space 관계이며 degree가 d라고 하자. 다항식 나눗셈으로

\[
P_m(x)=q(x)T(x)+R(x),\qquad \deg R<d
\]

를 얻는다. q(X)=0이면 **P_m(X)=R(X)**이므로 긴 n차원 Laguerre recurrence 자체를 실행할 필요가 없다. scalar quotient ring에서 기존 three-term recurrence를 먼저 계산하고 짧은 action만 실행한다. 새 계산에서는 기존 m-step vector recurrence의 rounding/adjoint 오차가 발생하지 않는다. 대신 scalar compilation과 짧은 Horner evaluation의 오차를 인증한다.

더 유연한 경우도 증명했다. X가 Hermitian, spec(X)⊂[−1,1], A=cI+sX≤0, h≥0이고 q의 interpolation nodes가 [−1,1]에 있으면, degree<d Hermite polynomial r_k에 대해

\[
\|\varphi_k(hA)-r_k(X)\|
\le \|q(X)\|\frac{(hs)^d}{(d+k)!}.
\]

따라서 exact 소거관계가 없어도 **현재 연산자의 near-annihilator를 인증할 수 있는 regime**에는 낮은 degree를 쓸 수 있다. 반대로 eigenvalue 목록만으로 nonnormal matrix에 이 상계를 적용하지 않는다.

새 exact-rational 예제에서 degree64 Laguerre는 q(x)=x²−1을 만족하는 X에 대해 degree1 나머지와 정확히 일치했다. action products는64회에서1회다. 이것은 실제 VigilODE trajectory의64배 가속 측정이 아니다. witness·계수 계산·다른 최강 structured comparator까지 넣은 `P-cost`가 양수일 때만 총 work 개선을 보장한다.

### 2.2 Homotopy의 순차성을 줄일 수 있는 이유

K-form의 exactly causal reference target은

\[
H_{\theta,\lambda}(K)=DK-b_0-[\theta+\lambda(1-\theta)]CK-\lambda hN(K),
\qquad D=I_s\otimes W
\]

이다. C는 strict-lower이고 N_i는 이전 stage만 읽는다. 따라서 W가 가역이면

\[
\det D_KH_{\theta,\lambda}=\det(W)^s\ne0.
\]

정확히 strict-lower인 reference 또는 declared target에서는 callback domain 안에서 stage를 차례로 소거하면 각 λ의 root는 유일하며 매끄러운 하나의 graph다. **이 조건을 만족하는 target의 branch 정체성을 보장하기 위해 λ를 반드시 순서대로 따라갈 필요는 없다.** 공통 predictor/tube가 있으면 여러 window의 correction을 독립적으로 실행할 수 있다. 필요한 endpoint가 λ=1 하나면 불필요한 여러 λ를 만들지 않는다.

현재 represented K-form은 generic inverse로 계수를 생성하면서 작은 diagonal/upper leakage를 남긴다는 기존 공개 증거가 있다. 따라서 위 exactly causal 전제를 현 코드에 자동 적용하지 않는다. 원래 target의 leakage를 별도 E로 감싸는 후속 정리와 current-operator witness가 필요하다. 계수를 단순히0으로 덮어써 원래 target을 바꾸는 방식을 승인하지 않는다. U-form의 명시적 j<i assembly는 별도의 exact-causal 적용 경로다.

비음수 strict-lower error transfer B를 인증한 reference에서 Jacobi correction error는

\[
e_m\le B^m e_0+\sum_{j=0}^{m-1}B^{m-1-j}\rho_j
\]

다. B^s=0이므로 exact arithmetic에서 최대 s rounds, 유리한 predictor/weak-coupling regime에서는 그보다 적은 rounds가 충분하다. 예를 들어 s=8에서3waves가 허용오차를 만족하고 wave 상계가 t, 모든 추가비용이2t 이하이며 지정 serial comparator의 하계가8t이면5t<8t를 보장한다. **이는 명시된 cost model의 span 보장**이다. 24 stage tasks가8 tasks보다 적다는 주장은 하지 않는다.

H5는 이 leakage를 포함한 actual map의 majorant를 B+E로 두고 S=(I−B)^−1, T=SE를 사용한다. ||T||≤q<1과 실제 tube closure를 검증하면 (I−B−E)^−1=(I−T)^−1 S가 비음수이므로 actual residual d에 대해 ||error||≤||Sd||/(1−q)다. 공통 λ-tube까지 확보하면 동일 branch를 인증할 수 있다. 서로 독립된 tube의 root를 근거 없이 같은 branch로 취급하지 않는다. 이번에는 실제 코드의 B/E/q를 계산하지 않았으므로 이것은 명시적인 이식 정리다.

총 work 자체를 낮추려면 H4의 invariant affine reduction이 더 직접적이다. f(y_ref+Uz)=Ug(z), JU=UJ_r 등 exact identities가 성립하면 full stage는 Uk_i이고 r차원 solve의 lift와 정확히 같다. Near-causal actual target에 적용하려면 full target의 인증된 유일성과 reduced root의 lift가 full certified tube 안에 있다는 조건도 필요하다. stiffness가 큰 transverse mode가 있어도 입력이 정확한 invariant manifold에 있으면 그 큰 factorization을 피할 수 있다. 작은 off-manifold 성분을 임의로 버리는 행위는 이 정리로 허용되지 않는다.

### 2.3 전역 정확도를 유지하면서 덜 푸는 방법

G1의 비음수 causal transfer에서 각 residual이 최종 출력에 미치는 가중치 w를 backward sweep으로 구한다. G3는 metric transport와 exact-flow stability를 포함해

\[
E_N\le\Bigl(\prod_n S_n\Bigr)E_0+
\sum_n\beta_n\{\tau_n+e_{out,n}+w_n^T(\rho_n+\xi_n+\omega_n)\}
\]

을 증명한다. 여기서 τ는 실제 ODE truncation 상계, ξ는 rhs/time/assembly, ω는 실제 operator mismatch다. 실제 binary64 tableau의 낮은 차수 order-condition defect도 τ에 포함한다. embedded error를 τ로 대체하지 않는다.

남은 budget B에 대해 influence a_i와 검증된 residual 감소 R_i q_i^k가 주어지면, p_i=c_i/(−log q_i)를 사용한 연속 최적해는

\[
\rho_i^*=\min\!\left(R_i,\frac{p_i}{\lambda a_i}\right),
\qquad \sum_i a_i\rho_i^*=B.
\]

정수 횟수는 위쪽으로 올리고 최종 합을 다시 outward 검사한다. a=(4096,1,1,1), q=1/2, 단위 correction 비용, B=1/16인 정확한 예에서 best 공통 threshold는68회, 최적 비균일 schedule은(18,6,6,6)으로36회다. **추가 allocation/인증비용이32회분보다 작으면 총 work도 줄어든다.** 이미 빠르게 수렴하는 early-stop baseline보다 무조건 빠르다는 주장은 아니다.

## 3. 고등 수학을 어디에 쓰고 어디에는 쓰지 않는가

몫환과 primary decomposition은 eigenvalue별 값뿐 아니라 Jordan multiplicity의 finite jets를 보존해 matrix function을 압축한다. 미분위상의 implicit-function theorem은 causal target의 local graph를 제공하고, 삼각 소거의 전역 유일성이 그 graph들을 동일 branch로 붙인다. 실대수적 Bernstein/SOS certificate는 한 점에서만 참인 작은 inverse defect를 매개변수 영역 전체의 명제로 바꾼다. 불변 affine manifold는 원래 discrete stage target을 바꾸지 않는 차원 축소를 허용한다.

이 기초 수학은 기존 문헌과 repo 연구의 연장이다. 일반 Gröbner elimination이 싸다거나, nonlinear coordinate transformation이 RODAS step을 그대로 보존한다거나, homotopy가 시간의 모든 순차성을 없앤다는 주장은 하지 않는다. 이번 성과는 **명시한 구조를 가진 client에 대해 정확성 및 감소량을 증명하고 현재 API의 빈칸과 연결한 것**이다.

## 4. 가장 먼저 개발할 것

실행 가능한 상세 순서는 [PORTING_DAG.json](../../../research/promotion_theory_20261011/PORTING_DAG.json)에 있다. 모든 새 native node는 아직 미실행이며, 기존 preregistration을 바꾸지 않는 새 extension node로 등록해야 한다.

1. **generated-model 구조 계약과 parameter-cell witness.** SP03의 실제 setup 병목을 겨냥한다. 동일 expression graph에서 RHS/Jacobian/interval enclosure를 생산하고, AS05의 current-target binding으로 연결한다.
2. **whole-stage influence와 global ledger.** AS04/05 다음에 G1–G3의 private authority types를 붙인다. fixed metric·선언된 polynomial model에서 먼저 완전한 finite-trajectory certificate를 만든다.
3. **비균일 잔차 allocator.** 검증된 Richardson 또는 polynomial-cycle contraction을 가진 backend에서 같은 budget의 uniform schedule과 비교한다. 가중치·tube가 allocation 변경에도 유효한지 확인한다.
4. **quotient action 두 family.** exact projector와 bounded-depth nilpotent 구조부터 시작한다. 일반 JVP로 minimal polynomial을 추정하는 경로는 첫 구현에서 제외한다. direct formula/triangular solve도 비교한다.
5. **실제 expensive client의 causal waves 또는 r≪n invariant reduction.** 전자는 span, 후자는 work를 겨냥한다. kernel timing만이 아니라 setup·certificate·packing·barrier·실패·fallback·physical lift를 모두 charge한다.

제안하는 API들은 이번 파일에서 설계한 것이며 현재 이미 존재하는 API 이름으로 오인하면 안 된다. 직렬화된 machine-readable 보고서는 authority object가 아니다. native constructor가 operator/model/tube/metric과 proof data를 확인한 뒤 private certificate를 만들어야 한다.

## 5. 실제 수행과 승격 판정의 해석

수행한 것은 최신 source 읽기, 공개103행 통합, 위 정리들의 전체 유도, 신규 exact/symbolic 예제, 두 독립 decision reviewer의 적대적 검토다. Wolfram Language는 Bézout identity·triangular Jacobian·nilpotent cell inverse 등을 기호 검산했고, Fraction 예제는 동일 identity와 계산량 예시를 확인했다. SciSpace는 문헌 발견에 사용하고 primary sources와 직접 유도로 분리했다. 자세한 열람 범위는 `LITERATURE.json`에 있다.

검토 중에는 restricted inverse gain의 범위, truncation 상계 표기, Hermitian 조건의 JSON 누락, dense-reference 경로 한정, represented K-form의 비인과 coefficient leakage 등 실제 수정이 있었다. 최초 지적과 수정 이유는 `CORRECTIONS.json`에 보존한다. 검토가 통과했다는 이유로 처음부터 완전했던 초안이라고 기록하지 않는다.

**현재 도달한 결론은 조건부 정확도·복잡도 정리와 이식 계약의 승격이다.** production/global executable/wall-time를 자동 승격할 수는 없지만, 남은 조건은 추상적인 “더 검증하라”가 아니다. current full-space witness, model/tube binding, truncation enclosure, actual cost receipts와 route별 native admission으로 구체화했다. 이 객체들이 구현되어 정리의 전제를 충족하면, 해당 모델·궤적·비용 regime에 대해서는 정리의 결론을 직접 적용할 수 있다.
