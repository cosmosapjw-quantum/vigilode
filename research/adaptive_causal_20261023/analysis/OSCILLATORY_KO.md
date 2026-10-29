# 선언된 진동·감쇠 구조를 사용하는 quotient-flow 연구

2026-10-23. 새 native 구현은 `crates/rodas5p-integrators/src/quotient_flow.rs`다. 기존 scalar/collective packet의 특수 구조 계산을 일반 residual callback과 조합할 수 있는 상수 block adapter로 확장했다. 사용자 제공 `RVJ_Report_20261023_KO.pdf` 7–17쪽의 유효한 구조 축약·위상 처리·실패 사례를 재사용했다. 입력 identity와 읽은 위치는 `evidence/RVJ_MAP.json`에 있다.

## 1. 해결한 계산 문제와 API

대상은 다음 identity-mass 초기값 문제다.

\[
 y'=Ay+N(t,y),\qquad A=\operatorname{diag}(A_j),\qquad
 A_j=\begin{pmatrix}-d_j&-\omega_j\\\omega_j&-d_j\end{pmatrix},
 \quad d_j\ge0.
\]

사용자가 실제 상수 연산자의 block 계수를 제공하고 callback에는 **잔차 우변 N만** 넣는다. 그 residual은 블록 사이를 비선형으로 결합할 수 있다. 전체 RHS를 callback에 넣으면 A를 두 번 더하므로 계약 위반이다. 계수를 RHS probe나 고유값 추정에서 몰래 만들지 않는다.

- `DampedRotationFlow::new/repeated`: 선언된 block과 공유 계수 인덱스를 구성한다.
- `apply_into`: $e^{\tau A}$의 직접 action.
- `shifted_inverse_into`: $(I-sA)^{-1}$의 직접 action.
- `LawsonDopri5Workspace::step_into`: 한 번의 미확정 physical-coordinate trial.
- `integrate_lawson_dopri5_observed`: FSAL을 사용하는 적응형 endpoint observation.
- `integrate_lawson_dopri5_observed_with_norm`: 기존 affine 물리 오차 norm 결합.

N callback만 평가한다. Jacobian, JVP, AD, finite-difference probe, Newton 반복은 없다. shifted inverse는 실제 frozen Jacobian이 이 block 연산자와 일치할 때 Rosenbrock W 풀이에 재사용할 수 있는 커널이다. 임의 Jacobian을 이 A로 대체한 Rosenbrock 방법의 차수·안정성은 여기에서 보장하지 않는다.

## 2. 몫대수에 의한 정확한 연산 감소

$K_j=A_j+d_jI$이면 $K_j^2=-\omega_j^2I$다. 따라서 analytic 함수의 계산은 이차 ideal의 몫대수에서 두 계수만 필요한다. 직접 급수를 짝수·홀수로 나누면

\[
 e^{\tau A_j}=e^{-d_j\tau}
 \begin{pmatrix}\cos\omega_j\tau&-\sin\omega_j\tau\\
 \sin\omega_j\tau&\cos\omega_j\tau\end{pmatrix}.
\tag{Q1}
\]

$\omega_j=0$에서도 같은 식이 정칙하며 $1/\omega_j$나 서로 분리된 root를 요구하지 않는다. 반복근을 가진 chart를 별도 diagonalization으로 분해할 필요가 없다.

또한 $a=1+sd_j,b=s\omega_j$로 두면

\[
 (I-sA_j)^{-1}=\frac1{a^2+b^2}
 \begin{pmatrix}a&-b\\b&a\end{pmatrix},\qquad
 \|(I-sA_j)^{-1}\|_2=(a^2+b^2)^{-1/2}\le1.
\tag{Q2}
\]

증명은 두 2×2 행렬을 곱하고, 역행렬의 transpose와의 곱이 $(a^2+b^2)^{-1}I$임을 확인하면 끝난다. 따라서 선언된 실제 operator의 resolvent는 반복 linear solve 없이 O(n) 시간, O(1) 추가 scratch로 계산할 수 있다. 전체 dense LU의 O(n³)와 대비되는 구조별 연산 보장이다. 기존 band/diagonal solver보다 항상 빠르다는 wall-time 주장은 아니다.

