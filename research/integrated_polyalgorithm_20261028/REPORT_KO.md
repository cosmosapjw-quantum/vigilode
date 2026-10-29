# VigilODE: 누적 PR의 통합 이론과 독립 검증 설계

2026-10-28 KST. 대상 저장소: `cosmosapjw-quantum/vigilode`.

## 1. 결론과 이번 작업의 범위

PR90–93의 결과를 **공통 물리 오차 계약 → 구조에 따른 계산 계획 → 같은 원래 문제의 승인 → 완전한 session의 지속**이라는 흐름으로 재구성했다. 핵심은 후보 수를 늘리는 것 자체가 아니다. 동일한 물리 요청에 대해 서로 다른 계산법이 제공하는 증명과 비용을 비교하고, 승인된 결과와 다음 계산에 필요한 상태를 함께 보존하는 것이다.

이번 연구에서 추가한 구성은 다음과 같다.

1. 불연속·history-dependent selector에도 적용되는 물리 오차 전파와, 혼합해서는 안 되는 두 induction anchor를 정리했다.
2. 선언된 저차 연산자 관계가 있는 affine causal Rosenbrock 계에서 **전체 stage 식을 scalar quotient algebra로 컴파일**하고, 필요한 endpoint만 평가하는 방법을 유도했다.
3. 생략한 stage를 실제로 만들지 않아도 원래 stage 방정식의 잔차를 감쌀 수 있는 near-structure·저장 계수·RHS·비선형 remainder의 합성식을 유도했다.
4. 반복 refinement의 rounding allowance로부터 floor와 충분한 최소 반복 수를 도출했다.
5. 동일한 continuation state 안에서만 가능한 Pareto pruning, 완전한 probe session 유지, 엄밀한 비용 하한에 따른 후보 중단을 하나의 실행 계약으로 연결했다.

두 이론 lane의 소스를 먼저 remote commit `af7f3e9f55d652554042d2386bf07abb7d857d75`에 게시하고 exact rational check를 실행했다. 두 프로그램 모두 exit 0, `PASS`였다. error lane은 6개 묶음, algebra lane은 9개 compiler fixture와 refinement·거부 조건을 검사했다. 이는 일반 증명을 지지하는 유한 검산이며 native binary64 구현, solver 성능, 전체 release qualification의 통과를 뜻하지 않는다. 현재 PR은 runtime source를 변경하지 않는다. native 코딩·검증은 사용자가 요청한 **후속 독립 PR**의 실행 대상으로 구체화했다.

전체 유도는 [error_contract/THEORY.md](error_contract/THEORY.md), [algebra_contract/THEORY.md](algebra_contract/THEORY.md), [EXECUTION_CONTRACT.md](EXECUTION_CONTRACT.md)에 있다. 수치 출력과 source-bound receipt는 `evidence/`에 보존했다.

## 2. 앞선 PR에서 가져오는 내용과 통합 지점

