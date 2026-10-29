# VigilODE polyalgorithm: 이전 PR 재사용과 남은 과제의 구성적 이론 조사

2026-10-28. 기준: PR92, `7eb2dd0ffda39afb2871d76700a720986ff09fe0`.

## 1. 결론과 이번 변경의 범위

가장 유망한 방향은 Krylov를 하나의 다른 기법으로 일괄 교체하는 것이 아니다.
이미 있는 저차 연산자 대수, conditioned resolvent, joint phi action, causal
재사용과 homotopy를 **동일한 목표와 물리 오차 계약을 유지하는 후보군**으로
연결하는 것이다. 모델이 제공하는 구조에 따라 불필요한 차원·벡터 작용·root
반복을 먼저 없애고, 남은 계산에 맞는 polynomial 또는 Krylov 경로를 선택한다.

이번 작업은 요청된 physmath-research-loop 1.3.0의 R2 문헌 조사와 R4 발산적
구성을 세 native subagent로 병렬 수행했다. 일반 정리의 증명, exact-rational
지원 프로그램, 17개 후보와 기존 PR-N01~06의 구현 계약을 추가했다. 과거의
전체 10단계 수행 기록이나 최대 3회 수정 예산을 새로 시작하지 않았다.

사전 등록·검산 소스는 공개 commit
`866668348f840bd43558859d2795576e292b6ca7`에서 고정한 뒤 실행했다.
세 지원 검산은 모두 통과했다. Rust solver 구현과 기본 경로는 이 보충 연구에서
변경하지 않았으며 native tests, solver benchmark, profiling은 실행하지 않았다.
따라서 아래의 작업 수·격자 수 감소를 새로 측정된 wall-time speedup으로 읽으면
안 된다. 기존 PR92의 실패한 CI/미완료 release qualification도 그대로 남는다.

## 2. 이전 PR에서 실제로 재사용한 내용

원격 조사 시점에는 PR92 이후의 PR이 없었다. 상세 목록은
`evidence/PR_SURVEY.json`이다. PR-N01~06은 계획의 과제 ID이며 실제 GitHub PR
번호가 아니다.

| 기존 근거 | 이미 확보된 내용 | 이번에 추가할 부분 |
|---|---|---|
| PR19, Givens candidate | incremental Givens, 원래 residual 재확인, 진단 작업 과금 | 역사적 기대값의 현재 CI 불일치부터 해석; 무조건 승격 금지 |
| PR29 및 stage certificate 계열 | projected residual과 원래 stage target의 차이 | 원래 공간 residual에서 endpoint/physical error로 이어지는 계약 |
| PR66/68/69 | joint Chebyshev/Laguerre phi action, 계수 enclosure, 실패한 scaling 후보 | 다른 평가 방향과 fused-only 연산 감소 |
| 최신 `laguerre_adjoint.rs` | signed output-adjoint와 Bernstein envelope | backward Clenshaw의 local-defect 전달로 인증 준비 비용 절감 가능성 |
| 최신 `polynomial_router.rs` | Leja/Chebyshev/Laguerre guarded routing, 실패 비용 | 현재 dense symmetric provider와 향후 matrix-free provider를 구별 |
| `conditioned_polynomial.rs` | chart/epoch/norm identity와 stored-coefficient defect | 적합한 구조가 있으면 일반 반복보다 우선 비교 |
| coupled `algebra.rs`, `causal.rs` | 8-stage 공통 소행렬 분해, prefix/input 재사용, finite causal propagation | 강한 forward comparator 대비 전체 작업·span 비교 |
| PR91/92 | 구조적 residual enclosure, scalar/rational cubic root, 정수 same-window 비교 | 실제 감쇠를 반영한 오차 계획, variable-step physical transport, 미래 cost 조건 |
| `quotient_flow.rs` | 선언된 damped-rotation flow와 Lawson-DOPRI5 | 다른 선언된 finite-algebra/mean-field flow 및 보존 구조별 분기 |

PR69의 Laguerre positive-majorant 실패를 최신 구현의 미해결 상태로 혼동하지
않았다. 이후 signed-adjoint complete-total admission이 이미 추가되었다.
현재 signed-adjoint의 차수 128 및 subdivision depth 3은 자원 정책이지 수학적
상수가 아니다. 새 증명이 나와도 기존 경로의 제한을 즉시 제거할 근거는 아니다.
과거 warm Chebyshev의 제한된 3.17배 기록과 continuous scaling의 0/22 차수
감소 실패는 역사적 근거로 보존하며 새 성능 주장에 합산하지 않는다.

