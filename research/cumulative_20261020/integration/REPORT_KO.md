# VigilODE 통합 연구: matrix-free, 단계 병렬 Rosenbrock, homotopy의 결합

기준 소스: `c03ca90f19a9e64ba7751c069325a48b54b39c2e`, 조회 브랜치 `claude/nice-dijkstra-208xq0`.\
이 결과는 기존 코드 조사, 구성적 유도, 독립 Python 탐색 프로토타입 및 경량 대조 실험이다. 생산 Rust 경로를 연결하거나 새 기본값을 배포한 결과가 아니다. Native subagent는 제공되지 않아 사용하지 않았으며 독립 연구자 심사를 주장하지 않는다.

## 결론

통합의 중심을 `matrix-free operator session + all-at-once stage target + adaptive homotopy corrector + shared error/work budget`로 두는 것이 적절하다. 기존 순차 fast/banded는 안전한 fallback과 작은 문제의 경쟁 후보로 유지하되 그것을 모든 문제의 기본 연구 목표로 삼지 않는다. 핵심은 8개의 원래 stage를 독립 작업이라고 가정하는 것이 아니라, 그 stage 방정식 전체의 동일한 해를 부분공간 예측과 동시 residual/correction으로 얻는 것이다.

세 가지 결론을 구별한다. (1) 작은 유효 Krylov 부분공간이 존재하면 큰 공간에서 8회의 순차 W solve를 하지 않는 구성은 가능하며, 이번 합성 예제로 직접 구현했다. (2) RHS rank가 낮다는 사실만으로 그 부분공간이 작다는 결론은 나오지 않는다. (3) 병렬 실행이 가능해도 메모리 대역폭·스케줄링 비용 때문에 빠르지 않을 수 있으므로 수학적 경로 선택과 하드웨어 실행 선택을 분리해야 한다.

## 1. 현재 코드의 통합 상태

`routing.rs`의 현재 router는 declared band, small dense, explicit Jacobian, unstructured JVP에 따라 기존 driver를 고른다. homotopy/단계 병렬/공유 Krylov 기반의 통합 dispatcher가 아니다. `ParallelExecution`은 실제 local Rayon stage-row 실행을 제공한다. 그러나 `Audit2MatrixFreeCommonWSession::solve_rows`는 하나의 mutable workspace를 재사용하면서 RHS를 순서대로 푸는 연구 세션이다. 이름의 batch만으로 병렬 common-W가 연결됐다고 볼 수 없다.

`homotopy.rs`에는 theta/lambda partial coupling, path predictor/corrector, common-W batch와 q차 truncated inverse가 있다. 확인한 legacy `run_fixed_homotopy_path_internal`은 explicit common W와 LU를 요구한다. `audit2_matrix_free_research`는 그와 별개의 연구 세션이며 자체 문서가 적분기 dispatch·Krylov basis 재사용을 주장하지 않는다. 따라서 "homotopy 경로 전체가 matrix-free이면서 병렬"이라는 검증은 인터페이스 연결 후 별도로 필요하다.

`rhs_telemetry.rs`는 directional rank, subspace drift와 seeded-shared/block/independent-Rayon backend 선택을 구분한다. 추천은 advisory이다. `transactional_q1_q2.rs`는 q1/q2 escalation과 operational diagnostic 대 outward certificate를 분리하고, `causal_majorant.rs`는 strictly-lower nonnegative majorant의 finite path sum과 radius-box closure를 이미 구현한다. 이를 새로 복제하기보다 공통 root/오차/작업 계약에 연결해야 한다.

`raw_stage_target.rs`는 raw-U와 native-K target의 floating-point 차이를 명시한다. 본 탐색은 원본 decimal A,C,gamma,b_code의 raw-U 방정식을 사용한다. 같은 차수/같은 이론식이라도 native-K와 bit-identical하지 않으며, 실제 통합에는 raw_stage_allowance가 관리하는 coefficient/target/output/embedded/dense 차이를 전달해야 한다.

