# 원래 단계 방정식의 오차 예산을 적응 제어에 연결하기

이번 변경은 기존 `causal_majorant.rs`의 유한 경로합을 재사용하여 **단계 잔차 → 원래 물리 출력 오차 → 후보 채택**을 연결하는 API를 추가한다. `causal_budget.rs`는 호출자 버퍼를 사용하며 성공 수치 경로에서 메모리를 할당하거나 RHS·Jacobian·JVP·Newton 콜백을 호출하지 않는다. 모델별 상계는 호출자가 제공한다. 예제에서는 실제 DOPRI5 tableau와 원래 비선형 방정식을 사용해 상계를 구성한다.

## 수학적 계약과 재사용

목표 단계 방정식을 \(W_i u_i=b_i(u_0,\ldots,u_{i-1})\)로 쓴다. 고정 노름에서 \(\|W_i^{-1}\|\le M_i\)이고, 정확한 단계와 후보 단계를 포함하는 영역에서

\[\|b_i(v)-b_i(w)\|\le\sum_{j<i}L_{ij}\|v_j-w_j\|\]

를 가정한다. 후보의 **원래 목표 방정식** 잔차 \(\rho_i\)에는 부동소수점 연산과 RHS 평가 오차를 포함한다. \(d_i\ge M_i\rho_i\), \(B_{ij}\ge M_iL_{ij}\)이면

\[ e_i=d_i+\sum_{j<i}B_{ij}e_j,\qquad \|u_i-\widehat u_i\|\le e_i.\]

증명은 단계 순서의 귀납이다. 첫 단계에서는 앞선 단계가 없어 역연산자 잔차 상계가 곧 결론이다. 앞선 단계의 결론을 다음 단계의 Lipschitz 부등식에 대입한다. \(B^s=0\)이므로 \(e=(I-B)^{-1}d=\sum_{k=0}^{s-1}B^kd\)이다. **\(\|B\|<1\)은 요구하지 않는다.** 아래 대각에 2가 있는 네 단계와 \(d_i=1/8\)이면 상계는 \((1,3,7,15)/8\)이다. 작은 단계 잔차가 큰 출력 오차로 전파되는 경우를 잡아낸다.

이 결과와 upward 연산은 기존 `crates/rodas5p-integrators/src/causal_majorant.rs`의 `upper_causal_solve`에 구현되어 있다. 이번 기여는 별도의 새 정리로 명명하는 대신 원래 목표·노름·좌표·스텝 identity와 출력 예산을 연결하고 O(s) 호출자 버퍼로 계산하는 것이다. s×s 배열은 단계 간 **스칼라 상계**이며 상태 차원의 Jacobian을 구성하지 않는다.

RODAS raw-U 단계에는 \(L_{ij}\le |h\gamma|L_f|a_{ij}|+|\gamma c_{ij}|\)를 쓸 수 있다. 이때 \(M_i\)의 유효한 상계가 필요하다. 기존 projected causal 후보의 기저 구성에는 base JVP와 작은 LU가 남아 있으므로 그 전체 경로를 완전 무미분으로 분류하지 않는다. RHS-only RK와 선언된 cofactor/quotient 경로는 해당 미분 호출을 요구하지 않는다.

## 출력 영향도와 내부 잔차 배분

출력 재구성의 단계별 Lipschitz 계수를 \(w_i\ge0\)로 두면 출력 기여는 \(w^Te\) 이하이다. 역방향 recurrence

\[\sigma_i=w_i+\sum_{j>i}B_{ji}\sigma_j\]

는 \(w^Te=\sigma^Td\)를 준다. 양의 영향도가 m개일 때 \(d_i\le\tau/(m\sigma_i)\)로 배분하면 원래 출력의 내부 오차 기여가 \(\tau\) 이하이다. `causal_residual_allowances`는 영향도를 위로, allowance를 아래로 반올림한다. 배분 대상은 bare residual \(\rho_i\)가 아니라 \(M_i\rho_i\)이다. 영향도 0인 단계에는 infinity 대신 0을 돌려준다. 그 단계가 별도 embedded·dense 출력에도 영향이 없다는 뜻은 아니다.

후속 연구에서는 실제 단계별 refinement 비용을 배분식에 반영할 수 있다. 비용 모형 \(a_i\log(D_i/d_i)\), \(a_i>0\)와 비포화 조건에서 제약 \(\sum\sigma_i d_i\le\tau\) 아래 최적 배분은 \(d_i=\tau a_i/(\sigma_i\sum_j a_j)\)이다. 비용 가중 배분을 측정 없이 기본값으로 넣지는 않았다.

## 적응 제어와 연결

`CausalTargetKey`는 model/provider/norm/tolerance epoch, chart/operator/state epoch, 정확히 표현된 t와 h를 묶는다. h·좌표·노름·callback binding이 바뀌면 기존 잔차와 상계를 재사용하지 않는다. lambda가 정확히 1인 원래 목표 평가만 채택된다. homotopy의 쉬운 중간 문제에서 얻은 작은 잔차가 원래 문제의 성공으로 전달되지 않는다.

