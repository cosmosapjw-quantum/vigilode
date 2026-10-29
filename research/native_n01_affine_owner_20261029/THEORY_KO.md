# VigilODE N01 시간 의존 affine source 인증: 유도 및 적용범위

Date: 2026-10-29. Parent PR113 HEAD `beee9aaf4f83eaedb5e4a3167cd3f42059215ec8`. Native PR111 GMRES blob remains `90bfaaa62afeda9dec27b3dfd594e67cd3034da1`.

## 정의와 전제

정확한 수학적 모델을 binary64로 표현된 입력을 **정확한 실수**로 승격하여 정의한다:

\[ f(t,z)=[J_0+(t-t_0)K]z+b_0+(t-t_0)q. \]

n>0, 모든 계수 유한, `sigma>=0`, `w_i>0`, `p_i>0`; 실제 stage 문제는 `A_* x_*=b_*`, `A_*=I-sigma J_*`, `J_*=J_0+(t-t_0)K`, `b_*=b_0+(t-t_0)q`. 정확한 원잔차에서 `||v||_w=max_i|v_i|/w_i`, 물리 오차에서 `||v||_p=max_i|v_i|/p_i`. `t`와 `t0`는 수학적 clock이며 적분기 전체 error가 아니다.

## 유도

1. 행렬 유도 노름 `||M||_{w,inf}=max_i sum_j |M_ij|w_j/w_i`, weighted lognorm `mu_w(J)=max_i[J_ii+sum_{j != i}|J_ij|w_j/w_i]`로 정의한다.
2. `D=|t-t0|`이면 source law에 의해 `||J_*-J0||_w<=D||K||_w=:delta_J`와 `||b_*-b0||_w<=D||q||_w=:delta_b`. 수치 구현은 각각의 오차 항을 outward로 위에서 근사한다.
3. source-owned `y`는 `fl[(I-sigma J0)xhat]`이다. *동일한 표현점*의 interval evaluation으로 `eta_apply>=||y-(I-sigma J0)xhat||_w`를 검증한다. 외부 callback으로부터 주장된 eta를 받아들이지 않는다.
4. 원잔차 항등식:

\[r_*=b_*-A_*\hat x=(b_*-b_0)+(b_0-y)+(y-A_0\hat x)+\sigma(J_*-J_0)\hat x.\]

따라서 `||r_*||_w <= rho0+delta_b+eta_apply+sigma delta_J ||xhat||_w`. 모든 항을 outward 연산하면 `rho_up`이 된다.
5. matrix measure 부등식 `mu_w(J_*) <= mu_w(J0)+delta_J`. `sigma[mu_w(J0)+delta_J]<1`이면 resolvent 적분

\[(I-\sigma J_*)^{-1}=\int_0^\infty e^{-s}e^{\sigma sJ_*}ds\]

및 `||exp(sJ*)||_w <= exp(s mu_w(J*))`로부터 `||(I-sigma J*)^-1||_w <= 1/[1-sigma(mu_w(J0)+delta_J)]`를 얻는다.
6. `||v||_p <= max_i(w_i/p_i)||v||_w`를 적용하여

\[||\hat x-x_*||_p \le C_{wp} \rho_{up}/[1-\sigma(\mu_{up}+\delta_{J,up})],\qquad C_{wp}=\max_i w_i/p_i.\]

따라서 원잔차 gate와 물리 stage-error gate 둘 다 만족해야 한다. 방정식 `r`는 preconditioned/projected GMRES residual이 아니다.

## 정확한 반례·독립 검산

Wolfram exact rational: `J0={{-1,10},{0,-1}}`, `K=-I`, `b0={1,-2}`, `q={1/2,-1/4}`, `w=p={100,1}`, `sigma=1/4`, `dt=1/16`, `xhat={-12/5,-8/5}`.
- `mu_w=-9/10`, `deltaJ=1/16`, `deltaB=1/64`, `eta=0`
- `rho_actual=3/320 <= rho_bound=13/320`
- `margin=387/320`, physical `error_actual=1/135 <= error_bound=13/387`
- Wolfram booleans: True, True.

경량 검산: Rust1.94.1 interface mirror 15/15 tests, fmt, strict Clippy PASS. 192 binary64 deterministic fixtures were independently reconstructed as exact Fraction values: 1920/1920 inequalities PASS. Physical-stage upper/actual ratio maximum ~3.0946 for this finite sample. 이들은 실제 물리 BASS 자료가 아니며 full native workspace build도 아니다.

## 의무 및 주장 경계

이 new native source class는 *자체 소유 affine law*의 clock/epoch/norm/JVP를 내부에 묶어, 제한된 시간변화와 수치 apply 오차를 인증한다. `OdeProblem`의 임의 비선형 RHS/JVP callback이나 BASS physical33의 실제 source/derivative remainder에는 아직 연결되지 않았다. 일반 비선형 model의 certified Jacobian bound, 실제 callback 오차, stage RHS 조립, adaptive full-IVP 및 누적 시간오차는 `NOT_RUN`이다. native source는 GitHub 공개되었으나 full workspace 통합은 `NOT_RUN`. 이 결과는 공식 N01/production admission이 아니다.

문헌 맥락: Simoncini & Szyld (2003), SIAM J Sci Comput, DOI 10.1137/S1064827502406415; van den Eshof & Sleijpen (2004), SIAM J Matrix Anal Appl, DOI 10.1137/S0895479802403459. 두 문헌은 inexact Krylov residual-gap 동기/방법의 지원이며 이 source-specific theorem 전체를 대신하지 않는다.
