# 유한 몫대수와 인과적 단계 구조를 이용한 ODE 계산 절감

2026-10-23. 읽기 기준 `14c74978b93339a79d7228a324fa77552dae548e`.
이 문서는 첨부 기하학 초안의 구조를 실제 ODE 연산으로 옮기는 이론 부분이다.
입력별 SHA256, 식 위치, 기존 증명 재사용 위치는 `SOURCE_MAP.json`에 기록한다.
새 정확 유리수 검산은 `../results/algebraic_exact_checks.json`, 독립 Wolfram 계산은
`../evidence/algebraic_wolfram.json`에 있다.

## 1. 이번에 옮기는 구조

기하학 초안에서 채택하는 세 가지는 **유한 몫대수에서의 계산**, **일반 chart와 예외 chart의 명시적 분리**, **동일한 물리 출력과 서로 다른 해 witness의 구별**이다. 이들은 각각 다음 계산에 직접 연결된다.

| 초안의 구조 | 실제 수치 연산 | 이번 연결 |
|---|---|---|
| 유한 대수 `Q[a]/(Q28)`에서 좌표를 polynomial로 복원 | 알려진 연산자 관계 `q(A)=0`에서 resolvent를 낮은 차수 polynomial로 복원 | AT1–AT2, 감쇠·회전 native action |
| `a L_H != 0` chart와 `a=0`, `L_H=0`, `c=0`의 별도 처리 | coefficient degree drop, resolvent pole, continuation fold를 다른 원인으로 기록 | AT3, quadratic branch adapter |
| projection 결과의 후보를 원래 네 식에 역대입 | 축약 stage root를 원래 full-space residual에 역대입 | AT4, causal corrector |
| squarefree quotient와 multiplicity를 가진 finite scheme의 구별 | 서로 가까운 eigenvalue와 Jordan multiplicity를 보존하는 quotient evaluation | AT1, confluent exact test |

기하학 논문의 두 번의 normal reflection은 ODE 시간 진화식 자체로 사용하지 않는다. 실제 이식은 위의 대수적 계산 원리를 통한다. v13의 §1.2가 분리한 deck group, monodromy, Galois group 역시 서로 바꾸어 쓰지 않는다. v13에서 철회 상태인 compactified counts 7128/3300은 본 연구의 근거로 쓰지 않는다. smooth cubic의 길이 116/58도 ODE root 개수나 성능 배수로 전이하지 않는다.

첨부 minor revision의 식 (24)–(25)는 특히 유용한 경계 예시다. 잔여 cubic
`C_(r,z)(w)=(1+r)zw^3+(3r-1)w^2+(3r-1)zw+(1+r)`의 discriminant는 `(r,z)=(1/3,0)`에서 0이지만, 해당 polynomial은 nonzero constant로 차수가 내려간다. 따라서 “discriminant=0이면 finite double root”라는 경로 선택은 잘못된다. 실제 알고리즘은 coefficient degree와 원래 정의역을 먼저 확인해야 한다. 이 구조 검토는 논문의 모든 큰 elimination computation을 다시 실행한 것이 아니다.

## 2. 기존 VigilODE 결과를 재사용하는 범위

`docs/reviews/20261011_promotion_theory/POLYNOMIAL_KO.md`의 P1–P3는 이미 다음을 증명했다.

1. Hermitian full-space near-annihilator의 `phi` action 오차 상한.
2. Laguerre/Chebyshev 등 affine recurrence의 quotient compilation.
3. nonnormal finite jet의 종료하는 resolvent와 near-nilpotent 확장.

이 파일의 마지막 수정 commit은 `2e944afc36a20fd20ce33438b2a1d4f4e0a6a5c4`이며, 파일 자체 SHA256도 `SOURCE_MAP.json`에 보존한다. 아래 AT1은 그 내용을 일반 scalar resolvent의 synthetic division 형태로 연결한다. 새 기여는 기존 항등식을 재증명했다고 주장하는 데 있지 않고, 이번 native damping/rotation action과 switching 계약에서 계산 경로를 구체화하는 데 있다.

