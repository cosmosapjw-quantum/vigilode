# N01 비선형 source-owned 원잔차 / 물리 stage-error 인증 정리

2026-10-29. Based on VigilODE PR114 HEAD `a62c4215fa2c329c6a48be2ccf544ac056b747a5`; preregistration commit `99aabee75957e349b3fea525f072c84b9c296757`. 작업 범위는 Robertson형 mass-action 3성분에 한정되며 BASS chemical33 또는 일반 nonlinear `OdeProblem`에 대한 인증은 아니다.

## 1. 정확한 실수 모델 및 단위

상태 `z=(a,b,c)`는 무차원 농도, `k1,k2,k3 >= 0`는 1/time, `sigma>=0`는 time으로 해석한다. 모든 binary64 입력을 그에 해당하는 정확한 실수로 해석한 다항식:

`f(z)=(-k1*a+k2*b*c, k1*a-k2*b*c-k3*b^2, k3*b^2)`.

`J(z)=[[-k1,k2*c,k2*b],[k1,-k2*c-2*k3*b,-k2*b],[0,2*k3*b,0]]`.

직접 미분 `J=Df`; `sum_i f_i=0`, 모든 열 `sum_i J_ij=0`이다. `a,b,c>=0`일 때 각 좌표 경계에서 외향 음의 흐름이 없어 양의 orthant는 이 연속 ODE에 전방 불변이다. 마지막 성질은 **연속 ODE**에 관한 것이고 개별 numerical trial의 positivity를 보장하지 않는다.

고정 reference state `z_ref`, live `z`는 동일 source의 비음수 box 안에 있어야 한다. Model epoch와 실제 clock은 각 callback에서 source owner가 갱신하며 과거 trial snapshot은 인증 시 거부된다. Stage 문제는 정확한 실수 `A_*=I-sigma J(z)`, `b_*=f(z)`, `A_*x_*=b_*`. `x` 및 `b`의 성분 단위는 1/time, `w_i,p_i>0` 역시 이 단위의 참조 scale이다. Operator J의 단위는 1/time. `A_*`는 무차원.

## 2. 반드시 포함되는 표현 오차

소스가 anchor에서 생성한 binary64 `J0=fl(J(z_ref))`, `b0=fl(f(z_ref))`는 정확한 수학적 `J(z_ref)`·`f(z_ref)`와 일반적으로 다르다. *이 차이는 반드시 불확실성에 포함*되어야 한다. Native outward interval evaluation에서

`deltaJ >= max_i sum_j mag(J_exact_ij(z)-[J0_ij]) w_j/w_i`,
`deltaB >= max_i mag(f_exact_i(z)-[b0_i])/w_i`.

따라서 (동일한 정확한 실수 모델에 대해) `||J(z)-J0||_(w,infty)<=deltaJ`, `||f(z)-b0||_w<=deltaB`이고, reference rounding과 live state 변화가 함께 포함된다. 별도 source-opaque JVP diagnostic에서 `jvp_callback_rounding_upper`는 `||JVP_float(z,v)-J_exact(z)v||_w`를 outward로 감싼다. 이것은 외부 generic callback에 대한 보편적 오차 주장과 다르다.

반면 native source가 직접 만든 `y=fl[(I-sigma J0)xhat]`는 `eta_apply >= ||y-(I-sigma J0)xhat||_w`라는 interval bound를 별도로 계산한다. `eta_apply`를 0으로 두거나 외부 callback이 추정한 값을 사용할 수 없다.

## 3. 원잔차 및 물리 stage-error의 수학적 부등식 (directly derived)

`||v||_w=max_i |v_i|/w_i`, 행렬 유도 노름 `||J||_(w,inf)=max_i sum_j |J_ij|w_j/w_i`,

`mu_w(J)=max_i[J_ii+sum_(j != i)|J_ij|w_j/w_i]`.

항등식:

`r_* = (b_*-b0)+(b0-y)+(y-(I-sigma J0)xhat)+sigma(J_*-J0)xhat`.

따라서

`rho_up = upward(||b0-y||_w + deltaB + eta_apply + sigma*deltaJ*||xhat||_w)`