## 3. Polynomial 대안의 핵심: fused backward Clenshaw

현재 다섯 `Distinct` 입력은 다섯 vector recurrence를 실행한다. 최종 합만
요구하면 다음 항등식으로 하나의 recurrence에 결합할 수 있다.

\[
F_m=\sum_{j=0}^{4}\sum_{k=0}^{m}c_{kj}P_k(X)w_j
    =\sum_{k=0}^{m}P_k(X)d_k,\qquad d_k=\sum_{j=0}^{4}c_{kj}w_j.
\]

Laguerre의 경우

\[
b_k=d_k+\frac{(2k+1)I-X}{k+1}b_{k+1}
       -\frac{k+1}{k+2}b_{k+2},\qquad b_{m+1}=b_{m+2}=0
\]

를 역순으로 계산하면 결과는 `b_0`이다. 각 recurrence의 모든 실제 rounding과
operator evaluation 오차를 additive defect \(\delta_k\)로 표현할 때

\[
\widehat b_0-F_m=\sum_{k=0}^{m}L_k(X)\delta_k.
\]

이는 local impulse가 Laguerre의 같은 three-term recurrence를 따라 전달된다는
사실과 선형성으로 증명된다. Self-adjoint \(X\), spectrum \([0,L]\)이면 DLMF
18.14.8의 \(|L_k(x)|\le e^{x/2}\)로

\[
\|\widehat b_0-F_m\|\le e^{L/2}\sum_k\|\delta_k\|
\]

를 얻는다. Chebyshev Clenshaw에서는 spectrum이 실제로 \([-1,1]\) 안일 때
전달 상수가 1이다. Rounded chart가 구간을 벗어나면 확장 구간의 증가 인자를
다시 사용해야 한다. 수식의 간결함은 forward 실행의 defect에 이 bound를
그대로 적용해도 된다는 뜻이 아니다. 계산 방향을 바꾸고 새 local defects를
실제로 감싸야 한다.

**정확한 작업 수 결과:** 같은 degree m, 다섯 distinct 입력, fused-only 출력에서
vector product는 \(5m\to m\)이다. `SameVector`는 이미 m회이므로 이 이득이
없다. 다섯 개별 column을 반환하는 계약에도 이 shortcut을 적용할 수 없다.
입력 결합, 계수 enclosure, 실제 local error와 입력/출력 normalization 비용은
여전히 필요하다. 상세 증명·가정·실험 대비군은 `polynomial/THEORY.md`에 있다.

### Krylov를 대체하거나 보완하는 다른 가지

| 후보 | 필요한 구조 | 줄일 수 있는 비용 | 남는 비용·위험 |
|---|---|---|---|
| exact/conditioned quotient | 전체 공간의 저차 관계 또는 엄밀한 near-annihilator | 큰 vector polynomial의 차수 자체 | 관계 증명, chart conditioning, 원래 residual |
| Chebyshev resolvent | SPD 또는 명시된 weighted SPD, spectrum enclosure | orthogonalization, basis 저장, global reductions | degree만큼의 순차 action; CG/shared coupled basis가 더 쌀 수 있음 |
| backward Laguerre | 검증된 self-adjoint 양의 chart와 total budget | fused actions, 일부 envelope 준비 | 큰 L에서 exp(L/2)가 불리함; 중간 벡터의 rounding |
| Leja | scalar approximation domain와 matrix-function error certificate | 적은 recurrence 저장, nested degree | 계수·점·scaling setup과 비정규성 인증 |
| ellipse Chebyshev/Faber | numerical-range ellipse enclosure | 비정규 영역의 짧은 polynomial action | spectral interval만으로는 불충분 |
| 구조적 P와 polynomial correction | 전체 공간 \(\|I-AP\|\le\theta<1\) | 기존 cheap inverse의 재활용 | 각 보정과 잔차 검증; 실패 비용 |
| polynomial warm start + Krylov | 해당 target의 true-residual acceptance | 쉬운 부분을 저렴하게 처리 | fallback까지 전체 비용; phi token은 현재 Krylov로 자동 대체 불가 |

SPD \(\sigma(A)\subset[a,b]\), \(0<a<b\)에서

\[
p_m(z)=\frac{T_m((c-z)/d)}{T_m(c/d)},\quad
c=(a+b)/2,\ d=(b-a)/2,
\]

\[
\|r_m\|\le\frac{2q^m}{1+q^{2m}}\|r_0\|,
\quad q=\frac{\sqrt{b/a}-1}{\sqrt{b/a}+1}.
\]