`research/all_dag_completion_20261018/nodes/A09/PARALLEL_THEORY.md`의 finite-front/binomial error 구조도 재사용한다. 현재 원래 raw-U tableau와 변환된 represented-K tableau의 상삼각 누출을 구별하여, exact causal target에서만 전용 corrector를 허용한다.

## 3. AT1 — root extraction 없는 유한 몫대수 resolvent

`A`는 유한차원 벡터 공간의 선형 연산자이고, `q`는 차수 `d>=1`의 monic polynomial이다. `s != 0`, `q(1/s) != 0`라 하자. 다음을 정의한다.

\[
p_s(z)=\frac{q(z)-q(1/s)}{s q(1/s)(z-1/s)}.
\tag{AT1.1}
\]

분자는 `z-1/s`로 나누어지므로 `p_s`는 차수 `d-1`의 polynomial이다. 그러면

\[
(I-sA)p_s(A)=I-\frac{q(A)}{q(1/s)}.
\tag{AT1.2}
\]

따라서 `q(A)=0`이면 `(I-sA)^(-1)=p_s(A)`다. `s=0`은 identity chart로 직접 계산한다.

**증명.** `1-sz=-s(z-1/s)`를 (AT1.1)에 곱하면 `1-q(z)/q(1/s)`가 된다. polynomial identity에 `A`를 대입한다. 오른쪽이 identity이면 유한차원 공간에서 right inverse는 inverse다. Jordan block, repeated root, nonnormality를 배제하지 않았다. □

**연산량.** 알려진 `q`의 coefficient를 가지고 synthetic division하면 scalar 준비 작업은 `O(d)`, 저장은 `O(d)`다. `p_s(A)v`는 Horner 방식에서 `d-1`회의 `A` action과 `O(d n)` vector arithmetic을 요구한다. 차수 `m>d-1`의 polynomial iterative solve/action에 비해 operator action 수가 정확히 `m-d+1`회 감소한다. `A`의 action이 구조식으로 주어질 때 state Jacobian 조립, JVP/AD, eigensolve, Newton 및 Krylov basis가 필요하지 않다. `q`를 일반 callback에서 찾아내는 비용은 이 정리에 포함되어 있지 않다.

**near-annihilator 확장.** `epsilon=||q(A)||/|q(1/s)|<1`이면 Neumann lemma로

\[
\|(I-sA)^{-1}\|\le\frac{\|p_s(A)\|}{1-\epsilon},\qquad
\|(I-sA)^{-1}-p_s(A)\|\le
\frac{\|p_s(A)\|\epsilon}{1-\epsilon}.
\tag{AT1.3}
\]

이는 `||A||s<1`을 요구하지 않는다. 다만 `q(A)`에 대한 full-space bound를 요구한다. 몇 개 probe vector에서 얻은 작은 polynomial residual은 그 bound를 대신하지 않는다. 계산한 후보 `xhat`의 원래 residual `r=b-(I-sA)xhat`를 평가하면 해 오차는 `(AT1.3)의 inverse bound × ||r||` 이하이다. residual evaluation 오차도 같은 bound 안에 포함해야 한다.

**수치 구현.** 식 (AT1.1)의 `1/s`를 그대로 만드는 것은 극소 `s`에 부적합하다. 저차 구조별로 coefficient 식을 정리하고 scale을 먼저 적용한다. `q(1/s)`가 작다는 것은 resolvent denominator의 문제이며, `q`의 discriminant가 작다는 것과 다르다. 서로 다른 roots가 합쳐져도 `I-sA`가 가역이면 polynomial action은 매끄러울 수 있다. 따라서 root separation만 보고 이 경로를 버릴 필요가 없다.

## 4. AT2 — 감쇠·회전 block의 confluent-safe 해석 action

한 block에서 `R=[[0,-1],[1,0]]`, `A=-kappa I+omega R`, `kappa>=0`라 하자. `R^2=-I`이므로

\[
q(A)=(A+\kappa I)^2+\omega^2 I=0.
\]

AT1은 즉시

\[
(I-sA)^{-1}=
\frac{(1+s\kappa)I+s\omega R}
{(1+s\kappa)^2+(s\omega)^2},\quad s\ge0,
\tag{AT2.1}
\]

