# Causal 원시 단계 방정식: 실제 궤적과 단일 스레드 비용

## 문제와 이번 연구의 범위

앞선 고정 단계 실험은 단계 방정식의 해를 정확히 계산하고 현재 단계 JVP를 제거할 수 있음을 보였다. 하지만 36개 고정 단계 중 물리적 내장 오차 조건을 만족한 경우는 6개뿐이었다. 따라서 그 시간비는 허용 오차를 만족하며 전체 구간을 적분한 비용을 뜻하지 않는다. 이번 실험은 이 공백을 직접 다룬다. 기존 코드의 autonomous, identity-mass, supplied-JVP 조건을 유지하고, **residual-first / causal-forward / causal-Picard**의 세 가지 실제 적응형 궤적을 같은 출력 시각과 원공간 잔차 기준으로 비교한다.

`frontier_causal_trajectory.rs`는 17개 물리적 출력 시각에 착지한다. 선형 감쇠(96 및 384차원), Riccati 비선형, 강성비 10⁴인 감쇠, 비정규 삼각 시스템, 각주파수 1000의 감쇠 진동(0.02 시간, 20 radians), 넓은 감쇠 스펙트럼을 포함한다. 마지막 계열은 basis 최대차원을 4로 제한해 투영 실패와 fallback 비용을 노출한다. 독립 해석해와 비교한 최대 절대 오차를 기준으로 `1e-5` 이하인 설정만 속도 후보가 된다. 동일한 tolerance라는 이유만으로 속도를 비교하지 않는다. 전체 시도, 거절, 투영 확장, fallback, 잔차 검사 비용을 기록한다. 기준은 연속 구간 전체 오차가 아니라 **17개 관측점의 오차**이며, 출력점 사이의 최대 오차를 보증하지 않는다. 진동 실험은 짧은 빠른 진동의 음성 대조군이며 장시간 위상 오차 검증을 대신하지 않는다.

## 정리 1: 공통 대각 블록을 이용한 정확한 제거

원시 RODAS 단계의 투영 식을

\[
W_m z_i=h\gamma V^T f(y+V\sum_{j<i}a_{ij}z_j)
 +\gamma\sum_{j<i}c_{ij}z_j,\qquad W_m=I-h\gamma V^T J_0V
\]

로 쓴다. A와 C는 엄밀히 하삼각이고, 모든 단계에서 같은 기저 V와 J₀를 사용하며 Wₘ이 가역이라고 가정한다. 첫 단계의 우변은 이미 알려져 있다. i−1단계까지의 값이 정해지면 i단계 우변에는 미지수가 없으므로 Wₘ의 한 번의 역적용으로 유일한 zᵢ가 정해진다. 수학적 귀납법으로 전체 투영 root는 유일하며 순방향 제거가 이를 정확히 계산한다. 따라서 **현재 단계 Jacobian과 Newton 갱신은 필요 없다.** 이는 frozen-base JVP까지 없앤다는 뜻은 아니다.

8단계의 reduced solve는 **공통 m×m LU 한 번 + 8회 backsolve**로 충분하다. 밀집 선형대수 비용은 O(m³+8m²), 저장공간은 O(m²+8m)이다. 전체 알고리즘에는 별도로 기저 생성 O(nm²), frozen JVP, lift/project O(8nm), 모델 RHS 및 원공간 잔차 검사가 남는다. 일반적인 8m×8m dense root 선형화의 한 번의 LU보다 leading cubic 차원이 줄어드는 것은 보장되지만, 선형화가 일어나지 않는 residual-first 입력에서 일률적인 총시간 개선은 보장되지 않는다.

또한 RHS가 비선형이어도 각 단계가 이전 단계에만 의존하므로 이 논증에 contraction 가정은 필요 없다. 반대로 Wₘ의 가역성만으로 원래 n차원 단계의 투영 정확도나 시간 적분 오차가 보장되지는 않는다. 기존 원공간 잔차 검사와 물리적 오차 검사를 유지해야 한다.

## 정리 2: 동시 Picard의 유한 정보 전파와 작업량

기존 `causal::picard`는 하삼각 affine 부분을 정확히 푼 뒤 nonlinear remainder를 동시에 평가한다. 0단계는 remainder와 무관하므로 초기 affine 해에서 이미 정확하다. k회 갱신 후 앞의 k+1단계가 정확하다고 하자. 다음 갱신에서 (k+1)번째 단계의 remainder는 정확한 선행 단계만 사용하므로 정확해진다. affine inverse도 하삼각이므로 이미 정확한 앞부분을 손상하지 않는다. 따라서 정확산술에서는 최대 7회 갱신 후 모든 단계가 일치한다. 구현은 표현 정밀도의 잔차를 별도로 확인하며 실패를 보존한다.