이는 임의 safety factor가 아니라 spectral theorem과 Chebyshev alternation의
정확한 worst-case 상수다. 최소 degree는 해당 부등식을 만족하는 최소 정수로
선택하며, rounded `acosh`의 ceiling만으로 승인하지 않는다. \([1,9]\),
상대 residual factor \(10^{-8}\) 예제에서는 정확히 28이다.

비정규 반례 \(A=I+KN\), \(N^2=0\), \(p(z)=1-z\)에서는 고유값이 모두 1인데
\(\|p(A)\|_2=|K|\)이다. 동시에 \(A^{-1}=I-KN\)이므로, 이 사례의 좋은 경로는
일반 SPD Chebyshev 추정이 아니라 기존 저차 구조다. 더 일반적인 numerical
range에는 Crouzeix–Palencia의 증명된 상수 \(1+\sqrt2\)를 사용한다.

근사 역연산 \(P\)에 대해서는 \(D=I-AP\), \(Q_r=P\sum_{j<r}D^j\)로
\(AQ_r=I-D^r\)를 얻는다. A와 P가 교환한다는 가정은 필요 없다. 이것은 선언된
exact block·quotient의 적용 범위를 bounded perturbation으로 넓히는 실용적
hybrid 구성이다.

## 4. PR-N02: root 비용뿐 아니라 필요한 step 수를 줄인다