소스 경로·blob은 SOURCES.json에 있다. 확인하지 않은 모든 repository 경로를 조사했다고 주장하지 않는다.

## 2. 중심 방정식과 homotopy 구조

설명을 위해 autonomous, identity-mass ODE y'=f(y), f가 필요한 stage domain에서 C1, h>0을 가정한다. J0=f'(y_n)은 한 macro-attempt 안에서 고정한다. gamma는 무차원이고 h는 시간, J0는 시간^-1, U는 state 단위다.

고정 snapshot의 raw A,C는 엄격한 하삼각이다. W=I-h gamma J0, D=I_s tensor W, s=8, x_i=sum\_{j\<i} A_ij U_j라 두면

F_i(U)=W U_i - gamma sum\_{j\<i} C_ij U_j - h gamma f(y_n+x_i)=0.

f(y_n+x)=f0+J0 x+N(x),\
B=gamma C tensor I + h gamma A tensor J0,\
b=1_s tensor(h gamma f0),\
R(U)\_i=N(x_i)

로 두면 F(U)=(D-B)U-b-h gamma R(U).

H(U,lambda;theta)=[D-eta(lambda)B]U-b-lambda h gamma R(U),\
eta=theta+lambda(1-theta).

lambda=1에서는 theta와 관계없이 같은 raw target이다. theta=0은 완전 decoupled seed를, theta=1은 전체 affine stage coupling을 먼저 처리하고 nonlinear remainder만 켜는 경로를 준다. theta=1의 seed 비용은 공짜가 아니며, 공유/투영 Krylov로 싸게 생성 가능할 때만 이점이 있다.

### 새로 정리한 구조적 관찰: 이 stage homotopy의 fold 조건

R_i는 U_1,...,U\_{i-1}에만 의존하므로 H_U의 diagonal blocks는 모두 W다. 따라서

det(H_U)=det(W)^s,

즉 고정된 가역 W에서 lambda나 U의 변화만으로 stage-root fold가 생기지 않는다. 각 lambda의 stage 해는 유효 domain 안에서 삼각 재귀로 유일하게 결정된다. 이 결과는 domain 이탈·overflow·W 특이성·variable-state mass·다른 물리 root system을 제외한다. condition number나 path curvature가 작다는 주장도 아니다.

따라서 이 stage homotopy에서는 generic pseudo-arclength machinery보다 예측값, 비선형 coupling의 활성화, 가역 common-W와 정확한 residual transport를 우선한다. 별도의 물리적 equilibrium/branch-selection root problem에서 fold가 존재할 가능성은 별개다. 이 둘을 같은 homotopy API의 같은 의미로 취급하지 않는다.

정확한 tangent는

H_U dU/dlambda=(1-theta)BU+h gamma R(U).

확인한 기존 predictor는 주로 truncated partial-linear inverse를 이용한다. nonlinear derivative 항까지 포함한 정확한 tangent라고 자동 해석하지 않는다. 저렴한 근사 predictor는 유효하지만 최종 lambda=1의 원래 target residual/output 검사가 필요하다.

### 왜 단순한 8-stage 동시 Picard가 답이 아닌가

affine 문제에서 T=D^-1 B는 strictly-lower block operator이므로 T^8=0이다. 하지만 rho(T)=0은 한 번 또는 두 번의 iteration이 충분하다는 뜻이 아니다. 이번 raw RODAS5P 계수의 scalar z=hJ=-100에서 ||T^3||\_inf는 약81.89였고, 중간 error는 초기보다 크게 증폭한 뒤 여덟 번째 전달에서 소멸했다. 유한 causal depth와 실용적인 작은-round 수렴을 구분해야 한다.

