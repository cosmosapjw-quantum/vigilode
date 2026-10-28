# vigilode 연구 이식 리뷰: 반경 인증, 계산량, 좌표변환의 적용 경계

검토일: 2026-10-02. 검토 기준은 `d1e9ba3b0125ee478c28d0b2c280ca0869289c15`, tree `25e00400a0b8a76ee41171e717094344069a1b28`이다. 게시 대상은 당시 가장 최근에 갱신된 기존 브랜치 `claude/jolly-wozniak-7wl15h-wu25-stiff-benchmark`다. 기본 브랜치 main 또는 더 오래된 PR #69의 head를 최신 상태로 간주하지 않았다. 먼저 프로토콜을 `f2797d8891eb7201da31576080f2118bb23b7b1b`에 게시한 뒤 이 문서의 새 실험을 실행했다.

## 1. 결론

이 스레드의 RVJ5를 기존 RODAS5P에 통째로 이식하는 것은 권하지 않는다. 스레드 자체에서 일반 semilinear uniform 5차와 embedded-only acceptance의 반례가 이미 나왔기 때문이다. 대신 현재 프로젝트에 바로 연결할 수 있는 결과는 (a) 정확한 stage target의 삼각 기하, (b) 반경 closure의 다항식·볼록 구조, (c) 합 행렬을 만들지 않는 인증 action, (d) 실제 물리 좌표로 오차를 운반하는 chart 계약이다.

이번에 실제로 수행한 exact-real 재계산에서는 R4의 8·16차원 예제에 허용 반경이 존재했다. 현재 여섯 번 factor-four 재시도의 최대 반경 1.024가 부족한 것이었다. 현재 residual로부터 얻은 B를 사용한 D=2B는 다섯 차원 모두에서 closure를 만족했다. 또한 같은 유한 path sum을 action-first로 계산하면 8-stage/component의 reference 산술 수가 624에서 294로 줄었다. 이것은 Rust 실행시간이나 native total certificate의 성공을 뜻하지 않는다.

특히 현재 `certify_q2_candidate`는 `certify_stage_target`라는 순차 인증기를 호출한다. 이번 반경 개선은 우선 HOM-04의 병렬 인증 연구 경로를 개선하는 것이며, 현재 q2 승인률이나 fast-v2의 성능을 이미 높였다고 보고하면 안 된다.

최신 L-0033 FAIL과 timing HOLD는 변경하지 않는다. 이번 게시물은 새 timing campaign 또는 solver admission이 아닌 문서 폴더의 독립 리뷰·증명용 코드다. 기존 연구 원장, 보호된 R-JF, coefficient snapshot, holdout, native 코드 및 과거 결과는 변경하지 않았다.

## 2. 실제로 읽은 범위와 이미 해결된 일

연구 원장, README, 브랜치·PR·commit 정보, R4 closure, fast-v2 preregistration/evaluation을 먼저 읽고, 그것을 coefficient loader, StageTarget, outward certificate, homotopy, transactional q1/q2, fast driver, polynomial action의 관련 구현과 대조했다. `SOURCE_MAP.json`은 읽은 파일·범위와 blob identity를 분리한다. 모든 대형 raw trace를 전부 재처리하거나 전체 저장소를 clone/build했다고 주장하지 않는다.

이미 구현된 것을 새 제안으로 반복하지 않는 것이 중요하다.

- coefficient snapshot의 process-lifetime cache와 파생 tableau 재사용이 있다.
- `StageTarget::sequential`은 strict-lower target을 별도로 정의한다. native full alpha/L의 반올림 leakage까지 무시해서 nilpotent라고 하지 않는다.
- `InverseWitness`는 sealed constructor를 사용하며 wire 값은 재검증한다. hash 일치만으로 inverse bound를 승인하지 않는다.
- component-blocked doubling은 diagonal J와 diagonal witness에 이미 존재한다. Rayon pool도 integration 소유로 재사용할 수 있다.
- q2 witness capability는 비싼 speculative work 전에 조사한다. 단, 현재 코드에서는 capability에서 만든 witness를 폐기한 뒤 인증 시 다시 만든다.
- fast-v2는 transformed raw stages, persistent workspace, in-place Jacobian, rejected-state reuse, zero-skipping LU와 row extent를 이미 갖는다.
- native CVODE/BDF, Hairer Radau/RODAS 비교가 존재한다. '비교군을 새로 도입하자'는 것은 현재 상태에 맞지 않는다.