를 준다. 더 나아가

\[
e^{tA}=e^{-\kappa t}\{\cos(\omega t)I+\sin(\omega t)R\},\quad t\ge0.
\tag{AT2.2}
\]

각 block의 Euclidean norm에서

\[
\|e^{tA}\|_2=e^{-\kappa t}\le1,\qquad
\|(I-sA)^{-1}\|_2=
[(1+s\kappa)^2+(s\omega)^2]^{-1/2}\le1.
\tag{AT2.3}
\]

**증명.** `(aI-bR)(aI+bR)=(a^2+b^2)I`, `R^T=-R`, `R^T R=I`를 사용한다. 지수함수의 even/odd series를 나누면 (AT2.2), orthogonal rotation 성질로 (AT2.3)이 나온다. `omega=0`에서도 같은 식이 정의되며 `1/omega`나 eigenvector reconstruction이 없다. □

상태가 여러 독립 block이면 전체 action은 `O(n)` 연산과 고정 크기 block 임시 공간으로 끝난다. 변화하는 `omega_j,kappa_j`를 block별로 선언할 수 있지만, 각 time step의 실제 linear part와 일치해야 한다. 알려진 선형 구조를 사용한다는 것은 generic nonlinear Jacobian을 작은 probe로 추정했다는 뜻이 아니다.

Lawson 변환 `y(t)=exp(tA)v(t)`는 constant `A`와 smooth remainder `N(t,y)`에서 `v'=exp(-tA)N(t,exp(tA)v)`를 준다. 원래 변수에서 DOPRI stage를 구성하면

\[
Y_i=e^{c_i hA}y_n+h\sum_{j<i}a_{ij}e^{(c_i-c_j)hA}N(t_n+c_jh,Y_j).
\tag{AT2.4}
\]

DOPRI의 nondecreasing stage times를 이용하면 위 expression에는 nonnegative exponent duration만 필요하다. 실제 구현은 감쇠를 거꾸로 지수화하는 `exp(-tA)`를 만들지 않아도 된다. embedded update에도 같은 변환을 적용한다. smooth constant-`A` 문제에서 classical order는 원래 RK order를 따른다. `N=0`에서는 임의의 `h kappa,h omega`에서 정확한 linear flow를 사용하므로 explicit stability 제한을 제거한다. 강한 nonlinear remainder에서 `h`와 오차상수의 `A` 독립성은 별도 stiff-order 분석의 대상이다.

시간 의존 `A(t)`를 단순히 freeze하면 commutator 및 coefficient variation 항이 생긴다. 이번 constant-block 계약을 그 경우에 적용하려면 variation-of-constants remainder/commutator bound를 새로 전달해야 한다. 또한 phase evaluation의 binary64 argument reduction과 장시간 phase drift는 별도 관측 정확도로 측정한다.

## 5. AT3 — discriminant margin을 가진 Newton-free scalar homotopy

작은 물리 반응 또는 closure가 정확히

\[
F_\lambda(u)=C u^2+a u-b=0,\qquad C=\lambda c,
\quad a>0,\quad0\le\lambda\le1
\tag{AT3.1}
\]

로 주어지고 `Delta=a^2+4Cb>0`라 하자. `lambda=0`의 causal branch `u=b/a`에서 이어지는 regular branch는

\[
u_* =\frac{2b}{a+\sqrt\Delta},\qquad
a+2C u_*=\sqrt\Delta>0.
\tag{AT3.2}
\]

`C=0`에서도 같은 식의 극한이 정의된다. square-root subtraction을 쓰지 않으므로 작은 `C b`에서 두 가까운 양수를 빼는 상쇄를 피한다. `b=0`은 `u=0`으로 직접 처리한다.

**증명.** `Delta-a^2=4Cb`를 사용해 (AT3.1)에 역대입하면 0이다. `a+2C u_*`를 정리하면 `sqrt(Delta)`이므로 다른 quadratic root와 branch가 구별된다. `lambda=0`에서 식은 `b/a`이며 `Delta>0` 구간에 연속이다. □

