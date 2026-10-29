# Loop 12 비용 정리의 정확화와 실행 가능한 계수 컴파일

근거 상태: 아래 LC1–LC3는 직접 유도한 조건부 주장이다. 독립 최종 판정은 이 패키지의 `DECISION.json`에 둔다. 첨부 원문과 원래 HOLD 판정은 수정하지 않는다.

## LC1. 출력 준비 비용을 포함하는 자원 상계

원문의 \(n\)은 모드 수, \(p\)는 주파수 kernel 차수, \(D\)는 mean waveform 차수, \(S\)는 순차 Picard sweep 수, \(R_o\)는 서로 다른 출력 시점 수다. 계수 곱은 직접 합성곱으로 수행하고 nonlinear cubic 비용은 \(O(D^2)\)다. 계수별 산술을 한 단위로 세며 가변 길이 정수의 bit 비용은 별도다.

원문 §7의 일반식은

\[
W=O\{np+S(D^2+pD)+R_o(np+pD)\}.
\]

이 식에서 마지막 \(R_opD\)는 각 출력 시점에서 공통 복원 다항식을 준비하는 비용이다. 서로 다른 출력 시점에서는 적분 가중치와 평가 시간이 바뀌므로 일반적으로 한 번만 지불했다고 처리할 수 없다. 따라서 고정 analytic margin에서 \(\ell=1+\log(1/\epsilon)\), \(p,D,S=O(\ell)\)를 대입한 올바른 일반 상계는

\[
\boxed{W=O\{n\ell+\ell^3+R_o(n\ell+\ell^2)\}.}
\]

증명은 각 항의 대입으로 끝난다. \(R_o=O(1)\)이면 \(R_o\ell^2\)를 \(\ell^3\)에, 또는 \(n\ge c\ell\)이면 \(R_o\ell^2\)를 \(R_on\ell\)에 흡수할 수 있다. 이러한 조건 없이 원문의 더 짧은 최종식을 사용하지 않는다. 원문 §7의 일반식과 동일 맵 항등식 자체는 이 수정으로 바뀌지 않는다.

임의 정확도에 대한 정리는 precision도 함께 선택하는 자원 정리다. `sufficient_parameters(..., bits=112)`의 기본값과 유한 반복 cap을 가진 현재 Python 함수를 무제한 정확도 실현으로 승격하지 않는다.

## LC2. factorial 재계산 없는 causal weight 표

시간을 \(\tau=t/T\)로 정규화한다. 차수 \(j,k\ge0\)에 대해

\[
w_{jk}=\int_0^1(1-s)^j s^k\,ds
=\frac{j!k!}{(j+k+1)!}
\]

이며

\[
w_{j0}=\frac1{j+1},\qquad
w_{j,k+1}=w_{jk}\frac{k+1}{j+k+2}.
\]

두 번째 식은 factorial 비를 취해서 얻는다. 따라서 \(0\le j\le p\), \(0\le k\le d_f\) 표를 \(O((p+1)(d_f+1))\)번의 유리수 연산으로 생성할 수 있다. 메모리는 같은 차수다. 표를 한번 생성한 후 모든 모드와 sweep이 공유한다. 실제 물리 적분에는 \(T\)를 곱한다. 이 표는 circular convolution 또는 pointwise multiplication을 나타내지 않는다.

원문의 `conv`는 각 \((j,k)\) 쌍에서 factorial을 다시 계산한다. 따라서 원문에서 센 coefficient-product 수와 Python 프로그램의 모든 scalar 연산 수를 동일시해서는 안 된다. 이번 `causal_weights.py`는 위 점화식으로 표를 생성하고, 동일한 causal integral을 factorial 재계산 없이 계산하는 실행 가능한 exact reference다. `check_causal_weights.py`는 104개 가중치의 닫힌 식 일치, causal 단항식, 가중합 교환, endpoint 공통 계수 및 잘못된 차수 거절을 검사한다. 이는 전체 solver나 native Rust 실행 검증이 아니다.

공간 상계도 구현에 맞추어 구분한다. 원문의 공간 상계를 유지하려면 가중치를 streaming recurrence로 생성할 수 있으며 이 경우 추가 공간은 한 행 \(O(d_f)\)까지 줄일 수 있으나 sweep마다 생성 비용을 지불한다. 이번 cache prototype은 전체 표 \(O(pd_f)\)를 유지한다. **이 prototype의 공간은 \(O(n+p+D+pD)\)**이고, 원문이 제시한 \(O(n+p+D)\)와 같다고 주장하지 않는다. 여러 차수를 시험하면 최대 차수 표의 prefix를 재사용할 수 있다.