Q1의 2-norm은 $e^{-d_j\tau}\le1$이다. 순수 선형 문제에서는 arbitrary positive step에서 정확한 연산의 감쇠·회전이 보존된다. 부동소수점 coefficient 평가에는 별도 rounding이 있다.

## 3. Residual만 적분하는 Lawson-DOPRI5

기존 검증된 Dormand–Prince 계수 $a_{ij},b_i,\widehat b_i,c_i$를 재사용한다. 상수 A에 대해 local variable $z(\theta)=e^{-\theta A}y(t_n+\theta)$를 형식적으로 도입하고 RK 단계를 원래 좌표로 되돌리면

\[
 Y_i=e^{c_ihA}y_n+h\sum_{j<i}a_{ij}e^{(c_i-c_j)hA}N(t_n+c_jh,Y_j),
\tag{Q3}
\]
\[
 y_{n+1}=e^{hA}y_n+h\sum_i b_i e^{(1-c_i)hA}N_i,
 \quad e_n=h\sum_i(b_i-\widehat b_i)e^{(1-c_i)hA}N_i.
\tag{Q4}
\]

이는 Lawson 방법이며 새로운 RK tableau라는 주장은 하지 않는다. 원전은 Lawson(1967), 직접 확인한 현대 유도는 Hochbruck–Leibold–Ostermann(2019)의 §2 식(4)다. 이 소스의 A 부호는 본 보고서와 반대다.

**정리 Q3 — 고정 A의 고전적 차수.** A가 상수이고 변환된 RHS의 필요한 도함수가 고려하는 유한 영역에서 유계이며 해와 모든 stages가 그 영역에 있으면, Q3–Q4의 endpoint는 고전적 5차, embedded partner는 4차다. 변환된 RK와의 대수적 동등성, 기존 DOPRI order conditions, 가역 선형 좌표변환으로부터 따른다. 상수에는 A와 관련 도함수의 크기가 들어갈 수 있다.

DOPRI nodes는 $0,1/5,3/10,4/5,8/9,1,1$로 비감소한다. 모든 Q3–Q4의 exponent duration은 비음수다. 코드에는 $e^{-tA}$의 큰 역감쇠를 계산하는 단계가 없다. 따라서 전역 integrating-factor의 overflow 문제를 피하면서 forward semigroup만 사용한다. 해당 semigroup 조건은 Hochbruck–Leibold–Ostermann §3 Assumption 3.1과 일치한다.

단계7의 상태가 accepted fifth-order endpoint와 같으므로 N의 FSAL도 유지된다. 완결 trial 수가 k이면 기본 residual callback 수는 $1+6k$다. 실패는 진입한 callback만 계상한다. 요청된 observation에 실제로 착지하며 dense polynomial을 새 method의 보간식으로 재사용하지 않는다.

## 4. 주파수와 독립적인 비선형 부분류의 보장

A가 skew block($d_j=0$)이고 $R(\theta)=e^{\theta A}$에 대해

\[
 N(t,R(\theta)z)=R(\theta)N(t,z)
\tag{Q5}
\]

가 성립한다고 하자. 그러면 $z=e^{-tA}y$는 정확히 $z'=N(t,z)$를 만족한다. Q3의 각 stage에서 induction하면 $Y_i=e^{c_ihA}Z_i$이며 $Z_i$는 같은 DOPRI의 느린 residual stage다. Q4의 오차도 회전한 느린 오차이므로 Euclidean norm이 같다. 따라서 같은 fixed mesh에서 원 상태 오차는 residual-only 계산의 오차와 정확히 같고 carrier 주파수를 포함하지 않는다. 공통 component absolute tolerance, rtol=0처럼 회전 불변인 acceptance norm을 쓰면 adaptive acceptance도 같은 결과다. component relative WRMS는 회전에 따라 scale이 달라질 수 있어 같은 adaptive mesh를 주장하지 않는다.

