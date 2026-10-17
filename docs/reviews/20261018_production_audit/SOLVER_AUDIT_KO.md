# 최신 비-packet solver 독립 감사

검토 대상은 baseline `76e59c691845851274cf27b26678cdb128daa408` 이후 구현 HEAD `2fc61b944fe431d40ba455c373579bce00b0c39a`의 SP02/SP04, AS04/AS05, PY01–PY05, PC01, HT01/HT02, EX01이다. 기존 캠페인은 재실행하지 않았다. 새로운 trust-boundary 입력과 변경된 코드의 소규모 테스트를 실행했다. 실행 도구는 Rust 1.94.1, 저장소 Cargo.lock, 제공된 offline vendor이다.

판정은 **범용 production 및 전역 오차 인증 승격 HOLD**이다. 이미 기록된 SP02 workspace 절감, AS04 telemetry 일치, AS05 제한된 선형 residual-gain 인증은 각 원래 범위로 유지한다. SP04 기본정책 승격 실패, PY02 ABSTAIN, PY03/PY05 속도 gate 실패, HT01 ABSTAIN/HT02 미실행은 source/기록과 일치한다. 이 결과들 자체를 새 범용 solver 성능의 증거로 확장하지 않는다.

## 새로 확인한 결함과 수정

### SA-01 — public Leja cache로 거짓 Certified 발급 (P0, 해당 opt-in API)

원본 `CertifiedLejaCoeffCache::insert_certified`는 외부에서 임의 coefficient/radius 배열을 저장하게 했다. A=diag(-1,-2), h=1, w=(1,1), exp 항만 켠 입력에서 정상 호출로 얻은 center/scale의 cache를 모든 0으로 덮으면 실제 native 실행은 `Certified { bound: 1.3435695494301256e-7 }`, 출력 `(0,0)`을 반환했다. 정확한 exp(A)w와의 오차는 `0.3919833186825007`이다.

수정: 배열 삽입을 private으로 제한하고, 공개 precompute API가 내부 outward coefficient engine을 호출하게 했다. 기존 profiler도 새 API를 쓴다. 외부 삽입 compile-fail doctest 통과. 원본 probe는 감사 evidence에 보존하고 현행 build target에서는 제거했다.

### SA-02 — Leja recurrence가 prefix bound를 suffix bound로 사용 (P1)

실제 Leja nodes 첫 세 개는 `(2,-2,1.2246467991473532e-16)`이다. 기존 prefix 두-node bound는 `4.0625`이지만 ξ=2에서 필요한 suffix `(ξ−ξ1)(ξ−ξ2)`는 `7.999999999999999`다. T3의 이 대입은 문서의 올바른 Newton tail polynomial 식을 구현하지 않는다. 이 반례는 propagation component의 일반 상계에 대한 것이며 정상 입력 전체 출력의 실제 위반을 임의로 주장하지 않는다.

수정: 모든 연속 node 구간의 product를 [-2,2]의 128개 구간에서 directed interval arithmetic으로 한 번 상계하고 cache한다. T3는 올바른 suffix 인덱스를 사용한다. 독립 검토 IR-01이 지적한 coefficient radius와 recurrence error의 교차항도 포함했다. 인증에 쓰이는 l2 norm은 nearest-rounded safe_l2에서 scaled outward l2로 바꿨다.

남은 실제 의무: recurrence/accumulation의 상대오차 모델은 unrestricted binary64의 additive underflow를 아직 상계하지 않는다. 따라서 전체 공개 Leja total authority를 `EstimateOnly`로 내려 `LEJA_ARITHMETIC_HOLD`를 명시했다. 수치 결과와 component sum, convergence budget은 유지한다. 추후 underflow를 포함한 전체 local residual enclosure를 구현하면 새 변경 범위만 재검증한다.

### SA-03 — 새 sparse direct API의 pattern/state validation (P1)

원본 actual native probe에서 out-of-range CSC row는 `new_checked` panic, numeric factorization 전 solve는 faer의 symbolic/numeric mismatch panic, 같은 길이의 다른 pattern은 refactorize에서 Ok였다. release profile은 panic=abort라 입력 오류가 호출자 오류 처리로 전달되지 않는다.

