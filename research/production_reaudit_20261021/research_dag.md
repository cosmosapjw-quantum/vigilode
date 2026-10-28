# 연구 DAG 통합 감사와 정리 교정

대상은 최신 production 코드 `f2f20c1d8cd606e70ddbd8409c0f3b4529e928a1`와 계획 `cc2e1a0`를 합친 감사 기준점 `4a5fc656730fe8243f06b6e603cb8aa758b00b2f`이다. 이 문서는 **기준점의 읽기 전용 감사**와 새 증명 결과를 기록한다. 이후 실제 이식·수정·테스트 결과 및 최종 production 판정은 이번 패키지의 최종 실행 보고서가 담당한다. 이 구분 때문에 아래의 “미이식”은 최종 납품 상태를 뜻하지 않는다.

## 1. 중요한 통합 사실

2026-10-18에 공개한 `33e45625be0803771156704263c3b1c1f92d2b40`는 최신 production 기준점의 조상이 아니다. 공통 조상은 `59d95eb7404882f024eb8bd3856f2ce4f1f830c9`이다. 해당 연구 브랜치의 최신 tip은 10월 19일 `ee65d4dbecf3b3cd5f4587c27f87ef297d2bb406`이다. 따라서 기존 A01–A12의 scoped PASS와 production 계획의 HOLD는 서로 다른 코드·범위에 관한 판단이다. 한쪽의 상태를 다른 쪽에 복사하면 안 된다.

기존 source manifest의 414개 항목을 기준점의 Git 객체와 비교했다. **321개는 SHA-256 동일, 46개는 변경, 47개는 부재**다. 부재 항목에는 `polynomial_router`, `packet_verified_trajectory`, `damped_packet`, `generic_verified`, `gray_scott_verified`, `collective_rodas5p`, `actual_k_homotopy`, `nonlinear_homotopy`, `affine_transport`의 9개 solver 모듈이 포함된다. 이는 divergent branch 간 차이이며, 의도적인 삭제가 있었다는 주장이 아니다. 전체 경로 목록은 `research_dag.json`에 있다.

이 때문에 production 계획의 “한 줄기를 골라 다른 줄기를 tombstone” 지침을 **증명이나 유효한 구현 폐기**로 해석하지 않는다. 현재 facade/fast/lagged 경로는 유지하고, 별도 연구 줄기의 원래 model binding·오차 권한을 보존한 scoped API를 이식한 뒤 변경된 인터페이스만 검증한다. ledger id 충돌은 역사적 행을 덮어쓰지 않고 `(branch, commit, id)`로 구분한다. 기존 수학 증명과 실패한 benchmark의 결론은 그대로 보존한다.

## 2. 원래 A01–A12: 모든 노드의 기준점 상태

여기서 A01 등의 번호는 10월 18일 감사 DAG다. 10월 20일 production 계획의 A1 등과 다른 이름 공간이다.

