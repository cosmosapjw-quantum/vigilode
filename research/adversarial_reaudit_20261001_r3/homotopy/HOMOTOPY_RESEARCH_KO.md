# R3 homotopy 연구: outward 성분별 인증과 인증 자체의 병렬화

소스 `cc2cd041737e7ff543624d1b59893a3b4397369f`에 대한 독립 연구 부록이다. GPT-6 Astra v4.0.0 수학·코딩 하네스의 bounded 실행 계약을 적용했다. 생산 solver 변경은 없다. 명시적인 strict-lower projection target에서 R2 진단의 rounding·inverse witness·WRMS 문제를 실제 계산으로 닫고, strict-lower 구조를 이용해 인증 계산까지 병렬화할 수 있는 표현을 만든다. 이 문서의 인증 대상은 **고정된 다항식 stage 방정식의 해와 후보의 차이**다. 원 ODE의 truncation/global error 인증과 동일하지 않다.

## 1. 대상과 전제

단위 없는 자율 toy에서 stage 수는 실제 RODAS5P의 8개이며 공간 차원은 1 또는 2다. `b`, `btilde`, `gamma`와 `alpha`, `l`의 **strict-lower entries**는 native 계수 dump의 정확한 bit pattern에 결합한다. 상삼각과 대각을 0으로 만드는 projection을 실험 인자로 명시한다. 원 full-block target과 정확히 동일하다고 주장하지 않는다. 최종 입력 의미와 실행 집계는 아래 결과 절에서 명시한다. $h>0$, 질량행렬 $M=I$, 고정 Jacobian $J$, 성분별 quadratic remainder $q\odot\delta^{\odot2}$를 쓴다.

\[
 W K_i=g+hJ\sum_{j<i}L_{ij}K_j+hq\odot\delta_i^{\odot2},\quad
 \delta_i=\sum_{j<i}\alpha_{ij}K_j,\quad W=I-h\gamma J.
\]

여기서 $g=h(Jy-q\odot y^{\odot2})$이다. 이 RHS는 $f(z)=(J-2\operatorname{diag}(q\odot y))z+q\odot z^{\odot2}$에서 초기점 $y$의 Jacobian이 $J$인 정확한 quadratic expansion에 대응한다. 자율 문제라 시간미분 항은 없다. 가역 $W$와 strict-lower stage 의존성 때문에 stage root는 귀납적으로 유일하다. 각 toy의 $W^{-1}$는 1×1/2×2 유리수식으로 정확히 구성한다. **대규모 inverse가 저렴하게 주어진다는 가정은 하지 않는다.**

후보 $\widehat K$는 binary64로 계산한 q1/q2 toy다. Rust controller를 직접 실행한 결과도, 임의 모델의 동치 구현도 아니다. 후보의 정확도를 전제로 삼지 않는 사후 인증이므로 후보 계산의 solver·반올림 오류는 residual을 통해 포함된다.

## 2. 성분별 정리와 계산식

후보 residual을 $r_i=W\widehat K_i-g-hJ\sum L_{ij}\widehat K_j-hq\odot\widehat\delta_i^{\odot2}$로 둔다. 검증된 비음수 $U\ge |W^{-1}|$와 $R_i\ge |r_i|$가 주어지고 이전 stage의 오차를 $E_j\ge |\widehat K_j-K_j|$가 감싼다고 하자. 그러면

\[
d_i=\sum_{j<i}|\alpha_{ij}|E_j,
\qquad
E_i=U\left[R_i+h|J|\sum_{j<i}|L_{ij}|E_j
 +h|q|\odot(2|\widehat\delta_i|\odot d_i+d_i^{\odot2})\right]
\tag{1}
\]

는 다음 stage의 성분별 상계다. 실제 계산에서는 우변 전체를 위쪽으로 반올림한다. 증명은 후보와 root 방정식을 빼고 $a^2-b^2=2a(a-b)-(a-b)^2$에 절댓값을 취한 뒤 $U$를 곱하는 유한 귀납이다. 전역 contraction이나 $\|hJ\|<1$은 필요하지 않는다. 그러나 계산된 상계는 큰 $h$·nonnormality에서 매우 커질 수 있다.

구현은 scalar add/multiply의 binary64 결과를 `nextafter`로 바깥쪽에 넓힌다. 입력은 정확한 binary64 실수로 해석하며 underflow도 확장에 포함한다. overflow/nonfinite에는 실패를 반환한다. 1×1/2×2 inverse witness는 정확한 `Fraction` 결과를 binary64 위쪽으로 변환한다. 따라서 이 코드는 **일부 exact arithmetic을 사용하는 outward reference prototype**이며 순수 hardware directed-rounding 최적화 구현이 아니다. 기본 floating arithmetic이 IEEE binary64 nearest라는 전제와 `nextafter`의 정상 동작을 요구한다.