예를 들어 한 block에서 $N(y)=\epsilon(1-|y|^2)y$는 Q5를 만족한다. step 수는 느린 진폭 변화로 정해진다. 반면 full-RHS explicit RK의 안정함수는 비상수 다항식 R이므로 순허수축에서 $|R(i\theta)|\le1$인 구간은 유계다. 각 step의 선형 nonamplification을 요구하면 $h|\omega|\le\beta$이고 fixed horizon의 단계 수가 적어도 $T|\omega|/\beta$다. Q5 부분류에서 주파수에 따른 이 작업 증가는 Q1–Q4로 제거된다. 이는 실제 조건이 주어진 구성적 연산량 감소다.

forcing resonance, time-dependent carrier, nonnormal coupling, residual에 남아 있는 빠른 시간 의존성은 Q5의 별도 조건이다. RVJ 보고서 13쪽의 forcing alias 반례를 그대로 유효한 한계로 유지했다. Q1만 정확히 계산한다고 arbitrary forcing이 자동으로 해결되지는 않는다.

## 5. 작은 계산환경에서의 구현

동일한 $(d,\omega)$ block은 생성 시 하나의 group으로 묶는다. 한 trial마다 각 group의 lower-triangular 28개 exponent coefficient를 계산하고 모든 state pair가 공유한다. g개의 서로 다른 group과 n개의 성분에서 coefficient 준비 O(g), stage vector 작업 O(n), residual 비용 6회/정상 trial이다. 성공한 workspace trial은 heap allocation이 없고 Rayon을 사용하지 않는다.

Principal workspace는 7개의 residual vector와 stage-state, block index, coefficient cache다. 반환 `workspace_bytes`는 실제 vector/cache 원소 크기로 계산한다. 결과 observation 저장과 allocator metadata는 제외한다. 한 group을 공유하는 큰 ensemble에 state-size만큼 transcendental 평가를 반복하지 않는다.

IEEE-754 edge case도 명시적으로 처리했다.

1. $1+sd$, $s\omega$의 제곱·곱 overflow를 scaled complex division으로 피한다. normalized coefficient를 denominator로 먼저 나누어 representable result 앞의 큰 합이 overflow하는 경우를 고쳤다.
2. $e^{-1000}$ 자체는 underflow하지만 $10^{308}e^{-1000}\simeq5.076\times10^{-127}$은 표현 가능하다. exponent가 −700보다 작으면 half-exponent를 입력에 두 번 곱한다. $d\tau>1500$에서만 모든 finite input이 0으로 반올림됨을 이용해 phase 평가도 생략한다.
3. 내부 stage fraction은 가능한 경우 $(\omega h)(c_i-c_j)$로 계산하여 작은 h에서 $(c_i-c_j)h$가 먼저 사라지는 것을 줄인다. 큰 곱이 overflow하면 반대 순서로 평가한다.
4. 입력·시간·출력 검사, callback poison, 실패 partial state, 마지막 accepted state와 FSAL 전이는 기존 explicit 계약을 따른다.

장시간 주파수 오차는 여전히 별도다. represented coefficient의 phase 곱 오차 $\delta\theta_j$는 선형 회전에서 최대 $\sum_j|\delta\theta_j|$의 상태 오차 기여를 줄 수 있다. 신뢰 가능한 frequency input과 장시간 phase-sensitive 비교를 local campaign에 포함해야 한다.

## 6. 검증과 읽어야 할 수치 결과

`tests/quotient_flow_contracts.rs`는 독립 원방정식 inverse residual, 극대 계수 산술, 반복근, pure carrier exact action, 비가환 residual의 5차 수렴, nonlinear radial analytic solution, callback 실패 원자성, malformed callback, 입력 검사, 큰 입력의 underflow 보존을 검사한다. 원시 실행 로그는 `results/quotient_tests.log`다.

`examples/quotient_flow_bench.rs`는 7개 family, 2개 tolerance, 2개 method, warm-up1+timed3회로 총112개 record를 만든다. slow, rapid-linear, stiff-damped, equivariant nonlinear, shared128, noncommuting, stiff-forcing을 포함한다. 각 observation의 모든 성분을 analytic solution과 비교한다. `src/analyze_quotient.py`는 공통 sampled absolute-error cap을 통과한 설정의 median만 비교한다.