| 노드 | 공개된 sibling 성과 | production 기준점의 상태 | 이식·검증 의무 |
|---|---|---|---|
| A01 | 불변 weighted initial-ball owner, 반환 중심에 결합된 radius | local represented-window certificate는 있으나 weighted owner 확장 부재 | 기존 weight sum·carrier 제한 유지, 중심 반올림 포함 여부 확인 |
| A02 | 원래 mesh/budget을 지키는 nonlinear adaptive verified trajectory | 해당 driver 부재 | 초기 radius·거부된 작업·headroom 및 A10 재시도 연결 |
| A03 | 공통 감쇠 packet, 평균 계수와 원래 ODE에 대한 변동 오차 | scalar damped moment만 존재 | κ=0·선형·공통 주파수 극한과 inherited radius 검증 |
| A04 | 원래 budget을 강제하는 immutable Leja admission token | legacy EstimateOnly와 218개 Fraction oracle는 존재 | token/total arithmetic 권한을 구분해 이식; 임의 nonsymmetric 확대 금지 |
| A05 | model-bound continuous residual controller | W-filtered **국소 추정기**는 있으나 generic verified owner 부재 | 두 mode를 병합; PR101 모델 타입 충돌 해소; 인증곡선과 callback 결합 유지 |
| A06 | 모든 8단계 rank-two solve 및 실제 stage residual witness | frozen linear solve만 존재 | 현재 exact-band 개선 보존, Schur inverse fallback과 stage owner 검증 |
| A07 | source-bound matched-error benchmark 도구 | 새 132-row 외부 비교 corpus 존재; 다른 대상의 측정 | 양쪽 raw 실패 보존; 서로 다른 host/대상 결과를 섞지 않음 |
| A08 | 같은 operator/h/vector/budget의 Leja·Laguerre·Chebyshev admission | legacy polynomial router만 존재 | guarded API 이식 후 original budget과 full candidate cost 검증 |
| A09 | 실제 K 계수 stage certificate 및 nonlinear window homotopy client | 기존 frozen homotopy/coupled 연구만 존재 | 실제 upper β 항·최종 λ=1·원래 ODE endpoint certificate 보존 |
| A10 | frozen represented curve의 Bernstein residual 재검증 | 보조 함수만 존재 | 동일 owner·동일 물리 budget, 비용 계수와 depth cap 확인 |
| A11 | immutable Gray–Scott provider와 verified trajectory | log-norm helper만 존재 | full reaction symmetric part 및 cubic degree-12 residual 유지 |
| A12 | scoped PASS, broad production/speed HOLD | 오래된 root DAG 및 새 production 계획이 병존 | 새 코드·각 scope·실행을 결합한 독립 결정 필요 |

과거 증거가 동일한 입력·계수·모델·구현에 대해서만 재사용된다는 점을 각 JSON 노드에 기록했다. 실제 이식은 별도 작업자에게 맡겼으며 여기서 실행하지 않은 native test를 PASS로 표기하지 않았다.

## 3. production 계획 전체와 wave 4

계획 본문은 workstream을 14개라 하지만 표에는 **15개**가 있다: A1–A4, B1–B3, C1–C4, D1–D2, E1–E2. 본 감사는 모두 포함한다.

| workstream | 기준점 판단 | 다음의 구체적인 완료 조건 |
|---|---|---|
| A1 facade | 부분 구현 | sequential/vector tolerance 조합, facade 소유 Stats, typed event 실패, scoped 연구 이식 |
| A2 interior | 국소 estimator 구현 | unavailable estimate 거부를 두 driver에서 일치시키고, global error와 분리해서 판정 |
| A3 controller | 미완료 | van der Pol 원래 정확도를 유지하며 거부율 목표 충족 |
| A4 routing | dense/banded/JVP 부분 구현 | sparse colouring·small-N·low-rank에 검증된 구조 선언과 work receipt |
| B1 lagged stages | 기존 kill gate FAIL; unrouted | HIRES 0.588 fact/accepted를 실패로 보존; 새 resolvent 기작을 별도 카드로 검증 |
| B2 JVP tier | 연구 구현 | 구조 preconditioner·forcing·PDE corpus에서 반복 상한/정확도 확인 |
| B3 batch | 연구 구현 | 1024 Robertson 조건의 facade 통합 및 기존 ≥4x throughput gate |
| C1 transaction | 독립 구현 | 모든 candidate의 공통 commit/rollback/work 경계로 사용 |
| C2 exponential | frozen 연구 | 원래 출력 계약과 full-cost acceptance, sealed holdout 유지 |
| C3 structure | 특정 모델 성과 | exact closure 및 current protected comparator에 대한 full-cost 검증 |
| C4 economics | 부분 구현 | causal cost accounting와 deterministic cap; 증명되지 않은 regret 주장 제거 |
| D1 generated certificate | 두 줄기의 scoped 성과 | 같은 source line에서 model binding·초기 불확실성·전역 전달 closure |
| D2 damped/Leja/generic | scoped sibling 이식 필요 | 기존 증명 재사용, 변경된 Rust 연결과 owner 권한 검증 |
| E1 benchmark | 4개 문제·132 rows | timing은 기존 INCONCLUSIVE 유지; 독립 reference와 corpus 확장 |
| E2 release | 미완료 | blocking semver baseline, release 문서·외부 검토·각 scope gate |

