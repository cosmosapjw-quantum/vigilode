# A06 — native rank-two direct baseline 확장

A06의 기존 8-stage native RODAS5P 구현은 이미 완료되어 있었다. 이번에는 동일 구현을 다시 제안하지 않고, 실제 코드에서 남아 있던 충분조건 때문에 발생하는 불필요한 거절을 제거하고 모든 stage의 선형계 잔차를 선택적으로 인증하는 경로를 추가했다. 모델·tableau·허용오차·requested output은 변경하지 않았다.

## 기존 근거와 변경 범위

`research/cloud_completion_20261018/STRUCTURED.md`의 WB1, all-stage K-coordinate 동일성 및 O(N) 복잡도 증명을 재사용한다. 해당 독립 감사는 `evidence/structured_review.json`에 있다. 과거 클라우드 allocator positive control 실패는 과거 환경의 기록으로 그대로 보존한다. 이번 실행에서는 positive control과 기존 plain/새 certified 경로의 warmed allocation gate가 실제로 통과했다.

기존 `CollectiveStageFactors::refactor`는 Schur determinant interval이 0을 배제하더라도 WB2의 `s L < a`를 만족하지 않으면 거절했다. 이것은 오류가 아니라 불필요하게 좁은 충분조건이었다. 새 코드는 기존 WB2 분기와 그 계산값을 유지하고, 그 조건 밖에서는 enclosed Schur inverse로 역행렬 노름을 구성한다. 이 확장은 위상·모델을 바꾸거나 tolerance를 늘리지 않는다.

## 정리 A06-S: Schur enclosure에 의한 선형 복잡도 inverse bound

전제는 코드가 검사한다. `carrier=0`, 양의 weights, **표현된 binary64 weights의 정확한 합이 1**, 유한 상태, `s>=0`, `κ>=0`, interval 연산의 유한 결과와 0을 포함하지 않는 Schur determinant가 필요하다. `CollectiveModel` 자체는 근사 정규화를 허용하지만 이 adapter는 더 강한 exact-sum admission을 계속 수행한다.

가중 노름을 `||x||w²=Σj wj |xj|²`라 하자. 고정 Jacobian의 stage matrix는

\[
 A=D-sUCW,\quad D_j=\begin{pmatrix}a&b_j\\-b_j&a\end{pmatrix},
 \quad a=1+s\kappa,\quad b_j=s\omega_j
\]

이다. `U`는 2-vector를 각 mode에 반복하고 `W`는 weighted mean이다. exact sum=1에서 `||U||=||W||=1`이다. `P=D⁻¹`, `H=WPU`, `S₂=I-sHC`라고 놓으면 Woodbury 항등식은

\[
 A^{-1}=P+sPUC S_2^{-1}WP.
\]

`Pj`는 회전배율 행렬이므로 `||P||w <= 1/a = p`이다. cubic mean Jacobian의 singular values는 `|χ||m|²`와 `3|χ||m|²`이므로 `L=3|χ||m|²`를 쓸 수 있다. Schur entries를 enclosure `Sij`로 계산하고 determinant enclosure가 0을 배제하면

\[
 \|S_2^{-1}\|_2\le\|S_2^{-1}\|_F
 \le \frac{\sqrt{\sum_{i,j}\operatorname{mag}(S_{ij})^2}}
 {\inf |\det S_2|}=B_S.
\]

여기서 2×2 adjugate의 Frobenius norm은 원행렬의 Frobenius norm과 같다. 따라서

\[
 \boxed{\|A^{-1}\|_w\le p+s p^2 L B_S}.
\]

모든 numerator는 upward, determinant의 절댓값 하한과 `a`는 downward enclosure에서 사용한다. 코드의 `sqrt_up`, `mul_up`, `add_up`, `div_up` 조합이 위 부등식의 방향을 보존한다. 기존 `sL<a` 영역은 기존 `(a-sL)⁻¹` 경로를 그대로 유지한다. 새 Schur bound가 유한하지 않으면 실패를 반환한다.

독립적인 interval residual `r=b-Ax`에 대해 `||x-x*||w <= B_A ||r||w`이므로 반환하는 `solution_error_upper`는 정확한 frozen mathematical Jacobian의 해 오차를 포함한다. 이 명제는 연속시간 ODE trajectory 오차를 인증하지 않는다.

### 구체적 개선 보장

동일 주파수 0, `κ=0`, real mean, `z=sχ|m|²`인 비선형 계열에서는 `S₂=[[1,z],[-3z,1]]`, `det S₂=1+3z²>0`이다. 기존 gate는 `3|z|>=1` 전체를 거절했다. 새 bound는 유한 산술이 가능한 이 계열에서 이를 허용하며