이 결과는 root finder의 실제 비용을 고정 크기 산술과 square root 한 번으로 줄인다. 일반 nonlinear residual을 quadratic으로 바꾸지는 않는다. polynomial 구조가 client 식에서 정확히 나오거나, 근사 polynomial 오차가 원래 residual과 함께 제어되는 경우에 사용한다.

**원래 residual에 의한 오차 bound.** 후보 `uhat`에서 `dhat=a+2C uhat>=0`이면

\[
|\widehat u-u_*|=
\frac{2|F_\lambda(\widehat u)|}{\widehat d+\sqrt\Delta}
\le\frac{2|F_\lambda(\widehat u)|}{\sqrt\Delta}.
\tag{AT3.3}
\]

실제로 `F(uhat)-F(u*)=(uhat-u*)[a+C(uhat+u*)]`이고 bracket은 `(dhat+sqrt Delta)/2`다. 따라서 discriminant margin `sqrt Delta>=delta>0`와 원래 residual은 branch 오차 예산을 제공한다. 부동소수점 residual의 크기만 측정할 때는 operational diagnostic이고, rounding enclosure까지 주어지면 해당 bound에 포함한다.

**scalar scaling.** `M=max(a,|b|,|C|)>0`로 coefficient 전체를 나누면 `A=a/M`, `B=b/M`, `D=C/M`은 절댓값 1 이하이고 normalized discriminant `A^2+4DB`의 크기는 5 이하이다. 원래 equation의 root는 변하지 않는다. `A`가 underflow하여 0이 되거나 discriminant margin을 계산할 수 없으면 degree가 줄었다고 가정하지 않고 typed refusal을 반환한다. `Delta`가 0에 가까우면 lambda subdivision 또는 다른 chart를 사용하되, 수학적으로 실제 fold에서 branch의 continuation이 끝날 수도 있음을 원래 식으로 판별한다.

드래프트의 geometric fold는 이 quadratic의 discriminant를 대신하는 데이터가 아니다. 연결되는 것은 **coefficient degree 확인 → regular branch 식 → denominator margin → 원래 residual**의 계산 절차다.

## 6. AT4 — raw-U Rosenbrock target의 causal exact root

현 coupled 구현의 orthonormal reduced basis를 `Q` (`n×m`), frozen projected operator를 `J_m=Q^T J_0 Q`, 공통 stage matrix를 `W_m=I-h gamma J_m`로 쓰자. 실제 사용한 raw-U 계수가 모든 `j>=i`에서 `a_ij=c_ij=0`이고 `W_m`이 가역이라고 가정한다. 목표식은

\[
W_m z_i=h\gamma Q^T f\left(y+Q\sum_{j<i}a_{ij}z_j\right)
+\gamma\sum_{j<i}c_{ij}z_j.
\tag{AT4.1}
\]

현재 native coupled route는 autonomous identity-mass 입력을 요구한다. 그러면 주어진 `y,h,Q,J_0`에서 (AT4.1)의 root는 유일하며 **한 번의 forward stage pass**로 구해진다. 현재 stage의 nonlinear Newton iteration이나 현재 stage JVP는 필요하지 않다.

**증명.** `i=0`에서는 RHS가 이미 정해져 있고 `W_m`이 가역이므로 `z_0`가 유일하다. `z_0,...,z_(i-1)`가 정해지면 stage `i`의 RHS도 정해지므로 `z_i`가 유일하다. 귀납법을 적용한다. 이 증명은 global contraction 상수가 1보다 작다는 조건을 요구하지 않는다. □

**계산량.** 한 시도의 공통 `m×m` factorization은 `O(m^3)`, `s`개의 solve는 `O(s m^2)`, basis lift/projection은 `O(s n m)`, stage assembly는 naive `O(s^2 m)`이다. RHS 호출은 `s`회이며 full-space validation과 basis 생성은 별도로 더한다. 기존 coupled Newton에서 매 갱신마다 수행하던 current-stage derivative work와 `(s m)×(s m)` factorization을 제거할 수 있다. 실제 source가 triangular structure를 가진 경우에 이 reduction을 적용하므로 원래 root를 바꾸지 않는다.