이 정리는 7회의 병렬 갱신이 7회의 직렬 RHS 평가보다 빠르다는 주장이 아니다. 각 갱신에서 최대 7개 RHS를 평가하므로 전체 RHS 작업은 증가할 수 있고, affine backsolve에는 순서 의존성이 남는다. 단일 스레드에서는 순방향 제거가 RHS 작업량 면에서 자연스러운 우선 후보다. Picard의 유지 조건은 **RHS가 충분히 비싸고, 이용 가능한 스레드에서 관측 오차를 맞춘 실제 궤적 시간이 줄어드는 것**이다. 저비용 RHS에서 동시화의 손실은 국소 후보 가지치기 근거이며, 병렬화 일반에 대한 불가능 정리는 아니다.

## 정리 3: 단계 잔차에서 전역 오차까지의 조건부 연결

가정: 원공간 W=I−hγJ₀가 가역이고 ‖W⁻¹‖₂≤K, 단계 상태를 연결하는 선분에서 f가 L-Lipschitz이며, 근사 단계의 원공간 residual을 ρᵢ라 한다. 정확/근사 단계 차이 eᵢ에 대해 식을 빼고 norm을 취하면

\[
\|e_i\|_2\le K\left(\|\rho_i\|_2+
 \sum_{j<i}(|\gamma c_{ij}|+|h\gamma|L|a_{ij}|)\|e_j\|_2\right).
\]

이 재귀는 하삼각이므로 작은 coupling을 가정하지 않아도 유한하게 계산된다. endpoint 교란은 η≤Σᵢ|bᵢ|‖eᵢ‖₂이다. 이것이 이미 구현된 `conditional_l2_error_estimate`의 근거이며, 실수 계산상의 residual 오차 allowance는 별도로 필요하다.

추가로 완전한 정확 root의 one-step map Ψₕ가 해당 궤적 주변에서 ‖Ψₕ(x)−Ψₕ(y)‖≤(1+λh)‖x−y‖ (λ≥0)을 만족하고, exact solution에서의 local defect가 C h⁶ 이하라고 하자. 각 accepted step의 root 교란이 ηₙ이면

\[
E_N\le e^{\lambda T}\left(E_0+C\sum_n h_n^6+\sum_n\eta_n\right)
\le e^{\lambda T}(E_0+CT h_{max}^5+\sum_n\eta_n).
\]

증명은 한 단계 오차 재귀 Eₙ₊₁≤(1+λhₙ)Eₙ+C hₙ⁶+ηₙ의 반복과 ∏(1+λhₙ)≤e^{λT}이다. 따라서 ηₙ≤ε_root hₙ/T로 root 예산을 배분하면 전체 root 기여는 e^{λT}ε_root 이하가 된다. **현재 operational residual screen만으로 K,L,λ,C 또는 이 root 예산이 검증되지는 않는다.** 여기서 λ는 수치 one-step map의 상수이며 연속 RHS의 Lipschitz 상수와 무조건 같다고 놓을 수 없다. 외부 검증된 dissipativity/resolvent bounds가 있는 클라이언트부터 이 조건부 예산을 구현하는 것이 다음 단계다.

## 후보 선택 및 남은 개발

- 순방향 제거: 전체 accepted trajectory 결과에서 오차와 비용을 확인한 뒤, supplied-JVP autonomous identity-mass 고객의 우선 모드로 노출한다. generic dispatcher default를 이번 데이터만으로 바꾸지 않는다.
- 동시 Picard: single-thread 기본에서 제외하는 방향을 검토하되, expensive RHS 및 2-thread 이상의 재현 가능한 이득이 확인되면 별도 모드로 유지한다.
- residual-first: 비교 기준 및 기존 비선형 continuation 연구 경로로 유지한다. 하삼각 target에서 매번 dense nonlinear root를 구성할 이유는 순방향 제거보다 약하다.

후속 개발 우선순위는 (1) 클라이언트가 제공한 조건부 K,L 및 수치 전달 안정도에 기반한 trajectory root budget, (2) raw-state workspace 재사용으로 per-step 할당 제거, (3) declared polynomial quotient 또는 물리적 분해로 frozen-basis JVP까지 줄이는 선택 경로, (4) variable mass/nonautonomous 확장을 **새 target 정의와 order 조건 검증** 하에 수행하는 것이다. 이번 supplied-JVP 경로를 완전 derivative-free라고 부르거나, 고정된 8단계의 causal depth를 시간 전체의 병렬 가속으로 부르지 않는다.

## 실제 실행 결과와 후보 가지치기

고정 native source `55ad7ec6af8b127cb90849db56abff4f02b9920b`와 동일 executable에서 단일 스레드 126회(3반복), 2스레드 84회(2반복)를 완료했다. 캠페인 wall은 각각 약 22.81초와 18.05초이며 warmup·참조 계산·직렬화도 포함한다. 개별 solve timer는 threadpool setup·참조 계산·직렬화를 제외한다. threadpool을 매 solve마다 생성하는 사용자의 cold latency는 이 수치보다 클 수 있다.

**210회가 구간 끝까지 도달한 것과 210회가 선택 정확도를 통과한 것은 다르다.** 단일 스레드에서는 117/126, 2스레드에서는 78/84가 사전 cap `1e-5`를 만족했다. 빠른 진동의 느슨한 tolerance `1e-4`는 모든 방법에서 최대 절대 오차 약 `2.66125e-5`로 cap을 넘었고 속도 선택에서 제외했다. 이를 숨기지 않고 `1e-6` 설정을 선택했다.