따라서 우선순위는 기능의 존재가 아니라 계산량, bound의 유용성, 정확한 적용 범위와 실제 native admission이다.

## 3. 현재 blocker 지도

### 3.1 Fast-v2의 작은 문제 성능

L-0033의 동일 rtol에서 instruction/attempt 비율은 HIRES 1.0113357409, van der Pol 0.9734587487이다. 둘 다 preregistered 0.95 목표를 만족하지 못했다. 400-component Brusselator는 0.3973320701로 개선됐으며 35개 parity point는 기록상 모두 동일했다. 원 verdict는 여전히 FAIL이다. 큰 문제의 진전을 작은 문제 성공으로 바꾸거나, descriptive timing을 허가된 일반 speedup으로 바꾸지 않는다.

v2 profile에서 `run_arm`에 HIRES 약 82.7%, van der Pol 약 74.8%가 모인다. 이 위치에는 inlined 코드가 포함되므로 이것만 보고 stage algebra가 그 비중을 차지한다고 단정할 수 없다. allocation 제거는 이미 이루어졌다. 다음 미세 실험은 dimension 2/3/8의 정적 kernel, dispatch·검사·stage copy·coefficient iteration·clock 계산을 operation별로 분리하는 것이어야 한다.

van der Pol의 두 clock 함수 share 합은 0.0903987193이다. 나머지가 그대로라는 가정에서 이 부분을 완전히 없애더라도 instruction-model speedup 상한은 1.0993827968이다. 이는 wall-time 예측이 아니며 represented-clock 검사를 제거하라는 제안도 아니다. 작은 문제에서 두 배 가까운 차이를 clock 한 곳만으로 해소할 수 있다는 기대는 이 데이터로 지지되지 않는다.

### 3.2 Banded LU의 실제 복잡도

`lu_in_place`는 매 pivot마다 아래의 모든 행을 탐색하고, multiplier zero 여부도 모든 아래 행에서 검사한다. 각각 정확히 n(n-1)/2개의 loop visit이 발생한다. W 형성 및 dense Jacobian 저장도 n²이다. 따라서 bandwidth b가 제한된 경우에도 전체 구현의 비용은 최소 O(n²+n b²), 저장은 O(n²)로 보아야 한다. row update 산술을 O(n b²)로 줄인 것과 total algorithm을 그렇게 만든 것은 다르다.

이것은 v2의 측정상 개선을 부정하지 않는다. 다만 더 큰 banded PDE로 확장할 때 남는 바닥 비용이다. native banded representation, 검증된 pivot search window, sparsity-preserving callback 계약이 필요한 다음 단계다. 관측된 sparse pattern만으로 다음 Jacobian도 같은 pattern이라고 가정하면 안 된다.

### 3.3 병렬 인증의 반경과 비용

R4의 8·16차원 doubling failure는 'root가 없다'거나 'HOM 경로가 fold에 걸렸다'는 증거가 아니었다. 아래 재계산은 허용 반경을 인증한다. 하지만 R4의 Euler-like 후보는 target-distance bound의 WRMS 항이 대략 2.1e4부터 3.7e6까지 크다. closure가 닫혀도 step acceptance는 전혀 별개다. 또한 이 후보는 실제 q2 후보가 아니라 `homotopy_cost_study`의 고정 probe다.

현재 operational q2 경로는 순차 인증기를 사용한다. 병렬 인증이 연구적으로 성립하더라도, 실제 q2 연결은 새로운 opt-in study와 cost gate가 필요하다. 기존 경로를 조용히 교체하지 않는다.

### 3.4 구조적 witness의 저장 비용

`InverseWitness::diagonal`은 수학적으로 diagonal이지만 `upper`를 n×n `Vec<Vec<f64>>`로 할당한다. `QuadraticStageProblem::jacobian`도 dense다. stage H를 component-blocked로 바꾸었다고 해서 certificate pipeline 전체의 저장이 O(n s²)가 되는 것은 아니다. 적어도 이 두 입력/중간 행렬에서 2n²개의 f64 slot이 남는다.

`WitnessWork::stored_values = n`은 logical diagonal entries와 실제 dense allocation을 구분하지 못할 수 있다. 기존 결과를 소급 수정하기보다 다음 schema에서 logical nonzeros, allocated slots, peak live slots를 구분하는 것이 맞다. 안전한 최적화는 '아무 Jv나 entrywise bound로 간주'하는 것이 아니라, diagonal/banded 구조를 타입으로 제공하는 witness action이다.

