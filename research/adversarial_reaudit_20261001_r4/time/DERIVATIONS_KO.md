# R4 시간 의미론: 독립 수학 유도

이 문서는 탐색자가 직접 유도한 식이다. 문헌의 정리를 인용하거나 CAS가
검증했다고 주장하지 않는다. 실행 관측과 최종 독립 판정은 별도 파일에 있다.
시간은 임의의 공통 단위 (T), 상태는 무차원으로 잡는다. 평행이동과 양의
시간 단위 변환에서 같은 물리적 경로를 비교하며, 서로 다른 floating-point
입력으로 변한 함수의 조건수를 알고리즘 오류와 구별한다.

## 1. 비균일 BDF2의 선형 함수 재현

현재 간격을 (h=t_{n+1}-t_n>0), 이전 간격을
(k=t_n-t_{n-1}>0), 비를 (r=h/k)라고 하자. 세 시간점에서의 이차
Lagrange 보간다항식을 (t_{n+1})에서 미분하고 (h)를 곱하면

\[
 a_0y_{n+1}+a_1y_n+a_2y_{n-1}=h f_{n+1},\qquad
 a_0=\frac{1+2r}{1+r},\quad a_1=-(1+r),\quad
 a_2=\frac{r^2}{1+r}.
\]

상수 함수에서 (a_0+a_1+a_2=0), 선형 함수
(y_{n+1}=y_n+vh, y_{n-1}=y_n-vk)에서는
(a_0h-a_2k=h)이다. 따라서 상수 속도 (f=v)의 선형해를 정확히
재현한다. (r=1)이면 식을 2배해 (3y_{n+1}-4y_n+y_{n-1}=2hv)를 얻는다.

그러나 실제 (h\ne k)에 마지막 등간격 식을 쓰면 정확한 선형해의 잔차는

\[
 3(y_n+vh)-4y_n+(y_n-vk)-2hv=v(h-k)
\]

이다. 이는 (f'=0)인 문제의 적분 오차가 아니다. 잘못 선택한 시간격자
계수의 일관성 결함이다. 본 실행의 `Fraction.from_float` oracle은 실제 저장된
모든 시간점과 속도를 정확한 유리수로 바꾸고, 해당 두 recurrence를 따로
계산한다. 생산 코드의 계수 함수를 import하지 않는다.

`same_step`의 `max(|h|,|k|,1)`은 숫자 1에 시간 단위를 부여하는 암묵적
절대 기준이다. (t\mapsto 2^q t, v\mapsto2^{-q}v)로 같은 무차원 경로를
표현해도 같은 단계비가 equality cutoff의 다른 쪽으로 이동한다. 그러므로
정확히 같은 간격만 등간격 전용 경로로 보내거나 모든 양의 간격에 실제 (r)를
써야 한다. 상대 비교를 쓸 경우 생략되는 (v(h-k))의 효과를 별도 오차로
정의해야 하며, 절대 1 floor로 대신할 수 없다.

## 2. 비대칭 두 단계와 Richardson 계수

정확한 초기값에서 시작한 차수 (p) 방법의 한 단계 leading error가
(C H^{p+1})이고, 분할 구간에서 같은 (C)를 사용하며 낮은 차수 전달항을
무시할 수 있는 매끄러운 국소 regime을 가정한다. (H=h_1+h_2),
(\theta=h_1/H\in(0,1))이면

\[
 \eta=\theta^{p+1}+(1-\theta)^{p+1},\qquad
 e_c\simeq C H^{p+1},\qquad e_f\simeq\eta C H^{p+1}.
\]

따라서 (y_f-y_c\simeq-(1-\eta)CH^{p+1})이고

\[
 e_f\simeq-\frac{\eta}{1-\eta}(y_f-y_c).
\]

오차 크기에는 (\eta/(1-\eta))가 필요하다. 동등한 두 절반에서는
(\eta=2^{-p})라서 (1/(2^p-1))로 환원된다. 비대칭 분할에는 이 특수값을
그대로 사용할 수 없다. 이는 leading-order 오차 추정식이며 엄밀한 ODE 전역
상계가 아니다. 계수가 구간에서 빠르게 변하거나 BDF startup처럼 순서와
history가 함께 바뀌면 별도의 유도가 필요하다.

이번 고정 witness는 (y'=2(t-t_0)/H^2, y(t_0)=0)이며 정확한 끝값은 1이다.
Radau IIA 1-stage는 implicit Euler이므로 RHS가 상태에 의존하지 않는 여기서는

\[
 y_c=2,\qquad y_f=2\frac{h_1^2+h_2H}{H^2}.
\]

큰 epoch에서 (H=3\,\mathrm{ULP}), 반올림된 가운데점이
((h_1,h_2)=(2,1)\,\mathrm{ULP})이면

\[
 y_f=14/9,\quad e_f=5/9,\quad y_c-y_f=4/9,\quad
 \eta=5/9,\quad\frac{\eta}{1-\eta}=5/4.
\]

따라서 이 witness에서는 일반 leading-order 근사가 정확한 대수식으로
강화된다. (mathrm{atol}=1/2), 매우 작은 rtol이면 기존 equal-half 추정의
정규화값은 약 (8/9<1)인 반면 실제 한 macro-step 오차는 약 (10/9>1)이다.
입력 함수는 analytic partial-time derivative와 영 Jacobian/JVP를 제공한다.
이 현상을 함수미분 근사나 Newton 미수렴과 혼동하지 않는다.

후보의 범위는 이 상수 leading coefficient witness와 geometry에 맞는
오차식이다. 모든 adaptive integrator가 사용자의 atol에 대한 수학적 보증을
준다는 주장이 아니다. 문제는 본래 equal-half 조건에서 유도한 추정치를
새로운 unequal-half 실행에 그대로 붙였다는 데 있다.