수정: canonical CSC validation, 원래 column/row pattern 보존 및 일치 검사, 성공한 현재 numeric factorization 상태, 실패한 refactorize 이후 상태 invalidation을 추가했다. NaN/Inf RHS와 출력도 오류 처리한다. 새 테스트는 invalid row, nonmonotone pointer, unsorted/duplicate row, zero/overflow dimension, 미인수분해 solve, pattern 변경과 실패 후 stale solve를 다룬다. 작은 정상 대각 solve가 `(1,2)`를 반환함도 확인했다. 변경으로 pattern 저장 O(n+nnz) 메모리와 O(n+nnz) 비교 비용이 추가된다.

### SA-04 — linear split verification NaN 통과 (P1)

원본 `SparseLinearPart::new`는 out-of-range row를 받아들였고, `verify_linear_part`는 nonlinear callback이 NaN을 출력해도 성공했다. 원인은 `rel_err > tol`가 NaN에서 false가 되는 조건식이다. native reproduction 두 항목 모두 true였다.

수정: public field를 통한 descriptor 변경까지 validate, matvec shape/finite check 및 CoreResult 반환, with_linear_part 재검증, verification input과 relative error finite check를 추가했다. 이 2점 검사는 선언된 split의 smoke check이다. opaque callback이 전체 상태영역에서 동일하다는 구조적 증명으로 승격하지 않는다.

### SA-05 — Gray–Scott N=2 periodic stencil의 중복 이웃 계수 누락 (P1)

공개 constructor는 N>=2를 허용하지만 N=2에서 좌/우와 위/아래가 같은 이웃이다. full RHS는 두 항을 모두 세며 CSC linear part는 duplicate를 버려 한 번만 센다. 새 N=2 native split test에서 relative mismatch `0.03199997018677811` > `1e-12`를 관측했다.

수정: 네 이웃을 모두 넣고 같은 row의 계수를 합쳐 canonical CSC로 저장한다. 같은 테스트 통과. N>=3에서는 이웃이 서로 달라 같은 stencil이 유지된다.

### SA-06 — EX01 generic semilinear logarithmic norm 전제 누락 (P1, 이론/승격)

`DEFECT_BOUND.md`는 일반 J_g에 대해 μ2(A+J_g)≤μ2(A)+max diag(J_g)를 사용한다. off-diagonal Jacobian에서는 성립하지 않는다. A=0, J_g=[[-1,100],[0,-1]]이면 max diag=-1인데 μ2(J_g)=49이다. g(y)=J_g y+(0,1), y0=0, 근사궤적 0에 대해 defect integral은 t이며 정확한 해는 `(100[1−(1+t)e^{−t}], 1−e^{−t})`다. t=.1의 오차는 약 .477을 넘어서 unweighted bound .1을 위반한다. 원래 EX01 Stage0 F1의 generic rigorous-bound 승격은 정정 대상이다. 기존 HOLD(rival) 외에 수학 전제와 실제 native enclosure 구현 의무를 기록해야 한다. 기존 문서와 로그는 provenance로 보존한다.

## 구성적인 이론–코드 연결

### S-T1. 올바른 Newton tail 상계와 스칼라 계산 감소

X symmetric, spec(X)⊂[-2,2]이고 computed recurrence가 rhat_{l+1}=(X−ξ_l I)rhat_l+δ_{l+1}라고 하자. coefficient enclosure |d*_l−dmid_l|≤rad_l가 주어졌을 때

Q_{i+1,j}(X)=Σ_{l=i+1}^j d*_l ∏_{r=i+1}^{l−1}(X−ξ_r I).

B[a][b]≥max_{x∈[-2,2]}∏_{r=a}^{b−1}|x−ξ_r|를 outward interval subdivision으로 만들면 spectral theorem과 triangle inequality로

T_{i,j}=Σ_{l=i+1}^j (|dmid_l|+rad_l)B[i+1][l] ≥ ||Q_{i+1,j}(X)||2.