컴파일 중 공유 호스트에서 얻은 초기 측정은 `quotient_benchmark_initial.json` 및 `quotient_summary_preliminary.json`으로 남겼다. 최종 성능 수치는 native commit `548526add83a38a996875b3bd2f3cd3357465ed6`을 고정하고 다른 agent의 build/benchmark를 멈춘 뒤 CPU 0 한 코어에 고정하여 실행한 `quotient_benchmark.json`과 `quotient_summary.json`을 사용한다. 실행 영수증은 `quotient_run_receipt.json`이다. 초기 결과에서 equivariant high-carrier는 큰 작업량 절감을 보였지만, stiff-forcing에서는 Lawson이 더 많은 steps와 시간을 필요로 했다. 이것이 선언된 carrier 경로를 상황별 opt-in으로 연결하는 이유다.

### 최종 단일 스레드 결과

총112개 record(28 warm-up +84 timed), 모든 실행 성공, campaign wall 1.257초였다. 아래는 같은 sampled absolute-error 상한 $10^{-4}$를 만족하는 **측정된 설정들 중 가장 작은 median**의 비교다. 비율은 full-RHS DOPRI 시간 / quotient-Lawson 시간이다. 세 반복은 정밀 통계 추정이 아니며 microsecond 사례의 차이는 가볍게 해석한다.

| 입력 | full-RHS DOPRI ms | quotient-Lawson ms | 시간비 | RHS 호출 수 |
|---|---:|---:|---:|---:|
| 느린 회전 control | 0.008962 | 0.012318 | 0.728 | 97 → 85 |
| 빠른 선형 회전 | 65.0703 | 0.012563 | 5179.5 | 798265 → 79 |
| 강직 감쇠 회전 | 0.114072 | 0.010241 | 11.139 | 1711 → 67 |
| 회전 equivariant 비선형 | 23.4981 | 0.013795 | 1703.4 | 300211 → 85 |
| 공통 carrier 128성분 | 54.2608 | 0.130097 | 417.08 | 66895 → 79 |
| 비가환 residual control | 0.012264 | 0.012311 | 0.996 | 169 → 85 |
| 강직 forcing control | 0.103591 | 1.259519 | 0.0822 | 1651 → 7891 |

빠른 선형 회전의 선택된 full-RHS 오차는 $1.257\times10^{-5}$, quotient 오차는 $4.540\times10^{-13}$였다. 비선형 사례도 원래 residual을 유지했지만 회전 equivariance를 만족하도록 구성했으므로 큰 속도비의 적용 조건이 명확하다. 강직 forcing의 quotient 경로는 약12.16배 느려졌고, 느린 control도 손해였다. 자동 전환에서 declared carrier만으로 무조건 선택해서는 안 된다는 실제 비용 근거다. 더 엄격한 공통 상한에서 baseline이 통과하지 못한 항목은 `unmatched`로 남겼으며 속도비를 만들지 않았다.

## 7. 개발 연결

1. facade에서 `LawsonDopri5`를 residual-only 명칭과 함께 노출하고 output/norm/mass/event 지원 조합을 먼저 검사한다.
2. switching은 declared structure가 있는 문제에서 이 경로를 후보로 삼는다. 일반 opaque RHS에는 기존 bounded explicit/implicit 비교를 사용한다.
3. 실제 Rosenbrock의 W가 Q2와 일치하는 client는 shifted inverse 커널을 연결하고 원래 stage residual로 다시 확인한다. 근사 W 대체는 별도 method 연구다.
4. frequency uncertainty·time-dependent/noncommuting A·full nonlinear stiff order 문제는 차기 연구로 분리한다. 특히 장시간 phase error와 forced equilibrium 유지 오차를 관측량으로 포함한다.

참고: https://doi.org/10.1137/0704033 ; https://www.waves.kit.edu/downloads/CRC1173_Preprint_2019-20.pdf . 후자의 §2–3 본문을 확인했고 전자의 direct open은 timeout이어서 출판 메타데이터까지만 채택했다. 방법의 유도는 위에 직접 제시했다.