이 경로의 **nonlinear corrector**는 Newton/JVP-free지만, 현재 projected basis와 frozen `J_0`를 만드는 callback은 남아 있다. 전체 solver를 fully derivative-free라고 표시하지 않는다. declared block algebra가 frozen action까지 제공하는 경우에만 AT1–AT2와 결합하여 derivative callback도 제거할 수 있다.

### 6.1 같은 root를 구하는 병렬 Picard와 정확한 finite front

affine seed의 block-lower matrix를

\[
M_{ii}=W_m,\qquad
M_{ij}=-\gamma c_{ij}I-h\gamma a_{ij}J_m\quad(j<i)
\]

로 두고 `b_i=h gamma Q^T f(y)` 및

\[
r_i(z)=Q^T\{f(y+Qv_i)-f(y)-J_0Qv_i\},\quad
v_i=\sum_{j<i}a_{ij}z_j
\]

를 정의한다. frozen-time/baseline conventions는 실제 caller와 동일해야 한다. iterative map

\[
T(z)=M^{-1}[b+h\gamma r(z)]
\tag{AT4.2}
\]

의 `i`번째 성분은 입력의 `z_0,...,z_(i-1)`에만 의존한다. 따라서 synchronous iteration의 첫 `k`회 뒤에는 처음 `k`개 stage가 exact fixed point와 같아진다. affine seed가 이미 stage 0을 풀면 최대 `s-1`회의 update면 충분하다. `s=8`에는 최대 7회이고, 구현의 마지막 residual 검사/여분 update는 rounding을 위해 유지한다.

**귀납 증명.** stage 0의 출력은 입력에 독립이다. 첫 `k`개 성분이 fixed point와 같으면 다음 iteration의 성분 `k`는 그 앞의 fixed-point 값들만 읽으므로 정확하다. lower solve에서도 `i`보다 큰 index의 입력이 들어오지 않는다. □

floating arithmetic에서는 iteration count만으로 수락하지 않는다. componentwise Lipschitz majorant `L>=0`가 strict lower이고 local roundoff/solve error vector가 `eta>=0`이면

\[
e^{k+1}\le L e^k+\eta,
\qquad e^s\le\sum_{j=0}^{s-1}L^j\eta
\tag{AT4.3}
\]

이다. `L^s=0`이므로 initial iterate error가 사라지지만, 누적 roundoff가 사라지는 것은 아니다. full-space residual과 physical-step error 수락은 원래 계약을 유지한다.

**work/span 선택.** 모든 stage를 매회 다시 계산하면 `s` rounds의 RHS 작업은 `s^2`, 이미 확정된 front를 건너뛰면 최대 `s(s+1)/2`, serial forward solve는 `s`다. 병렬 correction의 이득은 tolerance에 도달하는 실제 round 수, RHS cost, thread/dispatch 비용에 달려 있다. 작은 CPU와 싼 RHS에서는 direct causal pass를 우선하고, 충분히 비싼 RHS와 적은 correction round에서만 parallel path를 선택한다. `s`회의 worst-case Jacobi sweep 자체가 serial pass보다 빠르다는 정리는 없다. 여기서 보장한 것은 exact root를 보존하며 불필요한 nonlinear derivative 계산을 없애는 비용 감소다.

### 6.2 represented coefficient의 예외 처리

변환된 K-tableau에 작은 upper/diagonal leakage가 있으면 (AT4.1)의 strictly causal 가정은 성립하지 않는다. native 선택기는 실제 raw A,C entry를 검사하여 unsupported이면 기존 residual solver로 되돌아간다. 반올림 크기의 entry를 임의로 0으로 만드는 것은 target 변경이다.

향후 bounded leakage를 적극 활용하려면 `T=P-E`, `P=D-L`로 분해하고 현재 operator에 대해 `q=||P^{-1}E||<1`을 확인한다. residual correction

\[
x_{k+1}=x_k+P^{-1}(b-Tx_k)
\]

는 `||e_k||<=q^k||e_0||`를 만족하고,
`||(T)^(-1)||<=||P^(-1)||/(1-q)`이다. `E=UV^T`의 rank `r`가 선언되어 있으면 Woodbury의 `r×r` correction으로 바꿀 수 있다. 이는 신뢰 가능한 `E` bound나 구조 witness가 있을 때의 다음 구현이며, 이번 strict-causal guard에 그 기능이 이미 있다고 기록하지 않는다.