RK에서는 주 출력 b와 embedded 차이 \(b-\widehat b\)를 함께 제어한다. 고정 절대 노름에서 \(w_i=|b_i|+|b_i-\widehat b_i|\)를 쓰고 두 reconstruction 오차도 합하면 principal perturbation과 embedded estimate 교정의 **합**을 제한한다. 예제는 계산한 embedded 차이에 이 상계를 더한다. 이는 정확한 단계에 비해 조기 종료가 판단을 얼마나 바꾸는지 제어하며, embedded estimator 자체를 전역 ODE 오차 인증으로 변경하지 않는다. 상태 의존 WRMS에는 공통 물리 scale 또는 검증된 하한이 필요하다. 첫 초안의 max 가중치는 두 채널을 각각 제한했으나 합산 budget을 보장하지 않아 독립 검토 후 sum으로 교정했다.

## 실제 pilot

`causal_budget_pilot`은 \(y'=-y^3\), \(y(0)=1\), \(t\in[0,1]\)을 \(h=1/256\)으로 계산한다. 공개 DOPRI5 tableau를 그대로 사용한다. lambda=0 무결합 predictor에서 시작하여 lambda=1 causal Jacobi correction을 수행한다. 한 라운드의 단계는 앞 라운드 값만 읽는다. 이번 실행은 직렬 라운드 계산이며 thread speedup 측정이 아니다.

원래 잔차는 별도의 interval cube 평가로 계산한다. outward 연산으로 \(|y|+8h\max_i\sum_j|a_{ij}|<2\)를 확인하면 모든 정확한 단계 입력이 \([-2,2]\)에 있고 \(|f|\le8\), \(L_f\le12\)이다. 후보도 같은 영역인지 확인한다. stage output 대조군은 매 스텝 `Dopri5Workspace::step_into`로 독립 실행하고 ODE trajectory는 해석해 \(1/\sqrt{1+2t}\)와 비교한다.

같은 tableau·mesh·입력에서 두 정책을 비교한다. 첫 정책은 출력 perturbation 1e-9와 embedded-plus-bound 1e-8 기준이 성립하면 종료한다. 둘째는 추가로 모든 원래 잔차를 1e-14 이하로 줄인다. 후보 RHS, interval verifier RHS, correction rounds, 직접 순차 RK oracle RHS를 분리 기록한다. 작은 스칼라 문제에서는 직접 순차 RK가 유리할 수 있고, 자동 선택기는 그 결과를 따라야 한다. 결과는 불필요한 root polishing 비용을 줄이는 근거로 사용하며 순차 RK 대비 무조건적인 가속으로 사용하지 않는다.

## 다음 이식 단계

* coupled candidate의 operational residual에 검증된 inverse/Lipschitz/physical transport 상계를 연결한다.
* quotient 좌표 변경과 h 변경 시 physical norm·chart epoch와 budget을 같이 갱신한다.
* 내부 오차 예산을 통과한 후보 사이에서만 stage-parallel 비용을 비교한다. residual 검사·재구성·pool/setup·실패한 homotopy 라운드를 모두 포함한다.
* 비싼 RHS에서만 stage-parallel pilot을 요청한다. 저차원·저비용 RHS는 순차 경로를 유지한다.

### 다음 노드: 수렴한 causal prefix의 재평가 생략

기존 `crates/vigilode-coupled/src/causal.rs::picard`는 각 라운드에 7개 새 RHS를 예약하고 8개 stage task를 다시 실행한다. strict-lower 의존성은 이미 고정된 앞 단계의 재평가를 줄일 수 있게 한다. 단계 0의 입력은 고정이고, 동시 업데이트를 한 번 수행할 때마다 정확한 target과 일치하는 prefix가 한 단계 이상 확장된다. 같은 부동소수점 계산 순서와 deterministic callback을 유지하면 선행 입력 비트가 변하지 않는 단계의 RHS·projected remainder·interval target은 재사용할 수 있다. 이는 contraction에 의존하지 않는 유한 인과성의 귀결이다.

첫 이식에서는 **iteration 번호만 믿고 생략하지 않고**, cached dependency prefix의 실제 비트 동일성과 model/provider/operator/state/t/h epoch를 확인해야 한다. 원래 목표 residual을 별도 생략하는 것이 아니라, 동일한 입력에서 검증한 residual target만 재사용한다. 후보 stage 값이 변했으면 그 값과 cached target의 차이는 다시 계산한다. 영역·노름·h 또는 callback version이 바뀌면 cache를 폐기한다. 외부 side effect를 가진 callback에는 이 최적화를 적용하지 않는다.

검증 계획은 다음과 같다.

1. full-batch baseline과 suffix-only 후보에 같은 basis·root initial guess·model·표현된 h를 공급한다.
2. 각 라운드 stage vector, 최종 original-space residual, 출력 및 종료 사유의 비트 동일성을 검사한다.
3. 실제 수행한 RHS/task만 work counter와 예약 예산에 반영한다. 캐시 적중은 별도 기록하며 수행한 연산으로 더하지 않는다.
4. nonlinear stiff·oscillatory·changing-rate holdout에서 prefix가 늦게 고정되는 경우와 epoch 변경 거절을 검사한다.
5. 비트 parity를 통과한 뒤에만 charged 비용을 비교한다. 이상적인 7-stage fresh RHS 수열이 7,6,5,…로 줄어도 작은 문제에서는 직접 forward sweep이 여전히 유리할 수 있다.

이번 실행에는 이 추가 최적화를 넣지 않았다. 새 예산 API와 native pilot의 실행 identity를 고정하여 그 결과를 먼저 검증한다.

새 시험은 noncontractive amplification, adjoint allowance, 중간 lambda, 9종 context 변경, 잘못된 삼각 구조, reconstruction 오차, underflow와 overflow를 다룬다. 실제 실행 명령과 결과는 상위 evidence에 연결된다.