Residual과 $\widehat\delta_i$를 interval로 새로 평가하여 후보 계산과 구별한다. interval의 각 성분은 별도의 정확한 유리수 residual과 비교한다. 따라서 non-directed residual에 작은 경험적 fudge factor를 붙인 R2 actual-tableau 진단과 다르다.

출력 $Y=y+\sum_i b_iK_i$, embedded vector $e=\sum_i\widetilde b_iK_i$에 대해 반환된 floating projection의 반올림 반경을 $\rho_Y,\rho_e$라 하면

\[
B_Y=\rho_Y+\sum_i|b_i|E_i,\qquad
B_e=\rho_e+\sum_i|\widetilde b_i|E_i.
\tag{2}
\]

이번 실행은 고정 candidate scale $s_a=\mathrm{atol}+\mathrm{rtol}\max(|y_a|,|\widehat Y_a|)>0$에서 $\|B\|_{\mathrm{WRMS}(s)}=[n^{-1}\sum_a(B_a/s_a)^2]^{1/2}$의 상계를 계산한다. scale 자체를 `Fraction`으로 평가하고 sqrt 반환값의 제곱을 정확 유리수와 비교해 위쪽으로 조정한다. $\mathrm{atol}=10^{-8},\mathrm{rtol}=10^{-6}$은 fixture 계약이며 튜닝하지 않는다.

\[
\|e\|_{\mathrm{WRMS}(s)}\le
\big\||\widehat e|+B_e\big\|_{\mathrm{WRMS}(s)}.
\]

이것과 output bound의 합은 stage approximation과 embedded **proxy**를 함께 제한한다. embedded estimate가 원 ODE local error의 상계라는 별도 정리는 없으므로, 그 합을 total ODE-error certificate라고 부르지 않는다. exact-root의 상태에 따라 scale을 다시 정하는 계약으로 바꾸려면 분모의 독립 하한도 필요하다.

## 3. 인증 단계의 직렬 의존성을 줄이는 유도

(1)을 그대로 계산하면 인증에도 8개의 stage layer가 남는다. 이전 step 등에서 예측한 **독립적인** state-radius $D_i\ge0$를 먼저 고정한다. 후보 root를 보거나 (1)의 결과를 사용해 이번 실험의 $D_i$를 고르지 않았다. 실행에서는 모든 성분에 $D_i=10^{-4}$를 고정한다.

오차의 state-radius가 $d_i\le D_i$일 때 nonlinear Lipschitz factor는

\[
C_i=\operatorname{diag}\{|q|\odot(2|\widehat\delta_i|+D_i)\}.
\]

이에 따라 stage-majorant block을

\[
H_{ij}=hU\big(|J||L_{ij}|+C_i|\alpha_{ij}|\big),\quad j<i;
\quad a_i=UR_i
\tag{3}
\]

로 두고 나머지 block을 0으로 둔다. $H^8=0$이므로

\[
E=(I-H)^{-1}a=(I+H+\cdots+H^7)a
 =(I+H)(I+H^2)(I+H^4)a.
\tag{4}
\]

H의 norm이 1보다 작을 필요가 없다. 3회의 doubling level로 (4)를 만들 수 있다. 위쪽 반올림한 각 곱·합은 비음수이므로 계산된 $\widehat E$가 정확한 (4)를 감싼다. 마지막으로 $\sum_{j<i}|\alpha_{ij}|\widehat E_j\le D_i$를 바깥쪽 계산으로 확인해야 한다. 이 closure가 통과하면 stage 귀납으로 nonlinear target의 오차를 감싼다. 실패하면 반경을 넓히는 별도 예측 시도 또는 순차 fallback을 택하며 성공으로 소비하지 않는다.

본 Python 코드는 dense 8n×8n matrix를 사용해 이 표현을 **직렬로 실행**했다. 5개 dense matrix multiplication의 의존 그래프가 3개의 이론적 doubling level이라는 뜻이지 실측 3배 가속이라는 뜻이 아니다. 일반 문제에서 U가 dense이면 저장량·곱셈 비용이 이득을 지울 수 있다. 따라서 다음 구현은 block-diagonal/banded/positive comparison 구조를 우선 대상으로 해야 한다.

## 4. q2의 실질적인 개발 지점과 비용 조건