좌표 의존성은 별도로 확인했다. raw-U의 단순 diagonal splitting에는 gamma\*C라는 O(1) 결합이 남지만, 정확한 K 표현에서는 T_K=(h L tensor J)/W이다. 독립 triangular 변환으로 계산한 z=-.01에서 ||T_U||\_inf=68.676, ||T_K||\_inf=0.07838이었다. 이는 raw-U Picard의 transient를 모든 stage formulation의 본질적 장애로 해석하면 안 된다는 반례다. stiffness가 커지면 K도 자동 수축하지 않는다. 각 norm은 좌표에 의존하므로 원래 output에 대한 오차도 coordinate_comparison.json에 기록했다. 변환은 native faer coefficient bits와 동일하다고 주장하지 않는다. 기존 stage_chart_candidate.rs는 static stage-coordinate 후보를 원래 target으로 되돌려 검사하는 구조를 이미 제공하지만, 확인한 구현은 explicit projected/reference 용도이며 범용 matrix-free chart dispatcher가 아니다.

또한 (I+T)(I+T^2)(I+T^4)=sum\_{k=0}^7 T^k라는 항등식은 작은 명시적 stage matrix에서는 3단계 doubling을 허용하지만, 일반 matrix-free T에 T^2를 적용하는 비용은 여전히 T 두 번이다. 작은 majorant 행렬의 logarithmic certificate depth를 고차원 black-box stage solve의 무료 logarithmic critical path로 옮기면 안 된다.

## 3. 권장 결합 방식

### A. JVP 공유 부분공간 + coupled-stage homotopy: 최우선 연구 후보

f0, accepted history predictor 및 현재 stage RHS의 유효 방향으로 공유 V_m을 만든다. JVP만으로 J_m=V_m^T J0 V_m을 형성하고 8m 크기의 작은 stage system을 동시에 푼다. U=(I_s tensor V_m)z로 복원한 뒤 원래 n차원 target residual을 검사한다. projected residual만으로 승인하지 않는다.

부분공간을 nonlinear corrector와 다른 RHS에 재사용하되 새 nonlinear remainder가 V_m 밖으로 빠지면 그 orthogonal residual을 basis expansion에 사용한다. 작은 singular value라는 이유만으로 output-sensitive 방향을 버리지 않는다. 원래 W가 가역이어도 V^T W V가 가역이라는 보장은 없다. 예를 들어 W=[[0,1],[-1,0]], V=e1이면 det(W)=1이지만 V^T W V=0이다. 작은 projected solve의 가역성·조건수도 별도로 확인하고, 실패하면 basis/투영 방식/전처리를 바꾸어야 한다. krylov차원, 원래 residual, output transport, orthogonalization 비용이 예산을 넘으면 다른 backend로 넘긴다.

이 방식이 유리한 조건은 작은 state dimension 자체가 아니라 작은 effective Krylov dimension, 느린 operator/subspace drift, 저렴한 basis 재사용이다. 대규모 문제라도 m이 작으면 유리하고, rank1 RHS라도 넓은 spectrum/non-normality 때문에 m이 크면 불리하다.

### B. Full-space 동시 stage Newton-Krylov + common-W batch

투영 모델로 충분하지 않으면 같은 target F를 전체 stage vector에 대해 푼다. F'의 action은 각 stage에서 J0 v와 J(Y_i)v를 사용해 구성하고, explicit n\*n Jacobian 또는8n행렬을 만들지 않는다. 단계의 상태/벡터를 하나의 old iterate에서 고정한 뒤 RHS/JVP와 diagonal W solves를 동시에 실행한다. 전처리는 D^-1 또는 짧은 causal Neumann correction P_q로 시작한다.

q를 키워 inner iteration을 줄였다는 사실만으로 개선이라고 판정하지 않는다. 이번 toy에서 q0->q1은 inner iterations13->7이었으나 W-inverse batches17->22였다. 실제 총 JVP/W solve/orthogonalization/동기화/오류검사 비용을 비교해야 한다.

