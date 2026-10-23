# DOPRI5 연속 확장: 독립 대수 검증과 코드 검토

이 검토는 새 quartic continuous extension에 한정한다. 이미 게시된 DOPRI5 endpoint의
5차 조건은 다시 계산하지 않았다. 사전등록 commit은
`83c79b936efc613b73c41897b647fffa6e6d0a1d`이며, 실제 Wolfram Language kernel에서
정확한 유리수 대수로 새 조건을 계산했다. 원시 응답·실행 코드는 각각
`evidence/DENSE_ORDER_WOLFRAM_RAW.json`, `evidence/DENSE_ORDER_CHECK.wl`에 있다.
`DENSE_ORDER_CHECK.json`은 그 응답을 기계 판독 가능한 형식으로 옮긴 것이다.
Rust의 A, c, P 상수 literal과 Wolfram 입력을 Python Fraction 파싱으로 비교했고
모두 일치했다. 부동소수점 tolerance로 다항식 항등식을 판정한 것이 아니다.

## 식과 정확한 조건

기존 일곱 단계 기울기를 \(k_i\), 새 계수 행렬을 \(P\)라 하고

\[
b_i(\theta)=\sum_{q=1}^{4}P_{i,q}\theta^q,\qquad
p_n(\theta)=y_n+h\sum_{i=1}^{7}b_i(\theta)k_i,\quad 0\leq\theta\leq1
\]

로 둔다. A는 기존 명시적 tableau, \(c=A\mathbf1\)이다. Wolfram에서 다음
여덟 식의 좌변과 우변 차이가 모두 정확히 0임을 확인했다.

| 조건 | 정확한 값 |
|---|---|
| \(b(\theta)^T\mathbf1\) | \(\theta\) |
| \(b(\theta)^Tc\) | \(\theta^2/2\) |
| \(b(\theta)^Tc^{\circ2}\) | \(\theta^3/3\) |
| \(b(\theta)^TAc\) | \(\theta^3/6\) |
| \(b(\theta)^Tc^{\circ3}\) | \(\theta^4/4\) |
| \(b(\theta)^T\operatorname{diag}(c)Ac\) | \(\theta^4/8\) |
| \(b(\theta)^TAc^{\circ2}\) | \(\theta^4/12\) |
| \(b(\theta)^TA^2c\) | \(\theta^4/24\) |

또한 \(b(1)=b_5\), \(b'(0)=e_1\), \(b'(1)=e_7\)가 정확하다. 따라서
정확 연산에서 왼쪽 endpoint의 값·미분은 \(y_n,k_1\), 오른쪽 endpoint는
기존 5차 endpoint와 \(k_7\)이다. 구현은 오른쪽 endpoint query에 대해 다항식을
재계산하지 않고 실제 accepted state를 복사하므로 그 값의 비트 일치도 보존한다.

**국소 정확도 정리.** 한 단계와 주변 stage가 머무는 영역에서 autonomous RHS가
충분히 매끄럽고 관련 도함수가 유계이면, 정확한 시작값에서 얻은 위 다항식은
\(\sup_{0\leq\theta\leq1}\|p_n(\theta)-y(t_n+\theta h)\|\leq C h^5\)
를 만족한다. \(C\)는 고정된 smoothness 영역에 의존하며 작은 h와 theta에 대해 균일하다.

증명은 단계식의 Taylor 전개로 직접 얻는다. h부터 h⁴까지 RHS elementary differential의
계수는 순서대로

\[
 b^T1;\quad b^Tc;\quad \tfrac12b^Tc^2,\ b^TAc;\quad
 \tfrac16b^Tc^3,\ b^T\operatorname{diag}(c)Ac,
 \tfrac12b^TAc^2,\ b^TA^2c
\]