현재 `transactional_q1_q2.rs`는 q1의 6 W batches 다음에 보존한 causal term으로 q2 후보를 **7번째 batch**에서 만든다. 8번째 batch는 후보에 적용하지 않는 q=0 post-residual diagnostic이다(코드 658–708). 진짜 certificate를 만들면 이 diagnostic을 대체할 수 있다. 먼저 동일 q2 후보에 대한 output·embedded bound를 통과시킨 뒤 제거해야 하며 운영 gate만 없애는 변경은 허용하지 않는다.

각 batch에 8개 vector solve가 있고 P개 동일 worker에서 stage task를 처리한다고 단순화하면 batch 시간은 $\lceil8/P\rceil t_W$이다. 순차 baseline은 $8t_W$, q1은 $6\lceil8/P\rceil t_W$에 RHS/JVP·동기화·인증비가 추가된다. worker가 충분할 때도 certificate가 없는 이상적 q1 상한은 $8/6$이며 vector work는 48 대 8이다. 이 toy 모형에서 P≤4이면 모든 vector가 동일 비용인 6 batch 방식은 W 시간만으로도 baseline보다 느리다. zero/correlated RHS reuse, batch LU/GEMM 이득, 실제 Krylov iteration 차이는 별도 계측 대상이다.

q1 수락 확률 p1, q2 뒤 fallback 확률 pf, normalized 인증비 c1,c2, inverse-witness amortized cost a, 추가 RHS/JVP/scheduling 비용 r을 두자. q2의 diagnostic을 유지하면 충분 worker·고정 W 비용 모형에서

\[
T/t_W=8-2p_1+8p_f+c_1+(1-p_1)c_2+a+r.
\]

break-even에는 $2p_1>8p_f+c_1+(1-p_1)c_2+a+r$가 필요하다. diagnostic을 실제 certificate로 대체하면 7번째 후보까지이므로

\[
T/t_W=7-p_1+8p_f+c_1+(1-p_1)c_2+a+r,
\]

그리고 $1+p_1>8p_f+c_1+(1-p_1)c_2+a+r$가 필요하다. 이는 fixture 수락률을 운영분포 확률로 해석하는 식이 아니다. 실제 동일 정확도·동일 출력 workload에서 성공/실패를 모두 포함한 per-attempt time을 측정해야 한다.

## 5. 실제 실행 결과

최초 native-target 실행은 **exit 1**이었다. `load_rodas5p_coefficients()`는 일반 LU로 Γ를 만들며 native α의 상삼각·대각 28개, L의 34개 원소가 0이 아니었다. 최대 크기는 각각 `5.577737968635803e-16`, `3.7683229960916457e-16`이다. 정확한 Fraction 행렬곱에서도 native α⁸, L⁸은 0이 아니고 최대 원소 크기는 `1.974514526344302e-19`, `3.9042416155563606e-20`였다. 즉 bit pattern을 정확한 실수 계수로 해석하면 nilpotency 전제가 성립하지 않는다. 이것이 큰 solver 오차를 초래했다는 증거는 아니다.

`sequential.rs`는 이전 stage `[..i]`를 소비하지만 block mixer는 모든 원소를 소비한다. 또한 순차식은 α와 Γ를 별도로 사용하므로 그 정확한 실수 합과 반올림되어 저장된 L의 차이도 구분해야 한다. 이번 certificate를 native full-block이나 native sequential arithmetic에 즉시 붙이는 것은 승인하지 않는다. `first_run.stderr`, `first_failure.json`, 최초 script를 보존했다. 이어 `--target strict-lower-projection`을 명시해 위 정리의 입력을 고정했으며, 계수 leakage를 숨기거나 원 source를 바꾸지 않았다.

재실행은 **exit 0**, 실제 프로세스 시간 약 0.77초였다. 이는 성능 benchmark가 아니며 실행 영수증의 환경 내 관측치다. 4개 계열(affine scalar, nonnormal 2×2, scalar quadratic, nonnormal quadratic)에 h=0.1,1,10과 q1/q2를 적용한 **24개 후보**를 계산했다. 모든 후보에서 다음 검사가 정확 유리수 비교로 통과했다: inverse witness, residual interval 포함, stage 성분별 오차, 반환된 output/embedded projection 오차, 고정-scale output/embedded WRMS. 별도로 162개의 underflow·cancellation 포함 primitive 산술 enclosure 비교와 overflow fail-closed를 통과했다.

| 판별 항목 | 결과 | 해석 |
|---|---:|---|
| 유효한 outward stage/output/embedded 상계 | 24/24 | 상계가 넓어지는 실패 후보도 정확히 포함 |
| output WRMS 상계 ≤0.1 | 12/24 | 고정 toy tolerance 기준, 보편적 수락률 아님 |
| output bound + target embedded proxy 상계 ≤1 | 8/24 | ODE 총오차 인증이 아님 |
| D=10⁻⁴에서 3-doubling radius closure | 22/24 | 통과 22건의 stage bound를 exact root와 비교해 확인 |
| radius closure reject | 2/24 | scalar quadratic h=10의 q1/q2; explicit reject 유지 |