W^-1을 내부 iterative solve로 매번 다른 정확도에 적용하거나 preconditioner를 inner iteration 중 바꾸면 일반 GMRES의 고정 선형 preconditioner 가정이 깨질 수 있다. 그 경우 flexible Krylov가 적절하다. 고정 approximate P와 현재 W를 구분하고 true residual은 언제나 현재 W 또는 현재 F'에 대해 계산한다.

### C. Parallel exponential Rosenbrock: 같은 세션을 공유하는 별도 방법 후보

기존 pexprb/phi-prefix 경로는 선형 stiffness·oscillation을 작은 Krylov/phi action으로 잘 처리할 때 후보가 된다. 문헌의 parallel exponential Rosenbrock와 parallel peer/W 방법은 진정한 stage-parallel method family를 제공하지만, RODAS5P의 실행 순서만 바꾼 것이 아니다. 기존 exponential path는 common JVP/basis/execution/work service를 공유하고 독립 embedded/dense error contract를 가져야 한다.

논문상 효율이나 프로젝트의 구현 존재를 현재 문제에서의 승리로 해석하지 않는다. 실제 switching 시점의 오차, controller/history 의미, start-up/rollback, prefix 이월 비용이 검증돼야 한다. 이 연구에서는 method switch를 실행하지 않았다.

### D. Sequential 보호 경로의 역할

작은 n, 저렴한 RHS, 얕은 Krylov 이득이 없는 경우 또는 병렬 후보의 예산 실패에는 기존 direct/banded/matrix-free sequential 경로가 합리적이다. 이것은 본래 동기의 포기가 아니라 정확도 보호와 비교 기준이다. 다만 main router가 explicit-J presence만으로 항상 dense를 택하거나 JVP path를 항상8순차stage로 제한하면 본래 연구 목표를 테스트할 기회를 잃는다. capability screening 이후 cost-aware 후보 선택을 별도 계층으로 둔다.

## 4. Homotopy는 언제 켜는가

lambda=1로 바로 시도하는 affine-predictor + corrector가 가장 싼 homotopy의 endpoint case다. 처음부터4개 lambda구간을 의무적으로 밟지 않는다. current corrected residual/output bound가 줄어드는 동안 계속하고, path jump가 budget상 불리하거나 domain 제약을 위반할 때 lambda step을 줄인다. q와 lambda와 physical h를 동시에 맹목적으로 바꾸지 않는다. 먼저 무엇이 제한인지 분류한다.

- W solve가 비싸면 preconditioner/subspace를 개선한다.
- stage coupling/비선형 remainder가 크면 predictor, q 또는 lambda를 조정한다.
- physical truncation/dense error가 크면 h 또는 출력 알고리즘을 조정한다.
- stage-parallel scheduling이 비싸면 배치 벡터화/공간 병렬화/worker budget을 바꾼다.

root solver를 매우 엄격히 풀어 physical discretization error를 줄이려 하지 않는다. local LTE, stage-root error, inexact linear solve, JVP error, projected-basis leakage, coefficient-target conversion, dense output는 서로 다른 예산 성분이다.

## 5. 오류 승인과 경제성

stage error E_i에 대해 (유효한 domain과 inverse/derivative bound가 주어졌을 때)

E_i <= a_i + sum\_{j\<i} L_ij E_j,\
E <= (I-L)^-1 a=sum\_{k=0}^{s-1} L^k a

라는 causal majorant를 쓸 수 있다. L은 strictly lower, nonnegative이므로 전역 ||L||<1을 반드시 요구하지 않는다. 그러나 bound를 사용한 radius box가 실제로 닫히는지는 별도로 검증한다. endpoint 출력뿐 아니라 embedded weight 및 dense-output weights에 전달하여 해당 요청출력의 solver error를 제한한다. 프로젝트의 causal_majorant, raw_stage_allowance, Q2 certificate 구분을 재사용한다.