\(z'=-\phi(z)\), \(\phi(z)=kz+gz^3\), \(k>0,g\ge0,Y\ge0\)에 대해
\(L(u)=k+3gu^2\), \(D_h=1+hk/2\)라 하자. 정확해의 양 끝점 u,v를 잇는 chord를
쓰면 midpoint increment residual은 두 비음수 항 A,B의 차이다.

\[
A\le\frac{L(u)^2\phi(u)h^3}{12},\qquad
B=\frac{ghm(u-v)^2}{4}\le\frac{L(u)^2\phi(u)h^3}{12}.
\]

두 번째 bound는
\(L(u)^2-3gu\phi(u)=k^2+3kgu^2+6g^2u^4\ge0\)를 사용한다.
따라서 모든 h>0에서 \(|A-B|\le\max(A,B)\)이고 local endpoint 오차는

\[
|M_h(u)-v|\le\frac{L(u)^2\phi(u)}{12D_h}h^3.
\]

기존 1/8을 1/12로 개선하는 유한-step 증명이다. Midpoint map은 전역
nonexpansive이므로 수치 오차는 별도로 운반하고 truncation 항에만 정확해의
감쇠를 적용한다. \(q=(1+kh)^{-1}\)이면 \(z_j\le Ye^{-kjh}\le Yq^j\)이고,

\[
{\cal T}_N=\frac{h^3}{12D_h}
\left[k^3Y S_N(q)+7k^2gY^3S_N(q^3)
+15kg^2Y^5S_N(q^5)+9g^3Y^7S_N(q^7)\right].
\]

여기서 \(S_N(r)=\sum_{j<N}r^j\)이다. Dyadic N에서는
\((p,s)\mapsto(p^2,s(1+p))\)로 cancellation 없이 O(log N) 양수 연산으로
계산한다. 검증되지 않은 transcendental library가 필요하지 않다.

정확 유리수 fixture \(k=1,g=1/2,Y=T=1,\epsilon=10^{-5}\):

| 계획 | N=256 truncation bound | 최소 허용 dyadic N |
|---|---:|---:|
| 기존 1/8 | 1.7846536915e-5 | 512 |
| 1/12만 | 1.1897691277e-5 | 512 |
| 1/12 + rational decay | 3.3853071299e-6 | **256** |

Root residual, state addition, time-label 오차를 모두 포함하는 기존 prefix gate를
유지해야 한다. 1/12만으로는 이 예제의 grid가 줄지 않는다는 부정적 대비군도
보존했다. Native outward planner와 root backend 교차 비교는 아직 수행할 일이다.

## 5. PR-N03/N05: causal algebra와 physical certificate 연결

기존 \(P=11^T/n,Q=I-P\)의 two-channel 구조에서 accepted step의 선형 부분은
\(S_k=\sigma_{k,t}Q+\sigma_{k,c}P\)다. 동일 projector를 유지하면 variable h에도
미래 전달은 두 scalar product로 닫힌다. Full-stage residual의 endpoint weights도
기존 두 scalar adjoint를 step마다 재사용한다. 정확한 block norm은

\[
\nu_n(t,c)=\left|t+\frac{c-t}{n}\right|
 +(n-1)\left|\frac{c-t}{n}\right|.
\]

이를 통해 stage 잔차, local truncation, endpoint/clock rounding을 물리 단위로
연결할 수 있다. 미지의 미래 adaptive grid에 이 a posteriori product를 그대로
예산으로 사용하는 것은 허용되지 않는다. Projector/model 변경도 새 증명이 필요하다.

더 직접적인 후보는 선언된 affine model의 두-channel exact flow다. 이 경우
\(\zeta_k=\widehat y_{k+1}-F_{\Delta_k}(\widehat y_k)\)를 전체 local physical defect로
쓰면

\[
E_{k+1}\le e^{-(d-\kappa)\Delta_k}E_k+\|\zeta_k\|_\infty.
\]

따라서 정확 flow가 더 싸다면 8-stage 계산 자체를 생략하는 방향을 비교해야 한다.
실제 구현은 exp/phi의 outward enclosure와 mean/lift/clock 비용을 포함해야 한다.

일반 strictly causal stage에서는 이미 nilpotent majorant 정리가 있다. 이를
실제 raw-U target에 맞추면 full residual \(\rho\)에 대해

\[
e\le\omega\sum_{\ell=0}^{s-1}B^\ell\rho,
\quad B_{ij}=\omega(h\gamma L_i|a_{ij}|+|\gamma c_{ij}|),\ j<i.
\]

Carrier-aware inverse bound D와 nonlinear remainder majorant N을 쓰면
\(e\le\sum_{\ell<s}(DN)^\ell D\rho\)가 된다. 두 식은 전역 contraction을
요구하지 않는다. 대신 유효한 domain, inverse/remainder bounds와 원래 residual이
필요하며, 바뀐 carrier의 temporal order는 별도 증명이다.

기존 cache 예제의 56→28 RHS 절감은 강한 forward comparator의 7회보다 여전히
크다. 실제 병렬 이득은 batch RHS 비용, pool/barrier, projection/solve, certificate,
실패와 재사용 기간을 모두 합쳐 판단한다. Polynomial three-term recurrence도
degree 방향으로는 순차 의존성이 있다. 예외적으로 미리 정해진 affine step은
\((S_2,v_2)\circ(S_1,v_1)=(S_2S_1,S_2v_1+v_2)\)의 associative scan을 사용해
O(log N) span을 얻지만, 일반 adaptive nonlinear acceptance에는 바로 적용되지 않는다.

## 6. PR-N04/N06: adapter와 root 내부의 비용을 구분

| 계층 | 먼저 줄일 비용 | 정확한 조건 |
|---|---|---|
| adapter/session | winning probe의 재구축·FSAL refresh | full session, controller, schedule, budget, epoch를 그대로 이어감 |
| root workspace | Vec allocation/reinitialization | capacity 재사용과 operator history 재사용을 구별 |
| root continuation | 불필요한 중간 lambda station | invariant ball과 원래 target의 branch certificate |
| adapter residence | 비싼 재탐색과 잘못된 체류 | 미래 variation certificate; 관측 ratio만으로는 불충분 |

\(G_\lambda=(1-\lambda)a+\lambda G\)가 반지름 r인 ball에서 q-Lipschitz이고
\(e\ge\|G_\lambda(c)-c\|, b\ge\|G(c)-a\|\)이면 충분한 station 증가량은

\[
\delta\le\frac{(1-\lambda q)r-e}{qr+b},\quad
\delta\le1-\lambda,\quad(\lambda+\delta)q<1.
\]

반지름과 q는 모델에 의해 증명된 값이어야 한다. q>=1이면 이 식이 lambda=1을
보장하지 않는다. 직접 endpoint가 이미 contractive하면 continuation은 추가
비용일 수 있다. Scalar cubic은 analytic strong monotonicity와 bracket으로
Newton 없이 branch를 인증할 수 있다. 반면 \(x^2-p=0\)의 두 근은 residual이
모두 0이므로 residual만으로 물리적 branch를 선택할 수 없다.

미래 동일-comparison window에서 비용 차이의 초기 하한 d와 변화 bound V가
인증되는 경우에만 m-step saving의 다음 하한을 얻는다.

\[
\sum_{j=1}^{m}(C_B(j)-C_A(j))\ge md-\frac{Vm(m+1)}2.
\]

\(mV<d\)와 위 값이 switching/calibration 비용 S보다 크다는 조건이 strict
residence와 payback을 결정한다. ODE의 smoothness는 adaptive solver 작업 수의
smoothness를 뜻하지 않는다. Trace-margin/variation certificate는 열린 연구
과제이며, 그 인증 비용이 productive comparator보다 클 가능성도 비교해야 한다.

## 7. 기하 구조를 polyalgorithm의 독립적인 조건으로 사용

사용자 지정 DAE 논문은 유효한 reduction/reconstruction, rank, smoothness와
좌표 미분을 요구한다. Section 4의 symmetry 구조는 linear time-varying DAE에
대한 것이며, 논문 자체가 nonlinear pendulum에 그 구성을 적용할 수 없다고
명시한다. 이를 일반 RHS-only DAE solver의 근거로 확대하지 않았다.

Separable Hamiltonian에서는 선언된 force/kinetic subflow만으로 Verlet을 쓸 수
있다. Oscillator의 정확한 fixed-step 경계는 \(0<h\omega<2\)다. 그러나
\(\omega=1,h_1=1,h_2=19/10\)은 둘 다 이 조건을 만족하면서

\[
\det(M(h_2)M(h_1))=1,\quad
\operatorname{tr}(M(h_2)M(h_1))=-9661/4000<-2
\]

이므로 반복 합성은 불안정하다. State-dependent step choice는 symplecticity
자체도 잃게 할 수 있다. Geometric route와 dissipative route는 물리 요구와
인증 조건을 포함해 선택해야 한다.

반면 symplectic RK 계수 조건 아래 inexact homotopy root가 발생시키는 quadratic
invariant drift는 stage residual·RHS error·endpoint rounding으로 정확히 분해할
수 있다. `geometry/THEORY.md`의 G20~G22는 이 tolerance 배분의 근거를 제공한다.
Invariant drift bound를 정확한 symplecticity 보장으로 바꾸어 표현하지 않았다.

## 8. 남은 PR별 실행 방향과 열린 이론

| 과제 | 우선 구현·검증 | 이론적으로 아직 남은 것 |
|---|---|---|
| PR-N01 | fused-only Clenshaw; 기존 forward-adjoint 대비 total cost | 새 local-defect outward kernel, matrix-free provider authority; nonnormal enclosure |
| PR-N02 | 1/12+rational decay planner, old/new grid × 같은 root backend | 표현 가능한 rounding/prefix 예산 및 모든 경계 사례 |
| PR-N03 | variable-step two-channel transport; exact-flow comparator | 실제 physical defect enclosure와 adaptive consumer |
| PR-N04 | 기존 with_session 안에서 winning session 계속 실행; workspace capacity | generic branch/domain certificates, endpoint 불수축일 때의 보장 |
| PR-N05 | raw-U residual→endpoint budget, forward/batched 작업·span 비교 | 바뀐 carrier의 nonlinear closure, stiff-uniform order, 유효 domain |
| PR-N06 | 비용 종류와 epoch를 명시; 기존 productive comparator 유지 | 미래 counterfactual cost variation/trace certificate |

프로젝트의 고유 아이디어 전체가 잘못되었다는 증거는 발견하지 않았다. 검토된
한계는 각각 정확한 가정의 부재 또는 비용상 불리함에 속한다. 코드 최적화로
해결되는 allocation, fusion, cache와 달리 일반 비선형 order·물리 branch·미래
cost 인증은 명시적인 추가 이론을 요구한다.

`LOCAL_DEVELOPMENT_PLAN.json`은 기존 23-node/34-edge qualification DAG와 correction
예산을 보존하면서 17개 후보를 각 PR에 연결한다. 새 후보의 R2/R4가 완료되어도
native 구현 상태를 `pending_local`에서 완료로 바꾸지 않는다. 기존 seven Q-*
qualification tasks도 해결됐다고 표시하지 않는다. 상세 handoff는
`LOCAL_CODEX_HANDOFF_KO.md`이며, 지원 검산/패키지 검증은 `verify.py`에서 실행한다.

## 9. 문헌 및 검증 근거

사용자 지정 네 원문과 polynomial 1차 문헌의 URL·해석 범위는 `SOURCES.json`에
정리했다. 각 lane의 THEORY.md에는 증명과 code/experiment contract가 있다.
`evidence/*_RESULTS.json`은 세 등록 후 실행의 실제 출력이고 receipt는 source
SHA-256, command, exit status와 output hash를 보존한다.

검산은 일반 증명을 대신하지 않으며, Python Fraction 결과를 compiled binary64
certificate로 승격하지 않는다. 이번 새 연구의 결과는 이론과 재현 가능한
지원 계산이며, 실제 성능·release 적합성은 갱신된 local 계획의 명시적 다음 단계다.