### 3.5 Polynomial·Krylov·RVJ는 서로 다른 계약

`polynomial_action`의 symmetric nonpositive domain을 비정규 연산자까지 조용히 넓혀서는 안 된다. Laguerre의 큰 positive-majorant overestimate는 coefficient 정확도와 다른 문제다. scaling으로 degree를 조금 줄이는 것과 전체 recurrence error를 인증하는 것은 별개다. 우선 유용한 total bound와 비용이 확인된 Chebyshev domain을 보존하고 Laguerre의 error-transfer kernel을 별도 연구하는 편이 낫다.

또한 terminal Arnoldi residual은 output error가 아니다. 이전 R4의 비정규 반례와 이 스레드의 RVJ 내부 증폭 반례는 원인은 다르지만 '작은 residual 또는 local defect만으로 전체 정확성을 승인하지 말라'는 공통 회귀 테스트를 제공한다. 현재 J에 대한 preconditioner/basis reuse와 적분기 자체의 J를 W로 대체하는 것도 분리해야 한다.

## 4. 수학 결과 A: strict-lower stage homotopy 자체에는 일반적인 fold가 없다

F_i(K,lambda)=W K_i-g_i(lambda,K_0,...,K_(i-1))를 생각한다. W는 가역이고 모든 g_i가 필요한 영역에서 매끄럽다고 하자. 각 stage를 앞에서부터 풀면 root가 유일하다. 전체 target Jacobian은 diagonal block이 W인 block-lower-triangular 행렬이므로

    det D_K F = det(W)^s.

따라서 lambda에 따른 lower coupling 또는 nonlinear remainder의 변화만으로 target Jacobian의 fold가 생기지 않는다. 다항식 문제라면 stage root를 순차 대입으로 구성할 수 있고, 국소 formal lifting의 operator 역시 같은 strict-lower 구조를 가진다.

이 명제는 임의의 n과 polynomial degree에 성립한다. 그러나 W가 singular하거나 RHS/closure의 물리 영역을 벗어나는 경우, endpoint에서 무한대로 가는 경우, 또는 native full matrices의 upper leakage를 포함한 다른 target에는 그대로 적용할 수 없다. `StageTarget`의 정확한 구조가 전제다. `homotopy.rs`의 nonstationary theta 변경도 하나의 smooth homotopy라고 다시 해석하지 않는다.

따라서 현재 HOM 연구에 pseudo-arclength를 기본값으로 추가하기보다는 bound width, residual propagation, witness cost를 먼저 다루는 것이 맞다. 실제 algebraic closure G(y,q)=0의 fold는 별개 문제이며 그 경우에는 chart/branch 관리가 필요하다.

## 5. 수학 결과 B: 반경 closure는 볼록 다항식 부등식이다

서로 다른 물리 단위의 성분은 고정 스케일로 정규화한 뒤 공통 반경을 정의해야 한다. 이번 source probe와 스레드의 반례는 무차원 변수다. 현재 quadratic certificate에서 scalar state radius를 D>=0라 하자. nonnegative strict-lower H는

    H(D)=H0+D H1,  H(D)^s=0,
    a_i=U |r_i|,
    E(D)=sum_(k=0)^(s-1) H(D)^k a

이다. H0,H1,a의 성분은 모두 비음수다. 따라서 E_i(D)는 비음수 계수를 갖는 다항식이다. stage-state bound

    p_(i,u)(D)=sum_(j<i) |alpha_ij| E_(j,u)(D)

의 차수는 s-2 이하, 8-stage에서는 최대 6이다. 인증 조건은 모든 (i,u)에 대해 p_(i,u)(D)<=D이다.

각 g(D)=p(D)-D는 D>=0에서 볼록하다. 그러므로 각 feasible set은 구간, 점, 공집합 또는 ray이고 공통 feasible set도 구간이다. 그러나 위로 무한한 구간일 필요는 없다. '더 큰 반경이면 반드시 더 안전하다'는 추론은 틀리다.

예를 들어 p(D)=29/100+(4/5)D²이면 허용 구간은 약 [0.4573,0.7927]이다. 기존 여섯 점 0.001*4^k는 모두 이 구간을 놓치지만 D=1/2는 p(D)=49/100<=1/2를 만족한다. 이것은 일반적인 schedule miss의 정확한 반례다. 실제 R4의 failure 원인은 아래와 같이 더 단순한 상한 부족이다.