Wave-4 의존성은 다음과 같다. Task 0은 기준점/증거 정렬, Task 1은 sequential dense fail-closed, Task 2는 local/global 오차 분해, Task 3은 새 resolvent staleness, Task 4는 allocator/hygiene, Task 5는 Task 1 이후 Stats/events, Task 6은 **Task 2 PASS와 Task 3 이후에만** error-per-unit-step, Task 7은 통합 검토다. 이번에는 이론상 가능하다는 이유만으로 Task 6의 조건을 건너뛰지 않는다. 모든 task와 원래 의존성은 JSON에 있다.

## 4. T1–T9의 일회 적대적 감사

### T1: stage perturbation과 order의 범위

엄밀한 stage bound는 유효하다. 다만 벡터 함수의 `f(x)-f(y)=J(ξ)(x-y)`는 일반적인 다변수 평균값 정리가 아니다. 정확한 식은

\[
f(x)-f(y)=\left(\int_0^1J(y+s(x-y))\,ds\right)(x-y)
\]

이며, segment 전체의 Jacobian drift 상한을 가정했으므로 같은 majorant 증명이 성립한다. `tol` 비례 stage perturbation은 그 **한 step의 오차**를 제어한다. 고정 absolute residual을 유지하면서 h→0에 대해 5차 수렴을 보장하지는 않는다. 5차 global order를 정리로 쓰려면 안정성 가정 아래 추가 local perturbation을 O(h⁶)로 제한하거나 적응 tolerance와 h의 관계를 명시해야 한다.

계획의 6.25/23.34 sensitivity는 real negative scalar spectrum에서 구한 값이다. 일반 normal left-half-plane spectrum에는 적용되지 않는다. 적분기 감사자가 z=i,2i에서 독립적인 exact-complex 반례를 확보했다. 따라서 이 값은 그 범위를 벗어나면 operational target이며 certificate가 아니다.

### T2: norm과 좌표를 지킨 resolvent 개선

Neumann/GMRES bound는 GMRES가 최소화하는 norm과 bound의 norm이 맞을 때 유효하다. 임의 weighted norm에서 계산한 q를 Euclidean GMRES의 최소화 정리에 그대로 넣을 수 없다. 좌표를 바꾸었다면 그 metric을 실제 선형 연산에 적용해야 한다.

다음 결과는 계획의 factor 2를 줄인다. 고정 SPD metric M에서 μ_M(J)≤0이고 P=I−aJ, a=h_old γ>0라고 하자. R=P⁻¹, v=Rx이면

\[
\|x\|_M^2=\|v-aJv\|_M^2
=\|v\|_M^2-2a\langle v,Jv\rangle_M+a^2\|Jv\|_M^2
\ge\|(R-I)x\|_M^2.
\]

따라서 \(\|R-I\|_M\le1\). 같은 J의 normalized W=I−hγJ에는

\[
I-P^{-1}W=(h/h_{old}-1)(R-I),\quad q\le|h/h_{old}-1|.
\]

q≤1/2를 허용하면 h 변경 폭을 ±25%에서 ±50%까지 인증할 수 있다. 이것은 factorization 재사용 영역을 넓히는 **조건부 알고리즘 보장**이다. 현재 U-form 구현은 P=I/(h_oldγ)−J, W=I/(hγ)−J를 사용하므로 그대로 적용하지 않는다. 그 좌표의 same-J 식은 `(1−h_old/h)R`이다. normalized preconditioner로의 명시적 rescaling과 fixed-M dissipativity witness가 이 개선의 구현 전제다. 일반 infinity/WRMS norm에 대한 확대도 하지 않는다.

### T3: 끝까지 닫은 monotonicity 증명과 정확한 정의역

0<γ≤1/4, x=−z/2>0, a=2γ에 대해

\[
R(x)=\frac{x}{(1+ax)(1-e^{-x})},\quad
R'(x)=\frac{1-(1+x+ax^2)e^{-x}}{[(1+ax)(1-e^{-x})]^2}>0.
\]