## 7. root identity와 ODE order를 함께 보존하는 규칙

AT4는 동일한 stage equation을 더 싸게 푼다. 따라서 원래 coefficients와 acceptance를 유지한다. 반면 generic nonlinear ODE에서 Jacobian `f_y`를 arbitrary `A`로 바꾸는 경우에는 stage equation 자체가 달라진다. 고차 Rosenbrock order를 유지하려면 Rosenbrock-W order conditions 또는 명시적인 correction/order 분석이 필요하다. 이 구분은 Tranquilli–Sandu의 Rosenbrock-K 이론과 Steihaug–Wolfbrandt의 W-method 원전에서도 드러난다.

AT2의 Lawson 구현은 독립된, 공개 method identity를 가진 방법으로 연결한다. 원래 RODAS5P를 그 이름 그대로 바꾸지 않는다. 구조가 없는 문제에서는 비용이 측정된 DOPRI, 기존 RODAS/JVP route를 선택할 수 있게 하며, derivative fallback 권한과 실제 callback count를 기록한다. fully derivative-free가 느린 사례의 선택 근거는 같은 물리 오차를 충족한 실행 비용으로 얻는다.

## 8. 판별 계산 및 다음 이식

새 small exact script는 rotation, nonsemisimple confluent Jordan block, 서로 다른 primary factor가 섞인 block에서 AT1을 세 `s` 값으로 확인한다. nonlinear strict-causal 8-stage rational recurrence의 각 front와 최종 root를 독립 sequential oracle과 비교한다. quadratic regular branch의 원래 residual 및 오차 항등식을 exact rational arithmetic으로 확인한다. 총 14개 묶음 검사가 통과했다. Wolfram은 symbolic polynomial remainder, quadratic root 및 branch derivative를 독립 확인하여 `{0,0,sqrt(a^2+4bc lambda),0}`를 반환했다.

우선 구현 순서는 다음과 같다.

1. strict raw-U 구조에서 causal forward corrector를 연결하고, 기존 full residual과 fallback을 유지한다.
2. declared damped-rotation block에 exact flow/resolvent를 연결하여 rapid oscillation과 damping의 linear cost를 제거한다.
3. exact scalar quadratic 구조를 가진 물리 source에 causal root adapter를 연결한다. coefficient/margin/refusal을 telemetry에 저장한다.
4. bounded probe와 hysteresis로 explicit/structured/coupled fallback을 선택한다. 단일 thread 기본 경로의 전체 wall time과 실제 phase/amplitude/global output error를 측정한다.
5. 일반 quotient compiler와 bounded noncausal leakage는 current full-space witness 및 coefficient error 계약을 갖춘 뒤 확장한다.

### 확인한 1차 문헌

- N. J. Higham and L. Lin, *Matrix Functions: A Short Course*, MIMS EPrint 2013.73, §3.1.2. <https://eprints.maths.manchester.ac.uk/2067/01/covered/MIMS_ep2013_73.pdf>. 최소다항식과 Hermite matrix-function calculus의 기존 원전이다.
- P. Tranquilli and A. Sandu, *Rosenbrock–Krylov Methods for Large Systems of Differential Equations*, SIAM J. Sci. Comput. 36(3), A1313–A1338 (2014), DOI 10.1137/130923336. <https://dl.acm.org/doi/10.1137/130923336>. time discretization과 Jacobian approximation을 함께 다루는 order-condition 이론을 구별하는 근거다.
- T. Steihaug and A. Wolfbrandt, *An Attempt to Avoid Exact Jacobian and Nonlinear Equations in the Numerical Solution of Stiff Differential Equations*, Math. Comp. 33 (1979), DOI 10.2307/2006293. <https://doi.org/10.2307/2006293>. occasional Jacobian과 W-method의 기존 계보로 인용한다.

새로운 범용 복잡도 하한이나 universal no-go를 주장하지 않는다. 비용 절감 정리의 조건은 실제 구조 witness와 실행 경로에서 검사한다.