이다. 각각 정확해의 \(\theta,\theta^2/2,\theta^3/6,\theta^3/6,
\theta^4/24,\theta^4/8,\theta^4/24,\theta^4/24\)와 일치한다. 따라서 h⁴까지
모든 항이 소거되고 remainder는 h⁵다. 비자율 RHS는 \((t,y)'=(1,f(t,y))\)로
확장하면 같은 증명이 적용된다. \(A1=c\)가 실제 stage time과 이 확장의 일치를 보장한다.

이 보간을 continuous order five라고 부르면 안 된다. 예를 들어 \(y'=t^4\),
\(t_n=0,y_n=0\)에서 보간 오차를 h⁵로 나눈 값은 정확히

\[
b(\theta)^Tc^{\circ4}-\theta^5/5
=-\frac{(\theta-1)^2\theta^2(-31375885+58760846\theta)}{293804230}.
\]

\(\theta=1/2\)에서는 \(997731/2350433840\ne0\)이다. 이 구체적 항등식은
quartic polynomial의 국소 h⁵ 오차를 드러낸다. 기존 5차 endpoint의 오류나 보편적인
고차 보간 불가능성을 뜻하지 않는다.

## 조건부 전역 bound와 tolerance의 구분

보간 결과를 다음 accepted state로 되먹임하지 않는다는 점이 중요하다. 기존 endpoint
방법에 대해 별도의 고정 시간 구간 nodal bound
\(\max_n\|y_n-y(t_n)\|\leq C_N H^5\), \(H=\max_n h_n\)가 이미 성립하고,
stage와 dense map이 초기값에 대해 균일하게 Lipschitz이면

\[
\|p_n(\theta)-y(t_n+\theta h_n)\|
\leq (1+C_Lh_n)\|y_n-y(t_n)\|+C_Dh_n^5
\leq [(1+C_LT)C_N+C_D]H^5.
\]

첫 부등식은 같은 polynomial을 정확한 시작값에서 구성한 중간 항을 더하고 빼서 얻는다.
보간의 국소 오차는 관측에만 쓰이고 다음 단계로 누적되지 않으므로 이 부등식이 가능하다.
이는 **별도의 5차 nodal bound·smoothness·stability 조건을 가정한 결과**다. 현재 embedded
estimator가 모든 내부 시각에서 사용자 rtol/atol을 만족시킨다는 인증은 아니다.
roundoff·callback error·비매끄러운 limiter·강성 order reduction이 있으면 각각의
추가 오차항과 가정을 검토해야 한다.

## 추가 RHS 비용을 없애는 구성적 결과

성공적으로 반환한 dense sampling이 accepted state와 controller에 영향을 주지 않고,
보간 계수·상태가 finite하다고 가정한다. 이 구현에서 clock session은 오직 span의 양끝만
본다. 따라서 같은 RHS·초기값·endpoint·tolerance로 interior 관측 시각만 바꿀 때
trial 순서, accept/reject, FSAL, 최종 상태, RHS 호출 수는 변하지 않는다.
full trial 수 A에서 정상 FSAL 작업량은 기존 결과대로 \(1+6A\)다. 새 보간을 위해
RHS, Jacobian, matrix solve, Newton 반복이 추가되지 않는다.

n차원, m개 interior observations, 관측을 포함한 accepted intervals 수 \(S_o\)에 대해
계수 준비는 \(O(S_o n)\), 샘플 평가는 \(O(mn)\), 출력 저장은 \(O(mn)\)이다.
상태 출력 자체에 \(\Omega(mn)\) 비용이 필요하므로 이 비용은 숨기면 안 된다.
반면 관측마다 착지하는 경로는 m개의 서로 다른 interior 시각만으로 최소 m+1 accepted
intervals를 요구한다. accepted trajectory가 적은 단계로 정확하게 표현될 수 있는 문제에서
새 방식은 관측 밀도에 비례하던 RHS 비용을 제거한다. 전체 실행 시간의 실제 이득은
RHS 비용·계수 구성·출력 저장에 따라 달라지며 별도의 paired 측정으로 판단한다.

## 코드 검토와 경계 사례

검토한 구조는 다음과 같다.

- `DenseSamples::prepare`는 해당 interval의 모든 요청을 pending vector에 계산하고,
  하나라도 nonfinite이면 공개하지 않는다. `Dopri5OutputSession::accept`는 준비가
  끝난 뒤 clock commit을 수행하고 마지막에 pending을 공개한다. driver 역시 그 이후에
  accepted state와 FSAL을 교체한다. 실패한 구간에서 일부 관측점만 공개하는 순서를 피했다.
- schedule은 strictly increasing, span의 양끝과 일치해야 하며 zero span을 별도로
  처리한다. final accepted time과 마지막 scheduled observation은 partial run에서
  서로 다를 수 있다. 공개 facade는 실제 마지막 accepted state를 별도로 보존한다.
- 계수 계산은 `P*(h*k)` 순서로 scale하여 `h*P`가 먼저 subnormal underflow하는 경로를
  줄인다. finite coefficient/state 검사는 유지한다. 계산 가능한 exact polynomial이
  있더라도 중간 cancellation 때문에 overflow할 수 있으므로 현 구현은 완전한
  overflow-free interpolation이라고 주장하지 않는다.
- 독립 검토에서 `t_old=-1e16`, `t_new=2`, `query=2.next_down()`인 유효 내부 시각에 대해
  `(query-t_old)/(t_new-t_old)`가 1로 반올림되는 사례를 발견했다. physical interval은
  정상인데 엄격한 `theta < 1` 검사만으로는 실패할 수 있다. 이 지적은 구현 담당자가 physical-time membership 검사와 endpoint-anchored
  식으로 수정했다. 오른쪽 식의 다항식 동치는 별도 Wolfram 계산에서 정확히 0으로
  확인했다. 회귀 시험은 추가되어 있으며 실제 실행 상태는 root의 고정 소스 receipt를
  따른다. 독립 코드 검토 상태는 `evidence/DENSE_CODE_REVIEW.json`에 있다.

정확 계수 조건은 통과했다. source snapshot hash는 review 시점의 소스 식별용이다.
이후 formatting·경계 처리 수정은 계수 literal 동일성 확인으로 연결하며, 기존 endpoint
5차 증명을 다시 수행할 필요는 없다. 실제 compiled tests와 benchmark의 최종 판정은
root가 고정한 실행 source와 해당 receipt를 따른다.

## 원전과 계수 출처

[SciPy 1.17.0 공식 RK45 문서](https://docs.scipy.org/doc/scipy-1.17.0/reference/generated/scipy.integrate.RK45.html)와
[같은 버전의 RK45.P 소스](https://github.com/scipy/scipy/blob/v1.17.0/scipy/integrate/_ivp/rk.py)를
열어 quartic extension과 계수 표를 확인했다. 원 논문의 서지는 L. F. Shampine,
“Some Practical Runge–Kutta Formulas”, Mathematics of Computation 46(173),
135–150 (1986), DOI:10.1090/S0025-5718-1986-0815836-3이다. AMS 본문 요청은
403으로 끝났으므로 본문을 직접 읽었다고 주장하지 않는다. 계수의 정확한 차수 조건은
원전 인용에 의존하지 않고 위 실제 대수 계산으로 확인했다. 접근 상태는
`evidence/DENSE_PRIMARY_SOURCES.json`에 남겼다.