`feasible_radius`는 exact rational polynomial에 대해 Sturm root isolation으로 구간을 구한다. 해를 못 찾은 것과 공집합을 증명한 것을 구분하며, 겹침이 isolation 오차보다 작거나 algebraic tangency만 남으면 indeterminate를 반환한다. 찾은 rational point와 binary64로 변환한 point는 각각 원 부등식에 다시 대입한다. 이 CAS 구현은 진단 oracle이며 runtime마다 도입하자는 제안이 아니다.

## 6. 실제 R4 재계산 결과

입력은 `r4_studies::homotopy_cost_study`의 n=1,2,4,8,16, h=0.05, A_ii=-1-i, q_i=-0.05(1+i mod 3), y_i=1+0.1i다. coefficient fixture의 첫 target에서 native binary64 bits와 coupling interval endpoints를 읽었다. J와 후보의 floating input을 고정한 뒤 exact rational interval arithmetic으로 계산했다. U는 exact diagonal inverse의 절댓값이다. 따라서 이 결과는 native Rust의 모든 outward rounding을 재현한 receipt가 아니다.

| n | 현재 여섯 점의 첫 closure | 인증된 허용 구간 내부의 근사 범위 | D=2B 제안 | 새 bound/serial bound 최대 비 |
|---|---:|---:|---:|---:|
|1|0.064|[0.01941475, 75214.7182]|0.0388286691|1.00004474|
|2|0.256|[0.08903161, 20824.5942]|0.1780288685|1.00040679|
|4|1.024|[0.39503538, 9460.50446]|0.7897388962|1.00298297|
|8|실패|[2.29837946, 5868.67370]|4.5752938338|1.01945127|
|16|실패|[14.98650326, 2568.48124]|29.5514507603|1.13907754|

여기서 B=max_(i,u) p_(i,u)(0)이다. B는 현재 candidate의 residual과 H0로 계산하며, reference root 또는 현재 step의 serial error bound에서 가져오지 않았다. D=2B는 데이터별 multiplier tuning을 하지 않는 단순한 posterior proposal이다. 일반적 성공 정리는 아니므로 항상 최종 closure gate가 필요하다. 충분조건으로 max p'_i(2B)<=1/2이면 p_i(2B)<=B+B=2B가 성립한다.

이번 다섯 사례는 binary64로 표현한 D=2B를 exact 부등식에 대입해도 모두 통과했다. interval endpoints도 모든 부등식에 재대입했다. 그러나 native 도입 전에는 기존 directed routines, 실제 witness, residual enclosure, candidate binding으로 다시 검증해야 한다. PastStepData-only policy와 다른 정책이므로 새 opt-in 이름과 receipt를 사용해야 한다.

## 7. 수학 결과 C: 합 행렬을 만들지 않는 action-first doubling

H^s=0이면 L=ceil(log2 s)에 대해

    sum_(k=0)^(s-1) H^k a = product_(ell=0)^(L-1) (I+H^(2^ell)) a.

현재 코드는 합 행렬 S까지 만들면서 S<-S+Q S, Q<-Q²를 계산한다. 제안은

    e=a; Q=H
    for ell=0..L-1:
        e=e+Q e
        if ell+1<L: Q=Q Q

이다. induction으로 ell번째까지 (I+...+H^(2^(ell+1)-1))a를 계산함을 알 수 있다. 모든 stage를 순차로 푸는 것이 아니라 같은 finite polynomial을 다른 evaluation DAG로 계산한다.

비음수 행렬과 벡터에 대해 각 연산을 outward-up으로 수행하면 monotonicity에 의해 결과는 여전히 exact action의 upper bound다. 구조적으로 0인 upper/diagonal entries는 반드시 0으로 유지한다. IEEE 연산 순서가 달라지므로 기존 bound와 bit identity를 주장하지 않는다. 폭이 더 커질 수도 있어 native validation은 exact target inclusion과 실제 admission utility를 함께 검사해야 한다.

8-stage/component에서 같은 skip-zero 규칙으로 reference 산술을 직접 세면 기존 sum-matrix 방식은 216 multiplies+408 adds, action-first는 135+159다. 비율은 0.4711538462다. 두 방식과 독립적인 triangular recurrence가 exact rational로 일치했다. 이 계산에는 H 구축, witness, residual, pool, memory allocation, directed-rounding overhead, big-integer 비용이 포함되지 않는다. 따라서 2.12배 solver speedup이라고 주장할 수 없다.