\[
 B_A\le 1+3|z|\frac{\sqrt{2+10z^2}}{1+3z^2},
\]

로 구성된다. `|z|→∞`의 상한식 극한은 `1+√10`이다. 즉, 이 계열에서는 충분조건 때문에 발생하던 step 감소를 제거하면서 direct stage의 O(N) setup/O(N) per-RHS 복잡도를 유지한다. 실제 적응 오차에 필요한 step 감소까지 제거한다는 주장은 하지 않는다.

## 실제 API 및 비용

- `CollectiveStageFactors::factorize/refactor/solve_into/residual_witness`: 기존 API 유지. Schur-inverse overflow를 명시적으로 거절한다.
- `CollectiveRodas5pWorkspace::step_into_with_residual_witnesses`: 동일한 8개 native stage를 계산하고 각 stage마다 O(N) interval residual pass를 수행한다.
- `stage_residual_witness(i)`: 마지막 성공한 certified attempt의 witness만 반환한다. 실패 또는 다음 plain attempt가 stale witness를 제거한다.
- 기존 `step_into`는 interval residual 추가비용을 지불하지 않는다. 새 certified path와 stage/endpoint/embedded error는 bitwise 동일하다.
- `CollectiveDirectWork`에 `inverse_certificate_ops`, `residual_witness_calls`를 추가했다. 전자는 구조적 작업량 proxy이며 실측 flop이 아니다. 후자는 실제 witness 호출 수다.
- comparison JSON은 이전에 버리던 structured setup/Schur/inverse/reconstruction counters도 보존한다. 기존 timing 범위는 setup+integration+output 전체다. 이 numerical baseline에서는 opt-in stage residual 인증을 사용하지 않았다는 필드도 명시한다.

총 stage 수 8이 고정되므로 setup O(N), 각 solve O(N), 각 opt-in witness O(N), 전체 attempt O(N), workspace O(N)이다. 기존 dense factorization O(N³)를 대신하는 조건부 복잡도 이득을 재사용한다. elapsed speed는 A07의 matched-accuracy campaign으로만 판단한다.

## 실행 결과 및 적대적 fixture

`evidence/native_tests.log`: 3개 new-domain contract, 5개 inherited all-stage/trajectory contract, 6개 direct-stage contract **14/14 PASS**. `evidence/allocation_test.log`: 알려진 clone allocation의 positive control을 먼저 통과한 뒤 plain 10 attempts와 certified 10 attempts 모두 warmed allocation 0을 검사하여 **1/1 PASS**.

새 exact-dyadic fixture는 N=1,2,4,8,64,256에서 `sL=3>1`인 동일 frozen 선형계의 해를 독립적인 2×2 exact 식으로 만든다. candidate 한 성분을 `next_up`으로 변형해도 residual-based error bound가 참 오차를 포함하며 허용 slack을 추가하지 않는다. N=4의 `ω=-2`, mean=(1,0), `χ=1`, `s=1`은 정확한 singular Schur 사례이고, 거절 후 old factors와 output을 재사용하지 못한다. 새로운 certified workspace는 N=1,2,4,8,64에서 plain workspace와 모든 stage가 동일하다.

첫 실행은 singular test fixture가 `xi=-2`로 모델 admission 범위를 벗어나 실패했다. `width=2, xi=-1`로 같은 물리 주파수를 표현하도록 fixture를 수정했다. solver나 tolerance는 바꾸지 않았고 최초 로그 `native_tests_attempt_01.log`를 보존했다.

## 판정 및 다음 단계

A06 numerical implementation과 explicit frozen-linear-system certificate는 scoped 구현 완료다. 독립 delta review는 root 통합 단계에서 source hash와 함께 결합한다. 이 노드만으로 global ODE accuracy나 production speed를 승격하지 않는다.

1. A07에서 최종 source를 고정한 bounded campaign을 실행하고 새 work 필드까지 보존한다.
2. 지역 실행에서는 동일 native tests/allocator gate를 실행하고 최종 hardware timing만 수행한다. 새 핵심 기능 구현은 필요하지 않다.
3. 향후 exact-sum 이외의 모델까지 adapter를 확장할 때는 `||U||||W||=S`를 bound에 포함하고 실제 S의 outward sum으로 WB2 및 Schur inverse 항을 변경해야 한다. 현재는 이를 명시적으로 거절한다.