따라서 recurrence error 기여는 Σ_i T_{i,j}||δ_{i+1}||2 이하다. coefficient error는 Σ_l rad_l||rhat_l||2로 더한다. 정확한 분해 `d* r*−dmid rhat=(d*−dmid)rhat+d*(r*−rhat)`가 교차항 포함을 보장한다.

각 j에서 `T_{i,j}=T_{i,j−1}+(|dmid_j|+rad_j)B[i+1][j]`를 저장하므로 degree m까지 coefficient-tail 갱신 수는 m(m+1)/2이다. 종전 재합산은 m(m+1)(m+2)/6이었다. 계수 열 수 5는 양쪽 공통이다. 이 항은 **O(m³)에서 O(m²)**로 감소한다. B의 한 번 생성은 S=128에 대해 O(Sm²), cache는 O(m²), 전체 recurrence에는 기존 matvec와 다른 인증 비용이 남는다. 이것은 연산수 보장이고 wall-time 배수는 아니다. 전체 binary64 certificate는 별도 additive-underflow 의무 때문에 HOLD다.

### S-T2. EX01의 일반적이고 유효한 defect 수송

정확해와 근사해를 잇는 모든 선분이 검증된 tube에 있고, 그 위에서 μ2(A+J_g(t,y))≤ω(t), ||d(t)||≤D(t)이면 Dini derivative comparison으로

||e(t)|| ≤ exp(∫0^t ω) ||e(0)|| + ∫0^t exp(∫s^t ω)D(s)ds.

ω≤0에서만 defect의 비가중 적분을 사용한다. componentwise g처럼 J_g가 diagonal일 때 max diag 대입은 유효하다. Gray–Scott의 spatial diffusion은 주기격자에서 symmetric negative semidefinite이고 reaction은 각 grid point의 2×2 block이다. block [[a,b],[c,d]]의 logarithmic norm은 `(a+d)/2 + sqrt(((a−d)/2)^2+((b+c)/2)^2)`다. 따라서 interval tube에서 각 block의 outward 최대를 취하는 O(n) bound로 full-space error transport를 구현할 수 있다. dense eigen-decomposition이 필요한 구조가 아니다. 필요 구현: 상태 tube, reaction entry interval, outward sqrt, continuous defect, inherited uncertainty, step/window composition.

## 실행 근거와 다음 단계

새 core sparse 테스트 3개, split 테스트 4개, Leja unit 테스트 3개, 외부 cache 삽입 compile-fail 1개 통과했다. 임시 편집 시점의 ambiguous-float build error는 `Vec<Vec<f64>>` type annotation으로 수정했고 후속 빌드 통과했다. 원래 과거 캠페인을 새 결과로 재명명하지 않았다. root 통합 regression 결과는 최종 패키지의 실행 기록을 따른다.

1. 현재 narrow fixes를 독립 검토와 함께 병합하고 Leja 전체 authority HOLD를 유지한다.
2. Leja의 실제 recurrence local residual을 exact represented matrix/vector로 interval 평가하거나 additive underflow 항이 포함된 모델로 교체한다. q/stencil shape에 맞는 directed norm과 output accumulation까지 닫는다. 수치 budget을 변경하지 않는다.
3. Gray–Scott generated descriptor로 split을 구조적으로 구성하고 spatial full-tube lognorm을 O(n)으로 평가한다. 2점 callback 확인을 structural certificate로 사용하지 않는다.
4. AS05 선형 residual gain + AS04 실제 stage telemetry를 RODAS stage propagation 및 global transport와 연결한다. 현재 linear solve bound를 곧바로 전체 trajectory error라 부르지 않는다.
5. SP02 opt-in workspace는 기존 조건의 이미 관측된 instruction/allocation 절감을 재사용한다. Leja suffix setup/cache·norm·인증 작업을 cold/warm total cost로 포함해 이후 정확도가 같은 작은 비교만 수행한다.

통합 검증에서 HOLD reason String 생성이 warmed 호출당 allocation 1회를 추가한 것을 발견했다. 기존 0-allocation 기준을 유지하고 report의 reason buffer를 재사용하도록 수정했다. 해당 warm allocation test와 EstimateOnly authority test를 다시 실행해 통과했다. `leja_warm_regression.log`에 최종 결과가 있다.