| PR | 보존할 성과 | 이번 통합에서의 역할 |
|---|---|---|
| [90](https://github.com/cosmosapjw-quantum/vigilode/pull/90) | LU workspace, borrowed stage rows, mode work 감소, comparator/return evidence | 저장 공간의 재사용과 수학적 값의 재사용을 분리하는 기준; 실제 mode 비용의 baseline |
| [91](https://github.com/cosmosapjw-quantum/vigilode/pull/91) | causal reuse, structural mean-field 원래 raw-U 해석, RHS homotopy, exact constants, checkpoint seed | causal 오차 전달과 구조적 연산자 모델의 출발점; seed는 승인 근거가 아닌 proposal |
| [92](https://github.com/cosmosapjw-quantum/vigilode/pull/92) | original residual의 표현오차 enclosure, scalar/algebraic/rational cubic 후보, strict integer work 경계, production configuration, 실패 work 보정 | 모든 새 후보가 지켜야 할 승인·권한·실패·출력 계약 |
| [93](https://github.com/cosmosapjw-quantum/vigilode/pull/93) | fused polynomial, Chebyshev/Laguerre 및 nonnormal 대안, finite-step cubic, geometric/causal 및 residence 조사 | 공통 요청에 속하는 후보의 수학적 전제와 대안 목록; native 활성화는 아직 없음 |

이 네 source는 한 ancestry chain이다. 정확한 SHA와 PR metadata는 `PR_SCOPE.json`, ancestry 검사는 `validation_design/READ_ONLY_CHECKS.json`에 기록했다. 이번 대화의 runtime/workflow 변화는 pre-conversation baseline에서 PR93까지 **73개 고유 경로**다. PR별 경로 수 13, 20, 44, 0은 겹침을 포함하므로 합계가 고유 경로 수와 다르다. `validation_design/COVERAGE.json`은 각 경로의 SHA-256과 필요한 gate를 연결한다.

예전 PR의 제목에 production이라는 표현이 있어도 repository 전체의 모든 gate가 통과했다는 뜻으로 확대하지 않는다. 기존 보고서의 제한된 조건에서의 결과, 현재 누적 소스의 정확도, 현재 환경에서의 성능은 서로 다른 주장이다.

## 3. 공통 물리 요청과 정확도 정리

### 3.1 Selector를 미분할 필요가 없는 합성

원래 ODE의 유일한 flow를 \(\Phi(t_{n+1},t_n,\cdot)\)라 하자. 같은 norm과 유효한 domain에서

\[
\|\Phi(t_{n+1},t_n,x)-\Phi(t_{n+1},t_n,z)\|\le\Lambda_n\|x-z\|
\]

가 성립하고, 실제 선택된 후보가 같은 numerical input에서

\[
\|\widehat y_{n+1}-\Phi(t_{n+1},t_n,\widehat y_n)\|\le d_n
\]

를 만족하면 outward recurrence

\[
E_{n+1}\ge\Lambda_nE_n+d_n
\]

는 실제 accepted grid 위의 물리 오차를 감싼다. 증명은 삼각부등식과 induction이다. 선택 함수가 불연속이거나 과거 관측에 의존해도, **실제로 선택한 후보마다 같은 원래 flow에 대한 조건을 만족**하면 충분하다. 실패 후보는 accepted physical state를 바꾸지 않아야 한다.

이 정리는 termination, bounded work, timestep controller 안정성까지 보장하지 않는다. Ranocha–Giesselmann의 원전도 residual 기반 오차 평가와 controller 안정성을 별도 문제로 다룬다. Polynomial reconstruction \(v\)의 residual \(v'-f(t,v)\)은 RHS만으로 계산할 수 있지만, 그 적분 enclosure와 one-sided Lipschitz/tube 정보는 별도의 근거가 필요하다. reconstruction의 미분은 알려진 polynomial의 미분이며 숨겨진 Jacobian callback을 허용하는 근거가 아니다. [RG2024](https://link.springer.com/article/10.1007/s44207-024-00001-0)

### 3.2 두 induction anchor를 혼합하면 생기는 오류

선택된 고정 discrete recipe를 \(\Psi_n\), 실제 구현의 같은-input algebraic error를 \(a_n\)이라 하자. 유효한 두 경로는 다음과 같다.

\[
\begin{aligned}
\text{Flow anchor:}\quad E_{n+1}&\le\Lambda_nE_n+a_n+\tau_n^{\rm num},\\
\text{Discrete anchor:}\quad E_{n+1}&\le L_nE_n+a_n+\tau_n^{\rm exact}.
\end{aligned}
\]

첫 식의 temporal defect는 numerical input에서 측정한다. 두 번째는 exact trajectory input에서 측정하고 **같은 discrete recipe의** 안정성 \(L_n\)을 사용한다. 비교 중 tableau/carrier/chart를 exact input에 맞춰 조용히 재생성하면 같은 map의 비교가 아니다.

반례는 \(\Phi(x)=x/2\), \(\Psi(x)=x\), \(y_n=0\), \(\widehat y_n=e>0\)이다. 실제 다음 오차는 \(e\), exact-input truncation은 0이다. 이 truncation에 flow contraction \(1/2\)를 붙이면 거짓 상계 \(e/2\)가 된다. 올바른 두 경로는 모두 \(e\)를 준다. 검산은 \(e=1/5\)에서 실제/정상 상계 \(1/5\), 잘못된 상계 \(1/10\)을 확인했다.

PR93의 cubic 증명은 exact-state decay truncation과 midpoint nonexpansiveness \(L_n=1\)을 결합하므로 이 반례의 오류에 해당하지 않는다. 이번 정리는 그 증명을 폐기하지 않고, 새 polynomial/structural certificate와 연결할 때 유지해야 하는 anchor를 명시한다.

### 3.3 Norm·representation·clock의 오차 소유권

각 certificate는 이름을 붙인 두 exact intermediate object 사이의 edge를 감싼다. 해당 edge들을 한 번씩 합성한다. PR92 structural residual이 이미 RHS·mean·lift 오차를 포함한다면 같은 항을 다시 미포함 오차로 청구하지 않는다. 반대로 endpoint accumulation처럼 그 residual 밖에 있는 항을 빠뜨려서는 안 된다.

Clock은 일관된 두 방식 중 하나를 선택한다. Stored physical time labels를 기준으로 하면 실제 engine step과 label 간격의 차이가 local defect다. Exact internal clock \(\tau_n\)를 유지하는 경우에는 출력 시

\[
E_n^{\rm obs}\le E_n^{\rm int}+M_n|t_n-\tau_n|
\]

를 적용한다. 관측에만 해당하는 label 오차를 매 internal step에 반복 주입하면 필요 이상의 누적 예산을 소모한다. 비자율 문제의 시간 이동은 RHS의 시간 regularity가 필요하며 timestamp 두 개의 ULP만으로 전체 효과를 증명할 수 없다.

## 4. 전체 affine causal 계를 quotient algebra로 컴파일

### 4.1 원래 stage 방정식과 추가 전제

원래 raw-U 계를

\[
W U_i=\tau f\!\left(y+\sum_{j<i}a_{ij}U_j\right)+\gamma\sum_{j<i}c_{ij}U_j,
\quad W=I-\tau J_0,\quad\tau=h\gamma
\]

로 둔다. 선언된 full-space chart

\[
\tau J_0=cI+\sigma C,\quad\sigma>0,\quad\|C\|\le M,
\quad\pi(C)=0,\quad\deg\pi=d
\]

와 affine identity \(f(y+u)=f_0+J_0u\)가 있는 경우를 먼저 다룬다. 작은 수의 probe vector에서 맞았다는 사실은 full-space relation의 증명이 아니다. 이 조건은 analytic structural provider가 제공할 수 있으며 임의의 black-box RHS에서 Jacobian/AD를 자동 생성하라는 지시가 아니다.

\(\zeta=(1-c)/\sigma\), \(\pi(\zeta)\ne0\)라 하면 기존 conditioned-resolvent 항등식으로

\[
v(x)=\frac{\pi(\zeta)-\pi(x)}{\sigma\pi(\zeta)(\zeta-x)},\qquad
(1-c-\sigma x)v(x)=1-\frac{\pi(x)}{\pi(\zeta)}
\]

를 얻는다. 따라서 \(v(C)=W^{-1}\)이다. \(\pi(\zeta)=0\)이면 이 계획을 거부한다. 이것만으로 원래 문제의 모든 다른 풀이가 불가능하다고 결론내리지 않는다.

### 4.2 Scalar stage compilation과 endpoint 평가

\[
B_{ij}(x)=a_{ij}(c+\sigma x)+\gamma c_{ij},\qquad
r_i=\operatorname{rem}_\pi\!\left[v\left(1+\sum_{j<i}B_{ij}r_j\right)\right]
\]

를 순서대로 계산하면 induction으로 \(U_i=\tau r_i(C)f_0\)이다. Stage-linear endpoint는

\[
y_+=y+\tau p_\beta(C)f_0,\qquad p_\beta=\sum_i\beta_i r_i,\quad\deg p_\beta<d
\]

가 된다. **Scalar 준비 후 endpoint 하나에 필요한 full-space \(C\) action은 최대 \(d-1\)회**다. Stage 수가 8이어도 별도의 8개 stage-vector polynomial을 평가하거나 7개 새 stage RHS를 계산할 필요가 없다. 기존 두-channel specialization을 더 일반적인 선언된 quotient algebra로 확장한 것이며, 그 기존 방법을 신규 성과로 재분류하지 않는다.

Scalar 준비 비용은 일반적인 구현에서 \(O(sd^2+s^2d)\)다. 정확히 같은 step/tableau/chart/norm/model epoch일 때만 plan을 재사용한다. 여러 output functional은 powers를 공유할 수 있지만, 저장량·conditioning·rounding 비용 때문에 항상 유리하지는 않다. Dense output, stage observer, FSAL 또는 외부 callback effect가 실제 stage를 요구한다면 endpoint-only 계약으로 그 요구를 생략할 수 없다.

이는 일반 비선형 causal recurrence를 모두 병렬화한 정리가 아니다. Affine 구조가 있는 경우 vector recurrence 자체를 scalar preparation으로 옮겨 계산량을 줄이는 구성이다. 일반 비선형 모드에는 기존 finite causal majorant와 원래 잔차 검증을 유지한다.

### 4.3 Near-structure와 저장 계수의 원래 잔차

이제 \(\|\pi(C)\|\le\rho\)이고 실제 저장한 stage polynomial의 차수가 \(<d\)라고 하자. 원래 방정식에 대입한 polynomial residual은

\[
s_i=(1-c-\sigma x)r_i-1-\sum_{j<i}B_{ij}r_j=\alpha_i\pi+e_i,
\quad\deg e_i<d
\]

이며 \(\deg s_i\le d\)이므로 **quotient \(\alpha_i\)는 scalar**다. Leading coefficient로

\[
\alpha_i=-\frac{\sigma}{\pi_d}
\left(r_{i,d-1}+\sum_{j<i}a_{ij}r_{j,d-1}\right)
\]

를 직접 얻는다. 이것이 생략한 stage를 materialize하지 않는 certificate의 핵심이다.

Virtual stage \(V_i=\tau r_i(C)\widehat f_0\), \(q_i=\sum a_{ij}V_j\)에 대해 base RHS error \(\epsilon_0\), nonlinear remainder
\(\mu_i\ge\|f(y+q_i)-f(y)-J_0q_i\|\)가 유효하면

\[
b_i=|\tau|\left[\left(|\alpha_i|\rho+\sum_k|e_{ik}|M^k\right)\|\widehat f_0\|+\epsilon_0+\mu_i\right]
\]

가 full original stage residual을 감싼다. Inverse bound \(\omega\), 유효한 Lipschitz 상계로 만든 strictly lower triangular \(K\)에 대해

\[
\|\widehat y_+-\Psi_h(y)\|
\le |\beta|^T\left(\omega\sum_{\ell=0}^{s-1}K^\ell\right)b+\epsilon_{\rm eval}
\]

이다. Nilpotency 때문에 무한 반복을 가정하지 않는 유한식이다. Domain은 virtual stage뿐 아니라 비교 대상인 exact stage까지 감싸야 한다. \(\epsilon_{\rm eval}\)에는 actual polynomial evaluation·basis conversion·endpoint assembly를 포함한다.

이 식은 **같은 numerical input에서 exact discrete stage target까지의 algebraic error**다. 물리 flow의 temporal error는 3절의 올바른 anchor를 통해 별도로 합성한다. Affine self-adjoint 경우에는 모든 structure defect가 같은 \(\pi(C)f_0\)의 배수라는 상관관계를 signed adjoint로 보존해 absolute majorant보다 좁은 bound를 만들 수 있다. Nonnormal 경우 eigenvalue interval만으로 이 단계를 대체하지 않는다.

## 5. 임의 refinement 상수를 대체하는 수식

Linear target \(Ax=b\), standalone correction \(P\), true right defect
\(\|I-AP\|\le\theta<1\), \(\|A\|\le a\)를 가정한다. Residual 계산오차 \(\eta_k\), correction 오차 \(\zeta_k\), update 오차 \(\upsilon_k\)가 있으면

\[
R_{k+1}\le\theta R_k+(1+\theta)\eta_k+a(\zeta_k+\upsilon_k).
\]

이는 \(A\)와 \(P\)의 commutation을 요구하지 않는다. Additive allowance가 \(\kappa\) 이하이면

\[
R_m\le\theta^mR_0+F(1-\theta^m),\qquad F=\frac{\kappa}{1-\theta}.
\]

\(0<\theta<1\), \(R_0>\epsilon>F\)에서 최소 충분 반복 수는

\[
\theta^m\le\frac{\epsilon-F}{R_0-F}
\]

를 만족하는 최소 정수다. Native 구현은 rounded logarithm의 정수 변환에 의존하지 않고 outward power 비교로 경계를 확인해야 한다. \(\theta=0\)이면 한 번의 correction 후 \(R_1\le\kappa\)이므로 \(\epsilon=F=\kappa\)도 승인 가능하다. \(0<\theta<1\)에서 \(\epsilon\le F\)라는 사실은 이 bound가 충분한 finite certification을 주지 못한다는 뜻이며, 더 좋은 arithmetic·다른 알고리즘으로도 불가능하다는 정리는 아니다.

검산의 \(\theta=1/4\), \(\kappa=9/40000\), \(F=3/10000\), \(R_0=1\), \(\epsilon=1/1000\)에서는 최소 충분 반복 수가 **6**이다. 현재 코드의 `maximum_relation_ratio=0.5`, `stagnation_ratio=0.95`를 정당화된 수학 상수처럼 쓰는 대신 이 조건을 feasibility/work planner에 연결할 수 있다. 명시적인 user resource cap은 여전히 운영 정책이며, 원래 residual을 통과해야 output을 commit한다.

각 \(P_k\)에 독립적인 증명이 있는 standalone refinement는 product-sum bound를 사용할 수 있다. 이것이 기존 fixed-linear Krylov preconditioner를 iteration 도중 임의 변경해도 된다는 뜻은 아니다. 고정 조건을 유지하거나 별도로 정당화된 flexible outer method가 필요하다.

## 6. Polyalgorithm의 선택·예산·session을 함께 설계

### 6.1 같은 target끼리 비교한다

Shifted linear solve, fused \(\varphi\)-action, nonlinear stage/root, physical ODE step은 서로 다른 계약이다. Chebyshev inverse polynomial과 Laguerre exponential action이 둘 다 polynomial이라는 이유만으로 같은 후보 집합에 넣지 않는다. 먼저 target·norm·clock·output·permission을 고정하고, 그 요청에 실제로 적용 가능한 후보들만 비교한다.

Chebyshev의 SPD interval, Laguerre의 operator domain/scale/tail, Faber·Leja의 enclosure, Krylov의 true residual/orthogonalization, quotient compiler의 full-space relation은 각기 다른 admission 근거다. Prior PR93의 17개 후보를 유지하면서 이 공통 계약을 덧붙였다. Polynomial 대안은 reduction·orthogonalization 비용을 줄일 수 있지만 준비와 certification 비용, degree, nonnormality에 따라 이점이 사라질 수 있다. 따라서 strongest eligible direct/CG/common-factor/Krylov 비교를 남긴다. 원전 목록은 이전 `SOURCES.json`을 해시 고정해 재사용한다.

### 6.2 Tolerance 분할은 finite menu의 정확한 문제로

고정된 request/tube에서 각 component가 \((e_{ij},c_{ij})\)를 제공할 때

\[
\min\sum_i c_{ij_i}\quad\text{subject to}\quad r+We\le\epsilon
\]

를 finite menu와 componentwise Pareto frontier로 정확히 푼다. 비용 단위는 하나로 선언한다. 동일 label은 하나를 유지하고, 지배 관계는 적어도 한 coordinate가 strict해야 한다. Continuation state가 다른 후보끼리는 현재 cost/error만 보고 pruning하지 않는다. 현재 cost가 0인 cold state의 다음 비용이 100이고 현재 cost가 1인 warm state의 다음 비용이 0인 반례가 있다.

검산의 동일 오차 cap \(3/10\)에서 equal physical halves는 cost 10, finite-menu optimum은 cost 7이었다. 이는 **선언된 toy work cost**이며 30% native speedup을 측정했다는 뜻이 아니다.

\(C_i(\eta_i)=a_i\log(B_i/\eta_i)\)라는 명시적 relaxation에서는

\[
\eta_i=\frac{B a_i}{w_i\sum_j a_j}
\]

를 KKT로 얻는다. Bounded version은 \(0<\ell_i\le\eta_i\le u_i\), \(B\ge\sum_iw_i\ell_i\)를 요구한다. 실제 degree/iteration은 finite menu와 outward admission으로 결정한다. Hardware latency는 실측값이며, 증명에서 도출되는 오차 상수와 구분한다.

### 6.3 Probe 이후에는 완전한 session을 유지한다

현재 switcher probe는 `with_session` 안에서 session을 만들고 종료 시 버린다. 단순 endpoint만 복사하면 FSAL, controller history, dense state, observation cursor, provider epoch, global work budget을 잃을 수 있다. 기존 `NativeExplicitSession`을 유지하는 경로가 새 wrapper를 처음부터 만드는 것보다 먼저 검토할 대상이다.

중요한 조건은 **처음부터 전체 intended observation schedule을 등록**하는 것이다. 현재 probe schedule `[start, target, solve_end]`만 가진 session을 중간 observation 요청이 있는 production schedule로 그대로 이어 붙일 수 없다. 동일한 full state·callback law·schedule·epoch·남은 budget이면 deterministic transition induction으로 동일한 continuation trace를 얻는다. 이 정리는 실제 elapsed gain을 보장하지 않는다.

완료 후보 비용 \(C\)와 아직 끝나지 않은 후보의 total completion cost 하한 \(L\)이 같은 비음수 additive cost 단위이고 \(L\ge C\)이면, 완료 후보를 tie에서 유지하는 정책 아래 미완료 후보는 엄격히 이길 수 없다. Integer strict cap은 \(C-1\)로 구현할 수 있다. Callback score의 하한이 wall-time 하한인 것은 아니다.

시도·승인·거부 event는 \(A=E+R\), 완료·실패는 \(E=C+F\)로 관리하며, pending이 있으면 따로 포함한다. 물리 callback ledger는 별도다. 거부된 값은 rollback하되 실행한 work와 fatal failure를 지우지 않고, 모든 worker 결과를 join한다.

## 7. 이론 연구가 필요한 부분과 구현으로 줄일 수 있는 비용

| 분류 | 미해결 또는 제한 | 진행 방향 |
|---|---|---|
| 이론·모델 정보 | 유한 black-box probe로 임의 연산자의 full-space relation/nonnormal bound를 보장할 수 없음 | 선언된 provider와 domain certificate; 관계가 없으면 일반 경로 유지 |
| 이론·정확도 | local algebraic residual만으로 temporal order·stiff-uniform order·physical error를 보장할 수 없음 | dual anchor, reconstruction/tube 또는 method-specific truncation 증명 |
| 이론·branch | 작은 nonlinear residual은 원하는 root branch를 식별하지 못함 | monotonicity, invariant interval, continuation regularity, original-target 검증 |
| 이론·시간변화 | 개별 step 안정성과 임의 step product 안정성은 다름 | 공통 metric/증명된 variation 조건; 기존 variable-step Verlet negative control 유지 |
| 이론·비용 | 제한 없는 미래 workload에서 현재 최저비용 선택이 전역 최적이라는 보장은 없음 | continuation state·residence/variation 가정; exploration loss 별도 정리 |
| 구현 | stage-vector intermediate와 반복 basis preparation | quotient output compilation, exact-key plan reuse, fused action |
| 구현 | warm state를 버리는 adapter probe | full session ownership과 schedule 보존 |
| 구현 | 충분히 계산했는데도 고정 반복 정책으로 추가 work | proved floor·최소 충분 정수와 original-residual 승인 |
| 구현 | 이미 감싼 오차의 중복 계산, 불필요하게 동일 비율로 나눈 tolerance | edge ownership과 finite-menu allocation |
| 구현·qualification | false-pass, 실패 work 누락, feature/ignored test 누락 | 독립 callback oracle, negative controls, 누적 coverage와 명시적 gate status |

구조적 특수화의 적용 범위가 제한된다는 것은 polyalgorithm 자체의 결함이 아니다. 해당 범위를 감추고 일반 문제의 보증으로 확대하는 것이 결함이다. 충분조건이 실패했을 때는 그 certificate의 한계를 기록하고 다른 승인 가능한 후보로 넘어간다. 상계 floor와 representability lower bound도 구분한다. Exact fixture에서 \((1/8)\mathbb Z\) output으로 target \(1/3\)에 도달할 수 있는 최소 오차는 \(1/24\)지만, 느슨한 upper bound 하나만으로 이런 불가능성을 말할 수 없다.

Adjoint literature도 그대로 derivative-free certificate로 가져오지 않는다. Cao–Petzold의 nonlinear adjoint 구성은 Jacobian·trajectory·nonlinear remainder 조건을 포함하고, 일부 소수 sample 기반 conditioning 절차는 통계적 성격이 있다. 이번 full-space deterministic theorem은 그 조건을 생략한 재해석이 아니다. [CP2004](https://people.cs.vt.edu/~ycao/publication/siam_42096.pdf)

## 8. 실행된 검산과 남은 검증

| 항목 | 실제 결과 | 주장 가능한 범위 |
|---|---|---|
| Error exact check | 6개 묶음 PASS, exit 0 | switching induction, anchor 반례, norm/clock, menu, KKT, precision 구분의 유한 검산 |
| Algebra exact check | 9개 compiler fixture PASS, refinement/denominator PASS, exit 0 | 원래 작은 full-stage rational target과의 비교, near/stored/RHS/nonlinear defect, 6회 correction 예 |
| Wolfram | KKT residual 0, stationarity 0, Hessian positivity 결과; symbolic-global warning 보존 | exploratory 보조 계산; registered exact check와 별도 |
| Bounded independent review | 1회 종료 | theta=0, Pareto tie, bounded KKT domain, cubic \(|g|\) 수정 후 소스 등록; 추가 review pass를 수행했다고 주장하지 않음 |
| Native solver/test/benchmark/profile | 이번 연구에서 NOT_RUN | 이번 수학적 개선의 runtime 속도·강건성 향상은 아직 미측정 |

Algebra negative controls는 near relation의 오차를 0으로 둘 수 없다는 사실과 quotient denominator refusal을 포함한다. Nine fixtures의 정확한 분수 결과는 `evidence/ALGEBRA_RESULTS.json`에 있다. 작은 검산의 계산 시간은 solver 성능 지표가 아니다.

## 9. 현재 CI의 구체적 실패와 독립 검증 PR

PR93의 [E4 job](https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40609485096/job/122557234098)은 default graph/compile와 touched contract를 통과했으나 broader integrator Clippy의 **23개 고유 진단 위치**에서 실패했다. 이후 vendor/offline graph 비교·compile·test와 마지막 formatting은 skipped다. 두 compiler summary의 17/22를 더해서 진단 수를 만들지 않는다. Q-LINT 수정 뒤 skipped 단계를 실제로 실행해야 한다.

[Authority job](https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40609485121/job/122557234399)은 checker unit/node append-only 검사를 통과하고 full authority 검사에서 실패했다. 10,133 citations, 797 identifiers이며 missing 53, missing+allowlisted 27, non-commit 220, reachable 497이었다. Workflow는 `fetch-depth: 0`이므로 전체 실패를 shallow clone 탓으로 돌릴 수 없다. External repository commit, Julia `git-tree-sha1`, 검증 가능한 local/public source-tree equivalence, 실제 미해결 local authority를 분리해야 한다. 기존 receipt를 고치거나 전체를 allowlist하는 방식은 사용하지 않는다.

후속 독립 PR은 **이번 최종 delivered commit에서 새 branch를 만들고 `main`을 target**으로 한다. 이전 PR을 먼저 merge할 필요도, 같은 commit을 다시 cherry-pick할 필요도 없다. 로컬 시작 시 main과 PR head를 다시 확인하고 최신 main과 갈라졌으면 새 worktree에서 충돌을 해결해 실제 통합 tree를 검증한다. 최초 pin과 비교를 보존한다.

검증은 네 가지 증거를 분리한다: source inclusion, 현재 누적 구현의 semantic coverage, inherited release qualification, 과거 원본 source의 evidence replay. 다른 branch에서 같은 코드를 실행했다는 사실만으로 oracle까지 독립적이 되는 것은 아니다. Exact rational/full original equations/고정밀 interval 등의 독립 oracle을 연결한다.

`INDEPENDENT_VALIDATION.json`은 이전 **23개 node·34개 edge·8개 suite·17개 후보**를 해시로 고정해 상속한다. 18개 gate group, 73개 경로, 10개 교차 interaction, N01–N06 및 Q 7개 과제의 추가 계약을 연결한다. 기존 correction budget은 structural 3/3, adapter 3/3, homotopy 1/3, integration-accounting 1/3 그대로다. 후보 이름을 바꿔 새 budget을 만들지 않는다.

Historical Python check 중 일부는 기존 result JSON을 덮어쓴다. 반드시 원본 source의 disposable worktree/mirror에서 실행하고 새 receipt를 별도 보관한다. PR93 `verify.py`와 이번 node의 frozen-source verifier는 연구 당시 runtime이 변경되지 않았다는 조건을 검사하므로, 후속 native 수정 tree에 그대로 적용해 알고리즘 회귀라고 해석하지 않는다. 반대로 native semantic tests는 **최종 누적 수정 tree**에서 실행해야 한다. `run_local.py`는 자체 lock을 얻으므로 같은 outer `flock`으로 감싸지 않는다.

## 10. 전달물과 다음 구현 우선순위

1. 기존 cumulative source를 독립 worktree에서 고정하고 Q-GIVENS/Q-FAIR-AB/Q-LINT/Q-NEGATIVE 및 전체 build/feature/release를 검증한다. 실패를 실제 원인별로 해결하고 기존 음성 결과를 보존한다.
2. N01의 fused target-specific actions, N02의 finite-step planner, N04의 full-session continuation을 서로 다른 비용 원인으로 분리해 진행한다.
3. N03/N05에서 quotient compiler를 declared affine 구조부터 이식하고, outward coefficient/domain/evaluation certificate를 완성한 뒤 near-structure로 확장한다. Strongest current specialization과 common-factor 경로가 comparator다.
4. N06의 residence/미래 비용 주장은 명시한 variation·continuation 가정이 갖춰진 뒤 실행한다. 범용 switching 안정성이나 universal speedup을 가정하지 않는다.
5. 동일 physical cap·동일 observation schedule·동일 derivative permission에서 cold/warm setup, certification, discarded/failure work까지 포함한 paired benchmark와 profile을 수행한다. Accuracy 상수와 hardware service cost를 별도 기록한다.

복사해서 사용할 local Codex prompt는 `INDEPENDENT_PR_HANDOFF_KO.md`, dry-run 및 안전한 새 worktree 준비는 `prepare_validation.py`, 전체 연결은 `DEVELOPMENT_DAG.md`와 `INDEPENDENT_VALIDATION.json`에 있다. 최종 배포 commit은 PR 및 사용자 전달 메시지에서 full SHA로 고정한다. 이 문서는 자기 자신의 미래 commit hash를 만들어 넣지 않는다.