한 Newton correction의 projected output은 nonlinear remainder를 빠뜨리므로 일반적인 error bound가 아니다. 현재 homotopy 소스도 이를 명시한다. witness가 없는 범용 문제는 operational residual screening + 독립 reference validation 단계로 분리하고 certified status를 부여하지 않는다.

physical 5차 유지에는 stage solve가 유발하는 local state perturbation도 적절히 O(h^6) 수준이어야 한다. dimensionful h^6을 그대로 tolerance와 비교하지 말고 local error scale 또는 h/t_ref와 state scale로 단위를 맞춘다. 전역오차 보증은 별도의 propagation/stability 문제다.

같은 h/output contract, 성공확률p, 후보전체비용C_a, 승인비용C_v, fallback비용C_f라면 재사용이 없을 때

E[C]=C_a+C_v+(1-p)C_f,\
E[C]\<C_f <=> p>(C_a+C_v)/C_f.

프로브·실패 비용을 빼면 이 판단이 왜곡된다. 서로 h가 다른 후보는 expected cost / expected accepted physical-time advance로 비교한다. 공유 prefix가 있을 때는 실제로 절약된 fallback cost만 빼며 identity와 work receipt를 유지한다.

## 6. 이번 경량 계산 결과

### 6.1 전체30개 homotopy 대조

실험은 f_i(y)=a_i y_i+epsilon y_i^2, a=(-1,-10,-100,-1000), y0=(.1,.1,.1,.1), h/epsilon6조합이다. raw-U root와 비교했고 physical trajectory convergence는 평가하지 않았다. 총30설정이 종료했으며 최대 출력 차이는 설정된 WRMS scale에서0.0001594미만이었다. 이 중 linear_seed oracle를 쓰는 실험은 seed 비용을 제외했다고 원자료에 표시한다. 성능비로 쓰지 않는다.

h=.01,epsilon=10에서 decoupled-seed one-jump(q0)는3outer/18inner, affine-seed one-jump(q0)는2outer/13inner, affine-seed one-jump(q1)는2outer/7inner였다. q1은 W-inverse batches가17에서22로 늘었다. affine-seed four-lambda(q1)는8outer/28inner/88W-inverse-batches였다. 따라서 continuation subdivision과 q증가를 무조건 조합하는 것은 이 사례에서 비경제적이다.

### 6.2 실제 큰 공간을 조립하지 않는 통합 후보

별도 projected_parallel.py는 oracle affine seed를 쓰지 않는다. 12288변수, 세 spectral group의 합성 quadratic ODE에서 JVP로 m=3 부분공간을 구성하고24차원의 작은 all-stage root를 풀었다. stage작업4개가 실제 동시에 실행됐다. lambda=1 one-jump는2개의 작은 Newton linear solve와3개 stage batches, full-space relative residual6.64e-15, reference stage-root 출력WRMS1.28e-8이었다.

다만 '전체 작업이 JVP3회뿐'은 아니다. basis용 base-JVP3회 외에 projected stage derivative 연산72vector-equivalents, RHS24stage-vectors, full residual validation과 독립 reference 계산이 있다. 행렬이 없는 범용 spatial nonlinear problem이 아니라 invariant three-mode 대조군이며 native Rust backend나 전체 적분 wall-time우위를 검증하지 않았다.

동일 문제를4개 lambda구간으로 나누면 작은 Newton solve6회, stage batches10회, stage derivative240vector-equivalents와 RHS80stage-vectors가 됐다. 모든 문제에서 homotopy의 여러 path점이 필요한 것은 아니라는 정량 근거다.

### 6.3 낮은 RHS rank의 반례

affine stage RHS rank가1인 두 문제를 비교했다. 세 eigenvalue그룹의384변수 문제는 basis3에서 full residual7.31e-15와 output WRMS2.29e-8을 얻었다. 반대로 넓은 diagonal spectrum의64변수 문제는 basis32에서도 output WRMS1.482로 목표1을 넘었다. 이때 full relative residual은2.42e-9이었다. 따라서 RHS rank와 residual수치 하나만으로 projected candidate를 승인하면 안 된다.