는 `||r_*||_w`의 상계다. Lipschitz inequality `mu_w(J_*) <= mu_w(J0)+deltaJ`와 resolvent 적분에서, `m_lower=downward[1-sigma*(mu_w(J0)_up+deltaJ)]>0`이면

`||xhat-x_*||_p <= C_wp rho_up /m_lower`, `C_wp=upward(max_i w_i/p_i)`.

`original_tolerance`와 `physical_stage_tolerance`를 **동시에** 통과한 경우에만 output bitwise commit. 기존 GMRES projected/left-preconditioned stopping criterion은 admission의 증거가 아니다. `m_lower<=0`, nonfinite/overflow, source mismatch, box mismatch, epoch/clock mismatch는 FAIL CLOSED.

전제가 아닌 것: 전체 시간 slab에서의 모든 possible state bound, 전역 time-step error, 근사 Newton solve convergence, `OdeProblem`의 *임의* callback, BASS physical33 JVP authority. 본 정리의 정확성은 exact polynomial source + source-owned represented callbacks + outward arithmetic라는 제한된 class에 묶여 있다.

## 4. 독립 Wolfram exact rational 계산

`k=(1/16,8,4)`, `z_ref=(7/8,1/16,1/16)`, `z=(55/64,5/64,1/16)`, `sigma=1/16`, `w=(1,1,1)`, `p=(1,2,1)`.

정확한 `J==Df`: True. Mass conservation: 0. Column sums: `{0,0,0}`.

`mu_w(J0)=15/16`, `deltaJ=1/4`, `deltaB=9/512`, `margin=237/256`.

`rho_actual=275/262144 <= rho_bound=6163857/168034304` (True),

`stage_error_actual=1/1024 <= stage_error_bound=2054619/51854336` (True).

이 독립 기호 검산은 *정확한 유리수* 사례다. 실제 binary64 outward 커널의 전 범위 correct rounding 증명은 아니며 기존 core의 directed.rs에 조건부로 의존한다.

## 5. 수치 실행과 보수성

독립 `Fraction.from_float` verifier가 Rust CSV 128행을 exact binary64 rational로 읽어 모델 RHS/J, actual original residual, 3x3 exact stage solve와 callback JVP rounding을 재구성하여 10개 불등식/항등식 x 128 = 1280/1280을 확인했다. 이상 수치는 이전 synthetic affine result가 아니다. 실제 JVP·stage owner로 생성한 output에 대한 **point-stage bounded acceptance**이지 full-adaptive native solver의 인증은 아니다.

관측된 물리 stage-error 상계가 실제 오차보다 보수적이다. `C_wp`, matrix-measure resolvent와 `rho` 삼각부등식의 중복 여유 때문이다. 상계가 크다는 것과 틀렸다는 것은 분리한다. 잔차 플롯은 대체로 diagonal 부근, stage-error 플롯은 diagonal 위에 있었다.

## 6. 코드 연결 및 남은 blocker

`rodas5p-core::n01_mass_action_owner` = 완전 source-owned 모델과 point certificate. `rodas5p-integrators::n01_mass_action_client` = 동일 Arc<Mutex<owner>>에 묶인 opt-in `OdeProblem` RHS/Jacobian/JVP callback + epoch. 실제 RODAS5P solver의 자동 stage admission에는 아직 연결하지 않았고 native repository 전체 테스트도 이 세션에서 실행하지 않았다. Existing `problems::robertson_problem()`과 방정식은 같지만 float 연산 경로/비트 동등성을 가정하지 않는다. 로컬 Rust mirror 28/28와 independent exact oracle 1280/1280은 *scoped*이며 R6/C9의 별도 작성자 독립 검토는 NOT_RUN이다.

문헌 맥락: Simoncini & Szyld (2003), DOI: 10.1137/S1064827502406415; van den Eshof & Sleijpen (2004), DOI: 10.1137/S0895479802403459. 이 문헌들은 inexact Krylov residual-gap 분석을 뒷받침하며 본 source-specific 정리의 독립 증명은 아니다. Lin & Stadtherr (2006) interval ODE parameter enclosure는 이후 whole-slab 과제의 맥락으로 한정한다.