마지막 부등식은 `e^x>1+x+x²/2≥1+x+a x²`로 증명된다. 양 끝 극한은 1과 1/(2γ)이므로 기존 grid-only 주장을 real negative scalar **전 구간**의 증명으로 승격할 수 있다. 정상행렬 중에서도 실수 비양의 고유값에만 이 scalar 결과를 적용한다.

복소 왼쪽 반평면 전체의 같은 비율 주장은 잘못이다. z=4πi, 일정한 nonzero defect에서 midpoint convolution `(e^(z/2)−1)/z`는 0이지만 filtered estimate는 nonzero다. 실제 nonlinear quartic 곡선의 defect는 일정하지도 않다. 이 정리는 midpoint estimator의 한 scalar 해석이지, sparse sampling만으로 continuous/global error를 인증하는 정리가 아니다.

### T4: residual degree 교정

continuous defect와 two-state dissipativity로부터 얻는 comparison 정리는 유효하다. 그러나 “polynomial 또는 rational f이면 degree≤2p”라는 귀결은 틀리다. autonomous polynomial RHS의 total degree가 r이면 degree-p curve의 defect 차수는

\[
\deg(\hat y'-f(\hat y))\le\max(p-1,rp).
\]

quartic curve와 cubic Gray–Scott/collective nonlinearity에서는 degree 12가 필요하다. rational RHS는 일반적으로 rational defect를 만든다. 분모가 tube에서 0에서 떨어져 있다는 enclosure 또는 interval quotient가 필요하다. time-polynomial 계수까지 있으면 time degree를 더해야 한다. 이 교정은 Bernstein 구현의 배열 길이와 연산 비용을 결정하므로 단순 문구 문제가 아니다.

전역 budget과 작업량 최적 배분을 위한 별도 구성적 결과 G1/G2는 `DENSE_GLOBAL_BUDGET_THEORY.md`에서 증명했다. 본 작업자는 이를 독립 검토했다. Hölder 지수·gain 배치·1640→1000 작업량 예는 맞으며, positive-real relaxation의 의미와 곡선 recenter jump의 전파 비용에 대한 수정도 적용되어 독립 검토를 통과했다.

### T5: causal majorant

strict lower triangular B의 nilpotence와 finite-round propagation은 유효하다. nonnegative inverse를 쓰려면 E≥0도 명시해야 한다. 실제 부동소수점 tableau의 비삼각 성분은 E에 포함하고 true bound q<1을 확인한다. exact triangular model의 determinant identity를 actual K tableau나 window-parallel floating iteration에 자동 전용하지 않는다. 기존 A09는 그 구분을 수행했으므로 그 증명을 재사용한다.

### T6: 구조 감소의 정확한 비용

invariant manifold와 Woodbury의 대수적 identity는 유효하다. generic dense low-rank S,V의 비용은 다음 항을 포함한다:

\[
\text{setup}=O(nb^2+nbr+nr^2+r^3),\qquad
\text{RHS}=O(nb+nr+r^2).
\]

기존 표기에서 `VᵀZ`의 nr²와 작은 Schur solve의 r²가 빠졌다. r이 고정 상수이거나 충분히 작은 별도 regime에서만 흡수한다. residual 한 번은 residual 값만 계산한다. solution-error certificate에는 inverse-norm/conditioning witness가 추가로 필요하다.

정확한 annihilator q(X)=0에서 degree-m polynomial의 Horner action은 일반적으로 m operator products, quotient remainder는 최대 d−1 products다. quotient가 원래 target과 같다는 대수적 구조는 재사용하며, near-annihilator와 spectral enclosure의 조건을 유지한다. “degree-m에 m−1”이라는 표현은 degree와 항 개수를 혼동한다.

### T7: 비용 모형

exponential candidate의 regime 분석은 유용한 비용 모형이다. m≈2hρ는 모든 matrix/vector에 대한 필요 차원이 아니다. 고유벡터 입력은 hρ가 커도 1차원 invariant subspace에서 exact action이 가능하다. 해당 문구를 복잡도 하한이나 universal speed theorem으로 읽지 않는다. 구조 quotient, carrier 제거 및 full-cost admission으로 실제 이득을 검증한다.

### T8: deterministic economics와 bandit 주장의 분리

같은 accepted advance에서 fixed cost라면 `C_a+C_v+(1−p)C_f<C_f`와 threshold `p>(C_a+C_v)/C_f`는 맞다. 다만 임의의 Beta-posterior threshold 정책은 UCB1이 아니며, 상태에 따라 분포가 변하는 solver bin은 UCB1 정리의 고정 conditional mean을 자동으로 만족하지 않는다. 따라서 현재 문서의 regret rate는 미증명이다.

Auer–Cesa-Bianchi–Fischer 원문에서 실제 UCB1은 empirical mean과 Hoeffding bonus를 사용하며 bounded rewards 및 일정 conditional mean을 가정한다. 이 사실은 Exa로 원문을 대조했다. 현재 구현을 그 알고리즘으로 가장하지 않고 **deterministic work-credit cap**을 우선 사용하면 분포 가정 없이 speculative overhead를 제한할 수 있다. cap은 실제 charged work와 비교해야 하며 causal probe가 언젠가 끝났다는 관측만으로 사전 worst-case cost를 보장하지 않는다.

### T9: 새 exact Bernstein sensitivity proof와 실제 작업 감소

`verify_sensitivity_bounds.py`는 기존 400-point 수치 grid를 재실행하지 않는다. 미해결이었던 **전 구간 상한**을 exact Fraction arithmetic로 증명한다. transformed linear stage system에 대해

\[
T_{ij}(w)=\gamma C_{ij}w+A_{ij}(w-1),\quad
p=b^T(I-T)^{-1},\quad w=(1-\gamma z)^{-1}\in[0,1]
\]

를 backward polynomial recurrence로 계산한다. 각 p_j는 degree≤7이다. power→Bernstein conversion과 dyadic de Casteljau subdivision으로 8개 구간, 최대 깊이 7에서 coefficient convex-hull bound가 모두 닫힌다. decimal tableau와 JSON을 binary64로 읽은 **정확한 이진 값** 모두에 대해 다음을 증명했다:

- scalar 모든 w에서 `|p_j(w)|≤6.25`, `1≤Σ|p_j(w)|≤23.34`;
- normal nonpositive-real spectrum의 arbitrary residual direction에는 각 stage의 **전 구간 supremum**을 먼저 구해 더해야 한다. 그 합은 23.346026 미만이므로 간단한 유리수 bound **23.35**를 쓴다.

두 번째 구분은 독립 검토에서 수정했다. pointwise sum의 supremum과 supremum들의 sum을 바꾸면 operator certificate가 되지 않는다. unpreconditioned residual에는 추가 factor w가 필요하며, 이것은 exact linear-stage 분석이다. 실제 binary64 solve/recurrence rounding이나 nonlinear Jacobian drift를 이미 포함했다는 주장이 아니다.

**조건부 작업량 감소의 구체적 예:** 모든 stage의 초기 preconditioned residual R=1, 검증된 contraction q=1/2, output budget B=1/16이고 매 iteration cost가 같다고 하자. 기존 coefficient-sum bound 189.418…에 공통 threshold를 적용하면 stage당 12회, 총 96회가 필요하다. 새로운 normal-operator bound 23.35는 stage당 9회, 총 72회면 충분하다. 같은 budget에서 **24회, 25%의 선형 반복 작업 감소를 보장**한다. 이는 해당 contraction·spectrum·cost 가정의 정리이며 benchmark speedup을 뜻하지 않는다.

T9의 KKT allocation은 positive fixed a_i,c_i,R_i와 true q_i∈(0,1) 아래 유효하다. B≥Σa_iR_i이면 iteration 0이 optimum이고, 0<B<Σa_iR_i에서 equality budget을 풀어야 한다. real optimum k_i*를 ceil하면 error는 감소하고 추가 work는 `<Σc_i`다. 따라서 이 rounded construction은 integer optimum보다 작업량이 `<Σc_i`만큼만 더 많다. adaptive GMRES q 추정치·Ritz 값은 이 확정적 가정과 별도로 관리한다.

## 5. 구체적인 다음 코드 단계

1. **정확도 우선:** sequential estimate 실패를 fast path와 같은 reject/retry로 바꾸고, accepted curve/partial output의 transaction 경계를 검증한다. 실패한 sample을 unavailable로 세고 수용하는 흐름을 제거한다.
2. **scoped certificate 통합:** 원래 A01–A11의 private binding·admission API를 이식하되 최신 facade, WFiltered, banded defect fixes를 보존한다. PR101 타입 이름 충돌은 별도 model 이름으로 해소한다. A12는 이식 결과의 새 독립 결정이다.
3. **전역 정확도:** 원래 output mesh와 tolerance로 local flow error, continuous defect, propagation gain을 분리한다. degree-12 residual과 full tube를 제공할 수 있는 모델부터 G1 budget owner를 붙인다. generic callback에서 estimator만으로 global certification을 주장하지 않는다.
4. **선형 속도:** B1c FAIL를 보존한 채 B1d resolvent norm을 선택적 기작으로 검증한다. 정상 real-spectrum witness가 확보된 별도 경로에서 새 exact p_j bound와 T9 allocation을 연결한다. 임의 complex/nonnormal 경로는 더 큰 검증된 bound 또는 기존 diagnostic target을 유지한다.
5. **구조 속도:** Woodbury 비용에서 nr²/r²와 certificate/materialization을 모두 세고, current protected baseline과 같은 achieved error에서 비교한다. Laguerre·Leja·Chebyshev는 같은 입력과 original budget의 guarded selection으로 넣는다.
6. **homotopy:** actual target/upper coefficients와 최종 λ=1를 유지한다. direct λ=1과 continuation의 setup+failed trial+final certification까지 짝지어 측정한다. finite span 감소와 wall-time 이득은 서로 다른 주장으로 둔다.
7. **production 승격:** 알려진 scoped 사용 가능성과 full G-A…G-E 완료를 구분한다. 현재 local-control/empirical timing 증거로 broad global-accuracy 또는 generic speed를 promote하지 않는다. 변경되지 않은 published oracle/campaign은 다시 실행하지 않는다.

## 6. R1–R10 / C1–C10 실제 연결

R1은 위의 exact baseline과 scoped production 계약, R2는 기존 DAG/원문/sha 재사용, R3은 T1–T9의 정의역 분리, R4–R5는 exact Bernstein·resolvent·monotonicity 유도, R6은 dense/integrator 작업자와 독립 교차 검토, R7은 exact Fraction proof 실행, R8은 범위별 판정, R9는 위의 함수/owner/코드 계약, R10은 JSON 및 source identity 보존이다.

C1–C4는 두 코드 줄기의 실재 비교와 all-node 병렬 이식 분담, C5–C8은 다른 작업자의 실제 native 수정·표적 테스트 및 본 proof-check 실행, C9는 교차 검토, C10은 root 최종 패키지에 연결한다. 이 문서는 실행하지 않은 production-native 항목을 대신 완료 처리하지 않는다. 여기서 완성한 작업은 감사·수학적 proof artifact와 전체 DAG 연결이다.

문헌: [Auer et al. (2002), primary paper](https://cesa-bianchi.di.unimi.it/Pubblicazioni/ml-02.pdf). SciSpace는 Estep (1995), DOI [10.1137/0732001](https://doi.org/10.1137/0732001), Akrivis–Makridakis–Nochetto (2009), DOI [10.1007/s00211-009-0254-2](https://doi.org/10.1007/s00211-009-0254-2)를 global reconstruction의 관련 선행 연구로 찾아주었다. 이 두 논문의 abstract는 탐색에만 사용했고 검토하지 않은 원문 정리를 증명 전제로 가져오지 않았다.

## 7. 이번 소스 이식과 최신 sibling 실행 증거의 후속 확인

위 기준점 표 이후, 전체 scoped source port가 실제로 통합되었다. `evidence/port/PORT.json` 기준 **native 85개, compile-fail doctest 2개가 PASS**다. 기존 168개 연구 파일은 원래 33e45625의 bytes를 그대로 보존한다. root가 latest ee65의 추가 로컬 증거도 원본대로 보존한다. 현 source에 과거 source manifest를 다시 hash해서 과거 PASS를 위조하지 않는다. 이번 변경의 새 실행 identity는 별도 최종 manifest로 기록한다.

본 작업자는 포트의 두 충돌 지점을 독립 검토했다. 최신 `BoundPr101Problem` 파일은 기준점과 byte-identical이다. 별도의 parameterized PR101은 `pr101_verified.rs`/`Pr101VerifiedError`로 이름과 formatting만 바뀌었다. packet output의 기존 `weighted_center_displacement`와 새 `centered_weighted_radius`는 같은 query의 outward weighted displacement를 공유하며, weighted-ball carry에서만 radius에 한 번 더한다. 따라서 두 API가 공존하면서 증명 의미를 보존한다. dense union은 다른 독립 검토자가 담당했다. 상세 source hash는 `evidence/theory/INDEPENDENT_REVIEW.json`에 있다.

latest ee65의 LOCAL-01–05 및 R02 증거도 한 번 검토했다. **450 raw rows, 15 raw process-file hashes, 9개의 mode/tolerance 그룹**을 확인했고, 450개 모두 empirical accuracy 및 allocator positive control을 통과했다. full-arm allocation은 38–1857회여서 전체 실행 zero allocation은 아니다. rtol=1e-8의 dense/structured 시간비는 mode 8에서 **0.43693**, 32에서 **1.55831**, 128에서 **8.91756**이다. 따라서 dense가 8에서 빠르고 structured가 32/128에서 빠르다. 원래 `LOCAL_REPORT_KO.md`는 이 방향을 반대로 서술했지만 `LOCAL_TIMING_ERRATUM_KO.md`가 이미 교정했다. 교정문을 채택하고 원본 및 negative small-N 결과를 보존한다. instrumented timing/empirical RK4 reference라는 권한은 변하지 않아 speed promotion은 HOLD다. 해당 campaign을 다시 실행하지 않았다.

별도의 246-component campaign도 현재 상태를 구분했다. 169 probe PASS와 한 FD-resolution HOLD, P2 receipt-gated helper, 15 canonical cases·16 panel pairs·17 planned rounds의 P3 config source checkpoint가 기록되어 있다. **P3 inference가 완료된 결과는 아니다.** P3 config review는 별도 EXEC binding 이후 launch를 요구하고 scientific/production admission을 HOLD한다. 따라서 component inventory나 source-only PASS를 전체 solver performance 판정으로 바꾸지 않는다. 기존 누적 72 instance-hour ledger를 유지하며 이번 감사에서 원격 campaign을 시작하지 않는다. `RETAINED_SIBLING_REVIEW.json`에 이 재사용 결정을 기록했다.

추가로 integrator 작업자가 유도한 Cayley 개선을 독립 검토했다. dissipative Hilbert J에서 C=2(I−aJ)⁻¹−I는 contraction이다. `||(I+aJ)v||²≤||(I−aJ)v||²`에서 바로 따른다. p((1+C)/2), m((1+C)/2)의 exact 계수 합을 쓰면 normality 없이 raw residual majorant 합이 **189.418367→31.334689**, preconditioned residual majorant는 **37.602843**으로 줄어든다. 본 작업자는 32개의 exact inverse polynomial substitutions와 coefficient sums를 확인했다. 이 역시 fixed linear Jacobian/Hilbert dissipativity와 정확한 residual 좌표의 조건부 정리이며, nonlinear drift·floating arithmetic·실측 speed는 별도다. 좁은 normal negative-real 경로는 앞의 23.35 bound를, 더 넓은 dissipative 경로는 이 Cayley bound를 선택할 수 있다.