### 6.4 순진한 threading의 부정 결과

단계8개의 periodic1D reaction-diffusion RHS kernel을 1/2/4workers로 실행했다. 12개 같은-process block, block당5반복이며 warmed pool setup는 별도다. 결과 bits는 동일했다.

n=256:1worker0.139ms,4workers0.506ms.\
n=16384:1worker0.522ms,4workers1.694ms.\
n=131072:1worker3.645ms,2workers3.322ms,4workers3.396ms.

큰 사례의 paired speed ratio는2workers1.048,4workers1.017정도이며 block구간은1양쪽에 걸쳤다. Python/NumPy 및 shared sandbox kernel결과라 native Rayon crossover로 옮길 수 없다. 그렇지만 local Rayon microbenchmark의 concurrency만으로 실제 memory-bound JVP speedup을 주장하면 안 된다는 프로젝트의 기존 경고와 일관된다. preallocated SoA/batched JVP, spatial vectorization, threadbudget1/2/4측정이 실제 결합 전 필요한 이유다.

## 7. 구현 결합 단위와 승인 조건

새 최상위 연구 entry `integrate_coupled_matrix_free_adaptive_observed`를 제안한다. 이것은 현재 존재한다고 주장하는 함수가 아니다. 실제 재사용 대상과 신규 작업을 분리한다.

1. `StageTargetContract`: raw-U/native-K ID, coefficient bits/allowance, mass/JVP epoch, output/embedded/dense maps. 잔차 의미가 다른 두 경로를 bit-equivalent로 취급하지 않는다.
2. `OperatorSession`: 현재W/JVP와 별도의preconditioner, 공유/worker별GMRESworkspace와basis. W변경시 이전basis의새 residual을 계산한다. 오래된LU를preconditioner로사용할수있어도그것을현재W의정확solve로위장하지않는다.
3. `StageExecutionPlan`: shared Krylov, block Krylov, independent Rayon, component-major batch 중 선택. p_stage\*p_space가코어예산을넘지않게하고mutableworkspace를동시공유하지않는다. 실패한batch에서도완료·중단·진행중작업을회계한다.
4. `StageRootDriver`: affine/history predictor, lambda1fastattempt, residual/output-basedq/lambdaescalation. 같은세션을continuation동안재사용하고prediction/correction의중간결과를버리지않는다.
5. `CommitGate`: physicalLTE, roottransport, inner/JVP/basis/leakage와output예산을분리검사. 승인후에만state/controller/history를전진시키고실패하면trial상태를rollback한다.

먼저 검증할 native node는 'JVP-only에서 source legacy homotopy의 explicit-W요구를 끊고, 실제 병렬 batch와 residual transport를 연결하는 하나의 complete step'이다. 이 범위에서8단계순차fallback과동일h/output관측오차를비교하고, fail/partial/accounting까지검사한다. 그다음시간적분·denseoutput·actualswitchorder를검증한다. 범용optimalrouter는그자료없이기본값으로승격하지않는다.

## 8. 상태

Derived: stage homotopy의triangular Jacobian/fixed-W nonfold정리, finite causal errortransport, 비용손익분기.\
Numerically checked:30개의rawtargetprobe,small-Jacobianfinite-difference/target/determinant검사,실제projectedparallel예제,rank반례,kernelthreading대조.\
Implementation-verified scoped:Python프로토타입13개test통과.\
Not claimed:Rustproductionintegration,전체trajectory차수/연속오차인증,일반physicalclient,실제methodswitch,전체VigilODE/DVODE/Diffsol새speedrank,새W-method계수,외부동료심사.

원문코드와이미게시된실험을수정하지않았다. 최종zip은모든실험스크립트,원자료,tests와sourceidentity를포함한다.