주요 수치는 다음과 같다. 표의 오차는 같은 candidate scale의 WRMS다.

| 계열·h·후보 | exact output 오차 | outward 상계 | 비고 |
|---|---:|---:|---|
| nonnormal, 1, q1 | 1.28869×10² | 1.36643×10² | 잘못된 후보의 수락을 막음 |
| nonnormal, 1, q2 | 2.62446×10⁻⁹ | 1.60325×10⁻⁷ | rounding 포함으로 상계가 실제 오차보다 큼 |
| nonnormal quadratic, 1, q2 | 8.62967×10⁻¹ | 9.44379×10⁻¹ | output budget 0.1에는 불합격 |
| scalar quadratic, 10, q1 | 5.31672×10⁶ | 3.63089×10⁷ | 큰 h에서 predictor 실패 |
| scalar quadratic, 10, q2 | 8.71701×10⁶ | 7.89358×10⁷ | q2가 q1보다 악화될 수도 있음 |

이 결과는 “q를 늘리면 반드시 좋아진다”는 운영 가정을 지지하지 않는다. candidate generation과 independent admission을 분리하고 최종 출력 예산으로 거부해야 한다. 3-doubling closure 통과 자체도 작은 출력 오차를 뜻하지 않으며 반드시 output·embedded gate가 뒤따라야 한다.

`results.json`에 후보별 stage bound, interval 연산 수, actual reference error, radius closure, exact 판별 결과를 저장했다. 정확 reference는 동일 고정 계수를 공유하지만 별도의 Fraction forward solve이며 floating candidate/interval kernel의 값을 reference로 재사용하지 않는다. 이것은 독립 arithmetic oracle이지 독립 실행자 검토를 대신하지 않는다. 연구 승인 여부는 root의 별도 decision review를 따른다.

## 6. 문헌 근거의 범위

Rump, *Verification methods: rigorous results using floating-point arithmetic*, Acta Numerica (2010), 저자 공개 [원문 PDF](https://www.tuhh.de/ti3/rump/intlab/ActaNumerica2010.pdf)의 서론과 §10.3–10.4, 특히 (10.9)를 확인했다. 대략적인 inverse/preconditioner와 정확히 둘러싼 residual의 결합이 verified error bound에 필요하다는 방법론의 근거다. 이 문헌이 위의 RODAS-specific quadratic 정리나 q1/q2 성능을 증명한 것은 아니다. (1)–(4)는 여기서 직접 유도했다.

SciSpace 검색은 componentwise verified linear-system 문헌의 초록·서지 탐색으로 사용했다. 같은 제목의 ISSAC talk `10.1145/1837934.1837937`가 Acta Numerica review와 별도 결과로 검색되어, 그 DOI를 review의 DOI로 복사하지 않았다. 개별 문헌 초록을 production 구현 검증으로 올리지 않았다.

## 7. 구체적인 다음 개발

1. 계수의 구조 계약: native 계수 bit pattern, strict-lower stage dependency, common diagonal 및 output weights를 선언하고 full-block/순차 target 간 동치의 반올림 범위를 닫는다.
2. 작은 차원의 reference certificate: Rust interval primitive에 underflow·signed zero·overflow·FMA 정책을 고정하고 이 Python exact oracle fixture와 대조한다. `Unknown` 또는 inverse witness 실패는 reject다.
3. 구조를 보존하는 inverse witness: diagonal·block-triangular/banded W부터 확장한다. 일반 matrix-free JVP만으로 entrywise $|W^{-1}|$가 자동으로 주어지지 않는다.
4. 인증의 병렬 후보: D를 이전 accepted step에서 보수적으로 예측하고 (3)–(4)의 closure 실패율과 비용을 기록한다. native arbitrary nonlinear remainder에는 검증된 derivative enclosure callback이 필요하다.
5. 같은 q2 후보에서 8번째 diagnostic의 대체를 검증한다. output, embedded, RHS/JVP, work ledger, fallback transaction을 함께 검사한 뒤 그 batch만 제거한다.
6. 최종 성능 실험: 동일 출력·허용오차에서 순차 baseline과 1/2/4/8 worker를 비교하고 witness·rejected attempt·fallback 전체 시간을 합산한다. q1 수락률만으로 가속을 승격하지 않는다.

독립 decision reviewer의 제한된 수학/실행 승인 전에는 연구 산출물의 최종 승격을 보류한다. 생산 solver readiness는 본 부록에서 바꾸지 않는다.