## LC3. 같은 맵에 대한 엄격한 총 작업량 개선 조건

동일한 kernel 차수 \(p\), forcing 최대차수 \(d_f\), projection, sweep 수 \(S\), exact unrounded arithmetic을 고정한다. 비교 대상은 nonlinear mean forcing을 이미 한 번만 계산하지만 각 모드의 kernel convolution을 따로 수행하는 full waveform 방식이다. 공유 forcing 비용은 양쪽에서 같다. 공유 가중치 표를 양쪽 모두 사용해 구현상 불필요한 불이익을 baseline에 주지 않는다.

아래 정확한 계수는 모든 sweep에서 kernel을 p+1개, forcing을 d_f+1개로 zero-padding하고 영 계수도 세는 dense schedule에 대한 것이다.

한 coefficient pair의 비용을 양쪽에서 동일한 \(c_v>0\)라고 할 때 kernel 부분의 절감량은 정확히

\[
\Delta W_{\rm ker}
=S(n-1)(p+1)(d_f+1)c_v.
\]

실제 길이를 trim하는 schedule이라면 위 식 대신 \(\Delta W_{\rm ker}=(n-1)(p+1)c_v\sum_{s=1}^{S}(d_{f,s}+1)\)를 사용한다(kernel 길이는 고정 \(p+1\)개). 최대 forcing 차수를 실제 각 sweep 길이로 대체하지 않는다.

가중합과 causal 적분 및 동일한 선형 degree projection은 서로 교환한다. 초기 free profile에서도 가중합이 같으므로, forcing과 다음 mean profile이 같다는 사실을 sweep마다 귀납하여 정확한 동일 맵을 얻는다. 이것은 서로 다른 rounding 순서의 bit identity를 주장하지 않는다.

비공통 비용의 packet-minus-baseline 상계를 \(C_\Delta\)라 하자. moment compilation, metadata와 certificate, 출력별 공통 계수와 전 모드 복원, 실패 trial, allocation, table 보관/생성, scalar precision 비용 중 비교에서 상쇄되지 않은 부분은 모두 여기에 넣는다. 그러면

\[
\boxed{C_\Delta<S(n-1)(p+1)(d_f+1)c_v}
\]

이면 같은 산술 비용 모델에서 총 작업량이 엄격히 감소한다. 이는 양쪽 총 비용에서 공통 항을 빼고 위 정확한 차이를 대입하는 증명이다. 양쪽의 primitive 단가가 다르면 동일 \(c_v\) 대신 baseline의 검증된 하계와 packet의 상계를 사용해야 한다.

조건은 공집합이 아니다. \(n=128,p=4,d_f=18,S=6,c_v=1\)에서 kernel 절감은 \(72,390\) 단위다. 비공통 추가 비용을 \(10,000\) 단위 이하로 실제로 묶은 구현이라면 최소 \(62,390\) 단위의 총 작업량 개선이 보장된다. 이 추가 비용 상한은 **예제의 명시적 전제**이며 현재 native 구현에서 측정한 값이 아니다. 이미 같은 moment hoisting을 쓰는 최적화 baseline에는 이 \(n\)배 절감을 다시 청구하지 않는다.

## 기존 승격 이론과의 결합

이전 P1/P2의 소멸다항식 압축은 **원래 전체 연산자**에 검증된 \(q(X)\)가 있을 때 적용된다. Loop 12에서 만든 nilpotent moment shift는 **근사 packet 연산자**의 정확한 구조다. 이 구조에 finite-jet 대수를 적용할 수 있지만, 원래 서로 다른 \(n\)개 주파수가 같은 저차 다항식에 소멸한다고 바꿔 읽을 수 없다. 원모델과 packet 사이에는 kernel tail을 반드시 남긴다.

또한 packet shift의 nilpotence, 연속 Volterra 연산자의 factorial power bound, RVJ K-form stage의 삼각성은 서로 다른 명제다. 이번 upgrade는 이전 H5의 leakage 보정 및 실제 K-form 승격 보류를 해제하지 않는다. 실행 가능한 새 backend를 연결하려면 이 패키지의 source binding과 port DAG를 따른다.