nondiagonal full H에 적용하면 여전히 matrix squaring이 크다. 현재 가장 작은 안전한 native 적용 범위는 이미 존재하는 diagonal component blocks다. 순차 certificate가 아직 더 싸면 그것을 유지해야 한다.

## 8. RVJ·Darboux·Hensel 연구의 적용 계약

### 8.1 RVJ는 baseline 교체가 아니라 실패 회귀군으로 먼저 반입한다

스레드의 semilinear 반례는 x'=x², y'=(-kappa+2x)y+x², x0=1,y0=1/kappa다. kappa=h^-6, 두 step에서는 (y2-y(2h))/h³ -> 4/3이므로 원래 좌표 RVJ5의 일반 uniform 5차를 지지하지 않는다. 이것은 RODAS5P 자체에 같은 반례가 성립한다는 뜻이 아니다. 새로운 method identity를 부여하고, 스레드의 analytic reference를 실패를 포함해 보존한다.

또한 quintic PR의 RVJ5/RVJ4 estimator effectivity는

    -5(z²-5z+12) / [z(z²-4z+11)] -> 0, z->-infinity.

이번 verifier에서 이 항등식과 극한을 다시 확인했다. 이것을 이유로 RODAS5P의 기존 estimator를 즉시 바꾸는 것은 부당하다. 대신 'stage-target bound, embedded proxy, 실제 ODE local/global error'를 구분하는 공통 테스트 계약을 강화한다.

### 8.2 Darboux chart는 새로운 discrete method다

위 반례에서 C=kappa*y-x², D=kappa*x²는

    L_F C=(-kappa+2x)C,  L_F D=2xD

를 만족한다. w=C/D=y/x²-1/kappa로 놓으면 x'=x²,w'=-kappa*w다. x!=0,kappa>0인 chart에서 정확하며 inverse는 y=x²(w+1/kappa)다. 이번 코드가 세 cofactor/coordinate 항등식을 다시 검사했다.

새로운 좌표에서 적분하면 discrete map이 달라진다. 기존 `StageTarget`의 certificate를 그대로 붙이거나, 기존 RODAS5P driver id로 게시할 수 없다. 새 id, chart domain, forward/inverse identity, physical error map, dense output, nonzero fast initial data를 모두 명시해야 한다.

물리 오차 운반의 직접 경계는 다음이다. |x-xhat|<=Ex, |w-what|<=Ew라면

    |y-yhat| <= (2|xhat|Ex+Ex²)(|what|+1/kappa)
                 + (|xhat|+Ex)² Ew.

이는 곱의 차이를 전개한 결과이며 arbitrary w에 적용된다. 이번 테스트는 여러 kappa와 error-box 점에서 이를 검산했다. tolerance를 좌표변환 전후에 복사하지 말고 이 physical bound를 원래 WRMS scale에 넣어야 한다. 이 경계 자체가 Ex,Ew를 인증해주는 것은 아니다.

### 8.3 Hensel과 homotopy의 위치

정칙 algebraic closure에 대한 formal Hensel lift는 고차 predictor를 동일한 base inverse로 만드는 좋은 경로다. 하지만 현재 vigilode의 homotopy는 주로 nonlinear stage target에 관한 것이지 별도 G(y,q)=0 closure API가 아니다. 둘을 혼동하면 필요한 callback과 model authority가 빠진다.

formal order-doubling은 유한 h의 세 Newton 반복 보장이 아니다. 실제 finite-root certificate, projection critical locus, infinity escape, 물리 branch, inverse bound를 유지해야 한다. 현재 JVP callback만으로 fourth directional derivative나 global derivative bound가 생기지도 않는다. AD tensor materialization 대신 directional jets를 제공하는 별도 provider가 필요하며 비용을 계수해야 한다.

### 8.4 더 안전한 이식: 물리 ODE가 아니라 stage residual의 좌표를 바꾼다

원래 stage target을 R(K)=0이라고 하자. 정칙 chart K=Psi(Z)에 대해 R(Psi(Z))=0을 풀고 K로 복원하면 원래 target의 root는 변하지 않는다. Jacobian은 D_K R D_Z Psi이며, 이 정적인 방정식의 chain rule을 ODE push-forward의 curvature term과 혼동하지 않는다. 이번에는 역함수가 명시적인 polynomial triangular chart로 root 대응과 determinant를 기호적으로 검산했다.