아래 시간은 같은 관측 오차 cap 이하인 두 tolerance 중 가장 빠른 중앙값이다. 1보다 큰 시간비는 비교 기준보다 빠르다는 뜻이다. 정확도가 다른 tolerance 두 개 중 최선이라는 유한 탐색이며 전역 최적점은 아니다.

| 문제 | residual-first (ms) | forward (ms) | Picard (ms) | 기준/forward | 기준/Picard | forward fallback/전체 시도 |
|---|---:|---:|---:|---:|---:|---:|
| few-mode | 1.4019 | 1.0729 | 1.1465 | 1.307 | 1.223 | 0/23 |
| few-mode-large | 4.5030 | 3.8621 | 4.2957 | 1.166 | 1.048 | 0/23 |
| riccati | 2.4932 | 0.8924 | 1.7466 | 2.794 | 1.427 | 0/19 |
| stiff | 1.7732 | 1.3189 | 1.5161 | 1.344 | 1.170 | 0/29 |
| nonnormal | 0.8039 | 0.5184 | 0.5904 | 1.551 | 1.362 | 0/19 |
| oscillatory | 4.2643 | 3.2479 | 3.9252 | 1.313 | 1.086 | 0/92 |
| broad | 643.5851 | 644.5124 | 709.5712 | 0.999 | 0.907 | 33/35 |

순방향 제거는 저차원 작용 공간이 충분한 여섯 문제에서 residual-first보다 중앙값 기준 약 1.17–2.79배 빠르고, 단일 스레드에서는 일곱 문제 모두 Picard보다 빨랐다. 여섯 중앙값 이득 중 다섯만 후속 계획의 1.2배 material-gain 기준을 넘는다. 384차원 감쇠의 1.17배는 작은 개선으로 별도 표시한다. Riccati의 경우 같은 19개 accepted steps에서 RHS가 507→304, JVP가 761→209로 줄었다. Picard도 JVP는 209로 줄지만 RHS는 612로 늘어 forward보다 비싸다. 이 차이는 단계당 주장에 머물지 않고 실제 수락된 전체 구간에서도 확인되었다.

넓은 스펙트럼에서는 결과가 다르다. loose tolerance에서 35시도 중 33회 fallback, tight tolerance에서 60시도 중 59회 fallback이 발생했다. forward의 loose 설정은 JVP 18,610회, RHS 1,598회를 사용했다. 이 항목의 시간비 0.999는 개선 근거가 아니며, 투영 공간과 fallback 비용이 실제 병목이다. 단순히 단계 Newton을 제거했다는 이유로 이 입력의 큰 비용이 사라지지 않는다.

### 2스레드의 실제 손익

동일 case/tolerance/method의 42설정에 대해 반복을 맞춘 **84쌍**에서 관측 시각·모든 관측 상태·공통 work counters·candidate totals·terminal failure가 binary64 bit 단위로 일치했다. 이 검사는 thread scheduling 통계의 `peak_parallel_tasks`를 같다고 요구하지 않는다. 같은 설정의 work는 42개 모두 같았다.

시간은 오히려 대부분 증가했다. 아래는 동일 설정의 `단일 스레드 중앙값 / 2스레드 중앙값` 분포다. 각 방법에 14설정이 있으며, 2스레드가 1.2배 이상 빨라진 설정은 **없었다**.

| 방법 | 최소 시간비 | 중앙 시간비 | 최대 시간비 | 2스레드가 조금이라도 빠른 설정 |
|---|---:|---:|---:|---:|
| residual-first | 0.385 | 0.728 | 0.999 | 0/14 |
| causal-forward | 0.448 | 0.748 | 1.030 | 2/14 |
| causal-picard | 0.317 | 0.646 | 1.055 | 2/14 |

따라서 이번 작은·저비용 RHS 환경에서는 **causal-forward 단일 스레드**를 선택적 권장 경로로 남기고, Picard를 작은 장비의 기본값으로 선택하지 않는다. two-thread의 극소 이득 두 건은 최대 약 5.5%로 반복 변동과 분리된 유의한 가속이라고 주장할 근거가 부족하다. 이는 expensive-RHS 모델에서의 crossover를 부정하는 결과가 아니며, 그 조건은 다음 실험으로 남긴다.

2스레드 2회와 단일 스레드 3회의 서로 다른 순차 캠페인에서 얻은 중앙값이다. 작업량과 수치 출력은 검증했지만 이 소규모 timing을 통계적으로 확정된 speedup이나 모든 CPU의 scalability로 확장하지 않는다. 새로운 native 회귀시험 수는 그대로 **2개 PASS**이며, 새 timing 행을 시험 개수로 합산하지 않는다. 전체 범용 production은 여전히 보류이며, 이 결과의 자격은 autonomous identity-mass supplied-JVP 문제의 지정 범위다.

근거: `results/causal_campaign/`, `results/causal_campaign_threads2/`, `results/CAUSAL_THREADS_AUDIT.json`. 독립 읽기 전용 재분석은 `src/analyze_causal_threads.py`로 재현한다.