이 방식은 물리 ODE를 좌표변환한 뒤 다른 적분법을 적용하는 것과 달리 원래 RODAS5P target을 유지할 수 있다. 따라서 coordinate/homotopy/Hensel 아이디어를 우선 nonlinear preconditioner 또는 candidate provider로 사용하는 것이 안전하다. 최종 반환 K에 대해서는 여전히 원래 target residual, output projection, binding과 인증 budget을 검사한다. chart에서의 작은 residual만으로 승인하지 않는다.

RVJ가 만드는 endpoint를 8개의 native K stage로 바로 해석할 수는 없다. stage 좌표 의미와 coefficient convention을 보존하는 명시적인 candidate adapter가 필요하다. 이식의 첫 기준은 predictor의 차수 주장이 아니라 현재 target에 대한 실제 인증·비용 결과다.

## 9. 구체적인 다음 개발 우선순위

실행 명세는 `IMPLEMENTATION_PLAN.md`, 의존성·중단 조건은 `REVIEW.json`에 있다.

1. 병렬 certificate probe: action-first와 residual-seeded radius를 연구용 opt-in API로 포팅한다. 기존 순차/native q2는 그대로 둔다. 실제 target 포함, directed overflow, failed radius preservation, workers 1/2/4/8의 deterministic bound를 시험한다.
2. 구조적 certificate pipeline: diagonal Jacobian과 inverse bound를 action으로 유지하고 dense materialization을 hot path에서 제거한다. allocated slots와 nonzeros를 분리한다. capability에서 만든 immutable witness를 같은 attempt의 certification으로 전달하되 모든 identity/model/candidate gate는 보존한다.
3. 작은 문제 fast-v3: stage 수가 아니라 실제 instruction attribution을 먼저 분해한다. 고정 dimension kernel과 static dispatch를 일반 경로와 분리하고, represented-clock/rollback/parity를 그대로 둔다. banded 경로는 저장·pivot scan까지 개선해야 한다.
4. 안전한 연구 이식: RVJ 실패와 chart 성공을 다른 driver id 아래 regression corpus로 만든다. physical error transport부터 닫고, 그 다음 native AD/Krylov·dense output·adaptive controller를 연결한다.
5. 비정규·Laguerre: existing symmetric-only certified scope를 유지한다. metric/semigroup witness, recurrence Green-kernel bound, arithmetic error propagation을 별도의 후보로 연구한다. sampled residual이나 선언된 spectral range를 인증으로 승격하지 않는다.

P>=8이고 W vector solve 비용이 같다는 이상화에서 q1/q2/fallback work는 7-p1+8pf이다. 순차 8과의 차이는 1+p1-8pf다. 예를 들어 p1=0.7,pf=0.2이면 남는 여유는 0.1 W-vector solve뿐이다. certificate/RHS/synchronization을 빼놓은 speedup 주장은 이 작은 여유를 쉽게 소진한다. 반경 closure의 성공보다 이 전체 비용과 실제 target-distance budget을 먼저 확인해야 한다.

## 10. 검증, 미수행, 재현

`python run_review.py`는 네트워크 없이 `pytest`와 exact-real source replay를 실행한다. 최종 54개 테스트가 통과했다. core path-sum/feasibility 기능은 같은 테스트에서 4 failure를 먼저 기록한 뒤 구현하여 green을 얻었다. 나머지 수학 reference 검사는 보충 검산이며 전체를 test-first production 개발이라고 부르지 않는다.

시험 범위는 finite polynomial action, convex radius cases, source-defined 16개 component, 5개 aggregate dimension, midpoint target의 독립 순차 해, chart error transport, RVJ safety identities, arithmetic cost model이다. midpoint root 검산은 interval target의 한 member에 대한 독립 교차검사이며 모든 interval coefficient 조합을 열거했다는 뜻이 아니다. 모든 조합에 대한 포함은 section 5의 majorant induction에 의존한다.

이 runtime에는 cargo/rustc가 없고 direct git network도 사용할 수 없었다. 소스는 GitHub connector로 읽었으며 Rust workspace build, native directed-rounding port, native CI, production speedup을 실행했다고 주장하지 않는다. 이 제한 때문에 제품 코드는 수정하지 않고, 독립 proof/probe와 구체적인 native 포팅 명세를 게시했다.

기존 L-0033, R-JF, coefficient fixture, holdout, timing authority, 과거 ledger는 변경하지 않았다. 최종 publication은 기존 브랜치에 새 폴더만 추가하는 non-force commit이며 정확한 commit은 게시 receipt에서 확인한다. 복구 가능한 source selection과 코드·결과의 SHA-256 manifest를 함께 보관한다.
